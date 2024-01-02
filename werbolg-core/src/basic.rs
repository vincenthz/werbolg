use alloc::{boxed::Box, string::String, vec, vec::Vec};
use core::num::NonZeroUsize;

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

/// A Path (e.g. namespace::function, or a.g.d)
///
/// The path cannot be empty
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Path(PathType, Vec<Ident>);

/// The type of path (Absolute or Relative)
///
/// e.g. `::core::a` vs `core::a`
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PathType {
    /// Absolute path to use to specify according to the toplevel of all modules
    Absolute,
    /// Relative path according to the current namespace being processed
    Relative,
}

impl core::fmt::Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.path_type() {
            PathType::Absolute => write!(f, "::")?,
            PathType::Relative => {}
        }
        for (is_final, component) in self.components() {
            write!(f, "{:?}", component)?;
            if !is_final {
                write!(f, "::")?
            }
        }
        Ok(())
    }
}

impl Path {
    /// Split a path to a namespace and an ident
    pub fn split(&self) -> (Namespace, Ident) {
        let mut x = self.1.clone();
        let ident = x.pop().unwrap();
        (Namespace(x), ident)
    }

    /// Return the path type of this path
    pub fn path_type(&self) -> PathType {
        self.0
    }

    /// Check if a path is local
    pub fn is_local(&self) -> bool {
        self.0 == PathType::Relative && self.1.len() == 1
    }

    /// Get the local Ident if this is a local Path
    pub fn get_local(&self) -> Option<&Ident> {
        if self.is_local() {
            Some(&self.1[0])
        } else {
            None
        }
    }

    /// Return the number of elements of the path
    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.1.len()).expect("Path always not null")
    }

    /// Create a new Path
    pub fn new(namespace: Namespace, ident: Ident) -> Self {
        let mut n = namespace.0;
        n.push(ident);
        Path(PathType::Absolute, n)
    }

    /// Create a new raw path
    pub fn new_raw(pathtype: PathType, idents: Vec<Ident>) -> Self {
        Self(pathtype, idents)
    }

    /// Create a new relative path of 1 level
    pub fn relative(ident: Ident) -> Self {
        Self(PathType::Relative, vec![ident])
    }

    /// Create a new absolute path of 1 level
    pub fn absolute(ident: Ident) -> Self {
        Self(PathType::Absolute, vec![ident])
    }

    /// Append to the path
    pub fn append(mut self, ident: Ident) -> Self {
        self.1.push(ident);
        self
    }

    /// Prepend to the path
    pub fn prepend(mut self, ident: Ident) -> Self {
        self.1.insert(0, ident);
        self
    }

    /// Iterate over all components of the path, associate whether the element is final (i.e. a leaf)
    pub fn components(&self) -> impl Iterator<Item = (bool, &Ident)> {
        self.1
            .iter()
            .enumerate()
            .map(|(i, id)| (i + 1 == self.1.len(), id))
    }
}

/// An absolute Path (e.g. namespace::function, or a.g.d)
///
/// The path cannot be empty
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AbsPath(Vec<Ident>);

impl core::fmt::Debug for AbsPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "::")?;
        for (is_final, component) in self.components() {
            write!(f, "{:?}", component)?;
            if !is_final {
                write!(f, "::")?
            }
        }
        Ok(())
    }
}

impl AbsPath {
    /// Split a path to a namespace and an ident
    pub fn split(&self) -> (Namespace, Ident) {
        let mut x = self.0.clone();
        let ident = x.pop().unwrap();
        (Namespace(x), ident)
    }

    /// Return the number of elements of the path
    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.0.len()).expect("Path always not null")
    }

    /// Create a new Abs Path
    pub fn new(namespace: &Namespace, ident: &Ident) -> Self {
        let mut n = namespace.0.clone();
        n.push(ident.clone());
        Self(n)
    }

    /// Create a new Abs Path
    pub fn from_path(path: &Path) -> Self {
        Self(path.1.clone())
    }

    /// Iterate over all components of the path, associate whether the element is final (i.e. a leaf)
    pub fn components(&self) -> impl Iterator<Item = (bool, &Ident)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, id)| (i + 1 == self.0.len(), id))
    }
}

/// A namespace specifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Namespace(Vec<Ident>);

impl Namespace {
    /// Create the root namespace
    pub fn root() -> Self {
        Namespace(vec![])
    }

    /// Check if this is the root namespace
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Append a namespace to this namespace and create a new namespace
    pub fn append(mut self, ident: Ident) -> Self {
        self.0.push(ident);
        self
    }

    /// Append a namespace to this namespace and create a new namespace
    pub fn append_namespace(&self, namespace: &Namespace) -> Namespace {
        let mut out = self.0.clone();
        out.append(&mut namespace.0.clone());
        Self(out)
    }

    /// Drop the first element of the namespace
    pub fn drop_first(mut self) -> (Ident, Self) {
        if self.is_root() {
            panic!("trying to drop root namespace");
        } else {
            let first = self.0.remove(0);
            (first, self)
        }
    }

    /// Get a path combining the namespace and an ident
    pub fn path_with_ident(&self, ident: &Ident) -> Path {
        let mut x = self.0.clone();
        x.push(ident.clone());
        Path(PathType::Absolute, x)
    }

    /// Get a path combining the namespace and a path
    ///
    /// If the path to combine is absolute, then it just returns that
    /// otherwise it combine with the namespace
    pub fn path_with_path(&self, path: &Path) -> Path {
        if path.path_type() == PathType::Absolute {
            path.clone()
        } else {
            let mut x = self.0.clone();
            x.append(&mut path.1.clone());
            Path(PathType::Absolute, x)
        }
    }

    /// Iterate over all the components in the namespace
    pub fn iter(&self) -> impl Iterator<Item = &Ident> {
        self.0.iter()
    }

    /// Iterate over all the components in the namespace and also specify if it's the last components
    pub fn iter_with_last(&self) -> impl Iterator<Item = (bool, &Ident)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, id)| (i + 1 == self.0.len(), id))
    }
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

impl Literal {
    /// Create a new number literal
    pub fn number(s: &str) -> Self {
        Self::Number(s.into())
    }

    /// Create a new string literal
    pub fn string(s: &str) -> Self {
        Self::String(s.into())
    }
}
