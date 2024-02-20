use crate::instructions::{LocalBindIndex, ParamBindIndex};
use werbolg_core::{FunId, GlobalId, NifId};

#[derive(Clone, Copy)]
pub enum BindingType {
    Global(GlobalId),
    Nif(NifId),
    Fun(FunId),
    Param(ParamBindIndex),
    Local(LocalBindIndex),
}
