#![no_std]

extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use werbolg_core as ir;

pub struct FileUnit {
    pub filename: String,
    pub content: String,
}

pub struct Report<'a> {
    pub line: usize,
    pub col: usize,
    pub full_text: &'a str,
}

/// Store a fast resolver from raw bytes offset to (line,col) where line starts at 1
///
/// Also line 1 starts at 0 and is not stored, so effectively this starts with the index
/// of line 2
pub struct LinesMap {
    max_ofs: usize,
    lines: Vec<usize>,
}

pub type LineCol = (u32, u32);

impl LinesMap {
    pub fn new(content: &str) -> Self {
        let mut line_indexes = Vec::new();
        let mut pos = 0;
        let content = content.as_bytes();
        let max_ofs = content.len();
        for line_content in content.split(|c| *c == b'\n') {
            pos += line_content.len() + 1;
            line_indexes.push(pos)
        }
        Self {
            max_ofs,
            lines: line_indexes,
        }
    }

    pub fn resolve(&self, offset: usize) -> Option<LineCol> {
        if offset > self.max_ofs {
            return None;
        }
        match self.lines.binary_search(&offset) {
            Ok(found) => Some((found as u32 + 2, 0)),
            Err(not_found_above) => {
                if not_found_above == 0 {
                    Some((1, offset as u32))
                } else {
                    let prev_line_start = self.lines[not_found_above - 1];
                    let col = offset - prev_line_start;
                    Some(((not_found_above + 1) as u32, col as u32))
                }
            }
        }
    }

    pub fn resolve_span(&self, span: ir::Span) -> Option<(LineCol, LineCol)> {
        let Some(start) = self.resolve(span.start) else {
            return None;
        };
        let Some(end) = self.resolve(span.end) else {
            return None;
        };
        Some((start, end))
    }
}

impl FileUnit {
    pub fn from_string(filename: String, content: String) -> Self {
        Self { filename, content }
    }

    pub fn from_str(filename: &str, content: &str) -> Self {
        Self {
            filename: String::from(filename),
            content: String::from(content),
        }
    }

    pub fn slice(&self, span: ir::Span) -> Option<&str> {
        let bytes = self.content.as_bytes();
        if span.start < bytes.len() && span.end < bytes.len() {
            let slice = &bytes[span];
            Some(core::str::from_utf8(slice).expect("valid slicing"))
        } else {
            None
        }
    }

    pub fn report(&self, span: ir::Span) -> Option<Report> {
        let bytes = self.content.as_bytes();
        if span.start < bytes.len() && span.end < bytes.len() {
            let before = &bytes[0..span.start];
            let mut last = None;
            let mut line = 0usize;
            let mut pos_start_line = 0;
            for chunk in before.split(|b| *b == b'\n') {
                match last {
                    None => last = Some(chunk),
                    Some(l) => {
                        pos_start_line += l.len() + 1;
                        line += 1;
                        last = Some(chunk);
                    }
                }
            }
            let col = span.start - pos_start_line;
            let end = &bytes[span.end..];
            let end_line = match end.iter().position(|b| *b == b'\n') {
                None => span.end + end.len(),
                Some(pos_end) => span.end + pos_end + 1,
            };

            let full_text = &bytes[pos_start_line..end_line];

            Some(Report {
                line: line + 1,
                col,
                full_text: core::str::from_utf8(full_text).expect("valid slicing"),
            })
        } else {
            None
        }
    }
}

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
    use super::*;

    #[test]
    fn report() {
        //
    }

    #[test]
    fn linesmap() {
        let source = "this\nis\ntest\n";
        let linemap = LinesMap::new(source);
        assert_eq!(linemap.resolve(0), Some((1, 0)));
        assert_eq!(linemap.resolve(4), Some((1, 4)));
        assert_eq!(linemap.resolve(5), Some((2, 0)));
        assert_eq!(linemap.resolve(6), Some((2, 1)));
        assert_eq!(linemap.resolve(7), Some((2, 2)));
        assert_eq!(linemap.resolve(8), Some((3, 0)));
        assert_eq!(linemap.resolve(13), Some((4, 0)));
        assert_eq!(linemap.resolve(14), None);
    }
}
