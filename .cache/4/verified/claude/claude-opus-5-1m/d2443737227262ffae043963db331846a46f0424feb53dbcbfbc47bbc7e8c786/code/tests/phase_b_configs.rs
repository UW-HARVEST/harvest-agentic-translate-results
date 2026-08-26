//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `div_euclid` symbol (loaded with `libloading`) and requires byte-identical
//! results. Inputs are property-style: many randomized values per row from a
//! fixed-seed SplitMix64 PRNG, plus the row's boundary values.

mod common;

use common::{assert_all_same, assert_same, assert_same_raw, boundary_values, Rng, I32_MAX, I32_MIN};

/// iterations per randomized row
const N: usize = 20_000;

// ---------------------------------------------------------------------------
// C1 — P1: zero divisor
// ---------------------------------------------------------------------------
#[test]
fn c1_zero_divisor_random_v1() {
    let mut rng = Rng::new(0xC001);
    for i in 0..N {
        let v1 = match i {
            0 => 0,
            1 => I32_MIN,
            2 => I32_MAX,
            3 => -1,
            4 => 1,
            _ => rng.next_i32(),
        };
        assert_same("C1", v1, 0);
    }
}

// ---------------------------------------------------------------------------
// C2..C6 — P2: v1 >= 0, v2 > 0 (the early-return leaf)
// ---------------------------------------------------------------------------
#[test]
fn c2_pos_pos_non_divisible() {
    let mut rng = Rng::new(0xC002);
    let mut n = 0;
    while n < N {
        let v2 = rng.range_i32(2, I32_MAX);
        let v1 = rng.range_i32(1, I32_MAX);
        if v1 % v2 == 0 {
            continue;
        }
        assert_same("C2", v1, v2);
        n += 1;
    }
}

#[test]
fn c3_pos_pos_exact_multiple() {
    let mut rng = Rng::new(0xC003);
    for _ in 0..N {
        let v2 = rng.range_i32(1, 1 << 20);
        let m = rng.range_i32(0, I32_MAX / v2);
        let v1 = m.wrapping_mul(v2);
        assert_same("C3", v1, v2);
    }
}

#[test]
fn c4_pos_pos_quotient_zero() {
    let mut rng = Rng::new(0xC004);
    for _ in 0..N {
        let v2 = rng.range_i32(2, I32_MAX);
        let v1 = rng.range_i32(0, v2 - 1);
        assert_same("C4", v1, v2);
    }
}

#[test]
fn c5_zero_over_positive() {
    let mut rng = Rng::new(0xC005);
    for i in 0..N {
        let v2 = match i {
            0 => 1,
            1 => 2,
            2 => I32_MAX,
            _ => rng.pos_i32(I32_MAX),
        };
        assert_same("C5", 0, v2);
    }
}

#[test]
fn c6_pos_pos_extremes() {
    for v2 in [1, 2, 3, I32_MAX - 1, I32_MAX] {
        assert_same("C6", I32_MAX, v2);
        assert_same("C6", I32_MAX - 1, v2);
        assert_same("C6", 0, v2);
    }
    let mut rng = Rng::new(0xC006);
    for _ in 0..N {
        let v1 = rng.range_i32(0, I32_MAX);
        assert_same("C6", v1, 1);
        assert_same("C6", v1, I32_MAX);
    }
}

// ---------------------------------------------------------------------------
// C7..C11 — P3: v1 >= 0, INT_MIN < v2 < 0
// ---------------------------------------------------------------------------
#[test]
fn c7_pos_neg_non_divisible() {
    let mut rng = Rng::new(0xC007);
    let mut n = 0;
    while n < N {
        let v2 = rng.range_i32(I32_MIN + 1, -2);
        let v1 = rng.range_i32(1, I32_MAX);
        if v1 % v2.wrapping_neg() == 0 {
            continue;
        }
        assert_same("C7", v1, v2);
        n += 1;
    }
}

#[test]
fn c8_pos_neg_exact_multiple() {
    let mut rng = Rng::new(0xC008);
    for _ in 0..N {
        let d = rng.range_i32(1, 1 << 20); // |v2|
        let m = rng.range_i32(0, I32_MAX / d);
        assert_same("C8", m.wrapping_mul(d), -d);
    }
}

