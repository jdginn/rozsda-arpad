use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};
use uuid::Uuid;

use crate::midi::xtouch;
use crate::midi::xtouch::{
    ArmLEDMsg, FaderAbsMsg, LEDState, MuteLEDMsg, SoloLEDMsg, XTouchDownstreamMsg,
    XTouchUpstreamMsg,
};
use crate::modes::mode_manager::{Mode, ModeHandler, ModeState, State};
use crate::track::track;
use crate::track::track::{DataMsg as TrackDataMsg, TrackMsg};

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

struct FXParamIdent {
    fx_index: i32,
    param_index: i32,
}

/// Maps named, high-level channel strip concepts to their respective parameters
///
/// NOTE: we have one of these *PER TRACK*
///
/// TODO: the hard part will be getting this to update dynamically based on the actual FX chain on the track
struct ChannelStripMap {
    mux: Mutex<()>,
    plugin_names_by_index: Vec<String>,
    hp_filter: Option<FXParamIdent>,
    hp_slope: Option<FXParamIdent>,
    low_freq: Option<FXParamIdent>,
    low_q: Option<FXParamIdent>,
    low_slope: Option<FXParamIdent>,
    /// Chooses between bell and shelf for low band
    low_bell_shelf: Option<FXParamIdent>,
    low_gain: Option<FXParamIdent>,
    lm_freq: Option<FXParamIdent>,
    lm_q: Option<FXParamIdent>,
    lm_gain: Option<FXParamIdent>,
    hm_freq: Option<FXParamIdent>,
    hm_q: Option<FXParamIdent>,
    hm_gain: Option<FXParamIdent>,
    high_freq: Option<FXParamIdent>,
    high_q: Option<FXParamIdent>,
    high_slope: Option<FXParamIdent>,
    /// Chooses between bell and shelf for high band
    high_bell_shelf: Option<FXParamIdent>,
    high_gain: Option<FXParamIdent>,
    /// Gain for the "sides" channel in a mid-side EQ (if applicable)
    high_sides_gain: Option<FXParamIdent>,
    /// Toggles between various EQ plugins
    eq_type: Option<FXParamIdent>,
    eq_bypass: Option<FXParamIdent>,
    /// EQ before or after comprssion
    // eq_position: Option<FXParamIdent>,
    comp1_thresh: Option<FXParamIdent>,
    comp1_sc_filter: Option<FXParamIdent>,
    comp1_ratio: Option<FXParamIdent>,
    comp1_attack: Option<FXParamIdent>,
    comp1_release: Option<FXParamIdent>,
    comp1_makeup: Option<FXParamIdent>,
    /// Toggles between various compressor plugins
    comp1_type: Option<FXParamIdent>,
    comp1_bypass: Option<FXParamIdent>,
    comp2_thresh: Option<FXParamIdent>,
    comp2_sc_filter: Option<FXParamIdent>,
    comp2_ratio: Option<FXParamIdent>,
    comp2_attack: Option<FXParamIdent>,
    comp2_release: Option<FXParamIdent>,
    comp2_makeup: Option<FXParamIdent>,
    /// Toggles between various compressor plugins
    comp2_type: Option<FXParamIdent>,
    comp2_bypass: Option<FXParamIdent>,
    /// Comp 1 -> Comp 2 or Comp 2 -> Comp 1
    // comp_position: Option<FXParamIdent>,
    saturation: Option<FXParamIdent>,
    saturation_bypass: Option<FXParamIdent>,
    /// Toggles between various saturation plugins
    saturation_type: Option<FXParamIdent>,
    gain: Option<FXParamIdent>,
    /// Toggles between various gain plugins (e.g. preamp models)
    gain_type: Option<FXParamIdent>,
    /// Only active if the track is armed
    interface_gain: Option<FXParamIdent>,
}

impl ChannelStripMap {
    fn new() -> Self {
        ChannelStripMap {
            mux: Mutex::new(()),
            plugin_names_by_index: Vec::new(),
            hp_filter: None,
            hp_slope: None,
            low_freq: None,
            low_q: None,
            low_slope: None,
            low_bell_shelf: None,
            low_gain: None,
            lm_freq: None,
            lm_q: None,
            lm_gain: None,
            hm_freq: None,
            hm_q: None,
            hm_gain: None,
            high_freq: None,
            high_q: None,
            high_slope: None,
            high_bell_shelf: None,
            high_gain: None,
            high_sides_gain: None,
            eq_type: None,
            eq_bypass: None,
            comp1_thresh: None,
            comp1_sc_filter: None,
            comp1_ratio: None,
            comp1_attack: None,
            comp1_release: None,
            comp1_makeup: None,
            comp1_type: None,
            comp1_bypass: None,
            comp2_thresh: None,
            comp2_sc_filter: None,
            comp2_ratio: None,
            comp2_attack: None,
            comp2_release: None,
            comp2_makeup: None,
            comp2_type: None,
            comp2_bypass: None,
            saturation: None,
            saturation_bypass: None,
            saturation_type: None,
            gain: None,
            gain_type: None,
            interface_gain: None,
        }
    }

