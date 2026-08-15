use crate::midi::v1m;
use crate::midi::v1m::EncoderRingMode::{FromCenter, FromLeft, Point, Width};
use crate::modes::color;
use crate::modes::color::RgbColor;
use crate::modes::reaper_channel_strip_router::{
    BandMode, ChannelStripMsg, CompBypass, CompOrder, CompType, EqBypass, EqPosition, EqType,
    SaturationBypass, SaturationType,
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

#[derive(Clone, Debug, Default)]
pub struct HandledUpstreamOutcome {
    pub downstream_msgs: Vec<v1m::DownstreamMsg>,
    pub dirty: Dirty,
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

#[derive(Clone, Debug, Default)]
pub struct ChannelStripSnapshot {
    pub eq_type: Option<EqType>,
    pub eq_pos: Option<EqPosition>,
    pub low_mode: Option<BandMode>,
    pub high_mode: Option<BandMode>,
    pub comp1_type: Option<CompType>,
    pub comp2_type: Option<CompType>,
    pub comp_1_sidechain_freq: Option<f32>,
    pub comp_2_sidechain_freq: Option<f32>,
    pub comp_order: Option<CompOrder>,
}

#[derive(Clone, Debug, Default)]
pub struct HandledDownstreamOutcome {
    pub upstream_msgs: Vec<ChannelStripMsg>,
    pub downstream_msgs: Vec<v1m::DownstreamMsg>,
    pub dirty: Dirty,
    pub snapshot: Option<ChannelStripSnapshot>,
}

impl HandledDownstreamOutcome {
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

    fn handle_snapshot(
        &mut self,
        snapshot: &ChannelStripSnapshot,
    ) -> Option<HandledDownstreamOutcome> {
        let _ = snapshot;
        None
    }
    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome;

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        None
    }

    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        None
    }

    fn handle_shift_press(&mut self) -> Option<HandledDownstreamOutcome> {
        match self.view().mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.view_mut().mode = WidgetMode::Shift;
                Some(
                    HandledDownstreamOutcome::default()
                        .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR),
                )
            }
            WidgetMode::Press => {
                self.view_mut().mode = WidgetMode::ShiftPress;
                Some(
                    HandledDownstreamOutcome::default()
                        .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR),
                )
            }
            WidgetMode::Shift => None,
            WidgetMode::ShiftPress => None,
        }
    }

    fn handle_shift_release(&mut self) -> Option<HandledDownstreamOutcome> {
        match self.view().mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => None,
            WidgetMode::Press => None,
            WidgetMode::Shift => {
                self.view_mut().mode = WidgetMode::Normal;
                Some(
                    HandledDownstreamOutcome::default()
                        .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR),
                )
            }
            WidgetMode::ShiftPress => {
                self.view_mut().mode = WidgetMode::Press;
                Some(
                    HandledDownstreamOutcome::default()
                        .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR),
                )
            }
        }
    }

    fn handle_encoder_click(&mut self) -> Option<HandledDownstreamOutcome> {
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
                        Some(HandledDownstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    }
                }
                WidgetMode::Press => None,
                WidgetMode::Shift => {
                    if let Some(outcome) = self.on_shift_click() {
                        Some(outcome)
                    } else {
                        self.view_mut().mode = WidgetMode::ShiftPress;
                        self.view_mut().in_hold_mode = true;
                        Some(HandledDownstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
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
                        Some(HandledDownstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    } else {
                        None
                    }
                }
                WidgetMode::Shift => None,
                WidgetMode::ShiftPress => {
                    if self.view_mut().in_hold_mode {
                        self.view_mut().in_hold_mode = false;
                        self.view_mut().mode = WidgetMode::Shift;
                        Some(HandledDownstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2))
                    } else {
                        None
                    }
                }
            };
            self.view_mut().encoder_pressed = false;
            outcome
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
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
            1 => 0.01,
            2 => 0.05,
            3 => 0.1,
            4 => 0.2,
            _ => 0.2,
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
            1 => 0.01,
            2 => 0.05,
            3 => 0.1,
            4 => 0.2,
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

