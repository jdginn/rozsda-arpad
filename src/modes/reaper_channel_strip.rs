use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crossbeam_channel::{Receiver, Sender};

use crate::midi::xtouch;
use crate::midi::xtouch::{
    EncoderPressMsg, EncoderReleaseMsg, FaderAbsMsg, LEDState, XTouchDownstreamMsg,
    XTouchUpstreamMsg,
};
use crate::modes::mode_manager::{Barrier, Mode, ModeHandler, ModeState, State};
use crate::track::track::{
    DataPayload as TrackDataPayload, Direction, TrackDataMsg, TrackMsg, TrackQuery,
};

// Scribble strips:
//
// Top displays:
// - Color is set per function (EQ, Comp, etc)
// - Range
// - Top line: function
// - Second line: numeric
// - Third line: click-in function
// - Bottom line: shift function
//
// Lower displays:
// - Top line: track name
// - Bottom line: ?

// Architecture ideas:
//
// In this mode, faders do the same thing as VolPanMode. Faders are the surfaces where being out
// of step with Reaper can cause us problems, so for the unique channel strip stuff here, we have
// less stringent requirements around state synchronization.
//
// The channel-strip specific stuff here cares about the *top displays*. Each of these displays can
// implement its own adapter that encapsulates all channel-strip specific behavior:
//
// INPUT:
// - Encoder position
// - Encode press (used for push-in mode)
// - Encoder release (used to exit push-in mode or register click)
// - Shift (comes globally)
//
// OUTPUT:
// - Color
// - Range
// - Numeric value
// - Click-in function
// - Shift function
//
// In shift mode, display the click-in function on line 3 but not the shift function
//
// OUTPUTs can simply follow our best-known feedback values. We don't need to gate upstream
// messages for synchronization because the values shown on the displays do not affect inputs.
// Inputs are relative anyway.
//
// There should be some abstraction for hooking up messages to ChannelLayers. The naive
// implementation talks to some kind of reaper "compound" plugin. But we also need some kind of
// fallback implementation that does a best-effort mapping based on whatever plugins appear on the
// track. This is trickier.
//
// Dataflow goes:
// Reaper -> Router -> ChannelWidget -> Hardware
//
// Router links up messages to the appropriate ChannelWidget inputs?

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelWidgetMode {
    Default,
    Press,
    Shift,
    ShiftPress,
}

#[derive(Clone, Debug)]
struct ChannelWidgetLabels {
    default: &'static str,
    press: &'static str,
    shift: &'static str,
    shift_press: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct ChannelWidgetColors {
    default: u32, // Or some better datatype
    press: u32,
    shift: u32,
    shift_press: u32,
}

/// ChannelWidgetCore handles shared logic around mode switching and message passing.
struct ChannelWidgetCore {
    mode: ChannelWidgetMode,

    upstream_tx: Sender<TrackMsg>,
    downstream_tx: Sender<XTouchDownstreamMsg>,
}

impl ChannelWidgetCore {
    fn on_mode_change(&mut self) {}

    fn mode_change_button_press(&mut self) {
        match self.mode {
            ChannelWidgetMode::Default => self.mode = ChannelWidgetMode::Press,
            ChannelWidgetMode::Press => {}
            ChannelWidgetMode::Shift => self.mode = ChannelWidgetMode::ShiftPress,
            ChannelWidgetMode::ShiftPress => {}
        }
    }

    fn mode_change_button_release(&mut self) {
        // TODO: sometimes this sends a message upstream, sometimes it just changes mode.
        match self.mode {
            ChannelWidgetMode::Default => {}
            ChannelWidgetMode::Press => self.mode = ChannelWidgetMode::Default,
            ChannelWidgetMode::Shift => {}
            ChannelWidgetMode::ShiftPress => self.mode = ChannelWidgetMode::Shift,
        }
    }

    fn mode_change_shift_press(&mut self) {
        match self.mode {
            ChannelWidgetMode::Default => self.mode = ChannelWidgetMode::Shift,
            ChannelWidgetMode::Press => self.mode = ChannelWidgetMode::ShiftPress,
            ChannelWidgetMode::Shift => {}
            ChannelWidgetMode::ShiftPress => {}
        }
    }

    fn mode_change_shift_release(&mut self) {
        match self.mode {
            ChannelWidgetMode::Default => {}
            ChannelWidgetMode::Press => {}
            ChannelWidgetMode::Shift => self.mode = ChannelWidgetMode::Default,
            ChannelWidgetMode::ShiftPress => self.mode = ChannelWidgetMode::Press,
        }
    }
}

/// ChannelWidetBehavior defines the specific behavior of some specific widget.
trait ChannelWidgetBehavior {
    const LABELS: ChannelWidgetLabels;
    const COLORS: ChannelWidgetColors;
    const INDEX: usize; // Which encoder this widget is associated with (0-15)

