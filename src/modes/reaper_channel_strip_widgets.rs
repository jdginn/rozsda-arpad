use crate::midi::v1m;
use crate::modes::mode_manager::{DownstreamIo, UpstreamIo};
use crate::modes::reaper_channel_strip_router::{
    BandMode, BypassMode, ChannelStripMsg, CompOrder, CompType, EqPosition, EqType, SaturationType,
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
    Disabled,
    Default,
    Press,
    Shift,
    ShiftPress,
}

#[derive(Clone, Debug)]
struct ChannelWidgetLabels {
    disabled: &'static str,
    default: &'static str,
    press: &'static str,
    shift: &'static str,
    shift_press: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct ChannelWidgetColors {
    disabled: u32, // Or some better datatype
    default: u32,
    press: u32,
    shift: u32,
    shift_press: u32,
}

/// ChannelWidgetCore handles shared logic around mode switching and message passing.
struct ChannelWidgetCore {
    mode: ChannelWidgetMode,
}

impl ChannelWidgetCore {
    fn on_mode_change(&mut self) {}

    fn mode_change_button_press(&mut self) -> Vec<ChannelStripMsg> {
        match self.mode {
            ChannelWidgetMode::Disabled => {}
            ChannelWidgetMode::Default => self.mode = ChannelWidgetMode::Press,
            ChannelWidgetMode::Press => {}
            ChannelWidgetMode::Shift => self.mode = ChannelWidgetMode::ShiftPress,
            ChannelWidgetMode::ShiftPress => {}
        }
        vec![]
    }

    fn mode_change_button_release(&mut self) -> Vec<ChannelStripMsg> {
        // TODO: sometimes this sends a message upstream, sometimes it just changes mode.
        match self.mode {
            ChannelWidgetMode::Disabled => {}
            ChannelWidgetMode::Default => {}
            ChannelWidgetMode::Press => self.mode = ChannelWidgetMode::Default,
            ChannelWidgetMode::Shift => {}
            ChannelWidgetMode::ShiftPress => self.mode = ChannelWidgetMode::Shift,
        }
        vec![]
    }

    fn mode_change_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        match self.mode {
            ChannelWidgetMode::Disabled => {}
            ChannelWidgetMode::Default => self.mode = ChannelWidgetMode::Shift,
            ChannelWidgetMode::Press => self.mode = ChannelWidgetMode::ShiftPress,
            ChannelWidgetMode::Shift => {}
            ChannelWidgetMode::ShiftPress => {}
        }
        vec![]
    }

    fn mode_change_shift_release(&mut self) -> Vec<ChannelStripMsg> {
        match self.mode {
            ChannelWidgetMode::Disabled => {}
            ChannelWidgetMode::Default => {}
            ChannelWidgetMode::Press => {}
            ChannelWidgetMode::Shift => self.mode = ChannelWidgetMode::Default,
            ChannelWidgetMode::ShiftPress => self.mode = ChannelWidgetMode::Press,
        }
        vec![]
    }
}

/// ChannelWidetBehavior defines the specific behavior of some specific widget.
pub trait ChannelWidgetBehavior {
    const LABELS: ChannelWidgetLabels;
    const COLORS: ChannelWidgetColors;
    const INDEX: usize; // Which encoder this widget is associated with (0-15)

    fn new() -> Self
    where
        Self: Sized;
    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_inc_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_encoder_dec_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn on_click_shift(&mut self) -> Vec<ChannelStripMsg> {
        vec![]
    }
    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg);

    // TODO: need to expose downstream updates somehow...
}

/// ChannelWidget is the full implementation of some widget.
pub struct ChannelWidget<B: ChannelWidgetBehavior> {
    core: ChannelWidgetCore,
    behavior: B,
}

impl<B: ChannelWidgetBehavior> ChannelWidget<B> {
    pub fn new() -> Self {
        Self {
            core: ChannelWidgetCore {
                mode: ChannelWidgetMode::Default,
            },
            behavior: B::new(),
        }
    }

