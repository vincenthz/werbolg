mod associativity;

#[test]
fn associativity() {
    let mod1 = associativity::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}
