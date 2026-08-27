use crate::track;
use uuid::Uuid;

pub const FX_NAME: &str = "VST3: TDR Molotok (Tokyo Dawn Labs)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    SCFilterFreq(f32),
    SCFilterOn(bool),
    Attack(f32),
    Mode(f32),
    Release(f32),
    Ratio(f32),
    Thresh(f32),
    Knee(f32),
    Makeup(f32),
    DryMix(f32),
    Output(f32),
    StereoMode(bool),
    Quality(f32),
    Bypass(bool),
    SidechainMode(bool),
    Delta(bool),
    MeterRange(f32),
    MeterMode(bool),
    MeterSource(bool),
    MeterSpeed(bool),
    Program(f32),
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
            value,
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
            value,
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
        Param::Program(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 20,
            value,
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
        Param::Delta_2(value) => track::FXParamValue {
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
        0 => Some(Param::SCFilterFreq(msg.value)),
        1 => Some(Param::SCFilterOn(msg.value >= 0.5f32)),
        2 => Some(Param::Attack(msg.value)),
        3 => Some(Param::Mode(msg.value)),
        4 => Some(Param::Release(msg.value)),
        5 => Some(Param::Ratio(msg.value)),
        6 => Some(Param::Thresh(msg.value)),
        7 => Some(Param::Knee(msg.value)),
        8 => Some(Param::Makeup(msg.value)),
        9 => Some(Param::DryMix(msg.value)),
        10 => Some(Param::Output(msg.value)),
        11 => Some(Param::StereoMode(msg.value >= 0.5f32)),
        12 => Some(Param::Quality(msg.value)),
        13 => Some(Param::Bypass(msg.value >= 0.5f32)),
        14 => Some(Param::SidechainMode(msg.value >= 0.5f32)),
        15 => Some(Param::Delta(msg.value >= 0.5f32)),
        16 => Some(Param::MeterRange(msg.value)),
        17 => Some(Param::MeterMode(msg.value >= 0.5f32)),
        18 => Some(Param::MeterSource(msg.value >= 0.5f32)),
        19 => Some(Param::MeterSpeed(msg.value >= 0.5f32)),
        20 => Some(Param::Program(msg.value)),
        21 => Some(Param::Bypass_2(msg.value)),
        22 => Some(Param::Wet(msg.value)),
        23 => Some(Param::Delta_2(msg.value)),
        _ => None,
    }
}
