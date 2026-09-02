//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `dlopen`/`dlsym` and asserts the three output `f32`s are
//! bit-identical. Inputs are property-style batches from a fixed-seed PRNG (or
//! exhaustive where the domain is finite), never a single hand-picked value.

mod common;

use common::*;

const ITERS: usize = 20_000;

// ---------------------------------------------------------------------------
// Axis S — the `if (s == 0)` fast path
// ---------------------------------------------------------------------------

/// Row 1: `s = +0.0`, random `h`/`l`. `dest` must be `(l, l, l)`.
#[test]
fn cfg_row01_s_plus_zero_random_h_l() {
    let mut rng = Rng::new(1);
    let inputs = (0..ITERS).map(|_| [rng.finite(), 0.0f32, rng.finite()]);
    assert_same_batch("CONFIGS row 1", inputs.collect::<Vec<_>>());
}

/// Row 2: `s = -0.0`. IEEE says `-0.0 == 0`, so the fast path must still fire.
#[test]
fn cfg_row02_s_minus_zero_random_h_l() {
    let mut rng = Rng::new(2);
    let inputs = (0..ITERS).map(|_| [rng.finite(), -0.0f32, rng.finite()]);
    assert_same_batch("CONFIGS row 2", inputs.collect::<Vec<_>>());
}

/// Row 3: fast path copies `l` verbatim, including non-finite `l`. Exhaustive
/// over the edge pool × both zero signs × a few random hues.
#[test]
fn cfg_row03_s_zero_edge_lightness() {
    let mut rng = Rng::new(3);
    let mut inputs = Vec::new();
    for &s in &[0.0f32, -0.0f32] {
        for &l in edge_values().iter() {
            for _ in 0..64 {
                inputs.push([rng.any_bits(), s, l]);
            }
        }
    }
    assert_same_batch("CONFIGS row 3", inputs);
}

// ---------------------------------------------------------------------------
// Axis H — the seven hue arms
// ---------------------------------------------------------------------------

/// Random `s` strictly inside `(0, 1]` so the slow path is always taken.
fn sat(rng: &mut Rng) -> f32 {
    let v = rng.range(f32::MIN_POSITIVE, 1.0);
    if v == 0.0 { 1.0 } else { v }
}

fn hue_arm_inputs(stream: u64, lo: f32, hi: f32) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(stream);
    (0..ITERS)
        .map(|_| {
            let h = rng.range(lo, hi);
            [h, sat(&mut rng), rng.range(0.0, 1.0)]
        })
        .collect()
}

/// Row 4: arm 1, `h ∈ [0, 60)` → `(c+m, x+m, m)`.
#[test]
fn cfg_row04_arm1_hue_0_60() {
    assert_same_batch("CONFIGS row 4", hue_arm_inputs(4, 0.0, 60.0));
}

/// Row 5: arm 2, `h ∈ [60, 120)` → `(x+m, c+m, m)`.
#[test]
fn cfg_row05_arm2_hue_60_120() {
    assert_same_batch("CONFIGS row 5", hue_arm_inputs(5, 60.0, 120.0));
}

/// Row 6: `h ∈ [120, 180)`. The arm-3 predicate is `h < 120 && h < 180`, so this
/// range is orphaned and falls through to the terminal `else` → `(m, m, m)`.
#[test]
fn cfg_row06_arm7_hue_120_180_orphaned() {
    assert_same_batch("CONFIGS row 6", hue_arm_inputs(6, 120.0, 180.0));
}

/// Row 7: arm 4, `h ∈ [180, 240)` → `(m, x+m, c+m)`.
#[test]
fn cfg_row07_arm4_hue_180_240() {
    assert_same_batch("CONFIGS row 7", hue_arm_inputs(7, 180.0, 240.0));
}

/// Row 8: arm 5, `h ∈ [240, 300)` → `(x+m, m, c+m)`.
#[test]
fn cfg_row08_arm5_hue_240_300() {
    assert_same_batch("CONFIGS row 8", hue_arm_inputs(8, 240.0, 300.0));
}

