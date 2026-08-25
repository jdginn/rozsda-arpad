use crate::track::track;
use uuid::Uuid;

pub mod _1175compressor;
pub mod mju_cjr_au;
pub mod mju_cjr_vst;
pub mod rea_comp;
pub mod rea_eq;
pub mod tdr_molotok_au;
pub mod tdr_molotok_vst;
pub mod tdr_nova_au;
pub mod tdr_nova_vst;
pub mod tdrvos_slick_eq_au;
pub mod tdrvos_slick_eq_vst;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxId {
    MJUCjr_AU,
    MJUCjr_VST,
    ReaComp,
    ReaEQ,
    TDRMolotok_AU,
    TDRMolotok_VST,
    TDRNova_AU,
    TDRNova_VST,
    TDRVOSSlickEQ_AU,
    TDRVOSSlickEQ_VST,
    _1175Compressor,
}
