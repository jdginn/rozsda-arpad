pub mod _1175compressor;
pub mod rea_comp;
pub mod rea_eq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxId {
    ReaComp,
    ReaEQ,
    _1175Compressor,
}
