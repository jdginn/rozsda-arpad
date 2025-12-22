use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender, select};

use crate::midi::xtouch::{XTouchDownstreamMsg, XTouchUpstreamMsg};
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
        Barrier::new()
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

    reaper_currently_selected_track_guid: Option<String>,
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
            reaper_currently_selected_track_guid: None,
        };

        // Each mode's implementation struct needs to be initialized here
        let reaper_pan_vol = Arc::new(Mutex::new(VolumePanMode::new(
            8, // For now, assume we have 8 faders on the conroller
            from_reaper.clone(),
            to_reaper.clone(),
            from_xtouch.clone(),
            to_xtouch.clone(),
        )));

        let reaper_track_sends = Arc::new(Mutex::new(TrackSendsMode::new(
            8,
            from_reaper.clone(),
            to_reaper.clone(),
            from_xtouch.clone(),
            to_xtouch.clone(),
        )));

        let reaper_pan_vol_clone = reaper_pan_vol.clone();
        let reaper_track_sends_clone = reaper_track_sends.clone();

        thread::spawn(move || {
            let mut handle_transitions = |manager: &mut ModeManager, mode: ModeState| {
                if mode.state == State::RequestingModeTransition {
                    match mode.mode {
                        Mode::ReaperVolPan => {
                            manager.curr_mode = reaper_pan_vol_clone
                                .lock()
                                .unwrap()
                                .initiate_mode_transition(manager.to_reaper.clone());
                        }
                        Mode::ReaperSends => {
                            if let Some(currently_selected_track_guid) =
                                manager.reaper_currently_selected_track_guid.clone()
                            {
                                manager.curr_mode = reaper_track_sends_clone
                                    .lock()
                                    .unwrap()
                                    .initiate_mode_transition(
                                        manager.to_reaper.clone(),
                                        &currently_selected_track_guid,
                                    );
                            } else {
                                //TODO: log that we won't enter the mode because no track is selected
                                // If we can't transition, stay in current mode
                                manager.curr_mode = mode;
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
                        // Track currently selected track for mode transitions
                        if let TrackMsg::TrackDataMsg(ref data_msg) = track_msg {
                            if let crate::track::track::DataPayload::Selected(true) = data_msg.data {
                                manager.reaper_currently_selected_track_guid = Some(data_msg.guid.clone());
                            }
                        }
                        
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
                            handle_transitions(&mut manager, reaper_pan_vol.lock().unwrap().handle_downstream_messages(track_msg, curr_mode))
                        },
                            Mode::ReaperSends => {
                                handle_transitions(&mut manager, reaper_track_sends.lock().unwrap().handle_downstream_messages(track_msg, curr_mode))
                            },
                        _ => {panic!("Inside unknown mode in ModeManager")},
                        }
                    }
                }
                    recv(manager.from_xtouch) -> msg => {
                        if let Ok(xtouch_msg) = msg {
                            let curr_mode = manager.curr_mode;
                            match curr_mode.mode{
                                Mode::ReaperVolPan => {
                                    match curr_mode.state {
                                        State::Active => {
                                            let new_mode = reaper_pan_vol.lock().unwrap().handle_upstream_messages(xtouch_msg, curr_mode);
                                            handle_transitions(&mut manager, new_mode);
                                        },
                                        // We don't send any messages up from the hw until the hw
                                        // is confirmed to reflect the upsream state
                                        State::WaitingBarrierFromDownstream(_) => {
                                            // Block
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
                                            let new_mode = reaper_track_sends.lock().unwrap().handle_upstream_messages(xtouch_msg, curr_mode);
                                            handle_transitions(&mut manager, new_mode);
                                        },
                                        // We don't send any messages up from the hw until the hw
                                        // is confirmed to reflect the upsream state
                                        State::WaitingBarrierFromDownstream(_) => {
                                            // Block
                                        },
                                        State::WaitingBarrierFromUpstream(_) => {
                                            // Block
                                        },
                                        State::RequestingModeTransition => panic!("We should never be handling upstream messages while requesting a mode transition!")
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
