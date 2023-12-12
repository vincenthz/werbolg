use super::location::Location;
use super::NIFCall;
use super::{ExecutionError, ExecutionMachine, Value};
use alloc::{string::String, vec, vec::Vec};
use werbolg_core as ir;

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

    pub fn get_call(&self, arity: usize) -> (&Value, &[Value]) {
        let top = self.values.len();
        (
            &self.values[top - arity - 1],
            &self.values[top - arity..top],
        )
    }
}

pub fn exec<'module, T>(
    em: &mut ExecutionMachine<'module, T>,
    call: ir::Ident,
    args: &[Value],
) -> Result<Value, ExecutionError> {
    // setup the initial value stack, where we inject a dummy function call and then
    // the arguments to this function
    let mut vstack = ValueStack::new();
    vstack.push_call(em.get_binding(&call)?, args);

    match process_call(
        em,
        &Location {
            module: String::from(""),
            span: ir::Span { start: 0, end: 0 },
        },
        &mut vstack,
        args.len(),
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
    }
}

fn process_call<'m, T>(
    em: &mut ExecutionMachine<'m, T>,
    location: &Location,
    args: &mut ValueStack,
    arity: usize,
) -> Result<Option<Value>, ExecutionError> {
    let number_args = arity;

    let (first, args) = args.get_call(arity);

    todo!()
}

fn check_arity(expected: usize, got: usize) -> Result<(), ExecutionError> {
    if expected == got {
        Ok(())
    } else {
        Err(ExecutionError::ArityError { expected, got })
    }
}
