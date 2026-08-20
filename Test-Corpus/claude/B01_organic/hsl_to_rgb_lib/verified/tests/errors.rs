//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `hsl_to_rgb` returns `void` and contains no `assert`, no null check, no range
//! check and no error code (see the grep transcript in `ERRORS.md`), so its
//! "rejections" are the input classes for which it bypasses or short-circuits
//! the conversion. Each test constructs that exact condition and asserts that
//! the C and Rust libraries reject it in the *same* way — same branch taken,
//! same bit patterns written, same fault behaviour for the UB rows.

mod common;

use common::*;

/// `(m, m, m)`: the shape produced by the final `else` of the C chain.
fn assert_final_else(ctx: &str, src: [u32; 3]) {
    let h = harness();
    let c = call(h.c, src);
    assert_eq!(
        c.rgb[0], c.rgb[1],
        "{ctx}: expected the final else (r == g == b == m) for {src:#x?}, got {:#x?}",
        c.rgb
    );
    assert_eq!(c.rgb[1], c.rgb[2], "{ctx}: expected r == g == b for {src:#x?}");
    assert_same(ctx, src);
}

/// The third-branch shape: `dest[0] == m` and (for s != 0, l != 0.5-degenerate
/// inputs) `dest[1] != dest[0]`, i.e. *not* the final else.
fn assert_third_branch(ctx: &str, src: [u32; 3]) {
    let h = harness();
    let c = call(h.c, src);
    assert_ne!(
        c.rgb[0], c.rgb[1],
        "{ctx}: expected the third branch (dest[1] = c + m != m) for {src:#x?}"
    );
    assert_same(ctx, src);
}

// ---------------------------------------------------------------------------
// Row 1 — s == +0.0 takes the achromatic early-out
// ---------------------------------------------------------------------------

#[test]
fn test_row_01_s_positive_zero_early_out() {
    let h = harness();
    let mut rng = Rng::new(0xE001);
    let mut n = 0;
    for l in interesting_floats() {
        for hv in interesting_floats() {
            let src = [hv, 0x0000_0000, l];
            let c = call(h.c, src);
            assert_eq!(
                c.rgb,
                [l, l, l],
                "s = +0.0 must copy l verbatim into all three components"
            );
            assert_same("row 1: s = +0.0", src);
            n += 1;
        }
    }
    for _ in 0..3000 {
        let src = [rng.raw(), 0x0000_0000, rng.raw()];
        let c = call(h.c, src);
        assert_eq!(c.rgb, [src[2], src[2], src[2]]);
        assert_same("row 1: s = +0.0 (random)", src);
        n += 1;
    }
    assert!(n > 3000);
}

// ---------------------------------------------------------------------------
// Row 2 — s == -0.0 also takes the early-out (`-0.0f == 0` is true in C)
// ---------------------------------------------------------------------------

