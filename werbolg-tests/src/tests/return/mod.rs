mod at_top_level;
mod in_function;

#[test]
fn at_top_level() {
    let mod1 = at_top_level::module();
    let r = crate::execute(mod1);
    match r {
        Ok(a) => match a.int() {
            Ok(i) => assert_eq!(i, 1),
            Err(e) => panic!("{:?}", e),
        },
        Err(e) => panic!("{:?}", e),
    }
}
#[test]
fn in_function() {
    let mod1 = in_function::module();
    let r = crate::execute(mod1);
    assert!(r.is_ok(), "{:?}", r.err())
}
