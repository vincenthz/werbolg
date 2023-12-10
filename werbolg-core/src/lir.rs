//! lowlevel IR

use super::basic::*;
use super::id::{ConstrId, FunId, LitId};
use super::location::*;
use super::symbols::{SymbolsTableData, UniqueTable};

use alloc::{boxed::Box, vec::Vec};

pub struct Module {
    pub lits: UniqueTable<LitId, Literal>,
    pub constrs: SymbolsTableData<ConstrId, ConstrDef>,
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
pub struct EnumDef {
    pub name: Ident,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub enum ConstrDef {
    Struct(StructDef),
    Enum(EnumDef), //Enum { pub name: Ident, pub variants: },
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: Ident,
    pub constr: ConstrId,
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
