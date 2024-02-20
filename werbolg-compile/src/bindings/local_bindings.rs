use super::bindings_stack::BindingsStack;
use super::types::BindingType;
use crate::defs::LocalStackSize;
use crate::instructions::{LocalBindIndex, ParamBindIndex};
use alloc::{vec, vec::Vec};
use werbolg_core::Ident;

pub struct LocalBindings {
    bindings: BindingsStack<BindingType>,
    local: Vec<u16>,
    max_local: u16,
}

impl LocalBindings {
    pub fn new() -> Self {
        Self {
            bindings: BindingsStack::new(),
            local: vec![0],
            max_local: 0,
        }
    }

    pub fn add_param(&mut self, ident: Ident, n: u8) {
        self.bindings
            .add(ident, BindingType::Param(ParamBindIndex(n)))
    }

    pub fn add_local(&mut self, ident: Ident) -> LocalBindIndex {
        match self.local.last_mut() {
            None => panic!("internal error: cannot add local without an empty binding stack"),
            Some(x) => {
                let local = *x;
                *x += 1;

                let local = LocalBindIndex(local);
                self.bindings.add(ident, BindingType::Local(local));
                local
            }
        }
    }

    pub fn scope_enter(&mut self) {
        let top = self.local.last().unwrap();
        self.local.push(*top);
        self.bindings.scope_enter();
    }

    pub fn scope_leave(&mut self) {
        let _x = self.bindings.scope_pop();
        let local = self.local.pop().unwrap();
        self.max_local = core::cmp::max(self.max_local, local);
    }

    pub fn scope_terminate(mut self) -> LocalStackSize {
        self.scope_leave();
        assert_eq!(self.local.len(), 1, "internal compilation error");
        LocalStackSize(self.max_local as u16)
    }

    pub fn get(&self, ident: &Ident) -> Option<&BindingType> {
        self.bindings.get(ident)
    }
}
