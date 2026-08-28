//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`s
//! through their exported symbols, and asserts they return the SAME sentinel
//! (`-1`, `-2`, `-3`, `-4`, `-5`, `-6`) — not merely "both failed somehow" —
//! and emit the same rejection log bytes.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// `-3` / `-4` (allocation failure) and `-5` / `-6` (impossible state) must
/// never be produced for any input reachable across the FFI boundary.
const UNREACHABLE_SENTINELS: [c_int; 4] = [-3, -4, -5, -6];

fn assert_not_unreachable(got: c_int, a: Args) {
    assert!(
        !UNREACHABLE_SENTINELS.contains(&got),
        "{a} produced the statically-unreachable sentinel {got}"
    );
}

// ===========================================================================
// Row 1 — `iterations < 0` -> -1
// ===========================================================================

#[test]
fn err_01_iterations_negative() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 101);
    let pinned = [-1, -2, -3, -10, -100, -65_535, -65_536, c_int::MIN + 1, c_int::MIN];
    for &it in &pinned {
        for &sd in &[0, 1, 65_535, -1, 65_536, c_int::MIN, c_int::MAX] {
            for &md in &[0, 1, 2, 3, -1, c_int::MIN, c_int::MAX] {
                let a = args(it, sd, md, rng.i32_interesting());
                let (got, log) = h.assert_gotomach_logged(a);
                assert_eq!(got, -1, "expected -1 for {a}");
                assert_eq!(
                    log,
                    "[INFO] Starting gotomach function\n[ERROR] Invalid iteration count\n",
                    "wrong rejection log for {a}"
                );
            }
        }
    }
    for _ in 0..20_000 {
        let a = args(
            rng.range(c_int::MIN, -1),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        assert_eq!(h.assert_gotomach_ret(a), -1, "expected -1 for {a}");
    }
}

// ===========================================================================
// Row 2 — `iterations > UINT16_MAX` -> -1
// ===========================================================================

#[test]
fn err_02_iterations_above_uint16_max() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 102);
    let pinned = [65_536, 65_537, 70_000, 100_000, 1 << 20, c_int::MAX - 1, c_int::MAX];
    for &it in &pinned {
        for &sd in &[0, 1, 65_535, -1, 65_536] {
            let a = args(it, sd, 0, rng.i32_interesting());
            let (got, log) = h.assert_gotomach_logged(a);
            assert_eq!(got, -1, "expected -1 for {a}");
            assert_eq!(
                log,
                "[INFO] Starting gotomach function\n[ERROR] Invalid iteration count\n",
                "wrong rejection log for {a}"
            );
        }
    }
    for _ in 0..20_000 {
        let a = args(
            rng.range(65_536, c_int::MAX),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        assert_eq!(h.assert_gotomach_ret(a), -1, "expected -1 for {a}");
    }
}

// ===========================================================================
// Row 3 — `seed < 0` (with valid `iterations`) -> -2
// ===========================================================================

#[test]
fn err_03_seed_negative() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 103);
    let pinned = [-1, -2, -100, -65_535, -65_536, c_int::MIN + 1, c_int::MIN];
    for &sd in &pinned {
        for &it in &[0, 1, 2, 64, 65_534, 65_535] {
            for &md in &[0, 1, 2, 9, -9] {
                let a = args(it, sd, md, rng.i32_interesting());
                let (got, log) = h.assert_gotomach_logged(a);
                assert_eq!(got, -2, "expected -2 for {a}");
                assert_eq!(
                    log,
                    "[INFO] Starting gotomach function\n[ERROR] Invalid seed value\n",
                    "wrong rejection log for {a}"
                );
            }
        }
    }
    for _ in 0..20_000 {
        let a = args(
            rng.range(0, 65_535),
            rng.range(c_int::MIN, -1),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        assert_eq!(h.assert_gotomach_ret(a), -2, "expected -2 for {a}");
    }
}

