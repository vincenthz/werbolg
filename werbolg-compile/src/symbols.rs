use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;
use werbolg_core::id::IdF;
pub use werbolg_core::idvec::{IdVec, IdVecAfter};
use werbolg_core::{Ident, Namespace};

pub struct SymbolsTableFlat<ID: IdF> {
    pub(crate) tbl: HashMap<Ident, ID>,
    phantom: PhantomData<ID>,
}

impl<ID: IdF> SymbolsTableFlat<ID> {
    pub fn new() -> Self {
        Self {
            tbl: Default::default(),
            phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, ident: Ident, id: ID) {
        self.tbl.insert(ident, id);
    }

    pub fn get(&self, ident: &Ident) -> Option<ID> {
        self.tbl.get(ident).map(|i| *i)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ident, ID)> {
        self.tbl.iter().map(|(ident, id)| (ident, *id))
    }
}

pub struct SymbolsTable<ID: IdF> {
    pub(crate) root: SymbolsTableFlat<ID>,
}

impl<ID: IdF> SymbolsTable<ID> {
    pub fn new() -> Self {
        Self {
            root: SymbolsTableFlat::new(),
        }
    }

    pub fn insert(&mut self, namespace: &Namespace, ident: Ident, id: ID) {
        self.root.tbl.insert(ident, id);
    }

    pub fn get_in(&self, namespace: &Namespace, ident: &Ident) -> Option<ID> {
        self.root.tbl.get(ident).map(|i| *i)
    }

    pub fn get(&self, resolver: &NamespaceResolver, ident: &Ident) -> Option<ID> {
        self.root.tbl.get(ident).map(|i| *i)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ident, ID)> {
        self.root.tbl.iter().map(|(ident, id)| (ident, *id))
    }
}

/// Symbol Table Data maps Ident to ID and store the ID to T
pub struct SymbolsTableData<ID: IdF, T> {
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

    pub fn add(&mut self, namespace: &Namespace, ident: Ident, v: T) -> Option<ID> {
        if self.table.get_in(namespace, &ident).is_some() {
            return None;
        }
        let id = self.vecdata.push(v);
        self.table.insert(namespace, ident, id);
        Some(id)
    }

    pub fn add_anon(&mut self, v: T) -> ID {
        self.vecdata.push(v)
    }

    pub fn get(&self, resolver: &NamespaceResolver, ident: &Ident) -> Option<(ID, &T)> {
        self.table
            .get(resolver, ident)
            .map(|constr_id| (constr_id, &self.vecdata[constr_id]))
    }

    pub fn iter(&self) -> impl Iterator<Item = (ID, &Ident, &T)> {
        self.table
            .iter()
            .map(|(ident, id)| (id, ident, &self.vecdata[id]))
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

/// Namespace Resolver
pub struct NamespaceResolver;

impl NamespaceResolver {
    /// Create a empty namespace resolver
    pub fn none() -> Self {
        Self
    }
}
