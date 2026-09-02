//! Differential tests: C `.so` vs Rust `.so`, both reached via `libloading`.
//!
//! * Phase B — one test per row of `CONFIGS.md` (`cfg_r*`), each with
//!   `ROW_SAMPLES` randomized inputs from a fixed seed.
//! * Phase C — one test per row of `ERRORS.md` (`err_e*`).
//!
//! The C is the ground truth; any divergence is a Rust bug.

// The coefficients re-derived in `err_e2_...` are copied verbatim from the C, for
// the same reason as in `src/lib.rs`: identical decimal text is what makes the
// rounding identical.
#![allow(clippy::excessive_precision)]

mod common;

use common::{
    CbRgb255, EXTREMES, GAMMA_LINEAR_MAX, GAMMA_POW_MIN, Pair, ROW_SAMPLES, Rng, SEED, check_row,
    load,
};

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// R0 — EXHAUSTIVE: all 2^24 byte triples. Subsumes every other row.
///
/// Any divergence is reported with the exact input; the first few are collected
/// before failing so a systematic bug is obvious from a single run.
#[test]
fn cfg_r0_exhaustive_all_16777216_inputs() {
    let pair = load();
    let mut mismatches: Vec<(CbRgb255, CbRgb255, CbRgb255)> = Vec::new();
    let mut checked: u64 = 0;

    for r in 0..=255u8 {
        for g in 0..=255u8 {
            for b in 0..=255u8 {
                let input = CbRgb255::new(r, g, b);
                let expected = pair.c.call(input);
                let actual = pair.rust.call(input);
                checked += 1;
                if expected != actual && mismatches.len() < 32 {
                    mismatches.push((input, expected, actual));
                }
            }
        }
    }

    assert_eq!(checked, 1 << 24, "did not cover the whole input domain");
    assert!(
        mismatches.is_empty(),
        "R0: {} sampled divergences (of {checked} inputs); first cases (input, C, Rust): {:?}",
        mismatches.len(),
        mismatches
    );
}

/// R1 — pre-gamma linear branch on all three channels (every channel <= 10).
#[test]
fn cfg_r1_removegamma_linear_all_channels() {
    let pair = load();
    check_row(&pair, "R1", |rng| {
        CbRgb255::new(
            rng.u8_upto(GAMMA_LINEAR_MAX),
            rng.u8_upto(GAMMA_LINEAR_MAX),
            rng.u8_upto(GAMMA_LINEAR_MAX),
        )
    });
    // The regime is only 11^3 = 1331 points, so enumerate it exhaustively too.
    for r in 0..=GAMMA_LINEAR_MAX {
        for g in 0..=GAMMA_LINEAR_MAX {
            for b in 0..=GAMMA_LINEAR_MAX {
                pair.agree(CbRgb255::new(r, g, b));
            }
        }
    }
}

/// R2 — pre-gamma `pow` branch on all three channels (every channel >= 11).
#[test]
fn cfg_r2_removegamma_pow_all_channels() {
    let pair = load();
    check_row(&pair, "R2", |rng| {
        CbRgb255::new(
            rng.u8_in(GAMMA_POW_MIN, 255),
            rng.u8_in(GAMMA_POW_MIN, 255),
            rng.u8_in(GAMMA_POW_MIN, 255),
        )
    });
}

/// R3 — mixed: red linear, green/blue `pow`.
#[test]
fn cfg_r3_removegamma_mixed_red_linear() {
    let pair = load();
    check_row(&pair, "R3", |rng| {
        CbRgb255::new(
            rng.u8_upto(GAMMA_LINEAR_MAX),
            rng.u8_in(GAMMA_POW_MIN, 255),
            rng.u8_in(GAMMA_POW_MIN, 255),
        )
    });
}

/// R4 — mixed: green linear, red/blue `pow`.
#[test]
fn cfg_r4_removegamma_mixed_green_linear() {
    let pair = load();
    check_row(&pair, "R4", |rng| {
        CbRgb255::new(
            rng.u8_in(GAMMA_POW_MIN, 255),
            rng.u8_upto(GAMMA_LINEAR_MAX),
            rng.u8_in(GAMMA_POW_MIN, 255),
        )
    });
}

