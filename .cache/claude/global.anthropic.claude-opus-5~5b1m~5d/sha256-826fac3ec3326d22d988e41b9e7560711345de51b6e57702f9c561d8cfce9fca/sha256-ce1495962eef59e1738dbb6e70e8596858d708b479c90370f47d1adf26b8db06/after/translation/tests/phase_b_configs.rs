//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH shared objects via `libloading` and drives them
//! through their exported C symbols only. Unless a row pins a value, inputs are
//! randomized with a fixed seed (`common::SEED`) so failures reproduce.
//!
//! Rows are ordered lowest-level first: the three `operation_fn` exports
//! (`process_value`, `double_value`, `triple_value`) before the composed
//! `gotomach` pipeline that calls them through a function pointer.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Sizing knobs. Kept modest for the rows that drive 65 535-iteration loops.
// ---------------------------------------------------------------------------
const N_OP: usize = 20_000; // per-row cases for the leaf operations
const N_SMALL: usize = 2_000; // per-row cases for small `iterations`
const N_BAND: usize = 1_500; // per-row cases for the partial-append band
const N_WIDE: usize = 300; // per-row cases with `iterations` up to 65 535
const N_SWEEP: usize = 200_000; // the full-`i32` robustness sweep

/// The four classes the `switch (mode)` in `lib.c:126-140` distinguishes.
#[derive(Copy, Clone, Debug)]
enum ModeClass {
    Zero,
    One,
    Two,
    Default,
}

impl ModeClass {
    const ALL: [ModeClass; 4] = [
        ModeClass::Zero,
        ModeClass::One,
        ModeClass::Two,
        ModeClass::Default,
    ];
    fn value(self, rng: &mut Rng) -> c_int {
        match self {
            ModeClass::Zero => 0,
            ModeClass::One => 1,
            ModeClass::Two => 2,
            ModeClass::Default => rng.bad_mode(),
        }
    }
    /// The operation the C actually selects for this class.
    fn op(self) -> Op {
        match self {
            ModeClass::Zero => Op::Process,
            ModeClass::One => Op::Double,
            ModeClass::Two => Op::Triple,
            ModeClass::Default => Op::Process, // `default:` falls back
        }
    }
}

/// Reproduces the value sequence `gotomach` would feed to the append test,
/// using the **C** `.so`'s operation function as the source of truth.
fn produced_sequence(h: &Harness, mode: ModeClass, seed: c_int, n: usize) -> Vec<c_int> {
    let op = h.c.op(mode.op());
    let mut out = Vec::with_capacity(n);
    let mut cur = seed;
    for _ in 0..n {
        let v = unsafe { op(cur, 0, std::ptr::null_mut()) };
        out.push(v);
        cur = v % 1000;
    }
    out
}

// ===========================================================================
// Rows 1-3 — the three leaf `operation_fn` exports over the whole i32 range
// ===========================================================================

#[test]
fn cfg_01_process_value_full_i32_range() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 1);
    // Both signed-overflow edges explicitly, then randomized.
    for v in [
        c_int::MIN,
        c_int::MIN + 1,
        -10,
        -1,
        0,
        1,
        c_int::MAX - 10,
        c_int::MAX - 9,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        h.assert_op(Op::Process, v, 0, std::ptr::null_mut());
    }
    for _ in 0..N_OP {
        let v = rng.i32_interesting();
        h.assert_op(Op::Process, v, 0, std::ptr::null_mut());
    }
}

#[test]
fn cfg_02_double_value_full_i32_range() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 2);
    for v in [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX / 2,
        c_int::MAX / 2 + 1,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        h.assert_op(Op::Double, v, 0, std::ptr::null_mut());
    }
    for _ in 0..N_OP {
        let v = rng.i32_interesting();
        h.assert_op(Op::Double, v, 0, std::ptr::null_mut());
    }
}

