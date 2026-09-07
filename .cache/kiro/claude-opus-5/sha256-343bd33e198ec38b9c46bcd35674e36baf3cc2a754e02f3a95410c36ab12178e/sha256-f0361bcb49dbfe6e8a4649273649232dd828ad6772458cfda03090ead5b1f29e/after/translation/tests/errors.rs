//! Phase C — error-path differential tests, one test per row of `ERRORS.md`
//! plus the generic FFI-boundary cases.
//!
//! Each test asserts the two libraries return the SAME sentinel (the exact
//! negative code from the C source), not merely that "both failed".

mod common;

use common::{diff_gotomach, diff_op, Rng};
use std::ffi::c_void;

const UINT16_MAX: i32 = 65535;

/// Assert the differential result AND pin the exact expected sentinel.
#[track_caller]
fn expect(row: &str, it: i32, seed: i32, mode: i32, th: i32, want: i32) {
    let got = diff_gotomach(row, it, seed, mode, th);
    assert_eq!(
        got, want,
        "[{row}] gotomach({it}, {seed}, {mode}, {th}) should return {want}, both returned {got}"
    );
}

// ---------------------------------------------------------------------------
// Row 1: iterations < 0  =>  -1
// ---------------------------------------------------------------------------

#[test]
fn err_row1_iterations_negative() {
    let mut rng = Rng::new(0xE001);
    for it in [-1, -2, -100, -65535, -65536, i32::MIN, i32::MIN + 1] {
        expect("err1", it, 0, 0, 0, -1);
    }
    for _ in 0..200 {
        let it = rng.range(i32::MIN, -1);
        expect("err1", it, rng.range(0, UINT16_MAX), rng.next_i32(), rng.next_i32(), -1);
    }
}

// ---------------------------------------------------------------------------
// Row 2: iterations > UINT16_MAX  =>  -1
// ---------------------------------------------------------------------------

#[test]
fn err_row2_iterations_too_large() {
    let mut rng = Rng::new(0xE002);
    for it in [65536, 65537, 70000, 1 << 20, i32::MAX - 1, i32::MAX] {
        expect("err2", it, 0, 0, 0, -1);
    }
    for _ in 0..200 {
        let it = rng.range(65536, i32::MAX);
        expect("err2", it, rng.range(0, UINT16_MAX), rng.next_i32(), rng.next_i32(), -1);
    }
    // The bound itself is INSIDE the valid range (`> UINT16_MAX`, not `>=`).
    assert_ne!(
        diff_gotomach("err2", UINT16_MAX, 0, 0, i32::MIN),
        -1,
        "iterations == UINT16_MAX must be accepted"
    );
}

// ---------------------------------------------------------------------------
// Row 3: seed < 0  =>  -2
// ---------------------------------------------------------------------------

#[test]
fn err_row3_seed_negative() {
    let mut rng = Rng::new(0xE003);
    for sd in [-1, -2, -100, -65535, -65536, i32::MIN, i32::MIN + 1] {
        expect("err3", 8, sd, 0, 0, -2);
    }
    for _ in 0..200 {
        let sd = rng.range(i32::MIN, -1);
        expect("err3", rng.range(0, UINT16_MAX), sd, rng.next_i32(), rng.next_i32(), -2);
    }
}

// ---------------------------------------------------------------------------
// Row 4: seed > UINT16_MAX  =>  -2
// ---------------------------------------------------------------------------

#[test]
fn err_row4_seed_too_large() {
    let mut rng = Rng::new(0xE004);
    for sd in [65536, 65537, 70000, 1 << 20, i32::MAX - 1, i32::MAX] {
        expect("err4", 8, sd, 0, 0, -2);
    }
    for _ in 0..200 {
        let sd = rng.range(65536, i32::MAX);
        expect("err4", rng.range(0, UINT16_MAX), sd, rng.next_i32(), rng.next_i32(), -2);
    }
    assert_ne!(
        diff_gotomach("err4", 4, UINT16_MAX, 0, i32::MIN),
        -2,
        "seed == UINT16_MAX must be accepted"
    );
}

// ---------------------------------------------------------------------------
// Rows 5, 6, 9, 10: allocation failure (=> -3 / -4 / NULL).
//
// Unreachable by construction: after the range checks, the largest allocation
// the API can request is 65535 * sizeof(int) = 262140 bytes. See ERRORS.md
// note A. What IS testable, and what a naive translation gets wrong, is the
// malloc(0) case at iterations == 0: glibc returns a unique non-NULL pointer,
// so the -3/-4 branches must NOT be taken.
// ---------------------------------------------------------------------------

