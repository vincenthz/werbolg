//! Werbolg Execution machine
#![no_std]
#![deny(missing_docs)]

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

/// Execution environment with index Nifs by their NifId, and global variable with their GlobalId
pub struct ExecutionEnviron<'m, L, T, V> {
    /// Indexed NIFs
    pub nifs: IdVec<NifId, NIF<'m, L, T, V>>,
    /// Indexed Globals
    pub globals: IdVec<GlobalId, V>,
}

/// User driven Execution params
#[derive(Clone)]
pub struct ExecutionParams<L, V> {
    /// function to map from compilation L literal to a user chosen V value type
    pub literal_to_value: fn(&L) -> V,
}

/// Execution machine
pub struct ExecutionMachine<'m, L, T, V> {
    /// NIFs
    pub nifs: IdVec<NifId, NIF<'m, L, T, V>>,
    /// Global Values
    pub globals: IdVec<GlobalId, V>,
    /// Module
    pub module: &'m CompilationUnit<L>,
    /// call frame return values
    pub rets: Vec<(InstructionAddress, StackPointer, LocalStackSize, CallArity)>,
    /// stack
    pub stack: ValueStack<V>,
    /// instruction pointer
    pub ip: InstructionAddress,
    /// stack pointer
    pub sp: StackPointer,
    /// Current stack size
    pub current_stack_size: LocalStackSize,
    /// Execution params
    pub params: ExecutionParams<L, V>,
    /// User controlled data
    pub userdata: T,
}

/// Execution Stack pointer
///
/// It is the index in the Stack where:
/// * under you have the function parameters, and the function Value
/// * over it:
///   * the local stack for bounded value for this function
///   * then finally stack based (push/pop) values
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackPointer(usize);

/// Value stack
pub struct ValueStack<V> {
    values: Vec<V>,
}

impl<V: Valuable> ValueStack<V> {
    /// Create a new Value Stack
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// get the stack pointer for the top of the stack
    pub fn top(&self) -> StackPointer {
        StackPointer(self.values.len())
    }

    /// Push a call on the stack
    pub fn push_call(&mut self, call: V, args: &[V]) {
        self.values.push(call);
        self.values.extend_from_slice(args);
    }

    /// Pop a call from the stack
    pub fn pop_call(&mut self, arity: CallArity) {
        for _ in 0..(arity.0 as usize) + 1 {
            self.values.pop();
        }
    }

    /// Truncate the stack to n elements
    pub fn truncate(&mut self, n: usize) {
        self.values.truncate(n)
    }

    /// Get the call value (which should be a ValueFun) from the stack
    pub fn get_call(&self, arity: CallArity) -> &V {
        let top = self.values.len();
        &self.values[top - (arity.0 as usize) - 1]
    }

    /// Set a specific value on the stack to a given value
    pub fn set_at(&mut self, index: usize, value: V) {
        self.values[index] = value;
    }

    /// Get the value on the stack at a given index and push it a duplicate to the top
    pub fn get_and_push(&mut self, index: usize) {
        let value = self.values[index].clone();
        self.push_value(value)
    }

    /// Push a value on the top of the stack
    pub fn push_value(&mut self, arg: V) {
        self.values.push(arg);
    }

    /// Pop a value from the top of the stack
    pub fn pop_value(&mut self) -> V {
        self.values.pop().expect("can be popped")
    }

    /// Get the call value and associated arguments
    pub fn get_call_and_args(&self, arity: CallArity) -> (&V, &[V]) {
        let top = self.values.len();
        (
            &self.values[top - (arity.0 as usize) - 1],
            &self.values[top - (arity.0 as usize)..top],
        )
    }

    /// Iterate over all values in the stack, starting from the bottom, towards the end
    pub fn iter_pos(&self) -> impl Iterator<Item = (StackPointer, &V)> {
        self.values
            .iter()
            .enumerate()
            .map(|(pos, v)| (StackPointer(pos), v))
    }
}

impl<'m, L, T, V: Valuable> ExecutionMachine<'m, L, T, V> {
    /// Create a new execution machine
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

    /// increment the instruction pointer
    #[inline]
    pub fn ip_next(&mut self) {
        self.ip = self.ip.next()
    }

    /// Set the instruction pointer
    #[inline]
    pub fn ip_set(&mut self, ia: InstructionAddress) {
        self.ip = ia;
    }

    /// Jump the instruction to differential number of instruction, plus 1
    #[inline]
    pub fn ip_jump(&mut self, id: InstructionDiff) {
        self.ip_next();
        self.ip += id;
    }

