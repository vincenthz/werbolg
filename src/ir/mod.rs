use alloc::{boxed::Box, string::String, vec::Vec};

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

#[derive(Clone, Debug)]
pub struct Module {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(pub Spanned<Ident>);

impl From<&str> for Ident {
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Ident {
    pub fn matches(&self, s: &str) -> bool {
        self.0 == s
    }
}

#[derive(Clone, Debug)]
pub enum Statement {
    Function(Span, Ident, Vec<Variable>, Expr),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Span, Literal),
    List(Span, Vec<Expr>),
    Let(Spanned<Ident>, Box<Expr>, Box<Expr>),
    Then(Box<Expr>, Box<Expr>),
    Ident(Span, Ident),
    Lambda(Span, Vec<Variable>, Box<Expr>),
    Call(Span, Vec<Expr>),
    If {
        span: Span,
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub enum Literal {
    String(String),
    Number(Number),
    Decimal(Decimal),
    Bytes(Box<[u8]>),
}

#[cfg(feature = "backend-bignum")]
use num_traits::Num;

#[cfg(feature = "backend-bignum")]
use core::str::FromStr;

#[cfg(feature = "backend-bignum")]
pub type NumberInner = num_bigint::BigInt;

#[cfg(feature = "backend-smallnum")]
pub type NumberInner = u64;

#[derive(Clone, Debug)]
pub struct Number(pub NumberInner);

impl Number {
    pub fn from_str_radix(s: &str, n: u32) -> Result<Self, ()> {
        NumberInner::from_str_radix(s, n)
            .map(|n| Self(n))
            .map_err(|_| ())
    }
}

#[cfg(feature = "backend-bignum")]
pub type DecimalInner = bigdecimal::BigDecimal;

#[cfg(feature = "backend-smallnum")]
pub type DecimalInner = f64;

#[derive(Clone, Debug)]
pub struct Decimal(pub DecimalInner);

impl Decimal {
    pub fn from_str(s: &str) -> Result<Self, ()> {
        #[cfg(feature = "backend-bignum")]
        {
            DecimalInner::from_str(s).map(|n| Self(n)).map_err(|_| ())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            use core::str::FromStr;
            DecimalInner::from_str(s).map(|n| Self(n)).map_err(|_| ())
        }
    }
}
