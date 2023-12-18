use crate::Valuable;

use super::NIFCall;
use super::{ExecutionError, ExecutionMachine};
use werbolg_compile::{CallArity, Instruction, InstructionAddress, LocalStackSize};
use werbolg_core as ir;
use werbolg_core::ValueFun;

pub fn exec<'module, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'module, L, T, V>,
    call: ir::FunId,
    args: &[V],
) -> Result<V, ExecutionError> {
    let arity = args
        .len()
        .try_into()
        .map(CallArity)
        .map_err(|_| ExecutionError::ArityOverflow { got: args.len() })?;

    em.stack.push_call(V::make_fun(ValueFun::Fun(call)), args);

    match process_call(em, arity)? {
        CallResult::Jump(ip, local) => {
            em.ip_set(ip);
            em.sp_set(local);
        }
        CallResult::Value(value) => return Ok(value),
    };

    //println!("===== initial =====");
    //em.debug_state();
    //println!("===================");

    exec_loop(em)
}

pub fn exec_continue<'m, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, L, T, V>,
) -> Result<V, ExecutionError> {
    if em.rets.is_empty() {
        return Err(ExecutionError::ExecutionFinished);
    }
    exec_loop(em)
}

fn exec_loop<'m, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, L, T, V>,
) -> Result<V, ExecutionError> {
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

type StepResult<V> = Result<Option<V>, ExecutionError>;

/// Step through 1 single instruction, and returning a Step Result which is either:
///
/// * an execution error
/// * not an error : Either no value or a value if the execution of the program is finished
///
/// The step function need to update the execution IP
pub fn step<'m, L, T, V: Valuable>(em: &mut ExecutionMachine<'m, L, T, V>) -> StepResult<V> {
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
        Instruction::PushLiteral(lit) => {
            let literal = &em.module.lits[*lit];
            em.stack.push_value((em.params.literal_to_value)(literal));
            em.ip_next();
        }
        Instruction::FetchGlobal(global_id) => {
            em.sp_push_value_from_global(*global_id);
            em.ip_next();
        }
        Instruction::FetchNif(nif_id) => {
            em.stack.push_value(V::make_fun(ValueFun::Native(*nif_id)));
            em.ip_next();
        }
        Instruction::FetchFun(fun_id) => {
            em.stack.push_value(V::make_fun(ValueFun::Fun(*fun_id)));
            em.ip_next();
        }
        Instruction::FetchStackLocal(local_bind) => {
            em.sp_push_value_from_local(*local_bind);
            em.ip_next()
        }
        Instruction::FetchStackParam(param_bind) => {
            em.sp_push_value_from_param(*param_bind);
            em.ip_next()
        }
        Instruction::AccessField(expected_cid, idx) => {
            let val = em.stack.pop_value();
            let Some((got_cid, inner)) = val.structure() else {
                return Err(ExecutionError::ValueNotStruct {
                    value_is: val.descriptor(),
                });
            };

            if got_cid != *expected_cid {
                return Err(ExecutionError::StructMismatch {
                    constr_expected: *expected_cid,
                    constr_got: got_cid,
                });
            }
            if idx.0 as usize >= inner.len() {
                return Err(ExecutionError::StructFieldOutOfBound {
                    constr: got_cid,
                    field_index: *idx,
                    struct_len: inner.len(),
                });
            }
            em.stack.push_value(inner[idx.0 as usize].clone());
            em.ip_next()
        }
        Instruction::LocalBind(local_bind) => {
            let val = em.stack.pop_value();
            em.sp_set_value_at(*local_bind, val);
            em.ip_next();
        }
        Instruction::IgnoreOne => {
            let _ = em.stack.pop_value();
            em.ip_next();
        }
        Instruction::Call(arity) => {
            let val = process_call(em, *arity)?;
            match val {
                CallResult::Jump(fun_ip, local_stack_size) => {
                    em.rets
                        .push((em.ip.next(), em.sp, local_stack_size, *arity));
                    em.sp_set(local_stack_size);
                    em.ip_set(fun_ip);
                }
                CallResult::Value(nif_val) => {
                    em.stack.pop_call(*arity);
                    em.stack.push_value(nif_val);
                    em.ip_next()
                }
            }
        }
        Instruction::Jump(d) => em.ip_jump(*d),
        Instruction::CondJump(d) => {
            let val = em.stack.pop_value();
            let Some(b) = val.conditional() else {
                return Err(ExecutionError::ValueNotConditional {
                    value_is: val.descriptor(),
                });
            };
            if b {
                em.ip_next()
            } else {
                em.ip_jump(*d)
            }
        }
        Instruction::Ret => {
            let val = em.stack.pop_value();
            match em.rets.pop() {
                None => return Ok(Some(val)),
                Some((ret, sp, stack_size, arity)) => {
                    em.sp_unlocal(em.current_stack_size);
                    em.current_stack_size = stack_size;
                    em.stack.pop_call(arity);
                    em.sp = sp;
                    em.stack.push_value(val);
                    em.ip_set(ret)
                }
            }
        }
    }
    //println!("IP={} SP={} STACK={}", em.ip, em.sp.0, em.stack2.top().0);
    Ok(None)
}

enum CallResult<V> {
    Jump(InstructionAddress, LocalStackSize),
    Value(V),
}

fn process_call<'m, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, L, T, V>,
    arity: CallArity,
) -> Result<CallResult<V>, ExecutionError> {
    let first = em.stack.get_call(arity);
    let Some(fun) = first.fun() else {
        //em.debug_state();
        return Err(ExecutionError::CallingNotFunc {
            value_is: first.descriptor(),
        });
    };

    match fun {
        ValueFun::Native(nifid) => {
            let res = match &em.nifs[nifid].call {
                NIFCall::Pure(nif) => {
                    let (_first, args) = em.stack.get_call_and_args(arity);
                    nif(args)?
                }
                NIFCall::Mut(_nif) => {
                    todo!()
                }
            };
            Ok(CallResult::Value(res))
        }
        ValueFun::Fun(funid) => {
            let call_def = &em.module.funs[funid];
            Ok(CallResult::Jump(call_def.code_pos, call_def.stack_size))
        }
    }
}

fn _check_arity(expected: usize, got: usize) -> Result<(), ExecutionError> {
    if expected == got {
        Ok(())
    } else {
        Err(ExecutionError::ArityError { expected, got })
    }
}