    // By default, colors and labels switch between static values based on the mode.
    // In some situations, labels may need to change based on plugin state. In these cases,
    // override the method.
    fn color(&self) -> u32 {
        let colors = B::COLORS;
        match self.core.mode {
            ChannelWidgetMode::Disabled => colors.disabled,
            ChannelWidgetMode::Default => colors.default,
            ChannelWidgetMode::Press => colors.press,
            ChannelWidgetMode::Shift => colors.shift,
            ChannelWidgetMode::ShiftPress => colors.shift_press,
        }
    }

    fn label1(&self) -> &'static str {
        let labels = B::LABELS;
        match self.core.mode {
            ChannelWidgetMode::Disabled => labels.disabled,
            ChannelWidgetMode::Default => labels.default,
            ChannelWidgetMode::Press => labels.press,
            ChannelWidgetMode::Shift => labels.shift,
            ChannelWidgetMode::ShiftPress => labels.shift_press,
        }
    }

    fn label3(&self) -> &'static str {
        let labels = B::LABELS;
        match self.core.mode {
            ChannelWidgetMode::Disabled => "",
            ChannelWidgetMode::Default => labels.press,
            ChannelWidgetMode::Press => "",
            ChannelWidgetMode::Shift => labels.shift_press,
            ChannelWidgetMode::ShiftPress => "",
        }
    }

    fn label4(&self) -> &'static str {
        let labels = B::LABELS;
        match self.core.mode {
            ChannelWidgetMode::Disabled => "",
            ChannelWidgetMode::Default => labels.shift,
            ChannelWidgetMode::Press => labels.shift_press,
            ChannelWidgetMode::Shift => "",
            ChannelWidgetMode::ShiftPress => "",
        }
    }

    fn mode_change_button_press(&mut self) -> Vec<ChannelStripMsg> {
        self.core.mode_change_button_press()
    }
    fn mode_change_button_release(&mut self) -> Vec<ChannelStripMsg> {
        self.core.mode_change_button_release()
    }
    fn mode_change_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.core.mode_change_shift_press()
    }
    fn mode_change_shift_release(&mut self) -> Vec<ChannelStripMsg> {
        self.core.mode_change_shift_release()
    }
    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_inc_default()
    }
    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_dec_default()
    }
    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_inc_press()
    }
    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_dec_press()
    }
    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_inc_shift()
    }
    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_dec_shift()
    }
    fn on_encoder_inc_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_inc_shift_press()
    }
    fn on_encoder_dec_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_encoder_dec_shift_press()
    }
    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_click_default()
    }
    fn on_click_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.behavior.on_click_shift()
    }

    pub fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        self.behavior.handle_message_from_upstream(msg);

        // TODO: send feedback
    }

    fn send_feedback(&mut self, io: &mut dyn UpstreamIo) {
        // FIXME: depends on actually implementing scribble for v1m...

        // TODO: send color, labels, and range info downstream
        // Pseudocode:
        // self.core.downstream_tx.send(v1mv1m::DownstreamMsg::SetColor(B::INDEX, self.color()));
        // self.core.downstream_tx.send(v1mv1m::DownstreamMsg::SetLabel1(B::INDEX, self.label1()));
        // self.core.downstream_tx.send(v1mv1m::DownstreamMsg::SetLabel3(B::INDEX, self.label3()));
        // self.core.downstream_tx.send(v1mv1m::DownstreamMsg::SetLabel4(B::INDEX, self.label4()));
        // self.core.downstream_tx.send(v1mv1m::DownstreamMsg::SetRange(B::INDEX, self.behavior.range())); FIXME: add to trait
        // self.core.downstream_tx.send(v1mDownstreaMsg::SetLabel2(B::INDEX, self.behavior.label2()) FIXME: add to trait

        // Toy example
        io.send_to_v1m(
            v1m::EncoderRingMsg {
                idx: B::INDEX as i32,
                mode: v1m::EncoderRingMode::Point,
                val: 0, // TODO: get from behavior
            }
            .into(),
        )
    }

    // TODO: this needs to return ChannelStripMsg (and possibly send upstream through a channel?)
    pub fn handle_message_from_downstream(
        &mut self,
        msg: v1m::UpstreamMsg,
    ) -> Vec<ChannelStripMsg> {
        let index = B::INDEX;
        match msg {
            v1m::UpstreamMsg::EncoderPress(msg) => {
                if msg.idx as usize == index {
                    self.mode_change_button_press()
                } else {
                    vec![]
                }
            }
            v1m::UpstreamMsg::EncoderRelease(msg) => {
                if msg.idx as usize == index {
                    self.mode_change_button_release();
                    match self.core.mode {
                        ChannelWidgetMode::Default => self.on_click_default(),
                        ChannelWidgetMode::Shift => self.on_click_shift(),
                        _ => {
                            vec![]
                        }
                    }
                } else {
                    vec![]
                }
            }
            v1m::UpstreamMsg::EncoderTurnInc(msg) => {
                if msg.idx as usize == index {
                    match self.core.mode {
                        ChannelWidgetMode::Disabled => vec![],
                        ChannelWidgetMode::Default => self.on_encoder_inc_default(),
                        ChannelWidgetMode::Press => self.on_encoder_inc_press(),
                        ChannelWidgetMode::Shift => self.on_encoder_inc_shift(),
                        ChannelWidgetMode::ShiftPress => self.on_encoder_inc_shift_press(),
                    }
                } else {
                    vec![]
                }
            }
            v1m::UpstreamMsg::EncoderTurnDec(msg) => {
                if msg.idx as usize == index {
                    match self.core.mode {
                        ChannelWidgetMode::Disabled => vec![],
                        ChannelWidgetMode::Default => self.on_encoder_dec_default(),
                        ChannelWidgetMode::Press => self.on_encoder_dec_press(),
                        ChannelWidgetMode::Shift => self.on_encoder_dec_shift(),
                        ChannelWidgetMode::ShiftPress => self.on_encoder_dec_shift_press(),
                    }
                } else {
                    vec![]
                }
            }
            _ => {
                vec![]
                // Ignore other messages
            }
        }
    }
}

