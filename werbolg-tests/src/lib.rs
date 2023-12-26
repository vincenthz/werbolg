#![no_std]
extern crate alloc;

use werbolg_ir_write::module;

fn module1() -> werbolg_core::Module {
    module! {
        fn add(a, b) {
            a + b
        }

        fn sub(a, b) {
            a - b
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