    fn update_plugin_state(&mut self, plugin_index: i32, plugin_name: &str) {
        let _lock = self.mux.lock().unwrap();
        if (plugin_index as usize) >= self.plugin_names_by_index.len() {
            self.plugin_names_by_index
                .resize((plugin_index + 1) as usize, String::new());
        }
        self.plugin_names_by_index[plugin_index as usize] = plugin_name.to_string();
    }

    fn update_mapping_locked(&mut self) {}
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
/// 4. Turn the encoder WHILE holding down both the modifier AND pressing down the encoder
/// 5. Click the encoder (turning does nothing)
/// 6. Click the encoder WHILE holding down a modifier button (e.g., Shift -- turning does nothing)
///
/// In each case, the behavior updates the scribble strip to indicate what parameter is being controlled and the encoder ring to indicate the current value.
///
/// This mode assumes 16 encoders are available. The encoders have the following functions:
/// | #  | Normal      | Pressed                          | Shift            | Shift+Pressed  | Click          | Shift+Click     |
/// |----|-------------|----------------------------------|------------------|----------------|--------------- |-----------------|
/// | 1  | HP filter   | slope                            | EQ type          |                |                |                 |
/// | 2  | Low freq    | Low Q (bell) / slope (shelf)     | bell/shelf       |                |                |                 |
/// | 3  | Low gain    |                                  |                  |                | zero Low gain  |                 |
/// | 4  | LM freq     | LM Q                             |                  |                |                |                 |
/// | 5  | LM gain     |                                  |                  |                | zero LM gain   |                 |
/// | 6  | HM freq     | HM Q                             |                  |                |                |                 |
/// | 7  | HM gain     |                                  |                  |                | zero HM gain   |                 |
/// | 8  | High freq   | High Q (bell) / slope (slope)    | bell/shelf       |                |                |                 |
/// | 9  | High gain   |                                  | sides gain       |                | zero High gain | zero sides gain |
/// | 10 | EQ pos      |                                  | Comp order       |                | bypass EQ      |                 |
/// | 11 | Comp thresh | Comp SC filter                   | Comp2  thresh    | Comp2 SC filt  |                |                 |
/// | 12 | Comp ratio  | Comp attack                      | Comp2  ratio     | Comp2 attack   |                |                 |
/// | 13 | Comp makeup | Comp release                     | Comp2  makeup    | Comp2 release  |                |                 |
/// | 14 | Comp type   |                                  | Comp2  type      |                | bypass Comp    | bypass Comp2    |
/// | 15 | Saturation  |                                  | Saturation type  |                | bypass Sat     |                 |
/// | 16 | Gain        | Interface gain (only if armed    | Trim             |                |                |                 |
///
/// Notes on specific controls:
/// - By default, EQ is engaged, both compressors and saturation are bypassed.
/// - EQ type selects between EQ plugins with EQUIVALENT features. It may allow e.g. colourless EQ, SSL-style, Neve-style, etc.
/// - Depending on EQ type, Q may or may not take effect.
/// - Sides gain applies the "High" band only to the sides in a mid-side EQ. This value is offset from the main high gain.
/// - EQ pos sets the position of EQ in the signal chain. Modes:
///    - "FIRST": Gain -> EQ -> Comp -> Comp -> Saturation -> Trim
///    - "MIDDLE": Gain -> Comp -> Comp -> EQ -> Saturation -> Trim
///    - "LAST": Gain -> Comp -> Comp -> Saturation -> EQ -> Trim
/// - Comp order sets the ordering of compressors. Modes:
///    - "F->S": Comp -> Comp2
///    - "S->F": Comp2 -> Comp1
/// - Comp and Comp2 are separate compressors and controlled fully independently.
/// - Comp is a "fast", FET-style compressor. Comp2 is a "slow" optical-style compressor.
/// - Comp type selects between comprssors germain to the two categories above. Examples:
///     - Comp1: 1176, Distressor, Digital, SSL, API
///     - Comp2: LA2A, LA3A, Vari-MU, etc.
/// - For compressor types that do not have a threshold control, Comp thresh maps to input gain.
/// - Some compressor tyeps do not have a ratio control.
/// - For compressor types that do not have a ratio control, Comp makeup maps to output gain.
/// - Some compressor types lack attack and release controls.
/// - Comp SC filter is a high-pass filter on the compressor sidechain.
/// - Saturation type selects between various console, tape simulators up to full-on distortion.
/// - Gain adjusts level entering the channel strip, before any processing.
/// - Trim adjust level leaving the channel strip.
/// - Interface gain adjusts the gain at the audio interface, if the selected tack is armed. This does not affect recorded material.
pub struct ChannelStripMode {
    // Maps each channel on the hardware controller to a Reaper track
    track_hw_assignments: Arc<Mutex<Vec<Option<Uuid>>>>,
    track_states: HashMap<Uuid, MuteSoloArmButtonState>,
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

