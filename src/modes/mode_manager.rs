use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

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
    // One of the modes has requested the mode manager to transition to a new mode
    RequestingModeTransition,
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

/// Represents the current mode and state of the mode manager.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModeState {
    pub mode: Mode,
    pub state: State,
    pub new_selected_track_guid: Option<Uuid>,
}

/// Each mode implementation struct needs to implement this trait to handle messages
///
/// Each mode implementation should also implement initiate_mode_transition(self, ...) -> ModeState. This implementation
/// will vary from mode to mode but usually will require sending a barrier to the upstream channel.
pub trait ModeHandler<ToUpstream, FromUpstream, ToDownstream, FromDownstream> {
    fn handle_messages_from_downstream(
        &mut self,
        msg: FromDownstream,
        curr_mode: ModeState,
    ) -> ModeState;
    fn handle_messages_from_upstream(
        &mut self,
        msg: FromUpstream,
        curr_mode: ModeState,
    ) -> ModeState;
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
    from_v1m: Receiver<v1m::UpstreamMsg>,
    _to_v1m: Sender<v1m::DownstreamMsg>,
    pub curr_mode: ModeState,

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
        let mut manager = ModeManager {
            from_reaper: from_reaper.clone(),
            to_reaper: to_reaper.clone(),
            from_v1m: from_v1m.clone(),
            _to_v1m: to_v1m.clone(),
            curr_mode: ModeState {
                mode: Mode::ReaperVolPan,
                state: State::Active,
                new_selected_track_guid: None,
            },
            reaper_currently_selected_track_guid: None,
        };

        // Each mode's implementation struct needs to be initialized here
        let reaper_vol_pan = Arc::new(Mutex::new(VolumePanMode::new(
            8, // For now, assume we have 8 faders on the conroller
            from_reaper.clone(),
            to_reaper.clone(),
            from_v1m.clone(),
            to_v1m.clone(),
        )));

        let reaper_track_sends = Arc::new(Mutex::new(TrackSendsMode::new(
            8,
            from_reaper.clone(),
            to_reaper.clone(),
            from_v1m.clone(),
            to_v1m.clone(),
        )));

        let reaper_channel_strip = Arc::new(Mutex::new(ChannelStripMode::new(
            8,
            from_reaper.clone(),
            to_reaper.clone(),
            from_v1m.clone(),
            to_v1m.clone(),
        )));

        let reaper_vol_pan_clone = reaper_vol_pan.clone();
        let reaper_track_sends_clone = reaper_track_sends.clone();
        let reaper_channel_strip_clone = reaper_channel_strip.clone();

