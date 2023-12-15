//! lowlevel IR

//use crate::code::InstructionDiff;

use super::basic::*;
//use super::code::InstructionAddress;
use super::id::{ConstrId, FunId, GlobalId, LitId, NifId};
use super::location::*;
use super::symbols::{IdVec, SymbolsTable, SymbolsTableData};

use alloc::{boxed::Box, vec::Vec};

pub struct Module {
    pub lits: IdVec<LitId, Literal>,
    pub constrs: SymbolsTableData<ConstrId, ConstrDef>,
    pub funs_tbl: SymbolsTable<FunId>,
    pub funs: IdVec<FunId, FunDef>,
    //pub code: IdVec<InstructionAddress, Statement>,
}

/*
impl Module {
    pub fn print(&self) {
        println!("# literals");
        for (lit_id, lit) in self.lits.iter() {
            println!("* {:?} => {:?}", lit_id, lit);
        }
        println!("# functions map");
        for (k, v) in self.funs_tbl.tbl.iter() {
            println!("* {:?} => {:?}", k, v);
        }
        println!("# functions");
        for (fun_id, fun) in self.funs.iter() {
            println!("* {:?} => {:?} {:?}", fun_id, fun.name, fun.vars);
        }
    }
}
*/

#[derive(Clone, Debug)]
pub struct FunDef {
    pub name: Option<Ident>,
    pub vars: Vec<Variable>,
    //pub stack_size: LocalStackSize,
    pub body: Expr,
    //pub code_pos: InstructionAddress,
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
    Enum(EnumDef),
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
    Field(Box<Expr>, Ident),
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
