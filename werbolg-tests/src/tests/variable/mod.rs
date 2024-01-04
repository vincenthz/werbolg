mod allow_shadowing_on_parameter;
mod undefined_local;
#[test]
fn allow_shadowing_on_parameter() {
    let mod1 = allow_shadowing_on_parameter::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}
#[test]
#[should_panic]
fn undefined_local() {
    let mod1 = undefined_local::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}