    /// unlocalise the stack
    #[inline]
    fn sp_unlocal(&mut self, current_stack_size: LocalStackSize) {
        for _ in 0..current_stack_size.0 {
            self.stack.values.pop();
        }
    }

    /// Set value at stack pointer + local bind to the value in parameter
    #[inline]
    pub fn sp_set_value_at(&mut self, bind_index: LocalBindIndex, value: V) {
        let index = self.sp.0 + bind_index.0 as usize;
        self.stack.set_at(index, value);
    }

    /// Get the global value at GlobalId and push it to the top of the stack
    #[inline]
    pub fn sp_push_value_from_global(&mut self, bind_index: GlobalId) {
        let val = self.globals[bind_index].clone();
        self.stack.push_value(val);
    }

    /// Get the local bound value at bind_index and push it to the top of the stack
    #[inline]
    pub fn sp_push_value_from_local(&mut self, bind_index: LocalBindIndex) {
        let index = self.sp.0 + bind_index.0 as usize;
        self.stack.get_and_push(index);
    }

    /// Get the parameter to the function at param_index and push it to the top of the stack
    #[inline]
    pub fn sp_push_value_from_param(&mut self, param_index: ParamBindIndex) {
        let index = self.sp.0 - 1 - param_index.0 as usize;
        self.stack.get_and_push(index);
    }

    /// Set the stack pointer to the top and push dummy argument for the local stack
    #[inline]
    pub fn sp_set(&mut self, local_stack_size: LocalStackSize) {
        self.sp = self.stack.top();
        for _ in 0..local_stack_size.0 {
            self.stack.push_value(V::make_dummy());
        }
        self.current_stack_size = local_stack_size;
    }
}

impl<'m, L, T, V: Valuable + core::fmt::Debug> ExecutionMachine<'m, L, T, V> {
    /// print the debug state of the execution machine in a writer
    pub fn debug_state<W: core::fmt::Write>(&self, writer: &mut W) -> Result<(), core::fmt::Error> {
        writeln!(writer, "ip={} sp={:?}", self.ip, self.sp.0)?;

        for (stack_index, value) in self.stack.iter_pos() {
            match stack_index.cmp(&self.sp) {
                core::cmp::Ordering::Less => {
                    let diff = self.sp.0 - stack_index.0;
                    writeln!(writer, "[-{}] {:?}", diff, value)?;
                }
                core::cmp::Ordering::Greater => {
                    let diff = stack_index.0 - self.sp.0;
                    writeln!(writer, "[{}] {:?}", 1 + diff, value)?;
                }
                core::cmp::Ordering::Equal => {
                    writeln!(writer, "@ {:?}", value)?;
                }
            }
        }
        Ok(())
    }
}

/// Execution Error
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// The functions is being called with a different number of parameter it was expecting
    ArityError {
        /// The expected number by the function
        expected: CallArity,
        /// The number of actual parameters received by the function
        got: CallArity,
    },
    /// The initial call parameter is trying to use more parameters than allowed by the system
    ArityOverflow {
        /// the number of parameters that triggered the error
        got: usize,
    },
    //MissingBinding(ir::Ident),
    //InternalErrorFunc(ir::FunId),
    /// Structure expected is not of the right type
    StructMismatch {
        /// The constructor expected
        constr_expected: ConstrId,
        /// The constructor received
        constr_got: ConstrId,
    },
    /// Trying to access structure fields that is beyond the number of fields in this structure
    StructFieldOutOfBound {
        /// Constructor Id of the structure
        constr: ConstrId,
        /// the field index
        field_index: StructFieldIndex,
        /// the actual structure length
        struct_len: usize,
    },
    /// Value is not a function
    CallingNotFunc {
        /// the descriptor for the value that was not a fun
        value_is: ValueKind,
    },
    /// Value is not a struct
    ValueNotStruct {
        /// The descriptor for the value that was not a struct
        value_is: ValueKind,
    },
    /// Value is not a valid conditional
    ValueNotConditional {
        /// The descriptor for the value that was not a conditional
        value_is: ValueKind,
    },
    /// Value is not a the valid kind
    ValueKindUnexpected {
        /// The descriptor for the value that was expected
        value_expected: ValueKind,
        /// The descriptor for the value that was used
        value_got: ValueKind,
    },
    /// NIF return an error
    UserPanic {
        /// user message
        message: String,
    },
    /// Execution finished
    ExecutionFinished,
    /// NIF return a NotReady signal
    NotReady,
    /// Abort
    Abort,
}