/// R5 — mixed: blue linear, red/green `pow`.
#[test]
fn cfg_r5_removegamma_mixed_blue_linear() {
    let pair = load();
    check_row(&pair, "R5", |rng| {
        CbRgb255::new(
            rng.u8_in(GAMMA_POW_MIN, 255),
            rng.u8_in(GAMMA_POW_MIN, 255),
            rng.u8_upto(GAMMA_LINEAR_MAX),
        )
    });
}

/// R6 — the measured 10/11 threshold, all 64 combinations of `{9,10,11,12}`.
#[test]
fn cfg_r6_removegamma_threshold_boundary() {
    let pair = load();
    const NEAR: [u8; 4] = [9, 10, 11, 12];
    for &r in &NEAR {
        for &g in &NEAR {
            for &b in &NEAR {
                pair.agree(CbRgb255::new(r, g, b));
            }
        }
    }
    check_row(&pair, "R6", |rng| {
        CbRgb255::new(rng.pick(&NEAR), rng.pick(&NEAR), rng.pick(&NEAR))
    });
}

/// R7 — `G > B` driving the red row above 1.0, so `cbDenorm` converts a float
/// >= 256 and the C's `cvttss2si` + low-byte store wraps mod 256.
#[test]
fn cfg_r7_red_row_overflows_above_one() {
    let pair = load();
    check_row(&pair, "R7", |rng| {
        CbRgb255::new(rng.u8_in(200, 255), rng.u8_in(200, 255), rng.u8_upto(40))
    });
    // The measured argmax of the red row.
    pair.agree(CbRgb255::new(255, 255, 0));
}

/// R8 — `G < B` driving the red row negative, so `cbDenorm` converts a negative
/// float and wraps.
#[test]
fn cfg_r8_red_row_goes_negative() {
    let pair = load();
    check_row(&pair, "R8", |rng| {
        CbRgb255::new(rng.u8_upto(40), rng.u8_upto(40), rng.u8_in(200, 255))
    });
    // The measured argmin of the red row.
    pair.agree(CbRgb255::new(0, 0, 255));
}

/// R9 — `G == B`, where the red row's two coefficients differ only in the last
/// few ulps (`0.12739886310880` vs `0.12739886341072`), so the residue is a
/// value-dependent tiny quantity rather than exactly zero.
#[test]
fn cfg_r9_green_equals_blue() {
    let pair = load();
    for v in 0..=255u8 {
        for r in (0..=255u8).step_by(17) {
            pair.agree(CbRgb255::new(r, v, v));
        }
    }
    check_row(&pair, "R9", |rng| {
        let v = rng.next_u8();
        CbRgb255::new(rng.next_u8(), v, v)
    });
}

/// R10 — post-matrix red small enough to take `cbApplyGammaRGB`'s linear branch.
#[test]
fn cfg_r10_applygamma_linear_on_red() {
    let pair = load();
    check_row(&pair, "R10", |rng| {
        let g = rng.u8_upto(12);
        CbRgb255::new(rng.u8_upto(8), g, g.saturating_add(rng.u8_upto(2)))
    });
}

/// R11 — green and blue outputs both take the `cbApplyGammaRGB` linear branch
/// while red takes `pow`.
#[test]
fn cfg_r11_applygamma_linear_on_green_and_blue() {
    let pair = load();
    check_row(&pair, "R11", |rng| {
        CbRgb255::new(rng.u8_in(GAMMA_POW_MIN, 255), rng.u8_upto(4), rng.u8_upto(4))
    });
}

/// R12 — the grey axis `R = G = B`, exhaustively.
#[test]
fn cfg_r12_grey_axis() {
    let pair = load();
    for v in 0..=255u8 {
        pair.agree(CbRgb255::new(v, v, v));
    }
}

