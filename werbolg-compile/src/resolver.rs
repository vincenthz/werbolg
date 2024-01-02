use alloc::vec::Vec;
use werbolg_core::Namespace;

/// Symbol Resolver
#[derive(Clone)]
pub struct SymbolResolver {
    pub(crate) current: Namespace,
    pub(crate) uses: Vec<werbolg_core::Use>,
}

impl SymbolResolver {
    /// Create a resolver from
    pub fn new(current: Namespace, uses: Vec<werbolg_core::Use>) -> Self {
        Self { current, uses }
    }
}
