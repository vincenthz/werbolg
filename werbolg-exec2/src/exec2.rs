use super::NIFCall;
use super::{ExecutionError, ExecutionMachine, Value, ValueKind};
use ir::InstructionAddress;
use werbolg_core as ir;
use werbolg_core::lir::{CallArity, LocalStackSize};
use werbolg_core::ValueFun;

pub fn exec<'module, T>(
    em: &mut ExecutionMachine<'module, T>,
    call: ir::FunId,
    args: &[Value],
) -> Result<Value, ExecutionError> {
    em.stack2.push_call(Value::Fun(ValueFun::Fun(call)), args);

    match process_call(em, CallArity(args.len() as u32))? {
        CallResult::Jump(ip, local) => {
            em.ip_set(ip);
            em.sp_set(local);
        }
        CallResult::Value(value) => return Ok(value),
    };

    println!("===== initial =====");
    em.debug_state();
    println!("===================");

    exec_loop(em)
}
pub fn exec_continue<'m, T>(em: &mut ExecutionMachine<'m, T>) -> Result<Value, ExecutionError> {
    if em.rets.is_empty() {
        return Err(ExecutionError::ExecutionFinished);
    }
    exec_loop(em)
}

fn exec_loop<'m, T>(em: &mut ExecutionMachine<'m, T>) -> Result<Value, ExecutionError> {
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
    /*
    print!(
        "exec IP={} SP={} STACK={} INSTR={:?} => ",
        em.ip,
        em.sp.0,
        em.stack2.top().0,
        instr
    );
    */
    match instr {
        ir::lir::Statement::PushLiteral(lit) => {
            let literal = &em.module.lits[*lit];
            em.stack2.push_value(Value::from(literal));
            em.ip_next();
        }
        ir::lir::Statement::FetchGlobal(global_id) => {
            em.sp_push_value_from_global(*global_id);
            em.ip_next();
        }
        ir::lir::Statement::FetchNif(nif_id) => {
            em.stack2.push_value(Value::Fun(ValueFun::Native(*nif_id)));
            em.ip_next();
        }
        ir::lir::Statement::FetchFun(fun_id) => {
            em.stack2.push_value(Value::Fun(ValueFun::Fun(*fun_id)));
            em.ip_next();
        }
        ir::lir::Statement::FetchStackLocal(local_bind) => {
            em.sp_push_value_from_local(*local_bind);
            em.ip_next()
        }
        ir::lir::Statement::FetchStackParam(param_bind) => {
            em.sp_push_value_from_param(*param_bind);
            em.ip_next()
        }
        ir::lir::Statement::AccessField(_) => todo!(),
        ir::lir::Statement::LocalBind(local_bind) => {
            let val = em.stack2.pop_value();
            em.sp_set_value_at(*local_bind, val);
            em.ip_next();
        }
        ir::lir::Statement::IgnoreOne => {
            let _ = em.stack2.pop_value();
            em.ip_next();
        }
        ir::lir::Statement::Call(arity) => {
            let val = process_call(em, *arity)?;
            match val {
                CallResult::Jump(fun_ip, local_stack_size) => {
                    em.rets
                        .push((em.ip.next(), em.sp, local_stack_size, *arity));
                    em.sp_set(local_stack_size);
                    em.ip_set(fun_ip);
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
                em.ip_next()
            } else {
                em.ip_jump(*d)
            }
        }
        ir::lir::Statement::Ret => {
            let val = em.stack2.pop_value();
            match em.rets.pop() {
                None => return Ok(Some(val)),
                Some((ret, sp, stack_size, arity)) => {
                    em.sp_unlocal(em.current_stack_size);
                    em.current_stack_size = stack_size;
                    em.stack2.pop_call(arity);
                    em.sp = sp;
                    em.stack2.push_value(val);
                    em.ip_set(ret)
                }
            }
        }
    }
    //println!("IP={} SP={} STACK={}", em.ip, em.sp.0, em.stack2.top().0);
    Ok(None)
}

enum CallResult {
    Jump(InstructionAddress, LocalStackSize),
    Value(Value),
}

fn process_call<'m, T>(
    em: &mut ExecutionMachine<'m, T>,
    arity: CallArity,
) -> Result<CallResult, ExecutionError> {
    let first = em.stack2.get_call(arity);
    let Value::Fun(fun) = first else {
        em.debug_state();
        let kind = first.into();
        return Err(ExecutionError::ValueKindUnexpected {
            value_expected: ValueKind::Fun,
            value_got: kind,
        });
    };

    match fun {
        ValueFun::Native(nifid) => {
            let res = match &em.nifs[*nifid].call {
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
        ValueFun::Fun(funid) => {
            let call_def = &em.module.funs[*funid];
            Ok(CallResult::Jump(call_def.code_pos, call_def.stack_size))
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
