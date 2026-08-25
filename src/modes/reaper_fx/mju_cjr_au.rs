use crate::track::track;
use uuid::Uuid;

pub fn name() -> &'static str {
    "AU: MJUCjr (Klanghelm)"
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    Fast,
    Slow,
}
impl Timing {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::Fast => 0f32,
            Self::Slow => 1f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::Fast),
            1 => Some(Self::Slow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VuMode {
    GR,
}
impl VuMode {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::GR => 0f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::GR),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    Makeup(f32),
    Timing(Timing),
    VuMode(VuMode),
    Compress(f32),
    Bypass(bool),
    Bypass_2(f32),
    Wet(f32),
    Delta(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::Makeup(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value,
        }
        .into(),
        Param::Timing(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value: value.to_raw(),
        }
        .into(),
        Param::VuMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value: value.to_raw(),
        }
        .into(),
        Param::Compress(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value,
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Bypass_2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value,
        }
        .into(),
    }
}
pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {
    match msg.param_index {
        0 => Some(Param::Makeup(msg.value)),
        1 => Timing::from_raw(msg.value).map(Param::Timing),
        2 => VuMode::from_raw(msg.value).map(Param::VuMode),
        3 => Some(Param::Compress(msg.value)),
        4 => Some(Param::Bypass(msg.value >= 0.5f32)),
        5 => Some(Param::Bypass_2(msg.value)),
        6 => Some(Param::Wet(msg.value)),
        7 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
