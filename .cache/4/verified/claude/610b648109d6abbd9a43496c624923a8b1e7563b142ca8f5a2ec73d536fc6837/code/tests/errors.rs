//! Phase C — error/rejection-path differential tests for the library surface.
//! One test per `ERRORS.md` row (rows 5-15, G1, G2); the `main`/`atoi` rows live
//! in `driver_bin.rs`, and rows 16-17/G3-G4 in `globals.rs`.
//!
//! `mdcore.c` has no error returns, no asserts and no range checks, so the
//! "rejection" behaviour under test is (a) the `default: break;` arm of the
//! `DISPATCH_REP` switch, which silently yields `INIT_FOR(OP)`, and (b) the
//! wrapping of signed overflow. Both must agree exactly, not merely "both fail".

mod common;

use common::*;
use std::ffi::c_int;

/// `INIT_FOR(OP)` — what the `default:` arm leaves the accumulator at.
const INIT: c_int = init_for_op();

/// Assert `use_generated(n)` returns `INIT` in *both* libraries and prints the
/// matching `gen.acc=` line.
fn assert_default_arm(n: c_int) {
    let got = diff1("use_generated", n);
    assert_eq!(
        got, INIT,
        "use_generated({n}) must hit `default: break;` and return INIT_{OP}={INIT}"
    );
    // The printed line is the other half of the observable behaviour.
    let (c, r) = libs();
    let cf = c.func1("use_generated");
    let rf = r.func1("use_generated");
    let (_, cout) = capture(|| unsafe { cf(n) });
    let (_, rout) = capture(|| unsafe { rf(n) });
    assert_eq!(show(&cout), show(&rout));
    assert_eq!(show(&cout), format!("gen.acc={INIT}\n"));
}

/* ---------------- ERRORS.md rows 5-10: the switch arms ---------------- */

#[test]
fn use_generated_negative() {
    // Row 5
    for n in [-1, -2, -7, -100, -12345] {
        assert_default_arm(n);
    }
}

#[test]
fn use_generated_seven_is_default() {
    // Row 6 -- the important one: `case 6` is the last label, so n==7 falls to
    // `default:` EVEN in a REPEAT=7 build, where helper_call does perform 7 steps.
    assert_default_arm(7);

    if REPEAT == 7 {
        // Same build, two different answers for "7 steps": helper_call unrolls 7
        // times while accum_<OP>(7) does nothing at all. Pin that asymmetry.
        let mut acc = INIT;
        for i in 0..7 {
            acc = match OP {
                "add" => acc.wrapping_add(i),
                "sub" => acc.wrapping_sub(i),
                _ => acc.wrapping_mul(i.wrapping_add(1)),
            };
        }
        assert_ne!(
            acc, INIT,
            "sanity: 7 steps should differ from INIT for every OP"
        );
        let (c, _) = libs();
        let f = c.func1("use_generated");
        let (v, _) = capture(|| unsafe { f(7) });
        assert_eq!(v, INIT, "C really does take the default arm for n=7");
    }
}

#[test]
fn use_generated_far_out_of_range() {
    // Row 7
    for n in [8, 9, 100, 1000, 65536, c_int::MAX] {
        assert_default_arm(n);
    }
}

#[test]
fn use_generated_int_min() {
    // Row 8
    assert_default_arm(c_int::MIN);
    assert_default_arm(c_int::MIN + 1);
}

#[test]
fn use_generated_zero_boundary() {
    // Row 9 -- `case 0: REP0` expands to nothing, so 0 is indistinguishable from
    // the default arm; it must still be INIT and must not be an error.
    assert_default_arm(0);
}

#[test]
fn use_generated_last_valid_case() {
    // Row 10 -- n==6 must NOT fall through to the default arm.
    let want: c_int = match OP {
        "add" => 15,
        "sub" => -15,
        _ => 720,
    };
    let got = diff1("use_generated", 6);
    assert_eq!(got, want, "use_generated(6) with OP={OP}");
    assert_ne!(got, INIT, "n=6 is a real case, not the default arm");

    // ... and n==5, the case just below, differs from n==6.
    let five = diff1("use_generated", 5);
    assert_ne!(five, want, "case 5 and case 6 must differ");
}

