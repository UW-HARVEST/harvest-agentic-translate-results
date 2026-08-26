//! Phase B rows C1..C18: the three plain noise entry points.
//!
//! Both implementations are loaded as shared objects and only called through
//! their exported C symbols.

mod common;

use common::{Diff, Rng, SPECIAL_F32, SPECIAL_WRAPS};

/// C1: wraps (0,0,0), seed 0, random fractional coordinates.
#[test]
fn c1_internal_no_wrap_seed0() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C1 noise3_internal wraps=(0,0,0) seed=0");
    let mut rng = Rng::new(0xC1);
    for _ in 0..4000 {
        let (x, y, z) = (rng.coord(4), rng.coord(4), rng.coord(4));
        d.check(
            format_args!("x={x:e} y={y:e} z={z:e}"),
            unsafe { (c.noise3_internal)(x, y, z, 0, 0, 0, 0) },
            unsafe { (r.noise3_internal)(x, y, z, 0, 0, 0, 0) },
        );
    }
    d.finish();
}

/// C2: wraps (0,0,0), random seed in 0..=255.
#[test]
fn c2_internal_no_wrap_random_seed() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C2 noise3_internal wraps=(0,0,0) random seed");
    let mut rng = Rng::new(0xC2);
    for _ in 0..4000 {
        let (x, y, z) = (rng.coord(8), rng.coord(8), rng.coord(8));
        let s = rng.seed_u8();
        d.check(
            format_args!("x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, 0, 0, 0, s) },
            unsafe { (r.noise3_internal)(x, y, z, 0, 0, 0, s) },
        );
    }
    d.finish();
}

/// C3: exactly integral coordinates (so `x -= px` is exactly 0) and +-0.
#[test]
fn c3_internal_integral_coords() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C3 noise3_internal integral coords");
    let mut rng = Rng::new(0xC3);
    let zeros = [0.0f32, -0.0f32];
    for _ in 0..2000 {
        let coord = |rng: &mut Rng| -> f32 {
            match rng.below(3) {
                0 => *rng.pick(&zeros),
                1 => rng.range(-300, 300) as f32,
                _ => rng.range(-3, 3) as f32,
            }
        };
        let (x, y, z) = (coord(&mut rng), coord(&mut rng), coord(&mut rng));
        for s in [0u8, 255u8] {
            d.check(
                format_args!("x={x:e} y={y:e} z={z:e} seed={s}"),
                unsafe { (c.noise3_internal)(x, y, z, 0, 0, 0, s) },
                unsafe { (r.noise3_internal)(x, y, z, 0, 0, 0, s) },
            );
        }
    }
    d.finish();
}

/// C4: all three wraps the same power of two 1..256.
#[test]
fn c4_internal_pow2_wraps() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C4 noise3_internal uniform power-of-two wraps");
    let mut rng = Rng::new(0xC4);
    for w in [1, 2, 4, 8, 16, 32, 64, 128, 256] {
        for _ in 0..400 {
            let (x, y, z) = (rng.coord(600), rng.coord(600), rng.coord(600));
            let s = rng.seed_u8();
            d.check(
                format_args!("w={w} x={x:e} y={y:e} z={z:e} seed={s}"),
                unsafe { (c.noise3_internal)(x, y, z, w, w, w, s) },
                unsafe { (r.noise3_internal)(x, y, z, w, w, w, s) },
            );
        }
    }
    d.finish();
}

/// C5: per-axis different powers of two.
#[test]
fn c5_internal_mixed_pow2_wraps() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C5 noise3_internal mixed power-of-two wraps");
    let pow2 = [1, 2, 4, 8, 16, 32, 64, 128, 256];
    let mut rng = Rng::new(0xC5);
    for _ in 0..4000 {
        let (xw, yw, zw) = (*rng.pick(&pow2), *rng.pick(&pow2), *rng.pick(&pow2));
        let (x, y, z) = (rng.coord(400), rng.coord(400), rng.coord(400));
        let s = rng.seed_u8();
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// C6: wrap == 1 (mask 0) on one, two or three axes.
#[test]
fn c6_internal_wrap_one() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C6 noise3_internal wrap==1");
    let others = [0, 2, 8, 256, 3, -4];
    let mut rng = Rng::new(0xC6);
    for mask in 1..8u32 {
        for _ in 0..300 {
            let pick = |rng: &mut Rng, on: bool| if on { 1 } else { *rng.pick(&others) };
            let xw = pick(&mut rng, mask & 1 != 0);
            let yw = pick(&mut rng, mask & 2 != 0);
            let zw = pick(&mut rng, mask & 4 != 0);
            let (x, y, z) = (rng.coord(40), rng.coord(40), rng.coord(40));
            let s = rng.seed_u8();
            d.check(
                format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
                unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
                unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
            );
        }
    }
    d.finish();
}

