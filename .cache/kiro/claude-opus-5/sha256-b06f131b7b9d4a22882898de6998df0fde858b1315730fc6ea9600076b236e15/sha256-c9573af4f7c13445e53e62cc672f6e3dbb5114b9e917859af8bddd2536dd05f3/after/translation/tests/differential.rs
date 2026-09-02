//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through `dlopen`/`dlsym` on their
//! respective `.so`s.

mod common;

use common::{boundary_values, sweep, Pair, Rng, I32_MAX, I32_MIN};
use std::ffi::c_int;

/// Draws per randomized row.
const N: usize = 200_000;

// ---------------------------------------------------------------------------
// Group A — v1 >= 0, v2 > 0  (early `return v1 / v2`)
// ---------------------------------------------------------------------------

#[test]
fn row01_v1_zero_v2_pos() {
    let p = Pair::load();
    sweep(&p, N, |r| (0, r.pos()));
    p.check(0, 1);
    p.check(0, I32_MAX);
}

#[test]
fn row02_pos_lt_pos() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let v2 = r.range(2, I32_MAX);
        (r.range(1, v2 - 1), v2)
    });
}

#[test]
fn row03_pos_eq_pos() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.pos();
        (m, m)
    });
}

#[test]
fn row04_pos_gt_pos_exact() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, I32_MAX / 2);
        let k = r.range(2, I32_MAX / m);
        (k * m, m)
    });
}

#[test]
fn row05_pos_gt_pos_remainder() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, 1 << 15);
        let rem = r.range(1, m - 1);
        let k = r.range(2, (I32_MAX - rem) / m);
        (k * m + rem, m)
    });
}

#[test]
fn row06_v2_one_v1_nonneg() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(0, I32_MAX), 1));
    p.check(0, 1);
    p.check(I32_MAX, 1);
}

#[test]
fn row07_max_over_one() {
    Pair::load().check(I32_MAX, 1);
}

#[test]
fn row08_max_over_max() {
    Pair::load().check(I32_MAX, I32_MAX);
}

#[test]
fn row09_max_over_two() {
    Pair::load().check(I32_MAX, 2);
}

#[test]
fn row10_nonneg_over_max() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(0, I32_MAX - 1), I32_MAX));
    p.check(I32_MAX - 1, I32_MAX);
}

// ---------------------------------------------------------------------------
// Group B — v1 >= 0, INT_MIN < v2 < 0
// ---------------------------------------------------------------------------

#[test]
fn row11_v1_zero_v2_neg() {
    let p = Pair::load();
    sweep(&p, N, |r| (0, r.neg_non_min()));
    p.check(0, -1);
    p.check(0, I32_MIN + 1);
}

#[test]
fn row12_pos_lt_absneg() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, I32_MAX);
        (r.range(1, m - 1), -m)
    });
}

#[test]
fn row13_pos_eq_absneg() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.pos();
        (m, -m)
    });
}

#[test]
fn row14_pos_gt_absneg_exact() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, I32_MAX / 2);
        let k = r.range(2, I32_MAX / m);
        (k * m, -m)
    });
}

#[test]
fn row15_pos_gt_absneg_remainder() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, 1 << 15);
        let rem = r.range(1, m - 1);
        let k = r.range(2, (I32_MAX - rem) / m);
        (k * m + rem, -m)
    });
}

#[test]
fn row16_v2_minus_one_v1_nonneg() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(0, I32_MAX), -1));
    p.check(0, -1);
    p.check(I32_MAX, -1);
}

#[test]
fn row17_max_over_minus_one() {
    Pair::load().check(I32_MAX, -1);
}

#[test]
fn row18_max_over_min_plus_one() {
    Pair::load().check(I32_MAX, I32_MIN + 1);
}

// ---------------------------------------------------------------------------
// Group C — v1 >= 0, v2 == INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn row19_nonneg_over_int_min() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(0, I32_MAX), I32_MIN));
    for v1 in [0, 1, 2, I32_MAX - 1, I32_MAX] {
        p.check(v1, I32_MIN);
    }
}

// ---------------------------------------------------------------------------
// Group D — INT_MIN < v1 < 0, v2 > 0
// ---------------------------------------------------------------------------

#[test]
fn row20_absneg_lt_pos() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let v2 = r.range(2, I32_MAX);
        (-r.range(1, v2 - 1), v2)
    });
}

#[test]
fn row21_absneg_eq_pos() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.pos();
        (-m, m)
    });
}

#[test]
fn row22_absneg_gt_pos_exact() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, I32_MAX / 2);
        let k = r.range(2, I32_MAX / m);
        (-(k * m), m)
    });
}

#[test]
fn row23_absneg_gt_pos_remainder() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, 1 << 15);
        let rem = r.range(1, m - 1);
        let k = r.range(2, (I32_MAX - rem) / m);
        (-(k * m + rem), m)
    });
}

#[test]
fn row24_neg_over_one() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(I32_MIN + 1, -1), 1));
    p.check(-1, 1);
    p.check(I32_MIN + 1, 1);
}

#[test]
fn row25_min_plus_one_over_one() {
    Pair::load().check(I32_MIN + 1, 1);
}

#[test]
fn row26_min_plus_one_over_max() {
    Pair::load().check(I32_MIN + 1, I32_MAX);
}

#[test]
fn row27_minus_one_over_max() {
    Pair::load().check(-1, I32_MAX);
}

// ---------------------------------------------------------------------------
// Group E — INT_MIN < v1 < 0, INT_MIN < v2 < 0
// ---------------------------------------------------------------------------