/// Row 9: arm 6, `h ∈ [300, 360)` → `(c+m, m, x+m)`.
#[test]
fn cfg_row09_arm6_hue_300_360() {
    assert_same_batch("CONFIGS row 9", hue_arm_inputs(9, 300.0, 360.0));
}

/// Row 10: arm 3 is reachable ONLY for `h < 0`, because arms 1-2 consume
/// `[0, 120)` and the predicate is `h < 120`, not `h >= 120`.
#[test]
fn cfg_row10_arm3_negative_hue() {
    let mut rng = Rng::new(10);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| {
            let h = -rng.range(f32::MIN_POSITIVE, 1.0e6);
            [h, sat(&mut rng), rng.range(0.0, 1.0)]
        })
        .collect();
    assert_same_batch("CONFIGS row 10", inputs);
}

/// Row 11: `h >= 360` → terminal `else`.
#[test]
fn cfg_row11_arm7_hue_above_360() {
    assert_same_batch("CONFIGS row 11", hue_arm_inputs(11, 360.0, 1.0e9));
}

// ---------------------------------------------------------------------------
// Axis B — hue exactly on / adjacent to each threshold
// ---------------------------------------------------------------------------

/// Row 12: each of `{0,60,120,180,240,300,360}` exactly, plus both `nextafter`
/// neighbours, crossed with randomized `s` and `l`. `>=` is inclusive and `<`
/// exclusive, so a threshold belongs to the arm above it.
#[test]
fn cfg_row12_hue_threshold_boundaries() {
    let mut rng = Rng::new(12);
    let mut hues: Vec<f32> = Vec::new();
    for &t in THRESHOLDS {
        hues.push(t);
        hues.push(next_up(t));
        hues.push(next_down(t));
    }
    let mut inputs = Vec::new();
    for h in hues {
        for _ in 0..2000 {
            inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
        }
    }
    assert_same_batch("CONFIGS row 12", inputs);
}

/// Smallest representable step up, handling `0.0` and sign changes.
fn next_up(x: f32) -> f32 {
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

/// Smallest representable step down.
fn next_down(x: f32) -> f32 {
    if x == 0.0 {
        return f32::from_bits(1 | 0x8000_0000);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

// ---------------------------------------------------------------------------
// Axis L — lightness regimes
// ---------------------------------------------------------------------------

/// Row 13: `l = 0.5` exactly makes `|2l - 1| == 0`, so `c == s` bit-for-bit.
#[test]
fn cfg_row13_lightness_exactly_half() {
    let mut rng = Rng::new(13);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| [rng.range(0.0, 360.0), sat(&mut rng), 0.5f32])
        .collect();
    assert_same_batch("CONFIGS row 13", inputs);
}

/// Row 14: `l ∈ {0.0, 1.0}` makes `c == 0` and `m == l`, so every arm collapses
/// to grey — but through different `addss` operand orders.
#[test]
fn cfg_row14_lightness_at_endpoints() {
    let mut rng = Rng::new(14);
    let mut inputs = Vec::new();
    for &l in &[0.0f32, 1.0f32] {
        for _ in 0..10_000 {
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), l]);
        }
    }
    assert_same_batch("CONFIGS row 14", inputs);
}

/// Row 15: `l ∈ (0, 0.5)`.
#[test]
fn cfg_row15_lightness_lower_half() {
    let mut rng = Rng::new(15);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| {
            [
                rng.range(0.0, 360.0),
                sat(&mut rng),
                rng.range(f32::MIN_POSITIVE, 0.5),
            ]
        })
        .collect();
    assert_same_batch("CONFIGS row 15", inputs);
}

/// Row 16: `l ∈ (0.5, 1)`.
#[test]
fn cfg_row16_lightness_upper_half() {
    let mut rng = Rng::new(16);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| [rng.range(0.0, 360.0), sat(&mut rng), rng.range(0.5, 1.0)])
        .collect();
    assert_same_batch("CONFIGS row 16", inputs);
}

