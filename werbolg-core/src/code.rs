use hashbrown::HashMap;

use super::id::IdRemapper;
use super::lir;
use super::symbols::{IdVec, IdVecAfter};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct InstructionAddress(u32);

impl InstructionAddress {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

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
    stmts: IdVec<InstructionAddress, lir::Statement>,
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

    pub fn push(&mut self, stmt: lir::Statement) {
        self.stmts.push(stmt);
    }

    pub fn position(&self) -> InstructionAddress {
        InstructionAddress(self.stmts.next_id().0)
    }

    pub fn push_temp(&mut self) -> CodeRef {
        let r = self.position();
        self.stmts.push(lir::Statement::IgnoreOne);
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

    pub fn finalize(self) -> IdVec<InstructionAddress, lir::Statement> {
        self.stmts
    }
}

use crate::{lir::FunDef, FunId};

pub fn code_dump(code: &IdVec<InstructionAddress, lir::Statement>, fundefs: &IdVec<FunId, FunDef>) {
    let mut place = HashMap::new();
    for (funid, fundef) in fundefs.iter() {
        place.insert(fundef.code_pos, funid);
    }

    for (ia, stmt) in code.iter() {
        if let Some(funid) = place.get(&ia) {
            let fundef = &fundefs[*funid];
            println!(
                "[{} local-stack={}]",
                fundef
                    .name
                    .as_ref()
                    .map(|n| format!("{:?}", n))
                    .unwrap_or(format!("{:?}", funid)),
                fundef.stack_size.0
            );
        }
        println!("{}  {:?}", ia, stmt)
    }
}
