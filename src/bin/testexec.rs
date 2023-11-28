use hashbrown::HashMap;
use werbolg::{
    exec, ir::Ident, ir::Number, parse, ExecutionError, ExecutionMachine, FileUnit, Value,
};

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

fn nif_hashtable(_em: &ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let mut h = HashMap::<u32, u64>::new();
    h.insert(10, 20);
    h.insert(20, 40);
    Ok(Value::make_opaque(h))
}

fn nif_hashtable_get(_em: &ExecutionMachine, args: &[Value]) -> Result<Value, ExecutionError> {
    let h: &HashMap<u32, u64> = args[0].opaque()?;
    let index_bignum = args[1].number()?;
    let index: u32 = index_bignum
        .try_into()
        .map_err(|()| ExecutionError::UserPanic {
            message: String::from("cannot convert number to u64"),
        })?;

    match h.get(&index) {
        None => Ok(Value::Unit),
        Some(value) => Ok(Value::Number(Number::from_u64(*value))),
    }
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
                Some("rusty") => werbolg::lang::Lang::Rusty,
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
        let test_snippet = include_str!("../../test.lispy");
        let fileunit = FileUnit::from_string("test.lispy".to_string(), test_snippet.to_string());
        (fileunit, werbolg::lang::Lang::Lispy)
    };

    let module = parse(lang, &fileunit).expect("no parse error");

    let mut em = ExecutionMachine::new();
    em.add_native_fun("+", nif_plus);
    em.add_native_fun("-", nif_sub);
    em.add_native_fun("*", nif_mul);
    em.add_native_fun("==", nif_eq);
    em.add_native_fun("table_new", nif_hashtable);
    em.add_native_fun("table_get", nif_hashtable_get);

    let val = exec(&mut em, &module, Ident::from("main"), vec![]).expect("no execution error");

    println!("{:?}", val);
    Ok(())
}