// TODO: IMPORTANT: for Disabled mode, clicking encoder should instantiate the widget and set to
// normal.

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

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HpfFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.slope = apply_accel(self.slope, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HpfSlope(self.slope))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.eq_type = self.eq_type.step(turn);
                self.view.line2_shift = self.eq_type.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
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

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::LowFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.q = apply_accel(self.q, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::LowSlope(self.q))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.band_mode = self.band_mode.step(turn);
                self.view.line2_shift = self.band_mode.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::LowBandMode(self.band_mode))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        println!("LowFreqWidget: handle_message_from_upstream: {:?}", msg);
        match msg {
            ChannelStripMsg::LowFreq(msg) => {
                println!("Got LowFreq msg");
                self.freq = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.freq,
                    FromLeft,
                ))
            }
            ChannelStripMsg::LowQ(msg) => {
                self.q = msg;
                HandledUpstreamOutcome::default().dirty(Dirty::LINE2)
            }
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

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.gain = 0.5;
        Some(
            HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::LowGain(self.gain))
                .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::LowGain(self.gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            ChannelStripMsg::LowGain(msg) => {
                println!("Gain: {}", msg);
                self.gain = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.gain,
                    FromCenter,
                ))
            }
            _ => HandledUpstreamOutcome::default(),
        }
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

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::LmFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.q = apply_accel(self.q, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::LmQ(self.q))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            ChannelStripMsg::LmFreq(msg) => {
                self.freq = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.freq,
                    FromLeft,
                ))
            }
            ChannelStripMsg::LmQ(msg) => {
                self.q = msg;
                HandledUpstreamOutcome::default().dirty(Dirty::LINE2)
            }
            _ => HandledUpstreamOutcome::default(),
        }
    }

    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Normal | WidgetMode::Press => format!("{} Q", self.q),
            WidgetMode::Shift | WidgetMode::ShiftPress => self.view.line2_shift.clone(),
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

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.gain = 0.5;
        Some(
            HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::LmGain(self.gain))
                .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledDownstreamOutcome::default()
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
            ChannelStripMsg::LmGain(msg) => {
                self.gain = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.gain,
                    FromCenter,
                ))
            }
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
                .line1_normal("HMFreq")
                .line2_normal("Q")
                .line1_shift("HMFreq")
                .line2_shift("Q")
                .color_normal(color::GREEN)
                .color_shift(color::GREEN),
            freq: 0.5,
            q: 0.5,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HmFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.q = apply_accel(self.q, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HmQ(self.q))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            ChannelStripMsg::HmFreq(msg) => {
                self.freq = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.freq,
                    FromLeft,
                ))
            }
            ChannelStripMsg::HmQ(msg) => {
                self.q = msg;
                HandledUpstreamOutcome::default().dirty(Dirty::LINE2)
            }
            _ => HandledUpstreamOutcome::default(),
        }
    }

    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Normal | WidgetMode::Press => format!("{} Q", self.q),
            WidgetMode::Shift | WidgetMode::ShiftPress => self.view.line2_shift.clone(),
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

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.gain = 0.0;
        Some(
            HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::HmGain(0.0))
                .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledDownstreamOutcome::default()
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
            ChannelStripMsg::HmGain(msg) => {
                self.gain = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.gain,
                    FromCenter,
                ))
            }
            _ => HandledUpstreamOutcome::default(),
        }
    }
}

pub struct HiFreqWidget {
    hw_idx: usize,
    view: WidgetView,

    freq: f32,
    q: f32,
    band_mode: BandMode,
}

impl Widget for HiFreqWidget {
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
                .line1_normal("Hi Freq")
                .line2_normal("Q")
                .line1_shift("Hi Mode")
                .line2_shift(DEFAULT_BAND_MODE.as_str())
                .color_normal(color::RED)
                .color_shift(color::RED),
            freq: 0.5,
            q: 0.5,
            band_mode: DEFAULT_BAND_MODE,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            //TODO:
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Press => {
                self.freq = apply_accel(self.freq, turn);
                match self.band_mode {
                    BandMode::Shelf => self.view.line2_normal = format!("{} Q", self.q),
                    BandMode::Bell => self.view.line2_normal = String::new(),
                }
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HighFreq(self.freq))
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.band_mode = self.band_mode.step(turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HighBandMode(self.band_mode))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            ChannelStripMsg::HighFreq(msg) => {
                self.freq = msg;
                HandledUpstreamOutcome::default().downstream(encoder_ring_msg(
                    self.hw_idx,
                    self.freq,
                    FromLeft,
                ))
            }
            ChannelStripMsg::HighQ(msg) => {
                self.q = msg;
                HandledUpstreamOutcome::default().dirty(Dirty::LINE2)
            }
            _ => HandledUpstreamOutcome::default(),
        }
    }
    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            WidgetMode::Normal | WidgetMode::Press => format!("{} Q", self.q),
            WidgetMode::Shift | WidgetMode::ShiftPress => self.view.line2_shift.clone(),
        }
    }
}

