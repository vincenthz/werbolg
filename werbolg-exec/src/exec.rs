//! Execution core
//!
//! This is composed of 4 things:
//!
//! * IP : Instruction Pointer for the next instruction to execute
//! * SP : Stack Pointer of the current function
//! * Stack of values : Values being pushed on the stack for calls, and operations
//! * Call frames : a Stack of CallFrame to recover the context of previous function after returning
//!                 from a function call
//!
//! ## Call Frames
//!
//!
//!
//! ## Stack of Value
//!
//! The stack of values grows by adding values to the top.
//! Most operations takes values from the top of stack and
//! push values to the top of the stack.
//!
//! Local bounded variable are reserved on the stack after the current SP.
//!
//! Function are a "pointer" to the function itself (FunId or NifId) `Fun`,
//! and the arity of A `ValX`, followed by N Local value `LocalX`.
//!
//! Function Stack after a 'Call' operation:
//!
//! ```text
//!      ┌──────────────────┬───────────────────────┐
//!      │                  │                       │
//!      │ Fun callee stack │  Fun local stack      │
//!      ▼                  ▼                       ▼
//!  ──┬─┬───┬────┬────┬────┬──────┬──────┬──┬──────┐
//!  ..│X│Fun│Val1│Val2│Val3│Local1│Local2│..│LocalN│
//!  ──┴─┴───┴────┴────┴────┼──────┴──────┴──┴──────┤
//!                         │                       │
//!                         ▼                       ▼
//!                        SP                    Stack top
//! ```
//!
//! After a 'Ret' operation:
//!
//! ```text
//!  ──┬─┬──────┐
//!  ..|X│RetVal│
//!  ──┴─┴──────┤
//!             │
//!             ▼
//!           Stack top
//!
//! The initial setup of the call stack, considering the entry point `main` and the parameter values A and B is:
//!
//!  ┌────────┬─┬─┬──────┬──┬──────┐
//!  │Fun main│A│B│local1│..│localN│
//!  ┤────────┴─┴─┼──────┴──┴──────┤
//!  ▼            ▼                ▼
//! Stack bottom  SP            Stack top
//! ```
//!
use crate::{CallSave, Valuable};

use super::allocator::WAllocator;
use super::{ExecutionError, ExecutionMachine};
use werbolg_compile::{CallArity, Instruction, InstructionAddress, LocalStackSize, TailCall};
use werbolg_core as ir;
use werbolg_core::ValueFun;

/// Native Implemented Function
pub struct NIF<'m, 'e, A, L, T, V> {
    /// name of the NIF
    pub name: &'static str,
    /// the call itself
    pub call: NIFCall<'m, 'e, A, L, T, V>,
}

/// 2 Variants of Native calls
///
/// * "Pure" function that don't have access to the execution machine
/// * "Mut" function that have access to the execution machine and have more power / responsability.
pub enum NIFCall<'m, 'e, A, L, T, V> {
    /// "Pure" NIF call only takes the input parameter and return an output
    Pure(fn(&[V]) -> Result<V, ExecutionError>),
    /// "Raw" NIF takes the execution machine in parameter and return an output
    Raw(fn(&mut ExecutionMachine<'m, 'e, A, L, T, V>) -> Result<V, ExecutionError>),
}

/// Execute the module, calling function identified by FunId, with the arguments in parameters.
pub fn exec<'module, 'environ, A: WAllocator<Value = V>, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'module, 'environ, A, L, T, V>,
    call: ir::FunId,
    args: &[V],
) -> Result<V, ExecutionError> {
    match initialize(em, call, args)? {
        None => (),
        Some(v) => return Ok(v),
    };

    exec_loop(em)
}

/// Initialize the execution machine with a call to the specified function (by FunId)
/// and the arguments to this function as values
pub fn initialize<'module, 'environ, A: WAllocator<Value = V>, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'module, 'environ, A, L, T, V>,
    call: ir::FunId,
    args: &[V],
) -> Result<Option<V>, ExecutionError> {
    let arity = args
        .len()
        .try_into()
        .map(CallArity)
        .map_err(|_| ExecutionError::ArityOverflow { got: args.len() })?;

    // truncate the stack of values and rets if any
    em.stack.truncate(0);
    em.rets.truncate(0);

    em.stack.push_call(V::make_fun(ValueFun::Fun(call)), args);

    match process_call(em, arity)? {
        CallResult::Jump(ip, local) => {
            em.ip_set(ip);
            em.sp_set(local);
        }
        CallResult::Value(value) => return Ok(Some(value)),
    };
    Ok(None)
}

