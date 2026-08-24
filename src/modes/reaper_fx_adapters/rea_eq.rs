use crate::modes::reaper_channel_strip_router::ChannelStripMsg;
use crate::modes::reaper_fx::rea_eq;
use crate::modes::reaper_fx_adapters::FxAdapter;

pub struct ReaEqAdapter;

impl FxAdapter for ReaEqAdapter {
    fn id(&self) -> &'static str {
        rea_eq::name()
    }

    fn fx_names(&self) -> Vec<&'static str> {
        vec![rea_eq::name()]
    }

    fn to_track(
        &self,
        track: uuid::Uuid,
        fx_index: i32,
        msg: ChannelStripMsg,
    ) -> Vec<crate::track::track::TrackMsg> {
        match msg {
            ChannelStripMsg::LowFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqLowShelf(val),
            )],
            ChannelStripMsg::LowGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainLowShelf(val),
            )],
            ChannelStripMsg::LowQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWLowShelf(val),
            )],
            ChannelStripMsg::LmFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqBand2(val),
            )],
            ChannelStripMsg::LmGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainBand2(val),
            )],
            ChannelStripMsg::LmQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWBand2(val),
            )],
            ChannelStripMsg::HmFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqBand3(val),
            )],
            ChannelStripMsg::HmGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainBand3(val),
            )],
            ChannelStripMsg::HmQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWBand3(val),
            )],
            ChannelStripMsg::HighFreq(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqHighShelf4(val),
            )],
            ChannelStripMsg::HighGain(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainHighShelf4(val),
            )],
            ChannelStripMsg::HighQ(val) => vec![rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWHighShelf4(val),
            )],
            _ => vec![],
        }
    }

    fn from_track(&self, msg: crate::track::track::FXParamValue) -> Vec<ChannelStripMsg> {
        match rea_eq::decode_trackmsg(msg) {
            Some(rea_eq::Param::FreqLowShelf(val)) => vec![ChannelStripMsg::LowFreq(val)],
            Some(rea_eq::Param::GainLowShelf(val)) => vec![ChannelStripMsg::LowGain(val)],
            Some(rea_eq::Param::BWLowShelf(val)) => vec![ChannelStripMsg::LowQ(val)],
            Some(rea_eq::Param::FreqBand2(val)) => vec![ChannelStripMsg::LmFreq(val)],
            Some(rea_eq::Param::GainBand2(val)) => vec![ChannelStripMsg::LmGain(val)],
            Some(rea_eq::Param::BWBand2(val)) => vec![ChannelStripMsg::LmQ(val)],
            Some(rea_eq::Param::FreqBand3(val)) => vec![ChannelStripMsg::HmFreq(val)],
            Some(rea_eq::Param::GainBand3(val)) => vec![ChannelStripMsg::HmGain(val)],
            Some(rea_eq::Param::BWBand3(val)) => vec![ChannelStripMsg::HmQ(val)],
            Some(rea_eq::Param::FreqHighShelf4(val)) => vec![ChannelStripMsg::HighFreq(val)],
            Some(rea_eq::Param::GainHighShelf4(val)) => vec![ChannelStripMsg::HighGain(val)],
            Some(rea_eq::Param::BWHighShelf4(val)) => vec![ChannelStripMsg::HighQ(val)],
            _ => vec![],
        }
    }
}
