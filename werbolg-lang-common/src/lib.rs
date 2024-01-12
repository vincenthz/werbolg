#![no_std]

extern crate alloc;
extern crate std;

mod filemap;
mod fileunit;
mod report;
mod span;

use alloc::{format, string::String, vec::Vec};
use werbolg_core as ir;

pub use filemap::{Line, LineCol, LinesMap};
pub use fileunit::FileUnit;
pub use report::{Report, ReportKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub context: Option<ir::Span>,
    pub location: ir::Span,
    pub description: String,
    pub note: Option<String>,
    pub kind: ParseErrorKind,
}

impl ParseError {
    pub fn scope(self, scope: &str) -> ParseError {
        ParseError {
            context: self.context,
            location: self.location,
            description: self.description,
            note: self.note,
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
    use super::*;

    #[test]
    fn report() {
        //
    }

    #[test]
    fn linesmap() {
        let source = "aaa";
        let linemap = LinesMap::new(source);
        assert_eq!(linemap.resolve(0), Some(LineCol::new(Line(0), 0)));
        assert_eq!(linemap.resolve(1), Some(LineCol::new(Line(0), 1)));
        assert_eq!(linemap.resolve(2), Some(LineCol::new(Line(0), 2)));
        assert_eq!(linemap.resolve(3), None);

        let source = "aaa\n";
        let linemap = LinesMap::new(source);
        assert_eq!(linemap.resolve(0), Some(LineCol::new(Line(0), 0)));
        assert_eq!(linemap.resolve(1), Some(LineCol::new(Line(0), 1)));
        assert_eq!(linemap.resolve(2), Some(LineCol::new(Line(0), 2)));
        assert_eq!(linemap.resolve(3), Some(LineCol::new(Line(0), 3)));
        assert_eq!(linemap.resolve(4), None);

        let source = "aaa\nb";
        let linemap = LinesMap::new(source);
        assert_eq!(linemap.resolve(0), Some(LineCol::new(Line(0), 0)));
        assert_eq!(linemap.resolve(1), Some(LineCol::new(Line(0), 1)));
        assert_eq!(linemap.resolve(2), Some(LineCol::new(Line(0), 2)));
        assert_eq!(linemap.resolve(3), Some(LineCol::new(Line(0), 3)));
        assert_eq!(linemap.resolve(4), Some(LineCol::new(Line(1), 0)));
        assert_eq!(linemap.resolve(5), None);

        let source = "this\nis\ntest\n";
        let linemap = LinesMap::new(source);
        assert_eq!(linemap.resolve(0), Some(LineCol::new(Line(0), 0)));
        assert_eq!(linemap.resolve(4), Some(LineCol::new(Line(0), 4)));
        assert_eq!(linemap.resolve(5), Some(LineCol::new(Line(1), 0)));
        assert_eq!(linemap.resolve(6), Some(LineCol::new(Line(1), 1)));
        assert_eq!(linemap.resolve(7), Some(LineCol::new(Line(1), 2)));
        assert_eq!(linemap.resolve(8), Some(LineCol::new(Line(2), 0)));
        assert_eq!(linemap.resolve(12), Some(LineCol::new(Line(2), 4)));
        assert_eq!(linemap.resolve(13), None);
    }
}