pub struct HiGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
    sides_gain: f32,

    band_mode: BandMode,
}

impl Widget for HiGainWidget {
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
                .line1_normal("Hi Gain")
                .line2_normal("zero")
                .line1_shift("HiSides")
                .line2_shift("zero")
                .color_normal(color::RED)
                .color_shift(color::PINK),
            gain: 0.0,
            sides_gain: 0.0,
            band_mode: BandMode::Bell,
        }
    }

    // NOTE: I don't think we actually need this, since we trust upstream to give us a repr of the
    // actual value it has set for whatever mode we're in
    // fn handle_snapshot(
    //     &mut self,
    //     snapshot: &ChannelStripSnapshot,
    // ) -> Option<HandledDownstreamOutcome> {
    //     if let Some(band_mode) = snapshot.high_mode
    //         && band_mode != self.band_mode
    //     {
    //         self.band_mode = band_mode;
    //         return Some(
    //             HandledDownstreamOutcome::default()
    //                 .dirty(Dirty::LINE2)
    //                 .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter)),
    //         );
    //     }
    //     None
    // }

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.gain = 0.5;
        Some(
            HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::HighGain(self.gain))
                .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.sides_gain = 0.5;
        Some(
            HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::HighSidesGain(self.sides_gain))
                .downstream(encoder_ring_msg(self.hw_idx, self.sides_gain, FromCenter))
                .dirty(Dirty::LINE2),
        )
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HighGain(self.gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.sides_gain = apply_accel_center(self.sides_gain, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::HighSidesGain(self.sides_gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.sides_gain, FromCenter))
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

pub struct EqPosWidget {
    hw_idx: usize,
    view: WidgetView,

    eq_pos: EqPosition,
    eq_in: EqBypass,

    comp_order: CompOrder,
}

impl Widget for EqPosWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        const DEFAULT_EQ_POS: EqPosition = EqPosition::First;
        const DEFAULT_EQ_IN: EqBypass = EqBypass::IN;
        const DEFAULT_COMP_ORDER: CompOrder = CompOrder::C1toC2;
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal(DEFAULT_EQ_POS.as_str())
                .line2_normal(DEFAULT_EQ_IN.as_str())
                .line1_shift("CmpOrdr")
                .line2_shift(DEFAULT_COMP_ORDER.as_str())
                .color_normal(color::BLACK)
                .color_shift(color::BLACK),
            eq_pos: DEFAULT_EQ_POS,
            eq_in: DEFAULT_EQ_IN,
            comp_order: DEFAULT_COMP_ORDER,
        }
    }

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.eq_in.next();
        self.view.line2_normal = self.eq_in.as_str().to_string();
        Some(match self.eq_in {
            EqBypass::IN => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableEq)
                .dirty(Dirty::LINE2),
            EqBypass::OUT => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableEq)
                .dirty(Dirty::LINE2),
        })
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Press => {
                self.eq_pos = self.eq_pos.step(turn);
                self.view.line1_normal = self.eq_pos.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::EqPos(self.eq_pos))
                        .dirty(Dirty::LINE1 | Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.comp_order = self.comp_order.step(turn);
                self.view.line2_shift = self.comp_order.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompOrder(self.comp_order))
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

const COMP1_COLOR: RgbColor = color::AMBER;
const COMP2_COLOR: RgbColor = color::WHITE;
const DEFAULT_COMP1_TYPE: CompType = CompType::Eleven76;
const DEFAULT_COMP2_TYPE: CompType = CompType::LA2A;

pub struct CompThreshWidget {
    hw_idx: usize,
    view: WidgetView,

    comp1_thresh: f32,
    comp1_attack: f32,
    comp2_thresh: f32,
    comp2_attack: f32,

    comp1_type: CompType,
    comp2_type: CompType,
}

