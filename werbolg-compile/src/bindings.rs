use crate::hier::Hier;
use alloc::{vec, vec::Vec};
use hashbrown::HashMap;
use werbolg_core::{AbsPath, Ident};

#[derive(Clone)]
pub struct Bindings<T>(HashMap<Ident, T>);

pub struct GlobalBindings<T>(pub(crate) Hier<Bindings<T>>);

pub struct BindingsStack<T> {
    stack: Vec<Bindings<T>>,
}

impl<T> Default for Bindings<T> {
    fn default() -> Self {
        Bindings::new()
    }
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

impl<T: Clone> GlobalBindings<T> {
    pub fn new() -> Self {
        Self(Hier::default())
    }

    pub fn add(&mut self, name: AbsPath, value: T) -> Result<(), ()> {
        let (namespace, ident) = name.split();

        if !self.0.namespace_exist(namespace.clone()) {
            self.0.add_ns_hier(namespace.clone()).unwrap()
        }

        self.0.on_mut(&namespace, |bindings| {
            bindings.add(ident.clone(), value.clone())
        })
    }

    #[allow(unused)]
    pub fn get(&self, name: &AbsPath) -> Option<&T> {
        let (namespace, ident) = name.split();
        let bindings = self.0.get(&namespace).unwrap();
        bindings.get(&ident)
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

    pub fn add(&mut self, name: Ident, value: T) {
        match self.stack.last_mut() {
            None => {
                panic!("add failed {:?}", name);
                // fall through to the global
            }
            Some(bindings) => {
                bindings.add(name.clone(), value);
            }
        }
    }

    pub fn get(&self, name: &Ident) -> Option<&T> {
        for bindings in self.stack.iter().rev() {
            if let Some(t) = bindings.get(name) {
                return Some(t);
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