#[test]
fn cfg_03_triple_value_full_i32_range() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 3);
    for v in [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX / 3,
        c_int::MAX / 3 + 1,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        h.assert_op(Op::Triple, v, 0, std::ptr::null_mut());
    }
    for _ in 0..N_OP {
        let v = rng.i32_interesting();
        h.assert_op(Op::Triple, v, 0, std::ptr::null_mut());
    }
}

// ===========================================================================
// Row 4 — the two `(void)`-cast-away arguments must stay irrelevant
// ===========================================================================

#[test]
fn cfg_04_ops_ignore_unused_param_and_context() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 4);
    let mut scratch = [0u8; 64];
    for _ in 0..N_OP {
        let v = rng.i32_interesting();
        let unused = rng.i32_interesting();
        let ctx: *mut c_void = if rng.bool() {
            std::ptr::null_mut()
        } else {
            let off = (rng.next_u64() % 64) as usize;
            scratch.as_mut_ptr().wrapping_add(off) as *mut c_void
        };
        for which in Op::ALL {
            h.assert_op(which, v, unused, ctx);
        }
    }
}

// ===========================================================================
// Row 5 — the value range `gotomach` can actually feed the callbacks
// ===========================================================================

#[test]
fn cfg_05_ops_over_the_range_gotomach_feeds_them() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 5);
    // First iteration receives `seed` (0..=65535); every later one receives
    // `previous % 1000` (0..=999).
    for v in 0..=65535 {
        for which in Op::ALL {
            h.assert_op(which, v, 0, std::ptr::null_mut());
        }
    }
    for _ in 0..N_OP {
        let v = rng.range(0, 999);
        for which in Op::ALL {
            h.assert_op(which, v, 0, std::ptr::null_mut());
        }
    }
}

// ===========================================================================
// Rows 6-9 — `iterations == 0` (empty shape, `malloc(0)`), all four modes
// ===========================================================================

fn empty_shape(class: ModeClass, salt: u64) {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..N_SMALL {
        let mode = class.value(&mut rng);
        let a = args(0, rng.range(0, 65535), mode, rng.i32_interesting());
        let got = h.assert_gotomach(a);
        assert_eq!(got, 0, "empty shape must sum to 0 for {a}");
    }
}

#[test]
fn cfg_06_mode0_iterations_zero() {
    empty_shape(ModeClass::Zero, 6);
}
#[test]
fn cfg_07_mode1_iterations_zero() {
    empty_shape(ModeClass::One, 7);
}
#[test]
fn cfg_08_mode2_iterations_zero() {
    empty_shape(ModeClass::Two, 8);
}
#[test]
fn cfg_09_mode_default_iterations_zero() {
    empty_shape(ModeClass::Default, 9);
}

// ===========================================================================
// Rows 10-13 — `iterations == 1` (one shape), all four modes
// ===========================================================================

fn one_shape(class: ModeClass, salt: u64) {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..N_SMALL {
        let mode = class.value(&mut rng);
        h.assert_gotomach(args(1, rng.range(0, 65535), mode, rng.i32_interesting()));
    }
    // Plus a systematic sweep of the strict-`<` boundary for one element.
    for seed in [0, 1, 2, 999, 1000, 1001, 65534, 65535] {
        let mode = class.value(&mut rng);
        let produced = produced_sequence(&h, class, seed, 1)[0];
        for thr in [
            c_int::MIN,
            produced - 1,
            produced,
            produced + 1,
            c_int::MAX,
        ] {
            h.assert_gotomach(args(1, seed, mode, thr));
        }
    }
}

#[test]
fn cfg_10_mode0_iterations_one() {
    one_shape(ModeClass::Zero, 10);
}
#[test]
fn cfg_11_mode1_iterations_one() {
    one_shape(ModeClass::One, 11);
}
#[test]
fn cfg_12_mode2_iterations_one() {
    one_shape(ModeClass::Two, 12);
}
#[test]
fn cfg_13_mode_default_iterations_one() {
    one_shape(ModeClass::Default, 13);
}

