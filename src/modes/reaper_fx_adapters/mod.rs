use uuid::Uuid;

use crate::modes::reaper_channel_strip_router::RoutableMsg;
use crate::track;

pub mod rea_eq;
pub mod tdr_molotok;
pub mod tdrvos_slick_eq;

pub trait FxAdapterDyn: Send + Sync {
    fn id(&self) -> &'static str; // "rea_eq", "fabfilter_pro_q3", etc.
    fn fx_names(&self) -> &'static [&'static str]; // acceptable plugin display names
    fn to_track_dyn(&self, track: Uuid, fx_index: i32, msg: RoutableMsg) -> Vec<track::TrackMsg>;
    fn from_track_dyn(&self, msg: track::FXParamValue) -> Vec<RoutableMsg>;
}

pub trait FxAdapterTyped {
    type Msg: Copy; // EqMsg, CompMsg, etc.
    fn id(&self) -> &'static str;
    fn fx_names(&self) -> &'static [&'static str];
    fn to_track_typed(&self, track: Uuid, fx_index: i32, msg: Self::Msg) -> Vec<track::TrackMsg>;
    fn from_track_typed(&self, msg: track::FXParamValue) -> Vec<Self::Msg>;
}

/// Generates `FxAdapterDyn` impl by bridging a typed adapter (`FxAdapterTyped`) to a
/// specific `RoutableMsg` variant.
///
/// Usage:
/// impl_fx_adapter_dyn_bridge!(ReaEqAdapter, Eq);
/// impl_fx_adapter_dyn_bridge!(MolotokAdapter, Comp);
///
/// Assumptions:
/// - `RoutableMsg::{Eq, Comp, ...}` variants are tuple variants with one payload.
/// - The target adapter implements `FxAdapterTyped` where `type Msg` matches that payload.
/// - `FxAdapterDyn`, `FxAdapterTyped`, and `RoutableMsg` are in scope.
///
/// You can place this macro near your adapter traits module and `pub(crate) use` it.
#[macro_export]
macro_rules! impl_fx_adapter_dyn_bridge {
    ($adapter:ty, $variant:ident) => {
        impl FxAdapterDyn for $adapter {
            #[inline]
            fn id(&self) -> &'static str {
                <Self as FxAdapterTyped>::id(self)
            }

            #[inline]
            fn fx_names(&self) -> &'static [&'static str] {
                <Self as FxAdapterTyped>::fx_names(self)
            }

            #[inline]
            fn to_track_dyn(
                &self,
                track: uuid::Uuid,
                fx_index: i32,
                msg: RoutableMsg,
            ) -> Vec<crate::track::TrackMsg> {
                match msg {
                    RoutableMsg::$variant(inner) => {
                        <Self as FxAdapterTyped>::to_track_typed(self, track, fx_index, inner)
                    }
                    _ => Vec::new(), // not this adapter's domain
                }
            }

            #[inline]
            fn from_track_dyn(&self, msg: crate::track::FXParamValue) -> Vec<RoutableMsg> {
                <Self as FxAdapterTyped>::from_track_typed(self, msg)
                    .into_iter()
                    .map(RoutableMsg::$variant)
                    .collect()
            }
        }
    };
}

impl_fx_adapter_dyn_bridge!(rea_eq::ReaEqAdapter, Eq);
impl_fx_adapter_dyn_bridge!(tdrvos_slick_eq::TdrvosSlickEqAdapter, Eq);
impl_fx_adapter_dyn_bridge!(tdr_molotok::TdrMolotokAdapter, Comp);
