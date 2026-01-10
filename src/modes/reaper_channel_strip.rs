use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};

use crate::midi::xtouch;
use crate::midi::xtouch::{FaderAbsMsg, LEDState, XTouchDownstreamMsg, XTouchUpstreamMsg};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::track::track::{
    DataPayload as TrackDataPayload, Direction, TrackDataMsg, TrackMsg, TrackQuery,
};

struct Button {
    state: bool,
}

impl Button {
    fn new() -> Self {
        Button { state: false }
    }

    fn is_on(&self) -> bool {
        self.state
    }

    fn set(&mut self, new_state: bool) {
        self.state = new_state;
    }

    fn toggle(&mut self) -> bool {
        self.state = !self.state;
        self.state
    }
}

// Collection of state for the buttons repeated for each channel on the hw controller
//
// TODO: this might be too implementation-specific to live here?
struct MuteSoloArmButtonState {
    mute: Button,
    solo: Button,
    arm: Button,
}

/// Implements a mode where the faders and Arm/Mute/Solo/Select buttons behave the same as VolumePanMode
/// but the encoders and scribble strpes expose key tone-shaping functions like EQ, Compression, Saturation, etc.
///
/// The encoders ONLY control the selected track, and only control one track at a time. Track
/// selection still follows reaper and still responds to the select buttons on the surface.
///
/// Encoders support multiple behaviors, with each encoder supporting up to the following:
/// 1. Turn the encoder without pressing anything
/// 2. Turn the encoder WHILE holding it down
/// 3. Turn the encoder WHILE holding down a modifier button (e.g., Shift)
/// 4. Turn the encoder WHILE holding down both the modifier and pressing down the encoder
/// 5. Click the encoder (turning does nothing)
///
/// In each case, the behavior updates the scribble strip to indicate what parameter is being controlled and the encoder ring to indicate the current value.
///
/// This mode assumes 16 encoders are available. The encoders have the following functions:
/// | Encoder Number | Normal                           | Pressed          | Shift          | Shift+Pressed      | Click         |
/// |----------------|----------------------------------|------------------|----------------|--------------------|---------------|
/// | 1. HP filter   | slope                            |                  |                |                    |               |
/// | 2. Low freq    | Low Q (bell)/slope (slope)       | bell/shelf       |                |                    |               |
/// | 3. Low gain    |                                  |                  |                |                    | zero          |
/// | 4. LM freq     | LM Q                             |                  |                |                    |               |
/// | 5. LM gain     |                                  |                  |                |                    | zero          |
/// | 6. HM freq     | HM Q                             |                  |                |                    |               |
/// | 7. HM gain     |                                  |                  |                |                    | zero          |
/// | 8. High freq   | High Q (bell)/slope (slope)/LPF  | bell/shelf       |                |                    |               |
/// | 9. High gain   |                                  | sides gain       |                |                    | zero          |
/// | 10. EQ type    | bypass                           | EQ pos           |                |                    |               |
/// | 11. Comp thresh| SC filter                        | comp 2 thresh    | comp 2 SC filt |                    |               |
/// | 12. Comp ratio | attack                           | comp 2 ratio     | comp 2 attack  |                    |               |
/// | 13. Comp makeup| release                          | comp 2 makeup    | comp 2 release |                    |               |
/// | 14. Comp type  | bypass                           | comp 2 type      | comp 2 bypass  |                    |               |
/// | 15. Saturation | bypass                           | comp pos         |                |                    |               |
/// | 16. Gain       | interface gain (only if armed)   | saturation type  | gain type      |                    |               |
pub struct ChannelStripMode {
    // Maps each channel on the hardware controller to a Reaper track
    track_hw_assignments: Arc<Mutex<Vec<Option<String>>>>,
    track_states: HashMap<String, MuteSoloArmButtonState>,
    to_reaper: Sender<TrackMsg>,
    from_reaper: Receiver<TrackMsg>,
    to_xtouch: Sender<XTouchDownstreamMsg>,
    from_xtouch: Receiver<XTouchUpstreamMsg>,
}

impl ChannelStripMode {
    pub fn new(
        num_channels: usize,
        from_reaper: Receiver<TrackMsg>,
        to_reaper: Sender<TrackMsg>,
        from_xtouch: Receiver<XTouchUpstreamMsg>,
        to_xtouch: Sender<XTouchDownstreamMsg>,
    ) -> Self {
        let track_hw_assignments = Arc::new(Mutex::new(vec![None; num_channels]));
        let button_states = HashMap::new();

        ChannelStripMode {
            track_hw_assignments,
            track_states: button_states,
            to_reaper,
            from_reaper,
            to_xtouch,
            from_xtouch,
        }
    }

    fn get_track_state(&mut self, guid: String) -> &mut MuteSoloArmButtonState {
        self.track_states
            .entry(guid)
            .or_insert(MuteSoloArmButtonState {
                mute: Button::new(),
                solo: Button::new(),
                arm: Button::new(),
            })
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<String> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments[hw_channel].clone()
    }

    // For a given track GUID, find which hardware channel it's assigned to (if any)
    pub fn find_hw_channel(&self, guid: &str) -> Option<usize> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments
            .iter()
            .enumerate()
            .find(|(_, assigned_guid)| *assigned_guid == &Some(guid.to_string()))
            .map(|(hw_channel, _)| hw_channel)
    }
}

