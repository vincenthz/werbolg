//! Werbolg Execution machine

use crate::ast;

mod bindings;
mod location;
mod value;

use alloc::vec::Vec;
use bindings::BindingsStack;
use core::cell::RefCell;
pub use location::Location;
pub use value::{Value, ValueKind};

pub struct ExecutionMachine {
    pub _bindings: RefCell<BindingsStack<BindingValue>>,
    pub stacktrace: RefCell<Vec<Location>>,
}

impl ExecutionMachine {
    pub fn new() -> Self {
        Self {
            _bindings: RefCell::new(BindingsStack::new()),
            stacktrace: RefCell::new(Vec::new()),
        }
    }

    pub fn add_binding(&self, ident: ast::Ident, value: Value) {
        let mut bindings = self._bindings.borrow_mut();
        bindings.add(ident, value)
    }

    pub fn with_added_binding<A, F>(&self, ident: ast::Ident, value: Value, f: F) -> A
    where
        F: Fn() -> A,
    {
        {
            let mut bindings = self._bindings.borrow_mut();
            bindings.add(ident.clone(), value);
        };
        let r = f();
        {
            let mut bindings = self._bindings.borrow_mut();
            bindings.remove(ident);
        };
        r
    }

    pub fn get_binding(&self, ident: &ast::Ident) -> Result<Value, ExecutionError> {
        let bindings = self._bindings.borrow_mut();
        let bind = bindings.get(ident);
        match bind {
            None => Err(ExecutionError::MissingBinding(ident.clone())),
            Some(val) => Ok(val.clone()),
        }
    }

    pub fn scope_enter(&self, location: &Location) {
        let mut bindings = self._bindings.borrow_mut();
        bindings.scope_enter();
        let mut stacktrace = self.stacktrace.borrow_mut();
        stacktrace.push(location.clone())
    }

    pub fn scope_leave(&self) {
        let mut bindings = self._bindings.borrow_mut();
        bindings.scope_leave();
        let mut stacktrace = self.stacktrace.borrow_mut();
        stacktrace.pop();
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
        location: Location,
        value_is: ValueKind,
    },
    ValueKindUnexpected {
        value_expected: ValueKind,
        value_got: ValueKind,
    },
    Abort,
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
            ast::Statement::Function(span, name, params, stmts) => {
                em.add_binding(
                    name.clone(),
                    Value::Fun(Location::from_span(span), params.clone(), stmts.clone()),
                );
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

pub struct ExecutionStack {
    pub values: Vec<Value>,
    pub constr: Vec<()>,
}

pub fn exec_expr(em: &ExecutionMachine, e: &ast::Expr) -> Result<Value, ExecutionError> {
    match e {
        ast::Expr::Literal(_, lit) => Ok(Value::from(lit)),
        ast::Expr::Ident(_, ident) => em.get_binding(ident),
        ast::Expr::List(_, list_exprs) => {
            let r = list_exprs
                .iter()
                .map(|l| exec_expr(em, l))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::List(r))
        }
        ast::Expr::Let(ident, bind_expr, then_expr) => {
            let value = exec_expr(em, bind_expr)?;
            let ret = em.with_added_binding(ident.clone(), value, || exec_expr(em, then_expr));
            ret
        }
        ast::Expr::Then(first_expr, second_expr) => {
            let value1 = exec_expr(em, first_expr)?;
            value1.unit()?;
            let value2 = exec_expr(em, second_expr)?;
            Ok(value2)
        }
        ast::Expr::If {
            span: _,
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
        ast::Expr::Call(span, c) => {
            let resolved = c
                .iter()
                .map(|e| exec_expr(em, e))
                .collect::<Result<Vec<_>, _>>()?;
            if let Some((first, args)) = resolved.split_first() {
                let k = first.into();
                match first {
                    Value::Fun(location, bind_names, fun_stmts) => {
                        em.scope_enter(location);
                        check_arity(bind_names.len(), args.len())?;
                        for (bind_name, arg_value) in bind_names.iter().zip(args.iter()) {
                            em.add_binding(bind_name.clone(), arg_value.clone())
                        }
                        let value = exec_stmts(em, fun_stmts)?;
                        em.scope_leave();
                        Ok(value)
                    }
                    Value::NativeFun(f) => {
                        let call_location = Location::from_span(span);
                        em.scope_enter(&call_location);
                        let res = f(em, args)?;
                        em.scope_leave();
                        Ok(res)
                    }
                    Value::List(_)
                    | Value::Bool(_)
                    | Value::Number(_)
                    | Value::String(_)
                    | Value::Decimal(_)
                    | Value::Bytes(_)
                    | Value::Opaque(_)
                    | Value::Unit => Err(ExecutionError::CallingNotFunc {
                        location: Location::from_span(span),
                        value_is: k,
                    }),
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