/// Resume execution
///
/// If the stack is empty (if the program is terminated already), then it returns an ExecutionFinished error
pub fn exec_continue<'m, 'e, A: WAllocator<Value = V>, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, 'e, A, L, T, V>,
) -> Result<V, ExecutionError> {
    if em.rets.is_empty() {
        return Err(ExecutionError::ExecutionFinished);
    }
    exec_loop(em)
}

fn exec_loop<'m, 'e, A: WAllocator<Value = V>, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, 'e, A, L, T, V>,
) -> Result<V, ExecutionError> {
    loop {
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
pub fn step<'m, 'e, A: WAllocator<Value = V>, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, 'e, A, L, T, V>,
) -> StepResult<V> {
    // fetch the next instruction from ip, if ip points to a random place, raise an error
    let Some(instr) = &em.module.code.get(em.ip) else {
        return Err(ExecutionError::IpInvalid { ip: em.ip });
    };
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
            em.sp_set_local_value_at(*local_bind, val);
            em.ip_next();
        }
        Instruction::IgnoreOne => {
            let _ = em.stack.pop_value();
            em.ip_next();
        }
        Instruction::Call(tc, arity) => {
            let val = process_call(em, *arity)?;
            match val {
                CallResult::Jump(fun_ip, local_stack_size) => {
                    if *tc == TailCall::Yes {
                        // if we have a tail call, we don't need to save the current call frame
                        // we just shift the values to replace the call stack and
                        // replace the current state (sp, ip, current_arity)
                        em.sp_move_rel(*arity, em.current_arity, local_stack_size);
                        em.current_arity = *arity;
                        em.ip_set(fun_ip);
                    } else {
                        em.rets.push(CallSave {
                            ip: em.ip.next(),
                            sp: em.sp,
                            arity: em.current_arity,
                        });
                        em.current_arity = *arity;
                        em.sp_set(local_stack_size);
                        em.ip_set(fun_ip);
                    }
                }
                CallResult::Value(nif_val) => {
                    em.stack.pop_call(*arity);

                    if *tc == TailCall::Yes {
                        match em.rets.pop() {
                            None => return Ok(Some(nif_val)),
                            Some(call_frame) => do_ret(em, call_frame, nif_val),
                        }
                    } else {
                        em.stack.push_value(nif_val);
                        em.ip_next()
                    }
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
                Some(call_frame) => do_ret(em, call_frame, val),
            }
        }
    }
    //println!("IP={} SP={} STACK={}", em.ip, em.sp.0, em.stack2.top().0);
    Ok(None)
}

fn do_ret<'m, 'e, A: WAllocator, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, 'e, A, L, T, V>,
    CallSave { ip, sp, arity }: CallSave,
    value: V,
) {
    // remove any value after the current stack pointer (remove all local and temp values)
    em.stack.truncate(em.sp.0);
    // pop the calls from the stack
    em.stack.pop_call(em.current_arity);
    // restore state of the caller
    em.current_arity = arity;
    em.sp = sp;
    em.ip_set(ip);
    // push the return value to replace
    em.stack.push_value(value);
}

enum CallResult<V> {
    Jump(InstructionAddress, LocalStackSize),
    Value(V),
}

fn process_call<'m, 'e, A: WAllocator, L, T, V: Valuable>(
    em: &mut ExecutionMachine<'m, 'e, A, L, T, V>,
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
            let res = match &em.environ.nifs[nifid].call {
                NIFCall::Pure(nif) => {
                    let (_first, args) = em.stack.get_call_and_args(arity);
                    nif(args)?
                }
                NIFCall::Raw(nif) => nif(em)?,
            };
            Ok(CallResult::Value(res))
        }
        ValueFun::Fun(funid) => {
            let call_def = &em.module.funs[funid];
            if call_def.arity != arity {
                return Err(ExecutionError::ArityError {
                    funid,
                    expected: call_def.arity,
                    got: arity,
                });
            }
            Ok(CallResult::Jump(call_def.code_pos, call_def.stack_size))
        }
    }
}
