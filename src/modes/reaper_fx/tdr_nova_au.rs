use crate::track::track;
use uuid::Uuid;

pub const FX_NAME: &str = "AU: TDR Nova (Tokyo Dawn Labs)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band1Type {
    LowS,
    Bell,
}
impl Band1Type {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::LowS => 0f32,
            Self::Bell => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::LowS),
            1 => Some(Self::Bell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band1Dyn {
    Off,
    On,
}
impl Band1Dyn {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Off => 0f32,
            Self::On => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band2Type {
    LowS,
    Bell,
}
impl Band2Type {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::LowS => 0f32,
            Self::Bell => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::LowS),
            1 => Some(Self::Bell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band2Dyn {
    Off,
    On,
}
impl Band2Dyn {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Off => 0f32,
            Self::On => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band3Type {
    LowS,
    Bell,
}
impl Band3Type {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::LowS => 0f32,
            Self::Bell => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::LowS),
            1 => Some(Self::Bell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band3Dyn {
    Off,
    On,
}
impl Band3Dyn {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Off => 0f32,
            Self::On => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band4Type {
    LowS,
    Bell,
}
impl Band4Type {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::LowS => 0f32,
            Self::Bell => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::LowS),
            1 => Some(Self::Bell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band4Dyn {
    Off,
    On,
}
impl Band4Dyn {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Off => 0f32,
            Self::On => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HPType {
    _6dBOct,
    _12dBOct,
    _24dBOct,
}
impl HPType {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::_6dBOct => 0f32,
            Self::_12dBOct => 1f32,
            Self::_24dBOct => 2f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::_6dBOct),
            1 => Some(Self::_12dBOct),
            2 => Some(Self::_24dBOct),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LPType {
    _6dBOct,
    _12dBOct,
    _24dBOct,
}
impl LPType {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::_6dBOct => 0f32,
            Self::_12dBOct => 1f32,
            Self::_24dBOct => 2f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::_6dBOct),
            1 => Some(Self::_12dBOct),
            2 => Some(Self::_24dBOct),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Eco,
    Precise,
}
impl Quality {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Eco => 0f32,
            Self::Precise => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Eco),
            1 => Some(Self::Precise),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Channels {
    Mono,
    Stereo,
    Sum,
    Diff,
    Left,
    Right,
}
impl Channels {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Mono => 0f32,
            Self::Stereo => 1f32,
            Self::Sum => 2f32,
            Self::Diff => 3f32,
            Self::Left => 4f32,
            Self::Right => 5f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Mono),
            1 => Some(Self::Stereo),
            2 => Some(Self::Sum),
            3 => Some(Self::Diff),
            4 => Some(Self::Left),
            5 => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalyzerMode {
    AnalyzerIn,
    AnalyzerOut,
    AnalyzerSC,
    AnalyzerOff,
}
impl AnalyzerMode {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::AnalyzerIn => 0f32,
            Self::AnalyzerOut => 1f32,
            Self::AnalyzerSC => 2f32,
            Self::AnalyzerOff => 3f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::AnalyzerIn),
            1 => Some(Self::AnalyzerOut),
            2 => Some(Self::AnalyzerSC),
            3 => Some(Self::AnalyzerOff),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidechainMode {
    IntSC,
    ExtSC,
}
impl SidechainMode {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::IntSC => 0f32,
            Self::ExtSC => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::IntSC),
            1 => Some(Self::ExtSC),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalyzerSpeed {
    Fast,
    Normal,
}
impl AnalyzerSpeed {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Fast => 0f32,
            Self::Normal => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Fast),
            1 => Some(Self::Normal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    Band1Selected(bool),
    Band1Active(bool),
    Band1Gain(f32),
    Band1Q(f32),
    Band1Frequency(f32),
    Band1Type(Band1Type),
    Band1Dyn(Band1Dyn),
    Band1Threshold(f32),
    Band1Ratio(f32),
    Band1Split(bool),
    Band1Attack(f32),
    Band1Release(f32),
    Band2Selected(bool),
    Band2Active(bool),
    Band2Gain(f32),
    Band2Q(f32),
    Band2Frequency(f32),
    Band2Type(Band2Type),
    Band2Dyn(Band2Dyn),
    Band2Threshold(f32),
    Band2Ratio(f32),
    Band2Split(bool),
    Band2Attack(f32),
    Band2Release(f32),
    Band3Selected(bool),
    Band3Active(bool),
    Band3Gain(f32),
    Band3Q(f32),
    Band3Frequency(f32),
    Band3Type(Band3Type),
    Band3Dyn(Band3Dyn),
    Band3Threshold(f32),
    Band3Ratio(f32),
    Band3Split(bool),
    Band3Attack(f32),
    Band3Release(f32),
    Band4Selected(bool),
    Band4Active(bool),
    Band4Gain(f32),
    Band4Q(f32),
    Band4Frequency(f32),
    Band4Type(Band4Type),
    Band4Dyn(Band4Dyn),
    Band4Threshold(f32),
    Band4Ratio(f32),
    Band4Split(bool),
    Band4Attack(f32),
    Band4Release(f32),
    HPSelected(bool),
    HPActive(bool),
    HPFrequency(f32),
    HPType(HPType),
    LPSelected(bool),
    LPActive(bool),
    LPFrequency(f32),
    LPType(LPType),
    WideBandGain(f32),
    WideBandDyn(bool),
    WideBandThreshold(f32),
    WideBandRatio(f32),
    WideBandAttack(f32),
    WideBandRelease(f32),
    MasterBypass(bool),
    DynamicDelta(bool),
    DryMix(f32),
    OutputGain(f32),
    EQAutoGain(bool),
    Quality(Quality),
    Channels(Channels),
    AnalyzerMode(AnalyzerMode),
    DisplayEQRange(bool),
    DisplayFFTFloor(bool),
    SidechainMode(SidechainMode),
    AnalyzerSpeed(AnalyzerSpeed),
    Solo(bool),
    Bypass(f32),
    Wet(f32),
    Delta(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::Band1Selected(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band1Active(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band1Gain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value,
        }
        .into(),
        Param::Band1Q(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value,
        }
        .into(),
        Param::Band1Frequency(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value,
        }
        .into(),
        Param::Band1Type(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value: value.to_raw(),
        }
        .into(),
        Param::Band1Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value: value.to_raw(),
        }
        .into(),
        Param::Band1Threshold(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value,
        }
        .into(),
        Param::Band1Ratio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 8,
            value,
        }
        .into(),
        Param::Band1Split(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 9,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band1Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 10,
            value,
        }
        .into(),
        Param::Band1Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 11,
            value,
        }
        .into(),
        Param::Band2Selected(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 12,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band2Active(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 13,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band2Gain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 14,
            value,
        }
        .into(),
        Param::Band2Q(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 15,
            value,
        }
        .into(),
        Param::Band2Frequency(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 16,
            value,
        }
        .into(),
        Param::Band2Type(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 17,
            value: value.to_raw(),
        }
        .into(),
        Param::Band2Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 18,
            value: value.to_raw(),
        }
        .into(),
        Param::Band2Threshold(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 19,
            value,
        }
        .into(),
        Param::Band2Ratio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 20,
            value,
        }
        .into(),
        Param::Band2Split(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 21,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band2Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 22,
            value,
        }
        .into(),
        Param::Band2Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 23,
            value,
        }
        .into(),
        Param::Band3Selected(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 24,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band3Active(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 25,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band3Gain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 26,
            value,
        }
        .into(),
        Param::Band3Q(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 27,
            value,
        }
        .into(),
        Param::Band3Frequency(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 28,
            value,
        }
        .into(),
        Param::Band3Type(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 29,
            value: value.to_raw(),
        }
        .into(),
        Param::Band3Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 30,
            value: value.to_raw(),
        }
        .into(),
        Param::Band3Threshold(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 31,
            value,
        }
        .into(),
        Param::Band3Ratio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 32,
            value,
        }
        .into(),
        Param::Band3Split(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 33,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band3Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 34,
            value,
        }
        .into(),
        Param::Band3Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 35,
            value,
        }
        .into(),
        Param::Band4Selected(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 36,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band4Active(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 37,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band4Gain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 38,
            value,
        }
        .into(),
        Param::Band4Q(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 39,
            value,
        }
        .into(),
        Param::Band4Frequency(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 40,
            value,
        }
        .into(),
        Param::Band4Type(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 41,
            value: value.to_raw(),
        }
        .into(),
        Param::Band4Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 42,
            value: value.to_raw(),
        }
        .into(),
        Param::Band4Threshold(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 43,
            value,
        }
        .into(),
        Param::Band4Ratio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 44,
            value,
        }
        .into(),
        Param::Band4Split(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 45,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Band4Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 46,
            value,
        }
        .into(),
        Param::Band4Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 47,
            value,
        }
        .into(),
        Param::HPSelected(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 48,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::HPActive(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 49,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::HPFrequency(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 50,
            value,
        }
        .into(),
        Param::HPType(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 51,
            value: value.to_raw(),
        }
        .into(),
        Param::LPSelected(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 52,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::LPActive(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 53,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::LPFrequency(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 54,
            value,
        }
        .into(),
        Param::LPType(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 55,
            value: value.to_raw(),
        }
        .into(),
        Param::WideBandGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 56,
            value,
        }
        .into(),
        Param::WideBandDyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 57,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::WideBandThreshold(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 58,
            value,
        }
        .into(),
        Param::WideBandRatio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 59,
            value,
        }
        .into(),
        Param::WideBandAttack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 60,
            value,
        }
        .into(),
        Param::WideBandRelease(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 61,
            value,
        }
        .into(),
        Param::MasterBypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 62,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::DynamicDelta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 63,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::DryMix(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 64,
            value,
        }
        .into(),
        Param::OutputGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 65,
            value,
        }
        .into(),
        Param::EQAutoGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 66,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Quality(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 67,
            value: value.to_raw(),
        }
        .into(),
        Param::Channels(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 68,
            value: value.to_raw(),
        }
        .into(),
        Param::AnalyzerMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 69,
            value: value.to_raw(),
        }
        .into(),
        Param::DisplayEQRange(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 70,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::DisplayFFTFloor(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 71,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::SidechainMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 72,
            value: value.to_raw(),
        }
        .into(),
        Param::AnalyzerSpeed(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 73,
            value: value.to_raw(),
        }
        .into(),
        Param::Solo(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 74,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 75,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 76,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 77,
            value,
        }
        .into(),
    }
}
pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {
    match msg.param_index {
        0 => Some(Param::Band1Selected(msg.value >= 0.5f32)),
        1 => Some(Param::Band1Active(msg.value >= 0.5f32)),
        2 => Some(Param::Band1Gain(msg.value)),
        3 => Some(Param::Band1Q(msg.value)),
        4 => Some(Param::Band1Frequency(msg.value)),
        5 => Band1Type::from_raw(msg.value).map(Param::Band1Type),
        6 => Band1Dyn::from_raw(msg.value).map(Param::Band1Dyn),
        7 => Some(Param::Band1Threshold(msg.value)),
        8 => Some(Param::Band1Ratio(msg.value)),
        9 => Some(Param::Band1Split(msg.value >= 0.5f32)),
        10 => Some(Param::Band1Attack(msg.value)),
        11 => Some(Param::Band1Release(msg.value)),
        12 => Some(Param::Band2Selected(msg.value >= 0.5f32)),
        13 => Some(Param::Band2Active(msg.value >= 0.5f32)),
        14 => Some(Param::Band2Gain(msg.value)),
        15 => Some(Param::Band2Q(msg.value)),
        16 => Some(Param::Band2Frequency(msg.value)),
        17 => Band2Type::from_raw(msg.value).map(Param::Band2Type),
        18 => Band2Dyn::from_raw(msg.value).map(Param::Band2Dyn),
        19 => Some(Param::Band2Threshold(msg.value)),
        20 => Some(Param::Band2Ratio(msg.value)),
        21 => Some(Param::Band2Split(msg.value >= 0.5f32)),
        22 => Some(Param::Band2Attack(msg.value)),
        23 => Some(Param::Band2Release(msg.value)),
        24 => Some(Param::Band3Selected(msg.value >= 0.5f32)),
        25 => Some(Param::Band3Active(msg.value >= 0.5f32)),
        26 => Some(Param::Band3Gain(msg.value)),
        27 => Some(Param::Band3Q(msg.value)),
        28 => Some(Param::Band3Frequency(msg.value)),
        29 => Band3Type::from_raw(msg.value).map(Param::Band3Type),
        30 => Band3Dyn::from_raw(msg.value).map(Param::Band3Dyn),
        31 => Some(Param::Band3Threshold(msg.value)),
        32 => Some(Param::Band3Ratio(msg.value)),
        33 => Some(Param::Band3Split(msg.value >= 0.5f32)),
        34 => Some(Param::Band3Attack(msg.value)),
        35 => Some(Param::Band3Release(msg.value)),
        36 => Some(Param::Band4Selected(msg.value >= 0.5f32)),
        37 => Some(Param::Band4Active(msg.value >= 0.5f32)),
        38 => Some(Param::Band4Gain(msg.value)),
        39 => Some(Param::Band4Q(msg.value)),
        40 => Some(Param::Band4Frequency(msg.value)),
        41 => Band4Type::from_raw(msg.value).map(Param::Band4Type),
        42 => Band4Dyn::from_raw(msg.value).map(Param::Band4Dyn),
        43 => Some(Param::Band4Threshold(msg.value)),
        44 => Some(Param::Band4Ratio(msg.value)),
        45 => Some(Param::Band4Split(msg.value >= 0.5f32)),
        46 => Some(Param::Band4Attack(msg.value)),
        47 => Some(Param::Band4Release(msg.value)),
        48 => Some(Param::HPSelected(msg.value >= 0.5f32)),
        49 => Some(Param::HPActive(msg.value >= 0.5f32)),
        50 => Some(Param::HPFrequency(msg.value)),
        51 => HPType::from_raw(msg.value).map(Param::HPType),
        52 => Some(Param::LPSelected(msg.value >= 0.5f32)),
        53 => Some(Param::LPActive(msg.value >= 0.5f32)),
        54 => Some(Param::LPFrequency(msg.value)),
        55 => LPType::from_raw(msg.value).map(Param::LPType),
        56 => Some(Param::WideBandGain(msg.value)),
        57 => Some(Param::WideBandDyn(msg.value >= 0.5f32)),
        58 => Some(Param::WideBandThreshold(msg.value)),
        59 => Some(Param::WideBandRatio(msg.value)),
        60 => Some(Param::WideBandAttack(msg.value)),
        61 => Some(Param::WideBandRelease(msg.value)),
        62 => Some(Param::MasterBypass(msg.value >= 0.5f32)),
        63 => Some(Param::DynamicDelta(msg.value >= 0.5f32)),
        64 => Some(Param::DryMix(msg.value)),
        65 => Some(Param::OutputGain(msg.value)),
        66 => Some(Param::EQAutoGain(msg.value >= 0.5f32)),
        67 => Quality::from_raw(msg.value).map(Param::Quality),
        68 => Channels::from_raw(msg.value).map(Param::Channels),
        69 => AnalyzerMode::from_raw(msg.value).map(Param::AnalyzerMode),
        70 => Some(Param::DisplayEQRange(msg.value >= 0.5f32)),
        71 => Some(Param::DisplayFFTFloor(msg.value >= 0.5f32)),
        72 => SidechainMode::from_raw(msg.value).map(Param::SidechainMode),
        73 => AnalyzerSpeed::from_raw(msg.value).map(Param::AnalyzerSpeed),
        74 => Some(Param::Solo(msg.value >= 0.5f32)),
        75 => Some(Param::Bypass(msg.value)),
        76 => Some(Param::Wet(msg.value)),
        77 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
