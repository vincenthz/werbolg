//! AST for werbolg
//!
//! This try to remain generic to allow multiple different language (existing or new)
//! to target werbolg and be interpreted through it

use super::basic::*;
use super::location::*;

use alloc::{boxed::Box, vec::Vec};

/// AST for a module / source code unit
#[derive(Clone, Debug)]
pub struct Module {
    pub statements: Vec<Statement>,
}

/// AST for a high level statement
///
/// Current known types are:
///
/// * Function definition
/// * Struct definition
/// * Naked expression
#[derive(Clone, Debug)]
pub enum Statement {
    Function(Span, FunDef),
    Struct(Span, StructDef),
    Expr(Expr),
}

/// AST for function definition
///
/// Function definitions are something like:
///
/// ```text
/// function $name ( $vars ) { $body }
/// ```
///
#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Option<Ident>,
    pub vars: Vec<Variable>,
    pub body: Expr,
}

/// AST for Structure definition
///
/// Structure definitions are something like
///
/// ```text
/// struct $name { $fields }
/// ```
///
#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Spanned<Ident>,
    pub fields: Vec<Spanned<Ident>>,
}

/// AST for Enum definition
///
/// Enum definitions are something like
///
/// ```text
/// enum $name { $variants }
/// ```
///
#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: Spanned<Ident>,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub struct Variant(StructDef);

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
    Field(Box<Expr>, Ident),
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
