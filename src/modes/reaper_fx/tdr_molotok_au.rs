use crate::track;
use uuid::Uuid;

pub const FX_NAME: &str = "AU: TDR Molotok (Tokyo Dawn Labs)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    _00,
    _01,
    _02,
    _03,
    _04,
    _05,
    _06,
    _07,
    _08,
    _09,
}
impl Mode {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::_00 => 0f32,
            Self::_01 => 1f32,
            Self::_02 => 2f32,
            Self::_03 => 3f32,
            Self::_04 => 4f32,
            Self::_05 => 5f32,
            Self::_06 => 6f32,
            Self::_07 => 7f32,
            Self::_08 => 8f32,
            Self::_09 => 9f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::_00),
            1 => Some(Self::_01),
            2 => Some(Self::_02),
            3 => Some(Self::_03),
            4 => Some(Self::_04),
            5 => Some(Self::_05),
            6 => Some(Self::_06),
            7 => Some(Self::_07),
            8 => Some(Self::_08),
            9 => Some(Self::_09),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Live,
    Eco,
}
impl Quality {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Live => 0f32,
            Self::Eco => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Live),
            1 => Some(Self::Eco),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeterRange {
    _2DB,
    _3DB,
    _4DB,
    _6DB,
    _9DB,
    _12DB,
    _18DB,
    _24DB,
}
impl MeterRange {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::_2DB => 0f32,
            Self::_3DB => 1f32,
            Self::_4DB => 2f32,
            Self::_6DB => 3f32,
            Self::_9DB => 4f32,
            Self::_12DB => 5f32,
            Self::_18DB => 6f32,
            Self::_24DB => 7f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::_2DB),
            1 => Some(Self::_3DB),
            2 => Some(Self::_4DB),
            3 => Some(Self::_6DB),
            4 => Some(Self::_9DB),
            5 => Some(Self::_12DB),
            6 => Some(Self::_18DB),
            7 => Some(Self::_24DB),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    SCFilterFreq(f32),
    SCFilterOn(bool),
    Attack(f32),
    Mode(Mode),
    Release(f32),
    Ratio(f32),
    Thresh(f32),
    Knee(f32),
    Makeup(f32),
    DryMix(f32),
    Output(f32),
    StereoMode(bool),
    Quality(Quality),
    Bypass(bool),
    SidechainMode(bool),
    Delta(bool),
    MeterRange(MeterRange),
    MeterMode(bool),
    MeterSource(bool),
    MeterSpeed(bool),
    Bypass_2(f32),
    Wet(f32),
    Delta_2(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::SCFilterFreq(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value,
        }
        .into(),
        Param::SCFilterOn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value,
        }
        .into(),
        Param::Mode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value: value.to_raw(),
        }
        .into(),
        Param::Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value,
        }
        .into(),
        Param::Ratio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value,
        }
        .into(),
        Param::Thresh(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
        }
        .into(),
        Param::Knee(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value,
        }
        .into(),
        Param::Makeup(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 8,
            value,
        }
        .into(),
        Param::DryMix(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 9,
            value,
        }
        .into(),
        Param::Output(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 10,
            value,
        }
        .into(),
        Param::StereoMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 11,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Quality(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 12,
            value: value.to_raw(),
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 13,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::SidechainMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 14,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 15,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::MeterRange(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 16,
            value: value.to_raw(),
        }
        .into(),
        Param::MeterMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 17,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::MeterSource(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 18,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::MeterSpeed(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 19,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Bypass_2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 20,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 21,
            value,
        }
        .into(),
        Param::Delta_2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 22,
            value,
        }
        .into(),
    }
}
pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {
    match msg.param_index {
        0 => Some(Param::SCFilterFreq(msg.value)),
        1 => Some(Param::SCFilterOn(msg.value >= 0.5f32)),
        2 => Some(Param::Attack(msg.value)),
        3 => Mode::from_raw(msg.value).map(Param::Mode),
        4 => Some(Param::Release(msg.value)),
        5 => Some(Param::Ratio(msg.value)),
        6 => Some(Param::Thresh(msg.value)),
        7 => Some(Param::Knee(msg.value)),
        8 => Some(Param::Makeup(msg.value)),
        9 => Some(Param::DryMix(msg.value)),
        10 => Some(Param::Output(msg.value)),
        11 => Some(Param::StereoMode(msg.value >= 0.5f32)),
        12 => Quality::from_raw(msg.value).map(Param::Quality),
        13 => Some(Param::Bypass(msg.value >= 0.5f32)),
        14 => Some(Param::SidechainMode(msg.value >= 0.5f32)),
        15 => Some(Param::Delta(msg.value >= 0.5f32)),
        16 => MeterRange::from_raw(msg.value).map(Param::MeterRange),
        17 => Some(Param::MeterMode(msg.value >= 0.5f32)),
        18 => Some(Param::MeterSource(msg.value >= 0.5f32)),
        19 => Some(Param::MeterSpeed(msg.value >= 0.5f32)),
        20 => Some(Param::Bypass_2(msg.value)),
        21 => Some(Param::Wet(msg.value)),
        22 => Some(Param::Delta_2(msg.value)),
        _ => None,
    }
}
