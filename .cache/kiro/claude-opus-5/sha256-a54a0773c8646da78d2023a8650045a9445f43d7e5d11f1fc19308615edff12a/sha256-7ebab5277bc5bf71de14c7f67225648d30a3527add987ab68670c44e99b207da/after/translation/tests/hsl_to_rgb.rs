//! Differential tests for `hsl_to_rgb`, the single public entry point declared
//! in `c_src/include/lib.h`.
//!
//! Both the C reference and the Rust translation are loaded as shared objects
//! via `libloading` and compared bit-for-bit.

mod common;

use common::{Pair, Rng, assert_matches, assert_matches_aliased, special_floats, ulp_offset};

/// Both `.so`s load and export the symbol at all.
#[test]
fn symbol_is_exported_by_both() {
    let pair = Pair::load();
    let _c = pair.c_hsl_to_rgb();
    let _r = pair.rust_hsl_to_rgb();
}

/// `s == 0` short-circuit: all three channels take `l` verbatim, whatever `l`
/// is (including NaN, infinities and negative zero).
#[test]
fn saturation_zero_shortcut() {
    let pair = Pair::load();
    for &h in &special_floats() {
        for &s in &[0.0f32, -0.0f32] {
            for &l in &special_floats() {
                assert_matches(&pair, [h, s, l], "s==0");
            }
        }
    }
}

/// Exhaustive sweep of the hue branch boundaries crossed with a range of
/// saturation and lightness values.
#[test]
fn hue_branch_boundaries() {
    let pair = Pair::load();
    let hues = special_floats();
    let sats = [
        0.001f32, 0.25, 0.5, 0.75, 1.0, 2.0, -1.0, 1e-30, f32::MAX,
    ];
    let lights = [0.0f32, -0.0, 0.1, 0.25, 0.5, 0.75, 1.0, -1.0, 2.0, 1e30];
    for &h in &hues {
        for &s in &sats {
            for &l in &lights {
                assert_matches(&pair, [h, s, l], "hue-branch");
            }
        }
    }
}

/// Dense walk over the nominal hue domain plus a margin on either side, at a
/// step that lands both on and between the 60-degree sector edges.
#[test]
fn dense_hue_sweep() {
    let pair = Pair::load();
    let mut h = -400.0f32;
    while h <= 800.0 {
        for &s in &[0.05f32, 0.5, 1.0] {
            for &l in &[0.15f32, 0.5, 0.85] {
                assert_matches(&pair, [h, s, l], "dense-sweep");
            }
        }
        h += 0.5;
    }
}

/// Every combination of special values across all three inputs.
#[test]
fn special_value_cross_product() {
    let pair = Pair::load();
    let vals = special_floats();
    for &h in &vals {
        for &s in &vals {
            for &l in &vals {
                assert_matches(&pair, [h, s, l], "specials");
            }
        }
    }
}

/// Walk a few thousand ULPs either side of every 60-degree sector boundary.
///
/// These are the inputs where both the branch selection and `fmodf`'s quotient
/// can flip, so any disagreement in rounding or comparison shows up here.
#[test]
fn ulp_walk_around_sector_boundaries() {
    let pair = Pair::load();
    let sats = [0.25f32, 1.0];
    let lights = [0.2f32, 0.5, 0.8];
    for &boundary in &[
        0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0, -60.0, -120.0,
    ] {
        for step in -3000i32..=3000 {
            let h = ulp_offset(boundary, step);
            for &s in &sats {
                for &l in &lights {
                    assert_matches(&pair, [h, s, l], "ulp-walk");
                }
            }
        }
    }
}

/// `h` values whose quotient `h / 60` sits right at an even integer, which is
/// where `fmodf`'s wrap-around happens.
#[test]
fn fmod_wrap_points() {
    let pair = Pair::load();
    for k in -20i32..=20 {
        let base = 120.0f32 * k as f32; // h / 60 == 2k
        for step in -200i32..=200 {
            let h = ulp_offset(base, step);
            for &s in &[0.1f32, 0.6, 1.0] {
                for &l in &[0.35f32, 0.5, 0.65] {
                    assert_matches(&pair, [h, s, l], "fmod-wrap");
                }
            }
        }
    }
}

/// Randomised inputs drawn from plausible HSL ranges.
#[test]
fn fuzz_plausible_range() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x5EED_1234_ABCD_0001);
    for _ in 0..1_000_000 {
        let h = rng.range(-30.0, 400.0);
        let s = rng.range(0.0, 1.0);
        let l = rng.range(0.0, 1.0);
        assert_matches(&pair, [h, s, l], "fuzz-plausible");
    }
}

/// Randomised inputs over the whole `f32` bit space, so NaNs, infinities,
/// subnormals and huge magnitudes are all exercised.
#[test]
fn fuzz_full_bit_space() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    for _ in 0..1_000_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_matches(&pair, src, "fuzz-bits");
    }
}

/// Randomised inputs where `h` is plausible but `s`/`l` are arbitrary bit
/// patterns, so the NaN-heavy paths through `c`, `m` and `x` are all reachable
/// (a NaN `h` alone falls straight into the final `else` branch and would hide
/// them).
#[test]
fn fuzz_live_hue_wild_sat_light() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0BAD_F00D_0000_1357);
    for _ in 0..1_000_000 {
        let h = rng.range(-400.0, 400.0);
        let src = [h, rng.any_f32(), rng.any_f32()];
        assert_matches(&pair, src, "fuzz-live-hue");
    }
}

/// Randomised inputs with wide-but-finite magnitudes, biased towards values
/// that make `c`, `m` and `x` overflow or cancel.
#[test]
fn fuzz_wide_magnitudes() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xABCD_0000_FFFF_0007);
    let scales = [1e-38f32, 1e-20, 1e-6, 1.0, 1e6, 1e20, 1e38];
    for i in 0..120_000usize {
        let sc = scales[i % scales.len()];
        let sign = if rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        let h = rng.range(-720.0, 720.0);
        let s = sign * rng.range(0.0, 1.0) * sc;
        let l = rng.range(-1.0, 2.0) * scales[(i / 7) % scales.len()];
        assert_matches(&pair, [h, s, l], "fuzz-wide");
    }
}

/// `dest` and `src` pointing at the same buffer must behave identically in
/// both implementations.
#[test]
fn aliased_buffers() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for &h in &special_floats() {
        for &s in &[0.0f32, 0.4, 1.0, f32::NAN] {
            for &l in &[0.0f32, 0.3, 0.5, 1.0, f32::INFINITY] {
                assert_matches_aliased(&pair, [h, s, l], "aliased-special");
            }
        }
    }
    for _ in 0..50_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_matches_aliased(&pair, src, "aliased-fuzz");
    }
}