// ===========================================================================
// Row 4 — `seed > UINT16_MAX` (with valid `iterations`) -> -2
// ===========================================================================

#[test]
fn err_04_seed_above_uint16_max() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 104);
    let pinned = [65_536, 65_537, 100_000, 1 << 24, c_int::MAX - 1, c_int::MAX];
    for &sd in &pinned {
        for &it in &[0, 1, 2, 64, 65_534, 65_535] {
            let a = args(it, sd, 0, rng.i32_interesting());
            let (got, log) = h.assert_gotomach_logged(a);
            assert_eq!(got, -2, "expected -2 for {a}");
            assert_eq!(
                log,
                "[INFO] Starting gotomach function\n[ERROR] Invalid seed value\n",
                "wrong rejection log for {a}"
            );
        }
    }
    for _ in 0..20_000 {
        let a = args(
            rng.range(0, 65_535),
            rng.range(65_536, c_int::MAX),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        assert_eq!(h.assert_gotomach_ret(a), -2, "expected -2 for {a}");
    }
}

// ===========================================================================
// Row 5 — check ordering: `iterations` is validated before `seed`
// ===========================================================================

#[test]
fn err_05_check_ordering_iterations_first() {
    let mut h = harness();
    let bad_iters = [-1, c_int::MIN, 65_536, c_int::MAX];
    let bad_seeds = [-1, c_int::MIN, 65_536, c_int::MAX];
    for &it in &bad_iters {
        for &sd in &bad_seeds {
            let a = args(it, sd, 0, 0);
            let (got, log) = h.assert_gotomach_logged(a);
            assert_eq!(
                got, -1,
                "both arguments invalid: `iterations` is checked first, so {a} must yield -1"
            );
            assert_eq!(
                log,
                "[INFO] Starting gotomach function\n[ERROR] Invalid iteration count\n",
                "the seed check must never be reached for {a}"
            );
        }
    }
    // Mirror: valid `iterations` + invalid `seed` really does reach the second
    // check, so the ordering assertion above is meaningful.
    for &sd in &bad_seeds {
        let a = args(4, sd, 0, 0);
        assert_eq!(h.assert_gotomach_ret(a), -2);
    }
}

// ===========================================================================
// Rows 6, 7, 8, 11, 12 — allocation failure sentinels are never produced for
// any in-range request (the largest possible is 65 535 * 4 bytes), and
// `malloc(0)` (iterations == 0) is not treated as a failure either.
// ===========================================================================

#[test]
fn err_06_07_08_alloc_never_fails_in_range() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 106);

    // Both ends of the allocation range, every mode class.
    for &it in &[0, 1, 2, 65_534, 65_535] {
        for &md in &[0, 1, 2, 3, -1, c_int::MIN, c_int::MAX] {
            for &thr in &[c_int::MIN, 0, 1_000, c_int::MAX] {
                let a = args(it, rng.range(0, 65_535), md, thr);
                let (got, log) = h.assert_gotomach_logged(a);
                assert_ne!(got, -3, "init_processor must not fail for {a}");
                assert_ne!(got, -4, "temp_buffer malloc must not fail for {a}");
                assert!(
                    !log.contains("Failed to initialize processor"),
                    "{a}: {log:?}"
                );
                assert!(
                    !log.contains("Failed to allocate temporary buffer"),
                    "{a}: {log:?}"
                );
                assert!(
                    log.contains("[INFO] Processing completed successfully"),
                    "in-range request must reach the success path for {a}: {log:?}"
                );
            }
        }
    }

    // Repeated max-size allocations: catches a leak that would eventually make
    // one implementation start returning -3/-4 while the other does not.
    for _ in 0..300 {
        let a = args(65_535, rng.range(0, 65_535), rng.range(0, 3), c_int::MAX);
        let got = h.assert_gotomach_ret(a);
        assert_not_unreachable(got, a);
    }

    // Randomized in-range sweep.
    for _ in 0..20_000 {
        let a = args(
            rng.range(0, 400),
            rng.range(0, 65_535),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        let got = h.assert_gotomach_ret(a);
        assert_not_unreachable(got, a);
    }
}

