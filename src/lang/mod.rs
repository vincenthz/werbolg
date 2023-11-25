pub mod common;

#[cfg(feature = "lang-rusty")]
pub mod rusty;

#[cfg(feature = "lang-lispy")]
pub mod lispy;

use common::{ir, FileUnit};

pub use common::ParseError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Lispy,
    Rusty,
}

pub fn parse_unit(lang: Lang, unit: &FileUnit) -> Result<ir::Module, ParseError> {
    match lang {
        Lang::Lispy => {
            #[cfg(feature = "lang-lispy")]
            {
                lispy::module(unit)
            }

            #[cfg(not(feature = "lang-lispy"))]
            {
                panic!("lispy language not compiled in")
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

pub fn parse(lang: Lang, file: &FileUnit) -> Result<ir::Module, ParseError> {
    parse_unit(lang, file)
}
