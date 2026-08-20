//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Both implementations are called through their `.so` exports and must return
//! the SAME error sentinel (not merely "both failed") and emit the same log.

mod common;

use common::{assert_error_row, c_impl, capture_stdout, diff_gotomach_batch, rust_impl, Args, Rng};

const INT_MIN: i32 = i32::MIN;
const INT_MAX: i32 = i32::MAX;

const LOG_START: &str = "[INFO] Starting gotomach function";
const LOG_BAD_ITER: &str = "[ERROR] Invalid iteration count";
const LOG_BAD_SEED: &str = "[ERROR] Invalid seed value";
const LOG_BAD_MODE: &str = "[WARNING] Invalid mode, using default";
const LOG_MAX_COUNT: &str = "[WARNING] Reached maximum count";
const LOG_DONE: &str = "[INFO] Processing completed successfully";

// ---------------------------------------------------------------------------
// Row 1 — iterations < 0  => -1
// ---------------------------------------------------------------------------
#[test]
fn err_01_iterations_negative() {
    let mut rng = Rng::new(0xE001);
    let mut cases: Vec<i32> = vec![-1, -2, -10, -65535, -65536, INT_MIN, INT_MIN + 1];
    for _ in 0..200 {
        cases.push(rng.range_i32(INT_MIN, -1));
    }
    for it in cases {
        assert_error_row(
            "ERRORS row 1",
            Args::new(it, 0, 0, INT_MAX),
            -1,
            &[LOG_START, LOG_BAD_ITER],
        );
    }
}

