use std::error::Error;

use super::environ;
use super::value::Value;
use super::{Frontend, TalesParams};
use hashbrown::HashSet;
use werbolg_compile::{code_dump, compile, Environment, InstructionAddress};
use werbolg_core::{id::IdF, AbsPath, Ident, Module, Namespace};
use werbolg_exec::{ExecutionEnviron, ExecutionMachine, ExecutionParams, WAllocator, NIF};
use werbolg_lang_common::{Report, ReportKind, Source};

pub fn run_frontend(
    params: &TalesParams,
    args: &[String],
) -> Result<(Source, Module), Box<dyn Error>> {
    if args.is_empty() {
        crate::help();
        return Err(format!("no file specified").into());
    }

    let path = std::path::PathBuf::from(&args[0]);
    let source = get_file(&path)?;

    let parsing_res = match params.frontend {
        Some(Frontend::Rusty) => werbolg_lang_rusty::module(&source.file_unit),
        Some(Frontend::Lispy) => werbolg_lang_lispy::module(&source.file_unit),
        None => {
            // work similar to auto detect
            let parse1 = werbolg_lang_rusty::module(&source.file_unit);
            let parse2 = werbolg_lang_lispy::module(&source.file_unit);

            parse1.or_else(|_e1| parse2)
        }
    };
    let module = match parsing_res {
        Err(es) => {
            for e in es.into_iter() {
                let report = Report::new(ReportKind::Error, format!("Parse Error: {:?}", e))
                    .lines_before(1)
                    .lines_after(1)
                    .highlight(e.location, format!("parse error here"));

                report_print(&source, report)?;
            }
            return Err(format!("parse error").into());
        }
        Ok(module) => module,
    };

    if params.dump_ir {
        std::println!("{:#?}", module);
    }
    Ok((source, module))
}

pub fn report_print(source: &Source, report: Report) -> Result<(), Box<dyn Error>> {
    let mut s = String::new();
    report.write(&source, &mut s)?;
    println!("{}", s);
    Ok(())
}

pub fn run_compile<'m, 'e, A>(
    params: &TalesParams,
    env: &mut Environment<NIF<'m, 'e, A, environ::MyLiteral, (), Value>, Value>,
    source: Source,
    module: Module,
) -> Result<werbolg_compile::CompilationUnit<environ::MyLiteral>, Box<dyn Error>> {
    let module_ns = Namespace::root().append(Ident::from("main"));
    let modules = vec![(module_ns.clone(), module)];

    let compilation_params = werbolg_compile::CompilationParams {
        literal_mapper: environ::literal_mapper,
        sequence_constructor: None,
    };

    let exec_module = match compile(&compilation_params, modules, env) {
        Err(e) => {
            let report = Report::new(ReportKind::Error, format!("Compilation Error: {:?}", e))
                .lines_before(1)
                .lines_after(1)
                .highlight(e.span(), format!("compilation error here"));
            report_print(&source, report)?;
            return Err(format!("compilation error {:?}", e).into());
        }
        Ok(m) => m,
    };

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

fn get_file(path: &std::path::Path) -> std::io::Result<Source> {
    let path = std::path::PathBuf::from(&path);
    let content = std::fs::read_to_string(&path).expect("file read");
    let fileunit = Source::from_string(path.to_string_lossy().to_string(), content);
    Ok(fileunit)
}
