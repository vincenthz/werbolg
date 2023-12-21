//! Werbolg Core

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod basic;
pub mod id;
pub mod idvec;
pub mod ir;
mod location;

pub use basic::*;
pub use id::{ConstrId, FunId, GlobalId, LitId, NifId, ValueFun};
pub use ir::*;
pub use location::*;
