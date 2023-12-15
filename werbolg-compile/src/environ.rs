use super::symbols::IdVec;
use crate::symbols::SymbolsTableData;
use werbolg_core::{FunId, GlobalId, Ident, NifId, ValueFun};

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
