use bitflags::bitflags;

use crate::midi::v1m;
use crate::modes::color;
use crate::modes::color::RgbColor;
use crate::modes::reaper_channel_strip_router::{
    BandMode, ChannelStripMsg, CompOrder, CompType, EqPosition, EqType, SaturationType,
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
// In this mode, faders do the same thing as ReaperVolumePanMode. Faders are the surfaces where being out
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

// TODO:
// - Widgets should probably hold strings, not static &str
// - Scribble text doesn't blindly follow mode
// - Should probably rename mode->layer? Some other term? Mode is overloaded with the concept of
// Mode according to the ModeManager. Could maybe be WidgetMode or WidgetActionLayer.
// - Need to add support to change scribble strip based on feedback from upstream; i.e. an fx
// parameter changes. Also, need to change top scribble strip to show value for some time after
// encoder is turned.
// - Need some kind of timer support to change scribble strip for N seconds and then change it back.
// Watchdog timer? Does that become expensive? Do we need to dip into async rust for this? Is that expensive?

// Architecture
// ChannelMode is responsible for processing and dispatching messages from downstream. Router is
// resopnsible for dispatching messages both to and from upsream.
//
// ChannelMode needs a few more members:
// - Dirty flag (to know when to send feedback downstream)
//      - TODO: we probably need at least two different dirty flags, since there are at least two
//      downstream messages that set different portions of the text
//
// On receiving a message from downstream:
// - Update state for EACH widget
//      - Select Press/Release -> update all of them
//      - Encoder press -> update only the relevant widget
// - All of this happens in the main thread, all directly by ChannelMode.
// - For encoder turn messages, call the appropriate widget's message (part of the Widget trait)
// - Widget holds text for all the modes. Widget updates text accordingly. Widget returns a dirty flag to ChannelMode, which ChannelMode updates
// - Encoders return a vec of messages to send both upstream and downstream
//      - Downstream: encoder LED ring messages
//      - Upstream: ChannelMessages that go to the router
// - At the top of each loop where ChannelMode processes inputs on its crossbeam channels, check
// dity bit. If dirty AND we are not in backoff, coalesce text from all widgets and send downstream
// message(s) accordingly. If either not dirty or in backoff, no action.
//
// Notably, we can route all messages from downstream exactly where they need to go, meaning there
// is no need to send messages to all widgets and do fallthrough to the next widget if the message was irrelevant. Maybe we can do something similar in router for messages from upstream.
//
// TODO: define how router works
//
// ChannelMode processes messages from downstream and sets sets WidgetMode. Upstream can never
// change WidgetMode. WidgetMode is a key driver of scribble strip values.
//
// ChannelMode process incoming messages from upstream but delegates their processing to
// ChannelRouter. ChannelRouter turns these into ChannelMessages. Widgets also send ChannelMessages
// back to ChannelRouter, who converts them into the appropriate TrackMsg to send upward.
//
// Incoming ChannelMessages to a widget _can_ result in additional messages going downstream.
//
// IMPORTANT: widgets should NEVER access the upstream/downstream channels direcdtly

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetMode {
    Disabled,
    Default,
    Press,
    Shift,
    ShiftPress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderTurn {
    Inc { accel: u8 },
    Dec { accel: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderEvent {
    EncoderTurn(EncoderTurn),
    Click,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeEvent {
    EncoderPress,
    EncoderRelease,
    ShiftPress,
    ShiftRelease,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Dirty: u8 {
        const NONE = 0b000;
        const LINE1 = 0b001;
        const LINE2 = 0b010;
        const COLOR = 0b100;
    }
}

#[derive(Clone, Debug)]
pub struct HandledUpstreamOutcome {
    pub downstream_msgs: Vec<v1m::DownstreamMsg>,
    pub dirty: Dirty,
}

impl Default for HandledUpstreamOutcome {
    fn default() -> Self {
        Self {
            downstream_msgs: vec![],
            dirty: Dirty::default(),
        }
    }
}

impl HandledUpstreamOutcome {
    fn downstream(mut self, msg: v1m::DownstreamMsg) -> Self {
        self.downstream_msgs.push(msg);
        self
    }
    fn dirty(mut self, d: Dirty) -> Self {
        self.dirty |= d;
        self
    }
}

#[derive(Clone, Debug)]
pub struct HandledEncoderOutcome {
    pub upstream_msgs: Vec<ChannelStripMsg>,
    pub downstream_msgs: Vec<v1m::DownstreamMsg>,
    pub dirty: Dirty,
}

impl Default for HandledEncoderOutcome {
    fn default() -> Self {
        Self {
            upstream_msgs: vec![],
            downstream_msgs: vec![],
            dirty: Dirty::default(),
        }
    }
}

impl HandledEncoderOutcome {
    fn upstream(mut self, msg: ChannelStripMsg) -> Self {
        self.upstream_msgs.push(msg);
        self
    }
    fn downstream(mut self, msg: v1m::DownstreamMsg) -> Self {
        self.downstream_msgs.push(msg);
        self
    }
    fn dirty(mut self, d: Dirty) -> Self {
        self.dirty |= d;
        self
    }
}

pub trait Widget {
    fn new(hw_idx: usize) -> Self
    where
        Self: Sized;

    fn view(&self) -> &WidgetView;
    fn view_mut(&mut self) -> &mut WidgetView;

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome;
    fn handle_encoder_event(&mut self, event: EncoderEvent) -> HandledEncoderOutcome;

    fn is_clickable_default(&self) -> bool {
        false
    }

    fn is_clickable_shift(&self) -> bool {
        false
    }

    fn set_mode(&mut self, event: ModeEvent) {
        if self.is_clickable_default() && event == ModeEvent::EncoderPress {
            return;
        }
        if self.is_clickable_shift() && event == ModeEvent::ShiftPress {
            return;
        }
        self.view_mut().set_mode(event);
    }
    fn line1_text(&self) -> String {
        self.view().line1_text()
    }
    fn line2_text(&self) -> String {
        self.view().line2_text()
    }
    fn color(&self) -> RgbColor {
        self.view().color()
    }
}

pub struct WidgetView {
    mode: WidgetMode,

    line1_default: String,
    line1_shift: String,

    line2_default: String,
    line2_shift: String,

    color_default: RgbColor,
    color_shift: RgbColor,
}

impl WidgetView {
    fn set_mode(&mut self, event: ModeEvent) {
        self.mode = match (self.mode, event) {
            (WidgetMode::Disabled, _) => WidgetMode::Disabled,
            (WidgetMode::Default, ModeEvent::EncoderPress) => WidgetMode::Press,
            (WidgetMode::Default, ModeEvent::ShiftPress) => WidgetMode::Shift,
            (WidgetMode::Default, _) => WidgetMode::Default,
            (WidgetMode::Press, ModeEvent::EncoderRelease) => WidgetMode::Default,
            (WidgetMode::Press, ModeEvent::ShiftPress) => WidgetMode::ShiftPress,
            (WidgetMode::Press, _) => WidgetMode::Press,
            (WidgetMode::Shift, ModeEvent::EncoderPress) => WidgetMode::ShiftPress,
            (WidgetMode::Shift, ModeEvent::ShiftRelease) => WidgetMode::Default,
            (WidgetMode::Shift, _) => WidgetMode::Shift,
            (WidgetMode::ShiftPress, ModeEvent::EncoderRelease) => WidgetMode::Shift,
            (WidgetMode::ShiftPress, ModeEvent::ShiftRelease) => WidgetMode::Press,
            (WidgetMode::ShiftPress, _) => WidgetMode::ShiftPress,
        }
    }

    fn line1_text(&self) -> String {
        match self.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Default => self.line1_default.clone(),
            WidgetMode::Press => self.line1_default.clone(),
            WidgetMode::Shift => self.line1_shift.clone(),
            WidgetMode::ShiftPress => self.line1_shift.clone(),
        }
    }

    fn line2_text(&self) -> String {
        match self.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Default => self.line2_default.clone(),
            WidgetMode::Press => self.line2_default.clone(),
            WidgetMode::Shift => self.line2_shift.clone(),
            WidgetMode::ShiftPress => self.line2_shift.clone(),
        }
    }

    fn color(&self) -> RgbColor {
        match self.mode {
            WidgetMode::Disabled => RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
            WidgetMode::Default => self.color_default,
            WidgetMode::Press => self.color_default,
            WidgetMode::Shift => self.color_shift,
            WidgetMode::ShiftPress => self.color_shift,
        }
    }
}

fn apply_accel(prev: f32, turn: EncoderTurn) -> f32 {
    fn factor(accel: u8) -> f32 {
        match accel {
            1 => 0.05,
            2 => 0.1,
            3 => 0.2,
            4 => 0.4,
            _ => 0.4,
        }
    }
    match turn {
        EncoderTurn::Inc { accel } => (prev + factor(accel)).min(1.0),
        EncoderTurn::Dec { accel } => (prev - factor(accel)).max(0.0),
    }
}

fn apply_accel_center(prev: f32, turn: EncoderTurn) -> f32 {
    fn factor(accel: u8) -> f32 {
        match accel {
            1 => 0.05,
            2 => 0.1,
            3 => 0.2,
            4 => 0.4,
            _ => 0.4,
        }
    }
    match turn {
        EncoderTurn::Inc { accel } => (prev + factor(accel)).min(1.0),
        EncoderTurn::Dec { accel } => (prev - factor(accel)).max(-1.0),
    }
}

// ----------------------
// Individual Widget implementations
// ----------------------

pub struct HpfWidget {
    hw_idx: usize,
    view: WidgetView,
    freq: f32,
    slope: f32,
    eq_type: EqType,
}

impl Widget for HpfWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        const DEFAULT_EQ_TYPE: EqType = EqType::Digital;
        Self {
            hw_idx,
            view: WidgetView {
                mode: WidgetMode::Default,
                line1_default: "HP Filter".to_string(),
                line1_shift: "EQ Type".to_string(),
                line2_default: "".to_string(),
                line2_shift: DEFAULT_EQ_TYPE.as_str().to_string(),
                color_default: color::DARK_BROWN,
                color_shift: color::DARK_BROWN,
            },
            freq: 0.1,
            slope: 0.0, // TODO: how do we represent this?
            eq_type: DEFAULT_EQ_TYPE,
        }
    }

    fn handle_encoder_event(&mut self, event: EncoderEvent) -> HandledEncoderOutcome {
        match (self.view.mode, event) {
            (WidgetMode::Default, EncoderEvent::EncoderTurn(turn)) => {
                self.freq = apply_accel(self.freq, turn);
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::HpfFreq(self.freq))
                    .downstream(
                        v1m::EncoderRingMsg {
                            idx: self.hw_idx as i32,
                            mode: v1m::EncoderRingMode::FromLeft,
                            val: v1m::map_to_encoder_ring_from_left(self.freq),
                        }
                        .into(),
                    )
                    .dirty(Dirty::LINE1)
            }
            (WidgetMode::Press, EncoderEvent::EncoderTurn(turn)) => {
                self.slope = apply_accel(self.slope, turn);
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::HpfSlope(self.slope))
                    .dirty(Dirty::LINE2)
            }
            (WidgetMode::Shift, EncoderEvent::EncoderTurn(turn)) => {
                self.eq_type = self.eq_type.step(turn);
                self.view.line2_shift = self.eq_type.as_str().to_string();
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::EqType(self.eq_type))
                    .dirty(Dirty::LINE2)
            }
            _ => HandledEncoderOutcome::default(),
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            ChannelStripMsg::HpfFreq(freq) => {
                self.freq = freq;
                HandledUpstreamOutcome::default()
                    .dirty(Dirty::LINE1)
                    .downstream(
                        v1m::EncoderRingMsg {
                            idx: self.hw_idx as i32,
                            mode: v1m::EncoderRingMode::FromLeft,
                            val: v1m::map_to_encoder_ring_from_left(self.freq),
                        }
                        .into(),
                    )
            }
            ChannelStripMsg::HpfSlope(slope) => {
                self.slope = slope;
                HandledUpstreamOutcome::default().dirty(Dirty::LINE2)
            }
            ChannelStripMsg::EqType(eq_type) => {
                self.eq_type = eq_type;
                HandledUpstreamOutcome::default().dirty(Dirty::LINE2)
            }
            _ => HandledUpstreamOutcome::default(),
        }
    }
}

