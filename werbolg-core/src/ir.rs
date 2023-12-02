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
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Ident,
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
    Lambda(Span, Vec<Variable>, Box<Expr>),
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
