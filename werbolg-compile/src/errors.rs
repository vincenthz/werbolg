use werbolg_core::{Ident, Literal, Path, Span};

use super::symbols::NamespaceError;
use alloc::{boxed::Box, format, string::String};

/// Compilation error
#[derive(Debug)]
pub enum CompilationError {
    /// Duplicate symbol during compilation (e.g. 2 functions with the name)
    DuplicateSymbol(Span, Ident),
    /// Cannot find the symbol during compilation
    MissingSymbol(Span, Path),
    /// Cannot find the constructor symbol during compilation
    MissingConstructor(Span, Path),
    /// Number of parameters for a functions is above the limit we chose
    FunctionParamsMoreThanLimit(Span, usize),
    /// Core's Literal is not supported by this compiler
    LiteralNotSupported(Span, Literal),
    /// The constructor specified is a not a structure, but trying to access inner field
    ConstructorNotStructure(Span, Path),
    /// The structure specified doesn't have a field of the right name
    StructureFieldNotExistant(Span, Path, Ident),
    /// Namespace Error
    NamespaceError(NamespaceError),
    /// A recursive compilation with some context added
    Context(String, Box<CompilationError>),
}

impl CompilationError {
    /// Get the span of this compilation error
    pub fn span(&self) -> Span {
        match self {
            CompilationError::DuplicateSymbol(span, _) => span.clone(),
            CompilationError::MissingSymbol(span, _) => span.clone(),
            CompilationError::MissingConstructor(span, _) => span.clone(),
            CompilationError::FunctionParamsMoreThanLimit(span, _) => span.clone(),
            CompilationError::LiteralNotSupported(span, _) => span.clone(),
            CompilationError::ConstructorNotStructure(span, _) => span.clone(),
            CompilationError::StructureFieldNotExistant(span, _, _) => span.clone(),
            CompilationError::NamespaceError(_) => todo!(),
            CompilationError::Context(_, e) => e.span(),
        }
    }
}

impl From<NamespaceError> for CompilationError {
    fn from(n: NamespaceError) -> Self {
        CompilationError::NamespaceError(n)
    }
}

impl CompilationError {
    /// Add a context to a compilation error
    pub fn context(self, context: String) -> Self {
        match self {
            Self::Context(msg, c) => CompilationError::Context(format!("{} | {}", context, msg), c),
            _ => CompilationError::Context(context, Box::new(self)),
        }
    }
}

/*
pub fn error_ctx<A>(r: Result<A, CompilationError>, fmt: String) -> Result<A, CompilationError> {
    r.map_err(|e| e.context(fmt))
}
*/
