//#![no_std]

extern crate alloc;

mod basic;
mod environ;
mod id;
mod ir;
mod location;
mod symbols;

pub mod lir;

pub use basic::*;
pub use id::{ConstrId, FunId, Id, LitId};
pub use ir::*;
pub use location::*;

use alloc::boxed::Box;
use symbols::{IdVec, IdVecAfter, SymbolsTable, SymbolsTableData, UniqueTableBuilder};

struct RewriteState {
    funs_tbl: SymbolsTable<FunId>,
    funs_vec: IdVec<FunId, lir::FunDef>,
    constrs: SymbolsTableData<ConstrId, lir::ConstrDef>,
    lits: UniqueTableBuilder<LitId, basic::Literal>,
    lambdas: IdVecAfter<FunId, lir::FunDef>,
}

#[derive(Debug)]
pub enum CompilationError {
    DuplicateSymbol(Ident),
}

/// Compile a IR Module into an optimised-for-execution LIR Module
pub fn compile(module: ir::Module) -> Result<lir::Module, CompilationError> {
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

    let mut state = RewriteState {
        funs_tbl: table,
        funs_vec: IdVec::new(),
        lambdas: IdVecAfter::new(vecdata.next_id()),
        constrs: SymbolsTableData::new(),
        lits: UniqueTableBuilder::new(),
    };

    for (funid, fundef) in vecdata.into_iter() {
        let lirdef = rewrite_fun(&mut state, fundef)?;
        let lirid = state.funs_vec.push(lirdef);
        assert_eq!(funid, lirid)
    }

    state.funs_vec.concat(&mut state.lambdas);
    let funs = state.funs_vec;

    Ok(lir::Module {
        lits: state.lits.finalize(),
        constrs: state.constrs,
        funs: funs,
        funs_tbl: state.funs_tbl,
    })
}

fn rewrite_fun(state: &mut RewriteState, fundef: FunDef) -> Result<lir::FunDef, CompilationError> {
    let FunDef { name, vars, body } = fundef;
    let lir_vars = vars.into_iter().map(|v| lir::Variable(v.0)).collect();
    let lir_body = rewrite_expr(state, body)?;
    Ok(lir::FunDef {
        name,
        vars: lir_vars,
        body: lir_body,
    })
}

fn alloc_fun(
    state: &mut SymbolsTableData<FunId, FunDef>,
    fundef: FunDef,
) -> Result<FunId, CompilationError> {
    let ident = fundef.name.clone();
    if let Some(ident) = ident {
        state
            .add(ident.clone(), fundef)
            .ok_or_else(|| CompilationError::DuplicateSymbol(ident))
    } else {
        Ok(state.add_anon(fundef))
    }
}

fn alloc_struct(
    state: &mut SymbolsTableData<ConstrId, lir::ConstrDef>,
    StructDef { name, fields }: StructDef,
) -> Result<ConstrId, CompilationError> {
    let stru = lir::StructDef {
        name: name.unspan(),
        fields: fields.into_iter().map(|v| v.unspan()).collect(),
    };
    let name = stru.name.clone();
    state
        .add(name.clone(), lir::ConstrDef::Struct(stru))
        .ok_or_else(|| CompilationError::DuplicateSymbol(name))
}

fn rewrite_expr(state: &mut RewriteState, expr: Expr) -> Result<lir::Expr, CompilationError> {
    match expr {
        Expr::Literal(span, lit) => {
            let lit_id = state.lits.add(lit);
            Ok(lir::Expr::Literal(span, lit_id))
        }
        Expr::List(span, l) => {
            let l = l
                .into_iter()
                .map(|e| rewrite_expr(state, e))
                .collect::<Result<_, _>>()?;
            Ok(lir::Expr::List(span, l))
        }
        Expr::Let(binder, body, in_expr) => Ok(lir::Expr::Let(
            rewrite_binder(binder),
            rewrite_boxed_expr(state, body)?,
            rewrite_boxed_expr(state, in_expr)?,
        )),
        Expr::Field(expr, ident) => {
            let expr = rewrite_boxed_expr(state, expr)?;
            Ok(lir::Expr::Field(expr, ident))
        }
        Expr::Ident(span, ident) => Ok(lir::Expr::Ident(span, ident)),
        Expr::Lambda(span, fundef) => {
            let lirdef = rewrite_fun(state, *fundef)?;
            let lambda_id = state.lambdas.push(lirdef);
            Ok(lir::Expr::Lambda(span, lambda_id))
        }
        Expr::Call(span, args) => {
            let args = args
                .into_iter()
                .map(|e| rewrite_expr(state, e))
                .collect::<Result<_, _>>()?;
            Ok(lir::Expr::Call(span, args))
        }
        Expr::If {
            span,
            cond,
            then_expr,
            else_expr,
        } => Ok(lir::Expr::If {
            span,
            cond: rewrite_boxed_sexpr(state, cond)?,
            then_expr: rewrite_boxed_sexpr(state, then_expr)?,
            else_expr: rewrite_boxed_sexpr(state, else_expr)?,
        }),
    }
}

fn rewrite_boxed_expr(
    state: &mut RewriteState,
    expr: Box<Expr>,
) -> Result<Box<lir::Expr>, CompilationError> {
    Ok(Box::new(rewrite_expr(state, expr.as_ref().clone())?))
}

fn rewrite_boxed_sexpr(
    state: &mut RewriteState,
    expr: Box<Spanned<Expr>>,
) -> Result<Box<Spanned<lir::Expr>>, CompilationError> {
    let span = expr.span.clone();
    Ok(Box::new(Spanned {
        span,
        inner: rewrite_expr(state, expr.as_ref().clone().unspan())?,
    }))
}

fn rewrite_binder(binder: Binder) -> lir::Binder {
    match binder {
        Binder::Unit => lir::Binder::Unit,
        Binder::Ignore => lir::Binder::Ignore,
        Binder::Ident(i) => lir::Binder::Ident(i),
    }
}
