use super::span::Span;
use alloc::string::String;

pub struct FileUnit {
    pub filename: String,
    pub content: String,
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

    pub fn slice(&self, span: Span) -> &str {
        let bytes = self.content.as_bytes();
        let start = usize::min(span.start, bytes.len() - 1);
        let end = usize::min(span.end, bytes.len() - 1);
        let span = core::ops::Range { start, end };
        let slice = &bytes[span];
        core::str::from_utf8(slice).expect("valid slicing")
    }
}
