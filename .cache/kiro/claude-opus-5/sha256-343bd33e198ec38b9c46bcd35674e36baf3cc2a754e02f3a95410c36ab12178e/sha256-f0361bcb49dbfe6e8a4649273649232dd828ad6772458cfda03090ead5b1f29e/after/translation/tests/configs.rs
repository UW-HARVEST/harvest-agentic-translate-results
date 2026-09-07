//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row drives BOTH the C `.so` and the Rust `.so` through `dlsym` and
//! compares the return value and the byte-exact stdout. Every row uses many
//! randomized inputs from a fixed PRNG seed.

mod common;

use common::{diff_gotomach, diff_op, libs, Rng};
use std::ffi::c_void;

const N: usize = 120;

/// A non-NULL, never-dereferenced value for the `void *unused_context`
/// parameter. The C ignores it; so must the Rust.
fn garbage_ctx() -> *mut c_void {
    0xDEAD_BEEF_usize as *mut c_void
}

/// Straddles the produced-value range of all three operations, so some
/// iterations store and some do not.
fn straddling_threshold(rng: &mut Rng) -> i32 {
    rng.range(-50, 3200)
}

// ---------------------------------------------------------------------------
// Rows 1-6: the three low-level operation callbacks, called directly through
// their exported symbols (not only via the `gotomach` wrapper).
// ---------------------------------------------------------------------------

#[test]
fn row01_process_value_random_null_ctx() {
    let mut rng = Rng::new(0x0101);
    for _ in 0..N {
        diff_op(
            "row01",
            "process_value",
            rng.next_i32(),
            rng.next_i32(),
            std::ptr::null_mut(),
        );
    }
}

#[test]
fn row02_process_value_overflow_boundaries_garbage_ctx() {
    let vals = [
        0,
        1,
        -1,
        10,
        -10,
        i32::MAX,
        i32::MAX - 9,
        i32::MAX - 10,
        i32::MAX - 11,
        i32::MIN,
        i32::MIN + 1,
    ];
    let mut rng = Rng::new(0x0202);
    for v in vals {
        diff_op("row02", "process_value", v, rng.next_i32(), garbage_ctx());
    }
}

#[test]
fn row03_double_value_random_null_ctx() {
    let mut rng = Rng::new(0x0303);
    for _ in 0..N {
        diff_op(
            "row03",
            "double_value",
            rng.next_i32(),
            rng.next_i32(),
            std::ptr::null_mut(),
        );
    }
}

#[test]
fn row04_double_value_overflow_boundaries_garbage_ctx() {
    let vals = [
        0,
        1,
        -1,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
    ];
    let mut rng = Rng::new(0x0404);
    for v in vals {
        diff_op("row04", "double_value", v, rng.next_i32(), garbage_ctx());
    }
}

#[test]
fn row05_triple_value_random_null_ctx() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..N {
        diff_op(
            "row05",
            "triple_value",
            rng.next_i32(),
            rng.next_i32(),
            std::ptr::null_mut(),
        );
    }
}

#[test]
fn row06_triple_value_overflow_boundaries_garbage_ctx() {
    let vals = [
        0,
        1,
        -1,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 3,
        i32::MAX / 3 + 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 3,
        i32::MIN / 3 - 1,
    ];
    let mut rng = Rng::new(0x0606);
    for v in vals {
        diff_op("row06", "triple_value", v, rng.next_i32(), garbage_ctx());
    }
}

// ---------------------------------------------------------------------------
// Rows 7-21: gotomach, per mode x input shape. Written as one generic helper
// driven by the mode so each row stays a distinct, individually-checkable test.
// ---------------------------------------------------------------------------

fn row_zero_iterations(row: &str, mode: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        diff_gotomach(row, 0, rng.range(0, 65535), mode, rng.next_i32());
    }
}

fn row_one_iteration(row: &str, mode: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        diff_gotomach(row, 1, rng.range(0, 65535), mode, rng.next_i32());
    }
}

fn row_straddling(row: &str, mode: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let it = rng.range(2, 4096);
        let sd = rng.range(0, 65535);
        let th = straddling_threshold(&mut rng);
        diff_gotomach(row, it, sd, mode, th);
    }
}

fn row_threshold_min(row: &str, mode: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let it = rng.range(0, 4096);
        let sd = rng.range(0, 65535);
        assert_eq!(
            diff_gotomach(row, it, sd, mode, i32::MIN),
            0,
            "threshold=INT_MIN must store nothing, so the sum must be 0"
        );
    }
}

