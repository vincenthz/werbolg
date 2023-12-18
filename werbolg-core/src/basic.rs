use alloc::{boxed::Box, string::String};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

impl core::fmt::Debug for Ident {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "@{}", self.0)
    }
}

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

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Literal {
    Bool(Box<str>),
    String(Box<str>),
    Number(Box<str>),
    Decimal(Box<str>),
    Bytes(Box<[u8]>),
}

impl core::fmt::Debug for Literal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Literal::Bool(s) => {
                write!(f, "\"{}\"", s)
            }
            Literal::String(s) => {
                write!(f, "\"{}\"", s)
            }
            Literal::Number(n) => {
                write!(f, "{}", n)
            }
            Literal::Decimal(d) => {
                write!(f, "{}", d)
            }
            Literal::Bytes(bytes) => {
                write!(f, "#")?;
                for b in bytes.iter() {
                    write!(f, "{:02X}", b)?;
                }
                Ok(())
            }
        }
    }
}
