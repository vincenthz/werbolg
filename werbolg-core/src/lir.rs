//! lowlevel IR

use super::basic::*;
use super::location::*;
use super::symbols::{SymbolId, SymbolsTable};

use alloc::{boxed::Box, vec::Vec};

pub struct Module {
    pub funs: SymbolsTableData<FunDef>,
}

pub struct SymbolsTableData<T> {
    pub symtbl: SymbolsTable,
    pub syms: Vec<T>,
}

impl<T> SymbolsTableData<T> {
    pub fn new() -> Self {
        Self {
            symtbl: SymbolsTable::new(),
            syms: Vec::new(),
        }
    }
    pub fn resolve_id(&self, ident: &Ident) -> Option<SymbolId> {
        self.symtbl.get(ident)
    }

    pub fn get_symbol(&self, ident: &Ident) -> Option<&T> {
        if let Some(id) = self.resolve_id(ident) {
            self.get_symbol_by_id(id)
        } else {
            None
        }
    }

    pub fn get_symbol_by_id(&self, id: SymbolId) -> Option<&T> {
        if id.0 >= self.syms.len() as u32 {
            return None;
        }
        Some(&self.syms[id.0 as usize])
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
    Lambda(Span, SymbolId),
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
