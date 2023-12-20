use super::code::InstructionAddress;
use super::instructions::*;
use alloc::vec::Vec;
use werbolg_core::{ConstrId, Ident};

/// Local stack size (in unit of values)
#[derive(Copy, Clone, Debug)]
pub struct LocalStackSize(pub u32);

/// Function definition
///
/// For anonymous function the name is None
#[derive(Clone, Debug)]
pub struct FunDef {
    /// name of the function. anonymous function has no name
    pub name: Option<Ident>,
    /// Arity of the function.
    pub arity: CallArity,
    /// The local stack size needed for this function
    pub stack_size: LocalStackSize,
    /// The address of the first instruction (entry point) for this function
    pub code_pos: InstructionAddress,
}

/// Structure definition
#[derive(Clone, Debug)]
pub struct StructDef {
    /// name of this structure
    pub name: Ident,
    /// name of the fields
    pub fields: Vec<Ident>,
}

impl StructDef {
    /// Try to find the index for a given field
    pub fn find_field_index(&self, ident: &Ident) -> Option<StructFieldIndex> {
        self.fields
            .iter()
            .position(|x| x == ident)
            .map(|x| StructFieldIndex(x as u8))
    }
}

/// Enumeration definition
#[derive(Clone, Debug)]
pub struct EnumDef {
    /// name of this enumeration
    pub name: Ident,
    /// The variants for this enumeration
    pub variants: Vec<Variant>,
}

/// Constructor definition (enumeration or struct)
#[derive(Clone, Debug)]
pub enum ConstrDef {
    /// Struct variant of a constructor
    Struct(StructDef),
    /// Enumeration variant of a constructor
    Enum(EnumDef),
}

/// Enumeration Variant type
#[derive(Clone, Debug)]
pub struct Variant {
    /// Name of this variant
    pub name: Ident,
    /// Constructor Id for the content
    pub constr: ConstrId,
}
