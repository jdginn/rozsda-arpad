use crate::midi::v1m::EncoderRingMode::{FromCenter, FromLeft};
use crate::modes::color;
use crate::modes::color::RgbColor;
use crate::modes::reaper_channel_strip_router::{
    BandMode, ChannelStripMsg, CompBypass, CompMsg, CompOrder, CompType, EqBypass, EqMsg,
    EqPosition, EqType, GainMsg, SaturationBypass, SaturationMsg, SaturationType, TrimMsg,
};
use crate::modes::widget;
use crate::modes::widget::{
    Dirty, EncoderTurn, Widget, WidgetMode, WidgetView, apply_accel, apply_accel_center,
    encoder_ring_msg,
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

#[derive(Clone, Debug, Default)]
pub struct ChannelStripSnapshot {
    pub eq_enabled: Option<bool>,
    pub eq_type: Option<EqType>,
    pub eq_pos: Option<EqPosition>,
    pub low_mode: Option<BandMode>,
    pub high_mode: Option<BandMode>,
    pub comp1_enabled: Option<bool>,
    pub comp2_enabled: Option<bool>,
    pub comp1_type: Option<CompType>,
    pub comp2_type: Option<CompType>,
    pub comp_1_sidechain_freq: Option<f32>,
    pub comp_2_sidechain_freq: Option<f32>,
    pub comp_order: Option<CompOrder>,
}

// ----------------------
// Individual Widget implementations
// ----------------------

pub type ChannelStripWidget =
    dyn Widget<UpstreamMsg = ChannelStripMsg, Snapshot = ChannelStripSnapshot>;

pub type DownstreamOutcome =
    widget::HandledDownstreamOutcome<ChannelStripMsg, ChannelStripSnapshot>;
pub type UpstreamOutcome = widget::HandledUpstreamOutcome;

pub struct HpfWidget {
    hw_idx: usize,
    view: WidgetView,
    freq: f32,
    slope: f32,
    eq_type: EqType,
}

impl Widget for HpfWidget {
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
                .line1_disabled(DEFAULT_EQ_TYPE.as_str())
                .line2_disabled("EnablEQ")
                .line1_normal("HP Filter")
                .line2_normal("DisblEQ")
                .line1_shift("EQ Type")
                .line2_shift(DEFAULT_EQ_TYPE.as_str())
                .color_normal(color::DARK_BROWN)
                .color_shift(color::DARK_BROWN),
            freq: 0.1,
            slope: 0.0, // TODO: how do we represent this?
            eq_type: DEFAULT_EQ_TYPE,
        }
    }
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        if let Some(eq_type) = snapshot.eq_type {
            self.eq_type = eq_type;
            self.view.line2_disabled = eq_type.as_str().to_string();
            self.view.line2_shift = eq_type.as_str().to_string();
        }
        None
    }
    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => Some(
                DownstreamOutcome::default()
                    .upstream(ChannelStripMsg::EnableEq(self.eq_type))
                    .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
                    .snapshot(ChannelStripSnapshot {
                        eq_enabled: Some(true),
                        ..Default::default()
                    }),
            ),
            WidgetMode::Normal | WidgetMode::Shift => Some(
                DownstreamOutcome::default()
                    .upstream(ChannelStripMsg::DisableEq)
                    .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
                    .snapshot(ChannelStripSnapshot {
                        eq_enabled: Some(false),
                        ..Default::default()
                    }),
            ),
            _ => None,
        }
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => {
                self.eq_type = self.eq_type.step(turn);
                println!("Stepping to eq type: {:?}", self.eq_type);
                self.view.line2_shift = self.eq_type.as_str().to_string();
                self.view.line1_disabled = self.eq_type.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::EqType(self.eq_type).into())
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Normal => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HpfFreq(self.freq).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.slope = apply_accel(self.slope, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HpfSlope(self.slope).into())
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.eq_type = self.eq_type.step(turn);
                self.view.line2_shift = self.eq_type.as_str().to_string();
                self.view.line2_disabled = self.eq_type.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::EqType(self.eq_type).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(eq_type) => {
                self.view.mode = WidgetMode::Normal;
                self.eq_type = eq_type;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::HpfFreq(freq) => {
                    self.freq = freq;
                    UpstreamOutcome::default()
                        .dirty(Dirty::LINE1)
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                }
                EqMsg::HpfSlope(slope) => {
                    self.slope = slope;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                EqMsg::EqType(eq_type) => {
                    self.eq_type = eq_type;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LowFreq(self.freq).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.q = apply_accel(self.q, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LowSlope(self.q).into())
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.band_mode = self.band_mode.step(turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LowBandMode(self.band_mode).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        println!("LowFreqWidget: handle_message_from_upstream: {:?}", msg);
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::LowFreq(freq) => {
                    self.freq = freq;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.freq,
                        FromLeft,
                    ))
                }
                EqMsg::LowSlope(q) => {
                    self.q = q;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                EqMsg::LowBandMode(band_mode) => {
                    self.band_mode = band_mode;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
        }
        // TODO
    }

    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            _ => match self.band_mode {
                BandMode::Shelf => format!("{} Q", self.q),
                BandMode::Bell => "".to_string(),
            },
        }
    }
}

