use alloc::string::String;

#[derive(Debug, Clone)]
pub struct Location {
    pub module: String,
    pub span: core::ops::Range<usize>,
}

impl Location {
    pub fn from_span(span: &core::ops::Range<usize>) -> Self {
        Self {
            module: String::new(),
            span: span.clone(),
        }
    }
}
