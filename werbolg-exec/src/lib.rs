//! Werbolg Execution machine
#![no_std]

extern crate alloc;

use ir::{InstructionAddress, InstructionDiff};
use value::ValueFun;
use werbolg_core as ir;
use werbolg_core::lir;
use werbolg_core::lir::CallArity;

mod bindings;
pub mod exec2;
mod location;
mod stack;
mod value;

use alloc::{string::String, vec, vec::Vec};
use bindings::{Bindings, BindingsStack};
pub use location::Location;
use stack::{ExecutionAtom, ExecutionNext, ExecutionStack};
pub use value::{NIFCall, NifId, Value, ValueKind, NIF};

pub struct ExecutionMachine<'m, T> {
    pub nifs_binds: Bindings<NifId>,
    pub nifs: Vec<NIF<'m, T>>,
    pub module: &'m lir::Module,
    pub local: BindingsStack<BindingValue>,
    pub stacktrace: Vec<Location>,
    pub rets: Vec<(InstructionAddress, CallArity)>,
    pub stack: ExecutionStack<'m>,
    pub stack2: ValueStack,
    pub userdata: T,
    pub ip: ir::InstructionAddress,
    pub sp: usize,
}

pub struct ValueStack {
    values: Vec<Value>,
}

impl ValueStack {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn push_call(&mut self, call: Value, args: &[Value]) {
        self.values.push(call);
        self.values.extend_from_slice(args);
    }

    pub fn pop_call(&mut self, arity: CallArity) {
        for _ in 0..(arity.0 as usize) + 1 {
            self.values.pop();
        }
    }

    pub fn get_call(&self, arity: CallArity) -> &Value {
        let top = self.values.len();
        &self.values[top - (arity.0 as usize) - 1]
    }

    pub fn push_value(&mut self, arg: Value) {
        self.values.push(arg);
    }

    pub fn pop_value(&mut self) -> Value {
        self.values.pop().expect("can be popped")
    }

    pub fn get_call_and_args(&self, arity: CallArity) -> (&Value, &[Value]) {
        let top = self.values.len();
        (
            &self.values[top - (arity.0 as usize) - 1],
            &self.values[top - (arity.0 as usize)..top],
        )
    }
}

pub type BindingValue = Value;

impl<'m, T> ExecutionMachine<'m, T> {
    pub fn new(module: &'m lir::Module, userdata: T) -> Self {
        Self {
            nifs_binds: Bindings::new(),
            nifs: Vec::new(),
            module,
            local: BindingsStack::new(),
            stacktrace: Vec::new(),
            stack: ExecutionStack::new(),
            stack2: ValueStack::new(),
            rets: Vec::new(),
            userdata,
            ip: InstructionAddress::default(),
            sp: 0,
        }
    }

    pub fn aborted(&self) -> bool {
        false
    }

    pub fn add_local_binding(&mut self, ident: ir::Ident, value: Value) {
        self.local.add(ident, value)
    }

    pub fn add_native_call(&mut self, ident: &'static str, f: NIFCall<'m, T>) {
        let id = NifId(self.nifs.len() as u32);
        self.nifs_binds.add(ir::Ident::from(ident), id);
        self.nifs.push(NIF {
            name: ident,
            call: f,
        });
    }

    pub fn add_native_mut_fun(
        &mut self,
        ident: &'static str,
        f: fn(&mut ExecutionMachine<'m, T>, &[Value]) -> Result<Value, ExecutionError>,
    ) {
        self.add_native_call(ident, NIFCall::Mut(f))
    }

    pub fn add_native_pure_fun(
        &mut self,
        ident: &'static str,
        f: fn(&[Value]) -> Result<Value, ExecutionError>,
    ) {
        self.add_native_call(ident, NIFCall::Pure(f))
    }

    pub fn resolve_fun(&self, ident: &ir::Ident) -> Option<&'m lir::FunDef> {
        self.module
            .funs_tbl
            .get(ident)
            .map(|funid| &self.module.funs[funid])
    }

