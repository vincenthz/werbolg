use werbolg_ir_write::module;

#[allow(dead_code)]
pub fn module() -> werbolg_core::Module {
    module! {
        fn one() {
            1
        }
        fn main() {
            expect_int(one(), 1)
        }
    }
}
