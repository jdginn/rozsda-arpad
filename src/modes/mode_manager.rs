use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender, select};
use uuid::Uuid;

use crate::midi::v1m;
use crate::modes::coalesce::OrderedCoalescingBuffer;
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
}

// Generate a new barrier with a unique ID
impl Barrier {
    pub fn new() -> Self {
        let id = BARRIER_COUNTER.fetch_add(1, Ordering::SeqCst);
        Barrier { id }
    }
}

impl Default for Barrier {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents state of mode manager: mostly whether we are in a mode transition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    // Normal operation: forward messages in both directions
    Active,
    // Waiting from messages from upstream to be passed all the way downstream
    WaitingBarrierFromUpstream { barrier: Barrier },
    // All messages from upstream have been passed downward; waiting for downstream to confirm all
    // messages have been applied
    WaitingBarrierFromDownstream { barrier: Barrier },
}

/// Represents the various control modes supported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    ReaperVolPan,
    ReaperSends,
    ReaperChannelStrip,
    MotuVolPan,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransitionRequest {
    ToReaperVolumePan { selected_track_guid: Option<Uuid> },
    ToReaperSends { selected_track_guid: Uuid },
    ToReaperChannelStrip { selected_track_guid: Uuid },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeAction {
    None,
    Transition(TransitionRequest),
}

pub trait ToV1m {
    fn send_to_v1m(&mut self, msg: v1m::DownstreamMsg);
}

pub trait DownstreamIo: ToV1m {
    fn send_to_reaper(&self, msg: TrackMsg);
}

pub trait UpstreamIo: ToV1m {}

pub struct IoDirect {
    to_reaper: Sender<TrackMsg>,
    to_v1m: Sender<v1m::DownstreamMsg>,
}

impl IoDirect {
    fn new(to_reaper: Sender<TrackMsg>, to_v1m: Sender<v1m::DownstreamMsg>) -> Self {
        IoDirect { to_reaper, to_v1m }
    }
}

impl ToV1m for IoDirect {
    fn send_to_v1m(&mut self, msg: v1m::DownstreamMsg) {
        self.to_v1m.send(msg).unwrap();
    }
}

impl DownstreamIo for IoDirect {
    fn send_to_reaper(&self, msg: TrackMsg) {
        self.to_reaper.send(msg).unwrap();
    }
}

impl UpstreamIo for IoDirect {}

pub struct IoCoalescing {
    to_reaper: Sender<TrackMsg>,
    to_v1m_buffer: OrderedCoalescingBuffer<v1m::DownstreamMsg>,
}

impl IoCoalescing {
    fn new(to_reaper: Sender<TrackMsg>) -> Self {
        IoCoalescing {
            to_reaper,
            to_v1m_buffer: OrderedCoalescingBuffer::new(),
        }
    }
}

impl ToV1m for IoCoalescing {
    fn send_to_v1m(&mut self, msg: v1m::DownstreamMsg) {
        self.to_v1m_buffer.push(msg);
    }
}

impl DownstreamIo for IoCoalescing {
    fn send_to_reaper(&self, msg: TrackMsg) {
        self.to_reaper.send(msg).unwrap();
    }
}

impl UpstreamIo for IoCoalescing {}

impl IoCoalescing {
    pub fn flush_into(&mut self, io: &IoDirect) {
        for msg in self.to_v1m_buffer.drain() {
            io.to_v1m.send(msg).unwrap();
        }
    }
}

/// Each mode implementation struct needs to implement this trait to handle messages
///
/// Each mode implementation should also implement initiate_mode_transition(self, ...) -> ModeState. This implementation
/// will vary from mode to mode but usually will require sending a barrier to the upstream channel.
pub trait ModeHandler {
    fn handle_msg_from_upstream(&mut self, msg: TrackMsg, io: &mut dyn UpstreamIo) -> ModeAction;
    fn handle_msg_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
        io: &mut dyn DownstreamIo,
    ) -> ModeAction;
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
    io_direct: IoDirect,
    io_coalescing: IoCoalescing,

    curr_state: State,

    handler: Box<dyn ModeHandler>,
}

