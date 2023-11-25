mod basic;
mod location;

pub use basic::*;
pub use location::*;

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
pub enum Expr {
    Literal(Span, Literal),
    List(Span, Vec<Expr>),
    Let(Spanned<Ident>, Box<Expr>, Box<Expr>),
    Then(Box<Expr>, Box<Expr>),
    Ident(Span, Ident),
    Lambda(Span, Vec<Variable>, Box<Expr>),
    Call(Span, Vec<Expr>),
    If {
        span: Span,
        cond: SpannedBox<Expr>,
        then_expr: SpannedBox<Expr>,
        else_expr: SpannedBox<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(pub Spanned<Ident>);