    fn on_encoder_inc(&mut self);
    fn on_encoder_dec(&mut self);
    fn on_click(&mut self) -> Option<TrackMsg>;
    fn handle_downstream_message(&mut self, msg: TrackMsg);
}

/// ChannelWidget is the full implementation of some widget.
struct ChannelWidget<B: ChannelWidgetBehavior> {
    core: ChannelWidgetCore,
    behavior: B,
}

impl<B: ChannelWidgetBehavior> ChannelWidget<B> {
    // By default, colors and labels switch between static values based on the mode.
    // In some situations, labels may need to change based on plugin state. In these cases,
    // override the method.
    fn color(&self) -> u32 {
        let colors = B::COLORS;
        match self.core.mode {
            ChannelWidgetMode::Default => colors.default,
            ChannelWidgetMode::Press => colors.press,
            ChannelWidgetMode::Shift => colors.shift,
            ChannelWidgetMode::ShiftPress => colors.shift_press,
        }
    }

    fn label1(&self) -> &'static str {
        let labels = B::LABELS;
        match self.core.mode {
            ChannelWidgetMode::Default => labels.default,
            ChannelWidgetMode::Press => labels.press,
            ChannelWidgetMode::Shift => labels.shift,
            ChannelWidgetMode::ShiftPress => labels.shift_press,
        }
    }

    fn label3(&self) -> &'static str {
        let labels = B::LABELS;
        match self.core.mode {
            ChannelWidgetMode::Default => labels.press,
            ChannelWidgetMode::Press => "",
            ChannelWidgetMode::Shift => labels.shift_press,
            ChannelWidgetMode::ShiftPress => "",
        }
    }

    fn label4(&self) -> &'static str {
        let labels = B::LABELS;
        match self.core.mode {
            ChannelWidgetMode::Default => labels.shift,
            ChannelWidgetMode::Press => labels.shift_press,
            ChannelWidgetMode::Shift => "",
            ChannelWidgetMode::ShiftPress => "",
        }
    }

    fn mode_change_button_press(&mut self) {
        self.core.mode_change_button_press();
    }

    fn mode_change_button_release(&mut self) {
        self.core.mode_change_button_release();
    }

    fn mode_change_shift_press(&mut self) {
        self.core.mode_change_shift_press();
    }

    fn mode_change_shift_release(&mut self) {
        self.core.mode_change_shift_release();
    }

    fn on_encoder_inc(&mut self) {
        self.behavior.on_encoder_inc();
    }

    fn on_encoder_dec(&mut self) {
        self.behavior.on_encoder_dec();
    }

    fn on_click(&mut self) {
        self.behavior.on_click();
    }

    // FIXME: this probably should live elsewhere and delgate to all the configured widgets.
    //
    // It would be awkward to have this on each and every widget. Would we pass messages
    // sequentially? Would we multiplex them?
    fn handle_downstream_message(&mut self, msg: TrackMsg) {
        self.behavior.handle_downstream_message(msg);
    }

    fn handle_upstream_message(&mut self, msg: XTouchUpstreamMsg) {
        let index = B::INDEX;
        match msg {
            XTouchUpstreamMsg::EncoderPress(msg) => {
                if msg.idx as usize == index {
                    self.mode_change_button_press();
                }
            }
            XTouchUpstreamMsg::EncoderRelease(msg) => {
                if msg.idx as usize == index {
                    self.mode_change_button_release();
                    self.on_click();
                }
            }
            XTouchUpstreamMsg::EncoderTurnInc(msg) => {
                if msg.idx as usize == index {
                    // TODO:
                }
            }
            XTouchUpstreamMsg::EncoderTurnDec(msg) => {
                if msg.idx as usize == index {
                    // TODO:
                }
            }
            _ => {
                // Ignore other messages
            }
        }
    }
}

struct HPWidgetBehavior {
    hp_filt_freq: f32,
}

impl ChannelWidgetBehavior for HPWidgetBehavior {
    const INDEX: usize = 0;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        default: "HP Filt",
        press: "Slope",
        shift: "EQ Type",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn on_encoder_inc(&mut self) {
        self.hp_filt_freq += 1.0; // TODO: scale appropriately and add limits
    }

    fn on_encoder_dec(&mut self) {
        self.hp_filt_freq -= 1.0; // TODO: scale appropriately and add limits
    }

    fn on_click(&mut self) -> Option<TrackMsg> {
        None
    }

    fn handle_downstream_message(&mut self, msg: TrackMsg) {
        // TODO
    }
}

struct LowFreqWidgetBehavior {
    low_freq: f32,
}

impl ChannelWidgetBehavior for LowFreqWidgetBehavior {
    const INDEX: usize = 1;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        default: "Low Freq",
        press: "Low Q / Slope",
        shift: "Bell/Shelf",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn on_encoder_inc(&mut self) {
        self.low_freq += 1.0; // TODO: scale appropriately and add limits
    }

    fn on_encoder_dec(&mut self) {
        self.low_freq -= 1.0; // TODO: scale appropriately and add limits
    }

    fn on_click(&mut self) -> Option<TrackMsg> {
        None
    }

    fn handle_downstream_message(&mut self, msg: TrackMsg) {
        // TODO
    }
}

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
            // GlobalPress maps to ReaperVolPan mode
            XTouchUpstreamMsg::GlobalPress => ModeState {
                mode: Mode::ReaperVolPan,
                state: State::RequestingModeTransition,
            },
            // MIDITracksPress maps to ReaperSends mode
            XTouchUpstreamMsg::MIDITracksPress => ModeState {
                mode: Mode::ReaperSends,
                state: State::RequestingModeTransition,
            },
            XTouchUpstreamMsg::InputsPress => curr_mode, // Inputs maps to this mode!
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