        thread::spawn(move || {
            let handle_transitions = |manager: &mut ModeManager, mode: ModeState| {
                // If the mode has indicated a new track selection, update it with the mode manager
                if let Some(selected_track_guid) = mode.new_selected_track_guid {
                    manager.reaper_currently_selected_track_guid = Some(selected_track_guid)
                }
                if mode.state == State::RequestingModeTransition {
                    match mode.mode {
                        Mode::ReaperVolPan => {
                            println!("Transitioning to ReaperVolPan mode");
                            if let Err(e) = reaper_vol_pan_clone.try_lock() {
                                println!("Failed to lock reaper_vol_pane_clone: {:?}", e);
                            }
                            manager.curr_mode = reaper_vol_pan_clone
                                .lock()
                                .unwrap()
                                .initiate_mode_transition(
                                    manager.curr_mode,
                                    manager.to_reaper.clone(),
                                    manager.reaper_currently_selected_track_guid,
                                );
                            println!("New mode state: {:?}", manager.curr_mode);
                        }
                        Mode::ReaperSends => {
                            println!("Transitioning to ReaperSends mode");
                            // We can only enter this mode if we have a track selected
                            if let Some(selected_track_guid) =
                                manager.reaper_currently_selected_track_guid
                            {
                                manager.curr_mode = reaper_track_sends_clone
                                    .lock()
                                    .unwrap()
                                    .initiate_mode_transition(
                                        manager.curr_mode.mode,
                                        manager.to_reaper.clone(),
                                        selected_track_guid,
                                    );
                            }
                        }
                        Mode::ReaperChannelStrip => {
                            println!("Transitioning to ReaperChannelStrip mode");
                            // We can only enter this mode if we have a track selected
                            if let Some(selected_track_guid) =
                                manager.reaper_currently_selected_track_guid
                            {
                                manager.curr_mode = reaper_channel_strip_clone
                                    .lock()
                                    .unwrap()
                                    .initiate_mode_transition(
                                        manager.curr_mode.mode,
                                        manager.to_reaper.clone(),
                                        selected_track_guid,
                                    );
                            }
                        }
                        Mode::MotuVolPan => {
                            panic!("MotuVolPan mode transition not implemented yet!")
                        }
                    }
                } else {
                    // Not requesting a transition, just update the mode
                    manager.curr_mode = mode;
                }
            };

            loop {
                select! {
                    recv(manager.from_reaper) -> msg => {
                        if let Ok(track_msg) = msg {
                        let curr_mode = manager.curr_mode;
                        match curr_mode.mode {
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
                            let new_mode = reaper_vol_pan.lock().unwrap().handle_messages_from_upstream(track_msg, curr_mode);
                            if new_mode != curr_mode {
                                handle_transitions(&mut manager, new_mode)
                            }
                        },
                        Mode::ReaperSends => {
                            let new_mode = reaper_track_sends.lock().unwrap().handle_messages_from_upstream(track_msg, curr_mode);
                            if new_mode != curr_mode {
                                handle_transitions(&mut manager, new_mode)
                            }
                        },
                        Mode::ReaperChannelStrip => {
                            let new_mode = reaper_channel_strip.lock().unwrap().handle_messages_from_upstream(track_msg, curr_mode);
                            if new_mode != curr_mode {
                                handle_transitions(&mut manager, new_mode)
                            }
                        },
                        Mode::MotuVolPan => {
                            panic!("MotuVolPan currently unsupported")
                        }
                        }
                    }
                }
                    recv(manager.from_v1m) -> msg => {
                        if let Ok(v1m_msg) = msg {
                            let curr_mode = manager.curr_mode;
                            match curr_mode.mode{
                                Mode::ReaperVolPan => {
                                    match curr_mode.state {
                                        State::Active => {
                                            let new_mode = reaper_vol_pan.lock().unwrap().handle_messages_from_downstream(v1m_msg, curr_mode);
                                            if new_mode != curr_mode {
                                                handle_transitions(&mut manager, new_mode);
                                            }
                                        },
                                        // We don't send any messages up from the hw until the hw
                                        // is confirmed to reflect the upsream state
                                        State::WaitingBarrierFromDownstream(expected_barrier) => {
                                            match v1m_msg {
                                                v1m::UpstreamMsg::Barrier(barrier) => {
                                                    if barrier == expected_barrier {
                                                        manager.curr_mode = ModeState {
                                                            mode: curr_mode.mode,
                                                            state: State::Active,
                                                            new_selected_track_guid: None,
                                                        };
                                                    } else {
                                                        // This is a barrier for a previous mode transition that we have already passed, so we can ignore it
                                                        // (We should only be receiving barriers for the current mode transition we are in, but just in case...)
                                                    }
                                                },
                                                _ => {
                                                    // Block all non-barrier messages until the barrier comes through
                                                }
                                            }
                                        },
                                        State::WaitingBarrierFromUpstream(_) => {
                                            // Block
                                        },
                                        State::RequestingModeTransition => panic!("We should never be handling upstream messages while requesting a mode transition!")
                                    }
                                },
                                Mode::ReaperSends => {
                                    match curr_mode.state {
                                        State::Active => {
                                            let new_mode = reaper_track_sends.lock().unwrap().handle_messages_from_downstream(v1m_msg, curr_mode);
                                            if new_mode != curr_mode {
                                                handle_transitions(&mut manager, new_mode);
                                            }
                                        },
                                        // We don't send any messages up from the hw until the hw
                                        // is confirmed to reflect the upsream state
                                        State::WaitingBarrierFromDownstream(expected_barrier) => {
                                            match v1m_msg {
                                                v1m::UpstreamMsg::Barrier(barrier) => {
                                                    if barrier == expected_barrier {
                                                        manager.curr_mode = ModeState {
                                                            mode: curr_mode.mode,
                                                            state: State::Active,
                                                            new_selected_track_guid: None,
                                                        };
                                                    } else {
                                                        // This is a barrier for a previous mode transition that we have already passed, so we can ignore it
                                                        // (We should only be receiving barriers for the current mode transition we are in, but just in case...)
                                                    }
                                                },
                                                _ => {
                                                    // Block all non-barrier messages until the barrier comes through
                                                }
                                            }
                                        },
                                        State::WaitingBarrierFromUpstream(_) => {
                                            // Block
                                        },
                                        State::RequestingModeTransition => panic!("We should never be handling upstream messages while requesting a mode transition!")
                                    }
                                },
                                Mode::ReaperChannelStrip => {
                                    match curr_mode.state {
                                        State::Active => {
                                            let new_mode = reaper_channel_strip.lock().unwrap().handle_messages_from_downstream(v1m_msg, curr_mode);
                                            if new_mode != curr_mode {
                                                handle_transitions(&mut manager, new_mode);
                                            }
                                        },
                                        // We don't send any messages up from the hw until the hw
                                        // is confirmed to reflect the upsream state
                                        State::WaitingBarrierFromDownstream(expected_barrier) => {
                                            match v1m_msg {
                                                v1m::UpstreamMsg::Barrier(barrier) => {
                                                    if barrier == expected_barrier {
                                                        manager.curr_mode = ModeState {
                                                            mode: curr_mode.mode,
                                                            state: State::Active,
                                                            new_selected_track_guid: None,
                                                        };
                                                    } else {
                                                        // This is a barrier for a previous mode transition that we have already passed, so we can ignore it
                                                        // (We should only be receiving barriers for the current mode transition we are in, but just in case...)
                                                    }
                                                },
                                                _ => {
                                                    // Block all non-barrier messages until the barrier comes through
                                                }
                                            }
                                        },
                                        State::WaitingBarrierFromUpstream(_) => {
                                            // Block
                                        },
                                        State::RequestingModeTransition => panic!("We should never be handling upstream messages while requesting a mode transition!")
                                    }
                                },
                                Mode::MotuVolPan => {
                                    panic!("MotuVolPan currently unsupported")
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
