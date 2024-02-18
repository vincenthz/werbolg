use crate::hier::{Hier, HierError};
use alloc::vec::Vec;
use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;
use werbolg_core::id::IdF;
pub use werbolg_core::idvec::{IdVec, IdVecAfter};
use werbolg_core::{AbsPath, Ident, Namespace};

/// A simple lookup table from Ident to ID
///
/// this is a flat table (only use 1 Ident for lookup/insertion),
/// for hierarchical table use `SymbolsTable`
pub struct SymbolsTableFlat<ID> {
    pub(crate) tbl: HashMap<Ident, ID>,
    phantom: PhantomData<ID>,
}

impl<ID: Copy> Default for SymbolsTableFlat<ID> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ID: Copy> SymbolsTableFlat<ID> {
    pub fn new() -> Self {
        Self {
            tbl: Default::default(),
            phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, ident: Ident, id: ID) -> Result<(), SymbolInsertFlatError> {
        if self.tbl.get(&ident).is_some() {
            Err(SymbolInsertFlatError { ident })
        } else {
            self.tbl.insert(ident, id);
            Ok(())
        }
    }

    pub fn get(&self, ident: &Ident) -> Option<ID> {
        self.tbl.get(ident).map(|i| *i)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ident, ID)> {
        self.tbl.iter().map(|(ident, id)| (ident, *id))
    }
}

pub struct SymbolsTable<ID>(pub(crate) Hier<SymbolsTableFlat<ID>>);

#[derive(Clone, Debug)]
pub enum NamespaceError {
    /// Duplicate namespace found
    Duplicate(Namespace),
    /// Duplicate namespace found
    DuplicateLeaf(Namespace),
    /// Missing namespace
    Missing(Namespace, Ident),
}

pub struct SymbolInsertFlatError {
    ident: Ident,
}

#[derive(Clone, Debug)]
pub enum SymbolInsertError {
    AlreadyExist(Namespace, Ident),
    NamespaceNotPresent(Namespace),
}

impl<ID: Copy> SymbolsTable<ID> {
    pub fn new() -> Self {
        Self(Hier::default())
    }

    /*
    fn create_namespace_here(&mut self, ident: Ident) -> Result<(), ()> {
        self.0.add_ns(ident, SymbolsTableFlat::new())
    }

    fn flat_table(&self, namespace: &Namespace) -> Result<&SymbolsTableFlat<ID>, ()> {
        self.0.get(namespace)
    }
    */

    fn on_flat_table_mut<F, E>(&mut self, namespace: &Namespace, f: F) -> Result<(), HierError<E>>
    where
        F: FnMut(&mut SymbolsTableFlat<ID>) -> Result<(), E>,
    {
        self.0.on_mut(namespace, f)
    }

    pub fn create_namespace(&mut self, namespace: Namespace) -> Result<(), NamespaceError> {
        self.0
            .add_ns_hier(namespace.clone())
            .map_err(|()| NamespaceError::DuplicateLeaf(namespace))
    }

    pub fn insert(&mut self, path: &AbsPath, id: ID) -> Result<(), SymbolInsertError> {
        let (namespace, ident) = path.split();
        match self.on_flat_table_mut(&namespace, |table| table.insert(ident.clone(), id)) {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Some(err) = e.err {
                    Err(SymbolInsertError::AlreadyExist(namespace, err.ident))
                } else {
                    Err(SymbolInsertError::NamespaceNotPresent(namespace))
                }
            }
        }
    }

    pub fn get(&self, path: &AbsPath) -> Option<ID> {
        let (namespace, ident) = path.split();
        if let Ok(tbl) = self.0.get(&namespace) {
            tbl.get(&ident)
        } else {
            None
        }
    }

    fn dump_path(&self, current: Namespace, vec: &mut Vec<(AbsPath, ID)>) {
        let mut ts = Vec::new();
        self.0.dump(current, &mut ts);
        for (n, t) in ts.iter() {
            for (ident, id) in t.iter() {
                let path = AbsPath::new(n, ident);
                vec.push((path, id))
            }
        }
    }

    pub fn to_vec(&self, current: Namespace) -> Vec<(AbsPath, ID)> {
        let mut v = Vec::new();
        self.dump_path(current, &mut v);
        v
    }
}

/// Symbol Table Data maps Ident to ID and store the ID to T
pub struct SymbolsTableData<ID, T> {
    pub table: SymbolsTable<ID>,
    pub vecdata: IdVec<ID, T>,
}

impl<ID: IdF, T> SymbolsTableData<ID, T> {
    pub fn new() -> Self {
        Self {
            table: SymbolsTable::new(),
            vecdata: IdVec::new(),
        }
    }

    pub fn create_namespace(&mut self, namespace: Namespace) -> Result<(), NamespaceError> {
        self.table.create_namespace(namespace)
    }

    pub fn add(&mut self, path: &AbsPath, v: T) -> Option<ID> {
        if self.table.get(&path).is_some() {
            return None;
        }
        let id = self.vecdata.push(v);
        self.table.insert(path, id).unwrap();
        Some(id)
    }

    pub fn add_anon(&mut self, v: T) -> ID {
        self.vecdata.push(v)
    }

    pub fn get(&self, path: &AbsPath) -> Option<(ID, &T)> {
        self.table
            .get(path)
            .map(|constr_id| (constr_id, &self.vecdata[constr_id]))
    }

    pub fn get_by_id(&self, id: ID) -> Option<&T> {
        Some(&self.vecdata[id])
    }

    pub fn to_vec(&self, current: Namespace) -> Vec<(AbsPath, ID)> {
        self.table.to_vec(current)
    }
}

pub struct UniqueTableBuilder<ID: IdF, T: Eq + Hash> {
    pub symtbl: HashMap<T, ID>,
    pub syms: IdVec<ID, T>,
    pub phantom: PhantomData<ID>,
}

impl<ID: IdF, T: Clone + Eq + Hash> UniqueTableBuilder<ID, T> {
    pub fn new() -> Self {
        Self {
            symtbl: HashMap::new(),
            syms: IdVec::new(),
            phantom: PhantomData,
        }
    }

    pub fn add(&mut self, data: T) -> ID {
        if let Some(id) = self.symtbl.get(&data) {
            *id
        } else {
            let id = self.syms.push(data.clone());
            self.symtbl.insert(data, id);
            id
        }
    }

    pub fn finalize(self) -> IdVec<ID, T> {
        self.syms
    }
}
