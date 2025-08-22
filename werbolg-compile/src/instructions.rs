use super::code::InstructionDiff;
use werbolg_core::{ConstrId, FunId, GlobalId, LitId, NifId};

/// Instruction for execution
#[derive(Clone, Debug)]
pub enum Instruction {
    /// Push a literal value on the stack
    PushLiteral(LitId),
    /// Fetch from the global values array
    FetchGlobal(GlobalId),
    /// Fetch from the nifs array
    FetchNif(NifId),
    /// Fetch from the fun array
    FetchFun(FunId),
    /// Fetch from the callstack param (which is relative and before SP)
    FetchStackParam(ParamBindIndex),
    /// Fetch from the localstack values (which is relative and after SP)
    FetchStackLocal(LocalBindIndex),
    /// Access a field in a structure value as stack\[top\]
    AccessField(ConstrId, StructFieldIndex),
    /// Bind Locally a value
    LocalBind(LocalBindIndex),
    /// Ignore a value from the stack
    IgnoreOne,
    /// Call the function on the stack with the N value in arguments.
    ///
    /// expecting N+1 value on the value stack
    Call(TailCall, CallArity),
    /// Call the Nif function specified in the variant with the N value in arguments.
    ///
    /// expecting N value on the value stack, as the NifId is embedded in the instruction
    CallNif(NifId, CallArity),
    /// Jump by N instructions
    Jump(InstructionDiff),
    /// Jump by N instructions if stack\[top\] is true
    CondJump(InstructionDiff),
    /// Return from call
    Ret,
}

/// Whether or not the call is at the tail of a block and can be optimised
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TailCall {
    /// The call is at the end of a function block
    Yes,
    /// The call is not at the end of a function block
    No,
}

/// The index of locally (in the context of a function) bind value
///
/// This is limited (arbitrarily) to a maximum of 65535 values
#[derive(Clone, Copy, Debug)]
pub struct LocalBindIndex(pub u16);

/// the index of function parameter
#[derive(Clone, Copy, Debug)]
pub struct ParamBindIndex(pub u8);

/// A field in a structured indexed by its order in the structure
///
/// This is limited (arbitrarily) to a maximum of 255
#[derive(Clone, Copy, Debug)]
pub struct StructFieldIndex(pub u8);

/// The arity (number of parameter) of a function.
///
/// This is limited (arbitrarily) to a maximum of 255
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallArity(pub u8);

impl TryFrom<usize> for CallArity {
    type Error = usize;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value.try_into().map(|v| CallArity(v)).map_err(|_| value)
    }
}

impl TryFrom<u32> for CallArity {
    type Error = usize;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map(|v| CallArity(v))
            .map_err(|_| value as usize)
    }
}