// ----------------------
// Widget implementations
// ----------------------

pub struct HPWidgetBehavior {
    hpf_freq: f32,
    hpf_slope: f32,
    eq_type: EqType,
}

impl ChannelWidgetBehavior for HPWidgetBehavior {
    const INDEX: usize = 0;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "EQ",
        default: "HP Filt",
        press: "Slope",
        shift: "EQ Type",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            hpf_freq: 0.0,
            hpf_slope: 0.0,
            eq_type: EqType::Digital,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hpf_freq += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HpfFreq(self.hpf_freq)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hpf_freq -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HpfFreq(self.hpf_freq)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.hpf_slope += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HpfSlope(self.hpf_slope)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.hpf_slope -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HpfSlope(self.hpf_slope)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through EQ types
        self.eq_type = match self.eq_type {
            EqType::Digital => EqType::Digital,
        };
        vec![ChannelStripMsg::EqType(self.eq_type)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::HpfFreq(freq) => self.hpf_freq = freq,
            ChannelStripMsg::HpfSlope(slope) => self.hpf_slope = slope,
            ChannelStripMsg::EqType(eq_type) => self.eq_type = eq_type,
            _ => {}
        }
        // TODO
    }
}

pub struct LowFreqWidgetBehavior {
    low_freq: f32,
    low_q: f32,
    low_slope: f32,
    low_band_mode: BandMode,
}

