use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender, select};

use crate::midi::xtouch::{XTouchDownstreamMsg, XTouchUpstreamMsg};
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
        Barrier::new()
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
    MotuVolPan,
}

/// Represents the current mode and state of the mode manager.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModeState {
    pub mode: Mode,
    pub state: State,
}

/// Each mode implementation struct needs to implement this trait to handle messages
///
/// Each mode implementation should also implement initiate_mode_transition(self, ...) -> ModeState. This implementation
/// will vary from mode to mode but usually will require sending a barrier to the upstream channel.
pub trait ModeHandler<ToUpstream, FromUpstream, ToDownstream, FromDownstream> {
    fn handle_upstream_messages(&mut self, msg: FromDownstream, curr_mode: ModeState) -> ModeState;
    fn handle_downstream_messages(&mut self, msg: FromUpstream, curr_mode: ModeState) -> ModeState;
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
    to_reaper: Sender<TrackMsg>,
    from_xtouch: Receiver<XTouchUpstreamMsg>,
    to_xtouch: Sender<XTouchDownstreamMsg>,
    curr_mode: ModeState,
}

impl ModeManager {
    /// Spawns a thread that listens to upstream and downstream channels, forwarding messages as
    /// appropriate and silently handling mode transitions.
    pub fn start(
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_xtouch: Receiver<XTouchUpstreamMsg>,
        to_xtouch: Sender<XTouchDownstreamMsg>,
    ) {
        let mut manager = ModeManager {
            from_reaper: from_reaper.clone(),
            to_reaper: to_reaper.clone(),
            from_xtouch: from_xtouch.clone(),
            to_xtouch: to_xtouch.clone(),
            curr_mode: ModeState {
                mode: Mode::ReaperVolPan,
                state: State::Active,
            },
        };

        // Each mode's implementation struct needs to be initialized here
        let mut reaper_pan_vol = VolumePanMode::new(
            8, // For now, assume we have 8 faders on the conroller
            from_reaper.clone(),
            to_reaper.clone(),
            from_xtouch.clone(),
            to_xtouch.clone(),
        );

        thread::spawn(move || {
            loop {
                select! {
                    recv(manager.from_reaper) -> msg => {
                        if let Ok(track_msg) = msg {
                        match manager.curr_mode.mode {
                        Mode::ReaperVolPan => {
                            // TODO: Do we need to gate this during transition? I think probably
                                // not, since upstream changes are by definition authoritative, and
                                // if we apply the upstream change early, that should only be
                                // helping us be more correct.
                                // The only downside I can think of is if an upstream message gets
                                // superseded by a future upstream message, which could cause a bit
                                // of jitter on the hw. But even then, we are not propagating
                                // hardware settings upstream, so upstream should still always be
                                // correct.
                            manager.curr_mode = reaper_pan_vol.handle_downstream_messages(track_msg, manager.curr_mode)
                        },
                        _ => {panic!("Inside unknown mode in ModeManager")},
                        }
                    }
                }
                    recv(manager.from_xtouch) -> msg => {
                        if let Ok(xtouch_msg) = msg {
                            match manager.curr_mode.mode{
                                Mode::ReaperVolPan => {
                                    match manager.curr_mode.state {
                                        State::Active => {
                                            manager.curr_mode = reaper_pan_vol.handle_upstream_messages(xtouch_msg, manager.curr_mode);
                                        },
                                        // We don't send any messages up from the hw until the hw
                                        // is confirmed to reflect the upsream state
                                        State::WaitingBarrierFromDownstream(_) => {
                                            // Block
                                        },
                                        State::WaitingBarrierFromUpstream(_) => {
                                            // Block
                                        },
                                    }
                                },
                                _ => {panic!("Inside unknown mode in ModeManager")},
                            }
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for Barrier

    #[test]
    fn test_barrier_new_creates_unique_ids() {
        let barrier1 = Barrier::new();
        let barrier2 = Barrier::new();
        assert_ne!(barrier1, barrier2);
    }

    #[test]
    fn test_barrier_clone_preserves_id() {
        let barrier1 = Barrier::new();
        let barrier2 = barrier1.clone();
        assert_eq!(barrier1, barrier2);
    }

    #[test]
    fn test_barrier_default() {
        let barrier = Barrier::default();
        // Just verify it creates a barrier - can't check much else without exposing id
        assert_ne!(barrier, Barrier::new()); // Should get different ID
    }

    // Tests for State

    #[test]
    fn test_state_active_equality() {
        let state1 = State::Active;
        let state2 = State::Active;
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_state_waiting_barrier_upstream() {
        let barrier = Barrier::new();
        let state = State::WaitingBarrierFromUpstream(barrier);
        assert!(matches!(state, State::WaitingBarrierFromUpstream(_)));
    }

    #[test]
    fn test_state_waiting_barrier_downstream() {
        let barrier = Barrier::new();
        let state = State::WaitingBarrierFromDownstream(barrier);
        assert!(matches!(state, State::WaitingBarrierFromDownstream(_)));
    }

    #[test]
    fn test_state_clone() {
        let state1 = State::Active;
        let state2 = state1.clone();
        assert_eq!(state1, state2);
    }

    // Tests for Mode

    #[test]
    fn test_mode_equality() {
        assert_eq!(Mode::ReaperVolPan, Mode::ReaperVolPan);
        assert_ne!(Mode::ReaperVolPan, Mode::ReaperSends);
        assert_ne!(Mode::ReaperSends, Mode::MotuVolPan);
    }

    #[test]
    fn test_mode_clone() {
        let mode1 = Mode::ReaperSends;
        let mode2 = mode1.clone();
        assert_eq!(mode1, mode2);
    }

    #[test]
    fn test_mode_copy() {
        let mode1 = Mode::ReaperVolPan;
        let mode2 = mode1; // Copy, not move
        assert_eq!(mode1, mode2);
        // Verify mode1 is still usable (copy trait)
        assert_eq!(mode1, Mode::ReaperVolPan);
    }

    // Tests for ModeState

    #[test]
    fn test_mode_state_creation() {
        let mode_state = ModeState {
            mode: Mode::ReaperVolPan,
            state: State::Active,
        };
        assert_eq!(mode_state.mode, Mode::ReaperVolPan);
        assert_eq!(mode_state.state, State::Active);
    }

    #[test]
    fn test_mode_state_equality() {
        let state1 = ModeState {
            mode: Mode::ReaperVolPan,
            state: State::Active,
        };
        let state2 = ModeState {
            mode: Mode::ReaperVolPan,
            state: State::Active,
        };
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_mode_state_inequality() {
        let state1 = ModeState {
            mode: Mode::ReaperVolPan,
            state: State::Active,
        };
        let state2 = ModeState {
            mode: Mode::ReaperSends,
            state: State::Active,
        };
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_mode_state_with_barrier() {
        let barrier = Barrier::new();
        let mode_state = ModeState {
            mode: Mode::ReaperVolPan,
            state: State::WaitingBarrierFromUpstream(barrier),
        };
        assert_eq!(mode_state.mode, Mode::ReaperVolPan);
        assert!(matches!(
            mode_state.state,
            State::WaitingBarrierFromUpstream(_)
        ));
    }

    // NOTE: Testing ModeManager::start and the full message handling loop would require:
    // 1. Setting up channels for all four directions (to/from upstream, to/from downstream)
    // 2. Creating mode handler implementations
    // 3. Thread synchronization and message passing
    // 4. Complex state machine testing
    //
    // These are better suited for integration tests. For unit tests, we've focused on
    // the data structures and enums that can be tested in isolation.
}
