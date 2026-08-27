use crate::modes::reaper_channel_strip_router::{BandMode, EqMsg};
use crate::modes::reaper_fx::tdrvos_slick_eq_au;
use crate::modes::reaper_fx_adapters::FxAdapterTyped;

pub struct TdrvosSlickEqAdapter;

impl FxAdapterTyped for TdrvosSlickEqAdapter {
    type Msg = EqMsg;

    fn id(&self) -> &'static str {
        tdrvos_slick_eq_au::FX_NAME
    }

    fn fx_names(&self) -> &'static [&'static str] {
        &[tdrvos_slick_eq_au::FX_NAME]
    }

    fn to_track_typed(
        &self,
        track: uuid::Uuid,
        fx_index: i32,
        msg: EqMsg,
    ) -> Vec<crate::track::TrackMsg> {
        match msg {
            EqMsg::LowFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWFreq(val),
            )],
            EqMsg::LowQ(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWShape(val > 0.5),
            )],
            EqMsg::LowSlope(_) => vec![],
            EqMsg::LowBandMode(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWShape(val == BandMode::Shelf),
            )],
            EqMsg::LowGain(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::LOWGain(val),
            )],
            EqMsg::LmFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::MIDFreq(val),
            )],
            EqMsg::LmGain(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::MIDGain(val),
            )],
            EqMsg::LmQ(_) => vec![],
            EqMsg::HmFreq(_) => vec![],
            EqMsg::HmGain(_) => vec![],
            EqMsg::HmQ(_) => vec![],
            EqMsg::HighFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::HIGHFreq(val),
            )],
            EqMsg::HighQ(_) => vec![],
            EqMsg::HighSlope(_) => vec![],
            EqMsg::HighBandMode(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::HIGHShape(val == BandMode::Shelf),
            )],
            EqMsg::HighGain(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::HIGHGain(val),
            )],
            EqMsg::HighSidesGain(_) => vec![],
            EqMsg::HpfFreq(val) => vec![tdrvos_slick_eq_au::encode_trackmsg(
                track,
                fx_index,
                tdrvos_slick_eq_au::Param::HPFreq(val),
            )],
            EqMsg::HpfSlope(_) => vec![],
            EqMsg::EqType(_) => vec![], //TODO: we should actually use this
        }
    }

    fn from_track_typed(&self, msg: crate::track::FXParamValue) -> Vec<Self::Msg> {
        match tdrvos_slick_eq_au::decode_trackmsg(msg) {
            Some(tdrvos_slick_eq_au::Param::LOWFreq(val)) => vec![EqMsg::LowFreq(val)],
            Some(tdrvos_slick_eq_au::Param::LOWGain(val)) => vec![EqMsg::LowGain(val)],
            Some(tdrvos_slick_eq_au::Param::LOWShape(val)) => {
                vec![EqMsg::LowBandMode(if val {
                    BandMode::Shelf
                } else {
                    BandMode::Bell
                })]
            }
            Some(tdrvos_slick_eq_au::Param::MIDFreq(val)) => vec![EqMsg::LmFreq(val)],
            Some(tdrvos_slick_eq_au::Param::MIDGain(val)) => vec![EqMsg::LmGain(val)],
            Some(tdrvos_slick_eq_au::Param::HIGHFreq(val)) => vec![EqMsg::HighFreq(val)],
            Some(tdrvos_slick_eq_au::Param::HIGHShape(val)) => {
                vec![EqMsg::HighBandMode(if val {
                    BandMode::Shelf
                } else {
                    BandMode::Bell
                })]
            }
            Some(tdrvos_slick_eq_au::Param::HIGHGain(val)) => vec![EqMsg::HighGain(val)],
            Some(tdrvos_slick_eq_au::Param::HPFreq(val)) => vec![EqMsg::HpfFreq(val)],
            _ => vec![],
        }
    }
}
