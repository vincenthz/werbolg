pub mod common;

#[cfg(feature = "lang-rusty")]
pub mod rusty;

#[cfg(feature = "lang-schemy")]
pub mod scheme;

use common::{ast, FileUnit};
use std::path::Path;

pub use common::ParseError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Schemy,
    Rusty,
}

pub fn parse_unit(lang: Lang, unit: &FileUnit) -> Result<ast::Module, ParseError> {
    match lang {
        Lang::Schemy => {
            #[cfg(feature = "lang-schemy")]
            {
                scheme::module(unit)
            }

            #[cfg(not(feature = "lang-schemy"))]
            {
                panic!("schemy language not compiled in")
            }
        }
        Lang::Rusty => {
            #[cfg(feature = "lang-rusty")]
            {
                rusty::module(unit)
            }
            #[cfg(not(feature = "lang-rusty"))]
            {
                panic!("rusty language not compiled in")
            }
        }
    }
}

pub fn parse(lang: Lang, file: &Path) -> std::io::Result<Result<ast::Module, ParseError>> {
    let fileunit = FileUnit::from_file(file)?;
    Ok(parse_unit(lang, &fileunit))
}