pub struct LowGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
}

impl Widget for LowGainWidget {
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        self.gain = 0.5;
        Some(
            DownstreamOutcome::default()
                .upstream(EqMsg::LowGain(self.gain).into())
                .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<DownstreamOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LowGain(self.gain).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::LowGain(gain) => {
                    self.gain = gain;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.gain,
                        FromCenter,
                    ))
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LmFreq(self.freq).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.q = apply_accel(self.q, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LmQ(self.q).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::LmFreq(freq) => {
                    self.freq = freq;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.freq,
                        FromLeft,
                    ))
                }
                EqMsg::LmQ(q) => {
                    self.q = q;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
        }
    }

    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            _ => format!("{} Q", self.q),
        }
    }
}

pub struct LMGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
}

impl Widget for LMGainWidget {
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        self.gain = 0.5;
        Some(
            DownstreamOutcome::default()
                .upstream(EqMsg::LmGain(self.gain).into())
                .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn on_shift_click(&mut self) -> Option<DownstreamOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::LmGain(self.gain).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::LmGain(gain) => {
                    self.gain = gain;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.gain,
                        FromCenter,
                    ))
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.freq = apply_accel(self.freq, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HmFreq(self.freq).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.q = apply_accel(self.q, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HmQ(self.q).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::HmFreq(freq) => {
                    self.freq = freq;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.freq,
                        FromLeft,
                    ))
                }
                EqMsg::HmQ(q) => {
                    self.q = q;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
        }
    }

    fn line2_text(&self) -> String {
        match self.view.mode {
            WidgetMode::Disabled => String::new(),
            _ => format!("{} Q", self.q),
        }
    }
}

pub struct HMGainWidget {
    hw_idx: usize,
    view: WidgetView,

    gain: f32,
}

impl Widget for HMGainWidget {
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        self.gain = 0.5;
        Some(
            DownstreamOutcome::default()
                .upstream(EqMsg::HmGain(0.0).into())
                .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<DownstreamOutcome> {
        self.on_click()
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HmGain(self.gain).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::HmGain(gain) => {
                    self.gain = gain;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.gain,
                        FromCenter,
                    ))
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Press => {
                self.freq = apply_accel(self.freq, turn);
                match self.band_mode {
                    BandMode::Shelf => self.view.line2_normal = format!("{} Q", self.q),
                    BandMode::Bell => self.view.line2_normal = String::new(),
                }
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HighFreq(self.freq).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.freq, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.band_mode = self.band_mode.step(turn);
                self.view.line2_shift = self.band_mode.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HighBandMode(self.band_mode).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::Eq(msg) => match msg {
                EqMsg::HighFreq(freq) => {
                    self.freq = freq;
                    UpstreamOutcome::default().downstream(encoder_ring_msg(
                        self.hw_idx,
                        self.freq,
                        FromLeft,
                    ))
                }
                EqMsg::HighQ(q) => {
                    self.q = q;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                EqMsg::HighBandMode(band_mode) => {
                    self.band_mode = band_mode;
                    UpstreamOutcome::default().dirty(Dirty::LINE2)
                }
                _ => UpstreamOutcome::default(),
            },
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if snapshot.eq_enabled == Some(true) {
            self.view.mode = WidgetMode::Normal;
        } else {
            self.view.mode = WidgetMode::Disabled;
        }
        None
    }

    // NOTE: I don't think we actually need this, since we trust upstream to give us a repr of the
    // actual value it has set for whatever mode we're in
    // fn handle_snapshot(
    //     &mut self,
    //     snapshot: &ChannelStripSnapshot,
    // ) -> Option<DownstreamOutcome> {
    //     if let Some(band_mode) = snapshot.high_mode
    //         && band_mode != self.band_mode
    //     {
    //         self.band_mode = band_mode;
    //         return Some(
    //             DownstreamOutcome::default()
    //                 .dirty(Dirty::LINE2)
    //                 .downstream(encoder_ring_msg(self.hw_idx, 0.0, FromCenter)),
    //         );
    //     }
    //     None
    // }

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        self.gain = 0.5;
        Some(
            DownstreamOutcome::default()
                .upstream(EqMsg::HighGain(self.gain).into())
                .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                .dirty(Dirty::LINE1),
        )
    }

    fn on_shift_click(&mut self) -> Option<DownstreamOutcome> {
        self.sides_gain = 0.5;
        Some(
            DownstreamOutcome::default()
                .upstream(EqMsg::HighSidesGain(self.sides_gain).into())
                .downstream(encoder_ring_msg(self.hw_idx, self.sides_gain, FromCenter))
                .dirty(Dirty::LINE2),
        )
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HighGain(self.gain).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.sides_gain = apply_accel_center(self.sides_gain, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(EqMsg::HighSidesGain(self.sides_gain).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.sides_gain, FromCenter))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableEq(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableEq => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            _ => UpstreamOutcome::default(),
        }
    }
}

pub struct EqPosWidget {
    hw_idx: usize,
    view: WidgetView,

