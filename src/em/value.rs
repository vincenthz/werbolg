//! Execution machine value - define the Value type

use super::{ExecutionError, ExecutionMachine, Location};
use crate::ir::{self, Literal, Variable};
use alloc::{boxed::Box, string::String, vec::Vec};

/// Execution Machine Value
#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    // Simple values
    Bool(bool),
    Number(ir::Number),
    String(String),
    Decimal(ir::Decimal),
    Bytes(Box<[u8]>),
    Opaque(Opaque),
    // Composite
    List(Vec<Value>),
    // Functions
    NativeFun(&'static str, NIF),
    Fun(Location, Vec<Variable>, ir::Expr),
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    Unit,
    Bool,
    Number,
    String,
    Decimal,
    Bytes,
    Opaque,
    List,
    NativeFun,
    Fun,
}

impl<'a> From<&'a Value> for ValueKind {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Unit => ValueKind::Unit,
            Value::Bool(_) => ValueKind::Bool,
            Value::Number(_) => ValueKind::Number,
            Value::String(_) => ValueKind::String,
            Value::Decimal(_) => ValueKind::Decimal,
            Value::Bytes(_) => ValueKind::Bytes,
            Value::Opaque(_) => ValueKind::Opaque,
            Value::List(_) => ValueKind::List,
            Value::NativeFun(_, _) => ValueKind::NativeFun,
            Value::Fun(_, _, _) => ValueKind::Fun,
        }
    }
}

/// Native Implemented Function
pub type NIF = fn(&ExecutionMachine, &[Value]) -> Result<Value, ExecutionError>;

#[derive(Clone)]
pub struct Opaque(u64);

impl core::fmt::Debug for Opaque {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Opaque").finish()
    }
}

impl<'a> From<&'a Literal> for Value {
    fn from(literal: &'a Literal) -> Value {
        match literal {
            Literal::String(s) => Value::String(s.clone()),
            Literal::Number(n) => Value::Number(n.clone()),
            Literal::Decimal(d) => Value::Decimal(d.clone()),
            Literal::Bytes(b) => Value::Bytes(b.clone()),
        }
    }
}

impl Value {
    pub fn unit(&self) -> Result<(), ExecutionError> {
        match self {
            Value::Unit => Ok(()),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Unit,
                value_got: self.into(),
            }),
        }
    }

    pub fn bool(&self) -> Result<bool, ExecutionError> {
        match self {
            Value::Bool(v) => Ok(*v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Bool,
                value_got: self.into(),
            }),
        }
    }

    pub fn number(&self) -> Result<&ir::Number, ExecutionError> {
        match self {
            Value::Number(v) => Ok(v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Number,
                value_got: self.into(),
            }),
        }
    }

    pub fn decimal(&self) -> Result<&ir::Decimal, ExecutionError> {
        match self {
            Value::Decimal(v) => Ok(v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Decimal,
                value_got: self.into(),
            }),
        }
    }

    pub fn string(&self) -> Result<&String, ExecutionError> {
        match self {
            Value::String(v) => Ok(v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::String,
                value_got: self.into(),
            }),
        }
    }

    pub fn bytes(&self) -> Result<&Box<[u8]>, ExecutionError> {
        match self {
            Value::Bytes(v) => Ok(v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Bytes,
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
