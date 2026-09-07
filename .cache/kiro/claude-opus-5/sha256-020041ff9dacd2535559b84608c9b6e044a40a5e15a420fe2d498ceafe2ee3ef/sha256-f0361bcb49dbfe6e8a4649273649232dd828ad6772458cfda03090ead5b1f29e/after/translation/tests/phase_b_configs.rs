//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `cdylib` via `libloading` and compares the destination buffers
//! bit-for-bit. All randomized rows use a fixed seed.

mod common;
use common::*;

/// Row 1 — achromatic, `s = +0.0`, randomized `h`/`v` normals.
#[test]
fn cfg_01_achromatic_pos_zero_s() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..20_000 {
        let src = [rng.range(-1e6, 1e6), 0.0, rng.range(-1e6, 1e6)];
        assert_same("cfg01", &src);
    }
}

/// Row 2 — achromatic, `s = -0.0` (IEEE `-0.0 == 0.0`, so the branch is taken).
#[test]
fn cfg_02_achromatic_neg_zero_s() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..20_000 {
        let src = [rng.range(-1e6, 1e6), -0.0, rng.range(-1e6, 1e6)];
        assert_same("cfg02", &src);
    }
    // Pin the sign-of-zero explicitly.
    assert_eq!((-0.0f32).to_bits(), 0x8000_0000);
    assert_same("cfg02", &[123.0, -0.0, -0.0]);
    assert_same("cfg02", &[123.0, -0.0, f32::NAN]);
}

/// Row 3 — achromatic with every special `v`.
#[test]
fn cfg_03_achromatic_special_v() {
    for &s in &[0.0f32, -0.0f32] {
        for &v in SPECIAL_V {
            for &h in ARM_HUES {
                assert_same("cfg03", &[h, s, v]);
            }
        }
    }
}

/// Row 4 — achromatic: `h` must be ignored entirely, including NaN/inf.
#[test]
fn cfg_04_achromatic_ignored_h() {
    let hs = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        0.0,
        -0.0,
        1e-45,
        -1e-45,
        1e30,
        -1e30,
    ];
    for &s in &[0.0f32, -0.0f32] {
        for &h in &hs {
            for &v in SPECIAL_V {
                assert_same("cfg04", &[h, s, v]);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..20_000 {
        let src = [rng.any_f32(), if rng.next_u32() & 1 == 0 { 0.0 } else { -0.0 }, rng.any_f32()];
        assert_same("cfg04", &src);
    }
}

/// Helper for rows 5–12: sweep a hue interval that maps to one `switch` arm.
fn sweep_sector(row: &str, seed: u64, lo: f32, hi: f32) {
    let mut rng = Rng::new(seed);
    for _ in 0..30_000 {
        // s strictly non-zero so the chromatic path is taken.
        let mut s = rng.range(f32::MIN_POSITIVE, 1.0);
        if s == 0.0 {
            s = 0.5;
        }
        let src = [rng.range(lo, hi), s, rng.range(0.0, 1.0)];
        assert_same(row, &src);
    }
    // Plus every special s/v combination at the interval midpoint and edges.
    let mid = lo + (hi - lo) * 0.5;
    for &h in &[lo, mid, hi] {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same(row, &[h, s, v]);
            }
        }
    }
}

/// Row 5 — `switch` arm `case 0`, hue in `[0,60)`.
#[test]
fn cfg_05_sector_0() {
    sweep_sector("cfg05", SEED ^ 5, 0.0, 60.0);
}

/// Row 6 — `case 1`, hue in `[60,120)`.
#[test]
fn cfg_06_sector_1() {
    sweep_sector("cfg06", SEED ^ 6, 60.0, 120.0);
}

/// Row 7 — `case 2`, hue in `[120,180)`.
#[test]
fn cfg_07_sector_2() {
    sweep_sector("cfg07", SEED ^ 7, 120.0, 180.0);
}

/// Row 8 — `case 3`, hue in `[180,240)`.
#[test]
fn cfg_08_sector_3() {
    sweep_sector("cfg08", SEED ^ 8, 180.0, 240.0);
}

