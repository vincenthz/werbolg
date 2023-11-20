//! Werbolg is a library to help create dynamic interpreted environment
//!

use std::path::Path;

pub mod ast;
pub mod em;
pub mod lang;

pub use em::{ExecutionError, ExecutionMachine, Value};

pub fn parse(
    lang: lang::Lang,
    file: &Path,
) -> std::io::Result<Result<ast::Module, lang::ParseError>> {
    lang::parse(lang, file)
}

pub fn exec(em: &mut em::ExecutionMachine, ast: ast::Module) -> Result<Value, ExecutionError> {
    em::exec(em, ast)
}
