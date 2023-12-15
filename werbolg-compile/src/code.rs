use super::instructions::Instruction;
use super::symbols::{IdVec, IdVecAfter};
use werbolg_core::id::{Id, IdRemapper};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct InstructionAddress(u32);

impl InstructionAddress {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl IdRemapper for InstructionAddress {
    fn uncat(self) -> Id {
        Id(self.0)
    }

    fn cat(id: Id) -> Self {
        InstructionAddress(id.0)
    }
}

impl core::ops::AddAssign<InstructionDiff> for InstructionAddress {
    fn add_assign(&mut self, rhs: InstructionDiff) {
        self.0 += rhs.0
    }
}

impl core::ops::Sub for InstructionAddress {
    type Output = InstructionDiff;

    fn sub(self, rhs: Self) -> Self::Output {
        InstructionDiff(self.0 - rhs.0)
    }
}

impl core::fmt::Display for InstructionAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04x}_{:04x}", self.0 >> 16, self.0 & 0xffff)
    }
}

pub struct Code {
    stmts: IdVec<InstructionAddress, Instruction>,
    temps: usize,
}

/// placeholder instruction
pub struct CodeRef(InstructionAddress);

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

    pub fn merge(&mut self, later: Code) -> InstructionDiff {
        let ofs = self.stmts.next_id();
        self.stmts
            .concat(&mut IdVecAfter::from_idvec(later.stmts, ofs));
        InstructionDiff(ofs.0)
    }

    pub fn finalize(self) -> IdVec<InstructionAddress, Instruction> {
        self.stmts
    }
}
