//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH the C and
//! the Rust `.so`, and asserts the SAME error code (not merely "both failed").

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Asserts C and Rust agree AND that the shared answer is the documented code.
fn expect_code(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int, expected: c_int) {
    let rc = diff_gotomach(iterations, seed, mode, threshold);
    assert_eq!(
        rc, expected,
        "gotomach({iterations}, {seed}, {mode}, {threshold}) returned {rc}, expected {expected}"
    );
}

// ===========================================================================
// ERRORS.md row 1 — iterations < 0  =>  -1
// ===========================================================================

#[test]
fn err_row01_iterations_negative_returns_minus1() {
    let mut rng = Rng::new(0xE001);
    for it in [-1, -2, -10, -65535, -65536, i32::MIN + 1, i32::MIN] {
        for &mode in ALL_MODES {
            expect_code(it, 0, mode, 0, -1);
        }
    }
    for _ in 0..3_000 {
        let it = rng.range_i32(i32::MIN, -1);
        let seed = rng.next_i32();
        let mode = rng.next_i32();
        let threshold = rng.next_i32();
        assert_eq!(diff_gotomach_rc(it, seed, mode, threshold), -1);
    }
}

// ===========================================================================
// ERRORS.md row 2 — iterations > UINT16_MAX  =>  -1
// ===========================================================================

#[test]
fn err_row02_iterations_above_u16max_returns_minus1() {
    // One step past the documented valid range and beyond.
    for it in [65536, 65537, 70000, 1 << 20, i32::MAX - 1, i32::MAX] {
        for &mode in ALL_MODES {
            expect_code(it, 0, mode, 0, -1);
        }
    }
    // 65535 is the last ACCEPTED value: it must NOT error.
    for &mode in ALL_MODES {
        assert_eq!(diff_gotomach_rc(65535, 0, mode, i32::MIN), 0);
    }
    let mut rng = Rng::new(0xE002);
    for _ in 0..3_000 {
        let it = rng.range_i32(65536, i32::MAX);
        let seed = rng.next_i32();
        let mode = rng.next_i32();
        let threshold = rng.next_i32();
        assert_eq!(diff_gotomach_rc(it, seed, mode, threshold), -1);
    }
}

// ===========================================================================
// ERRORS.md row 3 — seed < 0 (iterations valid)  =>  -2
// ===========================================================================

#[test]
fn err_row03_seed_negative_returns_minus2() {
    for seed in [-1, -2, -1000, -65535, -65536, i32::MIN + 1, i32::MIN] {
        for &it in &[0, 1, 8, 65535] {
            for &mode in ALL_MODES {
                expect_code(it, seed, mode, 0, -2);
            }
        }
    }
    let mut rng = Rng::new(0xE003);
    for _ in 0..3_000 {
        let it = rng.range_i32(0, 65535);
        let seed = rng.range_i32(i32::MIN, -1);
        assert_eq!(diff_gotomach_rc(it, seed, rng.next_i32(), rng.next_i32()), -2);
    }
}

// ===========================================================================
// ERRORS.md row 4 — seed > UINT16_MAX (iterations valid)  =>  -2
// ===========================================================================

#[test]
fn err_row04_seed_above_u16max_returns_minus2() {
    for seed in [65536, 65537, 70000, 1 << 20, i32::MAX - 1, i32::MAX] {
        for &it in &[0, 1, 8, 65535] {
            for &mode in ALL_MODES {
                expect_code(it, seed, mode, 0, -2);
            }
        }
    }
    // 65535 is the last ACCEPTED seed: it must NOT error.
    for &mode in ALL_MODES {
        assert_eq!(diff_gotomach_rc(0, 65535, mode, 0), 0);
        assert_eq!(diff_gotomach_rc(8, 65535, mode, i32::MIN), 0);
    }
    let mut rng = Rng::new(0xE004);
    for _ in 0..3_000 {
        let it = rng.range_i32(0, 65535);
        let seed = rng.range_i32(65536, i32::MAX);
        assert_eq!(diff_gotomach_rc(it, seed, rng.next_i32(), rng.next_i32()), -2);
    }
}

// ===========================================================================
// ERRORS.md rows 5 & 6 — the allocation-failure paths (-3 / -4).
//
// Unreachable for every accepted `iterations` (the largest allocation is
// 65535 * 4 bytes). The observable contract is therefore "never returns -3 or
// -4", and in particular `iterations == 0` must NOT be mistaken for a malloc(0)
// failure. Asserted for BOTH implementations.
// ===========================================================================

