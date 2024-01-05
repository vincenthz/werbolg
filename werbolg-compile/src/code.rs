use super::instructions::Instruction;
use super::symbols::IdVec;
use werbolg_core::id::{IdArith, IdF};

/// Instruction Address
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionAddress(u32);

impl Default for InstructionAddress {
    fn default() -> Self {
        Self::from_collection_len(0)
    }
}

impl InstructionAddress {
    /// Increment the instruction address to the next instruction
    pub fn next(self) -> Self {
        InstructionAddress::add(self, InstructionDiff(1))
    }
}

impl IdF for InstructionAddress {
    fn as_index(self) -> usize {
        self.0 as usize
    }

    fn from_slice_len<T>(slice: &[T]) -> Self {
        Self(slice.len() as u32)
    }

    fn from_collection_len(len: usize) -> Self {
        Self(len as u32)
    }

    fn remap(left: Self, right: Self) -> Self {
        Self(left.0 + right.0)
    }
}
impl IdArith for InstructionAddress {
    type IdDiff = InstructionDiff;

    fn add(left: Self, right: InstructionDiff) -> Self {
        Self(left.0.checked_add(right.0).expect("ID valid add"))
    }

    fn diff(left: Self, right: Self) -> InstructionDiff {
        InstructionDiff(left.0.checked_sub(right.0).expect("ID valid diff"))
    }
}

impl core::ops::AddAssign<InstructionDiff> for InstructionAddress {
    fn add_assign(&mut self, rhs: InstructionDiff) {
        *self = InstructionAddress::add(*self, rhs)
    }
}

impl core::ops::Sub for InstructionAddress {
    type Output = InstructionDiff;

    fn sub(self, rhs: Self) -> Self::Output {
        InstructionAddress::diff(self, rhs)
    }
}

impl core::fmt::Display for InstructionAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let idx = self.as_index();
        write!(f, "{:04x}_{:04x}", idx >> 16, idx & 0xffff)
    }
}

pub struct Code {
    stmts: IdVec<InstructionAddress, Instruction>,
    temps: usize,
}

/// placeholder instruction
pub struct CodeRef(InstructionAddress);

/// A displacement type between instruction. i.e. the number of element between 2 different InstructionAddress
#[derive(Debug, Copy, Clone)]
pub struct InstructionDiff(u32);

impl Code {
    pub fn new() -> Self {
        Self {
            stmts: IdVec::new(),
            temps: 0,
        }
    }

    pub fn push(&mut self, stmt: Instruction) {
        self.stmts.push(stmt);
    }

    pub fn position(&self) -> InstructionAddress {
        InstructionAddress(self.stmts.next_id().0)
    }

    pub fn push_temp(&mut self) -> CodeRef {
        let r = self.position();
        self.stmts.push(Instruction::IgnoreOne);
        self.temps += 1;
        CodeRef(r)
    }

    pub fn resolve_temp(&mut self, r: CodeRef, stmt: Instruction) {
        self.stmts[r.0] = stmt;
        self.temps -= 1;
    }

    pub fn finalize(self) -> IdVec<InstructionAddress, Instruction> {
        if self.temps > 0 {
            panic!(
                "internal error: temporary code is still in place : {} instances",
                self.temps
            )
        }
        self.stmts
    }
}