#[test]
fn c9_nonneg_over_minus_one() {
    let mut rng = Rng::new(0xC009);
    for i in 0..N {
        let v1 = match i {
            0 => 0,
            1 => 1,
            2 => I32_MAX,
            3 => I32_MAX - 1,
            _ => rng.range_i32(0, I32_MAX),
        };
        assert_same("C9", v1, -1);
    }
}

#[test]
fn c10_zero_over_negative() {
    let mut rng = Rng::new(0xC010);
    for i in 0..N {
        let v2 = match i {
            0 => -1,
            1 => -2,
            2 => I32_MIN + 1,
            _ => rng.range_i32(I32_MIN + 1, -1),
        };
        assert_same("C10", 0, v2);
    }
}

#[test]
fn c11_pos_neg_quotient_zero() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..N {
        let d = rng.range_i32(2, I32_MAX); // |v2|
        let v1 = rng.range_i32(0, d - 1);
        assert_same("C11", v1, -d);
    }
}

// ---------------------------------------------------------------------------
// C12..C14 — P4: v1 >= 0, v2 == INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn c12_pos_over_int_min() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..N {
        let v1 = rng.range_i32(1, I32_MAX);
        assert_same("C12", v1, I32_MIN);
    }
}

#[test]
fn c13_zero_over_int_min() {
    assert_same("C13", 0, I32_MIN);
}

#[test]
fn c14_int_max_over_int_min() {
    for v1 in [I32_MAX, I32_MAX - 1, 1 << 30, (1 << 30) + 1, 1] {
        assert_same("C14", v1, I32_MIN);
    }
}

// ---------------------------------------------------------------------------
// C15..C18 — P5: INT_MIN < v1 < 0, v2 > 0
// ---------------------------------------------------------------------------
#[test]
fn c15_neg_pos_non_divisible() {
    let mut rng = Rng::new(0xC015);
    let mut n = 0;
    while n < N {
        let v1 = rng.range_i32(I32_MIN + 1, -1);
        let v2 = rng.range_i32(2, I32_MAX);
        if v1.wrapping_neg() % v2 == 0 {
            continue;
        }
        assert_same("C15", v1, v2);
        n += 1;
    }
}

#[test]
fn c16_neg_pos_exact_multiple() {
    let mut rng = Rng::new(0xC016);
    for _ in 0..N {
        let v2 = rng.range_i32(1, 1 << 20);
        let m = rng.range_i32(1, I32_MAX / v2);
        let v1 = m.wrapping_mul(v2).wrapping_neg();
        assert_same("C16", v1, v2);
    }
}

#[test]
fn c17_neg_over_one() {
    let mut rng = Rng::new(0xC017);
    for i in 0..N {
        let v1 = match i {
            0 => -1,
            1 => I32_MIN + 1,
            _ => rng.range_i32(I32_MIN + 1, -1),
        };
        assert_same("C17", v1, 1);
    }
}

#[test]
fn c18_neg_pos_quotient_zero() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..N {
        let v2 = rng.range_i32(2, I32_MAX);
        let v1 = rng.range_i32(1, v2 - 1).wrapping_neg();
        assert_same("C18", v1, v2);
    }
}

// ---------------------------------------------------------------------------
// C19..C22 — P6: INT_MIN < v1 < 0, INT_MIN < v2 < 0
// ---------------------------------------------------------------------------
#[test]
fn c19_neg_neg_non_divisible() {
    let mut rng = Rng::new(0xC019);
    let mut n = 0;
    while n < N {
        let v1 = rng.range_i32(I32_MIN + 1, -1);
        let v2 = rng.range_i32(I32_MIN + 1, -2);
        if v1.wrapping_neg() % v2.wrapping_neg() == 0 {
            continue;
        }
        assert_same("C19", v1, v2);
        n += 1;
    }
}

#[test]
fn c20_neg_neg_exact_multiple() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..N {
        let d = rng.range_i32(1, 1 << 20);
        let m = rng.range_i32(1, I32_MAX / d);
        assert_same("C20", m.wrapping_mul(d).wrapping_neg(), -d);
    }
}

