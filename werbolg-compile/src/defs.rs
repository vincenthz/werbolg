use super::code::InstructionAddress;
use super::instructions::*;
use werbolg_core::{ConstrId, Ident};

#[derive(Copy, Clone, Debug)]
pub struct LocalStackSize(pub u32);

#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Option<Ident>,
    pub arity: CallArity,
    pub stack_size: LocalStackSize,
    pub code_pos: InstructionAddress,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Ident,
    pub fields: Vec<Ident>,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: Ident,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub enum ConstrDef {
    Struct(StructDef),
    Enum(EnumDef),
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: Ident,
    pub constr: ConstrId,
}
