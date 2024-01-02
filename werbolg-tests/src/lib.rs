#![no_std]
extern crate alloc;
extern crate proc_macro;

use werbolg_ir_write::module;

pub fn module1() -> werbolg_core::Module {
    module! {
        fn add(a, b) {
            pop(1)
        }

        fn sub(a, b) {
            push(b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use werbolg_compile::{compile, CompilationError, CompilationParams, Environment};
    use werbolg_core::{AbsPath, Ident, Namespace};

    //extern crate std;

    fn literal_mapper(
        lit: werbolg_core::Literal,
    ) -> Result<werbolg_core::Literal, CompilationError> {
        Ok(lit)
    }

    #[test]
    fn it_compiles() {
        let mod1 = module1();
        //std::println!("{:?}", mod1);
        //assert!(false);
        let params = CompilationParams { literal_mapper };
        let mut environ = Environment::<(), ()>::new();
        environ.add_nif(&AbsPath::new(&Namespace::root(), &Ident::from("pop")), ());
        environ.add_nif(&AbsPath::new(&Namespace::root(), &Ident::from("push")), ());
        let r = compile(
            &params,
            vec![(werbolg_core::Namespace::root(), mod1)],
            &mut environ,
        );
        assert!(r.is_ok(), "{:?}", r.err())
    }
}
