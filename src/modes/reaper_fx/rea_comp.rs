use crate::track;
use uuid::Uuid;

pub const FX_NAME: &str = "VST: ReaComp (Cockos)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    Threshold(f32),
    Ratio(f32),
    Attack(f32),
    Release(f32),
    PreComp(f32),
    Resvd(f32),
    Lowpass(f32),
    Hipass(f32),
    SignIn(f32),
    AudIn(f32),
    Dry(f32),
    Wet(f32),
    FilterPreview(bool),
    RMSSize(f32),
    Knee(f32),
    AutoMakeUpGain(bool),
    AutoRelease(bool),
    LegacyAttackKneeOptions(f32),
    DeprecatedBrokenAntiAlias(f32),
    MultichannelMode(f32),
    MeteringIndex(f32),
    Bypass(f32),
    Wet_2(f32),
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
            value,
        }
        .into(),
        Param::Attack(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value,
        }
        .into(),
        Param::Release(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value,
        }
        .into(),
        Param::PreComp(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value,
        }
        .into(),
        Param::Resvd(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value,
        }
        .into(),
        Param::Lowpass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
        }
        .into(),
        Param::Hipass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value,
        }
        .into(),
        Param::SignIn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 8,
            value,
        }
        .into(),
        Param::AudIn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 9,
            value,
        }
        .into(),
        Param::Dry(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 10,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 11,
            value,
        }
        .into(),
        Param::FilterPreview(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 12,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::RMSSize(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 13,
            value,
        }
        .into(),
        Param::Knee(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 14,
            value,
        }
        .into(),
        Param::AutoMakeUpGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 15,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::AutoRelease(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 16,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::LegacyAttackKneeOptions(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 17,
            value,
        }
        .into(),
        Param::DeprecatedBrokenAntiAlias(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 18,
            value,
        }
        .into(),
        Param::MultichannelMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 19,
            value,
        }
        .into(),
        Param::MeteringIndex(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 20,
            value,
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 21,
            value,
        }
        .into(),
        Param::Wet_2(value) => track::FXParamValue {
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
        0 => Some(Param::Threshold(msg.value)),
        1 => Some(Param::Ratio(msg.value)),
        2 => Some(Param::Attack(msg.value)),
        3 => Some(Param::Release(msg.value)),
        4 => Some(Param::PreComp(msg.value)),
        5 => Some(Param::Resvd(msg.value)),
        6 => Some(Param::Lowpass(msg.value)),
        7 => Some(Param::Hipass(msg.value)),
        8 => Some(Param::SignIn(msg.value)),
        9 => Some(Param::AudIn(msg.value)),
        10 => Some(Param::Dry(msg.value)),
        11 => Some(Param::Wet(msg.value)),
        12 => Some(Param::FilterPreview(msg.value >= 0.5f32)),
        13 => Some(Param::RMSSize(msg.value)),
        14 => Some(Param::Knee(msg.value)),
        15 => Some(Param::AutoMakeUpGain(msg.value >= 0.5f32)),
        16 => Some(Param::AutoRelease(msg.value >= 0.5f32)),
        17 => Some(Param::LegacyAttackKneeOptions(msg.value)),
        18 => Some(Param::DeprecatedBrokenAntiAlias(msg.value)),
        19 => Some(Param::MultichannelMode(msg.value)),
        20 => Some(Param::MeteringIndex(msg.value)),
        21 => Some(Param::Bypass(msg.value)),
        22 => Some(Param::Wet_2(msg.value)),
        23 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
