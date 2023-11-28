use core::ops::Deref;

/// Span as a range of bytes in a file
pub type Span = core::ops::Range<usize>;

pub fn span_merge(start: &Span, end: &Span) -> Span {
    assert!(
        start.end <= end.start,
        "merging span failed start={:?} end={:?}",
        start,
        end
    );
    Span {
        start: start.start,
        end: end.end,
    }
}

pub fn spans_merge<'a, I>(it: &mut I) -> Span
where
    I: Iterator<Item = &'a Span>,
{
    let first = it.next().expect("spans merge need at least 1 element");
    let mut span = first.clone();
    while let Some(next) = it.next() {
        assert!(
            span.end < next.start,
            "merging span failed start={:?} end={:?}",
            span,
            next,
        );
        span.end = next.end
    }
    span
}

/// A type T with an attached Span
#[derive(Clone, Debug, Hash)]
pub struct Spanned<T> {
    pub span: Span,
    pub inner: T,
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: Eq> Spanned<T> {
    pub fn span_eq(&self, other: &Self) -> bool {
        self.span == other.span && self.inner == other.inner
    }
}

impl<T> Spanned<T> {
    pub fn new(span: Span, inner: T) -> Self {
        Self { span, inner }
    }
    pub fn unspan(self) -> T {
        self.inner
    }
}
