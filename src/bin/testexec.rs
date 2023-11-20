use std::path::PathBuf;
use werbolg::{ast::Ident, exec, parse, ExecutionError, ExecutionMachine, Value};

fn plus(_em: &mut ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = n1 + n2;

    Ok(Value::Number(ret))
}

fn sub(_em: &mut ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = n1 - n2;

    Ok(Value::Number(ret))
}

fn mul(_em: &mut ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = n1 * n2;

    Ok(Value::Number(ret))
}

fn eq(_em: &mut ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = n1 == n2;

    Ok(Value::Bool(ret))
}

fn main() {
    let args = std::env::args().into_iter().collect::<Vec<_>>();
    let path = PathBuf::from(&args[1]);
    let module = parse(werbolg::lang::Lang::Rusty, &path)
        .expect("file can be read")
        .expect("no parse error");

    let mut em = ExecutionMachine::new();
    em.add_binding(Ident::from("+"), Value::NativeFun(plus));
    em.add_binding(Ident::from("-"), Value::NativeFun(sub));
    em.add_binding(Ident::from("*"), Value::NativeFun(mul));
    em.add_binding(Ident::from("=="), Value::NativeFun(eq));

    let val = exec(&mut em, module).expect("no execution error");

    println!("{:?}", val)
}
