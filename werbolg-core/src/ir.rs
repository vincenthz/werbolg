//! AST for werbolg
//!
//! This try to remain generic to allow multiple different language (existing or new)
//! to target werbolg and be interpreted through it
//!
//! The AST try the terminology of Rust AST, but offer less / more flexibility at time,
//! as this is made for other languages to target also.
//!
//! The example in documentation use rust syntax, but the syntax supported is only defined
//! by the frontend, not in the core.

use super::basic::*;
use super::location::*;

use alloc::{boxed::Box, vec::Vec};

/// AST for a module / source code unit
#[derive(Clone, Debug)]
pub struct Module {
    /// Statement in this module
    pub statements: Vec<Statement>,
}

/// AST for a high level statement
///
/// Current known types are:
///
/// * Use statement for namespace manipulation
/// * Function definition
/// * Struct definition
/// * Naked expression
#[derive(Clone, Debug)]
pub enum Statement {
    /// Use statement
    Use(Use),
    /// Function definition
    Function(Span, FunDef),
    /// Struct definition
    Struct(Span, StructDef),
    /// A naked Expression
    Expr(Expr),
}

/// AST Use/Import
#[derive(Clone, Debug)]
pub struct Use {
    /// the name of the namespace to import
    pub namespace: Ident,
    /// hiding of symbols
    pub hiding: Vec<Ident>,
    /// renaming of symbols, e.g. `use namespace::{x as y}`
    pub renames: Vec<(Ident, Ident)>,
}

/// AST for symbol privacy (public / private)
#[derive(Clone, Copy, Debug)]
pub enum Privacy {
    /// Public privacy allow to define a function that will be reachable by other modules
    Public,
    /// Private privacy keeps the function not reachable to other modules
    Private,
}

/// AST for function definition
///
/// Function definitions are something like:
///
/// ```text
/// function $name ( $vars ) { $body }
/// ```
///
#[derive(Clone, Debug)]
pub struct FunDef {
    /// The privacy associated with this function definition
    pub privacy: Privacy,
    /// The name of this function
    pub name: Option<Ident>,
    /// The function parameters associated with this function
    pub vars: Vec<Variable>,
    /// The content of the function
    pub body: Expr,
}

/// AST for Structure definition
///
/// Structure definitions are something like
///
/// ```text
/// struct $name { $fields }
/// ```
///
#[derive(Clone, Debug)]
pub struct StructDef {
    /// Name of the structure
    pub name: Spanned<Ident>,
    /// Fields of the structure
    pub fields: Vec<Spanned<Ident>>,
}

/// AST for Enum definition
///
/// Enum definitions are something like
///
/// ```text
/// enum $name { $variants }
/// ```
///
#[derive(Clone, Debug)]
pub struct EnumDef {
    /// Name of the enumeration
    pub name: Spanned<Ident>,
    /// Variants for this enumeration
    pub variants: Vec<Variant>,
}

/// Define a variant for a enumeration
#[derive(Clone, Debug)]
pub struct Variant(StructDef);

/// A pattern "matching" for a let
#[derive(Clone, Debug)]
pub enum Binder {
    /// equivalent of `let () = ...`
    Unit,
    /// equivalent of `let _ = ...`
    Ignore,
    /// equivalent of `let $ident = ...`
    Ident(Ident),
}

/// Expression
#[derive(Clone, Debug)]
pub enum Expr {
    /// Literal, e.g. 1, or "abc"
    Literal(Span, Literal),
    /// A Variable, e.g. `a`
    Ident(Span, Ident),
    /// Structure Field access, e.g. `(some expr).$struct-name+$field`
    ///
    /// Note that this need to contains the name of the structure that we
    /// want to access into. it's up to the frontend to provide this information
    /// either by disambiguating at the frontend level by adding explicit struct name
    /// or by other methods
    Field(Box<Expr>, Spanned<Ident>, Spanned<Ident>),
    /// A List expression
    List(Span, Vec<Expr>),
    /// A Let binding of the form `let $binder = $expr in $expr`
    Let(Binder, Box<Expr>, Box<Expr>),
    /// An anonymous function definition expression, e.g. `|a| ...` or `\x -> ...`
    Lambda(Span, Box<FunDef>),
    /// A function call, e.g. `print("hello", "werbolg")`
    Call(Span, Vec<Expr>),
    /// An If expression `if $cond { $then_expr } else { $else_expr }`
    If {
        /// Span of the if
        span: Span,
        /// Condition expression
        cond: Box<Spanned<Expr>>,
        /// Then expression, to run if the conditional hold
        then_expr: Box<Spanned<Expr>>,
        /// Else expression, to run if the conditional does not hold
        else_expr: Box<Spanned<Expr>>,
    },
}

/// A variable (function parameter)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(pub Spanned<Ident>);