    fn get_track_state(&mut self, guid: Uuid) -> &mut MuteSoloArmButtonState {
        self.track_states
            .entry(guid)
            .or_insert(MuteSoloArmButtonState {
                mute: Button::new(),
                solo: Button::new(),
                arm: Button::new(),
            })
    }

    fn get_guid_for_hw_channel(&self, hw_channel: usize) -> Option<Uuid> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments[hw_channel + 1]
    }

    // For a given track GUID, find which hardware channel it's assigned to (if any)
    pub fn find_hw_channel(&self, guid: Uuid) -> Option<usize> {
        let assignments = self.track_hw_assignments.lock().unwrap();
        assignments
            .iter()
            .enumerate()
            .find(|(_, assigned_guid)| *assigned_guid == &Some(guid))
            .map(|(hw_channel, _)| hw_channel - 1)
    }
}

impl ModeHandler<TrackMsg, TrackMsg, XTouchDownstreamMsg, XTouchUpstreamMsg> for ChannelStripMode {
    fn handle_downstream_messages(&mut self, msg: TrackMsg, curr_mode: ModeState) -> ModeState {
        match TrackDataMsg::try_from(msg) {
            Err(TrackMsg::Barrier(barrier)) => {
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
            Ok(msg) => {
                match msg {
                    // We use track index according to reaper to assign tracks to hardware channels
                    TrackDataMsg::ReaperTrackIndex(msg) => {
                        if let Some(index) = msg.track_index {
                            self.track_hw_assignments.lock().unwrap()[index as usize] =
                                Some(msg.track_guid);
                            return curr_mode;
                        }
                    }
                    TrackDataMsg::Volume(msg) => {
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            // Send volume update to XTouch for the corresponding fader
                            let fader_value = msg.volume; // TODO: scale appropriately
                            let _ =
                                self.to_xtouch
                                    .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                                        idx: hw_channel as i32,
                                        value: fader_value as f64,
                                    }));
                        }
                        return curr_mode;
                    }
                    TrackDataMsg::Muted(msg) => {
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            self.get_track_state(msg.track_guid).mute.set(msg.muted);
                            // Send mute LED update to XTouch
                            let _ = self
                                .to_xtouch
                                .send(XTouchDownstreamMsg::MuteLED(MuteLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(msg.muted),
                                }));
                        }
                        return curr_mode;
                    }
                    TrackDataMsg::Soloed(msg) => {
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            self.get_track_state(msg.track_guid).solo.set(msg.soloed);
                            // Send solo LED update to XTouch
                            let _ = self
                                .to_xtouch
                                .send(XTouchDownstreamMsg::SoloLED(SoloLEDMsg {
                                    idx: hw_channel as i32,
                                    state: LEDState::from(msg.soloed),
                                }));
                        }
                        return curr_mode;
                    }
                    TrackDataMsg::Armed(msg) => {
                        if let Some(hw_channel) = self.find_hw_channel(msg.track_guid) {
                            self.get_track_state(msg.track_guid).arm.set(msg.armed);
                            // Send arm LED update to XTouch
                            let _ = self.to_xtouch.send(XTouchDownstreamMsg::ArmLED(ArmLEDMsg {
                                idx: hw_channel as i32,
                                state: LEDState::from(msg.armed),
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
            Err(_) => {
                // Ignore messages that fail to parse as TrackDataMsg (e.g., ModeTransition, etc.)
                return curr_mode;
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
                    let _ = self.to_reaper.send(
                        track::Volume {
                            track_guid: *guid,
                            volume: fader_msg.value as f32, // TODO: Need to scale appropriately
                        }
                        .into(),
                    );
                }
                curr_mode
            }
            XTouchUpstreamMsg::MutePress(mute_msg) => {
                if let Some(guid) = self.get_guid_for_hw_channel(mute_msg.idx as usize) {
                    let new_state = self.get_track_state(guid).mute.toggle();
                    // Send mute toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(
                            track::Muted {
                                track_guid: guid,
                                muted: new_state,
                            }
                            .into(),
                        )
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
                    let new_state = self.get_track_state(guid).solo.toggle();
                    // Send solo toggle to Reaper for the corresponding track
                    self.to_reaper
                        .send(
                            track::Soloed {
                                track_guid: guid,
                                soloed: new_state,
                            }
                            .into(),
                        )
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
                        .send(
                            track::Armed {
                                track_guid: guid.clone(),
                                armed: new_state,
                            }
                            .into(),
                        )
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
