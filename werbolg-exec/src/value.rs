//! Execution machine value - define the Value type

use super::{ExecutionError, ExecutionMachine};
use alloc::rc::Rc;
use core::any::Any;
use core::cell::RefCell;

/*
/// Execution Machine Value
#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    // Simple values
    Small(u64),
    Binary(Box<[u8]>),
    Opaque(Opaque),
    OpaqueMut(OpaqueMut),
    // Composite
    List(Box<[Value]>),
    Struct(ConstrId, Box<[Value]>),
    Enum(u32, Box<[Value]>),
    // Functions
    Fun(ValueFun),
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    Unit,
    Small,
    Binary,
    Opaque,
    OpaqueMut,
    List,
    Struct,
    Enum,
    Fun,
}

impl From<NifId> for Value {
    fn from(value: NifId) -> Self {
        Value::Fun(ValueFun::Native(value))
    }
}

impl From<FunId> for Value {
    fn from(value: FunId) -> Self {
        Value::Fun(ValueFun::Fun(value))
    }
}

impl<'a> From<&'a Value> for ValueKind {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Unit => ValueKind::Unit,
            Value::Small(_) => ValueKind::Small,
            Value::Binary(_) => ValueKind::Binary,
            Value::Opaque(_) => ValueKind::Opaque,
            Value::OpaqueMut(_) => ValueKind::OpaqueMut,
            Value::List(_) => ValueKind::List,
            Value::Struct(_, _) => ValueKind::Struct,
            Value::Enum(_, _) => ValueKind::Enum,
            Value::Fun(_) => ValueKind::Fun,
        }
    }
}
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

/*
impl<'a> From<&'a Literal> for Value {
    fn from(literal: &'a Literal) -> Value {
        match literal {
            Literal::Bool(b) =>
            Literal::String(s) => Value::String(s.clone().into_boxed_str()),
            Literal::Number(n) => Value::Number(n.clone()),
            Literal::Decimal(d) => Value::Decimal(d.clone()),
            Literal::Bytes(b) => Value::Bytes(b.clone()),
        }
    }
}
*/

impl Value {
    pub fn make_opaque<T: Any + Send + Sync>(t: T) -> Self {
        Value::Opaque(Opaque::new(t))
    }

    pub fn make_opaque_mut<T: Any + Send + Sync>(t: T) -> Self {
        Value::OpaqueMut(OpaqueMut::new(t))
    }

    pub fn small<T: Any + Send + Sync>(&self) -> Result<u64, ExecutionError> {
        match self {
            Value::Small(o) => Ok(*o),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Small,
                value_got: self.into(),
            }),
        }
    }

    pub fn binary<T: Any + Send + Sync>(&self) -> Result<Box<[u8]>, ExecutionError> {
        match self {
            Value::Binary(o) => Ok(*o),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Small,
                value_got: self.into(),
            }),
        }
    }

    pub fn opaque<T: Any + Send + Sync>(&self) -> Result<&T, ExecutionError> {
        match self {
            Value::Opaque(o) => o.downcast_ref(),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Opaque,
                value_got: self.into(),
            }),
        }
    }

    pub fn fun(&self) -> Result<ValueFun, ExecutionError> {
        match self {
            Value::Fun(valuefun) => Ok(*valuefun),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Fun,
                value_got: self.into(),
            }),
        }
    }

    pub fn unit(&self) -> Result<(), ExecutionError> {
        match self {
            Value::Unit => Ok(()),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Unit,
                value_got: self.into(),
            }),
        }
    }

    pub fn list(&self) -> Result<&[Value], ExecutionError> {
        match self {
            Value::List(v) => Ok(v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::List,
                value_got: self.into(),
            }),
        }
    }
}
*/