// ---------------------------------------------------------------------------
// Row 2 — iterations > UINT16_MAX  => -1
// ---------------------------------------------------------------------------
#[test]
fn err_02_iterations_too_large() {
    let mut rng = Rng::new(0xE002);
    let mut cases: Vec<i32> = vec![65536, 65537, 100_000, 1 << 20, INT_MAX - 1, INT_MAX];
    for _ in 0..200 {
        cases.push(rng.range_i32(65536, INT_MAX));
    }
    for it in cases {
        assert_error_row(
            "ERRORS row 2",
            Args::new(it, 12345, 1, 1000),
            -1,
            &[LOG_START, LOG_BAD_ITER],
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 — seed < 0  => -2   (iterations valid)
// ---------------------------------------------------------------------------
#[test]
fn err_03_seed_negative() {
    let mut rng = Rng::new(0xE003);
    let mut cases: Vec<i32> = vec![-1, -2, -1000, -65535, -65536, INT_MIN, INT_MIN + 1];
    for _ in 0..200 {
        cases.push(rng.range_i32(INT_MIN, -1));
    }
    for sd in cases {
        let iterations = rng.range_i32(0, 65535);
        assert_error_row(
            "ERRORS row 3",
            Args::new(iterations, sd, 2, 0),
            -2,
            &[LOG_START, LOG_BAD_SEED],
        );
    }
}

// ---------------------------------------------------------------------------
// Row 4 — seed > UINT16_MAX  => -2
// ---------------------------------------------------------------------------
#[test]
fn err_04_seed_too_large() {
    let mut rng = Rng::new(0xE004);
    let mut cases: Vec<i32> = vec![65536, 65537, 70_000, 1 << 24, INT_MAX - 1, INT_MAX];
    for _ in 0..200 {
        cases.push(rng.range_i32(65536, INT_MAX));
    }
    for sd in cases {
        let iterations = rng.range_i32(0, 4096);
        assert_error_row(
            "ERRORS row 4",
            Args::new(iterations, sd, 0, INT_MAX),
            -2,
            &[LOG_START, LOG_BAD_SEED],
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 5/6/7/8 (+12/13/15/17) — the malloc-failure and invariant-violation
// branches. They are unreachable through the public API; assert that BOTH
// implementations agree on that (never -3/-4/-5/-6) over a saturating sweep.
// ---------------------------------------------------------------------------
#[test]
fn err_05_to_08_unreachable_sentinels_never_observed() {
    let mut rng = Rng::new(0xE05608);
    let mut inputs = Vec::new();
    // whole valid domain corners + random interior
    for &iterations in &[0i32, 1, 2, 3, 17, 255, 1024, 4095, 65534, 65535] {
        for &seed in &[0i32, 1, 999, 1000, 32768, 65534, 65535] {
            for &mode in &[-7i32, 0, 1, 2, 3, INT_MIN, INT_MAX] {
                for &threshold in &[INT_MIN, -1, 0, 1, 1000, 3000, INT_MAX] {
                    inputs.push(Args::new(iterations, seed, mode, threshold));
                }
            }
        }
    }
    for _ in 0..4000 {
        inputs.push(Args::new(
            rng.range_i32(0, 2048),
            rng.range_i32(0, 65535),
            rng.next_i32(),
            rng.next_i32(),
        ));
    }

    let c = c_impl();
    let r = rust_impl();
    let (c_ret, _) = capture_stdout(|| {
        inputs
            .iter()
            .map(|a| unsafe { (c.gotomach)(a.iterations, a.seed, a.mode, a.threshold) })
            .collect::<Vec<i32>>()
    });
    let (r_ret, _) = capture_stdout(|| {
        inputs
            .iter()
            .map(|a| unsafe { (r.gotomach)(a.iterations, a.seed, a.mode, a.threshold) })
            .collect::<Vec<i32>>()
    });
    for (i, a) in inputs.iter().enumerate() {
        assert_eq!(c_ret[i], r_ret[i], "[ERRORS rows 5-8] mismatch for {a:?}");
        for bad in [-3, -4, -5, -6] {
            assert_ne!(
                c_ret[i], bad,
                "[ERRORS rows 5-8] C unexpectedly returned {bad} for {a:?}"
            );
            assert_ne!(
                r_ret[i], bad,
                "[ERRORS rows 5-8] Rust unexpectedly returned {bad} for {a:?} (C returned {})",
                c_ret[i]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — both iterations and seed invalid: iterations guard wins (-1, not -2)
// ---------------------------------------------------------------------------
#[test]
fn err_09_precedence_iterations_before_seed() {
    for &it in &[-1i32, -1000, INT_MIN, 65536, INT_MAX] {
        for &sd in &[-1i32, -1000, INT_MIN, 65536, INT_MAX] {
            assert_error_row(
                "ERRORS row 9",
                Args::new(it, sd, 0, 0),
                -1,
                &[LOG_START, LOG_BAD_ITER],
            );
        }
    }
    // and the log must NOT mention the seed
    let (_, out) = common::call_gotomach(c_impl(), Args::new(-5, -5, 0, 0));
    let text = String::from_utf8_lossy(&out).to_string();
    assert!(!text.contains(LOG_BAD_SEED), "unexpected seed log: {text:?}");
}

// ---------------------------------------------------------------------------
// Row 10 — invalid `mode` is NOT an error: warn + fall back to process_value
// ---------------------------------------------------------------------------
#[test]
fn err_10_invalid_mode_is_not_an_error() {
    let mut rng = Rng::new(0xE010);
    let mut modes: Vec<i32> = vec![-1, 3, 4, 5, 255, -255, 65536, -65536, INT_MIN, INT_MAX];
    for _ in 0..64 {
        let m = rng.next_i32();
        if !(0..=2).contains(&m) {
            modes.push(m);
        }
    }
    for m in modes {
        // Same arguments, mode 0 vs invalid mode -> identical numeric result,
        // because `default:` selects process_value, plus one extra WARNING line.
        let a_bad = Args::new(32, 7, m, INT_MAX);
        let a_ref = Args::new(32, 7, 0, INT_MAX);
        let (c_bad, c_bad_out) = common::call_gotomach(c_impl(), a_bad);
        let (r_bad, r_bad_out) = common::call_gotomach(rust_impl(), a_bad);
        let (c_ref, _) = common::call_gotomach(c_impl(), a_ref);
        assert_eq!(c_bad, r_bad, "[ERRORS row 10] mismatch for mode={m}");
        assert_eq!(
            c_bad, c_ref,
            "[ERRORS row 10] C: default: branch must behave like mode 0 (mode={m})"
        );
        assert_eq!(
            String::from_utf8_lossy(&c_bad_out),
            String::from_utf8_lossy(&r_bad_out),
            "[ERRORS row 10] log mismatch for mode={m}"
        );
        let text = String::from_utf8_lossy(&c_bad_out).to_string();
        assert!(text.contains(LOG_BAD_MODE), "missing warning: {text:?}");
        assert!(text.contains(LOG_DONE), "missing completion: {text:?}");
    }
    // ...and modes 0/1/2 must NOT warn.
    for m in [0i32, 1, 2] {
        let (_, out) = common::call_gotomach(c_impl(), Args::new(4, 3, m, INT_MAX));
        let (_, rout) = common::call_gotomach(rust_impl(), Args::new(4, 3, m, INT_MAX));
        assert!(!String::from_utf8_lossy(&out).contains(LOG_BAD_MODE));
        assert!(!String::from_utf8_lossy(&rout).contains(LOG_BAD_MODE));
    }
}

// ---------------------------------------------------------------------------
// Row 11 — state->count >= UINT16_MAX  =>  "[WARNING] Reached maximum count"
// ---------------------------------------------------------------------------
#[test]
fn err_11_reached_maximum_count() {
    for mode in [0i32, 1, 2, 9] {
        for seed in [0i32, 1, 999, 65535] {
            let a = Args::new(65535, seed, mode, INT_MAX);
            let (c_ret, c_out) = common::call_gotomach(c_impl(), a);
            let (r_ret, r_out) = common::call_gotomach(rust_impl(), a);
            assert_eq!(c_ret, r_ret, "[ERRORS row 11] mismatch for {a:?}");
            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out),
                "[ERRORS row 11] log mismatch for {a:?}"
            );
            let text = String::from_utf8_lossy(&c_out).to_string();
            assert!(
                text.contains(LOG_MAX_COUNT),
                "[ERRORS row 11] expected saturation warning for {a:?}, got {text:?}"
            );
        }
    }
    // One below the limit must NOT warn.
    for mode in [0i32, 1, 2, 9] {
        let a = Args::new(65534, 5, mode, INT_MAX);
        let (c_ret, c_out) = common::call_gotomach(c_impl(), a);
        let (r_ret, r_out) = common::call_gotomach(rust_impl(), a);
        assert_eq!(c_ret, r_ret);
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert!(!String::from_utf8_lossy(&c_out).contains(LOG_MAX_COUNT));
        assert!(!String::from_utf8_lossy(&r_out).contains(LOG_MAX_COUNT));
    }
}

// ---------------------------------------------------------------------------
// Rows 14/16 — cleanup: reaching `cleanup:` with state == NULL and
// temp_buffer == NULL must be a no-op (no crash, no double free). Hammer it.
// ---------------------------------------------------------------------------
#[test]
fn err_14_cleanup_null_state() {
    let mut inputs = Vec::new();
    for i in 0..2000 {
        // alternate the -1 and -2 exits, both of which reach cleanup with
        // state == NULL and temp_buffer == NULL
        if i % 2 == 0 {
            inputs.push(Args::new(-1 - i, 0, i, i));
        } else {
            inputs.push(Args::new(i % 65536, -i, -i, i));
        }
    }
    diff_gotomach_batch("ERRORS rows 14/16", &inputs);
}

// ---------------------------------------------------------------------------
// G1 — out-of-range "enum" values for `mode` crossing the FFI boundary.
// ---------------------------------------------------------------------------
#[test]
fn g1_mode_out_of_range_enum_values() {
    let mut rng = Rng::new(0x6001);
    let mut modes: Vec<i32> = vec![
        INT_MIN,
        INT_MIN + 1,
        -3,
        -2,
        -1,
        3,
        4,
        127,
        128,
        255,
        256,
        32767,
        32768,
        65535,
        65536,
        INT_MAX - 1,
        INT_MAX,
    ];
    for _ in 0..500 {
        modes.push(rng.next_i32());
    }
    let mut inputs = Vec::new();
    for m in modes {
        for &threshold in &[INT_MIN, 0, 1000, INT_MAX] {
            inputs.push(Args::new(37, 4242, m, threshold));
        }
    }
    diff_gotomach_batch("ERRORS G1", &inputs);
}

// ---------------------------------------------------------------------------
// G2 / G3 — boundary ladders for iterations and seed.
// ---------------------------------------------------------------------------
#[test]
fn g2_g3_iterations_and_seed_ladders() {
    let ladder = [
        INT_MIN,
        INT_MIN + 1,
        -65537,
        -65536,
        -2,
        -1,
        0,
        1,
        2,
        65534,
        65535,
        65536,
        65537,
        INT_MAX - 1,
        INT_MAX,
    ];
    let mut inputs = Vec::new();
    for &it in &ladder {
        for &sd in &ladder {
            for &mode in &[0i32, 1, 2, -9] {
                for &threshold in &[INT_MIN, 0, 1010, INT_MAX] {
                    // keep the runtime bounded: only run the huge-but-valid
                    // iteration counts for a single (mode, threshold) pair
                    if (it == 65534 || it == 65535) && !(mode == 0 && threshold == INT_MAX) {
                        continue;
                    }
                    inputs.push(Args::new(it, sd, mode, threshold));
                }
            }
        }
    }
    diff_gotomach_batch("ERRORS G2/G3", &inputs);
}

// ---------------------------------------------------------------------------
// G4 — threshold ladder, including exact-equality with produced values.
// ---------------------------------------------------------------------------
#[test]
fn g4_threshold_ladder() {
    let mut inputs = Vec::new();
    let thresholds: Vec<i32> = {
        let mut v = vec![
            INT_MIN,
            INT_MIN + 1,
            -2,
            -1,
            0,
            1,
            2,
            9,
            10,
            11,
            999,
            1000,
            1001,
            1008,
            1009,
            1010,
            1996,
            1998,
            2000,
            2994,
            2997,
            3000,
            65535,
            65545,
            196_605,
            INT_MAX - 1,
            INT_MAX,
        ];
        v.sort_unstable();
        v.dedup();
        v
    };
    for &threshold in &thresholds {
        for &mode in &[0i32, 1, 2, -1] {
            for &seed in &[0i32, 1, 499, 500, 665, 666, 999, 1000, 65535] {
                for &iterations in &[0i32, 1, 2, 3, 40, 257] {
                    inputs.push(Args::new(iterations, seed, mode, threshold));
                }
            }
        }
    }
    diff_gotomach_batch("ERRORS G4", &inputs);
}

// ---------------------------------------------------------------------------
// G5 — iterations == 0 (malloc(0)): must NOT be treated as an allocation
// failure by either side; both must return 0 with the success log.
// ---------------------------------------------------------------------------
#[test]
fn g5_zero_iterations_malloc_zero() {
    for &mode in &[INT_MIN, -1, 0, 1, 2, 3, INT_MAX] {
        for &seed in &[0i32, 1, 65535] {
            for &threshold in &[INT_MIN, 0, 1, INT_MAX] {
                let a = Args::new(0, seed, mode, threshold);
                let (c_ret, c_out) = common::call_gotomach(c_impl(), a);
                let (r_ret, r_out) = common::call_gotomach(rust_impl(), a);
                assert_eq!(c_ret, 0, "[ERRORS G5] C returned {c_ret} for {a:?}");
                assert_eq!(r_ret, c_ret, "[ERRORS G5] mismatch for {a:?}");
                assert_eq!(
                    String::from_utf8_lossy(&c_out),
                    String::from_utf8_lossy(&r_out),
                    "[ERRORS G5] log mismatch for {a:?}"
                );
                let text = String::from_utf8_lossy(&c_out).to_string();
                assert!(text.contains(LOG_DONE), "[ERRORS G5] {text:?}");
            }
        }
    }
    // Repeat many times: a bogus NULL check on malloc(0) would show up as -3/-4.
    let inputs: Vec<Args> = (0..3000)
        .map(|i| Args::new(0, i % 65536, i - 1500, i * 7 - 3))
        .collect();
    diff_gotomach_batch("ERRORS G5 (bulk)", &inputs);
}

// ---------------------------------------------------------------------------
// G6 / G7 — the `operation_fn`s: unused_param / unused_context must be ignored,
// including NULL and deliberately non-dereferenceable garbage pointers.
// ---------------------------------------------------------------------------
#[test]
fn g6_g7_ops_ignore_unused_params_and_pointers() {
    let mut rng = Rng::new(0x6767);
    let ctxs: [usize; 7] = [
        0,
        1,
        0xdead_beef,
        usize::MAX,
        usize::MAX - 7,
        0x1000,
        0x7fff_ffff_ffff,
    ];
    let values: Vec<i32> = {
        let mut v = vec![
            INT_MIN,
            INT_MIN + 1,
            INT_MIN / 3,
            INT_MIN / 2,
            -10,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            10,
            INT_MAX / 3,
            INT_MAX / 2,
            INT_MAX - 10,
            INT_MAX - 9,
            INT_MAX - 1,
            INT_MAX,
        ];
        for _ in 0..300 {
            v.push(rng.next_i32());
        }
        v
    };
    for which in ["process_value", "double_value", "triple_value"] {
        let mut inputs = Vec::new();
        for &v in &values {
            for &ctx in &ctxs {
                inputs.push((v, rng.next_i32(), ctx));
                inputs.push((v, 0, ctx));
            }
        }
        common::diff_op_batch("ERRORS G6/G7", which, &inputs);
    }
}
