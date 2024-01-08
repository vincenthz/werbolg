use werbolg_core::{ConstrId, ValueFun};
use werbolg_exec::{ExecutionError, Valuable, ValueKind};

pub type ValueInt = u64;

#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    Integral(ValueInt),
    Fun(ValueFun),
}

impl Value {
    fn desc(&self) -> ValueKind {
        match self {
            Value::Unit => UNIT_KIND,
            Value::Bool(_) => BOOL_KIND,
            Value::Integral(_) => INT_KIND,
            Value::Fun(_) => FUN_KIND,
        }
    }
}

pub const UNIT_KIND: ValueKind = "    unit";
pub const BOOL_KIND: ValueKind = "    bool";
pub const INT_KIND: ValueKind = "     int";
pub const FUN_KIND: ValueKind = "     fun";

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
    pub fn int(&self) -> Result<ValueInt, ExecutionError> {
        match self {
            Value::Integral(o) => Ok(*o),
            _ => Err(ExecutionError::ValueKindUnexpected {
                value_expected: INT_KIND,
                value_got: self.descriptor(),
            }),
        }
    }
}