/// Row 9 — `case 4`, hue in `[240,300)`.
#[test]
fn cfg_09_sector_4() {
    sweep_sector("cfg09", SEED ^ 9, 240.0, 300.0);
}

/// Row 10 — `default:` reached with `i == 5`, hue in `[300,360)`.
#[test]
fn cfg_10_default_i5() {
    sweep_sector("cfg10", SEED ^ 10, 300.0, 360.0);
}

/// Row 11 — `default:` reached with `i >= 6` (unwrapped hue above 360).
#[test]
fn cfg_11_default_i_ge_6() {
    sweep_sector("cfg11", SEED ^ 11, 360.0, 1e6);
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..20_000 {
        let src = [rng.range(360.0, 1e6), rng.range(0.001, 1.0), rng.range(0.0, 1.0)];
        assert_same("cfg11", &src);
    }
}

/// Row 12 — `default:` reached with `i < 0` (negative, unnormalised hue).
#[test]
fn cfg_12_default_negative_i() {
    sweep_sector("cfg12", SEED ^ 12, -1e6, -1e-30);
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..20_000 {
        let src = [rng.range(-1e6, 0.0), rng.range(0.001, 1.0), rng.range(0.0, 1.0)];
        assert_same("cfg12", &src);
    }
}

/// Row 13 — exact sector boundaries: `h` an exact multiple of 60 ⇒ `f == 0`.
#[test]
fn cfg_13_exact_boundaries() {
    let hues: Vec<f32> = (-8i32..=8).map(|k| k as f32 * 60.0).collect();
    for &h in &hues {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("cfg13", &[h, s, v]);
            }
        }
    }
    // Randomized s/v on the boundaries too.
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..20_000 {
        let h = hues[(rng.next_u32() as usize) % hues.len()];
        assert_same("cfg13", &[h, rng.range(0.001, 1.0), rng.range(0.0, 1.0)]);
    }
}

/// Row 14 — one ULP either side of every sector boundary.
#[test]
fn cfg_14_one_ulp_around_boundaries() {
    fn next_up(x: f32) -> f32 {
        if x == 0.0 {
            return f32::from_bits(1);
        }
        let b = x.to_bits();
        f32::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
    }
    fn next_down(x: f32) -> f32 {
        if x == 0.0 {
            return f32::from_bits(0x8000_0001);
        }
        let b = x.to_bits();
        f32::from_bits(if x > 0.0 { b - 1 } else { b + 1 })
    }

    let mut hues = Vec::new();
    for k in -8i32..=8 {
        let base = k as f32 * 60.0;
        hues.push(base);
        hues.push(next_up(base));
        hues.push(next_down(base));
        // Two ULPs out as well, to catch off-by-one in the floor/sector logic.
        hues.push(next_up(next_up(base)));
        hues.push(next_down(next_down(base)));
    }
    for &h in &hues {
        for &s in SPECIAL_S {
            for &v in &[0.0f32, -0.0, 0.5, 1.0, f32::MAX, f32::INFINITY, f32::NAN] {
                assert_same("cfg14", &[h, s, v]);
            }
        }
    }
}

/// Row 15 — `s == 1.0` exactly ⇒ `p = v * 0`.
#[test]
fn cfg_15_s_exactly_one() {
    let mut rng = Rng::new(SEED ^ 15);
    for &h in ARM_HUES {
        for &v in SPECIAL_V {
            assert_same("cfg15", &[h, 1.0, v]);
        }
    }
    for _ in 0..20_000 {
        assert_same("cfg15", &[rng.range(-720.0, 1080.0), 1.0, rng.range(-10.0, 10.0)]);
    }
}

/// Row 16 — subnormal / tiny `s` where `1 - s` rounds back to `1.0`.
#[test]
fn cfg_16_tiny_s() {
    let tiny = [
        f32::from_bits(1),
        f32::from_bits(2),
        1e-45f32,
        1e-40f32,
        f32::MIN_POSITIVE,
        f32::EPSILON,
        f32::EPSILON * 0.5,
        -f32::from_bits(1),
        -1e-40f32,
        -f32::EPSILON,
    ];
    for &s in &tiny {
        for &h in ARM_HUES {
            for &v in SPECIAL_V {
                assert_same("cfg16", &[h, s, v]);
            }
        }
    }
}