fn row_threshold_max(row: &str, mode: i32, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let it = rng.range(0, 4096);
        let sd = rng.range(0, 65535);
        diff_gotomach(row, it, sd, mode, i32::MAX);
    }
}

#[test]
fn row07_mode0_zero_iterations() {
    row_zero_iterations("row07", 0, 0x0707);
}

#[test]
fn row08_mode0_one_iteration() {
    row_one_iteration("row08", 0, 0x0808);
}

#[test]
fn row09_mode0_straddling_threshold() {
    row_straddling("row09", 0, 0x0909);
}

#[test]
fn row10_mode0_threshold_int_min() {
    row_threshold_min("row10", 0, 0x1010);
}

#[test]
fn row11_mode0_threshold_int_max() {
    row_threshold_max("row11", 0, 0x1111);
}

#[test]
fn row12_mode1_zero_iterations() {
    row_zero_iterations("row12", 1, 0x1212);
}

#[test]
fn row13_mode1_one_iteration() {
    row_one_iteration("row13", 1, 0x1313);
}

#[test]
fn row14_mode1_straddling_threshold() {
    row_straddling("row14", 1, 0x1414);
}

#[test]
fn row15_mode1_threshold_int_min() {
    row_threshold_min("row15", 1, 0x1515);
}

#[test]
fn row16_mode1_threshold_int_max() {
    row_threshold_max("row16", 1, 0x1616);
}

#[test]
fn row17_mode2_zero_iterations() {
    row_zero_iterations("row17", 2, 0x1717);
}

#[test]
fn row18_mode2_one_iteration() {
    row_one_iteration("row18", 2, 0x1818);
}

#[test]
fn row19_mode2_straddling_threshold() {
    row_straddling("row19", 2, 0x1919);
}

#[test]
fn row20_mode2_threshold_int_min() {
    row_threshold_min("row20", 2, 0x2020);
}

#[test]
fn row21_mode2_threshold_int_max() {
    row_threshold_max("row21", 2, 0x2121);
}

// ---------------------------------------------------------------------------
// Rows 22-23: the `default:` arm of the switch, reached from above and below.
// ---------------------------------------------------------------------------

#[test]
fn row22_mode_default_from_above() {
    let mut rng = Rng::new(0x2222);
    for _ in 0..N {
        let it = rng.range(0, 2048);
        let sd = rng.range(0, 65535);
        let mode = rng.range(3, i32::MAX);
        let th = straddling_threshold(&mut rng);
        diff_gotomach("row22", it, sd, mode, th);
    }
    // Explicitly pin the immediate off-by-one past the last valid mode and the
    // extreme, and confirm the fallback really is process_value (mode 0).
    for mode in [3, 4, 100, i32::MAX, i32::MAX - 1] {
        let a = diff_gotomach("row22", 16, 7, mode, 3000);
        let b = diff_gotomach("row22", 16, 7, 0, 3000);
        assert_eq!(a, b, "default arm must behave like mode 0");
    }
}

#[test]
fn row23_mode_default_from_below() {
    let mut rng = Rng::new(0x2323);
    for _ in 0..N {
        let it = rng.range(0, 2048);
        let sd = rng.range(0, 65535);
        let mode = rng.range(i32::MIN, -1);
        let th = straddling_threshold(&mut rng);
        diff_gotomach("row23", it, sd, mode, th);
    }
    for mode in [-1, -2, -1000, i32::MIN, i32::MIN + 1] {
        let a = diff_gotomach("row23", 16, 7, mode, 3000);
        let b = diff_gotomach("row23", 16, 7, 0, 3000);
        assert_eq!(a, b, "default arm must behave like mode 0");
    }
}

// ---------------------------------------------------------------------------
// Row 24: seed boundaries x every mode path.
// ---------------------------------------------------------------------------

