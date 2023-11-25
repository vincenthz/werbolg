use alloc::boxed::Box;

/// Span as a range of bytes in a file
pub type Span = core::ops::Range<usize>;

pub fn span_merge(start: &Span, end: &Span) -> Span {
    assert!(
        start.end < end.start,
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

/// A type T with an attached Span
#[derive(Clone, Debug, Hash)]
pub struct SpannedBox<T> {
    pub span: Span,
    pub inner: Box<T>,
}

impl<T: PartialEq> PartialEq for SpannedBox<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Eq> Eq for SpannedBox<T> {}

impl<T: Eq> SpannedBox<T> {
    pub fn span_eq(&self, other: &Self) -> bool {
        self.span == other.span && self.inner == other.inner
    }
}

impl<T: Clone> SpannedBox<T> {
    pub fn new(span: Span, inner: T) -> Self {
        Self {
            span,
            inner: Box::new(inner),
        }
    }

    pub fn unspan(&self) -> &T {
        self.inner.as_ref()
    }

    pub fn map<F, U>(self, f: F) -> SpannedBox<U>
    where
        F: FnOnce(T) -> U,
    {
        SpannedBox {
            span: self.span.clone(),
            inner: Box::new(f(self.inner.as_ref().clone())),
        }
    }

    pub fn map_result<E, F, U>(self, f: F) -> Result<SpannedBox<U>, E>
    where
        F: Fn(T) -> Result<U, E>,
    {
        let u = f(self.inner.as_ref().clone())?;
        Ok(SpannedBox {
            span: self.span.clone(),
            inner: Box::new(u),
        })
    }
}
