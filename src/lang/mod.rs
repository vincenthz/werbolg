pub mod common;
pub mod rusty;
pub mod scheme;

use common::{ast, FileUnit};
use std::path::Path;

pub use common::ParseError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Scheme,
    Rusty,
}

pub fn parse_unit(lang: Lang, unit: &FileUnit) -> Result<ast::Module, ParseError> {
    match lang {
        Lang::Scheme => scheme::module(unit),
        Lang::Rusty => rusty::module(unit),
    }
}

pub fn parse(lang: Lang, file: &Path) -> std::io::Result<Result<ast::Module, ParseError>> {
    let fileunit = FileUnit::from_file(file)?;
    Ok(parse_unit(lang, &fileunit))
}