impl ChannelWidgetBehavior for LowFreqWidgetBehavior {
    const INDEX: usize = 1;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "Low Freq",
        press: "Low Q / Slope",
        shift: "Bell/Shelf",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            low_freq: 80.0,
            low_q: 1.0,
            low_slope: 1.0,
            low_band_mode: BandMode::Shelf,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.low_freq += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LowFreq(self.low_freq)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.low_freq -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LowFreq(self.low_freq)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        match self.low_band_mode {
            BandMode::Bell => {
                self.low_q += 1.0; // TODO: scale appropriately and add limits
                vec![ChannelStripMsg::LowQ(self.low_q)]
            }
            BandMode::Shelf => {
                self.low_slope += 1.0; // TODO: scale appropriately and add limits
                vec![ChannelStripMsg::LowSlope(self.low_slope)]
            }
        }
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        match self.low_band_mode {
            BandMode::Bell => {
                self.low_q -= 1.0; // TODO: scale appropriately and add limits
                vec![ChannelStripMsg::LowQ(self.low_q)]
            }
            BandMode::Shelf => {
                self.low_slope -= 1.0; // TODO: scale appropriately and add limits
                vec![ChannelStripMsg::LowSlope(self.low_slope)]
            }
        }
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Bell/Shelf
        self.low_band_mode = match self.low_band_mode {
            BandMode::Bell => BandMode::Shelf,
            BandMode::Shelf => BandMode::Bell,
        };
        vec![ChannelStripMsg::LowBandMode(self.low_band_mode)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Bell/Shelf
        self.low_band_mode = match self.low_band_mode {
            BandMode::Bell => BandMode::Shelf,
            BandMode::Shelf => BandMode::Bell,
        };
        vec![ChannelStripMsg::LowBandMode(self.low_band_mode)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::LowFreq(freq) => self.low_freq = freq,
            ChannelStripMsg::LowQ(q) => self.low_q = q,
            ChannelStripMsg::LowSlope(slope) => self.low_slope = slope,
            ChannelStripMsg::LowBandMode(band_mode) => self.low_band_mode = band_mode,
            _ => {}
        }
    }
}

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
/// | 16 | Gain        | Interface gain (only if armed)   | Trim             |                |                |                 |

pub struct LowGainWidgetBehavior {
    low_gain: f32,
}

impl ChannelWidgetBehavior for LowGainWidgetBehavior {
    const INDEX: usize = 2;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "Low Gain",
        press: "zero",
        shift: "",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self { low_gain: 0.0 }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.low_gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LowGain(self.low_gain)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.low_gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LowGain(self.low_gain)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        self.low_gain = 0.0;
        vec![ChannelStripMsg::LowGain(self.low_gain)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        if let ChannelStripMsg::LowGain(gain) = msg {
            self.low_gain = gain;
        }
    }
}

pub struct LmFreqWidgetBehavior {
    lm_freq: f32,
    lm_q: f32,
}

impl ChannelWidgetBehavior for LmFreqWidgetBehavior {
    const INDEX: usize = 3;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "LM Freq",
        press: "LM Q",
        shift: "",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            lm_freq: 800.0,
            lm_q: 1.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_freq += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LmFreq(self.lm_freq)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_freq -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LmFreq(self.lm_freq)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_q += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LmQ(self.lm_q)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_q -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LmQ(self.lm_q)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::LmFreq(freq) => self.lm_freq = freq,
            ChannelStripMsg::LmQ(q) => self.lm_q = q,
            _ => {}
        }
    }
}

pub struct LmGainWidgetBehavior {
    lm_gain: f32,
}

impl ChannelWidgetBehavior for LmGainWidgetBehavior {
    const INDEX: usize = 4;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "LM Gain",
        press: "zero",
        shift: "",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self { lm_gain: 0.0 }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LmGain(self.lm_gain)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::LmGain(self.lm_gain)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        self.lm_gain = 0.0;
        vec![ChannelStripMsg::LmGain(self.lm_gain)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        if let ChannelStripMsg::LmGain(gain) = msg {
            self.lm_gain = gain;
        }
    }
}

pub struct HmFreqWidgetBehavior {
    hm_freq: f32,
    hm_q: f32,
}

impl ChannelWidgetBehavior for HmFreqWidgetBehavior {
    const INDEX: usize = 5;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "HM Freq",
        press: "HM Q",
        shift: "",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            hm_freq: 1500.0,
            hm_q: 1.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_freq += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HmFreq(self.hm_freq)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_freq -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HmFreq(self.hm_freq)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_q += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HmQ(self.hm_q)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_q -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HmQ(self.hm_q)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::HmFreq(freq) => self.hm_freq = freq,
            ChannelStripMsg::HmQ(q) => self.hm_q = q,
            _ => {}
        }
    }
}

pub struct HmGainWidgetBehavior {
    hm_gain: f32,
}

