//! Werbolg Execution machine

use crate::ast;

mod bindings;
mod value;

use alloc::{rc::Rc, vec::Vec};
use bindings::BindingsStack;
use core::{borrow::BorrowMut, cell::RefCell};
pub use value::{Value, ValueKind};

pub struct ExecutionMachine {
    pub _bindings: RefCell<BindingsStack<BindingValue>>,
}

impl ExecutionMachine {
    pub fn new() -> Self {
        Self {
            _bindings: RefCell::new(BindingsStack::new()),
        }
    }

    pub fn add_binding(&self, ident: ast::Ident, value: Value) {
        let mut bindings = self._bindings.borrow_mut();
        bindings.add(ident, value)
    }

    pub fn get_binding(&self, ident: &ast::Ident) -> Result<Value, ExecutionError> {
        let bindings = self._bindings.borrow_mut();
        let bind = bindings.get(ident);
        match bind {
            None => Err(ExecutionError::MissingBinding(ident.clone())),
            Some(val) => Ok(val.clone()),
        }
    }

    pub fn scope_enter(&self) {
        let mut bindings = self._bindings.borrow_mut();
        bindings.scope_enter()
    }

    pub fn scope_leave(&self) {
        let mut bindings = self._bindings.borrow_mut();
        bindings.scope_leave()
    }
}

pub type BindingValue = Value;

#[derive(Debug, Clone)]
pub enum ExecutionError {
    ArityError {
        expected: usize,
        got: usize,
    },
    MissingBinding(ast::Ident),
    CallingNotFunc {
        value_is: ValueKind,
    },
    ValueKindUnexpected {
        value_expected: ValueKind,
        value_got: ValueKind,
    },
}

pub fn exec(em: &ExecutionMachine, module: ast::Module) -> Result<Value, ExecutionError> {
    exec_stmts(em, &module.statements)
}

pub fn exec_stmts(
    em: &ExecutionMachine,
    stmts: &[ast::Statement],
) -> Result<Value, ExecutionError> {
    let mut last_value = None;
    for statement in stmts {
        match statement {
            ast::Statement::Function(name, params, stmts) => {
                em.add_binding(name.clone(), Value::Fun(params.clone(), stmts.clone()));
            }
            ast::Statement::Expr(e) => {
                let v = exec_expr(em, &e)?;
                last_value = Some(v)
            }
        }
    }
    match last_value {
        None => Ok(Value::Unit),
        Some(val) => Ok(val),
    }
}

pub fn exec_expr(em: &ExecutionMachine, e: &ast::Expr) -> Result<Value, ExecutionError> {
    match e {
        ast::Expr::Literal(lit) => Ok(Value::from(lit)),
        ast::Expr::List(list_exprs) => {
            let r = list_exprs
                .iter()
                .map(|l| exec_expr(em, l))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::List(r))
        }
        ast::Expr::Ident(ident) => em.get_binding(ident),
        ast::Expr::Let(ident, bind_expr, then_expr) => {
            let value = exec_expr(em, bind_expr)?;
            em.add_binding(ident.clone(), value);
            exec_expr(em, then_expr)
        }
        ast::Expr::Then(first_expr, second_expr) => {
            let value1 = exec_expr(em, first_expr)?;
            value1.unit()?;
            let value2 = exec_expr(em, second_expr)?;
            Ok(value2)
        }
        ast::Expr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            let cond_val = exec_expr(em, cond)?;
            let cond_bool = cond_val.bool()?;

            let ret = if cond_bool {
                exec_expr(em, then_expr)
            } else {
                exec_expr(em, else_expr)
            }?;
            Ok(ret)
        }
        ast::Expr::Call(c) => {
            let resolved = c
                .iter()
                .map(|e| exec_expr(em, e))
                .collect::<Result<Vec<_>, _>>()?;
            if let Some((first, args)) = resolved.split_first() {
                let k = first.into();
                match first {
                    Value::Fun(bind_names, fun_stmts) => {
                        em.scope_enter();
                        check_arity(bind_names.len(), args.len())?;
                        for (bind_name, arg_value) in bind_names.iter().zip(args.iter()) {
                            em.add_binding(bind_name.clone(), arg_value.clone())
                        }
                        let value = exec_stmts(em, fun_stmts)?;
                        em.scope_leave();
                        Ok(value)
                    }
                    Value::NativeFun(f) => {
                        let res = f(em, args)?;
                        Ok(res)
                    }
                    Value::List(_)
                    | Value::Bool(_)
                    | Value::Number(_)
                    | Value::String(_)
                    | Value::Decimal(_)
                    | Value::Bytes(_)
                    | Value::Opaque(_)
                    | Value::Unit => Err(ExecutionError::CallingNotFunc { value_is: k }),
                }
            } else {
                //panic!("empty call");
                Ok(Value::Unit)
            }
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