#[test]
fn row28_absneg_lt_absneg() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, I32_MAX);
        (-r.range(1, m - 1), -m)
    });
}

#[test]
fn row29_absneg_eq_absneg() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.pos();
        (-m, -m)
    });
}

#[test]
fn row30_absneg_gt_absneg_exact() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, I32_MAX / 2);
        let k = r.range(2, I32_MAX / m);
        (-(k * m), -m)
    });
}

#[test]
fn row31_absneg_gt_absneg_remainder() {
    let p = Pair::load();
    sweep(&p, N, |r| {
        let m = r.range(2, 1 << 15);
        let rem = r.range(1, m - 1);
        let k = r.range(2, (I32_MAX - rem) / m);
        (-(k * m + rem), -m)
    });
}

#[test]
fn row32_neg_over_minus_one() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(I32_MIN + 1, -1), -1));
    p.check(-1, -1);
    p.check(I32_MIN + 1, -1);
}

#[test]
fn row33_min_plus_one_over_minus_one() {
    Pair::load().check(I32_MIN + 1, -1);
}

#[test]
fn row34_minus_one_over_min_plus_one() {
    Pair::load().check(-1, I32_MIN + 1);
}

// ---------------------------------------------------------------------------
// Group F — INT_MIN < v1 < 0, v2 == INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn row35_neg_over_int_min() {
    let p = Pair::load();
    sweep(&p, N, |r| (r.range(I32_MIN + 1, -1), I32_MIN));
    for v1 in [-1, -2, I32_MIN + 1, I32_MIN + 2] {
        p.check(v1, I32_MIN);
    }
}

// ---------------------------------------------------------------------------
// Group G — v1 == INT_MIN, v2 > 0
// ---------------------------------------------------------------------------

#[test]
fn row36_int_min_over_one() {
    Pair::load().check(I32_MIN, 1);
}

#[test]
fn row37_int_min_over_two() {
    Pair::load().check(I32_MIN, 2);
}

#[test]
fn row38_int_min_over_three() {
    Pair::load().check(I32_MIN, 3);
}

#[test]
fn row39_int_min_over_max() {
    Pair::load().check(I32_MIN, I32_MAX);
}

#[test]
fn row40_int_min_over_pos_sweep() {
    let p = Pair::load();
    sweep(&p, N, |r| (I32_MIN, r.pos()));
    for v2 in 1..=4096 {
        p.check(I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// Group H — v1 == INT_MIN, INT_MIN < v2 < 0
// ---------------------------------------------------------------------------

#[test]
fn row41_int_min_over_minus_one() {
    // `q = INT_MAX + 1` overflows in the C. Whatever the compiled C `.so`
    // produces is the ground truth.
    Pair::load().check(I32_MIN, -1);
}

#[test]
fn row42_int_min_over_minus_two() {
    Pair::load().check(I32_MIN, -2);
}

#[test]
fn row43_int_min_over_minus_three() {
    Pair::load().check(I32_MIN, -3);
}

#[test]
fn row44_int_min_over_min_plus_one() {
    Pair::load().check(I32_MIN, I32_MIN + 1);
}

#[test]
fn row45_int_min_over_neg_sweep() {
    let p = Pair::load();
    sweep(&p, N, |r| (I32_MIN, r.neg_non_min()));
    for v2 in -4096..=-1 {
        p.check(I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// Group I
// ---------------------------------------------------------------------------

#[test]
fn row46_int_min_over_int_min() {
    Pair::load().check(I32_MIN, I32_MIN);
}

// ---------------------------------------------------------------------------
// Group J — saturation sweeps
// ---------------------------------------------------------------------------

#[test]
fn row47_boundary_cross_product() {
    let p = Pair::load();
    let vals = boundary_values();
    assert_eq!(vals.len(), 76, "boundary set size changed; update CONFIGS.md row 47");
    for &v1 in &vals {
        for &v2 in &vals {
            p.check(v1, v2);
        }
    }
}

#[test]
fn row48_exhaustive_small_band() {
    let p = Pair::load();
    for v1 in -400..=400 {
        for v2 in -400..=400 {
            p.check(v1, v2);
        }
    }
}

#[test]
fn row49_random_full_range() {
    let p = Pair::load();
    let mut r = Rng::fixed();
    for _ in 0..3_000_000 {
        let v1 = r.next_i32();
        let v2 = r.next_i32();
        p.check(v1, v2);
    }
}

#[test]
fn row50_boundary_times_random() {
    let p = Pair::load();
    let vals = boundary_values();
    let mut r = Rng::fixed();
    for i in 0..400_000usize {
        let b = vals[(r.next_u64() % vals.len() as u64) as usize];
        let x = r.next_i32();
        if i % 2 == 0 {
            p.check(b, x);
        } else {
            p.check(x, b);
        }
    }
}

#[test]
fn row51_small_divisor_full_numerator() {
    let p = Pair::load();
    let mut r = Rng::fixed();
    for _ in 0..400_000 {
        let v1 = r.next_i32();
        let mag = r.range(1, 64);
        let v2 = if r.next_u64() & 1 == 0 { mag } else { -mag };
        p.check(v1, v2);
    }
}

#[test]
fn row52_random_exact_multiples() {
    let p = Pair::load();
    let mut r = Rng::fixed();
    for _ in 0..400_000 {
        let mag = r.range(1, 1 << 20);
        let v2: c_int = if r.next_u64() & 1 == 0 { mag } else { -mag };
        let k = r.range(-2048, 2048);
        let v1 = k.wrapping_mul(v2);
        p.check(v1, v2);
    }
}
