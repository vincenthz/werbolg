use super::CompilationError;
use werbolg_core::{Literal, NifId, Span};

/// User driven compilation parameters
#[derive(Clone)]
pub struct CompilationParams<L: Clone + Eq + core::hash::Hash> {
    /// Map a werbolg-literal into a L type that will be used during execution
    pub literal_mapper: fn(Span, Literal) -> Result<L, CompilationError>,

    /// Constructor for a possible sequence of expression (list or array), that
    /// take usize argument from the stack
    pub sequence_constructor: Option<NifId>,
}
