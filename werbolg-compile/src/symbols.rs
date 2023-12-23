use alloc::vec::Vec;
use core::hash::Hash;
use core::marker::PhantomData;
use hashbrown::HashMap;
use werbolg_core::id::IdF;
pub use werbolg_core::idvec::{IdVec, IdVecAfter};
use werbolg_core::{Ident, Namespace, Path};

/// A simple lookup table from Ident to ID
///
/// this is a flat table (only use 1 Ident for lookup/insertion),
/// for hierarchical table use `SymbolsTable`
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
    pub(crate) current: SymbolsTableFlat<ID>,
    pub(crate) ns: HashMap<Ident, SymbolsTable<ID>>,
}

#[derive(Clone, Debug)]
pub enum NamespaceError {
    /// Duplicate namespace found
    Duplicate(Namespace, Ident),
    /// Missing namespace
    Missing(Namespace, Ident),
}

impl<ID: IdF> SymbolsTable<ID> {
    pub fn new() -> Self {
        Self {
            current: SymbolsTableFlat::new(),
            ns: HashMap::new(),
        }
    }

    fn create_namespace_here(&mut self, ident: Ident) -> Result<(), ()> {
        let already_exist = self.ns.insert(ident, SymbolsTable::new());
        if already_exist.is_some() {
            Err(())
        } else {
            Ok(())
        }
    }

    fn flat_table(&self, namespace: &Namespace) -> Option<&Self> {
        let mut current = self;
        for n in namespace.iter() {
            if let Some(child) = current.ns.get(n) {
                current = child;
            } else {
                return None;
            }
        }
        Some(current)
    }

    fn flat_table_mut(&mut self, namespace: &Namespace) -> Option<&mut SymbolsTableFlat<ID>> {
        if namespace.is_root() {
            return Some(&mut self.current);
        } else {
            let (id, child_ns) = namespace.clone().drop_first();
            if let Some(x) = self.ns.get_mut(&id) {
                x.flat_table_mut(&child_ns)
            } else {
                panic!("flat-table-mut oops")
            }
        }
        /*
        } else {
            let mut i = namespace.iter_with_last().collect::<Vec<_>>()();
            let (is_last, ns) = i.next();
            if is_last {
                self.ns.get_mut(ns).map(|x| &mut x.current)
            } else {
            }
            self.ns[]
            for (is_last, n) in namespace.iter_with_last() {
                if let Some(child) = current.ns.get(n) {
                    current = child;
                } else {
                    return None;
                }
            }
        }
        */
    }

    pub fn create_namespace(&mut self, namespace: Namespace) -> Result<(), NamespaceError> {
        if namespace.is_root() {
            return Ok(());
        }
        let mut current = self;
        for (is_last, n) in namespace.iter_with_last() {
            if is_last {
                current
                    .create_namespace_here(n.clone())
                    .map_err(|()| NamespaceError::Duplicate(namespace.clone(), n.clone()))?
            } else {
                if let Some(child) = current.ns.get_mut(n) {
                    current = child;
                } else {
                    return Err(NamespaceError::Missing(namespace.clone(), n.clone()));
                }
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, namespace: &Namespace, path: &Path, id: ID) {
        let path = namespace.path_with_path(path);
        let (namespace, ident) = path.split();
        if let Some(table) = self.flat_table_mut(&namespace) {
            table.insert(ident, id)
        } else {
            panic!("unknown namespace {:?}", namespace);
        }
    }

    pub fn get_in(&self, namespace: &Namespace, path: &Path) -> Option<ID> {
        let path = namespace.path_with_path(path);
        let mut table = self;
        for (is_final, fragment) in path.components() {
            if is_final {
                return table.current.get(fragment);
            } else {
                if let Some(child_table) = self.ns.get(fragment) {
                    table = child_table;
                } else {
                    panic!("unknown namespace {:?} from path {:?}", fragment, path)
                }
            }
        }
        return None;
    }

    pub fn get(&self, _resolver: &NamespaceResolver, path: &Path) -> Option<ID> {
        let (namespace, ident) = path.split();
        if namespace.is_root() {
            self.current.get(&ident)
        } else {
            let t = self.flat_table(&namespace);
            t.and_then(|x| x.current.get(&ident))
        }
    }

    fn dump_path(&self, current: Namespace, vec: &mut Vec<(Path, ID)>) {
        for (ident, id) in self.current.iter() {
            let path = current.path_with_ident(ident);
            vec.push((path, id))
        }
        for (ns_name, st) in self.ns.iter() {
            let child_namespace = current.clone().append(ns_name.clone());
            st.dump_path(child_namespace, vec)
        }
    }

    pub fn to_vec(&self, current: Namespace) -> Vec<(Path, ID)> {
        let mut v = Vec::new();
        self.dump_path(current, &mut v);
        v
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

    pub fn create_namespace(&mut self, namespace: Namespace) -> Result<(), NamespaceError> {
        self.table.create_namespace(namespace)
    }

    pub fn add(&mut self, namespace: &Namespace, path: &Path, v: T) -> Option<ID> {
        if self.table.get_in(namespace, &path).is_some() {
            return None;
        }
        let id = self.vecdata.push(v);
        self.table.insert(namespace, path, id);
        Some(id)
    }

    pub fn add_anon(&mut self, v: T) -> ID {
        self.vecdata.push(v)
    }

    pub fn get(&self, resolver: &NamespaceResolver, path: &Path) -> Option<(ID, &T)> {
        self.table
            .get(resolver, path)
            .map(|constr_id| (constr_id, &self.vecdata[constr_id]))
    }

    pub fn to_vec(&self, current: Namespace) -> Vec<(Path, ID)> {
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

/// Namespace Resolver
pub struct NamespaceResolver;

impl NamespaceResolver {
    /// Create a empty namespace resolver
    pub fn none() -> Self {
        Self
    }
}
