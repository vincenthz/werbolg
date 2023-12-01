use werbolg_core as ir;
use werbolg_lang_common::{FileUnit, ParseError};

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
                werbolg_lang_lispy::module(unit)
            }

            #[cfg(not(feature = "lang-lispy"))]
            {
                panic!("lispy language not compiled in")
            }
        }
        Lang::Rusty => {
            #[cfg(feature = "lang-rusty")]
            {
                werbolg_lang_rusty::module(unit)
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
