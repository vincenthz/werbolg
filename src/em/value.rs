//! Execution machine value - define the Value type

use super::{ExecutionError, ExecutionMachine};
use crate::ast::{self, Ident, Literal, Statement};
use strum::EnumDiscriminants;

/// Execution Machine Value
#[derive(Clone, Debug, EnumDiscriminants)]
#[strum_discriminants(name(ValueKind))]
pub enum Value {
    Unit,
    // Simple values
    Number(ast::Number),
    String(String),
    Decimal(ast::Decimal),
    Bytes(Box<[u8]>),
    // Composite
    List(Vec<Value>),
    // Functions
    NativeFun(fn(&mut ExecutionMachine, &[Value]) -> Result<Value, ExecutionError>),
    Fun(Vec<Ident>, Vec<Statement>),
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
    pub fn number(&self) -> Result<&ast::Number, ExecutionError> {
        match self {
            Value::Number(v) => Ok(v),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: ValueKind::Number,
                value_got: self.into(),
            }),
        }
    }

    pub fn decimal(&self) -> Result<&ast::Decimal, ExecutionError> {
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
