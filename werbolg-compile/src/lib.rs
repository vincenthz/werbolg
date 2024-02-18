//! Compile werbolg-core AST to an easy to execute set of instructions
#![no_std]
#![deny(missing_docs)]

extern crate alloc;
extern crate std;

mod bindings;
mod code;
mod compile;
mod defs;
mod environ;
mod errors;
mod hier;
mod instructions;
mod params;
mod prepare;
mod resolver;
mod symbols;

pub use code::{InstructionAddress, InstructionDiff};
pub use instructions::{
    CallArity, Instruction, LocalBindIndex, ParamBindIndex, StructFieldIndex, TailCall,
};
pub use params::CompilationParams;

pub use defs::*;
use werbolg_core as ir;
use werbolg_core::{ConstrId, FunId, LitId, Namespace};

pub use environ::Environment;
pub use errors::CompilationError;
pub use prepare::CompilationState;
use symbols::{IdVec, SymbolsTable, SymbolsTableData};

use alloc::{format, vec::Vec};
use core::fmt::Write;

/// A compiled unit
///
/// The L type parameter is the compilation-level literal type that the user wants
/// to compile to.
pub struct CompilationUnit<L> {
    /// Table of literal indexed by their LitId
    pub lits: IdVec<LitId, L>,
    /// Table of constructor (structure / enum) indexed by their ConstrId
    pub constrs: SymbolsTableData<ConstrId, ConstrDef>,
    /// Symbol table of function { Ident => FunId }
    pub funs_tbl: SymbolsTable<FunId>,
    /// Table of function indexed by their FunId
    pub funs: IdVec<FunId, FunDef>,
    /// A sequence of instructions of all the code, indexed by InstructionAddress
    pub code: IdVec<InstructionAddress, Instruction>,
}

/// Compile a IR Module into an optimised-for-execution `CompilationUnit`
pub fn compile<'a, L: Clone + Eq + core::hash::Hash, N, G>(
    params: &'a CompilationParams<L>,
    modules: Vec<(Namespace, ir::Module)>,
    environ: &mut Environment<N, G>,
) -> Result<CompilationUnit<L>, CompilationError> {
    let mut compiler = CompilationState::new(params.clone());
    for (ns, module) in modules.into_iter() {
        compiler
            .add_module(&ns, module)
            .map_err(|e| e.context(format!("compiling module {:?}", ns)))?;
    }
    compiler
        .finalize(environ)
        .map_err(|e| e.context(format!("Finalizing")))
}

/// Dump the instructions to a buffer
pub fn code_dump<W: Write>(
    writer: &mut W,
    code: &IdVec<InstructionAddress, Instruction>,
    fundefs: &IdVec<FunId, FunDef>,
) -> Result<(), core::fmt::Error> {
    let mut place = hashbrown::HashMap::new();
    for (funid, fundef) in fundefs.iter() {
        place.insert(fundef.code_pos, funid);
    }

    for (ia, stmt) in code.iter() {
        if let Some(funid) = place.get(&ia) {
            let fundef = &fundefs[*funid];
            writeln!(
                writer,
                "[{:?} {} local-stack={}]",
                *funid,
                fundef
                    .name
                    .as_ref()
                    .map(|n| format!("{:?}", n))
                    .unwrap_or(format!("{:?}", funid)),
                fundef.stack_size.0
            )?;
        }
        writeln!(writer, "{}  {:?}", ia, stmt)?
    }
    Ok(())
}
