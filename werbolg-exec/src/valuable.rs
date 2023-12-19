use werbolg_core::{ConstrId, ValueFun};

/// A mostly for error and debug useful descriptor for a type of value
pub type ValueKind = &'static [u8; 8];

/// Valuable trait that give access to some underlying type in the value type
pub trait Valuable: Clone {
    /// Get the descriptor of the Valuable object
    fn descriptor(&self) -> ValueKind;

    /// Get the boolean value of a conditional value, or None if not valid
    fn conditional(&self) -> Option<bool>;

    /// Get the a function value from a Valuable object, or None if not valid
    fn fun(&self) -> Option<ValueFun>;

    /// Get a structure out of a Valuable object, or None if not valid
    fn structure(&self) -> Option<(ConstrId, &[Self])>;

    /// Get the elements #index of a Valuable object, or None if not valid
    fn index(&self, index: usize) -> Option<&Self>;

    /// Create a Fun valuable object
    fn make_fun(fun: ValueFun) -> Self;

    /// Create a dummy parameter to push on the stack.
    fn make_dummy() -> Self;
}
