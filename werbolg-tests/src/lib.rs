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

    #[test]
    fn it_compiles() {
        let mod1 = module1();
        assert!(false)
    }
}
