//! Negative controls for the differential harness itself.
//!
//! A differential test suite that cannot fail is worthless, so these tests prove
//! that the capture actually observes the libraries' output and that a mismatch
//! is reported rather than silently swallowed.

mod common;

use common::*;
use std::ffi::c_char;

#[test]
fn selfcheck_capture_sees_c_output() {
    let out = capture(|| unsafe { (c_api().driver)() });
    assert!(
        !out.is_empty(),
        "the capture observed NOTHING from the C library — the harness is broken"
    );
    assert_eq!(
        out,
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n"
    );
}

#[test]
fn selfcheck_capture_sees_rust_output() {
    let out = capture(|| unsafe { (rust_api().driver)() });
    assert!(
        !out.is_empty(),
        "the capture observed NOTHING from the Rust library — the harness is broken"
    );
}

#[test]
fn selfcheck_capture_is_not_contaminated_by_the_test_harness() {
    // libtest writes its progress to fd 1; the capture swaps glibc's `stdout`
    // FILE*, so nothing but the libraries' own output may appear.
    println!("this println! must NOT show up in the capture");
    let out = capture(|| {
        println!("neither must this one");
        unsafe { (c_api().print_int_line)(1234) };
    });
    assert_eq!(out, b"1234\n", "capture was contaminated: {out:?}");
}

#[test]
fn selfcheck_divergence_is_detected() {
    // Deliberately mis-pair the two libraries: C's `good()` vs Rust's `bad()`.
    // The comparison logic must notice.
    let c_out = capture(|| unsafe { (c_api().good)() });
    let r_out = capture(|| unsafe { (rust_api().bad)() });
    assert_ne!(
        c_out, r_out,
        "harness cannot distinguish good() from bad() — comparisons are vacuous"
    );
    assert_eq!(c_out, b"0\n2\n");
    assert_eq!(r_out, b"0\n0\n");

    // And the `diff` helper must panic when the two sides disagree.
    let panicked = std::panic::catch_unwind(|| {
        let c = capture(|| unsafe { (c_api().print_int_line)(1) });
        let r = capture(|| unsafe { (rust_api().print_int_line)(2) });
        assert_eq!(c, r);
    })
    .is_err();
    assert!(panicked, "assert_eq! on differing captures did not fail");
}

#[test]
fn selfcheck_every_op_variant_reaches_both_libraries() {
    // Ensure `apply` really dispatches each Op (a typo'd match arm would make a
    // whole CONFIGS row vacuous).
    let ops = vec![
        Op::PrintLine(b"x".to_vec()),
        Op::PrintLineNull,
        Op::PrintIntLine(5),
        Op::PrintIntLineWide(0x1_0000_0007),
        Op::Bad,
        Op::Good,
        Op::Driver,
        Op::BadExtraArgs(1, 2, 3),
        Op::GoodExtraArgs(1, 2, 3),
        Op::DriverExtraArgs(1, 2, 3),
    ];
    for op in &ops {
        let c = capture(|| apply(c_api(), std::slice::from_ref(op)));
        let r = capture(|| apply(rust_api(), std::slice::from_ref(op)));
        assert_eq!(c, r, "op {op:?} diverged");
        if !matches!(op, Op::PrintLineNull) {
            assert!(!c.is_empty(), "op {op:?} produced no output at all");
        }
    }
}

#[test]
fn selfcheck_buffering_modes_all_capture() {
    for mode in [
        Buffering::Default,
        Buffering::Unbuffered,
        Buffering::LineBuffered,
        Buffering::FullyBuffered,
    ] {
        let out = capture_with(mode, || unsafe {
            (c_api().print_line)(b"abc\0".as_ptr() as *const c_char)
        });
        assert_eq!(out, b"abc\n", "mode {mode:?} lost output");
    }
}
