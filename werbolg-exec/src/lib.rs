//! Werbolg Execution machine
#![no_std]
#![deny(missing_docs)]

extern crate alloc;

use ir::{ConstrId, GlobalId, NifId, ValueFun};
use werbolg_compile::{
    CallArity, LocalBindIndex, LocalStackSize, ParamBindIndex, StructFieldIndex,
};
use werbolg_compile::{CompilationUnit, InstructionAddress, InstructionDiff};
use werbolg_core as ir;
use werbolg_core::idvec::IdVec;

mod allocator;
mod exec;
mod valuable;

use alloc::{string::String, vec::Vec};
pub use allocator::WAllocator;
pub use valuable::{Valuable, ValueKind};

pub use exec::{exec, exec_continue, initialize, step, NIFCall, NIF};

/// Execution environment with index Nifs by their NifId, and global variable with their GlobalId
pub struct ExecutionEnviron<'m, 'e, A, L, T, V> {
    /// Indexed NIFs
    pub nifs: IdVec<NifId, NIF<'m, 'e, A, L, T, V>>,
    /// Indexed Globals
    pub globals: IdVec<GlobalId, V>,
}

impl<'m, 'e, A, L, T, V> ExecutionEnviron<'m, 'e, A, L, T, V> {
    /// Pack a streamlined compilation environment into an execution environment
    ///
    /// this is the result of calling `Environment::finalize()`
    pub fn from_compile_environment(
        tuple: (IdVec<GlobalId, V>, IdVec<NifId, NIF<'m, 'e, A, L, T, V>>),
    ) -> Self {
        Self {
            nifs: tuple.1,
            globals: tuple.0,
        }
    }
}

/// User driven Execution params
#[derive(Clone)]
pub struct ExecutionParams<L, V> {
    /// function to map from compilation L literal to a user chosen V value type
    pub literal_to_value: fn(&L) -> V,
}

/// Execution machine
pub struct ExecutionMachine<'m, 'e, A, L, T, V> {
    /// Environ
    pub environ: &'e ExecutionEnviron<'m, 'e, A, L, T, V>,
    /// Module
    pub module: &'m CompilationUnit<L>,
    /// call frame return values
    pub rets: Vec<CallSave>,
    /// stack
    pub stack: ValueStack<V>,
    /// instruction pointer
    pub ip: InstructionAddress,
    /// stack pointer
    pub sp: StackPointer,
    /// arity current function
    pub current_arity: CallArity,
    /// Execution params
    pub params: ExecutionParams<L, V>,
    /// Allocator
    pub allocator: A,
    /// User controlled data
    pub userdata: T,
}

/// Call Save
pub struct CallSave {
    ip: InstructionAddress,
    sp: StackPointer,
    arity: CallArity,
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

impl core::ops::Add<usize> for StackPointer {
    type Output = StackPointer;

    fn add(self, rhs: usize) -> Self::Output {
        StackPointer(self.0 + rhs)
    }
}

impl core::ops::Sub<usize> for StackPointer {
    type Output = StackPointer;

    fn sub(self, rhs: usize) -> Self::Output {
        StackPointer(self.0 - rhs)
    }
}

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

    /// Pop a call from the stack without a function associated
    pub fn pop_call_nofun(&mut self, arity: CallArity) {
        for _ in 0..(arity.0 as usize) {
            self.values.pop();
        }
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
        let rewind = (arity.0 as usize) + 1;
        if top < rewind {
            panic!("trying to get-call {:?}, but only {}", arity, top);
        }
        &self.values[top - (arity.0 as usize) - 1]
    }

    /// Set a specific value on the stack to a given value
    pub fn set_at(&mut self, index: StackPointer, value: V) {
        self.values[index.0] = value;
    }

    /// Get a specific value on the stack
    pub fn get_at(&mut self, index: StackPointer) -> V {
        self.values[index.0].clone()
    }

