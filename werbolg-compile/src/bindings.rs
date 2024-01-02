use super::symbols::NamespaceResolver;
use alloc::{vec, vec::Vec};
use hashbrown::HashMap;
use werbolg_core::{Ident, Namespace, Path};

#[derive(Clone)]
pub struct Bindings<T>(HashMap<Ident, T>);

pub struct GlobalBindings<T> {
    root: Bindings<T>,
    ns: HashMap<Namespace, Bindings<T>>,
}

pub struct BindingsStack<T> {
    stack: Vec<Bindings<T>>,
}

impl<T> Bindings<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add(&mut self, name: Ident, value: T) {
        self.0.insert(name, value);
    }

    #[allow(unused)]
    pub fn remove(&mut self, name: Ident) {
        self.0.remove(&name);
    }

    pub fn get(&self, name: &Ident) -> Option<&T> {
        self.0.get(name)
    }

    pub fn dump<W: core::fmt::Write>(&self, writer: &mut W) -> Result<(), core::fmt::Error> {
        for (ident, _) in self.0.iter() {
            writeln!(writer, "{:?}", ident)?
        }
        Ok(())
    }
}

impl<T> GlobalBindings<T> {
    pub fn new() -> Self {
        Self {
            root: Bindings::new(),
            ns: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: Path, value: T) {
        let (namespace, ident) = name.split();
        if namespace.is_root() {
            self.root.add(ident, value)
        } else {
            if let Some(ns_bindings) = self.ns.get_mut(&namespace) {
                ns_bindings.add(ident, value);
            } else {
                let mut b = Bindings::new();
                b.add(ident, value);
                self.ns.insert(namespace, b);
            }
        }
    }

    pub fn get_path(&self, name: &Path) -> Option<&T> {
        let (namespace, ident) = name.split();
        if namespace.is_root() {
            self.root.get(&ident)
        } else {
            if let Some(ns_bindings) = self.ns.get(&namespace) {
                ns_bindings.get(&ident)
            } else {
                None
            }
        }
    }

    pub fn get(&self, resolver: &NamespaceResolver, name: &Path) -> Option<&T> {
        let (namespace, ident) = name.split();
        if namespace.is_root() {
            self.root.get(&ident)
        } else {
            if let Some(ns_bindings) = self.ns.get(&namespace) {
                ns_bindings.get(&ident)
            } else {
                None
            }
        }
    }
}

impl<T> BindingsStack<T> {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn scope_enter(&mut self) {
        self.stack.push(Bindings::new())
    }

    pub fn scope_pop(&mut self) -> Bindings<T> {
        self.stack.pop().unwrap()
    }

    pub fn add(&mut self, name: Path, value: T) {
        if let Some(local_ident) = name.get_local() {
            match self.stack.last_mut() {
                None => {
                    panic!("add failed {:?}", name);
                    // fall through to the global
                }
                Some(bindings) => {
                    bindings.add(local_ident.clone(), value);
                    return;
                }
            }
        } else {
            panic!("BindingsStack : not added {:?} as not local", name)
        }
    }

    pub fn get(&self, name: &Path) -> Option<&T> {
        if let Some(local_ident) = name.get_local() {
            for bindings in self.stack.iter().rev() {
                if let Some(t) = bindings.get(local_ident) {
                    return Some(t);
                }
            }
        }
        None
    }

    #[allow(unused)]
    pub fn dump<W: core::fmt::Write>(&self, writer: &mut W) -> Result<(), core::fmt::Error> {
        writeln!(writer, "bindings-stack: {}", self.stack.len())?;
        for (i, bindings) in self.stack.iter().rev().enumerate() {
            writeln!(writer, "== Level {}", i)?;
            bindings.dump(writer)?
        }
        Ok(())
    }
}
