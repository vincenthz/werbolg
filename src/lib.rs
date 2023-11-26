//! Werbolg is a library to help create dynamic interpreted environment
//!

#![no_std]

extern crate alloc;

//#[cfg(any(std, test))]
//#[macro_use]
extern crate std;

pub mod em;
pub mod ir;
pub mod lang;

pub use em::{ExecutionError, ExecutionMachine, Value};
pub use lang::common::FileUnit;

use alloc::vec::Vec;

pub fn parse(lang: lang::Lang, file: &FileUnit) -> Result<ir::Module, lang::ParseError> {
    lang::parse(lang, file)
}

pub fn exec<'module>(
    em: &mut ExecutionMachine,
    ast: &'module ir::Module,
    call: ir::Ident,
    args: Vec<em::Value>,
) -> Result<Value, ExecutionError> {
    em::exec(em, ast, call, args)
}
