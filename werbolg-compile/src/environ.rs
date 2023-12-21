use super::symbols::IdVec;
use crate::symbols::SymbolsTableData;
use werbolg_core::{GlobalId, Ident, NifId};

/// Environment of the compilation
pub struct Environment<N, G> {
    /// All the global values defined
    pub(crate) globals: IdVec<GlobalId, G>,
    /// The symbols
    pub(crate) symbols: SymbolsTableData<NifId, N>,
    /// the index of global
    pub(crate) global_index: u32,
}

impl<N, G> Environment<N, G> {
    /// Create a new empty environment
    pub fn new() -> Self {
        Self {
            symbols: SymbolsTableData::new(),
            globals: IdVec::new(),
            global_index: 0,
        }
    }

    /// Add ident and return the associated NifId
    pub fn add_nif(&mut self, ident: Ident, t: N) -> NifId {
        let nif = self.symbols.add(ident, t).unwrap();
        nif
    }

    /// Add global
    pub fn add_global(&mut self, p: G) -> GlobalId {
        let global_id = self.globals.push(p);
        self.global_index += 1;
        global_id
    }

    /// Finalize the environment and keep only the execution relevant information
    #[must_use]
    pub fn finalize(self) -> (IdVec<GlobalId, G>, IdVec<NifId, N>) {
        (self.globals, self.symbols.vecdata)
    }
}