    pub fn get_binding(&self, ident: &ir::Ident) -> Result<Value, ExecutionError> {
        let bind = self
            .local
            .get(ident)
            .map(|e| e.clone())
            .or_else(|| {
                self.module
                    .funs_tbl
                    .get(ident)
                    .map(|symbolic| Value::from(symbolic))
            })
            .or_else(|| self.nifs_binds.get(ident).map(|e| Value::from(*e)));
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

    #[inline]
    pub fn ip_next(&mut self) {
        self.ip = self.ip.next()
    }

    #[inline]
    pub fn ip_set(&mut self, ia: InstructionAddress) {
        self.ip = ia;
    }

    #[inline]
    pub fn ip_jump(&mut self, id: InstructionDiff) {
        self.ip_next();
        self.ip += id;
    }

    #[inline]
    pub fn sp_unwind(&mut self, arity: CallArity) {
        //
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionError {
    ArityError {
        expected: usize,
        got: usize,
    },
    AccessingInexistentField(ir::Ident, ir::Ident),
    AccessingFieldNotAStruct(ir::Ident, ValueKind),
    MissingBinding(ir::Ident),
    InternalErrorFunc(ir::FunId),
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
    ExecutionFinished,
    Abort,
}

pub fn exec<'module, T>(
    em: &mut ExecutionMachine<'module, T>,
    call: ir::Ident,
    args: &[Value],
) -> Result<Value, ExecutionError> {
    let mut values = vec![em.get_binding(&call)?];
    values.extend_from_slice(args);

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

pub fn exec_continue<'m, T>(em: &mut ExecutionMachine<'m, T>) -> Result<Value, ExecutionError> {
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
fn work<'m, T>(em: &mut ExecutionMachine<'m, T>, e: &'m lir::Expr) -> Result<(), ExecutionError> {
    match e {
        lir::Expr::Literal(_, lit) => {
            let literal = &em.module.lits[*lit];
            em.stack.push_value(Value::from(literal))
        }
        lir::Expr::Ident(_, ident) => em.stack.push_value(em.get_binding(ident)?),
        lir::Expr::List(_, l) => {
            if l.is_empty() {
                em.stack.push_value(Value::Unit);
            } else {
                em.stack.push_work(ExecutionAtom::List(l.len()), l)
            }
        }
        lir::Expr::Field(expr, ident) => em.stack.push_work1(ExecutionAtom::Field(ident), expr),
        lir::Expr::Lambda(_span, fundef) => {
            let val = Value::Fun(value::ValueFun::Fun(*fundef));
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

fn eval<'m, T>(
    em: &mut ExecutionMachine<'m, T>,
    ea: ExecutionAtom<'m>,
    args: Vec<Value>,
) -> Result<(), ExecutionError> {
    match ea {
        ExecutionAtom::List(_) => {
            em.stack.push_value(Value::List(args.into()));
            Ok(())
        }
        ExecutionAtom::Field(field) => {
            assert_eq!(args.len(), 1);
            let Value::Struct(constrid, inner_vals) = &args[0] else {
                return Err(ExecutionError::AccessingFieldNotAStruct(
                    field.clone(),
                    (&args[0]).into(),
                ));
            };

            match &em.module.constrs.vecdata[*constrid] {
                lir::ConstrDef::Enum(_) => {
                    return Err(ExecutionError::AccessingFieldNotAStruct(
                        field.clone(),
                        (&args[0]).into(),
                    ));
                }
                lir::ConstrDef::Struct(defs) => {
                    let Some(idx) = defs.fields.iter().position(|r| r == field) else {
                        return Err(ExecutionError::AccessingInexistentField(
                            field.clone(),
                            defs.name.clone(),
                        ));
                    };
                    em.stack.push_value(inner_vals[idx].clone());
                }
            };

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

fn process_call<'m, T>(
    em: &mut ExecutionMachine<'m, T>,
    location: &Location,
    args: Vec<Value>,
) -> Result<Option<Value>, ExecutionError> {
    let number_args = args.len();

    let mut values = args.into_iter();
    let Some(first) = values.next() else {
        return Ok(Some(Value::Unit));
    };

    let first_call = first.fun()?;

    match first_call {
        ValueFun::Fun(funid) => match em.module.funs.get(funid) {
            Some(fundef) => {
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
        },
        ValueFun::Native(nifid) => {
            em.scope_enter(&location);
            let args = values.collect::<Vec<_>>();
            let res = match &em.nifs[nifid.0 as usize].call {
                NIFCall::Pure(nif) => nif(&args)?,
                NIFCall::Mut(nif) => nif(em, &args)?,
            };
            em.scope_leave();
            Ok(Some(res))
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
