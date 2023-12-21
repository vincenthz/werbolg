use alloc::{boxed::Box, string::String};

/// An ident in the program
///
/// Note that the ident can contains pretty much anything the frontend wants.
/// For example, Space or '::' could be inside the ident
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
    /// check if the Ident matches the string in parameter
    pub fn matches(&self, s: &str) -> bool {
        self.0 == s
    }
}

/// A namespace specifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// No namespace specifier
    None,
    /// Root namespace specifier
    Root,
    /// Explicit named namespace specifier
    Some(Ident),
}

/// Core Literal
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Literal {
    /// Bool
    Bool(Box<str>),
    /// String
    String(Box<str>),
    /// Integral Number
    Number(Box<str>),
    /// Decimal Number
    Decimal(Box<str>),
    /// Bytes
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
