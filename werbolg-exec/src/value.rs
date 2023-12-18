//! Execution machine value - define the Value type

use super::{ExecutionError, ExecutionMachine};
/*
use alloc::rc::Rc;
use core::any::Any;
use core::cell::RefCell;
*/

/// Native Implemented Function
pub struct NIF<'m, L, T, V> {
    pub name: &'static str,
    pub call: NIFCall<'m, L, T, V>,
}

/// 2 Variants of Native calls
///
/// * "Pure" function that don't have access to the execution machine
/// * "Mut" function that have access to the execution machine and have more power / responsability.
pub enum NIFCall<'m, L, T, V> {
    Pure(fn(&[V]) -> Result<V, ExecutionError>),
    Mut(fn(&mut ExecutionMachine<'m, L, T, V>, &[V]) -> Result<V, ExecutionError>),
}

/*
#[derive(Clone)]
pub struct Opaque(Rc<dyn Any>);

impl Opaque {
    pub fn new<T: Any + Send + Sync>(t: T) -> Self {
        Self(Rc::new(t))
    }

    pub fn downcast_ref<T: Any + Send + Sync>(&self) -> Result<&T, ExecutionError> {
        self.0
            .downcast_ref()
            .ok_or(ExecutionError::OpaqueTypeTypeInvalid {
                got_type_id: self.0.type_id(),
            })
    }
}

#[derive(Clone)]
pub struct OpaqueMut(Rc<RefCell<dyn Any>>);

impl OpaqueMut {
    pub fn new<T: Any + Send + Sync>(t: T) -> Self {
        Self(Rc::new(RefCell::new(t)))
    }

    pub fn on_mut<F, T>(&self, f: F) -> Result<(), ExecutionError>
    where
        T: Any + Send + Sync,
        F: FnOnce(&mut T) -> Result<(), ExecutionError>,
    {
        let b = self.0.as_ref();

        let mut cell = b.borrow_mut();
        let r = cell
            .downcast_mut()
            .ok_or(ExecutionError::OpaqueTypeTypeInvalid {
                got_type_id: self.0.type_id(),
            })?;

        f(r)
    }
}

impl core::fmt::Debug for Opaque {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ty = self.0.type_id();
        f.debug_tuple("Opaque").field(&ty).finish()
    }
}

impl core::fmt::Debug for OpaqueMut {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ty = self.0.type_id();
        f.debug_tuple("OpaqueMut").field(&ty).finish()
    }
}
*/
