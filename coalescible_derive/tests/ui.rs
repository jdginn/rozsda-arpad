#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass_*.rs");
    t.compile_fail("tests/fail_*.rs");
}
