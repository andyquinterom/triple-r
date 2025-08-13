#[test]
#[cfg(not(miri))]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

#[test]
#[cfg(not(miri))]
fn ui_vec() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui-vec/*.rs");
}
