mod lang;
mod value;

use hashbrown::HashMap;
use value::{Value, HASHMAP_KIND};
use werbolg_compile::{code_dump, compile, CompilationError, Environment};
use werbolg_core::{idvec::IdVec, Ident, Literal, NifId};
use werbolg_exec::{
    ExecutionEnviron, ExecutionError, ExecutionMachine, ExecutionParams, NIFCall, Valuable, NIF,
};
use werbolg_lang_common::FileUnit;

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

fn nif_eq(args: &[Value]) -> Result<Value, ExecutionError> {
    let n1 = args[0].int()?;
    let n2 = args[1].int()?;

    let ret = n1 == n2;

    Ok(Value::Bool(ret))
}

fn nif_hashtable(_args: &[Value]) -> Result<Value, ExecutionError> {
    let mut h = HashMap::<u32, u64>::new();
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
    Int(u64),
}

fn literal_to_value(lit: &MyLiteral) -> Value {
    match lit {
        MyLiteral::Bool(b) => Value::Bool(*b),
        MyLiteral::Int(n) => Value::Integral(*n),
    }
}

// only support bool and number from the werbolg core literal
fn literal_mapper(lit: Literal) -> Result<MyLiteral, CompilationError> {
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
        Literal::String(_) => Err(CompilationError::LiteralNotSupported(lit)),
        Literal::Decimal(_) => Err(CompilationError::LiteralNotSupported(lit)),
        Literal::Bytes(_) => Err(CompilationError::LiteralNotSupported(lit)),
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

    pub struct Env<'m, L, T, V> {
        environment: Environment,
        nifs: IdVec<NifId, NIF<'m, L, T, V>>,
        //nifs_binds: werbolg_interpret::Bindings<NifId>,
    }

    impl<'m, L, T, V: Valuable> Env<'m, L, T, V> {
        pub fn new() -> Self {
            Self {
                environment: Environment::new(),
                nifs: IdVec::new(),
                //nifs_binds: werbolg_interpret::Bindings::new(),
            }
        }
        pub fn add_native_call(&mut self, ident: &'static str, f: NIFCall<'m, L, T, V>) {
            let id = self.environment.add(werbolg_core::Ident::from(ident));
            let id2 = self.nifs.push(NIF {
                name: ident,
                call: f,
            });
            //self.nifs_binds.add(werbolg_core::Ident::from(ident), id);
            assert_eq!(id, id2)
        }

        #[allow(unused)]
        pub fn add_native_mut_fun(
            &mut self,
            ident: &'static str,
            f: fn(&mut ExecutionMachine<'m, L, T, V>) -> Result<V, ExecutionError>,
        ) {
            self.add_native_call(ident, NIFCall::Raw(f))
        }

        pub fn add_native_pure_fun(
            &mut self,
            ident: &'static str,
            f: fn(&[V]) -> Result<V, ExecutionError>,
        ) {
            self.add_native_call(ident, NIFCall::Pure(f))
        }

        pub fn finalize(self) -> ExecutionEnviron<'m, L, T, V> {
            let globals = self.environment.global.remap(|f| V::make_fun(f));

            werbolg_exec::ExecutionEnviron {
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

    let compilation_params = werbolg_compile::CompilationParams { literal_mapper };
    let exec_module =
        compile(&compilation_params, module, &mut env.environment).expect("no compilation error");

    let mut out = String::new();
    code_dump(&mut out, &exec_module.code, &exec_module.funs).expect("writing to string work");
    println!("{}", out);

    let ee = env.finalize();

    let entry_point = exec_module
        .funs_tbl
        .get(&Ident::from("main"))
        .expect("existing function as entry point");

    let execution_params = ExecutionParams { literal_to_value };
    let mut em = ExecutionMachine::new(&exec_module, ee, execution_params, ());

    match werbolg_exec::exec(&mut em, entry_point, &[]) {
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