// ===========================================================================
// Rows 14-17 — `iterations` in 2..=64 (many shape), all four modes
// ===========================================================================

fn many_shape(class: ModeClass, salt: u64) {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..N_SMALL {
        let mode = class.value(&mut rng);
        h.assert_gotomach(args(
            rng.range(2, 64),
            rng.range(0, 65535),
            mode,
            rng.i32_interesting(),
        ));
    }
}

#[test]
fn cfg_14_mode0_iterations_many() {
    many_shape(ModeClass::Zero, 14);
}
#[test]
fn cfg_15_mode1_iterations_many() {
    many_shape(ModeClass::One, 15);
}
#[test]
fn cfg_16_mode2_iterations_many() {
    many_shape(ModeClass::Two, 16);
}
#[test]
fn cfg_17_mode_default_iterations_many() {
    many_shape(ModeClass::Default, 17);
}

// ===========================================================================
// Rows 18-21 — `threshold == INT_MIN`: nothing is ever appended, `count == 0`
// ===========================================================================

fn append_none(class: ModeClass, salt: u64) {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..N_SMALL {
        let mode = class.value(&mut rng);
        let a = args(rng.range(0, 400), rng.range(0, 65535), mode, c_int::MIN);
        let got = h.assert_gotomach(a);
        assert_eq!(got, 0, "nothing may be appended for {a}");
    }
    // All produced values are >= 0, so `threshold == 0` also appends nothing.
    for _ in 0..N_SMALL / 4 {
        let mode = class.value(&mut rng);
        let a = args(rng.range(0, 400), rng.range(0, 65535), mode, 0);
        let got = h.assert_gotomach(a);
        assert_eq!(got, 0, "nothing may be appended for {a}");
    }
}

#[test]
fn cfg_18_mode0_threshold_int_min() {
    append_none(ModeClass::Zero, 18);
}
#[test]
fn cfg_19_mode1_threshold_int_min() {
    append_none(ModeClass::One, 19);
}
#[test]
fn cfg_20_mode2_threshold_int_min() {
    append_none(ModeClass::Two, 20);
}
#[test]
fn cfg_21_mode_default_threshold_int_min() {
    append_none(ModeClass::Default, 21);
}

// ===========================================================================
// Rows 22-25 — `threshold == INT_MAX`: everything is appended
// ===========================================================================

fn append_all(class: ModeClass, salt: u64) {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..N_SMALL {
        let mode = class.value(&mut rng);
        let iterations = rng.range(0, 400);
        let seed = rng.range(0, 65535);
        let got = h.assert_gotomach(args(iterations, seed, mode, c_int::MAX));
        // Independent oracle: every produced value is kept, so the answer is
        // the plain sum of the sequence.
        let want: c_int = produced_sequence(&h, class, seed, iterations as usize)
            .iter()
            .fold(0i32, |acc, v| acc.wrapping_add(*v));
        assert_eq!(
            got, want,
            "append-all sum mismatch for iterations={iterations} seed={seed} mode={mode}"
        );
    }
}

#[test]
fn cfg_22_mode0_threshold_int_max() {
    append_all(ModeClass::Zero, 22);
}
#[test]
fn cfg_23_mode1_threshold_int_max() {
    append_all(ModeClass::One, 23);
}
#[test]
fn cfg_24_mode2_threshold_int_max() {
    append_all(ModeClass::Two, 24);
}
#[test]
fn cfg_25_mode_default_threshold_int_max() {
    append_all(ModeClass::Default, 25);
}

// ===========================================================================
// Row 26 — the strict-`<` append boundary, pinned to real produced values
// ===========================================================================

#[test]
fn cfg_26_threshold_exactly_on_produced_values() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 26);
    for class in ModeClass::ALL {
        for _ in 0..120 {
            let iterations = rng.range(1, 40);
            let seed = rng.range(0, 65535);
            let mode = class.value(&mut rng);
            let seq = produced_sequence(&h, class, seed, iterations as usize);
            for &v in &seq {
                for thr in [v - 1, v, v + 1] {
                    h.assert_gotomach(args(iterations, seed, mode, thr));
                }
            }
        }
    }
}

