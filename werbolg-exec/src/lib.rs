//! Werbolg Execution machine
//#![no_std]

extern crate alloc;

use ir::{ConstrId, GlobalId, NifId};
use werbolg_compile::{
    CallArity, LocalBindIndex, LocalStackSize, ParamBindIndex, StructFieldIndex,
};
use werbolg_compile::{CompilationUnit, InstructionAddress, InstructionDiff};
use werbolg_core as ir;
use werbolg_core::idvec::IdVec;

mod exec;
mod valuable;

use alloc::{string::String, vec::Vec};
pub use valuable::{Valuable, ValueKind};

pub use exec::{exec, exec_continue, step, NIFCall, NIF};

pub struct ExecutionEnviron<'m, L, T, V> {
    pub nifs: IdVec<NifId, NIF<'m, L, T, V>>,
    pub globals: IdVec<GlobalId, V>,
}

#[derive(Clone)]
pub struct ExecutionParams<L, V> {
    pub literal_to_value: fn(&L) -> V,
}

pub struct ExecutionMachine<'m, L, T, V> {
    pub nifs: IdVec<NifId, NIF<'m, L, T, V>>,
    pub globals: IdVec<GlobalId, V>,
    pub module: &'m CompilationUnit<L>,
    pub rets: Vec<(InstructionAddress, StackPointer, LocalStackSize, CallArity)>,
    pub stack: ValueStack<V>,
    pub ip: InstructionAddress,
    pub sp: StackPointer,
    pub current_stack_size: LocalStackSize,
    pub params: ExecutionParams<L, V>,
    pub userdata: T,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackPointer(usize);

pub struct ValueStack<V> {
    values: Vec<V>,
}

impl<V: Valuable> ValueStack<V> {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn top(&self) -> StackPointer {
        StackPointer(self.values.len())
    }

    pub fn push_call(&mut self, call: V, args: &[V]) {
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

    pub fn get_call(&self, arity: CallArity) -> &V {
        let top = self.values.len();
        &self.values[top - (arity.0 as usize) - 1]
    }

    pub fn set_at(&mut self, index: usize, value: V) {
        self.values[index] = value;
    }

    pub fn get_and_push(&mut self, index: usize) {
        let value = self.values[index].clone();
        self.push_value(value)
    }

    pub fn push_value(&mut self, arg: V) {
        self.values.push(arg);
    }

    pub fn pop_value(&mut self) -> V {
        self.values.pop().expect("can be popped")
    }

    pub fn get_call_and_args(&self, arity: CallArity) -> (&V, &[V]) {
        let top = self.values.len();
        (
            &self.values[top - (arity.0 as usize) - 1],
            &self.values[top - (arity.0 as usize)..top],
        )
    }

    pub fn iter_pos(&self) -> impl Iterator<Item = (StackPointer, &V)> {
        self.values
            .iter()
            .enumerate()
            .map(|(pos, v)| (StackPointer(pos), v))
    }
}

impl<'m, L, T, V: Valuable> ExecutionMachine<'m, L, T, V> {
    pub fn new(
        module: &'m CompilationUnit<L>,
        env: ExecutionEnviron<'m, L, T, V>,
        params: ExecutionParams<L, V>,
        userdata: T,
    ) -> Self {
        Self {
            nifs: env.nifs,
            globals: env.globals,
            module,
            stack: ValueStack::new(),
            rets: Vec::new(),
            userdata,
            ip: InstructionAddress::default(),
            sp: StackPointer::default(),
            params,
            current_stack_size: LocalStackSize(0),
        }
    }

    pub fn aborted(&self) -> bool {
        false
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
        self.stack.truncate(sp.0 + local_stack_size.0 as usize);
    }

    #[inline]
    pub fn sp_unlocal(&mut self, current_stack_size: LocalStackSize) {
        for _ in 0..current_stack_size.0 {
            self.stack.values.pop();
        }
    }

    #[inline]
    pub fn sp_set_value_at(&mut self, bind_index: LocalBindIndex, value: V) {
        let index = self.sp.0 + bind_index.0 as usize;
        self.stack.set_at(index, value);
    }

    #[inline]
    pub fn sp_push_value_from_global(&mut self, bind_index: GlobalId) {
        let val = self.globals[bind_index].clone();
        self.stack.push_value(val);
    }

    #[inline]
    pub fn sp_push_value_from_local(&mut self, bind_index: LocalBindIndex) {
        let index = self.sp.0 + bind_index.0 as usize;
        self.stack.get_and_push(index);
    }

    #[inline]
    pub fn sp_push_value_from_param(&mut self, bind_index: ParamBindIndex) {
        let index = self.sp.0 - 1 - bind_index.0 as usize;
        self.stack.get_and_push(index);
    }

    #[inline]
    pub fn sp_set(&mut self, local_stack_size: LocalStackSize) {
        self.sp = self.stack.top();
        for _ in 0..local_stack_size.0 {
            self.stack.push_value(V::make_dummy());
        }
        self.current_stack_size = local_stack_size;
        //println!("SP={} local={}", self.sp.0, local_stack_size.0)
    }
}

impl<'m, L, T, V: Valuable + core::fmt::Debug> ExecutionMachine<'m, L, T, V> {
    pub fn debug_state(&self) {
        println!("ip={} sp={:?}", self.ip, self.sp.0);

        for (stack_index, value) in self.stack.iter_pos() {
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
    ArityOverflow {
        got: usize,
    },
    MissingBinding(ir::Ident),
    InternalErrorFunc(ir::FunId),
    StructMismatch {
        constr_expected: ConstrId,
        constr_got: ConstrId,
    },
    StructFieldOutOfBound {
        constr: ConstrId,
        field_index: StructFieldIndex,
        struct_len: usize,
    },
    CallingNotFunc {
        value_is: ValueKind,
    },
    ValueNotStruct {
        value_is: ValueKind,
    },
    ValueNotConditional {
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
