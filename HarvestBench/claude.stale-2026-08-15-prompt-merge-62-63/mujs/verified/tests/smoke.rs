//! Sanity checks for the differential harness itself.
mod common;
use common::*;

#[test]
fn smoke_both_libs_load_and_agree() {
    let (c, r) = both(|api, _| run_script(api, 0, "print(1+1); print('hi'); print([1,2,3]);"));
    assert_eq!(c, r, "C={:?} Rust={:?}", c, r);
    assert_eq!(c.0, 0, "script should succeed");
    assert_eq!(String::from_utf8_lossy(&c.1), "2\nhi\n1,2,3\n");
}

#[test]
fn smoke_error_reporting_agrees() {
    let (c, r) = both(|api, _| run_script(api, 0, "null.x;"));
    assert_eq!(c, r, "C={:?} Rust={:?}", c, r);
    assert_eq!(c.0, 1, "script should fail");
    assert!(
        String::from_utf8_lossy(&c.1).starts_with("[report] TypeError"),
        "unexpected report: {:?}",
        String::from_utf8_lossy(&c.1)
    );
}

#[test]
fn smoke_strict_mode_differs_from_sloppy() {
    // Assignment to an undeclared variable: allowed sloppy, ReferenceError strict.
    let sloppy = both(|api, _| run_script(api, 0, "x = 1; print(x);"));
    let strict = both(|api, _| run_script(api, JS_STRICT, "x = 1; print(x);"));
    assert_eq!(sloppy.0, sloppy.1);
    assert_eq!(strict.0, strict.1);
    assert_eq!(sloppy.0 .0, 0);
    assert_eq!(strict.0 .0, 1);
}
