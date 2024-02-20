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

/// Code to execute as an array of instruction, generated instructions by instructions
/// with potential placeholder references `CodeRef`, which we only track how many of them
/// have not been resolved
pub struct Code {
    stmts: IdVec<InstructionAddress, Instruction>,
    temps: usize,
}

/// placeholder instruction reference
pub struct CodeRef(InstructionAddress);

/// A displacement type between instruction. i.e. the number of element between 2 different InstructionAddress
#[derive(Debug, Copy, Clone)]
pub struct InstructionDiff(u32);

impl Code {
    /// Create a new empty Code builder
    pub fn new() -> Self {
        Self {
            stmts: IdVec::new(),
            temps: 0,
        }
    }

    /// Append a new instruction at the end of the current instructions
    pub fn push(&mut self, stmt: Instruction) {
        self.stmts.push(stmt);
    }

    /// Return the position of the next instruction
    pub fn position(&self) -> InstructionAddress {
        InstructionAddress(self.stmts.next_id().0)
    }

    /// Push a dummy instruction into the instruction and return a `CodeRef` to
    /// replace the instruction later using `resolve_temp`
    #[must_use]
    pub fn push_temp(&mut self) -> CodeRef {
        let r = self.position();
        self.stmts.push(Instruction::IgnoreOne);
        self.temps += 1;
        CodeRef(r)
    }

    /// Resolve the temporary instruction, replacing it by the final instruction
    ///
    /// This also reduce the number of temporary instruction by one
    ///
    /// Note: that if a temporary is resolve multiple time, which should be difficult
    /// by design, since the CodeRef is not clone/copy and passed by value,
    /// this would mess up the counter of temps
    pub fn resolve_temp(&mut self, r: CodeRef, stmt: Instruction) {
        self.stmts[r.0] = stmt;
        self.temps -= 1;
    }

    /// finalize the code into just the instructions vector
    ///
    /// this function will panic if the code cannot be finalized, as there
    /// some unresolved instructions (temps > 0)
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
