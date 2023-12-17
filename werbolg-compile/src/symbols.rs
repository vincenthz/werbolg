use alloc::vec::Vec;
use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;
use werbolg_core::id::IdF;
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

pub struct IdVec<ID, T> {
    vec: Vec<T>,
    phantom: PhantomData<ID>,
}

impl<ID: IdF, T> core::ops::Index<ID> for IdVec<ID, T> {
    type Output = T;

    fn index(&self, index: ID) -> &Self::Output {
        &self.vec[index.as_index()]
    }
}

impl<ID: IdF, T> core::ops::IndexMut<ID> for IdVec<ID, T> {
    fn index_mut(&mut self, index: ID) -> &mut T {
        &mut self.vec[index.as_index()]
    }
}

impl<ID: IdF, T> IdVec<ID, T> {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, id: ID) -> Option<&T> {
        let idx = id.as_index();
        if self.vec.len() > idx {
            Some(&self.vec[idx])
        } else {
            None
        }
    }

    pub fn next_id(&self) -> ID {
        ID::from_slice_len(&self.vec)
    }

    pub fn push(&mut self, v: T) -> ID {
        let id = ID::from_slice_len(&self.vec);
        self.vec.push(v);
        id
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.vec.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ID, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(i, t)| (ID::from_collection_len(i), t))
    }

    pub fn into_iter(self) -> impl Iterator<Item = (ID, T)> {
        self.vec
            .into_iter()
            .enumerate()
            .map(|(i, t)| (ID::from_collection_len(i), t))
    }

    pub fn concat(&mut self, after: &mut IdVecAfter<ID, T>) {
        assert!(self.vec.len() == after.ofs.as_index());
        self.vec.append(&mut after.id_vec.vec)
    }

    pub fn remap<F, U>(self, f: F) -> IdVec<ID, U>
    where
        F: Fn(T) -> U,
    {
        let mut new = IdVec::<ID, U>::new();
        for (id, t) in self.into_iter() {
            let new_id = new.push(f(t));
            assert_eq!(new_id, id);
        }
        new
    }
}

pub struct IdVecAfter<ID, T> {
    id_vec: IdVec<ID, T>,
    ofs: ID,
}

impl<ID: IdF, T> IdVecAfter<ID, T> {
    pub fn new(first_id: ID) -> Self {
        Self {
            id_vec: IdVec::new(),
            ofs: first_id,
        }
    }

    pub fn from_idvec(id_vec: IdVec<ID, T>, first_id: ID) -> Self {
        Self {
            id_vec,
            ofs: first_id,
        }
    }

    pub fn push(&mut self, v: T) -> ID {
        let id = self.id_vec.push(v);
        let new_id = ID::remap(id, self.ofs);
        new_id
    }

    pub fn remap<F>(&mut self, f: F)
    where
        F: Fn(&mut T) -> (),
    {
        for elem in self.id_vec.iter_mut() {
            f(elem)
        }
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
