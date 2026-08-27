use crate::track;
use uuid::Uuid;

pub const FX_NAME: &str = "AU: TDR VOS SlickEQ (Tokyo Dawn Labs)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EQModel {
    American,
    British,
    German,
}
impl EQModel {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::American => 0f32,
            Self::British => 1f32,
            Self::German => 2f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::American),
            1 => Some(Self::British),
            2 => Some(Self::German),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OUTStage {
    Linear,
    Silky,
    Mellow,
    Deep,
}
impl OUTStage {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Linear => 0f32,
            Self::Silky => 1f32,
            Self::Mellow => 2f32,
            Self::Deep => 3f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Linear),
            1 => Some(Self::Silky),
            2 => Some(Self::Mellow),
            3 => Some(Self::Deep),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Mono,
    Stereo,
    Sum,
    Diff,
    Left,
}
impl Mode {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Mono => 0f32,
            Self::Stereo => 1f32,
            Self::Sum => 2f32,
            Self::Diff => 3f32,
            Self::Left => 4f32,
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
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Full,
    Eco,
}
impl Quality {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Full => 0f32,
            Self::Eco => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Full),
            1 => Some(Self::Eco),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    LOWGain(f32),
    LOWFreq(f32),
    LOWShape(bool),
    MIDGain(f32),
    MIDFreq(f32),
    HIGHGain(f32),
    HIGHFreq(f32),
    HIGHShape(bool),
    EQModel(EQModel),
    EQSat(bool),
    HPFreq(f32),
    OUTStage(OUTStage),
    OUTDrive(f32),
    OUTGain(f32),
    Bypass(bool),
    Mode(Mode),
    LOWBypass(bool),
    MIDBypass(bool),
    HIGHBypass(bool),
    AutoGain(bool),
    Quality(Quality),
    Bypass_2(f32),
    Wet(f32),
    Delta(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::LOWGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value,
        }
        .into(),
        Param::LOWFreq(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value,
        }
        .into(),
        Param::LOWShape(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::MIDGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value,
        }
        .into(),
        Param::MIDFreq(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value,
        }
        .into(),
        Param::HIGHGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value,
        }
        .into(),
        Param::HIGHFreq(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
        }
        .into(),
        Param::HIGHShape(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::EQModel(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 8,
            value: value.to_raw(),
        }
        .into(),
        Param::EQSat(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 9,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::HPFreq(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 10,
            value,
        }
        .into(),
        Param::OUTStage(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 11,
            value: value.to_raw(),
        }
        .into(),
        Param::OUTDrive(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 12,
            value,
        }
        .into(),
        Param::OUTGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 13,
            value,
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 14,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Mode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 15,
            value: value.to_raw(),
        }
        .into(),
        Param::LOWBypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 16,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::MIDBypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 17,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::HIGHBypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 18,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::AutoGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 19,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Quality(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 20,
            value: value.to_raw(),
        }
        .into(),
        Param::Bypass_2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 21,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 22,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 23,
            value,
        }
        .into(),
    }
}
pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {
    match msg.param_index {
        0 => Some(Param::LOWGain(msg.value)),
        1 => Some(Param::LOWFreq(msg.value)),
        2 => Some(Param::LOWShape(msg.value >= 0.5f32)),
        3 => Some(Param::MIDGain(msg.value)),
        4 => Some(Param::MIDFreq(msg.value)),
        5 => Some(Param::HIGHGain(msg.value)),
        6 => Some(Param::HIGHFreq(msg.value)),
        7 => Some(Param::HIGHShape(msg.value >= 0.5f32)),
        8 => EQModel::from_raw(msg.value).map(Param::EQModel),
        9 => Some(Param::EQSat(msg.value >= 0.5f32)),
        10 => Some(Param::HPFreq(msg.value)),
        11 => OUTStage::from_raw(msg.value).map(Param::OUTStage),
        12 => Some(Param::OUTDrive(msg.value)),
        13 => Some(Param::OUTGain(msg.value)),
        14 => Some(Param::Bypass(msg.value >= 0.5f32)),
        15 => Mode::from_raw(msg.value).map(Param::Mode),
        16 => Some(Param::LOWBypass(msg.value >= 0.5f32)),
        17 => Some(Param::MIDBypass(msg.value >= 0.5f32)),
        18 => Some(Param::HIGHBypass(msg.value >= 0.5f32)),
        19 => Some(Param::AutoGain(msg.value >= 0.5f32)),
        20 => Quality::from_raw(msg.value).map(Param::Quality),
        21 => Some(Param::Bypass_2(msg.value)),
        22 => Some(Param::Wet(msg.value)),
        23 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