impl ChannelWidgetBehavior for HmGainWidgetBehavior {
    const INDEX: usize = 6;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "HM Gain",
        press: "zero",
        shift: "",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self { hm_gain: 0.0 }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HmGain(self.hm_gain)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HmGain(self.hm_gain)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        self.hm_gain = 0.0;
        vec![ChannelStripMsg::HmGain(self.hm_gain)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        if let ChannelStripMsg::HmGain(gain) = msg {
            self.hm_gain = gain;
        }
    }
}

pub struct HighFreqWidgetBehavior {
    high_freq: f32,
    high_q: f32,
    high_band_mode: BandMode,
}

impl ChannelWidgetBehavior for HighFreqWidgetBehavior {
    const INDEX: usize = 7;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "High Freq",
        press: "High Q (bell) / slope (shelf)",
        shift: "Bell/Shelf",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            high_freq: 4000.0,
            high_q: 1.0,
            high_band_mode: BandMode::Bell,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.high_freq += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HighFreq(self.high_freq)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.high_freq -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HighFreq(self.high_freq)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        match self.high_band_mode {
            BandMode::Bell => {
                self.high_q += 1.0; // TODO: scale appropriately and add limits
                vec![ChannelStripMsg::HighQ(self.high_q)]
            }
            BandMode::Shelf => {
                // TODO
                vec![]
            }
        }
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        match self.high_band_mode {
            BandMode::Bell => {
                self.high_q -= 1.0; // TODO: scale appropriately and add limits
                vec![ChannelStripMsg::HighQ(self.high_q)]
            }
            BandMode::Shelf => {
                // TODO
                vec![]
            }
        }
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Bell/Shelf
        self.high_band_mode = match self.high_band_mode {
            BandMode::Bell => BandMode::Shelf,
            BandMode::Shelf => BandMode::Bell,
        };
        vec![ChannelStripMsg::HighBandMode(self.high_band_mode)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Bell/Shelf
        self.high_band_mode = match self.high_band_mode {
            BandMode::Bell => BandMode::Shelf,
            BandMode::Shelf => BandMode::Bell,
        };
        vec![ChannelStripMsg::HighBandMode(self.high_band_mode)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::HighFreq(freq) => self.high_freq = freq,
            ChannelStripMsg::HighQ(q) => self.high_q = q,
            ChannelStripMsg::HighBandMode(band_mode) => self.high_band_mode = band_mode,
            _ => {}
        }
    }
}

pub struct HighGainWidgetBehavior {
    high_gain: f32,
    sides_gain: f32,
}

impl ChannelWidgetBehavior for HighGainWidgetBehavior {
    const INDEX: usize = 8;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "High Gain",
        press: "zero",
        shift: "Sides Gain",
        shift_press: "zero Sides",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            high_gain: 1.0,
            sides_gain: 1.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.high_gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HighGain(self.high_gain)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.high_gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HighGain(self.high_gain)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.sides_gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HighSidesGain(self.sides_gain)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.sides_gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::HighSidesGain(self.sides_gain)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        self.high_gain = 0.0;
        vec![ChannelStripMsg::HighGain(self.high_gain)]
    }

    fn on_click_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.sides_gain = 0.0;
        vec![ChannelStripMsg::HighSidesGain(self.sides_gain)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::HighGain(gain) => self.high_gain = gain,
            ChannelStripMsg::HighSidesGain(gain) => self.sides_gain = gain,
            _ => {}
        }
    }
}

pub struct EqPosWidgetBehavior {
    eq_pos: EqPosition,
    comp_order: CompOrder,
    eq_bpyass: BypassMode,
}