#[test]
fn err_row5_row6_alloc_failure_unreachable() {
    let mut rng = Rng::new(0xE005);

    // malloc(0) path: must NOT yield -3 or -4.
    for mode in [0, 1, 2, 5, -5] {
        for th in [i32::MIN, 0, i32::MAX] {
            let r = diff_gotomach("err5", 0, 0, mode, th);
            assert_eq!(r, 0, "iterations == 0 must succeed with an empty sum");
        }
    }

    // Largest allocation the API permits, across all mode paths.
    for mode in [0, 1, 2, 77] {
        let r = diff_gotomach("err5", UINT16_MAX, rng.range(0, UINT16_MAX), mode, i32::MAX);
        assert!(
            r != -3 && r != -4,
            "the maximum permitted allocation must not fail, got {r}"
        );
    }

    // No reachable input anywhere in the valid domain may produce -3 or -4.
    for _ in 0..300 {
        let r = diff_gotomach(
            "err5",
            rng.range(0, UINT16_MAX),
            rng.range(0, UINT16_MAX),
            rng.next_i32(),
            rng.next_i32(),
        );
        assert!(r != -3 && r != -4, "unexpected allocation failure code {r}");
    }
}

// ---------------------------------------------------------------------------
// Row 7: state->status == 0  =>  -5. Unreachable: init_processor always sets
// status = 1 and nothing mutates it before the check. See ERRORS.md note B.
// ---------------------------------------------------------------------------

#[test]
fn err_row7_status_flag_unreachable() {
    let mut rng = Rng::new(0xE007);
    for _ in 0..400 {
        let r = diff_gotomach(
            "err7",
            rng.range(0, UINT16_MAX),
            rng.range(0, UINT16_MAX),
            rng.range(-4, 6),
            rng.next_i32(),
        );
        assert_ne!(r, -5, "the status flag is always 1, so -5 must be unreachable");
    }
}

// ---------------------------------------------------------------------------
// Row 8: !is_valid_state(state) inside the loop  =>  -6. Unreachable:
// capacity == iterations and count increments at most once per iteration.
// See ERRORS.md note C. The worst case is threshold = INT_MAX at the maximum
// iteration count, where every single iteration stores a result.
// ---------------------------------------------------------------------------

#[test]
fn err_row8_state_invalid_unreachable() {
    for mode in [0, 1, 2, 42] {
        for it in [1, 2, 3, 1000, 65534, 65535] {
            let r = diff_gotomach("err8", it, 65535, mode, i32::MAX);
            assert_ne!(r, -6, "count can never reach capacity, so -6 is unreachable");
        }
    }
    let mut rng = Rng::new(0xE008);
    for _ in 0..300 {
        let r = diff_gotomach(
            "err8",
            rng.range(0, 8192),
            rng.range(0, UINT16_MAX),
            rng.range(-4, 6),
            i32::MAX,
        );
        assert_ne!(r, -6);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary cases required by Phase C.
// ---------------------------------------------------------------------------

/// `mode` is switched on as a plain C `int`, so any value with no matching
/// `case` is a real input the `default:` arm must handle identically.
#[test]
fn err_generic_mode_out_of_range() {
    let mut rng = Rng::new(0xE100);
    let fixed = [
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -3,
        -2,
        -1,
        3,
        4,
        5,
        255,
        256,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    for mode in fixed {
        for it in [0, 1, 7, 64] {
            diff_gotomach("gen-mode", it, 12345, mode, 1200);
            diff_gotomach("gen-mode", it, 12345, mode, i32::MIN);
            diff_gotomach("gen-mode", it, 12345, mode, i32::MAX);
        }
    }
    for _ in 0..300 {
        let mode = rng.next_i32();
        if (0..=2).contains(&mode) {
            continue;
        }
        diff_gotomach(
            "gen-mode",
            rng.range(0, 256),
            rng.range(0, UINT16_MAX),
            mode,
            rng.next_i32(),
        );
    }
}

/// One step past each end of each documented range, in both parameters.
#[test]
fn err_generic_off_by_one_boundaries() {
    let its = [-2, -1, 0, 1, UINT16_MAX - 1, UINT16_MAX, 65536, 65537];
    let seeds = [-2, -1, 0, 1, UINT16_MAX - 1, UINT16_MAX, 65536, 65537];
    for it in its {
        for sd in seeds {
            for mode in [0, 1, 2, -1, 3] {
                diff_gotomach("gen-bounds", it, sd, mode, 1200);
            }
        }
    }
}

/// Both range checks violated at once: the C evaluates the `iterations` check
/// first, so `-1` must win over `-2`.
#[test]
fn err_generic_both_ranges_invalid() {
    for it in [-1, i32::MIN, 65536, i32::MAX] {
        for sd in [-1, i32::MIN, 65536, i32::MAX] {
            expect("gen-both", it, sd, 0, 0, -1);
        }
    }
}

/// `INT_MIN` / `INT_MAX` in every one of the four parameters, and all
/// combinations thereof.
#[test]
fn err_generic_extreme_ints() {
    let ext = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                for &d in &ext {
                    diff_gotomach("gen-ext", a, b, c, d);
                }
            }
        }
    }
}