    eq_pos: EqPosition,
    eq_in: EqBypass,

    comp_order: CompOrder,

    eq_type: EqType,
}

impl Widget for EqPosWidget {
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
        const DEFAULT_EQ_TYPE: EqType = EqType::Digital;
        Self {
            hw_idx,
            view: WidgetView::new()
                .line1_normal(DEFAULT_EQ_POS.as_str())
                .line2_normal(DEFAULT_EQ_IN.as_str())
                .line1_shift("CmpOrdr")
                .line2_shift(DEFAULT_COMP_ORDER.as_str())
                .color_normal(color::BLACK)
                .color_shift(color::BLACK)
                .starting_mode(WidgetMode::Normal),
            eq_pos: DEFAULT_EQ_POS,
            eq_in: DEFAULT_EQ_IN,
            comp_order: DEFAULT_COMP_ORDER,
            eq_type: DEFAULT_EQ_TYPE,
        }
    }

    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if let Some(eq_type) = snapshot.eq_type {
            self.eq_type = eq_type;
        }
        None
    }

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        self.eq_in.next();
        self.view.line2_normal = self.eq_in.as_str().to_string();
        Some(match self.eq_in {
            EqBypass::IN => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableEq(self.eq_type))
                .dirty(Dirty::LINE2),
            EqBypass::OUT => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableEq)
                .dirty(Dirty::LINE2),
        })
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Press => {
                self.eq_pos = self.eq_pos.step(turn);
                self.view.line1_normal = self.eq_pos.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(ChannelStripMsg::EqPos(self.eq_pos))
                        .dirty(Dirty::LINE1 | Dirty::LINE2),
                )
            }
            WidgetMode::Shift | WidgetMode::ShiftPress => {
                self.comp_order = self.comp_order.step(turn);
                self.view.line2_shift = self.comp_order.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompOrder(self.comp_order))
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            _ => UpstreamOutcome::default(),
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

    comp1_enabled: bool,
    comp2_enabled: bool,
    comp1_thresh: f32,
    comp1_attack: f32,
    comp2_thresh: f32,
    comp2_attack: f32,

    comp1_type: CompType,
    comp2_type: CompType,
}