#[test]
fn err_row05_row06_allocation_failures_never_observed() {
    let mut rng = Rng::new(0xE056);

    // iterations == 0 exercises malloc(0) for both `results` and `temp_buffer`.
    for &mode in ALL_MODES {
        for &seed in &[0, 1, 65535] {
            for &threshold in &[i32::MIN, 0, i32::MAX] {
                let rc = diff_gotomach(0, seed, mode, threshold);
                assert_eq!(rc, 0, "iterations==0 must return 0, not an alloc error");
            }
        }
    }

    // No accepted input may ever yield -3 or -4 in either implementation.
    for _ in 0..20_000 {
        let it = rng.range_i32(0, 65535);
        let seed = rng.range_i32(0, 65535);
        let rc = diff_gotomach_rc(it, seed, rng.next_i32(), rng.next_i32());
        assert_ne!(rc, -3, "unexpected init_processor failure for ({it}, {seed})");
        assert_ne!(rc, -4, "unexpected temp_buffer failure for ({it}, {seed})");
    }

    // The largest accepted allocation, for every mode.
    for &mode in ALL_MODES {
        let rc = diff_gotomach_rc(65535, 1, mode, i32::MIN);
        assert_ne!(rc, -3);
        assert_ne!(rc, -4);
    }
}

// ===========================================================================
// ERRORS.md row 7 — check_char_flag(state->status) false  =>  -5
//
// `init_processor` always sets status = 1, so -5 is unreachable. Contract:
// never returned, by either implementation, anywhere in the accepted domain.
// ===========================================================================

#[test]
fn err_row07_invalid_state_status_minus5_never_observed() {
    let mut rng = Rng::new(0xE007);
    for _ in 0..20_000 {
        let it = rng.range_i32(0, 65535);
        let seed = rng.range_i32(0, 65535);
        let rc = diff_gotomach_rc(it, seed, rng.next_i32(), rng.next_i32());
        assert_ne!(rc, -5, "unexpected -5 for ({it}, {seed})");
    }
    for &mode in ALL_MODES {
        for &it in &[0, 1, 2, 64, 1000, 65535] {
            assert_ne!(diff_gotomach_rc(it, 3, mode, i32::MAX), -5);
        }
    }
}

// ===========================================================================
// ERRORS.md row 8 — is_valid_state() false inside the loop  =>  -6
//
// `count` grows by at most 1 per iteration and `capacity == iterations`, so
// `count <= i < capacity` always holds and -6 is unreachable. The interesting
// stress case is `threshold == INT_MAX`, where EVERY element is appended and
// `count` tracks `i` exactly — i.e. the tightest approach to the bound.
// ===========================================================================

#[test]
fn err_row08_state_became_invalid_minus6_never_observed() {
    let mut rng = Rng::new(0xE008);
    for &mode in ALL_MODES {
        // Fill `results` completely: count == capacity on the final iteration.
        for &it in &[0, 1, 2, 3, 64, 255, 256, 1000, 65534, 65535] {
            let rc = diff_gotomach_rc(it, 7, mode, i32::MAX);
            assert_ne!(rc, -6, "unexpected -6 for iterations={it} mode={mode}");
        }
    }
    for _ in 0..20_000 {
        let it = rng.range_i32(0, 1024);
        let seed = rng.range_i32(0, 65535);
        let rc = diff_gotomach_rc(it, seed, rng.next_i32(), i32::MAX);
        assert_ne!(rc, -6, "unexpected -6 for ({it}, {seed})");
    }
}

// ===========================================================================
// ERRORS.md row 9 — out-of-range `mode` enum value  =>  [WARNING] + default op
// ===========================================================================

