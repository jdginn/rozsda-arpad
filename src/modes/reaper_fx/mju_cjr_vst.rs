use crate::track::track;
use uuid::Uuid;

pub const FX_NAME: &str = "VST3: MJUCjr (Klanghelm)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    Compress(f32),
    Makeup(f32),
    Timing(f32),
    VuMode(bool),
    Bypass(bool),
    Bypass_2(bool),
    Bypass_3(f32),
    Wet(f32),
    Delta(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::Compress(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value,
        }
        .into(),
        Param::Makeup(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value,
        }
        .into(),
        Param::Timing(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value,
        }
        .into(),
        Param::VuMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value: if value { 1f32 } else { 0f32 },
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
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Bypass_3(value) => track::FXParamValue {
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
        0 => Some(Param::Compress(msg.value)),
        1 => Some(Param::Makeup(msg.value)),
        2 => Some(Param::Timing(msg.value)),
        3 => Some(Param::VuMode(msg.value >= 0.5f32)),
        4 => Some(Param::Bypass(msg.value >= 0.5f32)),
        5 => Some(Param::Bypass_2(msg.value >= 0.5f32)),
        6 => Some(Param::Bypass_3(msg.value)),
        7 => Some(Param::Wet(msg.value)),
        8 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
