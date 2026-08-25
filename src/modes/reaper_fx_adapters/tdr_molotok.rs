use crate::modes::reaper_channel_strip_router::ChannelStripMsg;
use crate::modes::reaper_fx::tdr_molotok_au;
use crate::modes::reaper_fx_adapters::FxAdapter;

pub struct TdrMolotokAdapter;

impl FxAdapter for TdrMolotokAdapter {
    fn id(&self) -> &'static str {
        tdr_molotok_au::name()
    }

    fn fx_names(&self) -> Vec<&'static str> {
        vec![tdr_molotok_au::name()]
    }

    fn to_track(
        &self,
        track: uuid::Uuid,
        fx_index: i32,
        msg: ChannelStripMsg,
    ) -> Vec<crate::track::track::TrackMsg> {
        match msg {
            ChannelStripMsg::CompThresh(val) | ChannelStripMsg::Comp2Thresh(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Thresh(val),
                )]
            }
            ChannelStripMsg::CompRatio(val) | ChannelStripMsg::Comp2Ratio(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Ratio(val),
                )]
            }
            ChannelStripMsg::CompAttack(val) | ChannelStripMsg::Comp2Attack(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Attack(val),
                )]
            }
            ChannelStripMsg::CompRelease(val) | ChannelStripMsg::Comp2Release(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Release(val),
                )]
            }
            ChannelStripMsg::CompMakeup(val) | ChannelStripMsg::Comp2Makeup(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Makeup(val),
                )]
            }
            ChannelStripMsg::CompScFilter(val) | ChannelStripMsg::Comp2ScFilter(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::SCFilterFreq(val),
                )]
            }
            _ => vec![],
        }
    }

    fn from_track(&self, msg: crate::track::track::FXParamValue) -> Vec<ChannelStripMsg> {
        match tdr_molotok_au::decode_trackmsg(msg) {
            Some(tdr_molotok_au::Param::Thresh(val)) => vec![ChannelStripMsg::CompThresh(val)],
            Some(tdr_molotok_au::Param::Ratio(val)) => vec![ChannelStripMsg::CompRatio(val)],
            Some(tdr_molotok_au::Param::Attack(val)) => vec![ChannelStripMsg::CompAttack(val)],
            Some(tdr_molotok_au::Param::Release(val)) => vec![ChannelStripMsg::CompRelease(val)],
            Some(tdr_molotok_au::Param::Makeup(val)) => vec![ChannelStripMsg::CompMakeup(val)],
            Some(tdr_molotok_au::Param::SCFilterFreq(val)) => {
                vec![ChannelStripMsg::CompScFilter(val)]
            }
            _ => vec![],
        }
    }
}
