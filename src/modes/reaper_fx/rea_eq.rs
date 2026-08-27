use crate::track;
use uuid::Uuid;

pub const FX_NAME: &str = "VST: ReaEQ (Cockos)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    FreqLowShelf(f32),
    GainLowShelf(f32),
    BWLowShelf(f32),
    FreqBand2(f32),
    GainBand2(f32),
    BWBand2(f32),
    FreqBand3(f32),
    GainBand3(f32),
    BWBand3(f32),
    FreqHighShelf4(f32),
    GainHighShelf4(f32),
    BWHighShelf4(f32),
    FreqHighPass5(f32),
    GainHighPass5(f32),
    BWHighPass5(f32),
    GlobalGain(f32),
    Bypass(f32),
    Wet(f32),
    Delta(f32),
}
pub fn encode_trackmsg(track_guid: Uuid, fx_index: i32, param: Param) -> track::TrackMsg {
    match param {
        Param::FreqLowShelf(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 0,
            value,
        }
        .into(),
        Param::GainLowShelf(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 1,
            value,
        }
        .into(),
        Param::BWLowShelf(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 2,
            value,
        }
        .into(),
        Param::FreqBand2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 3,
            value,
        }
        .into(),
        Param::GainBand2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 4,
            value,
        }
        .into(),
        Param::BWBand2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 5,
            value,
        }
        .into(),
        Param::FreqBand3(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
        }
        .into(),
        Param::GainBand3(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 7,
            value,
        }
        .into(),
        Param::BWBand3(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 8,
            value,
        }
        .into(),
        Param::FreqHighShelf4(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 9,
            value,
        }
        .into(),
        Param::GainHighShelf4(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 10,
            value,
        }
        .into(),
        Param::BWHighShelf4(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 11,
            value,
        }
        .into(),
        Param::FreqHighPass5(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 12,
            value,
        }
        .into(),
        Param::GainHighPass5(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 13,
            value,
        }
        .into(),
        Param::BWHighPass5(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 14,
            value,
        }
        .into(),
        Param::GlobalGain(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 15,
            value,
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 16,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 17,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 18,
            value,
        }
        .into(),
    }
}
pub fn decode_trackmsg(msg: track::FXParamValue) -> Option<Param> {
    match msg.param_index {
        0 => Some(Param::FreqLowShelf(msg.value)),
        1 => Some(Param::GainLowShelf(msg.value)),
        2 => Some(Param::BWLowShelf(msg.value)),
        3 => Some(Param::FreqBand2(msg.value)),
        4 => Some(Param::GainBand2(msg.value)),
        5 => Some(Param::BWBand2(msg.value)),
        6 => Some(Param::FreqBand3(msg.value)),
        7 => Some(Param::GainBand3(msg.value)),
        8 => Some(Param::BWBand3(msg.value)),
        9 => Some(Param::FreqHighShelf4(msg.value)),
        10 => Some(Param::GainHighShelf4(msg.value)),
        11 => Some(Param::BWHighShelf4(msg.value)),
        12 => Some(Param::FreqHighPass5(msg.value)),
        13 => Some(Param::GainHighPass5(msg.value)),
        14 => Some(Param::BWHighPass5(msg.value)),
        15 => Some(Param::GlobalGain(msg.value)),
        16 => Some(Param::Bypass(msg.value)),
        17 => Some(Param::Wet(msg.value)),
        18 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
