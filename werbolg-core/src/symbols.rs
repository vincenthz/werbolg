use super::basic::Ident;
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