// ===========================================================================
// Rows 9, 10 — `-5` (status == 0) and `-6` (count >= capacity) are statically
// unreachable and must be unreachable in the Rust too, for every input.
// ===========================================================================

#[test]
fn err_09_10_status_and_state_never_invalid() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 109);

    // Exhaustive over the shapes where `count` could conceivably catch up with
    // `capacity`: append-everything (`threshold == INT_MAX`) at every small
    // `iterations`, which is the only way to drive `count == i`.
    for it in 0..=512 {
        for md in [0, 1, 2, 4242] {
            let a = args(it, rng.range(0, 65_535), md, c_int::MAX);
            let (got, log) = h.assert_gotomach_logged(a);
            assert_ne!(got, -5, "status must never be 0 for {a}");
            assert_ne!(got, -6, "state must never go invalid for {a}");
            assert!(!log.contains("Invalid state status"), "{a}: {log:?}");
            assert!(!log.contains("State became invalid"), "{a}: {log:?}");
        }
    }
    // And at the top of the valid range.
    for it in [65_530, 65_531, 65_532, 65_533, 65_534, 65_535] {
        for md in [0, 1, 2, -7] {
            let a = args(it, rng.range(0, 65_535), md, c_int::MAX);
            let got = h.assert_gotomach_ret(a);
            assert_ne!(got, -5, "{a}");
            assert_ne!(got, -6, "{a}");
        }
    }
    // Fully random sweep: neither sentinel may ever show up.
    for _ in 0..50_000 {
        let a = args(
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        let got = h.assert_gotomach_ret(a);
        assert_ne!(got, -5, "{a}");
        assert_ne!(got, -6, "{a}");
    }
}

// ===========================================================================
// Rows 13, 14, 15 — the `cleanup:` NULL guards
// ===========================================================================

#[test]
fn err_13_14_15_cleanup_null_guards() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 113);

    // Rows 13 & 15: every `-1` / `-2` rejection jumps to `cleanup:` with both
    // `state == NULL` and `temp_buffer == NULL`. Hammer that path so a bogus
    // free/double-free in either implementation would show up as a crash or a
    // divergent return value.
    for _ in 0..50_000 {
        let a = if rng.bool() {
            args(
                rng.range(c_int::MIN, -1),
                rng.i32_interesting(),
                rng.i32_interesting(),
                rng.i32_interesting(),
            )
        } else {
            args(
                rng.range(0, 65_535),
                if rng.bool() {
                    rng.range(c_int::MIN, -1)
                } else {
                    rng.range(65_536, c_int::MAX)
                },
                rng.i32_interesting(),
                rng.i32_interesting(),
            )
        };
        let got = h.assert_gotomach_ret(a);
        assert!(got == -1 || got == -2, "{a} -> {got}, expected -1 or -2");
    }

    // Row 14: `state->results == NULL` cannot be reached from the FFI surface
    // (`init_processor` returns NULL rather than a state with a NULL array), so
    // the inner guard is exercised only via the ordinary success path — where
    // `results` is always non-NULL. Confirm the success path is clean and
    // repeatable, which is what the guard protects.
    for _ in 0..2_000 {
        let a = args(
            rng.range(0, 256),
            rng.range(0, 65_535),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        let first = h.assert_gotomach(a);
        let second = h.assert_gotomach(a);
        assert_eq!(first, second, "cleanup must not corrupt state for {a}");
    }
}

// ===========================================================================
// Row 16 — `default:` arm of the mode switch
// ===========================================================================

