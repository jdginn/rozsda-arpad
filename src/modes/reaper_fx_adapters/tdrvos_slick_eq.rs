use crate::modes::reaper_channel_strip_router::ChannelStripMsg;
use crate::modes::reaper_fx::tdrvos_slick_eq_au;
use crate::modes::reaper_fx_adapters::FxAdapter;

pub struct TdrvosSlickEqAdapter;

impl FxAdapter for TdrvosSlickEqAdapter {
    fn id(&self) -> &'static str {
        tdrvos_slick_eq_au::name()
    }

    fn fx_names(&self) -> Vec<&'static str> {
        vec![tdrvos_slick_eq_au::name()]
    }

    fn to_track(
        &self,
        track: uuid::Uuid,
        fx_index: i32,
        msg: ChannelStripMsg,
    ) -> Vec<crate::track::track::TrackMsg> {
        match msg {
            ChannelStripMsg::LowFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWFreq(val),
            )],
            ChannelStripMsg::LowGain(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWGain(val),
            )],
            ChannelStripMsg::LowQ(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWShape(val > 0.5),
            )],
            ChannelStripMsg::LmFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::MIDFreq(val),
            )],
            ChannelStripMsg::LmGain(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::MIDGain(val),
            )],
            ChannelStripMsg::LmQ(val) => vec![],
            ChannelStripMsg::HmFreq(val) => vec![],
            ChannelStripMsg::HmGain(val) => vec![],
            ChannelStripMsg::HmQ(val) => vec![],
            ChannelStripMsg::HighFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::HIGHFreq(val),
            )],
            ChannelStripMsg::HighGain(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::HIGHGain(val),
            )],
            ChannelStripMsg::HighQ(val) => vec![],
            _ => vec![],
        }
    }
}
