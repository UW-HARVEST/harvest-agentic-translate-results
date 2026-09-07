//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` `memchra2` export and
//! asserts byte-identical `int` results, over many randomized inputs with a
//! fixed seed.

mod common;
use common::{check, Rng, EXTREMES};

/// Row 1 — all four args small non-negative (1 digit), exhaustive 10^4.
#[test]
fn row01_small_non_negative_exhaustive() {
    for a in 0..10 {
        for b in 0..10 {
            for c in 0..10 {
                for d in 0..10 {
                    check(a, b, c, d);
                }
            }
        }
    }
}

/// Row 2 — all four args small negative, exhaustive 9^4.
#[test]
fn row02_small_negative_exhaustive() {
    for a in -9..=-1 {
        for b in -9..=-1 {
            for c in -9..=-1 {
                for d in -9..=-1 {
                    check(a, b, c, d);
                }
            }
        }
    }
}

/// Row 3 — mixed signs, exhaustive over {-2..2}^4; hits 0..4 negatives.
#[test]
fn row03_mixed_signs_exhaustive() {
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                for d in -2..=2 {
                    check(a, b, c, d);
                }
            }
        }
    }
}

/// Row 4 — `a` puns to a float in [1.0, 1000.0): the only branch that adds (int)f.
#[test]
fn row04_float_pun_in_range() {
    let mut r = Rng::new(0x0401);
    let lo = 1.0f32.to_bits(); // 0x3F800000
    let hi = 1000.0f32.to_bits() - 1; // just below 1000.0
    for _ in 0..20_000 {
        let a = r.u32_in(lo, hi) as i32;
        check(a, r.next_i32(), r.next_i32(), r.next_i32());
    }
    // Named boundaries of the range.
    for f in [
        1.0f32, 1.5, 2.0, 2.999_999_8, 3.0, 9.999_999, 10.0, 99.999_99, 100.0, 500.5, 999.0,
        999.999_94, 999.999_99,
    ] {
        let a = f.to_bits() as i32;
        check(a, 1, 2, 3);
        check(a, 0, 0, 0);
        check(a, -1, -2, -3);
    }
    // Exactly 1000.0 must be REJECTED by `f < 1000.0f`.
    check(1000.0f32.to_bits() as i32, 1, 2, 3);
    // Exactly 1.0 accepted, just below 1.0 truncates to 0.
    check((1.0f32.to_bits() - 1) as i32, 1, 2, 3);
}

/// Row 5 — `a` a subnormal float pattern: f > 0 true but (int)f == 0.
#[test]
fn row05_float_pun_subnormal() {
    let mut r = Rng::new(0x0501);
    for _ in 0..10_000 {
        let a = r.u32_in(1, 0x007F_FFFF) as i32;
        check(a, r.next_i32(), r.next_i32(), r.next_i32());
    }
    for a in [1i32, 2, 3, 0x0000_FFFF, 0x0040_0000, 0x007F_FFFF] {
        check(a, 0, 0, 0);
        check(a, i32::MIN, i32::MAX, -1);
    }
    // Negative subnormals too.
    for a in [1i32, 0x0040_0000, 0x007F_FFFF] {
        check(a | i32::MIN, 7, 8, 9);
    }
}

/// Row 6 — `a` a normal float in (0.0, 1.0): f > 0 true, (int)f == 0.
#[test]
fn row06_float_pun_below_one() {
    let mut r = Rng::new(0x0601);
    for _ in 0..20_000 {
        let a = r.u32_in(0x0080_0000, 0x3F7F_FFFF) as i32;
        check(a, r.next_i32(), r.next_i32(), r.next_i32());
    }
    for f in [f32::MIN_POSITIVE, 1e-30, 1e-10, 0.001, 0.5, 0.999_999_94] {
        check(f.to_bits() as i32, 1, 2, 3);
    }
}

/// Row 7 — `a` puns to a finite float >= 1000.0: rejected by `f < 1000.0f`.
#[test]
fn row07_float_pun_at_least_1000() {
    let mut r = Rng::new(0x0701);
    let lo = 1000.0f32.to_bits();
    let hi = f32::MAX.to_bits();
    for _ in 0..20_000 {
        let a = r.u32_in(lo, hi) as i32;
        check(a, r.next_i32(), r.next_i32(), r.next_i32());
    }
    for f in [1000.0f32, 1000.000_1, 1001.0, 1e6, 1e30, f32::MAX] {
        check(f.to_bits() as i32, 1, 2, 3);
        check((f.to_bits() as i32) | i32::MIN, 1, 2, 3); // negative counterpart
    }
}

