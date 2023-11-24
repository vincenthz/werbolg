pub use crate::ast;
use alloc::{format, string::String, vec::Vec};

pub struct FileUnit {
    pub filename: String,
    pub content: String,
}

pub struct Report<'a> {
    pub line: usize,
    pub col: usize,
    pub full_text: &'a str,
}

impl FileUnit {
    #[cfg(std)]
    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            filename: path.into(),
            content: content,
        })
    }

    pub fn from_string(filename: String, content: String) -> Self {
        Self { filename, content }
    }

    pub fn from_str(filename: &str, content: &str) -> Self {
        Self {
            filename: String::from(filename),
            content: String::from(content),
        }
    }

    pub fn slice(&self, span: core::ops::Range<usize>) -> Option<&str> {
        let bytes = self.content.as_bytes();
        if span.start < bytes.len() && span.end < bytes.len() {
            let slice = &bytes[span];
            Some(core::str::from_utf8(slice).expect("valid slicing"))
        } else {
            None
        }
    }

    pub fn report(&self, span: core::ops::Range<usize>) -> Option<Report> {
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
    pub location: core::ops::Range<usize>,
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
    #[test]
    fn report() {
        //
    }
}