impl ModeHandler<TrackMsg, TrackMsg, XTouchDownstreamMsg, XTouchUpstreamMsg> for ChannelStripMode {
    fn handle_downstream_messages(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        if let TrackMsg::Barrier(barrier) = msg {
            // Forward barriers downstream (they need to reflect back upstream for the mode to
            // transition)
            self.to_xtouch
                .send(XTouchDownstreamMsg::Barrier(barrier))
                .unwrap();
            match curr_mode.state {
                // If we were already waiting on a barrier from upstream, check if this is the one
                // we were waiting for. If yes, transition to waiting for the barrier to reflect back up from downstream.
                State::WaitingBarrierFromUpstream(expected_barrier) => {
                    if barrier == expected_barrier {
                        return ModeState {
                            mode: curr_mode.mode,
                            state: State::WaitingBarrierFromDownstream(barrier),
                        };
                    } else {
                        return curr_mode;
                    }
                }
                _ => return curr_mode,
            }
        }
        if let TrackMsg::TrackDataMsg(msg) = msg {
            match msg.data {
                // We use track index according to reaper to assign tracks to hardware channels
                TrackDataPayload::ReaperTrackIndex(Some(index)) => {
                    self.track_hw_assignments.lock().unwrap()[index as usize] =
                        Some(msg.guid.clone());
                    return curr_mode;
                }
                TrackDataPayload::Volume(value) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        // Send volume update to XTouch for the corresponding fader
                        let fader_value = value; // TODO: scale appropriately
                        let _ = self
                            .to_xtouch
                            .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                idx: hw_channel as i32,
                                value: fader_value as f64,
                            }));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Muted(muted) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        self.get_track_state(msg.guid).mute.set(muted);
                        // Send mute LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(muted),
                                }));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Soloed(soloed) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        self.get_track_state(msg.guid).solo.set(soloed);
                        // Send solo LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(soloed),
                                }));
                    }
                    return curr_mode;
                }
                TrackDataPayload::Armed(armed) => {
                    if let Some(hw_channel) = self.find_hw_channel(&msg.guid) {
                        self.get_track_state(msg.guid).arm.set(armed);
                        // Send arm LED update to XTouch
                        let _ =
                            self.to_xtouch
                                .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(armed),
                                }));
                    }
                    return curr_mode;
                }
                _ => {
                    // Ignore unhandled payloads (e.g., Selected, SendIndex, etc.)
                    return curr_mode;
                }
            }
        }
        curr_mode
    }
    fn handle_upstream_messages(
        &mut self,
        msg: XTouchUpstreamMsg,
        curr_mode: ModeState,
    ) -> ModeState {
        match msg {
            // If we were already waiting on a barrier from downstream, check if this is the one
            // we were waiting for. If yes, the state transition is finished.
            //
            // Note, we do not need to forward this barrier onward, since the hardware is not
            // allowed to reflect barriers back upstream.
            XTouchUpstreamMsg::Barrier(barrier) => {
                match curr_mode.state {
                    State::WaitingBarrierFromDownstream(expected_barrier) => {
                        if barrier == expected_barrier {
                            ModeState {
                                mode: curr_mode.mode,
                                state: State::Active,
                            }
                        } else {
                            curr_mode
                        }
                    }
                    _ => {
                        // TODO: This is a barrier message we don't care about. Do we need to do
                        // anything with it?
                        //
                        // Presumably if a barrier comes back that we weren't looking for, it's for
                        // some old irrelevant state transition that has already been superseded.
                        curr_mode
                    }
                }
                // Handle barrier messages if needed
            }
            XTouchUpstreamMsg::GlobalPress => curr_mode, // GlobalPress maps to this mode!
            // MIDITracksPress maps to ReaperSends mode
            XTouchUpstreamMsg::MIDITracksPress => {
                // Request transition to ReaperSends mode
                ModeState {
                    mode: Mode::ReaperSends,
                    state: State::RequestingModeTransition,
                }
            }
            XTouchUpstreamMsg::FaderAbs(fader_msg) => {
                if let Some(guid) =
                    &self.track_hw_assignments.lock().unwrap()[fader_msg.idx as usize]
                {
                    // Send volume update to Reaper for the corresponding track
                    let _ = self.to_reaper.send(TrackMsg::TrackDataMsg(TrackDataMsg {
                        direction: Direction::Upstream,
                        guid: guid.clone(),
                        data: TrackDataPayload::Volume(fader_msg.value as f32), // TODO: Need to scale appropriately
                    }));
                }
                curr_mode
            }
            XTouchUpstreamMsg::MutePress(mute_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(mute_msg.idx as usize) {
                    let new_state = self.get_track_state(guid.clone()).mute.toggle();
                    // Send mute toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Muted(new_state),
                        }))
                        .unwrap();
                    // Update the toggle on the hardware
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::MuteLED(xtouch::MuteLEDMsg {
                            idx: mute_msg.idx,
                            state: LEDState::from(new_state),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            XTouchUpstreamMsg::SoloPress(solo_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(solo_msg.idx as usize) {
                    let new_state = self.get_track_state(guid.clone()).solo.toggle();
                    // Send solo toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Soloed(new_state),
                        }))
                        .unwrap();
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::SoloLED(xtouch::SoloLEDMsg {
                            idx: solo_msg.idx,
                            state: LEDState::from(new_state),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            XTouchUpstreamMsg::ArmPress(arm_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(arm_msg.idx as usize) {
                    let new_state = self.get_track_state(guid.clone()).arm.toggle();
                    // Send arm toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(TrackMsg::TrackDataMsg(TrackDataMsg {
                            direction: Direction::Upstream,
                            guid: guid.clone(),
                            data: TrackDataPayload::Armed(new_state),
                        }))
                        .unwrap();
                    self.to_xtouch
                        .send(XTouchDownstreamMsg::ArmLED(xtouch::ArmLEDMsg {
                            idx: arm_msg.idx,
                            state: LEDState::from(new_state),
                        }))
                        .unwrap();
                }
                curr_mode
            }
            _ => curr_mode,
        }
    }
}