/// Row 8 — `a` puns to +/- infinity.
#[test]
fn row08_float_pun_infinities() {
    let pos = f32::INFINITY.to_bits() as i32; // 0x7F800000
    let neg = f32::NEG_INFINITY.to_bits() as i32; // 0xFF800000
    let mut r = Rng::new(0x0801);
    for a in [pos, neg] {
        check(a, 0, 0, 0);
        check(a, 1, 2, 3);
        check(a, i32::MIN, i32::MAX, -1);
        for _ in 0..2_000 {
            check(a, r.next_i32(), r.next_i32(), r.next_i32());
        }
    }
}

/// Row 9 — `a` a NaN pattern: both `f > 0` and `f < 1000` are false.
#[test]
fn row09_float_pun_nans() {
    let mut r = Rng::new(0x0901);
    let named: [i32; 8] = [
        0x7F80_0001u32 as i32, // smallest signalling NaN
        0x7FBF_FFFFu32 as i32,
        0x7FC0_0000u32 as i32, // canonical quiet NaN
        0x7FFF_FFFFu32 as i32,
        0xFF80_0001u32 as i32, // negative NaNs
        0xFFBF_FFFFu32 as i32,
        0xFFC0_0000u32 as i32,
        0xFFFF_FFFFu32 as i32, // == -1
    ];
    for a in named {
        assert!(f32::from_bits(a as u32).is_nan(), "0x{a:08x} should be NaN");
        check(a, 0, 0, 0);
        check(a, 1, 2, 3);
        check(a, -1, -1, -1);
        for _ in 0..1_000 {
            check(a, r.next_i32(), r.next_i32(), r.next_i32());
        }
    }
    // Random NaN payloads.
    for _ in 0..5_000 {
        let payload = r.u32_in(1, 0x007F_FFFF);
        let a = (0x7F80_0000u32 | payload) as i32;
        check(a, r.next_i32(), r.next_i32(), r.next_i32());
        let a = (0xFF80_0000u32 | payload) as i32;
        check(a, r.next_i32(), r.next_i32(), r.next_i32());
    }
}

/// Row 10 — `a == 0` (+0.0) and `a == INT_MIN` (-0.0).
#[test]
fn row10_float_pun_zeros() {
    let mut r = Rng::new(0x0A01);
    for a in [0i32, i32::MIN] {
        assert_eq!(f32::from_bits(a as u32), 0.0);
        for &b in EXTREMES {
            check(a, b, 0, 0);
            check(a, 0, b, 0);
            check(a, 0, 0, b);
        }
        for _ in 0..5_000 {
            check(a, r.next_i32(), r.next_i32(), r.next_i32());
        }
    }
}

/// Row 11 — decimal-width boundaries swept one position at a time.
#[test]
fn row11_decimal_width_boundaries() {
    let mut widths: Vec<i32> = Vec::new();
    let mut p: i64 = 1;
    for _ in 0..10 {
        for v in [p - 1, p, p + 1] {
            if v <= i32::MAX as i64 {
                widths.push(v as i32);
                widths.push(-(v as i32));
            }
        }
        p *= 10;
    }
    widths.extend_from_slice(&[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 0]);

    let mut r = Rng::new(0x0B01);
    for &w in &widths {
        // sweep each position with the others fixed, then randomized
        check(w, 1, 2, 3);
        check(1, w, 2, 3);
        check(1, 2, w, 3);
        check(1, 2, 3, w);
        for _ in 0..40 {
            let (x, y, z) = (r.next_i32(), r.next_i32(), r.next_i32());
            check(w, x, y, z);
            check(x, w, y, z);
            check(x, y, w, z);
            check(x, y, z, w);
        }
    }
    // Full cross-product of the width set in all 4 slots would be huge; sample it.
    for _ in 0..30_000 {
        let pick = |r: &mut Rng| widths[(r.next_u64() % widths.len() as u64) as usize];
        let (a, b, c, d) = (pick(&mut r), pick(&mut r), pick(&mut r), pick(&mut r));
        check(a, b, c, d);
    }
}

/// Row 12 — longest formatted buffer; probes the snprintf size-64 boundary.
#[test]
fn row12_longest_buffer() {
    check(i32::MIN, i32::MIN, i32::MIN, i32::MIN); // "test-2147483648-..." = 51 bytes
    check(i32::MAX, i32::MAX, i32::MAX, i32::MAX); // 4 + 4*10 + 3 = 47 bytes
    check(i32::MIN, i32::MAX, i32::MIN, i32::MAX);
    check(i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    check(i32::MIN + 1, i32::MIN + 1, i32::MIN + 1, i32::MIN + 1);
    for &e in &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1_000_000_000, 1_000_000_000] {
        for &f in &[i32::MIN, i32::MAX, -999_999_999, 999_999_999] {
            check(e, f, e, f);
            check(f, e, f, e);
            check(e, e, f, f);
        }
    }
}

