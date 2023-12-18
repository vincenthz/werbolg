use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;
use werbolg_core::id::IdF;
pub use werbolg_core::idvec::{IdVec, IdVecAfter};
use werbolg_core::Ident;

pub struct SymbolsTable<ID: IdF> {
    pub(crate) tbl: HashMap<Ident, ID>,
    phantom: PhantomData<ID>,
}

impl<ID: IdF> SymbolsTable<ID> {
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

    pub fn add(&mut self, ident: Ident, v: T) -> Option<ID> {
        if self.table.get(&ident).is_some() {
            return None;
        }
        let id = self.vecdata.push(v);
        self.table.insert(ident, id);
        Some(id)
    }

    pub fn add_anon(&mut self, v: T) -> ID {
        self.vecdata.push(v)
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
