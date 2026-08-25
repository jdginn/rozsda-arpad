use crate::track::track;
use uuid::Uuid;

pub fn name() -> &'static str {
    "VST3: TDR VOS SlickEQ (Tokyo Dawn Labs)"
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
    EQModel(f32),
    EQSat(bool),
    HPFreq(f32),
    OUTStage(f32),
    OUTDrive(f32),
    OUTGain(f32),
    Bypass(bool),
    Mode(f32),
    LOWBypass(bool),
    MIDBypass(bool),
    HIGHBypass(bool),
    AutoGain(bool),
    Quality(f32),
    Program(f32),
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
            value,
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
            value,
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
            value,
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
            value,
        }
        .into(),
        Param::Program(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 21,
            value,
        }
        .into(),
        Param::Bypass_2(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 22,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 23,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 24,
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
        8 => Some(Param::EQModel(msg.value)),
        9 => Some(Param::EQSat(msg.value >= 0.5f32)),
        10 => Some(Param::HPFreq(msg.value)),
        11 => Some(Param::OUTStage(msg.value)),
        12 => Some(Param::OUTDrive(msg.value)),
        13 => Some(Param::OUTGain(msg.value)),
        14 => Some(Param::Bypass(msg.value >= 0.5f32)),
        15 => Some(Param::Mode(msg.value)),
        16 => Some(Param::LOWBypass(msg.value >= 0.5f32)),
        17 => Some(Param::MIDBypass(msg.value >= 0.5f32)),
        18 => Some(Param::HIGHBypass(msg.value >= 0.5f32)),
        19 => Some(Param::AutoGain(msg.value >= 0.5f32)),
        20 => Some(Param::Quality(msg.value)),
        21 => Some(Param::Program(msg.value)),
        22 => Some(Param::Bypass_2(msg.value)),
        23 => Some(Param::Wet(msg.value)),
        24 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
