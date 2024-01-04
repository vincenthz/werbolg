mod literals;

#[test]
fn literals() {
    let mod1 = literals::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}
