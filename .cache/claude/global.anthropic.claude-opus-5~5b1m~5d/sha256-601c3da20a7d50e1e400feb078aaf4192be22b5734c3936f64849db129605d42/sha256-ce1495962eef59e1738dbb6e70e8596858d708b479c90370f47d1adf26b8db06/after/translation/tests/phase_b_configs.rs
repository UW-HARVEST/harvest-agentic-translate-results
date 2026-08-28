//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Row -> test mapping:
//!   1  cfg_row01_pos_pos_exact          22 cfg_row22_neg_neg_remainder
//!   2  cfg_row02_pos_pos_remainder      23 cfg_row23_neg_neg_small
//!   3  cfg_row03_pos_pos_small          24 cfg_row24_neg_neg_equal
//!   4  cfg_row04_pos_pos_equal          25 cfg_row25_neg_neg_v2_minus_one
//!   5  cfg_row05_zero_over_pos          26 cfg_row26_neg_neg_v2_min_plus_one
//!   6  cfg_row06_pos_over_one           27 cfg_row27_L6_neg_over_intmin
//!   7  cfg_row07_pos_over_intmax        28 cfg_row28_L7_intmin_over_pos_exact
//!   8  cfg_row08_pos_neg_exact          29 cfg_row29_L7_intmin_over_pos_remainder
//!   9  cfg_row09_pos_neg_remainder      30 cfg_row30_L7_intmin_over_one
//!   10 cfg_row10_pos_neg_small          31 cfg_row31_L7_intmin_over_intmax
//!   11 cfg_row11_pos_neg_equal          32 cfg_row32_L8_intmin_over_neg_exact
//!   12 cfg_row12_zero_over_neg          33 cfg_row33_L8_intmin_over_neg_remainder
//!   13 cfg_row13_pos_neg_extremes       34 cfg_row34_L8_intmin_over_minus_one
//!   14 cfg_row14_L3_nonneg_over_intmin  35 cfg_row35_L8_intmin_over_min_plus_one
//!   15 cfg_row15_neg_pos_exact          36 cfg_row36_L9_both_intmin
//!   16 cfg_row16_neg_pos_remainder      37 cfg_row37_v2_zero_all_v1_classes
//!   17 cfg_row17_neg_pos_small          38 cfg_row38_exhaustive_small_square
//!   18 cfg_row18_neg_pos_equal          39 cfg_row39_boundary_neighbourhood_cross
//!   19 cfg_row19_neg_over_one           40 cfg_row40_uniform_random_full_domain
//!   20 cfg_row20_neg_over_intmax        41 cfg_row41_single_axis_full_sweeps
//!   21 cfg_row21_neg_neg_exact          42 cfg_row42_structured_powers_of_two

mod common;

use common::{check, Cmp, Pcg32, BOUNDARIES, I32_MAX, I32_MIN};

/// Iterations per randomized row. Enough to hit many quotient magnitudes and
/// both tail branches, cheap enough to keep the whole suite well under the
/// command timeout.
const N: usize = 20_000;

/// Which leaf of `lib.c`'s `if`/`else` ladder `(v1, v2)` selects.
/// `0` = the `v2 == 0` early return, `1..=9` = leaves L1..L9.
fn leaf_of(v1: i32, v2: i32) -> usize {
    if v2 == 0 {
        return 0;
    }
    if v1 >= 0 {
        if v2 >= 0 {
            1
        } else if v2 != I32_MIN {
            2
        } else {
            3
        }
    } else if v1 != I32_MIN {
        if v2 >= 0 {
            4
        } else if v2 != I32_MIN {
            5
        } else {
            6
        }
    } else if v2 >= 0 {
        7
    } else if v2 != I32_MIN {
        8
    } else {
        9
    }
}

/// Assert the given inputs collectively reach every leaf listed in `want`.
fn assert_leaves_covered(pairs: &[(i32, i32)], want: &[usize], label: &str) {
    let mut seen = [false; 10];
    for &(v1, v2) in pairs {
        seen[leaf_of(v1, v2)] = true;
    }
    for &w in want {
        assert!(seen[w], "{label}: leaf L{w} was never exercised");
    }
}

