use super::bindings::Bindings;
use alloc::{vec, vec::Vec};
use werbolg_core::Ident;

pub struct BindingsStack<T> {
    stack: Vec<Bindings<T>>,
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
                bindings.add_replace(name.clone(), value);
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
