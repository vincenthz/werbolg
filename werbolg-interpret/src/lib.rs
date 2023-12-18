//! Werbolg Execution machine
//#![no_std]

extern crate alloc;

use werbolg_core as ir;
use werbolg_core::{idvec::IdVec, FunDef, FunId, Literal, NifId, ValueFun};

mod bindings;
mod location;
mod stack;
mod value;

use alloc::{string::String, vec, vec::Vec};
pub use bindings::{Bindings, BindingsStack};
pub use location::Location;
use stack::{ExecutionAtom, ExecutionNext, ExecutionStack};
pub use value::{NIFCall, Value, ValueKind, NIF};

#[derive(Clone)]
pub struct ExecutionParams {
    pub literal_to_value: fn(&Literal) -> Value,
    pub literal_to_condition: fn(&Literal) -> bool,
}

pub struct ExecutionMachine<'m, T> {
    //pub nifs_binds: Bindings<NifId>,
    pub nifs: IdVec<NifId, NIF<'m, T>>,
    pub funs: IdVec<FunId, &'m FunDef>,
    pub global: Bindings<BindingValue>,
    pub module: &'m ir::Module,
    pub local: BindingsStack<BindingValue>,
    pub stack: ExecutionStack<'m>,
    pub params: ExecutionParams,
    pub userdata: T,
}

pub type BindingValue = Value;

impl<'m, T> ExecutionMachine<'m, T> {
    pub fn new(params: ExecutionParams, module: &'m ir::Module, userdata: T) -> Self {
        let mut b = Bindings::new();
        let mut funs = IdVec::new();
        for stat in module.statements.iter() {
            match stat {
                ir::Statement::Function(_, fundef) => {
                    let fun_id = funs.push(fundef);
                    if let Some(name) = &fundef.name {
                        b.add(name.clone(), Value::Fun(ValueFun::Fun(fun_id)));
                    }
                }
                ir::Statement::Struct(_, _) => {
                    todo!()
                }
                ir::Statement::Expr(_) => (),
            }
        }
        Self {
            params,
            nifs: IdVec::new(),
            funs,
            global: b,
            module,
            local: BindingsStack::new(),
            stack: ExecutionStack::new(),
            userdata,
        }
    }

    pub fn aborted(&self) -> bool {
        false
    }

    pub fn add_local_binding(&mut self, ident: ir::Ident, value: Value) {
        self.local.add(ident, value)
    }

    /*
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

    pub fn resolve_fun(&self, ident: &ir::Ident) -> Option<&'m ir::FunDef> {
        self.module
            .funs_tbl
            .get(ident)
            .map(|funid| &self.module.funs[funid])
    }
    */

    pub fn get_binding(&self, ident: &ir::Ident) -> Result<Value, ExecutionError> {
        let bind = self
            .local
            .get(ident)
            .map(|e| e.clone())
            .or_else(|| self.global.get(ident).map(|x| x.clone()));
        match bind {
            None => Err(ExecutionError::MissingBinding(ident.clone())),
            Some(val) => Ok(val),
        }
    }

    pub fn scope_enter(&mut self) {
        self.local.scope_enter();
    }

    pub fn scope_leave(&mut self) {
        self.local.scope_leave();
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

    match process_call(em, values)? {
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
fn work<'m, T>(em: &mut ExecutionMachine<'m, T>, e: &'m ir::Expr) -> Result<(), ExecutionError> {
    match e {
        ir::Expr::Literal(_, lit) => em.stack.push_value((em.params.literal_to_value)(lit)),
        ir::Expr::Ident(_, ident) => em.stack.push_value(em.get_binding(ident)?),
        ir::Expr::List(_, l) => {
            if l.is_empty() {
                em.stack.push_value(Value::Unit);
            } else {
                em.stack.push_work(ExecutionAtom::List(l.len()), l)
            }
        }
        ir::Expr::Field(expr, ident) => em.stack.push_work1(ExecutionAtom::Field(ident), expr),
        ir::Expr::Lambda(_span, _fundef) => {
            //em.global.get()
            //let val = Value::Fun(ValueFun::Fun(*fundef));
            //em.stack.push_value(val)
            todo!()
        }
        ir::Expr::Let(ident, e1, e2) => em
            .stack
            .push_work1(ExecutionAtom::Let(ident.clone(), e2.as_ref()), e1),
        ir::Expr::Call(span, v) => em
            .stack
            .push_work(ExecutionAtom::Call(v.len(), Location::from_span(span)), v),
        ir::Expr::If {
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
            let Value::Struct(_constrid, _inner_vals) = &args[0] else {
                return Err(ExecutionError::AccessingFieldNotAStruct(
                    field.clone(),
                    (&args[0]).into(),
                ));
            };
            todo!();
        }
        ExecutionAtom::ThenElse(then_expr, else_expr) => {
            let cond_val = args.into_iter().next().unwrap();
            let cond_bool = (em.params.literal_to_condition)(cond_val.literal()?);

            if cond_bool {
                work(em, &then_expr)?
            } else {
                work(em, &else_expr)?
            }
            Ok(())
        }
        ExecutionAtom::Call(_, _loc) => match process_call(em, args)? {
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
                ir::Binder::Unit => bind_val.unit()?,
                ir::Binder::Ignore => {}
                ir::Binder::Ident(ident) => {
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
    args: Vec<Value>,
) -> Result<Option<Value>, ExecutionError> {
    let number_args = args.len();

    let mut values = args.into_iter();
    let Some(first) = values.next() else {
        return Ok(Some(Value::Unit));
    };

    let first_call = first.fun()?;

    match first_call {
        ValueFun::Fun(funid) => match em.funs.get(funid).map(|f| *f) {
            Some(fundef) => {
                em.scope_enter();
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
            em.scope_enter();
            let args = values.collect::<Vec<_>>();
            let res = match &em.nifs[nifid].call {
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
