#![no_std]
extern crate alloc;

extern crate proc_macro;

use alloc::{vec, vec::Vec};
use proc_macro::{Literal, TokenTree};

pub trait ToTokenTrees {
    fn generate(ty: Self) -> Vec<TokenTree>;
}

impl ToTokenTrees for alloc::string::String {
    fn generate(ty: Self) -> Vec<TokenTree> {
        vec![Literal::string(ty.as_str()).into()]
    }
}

impl ToTokenTrees for u32 {
    fn generate(ty: Self) -> Vec<TokenTree> {
        vec![Literal::u32_suffixed(ty).into()]
    }
}

impl ToTokenTrees for proc_macro::TokenStream {
    fn generate(ty: Self) -> Vec<TokenTree> {
        ty.into_iter().collect::<Vec<_>>()
    }
}
