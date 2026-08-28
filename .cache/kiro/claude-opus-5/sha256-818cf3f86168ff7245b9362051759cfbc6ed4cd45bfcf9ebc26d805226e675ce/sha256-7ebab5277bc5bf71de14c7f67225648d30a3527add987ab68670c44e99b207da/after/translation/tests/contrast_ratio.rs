//! Differential tests for `contrast_ratio`.
//!
//! `cbLuminance` and `cbContrastRatio` are `static` in `c_src/src/lib.c`, so
//! they are not exported by the C `.so`; the only symbol either library
//! publishes is `contrast_ratio`. The suite therefore drives them through that
//! entry point, working from the narrowest behaviours outward:
//!
//! 1. the `> 0.04045` branch of the per-channel linearisation,
//! 2. one channel at a time, isolating each luminance coefficient,
//! 3. the `High`/`Low` swap and the unguarded division in `cbContrastRatio`,
//! 4. exhaustive and randomised sweeps of the full public surface.

mod common;

use common::{CbRgb255, Harness, INTERESTING, Rng};

/// Sanity check that both libraries actually loaded and agree on a plain case.
#[test]
fn loads_both_libraries_and_agrees_on_black_on_white() {
    let h = Harness::load();
    let white = CbRgb255::new(255, 255, 255);
    let black = CbRgb255::new(0, 0, 0);

    // Pure black has zero luminance, so the C code's unguarded `High / Low`
    // divides by zero; the Rust port must reproduce the same infinity.
    let c = h.c(white, black);
    assert_eq!(c.to_bits(), h.rust(white, black).to_bits());
    assert!(c.is_infinite(), "expected +inf from the unguarded divide, got {c}");

    h.check(white, white);
    h.check(black, black);
}

// ---------------------------------------------------------------------------
// Level 1: the per-channel linearisation branch inside cbLuminance
// ---------------------------------------------------------------------------

/// `0.04045 * 255 == 10.31`, so `i/255` crosses the threshold between 10 and
/// 11. Sweeping every byte on a grey ramp exercises both branches, including
/// the exact flip, on all three channels at once.
#[test]
fn linearize_branch_over_grey_ramp() {
    let h = Harness::load();
    // A fixed non-degenerate partner keeps the ratio finite so a difference in
    // the numerator cannot be masked by an infinity on both sides.
    let reference = CbRgb255::new(128, 128, 128);

    for v in 0u8..=255 {
        let grey = CbRgb255::new(v, v, v);
        h.check(grey, reference);
        h.check(reference, grey);
        h.check(grey, grey);
    }
}

