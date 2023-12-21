use super::Ident;
use alloc::{vec, vec::Vec};
use hashbrown::HashMap;

type BindingName = Ident;

#[derive(Clone)]
pub struct Bindings<T>(HashMap<BindingName, T>);

#[derive(Clone)]
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

    #[allow(unused)]
    pub fn remove(&mut self, name: BindingName) {
        self.0.remove(&name);
    }

    pub fn get(&self, name: &BindingName) -> Option<&T> {
        self.0.get(name)
    }
}

impl<T> BindingsStack<T> {
    pub fn new() -> Self {
        Self {
            stack: vec![Bindings::new()],
        }
    }

    pub fn scope_enter(&mut self) {
        self.stack.push(Bindings::new())
    }

    pub fn scope_pop(&mut self) -> Bindings<T> {
        self.stack.pop().unwrap()
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
        for bindings in self.stack.iter().rev() {
            if let Some(ident) = bindings.get(name) {
                return Some(ident);
            }
        }
        return None;
    }
}
