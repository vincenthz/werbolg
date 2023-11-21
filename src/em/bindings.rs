use crate::ast::Ident;
use alloc::{vec, vec::Vec};
use hashbrown::HashMap;

pub struct BindingsStack<T> {
    stack: Vec<HashMap<BindingName, T>>,
}

type BindingName = Ident;

impl<T> BindingsStack<T> {
    pub fn new() -> Self {
        Self {
            stack: vec![HashMap::new()],
        }
    }

    pub fn scope_enter(&mut self) {
        self.stack.push(HashMap::new())
    }

    pub fn scope_leave(&mut self) {
        self.stack.pop();
    }

    pub fn add(&mut self, name: BindingName, value: T) {
        match self.stack.last_mut() {
            None => panic!("cannot add to empty"),
            Some(hashm) => {
                hashm.insert(name, value);
            }
        }
    }

    pub fn get(&self, name: &BindingName) -> Option<&T> {
        for h in self.stack.iter().rev() {
            if let Some(v) = h.get(name) {
                return Some(v);
            }
        }
        None
    }
}
