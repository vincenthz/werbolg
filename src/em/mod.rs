//! Werbolg Execution machine

use werbolg_core as ir;

mod bindings;
mod location;
mod stack;
mod value;

use alloc::{string::String, vec, vec::Vec};
use bindings::{Bindings, BindingsStack};
pub use location::Location;
use stack::{ExecutionAtom, ExecutionNext, ExecutionStack};
pub use value::{Value, ValueKind, NIF};

pub struct ExecutionMachine {
    pub root: Bindings<BindingValue>,
    pub module: Bindings<BindingValue>,
    pub local: BindingsStack<BindingValue>,
    pub stacktrace: Vec<Location>,
    pub stack: ExecutionStack,
}

pub type BindingValue = Value;

impl ExecutionMachine {
    pub fn new() -> Self {
        Self {
            root: Bindings::new(),
            module: Bindings::new(),
            local: BindingsStack::new(),
            stacktrace: Vec::new(),
            stack: ExecutionStack::new(),
        }
    }

    pub fn aborted(&self) -> bool {
        false
    }

    pub fn add_module_binding(&mut self, ident: ir::Ident, value: Value) {
        self.module.add(ident, value)
    }

    pub fn add_local_binding(&mut self, ident: ir::Ident, value: Value) {
        self.local.add(ident, value)
    }

    pub fn add_native_fun(&mut self, ident: &'static str, f: NIF) {
        let value = Value::NativeFun(ident, f);
        let ident = ir::Ident::from(ident);
        self.root.add(ident, value)
    }

    pub fn get_binding(&self, ident: &ir::Ident) -> Result<Value, ExecutionError> {
        let bind = self
            .local
            .get(ident)
            .or_else(|| self.module.get(ident))
            .or_else(|| self.root.get(ident));
        match bind {
            None => Err(ExecutionError::MissingBinding(ident.clone())),
            Some(val) => Ok(val.clone()),
        }
    }

    pub fn scope_enter(&mut self, location: &Location) {
        self.local.scope_enter();
        self.stacktrace.push(location.clone())
    }