/// Row 17: `l` outside `[0, 1]`. No clamping happens; `c` goes negative.
#[test]
fn cfg_row17_lightness_outside_unit() {
    let mut rng = Rng::new(17);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| {
            let l = if rng.next_u32() & 1 == 0 {
                rng.range(-100.0, 0.0)
            } else {
                rng.range(1.0, 100.0)
            };
            [rng.range(-400.0, 400.0), sat(&mut rng), l]
        })
        .collect();
    assert_same_batch("CONFIGS row 17", inputs);
}

/// Row 18: `s` outside `(0, 1]`. No clamping.
#[test]
fn cfg_row18_saturation_outside_unit() {
    let mut rng = Rng::new(18);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| {
            let s = if rng.next_u32() & 1 == 0 {
                rng.range(-100.0, -f32::MIN_POSITIVE)
            } else {
                rng.range(1.0, 100.0)
            };
            [rng.range(-400.0, 400.0), s, rng.range(-2.0, 3.0)]
        })
        .collect();
    assert_same_batch("CONFIGS row 18", inputs);
}

// ---------------------------------------------------------------------------
// Axis F — the `fmodf(h / 60, 2)` reduction
// ---------------------------------------------------------------------------

/// Row 19: `h` an exact multiple of 60, so `h/60` is an exact integer and
/// `fmodf(·, 2)` is exactly `0` (even) or `±1` (odd) → `x == c` or `x == 0`.
#[test]
fn cfg_row19_fmod_integer_multiples() {
    let mut rng = Rng::new(19);
    let mut inputs = Vec::new();
    for k in -400i32..=400 {
        let h = (k as f32) * 60.0;
        for _ in 0..8 {
            inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
        }
    }
    assert_same_batch("CONFIGS row 19", inputs);
}

/// Row 20: `|h|` huge, so glibc `fmodf` performs a long argument reduction.
/// This is the row most likely to expose a `%`-vs-`fmodf` mismatch.
#[test]
fn cfg_row20_fmod_huge_hue() {
    let mut rng = Rng::new(20);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| {
            // Random exponent from 2^50 up to near f32::MAX, random mantissa.
            let exp: u32 = 177 + (rng.next_u32() % 78); // biased exponents 177..254
            let mant: u32 = rng.next_u32() & 0x007F_FFFF;
            let sign: u32 = (rng.next_u32() & 1) << 31;
            let h = f32::from_bits(sign | (exp << 23) | mant);
            [h, sat(&mut rng), rng.range(0.0, 1.0)]
        })
        .collect();
    assert_same_batch("CONFIGS row 20", inputs);
}

/// Row 21: `h` subnormal or tiny, so `h/60` underflows to a subnormal or `±0`.
#[test]
fn cfg_row21_fmod_subnormal_hue() {
    let mut rng = Rng::new(21);
    let inputs: Vec<_> = (0..ITERS)
        .map(|_| {
            let exp: u32 = rng.next_u32() % 12; // 0 (subnormal) .. 11
            let mant: u32 = rng.next_u32() & 0x007F_FFFF;
            let sign: u32 = (rng.next_u32() & 1) << 31;
            let h = f32::from_bits(sign | (exp << 23) | mant);
            [h, sat(&mut rng), rng.range(0.0, 1.0)]
        })
        .collect();
    assert_same_batch("CONFIGS row 21", inputs);
}

// ---------------------------------------------------------------------------
// Axis N — non-finite / edge bit patterns
// ---------------------------------------------------------------------------

/// Row 22: `h` from the edge pool.
#[test]
fn cfg_row22_edge_hue_patterns() {
    let mut rng = Rng::new(22);
    let mut inputs = Vec::new();
    for &h in edge_values().iter() {
        for _ in 0..2000 {
            inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
        }
    }
    assert_same_batch("CONFIGS row 22", inputs);
}