#[test]
fn test_row_02_s_negative_zero_early_out() {
    let h = harness();
    let mut rng = Rng::new(0xE002);
    for l in interesting_floats() {
        for hv in interesting_floats() {
            let src = [hv, 0x8000_0000, l];
            let c = call(h.c, src);
            assert_eq!(
                c.rgb,
                [l, l, l],
                "s = -0.0 must take the same early-out as s = +0.0"
            );
            assert_same("row 2: s = -0.0", src);
        }
    }
    for _ in 0..3000 {
        let src = [rng.raw(), 0x8000_0000, rng.raw()];
        assert_eq!(call(h.c, src).rgb, [src[2], src[2], src[2]]);
        assert_same("row 2: s = -0.0 (random)", src);
    }
    // +0.0 and -0.0 must be indistinguishable in the output.
    for _ in 0..2000 {
        let (hv, l) = (rng.raw(), rng.raw());
        let p = call(h.c, [hv, 0x0000_0000, l]);
        let n = call(h.c, [hv, 0x8000_0000, l]);
        assert_eq!(p.rgb, n.rgb, "C: +0.0 and -0.0 saturation must agree");
        for (label, f) in &h.rust {
            assert_eq!(
                call(*f, [hv, 0x0000_0000, l]).rgb,
                call(*f, [hv, 0x8000_0000, l]).rgb,
                "{label}: +0.0 and -0.0 saturation must agree"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — s = NaN skips the early-out
// ---------------------------------------------------------------------------

#[test]
fn test_row_03_s_nan_skips_early_out() {
    let h = harness();
    let mut rng = Rng::new(0xE003);
    for _ in 0..4000 {
        let s = rng.nan();
        let src = [
            rng.range(0.0, 360.0).to_bits(),
            s,
            rng.range(0.01, 0.99).to_bits(),
        ];
        let c = call(h.c, src);
        assert_ne!(
            c.rgb,
            [src[2], src[2], src[2]],
            "s = NaN must NOT take the s == 0 early-out"
        );
        assert_same("row 3: s = NaN", src);
    }
    for s in [0x7FC0_0000u32, 0xFFC0_0000, 0x7F80_0001, 0xFF80_0001, 0x7FFF_FFFF] {
        for hv in interesting_floats() {
            assert_same("row 3: s = NaN (interesting h)", [hv, s, 0.25f32.to_bits()]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — h = NaN falls into the final else
// ---------------------------------------------------------------------------

#[test]
fn test_row_04_h_nan_falls_into_final_else() {
    let mut rng = Rng::new(0xE004);
    for _ in 0..4000 {
        let src = [
            rng.nan(),
            rng.range(0.01, 1.0).to_bits(),
            rng.range(0.01, 0.99).to_bits(),
        ];
        assert_final_else("row 4: h = NaN", src);
    }
    // Quiet, signalling, both signs, extreme payloads.
    for hv in [
        0x7FC0_0000u32, 0xFFC0_0000, 0x7FC0_0001, 0xFFC0_0001, 0x7F80_0001, 0xFF80_0001,
        0x7FBF_FFFF, 0xFFBF_FFFF, 0x7FFF_FFFF, 0xFFFF_FFFF,
    ] {
        assert_final_else(
            "row 4: h = NaN (explicit payloads)",
            [hv, 1.0f32.to_bits(), 0.5f32.to_bits()],
        );
        for s in interesting_floats() {
            assert_same("row 4: h = NaN x s", [hv, s, 0.5f32.to_bits()]);
        }
        for l in interesting_floats() {
            assert_same("row 4: h = NaN x l", [hv, 1.0f32.to_bits(), l]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — h in [120, 180) matches no guard (the `h < 120` typo)
// ---------------------------------------------------------------------------

#[test]
fn test_row_05_hue_120_to_180_matches_no_guard() {
    let mut rng = Rng::new(0xE005);
    for _ in 0..4000 {
        let src = [
            rng.range(120.0, 180.0).to_bits(),
            rng.range(0.01, 1.0).to_bits(),
            rng.range(0.01, 0.99).to_bits(),
        ];
        assert_final_else("row 5: h in [120,180)", src);
    }
    // The exact endpoints of the dead interval.
    for hv in [
        120.0f32,
        next_after(120.0, f32::INFINITY),
        150.0,
        next_after(180.0, f32::NEG_INFINITY),
    ] {
        assert_final_else(
            "row 5: dead-interval endpoints",
            [hv.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()],
        );
    }
    // 180 itself must NOT be in the dead interval (it matches guard 4).
    assert_third_branch_or_sector(
        "row 5: h = 180 is alive",
        [180.0f32.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()],
    );
}

/// Assert the input does *not* end up in the final else.
fn assert_third_branch_or_sector(ctx: &str, src: [u32; 3]) {
    let h = harness();
    let c = call(h.c, src);
    assert!(
        !(c.rgb[0] == c.rgb[1] && c.rgb[1] == c.rgb[2]),
        "{ctx}: expected a real sector, not the final else, for {src:#x?} (got {:#x?})",
        c.rgb
    );
    assert_same(ctx, src);
}

// ---------------------------------------------------------------------------
// Row 6 — h >= 360 is not wrapped, it falls into the final else
// ---------------------------------------------------------------------------

#[test]
fn test_row_06_hue_at_or_above_360_final_else() {
    let mut rng = Rng::new(0xE006);
    for hv in [360.0f32, next_after(360.0, f32::INFINITY), 361.0, 720.0, 1e9, f32::MAX] {
        assert_final_else(
            "row 6: h >= 360",
            [hv.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()],
        );
    }
    for _ in 0..4000 {
        let src = [
            rng.range(360.0, f32::MAX).to_bits(),
            rng.range(0.01, 1.0).to_bits(),
            rng.range(0.01, 0.99).to_bits(),
        ];
        assert_final_else("row 6: h >= 360 (random)", src);
    }
    // 360 - 1 ULP must still be a real sector.
    assert_third_branch_or_sector(
        "row 6: h just below 360",
        [
            next_after(360.0, f32::NEG_INFINITY).to_bits(),
            1.0f32.to_bits(),
            0.5f32.to_bits(),
        ],
    );
}

// ---------------------------------------------------------------------------
// Row 7 — h < 0 silently aliases the third branch
// ---------------------------------------------------------------------------

#[test]
fn test_row_07_negative_hue_aliases_third_branch() {
    let mut rng = Rng::new(0xE007);
    for hv in [
        next_after(0.0, f32::NEG_INFINITY),
        -f32::MIN_POSITIVE,
        -1.0f32,
        -30.0,
        -119.0,
        -400.0,
        -1e9,
        f32::MIN,
    ] {
        assert_third_branch("row 7: h < 0", [hv.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()]);
    }
    for _ in 0..4000 {
        let src = [
            (-rng.range(f32::MIN_POSITIVE, 1e9)).to_bits(),
            rng.range(0.05, 1.0).to_bits(),
            rng.range(0.05, 0.95).to_bits(),
        ];
        assert_third_branch("row 7: h < 0 (random)", src);
    }
    // `-0.0 >= 0.0` is true, so -0.0 is NOT negative for the guard: it must take
    // the FIRST sector, not the third branch.
    let h = harness();
    let neg_zero = [(-0.0f32).to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()];
    let pos_zero = [0.0f32.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()];
    assert_eq!(
        call(h.c, neg_zero).rgb,
        call(h.c, pos_zero).rgb,
        "C: h = -0.0 must behave like h = +0.0 (sector 1)"
    );
    assert_same("row 7: h = -0.0", neg_zero);
    assert_same("row 7: h = +0.0", pos_zero);
}

// ---------------------------------------------------------------------------
// Row 8 — h = -inf drives fmodf's domain-error path AND uses the result
// ---------------------------------------------------------------------------

#[test]
fn test_row_08_hue_negative_infinity_fmodf_domain_error() {
    let h = harness();
    let src = [f32::NEG_INFINITY.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()];
    let c = call(h.c, src);
    // Third branch: dest[0] = m = 0, dest[1] = c + m = 1, dest[2] = x + m = NaN.
    assert_eq!(c.rgb[0], 0.0f32.to_bits(), "m must be 0 for s=1, l=0.5");
    assert_eq!(c.rgb[1], 1.0f32.to_bits(), "c + m must be 1 for s=1, l=0.5");
    assert!(
        f32::from_bits(c.rgb[2]).is_nan(),
        "x = (1 - |fmodf(-inf, 2) - 1|) * c must be NaN, got {:#010x}",
        c.rgb[2]
    );
    assert_same("row 8: h = -inf", src);

    // Sweep every s / l class so both the NaN payload and its sign are pinned.
    for s in interesting_floats() {
        for l in interesting_floats() {
            assert_same("row 8: h = -inf x (s, l)", [src[0], s, l]);
        }
    }
    let mut rng = Rng::new(0xE008);
    for _ in 0..3000 {
        assert_same("row 8: h = -inf (random s, l)", [src[0], rng.raw(), rng.raw()]);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — h = +inf falls into the final else
// ---------------------------------------------------------------------------

#[test]
fn test_row_09_hue_positive_infinity_final_else() {
    let hv = f32::INFINITY.to_bits();
    assert_final_else(
        "row 9: h = +inf",
        [hv, 1.0f32.to_bits(), 0.5f32.to_bits()],
    );
    for s in interesting_floats() {
        for l in interesting_floats() {
            assert_same("row 9: h = +inf x (s, l)", [hv, s, l]);
        }
    }
    let mut rng = Rng::new(0xE009);
    for _ in 0..3000 {
        assert_same("row 9: h = +inf (random s, l)", [hv, rng.raw(), rng.raw()]);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — l = NaN
// ---------------------------------------------------------------------------

#[test]
fn test_row_10_lightness_nan() {
    let h = harness();
    let mut rng = Rng::new(0xE00A);
    for _ in 0..4000 {
        let src = [
            rng.range(0.0, 360.0).to_bits(),
            rng.range(0.01, 1.0).to_bits(),
            rng.nan(),
        ];
        let c = call(h.c, src);
        assert!(
            f32::from_bits(c.rgb[0]).is_nan()
                && f32::from_bits(c.rgb[1]).is_nan()
                && f32::from_bits(c.rgb[2]).is_nan(),
            "l = NaN with s != 0 must make every component NaN, got {:#x?}",
            c.rgb
        );
        assert_same("row 10: l = NaN", src);
    }
    for l in [0x7FC0_0000u32, 0xFFC0_0000, 0x7F80_0001, 0xFF80_0001, 0x7FFF_FFFF] {
        for hv in interesting_floats() {
            assert_same("row 10: l = NaN (interesting h)", [hv, 1.0f32.to_bits(), l]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — l = +-inf
// ---------------------------------------------------------------------------

#[test]
fn test_row_11_lightness_infinite() {
    let mut rng = Rng::new(0xE00B);
    for l in [f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()] {
        for hv in interesting_floats() {
            for s in [1.0f32.to_bits(), 0.5f32.to_bits(), 0u32, 0x8000_0000] {
                assert_same("row 11: l = +-inf", [hv, s, l]);
            }
        }
        for _ in 0..3000 {
            assert_same(
                "row 11: l = +-inf (random)",
                [rng.raw(), rng.range(0.01, 1.0).to_bits(), l],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — finite l outside [0, 1] is not clamped
// ---------------------------------------------------------------------------

#[test]
fn test_row_12_lightness_out_of_range_not_clamped() {
    let h = harness();
    let mut rng = Rng::new(0xE00C);
    // l = 2 with s = 1, h = 30: c = (1 - |3|) * 1 = -2, m = 2 - 0.5*(-2) = 3,
    // so dest[0] = c + m = 1 but dest[1]/dest[2] leave [0, 1]. Nothing rejects.
    let src = [30.0f32.to_bits(), 1.0f32.to_bits(), 2.0f32.to_bits()];
    let c = call(h.c, src);
    assert!(
        f32::from_bits(c.rgb[2]) > 1.0,
        "an out-of-range l must produce out-of-gamut output, not a rejection: {:#x?}",
        c.rgb
    );
    assert_same("row 12: l = 2", src);

    for _ in 0..4000 {
        let s = rng.range(0.01, 1.0).to_bits();
        let hv = rng.range(0.0, 360.0).to_bits();
        assert_same("row 12: l > 1", [hv, s, rng.range(1.0, 1e6).to_bits()]);
        assert_same("row 12: l < 0", [hv, s, (-rng.range(0.0, 1e6)).to_bits()]);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — finite non-zero s outside [0, 1] is not clamped
// ---------------------------------------------------------------------------

#[test]
fn test_row_13_saturation_out_of_range_not_clamped() {
    let h = harness();
    let mut rng = Rng::new(0xE00D);
    let src = [30.0f32.to_bits(), 7.0f32.to_bits(), 0.5f32.to_bits()];
    let c = call(h.c, src);
    assert!(
        f32::from_bits(c.rgb[0]) > 1.0,
        "s = 7 must produce out-of-gamut output, not a rejection: {:#x?}",
        c.rgb
    );
    assert_same("row 13: s = 7", src);

    for _ in 0..4000 {
        let hv = rng.range(0.0, 360.0).to_bits();
        let l = rng.range(0.01, 0.99).to_bits();
        assert_same("row 13: s > 1", [hv, rng.range(1.0, 1e6).to_bits(), l]);
        assert_same(
            "row 13: s < 0",
            [hv, (-rng.range(f32::MIN_POSITIVE, 1e6)).to_bits(), l],
        );
    }
}

// ---------------------------------------------------------------------------
// Row 14 — s = +-inf (including the 0 * inf NaN)
// ---------------------------------------------------------------------------

#[test]
fn test_row_14_saturation_infinite() {
    let h = harness();
    // l = 0 makes `1 - |2l - 1|` exactly +0.0, so c = 0 * inf = NaN.
    let src = [30.0f32.to_bits(), f32::INFINITY.to_bits(), 0.0f32.to_bits()];
    let c = call(h.c, src);
    assert!(
        f32::from_bits(c.rgb[0]).is_nan(),
        "s = inf with l = 0 must give c = 0 * inf = NaN, got {:#x?}",
        c.rgb
    );
    assert_same("row 14: s = inf, l = 0", src);

    let mut rng = Rng::new(0xE00E);
    for s in [f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()] {
        for hv in interesting_floats() {
            for l in interesting_floats() {
                assert_same("row 14: s = +-inf", [hv, s, l]);
            }
        }
        for _ in 0..3000 {
            assert_same(
                "row 14: s = +-inf (random)",
                [rng.raw(), s, rng.range(0.0, 1.0).to_bits()],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — the exact guard boundaries select the documented branch
// ---------------------------------------------------------------------------

#[test]
fn test_row_15_exact_boundary_branch_selection() {
    let h = harness();
    // With s = 1, l = 0.5: c = 1, m = 0. At every multiple of 60 the value of
    // `x` is 0 (even multiples) or 1 (odd multiples), which makes the branch
    // identifiable from the output triple.
    let s = 1.0f32.to_bits();
    let l = 0.5f32.to_bits();
    let one = 1.0f32.to_bits();
    let zero = 0.0f32.to_bits();
    let expected: [(f32, [u32; 3]); 7] = [
        (0.0, [one, zero, zero]),   // sector 1: (c+m, x+m, m), x = 0
        (60.0, [one, one, zero]),   // sector 2: (x+m, c+m, m), x = 1
        (120.0, [zero, zero, zero]),// final else: (m, m, m)  <-- the C bug
        (180.0, [zero, one, one]),  // sector 4: (m, x+m, c+m), x = 1
        (240.0, [zero, zero, one]), // sector 5: (x+m, m, c+m), x = 0
        (300.0, [one, zero, one]),  // sector 6: (c+m, m, x+m), x = 1
        (360.0, [zero, zero, zero]),// final else
    ];
    for (hv, want) in expected {
        let src = [hv.to_bits(), s, l];
        let c = call(h.c, src);
        assert_eq!(
            c.rgb, want,
            "h = {hv}: unexpected branch; got {:#x?}, want {want:#x?}",
            c.rgb
        );
        assert_same("row 15: exact boundary", src);
    }
    let mut rng = Rng::new(0xE00F);
    for b in BOUNDARIES {
        for _ in 0..500 {
            assert_same(
                "row 15: exact boundary (random s, l)",
                [b.to_bits(), rng.raw(), rng.raw()],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — one ULP past each boundary
// ---------------------------------------------------------------------------

#[test]
fn test_row_16_one_step_past_each_boundary() {
    let h = harness();
    let s = 1.0f32.to_bits();
    let l = 0.5f32.to_bits();
    // Just *below* each boundary the previous sector must still be selected.
    let below_is_else = [
        (60.0f32, false),  // -> sector 1
        (120.0, false),    // -> sector 2
        (180.0, true),     // -> final else (the dead 120..180 interval)
        (240.0, false),    // -> sector 4
        (300.0, false),    // -> sector 5
        (360.0, false),    // -> sector 6
    ];
    for (b, is_else) in below_is_else {
        let hv = next_after(b, f32::NEG_INFINITY);
        let src = [hv.to_bits(), s, l];
        let c = call(h.c, src);
        let all_eq = c.rgb[0] == c.rgb[1] && c.rgb[1] == c.rgb[2];
        assert_eq!(
            all_eq, is_else,
            "h = {hv} (just below {b}): expected final-else = {is_else}, got {:#x?}",
            c.rgb
        );
        assert_same("row 16: one ULP below a boundary", src);
    }
    // One ULP below zero: strictly negative, so the third branch.
    let hv = next_after(0.0, f32::NEG_INFINITY);
    assert_third_branch("row 16: h = -1e-45", [hv.to_bits(), s, l]);

    let mut rng = Rng::new(0xE010);
    for b in BOUNDARIES {
        for hv in [next_after(b, f32::NEG_INFINITY), next_after(b, f32::INFINITY)] {
            for _ in 0..400 {
                assert_same("row 16: +-1 ULP (random s, l)", [hv.to_bits(), rng.raw(), rng.raw()]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — subnormal inputs (no FTZ / DAZ)
// ---------------------------------------------------------------------------

#[test]
fn test_row_17_subnormal_inputs() {
    let h = harness();
    let subs = [
        1u32,
        0x8000_0001,
        0x0000_0002,
        0x0040_0000,
        0x007F_FFFF,
        0x807F_FFFF,
        f32::MIN_POSITIVE.to_bits(),
    ];
    // A subnormal s is non-zero => the early-out must not trigger. With a finite
    // l that is unobservable, because `c` stays subnormal and both `l - 0.5c`
    // and `c + m` round back to `l`; `l = +inf` exposes the difference
    // (c = -inf, m = +inf, c + m = NaN).
    for &s in &subs {
        if f32::from_bits(s) == 0.0 {
            continue;
        }
        let src = [30.0f32.to_bits(), s, f32::INFINITY.to_bits()];
        let c = call(h.c, src);
        assert_ne!(
            c.rgb,
            [src[2], src[2], src[2]],
            "subnormal s = {s:#010x} must not take the s == 0 early-out"
        );
        assert_same("row 17: subnormal s", src);
        assert_same("row 17: subnormal s, finite l", [30.0f32.to_bits(), s, 0.25f32.to_bits()]);
    }
    for &a in &subs {
        for &b in &subs {
            assert_same("row 17: subnormal h/s", [a, b, 0.5f32.to_bits()]);
            assert_same("row 17: subnormal h/l", [a, 1.0f32.to_bits(), b]);
            assert_same("row 17: subnormal s/l", [30.0f32.to_bits(), a, b]);
            assert_same("row 17: subnormal all", [a, b, a]);
        }
    }
    let mut rng = Rng::new(0xE011);
    for _ in 0..4000 {
        let a = 0x0000_0001 + (rng.next_u32() & 0x007F_FFFE);
        let b = 0x0000_0001 + (rng.next_u32() & 0x007F_FFFE);
        let c = 0x8000_0001 + (rng.next_u32() & 0x007F_FFFE);
        assert_same("row 17: random subnormals", [a, b, c]);
        assert_same("row 17: random subnormal s", [rng.raw(), b, rng.raw()]);
        assert_same("row 17: random subnormal l", [rng.raw(), 1.0f32.to_bits(), c]);
    }
}

// ---------------------------------------------------------------------------
// Row 18 — dest == src (in place) is well defined, not rejected
// ---------------------------------------------------------------------------

#[test]
fn test_row_18_in_place_is_well_defined() {
    let h = harness();
    let mut rng = Rng::new(0xE012);
    for _ in 0..5000 {
        let src = [rng.raw(), rng.raw(), rng.raw()];
        let disjoint = call(h.c, src);
        let inplace = call_overlapping(h.c, src, 0);
        assert_eq!(
            [inplace[4], inplace[5], inplace[6]],
            disjoint.rgb,
            "C: in-place must equal the disjoint result for {src:#x?}"
        );
        for (label, f) in &h.rust {
            let r = call_overlapping(*f, src, 0);
            assert_eq!(r, inplace, "{label}: in-place mismatch for {src:#x?}");
        }
    }
    // The s == 0 early-out in place too.
    for s in [0u32, 0x8000_0000] {
        for _ in 0..1000 {
            let src = [rng.raw(), s, rng.raw()];
            let c = call_overlapping(h.c, src, 0);
            assert_eq!(
                [c[4], c[5], c[6]],
                [src[2], src[2], src[2]],
                "in-place early-out must broadcast l"
            );
            for (label, f) in &h.rust {
                assert_eq!(
                    call_overlapping(*f, src, 0),
                    c,
                    "{label}: in-place early-out mismatch"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — partially overlapping dest / src
// ---------------------------------------------------------------------------

#[test]
fn test_row_19_partial_overlap() {
    let h = harness();
    let mut rng = Rng::new(0xE013);
    for off in [-3isize, -2, -1, 1, 2, 3] {
        for _ in 0..2000 {
            let src = [rng.raw(), rng.raw(), rng.raw()];
            let c = call_overlapping(h.c, src, off);
            for (label, f) in &h.rust {
                assert_eq!(
                    call_overlapping(*f, src, off),
                    c,
                    "{label}: dest = src{off:+} mismatch for {src:#x?}"
                );
            }
            // All three loads precede all three stores, so the result must equal
            // the disjoint result regardless of the overlap.
            let disjoint = call(h.c, src);
            let start = (4 + off) as usize;
            assert_eq!(
                [c[start], c[start + 1], c[start + 2]],
                disjoint.rgb,
                "C: overlap {off:+} changed the result for {src:#x?}"
            );
        }
        for s in [0u32, 0x8000_0000] {
            for _ in 0..500 {
                let src = [rng.raw(), s, rng.raw()];
                let c = call_overlapping(h.c, src, off);
                for (label, f) in &h.rust {
                    assert_eq!(
                        call_overlapping(*f, src, off),
                        c,
                        "{label}: early-out overlap {off:+} mismatch"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — null pointers: both libraries must fault, identically
// ---------------------------------------------------------------------------

/// Child entry point. Does nothing unless `HSL_NULL_MODE` is set, so it is inert
/// during a normal test run.
#[test]
fn nullptr_child_helper() {
    let Ok(mode) = std::env::var("HSL_NULL_MODE") else {
        return;
    };
    let (which, kind) = mode.split_once(':').expect("HSL_NULL_MODE=<which>:<kind>");
    let h = harness();
    let f: HslFn = match which {
        "c" => h.c,
        other => {
            *h.rust
                .iter()
                .find(|(l, _)| *l == other)
                .map(|(_, f)| f)
                .unwrap_or_else(|| panic!("unknown library {other}"))
        }
    };
    let mut dest = [0.0f32; 3];
    let src = [30.0f32, 1.0, 0.5];
    // SAFETY: intentionally passing null to observe the fault; this process is a
    // disposable child spawned by `test_row_20_null_pointers_both_fault`.
    unsafe {
        match kind {
            "dest" => f(std::ptr::null_mut(), src.as_ptr()),
            "src" => f(dest.as_mut_ptr(), std::ptr::null()),
            "both" => f(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("unknown kind {other}"),
        }
    }
    // Reached only if the call did NOT fault.
    std::hint::black_box(&mut dest);
    println!("survived {mode}");
    std::process::exit(0);
}

#[test]
fn test_row_20_null_pointers_both_fault() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let h = harness();
    let libs: Vec<String> = std::iter::once("c".to_string())
        .chain(h.rust.iter().map(|(l, _)| l.to_string()))
        .collect();
    let exe = std::env::current_exe().expect("current_exe");

    for kind in ["dest", "src", "both"] {
        let mut outcomes = Vec::new();
        for lib in &libs {
            let out = Command::new(&exe)
                .args(["nullptr_child_helper", "--exact", "--test-threads=1"])
                .env("HSL_NULL_MODE", format!("{lib}:{kind}"))
                .output()
                .expect("spawn child");
            let signal = out.status.signal();
            let code = out.status.code();
            outcomes.push((lib.clone(), signal, code));
        }
        let reference = outcomes[0].1;
        assert!(
            reference.is_some(),
            "the C library was expected to fault on a null {kind} pointer, \
             but the child exited normally: {outcomes:?}"
        );
        for (lib, signal, code) in &outcomes {
            assert_eq!(
                *signal, reference,
                "null {kind}: {lib} terminated with signal {signal:?} / code {code:?}, \
                 but the C reference terminated with signal {reference:?}. The Rust \
                 translation must not add a null check the C code does not have. \
                 All outcomes: {outcomes:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — the function touches exactly three floats in each direction
// ---------------------------------------------------------------------------

#[test]
fn test_row_21_touches_exactly_three_floats() {
    let h = harness();
    let mut rng = Rng::new(0xE015);
    for _ in 0..20_000 {
        let src = [rng.raw(), rng.raw(), rng.raw()];
        let c = call(h.c, src);
        assert_eq!(
            c.dest_guards,
            [GUARD_LO, GUARD_HI],
            "C wrote outside dest[0..3] for {src:#x?}"
        );
        assert_eq!(
            c.src_after,
            [GUARD_LO, src[0], src[1], src[2], GUARD_HI],
            "C modified the source buffer for {src:#x?}"
        );
        for (label, f) in &h.rust {
            let r = call(*f, src);
            assert_eq!(
                r.dest_guards,
                [GUARD_LO, GUARD_HI],
                "{label} wrote outside dest[0..3] for {src:#x?}"
            );
            assert_eq!(
                r.src_after,
                [GUARD_LO, src[0], src[1], src[2], GUARD_HI],
                "{label} modified the source buffer for {src:#x?}"
            );
            assert_eq!(r.rgb, c.rgb, "{label} output mismatch for {src:#x?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks required by Phase C even though the C API has no
// enums or integer parameters.
// ---------------------------------------------------------------------------

#[test]
fn generic_no_enum_or_integer_parameters_to_fuzz() {
    // `void hsl_to_rgb(float *dest, const float *src)` has no enum, no flag and
    // no length argument, so the only "out of range value across the FFI
    // boundary" that exists is an arbitrary `float` bit pattern. Assert that the
    // exported signature really is the two-pointer one by driving it with the
    // most hostile bit patterns available and requiring agreement.
    let mut rng = Rng::new(0xE016);
    let hostile = [
        0x0000_0000u32,
        0x8000_0000,
        0x7F80_0000,
        0xFF80_0000,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x7F7F_FFFF,
        0xFF7F_FFFF,
        0x0000_0001,
        0x8000_0001,
        0x7FC0_0000,
        0xFFC0_0000,
        0x0080_0000,
        0x8080_0000,
    ];
    for &a in &hostile {
        for &b in &hostile {
            for &c in &hostile {
                assert_same("hostile bit patterns", [a, b, c]);
            }
        }
    }
    for _ in 0..20_000 {
        assert_same("random hostile", [rng.raw(), rng.raw(), rng.raw()]);
    }
}
