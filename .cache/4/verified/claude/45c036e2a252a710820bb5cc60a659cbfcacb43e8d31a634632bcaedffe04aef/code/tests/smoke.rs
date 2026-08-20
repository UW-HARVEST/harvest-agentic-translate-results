// Harness self-check: proves both .so files load, that symbols resolve through
// dlsym only, and that the stdout capture actually captures library output.

mod common;

use common::*;

#[test]
fn harness_loads_both_shared_objects() {
    let _g = gate();
    let (c, r) = apis();
    assert!(c.path.ends_with("libtranslated_rust.so"), "{:?}", c.path);
    assert!(r.path.ends_with("libcharinbuf_lib.so"), "{:?}", r.path);
}

#[test]
fn harness_captures_stdout_from_both() {
    let _g = gate();
    let (c, r) = apis();

    let (cv, cb) = capture(|| (c.charinbuf)(1, 0, 0, 0));
    let (rv, rb) = capture(|| (r.charinbuf)(1, 0, 0, 0));

    assert_eq!(cv, 10, "C mode 1 return");
    assert_eq!(rv, 10, "Rust mode 1 return");
    assert!(
        !cb.is_empty() && !rb.is_empty(),
        "capture produced nothing: C={:?} Rust={:?}",
        show(&cb),
        show(&rb)
    );
    assert!(
        cb.starts_with(b"Mode 1:"),
        "unexpected captured bytes (contamination?): {}",
        show(&cb)
    );
    assert_eq!(cb, rb, "C=\"{}\" Rust=\"{}\"", show(&cb), show(&rb));
}
