//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `driver` symbol (loaded with `libloading`) and asserts the stdout byte
//! streams are identical. Inputs are randomized per row with a fixed seed.

mod support;

use support::{assert_same, assert_same_all, rng_for, terminates, Impl};

/// Samples per randomized row.
const N: usize = 64;

// --- input-class generators, mirroring the `CONFIGS.md` equivalence classes ---

fn xs_neg(rng: &mut support::Rng) -> i32 {
    rng.range(-2_000, -1)
}
fn xs_gt3(rng: &mut support::Rng) -> i32 {
    rng.range(4, 300)
}
fn ys_neg(rng: &mut support::Rng) -> i32 {
    rng.range(-2_000, -1)
}
fn ys_small(rng: &mut support::Rng) -> i32 {
    rng.range(1, 3)
}
fn ys_gt4(rng: &mut support::Rng) -> i32 {
    rng.range(5, 300)
}

/// Builds `N` randomized `(x, y)` pairs from a pair of class generators.
fn samples(
    row: &str,
    fx: impl Fn(&mut support::Rng) -> i32,
    fy: impl Fn(&mut support::Rng) -> i32,
) -> Vec<(i32, i32)> {
    let mut rng = rng_for(row);
    (0..N).map(|_| (fx(&mut rng), fy(&mut rng))).collect()
}

fn fixed(x: i32, fy: impl Fn(&mut support::Rng) -> i32, row: &str) -> Vec<(i32, i32)> {
    let mut rng = rng_for(row);
    (0..N).map(|_| (x, fy(&mut rng))).collect()
}

fn constant(x: i32, y: i32) -> Vec<(i32, i32)> {
    vec![(x, y)]
}

// ---------------------------------------------------------------- rows 1..5
// x < 0

#[test]
fn cfg_row_01_xneg_yneg() {
    let row = "CONFIGS row 1 (x<0, y<0)";
    assert_same_all(row, samples(row, xs_neg, ys_neg));
}

#[test]
fn cfg_row_02_xneg_yzero() {
    let row = "CONFIGS row 2 (x<0, y==0)";
    assert_same_all(row, fixed_y(row, xs_neg, 0));
}

#[test]
fn cfg_row_03_xneg_ysmall() {
    let row = "CONFIGS row 3 (x<0, 0<y<4)";
    assert_same_all(row, samples(row, xs_neg, ys_small));
}

#[test]
fn cfg_row_04_xneg_yfour() {
    let row = "CONFIGS row 4 (x<0, y==4)";
    assert_same_all(row, fixed_y(row, xs_neg, 4));
}

#[test]
fn cfg_row_05_xneg_ygt4() {
    let row = "CONFIGS row 5 (x<0, y>4)";
    assert_same_all(row, samples(row, xs_neg, ys_gt4));
}

// ---------------------------------------------------------------- rows 6..10
// x == 0

#[test]
fn cfg_row_06_xzero_yneg() {
    let row = "CONFIGS row 6 (x==0, y<0)";
    assert_same_all(row, fixed(0, ys_neg, row));
}

#[test]
fn cfg_row_07_xzero_yzero() {
    let row = "CONFIGS row 7 (x==0, y==0)";
    let out = assert_same(row, 0, 0);
    assert!(out.is_empty(), "{row}: expected no output, got {out:?}");
    assert_same_all(row, constant(0, 0));
}

#[test]
fn cfg_row_08_xzero_ysmall() {
    let row = "CONFIGS row 8 (x==0, 0<y<4)";
    assert_same_all(row, fixed(0, ys_small, row));
}

#[test]
fn cfg_row_09_xzero_yfour() {
    let row = "CONFIGS row 9 (x==0, y==4)";
    assert_same_all(row, constant(0, 4));
}

#[test]
fn cfg_row_10_xzero_ygt4() {
    let row = "CONFIGS row 10 (x==0, y>4)";
    assert_same_all(row, fixed(0, ys_gt4, row));
}

// ---------------------------------------------------------------- rows 11..15
// x == 1 — the class that triggers the `x == 1 && y == 4` forward goto

#[test]
fn cfg_row_11_xone_yneg_is_nonterminating() {
    let row = "CONFIGS row 11 (x==1, y<0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let y = ys_neg(&mut rng);
        assert!(!terminates(1, y), "{row}: driver(1, {y}) must be classified divergent");
    }
    // Behavioural equivalence for this class is asserted by
    // `phase_c_errors::err_row_12_nonterminating_x_pos_y_neg`.
}

