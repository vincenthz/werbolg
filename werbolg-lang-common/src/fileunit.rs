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

    /*
    pub fn report(&self, span: Span) -> Option<Report> {
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
    */
}