impl Widget for CompThreshWidget {
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
                .line1_normal("CompThr")
                .line2_normal("20ms") //TODO:
                .line1_shift("Cmp2Thr")
                .line2_shift("20ms") //TODO:
                .color_normal(COMP1_COLOR)
                .color_shift(COMP2_COLOR),
            comp1_thresh: 0.5,
            comp1_attack: 0.5,
            comp2_thresh: 0.5,
            comp2_attack: 0.5,
            comp1_type: DEFAULT_COMP1_TYPE,
            comp2_type: DEFAULT_COMP2_TYPE,
        }
    }

    fn handle_snapshot(
        &mut self,
        snapshot: &ChannelStripSnapshot,
    ) -> Option<HandledDownstreamOutcome> {
        if let Some(comp1_type) = snapshot.comp1_type {
            self.comp1_type = comp1_type;
        }
        if let Some(comp2_type) = snapshot.comp2_type {
            self.comp2_type = comp2_type;
        }

        // TODO: may need to change how we display Atk?

        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_thresh = apply_accel(self.comp1_thresh, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompThresh(self.comp1_thresh)),
                )
            }
            WidgetMode::Press => {
                self.comp1_attack = apply_accel(self.comp1_attack, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompAttack(self.comp1_attack))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift => {
                self.comp2_thresh = apply_accel(self.comp2_thresh, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Thresh(self.comp2_thresh)),
                )
            }
            WidgetMode::ShiftPress => {
                self.comp2_attack = apply_accel(self.comp2_attack, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Attack(self.comp2_attack))
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

pub struct CompRatWidget {
    hw_idx: usize,
    view: WidgetView,

    comp1_ratio: f32,
    comp2_ratio: f32,
    comp1_release: f32,
    comp2_release: f32,

    comp1_type: CompType,
    comp2_type: CompType,
}

impl Widget for CompRatWidget {
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
                .line1_normal("CompRat")
                .line2_normal("200ms") //TODO:
                .line1_shift("Cmp2Rat")
                .line2_shift("200ms") //TODO:
                .color_normal(COMP1_COLOR)
                .color_shift(COMP2_COLOR),
            comp1_ratio: 0.5,
            comp2_ratio: 0.5,
            comp1_release: 0.5,
            comp2_release: 0.5,
            comp1_type: DEFAULT_COMP1_TYPE,
            comp2_type: DEFAULT_COMP2_TYPE,
        }
    }

    fn handle_snapshot(
        &mut self,
        snapshot: &ChannelStripSnapshot,
    ) -> Option<HandledDownstreamOutcome> {
        if let Some(comp1_type) = snapshot.comp1_type {
            self.comp1_type = comp1_type;
        }
        if let Some(comp2_type) = snapshot.comp2_type {
            self.comp2_type = comp2_type;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_ratio = apply_accel(self.comp1_ratio, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompRatio(self.comp1_ratio)),
                )
            }
            WidgetMode::Press => {
                self.comp1_release = apply_accel(self.comp1_release, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompRelease(self.comp1_release))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift => {
                self.comp2_ratio = apply_accel(self.comp2_ratio, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Ratio(self.comp2_ratio)),
                )
            }
            WidgetMode::ShiftPress => {
                self.comp2_release = apply_accel(self.comp2_release, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Release(self.comp2_release))
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

pub struct CompMkpWidget {
    hw_idx: usize,
    view: WidgetView,

    comp1_makeup: f32,
    comp2_makeup: f32,
    comp1_sidechain_freq: f32,
    comp2_sidechain_freq: f32,

    comp1_type: CompType,
    comp2_type: CompType,
}

impl Widget for CompMkpWidget {
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
                .line1_normal("CompMkp")
                .line2_normal("SideFreq") //TODO:
                .line1_shift("Cmp2Mkp")
                .line2_shift("SideFreq") //TODO:
                .color_normal(COMP1_COLOR)
                .color_shift(COMP2_COLOR),
            comp1_makeup: 0.5,
            comp2_makeup: 0.5,
            comp1_sidechain_freq: 0.5,
            comp2_sidechain_freq: 0.5,
            comp1_type: DEFAULT_COMP1_TYPE,
            comp2_type: DEFAULT_COMP2_TYPE,
        }
    }

    fn handle_snapshot(
        &mut self,
        snapshot: &ChannelStripSnapshot,
    ) -> Option<HandledDownstreamOutcome> {
        if let Some(comp1_type) = snapshot.comp1_type {
            self.comp1_type = comp1_type;
        }
        if let Some(comp2_type) = snapshot.comp2_type {
            self.comp2_type = comp2_type;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_makeup = apply_accel_center(self.comp1_makeup, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompMakeup(self.comp1_makeup)),
                )
            }
            WidgetMode::Press => {
                self.comp1_sidechain_freq = apply_accel(self.comp1_sidechain_freq, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompScFilter(self.comp1_sidechain_freq))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift => {
                self.comp2_makeup = apply_accel_center(self.comp2_makeup, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Makeup(self.comp2_makeup)),
                )
            }
            WidgetMode::ShiftPress => {
                self.comp2_sidechain_freq = apply_accel(self.comp2_sidechain_freq, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2ScFilter(self.comp2_sidechain_freq))
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

pub struct CompTypeWidget {
    hw_idx: usize,
    view: WidgetView,

    comp1_type: CompType,
    comp2_type: CompType,
    comp1_in: CompBypass,
    comp2_in: CompBypass,
}

impl Widget for CompTypeWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        const DEFAULT_COMP1_BYPASS: CompBypass = CompBypass::IN;
        const DEFAULT_COMP2_BYPASS: CompBypass = CompBypass::OUT;
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal(DEFAULT_COMP1_TYPE.as_str())
                .line2_normal(DEFAULT_COMP1_BYPASS.as_str())
                .line1_shift(DEFAULT_COMP2_TYPE.as_str())
                .line2_shift(DEFAULT_COMP2_BYPASS.as_str())
                .color_normal(COMP1_COLOR)
                .color_shift(COMP2_COLOR),
            comp1_type: DEFAULT_COMP1_TYPE,
            comp2_type: DEFAULT_COMP2_TYPE,
            comp1_in: CompBypass::IN,
            comp2_in: CompBypass::IN,
        }
    }

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.comp1_in.next();
        self.view.line2_normal = self.comp1_in.as_str().to_string();
        Some(match self.comp1_in {
            CompBypass::IN => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableComp1)
                .dirty(Dirty::LINE2),
            CompBypass::OUT => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableComp1)
                .dirty(Dirty::LINE2),
        })
    }

    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.comp2_in.next();
        self.view.line2_shift = self.comp2_in.as_str().to_string();
        Some(match self.comp2_in {
            CompBypass::IN => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableComp2)
                .dirty(Dirty::LINE2),
            CompBypass::OUT => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableComp2)
                .dirty(Dirty::LINE2),
        })
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_type = self.comp1_type.step(turn);
                self.view.line1_normal = self.comp1_type.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompType(self.comp1_type))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Shift => {
                self.comp2_type = self.comp2_type.step(turn);
                self.view.line1_shift = self.comp2_type.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Type(self.comp2_type))
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