/// Row 17 — `v = ±0.0` with a non-zero `s` (sign-of-zero propagation).
#[test]
fn cfg_17_v_signed_zero() {
    let mut rng = Rng::new(SEED ^ 17);
    for &v in &[0.0f32, -0.0f32] {
        for &s in SPECIAL_S {
            for &h in ARM_HUES {
                assert_same("cfg17", &[h, s, v]);
            }
        }
        for _ in 0..10_000 {
            assert_same("cfg17", &[rng.range(-720.0, 1080.0), rng.range(-2.0, 3.0), v]);
        }
    }
}

/// Row 18 — subnormal / near-underflow `v`.
#[test]
fn cfg_18_v_subnormal() {
    let vs = [
        f32::from_bits(1),
        f32::from_bits(0x0000_00FF),
        1e-45f32,
        1e-38f32,
        f32::MIN_POSITIVE,
        -f32::from_bits(1),
        -1e-38f32,
    ];
    for &v in &vs {
        for &s in SPECIAL_S {
            for &h in ARM_HUES {
                assert_same("cfg18", &[h, s, v]);
            }
        }
    }
}

/// Row 19 — huge `v` combined with `s > 1`, forcing overflow of `p`/`q`/`t`.
#[test]
fn cfg_19_v_huge_overflow() {
    let vs = [1e38f32, f32::MAX, -1e38f32, f32::MIN, 3.0e38];
    let ss = [1.5f32, 2.0, 10.0, 1e30, f32::MAX, -1e30, -2.0];
    for &v in &vs {
        for &s in &ss {
            for &h in ARM_HUES {
                assert_same("cfg19", &[h, s, v]);
            }
        }
    }
}

/// Row 20 — `s > 1` (unclamped) across all six arms.
#[test]
fn cfg_20_s_above_one() {
    let mut rng = Rng::new(SEED ^ 20);
    for &s in &[1.0000001f32, 1.5, 2.0, 10.0, 1e30, f32::MAX, f32::INFINITY] {
        for &h in ARM_HUES {
            for &v in SPECIAL_V {
                assert_same("cfg20", &[h, s, v]);
            }
        }
    }
    for _ in 0..30_000 {
        assert_same("cfg20", &[rng.range(-720.0, 1080.0), rng.range(1.0, 100.0), rng.range(-5.0, 5.0)]);
    }
}

/// Row 21 — `s < 0` (unclamped) across all six arms.
#[test]
fn cfg_21_s_below_zero() {
    let mut rng = Rng::new(SEED ^ 21);
    for &s in &[-f32::from_bits(1), -0.5f32, -1.0, -10.0, -1e30, f32::MIN, f32::NEG_INFINITY] {
        for &h in ARM_HUES {
            for &v in SPECIAL_V {
                assert_same("cfg21", &[h, s, v]);
            }
        }
    }
    for _ in 0..30_000 {
        assert_same("cfg21", &[rng.range(-720.0, 1080.0), rng.range(-100.0, -1e-6), rng.range(-5.0, 5.0)]);
    }
}

/// Row 22 — hue driving `(int)floorf(h/60)` out of `int` range (the UB cast).
#[test]
fn cfg_22_hue_out_of_int_range() {
    const TWO31: f32 = 2147483648.0;
    fn bump(x: f32, steps: i32) -> f32 {
        let mut b = x.to_bits() as i64;
        b += steps as i64;
        f32::from_bits(b as u32)
    }
    let mut hues = vec![
        1e30f32,
        -1e30f32,
        f32::MAX,
        f32::MIN,
        3.0e38,
        -3.0e38,
        TWO31 * 60.0,
        -TWO31 * 60.0,
        (TWO31 - 128.0) * 60.0,
        -(TWO31 - 128.0) * 60.0,
        2147483520.0 * 60.0,
        1e10,
        -1e10,
        1e20,
        -1e20,
    ];
    for steps in [-2i32, -1, 0, 1, 2] {
        hues.push(bump(TWO31 * 60.0, steps));
        hues.push(bump(-TWO31 * 60.0, steps));
        hues.push(bump(TWO31, steps) * 60.0);
    }
    for &h in &hues {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("cfg22", &[h, s, v]);
            }
        }
    }
}

