use super::symbols::IdVec;
use crate::symbols::SymbolsTableData;
use werbolg_core::{FunId, GlobalId, Ident, NifId, ValueFun};

/// Environment of the compilation
pub struct Environment {
    /// All the global values defined
    pub global: IdVec<GlobalId, ValueFun>,
    /// ?
    pub symbols: SymbolsTableData<NifId, (Ident, GlobalId)>,
    /// the index of global
    pub global_index: u32,
}

impl Environment {
    /// Create a new empty environment
    pub fn new() -> Self {
        Self {
            symbols: SymbolsTableData::new(),
            global: IdVec::new(),
            global_index: 0,
        }
    }

    /// Add ident and return the associated NifId
    pub fn add(&mut self, ident: Ident) -> NifId {
        let global_id = self.global.next_id();
        let nif = self.symbols.add(ident.clone(), (ident, global_id)).unwrap();
        self.global.push(ValueFun::Native(nif));
        self.global_index += 1;
        nif
    }

    /// Add global
    pub fn add_global(&mut self, fun: FunId) -> GlobalId {
        let global_id = self.global.push(ValueFun::Fun(fun));
        self.global_index += 1;
        global_id
    }

    /// Finalize the environment - TODO
    pub fn finalize(self) -> () {}
}
