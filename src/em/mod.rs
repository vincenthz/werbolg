//! Werbolg Execution machine

use crate::ast;

mod bindings;
mod value;

use bindings::BindingsStack;
pub use value::{Value, ValueKind};

pub struct ExecutionMachine {
    pub _bindings: BindingsStack<BindingValue>,
}

impl ExecutionMachine {
    pub fn new() -> Self {
        Self {
            _bindings: BindingsStack::new(),
        }
    }

    pub fn add_binding(&mut self, ident: ast::Ident, value: Value) {
        self._bindings.add(ident, value)
    }

    pub fn get_binding(&mut self, ident: &ast::Ident) -> Result<Value, ExecutionError> {
        let bind = self._bindings.get(ident);
        match bind {
            None => Err(ExecutionError::MissingBinding(ident.clone())),
            Some(val) => Ok(val.clone()),
        }
    }

    pub fn scope_enter(&mut self) {
        self._bindings.scope_enter()
    }

    pub fn scope_leave(&mut self) {
        self._bindings.scope_leave()
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

pub fn exec(em: &mut ExecutionMachine, module: ast::Module) -> Result<Value, ExecutionError> {
    exec_stmts(em, &module.statements)
}

pub fn exec_stmts(
    em: &mut ExecutionMachine,
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

pub fn exec_expr(em: &mut ExecutionMachine, e: &ast::Expr) -> Result<Value, ExecutionError> {
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
                    | Value::Number(_)
                    | Value::String(_)
                    | Value::Decimal(_)
                    | Value::Bytes(_)
                    | Value::Unit => Err(ExecutionError::CallingNotFunc { value_is: k }),
                }
            } else {
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
