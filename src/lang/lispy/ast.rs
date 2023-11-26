use crate::ir::{Ident, Spanned, Variable};
use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Clone)]
pub enum Ast {
    /// Atom is just some ident like 'foo' or '+'
    Atom(Ident),
    /// Literal value a number '123', string '"foo"', or bytes '#ABCD#'
    Literal(Literal),
    /// List of expression '(a b c)'
    List(ListExpr),
    // (define (id args) expr+)
    Define(Spanned<Ident>, Vec<Variable>, Vec<Spanned<Ast>>),
    // (if cond then_expr else_expr)
    If(Box<Spanned<Ast>>, Box<Spanned<Ast>>, Box<Spanned<Ast>>),
}

impl Ast {
    pub fn literal(&self) -> Option<&Literal> {
        match &self {
            Ast::Literal(lit) => Some(lit),
            _ => None,
        }
    }
    pub fn atom(&self) -> Option<&Ident> {
        match &self {
            Ast::Atom(atom) => Some(atom),
            _ => None,
        }
    }

    pub fn atom_eq(&self, s: &str) -> bool {
        match &self {
            Ast::Atom(ident) => ident.matches(s),
            _ => false,
        }
    }

    #[allow(unused)]
    pub fn list(&self) -> Option<&ListExpr> {
        match &self {
            Ast::List(si) => Some(&si),
            _ => None,
        }
    }
}

type ListExpr = Vec<Spanned<Ast>>;

#[derive(Clone, PartialEq, Eq)]
pub enum Literal {
    Bytes(Vec<u8>),
    Number(String),
    String(String),
}
