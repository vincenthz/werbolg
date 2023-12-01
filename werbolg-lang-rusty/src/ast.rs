#![allow(unused)]

use alloc::{boxed::Box, string::String, vec::Vec};

pub struct Grammar {
    item: Vec<Statement>,
}

pub struct Statement {
    annotations: Vec<Annotation>,
    visibility: Visibility,
    kind: StatementKind,
}

pub enum StatementKind {}

pub struct Annotation {
    name: String,
    arg: Option<(String, String)>,
}

pub enum Visibility {
    Public,
    Private,
}

pub struct Function {
    id: String,
    args: Vec<String>,
    body: Body,
}

pub struct Operator {
    prec: u64,
    op: String,
    args: Vec<String>,
    body: Body,
}

pub struct Body {}

pub enum Expr {
    Literal(Literal),
    Ident(String),
    Paren(Box<Expr>),
    Let(String, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

pub enum Literal {
    String(String),
    Number(u64),
    Bool(bool),
}
