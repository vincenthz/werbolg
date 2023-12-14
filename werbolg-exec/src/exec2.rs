use super::NIFCall;
use super::{ExecutionError, ExecutionMachine, Value};
use ir::code::InstructionDiff;
use ir::InstructionAddress;
use werbolg_core as ir;
use werbolg_core::lir::CallArity;

pub fn exec<'module, T>(
    em: &mut ExecutionMachine<'module, T>,
    call: ir::Ident,
    args: &[Value],
) -> Result<Value, ExecutionError> {
    // setup the initial value stack, where we inject a dummy function call and then
    // the arguments to this function
    em.stack2.push_call(em.get_binding(&call)?, args);

    match process_call(em, CallArity(args.len() as u32))? {
        CallResult::Jump(ip) => em.ip_set(ip),
        CallResult::Value(value) => return Ok(value),
    };

    exec_continue(em)
}

pub fn exec_continue<'m, T>(em: &mut ExecutionMachine<'m, T>) -> Result<Value, ExecutionError> {
    if em.rets.is_empty() {
        return Err(ExecutionError::ExecutionFinished);
    }

    loop {
        if em.aborted() {
            return Err(ExecutionError::Abort);
        }
        match step(em)? {
            None => {}
            Some(v) => break Ok(v),
        }
    }
}

type StepResult = Result<Option<Value>, ExecutionError>;

/// Step through 1 single instruction, and returning a Step Result which is either:
///
/// * an execution error
/// * not an error : Either no value or a value if the execution of the program is finished
///
/// The step function need to update the execution IP
pub fn step<'m, T>(em: &mut ExecutionMachine<'m, T>) -> StepResult {
    let instr = &em.module.code[em.ip];
    match instr {
        ir::lir::Statement::PushLiteral(lit) => {
            let literal = &em.module.lits[*lit];
            em.stack2.push_value(Value::from(literal));
            em.ip_next();
        }
        ir::lir::Statement::FetchIdent(ident) => {
            em.stack2.push_value(em.get_binding(ident)?);
            em.ip_next()
        }
        ir::lir::Statement::AccessField(_) => todo!(),
        ir::lir::Statement::LocalBind(_) => todo!(),
        ir::lir::Statement::IgnoreOne => todo!(),
        ir::lir::Statement::Call(arity) => {
            let val = process_call(em, *arity)?;
            match val {
                CallResult::Jump(fun_ip) => {
                    let saved = em.ip;
                    em.rets.push((saved, *arity));
                    em.ip_set(fun_ip)
                }
                CallResult::Value(nif_val) => {
                    em.stack2.pop_call(*arity);
                    em.stack2.push_value(nif_val);
                    em.ip_next()
                }
            }
        }
        ir::lir::Statement::Jump(d) => em.ip_jump(*d),
        ir::lir::Statement::CondJump(d) => {
            let val = em.stack2.pop_value();
            let b = val.bool()?;
            if b {
                em.ip_jump(*d)
            } else {
                em.ip_next()
            }
        }
        ir::lir::Statement::Ret => {
            let val = em.stack2.pop_value();
            match em.rets.pop() {
                None => return Ok(Some(val)),
                Some((ret, arity)) => {
                    em.sp_unwind(arity);
                    em.stack2.pop_call(arity);
                    em.stack2.push_value(val);
                    em.ip_set(ret)
                }
            }
        }
    }
    Ok(None)
}

enum CallResult {
    Jump(InstructionAddress),
    Value(Value),
}

fn process_call<'m, T>(
    em: &mut ExecutionMachine<'m, T>,
    arity: CallArity,
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
            let call_def = &em.module.funs[funid];
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
