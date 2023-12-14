use crate::symbols::SymbolsTableData;

use super::basic::Ident;
use super::bindings::Bindings;
use super::id::{FunId, GlobalId, NifId};
use super::symbols::IdVec;

#[derive(Clone, Copy, Debug)]
pub enum ValueFun {
    Native(NifId),
    Fun(FunId),
}

pub struct Environment {
    pub global: IdVec<GlobalId, ValueFun>,
    pub symbols: SymbolsTableData<NifId, (Ident, GlobalId)>,
    pub global_index: u32,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            symbols: SymbolsTableData::new(),
            global: IdVec::new(),
            global_index: 0,
        }
    }

    pub fn add(&mut self, ident: Ident) -> NifId {
        let global_id = self.global.next_id();
        let nif = self.symbols.add(ident.clone(), (ident, global_id)).unwrap();
        self.global.push(ValueFun::Native(nif));
        self.global_index += 1;
        nif
    }

    pub fn add_global(&mut self, fun: FunId) -> GlobalId {
        let global_id = self.global.push(ValueFun::Fun(fun));
        self.global_index += 1;
        global_id
    }

    pub fn finalize(self) -> () {}
}