#[test]
fn cfg_row_12_xone_yzero() {
    let row = "CONFIGS row 12 (x==1, y==0)";
    assert_same_all(row, constant(1, 0));
}

#[test]
fn cfg_row_13_xone_ysmall() {
    let row = "CONFIGS row 13 (x==1, 0<y<4)";
    assert_same_all(row, fixed(1, ys_small, row));
}

#[test]
fn cfg_row_14_xone_yfour_special_case() {
    let row = "CONFIGS row 14 (x==1, y==4 — S2 forward goto)";
    let out = assert_same(row, 1, 4);
    // The skip must apply to the first pass only: `label1` is bypassed once, so
    // the very first line pair is "loop" then "y", with no "x" between them.
    let text = String::from_utf8(out).expect("ascii output");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(&lines[..2], &["loop", "y"], "{row}: forward goto did not skip label1");
    assert!(lines.contains(&"x"), "{row}: the skip must not persist past the first pass");
}

#[test]
fn cfg_row_15_xone_ygt4() {
    let row = "CONFIGS row 15 (x==1, y>4)";
    assert_same_all(row, fixed(1, ys_gt4, row));
}

// ---------------------------------------------------------------- rows 16..20
// x == 2 — S5 back-edge taken (2 < 3)

#[test]
fn cfg_row_16_xtwo_yneg_is_nonterminating() {
    let row = "CONFIGS row 16 (x==2, y<0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let y = ys_neg(&mut rng);
        assert!(!terminates(2, y), "{row}: driver(2, {y}) must be classified divergent");
    }
}

#[test]
fn cfg_row_17_xtwo_yzero() {
    let row = "CONFIGS row 17 (x==2, y==0)";
    assert_same_all(row, constant(2, 0));
}

#[test]
fn cfg_row_18_xtwo_ysmall() {
    let row = "CONFIGS row 18 (x==2, 0<y<4)";
    assert_same_all(row, fixed(2, ys_small, row));
}

#[test]
fn cfg_row_19_xtwo_yfour() {
    let row = "CONFIGS row 19 (x==2, y==4)";
    assert_same_all(row, constant(2, 4));
}

#[test]
fn cfg_row_20_xtwo_ygt4() {
    let row = "CONFIGS row 20 (x==2, y>4)";
    assert_same_all(row, fixed(2, ys_gt4, row));
}

// ---------------------------------------------------------------- rows 21..25
// x == 3 — S5 back-edge declined (3 < 3 is false)

#[test]
fn cfg_row_21_xthree_yneg_is_nonterminating() {
    let row = "CONFIGS row 21 (x==3, y<0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let y = ys_neg(&mut rng);
        assert!(!terminates(3, y), "{row}: driver(3, {y}) must be classified divergent");
    }
}

#[test]
fn cfg_row_22_xthree_yzero() {
    let row = "CONFIGS row 22 (x==3, y==0)";
    assert_same_all(row, constant(3, 0));
}

#[test]
fn cfg_row_23_xthree_ysmall() {
    let row = "CONFIGS row 23 (x==3, 0<y<4)";
    assert_same_all(row, fixed(3, ys_small, row));
}

#[test]
fn cfg_row_24_xthree_yfour() {
    let row = "CONFIGS row 24 (x==3, y==4)";
    assert_same_all(row, constant(3, 4));
}

#[test]
fn cfg_row_25_xthree_ygt4() {
    let row = "CONFIGS row 25 (x==3, y>4)";
    assert_same_all(row, fixed(3, ys_gt4, row));
}

// ---------------------------------------------------------------- rows 26..30
// x > 3 — S5 flips from false to true as x drains below 3

#[test]
fn cfg_row_26_xgt3_yneg_is_nonterminating() {
    let row = "CONFIGS row 26 (x>3, y<0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let (x, y) = (xs_gt3(&mut rng), ys_neg(&mut rng));
        assert!(!terminates(x, y), "{row}: driver({x}, {y}) must be classified divergent");
    }
}

#[test]
fn cfg_row_27_xgt3_yzero() {
    let row = "CONFIGS row 27 (x>3, y==0)";
    assert_same_all(row, fixed_y(row, xs_gt3, 0));
}

#[test]
fn cfg_row_28_xgt3_ysmall() {
    let row = "CONFIGS row 28 (x>3, 0<y<4)";
    assert_same_all(row, samples(row, xs_gt3, ys_small));
}

#[test]
fn cfg_row_29_xgt3_yfour() {
    let row = "CONFIGS row 29 (x>3, y==4)";
    assert_same_all(row, fixed_y(row, xs_gt3, 4));
}