pub struct LowFreqWidget {
    hw_idx: usize,
    view: WidgetView,

    freq: f32,
    q: f32,
    band_mode: BandMode,
}

impl Widget for LowFreqWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        const DEFAULT_BAND_MODE: BandMode = BandMode::Shelf;
        Self {
            hw_idx,
            view: WidgetView {
                mode: WidgetMode::Default,
                line1_default: "LowFreq".to_string(),
                line1_shift: "LowMode".to_string(),
                line2_default: "".to_string(),
                line2_shift: DEFAULT_BAND_MODE.as_str().to_string(),
                color_default: color::BROWN,
                color_shift: color::BROWN,
            },
            freq: 0.2,
            q: 0.72,
            band_mode: DEFAULT_BAND_MODE,
        }
    }

    fn handle_encoder_event(&mut self, event: EncoderEvent) -> HandledEncoderOutcome {
        match (self.view.mode, event) {
            (WidgetMode::Default, EncoderEvent::EncoderTurn(turn)) => {
                self.freq = apply_accel(self.freq, turn);
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::LowFreq(self.freq))
                    .downstream(
                        v1m::EncoderRingMsg {
                            idx: self.hw_idx as i32,
                            mode: v1m::EncoderRingMode::FromLeft,
                            val: v1m::map_to_encoder_ring_from_left(self.freq),
                        }
                        .into(),
                    )
                    .dirty(Dirty::LINE1)
            }
            (WidgetMode::Press, EncoderEvent::EncoderTurn(turn)) => {
                self.q = apply_accel(self.q, turn);
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::LowSlope(self.q))
                    .dirty(Dirty::LINE2)
            }
            (WidgetMode::Shift, EncoderEvent::EncoderTurn(turn)) => {
                self.band_mode = self.band_mode.step(turn);
                self.view.line2_shift = self.band_mode.as_str().to_string();
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::LowBandMode(self.band_mode))
                    .dirty(Dirty::LINE2)
            }
            _ => HandledEncoderOutcome::default(),
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            // TODO:
            _ => HandledUpstreamOutcome::default(),
        }
        // TODO
    }

    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Default | WidgetMode::Press => match self.band_mode {
                BandMode::Shelf => format!("{} Q", self.q),
                BandMode::Bell => "".to_string(),
            },
            WidgetMode::Shift | WidgetMode::ShiftPress => self.view.line2_shift.clone(),
        }
    }
}