/// Row 13 — `b, c, d` low bytes all zero: `interpret_as_int` yields 0.
#[test]
fn row13_low_bytes_zero() {
    let mut r = Rng::new(0x0D01);
    for _ in 0..20_000 {
        let a = r.next_i32();
        let b = r.next_i32() & !0xFF;
        let c = r.next_i32() & !0xFF;
        let d = r.next_i32() & !0xFF;
        check(a, b, c, d);
    }
    check(0, 0, 0, 0);
    check(0, 256, 512, -256);
    check(1, -256, -512, 65_536);
    check(i32::MIN, i32::MIN, 0x7FFF_FF00, -0x100);
}

/// Row 14 — `b, c, d` low bytes all 0xFF.
#[test]
fn row14_low_bytes_ff() {
    let mut r = Rng::new(0x0E01);
    for _ in 0..20_000 {
        let a = r.next_i32();
        let b = r.next_i32() | 0xFF;
        let c = r.next_i32() | 0xFF;
        let d = r.next_i32() | 0xFF;
        check(a, b, c, d);
    }
    check(0, 255, 255, 255);
    check(-1, -1, -1, -1);
    check(1, 0xFF, -1, 0x7FFF_FFFF);
}

/// Row 15 — low bytes mixed while high bytes vary independently.
#[test]
fn row15_low_bytes_mixed_high_bytes_independent() {
    let mut r = Rng::new(0x0F01);
    for _ in 0..30_000 {
        let a = r.next_i32();
        let lo_b = r.u32_in(0, 255) as i32;
        let lo_c = r.u32_in(0, 255) as i32;
        let lo_d = r.u32_in(0, 255) as i32;
        let hi = |r: &mut Rng| (r.next_i32() & !0xFF);
        let (hb, hc, hd) = (hi(&mut r), hi(&mut r), hi(&mut r));
        check(a, hb | lo_b, hc | lo_c, hd | lo_d);
    }
    // Same low bytes, deliberately different high bytes.
    for lo in [0u32, 1, 0x7F, 0x80, 0xFE, 0xFF] {
        for &hi in &[0i32, 0x0100, 0x7FFF_FF00u32 as i32, i32::MIN, -0x100] {
            let v = hi | lo as i32;
            check(v, v, v, v);
            check(0, v, v, v);
        }
    }
}

/// Row 16 — a == b == c == d, so `complex_iteration`'s XOR fold cancels to 0.
#[test]
fn row16_all_equal() {
    let mut r = Rng::new(0x1001);
    for _ in 0..20_000 {
        let v = r.next_i32();
        check(v, v, v, v);
    }
    for &v in EXTREMES {
        check(v, v, v, v);
    }
}

/// Row 17 — pairwise-equal low bytes, differing high bytes.
#[test]
fn row17_pairwise_equal_low_bytes() {
    let mut r = Rng::new(0x1101);
    for _ in 0..20_000 {
        let lo1 = r.u32_in(0, 255) as i32;
        let lo2 = r.u32_in(0, 255) as i32;
        let hi = |r: &mut Rng| (r.next_i32() & !0xFF);
        let a = hi(&mut r) | lo1;
        let b = hi(&mut r) | lo1;
        let c = hi(&mut r) | lo2;
        let d = hi(&mut r) | lo2;
        check(a, b, c, d);
        check(a, c, b, d);
        check(a, b, c, c);
    }
}

