use super::value::{self, Value, HASHMAP_KIND};
use hashbrown::HashMap;
use werbolg_compile::{CompilationError, Environment};
use werbolg_core::{AbsPath, Ident, Literal, Namespace};
use werbolg_exec::{ExecutionError, NIFCall, Valuable, NIF};

fn nif_plus(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;

    let ret = Value::Integral(n1 + n2);

    Ok(ret)
}

fn nif_sub(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;

    let ret = Value::Integral(n1 - n2);

    Ok(ret)
}

fn nif_mul(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;

    let ret = Value::Integral(n1 * n2);

    Ok(ret)
}

fn nif_neg(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;

    let ret = !n1;

    Ok(Value::Integral(ret))
}

fn nif_eq(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;

    let ret = n1 == n2;

    Ok(Value::Bool(ret))
}

fn nif_le(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;

    let ret = n1 <= n2;

    Ok(Value::Bool(ret))
}

fn nif_hashtable(_args: &[Value]) -> Result<Value, ExecutionError> {
    let mut h = HashMap::new();
    h.insert(10, 20);
    h.insert(20, 40);
    Ok(Value::HashMap(h))
}

fn nif_hashtable_get(args: &[Value]) -> Result<Value, ExecutionError> {
    let Value::HashMap(h) = &args[0] else {
        return Err(ExecutionError::ValueKindUnexpected {
            value_expected: HASHMAP_KIND,
            value_got: args[0].descriptor(),
        });
    };
    let index_bignum = args[1].int()?;
    let index: u32 = index_bignum
        .try_into()
        .map_err(|_| ExecutionError::UserPanic {
            message: String::from("cannot convert Integral to u32"),
        })?;

    match h.get(&index) {
        None => Ok(Value::Unit),
        Some(value) => Ok(Value::Integral(*value)),
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum MyLiteral {
    Bool(bool),
    Int(value::ValueInt),
}

pub fn literal_to_value(lit: &MyLiteral) -> Value {
    match lit {
        MyLiteral::Bool(b) => Value::Bool(*b),
        MyLiteral::Int(n) => Value::Integral(*n),
    }
}

// only support bool and number from the werbolg core literal
pub fn literal_mapper(lit: Literal) -> Result<MyLiteral, CompilationError> {
    match lit {
        Literal::Bool(b) => {
            let b = b.as_ref() == "true";
            Ok(MyLiteral::Bool(b))
        }
        Literal::Number(s) => {
            let Ok(v) = value::ValueInt::from_str_radix(s.as_ref(), 10) else {
                todo!()
            };
            Ok(MyLiteral::Int(v))
        }
        Literal::String(_) => Err(CompilationError::LiteralNotSupported(lit)),
        Literal::Decimal(_) => Err(CompilationError::LiteralNotSupported(lit)),
        Literal::Bytes(_) => Err(CompilationError::LiteralNotSupported(lit)),
    }
}

pub fn create_env<'m, 'e>(
) -> Environment<NIF<'m, 'e, crate::DummyAlloc, MyLiteral, (), Value>, Value> {
    macro_rules! add_pure_nif {
        ($env:ident, $i:literal, $e:expr) => {
            let nif = NIF {
                name: $i,
                call: NIFCall::Pure($e),
            };
            let path = AbsPath::new(&Namespace::root(), &Ident::from($i));
            $env.add_nif(&path, nif);
        };
    }

    let mut env = Environment::new();
    add_pure_nif!(env, "+", nif_plus);
    add_pure_nif!(env, "-", nif_sub);
    add_pure_nif!(env, "*", nif_mul);
    add_pure_nif!(env, "==", nif_eq);
    add_pure_nif!(env, "<=", nif_le);
    add_pure_nif!(env, "neg", nif_neg);
    add_pure_nif!(env, "table_new", nif_hashtable);
    add_pure_nif!(env, "table_get", nif_hashtable_get);

    env
}
