//! lowlevel IR

use super::basic::*;
use super::id::{FunId, LitId, StructId};
use super::location::*;
use super::symbols::{SymbolsTableData, UniqueTable};

use alloc::{boxed::Box, vec::Vec};

pub struct Module {
    pub lits: UniqueTable<LitId, Literal>,
    pub structs: SymbolsTableData<StructId, StructDef>,
    pub funs: SymbolsTableData<FunId, FunDef>,
}

#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Option<Ident>,
    pub vars: Vec<Variable>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Ident,
    pub fields: Vec<Ident>,
}

#[derive(Clone, Debug)]
pub enum Binder {
    Unit,
    Ignore,
    Ident(Ident),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Span, LitId),
    Ident(Span, Ident),
    List(Span, Vec<Expr>),
    Let(Binder, Box<Expr>, Box<Expr>),
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
