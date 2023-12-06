use super::basic::Ident;
use alloc::vec::Vec;
use core::marker::PhantomData;
use hashbrown::HashMap;

pub struct SymbolsTable {
    tbl: HashMap<Ident, SymbolId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u32);

impl SymbolsTable {
    pub fn new() -> Self {
        Self {
            tbl: Default::default(),
        }
    }

    pub fn insert(&mut self, ident: Ident, id: SymbolId) {
        self.tbl.insert(ident, id);
    }

    pub fn get(&self, ident: &Ident) -> Option<SymbolId> {
        self.tbl.get(ident).map(|i| *i)
    }
}

pub struct SymbolIdAllocator(u32);

pub struct SymbolsTableBuilder {
    table: SymbolsTable,
    allocator: SymbolIdAllocator,
}

impl SymbolIdAllocator {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn allocate(&mut self) -> SymbolId {
        let v = self.0;
        self.0 += 1;
        SymbolId(v)
    }
}

impl SymbolsTableBuilder {
    pub fn new() -> Self {
        Self {
            table: SymbolsTable::new(),
            allocator: SymbolIdAllocator::new(),
        }
    }

    pub fn allocate(&mut self, ident: Ident) -> Option<SymbolId> {
        if self.table.get(&ident).is_some() {
            return None;
        }
        let id = self.allocator.allocate();
        self.table.insert(ident, id);
        Some(id)
    }

    pub fn allocate_anon(&mut self) -> SymbolId {
        self.allocator.allocate()
    }

    pub fn finalize(self) -> SymbolsTable {
        self.table
    }
}

pub struct SymbolsTableData<T, ID: SymbolIdRemapper> {
    pub symtbl: SymbolsTable,
    pub syms: Vec<T>,
    pub phantom: PhantomData<ID>,
}

pub trait SymbolIdRemapper: Copy {
    fn uncat(self) -> SymbolId;
    fn cat(id: SymbolId) -> Self;
}

impl<T, ID: SymbolIdRemapper> SymbolsTableData<T, ID> {
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

pub struct SymbolsTableDataBuilder<T, ID: SymbolIdRemapper> {
    builder: SymbolsTableBuilder,
    vec: Vec<T>,
    phantom: PhantomData<ID>,
}

impl<T, ID: SymbolIdRemapper> SymbolsTableDataBuilder<T, ID> {
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

    pub fn finalize(self) -> SymbolsTableData<T, ID> {
        SymbolsTableData {
            syms: self.vec,
            symtbl: self.builder.finalize(),
            phantom: self.phantom,
        }
    }
}