#[test]
fn err_16_invalid_mode_default_branch() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 116);
    for _ in 0..5_000 {
        let bad = rng.bad_mode();
        let it = rng.range(0, 200);
        let sd = rng.range(0, 65_535);
        let thr = rng.i32_interesting();

        let a_bad = args(it, sd, bad, thr);
        let (got_bad, log) = h.assert_gotomach_logged(a_bad);
        assert!(
            log.contains("[WARNING] Invalid mode, using default"),
            "{a_bad} must warn about the invalid mode, got {log:?}"
        );

        // `default:` falls back to `process_value`, i.e. numerically identical
        // to `mode == 0` — with the extra warning line as the only difference.
        let a_zero = args(it, sd, 0, thr);
        let (got_zero, log_zero) = h.assert_gotomach_logged(a_zero);
        assert_eq!(
            got_bad, got_zero,
            "{a_bad} must behave like {a_zero} (default -> process_value)"
        );
        assert!(
            !log_zero.contains("Invalid mode"),
            "mode 0 must not warn, got {log_zero:?}"
        );
        assert_eq!(
            log_bad_without_warning(&log),
            log_zero,
            "{a_bad}: removing the warning line must leave mode 0's log exactly"
        );
    }
}

fn log_bad_without_warning(log: &str) -> String {
    log.replace("[WARNING] Invalid mode, using default\n", "")
}

// ===========================================================================
// Row 17 — `count >= UINT16_MAX` warning + break, then still sums
// ===========================================================================

#[test]
fn err_17_reached_maximum_count() {
    let mut h = harness();
    // Saturation needs `iterations == 65535` and every value appended.
    for md in [0, 1, 2, 31_337] {
        for sd in [0, 1, 500, 999, 1_000, 65_535] {
            let a = args(65_535, sd, md, c_int::MAX);
            let (got, log) = h.assert_gotomach_logged(a);
            assert!(
                log.contains("[WARNING] Reached maximum count"),
                "{a} must saturate, got {log:?}"
            );
            // Saturation is not an error: the sum is returned, never a
            // sentinel. (`seed == 0` with `double_value` legitimately sums to
            // 0, so the assertion is `>= 0`, not `> 0`.)
            assert!(
                got >= 0,
                "{a} must still return the sum, not a sentinel, got {got}"
            );
            assert_not_unreachable(got, a);
            assert!(
                log.ends_with("[INFO] Processing completed successfully\n"),
                "{a} must not be treated as an error, got {log:?}"
            );
        }
    }
    // One element short of saturation: warning must be absent.
    for md in [0, 1, 2, 31_337] {
        for sd in [0, 1, 65_535] {
            for it in [65_534, 65_533] {
                let a = args(it, sd, md, c_int::MAX);
                let (_got, log) = h.assert_gotomach_logged(a);
                assert!(!log.contains("Reached maximum count"), "{a}: {log:?}");
            }
        }
    }
    // Saturation is impossible when some values are filtered out.
    let a = args(65_535, 1, 0, 500);
    let (_got, log) = h.assert_gotomach_logged(a);
    assert!(!log.contains("Reached maximum count"), "{a}: {log:?}");
}

// ===========================================================================
// Row 18 — `iterations == 0` -> `malloc(0)` is NOT an error
// ===========================================================================

#[test]
fn err_18_zero_iterations_malloc_zero() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 118);
    for _ in 0..5_000 {
        let a = args(0, rng.range(0, 65_535), rng.i32_interesting(), rng.i32_interesting());
        let (got, log) = h.assert_gotomach_logged(a);
        assert_eq!(got, 0, "{a} must succeed with an empty sum");
        assert!(
            log.ends_with("[INFO] Processing completed successfully\n"),
            "{a} must take the success path, got {log:?}"
        );
        assert_not_unreachable(got, a);
    }
    // Also with an invalid seed, to confirm ordering is unaffected.
    assert_eq!(h.assert_gotomach_ret(args(0, -1, 0, 0)), -2);
    assert_eq!(h.assert_gotomach_ret(args(0, 65_536, 0, 0)), -2);
}

