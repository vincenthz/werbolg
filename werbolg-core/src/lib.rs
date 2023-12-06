#![no_std]

extern crate alloc;

mod basic;
mod ir;
mod location;
mod symbols;

pub mod lir;

pub use basic::*;
pub use ir::*;
pub use location::*;
pub use symbols::SymbolId;

use alloc::boxed::Box;
use symbols::SymbolsTableDataBuilder;

pub struct RewriteState {
    funs: SymbolsTableDataBuilder<lir::FunDef, lir::FunId>,
}

impl RewriteState {
    pub fn add_fun(&mut self, fun: lir::FunDef) -> Result<lir::FunId, CompilationError> {
        let name = fun.name.clone();
        self.funs
            .add(name.clone(), fun)
            .map_err(|()| CompilationError::DuplicateSymbol(name.unwrap().clone()))
    }
}

#[derive(Debug)]
pub enum CompilationError {
    DuplicateSymbol(Ident),
}

/// Compile a IR Module into an optimised-for-execution LIR Module
pub fn compile(module: ir::Module) -> Result<lir::Module, CompilationError> {
    let mut state = RewriteState {
        funs: SymbolsTableDataBuilder::new(),
    };

    for stmt in module.statements {
        match stmt {
            ir::Statement::Function(_span, fundef) => {
                rewrite_fun(&mut state, fundef)?;
            }
            ir::Statement::Expr(_) => {
                todo!()
            }
        }
    }

    Ok(lir::Module {
        funs: state.funs.finalize(),
    })
}

fn rewrite_fun(
    state: &mut RewriteState,
    FunDef { name, vars, body }: FunDef,
) -> Result<lir::FunId, CompilationError> {
    let body = rewrite_expr(state, body)?;
    let fun = lir::FunDef {
        name,
        vars: vars.into_iter().map(|v| lir::Variable(v.0)).collect(),
        body,
    };
    state.add_fun(fun)
}

fn rewrite_expr(state: &mut RewriteState, expr: Expr) -> Result<lir::Expr, CompilationError> {
    match expr {
        Expr::Literal(span, lit) => Ok(lir::Expr::Literal(span, lit)),
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
        Expr::Ident(span, ident) => Ok(lir::Expr::Ident(span, ident)),
        Expr::Lambda(span, fundef) => {
            let symbolid = rewrite_fun(state, fundef.as_ref().clone())?;
            Ok(lir::Expr::Lambda(span, symbolid))
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
