use super::symbols::IdVec;
use crate::symbols::{NamespaceError, SymbolsTableData};
use werbolg_core::{AbsPath, GlobalId, Namespace, NifId};

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

    /// Create a namespace in the environment
    pub fn create_namespace(&mut self, namespace: &Namespace) -> Result<(), NamespaceError> {
        self.symbols.create_namespace(namespace.clone())?;
        self.globals.create_namespace(namespace.clone())?;
        Ok(())
    }

    /// Add NIF to the environment
    pub fn add_nif(&mut self, path: &AbsPath, t: N) -> NifId {
        let nif_id = self.symbols.add(&path, t).expect("unique NIF");
        nif_id
    }

    /// Add global to the environment
    pub fn add_global(&mut self, path: &AbsPath, p: G) -> GlobalId {
        let global_id = self.globals.add(&path, p).expect("unique Global");
        global_id
    }

    /// Finalize the environment and keep only the execution relevant information
    #[must_use]
    pub fn finalize(self) -> (IdVec<GlobalId, G>, IdVec<NifId, N>) {
        (self.globals.vecdata, self.symbols.vecdata)
    }
}