/// `threshold` regimes, including exact equality with a produced value (the
/// comparison is a strict `<`).
#[test]
fn err_generic_threshold_extremes() {
    let mut rng = Rng::new(0xE200);
    for mode in [0, 1, 2, -9, 9] {
        assert_eq!(
            diff_gotomach("gen-th", 100, 500, mode, i32::MIN),
            0,
            "threshold = INT_MIN stores nothing"
        );
        diff_gotomach("gen-th", 100, 500, mode, i32::MAX);
        diff_gotomach("gen-th", 100, 500, mode, i32::MIN + 1);
        diff_gotomach("gen-th", 100, 500, mode, 0);
        diff_gotomach("gen-th", 100, 500, mode, 1);
        diff_gotomach("gen-th", 100, 500, mode, -1);
    }
    // seed + 10 / seed * 2 / seed * 3, the first produced value per mode.
    for seed in [0, 1, 7, 500, 999, 1000, 65535] {
        for (mode, p) in [seed + 10, seed * 2, seed * 3].iter().enumerate() {
            for th in [*p - 1, *p, *p + 1] {
                diff_gotomach("gen-th", 1, seed, mode as i32, th);
                diff_gotomach("gen-th", 5, seed, mode as i32, th);
            }
        }
    }
    for _ in 0..200 {
        diff_gotomach(
            "gen-th",
            rng.range(0, 300),
            rng.range(0, UINT16_MAX),
            rng.range(-2, 4),
            rng.range(-4000, 4000),
        );
    }
}

/// The only pointer in the exported ABI is the operation callbacks'
/// `void *unused_context`. NULL and non-NULL garbage must both be ignored.
#[test]
fn err_generic_op_null_and_garbage_context() {
    let ctxs: [*mut c_void; 5] = [
        std::ptr::null_mut(),
        1usize as *mut c_void,
        usize::MAX as *mut c_void,
        0xDEAD_BEEF_usize as *mut c_void,
        8usize as *mut c_void,
    ];
    let mut rng = Rng::new(0xE300);
    for name in ["process_value", "double_value", "triple_value"] {
        for &ctx in &ctxs {
            for v in [0, 1, -1, 12345, -12345] {
                diff_op("gen-ctx", name, v, rng.next_i32(), ctx);
            }
            for _ in 0..40 {
                diff_op("gen-ctx", name, rng.next_i32(), rng.next_i32(), ctx);
            }
        }
    }
}

/// Signed-overflow inputs to the callbacks. In C this is UB; the Rust must
/// reproduce whatever the compiled C actually does.
#[test]
fn err_generic_op_overflow() {
    let vals = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 9,
        i32::MAX - 10,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 9,
        i32::MIN + 10,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        i32::MAX / 3,
        i32::MAX / 3 + 1,
        i32::MIN / 3,
        i32::MIN / 3 - 1,
    ];
    for name in ["process_value", "double_value", "triple_value"] {
        for v in vals {
            diff_op("gen-ovf", name, v, 0, std::ptr::null_mut());
            diff_op("gen-ovf", name, v, i32::MIN, std::ptr::null_mut());
            diff_op("gen-ovf", name, v, i32::MAX, std::ptr::null_mut());
        }
    }
}
