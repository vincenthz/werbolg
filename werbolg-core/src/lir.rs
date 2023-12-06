//! lowlevel IR

use super::basic::*;
use super::location::*;
use super::symbols::{SymbolId, SymbolIdRemapper, SymbolsTableData};

use alloc::{boxed::Box, vec::Vec};

pub struct Module {
    pub funs: SymbolsTableData<FunDef, FunId>,
}

#[derive(Debug, Copy, Clone)]
pub struct FunId(SymbolId);

impl SymbolIdRemapper for FunId {
    fn uncat(self) -> SymbolId {
        self.0
    }

    fn cat(id: SymbolId) -> Self {
        FunId(id)
    }
}

#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Option<Ident>,
    pub vars: Vec<Variable>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub enum Binder {
    Unit,
    Ignore,
    Ident(Ident),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Span, Literal),
    List(Span, Vec<Expr>),
    Let(Binder, Box<Expr>, Box<Expr>),
    Ident(Span, Ident),
    Lambda(Span, FunId),
    Call(Span, Vec<Expr>),
    If {
        span: Span,
        cond: Box<Spanned<Expr>>,
        then_expr: Box<Spanned<Expr>>,
        else_expr: Box<Spanned<Expr>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(pub Spanned<Ident>);
