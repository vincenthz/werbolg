use super::basic::Ident;
use super::id::{Id, IdRemapper};
use alloc::vec::Vec;
use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;

pub struct SymbolsTable<ID: IdRemapper> {
    pub(crate) tbl: HashMap<Ident, Id>,
    phantom: PhantomData<ID>,
}

impl<ID: IdRemapper> SymbolsTable<ID> {
    pub fn new() -> Self {
        Self {
            tbl: Default::default(),
            phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, ident: Ident, id: ID) {
        self.tbl.insert(ident, id.uncat());
    }

    pub fn get(&self, ident: &Ident) -> Option<ID> {
        self.tbl.get(ident).map(|i| ID::cat(*i))
    }
}

pub struct IdVec<ID, T> {
    vec: Vec<T>,
    phantom: PhantomData<ID>,
}

impl<ID: IdRemapper, T> core::ops::Index<ID> for IdVec<ID, T> {
    type Output = T;

    fn index(&self, index: ID) -> &Self::Output {
        &self.vec[index.uncat().0 as usize]
    }
}

impl<ID: IdRemapper, T> IdVec<ID, T> {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, id: ID) -> Option<&T> {
        let idx = id.uncat().0 as usize;
        if self.vec.len() > idx {
            Some(&self.vec[id.uncat().0 as usize])
        } else {
            None
        }
    }

    pub fn next_id(&self) -> ID {
        ID::cat(Id(self.vec.len() as u32))
    }

    pub fn push(&mut self, v: T) -> ID {
        let id = Id(self.vec.len() as u32);
        self.vec.push(v);
        ID::cat(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ID, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(i, t)| (ID::cat(Id(i as u32)), t))
    }

    pub fn into_iter(self) -> impl Iterator<Item = (ID, T)> {
        self.vec
            .into_iter()
            .enumerate()
            .map(|(i, t)| (ID::cat(Id(i as u32)), t))
    }

    pub fn concat(&mut self, after: &mut IdVecAfter<ID, T>) {
        assert!(self.vec.len() == after.ofs as usize);
        self.vec.append(&mut after.id_vec.vec)
    }
}

pub struct IdVecAfter<ID, T> {
    id_vec: IdVec<ID, T>,
    ofs: u32,
}

impl<ID: IdRemapper, T> IdVecAfter<ID, T> {
    pub fn new(first_id: ID) -> Self {
        Self {
            id_vec: IdVec::new(),
            ofs: first_id.uncat().0,
        }
    }

    pub fn push(&mut self, v: T) -> ID {
        let id = self.id_vec.push(v).uncat();
        let new_id = Id(id.0 + self.ofs as u32);
        ID::cat(new_id)
    }
}

pub struct SymbolsTableData<ID: IdRemapper, T> {
    pub table: SymbolsTable<ID>,
    pub vecdata: IdVec<ID, T>,
}

impl<ID: IdRemapper, T> SymbolsTableData<ID, T> {
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

/*
impl<T, ID: IdRemapper> SymbolsTableData<ID, T> {
    pub fn new() -> Self {
        Self {
            symtbl: SymbolsTable::new(),
            syms: Vec::new(),
            phantom: PhantomData,
        }
    }
    pub fn resolve_id(&self, ident: &Ident) -> Option<ID> {
        self.symtbl.get(ident).map(ID::cat)
    }

    pub fn get_symbol(&self, ident: &Ident) -> Option<&T> {
        if let Some(id) = self.resolve_id(ident) {
            self.get_symbol_by_id(id)
        } else {
            None
        }
    }

    pub fn get_symbol_by_id(&self, id: ID) -> Option<&T> {
        if id.uncat().0 >= self.syms.len() as u32 {
            return None;
        }
        Some(&self.syms[id.uncat().0 as usize])
    }

    pub fn remap<A, F, U, E>(self, f: F) -> Result<SymbolsTableData<ID, U>, E>
    where
        F: Fn(T) -> Result<U, E>,
    {
        let Self {
            syms,
            symtbl,
            phantom,
        } = self;
        let syms = syms
            .into_iter()
            .map(|s| f(s))
            .collect::<Result<Vec<_>, E>>()?;
        Ok(SymbolsTableData {
            syms,
            symtbl,
            phantom,
        })
    }
}

pub struct SymbolsTableDataBuilder<ID: IdRemapper, T> {
    builder: SymbolsTableBuilder,
    vec: Vec<T>,
    phantom: PhantomData<ID>,
}

impl<T, ID: IdRemapper> SymbolsTableDataBuilder<ID, T> {
    pub fn new() -> Self {
        Self {
            builder: SymbolsTableBuilder::new(),
            vec: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn add(&mut self, ident: Option<Ident>, t: T) -> Result<ID, ()> {
        let id = if let Some(ident) = ident {
            self.builder.allocate(ident).ok_or_else(|| ())?
        } else {
            self.builder.allocate_anon()
        };
        self.vec.push(t);
        Ok(ID::cat(id))
    }

    pub fn finalize(self) -> SymbolsTableData<ID, T> {
        SymbolsTableData {
            syms: self.vec,
            symtbl: self.builder.finalize(),
            phantom: self.phantom,
        }
    }
}

*/

pub struct UniqueTableBuilder<ID: IdRemapper, T: Eq + Hash> {
    pub symtbl: HashMap<T, ID>,
    pub syms: IdVec<ID, T>,
    pub phantom: PhantomData<ID>,
}

impl<ID: IdRemapper, T: Clone + Eq + Hash> UniqueTableBuilder<ID, T> {
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