#[test]
fn err_row09_out_of_range_mode_falls_back_to_process_value() {
    let p = pair();
    let mut rng = Rng::new(0xE009);

    for &mode in INVALID_MODES {
        // Byte-identical stdout including the [WARNING] line, and the same rc.
        let (rc_c, out_c) = {
            let g = p.c.gotomach;
            capture_stdout(|| unsafe { g(8, 5, mode, 1500) })
        };
        let (rc_r, out_r) = {
            let g = p.rust.gotomach;
            capture_stdout(|| unsafe { g(8, 5, mode, 1500) })
        };
        assert_eq!(rc_c, rc_r, "rc mismatch for invalid mode={mode}");
        assert_eq!(out_c, out_r, "stdout mismatch for invalid mode={mode}");
        assert!(
            String::from_utf8_lossy(&out_c).contains("[WARNING] Invalid mode, using default\n"),
            "C did not warn for invalid mode={mode}"
        );
        // The fallback is `process_value`, i.e. identical to mode 0 except for
        // the extra warning line.
        assert_eq!(
            rc_c,
            diff_gotomach_rc(8, 5, 0, 1500),
            "invalid mode={mode} must behave like mode 0"
        );
    }

    // Every valid mode must NOT warn.
    for &mode in VALID_MODES {
        let g = p.c.gotomach;
        let (_rc, out) = capture_stdout(|| unsafe { g(8, 5, mode, 1500) });
        assert!(
            !String::from_utf8_lossy(&out).contains("Invalid mode"),
            "mode={mode} must not warn"
        );
    }

    // Randomized out-of-range enum values across the full int domain.
    for _ in 0..5_000 {
        let mode = loop {
            let m = rng.next_i32();
            if !(0..=2).contains(&m) {
                break m;
            }
        };
        let it = rng.range_i32(0, 128);
        let seed = rng.range_i32(0, 65535);
        let threshold = rng.next_i32();
        let rc = diff_gotomach_rc(it, seed, mode, threshold);
        assert_eq!(
            rc,
            diff_gotomach_rc(it, seed, 0, threshold),
            "invalid mode={mode} diverged from mode 0 for ({it}, {seed}, {threshold})"
        );
    }
}

// ===========================================================================
// ERRORS.md row 10 — count >= UINT16_MAX  =>  [WARNING] + break, NOT an error
// ===========================================================================

#[test]
fn err_row10_count_cap_warns_and_breaks_without_error() {
    let p = pair();
    for &mode in ALL_MODES {
        let (rc_c, out_c) = {
            let g = p.c.gotomach;
            capture_stdout(|| unsafe { g(65535, 3, mode, i32::MAX) })
        };
        let (rc_r, out_r) = {
            let g = p.rust.gotomach;
            capture_stdout(|| unsafe { g(65535, 3, mode, i32::MAX) })
        };
        assert_eq!(rc_c, rc_r, "rc mismatch at count cap, mode={mode}");
        assert_eq!(out_c, out_r, "stdout mismatch at count cap, mode={mode}");
        let s = String::from_utf8_lossy(&out_c);
        assert!(
            s.contains("[WARNING] Reached maximum count\n"),
            "C did not hit the count cap for mode={mode}: {s:?}"
        );
        // It is a warning, not an error: the sum is still computed and the
        // success message is still printed.
        assert!(
            s.contains("[INFO] Processing completed successfully\n"),
            "count cap must not abort processing, mode={mode}"
        );
        assert!(rc_c >= 0 || rc_c < -6, "count cap must not return an error code");
    }
    // 65534 iterations is one short of the cap: no warning.
    let g = p.c.gotomach;
    let (_rc, out) = capture_stdout(|| unsafe { g(65534, 3, 0, i32::MAX) });
    assert!(!String::from_utf8_lossy(&out).contains("Reached maximum count"));
    diff_gotomach(65534, 3, 0, i32::MAX);
}

// ===========================================================================
// Precedence / ordering guarantees from ERRORS.md
// ===========================================================================

#[test]
fn err_precedence_iterations_check_before_seed_check() {
    // Both invalid -> the iterations error (-1) wins.
    for it in [-1, i32::MIN, 65536, i32::MAX] {
        for seed in [-1, i32::MIN, 65536, i32::MAX] {
            expect_code(it, seed, 0, 0, -1);
            expect_code(it, seed, 9999, 0, -1);
        }
    }
}

#[test]
fn err_precedence_range_checks_before_mode_switch() {
    let p = pair();
    for &(it, seed, expect) in &[
        (-1i32, 0i32, -1i32),
        (65536, 0, -1),
        (8, -1, -2),
        (8, 65536, -2),
    ] {
        for &mode in INVALID_MODES {
            let g = p.c.gotomach;
            let (rc_c, out_c) = capture_stdout(|| unsafe { g(it, seed, mode, 0) });
            let gr = p.rust.gotomach;
            let (rc_r, out_r) = capture_stdout(|| unsafe { gr(it, seed, mode, 0) });
            assert_eq!(rc_c, expect);
            assert_eq!(rc_c, rc_r);
            assert_eq!(out_c, out_r);
            assert!(
                !String::from_utf8_lossy(&out_c).contains("Invalid mode"),
                "range check must short-circuit before the mode switch \
                 (it={it}, seed={seed}, mode={mode})"
            );
        }
    }
}

