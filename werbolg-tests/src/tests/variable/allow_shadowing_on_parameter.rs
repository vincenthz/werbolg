use werbolg_ir_write::module;

#[allow(dead_code)]
pub fn module() -> werbolg_core::Module {
    module! {
        fn func(a) {
            let a = 1;
            1
        }
        fn main() {
            func(1)
        }
    }
}
