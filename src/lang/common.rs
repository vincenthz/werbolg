use s_expr::Span;

pub use crate::ast;
use alloc::{format, string::String, vec::Vec};

pub struct FileUnit {
    pub filename: String,
    pub content: String,
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

    pub fn resolve_error(&self, err: &ParseError) -> String {
        let lines = self.content.lines();
        let start = lines.skip(err.location.start.line - 1);
        let x = start.take(err.location.end.line - err.location.start.line + 1);
        x.collect()
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub location: Span,
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