#[test]
fn row24_seed_boundaries_all_modes() {
    let mut rng = Rng::new(0x2424);
    for seed in [0, 1, 2, 999, 1000, 1001, 65534, 65535] {
        for mode in [0, 1, 2, 7, -7] {
            for it in [0, 1, 2, 3, 8, 33] {
                diff_gotomach("row24", it, seed, mode, straddling_threshold(&mut rng));
                diff_gotomach("row24", it, seed, mode, i32::MAX);
                diff_gotomach("row24", it, seed, mode, i32::MIN);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25: the strict `<` boundary of `produced < threshold`.
// ---------------------------------------------------------------------------

#[test]
fn row25_threshold_exactly_at_produced_value() {
    for seed in [0, 1, 2, 5, 100, 333, 1000, 65535] {
        // f(seed) for each mode, per the C: +10, *2, *3.
        let produced = [seed + 10, seed * 2, seed * 3];
        for (mode, p) in produced.iter().enumerate() {
            for th in [*p - 1, *p, *p + 1] {
                diff_gotomach("row25", 1, seed, mode as i32, th);
            }
        }
        // Same boundary sweep with several iterations, so later values (which
        // are derived through `% 1000`) also sit on the boundary.
        for mode in 0..3 {
            for delta in -2i32..=2 {
                diff_gotomach("row25", 12, seed, mode, 1000 + delta);
                diff_gotomach("row25", 12, seed, mode, delta);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26: the upper bound on iterations, the only shape that can reach the
// `[WARNING] Reached maximum count` break.
// ---------------------------------------------------------------------------

#[test]
fn row26_max_iterations() {
    let mut rng = Rng::new(0x2626);
    for mode in [0, 1, 2, 9] {
        for th in [i32::MIN, i32::MAX, 1500, 0, 1] {
            diff_gotomach("row26", 65535, rng.range(0, 65535), mode, th);
        }
        // A few near-the-bound sizes as well.
        for it in [65530, 65534, 65535] {
            diff_gotomach("row26", it, rng.range(0, 65535), mode, i32::MAX);
        }
    }
    // Sanity: the max-count warning path really is exercised (count reaches
    // UINT16_MAX only when iterations == 65535 and everything is stored).
    let l = libs();
    let f: libloading::Symbol<common::GotomachFn> =
        unsafe { l.c.get(b"gotomach\0") }.expect("dlsym");
    let (_, out) = common::capture(|| unsafe { f(65535, 0, 0, i32::MAX) });
    assert!(
        String::from_utf8_lossy(&out).contains("Reached maximum count"),
        "expected the C max-count warning to be reachable, got: {}",
        String::from_utf8_lossy(&out)
    );
}

// ---------------------------------------------------------------------------
// Row 27: unconstrained fuzz over all four parameters.
// ---------------------------------------------------------------------------

#[test]
fn row27_full_random_fuzz() {
    let mut rng = Rng::new(0x2727);
    for _ in 0..600 {
        let it = rng.next_i32();
        let sd = rng.next_i32();
        let mode = rng.next_i32();
        let th = rng.next_i32();
        diff_gotomach("row27", it, sd, mode, th);
    }
    // Biased fuzz that stays inside the valid domain so the loop body actually
    // runs, with fully random mode/threshold.
    let mut rng = Rng::new(0x2728);
    for _ in 0..300 {
        let it = rng.range(0, 1024);
        let sd = rng.range(0, 65535);
        let mode = rng.next_i32();
        let th = rng.next_i32();
        diff_gotomach("row27", it, sd, mode, th);
    }
}

// ---------------------------------------------------------------------------
// Row 28: repeated invocations in one process must not leak state.
// ---------------------------------------------------------------------------

#[test]
fn row28_repeated_invocations_are_independent() {
    let mut rng = Rng::new(0x2828);
    let mut first: Vec<i32> = Vec::new();
    let cases: Vec<(i32, i32, i32, i32)> = (0..60)
        .map(|_| {
            (
                rng.range(0, 512),
                rng.range(0, 65535),
                rng.range(-3, 5),
                straddling_threshold(&mut rng),
            )
        })
        .collect();
    for &(it, sd, m, th) in &cases {
        first.push(diff_gotomach("row28", it, sd, m, th));
    }
    // Replay the same sequence; both libraries must be idempotent per call.
    for (idx, &(it, sd, m, th)) in cases.iter().enumerate() {
        let again = diff_gotomach("row28", it, sd, m, th);
        assert_eq!(
            first[idx], again,
            "gotomach({it}, {sd}, {m}, {th}) is not idempotent across calls"
        );
    }
    // And interleave the two libraries in the reverse order, too.
    for &(it, sd, m, th) in cases.iter().rev() {
        diff_gotomach("row28", it, sd, m, th);
    }
}
