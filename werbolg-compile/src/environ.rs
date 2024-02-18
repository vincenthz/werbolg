use super::symbols::IdVec;
use crate::symbols::{NamespaceError, SymbolInsertError, SymbolsTable};
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
    pub(crate) symbols: SymbolsTable<EnvironmentId>,
    /// All the global values defined
    pub(crate) globals: IdVec<GlobalId, G>,
    /// The symbols
    pub(crate) nifs: IdVec<NifId, N>,
}

#[derive(Debug, Clone, Copy)]
pub enum EnvironmentId {
    Global(GlobalId),
    Nif(NifId),
}

#[derive(Debug, Clone)]
pub enum EnvironmentError {
    DuplicateSymbol(AbsPath, EnvironmentId),
    SymbolInsertError(SymbolInsertError),
}

impl<N, G> Environment<N, G> {
    /// Create a new empty environment
    pub fn new() -> Self {
        Self {
            symbols: SymbolsTable::new(),
            nifs: IdVec::new(),
            globals: IdVec::new(),
        }
    }

    /// Create a namespace in the environment
    pub fn create_namespace(&mut self, namespace: &Namespace) -> Result<(), NamespaceError> {
        self.symbols.create_namespace(namespace.clone())?;
        Ok(())
    }

    /// Add NIF to the environment
    pub fn add_nif(&mut self, path: &AbsPath, t: N) -> Result<NifId, EnvironmentError> {
        let nif_id = self.nifs.next_id();
        if let Some(id) = self.symbols.get(&path) {
            return Err(EnvironmentError::DuplicateSymbol(path.clone(), id));
        }

        self.symbols
            .insert(&path, EnvironmentId::Nif(nif_id))
            .map_err(|e| EnvironmentError::SymbolInsertError(e))?;

        let id = self.nifs.push(t);
        assert_eq!(nif_id, id);

        Ok(nif_id)
    }

    /// Add global to the environment
    pub fn add_global(&mut self, path: &AbsPath, p: G) -> Result<GlobalId, EnvironmentError> {
        let global_id = self.globals.next_id();
        if let Some(id) = self.symbols.get(&path) {
            return Err(EnvironmentError::DuplicateSymbol(path.clone(), id));
        }

        self.symbols
            .insert(&path, EnvironmentId::Global(global_id))
            .map_err(|e| EnvironmentError::SymbolInsertError(e))?;

        let id = self.globals.push(p);
        assert_eq!(global_id, id);

        Ok(global_id)
    }

    /// Finalize the environment and keep only the execution relevant information
    #[must_use]
    pub fn finalize(self) -> (IdVec<GlobalId, G>, IdVec<NifId, N>) {
        (self.globals, self.nifs)
    }
}