#[test]
fn err_threshold_is_never_validated() {
    // Every int is a legal threshold; INT_MIN appends nothing (sum 0).
    for &mode in ALL_MODES {
        assert_eq!(diff_gotomach_rc(64, 1, mode, i32::MIN), 0);
        assert_eq!(diff_gotomach_rc(64, 1, mode, i32::MIN + 1), 0);
    }
    let mut rng = Rng::new(0xE0FF);
    for _ in 0..5_000 {
        let rc = diff_gotomach_rc(rng.range_i32(0, 64), rng.range_i32(0, 65535), rng.range_i32(-2, 4), rng.next_i32());
        assert!(rc >= -2 || rc == 0, "unexpected rc {rc} from a valid call");
    }
}

// ===========================================================================
// Generic FFI-boundary cases: pointers, zero/oversized lengths, one-past-range
// ===========================================================================

#[test]
fn err_generic_helper_pointer_arguments() {
    // `unused_context` is `(void)`-cast away, so NULL and arbitrary non-null
    // pointers must both be accepted and produce identical results.
    let mut local: i64 = 0x1234_5678_9ABC_DEF0u64 as i64;
    let real_ptr = (&mut local) as *mut i64 as *mut c_void;
    let ptrs: [*mut c_void; 5] = [
        std::ptr::null_mut(),
        1 as *mut c_void,
        usize::MAX as *mut c_void,
        0xDEAD_BEEFusize as *mut c_void,
        real_ptr,
    ];
    for name in ["process_value", "double_value", "triple_value"] {
        for &p in &ptrs {
            for v in [i32::MIN, -1, 0, 1, 12345, i32::MAX] {
                diff_op(name, v, 0, p);
                diff_op(name, v, i32::MIN, p);
                diff_op(name, v, i32::MAX, p);
            }
        }
    }
    assert_eq!(local, 0x1234_5678_9ABC_DEF0u64 as i64, "context must be untouched");
}

#[test]
fn err_generic_zero_and_oversized_lengths() {
    for &mode in ALL_MODES {
        // zero length
        assert_eq!(diff_gotomach(0, 0, mode, i32::MAX), 0);
        // exactly at the limit
        diff_gotomach_rc(65535, 0, mode, i32::MIN);
        // one past the limit, and grossly oversized
        expect_code(65536, 0, mode, 0, -1);
        expect_code(i32::MAX, 0, mode, 0, -1);
    }
}

#[test]
fn err_generic_one_step_past_documented_ranges() {
    for &mode in ALL_MODES {
        // iterations: -1 | 0 | 65535 | 65536
        expect_code(-1, 0, mode, 0, -1);
        diff_gotomach(0, 0, mode, 0);
        diff_gotomach_rc(65535, 0, mode, 0);
        expect_code(65536, 0, mode, 0, -1);
        // seed: -1 | 0 | 65535 | 65536
        expect_code(8, -1, mode, 0, -2);
        diff_gotomach(8, 0, mode, 0);
        diff_gotomach(8, 65535, mode, 0);
        expect_code(8, 65536, mode, 0, -2);
    }
}

#[test]
fn err_generic_out_of_range_enum_values_exhaustive_near_valid() {
    // Dense sweep around the valid switch arms, where an off-by-one in the
    // match would show up.
    for mode in -8..=12 {
        let expected_like_mode_0 = !(0..=2).contains(&mode);
        let rc = diff_gotomach(16, 9, mode, 1500);
        if expected_like_mode_0 {
            assert_eq!(rc, diff_gotomach_rc(16, 9, 0, 1500), "mode={mode}");
        }
    }
    // The extreme int values as enum inputs.
    for mode in [i32::MIN, i32::MIN + 1, -1, 3, i32::MAX - 1, i32::MAX] {
        let rc = diff_gotomach(16, 9, mode, 1500);
        assert_eq!(rc, diff_gotomach_rc(16, 9, 0, 1500), "mode={mode}");
    }
}
