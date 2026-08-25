use crate::modes::reaper_channel_strip_router::EqMsg;
use crate::modes::reaper_fx::rea_eq;
use crate::modes::reaper_fx_adapters::FxAdapterTyped;

pub struct ReaEqAdapter;

impl FxAdapterTyped for ReaEqAdapter {
    type Msg = EqMsg;

    fn id(&self) -> &'static str {
        rea_eq::FX_NAME
    }
    fn fx_names(&self) -> &'static [&'static str] {
        &[rea_eq::FX_NAME]
    }

    fn to_track_typed(
        &self,
        track: uuid::Uuid,
        fx_index: i32,
        msg: EqMsg,
    ) -> Vec<crate::track::track::TrackMsg> {
        match msg {
            EqMsg::LowFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqLowShelf(val),
            )],
            EqMsg::LowGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainLowShelf(val),
            )],
            EqMsg::LowQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWLowShelf(val),
            )],
            EqMsg::LmFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqBand2(val),
            )],
            EqMsg::LmGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainBand2(val),
            )],
            EqMsg::LmQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWBand2(val),
            )],
            EqMsg::HmFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqBand3(val),
            )],
            EqMsg::HmGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainBand3(val),
            )],
            EqMsg::HmQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWBand3(val),
            )],
            EqMsg::HighFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqHighShelf4(val),
            )],
            EqMsg::HighGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainHighShelf4(val),
            )],
            EqMsg::HighQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWHighShelf4(val),
            )],
            _ => vec![],
        }
    }

    fn from_track_typed(&self, msg: crate::track::track::FXParamValue) -> Vec<EqMsg> {
        match rea_eq::decode_trackmsg(msg) {
            Some(rea_eq::Param::FreqLowShelf(val)) => vec![EqMsg::LowFreq(val)],
            Some(rea_eq::Param::GainLowShelf(val)) => vec![EqMsg::LowGain(val)],
            Some(rea_eq::Param::BWLowShelf(val)) => vec![EqMsg::LowQ(val)],
            Some(rea_eq::Param::FreqBand2(val)) => vec![EqMsg::LmFreq(val)],
            Some(rea_eq::Param::GainBand2(val)) => vec![EqMsg::LmGain(val)],
            Some(rea_eq::Param::BWBand2(val)) => vec![EqMsg::LmQ(val)],
            Some(rea_eq::Param::FreqBand3(val)) => vec![EqMsg::HmFreq(val)],
            Some(rea_eq::Param::GainBand3(val)) => vec![EqMsg::HmGain(val)],
            Some(rea_eq::Param::BWBand3(val)) => vec![EqMsg::HmQ(val)],
            Some(rea_eq::Param::FreqHighShelf4(val)) => vec![EqMsg::HighFreq(val)],
            Some(rea_eq::Param::GainHighShelf4(val)) => vec![EqMsg::HighGain(val)],
            Some(rea_eq::Param::BWHighShelf4(val)) => vec![EqMsg::HighQ(val)],
            _ => vec![],
        }
    }
}
