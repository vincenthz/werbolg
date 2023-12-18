use hashbrown::HashMap;
use werbolg_core::{ConstrId, ValueFun};
use werbolg_exec::{ExecutionError, Valuable, ValueKind};

#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    Integral(u64),
    HashMap(HashMap<u32, u64>),
    Fun(ValueFun),
}

impl Value {
    fn desc(&self) -> ValueKind {
        match self {
            Value::Unit => b"    unit",
            Value::Bool(_) => b"    bool",
            Value::HashMap(_) => b" hashmap",
            Value::Integral(_) => b"     int",
            Value::Fun(_) => b"     fun",
        }
    }
}

impl Valuable for Value {
    fn descriptor(&self) -> werbolg_exec::ValueKind {
        self.desc()
    }

    fn conditional(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn fun(&self) -> Option<ValueFun> {
        match self {
            Self::Fun(valuefun) => Some(*valuefun),
            _ => None,
        }
    }

    fn structure(&self) -> Option<(ConstrId, &[Self])> {
        None
    }

    fn index(&self, _index: usize) -> Option<&Self> {
        None
    }

    fn make_fun(fun: ValueFun) -> Self {
        Value::Fun(fun)
    }

    fn make_dummy() -> Self {
        Value::Unit
    }
}

impl Value {
    pub fn int(&self) -> Result<u64, ExecutionError> {
        match self {
            Value::Integral(o) => Ok(*o),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: Value::Integral(0).descriptor(),
                value_got: self.descriptor(),
            }),
        }
    }
}