/// R13 — exactly two channels equal, in all three pairings.
#[test]
fn cfg_r13_two_channels_equal() {
    let pair = load();
    check_row(&pair, "R13-rg", |rng| {
        let v = rng.next_u8();
        CbRgb255::new(v, v, rng.next_u8())
    });
    check_row(&pair, "R13-gb", |rng| {
        let v = rng.next_u8();
        CbRgb255::new(rng.next_u8(), v, v)
    });
    check_row(&pair, "R13-rb", |rng| {
        let v = rng.next_u8();
        CbRgb255::new(v, rng.next_u8(), v)
    });
}

/// R14 — the three coordinate planes of the cube (one channel pinned to 0).
#[test]
fn cfg_r14_one_channel_zero() {
    let pair = load();
    check_row(&pair, "R14-r0", |rng| {
        CbRgb255::new(0, rng.next_u8(), rng.next_u8())
    });
    check_row(&pair, "R14-g0", |rng| {
        CbRgb255::new(rng.next_u8(), 0, rng.next_u8())
    });
    check_row(&pair, "R14-b0", |rng| {
        CbRgb255::new(rng.next_u8(), rng.next_u8(), 0)
    });
}

/// R15 — the three far faces of the cube (one channel pinned to 255).
#[test]
fn cfg_r15_one_channel_max() {
    let pair = load();
    check_row(&pair, "R15-r255", |rng| {
        CbRgb255::new(255, rng.next_u8(), rng.next_u8())
    });
    check_row(&pair, "R15-g255", |rng| {
        CbRgb255::new(rng.next_u8(), 255, rng.next_u8())
    });
    check_row(&pair, "R15-b255", |rng| {
        CbRgb255::new(rng.next_u8(), rng.next_u8(), 255)
    });
}

/// R16 — all 512 combinations of the extreme / boundary byte set, which includes
/// all 8 corners of the RGB cube and both sides of the 10/11 threshold.
#[test]
fn cfg_r16_corners_of_the_cube() {
    let pair = load();
    let mut n = 0;
    for &r in &EXTREMES {
        for &g in &EXTREMES {
            for &b in &EXTREMES {
                pair.agree(CbRgb255::new(r, g, b));
                n += 1;
            }
        }
    }
    assert_eq!(n, EXTREMES.len().pow(3));
    for &(r, g, b) in &[
        (0u8, 0u8, 0u8),
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 0),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ] {
        pair.agree(CbRgb255::new(r, g, b));
    }
}

/// R17 — junk in the unused bytes 3..7 of the argument eightbyte must not change
/// the result on either side (see `ERRORS.md` E6).
#[test]
fn cfg_r17_argument_padding_is_irrelevant() {
    let pair = load();
    let mut rng = Rng::new(SEED ^ 0x17);
    for _ in 0..ROW_SAMPLES {
        let input = CbRgb255::new(rng.next_u8(), rng.next_u8(), rng.next_u8());
        let junk = [
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
        ];
        let baseline = pair.agree(input);
        let c = pair.c.call_with_padding(input, junk);
        let rs = pair.rust.call_with_padding(input, junk);
        assert_eq!(c, rs, "R17: C/Rust diverge for {input:?} with junk {junk:?}");
        assert_eq!(c, baseline, "R17: C itself changed when junk was added");
    }
}

