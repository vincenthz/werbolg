extern crate alloc;

mod bindings;
mod code;
mod compile;
mod defs;
mod environ;
mod instructions;
mod params;
pub mod symbols;

pub use code::{InstructionAddress, InstructionDiff};
pub use instructions::{CallArity, Instruction, LocalBindIndex, ParamBindIndex, StructFieldIndex};
pub use params::CompilationParams;

use compile::*;
pub use defs::*;
use werbolg_core as ir;
use werbolg_core::{ConstrId, FunId, Ident, LitId, Span};

use bindings::BindingsStack;
pub use environ::Environment;
use symbols::{IdVec, IdVecAfter, SymbolsTable, SymbolsTableData};

#[derive(Debug)]
pub enum CompilationError {
    DuplicateSymbol(Ident),
    MissingSymbol(Span, Ident),
    FunctionParamsMoreThanLimit(usize),
}

pub struct CompilationUnit<L> {
    pub lits: IdVec<LitId, L>,
    pub constrs: SymbolsTableData<ConstrId, ConstrDef>,
    pub funs_tbl: SymbolsTable<FunId>,
    pub funs: IdVec<FunId, FunDef>,
    pub code: IdVec<InstructionAddress, Instruction>,
}

/// Compile a IR Module into an optimised-for-execution LIR Module
pub fn compile<'a, L: Clone + Eq + core::hash::Hash>(
    params: &'a CompilationParams<L>,
    module: ir::Module,
    environ: &mut Environment,
) -> Result<CompilationUnit<L>, CompilationError> {
    let mut funs = SymbolsTableData::new();
    let mut constrs = SymbolsTableData::new();

    for stmt in module.statements.into_iter() {
        match stmt {
            ir::Statement::Function(_span, fundef) => {
                alloc_fun(&mut funs, fundef)?;
            }
            ir::Statement::Struct(_span, structdef) => {
                alloc_struct(&mut constrs, structdef)?;
            }
            ir::Statement::Expr(_) => (),
        }
    }

    let SymbolsTableData { table, vecdata } = funs;

    let mut bindings = BindingsStack::new();
    for (_id, (ident, _idx)) in environ.symbols.vecdata.iter() {
        bindings.add(ident.clone(), BindingType::Nif(_id))
    }

    for (ident, fun_id) in table.iter() {
        bindings.add(ident.clone(), BindingType::Fun(fun_id))
    }

    let mut state =
        compile::RewriteState::new(params, table, IdVecAfter::new(vecdata.next_id()), bindings);

    for (funid, fundef) in vecdata.into_iter() {
        let lirdef = compile::rewrite_fun(&mut state, fundef)?;
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

pub fn code_dump(code: &IdVec<InstructionAddress, Instruction>, fundefs: &IdVec<FunId, FunDef>) {
    let mut place = hashbrown::HashMap::new();
    for (funid, fundef) in fundefs.iter() {
        place.insert(fundef.code_pos, funid);
    }

    for (ia, stmt) in code.iter() {
        if let Some(funid) = place.get(&ia) {
            let fundef = &fundefs[*funid];
            println!(
                "[{} local-stack={}]",
                fundef
                    .name
                    .as_ref()
                    .map(|n| format!("{:?}", n))
                    .unwrap_or(format!("{:?}", funid)),
                fundef.stack_size.0
            );
        }
        println!("{}  {:?}", ia, stmt)
    }
}
