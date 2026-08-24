use uuid::Uuid;

use crate::modes::reaper_channel_strip_router::ChannelStripMsg;
use crate::track::track;

pub mod rea_eq;

pub trait FxAdapter: Send + Sync {
    fn id(&self) -> &'static str; // "rea_eq", "fabfilter_pro_q3", etc.
    fn fx_names(&self) -> Vec<&'static str>; // acceptable plugin display names
    fn to_track(&self, track: Uuid, fx_index: i32, msg: ChannelStripMsg) -> Vec<track::TrackMsg>;
    fn from_track(&self, msg: track::FXParamValue) -> Vec<ChannelStripMsg>;
}
