//! Compile werbolg-core AST to an easy to execute set of instructions
#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod bindings;
mod code;
mod compile;
mod defs;
mod environ;
mod instructions;
mod params;
mod symbols;

pub use code::{InstructionAddress, InstructionDiff};
pub use instructions::{CallArity, Instruction, LocalBindIndex, ParamBindIndex, StructFieldIndex};
pub use params::CompilationParams;

use compile::*;
pub use defs::*;
use werbolg_core as ir;
use werbolg_core::{ConstrId, FunId, Ident, LitId, Literal, Span};

use bindings::BindingsStack;
pub use environ::Environment;
use symbols::{IdVec, IdVecAfter, SymbolsTable, SymbolsTableData};

use alloc::format;
use core::fmt::Write;

/// Compilation error
#[derive(Debug)]
pub enum CompilationError {
    /// Duplicate symbol during compilation (e.g. 2 functions with the name)
    DuplicateSymbol(Ident),
    /// Cannot find the symbol during compilation
    MissingSymbol(Span, Ident),
    /// Cannot find the constructor symbol during compilation
    MissingConstructor(Span, Ident),
    /// Number of parameters for a functions is above the limit we chose
    FunctionParamsMoreThanLimit(usize),
    /// Core's Literal is not supported by this compiler
    LiteralNotSupported(Literal),
    /// The constructor specified is a not a structure, but trying to access inner field
    ConstructorNotStructure(Span, Ident),
    /// The structure specified doesn't have a field of the right name
    StructureFieldNotExistant(Span, Ident, Ident),
}

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

/// State of compilation
pub struct CompilationState<L: Clone + Eq + core::hash::Hash> {
    params: CompilationParams<L>,
    funs: SymbolsTableData<FunId, ir::FunDef>,
    constrs: SymbolsTableData<ConstrId, ConstrDef>,
}

impl<L: Clone + Eq + core::hash::Hash> CompilationState<L> {
    /// Create a new compilation state
    pub fn new(params: CompilationParams<L>) -> Self {
        Self {
            params,
            funs: SymbolsTableData::new(),
            constrs: SymbolsTableData::new(),
        }
    }

    /// Add a ir::module to the compilation state
    pub fn add_module(&mut self, module: ir::Module) -> Result<(), CompilationError> {
        for stmt in module.statements.into_iter() {
            match stmt {
                ir::Statement::Use(_u) => {
                    // todo
                    ()
                }
                ir::Statement::Function(_span, fundef) => {
                    let ident = fundef.name.clone();
                    let _funid = if let Some(ident) = ident {
                        self.funs
                            .add(ident.clone(), fundef)
                            .ok_or_else(|| CompilationError::DuplicateSymbol(ident))?
                    } else {
                        self.funs.add_anon(fundef)
                    };
                    ()
                }
                ir::Statement::Struct(_span, structdef) => {
                    let stru = StructDef {
                        name: structdef.name.unspan(),
                        fields: structdef.fields.into_iter().map(|v| v.unspan()).collect(),
                    };
                    let name = stru.name.clone();
                    self.constrs
                        .add(name.clone(), ConstrDef::Struct(stru))
                        .ok_or_else(|| CompilationError::DuplicateSymbol(name))?;
                }
                ir::Statement::Expr(_) => (),
            }
        }
        Ok(())
    }

    /// Finalize compilation and return a CompilationUnit containing all the modules compiled in the state
    pub fn finalize(
        self,
        environ: &mut Environment,
    ) -> Result<CompilationUnit<L>, CompilationError> {
        let SymbolsTableData { table, vecdata } = self.funs;

        let mut bindings = BindingsStack::new();
        for (_id, (ident, _idx)) in environ.symbols.vecdata.iter() {
            bindings.add(ident.clone(), BindingType::Nif(_id))
        }

        for (ident, fun_id) in table.iter() {
            bindings.add(ident.clone(), BindingType::Fun(fun_id))
        }

        let mut state = compile::RewriteState::new(
            &self.params,
            table,
            IdVecAfter::new(vecdata.next_id()),
            bindings,
        );

        for (funid, fundef) in vecdata.into_iter() {
            let lirdef = compile::generate_func_code(&mut state, fundef)?;
            let lirid = state.funs_vec.push(lirdef);
            assert_eq!(funid, lirid)
        }

        // merge the lambdas code with the main code
        // also remap the fundef of all lambdas to include this new offset
        let lambda_instruction_diff = state.main_code.merge(state.lambdas_code);
        state
            .lambdas
            .remap(|fundef| fundef.code_pos += lambda_instruction_diff);

        state.funs_vec.concat(&mut state.lambdas);
        let funs = state.funs_vec;

        Ok(CompilationUnit {
            lits: state.lits.finalize(),
            constrs: state.constrs,
            funs: funs,
            funs_tbl: state.funs_tbl,
            code: state.main_code.finalize(),
        })
    }
}

/// Compile a IR Module into an optimised-for-execution LIR Module
pub fn compile<'a, L: Clone + Eq + core::hash::Hash>(
    params: &'a CompilationParams<L>,
    module: ir::Module,
    environ: &mut Environment,
) -> Result<CompilationUnit<L>, CompilationError> {
    let mut compiler = CompilationState::new(params.clone());
    compiler.add_module(module)?;
    compiler.finalize(environ)
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
                "[{} local-stack={}]",
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
