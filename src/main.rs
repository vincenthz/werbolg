mod lang;

use hashbrown::HashMap;
use werbolg_core::{compile, Ident, Number};
use werbolg_exec::{ExecutionError, ExecutionMachine, Value};
use werbolg_lang_common::FileUnit;

fn nif_plus(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number::new(n1.0.as_ref() + n2.0.as_ref());

    Ok(Value::Number(ret))
}

fn nif_sub(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number::new(n1.0.as_ref() - n2.0.as_ref());

    Ok(Value::Number(ret))
}

fn nif_mul(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number::new(n1.0.as_ref() * n2.0.as_ref());

    Ok(Value::Number(ret))
}

fn nif_eq(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = n1.0 == n2.0;

    Ok(Value::Bool(ret))
}

fn nif_hashtable(_args: &[Value]) -> Result<Value, ExecutionError> {
    let mut h = HashMap::<u32, u64>::new();
    h.insert(10, 20);
    h.insert(20, 40);
    Ok(Value::make_opaque(h))
}

fn nif_hashtable_get(args: &[Value]) -> Result<Value, ExecutionError> {
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

fn get_content(args: &[String]) -> Result<(FileUnit, lang::Lang), ()> {
    let path = std::path::PathBuf::from(&args[1]);

    let default = lang::Lang::Rusty;
    let lang = match path.extension() {
        None => default,
        Some(os_str) => match os_str.to_str() {
            None => default,
            Some("rusty") => lang::Lang::Rusty,
            Some("lispy") => lang::Lang::Lispy,
            Some(s) => {
                println!("error: unknown extension {}", s);
                return Err(());
            }
        },
    };

    let content = std::fs::read_to_string(&path).expect("file read");
    let fileunit = FileUnit::from_string(path.to_string_lossy().to_string(), content);
    Ok((fileunit, lang))
}

fn get_content_nostd() -> (FileUnit, lang::Lang) {
    let test_snippet = include_str!("../test.lispy");
    let fileunit = FileUnit::from_string("test.lispy".to_string(), test_snippet.to_string());
    (fileunit, lang::Lang::Lispy)
}

fn main() -> Result<(), ()> {
    // in no_std environment, we can't read from file, so add a way to load a file from an example
    let use_file = true;

    let (fileunit, lang) = {
        if use_file {
            let args = std::env::args().into_iter().collect::<Vec<_>>();

            if args.len() < 2 {
                println!("usage: {} <FILE>", args[0]);
                return Err(());
            }
            get_content(&args)?
        } else {
            get_content_nostd()
        }
    };

    let module = lang::parse(lang, &fileunit).expect("no parse error");
    let exec_module = compile(module).expect("no compilation error");

    let mut em = ExecutionMachine::new(&exec_module, ());
    em.add_native_pure_fun("+", nif_plus);
    em.add_native_pure_fun("-", nif_sub);
    em.add_native_pure_fun("*", nif_mul);
    em.add_native_pure_fun("==", nif_eq);
    em.add_native_pure_fun("table_new", nif_hashtable);
    em.add_native_pure_fun("table_get", nif_hashtable_get);

    let val = werbolg_exec::exec(&mut em, Ident::from("main"), vec![]).expect("no execution error");

    println!("{:?}", val);
    Ok(())
}