/// C7: wrap == 256 (identical mask to wrap == 0).
#[test]
fn c7_internal_wrap_256() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C7 noise3_internal wrap==256");
    let mut rng = Rng::new(0xC7);
    for _ in 0..2000 {
        let (x, y, z) = (rng.coord(1000), rng.coord(1000), rng.coord(1000));
        let s = rng.seed_u8();
        d.check(
            format_args!("x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, 256, 256, 256, s) },
            unsafe { (r.noise3_internal)(x, y, z, 256, 256, 256, s) },
        );
    }
    d.finish();
}

/// C8: powers of two above 256 (mask stays 255).
#[test]
fn c8_internal_wrap_pow2_above_256() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C8 noise3_internal power-of-two wraps > 256");
    let big = [512, 1024, 4096, 1 << 20, 1 << 30];
    let mut rng = Rng::new(0xC8);
    for _ in 0..3000 {
        let (xw, yw, zw) = (*rng.pick(&big), *rng.pick(&big), *rng.pick(&big));
        let (x, y, z) = (rng.coord(2000), rng.coord(2000), rng.coord(2000));
        let s = rng.seed_u8();
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// C9: non-power-of-two wraps.
#[test]
fn c9_internal_non_pow2_wraps() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C9 noise3_internal non-power-of-two wraps");
    let mut rng = Rng::new(0xC9);
    let fixed = [3, 5, 7, 100, 255, 257, 1000, 12345];
    for _ in 0..4000 {
        let (xw, yw, zw) = if rng.boolean() {
            (*rng.pick(&fixed), *rng.pick(&fixed), *rng.pick(&fixed))
        } else {
            (rng.range(1, 100000), rng.range(1, 100000), rng.range(1, 100000))
        };
        let (x, y, z) = (rng.coord(400), rng.coord(400), rng.coord(400));
        let s = rng.seed_u8();
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// C10: negative wraps.
#[test]
fn c10_internal_negative_wraps() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C10 noise3_internal negative wraps");
    let mut rng = Rng::new(0xCA);
    let fixed = [-1, -2, -5, -256, -257, -100000];
    for _ in 0..4000 {
        let (xw, yw, zw) = if rng.boolean() {
            (*rng.pick(&fixed), *rng.pick(&fixed), *rng.pick(&fixed))
        } else {
            (
                rng.range(i32::MIN + 1, -1),
                rng.range(i32::MIN + 1, -1),
                rng.range(i32::MIN + 1, -1),
            )
        };
        let (x, y, z) = (rng.coord(400), rng.coord(400), rng.coord(400));
        let s = rng.seed_u8();
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// C11: wraps at INT_MIN / INT_MAX (`x_wrap-1` overflows).
#[test]
fn c11_internal_wrap_int_extremes() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C11 noise3_internal wraps at int extremes");
    let extremes = [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, 1, -1];
    let mut rng = Rng::new(0xCB);
    for &xw in &extremes {
        for &yw in &extremes {
            for &zw in &extremes {
                for _ in 0..8 {
                    let (x, y, z) = (rng.coord(50), rng.coord(50), rng.coord(50));
                    let s = rng.seed_u8();
                    d.check(
                        format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
                        unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
                        unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
                    );
                }
            }
        }
    }
    d.finish();
}

/// C12: coordinates one ULP away from integers, +-0, subnormals.
#[test]
fn c12_internal_near_integer_coords() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C12 noise3_internal near-integer coords");
    let mut rng = Rng::new(0xCC);
    let nudge = |rng: &mut Rng| -> f32 {
        let base = rng.range(-64, 64) as f32;
        match rng.below(6) {
            0 => base,
            1 => next_after(base, f32::INFINITY),
            2 => next_after(base, f32::NEG_INFINITY),
            3 => base + 0.5,
            4 => f32::from_bits(rng.next_u32() & 0x007f_ffff), // subnormal
            _ => -f32::from_bits(rng.next_u32() & 0x007f_ffff),
        }
    };
    for _ in 0..4000 {
        let (x, y, z) = (nudge(&mut rng), nudge(&mut rng), nudge(&mut rng));
        let s = rng.seed_u8();
        let (xw, yw, zw) = (*rng.pick(SPECIAL_WRAPS), *rng.pick(SPECIAL_WRAPS), *rng.pick(SPECIAL_WRAPS));
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

fn next_after(v: f32, towards: f32) -> f32 {
    if v == towards {
        return v;
    }
    let bits = v.to_bits();
    let up = (towards > v) == (v.is_sign_positive() || v == 0.0);
    if v == 0.0 {
        return if towards > 0.0 { f32::from_bits(1) } else { -f32::from_bits(1) };
    }
    f32::from_bits(if up { bits + 1 } else { bits - 1 })
}

/// C13: large finite coordinates.
#[test]
fn c13_internal_large_coords() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C13 noise3_internal large coords");
    let big = [
        1048576.0f32,
        -1048576.0,
        16777216.0,
        -16777216.0,
        1073741824.0,
        -1073741824.0,
        2147483520.0,
        -2147483648.0,
        4294967296.0,
        -4294967296.0,
        1e30,
        -1e30,
        f32::MAX,
        f32::MIN,
    ];
    let mut rng = Rng::new(0xCD);
    for _ in 0..4000 {
        let (x, y, z) = (*rng.pick(&big), *rng.pick(&big), *rng.pick(&big));
        let s = rng.seed_u8();
        let (xw, yw, zw) = (*rng.pick(SPECIAL_WRAPS), *rng.pick(SPECIAL_WRAPS), *rng.pick(SPECIAL_WRAPS));
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={s}"),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// C14: everything random, including arbitrary float bit patterns.
#[test]
fn c14_internal_random_everything() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C14 noise3_internal fully randomised");
    let mut rng = Rng::new(0xCE);
    for _ in 0..8000 {
        let f = |rng: &mut Rng| match rng.below(4) {
            0 => rng.coord(16),
            1 => rng.finite_f32(),
            2 => *rng.pick(SPECIAL_F32),
            _ => rng.any_f32(),
        };
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let w = |rng: &mut Rng| match rng.below(3) {
            0 => *rng.pick(SPECIAL_WRAPS),
            1 => rng.next_i32(),
            _ => rng.range(-1000, 1000),
        };
        let (xw, yw, zw) = (w(&mut rng), w(&mut rng), w(&mut rng));
        let s = rng.seed_u8();
        d.check(
            format_args!(
                "wraps=({xw},{yw},{zw}) x={:#010x} y={:#010x} z={:#010x} seed={s}",
                x.to_bits(),
                y.to_bits(),
                z.to_bits()
            ),
            unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
        );
    }
    d.finish();
}

/// C15: exhaustive seed sweep.
#[test]
fn c15_internal_all_seeds() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C15 noise3_internal seed 0..=255");
    let coords = [
        (0.5f32, 0.5f32, 0.5f32),
        (1.25, -2.5, 3.125),
        (-0.75, 100.5, -1000.25),
        (0.0, 0.0, 0.0),
        (7.0, 7.0, 7.0),
    ];
    let wraps = [(0, 0, 0), (4, 8, 16), (3, 5, 7), (256, 1, 64), (-1, -5, 1024)];
    for &(x, y, z) in &coords {
        for &(xw, yw, zw) in &wraps {
            for s in 0..=255u8 {
                d.check(
                    format_args!("wraps=({xw},{yw},{zw}) x={x} y={y} z={z} seed={s}"),
                    unsafe { (c.noise3_internal)(x, y, z, xw, yw, zw, s) },
                    unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, s) },
                );
            }
        }
    }
    d.finish();
}