#[test]
fn c21_neg_over_minus_one() {
    let mut rng = Rng::new(0xC021);
    for i in 0..N {
        let v1 = match i {
            0 => -1,
            1 => -2,
            2 => I32_MIN + 1, // -> INT_MAX
            3 => I32_MIN + 2,
            _ => rng.range_i32(I32_MIN + 1, -1),
        };
        assert_same("C21", v1, -1);
    }
}

#[test]
fn c22_neg_neg_quotient_zero() {
    let mut rng = Rng::new(0xC022);
    for _ in 0..N {
        let d = rng.range_i32(2, I32_MAX);
        let v1 = rng.range_i32(1, d - 1).wrapping_neg();
        assert_same("C22", v1, -d);
    }
}

// ---------------------------------------------------------------------------
// C23..C24 — P7: INT_MIN < v1 < 0, v2 == INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn c23_neg_over_int_min() {
    let mut rng = Rng::new(0xC023);
    for _ in 0..N {
        let v1 = rng.range_i32(I32_MIN + 1, -1);
        assert_same("C23", v1, I32_MIN);
    }
}

#[test]
fn c24_neg_over_int_min_boundaries() {
    for v1 in [-1, -2, I32_MIN + 1, I32_MIN + 2, -(1 << 30), -((1 << 30) + 1)] {
        assert_same("C24", v1, I32_MIN);
    }
}

// ---------------------------------------------------------------------------
// C25..C28 — P8: v1 == INT_MIN, v2 > 0
// ---------------------------------------------------------------------------
#[test]
fn c25_int_min_over_positive_non_divisible() {
    let mut rng = Rng::new(0xC025);
    let mut n = 0;
    while n < N {
        let v2 = rng.range_i32(2, I32_MAX);
        // non-divisible: INT_MIN % v2 != 0
        if (I32_MIN as i64) % (v2 as i64) == 0 {
            continue;
        }
        assert_same("C25", I32_MIN, v2);
        n += 1;
    }
}

#[test]
fn c26_int_min_over_power_of_two() {
    for k in 0..31u32 {
        assert_same("C26", I32_MIN, 1i32 << k);
    }
}

#[test]
fn c27_int_min_over_one() {
    assert_same("C27", I32_MIN, 1);
    assert_same("C27", I32_MIN, 2);
    assert_same("C27", I32_MIN, 3);
}