pub struct LowGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
}

impl Widget for LowGainWidget {
    fn is_clickable_default(&self) -> bool {
        true
    }
    fn is_clickable_shift(&self) -> bool {
        true
    }
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        Self {
            hw_idx,
            view: WidgetView {
                mode: WidgetMode::Default,
                line1_default: "LowGain".to_string(),
                line1_shift: "LowGain".to_string(),
                line2_default: "zero".to_string(),
                line2_shift: "zero".to_string(),
                color_default: color::BROWN,
                color_shift: color::BROWN,
            },
            gain: 0.0,
        }
    }

    fn handle_encoder_event(&mut self, event: EncoderEvent) -> HandledEncoderOutcome {
        match (self.view.mode, event) {
            (WidgetMode::Default | WidgetMode::Shift, EncoderEvent::EncoderTurn(turn)) => {
                self.gain = apply_accel_center(self.gain, turn);
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::LowFreq(self.gain))
                    .downstream(
                        v1m::EncoderRingMsg {
                            idx: self.hw_idx as i32,
                            mode: v1m::EncoderRingMode::FromCenter,
                            val: v1m::map_to_encoder_ring_from_center(self.gain),
                        }
                        .into(),
                    )
                    .dirty(Dirty::LINE1)
            }
            (_, EncoderEvent::Click) => {
                println!("CLICK!");
                self.gain = 0.0;
                HandledEncoderOutcome::default()
                    .upstream(ChannelStripMsg::LowGain(0.0))
                    .downstream(
                        v1m::EncoderRingMsg {
                            idx: self.hw_idx as i32,
                            mode: v1m::EncoderRingMode::FromCenter,
                            val: v1m::map_to_encoder_ring_from_center(0.0),
                        }
                        .into(),
                    )
            }
            _ => HandledEncoderOutcome::default(),
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            // TODO:
            _ => HandledUpstreamOutcome::default(),
        }
        // TODO
    }
}