// ===========================================================================
// Row 27 — thresholds inside the band where produced values actually live
// ===========================================================================

#[test]
fn cfg_27_threshold_inside_the_partial_append_band() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 27);
    for class in ModeClass::ALL {
        for _ in 0..N_BAND {
            let mode = class.value(&mut rng);
            // Produced values live in 0..=196 605 (first) and 0..=2 997 after.
            let thr = if rng.next_u64() % 8 == 0 {
                rng.range(0, 200_000)
            } else {
                rng.range(0, 3_100)
            };
            h.assert_gotomach(args(rng.range(0, 300), rng.range(0, 65535), mode, thr));
        }
    }
}

// ===========================================================================
// Rows 28-30 — `seed` boundaries and the `% 1000` fold
// ===========================================================================

#[test]
fn cfg_28_seed_zero_boundary() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 28);
    for class in ModeClass::ALL {
        for _ in 0..N_SMALL / 2 {
            let mode = class.value(&mut rng);
            h.assert_gotomach(args(rng.range(0, 400), 0, mode, rng.i32_interesting()));
        }
    }
}

#[test]
fn cfg_29_seed_uint16_max_boundary() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 29);
    for class in ModeClass::ALL {
        for _ in 0..N_SMALL / 2 {
            let mode = class.value(&mut rng);
            h.assert_gotomach(args(rng.range(0, 400), 65535, mode, rng.i32_interesting()));
        }
    }
}

#[test]
fn cfg_30_seed_below_and_above_the_fold() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 30);
    for class in ModeClass::ALL {
        // Below the fold: `process_value` never wraps on the first step.
        for _ in 0..N_SMALL / 2 {
            let mode = class.value(&mut rng);
            h.assert_gotomach(args(
                rng.range(0, 200),
                rng.range(0, 999),
                mode,
                rng.i32_interesting(),
            ));
        }
        // At and above the fold: the first op output exceeds 1000.
        for _ in 0..N_SMALL / 2 {
            let mode = class.value(&mut rng);
            h.assert_gotomach(args(
                rng.range(0, 200),
                rng.range(1000, 65535),
                mode,
                rng.i32_interesting(),
            ));
        }
        // Exactly on the fold edges.
        for seed in [995, 996, 999, 1000, 1001, 32_768, 65_534, 65_535] {
            let mode = class.value(&mut rng);
            for thr in [c_int::MIN, 0, 1, 1000, 1006, 2998, c_int::MAX] {
                h.assert_gotomach(args(50, seed, mode, thr));
            }
        }
    }
}

// ===========================================================================
// Rows 31-33 — the `count >= UINT16_MAX` saturation path
// ===========================================================================

#[test]
fn cfg_31_count_saturation_at_iterations_uint16_max() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 31);
    for class in ModeClass::ALL {
        for seed in [0, 1, 7, 999, 1000, 32_768, 65_534, 65_535] {
            let mode = class.value(&mut rng);
            let a = args(65_535, seed, mode, c_int::MAX);
            let (got, log) = h.assert_gotomach_logged(a);
            assert!(
                log.contains("[WARNING] Reached maximum count"),
                "expected the saturation warning for {a}, got {log:?}"
            );
            assert!(
                log.contains("[INFO] Processing completed successfully"),
                "saturation must still fall through to the sum for {a}, got {log:?}"
            );
            let want: c_int = produced_sequence(&h, class, seed, 65_535)
                .iter()
                .fold(0i32, |acc, v| acc.wrapping_add(*v));
            assert_eq!(got, want, "saturated sum mismatch for {a}");
        }
    }
}

