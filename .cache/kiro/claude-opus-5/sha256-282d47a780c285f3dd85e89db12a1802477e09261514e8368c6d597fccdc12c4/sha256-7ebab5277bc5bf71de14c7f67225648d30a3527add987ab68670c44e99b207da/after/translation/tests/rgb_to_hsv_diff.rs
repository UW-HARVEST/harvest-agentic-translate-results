//! Differential tests for the single public entry point, `rgb_to_hsv`.
//!
//! Both the C reference `.so` and the Rust `.so` are loaded with `libloading`
//! and invoked purely through their exported symbols, so the `#[no_mangle]`
//! wrapper is exercised exactly as an external caller would exercise it.

mod common;

use common::{Impls, Rng};

#[test]
fn exports_are_loadable() {
    // Loading alone proves both libraries export `rgb_to_hsv`.
    let _ = Impls::load();
}

/// Hand-picked values covering every branch: the early-return
/// (`delta == 0 || max == 0`), and each of the `r`/`g`/`b`-is-max hue arms
/// including the `h < 0` wrap-around.
#[test]
fn known_edge_cases() {
    let impls = Impls::load();
    let cases: &[[f32; 3]] = &[
        // early return: all channels equal (delta == 0)
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
        [-1.0, -1.0, -1.0],
        // max == 0 with non-zero delta (negative channels)
        [0.0, -1.0, -2.0],
        [-3.0, 0.0, -1.0],
        [-1.0, -2.0, 0.0],
        // primaries / secondaries
        [1.0, 0.0, 0.0], // red    -> h = 0
        [1.0, 1.0, 0.0], // yellow -> h = 60
        [0.0, 1.0, 0.0], // green  -> h = 120
        [0.0, 1.0, 1.0], // cyan   -> h = 180
        [0.0, 0.0, 1.0], // blue   -> h = 240
        [1.0, 0.0, 1.0], // magenta-> h = 300
        // r is max, g < b  => negative hue, exercises h += 360
        [1.0, 0.0, 0.5],
        [1.0, 0.25, 0.75],
        [0.9, 0.1, 0.899],
        // g is max
        [0.2, 0.9, 0.4],
        [0.0, 0.5, 0.25],
        // b is max
        [0.3, 0.1, 0.8],
        [0.25, 0.0, 0.5],
        // ties between channels (branch order matters)
        [1.0, 1.0, 0.5],
        [0.5, 1.0, 1.0],
        [1.0, 0.5, 1.0],
        // out-of-gamut / large magnitudes
        [255.0, 128.0, 0.0],
        [1e30, 1.0, -1e30],
        [f32::MAX, 0.0, 0.0],
        [f32::MIN, f32::MAX, 0.0],
        // subnormals and tiny deltas
        [f32::MIN_POSITIVE, 0.0, 0.0],
        [1.0e-45, 0.0, 0.0], // smallest subnormal
        [1.0, 1.0 - f32::EPSILON, 1.0],
        // signed zeros (-0.0 == 0.0, but comparisons/ordering still matter)
        [-0.0, 0.0, 0.0],
        [0.0, -0.0, -0.0],
        [-0.0, -0.0, -0.0],
        [1.0, -0.0, 0.0],
        // infinities
        [f32::INFINITY, 0.0, 0.0],
        [f32::INFINITY, f32::INFINITY, 0.0],
        [f32::NEG_INFINITY, 0.0, 1.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
    ];
    for &src in cases {
        impls.assert_match(src);
    }
}

/// NaN inputs make every comparison false, so the C ternaries always pick the
/// second operand. Verified separately because it is the most fragile part of
/// the translation.
#[test]
fn nan_inputs() {
    let impls = Impls::load();
    let n = f32::NAN;
    let cases: &[[f32; 3]] = &[
        [n, 0.0, 0.0],
        [0.0, n, 0.0],
        [0.0, 0.0, n],
        [n, n, 0.0],
        [n, 0.0, n],
        [0.0, n, n],
        [n, n, n],
        [n, 1.0, 2.0],
        [1.0, n, 2.0],
        [1.0, 2.0, n],
        [-n, 1.0, 2.0],
        [1.0, -n, 2.0],
        [1.0, 2.0, -n],
        [n, f32::INFINITY, f32::NEG_INFINITY],
    ];
    for &src in cases {
        impls.assert_match(src);
    }
}

/// Normalised colour channels, the intended input domain.
#[test]
fn random_unit_range() {
    let impls = Impls::load();
    let mut rng = Rng::new(0x5EED_1234);
    for _ in 0..200_000 {
        impls.assert_match([rng.unit(), rng.unit(), rng.unit()]);
    }
}

/// Quantised values force frequent exact ties between channels, hitting the
/// `r == max` / `g == max` equality branches far more often than uniform
/// random floats do.
#[test]
fn random_quantised_ties() {
    let impls = Impls::load();
    let mut rng = Rng::new(0xABCD_EF01);
    for _ in 0..200_000 {
        let q = |r: &mut Rng| ((r.next_u32() % 5) as f32) / 4.0;
        impls.assert_match([q(&mut rng), q(&mut rng), q(&mut rng)]);
    }
}

/// Signed, wide-exponent values including out-of-gamut and negative channels.
#[test]
fn random_wide_range() {
    let impls = Impls::load();
    let mut rng = Rng::new(0x1357_9BDF);
    for _ in 0..200_000 {
        let v = |r: &mut Rng| {
            let mag = r.unit() * 2.0 - 1.0;
            let exp = (r.next_u32() % 60) as i32 - 30;
            mag * 2.0f32.powi(exp)
        };
        impls.assert_match([v(&mut rng), v(&mut rng), v(&mut rng)]);
    }
}

/// Fully arbitrary bit patterns: NaNs with assorted payloads, infinities,
/// subnormals and signed zeros in every position.
#[test]
fn random_arbitrary_bits() {
    let impls = Impls::load();
    let mut rng = Rng::new(0x2468_ACE0);
    for _ in 0..200_000 {
        impls.assert_match([rng.any_f32(), rng.any_f32(), rng.any_f32()]);
    }
}

/// Exhaustive sweep over a structured grid of "interesting" float values in
/// all three positions (23^3 = 12167 combinations).
#[test]
fn exhaustive_interesting_grid() {
    let impls = Impls::load();
    let vals: &[f32] = &[
        0.0,
        -0.0,
        1.0e-45,
        -1.0e-45,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::EPSILON,
        0.25,
        0.5,
        1.0 - f32::EPSILON,
        1.0,
        1.0 + f32::EPSILON,
        2.0,
        -1.0,
        -0.5,
        255.0,
        1.0e20,
        -1.0e20,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &r in vals {
        for &g in vals {
            for &b in vals {
                impls.assert_match([r, g, b]);
            }
        }
    }
}

/// Exhaustive sweep of the low 16 bits of the float encoding in the first
/// channel (subnormals and the smallest normals) against fixed partners.
#[test]
fn exhaustive_low_bit_patterns() {
    let impls = Impls::load();
    let partners: &[(f32, f32)] = &[
        (0.0, 0.0),
        (1.0, 0.5),
        (-1.0, -2.0),
        (0.0, -0.0),
        (f32::NAN, 1.0),
    ];
    for hi in [0u32, 0x8000_0000] {
        for low in 0u32..=0xFFFF {
            let x = f32::from_bits(hi | low);
            for &(g, b) in partners {
                impls.assert_match([x, g, b]);
                impls.assert_match([g, x, b]);
                impls.assert_match([g, b, x]);
            }
        }
    }
}
