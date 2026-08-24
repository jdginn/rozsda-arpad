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
    ) -> Option<crate::track::track::TrackMsg> {
        match msg {
            ChannelStripMsg::LowFreq(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqLowShelf(val),
            )),
            ChannelStripMsg::LowGain(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainLowShelf(val),
            )),
            ChannelStripMsg::LowQ(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWLowShelf(val),
            )),
            ChannelStripMsg::LmFreq(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqBand2(val),
            )),
            ChannelStripMsg::LmGain(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainBand2(val),
            )),
            ChannelStripMsg::LmQ(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWBand2(val),
            )),
            ChannelStripMsg::HmFreq(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqBand3(val),
            )),
            ChannelStripMsg::HmGain(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainBand3(val),
            )),
            ChannelStripMsg::HmQ(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWBand3(val),
            )),
            ChannelStripMsg::HighFreq(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::FreqHighShelf4(val),
            )),
            ChannelStripMsg::HighGain(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::GainHighShelf4(val),
            )),
            ChannelStripMsg::HighQ(val) => Some(rea_eq::encode_trackmsg(
                track,
                fx_index,
                rea_eq::Param::BWHighShelf4(val),
            )),
            _ => None,
        }
    }

    fn from_track(&self, msg: crate::track::track::FXParamValue) -> Option<ChannelStripMsg> {
        match rea_eq::decode_trackmsg(msg) {
            Some(rea_eq::Param::FreqLowShelf(val)) => Some(ChannelStripMsg::LowFreq(val)),
            Some(rea_eq::Param::GainLowShelf(val)) => Some(ChannelStripMsg::LowGain(val)),
            Some(rea_eq::Param::BWLowShelf(val)) => Some(ChannelStripMsg::LowQ(val)),
            Some(rea_eq::Param::FreqBand2(val)) => Some(ChannelStripMsg::LmFreq(val)),
            Some(rea_eq::Param::GainBand2(val)) => Some(ChannelStripMsg::LmGain(val)),
            Some(rea_eq::Param::BWBand2(val)) => Some(ChannelStripMsg::LmQ(val)),
            Some(rea_eq::Param::FreqBand3(val)) => Some(ChannelStripMsg::HmFreq(val)),
            Some(rea_eq::Param::GainBand3(val)) => Some(ChannelStripMsg::HmGain(val)),
            Some(rea_eq::Param::BWBand3(val)) => Some(ChannelStripMsg::HmQ(val)),
            Some(rea_eq::Param::FreqHighShelf4(val)) => Some(ChannelStripMsg::HighFreq(val)),
            Some(rea_eq::Param::GainHighShelf4(val)) => Some(ChannelStripMsg::HighGain(val)),
            Some(rea_eq::Param::BWHighShelf4(val)) => Some(ChannelStripMsg::HighQ(val)),
            _ => None,
        }
    }
}