/// Row 18 — arguments whose sum overflows signed positive.
#[test]
fn row18_overflow_positive() {
    let mut r = Rng::new(0x1201);
    check(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    check(i32::MAX, 1, 0, 0);
    check(2_000_000_000, 2_000_000_000, 1, 1);
    check(1_073_741_824, 1_073_741_824, 1_073_741_824, 1_073_741_824);
    for _ in 0..20_000 {
        let near = |r: &mut Rng| i32::MAX - r.i32_in(0, 1_000_000);
        check(near(&mut r), near(&mut r), near(&mut r), near(&mut r));
    }
}

/// Row 19 — arguments whose sum overflows signed negative.
#[test]
fn row19_overflow_negative() {
    let mut r = Rng::new(0x1301);
    check(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    check(i32::MIN, -1, 0, 0);
    check(-2_000_000_000, -2_000_000_000, -1, -1);
    for _ in 0..20_000 {
        let near = |r: &mut Rng| i32::MIN + r.i32_in(0, 1_000_000);
        check(near(&mut r), near(&mut r), near(&mut r), near(&mut r));
    }
    // Mixed overflow directions.
    for _ in 0..10_000 {
        check(i32::MIN, i32::MAX, r.next_i32(), r.next_i32());
        check(i32::MAX, i32::MIN, r.next_i32(), r.next_i32());
    }
}

/// Row 20 — 200 000 fully random tuples over the whole int32 range.
#[test]
fn row20_uniform_random_full_range() {
    let mut r = Rng::new(0xDEAD_BEEF_CAFE_1234);
    for _ in 0..200_000 {
        check(r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

/// Row 21 — dense coverage over int8-magnitude values (A1 x A5 x A6 together).
#[test]
fn row21_int8_magnitude_dense() {
    let mut r = Rng::new(0x1501);
    for _ in 0..60_000 {
        let v = |r: &mut Rng| r.i32_in(-128, 127);
        check(v(&mut r), v(&mut r), v(&mut r), v(&mut r));
    }
    // Exhaustive over a coarse lattice of the int8 range in all four slots.
    let lattice: Vec<i32> = (-128..=127).step_by(43).collect();
    for &a in &lattice {
        for &b in &lattice {
            for &c in &lattice {
                for &d in &lattice {
                    check(a, b, c, d);
                }
            }
        }
    }
}

/// Row 22 — one argument extreme, the other three random; all 4 positions.
#[test]
fn row22_one_extreme_three_random() {
    let mut r = Rng::new(0x1601);
    for &e in &[i32::MIN, 0, i32::MAX] {
        for _ in 0..8_000 {
            let (x, y, z) = (r.next_i32(), r.next_i32(), r.next_i32());
            check(e, x, y, z);
            check(x, e, y, z);
            check(x, y, e, z);
            check(x, y, z, e);
        }
    }
}

/// Row 23 — A2 x A5 interaction: `a` extreme x b/c/d low-byte extremes.
#[test]
fn row23_float_pun_x_low_byte_grid() {
    let mut r = Rng::new(0x1701);
    let a_vals: [i32; 9] = [
        0,
        i32::MIN,
        1,                                     // subnormal
        0.5f32.to_bits() as i32,               // in (0,1)
        1.0f32.to_bits() as i32,               // == 1.0
        123.456f32.to_bits() as i32,           // in [1,1000)
        1000.0f32.to_bits() as i32,            // == 1000.0, rejected
        f32::INFINITY.to_bits() as i32,        // +inf
        0x7FC0_0000u32 as i32,                 // NaN
    ];
    // low-byte "modes": all zero, all 0xFF, all 0x80 (sign bit of the byte)
    let modes: [i32; 3] = [0x00, 0xFF, 0x80];
    for &a in &a_vals {
        for &m in &modes {
            for _ in 0..800 {
                let hi = |r: &mut Rng| (r.next_i32() & !0xFF);
                check(a, hi(&mut r) | m, hi(&mut r) | m, hi(&mut r) | m);
            }
            check(a, m, m, m);
            check(a, m, 0, 0);
            check(a, 0, m, 0);
            check(a, 0, 0, m);
        }
    }
}

/// Row 24 — powers of two and 2^k - 1 (and negatives) in all four positions.
#[test]
fn row24_powers_of_two() {
    let mut pow: Vec<i32> = Vec::new();
    for k in 0..31u32 {
        let v = 1i32 << k;
        pow.push(v);
        pow.push(-v);
        pow.push(v - 1);
        pow.push(-(v - 1));
    }
    pow.push(i32::MIN);
    pow.push(i32::MAX);

    let mut r = Rng::new(0x1801);
    for &v in &pow {
        check(v, 1, 2, 3);
        check(1, v, 2, 3);
        check(1, 2, v, 3);
        check(1, 2, 3, v);
        check(v, v, v, v);
        for _ in 0..30 {
            let (x, y, z) = (r.next_i32(), r.next_i32(), r.next_i32());
            check(v, x, y, z);
            check(x, v, y, z);
            check(x, y, v, z);
            check(x, y, z, v);
        }
    }
    // Sampled cross-product of the power set.
    for _ in 0..40_000 {
        let pick = |r: &mut Rng| pow[(r.next_u64() % pow.len() as u64) as usize];
        let (a, b, c, d) = (pick(&mut r), pick(&mut r), pick(&mut r), pick(&mut r));
        check(a, b, c, d);
    }
}