#[test]
fn c28_int_min_over_positive_extremes() {
    for v2 in [
        I32_MAX,
        I32_MAX - 1,
        1 << 30,
        (1 << 30) + 1,
        (1 << 30) - 1,
        1_073_741_824,
    ] {
        assert_same("C28", I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// C29..C32 — P9: v1 == INT_MIN, INT_MIN < v2 < 0
// ---------------------------------------------------------------------------
#[test]
fn c29_int_min_over_negative_non_divisible() {
    let mut rng = Rng::new(0xC029);
    let mut n = 0;
    while n < N {
        let v2 = rng.range_i32(I32_MIN + 1, -2);
        if (I32_MIN as i64) % (v2 as i64) == 0 {
            continue;
        }
        assert_same("C29", I32_MIN, v2);
        n += 1;
    }
}

#[test]
fn c30_int_min_over_negative_power_of_two() {
    for k in 0..31u32 {
        assert_same("C30", I32_MIN, -(1i32 << k));
    }
}

#[test]
fn c31_int_min_over_minus_one() {
    // C performs a signed overflow here (INT_MAX + 1); the Rust must wrap the
    // same way the compiled C does.
    assert_same("C31", I32_MIN, -1);
}

#[test]
fn c32_int_min_over_negative_extremes() {
    for v2 in [
        I32_MIN + 1,
        I32_MIN + 2,
        -(1 << 30),
        -((1 << 30) + 1),
        -((1 << 30) - 1),
        -2,
        -3,
    ] {
        assert_same("C32", I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// C33 — P10
// ---------------------------------------------------------------------------
#[test]
fn c33_int_min_over_int_min() {
    assert_same("C33", I32_MIN, I32_MIN);
}

// ---------------------------------------------------------------------------
// C34 — full cross product of the curated boundary set
// ---------------------------------------------------------------------------
#[test]
fn c34_boundary_cross_product() {
    let vals = boundary_values();
    assert!(vals.len() > 200, "boundary set too small: {}", vals.len());
    let mut pairs = 0usize;
    for &v1 in &vals {
        for &v2 in &vals {
            assert_same("C34", v1, v2);
            pairs += 1;
        }
    }
    assert!(pairs > 40_000, "only {pairs} pairs");
}

// ---------------------------------------------------------------------------
// C35 — dense contiguous sweeps around the interesting neighbourhoods
// ---------------------------------------------------------------------------
#[test]
fn c35_dense_sweeps() {
    let divisors = [
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        I32_MAX,
        -I32_MAX,
        I32_MIN,
        1 << 30,
        -(1 << 30),
    ];
    let mut v1s: Vec<i32> = Vec::new();
    for d in 0..=512i32 {
        v1s.push(I32_MIN.wrapping_add(d));
        v1s.push(I32_MAX - d);
        v1s.push(d);
        v1s.push(-d);
    }
    for &v1 in &v1s {
        for &v2 in &divisors {
            assert_same("C35", v1, v2);
        }
    }
}

// ---------------------------------------------------------------------------
// C36 — uniform random over the whole 2^64 input space
// ---------------------------------------------------------------------------
#[test]
fn c36_uniform_random_pairs() {
    let mut rng = Rng::new(0xDEAD_BEEF_C036);
    for _ in 0..2_000_000 {
        let v1 = rng.next_i32();
        let v2 = rng.next_i32();
        assert_same("C36", v1, v2);
    }
}

// ---------------------------------------------------------------------------
// C37 — structured random divisors + constructed multiples
// ---------------------------------------------------------------------------
#[test]
fn c37_structured_random() {
    let mut rng = Rng::new(0xC037);
    let mut divisors: Vec<i32> = vec![I32_MIN, I32_MAX, -I32_MAX];
    for k in 0..31u32 {
        let p = 1i32 << k;
        for d in [-1i32, 0, 1] {
            if let Some(x) = p.checked_add(d) {
                divisors.push(x);
                divisors.push(x.wrapping_neg());
            }
        }
    }
    for _ in 0..N {
        let v2 = divisors[(rng.next_u32() as usize) % divisors.len()];
        // fully random dividend
        assert_same("C37", rng.next_i32(), v2);
        // dividend built as an exact multiple of v2, and one step off it
        if v2 != 0 {
            let q = rng.next_i32();
            let base = (q as i64).wrapping_mul(v2 as i64) as i32; // wraps like C
            for delta in [-2i32, -1, 0, 1, 2] {
                assert_same("C37", base.wrapping_add(delta), v2);
            }
        }
    }
    // and every divisor against every boundary dividend
    let vals = boundary_values();
    for &v2 in &divisors {
        assert_all_same("C37", vals.iter().map(|&v1| (v1, v2)));
    }
}

// ---------------------------------------------------------------------------
// C38 — raw ABI view: dirty upper 32 bits in the argument registers
// ---------------------------------------------------------------------------
#[test]
fn c38_raw_abi_dirty_high_bits() {
    let mut rng = Rng::new(0xC038);
    let dirt = [
        0x0000_0000_0000_0000u64,
        0xFFFF_FFFF_0000_0000,
        0x7FFF_FFFF_0000_0000,
        0xDEAD_BEEF_0000_0000,
        0x8000_0000_0000_0000,
    ];
    let vals = boundary_values();
    for &v1 in &vals {
        for &v2 in [0i32, 1, -1, 2, -2, 7, -7, I32_MIN, I32_MAX].iter() {
            for &d1 in &dirt {
                for &d2 in &dirt {
                    let a = ((v1 as u32) as u64 | d1) as i64;
                    let b = ((v2 as u32) as u64 | d2) as i64;
                    assert_same_raw("C38", a, b);
                }
            }
        }
    }
    for _ in 0..50_000 {
        let a = rng.next_u64() as i64;
        let b = rng.next_u64() as i64;
        assert_same_raw("C38", a, b);
        // the low halves must also agree with the plain c_int call
        assert_same("C38", a as i32, b as i32);
    }
}
