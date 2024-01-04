mod bool;
mod int;

#[test]
#[should_panic]
fn bool() {
    let mod1 = bool::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}

#[test]
#[should_panic]
fn int() {
    let mod1 = int::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}
