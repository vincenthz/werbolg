use super::location::Location;
use super::NIFCall;
use super::{ExecutionError, ExecutionMachine, Value};
use alloc::{string::String, vec, vec::Vec};
use ir::InstructionAddress;
use werbolg_core as ir;

pub fn exec<'module, T>(
    em: &mut ExecutionMachine<'module, T>,
    call: ir::Ident,
    args: &[Value],
) -> Result<Value, ExecutionError> {
    // setup the initial value stack, where we inject a dummy function call and then
    // the arguments to this function
    em.stack2.push_call(em.get_binding(&call)?, args);

    match process_call(em, args.len())? {
        CallResult::Jump(ip) => {
            em.ip = ip;
        }
        CallResult::Value(_) => {
            panic!("NIF cannot be used as entry point")
        }
    };

    exec_continue(em)
}

pub fn exec_continue<'m, T>(em: &mut ExecutionMachine<'m, T>) -> Result<Value, ExecutionError> {
    loop {
        if em.aborted() {
            return Err(ExecutionError::Abort);
        }
        let instr = &em.module.code[em.ip];
        em.ip = em.ip.next();
        match instr {
            ir::lir::Statement::PushLiteral(lit) => {
                let literal = &em.module.lits[*lit];
                em.stack2.push_value(Value::from(literal))
            }
            ir::lir::Statement::FetchIdent(ident) => em.stack2.push_value(em.get_binding(ident)?),
            ir::lir::Statement::AccessField(_) => todo!(),
            ir::lir::Statement::LocalBind(_) => todo!(),
            ir::lir::Statement::IgnoreOne => todo!(),
            ir::lir::Statement::Call(arity) => {
                let val = process_call(em, *arity)?;
                match val {
                    CallResult::Jump(fun_ip) => {
                        let saved = em.ip;
                        em.rets.push(saved);
                        em.ip = fun_ip;
                    }
                    CallResult::Value(nif_val) => {
                        em.stack2.pop_call(*arity);
                        em.stack2.push_value(nif_val);
                    }
                }
            }
            ir::lir::Statement::Jump(_) => todo!(),
            ir::lir::Statement::CondJump(_) => todo!(),
            ir::lir::Statement::Ret(arity) => {
                let val = em.stack2.pop_value();
                em.stack2.pop_call(*arity);
                match em.rets.pop() {
                    None => break Ok(val),
                    Some(ret) => {
                        em.stack2.push_value(val);
                        em.ip = ret;
                    }
                }
            }
        }
    }
}

enum CallResult {
    Jump(InstructionAddress),
    Value(Value),
}

fn process_call<'m, T>(
    em: &mut ExecutionMachine<'m, T>,
    arity: usize,
) -> Result<CallResult, ExecutionError> {
    let first = em.stack2.get_call(arity);
    let fun = first.fun()?;
    match fun {
        crate::value::ValueFun::Native(nifid) => {
            let res = match &em.nifs[nifid.0 as usize].call {
                NIFCall::Pure(nif) => {
                    let (_first, args) = em.stack2.get_call_and_args(arity);
                    nif(args)?
                }
                NIFCall::Mut(nif) => {
                    todo!()
                }
            };
            Ok(CallResult::Value(res))
        }
        crate::value::ValueFun::Fun(funid) => {
            // setup the instruction pointer to the called function
            let call_def = &em.module.funs[funid];
            //let saved = em.ip.next();
            //em.ip = call_def.code_pos;
            Ok(CallResult::Jump(call_def.code_pos))
        }
    }
}

fn check_arity(expected: usize, got: usize) -> Result<(), ExecutionError> {
    if expected == got {
        Ok(())
    } else {
        Err(ExecutionError::ArityError { expected, got })
    }
}
