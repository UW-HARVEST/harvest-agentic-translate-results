//! Sanity checks for the harness itself: both `.so`s load, both export the
//! symbols, and stdout capture actually captures something.

mod harness;
use harness::*;

#[test]
fn both_libraries_load_and_export_symbols() {
    let l = libs();
    assert_eq!(l.c.name, "C libdriver.so");
    assert_eq!(l.rs.name, "Rust libdriver.so");
    // `load()` panics if `driver` or `run` is missing, so reaching here is proof.
}

#[test]
fn capture_actually_captures() {
    let l = libs();
    let z = cbuf(b"3");
    let out = capture_stdout(|| unsafe { (l.c.driver)(z.as_ptr() as *const _) });
    assert!(!out.is_empty(), "captured nothing from the C library");
    assert!(
        out.starts_with(b"The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n"),
        "unexpected first line: {}",
        show(&out)
    );

    let out_rs = capture_stdout(|| unsafe { (l.rs.driver)(z.as_ptr() as *const _) });
    assert_eq!(out, out_rs, "C: {} Rust: {}", show(&out), show(&out_rs));
}

#[test]
fn capture_is_isolated_between_calls() {
    let a = diff_driver("1", "smoke-a");
    let b = diff_driver("1", "smoke-b");
    assert_eq!(a, b, "capture leaked between invocations");
    assert_eq!(a.split(|&c| c == b'\n').filter(|l| !l.is_empty()).count(), 8);
}

#[test]
fn error_sentinel_is_exactly_18_bytes() {
    // Guards against the C `printf`->`puts` rewrite changing the bytes.
    let out = diff_driver("", "smoke-err");
    assert_eq!(out, ERR_MSG);
    assert_eq!(out.len(), 18);
}