/// R18 — statelessness: the same inputs, revisited in a shuffled, interleaved
/// order, must give the same answers as the first pass.
#[test]
fn cfg_r18_stateless_and_order_independent() {
    let pair = load();
    let mut rng = Rng::new(SEED ^ 0x18);
    let inputs: Vec<CbRgb255> = (0..512)
        .map(|_| CbRgb255::new(rng.next_u8(), rng.next_u8(), rng.next_u8()))
        .collect();

    let first: Vec<CbRgb255> = inputs.iter().map(|&i| pair.agree(i)).collect();

    for pass in 0..4 {
        for k in 0..inputs.len() {
            let idx = (k * 37 + pass * 11) % inputs.len();
            let input = inputs[idx];
            let (c, rs) = if (k + pass) % 2 == 0 {
                (pair.c.call(input), pair.rust.call(input))
            } else {
                let rs = pair.rust.call(input);
                (pair.c.call(input), rs)
            };
            assert_eq!(c, rs, "R18: diverge for {input:?} on pass {pass}");
            assert_eq!(c, first[idx], "R18: C is not stateless for {input:?}");
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
//
// The C library has no error-return surface at all (no `return -1`, no NULL, no
// enum, no assert, no pointer parameters, no length parameters — see ERRORS.md).
// What stands in for it is the set of implementation-defined / UB conversions;
// these tests pin each one to the C's exact byte-level outcome rather than to
// "both failed somehow".
// ===========================================================================

/// E1 — out-of-range-high float -> `unsigned char` must WRAP mod 256, not
/// saturate. `(255,255,0)` drives the red row to ~1.1274, gamma-encodes to
/// ~1.0546, and `*255+0.5` lands near 269 -> low byte 13.
#[test]
fn err_e1_denorm_overflow_wraps_mod_256() {
    let pair = load();
    let mut wrapped = 0usize;

    for g in 128..=255u8 {
        for r in (128..=255u8).step_by(8) {
            let input = CbRgb255::new(r, g, 0);
            let out = pair.agree(input);
            // A saturating implementation would return 255 here; the C wraps to a
            // small value instead.
            if out.r < 128 && r >= 200 && g >= 200 {
                wrapped += 1;
            }
        }
    }
    assert!(
        wrapped > 0,
        "E1: expected to observe the mod-256 wrap; saw none (did the C saturate?)"
    );

    let out = pair.agree(CbRgb255::new(255, 255, 0));
    assert_ne!(
        out.r, 255,
        "E1: red saturated at 255, but the C wraps (cvttss2si + mov %al)"
    );
}

/// E2 — out-of-range-low (negative) float -> `unsigned char` must WRAP, not
/// clamp to 0. `(0,0,255)`: red row ~-0.1274 -> linear branch -> ~-1.646 ->
/// `*255+0.5` ~ -419.2 -> truncate to -419 -> low byte 0x5D = 93.
#[test]
fn err_e2_denorm_negative_wraps_mod_256() {
    let pair = load();

    let out = pair.agree(CbRgb255::new(0, 0, 255));
    assert_ne!(
        out.r, 0,
        "E2: red clamped to 0, but the C wraps a negative conversion"
    );

    // Independently model the exact C arithmetic for this input.
    let expected_r = {
        let b_norm = 255.0f32 / 255.0f32; // cbNorm
        let b_lin = {
            let c = b_norm as f64;
            ((c + 0.055) / 1.055).powf(2.4) as f32 // cbRemoveGammaRGB, pow branch
        };
        let red = 0.0f32 + 0.127_398_863_108_80f32 * 0.0f32 - 0.127_398_863_410_72f32 * b_lin;
        let gamma = (red as f64 * 12.92) as f32; // <= threshold -> linear branch
        ((gamma * 255.0f32 + 0.5f32) as i32) as u8 // truncate then take low byte
    };
    assert_eq!(
        out.r, expected_r,
        "E2: the wrapped negative conversion does not match the modelled cvttss2si"
    );

    for b in 200..=255u8 {
        for g in (0..=32u8).step_by(4) {
            pair.agree(CbRgb255::new(0, g, b));
            pair.agree(CbRgb255::new(g, g, b));
        }
    }
}

/// E3 — the `cvttss2si` "integer indefinite" case (NaN / beyond i32) is
/// UNREACHABLE from the public API because the inputs are bounded bytes. This
/// test agrees on the extremes that come closest to it; R0's exhaustive pass is
/// what proves the branch is never taken asymmetrically.
#[test]
fn err_e3_indefinite_unreachable_but_modelled() {
    let pair = load();
    for &(r, g, b) in &[
        (255u8, 255u8, 0u8),
        (0, 0, 255),
        (255, 0, 0),
        (0, 255, 255),
        (0, 0, 0),
        (255, 255, 255),
    ] {
        pair.agree(CbRgb255::new(r, g, b));
    }
}

/// E4 / E5 — `pow` never receives a negative base in either gamma function, so
/// there is no domain error to reproduce. Both guards are exercised at the
/// values that would otherwise go negative.
#[test]
fn err_e4_pow_never_sees_negative_base() {
    let pair = load();
    // cbRemoveGammaRGB: input is byte/255 in [0,1]; the <= 0.04045 side is linear.
    for v in 0..=GAMMA_LINEAR_MAX {
        pair.agree(CbRgb255::new(v, v, v));
        pair.agree(CbRgb255::new(v, 0, 255));
        pair.agree(CbRgb255::new(0, v, 255));
    }
    // cbApplyGammaRGB: the most negative post-matrix red is at (0,0,255). If pow
    // had been called with that negative base it would return NaN, which
    // cvttss2si turns into 0. A non-zero result therefore proves the linear
    // branch was taken.
    let out = pair.agree(CbRgb255::new(0, 0, 255));
    assert_ne!(
        out.r, 0,
        "E4: red is 0, which is what a NaN would truncate to; pow must not have been called"
    );
}

/// E6 — the only "malformed argument" this ABI can express: garbage in the
/// unused bytes of the argument eightbyte. Both sides must ignore it identically.
#[test]
fn err_e6_argument_register_padding_ignored() {
    let pair = load();
    let patterns: [[u8; 5]; 6] = [
        [0x00, 0x00, 0x00, 0x00, 0x00],
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        [0xAA, 0x55, 0xAA, 0x55, 0xAA],
        [0x80, 0x00, 0x00, 0x00, 0x00],
        [0x01, 0x02, 0x03, 0x04, 0x05],
        [0xDE, 0xAD, 0xBE, 0xEF, 0xC0],
    ];
    let inputs = [
        CbRgb255::new(0, 0, 0),
        CbRgb255::new(255, 255, 255),
        CbRgb255::new(255, 255, 0),
        CbRgb255::new(0, 0, 255),
        CbRgb255::new(11, 10, 12),
        CbRgb255::new(128, 127, 129),
    ];
    for &input in &inputs {
        let baseline = pair.agree(input);
        for &junk in &patterns {
            let c = pair.c.call_with_padding(input, junk);
            let rs = pair.rust.call_with_padding(input, junk);
            assert_eq!(c, rs, "E6: diverge for {input:?} junk {junk:?}");
            assert_eq!(c, baseline, "E6: junk changed the C's own answer");
        }
    }
}

/// E7 — only bytes 0..2 of the returned eightbyte are defined by the C
/// (`cbDenorm` ORs three bytes into `rax`). This pins the layout contract the
/// differential comparison relies on and compares the three bytes explicitly.
#[test]
fn err_e7_only_three_bytes_are_defined() {
    let pair = load();
    assert_eq!(
        std::mem::size_of::<CbRgb255>(),
        3,
        "E7: cb_rgb_255 must be exactly 3 bytes"
    );
    assert_eq!(
        std::mem::align_of::<CbRgb255>(),
        1,
        "E7: cb_rgb_255 must have alignment 1"
    );
    let mut rng = Rng::new(SEED ^ 0xE7);
    for _ in 0..1024 {
        let input = CbRgb255::new(rng.next_u8(), rng.next_u8(), rng.next_u8());
        let c = pair.c.call(input);
        let rs = pair.rust.call(input);
        let cb = [c.r, c.g, c.b];
        let rb = [rs.r, rs.g, rs.b];
        assert_eq!(cb, rb, "E7: byte-level divergence for {input:?}");
    }
}

// ===========================================================================
// Phase D — symbol parity, also enforced from inside the test suite.
// ===========================================================================

/// Both `.so`s must resolve `tritanopia`. The full `nm -D` diff lives in
/// `SYMBOLS.md`; this keeps the invariant enforced by `cargo test`.
#[test]
fn symbols_both_export_tritanopia() {
    let pair: Pair = load();
    assert!(pair.c.path.is_file(), "C .so missing: {:?}", pair.c.path);
    assert!(
        pair.rust.path.is_file(),
        "Rust .so missing: {:?}",
        pair.rust.path
    );
    // Reaching here means dlsym("tritanopia") succeeded on both.
    pair.agree(CbRgb255::new(1, 2, 3));
}