#[test]
fn cfg_row_30_xgt3_ygt4() {
    let row = "CONFIGS row 30 (x>3, y>4)";
    assert_same_all(row, samples(row, xs_gt3, ys_gt4));
}

// ---------------------------------------------------------------- rows 31..36
// extremes, sweeps, and residual state

#[test]
fn cfg_row_31_x_int_min() {
    let row = "CONFIGS row 31 (x==INT_MIN)";
    let ys = [i32::MIN, -1, 0, 1, 4, 5, 37];
    let mut n = 0;
    for y in ys {
        // x == INT_MIN <= 0, so every one of these terminates.
        assert!(terminates(i32::MIN, y));
        assert_same(row, i32::MIN, y);
        n += 1;
    }
    assert_eq!(n, ys.len());
}

#[test]
fn cfg_row_32_extreme_mixed() {
    let row = "CONFIGS row 32 (extreme y)";
    let mut compared = 0;
    for x in [-1, 0, 1, 2, 3, 4] {
        for y in [i32::MIN, i32::MIN + 1] {
            if terminates(x, y) {
                assert_same(row, x, y);
                compared += 1;
            } else {
                assert!(x > 0, "{row}: only x>0 may diverge with y<0");
            }
        }
    }
    // x in {-1, 0} terminates for both extreme y values.
    assert_eq!(compared, 4, "{row}: unexpected terminating-set size");
}

#[test]
fn cfg_row_33_large_magnitudes() {
    let row = "CONFIGS row 33 (large magnitudes)";
    let mut rng = rng_for(row);
    let mut n = 0;
    for _ in 0..6 {
        let x = rng.range(10_000, 60_000);
        let y = rng.range(10_000, 60_000);
        assert_same(row, x, y);
        n += 1;
    }
    // Plus the pure-drain extremes at scale.
    for (x, y) in [(60_000, 0), (0, 60_000), (-60_000, 60_000), (60_000, 4)] {
        assert_same(row, x, y);
        n += 1;
    }
    assert_eq!(n, 10);
}

#[test]
fn cfg_row_34_exhaustive_small_grid() {
    let row = "CONFIGS row 34 (exhaustive grid [-6,12]^2)";
    let mut compared = 0;
    let mut skipped = 0;
    for x in -6..=12 {
        for y in -6..=12 {
            if terminates(x, y) {
                assert_same(row, x, y);
                compared += 1;
            } else {
                skipped += 1;
            }
        }
    }
    // 19x19 = 361 pairs; the divergent ones are exactly x in 1..=12 and y in -6..=-1.
    assert_eq!(compared + skipped, 361);
    assert_eq!(skipped, 12 * 6, "{row}: divergent set is not the expected quadrant");
}

#[test]
fn cfg_row_35_random_domain_sweep() {
    let row = "CONFIGS row 35 (random sweep [-64,512]^2)";
    let mut rng = rng_for(row);
    let mut compared = 0;
    for _ in 0..2_000 {
        let x = rng.range(-64, 512);
        let y = rng.range(-64, 512);
        if terminates(x, y) {
            assert_same(row, x, y);
            compared += 1;
        }
    }
    assert!(compared > 1_000, "{row}: only {compared} pairs were comparable");
}

#[test]
fn cfg_row_36_no_residual_state_across_calls() {
    let row = "CONFIGS row 36 (repeated calls, no residual state)";
    // Interleave shapes, then re-run the first shape and require the same bytes:
    // the C keeps no `static` state, so neither may the Rust.
    let seq = [(1, 4), (0, 0), (7, 9), (3, 1), (-5, 6), (2, 0), (1, 4)];
    let mut first: Option<Vec<u8>> = None;
    for (x, y) in seq {
        let out = assert_same(row, x, y);
        if (x, y) == (1, 4) {
            match &first {
                None => first = Some(out),
                Some(prev) => assert_eq!(*prev, out, "{row}: driver(1, 4) is not idempotent"),
            }
        }
    }
    // And the same again through a fresh capture, both implementations.
    let a = support::run(Impl::C, 1, 4);
    let b = support::run(Impl::Rust, 1, 4);
    assert_eq!(a, b, "{row}: divergence after repeated calls");
    assert_eq!(Some(a), first, "{row}: output changed across calls");
}

// --- helper used by rows with a randomized x and a pinned y -----------------

fn fixed_y(row: &str, fx: impl Fn(&mut support::Rng) -> i32, y: i32) -> Vec<(i32, i32)> {
    let mut rng = rng_for(row);
    (0..N).map(|_| (fx(&mut rng), y)).collect()
}
