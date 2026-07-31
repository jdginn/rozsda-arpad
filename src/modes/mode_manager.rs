use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender, select};
use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::reaper_channel_strip_mode::ChannelStripMode;
use crate::modes::reaper_track_sends::TrackSendsMode;
use crate::modes::reaper_vol_pan::VolumePanMode;
use crate::track::track::TrackMsg;

// High-level rethink of how ModeManager should work:
//
// - We want to switch between modes and manage as much of the state change as possible here.
// - We don't want to cache track state here; that should happen upstream from us in TrackManager.
// - We want to handle Barrier propagation: when we switch modes, we need to send a Barrier upstream
//   and wait for ALL the accumulated state from upstream to come back down to us. We don't process ANY messages until that happens.
// - We also need to wait for the Barrier to be sent downstream and reflected back up before we
//   forward ANY messages from downstream to upstream. This ensures that all the stimulus we receive
//   from downstream is based on the latest state from upstream.
// - Bonus points if we coalesce messages when sending downstream; i.e. avoid duplicate or
//   superseded messages. This may not be possible, and we may be able to implement the coalescing
//   at a lower level, but it's worth thinking about.
// - Ideally, the individual modes DO NOT need to know anything about barriers or mode transition
//   with these exceptions:
//   1. Modes need to be able to request a mode transition.
//   2. Modes may need to do custom initialization when we enter them.
//
// How I think it works today:
// - ModeManager runs a thread that receives messages from both upstream and downstream
// - ModeManager knows which mode is active and calls specialized message handling logic for that mode
//      Specialized logic is defined by a method within each mode
// - ModeManager keeps track of mode change requests including:
//   1. What mode we came from
//   2. What mode we are transitioning to
//   3. Which track, if any, is selected, because selected track dramatically influences how certain modes work
//   4. ModeManager is responsible for sending barriers upstream and downstream
// - HOWEVER, processing barriers is handled in each mode. FIXME: this is a problem
//
// - The ModeManager thread calls a private function `handle_transitions`, which is a bit clunky and
//   difficult to reason about
//
// How it might work better:
// - Don't make modes responsible for doing anything with barriers. ModeManager would need to have a
//   reference to the downstream channel (i.e. `to_v1m`) so it can send barriers downstream.
// - It should be a hard panic if a mode ever receives a barrier message
// - When we do not have an active mode (i.e. we are performing a mode transition), we should not
//   call ANYTHING in the mode implementations. It should all live in mode_manager.
// - The modes themselves should be as stateless as possible. We should not see any transition struct
//   types in the mode implementations. In fact, it might be best if these struct types are private to this module?
// - Perhaps each mode should respond upward with a special RequestTransition message, then all
//   communication happens over channels and we don't have both function calls and channels communicating state at the same time.
//   A potential weakness of such an approach is that it woudl be tempting to make each mode run on its own thread, which we don'think
//   really want either. Two modes shouldn't ever run concurrently, since they are each stateless
//   views into the same upstream state and downstream stimuli. Modes are mutually exclusive.
// - It may be better to reconstruct the mode each time we enter it? There could potentially be
//   initialization cost (allocations), although we are already reseting the modes within their own `initialize_mode_tansition` methods
// - Ideally, modes shouldn't have to call reset on themselves. If they need to be reset, we should be able to call them from here.
// - IMPORTANT ModeManager should not need to store things behind Arc<Mutex<>>! If we find ourselves
//   needing to do this, our implementation is probably wrong!

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
                State::Active => match manager.curr_mode {
                    // TODO: what do we do with selected track?
                    Mode::ReaperVolPan => {
                        // TODO: flush all coalesced messages downstream
                        let mut handler = VolumePanMode::new(8);
                        loop {
                            select! {
                            recv(manager.from_reaper) -> msg => {
                                if let Ok(msg) = msg {
                                    match handler.handle_msg_from_upstream(msg, &manager.senders) {
                                        ModeAction::None => {},
                                        ModeAction::SelectedTrackChanged(guid) => {
                                            manager.reaper_currently_selected_track_guid = guid
                                        }
                                        ModeAction::Transition(new_mode) => {
                                            manager.senders.send_to_reaper(TrackMsg::QueryAll);
                                            let barrier = Barrier::new(manager.curr_mode, new_mode);
                                            manager.senders.send_to_reaper(TrackMsg::Barrier(barrier));
                                            manager.curr_mode = new_mode;
                                            manager.curr_state = State::WaitingBarrierFromUpstream(barrier);
                                            break
                                        }
                                    }
                                }
                            }
                            recv(manager.from_v1m) -> msg => {
                                if let Ok(msg) = msg {
                                    match handler.handle_msg_from_downstream(msg, &manager.senders) {
                                        ModeAction::None => {},
                                        ModeAction::SelectedTrackChanged(guid) => {
                                            manager.reaper_currently_selected_track_guid = guid
                                        }
                                        ModeAction::Transition(new_mode) => {
                                            manager.senders.send_to_reaper(TrackMsg::QueryAll);
                                            let barrier = Barrier::new(manager.curr_mode, new_mode);
                                            manager.senders.send_to_reaper(TrackMsg::Barrier(barrier));
                                            manager.curr_mode = new_mode;
                                            manager.curr_state = State::WaitingBarrierFromUpstream(barrier);
                                            break
                                        }
                                    }
                                }
                            }
                            }
                        }
                    }
                    Mode::ReaperSends => {
                        //TODO:
                    }
                    _ => panic!("Unimplemented"),
                },
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
