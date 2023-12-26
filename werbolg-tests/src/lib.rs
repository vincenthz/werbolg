#![no_std]
extern crate alloc;
extern crate proc_macro;

use werbolg_ir_write::module;

fn module1() -> werbolg_core::Module {
    module! {
        fn add(a, b) {
            a
        }

        fn sub(a, b) {
            b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};
    use werbolg_compile::{compile, CompilationError, CompilationParams, Environment};

    fn literal_mapper(
        lit: werbolg_core::Literal,
    ) -> Result<werbolg_core::Literal, CompilationError> {
        Ok(lit)
    }

    #[test]
    fn it_compiles() {
        let mod1 = module1();
        let params = CompilationParams { literal_mapper };
        let mut environ = Environment::<(), ()>::new();
        let r = compile(
            &params,
            vec![(werbolg_core::Namespace::root(), mod1)],
            &mut environ,
        );
        assert!(r.is_ok(), "{:?}", r.err())
    }
}