    pub fn scope_leave(&mut self) {
        self.stacktrace.pop().unwrap();
        self.local.scope_leave();
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionError {
    ArityError {
        expected: usize,
        got: usize,
    },
    MissingBinding(ir::Ident),
    CallingNotFunc {
        location: Location,
        value_is: ValueKind,
    },
    ValueKindUnexpected {
        value_expected: ValueKind,
        value_got: ValueKind,
    },
    OpaqueTypeTypeInvalid {
        got_type_id: core::any::TypeId,
    },
    UserPanic {
        message: String,
    },
    Abort,
}

pub fn exec<'module>(
    em: &mut ExecutionMachine,
    module: &'module ir::Module,
    call: ir::Ident,
    args: Vec<Value>,
) -> Result<Value, ExecutionError> {
    load_stmts(em, &module.statements)?;

    let mut values = vec![em.get_binding(&call)?];
    values.extend_from_slice(&args);

    match process_call(
        em,
        &Location {
            module: String::from(""),
            span: ir::Span { start: 0, end: 0 },
        },
        values,
    )? {
        None => (),
        Some(_) => {
            panic!("NIF cannot be used as entry point")
        }
    };

    exec_continue(em)
}

pub fn exec_continue(em: &mut ExecutionMachine) -> Result<Value, ExecutionError> {
    loop {
        if em.aborted() {
            return Err(ExecutionError::Abort);
        }
        match em.stack.next_work() {
            ExecutionNext::Finish(v) => return Ok(v),
            ExecutionNext::Shift(e) => work(em, &e)?,
            ExecutionNext::Reduce(ea, args) => {
                eval(em, ea, args)?;
            }
        }
    }
}

pub fn load_stmts(
    em: &mut ExecutionMachine,
    stmts: &[ir::Statement],
) -> Result<(), ExecutionError> {
    for statement in stmts {
        match statement {
            ir::Statement::Function(span, ir::FunDef { name, vars, body }) => {
                em.add_module_binding(
                    name.clone(),
                    Value::Fun(Location::from_span(span), vars.clone(), body.clone()),
                );
            }
            ir::Statement::Expr(_) => {}
        }
    }
    Ok(())
}

/// Decompose the work for a given expression
///
/// It either:
/// * Push a value when the work doesn't need further evaluation
/// * Push expressions to evaluate on the work stack and the action to complete
///   when all the evaluation of those expression is commplete
fn work(em: &mut ExecutionMachine, e: &ir::Expr) -> Result<(), ExecutionError> {
    match e {
        ir::Expr::Literal(_, lit) => em.stack.push_value(Value::from(lit)),
        ir::Expr::Ident(_, ident) => em.stack.push_value(em.get_binding(ident)?),
        ir::Expr::List(_, l) => {
            if l.is_empty() {
                em.stack.push_value(Value::Unit);
            } else {
                em.stack.push_work(ExecutionAtom::List(l.len()), l)
            }
        }
        ir::Expr::Lambda(span, args, body) => {
            let val = Value::Fun(
                Location::from_span(span),
                args.clone(),
                body.as_ref().clone(),
            );
            em.stack.push_value(val)
        }
        ir::Expr::Let(ident, e1, e2) => em.stack.push_work1(
            ExecutionAtom::Let(ident.clone().unspan(), e2.as_ref().clone()),
            e1,
        ),
        ir::Expr::Then(e1, e2) => em
            .stack
            .push_work1(ExecutionAtom::Then(e2.as_ref().clone()), e1),
        ir::Expr::Call(span, v) => em
            .stack
            .push_work(ExecutionAtom::Call(v.len(), Location::from_span(span)), v),
        ir::Expr::If {
            span: _,
            cond,
            then_expr,
            else_expr,
        } => em.stack.push_work1(
            ExecutionAtom::ThenElse(then_expr.clone().unspan(), else_expr.clone().unspan()),
            &cond.inner,
        ),
    };
    Ok(())
}

fn eval(
    em: &mut ExecutionMachine,
    ea: ExecutionAtom,
    args: Vec<Value>,
) -> Result<(), ExecutionError> {
    match ea {
        ExecutionAtom::List(_) => {
            em.stack.push_value(Value::List(args));
            Ok(())
        }
        ExecutionAtom::ThenElse(then_expr, else_expr) => {
            let cond_val = args.into_iter().next().unwrap();
            let cond_bool = cond_val.bool()?;

            if cond_bool {
                work(em, &then_expr)?
            } else {
                work(em, &else_expr)?
            }
            Ok(())
        }
        ExecutionAtom::Call(_, loc) => match process_call(em, &loc, args)? {
            None => Ok(()),
            Some(value) => {
                em.stack.push_value(value);
                Ok(())
            }
        },
        ExecutionAtom::Then(e) => {
            let first_val = args.into_iter().next().unwrap();
            first_val.unit()?;
            work(em, &e)?;
            Ok(())
        }
        ExecutionAtom::PopScope => {
            assert_eq!(args.len(), 1);
            em.scope_leave();
            em.stack.push_value(args[0].clone());
            Ok(())
        }
        ExecutionAtom::Let(ident, then) => {
            let bind_val = args.into_iter().next().unwrap();
            em.add_local_binding(ident, bind_val);
            work(em, &then)?;
            Ok(())
        }
    }
}

fn process_call(
    em: &mut ExecutionMachine,
    location: &Location,
    args: Vec<Value>,
) -> Result<Option<Value>, ExecutionError> {
    if let Some((first, args)) = args.split_first() {
        let k = first.into();
        match first {
            Value::Fun(location, bind_names, fun_stmts) => {
                em.scope_enter(location);
                check_arity(bind_names.len(), args.len())?;
                for (bind_name, arg_value) in bind_names.iter().zip(args.iter()) {
                    em.add_local_binding(bind_name.0.clone().unspan(), arg_value.clone())
                }
                em.stack.push_work1(ExecutionAtom::PopScope, fun_stmts);
                Ok(None)
            }
            Value::NativeFun(_name, f) => {
                em.scope_enter(&location);
                let res = f(em, args)?;
                em.scope_leave();
                Ok(Some(res))
            }
            Value::List(_)
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Decimal(_)
            | Value::Bytes(_)
            | Value::Opaque(_)
            | Value::OpaqueMut(_)
            | Value::Unit => Err(ExecutionError::CallingNotFunc {
                location: location.clone(),
                value_is: k,
            }),
        }
    } else {
        Ok(Some(Value::Unit))
    }
}

fn check_arity(expected: usize, got: usize) -> Result<(), ExecutionError> {
    if expected == got {
        Ok(())
    } else {
        Err(ExecutionError::ArityError { expected, got })
    }
}
