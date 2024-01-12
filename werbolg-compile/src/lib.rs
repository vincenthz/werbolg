//! Compile werbolg-core AST to an easy to execute set of instructions
#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod bindings;
mod code;
mod compile;
mod defs;
mod environ;
mod errors;
mod instructions;
mod params;
mod resolver;
mod symbols;

pub use code::{InstructionAddress, InstructionDiff};
pub use instructions::{
    CallArity, Instruction, LocalBindIndex, ParamBindIndex, StructFieldIndex, TailCall,
};
pub use params::CompilationParams;

use compile::*;
pub use defs::*;
use resolver::SymbolResolver;
use werbolg_core as ir;
use werbolg_core::{AbsPath, ConstrId, FunId, LitId, Namespace};

use bindings::GlobalBindings;
pub use environ::Environment;
pub use errors::CompilationError;
use symbols::{IdVec, IdVecAfter, SymbolsTable, SymbolsTableData};

use alloc::{format, vec::Vec};
use core::fmt::Write;
use hashbrown::HashMap;

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
    funs: SymbolsTableData<FunId, (Namespace, ir::FunDef, ir::FunImpl)>,
    constrs: SymbolsTableData<ConstrId, ConstrDef>,
    namespaces: HashMap<Namespace, SymbolResolver>,
}

impl<L: Clone + Eq + core::hash::Hash> CompilationState<L> {
    /// Create a new compilation state
    pub fn new(params: CompilationParams<L>) -> Self {
        Self {
            params,
            funs: SymbolsTableData::new(),
            constrs: SymbolsTableData::new(),
            namespaces: HashMap::new(),
        }
    }

    /// Add a ir::module to the compilation state
    pub fn add_module(
        &mut self,
        namespace: &Namespace,
        module: ir::Module,
    ) -> Result<(), CompilationError> {
        let mut uses = Vec::new();
        self.funs.create_namespace(namespace.clone())?;
        self.constrs.create_namespace(namespace.clone())?;

        for stmt in module.statements.into_iter() {
            match stmt {
                ir::Statement::Use(u) => {
                    uses.push(u);
                }
                ir::Statement::Function(span, fundef, funimpl) => {
                    let ident = fundef.name.clone();
                    let path = AbsPath::new(namespace, &ident);
                    let _funid = self
                        .funs
                        .add(&path, (namespace.clone(), fundef, funimpl))
                        .ok_or_else(|| CompilationError::DuplicateSymbol(span, ident))?;
                    ()
                }
                ir::Statement::Struct(span, structdef) => {
                    let stru = StructDef {
                        name: structdef.name.unspan(),
                        fields: structdef.fields.into_iter().map(|v| v.unspan()).collect(),
                    };
                    let name = stru.name.clone();
                    let path = AbsPath::new(namespace, &name);
                    self.constrs
                        .add(&path, ConstrDef::Struct(stru))
                        .ok_or_else(|| CompilationError::DuplicateSymbol(span, name))?;
                }
                ir::Statement::Expr(_) => (),
            }
        }

        if self
            .namespaces
            .insert(
                namespace.clone(),
                SymbolResolver::new(namespace.clone(), uses),
            )
            .is_some()
        {
            return Err(CompilationError::NamespaceError(
                symbols::NamespaceError::Duplicate(namespace.clone()),
            ));
        }

        Ok(())
    }

    /// Finalize compilation and return a CompilationUnit containing all the modules compiled in the state
    pub fn finalize<N, G>(
        self,
        environ: &mut Environment<N, G>,
    ) -> Result<CompilationUnit<L>, CompilationError> {
        let SymbolsTableData { table, vecdata } = self.funs;

        /*
        for (p, _id) in table.to_vec(Namespace::root()) {
            std::println!("{:?}", p)
        }
        */

        let mut root_bindings = GlobalBindings::new();
        for (path, id) in environ.symbols.to_vec(Namespace::root()) {
            root_bindings.add(path, BindingType::Nif(id))
        }

        for (path, id) in environ.globals.to_vec(Namespace::root()) {
            root_bindings.add(path, BindingType::Global(id))
        }

        for (path, fun_id) in table.to_vec(Namespace::root()) {
            root_bindings.add(path, BindingType::Fun(fun_id))
        }

        let mut state = compile::CodeBuilder::new(
            &self.params,
            table,
            IdVecAfter::new(vecdata.next_id()),
            root_bindings,
        );

        for (funid, (namespace, fundef, funimpl)) in vecdata.into_iter() {
            let Some(uses) = self.namespaces.get(&namespace) else {
                panic!("internal error: namespace not defined");
            };
            state.set_module_resolver(uses);

            let fun_name = fundef.name.clone();
            let lirdef =
                compile::generate_func_code(&mut state, Some(fundef), funimpl).map_err(|e| {
                    e.context(format!(
                        "namespace {:?} function code {:?}",
                        namespace, fun_name
                    ))
                })?;
            let lirid = state.funs_vec.push(lirdef);
            assert_eq!(funid, lirid)
        }

        // merge the lambdas vec with the main fun vec
        state.funs_vec.concat(&mut state.lambdas_vec);
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
