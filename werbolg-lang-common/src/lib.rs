#![no_std]

extern crate alloc;
extern crate std;

mod filemap;
mod fileunit;
mod report;
mod span;

use alloc::{format, string::String, vec::Vec};
use werbolg_core as ir;

pub use filemap::LinesMap;
pub use fileunit::FileUnit;
pub use report::{Report, ReportKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub location: ir::Span,
    pub kind: ParseErrorKind,
}

impl ParseError {
    pub fn scope(self, scope: &str) -> ParseError {
        ParseError {
            location: self.location,
            kind: match self.kind {
                ParseErrorKind::Str(s) => ParseErrorKind::Str(format!("{}{}", scope, s)),
                ParseErrorKind::Unknown => ParseErrorKind::Str(format!("{} Unknown", scope)),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    Unknown,
    Str(String),
}

pub fn hex_decode(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.bytes().len() / 2);
    for i in (0..s.len()).step_by(2) {
        v[i / 2] = u8::from_str_radix(&s[i..i + 2], 16).unwrap()
    }
    v
}

#[cfg(test)]
mod tests {
    use crate::filemap::LineCol;

    use super::*;

    #[test]
    fn report() {
        //
    }

    #[test]
    fn linesmap() {
        let source = "this\nis\ntest\n";
        let linemap = LinesMap::new(source);
        assert_eq!(linemap.resolve(0), Some(LineCol::new(1, 0)));
        assert_eq!(linemap.resolve(4), Some(LineCol::new(1, 4)));
        assert_eq!(linemap.resolve(5), Some(LineCol::new(2, 0)));
        assert_eq!(linemap.resolve(6), Some(LineCol::new(2, 1)));
        assert_eq!(linemap.resolve(7), Some(LineCol::new(2, 2)));
        assert_eq!(linemap.resolve(8), Some(LineCol::new(3, 0)));
        assert_eq!(linemap.resolve(13), Some(LineCol::new(4, 0)));
        assert_eq!(linemap.resolve(14), None);
    }
}
