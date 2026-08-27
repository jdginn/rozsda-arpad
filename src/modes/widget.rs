use crate::midi::v1m;
use crate::midi::v1m::EncoderRingMode::FromLeft;
use crate::modes::color::RgbColor;

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
    pub fn downstream(mut self, msg: v1m::DownstreamMsg) -> Self {
        self.downstream_msgs.push(msg);
        self
    }
    pub fn dirty(mut self, d: Dirty) -> Self {
        self.dirty |= d;
        self
    }
}

#[derive(Clone, Debug)]
pub struct HandledDownstreamOutcome<U, S> {
    pub upstream_msgs: Vec<U>,
    pub downstream_msgs: Vec<v1m::DownstreamMsg>,
    pub dirty: Dirty,
    pub snapshot: Option<S>,
}

impl<U, S> Default for HandledDownstreamOutcome<U, S> {
    fn default() -> Self {
        Self {
            upstream_msgs: Vec::new(),
            downstream_msgs: Vec::new(),
            dirty: Dirty::NONE,
            snapshot: None,
        }
    }
}

impl<U, S> HandledDownstreamOutcome<U, S> {
    pub fn upstream(mut self, msg: U) -> Self {
        self.upstream_msgs.push(msg);
        self
    }
    pub fn downstream(mut self, msg: v1m::DownstreamMsg) -> Self {
        self.downstream_msgs.push(msg);
        self
    }
    pub fn dirty(mut self, d: Dirty) -> Self {
        self.dirty |= d;
        self
    }
    pub fn snapshot(mut self, snapshot: S) -> Self {
        self.snapshot = Some(snapshot);
        self
    }
}

pub trait Widget {
    type UpstreamMsg;
    type Snapshot: Default;

    fn new(hw_idx: usize) -> Self
    where
        Self: Sized;

    fn view(&self) -> &WidgetView;
    fn view_mut(&mut self) -> &mut WidgetView;

    fn init(&self) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
        Some(
            HandledDownstreamOutcome::default()
                .downstream(v1m::DownstreamMsg::EncoderRingLED(v1m::EncoderRingMsg {
                    idx: self.view().mode as i32,
                    mode: FromLeft,
                    val: 0,
                }))
                .dirty(Dirty::LINE1 | Dirty::LINE2 | Dirty::COLOR),
        )
    }

    fn handle_snapshot(
        &mut self,
        snapshot: &Self::Snapshot,
    ) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
        let _ = snapshot;
        None
    }
    fn handle_message_from_upstream(&mut self, msg: Self::UpstreamMsg) -> HandledUpstreamOutcome {
        let _ = msg;
        HandledUpstreamOutcome::default()
    }

    fn on_click(&mut self) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
        None
    }

    fn on_shift_click(
        &mut self,
    ) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
        None
    }

    fn handle_shift_press(
        &mut self,
    ) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
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

    fn handle_shift_release(
        &mut self,
    ) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
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

    fn handle_encoder_click(
        &mut self,
    ) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
        if !self.view().encoder_pressed {
            let outcome = match self.view().mode {
                WidgetMode::Normal | WidgetMode::Disabled => {
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

    fn handle_encoder_turn(
        &mut self,
        turn: EncoderTurn,
    ) -> Option<HandledDownstreamOutcome<Self::UpstreamMsg, Self::Snapshot>> {
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
    pub mode: WidgetMode,

    pub encoder_pressed: bool,
    pub in_hold_mode: bool,

    pub line1_disabled: String,
    pub line1_normal: String,
    pub line1_shift: String,
    pub line2_disabled: String,
    pub line2_normal: String,
    pub line2_shift: String,

    pub color_disabled: RgbColor,
    pub color_normal: RgbColor,
    pub color_shift: RgbColor,
}

impl Default for WidgetView {
    fn default() -> Self {
        Self {
            mode: WidgetMode::Disabled,
            encoder_pressed: false,
            in_hold_mode: false,
            line1_disabled: String::new(),
            line1_normal: String::new(),
            line1_shift: String::new(),
            line2_disabled: String::new(),
            line2_normal: String::new(),
            line2_shift: String::new(),
            color_disabled: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
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
    pub fn new() -> Self {
        Self {
            mode: WidgetMode::Disabled,
            encoder_pressed: false,
            in_hold_mode: false,
            line1_disabled: String::new(),
            line1_normal: String::new(),
            line1_shift: String::new(),
            line2_disabled: String::new(),
            line2_normal: String::new(),
            line2_shift: String::new(),
            color_disabled: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
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

    pub fn line1_disabled(self, txt: &str) -> Self {
        Self {
            line1_disabled: txt.to_string(),
            ..self
        }
    }

    pub fn line1_normal(self, txt: &str) -> Self {
        Self {
            line1_normal: txt.to_string(),
            ..self
        }
    }

    pub fn line1_shift(self, txt: &str) -> Self {
        Self {
            line1_shift: txt.to_string(),
            ..self
        }
    }

    pub fn line2_disabled(self, txt: &str) -> Self {
        Self {
            line2_disabled: txt.to_string(),
            ..self
        }
    }

    pub fn line2_normal(self, txt: &str) -> Self {
        Self {
            line2_normal: txt.to_string(),
            ..self
        }
    }

    pub fn line2_shift(self, txt: &str) -> Self {
        Self {
            line2_shift: txt.to_string(),
            ..self
        }
    }

    pub fn color_disabled(self, color: RgbColor) -> Self {
        Self {
            color_disabled: color,
            ..self
        }
    }

    pub fn color_normal(self, color: RgbColor) -> Self {
        Self {
            color_normal: color,
            ..self
        }
    }

    pub fn color_shift(self, color: RgbColor) -> Self {
        Self {
            color_shift: color,
            ..self
        }
    }

    pub fn starting_mode(self, mode: WidgetMode) -> Self {
        Self { mode, ..self }
    }

    // Used at runtime
    fn line1_text(&self) -> String {
        match self.mode {
            WidgetMode::Disabled => self.line1_disabled.clone(),
            WidgetMode::Normal => self.line1_normal.clone(),
            WidgetMode::Press => self.line1_normal.clone(),
            WidgetMode::Shift => self.line1_shift.clone(),
            WidgetMode::ShiftPress => self.line1_shift.clone(),
        }
    }

    fn line2_text(&self) -> String {
        match self.mode {
            WidgetMode::Disabled => self.line2_disabled.clone(),
            WidgetMode::Normal => self.line2_normal.clone(),
            WidgetMode::Press => self.line2_normal.clone(),
            WidgetMode::Shift => self.line2_shift.clone(),
            WidgetMode::ShiftPress => self.line2_shift.clone(),
        }
    }

    fn color(&self) -> RgbColor {
        match self.mode {
            WidgetMode::Disabled => self.color_disabled,
            WidgetMode::Normal => self.color_normal,
            WidgetMode::Press => self.color_normal,
            WidgetMode::Shift => self.color_shift,
            WidgetMode::ShiftPress => self.color_shift,
        }
    }
}

pub fn apply_accel(prev: f32, turn: EncoderTurn) -> f32 {
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

pub fn apply_accel_center(prev: f32, turn: EncoderTurn) -> f32 {
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

pub fn encoder_ring_msg(hw_idx: usize, val: f32, mode: v1m::EncoderRingMode) -> v1m::DownstreamMsg {
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
