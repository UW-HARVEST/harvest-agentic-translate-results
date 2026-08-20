// Harness smoke test: verifies that both shared objects load, that dlopen with
// RTLD_LOCAL keeps the two identically-named symbol sets from interposing on
// each other, and that stdout capture works.

mod common;

use common::*;

/// Reference output produced by a standalone C program linked against
/// `libtranslated_rust.so` (`confusion(1, 2, 3, 4)`).
const REFERENCE: &str = "Debug: param1 = 1\n\
Debug: param2 = 2\n\
Debug: param3 = 3\n\
Debug: param4 = 4\n\
Debug: state->flags.counter = 1\n\
Bit fields - flag1:0 flag2:1 flag3:0 mode:0\n\
Operation: memchr_found with value 1\n\
Set as int: 1078530011\n\
Final result: 15\n";

#[test]
fn smoke_c_matches_standalone_reference() {
    let (c, _r) = impls();
    let out = capture(|| unsafe { (c.confusion)(1, 2, 3, 4) });
    assert_eq!(out.0, 15);
    assert_eq!(
        String::from_utf8_lossy(&out.1),
        REFERENCE,
        "the C library behaves differently when dlopen'd next to the Rust \
         library -> symbol interposition is happening and every differential \
         test would be meaningless"
    );
}

#[test]
fn smoke_rust_matches_standalone_reference() {
    let (_c, r) = impls();
    let out = capture(|| unsafe { (r.confusion)(1, 2, 3, 4) });
    assert_eq!(out.0, 15);
    assert_eq!(String::from_utf8_lossy(&out.1), REFERENCE);
}

#[test]
fn smoke_both_orders() {
    let (c, r) = impls();
    // Rust first, then C, then Rust again: order must not matter.
    let a = capture(|| unsafe { (r.confusion)(7, 9, 11, 13) });
    let b = capture(|| unsafe { (c.confusion)(7, 9, 11, 13) });
    let d = capture(|| unsafe { (r.confusion)(7, 9, 11, 13) });
    assert_same("order rust/c", b, a);
    assert_eq!(d.1, capture(|| unsafe { (c.confusion)(7, 9, 11, 13) }).1);
}

#[test]
fn smoke_layout_matches() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<ProcessState>(), 24);
    assert_eq!(align_of::<ProcessState>(), 8);
    assert_eq!(std::mem::offset_of!(ProcessState, flags), 0);
    assert_eq!(std::mem::offset_of!(ProcessState, data), 4);
    assert_eq!(std::mem::offset_of!(ProcessState, buffer), 8);
    assert_eq!(std::mem::offset_of!(ProcessState, capacity), 16);
}
