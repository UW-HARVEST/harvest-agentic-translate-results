//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` with `libloading` and
//! compares their results through the FFI boundary. Rows start at the lowest
//! level (`process_value` / `double_value` / `triple_value`) and work up to the
//! composed `gotomach` pipeline.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const NULLCTX: *mut c_void = std::ptr::null_mut();
/// A garbage, never-dereferenced non-null `void *unused_context`.
fn junk_ctx(n: u64) -> *mut c_void {
    (0xDEAD_0000_0000_0000u64 | n) as *mut c_void
}

// ===========================================================================
// Rows 1-6: the three operation_fn helpers, randomized + boundary values
// ===========================================================================

#[test]
fn row01_process_value_randomized_full_i32() {
    let mut rng = Rng::new(0x0101);
    for _ in 0..20_000 {
        diff_op("process_value", rng.next_i32(), 0, NULLCTX);
    }
}

#[test]
fn row02_process_value_boundaries() {
    let mut rng = Rng::new(0x0202);
    let values = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 10,
        -11,
        -10,
        -1,
        0,
        1,
        10,
        i32::MAX - 11,
        i32::MAX - 10,
        i32::MAX - 1,
        i32::MAX,
    ];
    for (i, &v) in values.iter().enumerate() {
        // unused_param / unused_context must be ignored entirely.
        diff_op("process_value", v, 0, NULLCTX);
        diff_op("process_value", v, rng.next_i32(), junk_ctx(i as u64));
        diff_op("process_value", v, i32::MIN, junk_ctx(i as u64));
        diff_op("process_value", v, i32::MAX, NULLCTX);
    }
}

#[test]
fn row03_double_value_randomized_full_i32() {
    let mut rng = Rng::new(0x0303);
    for _ in 0..20_000 {
        diff_op("double_value", rng.next_i32(), 0, NULLCTX);
    }
}

#[test]
fn row04_double_value_boundaries() {
    let mut rng = Rng::new(0x0404);
    let values = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2 - 1,
        i32::MIN / 2,
        -1,
        0,
        1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for (i, &v) in values.iter().enumerate() {
        diff_op("double_value", v, 0, NULLCTX);
        diff_op("double_value", v, rng.next_i32(), junk_ctx(i as u64));
        diff_op("double_value", v, i32::MIN, junk_ctx(i as u64));
        diff_op("double_value", v, i32::MAX, NULLCTX);
    }
}

#[test]
fn row05_triple_value_randomized_full_i32() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..20_000 {
        diff_op("triple_value", rng.next_i32(), 0, NULLCTX);
    }
}

#[test]
fn row06_triple_value_boundaries() {
    let mut rng = Rng::new(0x0606);
    let values = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 3 - 1,
        i32::MIN / 3,
        -1,
        0,
        1,
        i32::MAX / 3,
        i32::MAX / 3 + 1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for (i, &v) in values.iter().enumerate() {
        diff_op("triple_value", v, 0, NULLCTX);
        diff_op("triple_value", v, rng.next_i32(), junk_ctx(i as u64));
        diff_op("triple_value", v, i32::MIN, junk_ctx(i as u64));
        diff_op("triple_value", v, i32::MAX, NULLCTX);
    }
}

// ===========================================================================
// Row 7: helpers driven through the operation_fn indirection over exactly the
// value domain `gotomach` feeds them (seed in 0..=65535, then `% 1000`).
// ===========================================================================

#[test]
fn row07_helpers_via_operation_fn_pointer_in_gotomach_domain() {
    let p = pair();
    let mut rng = Rng::new(0x0707);
    for &mode in ALL_MODES {
        let op_c = p.c.op_for_mode(mode);
        let op_r = p.rust.op_for_mode(mode);
        for _ in 0..2_000 {
            // Emulate the recurrence: start anywhere in the seed range, then
            // keep folding through `% 1000` like gotomach's loop does.
            let mut v = rng.range_i32(0, 65535);
            for _ in 0..16 {
                let a = unsafe { op_c(v, 0, NULLCTX) };
                let b = unsafe { op_r(v, 0, NULLCTX) };
                assert_eq!(a, b, "mode={mode} value={v}: C={a} Rust={b}");
                v = a % 1000;
            }
        }
        // Also walk every value in the reachable post-`% 1000` band densely.
        for v in -999..=999 {
            let a = unsafe { op_c(v, 0, NULLCTX) };
            let b = unsafe { op_r(v, 0, NULLCTX) };
            assert_eq!(a, b, "mode={mode} value={v}: C={a} Rust={b}");
        }
    }
}

// ===========================================================================
// Row 8: unused_param / unused_context must never influence the result.
// ===========================================================================