/// Row 23 — NaN in every position and combination, including non-default
/// payloads and a signalling NaN.
#[test]
fn cfg_23_nan_matrix() {
    let nans = [
        f32::NAN,
        f32::from_bits(0x7FC0_0000), // canonical quiet NaN
        f32::from_bits(0xFFC0_0000), // negative quiet NaN
        f32::from_bits(0x7FC0_1234), // quiet NaN, non-default payload
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0xFF80_4321), // negative signalling NaN
    ];
    let normals = [0.0f32, -0.0, 0.5, 1.0, 30.0, 210.0, -30.0, 1e30];
    for &n in &nans {
        // NaN in one position at a time.
        for &a in &normals {
            for &b in &normals {
                assert_same("cfg23", &[n, a, b]);
                assert_same("cfg23", &[a, n, b]);
                assert_same("cfg23", &[a, b, n]);
            }
        }
        // NaN in two and three positions.
        for &m in &nans {
            for &a in &normals {
                assert_same("cfg23", &[n, m, a]);
                assert_same("cfg23", &[n, a, m]);
                assert_same("cfg23", &[a, n, m]);
            }
            assert_same("cfg23", &[n, m, n]);
            assert_same("cfg23", &[n, n, m]);
        }
    }
}

/// Row 24 — `±inf` in every position and combination (`inf*0` and `inf-inf`).
#[test]
fn cfg_24_infinity_matrix() {
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    let normals = [0.0f32, -0.0, 0.5, 1.0, 30.0, 90.0, 150.0, 210.0, 270.0, 330.0, -30.0, 1e30, -1e30];
    for &i0 in &infs {
        for &a in &normals {
            for &b in &normals {
                assert_same("cfg24", &[i0, a, b]);
                assert_same("cfg24", &[a, i0, b]);
                assert_same("cfg24", &[a, b, i0]);
            }
        }
        for &i1 in &infs {
            for &a in &normals {
                assert_same("cfg24", &[i0, i1, a]);
                assert_same("cfg24", &[i0, a, i1]);
                assert_same("cfg24", &[a, i0, i1]);
            }
            for &i2 in &infs {
                assert_same("cfg24", &[i0, i1, i2]);
            }
        }
    }
}

/// Row 25 — `dest == src` (full aliasing).
#[test]
fn cfg_25_full_aliasing() {
    let mut rng = Rng::new(SEED ^ 25);
    for &h in ARM_HUES {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same_aliased("cfg25", &[h, s, v, 7.0, 8.0, 9.0], 0, 0);
            }
        }
    }
    for _ in 0..20_000 {
        let buf = [
            rng.range(-720.0, 1080.0),
            rng.range(-2.0, 3.0),
            rng.range(-5.0, 5.0),
            1.5,
            2.5,
            3.5,
        ];
        assert_same_aliased("cfg25", &buf, 0, 0);
    }
}

/// Row 26 — partial overlap in both directions.
#[test]
fn cfg_26_partial_overlap() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..20_000 {
        let buf: Vec<f32> = (0..6)
            .map(|i| match i {
                0 | 3 => rng.range(-720.0, 1080.0),
                1 | 4 => rng.range(-2.0, 3.0),
                _ => rng.range(-5.0, 5.0),
            })
            .collect();
        // dest = src + 1
        assert_same_aliased("cfg26", &buf, 1, 0);
        // dest = src - 1
        assert_same_aliased("cfg26", &buf, 0, 1);
        // two-element overlap
        assert_same_aliased("cfg26", &buf, 2, 0);
        assert_same_aliased("cfg26", &buf, 0, 2);
    }
    for &h in ARM_HUES {
        for &s in SPECIAL_S {
            let buf = [h, s, 0.75, h, s, 0.75, 1.0, 2.0];
            assert_same_aliased("cfg26", &buf, 1, 0);
            assert_same_aliased("cfg26", &buf, 0, 1);
            assert_same_aliased("cfg26", &buf, 2, 0);
            assert_same_aliased("cfg26", &buf, 0, 2);
        }
    }
}

