use super::basic::Ident;
use super::id::{Id, IdAllocator, IdRemapper};
use alloc::vec::Vec;
use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;

pub struct SymbolsTable {
    tbl: HashMap<Ident, Id>,
}

impl SymbolsTable {
    pub fn new() -> Self {
        Self {
            tbl: Default::default(),
        }
    }

    pub fn insert(&mut self, ident: Ident, id: Id) {
        self.tbl.insert(ident, id);
    }

    pub fn get(&self, ident: &Ident) -> Option<Id> {
        self.tbl.get(ident).map(|i| *i)
    }
}

pub struct SymbolsTableBuilder {
    table: SymbolsTable,
    allocator: IdAllocator,
}

impl SymbolsTableBuilder {
    pub fn new() -> Self {
        Self {
            table: SymbolsTable::new(),
            allocator: IdAllocator::new(),
        }
    }

    pub fn allocate(&mut self, ident: Ident) -> Option<Id> {
        if self.table.get(&ident).is_some() {
            return None;
        }
        let id = self.allocator.allocate();
        self.table.insert(ident, id);
        Some(id)
    }

    pub fn allocate_anon(&mut self) -> Id {
        self.allocator.allocate()
    }

    pub fn finalize(self) -> SymbolsTable {
        self.table
    }
}

pub struct SymbolsTableData<ID: IdRemapper, T> {
    pub symtbl: SymbolsTable,
    pub syms: Vec<T>,
    pub phantom: PhantomData<ID>,
}

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

pub struct UniqueTable<ID: IdRemapper, T> {
    pub syms: Vec<T>,
    pub phantom: PhantomData<ID>,
}

impl<ID: IdRemapper, T> core::ops::Index<ID> for UniqueTable<ID, T> {
    type Output = T;

    fn index(&self, index: ID) -> &Self::Output {
        &self.syms[index.uncat().0 as usize]
    }
}

pub struct UniqueTableBuilder<ID: IdRemapper, T: Eq + Hash> {
    pub symtbl: HashMap<T, Id>,
    pub syms: Vec<T>,
    pub allocator: IdAllocator,
    pub phantom: PhantomData<ID>,
}

impl<ID: IdRemapper, T: Clone + Eq + Hash> UniqueTableBuilder<ID, T> {
    pub fn new() -> Self {
        Self {
            symtbl: HashMap::new(),
            syms: Vec::new(),
            allocator: IdAllocator::new(),
            phantom: PhantomData,
        }
    }

    pub fn add(&mut self, data: T) -> ID {
        if let Some(id) = self.symtbl.get(&data) {
            ID::cat(*id)
        } else {
            let id = self.allocator.allocate();
            self.syms.push(data.clone());
            self.symtbl.insert(data, id);
            ID::cat(id)
        }
    }

    pub fn finalize(self) -> UniqueTable<ID, T> {
        UniqueTable {
            syms: self.syms,
            phantom: self.phantom,
        }
    }
}
