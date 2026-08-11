use crate::midi::v1m;
use crate::midi::v1m::EncoderRingMode::{FromCenter, FromLeft, Point, Width};
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
    Normal,
    Press,
    Shift,
    ShiftPress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderTurn {
    Inc { accel: u8 },
    Dec { accel: u8 },
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

struct ClickEncoderBehavior {
    is_pressed: bool,
}

struct HoldEncoderBehavior {
    is_pressed: bool,
}

enum EncoderClickBehavior {
    Click(ClickEncoderBehavior),
    Hold(HoldEncoderBehavior),
}

pub trait Widget {
    fn new(hw_idx: usize) -> Self
    where
        Self: Sized;

    fn view(&self) -> &WidgetView;
    fn view_mut(&mut self) -> &mut WidgetView;

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome;

    fn on_click(&mut self) -> Option<HandledEncoderOutcome> {
        None
    }

    fn on_shift_click(&mut self) -> Option<HandledEncoderOutcome> {
        None
    }

    fn handle_encoder_click(&mut self) -> Option<HandledEncoderOutcome> {
        if !self.view().encoder_pressed {
            let outcome = match self.view().mode {
                WidgetMode::Disabled => None,
                WidgetMode::Normal => {
                    if let Some(outcome) = self.on_click() {
                        Some(outcome)
                    } else {
                        self.view_mut().mode = WidgetMode::Press;
                        self.view_mut().in_hold_mode = true;
                        // FIXME
                        Some(HandledEncoderOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    }
                }
                WidgetMode::Press => None,
                WidgetMode::Shift => {
                    if let Some(outcome) = self.on_shift_click() {
                        Some(outcome)
                    } else {
                        self.view_mut().mode = WidgetMode::ShiftPress;
                        self.view_mut().in_hold_mode = true;
                        Some(HandledEncoderOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    }
                }
                WidgetMode::ShiftPress => None,
            };
            self.view_mut().encoder_pressed = true;
            outcome
        } else {
            let outcome = match self.view().mode {
                WidgetMode::Disabled => None,
                WidgetMode::Normal => None,
                WidgetMode::Press => {
                    if self.view_mut().in_hold_mode {
                        self.view_mut().in_hold_mode = false;
                        self.view_mut().mode = WidgetMode::Normal;
                        Some(HandledEncoderOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    } else {
                        None
                    }
                }
                WidgetMode::Shift => None,
                WidgetMode::ShiftPress => {
                    if self.view_mut().in_hold_mode {
                        self.view_mut().in_hold_mode = false;
                        self.view_mut().mode = WidgetMode::Shift;
                        Some(HandledEncoderOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    } else {
                        None
                    }
                }
            };
            self.view_mut().encoder_pressed = false;
            outcome
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        let _ = turn;
        None
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

    encoder_pressed: bool,
    in_hold_mode: bool,

    line1_normal: String,
    line1_shift: String,

    line2_normal: String,
    line2_shift: String,

    color_normal: RgbColor,
    color_shift: RgbColor,
}

impl Default for WidgetView {
    fn default() -> Self {
        Self {
            mode: WidgetMode::Normal,
            encoder_pressed: false,
            in_hold_mode: false,
            line1_normal: String::new(),
            line1_shift: String::new(),
            line2_normal: String::new(),
            line2_shift: String::new(),
            color_normal: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
            color_shift: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
        }
    }
}

impl WidgetView {
    fn new() -> Self {
        Self {
            mode: WidgetMode::Normal,
            encoder_pressed: false,
            in_hold_mode: false,
            line1_normal: String::new(),
            line1_shift: String::new(),
            line2_normal: String::new(),
            line2_shift: String::new(),
            color_normal: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
            color_shift: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
        }
    }

    fn line1_normal(self, txt: &str) -> Self {
        Self {
            line1_normal: txt.to_string(),
            ..self
        }
    }

    fn line1_shift(self, txt: &str) -> Self {
        Self {
            line1_shift: txt.to_string(),
            ..self
        }
    }

    fn line2_normal(self, txt: &str) -> Self {
        Self {
            line2_normal: txt.to_string(),
            ..self
        }
    }

    fn line2_shift(self, txt: &str) -> Self {
        Self {
            line2_shift: txt.to_string(),
            ..self
        }
    }

    fn color_normal(self, color: RgbColor) -> Self {
        Self {
            color_normal: color,
            ..self
        }
    }

    fn color_shift(self, color: RgbColor) -> Self {
        Self {
            color_shift: color,
            ..self
        }
    }

    // Used at runtime
    fn line1_text(&self) -> String {
        match self.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Normal => self.line1_normal.clone(),
            WidgetMode::Press => self.line1_normal.clone(),
            WidgetMode::Shift => self.line1_shift.clone(),
            WidgetMode::ShiftPress => self.line1_shift.clone(),
        }
    }

    fn line2_text(&self) -> String {
        match self.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Normal => self.line2_normal.clone(),
            WidgetMode::Press => self.line2_normal.clone(),
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
            WidgetMode::Normal => self.color_normal,
            WidgetMode::Press => self.color_normal,
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

fn encoder_ring_msg(hw_idx: usize, val: f32, mode: v1m::EncoderRingMode) -> v1m::DownstreamMsg {
    v1m::EncoderRingMsg {
        idx: hw_idx as i32,
        mode,
        val: match mode {
            v1m::EncoderRingMode::FromLeft => v1m::map_to_encoder_ring_from_left(val),
            v1m::EncoderRingMode::Point => v1m::map_to_encoder_ring_from_left(val),
            v1m::EncoderRingMode::Width => v1m::map_to_encoder_ring_from_left(val), //TODO:
            v1m::EncoderRingMode::FromCenter => v1m::map_to_encoder_ring_from_center(val),
        },
    }
    .into()
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
            view: WidgetView::new()
                .line1_normal("HP Filter")
                .line1_shift("EQ Type")
                .line2_shift(DEFAULT_EQ_TYPE.as_str())
                .color_normal(color::DARK_BROWN)
                .color_shift(color::DARK_BROWN),
            freq: 0.1,
            slope: 0.0, // TODO: how do we represent this?
            eq_type: DEFAULT_EQ_TYPE,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::HpfFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.slope = apply_accel(self.slope, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::HpfSlope(self.slope))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.eq_type = self.eq_type.step(turn);
                self.view.line2_shift = self.eq_type.as_str().to_string();
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::EqType(self.eq_type))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            ChannelStripMsg::HpfFreq(freq) => {
                self.freq = freq;
                HandledUpstreamOutcome::default()
                    .dirty(Dirty::LINE1)
                    .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
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
            view: WidgetView::new()
                .line1_normal("LowFreq")
                .line1_shift("LowMode")
                .line2_shift(DEFAULT_BAND_MODE.as_str())
                .color_normal(color::BROWN)
                .color_shift(color::BROWN),
            freq: 0.2,
            q: 0.72,
            band_mode: DEFAULT_BAND_MODE,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LowFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.q = apply_accel(self.q, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LowSlope(self.q))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.band_mode = self.band_mode.step(turn);
                self.view.line2_shift = self.band_mode.as_str().to_string();
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LowBandMode(self.band_mode))
                        .dirty(Dirty::LINE2),
                )
            }
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
            WidgetMode::Normal | WidgetMode::Press => match self.band_mode {
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
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal("LowGain")
                .line2_normal("zero")
                .line1_shift("LowGain")
                .line2_shift("zero")
                .color_normal(color::BROWN)
                .color_shift(color::BROWN),
            gain: 0.0,
        }
    }

    fn on_click(&mut self) -> Option<HandledEncoderOutcome> {
        self.gain = 0.0;
        Some(
            HandledEncoderOutcome::default()
                .upstream(ChannelStripMsg::LowGain(0.0))
                .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledEncoderOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LowFreq(self.gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
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

pub struct LMFreqWidget {
    hw_idx: usize,
    view: WidgetView,

    freq: f32,
    q: f32,
}

impl Widget for LMFreqWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal("LMFreq")
                .line2_normal("Q")
                .line1_shift("LMFreq")
                .line2_shift("Q")
                .color_normal(color::DARK_BLUE)
                .color_shift(color::DARK_BLUE),
            freq: 0.5,
            q: 0.5,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LmFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.q = apply_accel(self.q, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LmQ(self.q))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            _ => HandledUpstreamOutcome::default(),
        }
    }
}

pub struct LMGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
}

impl Widget for LMGainWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal("LMGain")
                .line2_normal("zero")
                .line1_shift("LMGain")
                .line2_shift("zero")
                .color_normal(color::DARK_BLUE)
                .color_shift(color::DARK_BLUE),
            gain: 0.0,
        }
    }

    fn on_click(&mut self) -> Option<HandledEncoderOutcome> {
        self.gain = 0.0;
        Some(
            HandledEncoderOutcome::default()
                .upstream(ChannelStripMsg::LmGain(0.0))
                .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledEncoderOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::LmGain(self.gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            _ => HandledUpstreamOutcome::default(),
        }
    }
}

pub struct HMGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
}

impl Widget for HMGainWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal("HMGain")
                .line2_normal("zero")
                .line1_shift("HMGain")
                .line2_shift("zero")
                .color_normal(color::GREEN)
                .color_shift(color::GREEN),
            gain: 0.0,
        }
    }

    fn on_click(&mut self) -> Option<HandledEncoderOutcome> {
        self.gain = 0.0;
        Some(
            HandledEncoderOutcome::default()
                .upstream(ChannelStripMsg::HmGain(0.0))
                .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledEncoderOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::HmGain(self.gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            _ => HandledUpstreamOutcome::default(),
        }
    }
}

pub struct HMFreqWidget {
    hw_idx: usize,
    view: WidgetView,

    freq: f32,
    q: f32,
}

impl Widget for HMFreqWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal("Hi Freq")
                .line2_normal("Q")
                .line1_shift("Hi Mode")
                .line2_shift("Q")
                .color_normal(color::RED)
                .color_shift(color::RED),
            freq: 0.5,
            q: 0.5,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledEncoderOutcome> {
        match self.view.mode {
            //TODO:
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::HmFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.q = apply_accel(self.q, turn);
                Some(
                    HandledEncoderOutcome::default()
                        .upstream(ChannelStripMsg::HmQ(self.q))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            _ => HandledUpstreamOutcome::default(),
        }
    }
}