// ---------------------------------------------------------------------------
// L1: v1 >= 0, v2 > 0  -> early `return v1 / v2` (line 10), tail skipped
// ---------------------------------------------------------------------------

#[test]
fn cfg_row01_pos_pos_exact() {
    let mut rng = Pcg32::new(0x0000_0001);
    let mut cmp = Cmp::new("row01 L1 pos/pos exact multiple");
    for _ in 0..N {
        let v2 = rng.pos();
        // k * v2 without overflow: pick k in [0, INT_MAX / v2]
        let kmax = I32_MAX / v2;
        let k = rng.i32_in(0, kmax);
        cmp.feed(k * v2, v2);
    }
    // hand-picked exact multiples
    for &(a, b) in &[(84, 12), (0, 7), (7, 7), (2147483646, 2), (1 << 30, 1 << 15)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row02_pos_pos_remainder() {
    let mut rng = Pcg32::new(0x0000_0002);
    let mut cmp = Cmp::new("row02 L1 pos/pos with remainder");
    for _ in 0..N {
        let v2 = rng.i32_in(2, I32_MAX);
        let v1 = rng.nonneg();
        cmp.feed(v1, v2);
    }
    for &(a, b) in &[(85, 12), (1, 2), (I32_MAX, 2), (I32_MAX, 3), (I32_MAX, I32_MAX - 1)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row03_pos_pos_small() {
    // 0 < v1 < v2  -> quotient 0
    let mut rng = Pcg32::new(0x0000_0003);
    let mut cmp = Cmp::new("row03 L1 |v1| < |v2|");
    for _ in 0..N {
        let v2 = rng.i32_in(2, I32_MAX);
        let v1 = rng.i32_in(0, v2 - 1);
        cmp.feed(v1, v2);
    }
    for &(a, b) in &[(1, 2), (0, 1), (I32_MAX - 1, I32_MAX), (5, 1000)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row04_pos_pos_equal() {
    let mut rng = Pcg32::new(0x0000_0004);
    let mut cmp = Cmp::new("row04 L1 v1 == v2");
    for _ in 0..N {
        let v = rng.pos();
        cmp.feed(v, v);
    }
    for &v in &[1, 2, 3, I32_MAX - 1, I32_MAX] {
        cmp.feed(v, v);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row05_zero_over_pos() {
    let mut rng = Pcg32::new(0x0000_0005);
    let mut cmp = Cmp::new("row05 L1 v1 == 0, v2 > 0");
    for _ in 0..N {
        cmp.feed(0, rng.pos());
    }
    for &b in &[1, 2, 3, I32_MAX - 1, I32_MAX] {
        cmp.feed(0, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row06_pos_over_one() {
    // v2 == 1 -> full-width quotient
    let mut rng = Pcg32::new(0x0000_0006);
    let mut cmp = Cmp::new("row06 L1 v2 == 1");
    for _ in 0..N {
        cmp.feed(rng.nonneg(), 1);
    }
    for &a in &[0, 1, 2, I32_MAX - 1, I32_MAX] {
        cmp.feed(a, 1);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row07_pos_over_intmax() {
    let mut cmp = Cmp::new("row07 L1 v2 near INT_MAX");
    let mut rng = Pcg32::new(0x0000_0007);
    for &v2 in &[I32_MAX, I32_MAX - 1, I32_MAX - 2] {
        for &v1 in &[0, 1, I32_MAX - 2, I32_MAX - 1, I32_MAX] {
            cmp.feed(v1, v2);
        }
        for _ in 0..N / 3 {
            cmp.feed(rng.nonneg(), v2);
        }
    }
    cmp.finish(N as u64 - 3);
}

// ---------------------------------------------------------------------------
// L2: v1 >= 0, v2 < 0, v2 != INT_MIN (line 12)
//     q = -(v1 / -v2), r = v1 % (-v2)   <-- r NOT negated
// ---------------------------------------------------------------------------

#[test]
fn cfg_row08_pos_neg_exact() {
    let mut rng = Pcg32::new(0x0000_0008);
    let mut cmp = Cmp::new("row08 L2 pos/neg exact multiple");
    for _ in 0..N {
        let v2 = rng.neg_nonmin();
        let m = v2.wrapping_neg(); // in [1, INT_MAX]
        let k = rng.i32_in(0, I32_MAX / m);
        cmp.feed(k * m, v2);
    }
    for &(a, b) in &[(84, -12), (0, -7), (7, -7), (2147483646, -2)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row09_pos_neg_remainder() {
    let mut rng = Pcg32::new(0x0000_0009);
    let mut cmp = Cmp::new("row09 L2 pos/neg with remainder (non-negated r)");
    for _ in 0..N {
        let v2 = rng.i32_in(I32_MIN + 1, -2);
        cmp.feed(rng.nonneg(), v2);
    }
    for &(a, b) in &[(85, -12), (1, -2), (I32_MAX, -2), (I32_MAX, -3), (7, -3)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row10_pos_neg_small() {
    let mut rng = Pcg32::new(0x0000_000a);
    let mut cmp = Cmp::new("row10 L2 0 < v1 < -v2");
    for _ in 0..N {
        let v2 = rng.i32_in(I32_MIN + 1, -2);
        let m = v2.wrapping_neg();
        cmp.feed(rng.i32_in(0, m - 1), v2);
    }
    for &(a, b) in &[(1, -2), (0, -1), (I32_MAX - 1, I32_MIN + 1), (5, -1000)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row11_pos_neg_equal() {
    let mut rng = Pcg32::new(0x0000_000b);
    let mut cmp = Cmp::new("row11 L2 v1 == -v2");
    for _ in 0..N {
        let v2 = rng.neg_nonmin();
        cmp.feed(v2.wrapping_neg(), v2);
    }
    for &b in &[-1, -2, -3, I32_MIN + 2, I32_MIN + 1] {
        cmp.feed(b.wrapping_neg(), b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row12_zero_over_neg() {
    let mut rng = Pcg32::new(0x0000_000c);
    let mut cmp = Cmp::new("row12 L2 v1 == 0, v2 < 0");
    for _ in 0..N {
        cmp.feed(0, rng.neg_nonmin());
    }
    for &b in &[-1, -2, -3, I32_MIN + 1, I32_MIN] {
        cmp.feed(0, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row13_pos_neg_extremes() {
    let mut rng = Pcg32::new(0x0000_000d);
    let mut cmp = Cmp::new("row13 L2 v2 == -1 / INT_MIN+1");
    for &v2 in &[-1, I32_MIN + 1, I32_MIN + 2] {
        for &v1 in &[0, 1, 2, I32_MAX - 1, I32_MAX] {
            cmp.feed(v1, v2);
        }
        for _ in 0..N / 3 {
            cmp.feed(rng.nonneg(), v2);
        }
    }
    cmp.finish(N as u64 - 3);
}

// ---------------------------------------------------------------------------
// L3: v1 >= 0, v2 == INT_MIN (line 14) -> q = 0, r = v1 -> returns 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row14_l3_nonneg_over_intmin() {
    let mut rng = Pcg32::new(0x0000_000e);
    let mut cmp = Cmp::new("row14 L3 v1 >= 0, v2 == INT_MIN");
    for _ in 0..N {
        cmp.feed(rng.nonneg(), I32_MIN);
    }
    for &v1 in &[0, 1, 2, 1 << 30, I32_MAX - 1, I32_MAX] {
        // C: q=0, r=v1>=0 -> return 0
        common::check_eq(v1, I32_MIN, 0);
        cmp.feed(v1, I32_MIN);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// L4: v1 < 0 (not MIN), v2 > 0 (line 17)
//     q = -((-v1)/v2), r = -((-v1)%v2)  -> tail q-1 when r<0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row15_neg_pos_exact() {
    let mut rng = Pcg32::new(0x0000_000f);
    let mut cmp = Cmp::new("row15 L4 neg/pos exact multiple");
    for _ in 0..N {
        let v2 = rng.pos();
        let k = rng.i32_in(0, I32_MAX / v2);
        cmp.feed((k * v2).wrapping_neg(), v2);
    }
    for &(a, b) in &[(-84, 12), (-7, 7), (-2147483646, 2), (I32_MIN + 1, 1)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row16_neg_pos_remainder() {
    let mut rng = Pcg32::new(0x0000_0010);
    let mut cmp = Cmp::new("row16 L4 neg/pos with remainder -> q-1");
    for _ in 0..N {
        let v2 = rng.i32_in(2, I32_MAX);
        cmp.feed(rng.neg_nonmin(), v2);
    }
    for &(a, b) in &[(-85, 12), (-1, 2), (I32_MIN + 1, 2), (I32_MIN + 1, 3), (-7, 3)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row17_neg_pos_small() {
    let mut rng = Pcg32::new(0x0000_0011);
    let mut cmp = Cmp::new("row17 L4 -v2 < v1 < 0");
    for _ in 0..N {
        let v2 = rng.i32_in(2, I32_MAX);
        cmp.feed(rng.i32_in(-(v2 - 1), -1), v2);
    }
    for &(a, b) in &[(-1, 2), (-1, 1), (I32_MIN + 1, I32_MAX), (-5, 1000)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row18_neg_pos_equal() {
    let mut rng = Pcg32::new(0x0000_0012);
    let mut cmp = Cmp::new("row18 L4 v1 == -v2");
    for _ in 0..N {
        let v2 = rng.pos();
        cmp.feed(v2.wrapping_neg(), v2);
    }
    for &b in &[1, 2, 3, I32_MAX - 1, I32_MAX] {
        cmp.feed(b.wrapping_neg(), b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row19_neg_over_one() {
    let mut rng = Pcg32::new(0x0000_0013);
    let mut cmp = Cmp::new("row19 L4 v2 == 1");
    for _ in 0..N {
        cmp.feed(rng.neg_nonmin(), 1);
    }
    for &a in &[-1, -2, I32_MIN + 2, I32_MIN + 1] {
        cmp.feed(a, 1);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row20_neg_over_intmax() {
    let mut rng = Pcg32::new(0x0000_0014);
    let mut cmp = Cmp::new("row20 L4 v2 near INT_MAX");
    for &v2 in &[I32_MAX, I32_MAX - 1, I32_MAX - 2] {
        for &v1 in &[-1, -2, I32_MIN + 2, I32_MIN + 1] {
            cmp.feed(v1, v2);
        }
        for _ in 0..N / 3 {
            cmp.feed(rng.neg_nonmin(), v2);
        }
    }
    cmp.finish(N as u64 - 3);
}

// ---------------------------------------------------------------------------
// L5: v1 < 0 (not MIN), v2 < 0 (not MIN) (line 19)
//     q = (-v1)/(-v2), r = -((-v1)%(-v2))  -> tail q+1 when r<0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row21_neg_neg_exact() {
    let mut rng = Pcg32::new(0x0000_0015);
    let mut cmp = Cmp::new("row21 L5 neg/neg exact multiple");
    for _ in 0..N {
        let v2 = rng.neg_nonmin();
        let m = v2.wrapping_neg();
        let k = rng.i32_in(0, I32_MAX / m);
        cmp.feed((k * m).wrapping_neg(), v2);
    }
    for &(a, b) in &[(-84, -12), (-7, -7), (-2147483646, -2), (I32_MIN + 1, -1)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row22_neg_neg_remainder() {
    let mut rng = Pcg32::new(0x0000_0016);
    let mut cmp = Cmp::new("row22 L5 neg/neg with remainder -> q+1");
    for _ in 0..N {
        let v2 = rng.i32_in(I32_MIN + 1, -2);
        cmp.feed(rng.neg_nonmin(), v2);
    }
    for &(a, b) in &[(-85, -12), (-1, -2), (I32_MIN + 1, -2), (I32_MIN + 1, -3), (-7, -3)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row23_neg_neg_small() {
    let mut rng = Pcg32::new(0x0000_0017);
    let mut cmp = Cmp::new("row23 L5 |v1| < |v2|");
    for _ in 0..N {
        let v2 = rng.i32_in(I32_MIN + 1, -2);
        let m = v2.wrapping_neg();
        cmp.feed(rng.i32_in(-(m - 1), -1), v2);
    }
    for &(a, b) in &[(-1, -2), (I32_MIN + 1, I32_MIN + 1), (-5, -1000)] {
        cmp.feed(a, b);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row24_neg_neg_equal() {
    let mut rng = Pcg32::new(0x0000_0018);
    let mut cmp = Cmp::new("row24 L5 v1 == v2");
    for _ in 0..N {
        let v = rng.neg_nonmin();
        cmp.feed(v, v);
    }
    for &v in &[-1, -2, -3, I32_MIN + 2, I32_MIN + 1] {
        cmp.feed(v, v);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row25_neg_neg_v2_minus_one() {
    let mut rng = Pcg32::new(0x0000_0019);
    let mut cmp = Cmp::new("row25 L5 v2 == -1");
    for _ in 0..N {
        cmp.feed(rng.neg_nonmin(), -1);
    }
    for &a in &[-1, -2, I32_MIN + 2, I32_MIN + 1] {
        cmp.feed(a, -1);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row26_neg_neg_v2_min_plus_one() {
    let mut rng = Pcg32::new(0x0000_001a);
    let mut cmp = Cmp::new("row26 L5 v2 == INT_MIN+1");
    for &v2 in &[I32_MIN + 1, I32_MIN + 2] {
        for &v1 in &[-1, -2, I32_MIN + 2, I32_MIN + 1] {
            cmp.feed(v1, v2);
        }
        for _ in 0..N / 2 {
            cmp.feed(rng.neg_nonmin(), v2);
        }
    }
    cmp.finish(N as u64 - 2);
}

// ---------------------------------------------------------------------------
// L6: v1 < 0 (not MIN), v2 == INT_MIN (line 21) -> q=1, r=v1-INT_MIN>0 -> 1
// ---------------------------------------------------------------------------

#[test]
fn cfg_row27_l6_neg_over_intmin() {
    let mut rng = Pcg32::new(0x0000_001b);
    let mut cmp = Cmp::new("row27 L6 v1<0 (not MIN), v2 == INT_MIN");
    for _ in 0..N {
        cmp.feed(rng.neg_nonmin(), I32_MIN);
    }
    for &v1 in &[-1, -2, -(1 << 30), I32_MIN + 2, I32_MIN + 1] {
        common::check_eq(v1, I32_MIN, 1);
        cmp.feed(v1, I32_MIN);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// L7: v1 == INT_MIN, v2 > 0 (line 23) -- the -(v1+v2) rewrite
// ---------------------------------------------------------------------------

/// Positive divisors of 2^31 (so that INT_MIN is an exact multiple).
const POW2_POS_DIVISORS: &[i32] = &[
    1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
    65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216,
    33554432, 67108864, 134217728, 268435456, 536870912, 1073741824,
];

#[test]
fn cfg_row28_l7_intmin_over_pos_exact() {
    let mut cmp = Cmp::new("row28 L7 INT_MIN / positive divisor (exact)");
    for &v2 in POW2_POS_DIVISORS {
        cmp.feed(I32_MIN, v2);
    }
    cmp.finish(POW2_POS_DIVISORS.len() as u64);
}

#[test]
fn cfg_row29_l7_intmin_over_pos_remainder() {
    let mut rng = Pcg32::new(0x0000_001c);
    let mut cmp = Cmp::new("row29 L7 INT_MIN / positive non-divisor -> q-1");
    for _ in 0..N {
        cmp.feed(I32_MIN, rng.pos());
    }
    for &v2 in &[3, 5, 6, 7, 9, 10, 100, 1000, 12345, (1 << 30) - 1, (1 << 30) + 1] {
        cmp.feed(I32_MIN, v2);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row30_l7_intmin_over_one() {
    // The -(v1+v2) rewrite at its extreme: v2 == 1
    common::check_eq(I32_MIN, 1, I32_MIN);
    check(I32_MIN, 1);
    check(I32_MIN, 2);
    check(I32_MIN, 3);
}

#[test]
fn cfg_row31_l7_intmin_over_intmax() {
    for &v2 in &[I32_MAX, I32_MAX - 1, I32_MAX - 2, (1 << 30) + 1, 1 << 30] {
        check(I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// L8: v1 == INT_MIN, v2 < 0 (not MIN) (line 25) -- the -(v1-v2) rewrite
// ---------------------------------------------------------------------------

#[test]
fn cfg_row32_l8_intmin_over_neg_exact() {
    let mut cmp = Cmp::new("row32 L8 INT_MIN / negative divisor (exact)");
    for &m in POW2_POS_DIVISORS {
        cmp.feed(I32_MIN, m.wrapping_neg());
    }
    cmp.finish(POW2_POS_DIVISORS.len() as u64);
}

#[test]
fn cfg_row33_l8_intmin_over_neg_remainder() {
    let mut rng = Pcg32::new(0x0000_001d);
    let mut cmp = Cmp::new("row33 L8 INT_MIN / negative non-divisor -> q+1");
    for _ in 0..N {
        cmp.feed(I32_MIN, rng.neg_nonmin());
    }
    for &v2 in &[-3, -5, -6, -7, -9, -10, -100, -1000, -12345, -((1 << 30) - 1)] {
        cmp.feed(I32_MIN, v2);
    }
    cmp.finish(N as u64);
}

#[test]
fn cfg_row34_l8_intmin_over_minus_one() {
    // Signed-overflow path: q = INT_MAX + 1. The -O0 C build wraps to INT_MIN.
    common::check_eq(I32_MIN, -1, I32_MIN);
}

#[test]
fn cfg_row35_l8_intmin_over_min_plus_one() {
    for &v2 in &[I32_MIN + 1, I32_MIN + 2, I32_MIN + 3, -(1 << 30), -((1 << 30) + 1)] {
        check(I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// L9: both INT_MIN (line 27) -> q=1, r=0 -> 1
// ---------------------------------------------------------------------------

#[test]
fn cfg_row36_l9_both_intmin() {
    common::check_eq(I32_MIN, I32_MIN, 1);
}

// ---------------------------------------------------------------------------
// v2 == 0 across every v1 class (valid input with a defined result)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row37_v2_zero_all_v1_classes() {
    let mut rng = Pcg32::new(0x0000_001e);
    let mut cmp = Cmp::new("row37 v2 == 0");
    for &v1 in BOUNDARIES {
        common::check_eq(v1, 0, 0);
        cmp.feed(v1, 0);
    }
    for _ in 0..N {
        cmp.feed(rng.i32_any(), 0);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Broad sweeps
// ---------------------------------------------------------------------------

#[test]
fn cfg_row38_exhaustive_small_square() {
    // Every (v1, v2) in [-512, 512]^2: 1_050_625 pairs. Covers all 9 leaves
    // (except the INT_MIN ones), both tail branches, all divisibility shapes.
    let (c, r) = common::funcs();
    let mut n: u64 = 0;
    for v1 in -512..=512i32 {
        for v2 in -512..=512i32 {
            let cv = unsafe { c(v1, v2) };
            let rv = unsafe { r(v1, v2) };
            assert_eq!(cv, rv, "DIVERGENCE div_euclid({v1}, {v2}): C={cv} Rust={rv}");
            n += 1;
        }
    }
    assert_eq!(n, 1025 * 1025);
}

/// Meta-test: the boundary cross-product provably reaches **all ten** control
/// paths of `lib.c` (the `v2 == 0` early return plus leaves L1..L9), so no leaf
/// can be silently unexercised by the suite.
#[test]
fn cfg_meta_all_ten_control_paths_reached() {
    let mut vals: Vec<i32> = Vec::new();
    for k in 0..=8 {
        vals.push(I32_MIN + k);
        vals.push(I32_MAX - k);
    }
    for v in -8..=8 {
        vals.push(v);
    }
    vals.sort_unstable();
    vals.dedup();

    let mut pairs = Vec::new();
    for &v1 in &vals {
        for &v2 in &vals {
            pairs.push((v1, v2));
        }
    }
    assert_leaves_covered(
        &pairs,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        "boundary cross-product",
    );

    // ...and every one of those pairs agrees between C and Rust.
    let mut cmp = Cmp::new("meta all-ten-paths");
    for &(v1, v2) in &pairs {
        cmp.feed(v1, v2);
    }
    cmp.finish(pairs.len() as u64);
}

#[test]
fn cfg_row39_boundary_neighbourhood_cross() {
    // {INT_MIN..INT_MIN+8} u {-8..8} u {INT_MAX-8..INT_MAX}, squared.
    let mut vals: Vec<i32> = Vec::new();
    for k in 0..=8 {
        vals.push(I32_MIN + k);
        vals.push(I32_MAX - k);
    }
    for v in -8..=8 {
        vals.push(v);
    }
    vals.sort_unstable();
    vals.dedup();

    let mut cmp = Cmp::new("row39 boundary neighbourhood cross-product");
    for &v1 in &vals {
        for &v2 in &vals {
            cmp.feed(v1, v2);
        }
    }
    let n = vals.len() as u64;
    cmp.finish(n * n);
}

#[test]
fn cfg_row40_uniform_random_full_domain() {
    // Uniform over the entire 2 x 32-bit domain.
    let mut rng = Pcg32::new(0xDEAD_BEEF_CAFE_F00D);
    let (c, r) = common::funcs();
    let mut bad = Vec::new();
    let iters = 4_000_000u32;
    let mut n: u64 = 0;
    // Track which of the 9 leaf branches the random stream actually reached, so
    // "4 million pairs" cannot silently degenerate into one code path.
    let mut leaves = [0u64; 10];
    for _ in 0..iters {
        let v1 = rng.i32_any();
        let v2 = rng.i32_any();
        let cv = unsafe { c(v1, v2) };
        let rv = unsafe { r(v1, v2) };
        n += 1;
        if cv != rv && bad.len() < 40 {
            bad.push((v1, v2, cv, rv));
        }
        leaves[leaf_of(v1, v2)] += 1;
    }
    assert!(bad.is_empty(), "row40 divergences: {bad:?}");
    assert_eq!(n, iters as u64, "row40 did not run the full iteration count");
    // v2 == 0 (index 0) and the INT_MIN leaves are astronomically unlikely from
    // a uniform stream; the four bulk leaves L1/L2/L4/L5 must all be well hit.
    for idx in [1usize, 2, 4, 5] {
        assert!(
            leaves[idx] > iters as u64 / 8,
            "row40 leaf L{idx} hit only {} times out of {iters}: {leaves:?}",
            leaves[idx]
        );
    }
}

#[test]
fn cfg_row41_single_axis_full_sweeps() {
    // Pin one argument to each boundary representative and walk the *entire*
    // 2^32 range of the other with a large prime stride, so every region of the
    // domain (and both signs, and INT_MIN/INT_MAX) is visited.
    const STRIDE: i64 = 65_519; // prime
    let (c, r) = common::funcs();
    let mut bad = Vec::new();
    let mut n: u64 = 0;

    for &pin in BOUNDARIES {
        let mut x: i64 = I32_MIN as i64;
        while x <= I32_MAX as i64 {
            let other = x as i32;
            for &(v1, v2) in &[(pin, other), (other, pin)] {
                let cv = unsafe { c(v1, v2) };
                let rv = unsafe { r(v1, v2) };
                n += 1;
                if cv != rv && bad.len() < 40 {
                    bad.push((v1, v2, cv, rv));
                }
            }
            x += STRIDE;
        }
    }
    assert!(bad.is_empty(), "row41 divergences: {bad:?}");
    assert!(n > 2_000_000, "row41 made only {n} comparisons");
}

#[test]
fn cfg_row42_structured_powers_of_two() {
    // +-(2^k), +-(2^k +- 1) for all k in 0..32, every sign combination.
    let mut vals: Vec<i32> = Vec::new();
    for k in 0..32u32 {
        let p: i64 = 1i64 << k;
        for d in [-1i64, 0, 1] {
            let v = p + d;
            for s in [1i64, -1] {
                let w = s * v;
                if w >= I32_MIN as i64 && w <= I32_MAX as i64 {
                    vals.push(w as i32);
                }
            }
        }
    }
    vals.push(I32_MIN);
    vals.push(I32_MAX);
    vals.push(0);
    vals.sort_unstable();
    vals.dedup();

    let mut cmp = Cmp::new("row42 structured power-of-two operands");
    for &v1 in &vals {
        for &v2 in &vals {
            cmp.feed(v1, v2);
        }
    }
    let n = vals.len() as u64;
    cmp.finish(n * n);
}
