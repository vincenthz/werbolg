use s_expr::Span;

pub use crate::ast;
use std::path::PathBuf;

pub struct FileUnit {
    pub filename: PathBuf,
    pub content: String,
}

impl FileUnit {
    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            filename: path.into(),
            content: content,
        })
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
