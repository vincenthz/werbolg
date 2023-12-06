//! Werbolg Execution machine
#![no_std]

extern crate alloc;

use ir::lir::Symbolic;
use werbolg_core as ir;
use werbolg_core::lir;

mod bindings;
mod location;
mod stack;
mod value;

use alloc::{string::String, vec, vec::Vec};
use bindings::{Bindings, BindingsStack};
pub use location::Location;
use stack::{ExecutionAtom, ExecutionNext, ExecutionStack};
pub use value::{Value, ValueKind, NIF};

pub struct ExecutionMachine<'m> {
    pub root: Bindings<BindingValue>,
    pub module: &'m lir::Module,
    pub local: BindingsStack<BindingValue>,
    pub stacktrace: Vec<Location>,
    pub stack: ExecutionStack<'m>,
}

pub type BindingValue = Value;

impl<'m> ExecutionMachine<'m> {
    pub fn new(module: &'m lir::Module) -> Self {
        Self {
            root: Bindings::new(),
            module,
            local: BindingsStack::new(),
            stacktrace: Vec::new(),
            stack: ExecutionStack::new(),
        }
    }

    pub fn aborted(&self) -> bool {
        false
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
            .map(|e| e.clone())
            .or_else(|| {
                self.module
                    .resolve_id(ident)
                    .map(|symbolic| Value::Fun(symbolic))
            })
            .or_else(|| self.root.get(ident).map(|e| e.clone()));
        match bind {
            None => Err(ExecutionError::MissingBinding(ident.clone())),
            Some(val) => Ok(val),
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
    em: &mut ExecutionMachine<'module>,
    call: ir::Ident,
    args: Vec<Value>,
) -> Result<Value, ExecutionError> {
    //load_stmts(em, &module.statements)?;

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

pub fn exec_continue<'m>(em: &mut ExecutionMachine<'m>) -> Result<Value, ExecutionError> {
    loop {
        if em.aborted() {
            return Err(ExecutionError::Abort);
        }
        match em.stack.next_work() {
            ExecutionNext::Finish(v) => return Ok(v),
            ExecutionNext::Shift(e) => work(em, e)?,
            ExecutionNext::Reduce(ea, args) => {
                eval(em, ea, args)?;
            }
        }
    }
}

/// Decompose the work for a given expression
///
/// It either:
/// * Push a value when the work doesn't need further evaluation
/// * Push expressions to evaluate on the work stack and the action to complete
///   when all the evaluation of those expression is commplete
fn work<'m>(em: &mut ExecutionMachine<'m>, e: &'m lir::Expr) -> Result<(), ExecutionError> {
    match e {
        lir::Expr::Literal(_, lit) => em.stack.push_value(Value::from(lit)),
        lir::Expr::Ident(_, ident) => em.stack.push_value(em.get_binding(ident)?),
        lir::Expr::List(_, l) => {
            if l.is_empty() {
                em.stack.push_value(Value::Unit);
            } else {
                em.stack.push_work(ExecutionAtom::List(l.len()), l)
            }
        }
        lir::Expr::Lambda(_span, fundef) => {
            let val = Value::Fun(*fundef);
            em.stack.push_value(val)
        }
        lir::Expr::Let(ident, e1, e2) => em
            .stack
            .push_work1(ExecutionAtom::Let(ident.clone(), e2.as_ref()), e1),
        lir::Expr::Call(span, v) => em
            .stack
            .push_work(ExecutionAtom::Call(v.len(), Location::from_span(span)), v),
        lir::Expr::If {
            span: _,
            cond,
            then_expr,
            else_expr,
        } => em
            .stack
            .push_work1(ExecutionAtom::ThenElse(then_expr, else_expr), &cond.inner),
    };
    Ok(())
}

fn eval<'m>(
    em: &mut ExecutionMachine<'m>,
    ea: ExecutionAtom<'m>,
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
        ExecutionAtom::PopScope => {
            assert_eq!(args.len(), 1);
            em.scope_leave();
            em.stack.push_value(args[0].clone());
            Ok(())
        }
        ExecutionAtom::Let(ident, then) => {
            let bind_val = args.into_iter().next().unwrap();
            match ident {
                lir::Binder::Unit => bind_val.unit()?,
                lir::Binder::Ignore => {}
                lir::Binder::Ident(ident) => {
                    em.add_local_binding(ident, bind_val);
                }
            }
            work(em, then)?;
            Ok(())
        }
    }
}

fn process_call<'m>(
    em: &mut ExecutionMachine<'m>,
    location: &Location,
    args: Vec<Value>,
) -> Result<Option<Value>, ExecutionError> {
    let number_args = args.len();

    let mut values = args.into_iter();
    let Some(first) = values.next() else {
        return Ok(Some(Value::Unit));
    };
    let first_k = (&first).into();

    match first {
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
            value_is: first_k,
        }),
        Value::Fun(symbol) => {
            // location, bind_names, fun_stmts) => {
            match em.module.get_symbol_by_id(symbol) {
                Some(Symbolic::Fun(fundef)) => {
                    em.scope_enter(&location);
                    check_arity(fundef.vars.len(), number_args - 1)?;
                    for (bind_name, arg_value) in fundef.vars.iter().zip(values) {
                        em.add_local_binding(bind_name.0.clone().unspan(), arg_value.clone())
                    }
                    em.stack.push_work1(ExecutionAtom::PopScope, &fundef.body);
                    Ok(None)
                }
                None => {
                    panic!("internal error: fun of symbol that doens't exist")
                }
            }
        }
        Value::NativeFun(_name, f) => {
            em.scope_enter(&location);
            let args = values.collect::<Vec<_>>();
            let res = f(em, &args)?;
            em.scope_leave();
            Ok(Some(res))
        }
    }

    /*
    if let Some((first, args)) = args.split_first() {
        let k = first.into();
    } else {
        Ok(Some(Value::Unit))
    }
    */
}

fn check_arity(expected: usize, got: usize) -> Result<(), ExecutionError> {
    if expected == got {
        Ok(())
    } else {
        Err(ExecutionError::ArityError { expected, got })
    }
}