impl ChannelWidgetBehavior for EqPosWidgetBehavior {
    const INDEX: usize = 9;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "EQ Pos",
        default: "EQ Pos",
        press: "EQ Bypass",
        shift: "Comp Order",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            eq_pos: EqPosition::First,
            comp_order: CompOrder::FtoS,
            eq_bpyass: BypassMode::Engaged,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through EQ positions
        self.eq_pos = match self.eq_pos {
            EqPosition::First => EqPosition::Middle,
            EqPosition::Middle => EqPosition::Last,
            EqPosition::Last => EqPosition::First,
        };
        vec![ChannelStripMsg::EqPos(self.eq_pos)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through EQ positions
        self.eq_pos = match self.eq_pos {
            EqPosition::First => EqPosition::Last,
            EqPosition::Middle => EqPosition::First,
            EqPosition::Last => EqPosition::Middle,
        };
        vec![ChannelStripMsg::EqPos(self.eq_pos)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Comp order
        self.comp_order = match self.comp_order {
            CompOrder::FtoS => CompOrder::StoF,
            CompOrder::StoF => CompOrder::FtoS,
        };
        vec![ChannelStripMsg::CompOrder(self.comp_order)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Comp order
        self.comp_order = match self.comp_order {
            CompOrder::FtoS => CompOrder::StoF,
            CompOrder::StoF => CompOrder::FtoS,
        };
        vec![ChannelStripMsg::CompOrder(self.comp_order)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        // Toggle EQ bypass
        self.eq_bpyass = match self.eq_bpyass {
            BypassMode::Bypassed => BypassMode::Engaged,
            BypassMode::Engaged => BypassMode::Bypassed,
        };
        vec![ChannelStripMsg::EqBypass(self.eq_bpyass)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::EqPos(pos) => self.eq_pos = pos,
            ChannelStripMsg::CompOrder(order) => self.comp_order = order,
            ChannelStripMsg::EqBypass(bypass) => self.eq_bpyass = bypass,
            _ => {}
        }
    }
}

pub struct CompThreshWidgetBehavior {
    comp_thresh: f32,
    comp_sc_filter: f32,
    comp2_thresh: f32,
    comp2_sc_filter: f32,
}

impl ChannelWidgetBehavior for CompThreshWidgetBehavior {
    const INDEX: usize = 10;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "Compressor",
        default: "Comp Thresh",
        press: "Comp SC Filter",
        shift: "Comp2 Thresh",
        shift_press: "Comp2 SC Filter",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            comp_thresh: -4.0,
            comp_sc_filter: 60.0,
            comp2_thresh: -4.0,
            comp2_sc_filter: 60.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_thresh += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompThresh(self.comp_thresh)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_thresh -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompThresh(self.comp_thresh)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_sc_filter += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompScFilter(self.comp_sc_filter)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_sc_filter -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompScFilter(self.comp_sc_filter)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_thresh += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Thresh(self.comp2_thresh)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_thresh -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Thresh(self.comp2_thresh)]
    }

    fn on_encoder_inc_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_sc_filter += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2ScFilter(self.comp2_sc_filter)]
    }

    fn on_encoder_dec_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_sc_filter -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2ScFilter(self.comp2_sc_filter)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::CompThresh(thresh) => self.comp_thresh = thresh,
            ChannelStripMsg::CompScFilter(filter) => self.comp_sc_filter = filter,
            ChannelStripMsg::Comp2Thresh(thresh) => self.comp2_thresh = thresh,
            ChannelStripMsg::Comp2ScFilter(filter) => self.comp2_sc_filter = filter,
            _ => {}
        }
    }
}

pub struct CompRatioWidgetBehavior {
    comp_ratio: f32,
    comp_attack: f32,
    comp2_ratio: f32,
    comp2_attack: f32,
}

impl ChannelWidgetBehavior for CompRatioWidgetBehavior {
    const INDEX: usize = 11;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "Comp Ratio",
        press: "Comp Attack",
        shift: "Comp2 Ratio",
        shift_press: "Comp2 Attack",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            comp_ratio: 1.0,
            comp_attack: 10.0,
            comp2_ratio: 1.0,
            comp2_attack: 10.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_ratio += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompRatio(self.comp_ratio)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_ratio -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompRatio(self.comp_ratio)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_attack += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompAttack(self.comp_attack)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_attack -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompAttack(self.comp_attack)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_ratio += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Ratio(self.comp2_ratio)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_ratio -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Ratio(self.comp2_ratio)]
    }

    fn on_encoder_inc_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_attack += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Attack(self.comp2_attack)]
    }

    fn on_encoder_dec_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_attack -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Attack(self.comp2_attack)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::CompRatio(ratio) => self.comp_ratio = ratio,
            ChannelStripMsg::CompAttack(attack) => self.comp_attack = attack,
            ChannelStripMsg::Comp2Ratio(ratio) => self.comp2_ratio = ratio,
            ChannelStripMsg::Comp2Attack(attack) => self.comp2_attack = attack,
            _ => {}
        }
    }
}

