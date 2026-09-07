//! Harness smoke test: proves both `.so`s load, all six symbols resolve, and
//! stdout capture works.

mod common;
use common::*;

#[test]
fn harness_loads_both_libraries_and_captures_stdout() {
    let p = pair();

    let (rc, out_c) = capture(|| unsafe { (p.c.confusion)(42, 7, 3, 1) });
    let (rr, out_r) = capture(|| unsafe { (p.r.confusion)(42, 7, 3, 1) });

    assert_eq!(rc, rr, "confusion return value differs");
    assert_eq!(out_c, out_r, "stdout differs:\nC   = {}\nRust= {}", show(&out_c), show(&out_r));
    assert!(!out_c.is_empty(), "stdout capture produced nothing — harness broken");
    assert!(
        out_c.starts_with(b"Debug: param1 = 42\n"),
        "unexpected captured stdout: {}",
        show(&out_c)
    );
}
