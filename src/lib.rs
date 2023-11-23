//! Werbolg is a library to help create dynamic interpreted environment
//!

#![no_std]

extern crate alloc;

#[cfg(any(std, test))]
#[macro_use]
extern crate std;

pub mod ast;
pub mod em;
pub mod lang;

pub use em::{ExecutionError, ExecutionMachine, Value};
pub use lang::common::FileUnit;

pub fn parse(lang: lang::Lang, file: &FileUnit) -> Result<ast::Module, lang::ParseError> {
    lang::parse(lang, file)
}

pub fn exec(em: &ExecutionMachine, ast: ast::Module) -> Result<Value, ExecutionError> {
    em::exec(em, ast)
}
