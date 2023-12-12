use super::id::IdRemapper;
use super::lir;
use super::symbols::{IdVec, IdVecAfter};

#[derive(Clone, Copy, Debug)]
pub struct InstructionAddress(u32);

impl IdRemapper for InstructionAddress {
    fn uncat(self) -> crate::Id {
        crate::Id(self.0)
    }

    fn cat(id: crate::Id) -> Self {
        InstructionAddress(id.0)
    }
}

impl core::ops::AddAssign<InstructionDiff> for InstructionAddress {
    fn add_assign(&mut self, rhs: InstructionDiff) {
        self.0 += rhs.0
    }
}

pub struct Code {
    stmts: IdVec<InstructionAddress, lir::Statement>,
    temps: usize,
}

/// placeholder instruction
pub struct CodeRef(InstructionAddress);

#[derive(Copy, Clone)]
pub struct InstructionDiff(u32);

impl Code {
    pub fn new() -> Self {
        Self {
            stmts: IdVec::new(),
            temps: 0,
        }
    }

    pub fn push(&mut self, stmt: lir::Statement) {
        self.stmts.push(stmt);
    }

    pub fn position(&self) -> InstructionAddress {
        InstructionAddress(self.stmts.next_id().0)
    }

    pub fn push_temp(&mut self) -> CodeRef {
        let r = self.position();
        self.stmts.push(lir::Statement::CondJump(0));
        self.temps += 1;
        CodeRef(r)
    }

    pub fn resolve_temp(&mut self, r: CodeRef, stmt: lir::Statement) {
        self.stmts[r.0] = stmt;
        self.temps -= 1;
    }

    pub fn merge(&mut self, later: Code) -> InstructionDiff {
        let ofs = self.stmts.next_id();
        self.stmts
            .concat(&mut IdVecAfter::from_idvec(later.stmts, ofs));
        InstructionDiff(ofs.0)
    }
}
