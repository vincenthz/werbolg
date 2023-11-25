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

pub fn parse(lang: lang::Lang, file: &FileUnit) -> Result<ir::Module, lang::ParseError> {
    lang::parse(lang, file)
}

pub fn exec(em: &mut ExecutionMachine, ast: ir::Module) -> Result<Value, ExecutionError> {
    em::exec(em, ast)
}