/// Row 27 — disjoint but adjacent buffers (the non-aliasing baseline).
#[test]
fn cfg_27_disjoint_adjacent() {
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..20_000 {
        let buf = [
            rng.range(-720.0, 1080.0),
            rng.range(-2.0, 3.0),
            rng.range(-5.0, 5.0),
            f32::from_bits(0x7F80_1D0D),
            f32::from_bits(0x7F80_1D0D),
            f32::from_bits(0x7F80_1D0D),
        ];
        // src = [0..3), dest = [3..6): adjacent, non-overlapping.
        assert_same_aliased("cfg27", &buf, 3, 0);
    }
    // And the ordinary separate-allocation path.
    for &h in ARM_HUES {
        for &s in SPECIAL_S {
            for &v in SPECIAL_V {
                assert_same("cfg27", &[h, s, v]);
            }
        }
    }
}

/// Row 28 — unconstrained property sweep: fully random 32-bit patterns.
#[test]
fn cfg_28_property_random_bits() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..ITERS {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_same("cfg28", &src);
    }
}

/// Row 29 — nominal-consumer property sweep (`h∈[0,360)`, `s,v∈[0,1]`).
#[test]
fn cfg_29_property_nominal() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..ITERS {
        let src = [rng.range(0.0, 360.0), rng.unit(), rng.unit()];
        assert_same("cfg29", &src);
    }
}

/// Row 30 — wide-hue property sweep across sign combinations.
#[test]
fn cfg_30_property_wide() {
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..ITERS {
        let src = [
            rng.range(-1e7, 1e7),
            rng.range(-2.0, 3.0),
            rng.range(-1e5, 1e5),
        ];
        assert_same("cfg30", &src);
    }
}

/// Row 31 — `floorf` implementation parity.
///
/// The C `.so` imports `floorf@GLIBC_2.2.5`; the Rust `.so` resolves its
/// `floorf` to a *local* copy supplied by `compiler_builtins`' libm (`nm` shows
/// `t floorf`, a local text symbol). Those are two different implementations of
/// the same exactly-specified operation, so this row hammers the hue axis
/// specifically — a strided sweep over the entire `f32` bit space plus a dense
/// pseudo-random sweep — to confirm the sector index they produce is identical
/// for every hue class.
#[test]
fn cfg_31_floorf_implementation_parity() {
    // Strided sweep over all 2^32 bit patterns (prime stride => hits every
    // exponent and a well-spread set of mantissas).
    const STRIDE: u32 = 4_093;
    let mut bits: u32 = 0;
    let mut n: u64 = 0;
    loop {
        let h = f32::from_bits(bits);
        // s = 1 keeps p exactly 0 and makes q/t depend directly on f, so a
        // one-ULP error in the floor result is visible in the output.
        assert_same("cfg31", &[h, 1.0, 1.0]);
        assert_same("cfg31", &[h, 0.375, 0.625]);
        n += 1;
        let (next, ovf) = bits.overflowing_add(STRIDE);
        if ovf {
            break;
        }
        bits = next;
    }
    assert!(n > 1_000_000, "expected >1e6 strided hue samples, got {n}");

    // Dense pseudo-random hue bit patterns with randomized s/v.
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..300_000 {
        let h = f32::from_bits(rng.next_u32());
        assert_same("cfg31", &[h, rng.range(-2.0, 3.0), rng.range(-100.0, 100.0)]);
    }

    // Every hue that is an exact integer number of degrees over a wide span,
    // where the sector boundary lands exactly on a floor step.
    for d in -5000i32..=5000 {
        assert_same("cfg31", &[d as f32, 1.0, 1.0]);
        assert_same("cfg31", &[d as f32 + 0.5, 0.75, 0.25]);
    }
}
