mod lang;

use hashbrown::HashMap;
use werbolg_compile::{code_dump, compile, symbols::IdVec, Environment};
use werbolg_core::{Ident, NifId, Number};
use werbolg_exec2::{ExecutionEnviron, ExecutionError, ExecutionMachine, NIFCall, Value, NIF};
use werbolg_lang_common::FileUnit;

fn nif_plus(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number::new(n1.as_ref() + n2.as_ref());

    Ok(Value::Number(ret))
}

fn nif_sub(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number::new(n1.as_ref() - n2.as_ref());

    Ok(Value::Number(ret))
}

fn nif_mul(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].number()?;
    let n2 = args[1].number()?;

    let ret = Number::new(n1.as_ref() * n2.as_ref());

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

    pub struct Env<'m, T> {
        environment: Environment,
        nifs: IdVec<NifId, NIF<'m, T>>,
        nifs_binds: werbolg_exec::Bindings<NifId>,
    }

    impl<'m, T> Env<'m, T> {
        pub fn new() -> Self {
            Self {
                environment: Environment::new(),
                nifs: IdVec::new(),
                nifs_binds: werbolg_exec::Bindings::new(),
            }
        }
        pub fn add_native_call(&mut self, ident: &'static str, f: NIFCall<'m, T>) {
            let id = self.environment.add(werbolg_core::Ident::from(ident));
            let id2 = self.nifs.push(NIF {
                name: ident,
                call: f,
            });
            self.nifs_binds.add(werbolg_core::Ident::from(ident), id);
            assert_eq!(id, id2)
        }

        #[allow(unused)]
        pub fn add_native_mut_fun(
            &mut self,
            ident: &'static str,
            f: fn(&mut ExecutionMachine<'m, T>, &[Value]) -> Result<Value, ExecutionError>,
        ) {
            self.add_native_call(ident, NIFCall::Mut(f))
        }

        pub fn add_native_pure_fun(
            &mut self,
            ident: &'static str,
            f: fn(&[Value]) -> Result<Value, ExecutionError>,
        ) {
            self.add_native_call(ident, NIFCall::Pure(f))
        }

        pub fn finalize(self) -> ExecutionEnviron<'m, T> {
            let globals = self.environment.global.remap(|f| Value::Fun(f));

            werbolg_exec2::ExecutionEnviron {
                nifs: self.nifs,
                globals: globals,
            }
        }
    }

    let mut env = Env::new();
    env.add_native_pure_fun("+", nif_plus);
    env.add_native_pure_fun("-", nif_sub);
    env.add_native_pure_fun("*", nif_mul);
    env.add_native_pure_fun("==", nif_eq);
    env.add_native_pure_fun("table_new", nif_hashtable);
    env.add_native_pure_fun("table_get", nif_hashtable_get);
    //environment.add(ident);

    let exec_module = compile(module, &mut env.environment).expect("no compilation error");

    //exec_module.print();
    code_dump(&exec_module.code, &exec_module.funs);

    let ee = env.finalize();
    //let mut em = ExecutionMachine::new(&exec_module, ee, ());

    let entry_point = exec_module
        .funs_tbl
        .get(&Ident::from("main"))
        .expect("existing function as entry point");

    let mut em = werbolg_exec2::ExecutionMachine::new(&exec_module, ee, ());

    //let val = werbolg_exec::exec(&mut em, Ident::from("main"), &[]).expect("no execution error");

    //println!("{:?}", val);

    match werbolg_exec2::exec(&mut em, entry_point, &[]) {
        Err(e) => {
            println!("error: {:?} at {}", e, em.ip);
            return Err(());
        }
        Ok(val) => {
            println!("{:?}", val);
        }
    }

    Ok(())
}
