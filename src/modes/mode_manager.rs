use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender, select};
use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::reaper_channel_strip_mode::ChannelStripMode;
use crate::modes::reaper_track_sends::TrackSendsMode;
use crate::modes::reaper_vol_pan::VolumePanMode;
use crate::track::track::TrackMsg;

// Global atomic counter for unique IDs
static BARRIER_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

/// A synchronization barrier to allow us to ensure that all data relevant to some mode transition
/// is processed before we continue forwarding messages.
///
/// Barriers are unique.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Barrier {
    id: u64,
    pub from: Mode,
    pub to: Mode,
}

// Generate a new barrier with a unique ID
impl Barrier {
    pub fn new(from: Mode, to: Mode) -> Self {
        let id = BARRIER_COUNTER.fetch_add(1, Ordering::SeqCst);
        Barrier { id, from, to }
    }
}

/// Represents state of mode manager: mostly whether we are in a mode transition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    // Normal operation: forward messages in both directions
    Active,
    // Waiting from messages from upstream to be passed all the way downstream
    WaitingBarrierFromUpstream(Barrier),
    // All messages from upstream have been passed downward; waiting for downstream to confirm all
    // messages have been applied
    WaitingBarrierFromDownstream(Barrier),
}

/// Represents the various control modes supported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    ReaperVolPan,
    ReaperSends,
    ReaperChannelStrip,
    MotuVolPan,
}

pub struct Senders {
    to_reaper: Sender<TrackMsg>,
    to_v1m: Sender<v1m::DownstreamMsg>,
}

impl Senders {
    pub fn new(to_reaper: Sender<TrackMsg>, to_v1m: Sender<v1m::DownstreamMsg>) -> Self {
        Senders { to_reaper, to_v1m }
    }

    pub fn send_to_reaper(&self, msg: TrackMsg) {
        self.to_reaper.send(msg).unwrap();
    }

    pub fn to_v1m(&self, msg: v1m::DownstreamMsg) {
        self.to_v1m.send(msg).unwrap();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeAction {
    None,
    SelectedTrackChanged(Option<Uuid>),
    Transition(Mode),
}

/// Each mode implementation struct needs to implement this trait to handle messages
///
/// Each mode implementation should also implement initiate_mode_transition(self, ...) -> ModeState. This implementation
/// will vary from mode to mode but usually will require sending a barrier to the upstream channel.
pub trait ModeHandler {
    fn handle_msg_from_upstream(&mut self, msg: TrackMsg, io: &Senders) -> ModeAction;
    fn handle_msg_from_downstream(&mut self, msg: v1m::UpstreamMsg, io: &Senders) -> ModeAction;
}

/// Presents all modes with a uniform interface, (mostly) seamlessly handling switching between modes.
///
/// Shields upstream and downstream from having to know anything about the modes.
/// The only exception is that both upstream and downstream need to support refleting barriers when
/// they receive them.
///
/// Logic for each mode's behavior lives in a separate struct that exposes message handlers
//
// TODO: someday turn handler methods into a trait?
pub struct ModeManager {
    from_reaper: Receiver<TrackMsg>,
    from_v1m: Receiver<v1m::UpstreamMsg>,
    senders: Senders,

    curr_mode: Mode,
    curr_state: State,

    reaper_currently_selected_track_guid: Option<Uuid>,
}

/// Returns true if we should break the loop
fn apply_mode_action(manager: &mut ModeManager, action: ModeAction) -> bool {
    match action {
        ModeAction::None => false,
        ModeAction::SelectedTrackChanged(guid) => {
            manager.reaper_currently_selected_track_guid = guid;
            false
        }
        ModeAction::Transition(new_mode) => {
            manager.senders.send_to_reaper(TrackMsg::QueryAll);
            let barrier = Barrier::new(manager.curr_mode, new_mode);
            manager.senders.send_to_reaper(TrackMsg::Barrier(barrier));
            manager.curr_mode = new_mode;
            manager.curr_state = State::WaitingBarrierFromUpstream(barrier);
            false
        }
    }
}

impl ModeManager {
    /// Spawns a thread that listens to upstream and downstream channels, forwarding messages as
    /// appropriate and silently handling mode transitions.
    pub fn start(
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_v1m: Receiver<v1m::UpstreamMsg>,
        to_v1m: Sender<v1m::DownstreamMsg>,
    ) {
        let senders = Senders { to_reaper, to_v1m };

        let mut manager = ModeManager {
            from_reaper,
            from_v1m,
            senders,

            curr_mode: Mode::ReaperVolPan,
            curr_state: State::Active,

            reaper_currently_selected_track_guid: None,
        };

        loop {
            match manager.curr_state {
                State::Active => {
                    // TODO: flush all coalesced messages downstream
                    let mut handler = VolumePanMode::new(8);
                    loop {
                        select! {
                            recv(manager.from_reaper) -> msg => {
                                if let Ok(msg) = msg {
                                    let action = match manager.curr_mode {
                                        Mode::ReaperVolPan => handler.handle_msg_from_upstream(msg, &manager.senders),
                                        _ => panic!("Unimplemented"),
                                    };
                                    if apply_mode_action(&mut manager, action) {
                                        break
                                    }
                                }
                            }
                            recv(manager.from_v1m) -> msg => {
                                if let Ok(msg) = msg {
                                    let action = match manager.curr_mode {
                                        Mode::ReaperVolPan => handler.handle_msg_from_downstream(msg, &manager.senders),
                                        _ => panic!("Unimplemented"),
                                    };
                                    if apply_mode_action(&mut manager, action) {
                                        break
                                    }
                                }
                            }
                        }
                    }
                }
                State::WaitingBarrierFromUpstream(expected_barrier) => {
                    loop {
                        select! {
                            recv(manager.from_reaper) -> msg => {
                                // Yes, for now we just want to panic on error
                                match msg.unwrap() {
                                    TrackMsg::Barrier(barrier) => {
                                        // Forward barriers downstream (they need to reflect back upstream for the mode to
                                        // transition)
                                        manager.senders.to_v1m
                                            (v1m::DownstreamMsg::Barrier(barrier));
                                        if barrier == expected_barrier {
                                            // If we were already waiting on a barrier from upstream, check if this is the one
                                            // we were waiting for. If yes, transition to waiting for the barrier to reflect back up from downstream.
                                            manager.curr_state = State::WaitingBarrierFromDownstream(barrier);
                                            break
                                        }
                                    },
                                    _ => {
                                        //TODO: coalesce
                                    }
                                }
                            }
                            recv(manager.from_v1m) -> msg => {
                                // Discard all messages from downstream until we have received the barrier from upstream
                            }
                        }
                    }
                }
                State::WaitingBarrierFromDownstream(expected_barrier) => {
                    loop {
                        select! {
                            recv(manager.from_reaper) -> msg => {
                                // TODO: coalesce all messages
                                // TODO: what do we do with other barriers?
                            }
                            recv(manager.from_v1m) -> msg => {
                                match msg.unwrap() {
                                    v1m::UpstreamMsg::Barrier(barrier) => {
                                        if barrier == expected_barrier {
                                            manager.curr_state = State::Active;
                                            break
                                        } else {
                                            // This is a barrier for a previous mode transition that we have already passed, so we can ignore it
                                            // (We should only be receiving barriers for the current mode transition we are in, but just in case...)
                                        }
                                    },
                                    _ => {
                                        // Discard all messages from downstream until we have received the barrier from downstream
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
