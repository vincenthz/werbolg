use std::error::Error;

mod args;
mod environ;
mod exec;
mod params;
mod value;

use environ::create_env;
use exec::*;
use params::{Frontend, TalesParams};
use werbolg_exec::WerRefCount;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Flag {
    Help,
    Version,
    DumpIr,
    DumpInstr,
    ExecStepTrace,
    StepAddress(u64),
    Frontend(Frontend),
}

fn version() {
    println!("v0.0.1")
}

fn help() {
    println!(
        r#"
usage: werbolg-tales [options] <file>

Options:
  --help              Print this help
  --version           Print the version of werbolg-tales
  --dump-ir           Dump the IR on stdout
  --dump-instr        Dump the Code Instructions on stdout
  --exec-step-trace   Trace every step of execution
  --step-address <a>  Address to print a debug trace
  --frontend <value>  Set the frontend to use a specific frontend
    "#
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = args::ArgOptions {
        short: &[],
        long: &[
            ("help", args::FlagDescr::NoArg(Box::new(|| Flag::Help))),
            (
                "version",
                args::FlagDescr::NoArg(Box::new(|| Flag::Version)),
            ),
            ("dump-ir", args::FlagDescr::NoArg(Box::new(|| Flag::DumpIr))),
            (
                "dump-instr",
                args::FlagDescr::NoArg(Box::new(|| Flag::DumpInstr)),
            ),
            (
                "exec-step-trace",
                args::FlagDescr::NoArg(Box::new(|| Flag::ExecStepTrace)),
            ),
            (
                "step-address",
                args::FlagDescr::Arg(Box::new(|s| {
                    if let Ok(p) = u64::from_str_radix(&s, 16) {
                        Ok(Flag::StepAddress(p))
                    } else {
                        Err(format!("step address '{}' is invalid", s))
                    }
                })),
            ),
            (
                "frontend",
                args::FlagDescr::Arg(Box::new(|s| {
                    if s == "rusty" {
                        Ok(Flag::Frontend(Frontend::Rusty))
                    } else if s == "lispy" {
                        Ok(Flag::Frontend(Frontend::Lispy))
                    } else {
                        Err(format!("unknown frontend {}", s))
                    }
                })),
            ),
        ],
    };
    let (flags, args) = args::args(options)?;

    let help_req = flags.contains(&Flag::Help);
    let ver_req = flags.contains(&Flag::Version);

    if help_req {
        help();
        return Ok(());
    }
    if ver_req {
        version();
        return Ok(());
    }

    let dump_ir = flags.contains(&Flag::DumpIr);
    let dump_instr = flags.contains(&Flag::DumpInstr);
    let exec_step_trace = flags.contains(&Flag::ExecStepTrace);
    let step_address = flags
        .iter()
        .filter_map(|f| match f {
            Flag::StepAddress(f) => Some(*f),
            _ => None,
        })
        .collect::<Vec<_>>();
    let frontend = flags
        .iter()
        .filter_map(|f| match f {
            Flag::Frontend(f) => Some(*f),
            _ => None,
        })
        .last();

    let params = TalesParams {
        dump_ir,
        dump_instr,
        exec_step_trace,
        step_address,
        frontend,
    };

    let (source, module) = run_frontend(&params, &args)?;

    let mut env = create_env();
    let compile_unit = run_compile(&params, &mut env, source, module)?;

    let ee = werbolg_exec::WerRefCount::new(
        werbolg_exec::ExecutionEnviron::from_compile_environment(env.finalize()),
    );
    run_exec(&params, ee, WerRefCount::new(compile_unit))?;

    Ok(())
}