// ===========================================================================
// Row 19 — `threshold` rejects every produced value
// ===========================================================================

#[test]
fn err_19_threshold_rejects_everything() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 119);
    // Every produced value is >= 0, so any `threshold <= 0` appends nothing.
    for thr in [c_int::MIN, c_int::MIN + 1, -1_000_000, -1, 0] {
        for _ in 0..1_000 {
            let a = args(rng.range(0, 400), rng.range(0, 65_535), rng.i32_interesting(), thr);
            let got = h.assert_gotomach(a);
            assert_eq!(got, 0, "{a} must append nothing and sum to 0");
        }
    }
    for thr in [c_int::MIN, -1, 0] {
        for md in [0, 1, 2, 77] {
            let a = args(65_535, 65_535, md, thr);
            let got = h.assert_gotomach(a);
            assert_eq!(got, 0, "{a} must append nothing and sum to 0");
        }
    }
}

// ===========================================================================
// Row 20 — range boundaries, one step past on every axis
// ===========================================================================

#[test]
fn err_20_range_boundaries() {
    let mut h = harness();
    let iters = [c_int::MIN, -2, -1, 0, 1, 65_534, 65_535, 65_536, 65_537, c_int::MAX];
    let seeds = [c_int::MIN, -2, -1, 0, 1, 65_534, 65_535, 65_536, 65_537, c_int::MAX];
    let modes = [c_int::MIN, -1, 0, 1, 2, 3, c_int::MAX];
    let thrs = [c_int::MIN, -1, 0, 1, 1_006, c_int::MAX];
    for &it in &iters {
        for &sd in &seeds {
            for &md in &modes {
                for &thr in &thrs {
                    let a = args(it, sd, md, thr);
                    let got = h.assert_gotomach_ret(a);
                    // Cross-check the sentinel against the C's own ordering.
                    let want_class = if it < 0 || it > 65_535 {
                        Some(-1)
                    } else if sd < 0 || sd > 65_535 {
                        Some(-2)
                    } else {
                        None
                    };
                    if let Some(w) = want_class {
                        assert_eq!(got, w, "{a} must be rejected with {w}");
                    } else {
                        assert_not_unreachable(got, a);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 21 — out-of-range "enum" values for `mode` crossing the FFI boundary
// ===========================================================================

#[test]
fn err_21_out_of_range_mode_enum() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 121);
    // A C `switch` on an `int` accepts any value; there is no valid-variant
    // check, so every one of these must land in `default:`.
    let modes = [
        c_int::MIN,
        c_int::MIN + 1,
        -2_147_483_647,
        -1_000_000,
        -3,
        -2,
        -1,
        3,
        4,
        5,
        99,
        1_000_000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for &md in &modes {
        for &it in &[0, 1, 2, 17, 256] {
            let sd = rng.range(0, 65_535);
            let thr = rng.i32_interesting();
            let a = args(it, sd, md, thr);
            let (got, log) = h.assert_gotomach_logged(a);
            if it > 0 {
                assert!(
                    log.contains("[WARNING] Invalid mode, using default"),
                    "{a} must take the default arm, got {log:?}"
                );
            }
            assert_eq!(
                got,
                h.assert_gotomach_ret(args(it, sd, 0, thr)),
                "{a} must match mode 0 numerically"
            );
        }
    }
    // Randomized: every non-{0,1,2} int must behave exactly like mode 0.
    for _ in 0..10_000 {
        let md = rng.bad_mode();
        let it = rng.range(0, 128);
        let sd = rng.range(0, 65_535);
        let thr = rng.i32_interesting();
        let bad = h.assert_gotomach_ret(args(it, sd, md, thr));
        let zero = h.assert_gotomach_ret(args(it, sd, 0, thr));
        assert_eq!(bad, zero, "mode={md} diverged from mode=0 (it={it} sd={sd} thr={thr})");
    }
}

// ===========================================================================
// Row 22 — extreme `threshold` values
// ===========================================================================

#[test]
fn err_22_extreme_thresholds() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 122);
    let thrs = [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        2,
        999,
        1_000,
        1_005,
        1_006,
        1_998,
        1_999,
        2_997,
        2_998,
        65_545,
        131_070,
        196_605,
        196_606,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for &thr in &thrs {
        for &md in &[0, 1, 2, -55] {
            for &sd in &[0, 1, 999, 1_000, 65_535] {
                for &it in &[0, 1, 2, 3, 64, 200] {
                    let a = args(it, sd, md, thr);
                    let got = h.assert_gotomach(a);
                    assert_not_unreachable(got, a);
                }
            }
        }
    }
    for _ in 0..10_000 {
        let a = args(rng.range(0, 300), rng.range(0, 65_535), rng.i32_interesting(), rng.pick(&thrs));
        h.assert_gotomach_ret(a);
    }
}

// ===========================================================================
// Row 23 — the callbacks' `(void)`-cast arguments, incl. NULL context
// ===========================================================================

#[test]
fn err_23_op_ignores_extra_args() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 123);
    let mut scratch = [0xABu8; 32];
    let ptrs: [*mut c_void; 5] = [
        std::ptr::null_mut(),
        1usize as *mut c_void,
        usize::MAX as *mut c_void,
        scratch.as_mut_ptr() as *mut c_void,
        (&mut scratch[31] as *mut u8) as *mut c_void,
    ];
    for which in Op::ALL {
        for &ctx in &ptrs {
            for &v in &[c_int::MIN, -1, 0, 1, 65_535, c_int::MAX] {
                for &unused in &[c_int::MIN, -1, 0, 1, c_int::MAX] {
                    let a = h.assert_op(which, v, unused, ctx);
                    // Args 2 and 3 must not influence the result at all.
                    let b = h.assert_op(which, v, 0, std::ptr::null_mut());
                    assert_eq!(
                        a,
                        b,
                        "{}({v}, {unused}, {ctx:p}) != {}({v}, 0, NULL)",
                        which.sym_name(),
                        which.sym_name()
                    );
                }
            }
        }
    }
    for _ in 0..10_000 {
        let v = rng.i32_interesting();
        let unused = rng.i32_interesting();
        let ctx = rng.pick(&ptrs);
        for which in Op::ALL {
            h.assert_op(which, v, unused, ctx);
        }
    }
}

// ===========================================================================
// Rows 24-26 — signed-overflow edges of the three leaf operations
// ===========================================================================

#[test]
fn err_24_process_value_overflow() {
    let mut h = harness();
    for v in [
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX - 9,
        c_int::MAX - 10,
        c_int::MAX - 11,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 9,
        c_int::MIN + 10,
        -10,
        -9,
        -1,
        0,
    ] {
        let got = h.assert_op(Op::Process, v, 0, std::ptr::null_mut());
        assert_eq!(got, v.wrapping_add(10), "process_value({v}) is not a wrapping add");
    }
}

#[test]
fn err_25_double_value_overflow() {
    let mut h = harness();
    for v in [
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX / 2,
        c_int::MAX / 2 + 1,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN / 2,
        c_int::MIN / 2 - 1,
        -1,
        0,
        1,
    ] {
        let got = h.assert_op(Op::Double, v, 0, std::ptr::null_mut());
        assert_eq!(got, v.wrapping_mul(2), "double_value({v}) is not a wrapping mul");
    }
}

#[test]
fn err_26_triple_value_overflow() {
    let mut h = harness();
    for v in [
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX / 3,
        c_int::MAX / 3 + 1,
        c_int::MAX / 3 + 2,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN / 3,
        c_int::MIN / 3 - 1,
        -1,
        0,
        1,
    ] {
        let got = h.assert_op(Op::Triple, v, 0, std::ptr::null_mut());
        assert_eq!(got, v.wrapping_mul(3), "triple_value({v}) is not a wrapping mul");
    }
}
