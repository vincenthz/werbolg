//! Werbolg Execution machine
//#![no_std]

extern crate alloc;

use ir::{GlobalId, InstructionAddress, InstructionDiff, NifId};
use werbolg_core as ir;
use werbolg_core::lir;
use werbolg_core::lir::{CallArity, LocalBindIndex, LocalStackSize, ParamBindIndex};
use werbolg_core::symbols::IdVec;

mod exec2;
mod value;

use alloc::{string::String, vec::Vec};
pub use value::{NIFCall, Value, ValueKind, NIF};

pub use exec2::{exec, exec_continue, step};

pub struct ExecutionEnviron<'m, T> {
    pub nifs: IdVec<NifId, NIF<'m, T>>,
    pub globals: IdVec<GlobalId, Value>,
}

pub struct ExecutionMachine<'m, T> {
    pub nifs: IdVec<NifId, NIF<'m, T>>,
    pub globals: IdVec<GlobalId, Value>,
    pub module: &'m lir::Module,
    pub rets: Vec<(InstructionAddress, StackPointer, LocalStackSize, CallArity)>,
    pub stack2: ValueStack,
    pub userdata: T,
    pub ip: ir::InstructionAddress,
    pub sp: StackPointer,
    pub current_stack_size: LocalStackSize,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackPointer(usize);

pub struct ValueStack {
    values: Vec<Value>,
}

impl ValueStack {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn top(&self) -> StackPointer {
        StackPointer(self.values.len())
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

    pub fn truncate(&mut self, n: usize) {
        self.values.truncate(n)
    }

    pub fn get_call(&self, arity: CallArity) -> &Value {
        let top = self.values.len();
        &self.values[top - (arity.0 as usize) - 1]
    }

    pub fn set_at(&mut self, index: usize, value: Value) {
        self.values[index] = value;
    }

    pub fn get_and_push(&mut self, index: usize) {
        let value = self.values[index].clone();
        self.push_value(value)
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

    pub fn iter_pos(&self) -> impl Iterator<Item = (StackPointer, &Value)> {
        self.values
            .iter()
            .enumerate()
            .map(|(pos, v)| (StackPointer(pos), v))
    }
}

pub type BindingValue = Value;

impl<'m, T> ExecutionMachine<'m, T> {
    pub fn new(module: &'m lir::Module, env: ExecutionEnviron<'m, T>, userdata: T) -> Self {
        Self {
            nifs: env.nifs,
            globals: env.globals,
            module,
            stack2: ValueStack::new(),
            rets: Vec::new(),
            userdata,
            ip: InstructionAddress::default(),
            sp: StackPointer::default(),
            current_stack_size: LocalStackSize(0),
        }
    }

    pub fn aborted(&self) -> bool {
        false
    }

    /*
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
    */

    pub fn resolve_fun(&self, ident: &ir::Ident) -> Option<&'m lir::FunDef> {
        self.module
            .funs_tbl
            .get(ident)
            .map(|funid| &self.module.funs[funid])
    }

    pub fn get_binding(&self, ident: &ir::Ident) -> Result<Value, ExecutionError> {
        todo!()
        /*
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
        */
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
    pub fn sp_unwind(&mut self, sp: StackPointer, local_stack_size: LocalStackSize) {
        self.stack2.truncate(sp.0 + local_stack_size.0 as usize);
    }

    #[inline]
    pub fn sp_unlocal(&mut self, current_stack_size: LocalStackSize) {
        for _ in 0..current_stack_size.0 {
            self.stack2.values.pop();
        }
    }

    #[inline]
    pub fn sp_set_value_at(&mut self, bind_index: LocalBindIndex, value: Value) {
        let index = self.sp.0 + bind_index.0 as usize;
        self.stack2.set_at(index, value);
    }

    #[inline]
    pub fn sp_push_value_from_global(&mut self, bind_index: GlobalId) {
        let val = self.globals[bind_index].clone();
        self.stack2.push_value(val);
    }

    #[inline]
    pub fn sp_push_value_from_local(&mut self, bind_index: LocalBindIndex) {
        let index = self.sp.0 + bind_index.0 as usize;
        self.stack2.get_and_push(index);
    }

    #[inline]
    pub fn sp_push_value_from_param(&mut self, bind_index: ParamBindIndex) {
        let index = self.sp.0 - 1 - bind_index.0 as usize;
        self.stack2.get_and_push(index);
    }

    #[inline]
    pub fn sp_set(&mut self, local_stack_size: LocalStackSize) {
        self.sp = self.stack2.top();
        for _ in 0..local_stack_size.0 {
            self.stack2.push_value(Value::Unit);
        }
        self.current_stack_size = local_stack_size;
        //println!("SP={} local={}", self.sp.0, local_stack_size.0)
    }

    pub fn debug_state(&self) {
        println!("ip={} sp={:?}", self.ip, self.sp.0);

        for (stack_index, value) in self.stack2.iter_pos() {
            match stack_index.cmp(&self.sp) {
                core::cmp::Ordering::Less => {
                    let diff = self.sp.0 - stack_index.0;
                    println!("[-{}] {:?}", diff, value);
                }
                core::cmp::Ordering::Greater => {
                    let diff = stack_index.0 - self.sp.0;
                    println!("[{}] {:?}", 1 + diff, value);
                }
                core::cmp::Ordering::Equal => {
                    println!("@ {:?}", value);
                }
            }
        }
        println!("")
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