#[test]
fn cfg_32_one_below_saturation_emits_no_warning() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 32);
    for class in ModeClass::ALL {
        for seed in [0, 1, 999, 1000, 65_535] {
            let mode = class.value(&mut rng);
            let a = args(65_534, seed, mode, c_int::MAX);
            let (_got, log) = h.assert_gotomach_logged(a);
            assert!(
                !log.contains("Reached maximum count"),
                "iterations=65534 must not saturate for {a}, got {log:?}"
            );
        }
    }
}

#[test]
fn cfg_33_iterations_uint16_max_with_partial_appends() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 33);
    for class in ModeClass::ALL {
        for _ in 0..12 {
            let mode = class.value(&mut rng);
            let thr = rng.range(0, 3_100);
            h.assert_gotomach(args(65_535, rng.range(0, 65535), mode, thr));
        }
    }
}

// ===========================================================================
// Row 34 — near-max `iterations` crossed with append density
// ===========================================================================

#[test]
fn cfg_34_near_max_iterations_cross_threshold() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 34);
    let iters = [
        65_000, 65_100, 65_500, 65_530, 65_531, 65_532, 65_533, 65_534, 65_535,
    ];
    let thrs = [c_int::MAX, 3_000, 1_006, 1_000, 100, 1, 0, c_int::MIN];
    for class in ModeClass::ALL {
        for &it in &iters {
            for &thr in &thrs {
                let mode = class.value(&mut rng);
                h.assert_gotomach(args(it, rng.range(0, 65535), mode, thr));
            }
        }
    }
}

// ===========================================================================
// Rows 35-36 — broad randomized sweeps
// ===========================================================================

#[test]
fn cfg_35_full_i32_sweep_all_four_arguments() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 35);
    for _ in 0..N_SWEEP {
        let a = args(
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        h.assert_gotomach_ret(a);
    }
}

