use crate::track::track;
use uuid::Uuid;

pub fn name() -> &'static str {
    "JS: 1175 Compressor"
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ratio {
    BlownCapacitor4,
    BlownCapacitor8,
    BlownCapacitor12,
    BlownCapacitor20,
    BlownCapacitorAll,
    _4,
    _8,
    _12,
    _20,
}
impl Ratio {
    pub fn to_raw(self) -> f32 {
        match self {
            Self::BlownCapacitor4 => 0f32,
            Self::BlownCapacitor8 => 1f32,
            Self::BlownCapacitor12 => 2f32,
            Self::BlownCapacitor20 => 3f32,
            Self::BlownCapacitorAll => 4f32,
            Self::_4 => 5f32,
            Self::_8 => 6f32,
            Self::_12 => 7f32,
            Self::_20 => 8f32,
        }
    }
    pub fn from_raw(value: f32) -> Option<Self> {
        let rounded = value.round() as isize;
        match rounded {
            0 => Some(Self::BlownCapacitor4),
            1 => Some(Self::BlownCapacitor8),
            2 => Some(Self::BlownCapacitor12),
            3 => Some(Self::BlownCapacitor20),
            4 => Some(Self::BlownCapacitorAll),
            5 => Some(Self::_4),
            6 => Some(Self::_8),
            7 => Some(Self::_12),
            8 => Some(Self::_20),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    Threshold(f32),
    Ratio(Ratio),
    Gain(f32),
    Attack(f32),
    Release(f32),
    Mix(f32),
    Bypass(f32),
    Wet(f32),
    Delta(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::Threshold(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value,
        }
        .into(),
        Param::Ratio(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value: value.to_raw(),
        }
        .into(),
        Param::Gain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value,
        }
        .into(),
        Param::Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value,
        }
        .into(),
        Param::Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value,
        }
        .into(),
        Param::Mix(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value,
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 8,
            value,
        }
        .into(),
    }
}
pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {
    match msg.param_index {
        0 => Some(Param::Threshold(msg.value)),
        1 => Ratio::from_raw(msg.value).map(Param::Ratio),
        2 => Some(Param::Gain(msg.value)),
        3 => Some(Param::Attack(msg.value)),
        4 => Some(Param::Release(msg.value)),
        5 => Some(Param::Mix(msg.value)),
        6 => Some(Param::Bypass(msg.value)),
        7 => Some(Param::Wet(msg.value)),
        8 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
