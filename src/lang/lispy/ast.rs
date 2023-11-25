use crate::ir::{Ident, Span, Spanned, SpannedBox, Variable};
use alloc::{string::String, vec::Vec};

#[derive(Clone)]
pub enum Ast {
    /// Atom is just some ident like 'foo' or '+'
    Atom(Span, Ident),
    /// Literal value a number '123', string '"foo"', or bytes '#ABCD#'
    Literal(Span, Literal),
    /// List of expression '(a b c)'
    List(Span, ListExpr),
    // (define (id args) expr
    Define(Span, Spanned<Ident>, Vec<Variable>, Vec<Ast>),
    // (if cond then_expr else_expre
    If(Span, SpannedBox<Ast>, SpannedBox<Ast>, SpannedBox<Ast>),
}

impl Ast {
    pub fn literal(&self) -> Option<(&Literal, &Span)> {
        match &self {
            Ast::Literal(span, lit) => Some((lit, span)),
            _ => None,
        }
    }
    pub fn atom(&self) -> Option<(&Ident, &Span)> {
        match &self {
            Ast::Atom(span, atom) => Some((atom, span)),
            _ => None,
        }
    }

    pub fn atom_eq(&self, s: &str) -> bool {
        match &self {
            Ast::Atom(_, ident) => ident.matches(s),
            _ => false,
        }
    }

    #[allow(unused)]
    pub fn list(&self) -> Option<(&ListExpr, &Span)> {
        match &self {
            Ast::List(span, si) => Some((&si, &span)),
            _ => None,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Ast::Atom(span, _) => span.clone(),
            Ast::Literal(span, _) => span.clone(),
            Ast::List(span, _) => span.clone(),
            Ast::Define(span, _, _, _) => span.clone(),
            Ast::If(span, _, _, _) => span.clone(),
        }
    }
}

type ListExpr = Vec<Ast>;

#[derive(Clone, PartialEq, Eq)]
pub enum Literal {
    Bytes(Vec<u8>),
    Number(String),
    String(String),
}
