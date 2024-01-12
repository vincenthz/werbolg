use std::error::Error;

use super::environ;
use super::value::Value;
use super::{Frontend, TalesParams};
use hashbrown::HashSet;
use werbolg_compile::{code_dump, compile, Environment, InstructionAddress};
use werbolg_core::{id::IdF, AbsPath, Ident, Module, Namespace};
use werbolg_exec::{ExecutionEnviron, ExecutionMachine, ExecutionParams, WAllocator, NIF};
use werbolg_lang_common::{FileUnit, LinesMap, Report, ReportKind};

pub fn run_frontend(params: &TalesParams, args: &[String]) -> Result<Module, Box<dyn Error>> {
    if args.is_empty() {
        crate::help();
        return Err(format!("no file specified").into());
    }

    let path = std::path::PathBuf::from(&args[0]);
    let file_unit = get_file(&path)?;
    let file_map = LinesMap::new(&file_unit.content);

    let parsing_res = match params.frontend {
        Frontend::Rusty => werbolg_lang_rusty::module(&file_unit),
        Frontend::Lispy => werbolg_lang_lispy::module(&file_unit),
    };
    let module = match parsing_res {
        Err(e) => {
            let report = Report::new(ReportKind::Error, format!("Parse Error: {:?}", e))
                .lines_before(1)
                .lines_after(1)
                .highlight(e.location, format!("parse error here"));

            let mut s = String::new();
            report.write(&file_unit, &file_map, &mut s)?;
            println!("{}", s);
            return Err(format!("parse error").into());
            //return Err(format!("parse error \"{}\" : {:?}", path.to_string_lossy(), e).into());
        }
        Ok(module) => module,
    };

    if params.dump_ir {
        std::println!("{:#?}", module);
    }
    Ok(module)
}

pub fn run_compile<'m, 'e, A>(
    params: &TalesParams,
    env: &mut Environment<NIF<'m, 'e, A, environ::MyLiteral, (), Value>, Value>,
    module: Module,
) -> Result<werbolg_compile::CompilationUnit<environ::MyLiteral>, Box<dyn Error>> {
    let module_ns = Namespace::root().append(Ident::from("main"));
    let modules = vec![(module_ns.clone(), module)];

    let compilation_params = werbolg_compile::CompilationParams {
        literal_mapper: environ::literal_mapper,
    };

    let exec_module = compile(&compilation_params, modules, env)
        .map_err(|e| format!("compilation error {:?}", e))?;

    if params.dump_instr {
        let mut out = String::new();
        code_dump(&mut out, &exec_module.code, &exec_module.funs).expect("writing to string work");
        println!("{}", out);
    }

    Ok(exec_module)
}

pub struct DummyAlloc;

impl WAllocator for DummyAlloc {
    type Value = Value;
}

pub fn run_exec<'m, 'e>(
    params: &TalesParams,
    ee: &'e ExecutionEnviron<'m, 'e, DummyAlloc, environ::MyLiteral, (), Value>,
    exec_module: &'m werbolg_compile::CompilationUnit<environ::MyLiteral>,
) -> Result<(), Box<dyn Error>> {
    let module_ns = Namespace::root().append(Ident::from("main"));

    let entry_point = exec_module
        .funs_tbl
        .get(&AbsPath::new(&module_ns, &Ident::from("main")))
        .expect("existing function as entry point");

    let execution_params = ExecutionParams {
        literal_to_value: environ::literal_to_value,
    };

    let mut em = ExecutionMachine::new(&exec_module, &ee, execution_params, DummyAlloc, ());

    let mut stepper = HashSet::<InstructionAddress>::new();
    for a in params.step_address.iter() {
        stepper.insert(InstructionAddress::from_collection_len(*a as usize));
    }

    let ret = if !stepper.is_empty() | params.exec_step_trace {
        werbolg_exec::initialize(&mut em, entry_point, &[]).unwrap();
        loop {
            if params.exec_step_trace || stepper.contains(&em.ip) {
                let mut out = String::new();
                em.debug_state(&mut out).unwrap();
                println!("{}", out);
            }
            match werbolg_exec::step(&mut em) {
                Err(e) => break Err(e),
                Ok(None) => {}
                Ok(Some(v)) => break Ok(v),
            }
        }
    } else {
        werbolg_exec::exec(&mut em, entry_point, &[])
    };

    match ret {
        Err(e) => {
            let mut out = String::new();
            em.debug_state(&mut out).unwrap();

            println!("error: {:?} at {}", e, em.ip);
            println!("{}", out);
            return Err(format!("execution error").into());
        }
        Ok(val) => {
            println!("{:?}", val);
            Ok(())
        }
    }
}

fn get_file(path: &std::path::Path) -> std::io::Result<FileUnit> {
    let path = std::path::PathBuf::from(&path);
    let content = std::fs::read_to_string(&path).expect("file read");
    let fileunit = FileUnit::from_string(path.to_string_lossy().to_string(), content);
    Ok(fileunit)
}
