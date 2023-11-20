pub mod common;
pub mod scheme;

use common::{ast, FileUnit};
use std::path::Path;

pub use common::ParseError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Scheme,
}

pub fn parse_unit(lang: Lang, unit: &FileUnit) -> Result<ast::Module, ParseError> {
    match lang {
        Lang::Scheme => scheme::module(unit),
    }
}

pub fn parse(lang: Lang, file: &Path) -> std::io::Result<Result<ast::Module, ParseError>> {
    let fileunit = FileUnit::from_file(file)?;
    Ok(parse_unit(lang, &fileunit))
}