    /// Get the value on the stack at a given index and push it a duplicate to the top
    pub fn get_and_push(&mut self, index: StackPointer) {
        let value = self.values[index.0].clone();
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

    /// Get the associated arguments with a call
    pub fn get_call_args(&self, arity: CallArity) -> &[V] {
        let top = self.values.len();
        &self.values[top - (arity.0 as usize)..top]
    }

    /// Iterate over all values in the stack, starting from the bottom, towards the end
    pub fn iter_pos(&self) -> impl Iterator<Item = (StackPointer, &V)> {
        self.values
            .iter()
            .enumerate()
            .map(|(pos, v)| (StackPointer(pos), v))
    }
}

impl<'m, 'e, A, L, T, V: Valuable> ExecutionMachine<'m, 'e, A, L, T, V> {
    /// Create a new execution machine
    pub fn new(
        module: &'m CompilationUnit<L>,
        environ: &'e ExecutionEnviron<'m, 'e, A, L, T, V>,
        params: ExecutionParams<L, V>,
        allocator: A,
        userdata: T,
    ) -> Self {
        Self {
            environ,
            module,
            stack: ValueStack::new(),
            rets: Vec::new(),
            userdata,
            allocator,
            ip: InstructionAddress::default(),
            sp: StackPointer::default(),
            params,
            current_arity: CallArity(0),
            //current_stack_size: LocalStackSize(0),
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

    /// Get the current instruction
    pub fn get_current_instruction(&self) -> Option<&'m werbolg_compile::Instruction> {
        self.module.code.get(self.ip)
    }

    /// Set value at stack pointer + local bind to the value in parameter
    #[inline]
    pub fn sp_set_local_value_at(&mut self, bind_index: LocalBindIndex, value: V) {
        let index = self.sp + (bind_index.0 as usize);
        self.stack.set_at(index, value);
    }

    /// Get the global value at GlobalId and push it to the top of the stack
    #[inline]
    pub fn sp_push_value_from_global(&mut self, bind_index: GlobalId) {
        let val = self.environ.globals[bind_index].clone();
        self.stack.push_value(val);
    }

    /// Get the local bound value at bind_index and push it to the top of the stack
    #[inline]
    pub fn sp_push_value_from_local(&mut self, bind_index: LocalBindIndex) {
        let index = self.sp + bind_index.0 as usize;
        self.stack.get_and_push(index);
    }

    /// Get the parameter to the function at param_index and push it to the top of the stack
    #[inline]
    pub fn sp_push_value_from_param(&mut self, param_index: ParamBindIndex) {
        let index = self.sp - self.current_arity.0 as usize + param_index.0 as usize;
        self.stack.get_and_push(index);
    }

    /// Set the stack pointer to the top and push dummy argument for the local stack
    #[inline]
    pub fn sp_set(&mut self, local_stack_size: LocalStackSize) {
        self.sp = self.stack.top();
        for _ in 0..local_stack_size.0 {
            self.stack.push_value(V::make_dummy());
        }
        //self.current_stack_size = local_stack_size;
    }

    fn sp_move_rel(
        &mut self,
        arity: CallArity,
        prev_arity: CallArity,
        local_stack: LocalStackSize,
    ) {
        let nb_values_to_move = arity.0 as usize + 1;
        let stack_top = self.stack.top();
        let top_fun = stack_top - nb_values_to_move;
        let begin = self.sp - (prev_arity.0 as usize) - 1;

        for index in 0..nb_values_to_move {
            let v = self.stack.get_at(top_fun + index);
            self.stack.set_at(begin + index, v);
        }
        self.stack.truncate((begin + nb_values_to_move).0);
        self.sp_set(local_stack)
    }
}

impl<'m, 'e, A, L, T, V: Valuable + core::fmt::Debug> ExecutionMachine<'m, 'e, A, L, T, V> {
    /// print the debug state of the execution machine in a writer
    pub fn debug_state<W: core::fmt::Write>(&self, writer: &mut W) -> Result<(), core::fmt::Error> {
        let instr = self.get_current_instruction();
        writeln!(
            writer,
            "ip={} sp={:?} instruction={:?}",
            self.ip,
            self.sp.0,
            instr.unwrap_or(&werbolg_compile::Instruction::IgnoreOne)
        )?;

        for (stack_index, value) in self.stack.iter_pos() {
            if (stack_index.0 % 4) == 0 {
                writeln!(writer, "")?;
                write!(writer, "{:04x} ", stack_index.0)?;
            }

            if self.sp.0 == stack_index.0 {
                write!(writer, " |@ ")?;
            } else {
                write!(writer, " |  ")?;
            }
            use core::fmt::Write;
            let mut out = String::new();
            write!(&mut out, "{:?}", value)?;
            let out = if out.len() > 16 {
                out.chars().take(16).collect()
            } else if out.len() < 16 {
                alloc::format!("{:>16}", out)
            } else {
                out
            };
            write!(writer, "{}", out)?;
        }
        writeln!(writer, "")?;
        Ok(())
    }
}

/// Execution Error
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// The functions is being called with a different number of parameter it was expecting
    ArityError {
        /// FunId
        funid: ValueFun,
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
    /// Trying to access a NIF id that doesn't exist
    NifOutOfBound {
        /// Constructor Id of the structure
        nifid: NifId,
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
    /// Instruction Pointer is invalid
    IpInvalid {
        /// the instruction pointer triggering this error
        ip: InstructionAddress,
    },
    /// Execution finished
    ExecutionFinished,
    /// NIF return a NotReady signal
    NotReady,
    /// Abort
    Abort,
}
