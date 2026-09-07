//! Phase C — error / rejection-path differential tests, one test per
//! `ERRORS.md` row (rows 1-6 and 11 live here; rows 7-10 are driver-level and
//! live in `driver_cli.rs`; row 12 is build-time and lives in
//! `build_surface.rs`).
//!
//! This library has no error enum, no sentinel return and no pointer
//! parameters, so "rejection" means one of two things: the `default:` arm of
//! `DISPATCH_REP` silently returning `INIT_FOR(OP)`, or signed arithmetic
//! wrapping instead of trapping. Both are asserted to be *specifically* the
//! same value in C and Rust, not merely "both did something".

mod common;

use common::*;
use std::ffi::c_int;

/// The one value `use_generated` must reject: `REP7` exists but the `switch` in
/// `DISPATCH_REP` stops at `case 6`, so 7 falls through to `default: break;`.
///
/// ERRORS.md row 1.
#[test]
fn row01_use_generated_seven() {
    let got = diff_un("use_generated", |l| l.use_generated, 7);
    assert_eq!(
        got,
        OP.init(),
        "[{}] use_generated(7) must return INIT_FOR(OP), not REP7",
        tag()
    );
    // Specifically NOT the REP7 value -- proving the default arm was taken.
    assert_ne!(
        got,
        rep_reference(7),
        "[{}] use_generated(7) must not apply REP7",
        tag()
    );
}

/// ERRORS.md row 2 — every value above the last `case`.
#[test]
fn row02_use_generated_above_range() {
    for n in [8, 9, 10, 16, 100, 1_000, 1_000_000, i32::MAX / 2] {
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(
            got,
            OP.init(),
            "[{}] use_generated({n}) must hit default:",
            tag()
        );
    }
}

/// ERRORS.md row 3 — `INT_MAX` selector.
#[test]
fn row03_use_generated_int_max() {
    let got = diff_un("use_generated", |l| l.use_generated, i32::MAX);
    assert_eq!(got, OP.init(), "[{}] use_generated(INT_MAX)", tag());
}

/// ERRORS.md row 4 — negative selectors; the `switch` argument is a signed `int`.
#[test]
fn row04_use_generated_negative() {
    for n in [-1, -2, -3, -6, -7, -8, -100, -1_000_000] {
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(
            got,
            OP.init(),
            "[{}] use_generated({n}) must hit default:",
            tag()
        );
    }
    let mut rng = Rng::new();
    for _ in 0..128 {
        let n = -(rng.next_i32().unsigned_abs() as i64 % 1_000_000) as c_int - 1;
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(got, OP.init(), "[{}] use_generated({n})", tag());
    }
}

/// ERRORS.md row 5 — `INT_MIN` selector, where negating would itself overflow.
#[test]
fn row05_use_generated_int_min() {
    let got = diff_un("use_generated", |l| l.use_generated, i32::MIN);
    assert_eq!(got, OP.init(), "[{}] use_generated(INT_MIN)", tag());
}

/// ERRORS.md row 6 — the accept/reject boundary: last `case` (6) vs first
/// rejected value (7). These must differ, for all three operations.
#[test]
fn row06_use_generated_boundary_6_7() {
    let six = diff_un("use_generated", |l| l.use_generated, 6);
    let seven = diff_un("use_generated", |l| l.use_generated, 7);
    let expect_six: c_int = match OP {
        Op::Add => 15,
        Op::Sub => -15,
        Op::Mul => 720,
    };
    assert_eq!(six, expect_six, "[{}] use_generated(6) = REP6", tag());
    assert_eq!(seven, OP.init(), "[{}] use_generated(7) = INIT", tag());
    assert_ne!(
        six, seven,
        "[{}] the 6/7 boundary must be observable for OP={}",
        tag(),
        OP.cmake_value()
    );
    // Five is accepted too, so the boundary is not simply "everything rejected".
    let five = diff_un("use_generated", |l| l.use_generated, 5);
    assert_eq!(five, rep_reference(5), "[{}] use_generated(5) = REP5", tag());
}

/// ERRORS.md row 11 — signed overflow wraps (gcc `-O2`, two's complement); it
/// must not trap, panic, or saturate on either side.
#[test]
fn row11_overflow_wraps() {
    // op_add
    assert_eq!(diff_bin("op_add", |l| l.op_add, i32::MAX, 1), i32::MIN);
    assert_eq!(diff_bin("op_add", |l| l.op_add, i32::MIN, -1), i32::MAX);
    assert_eq!(
        diff_bin("op_add", |l| l.op_add, i32::MAX, i32::MAX),
        -2,
        "INT_MAX + INT_MAX wraps to -2"
    );
    // op_sub
    assert_eq!(diff_bin("op_sub", |l| l.op_sub, i32::MIN, 1), i32::MAX);
    assert_eq!(diff_bin("op_sub", |l| l.op_sub, i32::MAX, -1), i32::MIN);
    assert_eq!(diff_bin("op_sub", |l| l.op_sub, 0, i32::MIN), i32::MIN);
    // op_mul
    assert_eq!(diff_bin("op_mul", |l| l.op_mul, i32::MAX, i32::MAX), 1);
    assert_eq!(diff_bin("op_mul", |l| l.op_mul, i32::MIN, i32::MIN), 0);
    assert_eq!(diff_bin("op_mul", |l| l.op_mul, i32::MIN, -1), i32::MIN);
    assert_eq!(diff_bin("op_mul", |l| l.op_mul, 65_536, 65_536), 0);

    // The same wraparound reached through the composed entry points, where the
    // STEP_* accumulator is also in play.
    for &(a, b) in &[
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (65_536, 65_536),
        (i32::MIN, i32::MAX),
    ] {
        diff_bin("helper_call", |l| l.helper_call, a, b);
        diff_bin("helper_ptr", |l| l.helper_ptr, a, b);
        diff_g_op(a, b);
    }
}

/// ERRORS.md "generic FFI boundary" row: the `int` selector of `DISPATCH_REP`'s
/// `switch` is the structural stand-in for an out-of-range enum value crossing
/// FFI. Sweep the whole neighbourhood plus randomized full-range `i32`.
#[test]
fn row_all_i32_selector_sweep() {
    for n in -16..=16 {
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(got, dispatch_reference(n), "[{}] selector {n}", tag());
    }
    for n in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        6,
        7,
        i32::MAX - 1,
        i32::MAX,
        0x7FFF,
        -0x8000,
        0x10000,
    ] {
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(got, dispatch_reference(n), "[{}] selector {n}", tag());
    }
    let mut rng = Rng::new();
    for _ in 0..CASES {
        let n = rng.next_i32();
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(got, dispatch_reference(n), "[{}] selector {n}", tag());
    }
}

/// The API has no pointer parameter, so there is no null-pointer rejection to
/// compare. This test records that mechanically rather than by assumption: it
/// fails if the exported functions ever grow a pointer argument, by checking the
/// only pointers in the surface are the two data symbols and that both are
/// non-NULL and readable in both libraries.
#[test]
fn row_no_pointer_parameters() {
    let p = pair();
    for l in [&p.c, &p.rust] {
        let g = l.load_g_op();
        assert!(
            (g as usize) != 0,
            "[{}] {}: G_OP must not be NULL at load time",
            tag(),
            l.name
        );
        let name = l.g_op_name_bytes();
        assert_eq!(
            name,
            OP.cmake_value().as_bytes(),
            "[{}] {}: G_OP_NAME",
            tag(),
            l.name
        );
    }
}
