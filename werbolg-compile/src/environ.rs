use super::symbols::IdVec;
use crate::symbols::SymbolsTableData;
use werbolg_core::{GlobalId, Ident, Namespace, NifId};

/// Environment of the compilation
///
/// Define the NIF and global symbols
///
/// The type parameters are only relevant to the execution,
/// and are completly unused for compilation
///
/// * N is the type for NIF
/// * G is the type for global
///
pub struct Environment<N, G> {
    /// All the global values defined
    pub(crate) globals: SymbolsTableData<GlobalId, G>,
    /// The symbols
    pub(crate) symbols: SymbolsTableData<NifId, N>,
}

impl<N, G> Environment<N, G> {
    /// Create a new empty environment
    pub fn new() -> Self {
        Self {
            symbols: SymbolsTableData::new(),
            globals: SymbolsTableData::new(),
        }
    }

    /// Add NIF to the environment
    pub fn add_nif(&mut self, namespace: &Namespace, ident: Ident, t: N) -> NifId {
        let nif_id = self.symbols.add(namespace, ident, t).expect("unique NIF");
        nif_id
    }

    /// Add global to the environment
    pub fn add_global(&mut self, namespace: &Namespace, ident: Ident, p: G) -> GlobalId {
        let global_id = self
            .globals
            .add(namespace, ident, p)
            .expect("unique Global");
        global_id
    }

    /// Finalize the environment and keep only the execution relevant information
    #[must_use]
    pub fn finalize(self) -> (IdVec<GlobalId, G>, IdVec<NifId, N>) {
        (self.globals.vecdata, self.symbols.vecdata)
    }
}
