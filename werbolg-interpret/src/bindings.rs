use alloc::{vec, vec::Vec};
use hashbrown::HashMap;
use werbolg_core::Ident;

type BindingName = Ident;

pub struct Bindings<T>(HashMap<BindingName, T>);

pub struct BindingsStack<T> {
    stack: Vec<Bindings<T>>,
}

impl<T> Bindings<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add(&mut self, name: BindingName, value: T) {
        self.0.insert(name, value);
    }

    pub fn remove(&mut self, name: BindingName) {
        self.0.remove(&name);
    }

    pub fn get(&self, name: &BindingName) -> Option<&T> {
        self.0.get(name)
    }
}

impl<T> BindingsStack<T> {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn scope_enter(&mut self) {
        self.stack.push(Bindings::new())
    }

    pub fn scope_leave(&mut self) {
        self.stack.pop();
    }

    pub fn add(&mut self, name: BindingName, value: T) {
        match self.stack.last_mut() {
            None => panic!("cannot add to empty"),
            Some(bindings) => {
                bindings.add(name, value);
            }
        }
    }

    pub fn get(&self, name: &BindingName) -> Option<&T> {
        match self.stack.last() {
            None => None,
            Some(bindings) => bindings.get(name),
        }
    }
}