/// C16: the `seed = 0` convenience wrapper.
#[test]
fn c16_noise3_wrapper() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C16 stb_perlin_noise3");
    let mut rng = Rng::new(0xD0);
    for _ in 0..6000 {
        let f = |rng: &mut Rng| match rng.below(3) {
            0 => rng.coord(8),
            1 => rng.coord(2000),
            _ => *rng.pick(SPECIAL_F32),
        };
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let (xw, yw, zw) = (
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
        );
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e}"),
            unsafe { (c.noise3)(x, y, z, xw, yw, zw) },
            unsafe { (r.noise3)(x, y, z, xw, yw, zw) },
        );
        // The wrapper must agree with the internal function at seed 0 as well.
        d.check(
            format_args!("vs internal wraps=({xw},{yw},{zw})"),
            unsafe { (c.noise3)(x, y, z, xw, yw, zw) },
            unsafe { (r.noise3_internal)(x, y, z, xw, yw, zw, 0) },
        );
    }
    d.finish();
}

/// C17: `stb_perlin_noise3_seed` with random full-range `int` seeds.
#[test]
fn c17_noise3_seed_full_int() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C17 stb_perlin_noise3_seed random int seed");
    let mut rng = Rng::new(0xD1);
    for _ in 0..6000 {
        let (x, y, z) = (rng.coord(64), rng.coord(64), rng.coord(64));
        let (xw, yw, zw) = (
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
        );
        let seed = rng.next_i32();
        d.check(
            format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={seed}"),
            unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, seed) },
            unsafe { (r.noise3_seed)(x, y, z, xw, yw, zw, seed) },
        );
    }
    d.finish();
}

/// C18: seed boundary values.
#[test]
fn c18_noise3_seed_boundaries() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C18 stb_perlin_noise3_seed boundary seeds");
    let seeds = [
        0,
        1,
        127,
        128,
        255,
        256,
        257,
        -1,
        -255,
        -256,
        -257,
        i32::MAX,
        i32::MIN,
        65535,
        65536,
    ];
    let wraps = [(0, 0, 0), (4, 4, 4), (3, 5, 7), (256, 256, 256), (1, 1, 1)];
    let mut rng = Rng::new(0xD2);
    for &seed in &seeds {
        for &(xw, yw, zw) in &wraps {
            for _ in 0..40 {
                let (x, y, z) = (rng.coord(32), rng.coord(32), rng.coord(32));
                d.check(
                    format_args!("wraps=({xw},{yw},{zw}) x={x:e} y={y:e} z={z:e} seed={seed}"),
                    unsafe { (c.noise3_seed)(x, y, z, xw, yw, zw, seed) },
                    unsafe { (r.noise3_seed)(x, y, z, xw, yw, zw, seed) },
                );
            }
        }
    }
    d.finish();
}
