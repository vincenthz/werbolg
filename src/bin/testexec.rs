use werbolg::{ast::Number, exec, parse, ExecutionError, ExecutionMachine, FileUnit, Value};

fn nif_plus(_em: &ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number(&n1.0 + &n2.0);

    Ok(Value::Number(ret))
}

fn nif_sub(_em: &ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number(&n1.0 - &n2.0);

    Ok(Value::Number(ret))
}

fn nif_mul(_em: &ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number(&n1.0 * &n2.0);

    Ok(Value::Number(ret))
}

fn nif_eq(_em: &ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = n1.0 == n2.0;

    Ok(Value::Bool(ret))
}

fn main() -> Result<(), ()> {
    #[cfg(std)]
    let (fileunit, lang) = {
        let args = std::env::args().into_iter().collect::<Vec<_>>();

        if args.len() < 2 {
            println!("usage: {} <FILE>", args[0]);
            return Err(());
        }

        let path = PathBuf::from(&args[1]);

        let default = werbolg::lang::Lang::Rusty;
        let lang = match path.extension() {
            None => default,
            Some(os_str) => match os_str.to_str() {
                None => default,
                Some("rs") => werbolg::lang::Lang::Rusty,
                Some("scheme") => werbolg::lang::Lang::Lispy,
                Some(s) => {
                    println!("error: unknown extension {}", s);
                    return Err(());
                }
            },
        };

        let fileunit = FileUnit::from_file(path).expect("file read");
        (fileunit, lang)
    };
    #[cfg(not(std))]
    let (fileunit, lang) = {
        let test_snippet = include_str!("../../test.scheme");
        let fileunit = FileUnit::from_string("test.scheme".to_string(), test_snippet.to_string());
        (fileunit, werbolg::lang::Lang::Lispy)
    };

    let module = parse(lang, &fileunit).expect("no parse error");

    let mut em = ExecutionMachine::new();
    em.add_native_fun("+", nif_plus);
    em.add_native_fun("-", nif_sub);
    em.add_native_fun("*", nif_mul);
    em.add_native_fun("==", nif_eq);

    let val = exec(&mut em, module).expect("no execution error");

    println!("{:?}", val);
    Ok(())
}