/// Returns true if we should break the loop
fn apply_mode_action(manager: &mut ModeManager, action: ModeAction) -> bool {
    match action {
        ModeAction::None => false,
        ModeAction::Transition(transition_request) => {
            manager.handler = match transition_request {
                //TODO: pass io_coalescing
                TransitionRequest::ToReaperVolumePan {
                    selected_track_guid,
                } => Box::new(VolumePanMode::new(8, selected_track_guid)),
                TransitionRequest::ToReaperSends {
                    selected_track_guid,
                } => Box::new(TrackSendsMode::new(8, selected_track_guid)),
                TransitionRequest::ToReaperChannelStrip {
                    selected_track_guid,
                } => Box::new(ChannelStripMode::new(8, selected_track_guid)),
            };
            manager.io_direct.send_to_reaper(TrackMsg::QueryAll);
            let barrier = Barrier::new();
            manager.io_direct.send_to_reaper(TrackMsg::Barrier(barrier));
            manager.curr_state = State::WaitingBarrierFromUpstream { barrier };
            true
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
        let mut manager = ModeManager {
            from_reaper,
            from_v1m,
            io_direct: IoDirect::new(to_reaper.clone(), to_v1m),
            io_coalescing: IoCoalescing::new(to_reaper),

            curr_state: State::Active,

            handler: Box::new(VolumePanMode::new(8, None)),
        };

        loop {
            match manager.curr_state {
                State::Active => {
                    // TODO: flush all coalesced messages downstream
                    loop {
                        select! {
                            recv(manager.from_reaper) -> msg => {
                                if let Ok(msg) = msg {
                                    let action = manager.handler.handle_msg_from_upstream(msg, &mut manager.io_direct);
                                    if apply_mode_action(&mut manager, action) {
                                        break
                                    }
                                }
                            }
                            recv(manager.from_v1m) -> msg => {
                                if let Ok(msg) = msg {
                                    let action = manager.handler.handle_msg_from_downstream(msg, &mut manager.io_direct);
                                    if apply_mode_action(&mut manager, action) {
                                        break
                                    }
                                }
                            }
                        }
                    }
                }
                State::WaitingBarrierFromUpstream {
                    barrier: expected_barrier,
                } => {
                    loop {
                        select! {
                            recv(manager.from_reaper) -> msg => {
                                if let Ok(msg) = msg {
                                    match msg {
                                        TrackMsg::Barrier(barrier) => {
                                            if barrier == expected_barrier {
                                                // If this is the barrier we were waiting for, flush all coalesced messages downstream and transition to
                                                // waiting for the barrier to reflect back up from downstream.
                                                manager.io_coalescing.flush_into(&manager.io_direct);
                                                manager.io_direct.send_to_v1m
                                                    (v1m::DownstreamMsg::Barrier(barrier));
                                                // If we were already waiting on a barrier from upstream, check if this is the one
                                                // we were waiting for. If yes, transition to waiting for the barrier to reflect back up from downstream.
                                                manager.curr_state = State::WaitingBarrierFromDownstream{barrier};
                                                break
                                            }
                                            // Forward all barriers downstream
                                            manager.io_direct.send_to_v1m
                                                (v1m::DownstreamMsg::Barrier(barrier));
                                        },
                                        _ => {
                                            // Coalesce all messages from upstream
                                            manager.handler.handle_msg_from_upstream(msg, &mut manager.io_coalescing);
                                        }
                                    }
                                } else {panic!("Error receiving from reaper channel")}
                            }
                            recv(manager.from_v1m) -> _ => {
                                // Discard all messages from downstream until we have received the barrier from upstream
                            }
                        }
                    }
                }
                State::WaitingBarrierFromDownstream {
                    barrier: expected_barrier,
                } => {
                    loop {
                        select! {
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