#[test]
fn row08_helpers_ignore_unused_param_and_context() {
    let mut rng = Rng::new(0x0808);
    for name in ["process_value", "double_value", "triple_value"] {
        for _ in 0..4_000 {
            let v = rng.next_i32();
            let base = diff_op(name, v, 0, NULLCTX);
            for (up, cx) in [
                (0, junk_ctx(1)),
                (i32::MIN, junk_ctx(2)),
                (i32::MAX, junk_ctx(0xFFFF)),
                (rng.next_i32(), NULLCTX),
                (-1, junk_ctx(rng.next_u64() & 0xFFFF)),
            ] {
                let got = diff_op(name, v, up, cx);
                assert_eq!(
                    base, got,
                    "{name} result depends on unused_param/unused_context (value={v})"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 9-12: iterations == 0 (empty workload) for every mode arm.
// ===========================================================================

fn empty_workload(mode: c_int, seed_key: u64) {
    let mut rng = Rng::new(seed_key);
    for _ in 0..200 {
        let seed = rng.range_i32(0, 65535);
        let threshold = rng.next_i32();
        assert_eq!(
            diff_gotomach(0, seed, mode, threshold),
            0,
            "iterations==0 must sum to 0"
        );
    }
    // Boundaries of the accepted seed range.
    for seed in [0, 1, 65534, 65535] {
        for threshold in [i32::MIN, -1, 0, 1, 1000, i32::MAX] {
            diff_gotomach(0, seed, mode, threshold);
        }
    }
}

#[test]
fn row09_mode0_iterations0() {
    empty_workload(0, 0x0909);
}

#[test]
fn row10_mode1_iterations0() {
    empty_workload(1, 0x1010);
}

#[test]
fn row11_mode2_iterations0() {
    empty_workload(2, 0x1111);
}

#[test]
fn row12_invalid_mode_iterations0() {
    for (i, &mode) in INVALID_MODES.iter().enumerate() {
        empty_workload(mode, 0x1200 + i as u64);
    }
}

// ===========================================================================
// Rows 13-15: iterations == 1 (single element).
// ===========================================================================

fn single_element(mode: c_int, seed_key: u64) {
    let mut rng = Rng::new(seed_key);
    for _ in 0..3_000 {
        let seed = rng.range_i32(0, 65535);
        let threshold = rng.next_i32();
        diff_gotomach_rc(1, seed, mode, threshold);
    }
    // Dense sweep near the interesting threshold band plus seed boundaries.
    for seed in [0, 1, 2, 999, 1000, 1001, 32767, 65534, 65535] {
        for threshold in [i32::MIN, -1, 0, 1, 10, 11, 12, 1000, 3000, i32::MAX] {
            diff_gotomach(1, seed, mode, threshold);
        }
    }
}

#[test]
fn row13_mode0_iterations1() {
    single_element(0, 0x1313);
}

#[test]
fn row14_mode1_iterations1() {
    single_element(1, 0x1414);
}

#[test]
fn row15_mode2_iterations1() {
    single_element(2, 0x1515);
}

// ===========================================================================
// Rows 16-25: small workloads across the three `results` fill levels.
// ===========================================================================

/// `threshold` fixed => a specific fill level; `iterations` in 2..=64.
fn small_workload(mode: c_int, threshold_pick: impl Fn(&mut Rng) -> c_int, seed_key: u64) {
    let mut rng = Rng::new(seed_key);
    for _ in 0..3_000 {
        let iterations = rng.range_i32(2, 64);
        let seed = rng.range_i32(0, 65535);
        let threshold = threshold_pick(&mut rng);
        diff_gotomach_rc(iterations, seed, mode, threshold);
    }
    // A handful with full stdout comparison too.
    for _ in 0..40 {
        let iterations = rng.range_i32(2, 64);
        let seed = rng.range_i32(0, 65535);
        let threshold = threshold_pick(&mut rng);
        diff_gotomach(iterations, seed, mode, threshold);
    }
}

#[test]
fn row16_mode0_small_threshold_int_min_nothing_appended() {
    small_workload(0, |_| i32::MIN, 0x1616);
}

#[test]
fn row17_mode1_small_threshold_int_min_nothing_appended() {
    small_workload(1, |_| i32::MIN, 0x1717);
}

#[test]
fn row18_mode2_small_threshold_int_min_nothing_appended() {
    small_workload(2, |_| i32::MIN, 0x1818);
}

#[test]
fn row19_mode0_small_threshold_int_max_everything_appended() {
    small_workload(0, |_| i32::MAX, 0x1919);
}

#[test]
fn row20_mode1_small_threshold_int_max_everything_appended() {
    small_workload(1, |_| i32::MAX, 0x2020);
}

#[test]
fn row21_mode2_small_threshold_int_max_everything_appended() {
    small_workload(2, |_| i32::MAX, 0x2121);
}

/// The interleaving band: operation outputs live in roughly `0..=3000` once the
/// `% 1000` recurrence kicks in, so a threshold in this window appends SOME
/// elements and skips others — the value-dependent partial-fill path.
fn interleaving_threshold(rng: &mut Rng) -> c_int {
    rng.range_i32(-50, 3100)
}

#[test]
fn row22_mode0_small_interleaving_threshold() {
    small_workload(0, interleaving_threshold, 0x2222);
}

#[test]
fn row23_mode1_small_interleaving_threshold() {
    small_workload(1, interleaving_threshold, 0x2323);
}

#[test]
fn row24_mode2_small_interleaving_threshold() {
    small_workload(2, interleaving_threshold, 0x2424);
}

#[test]
fn row25_invalid_mode_small_interleaving_threshold() {
    for (i, &mode) in INVALID_MODES.iter().enumerate() {
        small_workload(mode, interleaving_threshold, 0x2500 + i as u64);
    }
}

// ===========================================================================
// Row 26: the pruned full cross-product mode x seed-boundary x threshold.
// ===========================================================================

#[test]
fn row26_full_cross_product_modes_seeds_thresholds() {
    let modes: Vec<c_int> = ALL_MODES.to_vec();
    let seeds = [0, 1, 2, 500, 999, 1000, 1001, 32768, 65534, 65535];
    let thresholds = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -1,
        0,
        1,
        11,
        999,
        1000,
        1001,
        2000,
        3000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &mode in &modes {
        for &seed in &seeds {
            for &threshold in &thresholds {
                for &iterations in &[0, 1, 2, 3, 8, 17] {
                    diff_gotomach(iterations, seed, mode, threshold);
                }
            }
        }
    }
}

// ===========================================================================
// Row 27: large workloads — long `% 1000` recurrence.
// ===========================================================================

#[test]
fn row27_large_iterations_all_modes() {
    let mut rng = Rng::new(0x2727);
    for &mode in ALL_MODES {
        for _ in 0..60 {
            let iterations = rng.range_i32(500, 2000);
            let seed = rng.range_i32(0, 65535);
            let threshold = match rng.next_u64() % 4 {
                0 => i32::MIN,
                1 => i32::MAX,
                2 => interleaving_threshold(&mut rng),
                _ => rng.next_i32(),
            };
            diff_gotomach_rc(iterations, seed, mode, threshold);
        }
        // With stdout too, for a few.
        for _ in 0..5 {
            let iterations = rng.range_i32(500, 2000);
            let seed = rng.range_i32(0, 65535);
            diff_gotomach(iterations, seed, mode, interleaving_threshold(&mut rng));
        }
    }
}

// ===========================================================================
// Rows 28-29: iterations == 65535, the maximum accepted workload. Row 28 is
// the only configuration that reaches `count >= UINT16_MAX` and prints
// `[WARNING] Reached maximum count`.
// ===========================================================================

#[test]
fn row28_max_iterations_threshold_int_max_reaches_count_cap() {
    for &mode in ALL_MODES {
        let (_rc, out) = {
            let p = pair();
            let g = p.c.gotomach;
            capture_stdout(|| unsafe { g(65535, 12345, mode, i32::MAX) })
        };
        assert!(
            String::from_utf8_lossy(&out).contains("[WARNING] Reached maximum count"),
            "row 28 precondition: C did not reach the count cap for mode={mode}; got {:?}",
            String::from_utf8_lossy(&out)
        );
        // Now the actual differential assertion (return value + stdout).
        for seed in [0, 1, 12345, 65535] {
            diff_gotomach(65535, seed, mode, i32::MAX);
        }
    }
}

#[test]
fn row29_max_iterations_other_thresholds() {
    let mut rng = Rng::new(0x2929);
    for &mode in ALL_MODES {
        diff_gotomach(65535, 0, mode, i32::MIN);
        diff_gotomach(65535, 65535, mode, i32::MIN);
        for _ in 0..6 {
            let seed = rng.range_i32(0, 65535);
            let threshold = interleaving_threshold(&mut rng);
            diff_gotomach(65535, seed, mode, threshold);
        }
    }
    // Just under / at the boundary of the accepted iteration range.
    for &iterations in &[65533, 65534, 65535] {
        for &mode in VALID_MODES {
            diff_gotomach(iterations, 777, mode, 1500);
        }
    }
}

// ===========================================================================
// Row 30: broad randomized fuzz over the entire accepted domain.
// ===========================================================================

#[test]
fn row30_fuzz_accepted_domain() {
    let mut rng = Rng::new(0x3030);
    for _ in 0..40_000 {
        let iterations = rng.range_i32(0, 1024);
        let seed = rng.range_i32(0, 65535);
        let mode = match rng.next_u64() % 3 {
            0 => rng.range_i32(-3, 5),
            1 => rng.next_i32(),
            _ => *ALL_MODES.get((rng.next_u64() as usize) % ALL_MODES.len()).unwrap(),
        };
        let threshold = match rng.next_u64() % 5 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => interleaving_threshold(&mut rng),
            3 => rng.range_i32(-2, 2),
            _ => rng.next_i32(),
        };
        diff_gotomach_rc(iterations, seed, mode, threshold);
    }
}

#[test]
fn row30b_fuzz_with_stdout_comparison() {
    let mut rng = Rng::new(0x303B);
    for _ in 0..1_500 {
        let iterations = rng.range_i32(0, 64);
        let seed = rng.range_i32(0, 65535);
        let mode = rng.range_i32(-4, 6);
        let threshold = match rng.next_u64() % 4 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => interleaving_threshold(&mut rng),
            _ => rng.next_i32(),
        };
        diff_gotomach(iterations, seed, mode, threshold);
    }
}

// ===========================================================================
// Row 31: byte-for-byte stdout parity for each distinct log-line combination.
// This is the stand-in for the "compare driver stdout" gate — the project has
// no binary target, so stdout is compared around the `.so` calls themselves.
// ===========================================================================

#[test]
fn row31_stdout_byte_for_byte_every_log_combination() {
    // (iterations, seed, mode, threshold, expected exact stdout)
    let info_start = "[INFO] Starting gotomach function\n";
    let info_done = "[INFO] Processing completed successfully\n";
    let warn_mode = "[WARNING] Invalid mode, using default\n";

    let cases: &[(c_int, c_int, c_int, c_int, String)] = &[
        // plain success, valid mode
        (0, 0, 0, 0, format!("{info_start}{info_done}")),
        (8, 5, 1, i32::MAX, format!("{info_start}{info_done}")),
        (8, 5, 2, i32::MIN, format!("{info_start}{info_done}")),
        // success via the default switch arm
        (8, 5, 7, 1500, format!("{info_start}{warn_mode}{info_done}")),
        (0, 0, i32::MIN, 0, format!("{info_start}{warn_mode}{info_done}")),
        // each error path: the range checks precede the mode switch, so no
        // [WARNING] must appear even with an invalid mode.
        (-1, 0, 0, 0, format!("{info_start}[ERROR] Invalid iteration count\n")),
        (65536, 0, 9, 0, format!("{info_start}[ERROR] Invalid iteration count\n")),
        (i32::MIN, 0, 9, 0, format!("{info_start}[ERROR] Invalid iteration count\n")),
        (8, -1, 0, 0, format!("{info_start}[ERROR] Invalid seed value\n")),
        (8, 65536, 9, 0, format!("{info_start}[ERROR] Invalid seed value\n")),
        (8, i32::MAX, 9, 0, format!("{info_start}[ERROR] Invalid seed value\n")),
    ];

    let p = pair();
    for &(iterations, seed, mode, threshold, ref expected) in cases {
        let g_c = p.c.gotomach;
        let g_r = p.rust.gotomach;
        let (rc_c, out_c) = capture_stdout(|| unsafe { g_c(iterations, seed, mode, threshold) });
        let (rc_r, out_r) = capture_stdout(|| unsafe { g_r(iterations, seed, mode, threshold) });
        let ctx = format!("({iterations}, {seed}, {mode}, {threshold})");
        assert_eq!(rc_c, rc_r, "rc mismatch {ctx}");
        assert_eq!(
            String::from_utf8_lossy(&out_c),
            *expected,
            "C stdout unexpected for {ctx}"
        );
        assert_eq!(out_c, out_r, "C/Rust stdout differ for {ctx}");
    }

    // The count-cap warning combination.
    let g_c = p.c.gotomach;
    let g_r = p.rust.gotomach;
    let (rc_c, out_c) = capture_stdout(|| unsafe { g_c(65535, 1, 0, i32::MAX) });
    let (rc_r, out_r) = capture_stdout(|| unsafe { g_r(65535, 1, 0, i32::MAX) });
    assert_eq!(rc_c, rc_r);
    assert_eq!(
        String::from_utf8_lossy(&out_c),
        format!("{info_start}[WARNING] Reached maximum count\n{info_done}")
    );
    assert_eq!(out_c, out_r);

    // Count cap AND the invalid-mode warning together.
    let (rc_c, out_c) = capture_stdout(|| unsafe { g_c(65535, 1, 42, i32::MAX) });
    let (rc_r, out_r) = capture_stdout(|| unsafe { g_r(65535, 1, 42, i32::MAX) });
    assert_eq!(rc_c, rc_r);
    assert_eq!(
        String::from_utf8_lossy(&out_c),
        format!("{info_start}{warn_mode}[WARNING] Reached maximum count\n{info_done}")
    );
    assert_eq!(out_c, out_r);
}
