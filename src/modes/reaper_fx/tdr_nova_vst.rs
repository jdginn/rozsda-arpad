use crate::track::track;
use uuid::Uuid;

pub const FX_NAME: &str = "VST3: TDR Nova (Tokyo Dawn Labs)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    Band1Selected(bool),
    Band1Active(bool),
    Band1Gain(f32),
    Band1Q(f32),
    Band1Frequency(f32),
    Band1Type(f32),
    Band1Dyn(f32),
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
    Band2Type(f32),
    Band2Dyn(f32),
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
    Band3Type(f32),
    Band3Dyn(f32),
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
    Band4Type(f32),
    Band4Dyn(f32),
    Band4Threshold(f32),
    Band4Ratio(f32),
    Band4Split(bool),
    Band4Attack(f32),
    Band4Release(f32),
    HPSelected(bool),
    HPActive(bool),
    HPFrequency(f32),
    HPType(f32),
    LPSelected(bool),
    LPActive(bool),
    LPFrequency(f32),
    LPType(f32),
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
    Quality(f32),
    Channels(f32),
    AnalyzerMode(f32),
    DisplayEQRange(bool),
    DisplayFFTFloor(bool),
    SidechainMode(f32),
    AnalyzerSpeed(f32),
    Solo(bool),
    Program(f32),
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
            value,
        }
        .into(),
        Param::Band1Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 6,
            value,
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
            value,
        }
        .into(),
        Param::Band2Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 18,
            value,
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
            value,
        }
        .into(),
        Param::Band3Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 30,
            value,
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
            value,
        }
        .into(),
        Param::Band4Dyn(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 42,
            value,
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
            value,
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
            value,
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
            value,
        }
        .into(),
        Param::Channels(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 68,
            value,
        }
        .into(),
        Param::AnalyzerMode(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 69,
            value,
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
            value,
        }
        .into(),
        Param::AnalyzerSpeed(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 73,
            value,
        }
        .into(),
        Param::Solo(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 74,
            value: if value { 1f32 } else { 0f32 },
        }
        .into(),
        Param::Program(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 75,
            value,
        }
        .into(),
        Param::Bypass(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 76,
            value,
        }
        .into(),
        Param::Wet(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 77,
            value,
        }
        .into(),
        Param::Delta(value) => track::FXParamValue {
            track_guid,
            fx_index,
            param_index: 78,
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
        5 => Some(Param::Band1Type(msg.value)),
        6 => Some(Param::Band1Dyn(msg.value)),
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
        17 => Some(Param::Band2Type(msg.value)),
        18 => Some(Param::Band2Dyn(msg.value)),
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
        29 => Some(Param::Band3Type(msg.value)),
        30 => Some(Param::Band3Dyn(msg.value)),
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
        41 => Some(Param::Band4Type(msg.value)),
        42 => Some(Param::Band4Dyn(msg.value)),
        43 => Some(Param::Band4Threshold(msg.value)),
        44 => Some(Param::Band4Ratio(msg.value)),
        45 => Some(Param::Band4Split(msg.value >= 0.5f32)),
        46 => Some(Param::Band4Attack(msg.value)),
        47 => Some(Param::Band4Release(msg.value)),
        48 => Some(Param::HPSelected(msg.value >= 0.5f32)),
        49 => Some(Param::HPActive(msg.value >= 0.5f32)),
        50 => Some(Param::HPFrequency(msg.value)),
        51 => Some(Param::HPType(msg.value)),
        52 => Some(Param::LPSelected(msg.value >= 0.5f32)),
        53 => Some(Param::LPActive(msg.value >= 0.5f32)),
        54 => Some(Param::LPFrequency(msg.value)),
        55 => Some(Param::LPType(msg.value)),
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
        67 => Some(Param::Quality(msg.value)),
        68 => Some(Param::Channels(msg.value)),
        69 => Some(Param::AnalyzerMode(msg.value)),
        70 => Some(Param::DisplayEQRange(msg.value >= 0.5f32)),
        71 => Some(Param::DisplayFFTFloor(msg.value >= 0.5f32)),
        72 => Some(Param::SidechainMode(msg.value)),
        73 => Some(Param::AnalyzerSpeed(msg.value)),
        74 => Some(Param::Solo(msg.value >= 0.5f32)),
        75 => Some(Param::Program(msg.value)),
        76 => Some(Param::Bypass(msg.value)),
        77 => Some(Param::Wet(msg.value)),
        78 => Some(Param::Delta(msg.value)),
        _ => None,
    }
}