/* ------------- ERRORS.md rows 11-13: signed-overflow wrapping ------------- */

#[test]
fn op_overflow_boundaries() {
    // Rows 11-13. Signed overflow is UB in C, but with the flags CMake uses gcc
    // emits a plain wrapping add/sub/imul. The Rust must wrap too -- and must
    // never panic (a debug-profile `a + b` would abort here).
    let cases: [(c_int, c_int); 14] = [
        (c_int::MAX, 1),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, -1),
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, 1),
        (c_int::MAX, -1),
        (c_int::MAX, c_int::MIN),
        (c_int::MIN, c_int::MAX),
        (-1, c_int::MIN),
        (2, c_int::MAX),
        (c_int::MAX / 2 + 1, 2),
        (65536, 65536),
        (-65536, 65536),
        (46341, 46341),
    ];
    for (a, b) in cases {
        assert_eq!(diff2("op_add", a, b), a.wrapping_add(b), "op_add({a},{b})");
        assert_eq!(diff2("op_sub", a, b), a.wrapping_sub(b), "op_sub({a},{b})");
        assert_eq!(diff2("op_mul", a, b), a.wrapping_mul(b), "op_mul({a},{b})");
    }
}

#[test]
fn helper_call_return_overflow() {
    // Rows 14-15: `return r + acc;` can overflow independently of `r` itself,
    // and the printf must still show the pre-addition r and acc.
    let cases: [(c_int, c_int); 8] = [
        (c_int::MAX, 0),
        (c_int::MAX, -1),
        (c_int::MIN, 0),
        (c_int::MIN, 1),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX - 5, 0),
        (c_int::MIN + 5, 0),
    ];
    for (a, b) in cases {
        diff2("helper_call", a, b);
        diff2("helper_ptr", a, b);
    }
}

/* ---------------------- ERRORS.md G1 / G2: sweeps ---------------------- */

#[test]
fn exhaustive_boundary_cross_product() {
    // G1: every binary entry point x the 9x9 boundary cross-product.
    for f in BIN_FUNCS {
        for a in BOUNDARIES {
            for b in BOUNDARIES {
                diff2(f, a, b);
            }
        }
    }
}

#[test]
fn use_generated_full_sweep() {
    // G2: the "out-of-range enum" analogue. A C `switch` accepts any int, so
    // every one of these is a real input the C handles and the Rust must match.
    for n in -8..=15 {
        diff1("use_generated", n);
    }
    for n in [
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN / 2,
        -1,
        c_int::MAX / 2,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        diff1("use_generated", n);
    }
    // Only 0..=6 are real cases; everything else is the default arm.
    for n in -8..=15 {
        let got = diff1("use_generated", n);
        if (0..=6).contains(&n) {
            let mut acc = INIT;
            for i in 0..n {
                acc = match OP {
                    "add" => acc.wrapping_add(i),
                    "sub" => acc.wrapping_sub(i),
                    _ => acc.wrapping_mul(i.wrapping_add(1)),
                };
            }
            assert_eq!(got, acc, "use_generated({n}) valid case");
        } else {
            assert_eq!(got, INIT, "use_generated({n}) default arm");
        }
    }
}

#[test]
fn repeated_calls_are_stateless() {
    // The C functions keep no state between calls (acc is a local). Calling the
    // same input twice must give the same answer and the same output bytes.
    for f in BIN_FUNCS {
        let first = diff2(f, 11, 5);
        for _ in 0..4 {
            assert_eq!(diff2(f, 11, 5), first, "{f} is not stateless");
        }
    }
    let g = diff1("use_generated", 4);
    for _ in 0..4 {
        assert_eq!(diff1("use_generated", 4), g);
    }
}
