use super::basic::*;
use super::location::*;

use alloc::{boxed::Box, vec::Vec};

#[derive(Clone, Debug)]
pub struct Module {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Function(Span, FunDef),
    Struct(Span, StructDef),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Option<Ident>,
    pub vars: Vec<Variable>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Spanned<Ident>,
    pub fields: Vec<Spanned<Ident>>,
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
    Ident(Span, Ident),
    List(Span, Vec<Expr>),
    Let(Binder, Box<Expr>, Box<Expr>),
    Lambda(Span, Box<FunDef>),
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
