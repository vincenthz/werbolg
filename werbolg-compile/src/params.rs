use werbolg_core::Literal;

pub struct CompilationParams<L: Clone + Eq + core::hash::Hash> {
    /// Map a werbolg-literal into a L type that will be used during execution
    pub literal_mapper: fn(Literal) -> L,
}