#[test]
fn cfg_36_valid_only_sweep() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 36);
    // Bulk: small shapes, full log comparison.
    for _ in 0..20_000 {
        let a = args(
            rng.range(0, 200),
            rng.range(0, 65535),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        h.assert_gotomach(a);
    }
    // Wide: `iterations` uniform over the whole valid range.
    for _ in 0..N_WIDE {
        let a = args(
            rng.range(0, 65535),
            rng.range(0, 65535),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        h.assert_gotomach(a);
    }
}

// ===========================================================================
// Row 37 — stdout byte comparison for every distinct log path
// ===========================================================================

#[test]
fn cfg_37_every_distinct_log_path_matches_byte_for_byte() {
    let mut h = harness();

    // [INFO] entry + [INFO] success
    let (_, log) = h.assert_gotomach_logged(args(8, 3, 0, 100));
    assert_eq!(
        log,
        "[INFO] Starting gotomach function\n[INFO] Processing completed successfully\n"
    );

    // [INFO] entry + [INFO] success, empty shape
    let (_, log) = h.assert_gotomach_logged(args(0, 0, 1, 0));
    assert_eq!(
        log,
        "[INFO] Starting gotomach function\n[INFO] Processing completed successfully\n"
    );

    // [ERROR] Invalid iteration count
    for it in [-1, c_int::MIN, 65_536, c_int::MAX] {
        let (ret, log) = h.assert_gotomach_logged(args(it, 0, 0, 0));
        assert_eq!(ret, -1);
        assert_eq!(
            log,
            "[INFO] Starting gotomach function\n[ERROR] Invalid iteration count\n"
        );
    }

    // [ERROR] Invalid seed value
    for sd in [-1, c_int::MIN, 65_536, c_int::MAX] {
        let (ret, log) = h.assert_gotomach_logged(args(4, sd, 0, 0));
        assert_eq!(ret, -2);
        assert_eq!(
            log,
            "[INFO] Starting gotomach function\n[ERROR] Invalid seed value\n"
        );
    }

    // [WARNING] Invalid mode, using default
    for md in [3, -1, c_int::MIN, c_int::MAX, 99] {
        let (_, log) = h.assert_gotomach_logged(args(4, 1, md, 100));
        assert_eq!(
            log,
            "[INFO] Starting gotomach function\n[WARNING] Invalid mode, using default\n\
             [INFO] Processing completed successfully\n"
        );
    }

    // [WARNING] Reached maximum count
    let (_, log) = h.assert_gotomach_logged(args(65_535, 1, 2, c_int::MAX));
    assert_eq!(
        log,
        "[INFO] Starting gotomach function\n[WARNING] Reached maximum count\n\
         [INFO] Processing completed successfully\n"
    );

    // Both warnings together (default mode + saturation).
    let (_, log) = h.assert_gotomach_logged(args(65_535, 1, 12_345, c_int::MAX));
    assert_eq!(
        log,
        "[INFO] Starting gotomach function\n[WARNING] Invalid mode, using default\n\
         [WARNING] Reached maximum count\n[INFO] Processing completed successfully\n"
    );

    // The `-5` and `-6` log lines are statically unreachable, so they must
    // never appear — for any input at all (checked over the sweeps too).
    for a in [
        args(0, 0, 0, 0),
        args(1, 65_535, 2, c_int::MAX),
        args(65_535, 0, 1, 1),
        args(-1, -1, -1, -1),
    ] {
        let (_, log) = h.assert_gotomach_logged(a);
        assert!(!log.contains("Invalid state status"), "{a}: {log:?}");
        assert!(!log.contains("State became invalid"), "{a}: {log:?}");
    }
}

// ===========================================================================
// Row 38 — no hidden state: repeated and interleaved calls are invariant
// ===========================================================================

#[test]
fn cfg_38_repeat_and_interleave_invariance() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 38);
    for _ in 0..200 {
        let a = args(
            rng.range(0, 300),
            rng.range(0, 65535),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        let first = h.assert_gotomach(a);
        for _ in 0..49 {
            let again = h.assert_gotomach(a);
            assert_eq!(again, first, "{a} is not idempotent across repeats");
        }
    }
    // Interleave two different configurations to catch cross-call leakage.
    let a = args(37, 65_535, 2, 2_000);
    let b = args(64, 0, 1, 500);
    let ra = h.assert_gotomach(a);
    let rb = h.assert_gotomach(b);
    for _ in 0..200 {
        assert_eq!(h.assert_gotomach(a), ra);
        assert_eq!(h.assert_gotomach(b), rb);
        assert_eq!(h.assert_gotomach(b), rb);
        assert_eq!(h.assert_gotomach(a), ra);
    }
}

// ===========================================================================
// Row 39 — independent oracle for the composed pipeline
// ===========================================================================

#[test]
fn cfg_39_oracle_cross_check_of_the_composed_pipeline() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 39);

    let c_process = h.c.process_value;
    let c_double = h.c.double_value;
    let c_triple = h.c.triple_value;
    let op_for_mode = move |mode: c_int| -> OpFn {
        match mode {
            0 => c_process,
            1 => c_double,
            2 => c_triple,
            _ => c_process,
        }
    };

    for _ in 0..20_000 {
        let a = args(
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        let want = oracle(a, &op_for_mode);
        let got = h.assert_gotomach_ret(a);
        assert_eq!(got, want, "oracle disagrees for {a}");
    }
    for _ in 0..5_000 {
        let a = args(
            rng.range(0, 250),
            rng.range(0, 65535),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        let want = oracle(a, &op_for_mode);
        let got = h.assert_gotomach_ret(a);
        assert_eq!(got, want, "oracle disagrees for {a}");
    }
    for it in [65_533, 65_534, 65_535] {
        for thr in [c_int::MAX, 3_000, 1_000, 0, c_int::MIN] {
            for mode in [0, 1, 2, 7] {
                let a = args(it, rng.range(0, 65535), mode, thr);
                let want = oracle(a, &op_for_mode);
                let got = h.assert_gotomach_ret(a);
                assert_eq!(got, want, "oracle disagrees for {a}");
            }
        }
    }
}