/// Same threshold sweep, but per channel, so a wrong branch in a single
/// component cannot hide behind the other two.
#[test]
fn linearize_branch_per_channel() {
    let h = Harness::load();
    let reference = CbRgb255::new(200, 100, 50);

    for v in 0u8..=255 {
        for probe in [
            CbRgb255::new(v, 0, 0),
            CbRgb255::new(0, v, 0),
            CbRgb255::new(0, 0, v),
            CbRgb255::new(v, 255, 255),
            CbRgb255::new(255, v, 255),
            CbRgb255::new(255, 255, v),
        ] {
            h.check(probe, reference);
            h.check(reference, probe);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: the 0.2126 / 0.7152 / 0.0722 weighted sum
// ---------------------------------------------------------------------------

/// Each channel is swept alone against a single-channel partner, so the result
/// depends on exactly one coefficient per side and on the order of the
/// single-precision additions.
#[test]
fn luminance_coefficients_isolated() {
    let h = Harness::load();

    for a in 0u8..=255 {
        for b in INTERESTING.iter().copied() {
            // Red vs red, green vs green, blue vs blue.
            h.check(CbRgb255::new(a, 0, 0), CbRgb255::new(b, 0, 0));
            h.check(CbRgb255::new(0, a, 0), CbRgb255::new(0, b, 0));
            h.check(CbRgb255::new(0, 0, a), CbRgb255::new(0, 0, b));
            // Cross-channel: different coefficients on either side of the
            // division, which is where a swapped weight would show up.
            h.check(CbRgb255::new(a, 0, 0), CbRgb255::new(0, b, 0));
            h.check(CbRgb255::new(0, a, 0), CbRgb255::new(0, 0, b));
            h.check(CbRgb255::new(0, 0, a), CbRgb255::new(b, 0, 0));
        }
    }
}

/// Pairs of channels, to catch a mis-ordered or double-rounded accumulation
/// that a single-channel input would leave undetected.
#[test]
fn luminance_channel_pairs() {
    let h = Harness::load();

    for x in INTERESTING.iter().copied() {
        for y in INTERESTING.iter().copied() {
            let probes = [
                CbRgb255::new(x, y, 0),
                CbRgb255::new(x, 0, y),
                CbRgb255::new(0, x, y),
                CbRgb255::new(y, x, 255),
                CbRgb255::new(255, y, x),
            ];
            for (i, &p) in probes.iter().enumerate() {
                for &q in probes.iter().skip(i) {
                    h.check(p, q);
                    h.check(q, p);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 3: the High/Low swap and the unguarded division
// ---------------------------------------------------------------------------

/// The C code assigns `High = LumA, Low = LumB` and swaps only when
/// `High < Low`, so the result must be symmetric in its arguments — and the
/// Rust port must be symmetric in the same way.
#[test]
fn ratio_is_argument_order_independent_like_c() {
    let h = Harness::load();
    let mut rng = Rng::new(0x5EED_1234_ABCD_0001);

    for _ in 0..20_000 {
        let a = rng.color();
        let b = rng.color();

        h.check(a, b);
        h.check(b, a);

        // Whatever the C code produces for (a, b) it must also produce for
        // (b, a); confirm that on the C side too, then hold Rust to it.
        let forward = h.c(a, b);
        let reverse = h.c(b, a);
        assert_eq!(
            forward.to_bits(),
            reverse.to_bits(),
            "C is order-dependent for {a:?} / {b:?}?"
        );
        assert_eq!(h.rust(a, b).to_bits(), forward.to_bits());
        assert_eq!(h.rust(b, a).to_bits(), reverse.to_bits());
    }
}

/// Equal inputs drive `High == Low`: exactly `1.0` normally, but `NaN` for
/// black because `0.0 / 0.0` is not special-cased in the C source.
#[test]
fn ratio_with_equal_operands() {
    let h = Harness::load();

    for v in 0u8..=255 {
        let grey = CbRgb255::new(v, v, v);
        h.check(grey, grey);
    }

    let black = CbRgb255::new(0, 0, 0);
    let got = h.c(black, black);
    assert!(got.is_nan(), "expected NaN from 0/0, got {got}");
    assert_eq!(
        h.rust(black, black).to_bits(),
        got.to_bits(),
        "NaN sign/payload must match bit-for-bit"
    );
}

/// Zero-luminance denominators, i.e. every input whose ratio is `inf` or `NaN`.
/// Only `(0, 0, 0)` has zero luminance, so pair it with everything.
#[test]
fn ratio_with_zero_luminance_denominator() {
    let h = Harness::load();
    let black = CbRgb255::new(0, 0, 0);

    for v in 0u8..=255 {
        for probe in [
            CbRgb255::new(v, 0, 0),
            CbRgb255::new(0, v, 0),
            CbRgb255::new(0, 0, v),
            CbRgb255::new(v, v, v),
            CbRgb255::new(v, 255 - v, v / 2),
        ] {
            h.check(black, probe);
            h.check(probe, black);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 4: full public surface
// ---------------------------------------------------------------------------

/// Every grey-on-grey pair: 256 x 256 inputs covering both branches on both
/// sides of the division with no gaps.
#[test]
fn exhaustive_grey_pairs() {
    let h = Harness::load();

    for a in 0u8..=255 {
        for b in 0u8..=255 {
            h.check(CbRgb255::new(a, a, a), CbRgb255::new(b, b, b));
        }
    }
}

/// Cartesian product of the boundary-relevant byte values across all six
/// channels: 27^3 colours for A against a rotated set for B.
#[test]
fn exhaustive_interesting_channel_combinations() {
    let h = Harness::load();
    let n = INTERESTING.len();

    for (i, &r) in INTERESTING.iter().enumerate() {
        for (j, &g) in INTERESTING.iter().enumerate() {
            for (k, &b) in INTERESTING.iter().enumerate() {
                let a = CbRgb255::new(r, g, b);
                // Rotate the partner through the same table so every colour is
                // paired with a differently-shaped counterpart.
                let partner = CbRgb255::new(
                    INTERESTING[(i + 1) % n],
                    INTERESTING[(j + 7) % n],
                    INTERESTING[(k + 13) % n],
                );
                h.check(a, partner);
                h.check(partner, a);
            }
        }
    }
}

/// The whole 16.7M-colour cube for A against a handful of fixed partners,
/// so every representable `cb_rgb_255` value is passed through the FFI
/// boundary at least once.
#[test]
fn exhaustive_full_cube_against_fixed_partners() {
    let h = Harness::load();
    let partners = [
        CbRgb255::new(255, 255, 255),
        CbRgb255::new(0, 0, 0),
        CbRgb255::new(10, 11, 12),
        CbRgb255::new(37, 211, 3),
    ];

    for r in 0u8..=255 {
        for g in 0u8..=255 {
            for b in 0u8..=255 {
                let a = CbRgb255::new(r, g, b);
                for &p in &partners {
                    h.check(a, p);
                }
            }
        }
    }
}

/// Randomised pairs over the full input space, seeded for reproducibility.
#[test]
fn randomized_pairs() {
    let h = Harness::load();
    let mut rng = Rng::new(0xD1CE_F00D_1234_5678);

    for _ in 0..2_000_000 {
        h.check(rng.color(), rng.color());
    }
}

/// Randomised pairs biased toward the dark end, where the `/ 12.92` branch and
/// the tiny denominators live and where float rounding differences would be
/// most visible.
#[test]
fn randomized_dark_pairs() {
    let h = Harness::load();
    let mut rng = Rng::new(0x0BAD_C0DE_5EED_9999);

    for _ in 0..500_000 {
        let v = rng.next_u64();
        let a = CbRgb255::new((v % 14) as u8, ((v >> 8) % 14) as u8, ((v >> 16) % 14) as u8);
        let w = rng.next_u64();
        let b = CbRgb255::new((w % 14) as u8, ((w >> 8) % 14) as u8, ((w >> 16) % 14) as u8);
        h.check(a, b);
    }
}