pub struct CompMakeupWidgetBehavior {
    comp_makeup: f32,
    comp_release: f32,
    comp2_makeup: f32,
    comp2_release: f32,
}

impl ChannelWidgetBehavior for CompMakeupWidgetBehavior {
    const INDEX: usize = 12;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "Comp Makeup",
        press: "Comp Release",
        shift: "Comp2 Makeup",
        shift_press: "Comp2 Release",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            comp_makeup: 0.0,
            comp_release: 100.0,
            comp2_makeup: 0.0,
            comp2_release: 100.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_makeup += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompMakeup(self.comp_makeup)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_makeup -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompMakeup(self.comp_makeup)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_release += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompRelease(self.comp_release)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp_release -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::CompRelease(self.comp_release)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_makeup += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Makeup(self.comp2_makeup)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_makeup -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Makeup(self.comp2_makeup)]
    }

    fn on_encoder_inc_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_release += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Release(self.comp2_release)]
    }

    fn on_encoder_dec_shift_press(&mut self) -> Vec<ChannelStripMsg> {
        self.comp2_release -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Comp2Release(self.comp2_release)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::CompMakeup(makeup) => self.comp_makeup = makeup,
            ChannelStripMsg::CompRelease(release) => self.comp_release = release,
            ChannelStripMsg::Comp2Makeup(makeup) => self.comp2_makeup = makeup,
            ChannelStripMsg::Comp2Release(release) => self.comp2_release = release,
            _ => {}
        }
    }
}

pub struct CompTypeWidgetBehavior {
    comp_type: CompType,
    comp2_type: CompType,
    comp_bypass: BypassMode,
    comp2_bypass: BypassMode,
}

impl ChannelWidgetBehavior for CompTypeWidgetBehavior {
    const INDEX: usize = 13;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "",
        default: "Comp Type",
        press: "Bypass Comp",
        shift: "Comp2 Type",
        shift_press: "Bypass Comp2",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            comp_type: CompType::Digital,
            comp2_type: CompType::Digital,
            comp_bypass: BypassMode::Engaged,
            comp2_bypass: BypassMode::Bypassed,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Comp types
        self.comp_type = match self.comp_type {
            CompType::Digital => CompType::Digital,
        };
        vec![ChannelStripMsg::CompType(self.comp_type)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Comp types
        self.comp_type = match self.comp_type {
            CompType::Digital => CompType::Digital,
        };
        vec![ChannelStripMsg::CompType(self.comp_type)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        // Toggle Comp bypass
        self.comp_bypass = match self.comp_bypass {
            BypassMode::Bypassed => BypassMode::Engaged,
            BypassMode::Engaged => BypassMode::Bypassed,
        };
        vec![ChannelStripMsg::CompBypass(self.comp_bypass)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Comp2 types
        self.comp2_type = match self.comp2_type {
            CompType::Digital => CompType::Digital,
        };
        vec![ChannelStripMsg::Comp2Type(self.comp2_type)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Comp2 types
        self.comp2_type = match self.comp2_type {
            CompType::Digital => CompType::Digital,
        };
        vec![ChannelStripMsg::Comp2Type(self.comp2_type)]
    }

    fn on_click_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Toggle Comp2 bypass
        self.comp2_bypass = match self.comp2_bypass {
            BypassMode::Bypassed => BypassMode::Engaged,
            BypassMode::Engaged => BypassMode::Bypassed,
        };
        vec![ChannelStripMsg::Comp2Bypass(self.comp2_bypass)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::CompType(comp_type) => self.comp_type = comp_type,
            ChannelStripMsg::CompBypass(bypass) => self.comp_bypass = bypass,
            ChannelStripMsg::Comp2Type(comp2_type) => self.comp2_type = comp2_type,
            ChannelStripMsg::Comp2Bypass(bypass) => self.comp2_bypass = bypass,
            _ => {}
        }
    }
}

pub struct SaturationWidgetBehavior {
    saturation: f32,
    saturation_type: SaturationType,
    saturation_bypass: BypassMode,
}

impl ChannelWidgetBehavior for SaturationWidgetBehavior {
    const INDEX: usize = 14;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "Saturation",
        default: "Saturation",
        press: "Bypass Sat",
        shift: "Saturation Type",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            saturation: 0.0,
            saturation_type: SaturationType::Console,
            saturation_bypass: BypassMode::Bypassed,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.saturation += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Saturation(self.saturation)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.saturation -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Saturation(self.saturation)]
    }