pub struct SatWidget {
    hw_idx: usize,
    view: WidgetView,

    sat_drive: f32,
    sat_in: SaturationBypass,
    sat_type: SaturationType,
}

impl Widget for SatWidget {
    fn view(&self) -> &WidgetView {
        &self.view
    }
    fn view_mut(&mut self) -> &mut WidgetView {
        &mut self.view
    }
    fn new(hw_idx: usize) -> Self {
        const DEFAULT_SAT_TYPE: SaturationType = SaturationType::Tape;
        const DEFAULT_SAT_IN: SaturationBypass = SaturationBypass::IN;
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal("Saturation")
                .line2_normal(DEFAULT_SAT_TYPE.as_str())
                .line1_shift("Saturation")
                .line2_shift(DEFAULT_SAT_IN.as_str())
                .color_normal(color::ORANGE)
                .color_shift(color::ORANGE),
            sat_drive: 0.5,
            sat_in: SaturationBypass::IN,
            sat_type: SaturationType::Tape,
        }
    }
    fn on_shift_click(&mut self) -> Option<HandledDownstreamOutcome> {
        self.sat_in.next();
        self.view.line2_normal = self.sat_in.as_str().to_string();
        Some(match self.sat_in {
            SaturationBypass::IN => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableSaturation)
                .dirty(Dirty::LINE2),
            SaturationBypass::OUT => HandledDownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableSaturation)
                .dirty(Dirty::LINE2),
        })
    }
    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.sat_drive = apply_accel_center(self.sat_drive, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Saturation(self.sat_drive))
                        .downstream(encoder_ring_msg(self.hw_idx, self.sat_drive, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.sat_type = self.sat_type.step(turn);
                self.view.line2_normal = self.sat_type.as_str().to_string();
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::SaturationType(self.sat_type))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::ShiftPress => None,
        }
    }
    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> HandledUpstreamOutcome {
        match msg {
            _ => HandledUpstreamOutcome::default(),
        }
    }
}

pub struct GainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
    interface_gain: f32,
    trim: f32,
}

impl Widget for GainWidget {
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
                .line1_normal("Gain")
                .line2_normal("Trim")
                .line1_shift("Gain")
                .line2_shift("Trim")
                .color_normal(color::BLACK)
                .color_shift(color::BLACK),
            gain: 0.5,
            interface_gain: 0.5,
            trim: 0.5,
        }
    }

    // TODO:
    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<HandledDownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Gain(self.gain))
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.trim = apply_accel_center(self.trim, turn);
                Some(
                    HandledDownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Trim(self.trim))
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
