#![no_std]
extern crate alloc;
extern crate proc_macro;

mod tests;
mod value;

use alloc::vec;
use value::Value;
use werbolg_compile::{compile as comp, CallArity, CompilationError, Environment};
use werbolg_core::Literal;
use werbolg_core::{AbsPath, Ident, Namespace, Span};
use werbolg_exec::{
    ExecutionEnviron, ExecutionError, ExecutionMachine, ExecutionParams, NIFCall, WAllocator,
};

pub struct DummyAlloc;
impl WAllocator for DummyAlloc {
    type Value = Value;
}

fn nif_bool_eq(_: &DummyAlloc, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].bool()?;
    let n2 = args[1].bool()?;
    let ret = n1 == n2;
    Ok(Value::Bool(ret))
}

fn nif_expect_bool_eq(_: &DummyAlloc, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].bool()?;
    let n2 = args[1].bool()?;
    assert_eq!(n1, n2);
    let ret = n1 == n2;
    Ok(Value::Bool(ret))
}

fn nif_int_eq(_: &DummyAlloc, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;
    let ret = n1 == n2;
    Ok(Value::Bool(ret))
}

fn nif_expect_int_eq(_: &DummyAlloc, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;
    assert_eq!(n1, n2);
    let ret = n1 == n2;
    Ok(Value::Bool(ret))
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum MyLiteral {
    Bool(bool),
    Int(u64),
}

fn literal_to_value(lit: &MyLiteral) -> Value {
    match lit {
        MyLiteral::Bool(b) => Value::Bool(*b),
        MyLiteral::Int(n) => Value::Integral(*n),
    }
}

fn literal_mapper(span: Span, lit: Literal) -> Result<MyLiteral, CompilationError> {
    match lit {
        Literal::Bool(b) => {
            let b = b.as_ref() == "true";
            Ok(MyLiteral::Bool(b))
        }
        Literal::Number(s) => {
            let Ok(v) = u64::from_str_radix(s.as_ref(), 10) else {
                todo!()
            };
            Ok(MyLiteral::Int(v))
        }
        Literal::String(_) => Err(CompilationError::LiteralNotSupported(span, lit)),
        Literal::Decimal(_) => Err(CompilationError::LiteralNotSupported(span, lit)),
        Literal::Bytes(_) => Err(CompilationError::LiteralNotSupported(span, lit)),
    }
}

pub fn execute(mod1: werbolg_core::Module) -> Result<Value, ExecutionError> {
    macro_rules! add_pure_nif {
        ($env:ident, $i:literal, $arity:literal, $e:expr) => {
            let nif = NIFCall::Pure($e).info($i, CallArity::try_from($arity as usize).unwrap());
            let path = AbsPath::new(&Namespace::root(), &Ident::from($i));
            $env.add_nif(&path, nif);
        };
    }
    let module_ns = Namespace::root().append(Ident::from("main"));
    let modules = vec![(module_ns.clone(), mod1)];
    let mut environ = Environment::new();
    add_pure_nif!(environ, "expect_bool", 2, nif_expect_bool_eq);
    add_pure_nif!(environ, "bool_eq", 2, nif_bool_eq);
    add_pure_nif!(environ, "expect_int", 2, nif_expect_int_eq);
    add_pure_nif!(environ, "int_eq", 2, nif_int_eq);
    let compilation_params = werbolg_compile::CompilationParams {
        literal_mapper,
        sequence_constructor: None,
    };
    let exec_module =
        comp(&compilation_params, modules, &mut environ).expect("no compilation error");
    let ee = ExecutionEnviron::from_compile_environment(environ.finalize());
    let entry_point = exec_module
        .funs_tbl
        .get(&AbsPath::new(&module_ns, &Ident::from("main")))
        .expect("existing function as entry point");
    let execution_params = ExecutionParams { literal_to_value };
    let mut em = ExecutionMachine::new(&exec_module, &ee, execution_params, DummyAlloc, ());
    werbolg_exec::exec(&mut em, entry_point, &[])
}
