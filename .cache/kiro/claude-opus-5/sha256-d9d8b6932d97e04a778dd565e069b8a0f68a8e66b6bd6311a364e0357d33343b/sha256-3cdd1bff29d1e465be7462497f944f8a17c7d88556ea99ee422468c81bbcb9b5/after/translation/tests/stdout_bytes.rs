//! `printf` byte-fidelity tests.
//!
//! `mdcore.c` reaches `printf` three times; the Rust translation formats with
//! `format!` and writes to `std::io::stdout()` instead. That swaps the libc
//! function used, so the emitted bytes are asserted directly here rather than
//! being taken on trust -- including the `%d` cases where a hand-rolled
//! formatter is most likely to differ from glibc (`INT_MIN`, which has no
//! positive counterpart).

mod common;

use common::*;
use std::ffi::c_int;

fn c_out(f: impl FnOnce() -> c_int) -> String {
    let (_, bytes) = capture_stdout(f);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `printf("helper.call=%d helper.acc=%d\n", r, acc)`
#[test]
fn helper_call_line_bytes() {
    let p = pair();
    let acc = rep_reference(REPEAT);
    for &(a, b) in &[
        (0, 0),
        (7, 3),
        (-1, 1),
        (i32::MIN, 0),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
    ] {
        let cf = p.c.helper_call;
        let rf = p.rust.helper_call;
        let c = c_out(|| unsafe { cf(a, b) });
        let r = c_out(|| unsafe { rf(a, b) });
        let want = format!("helper.call={} helper.acc={}\n", OP.apply(a, b), acc);
        assert_eq!(c, want, "[{}] C helper.call line for ({a}, {b})", tag());
        assert_eq!(r, want, "[{}] Rust helper.call line for ({a}, {b})", tag());
    }
}

/// `printf("helper.ptr=%d\n", r)`
#[test]
fn helper_ptr_line_bytes() {
    let p = pair();
    for &(a, b) in &[
        (0, 0),
        (7, 3),
        (i32::MIN, 0),
        (i32::MIN, 1),
        (i32::MAX, i32::MAX),
        (-2, 3),
    ] {
        let cf = p.c.helper_ptr;
        let rf = p.rust.helper_ptr;
        let c = c_out(|| unsafe { cf(a, b) });
        let r = c_out(|| unsafe { rf(a, b) });
        let want = format!("helper.ptr={}\n", OP.apply(a, b));
        assert_eq!(c, want, "[{}] C helper.ptr line for ({a}, {b})", tag());
        assert_eq!(r, want, "[{}] Rust helper.ptr line for ({a}, {b})", tag());
    }
}

/// `printf("gen.acc=%d\n", r)`
#[test]
fn gen_acc_line_bytes() {
    let p = pair();
    for n in [0, 1, 2, 3, 4, 5, 6, 7, 8, -1, i32::MIN, i32::MAX] {
        let cf = p.c.use_generated;
        let rf = p.rust.use_generated;
        let c = c_out(|| unsafe { cf(n) });
        let r = c_out(|| unsafe { rf(n) });
        let want = format!("gen.acc={}\n", dispatch_reference(n));
        assert_eq!(c, want, "[{}] C gen.acc line for n={n}", tag());
        assert_eq!(r, want, "[{}] Rust gen.acc line for n={n}", tag());
    }
}

/// The leaf operations print nothing at all; a translation that added a stray
/// line would be caught by every `diff_bin`, but assert it explicitly so the
/// "empty output" baseline is not vacuous.
#[test]
fn leaf_ops_print_nothing() {
    let p = pair();
    for l in [&p.c, &p.rust] {
        for f in [l.op_add, l.op_sub, l.op_mul] {
            let out = c_out(|| unsafe { f(7, 3) });
            assert_eq!(out, "", "[{}] {} leaf op printed {out:?}", tag(), l.name);
        }
    }
}

/// Guards the harness itself: if `capture_stdout` silently captured nothing,
/// every byte comparison above would pass vacuously.
#[test]
fn capture_actually_captures() {
    let p = pair();
    let cf = p.c.helper_ptr;
    let rf = p.rust.helper_ptr;
    assert!(
        !c_out(|| unsafe { cf(1, 2) }).is_empty(),
        "capture_stdout saw nothing from the C .so"
    );
    assert!(
        !c_out(|| unsafe { rf(1, 2) }).is_empty(),
        "capture_stdout saw nothing from the Rust .so"
    );
}
