use crate::modes::reaper_channel_strip_router::CompMsg;
use crate::modes::reaper_fx::tdr_molotok_au;
use crate::modes::reaper_fx_adapters::FxAdapterTyped;

pub struct TdrMolotokAdapter;

impl FxAdapterTyped for TdrMolotokAdapter {
    type Msg = CompMsg;

    fn id(&self) -> &'static str {
        tdr_molotok_au::FX_NAME
    }

    fn fx_names(&self) -> &'static [&'static str] {
        &[tdr_molotok_au::FX_NAME]
    }

    fn to_track_typed(
        &self,
        track: uuid::Uuid,
        fx_index: i32,
        msg: CompMsg,
    ) -> Vec<crate::track::track::TrackMsg> {
        match msg {
            CompMsg::CompThresh(val) | CompMsg::Comp2Thresh(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Thresh(val),
                )]
            }
            CompMsg::CompRatio(val) | CompMsg::Comp2Ratio(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Ratio(val),
                )]
            }
            CompMsg::CompAttack(val) | CompMsg::Comp2Attack(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Attack(val),
                )]
            }
            CompMsg::CompRelease(val) | CompMsg::Comp2Release(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Release(val),
                )]
            }
            CompMsg::CompMakeup(val) | CompMsg::Comp2Makeup(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::Makeup(val),
                )]
            }
            CompMsg::CompScFilter(val) | CompMsg::Comp2ScFilter(val) => {
                vec![tdr_molotok_au::encode_trackmsg(
                    track,
                    fx_index,
                    tdr_molotok_au::Param::SCFilterFreq(val),
                )]
            }
            _ => vec![],
        }
    }

    fn from_track_typed(&self, msg: crate::track::track::FXParamValue) -> Vec<CompMsg> {
        match tdr_molotok_au::decode_trackmsg(msg) {
            Some(tdr_molotok_au::Param::Thresh(val)) => vec![CompMsg::CompThresh(val)],
            Some(tdr_molotok_au::Param::Ratio(val)) => vec![CompMsg::CompRatio(val)],
            Some(tdr_molotok_au::Param::Attack(val)) => vec![CompMsg::CompAttack(val)],
            Some(tdr_molotok_au::Param::Release(val)) => vec![CompMsg::CompRelease(val)],
            Some(tdr_molotok_au::Param::Makeup(val)) => vec![CompMsg::CompMakeup(val)],
            Some(tdr_molotok_au::Param::SCFilterFreq(val)) => {
                vec![CompMsg::CompScFilter(val)]
            }
            _ => vec![],
        }
    }
}