impl Widget for CompThreshWidget {
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
                .line1_disabled(DEFAULT_COMP1_TYPE.as_str())
                .line2_disabled("EblCmp1")
                .line1_normal("CompThr")
                .line2_normal("20ms") //TODO:
                .line1_shift("Cmp2Thr")
                .line2_shift("20ms") //TODO:
                .color_normal(COMP1_COLOR)
                .color_shift(COMP2_COLOR),
            comp1_enabled: false,
            comp2_enabled: false,
            comp1_thresh: 0.5,
            comp1_attack: 0.5,
            comp2_thresh: 0.5,
            comp2_attack: 0.5,
            comp1_type: DEFAULT_COMP1_TYPE,
            comp2_type: DEFAULT_COMP2_TYPE,
        }
    }

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => {
                self.comp1_enabled = true;
                self.view.mode = WidgetMode::Normal;
                Some(
                    DownstreamOutcome::default()
                        .upstream(ChannelStripMsg::EnableComp1(self.comp1_type))
                        .snapshot(ChannelStripSnapshot {
                            comp1_enabled: Some(self.comp1_enabled),
                            ..Default::default()
                        })
                        .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR),
                )
            }
            _ => None,
        }
    }

    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if let Some(comp1_enabled) = snapshot.comp1_enabled {
            self.comp1_enabled = comp1_enabled;
            if self.view.mode == WidgetMode::Disabled && self.comp1_enabled {
                self.view.mode = WidgetMode::Normal;
            }
        }
        if let Some(comp2_enabled) = snapshot.comp2_enabled {
            self.comp2_enabled = comp2_enabled;
        }
        if let Some(comp1_type) = snapshot.comp1_type {
            self.comp1_type = comp1_type;
        }
        if let Some(comp2_type) = snapshot.comp2_type {
            self.comp2_type = comp2_type;
        }

        // TODO: may need to change how we display Atk?

        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_thresh = apply_accel(self.comp1_thresh, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::CompThresh(self.comp1_thresh).into()),
                )
            }
            WidgetMode::Press => {
                self.comp1_attack = apply_accel(self.comp1_attack, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::CompAttack(self.comp1_attack).into())
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift => {
                self.comp2_thresh = apply_accel(self.comp2_thresh, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::Comp2Thresh(self.comp2_thresh).into()),
                )
            }
            WidgetMode::ShiftPress => {
                self.comp2_attack = apply_accel(self.comp2_attack, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::Comp2Attack(self.comp2_attack).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableComp1(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp1 => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            // TODO: need to do something about this but it gets a little tricky
            ChannelStripMsg::EnableComp2(_) => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp2 => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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

    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if let Some(comp1_enabled) = snapshot.comp1_enabled
            && self.view.mode == WidgetMode::Disabled
            && comp1_enabled
        {
            self.view.mode = WidgetMode::Normal;
        }
        if let Some(comp1_type) = snapshot.comp1_type {
            self.comp1_type = comp1_type;
        }
        if let Some(comp2_type) = snapshot.comp2_type {
            self.comp2_type = comp2_type;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_ratio = apply_accel(self.comp1_ratio, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::CompRatio(self.comp1_ratio).into()),
                )
            }
            WidgetMode::Press => {
                self.comp1_release = apply_accel(self.comp1_release, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::CompRelease(self.comp1_release).into())
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift => {
                self.comp2_ratio = apply_accel(self.comp2_ratio, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::Comp2Ratio(self.comp2_ratio).into()),
                )
            }
            WidgetMode::ShiftPress => {
                self.comp2_release = apply_accel(self.comp2_release, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::Comp2Release(self.comp2_release).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableComp1(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp1 => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            // TODO: need to do something about this but it gets a little tricky
            ChannelStripMsg::EnableComp2(_) => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp2 => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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

    fn handle_snapshot(&mut self, snapshot: &ChannelStripSnapshot) -> Option<DownstreamOutcome> {
        if let Some(comp1_enabled) = snapshot.comp1_enabled
            && self.view.mode == WidgetMode::Disabled
            && comp1_enabled
        {
            self.view.mode = WidgetMode::Normal;
        }
        if let Some(comp1_type) = snapshot.comp1_type {
            self.comp1_type = comp1_type;
        }
        if let Some(comp2_type) = snapshot.comp2_type {
            self.comp2_type = comp2_type;
        }
        None
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_makeup = apply_accel_center(self.comp1_makeup, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::CompMakeup(self.comp1_makeup).into()),
                )
            }
            WidgetMode::Press => {
                self.comp1_sidechain_freq = apply_accel(self.comp1_sidechain_freq, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::CompScFilter(self.comp1_sidechain_freq).into())
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::Shift => {
                self.comp2_makeup = apply_accel_center(self.comp2_makeup, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::Comp2Makeup(self.comp2_makeup).into()),
                )
            }
            WidgetMode::ShiftPress => {
                self.comp2_sidechain_freq = apply_accel(self.comp2_sidechain_freq, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(CompMsg::Comp2ScFilter(self.comp2_sidechain_freq).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableComp1(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp1 => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            // TODO: need to do something about this but it gets a little tricky
            ChannelStripMsg::EnableComp2(_) => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp2 => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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

    fn on_click(&mut self) -> Option<DownstreamOutcome> {
        self.comp1_in.next();
        self.view.line2_normal = self.comp1_in.as_str().to_string();
        Some(match self.comp1_in {
            CompBypass::IN => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableComp1(self.comp1_type))
                .dirty(Dirty::LINE2),
            CompBypass::OUT => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableComp1)
                .dirty(Dirty::LINE2),
        })
    }

    fn on_shift_click(&mut self) -> Option<DownstreamOutcome> {
        self.comp2_in.next();
        self.view.line2_shift = self.comp2_in.as_str().to_string();
        Some(match self.comp2_in {
            CompBypass::IN => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableComp2(self.comp2_type))
                .dirty(Dirty::LINE2),
            CompBypass::OUT => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableComp2)
                .dirty(Dirty::LINE2),
        })
    }

    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal => {
                self.comp1_type = self.comp1_type.step(turn);
                self.view.line1_normal = self.comp1_type.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(ChannelStripMsg::CompType(self.comp1_type))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Shift => {
                self.comp2_type = self.comp2_type.step(turn);
                self.view.line1_shift = self.comp2_type.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(ChannelStripMsg::Comp2Type(self.comp2_type))
                        .dirty(Dirty::LINE1),
                )
            }
            _ => None,
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableComp1(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp1 => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            // TODO: need to do something about this but it gets a little tricky
            ChannelStripMsg::EnableComp2(_) => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableComp2 => {
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
    fn on_shift_click(&mut self) -> Option<DownstreamOutcome> {
        self.sat_in.next();
        self.view.line2_normal = self.sat_in.as_str().to_string();
        Some(match self.sat_in {
            SaturationBypass::IN => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::EnableSaturation(self.sat_type))
                .dirty(Dirty::LINE2),
            SaturationBypass::OUT => DownstreamOutcome::default()
                .upstream(ChannelStripMsg::DisableSaturation)
                .dirty(Dirty::LINE2),
        })
    }
    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.sat_drive = apply_accel_center(self.sat_drive, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(SaturationMsg::Saturation(self.sat_drive).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.sat_drive, FromLeft))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press => {
                self.sat_type = self.sat_type.step(turn);
                self.view.line2_normal = self.sat_type.as_str().to_string();
                Some(
                    DownstreamOutcome::default()
                        .upstream(ChannelStripMsg::SaturationType(self.sat_type))
                        .dirty(Dirty::LINE2),
                )
            }
            WidgetMode::ShiftPress => None,
        }
    }
    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            ChannelStripMsg::EnableSaturation(_) => {
                self.view.mode = WidgetMode::Normal;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            ChannelStripMsg::DisableSaturation => {
                self.view.mode = WidgetMode::Disabled;
                UpstreamOutcome::default().dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR)
            }
            _ => UpstreamOutcome::default(),
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
    type UpstreamMsg = ChannelStripMsg;
    type Snapshot = ChannelStripSnapshot;

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
                .color_shift(color::BLACK)
                .starting_mode(WidgetMode::Normal),
            gain: 0.5,
            interface_gain: 0.5,
            trim: 0.5,
        }
    }

    // TODO:
    fn handle_encoder_turn(&mut self, turn: EncoderTurn) -> Option<DownstreamOutcome> {
        match self.view.mode {
            WidgetMode::Disabled => None,
            WidgetMode::Normal | WidgetMode::Shift => {
                self.gain = apply_accel_center(self.gain, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(GainMsg::Gain(self.gain).into())
                        .downstream(encoder_ring_msg(self.hw_idx, self.gain, FromCenter))
                        .dirty(Dirty::LINE1),
                )
            }
            WidgetMode::Press | WidgetMode::ShiftPress => {
                self.trim = apply_accel_center(self.trim, turn);
                Some(
                    DownstreamOutcome::default()
                        .upstream(TrimMsg::Trim(self.trim).into())
                        .dirty(Dirty::LINE2),
                )
            }
        }
    }

    fn handle_message_from_upstream(&mut self, msg: ChannelStripMsg) -> UpstreamOutcome {
        match msg {
            _ => UpstreamOutcome::default(),
        }
    }
}