/// Row 23: `s` from the edge pool, excluding `±0.0` (that is row 3).
#[test]
fn cfg_row23_edge_saturation_patterns() {
    let mut rng = Rng::new(23);
    let mut inputs = Vec::new();
    for &s in edge_values_nonzero().iter() {
        for _ in 0..2000 {
            inputs.push([rng.range(-400.0, 400.0), s, rng.range(-1.0, 2.0)]);
        }
    }
    assert_same_batch("CONFIGS row 23", inputs);
}

/// Row 24: `l` from the edge pool. This is the row that exposes NaN
/// operand-order bugs: `fabsf` is an `andps` (clears the sign without quieting)
/// so `c` and `x` end up sign-positive, while `m = l - 0.5c` re-propagates `l`
/// itself and keeps `l`'s sign. `addss` returns the DESTINATION operand's NaN
/// when both are NaN, so `c + m` and `m + c` differ observably.
#[test]
fn cfg_row24_edge_lightness_patterns() {
    let mut rng = Rng::new(24);
    let mut inputs = Vec::new();
    for &l in edge_values().iter() {
        for _ in 0..2000 {
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), l]);
        }
    }
    assert_same_batch("CONFIGS row 24", inputs);
}

/// Row 25: full cross-product of the edge pool across all three slots.
#[test]
fn cfg_row25_edge_pattern_cross_product() {
    let pool = edge_values();
    let mut inputs = Vec::with_capacity(pool.len().pow(3));
    for &h in &pool {
        for &s in &pool {
            for &l in &pool {
                inputs.push([h, s, l]);
            }
        }
    }
    assert_same_batch("CONFIGS row 25", inputs);
}

/// Row 26: unconstrained fuzz over raw 32-bit patterns in all three slots.
#[test]
fn cfg_row26_uniform_bitpattern_fuzz() {
    let mut rng = Rng::new(26);
    let inputs: Vec<_> = (0..300_000)
        .map(|_| [rng.any_bits(), rng.any_bits(), rng.any_bits()])
        .collect();
    assert_same_batch("CONFIGS row 26", inputs);
}

// ---------------------------------------------------------------------------
// Axis A — aliasing, and buffer bounds
// ---------------------------------------------------------------------------

/// Random inputs biased to hit both the fast and slow paths, plus edge patterns.
fn alias_inputs(stream: u64) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(stream);
    let pool = edge_values();
    (0..ITERS)
        .map(|_| match rng.next_u32() % 4 {
            0 => [rng.range(-400.0, 400.0), 0.0, rng.range(0.0, 1.0)],
            1 => [rng.range(-400.0, 400.0), -0.0, rng.range(0.0, 1.0)],
            2 => [rng.pick(&pool), rng.pick(&pool), rng.pick(&pool)],
            _ => [rng.range(-400.0, 400.0), rng.range(0.0, 1.0), rng.range(0.0, 1.0)],
        })
        .collect()
}

/// Row 27: `dest == src`. At `-O0` the C loads `h`, `s`, `l` into stack slots
/// before any store, so this is well-defined in practice.
#[test]
fn cfg_row27_alias_dest_equals_src() {
    assert_same_aliased_batch("CONFIGS row 27", 2, 2, alias_inputs(27));
}

/// Row 28: `dest == src + 1` (forward partial overlap).
#[test]
fn cfg_row28_alias_dest_offset_plus_one() {
    assert_same_aliased_batch("CONFIGS row 28", 3, 2, alias_inputs(28));
}

/// Row 29: `dest == src - 1` (backward partial overlap).
#[test]
fn cfg_row29_alias_dest_offset_minus_one() {
    assert_same_aliased_batch("CONFIGS row 29", 1, 2, alias_inputs(29));
}

/// Row 30: disjoint over-provisioned buffers. `assert_same_aliased_batch`
/// compares the entire 8-float arena, so it also proves neither library writes
/// outside `dest[0..3]` on either path.
#[test]
fn cfg_row30_no_out_of_bounds_write() {
    assert_same_aliased_batch("CONFIGS row 30", 0, 4, alias_inputs(30));
}