    fn on_click_default(&mut self) -> Vec<ChannelStripMsg> {
        // Toggle Saturation bypass
        self.saturation_bypass = match self.saturation_bypass {
            BypassMode::Bypassed => BypassMode::Engaged,
            BypassMode::Engaged => BypassMode::Bypassed,
        };
        vec![(ChannelStripMsg::SaturationBypass(self.saturation_bypass))]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Saturation types
        self.saturation_type = match self.saturation_type {
            SaturationType::Console => SaturationType::Console,
        };
        vec![(ChannelStripMsg::SaturationType(self.saturation_type))]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        // Cycle through Saturation types
        self.saturation_type = match self.saturation_type {
            SaturationType::Console => SaturationType::Console,
        };
        vec![(ChannelStripMsg::SaturationType(self.saturation_type))]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::Saturation(saturation) => self.saturation = saturation,
            ChannelStripMsg::SaturationType(sat_type) => self.saturation_type = sat_type,
            ChannelStripMsg::SaturationBypass(bypass) => self.saturation_bypass = bypass,
            _ => {}
        }
    }
}

pub struct GainWidgetBehavior {
    gain: f32,
    interface_gain: f32,
    trim: f32,
}

impl ChannelWidgetBehavior for GainWidgetBehavior {
    const INDEX: usize = 15;
    const LABELS: ChannelWidgetLabels = ChannelWidgetLabels {
        disabled: "Gain",
        default: "Gain",
        press: "Interface Gain (if armed)",
        shift: "Trim",
        shift_press: "",
    };
    const COLORS: ChannelWidgetColors = ChannelWidgetColors {
        disabled: 0x000000,
        default: 0xFF0000,
        press: 0x00FF00,
        shift: 0x0000FF,
        shift_press: 0xFFFF00,
    };

    fn new() -> Self {
        Self {
            gain: 0.0,
            interface_gain: 0.0,
            trim: 0.0,
        }
    }

    fn on_encoder_inc_default(&mut self) -> Vec<ChannelStripMsg> {
        self.gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Gain(self.gain)]
    }

    fn on_encoder_dec_default(&mut self) -> Vec<ChannelStripMsg> {
        self.gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Gain(self.gain)]
    }

    fn on_encoder_inc_press(&mut self) -> Vec<ChannelStripMsg> {
        self.interface_gain += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::InterfaceGain(self.interface_gain)]
    }

    fn on_encoder_dec_press(&mut self) -> Vec<ChannelStripMsg> {
        self.interface_gain -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::InterfaceGain(self.interface_gain)]
    }

    fn on_encoder_inc_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.trim += 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Trim(self.trim)]
    }

    fn on_encoder_dec_shift(&mut self) -> Vec<ChannelStripMsg> {
        self.trim -= 1.0; // TODO: scale appropriately and add limits
        vec![ChannelStripMsg::Trim(self.trim)]
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) {
        match msg {
            ChannelStripMsg::Gain(gain) => self.gain = gain,
            ChannelStripMsg::InterfaceGain(interface_gain) => self.interface_gain = interface_gain,
            ChannelStripMsg::Trim(trim) => self.trim = trim,
            _ => {}
        }
    }
}
