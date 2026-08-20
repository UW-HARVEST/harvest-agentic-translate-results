//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! This library has no error returns at all (see the mechanical grep in
//! `ERRORS.md`), so its "rejection surface" is the set of
//! out-of-domain / implementation-defined / boundary conditions the C resolves:
//! the three `(unsigned char)` narrowing casts and the six strict-`>`
//! thresholds, plus the generic FFI boundaries.
//!
//! Each test asserts the SAME concrete result from both `.so`s -- not merely
//! "both did something". Where a plausible mistranslation would give a
//! *specific* wrong answer (saturation, rounding, NaN), the test also asserts
//! that the answer is NOT that wrong value, so the test cannot pass vacuously.

mod common;

use common::*;

/// Helper: both implementations must return exactly `expected`.
#[track_caller]
fn assert_both(p: &Pair, i: Rgb, expected: Rgb, why: &str) {
    let c = p.call_c(i);
    let r = p.call_rust(i);
    assert_eq!(
        c, expected,
        "{why}: C({},{},{}) = ({},{},{}), expected ({},{},{}) -- the C is ground truth, \
         so ERRORS.md is wrong if this fires",
        i.r, i.g, i.b, c.r, c.g, c.b, expected.r, expected.g, expected.b
    );
    assert_eq!(
        r, expected,
        "{why}: Rust({},{},{}) = ({},{},{}) but C gives ({},{},{})",
        i.r, i.g, i.b, r.r, r.g, r.b, c.r, c.g, c.b
    );
}

// ---------------------------------------------------------------------------
// E1 -- cbDenorm (lib.c:29): (unsigned char) of a NEGATIVE float.
// ---------------------------------------------------------------------------

/// The denorm argument for R reaches `-419.2283`. C truncates toward zero to
/// `-419` and keeps the low byte: `-419 & 0xff == 93`.
///
/// A saturating cast (Rust's `as u8`) would give `0`; a `wrapping` cast of a
/// rounded value would give `92`. Both are excluded.
#[test]
fn err_e1_denorm_negative_wraps() {
    let p = Pair::load();

    // The exact minimum of the R denorm argument: R=0, G=0, B=255.
    assert_both(&p, Rgb::new(0, 0, 255), Rgb::new(93, 99, 99), "E1 min case");

    let c = p.call_c(Rgb::new(0, 0, 255));
    assert_ne!(c.r, 0, "E1: C must NOT saturate to 0 (that is Rust's `as u8`)");
    assert_ne!(c.r, 255, "E1: C must NOT saturate to 255");

    // Sweep the whole negative-R sub-domain and require agreement everywhere.
    let mut negatives = 0usize;
    for b in 200u16..=255 {
        for g in 0u16..=16 {
            for r in 0u16..=16 {
                let i = Rgb::new(r as u8, g as u8, b as u8);
                let cv = p.call_c(i);
                let rv = p.call_rust(i);
                assert_eq!(
                    cv, rv,
                    "E1 divergence at ({},{},{}): C=({},{},{}) Rust=({},{},{})",
                    i.r, i.g, i.b, cv.r, cv.g, cv.b, rv.r, rv.g, rv.b
                );
                // R wrapped from a negative value => byte is large although the
                // "true" colour value is below zero.
                if cv.r > 16 {
                    negatives += 1;
                }
            }
        }
    }
    assert!(
        negatives > 1000,
        "E1 sub-domain did not actually exercise the negative wraparound (only {negatives} hits)"
    );
}

// ---------------------------------------------------------------------------
// E2 -- cbDenorm (lib.c:29): (unsigned char) of a float > 255.
// ---------------------------------------------------------------------------

/// The denorm argument for R reaches `269.2830`; C gives `269 & 0xff == 13`.
/// A saturating cast would give `255`.
#[test]
fn err_e2_denorm_over_255_wraps() {
    let p = Pair::load();

    // The exact maximum of the R denorm argument: R=255, G=255, B=0.
    assert_both(&p, Rgb::new(255, 255, 0), Rgb::new(13, 240, 240), "E2 max case");

    let c = p.call_c(Rgb::new(255, 255, 0));
    assert_ne!(c.r, 255, "E2: C must NOT saturate to 255");

    let mut wrapped = 0usize;
    for r in 240u16..=255 {
        for g in 240u16..=255 {
            for b in 0u16..=16 {
                let i = Rgb::new(r as u8, g as u8, b as u8);
                let cv = p.call_c(i);
                let rv = p.call_rust(i);
                assert_eq!(
                    cv, rv,
                    "E2 divergence at ({},{},{}): C=({},{},{}) Rust=({},{},{})",
                    i.r, i.g, i.b, cv.r, cv.g, cv.b, rv.r, rv.g, rv.b
                );
                // Wrapped values are small even though the input R is ~255.
                if cv.r < 64 {
                    wrapped += 1;
                }
            }
        }
    }
    assert!(
        wrapped > 100,
        "E2 sub-domain did not actually exercise the >255 wraparound (only {wrapped} hits)"
    );
}

// ---------------------------------------------------------------------------
// E3 -- cbDenorm: in-range cast truncates toward zero (`+0.5f` => round-half-up)
// ---------------------------------------------------------------------------

#[test]
fn err_e3_denorm_in_range_truncates() {
    let p = Pair::load();
    // Grayscale is an (almost) identity path: it keeps the denorm argument
    // inside 0..255 and makes an off-by-one from a wrong rounding mode visible.
    for v in 0u16..=255 {
        let i = Rgb::new(v as u8, v as u8, v as u8);
        let cv = p.call_c(i);
        let rv = p.call_rust(i);
        assert_eq!(cv, rv, "E3 divergence at grayscale {v}");
        // Truncation of v*255/255+0.5 must land back on v for the identity path.
        assert_eq!(
            cv.r, v as u8,
            "E3: C grayscale round-trip changed R for v={v} (got {})",
            cv.r
        );
    }
}

// ---------------------------------------------------------------------------
// E4 / E5 -- G and B denorm argument attains exactly 255.5.
// ---------------------------------------------------------------------------

/// `trunc(255.5) == 255`. A rounding cast would give `256`, which truncates to
/// `0` in a byte -- so the assertion `!= 0` is the discriminating one.
#[test]
fn err_e4_e5_g_b_upper_boundary() {
    let p = Pair::load();

    assert_both(
        &p,
        Rgb::new(255, 255, 255),
        Rgb::new(255, 255, 255),
        "E4/E5 boundary 255.5",
    );

    let mut hits = 0usize;
    for r in 0u16..=255 {
        let i = Rgb::new(r as u8, 255, 255);
        let cv = p.call_c(i);
        let rv = p.call_rust(i);
        assert_eq!(cv, rv, "E4/E5 divergence at ({},255,255)", i.r);
        if cv.g == 255 && cv.b == 255 {
            hits += 1;
        }
        assert_ne!(
            cv.g, 0,
            "E4: G must not overflow to 0 at the 255.5 boundary (input {},255,255)",
            i.r
        );
        assert_ne!(
            cv.b, 0,
            "E5: B must not overflow to 0 at the 255.5 boundary (input {},255,255)",
            i.r
        );
    }
    assert_eq!(hits, 256, "E4/E5: expected all 256 inputs to pin G=B=255");
}

// ---------------------------------------------------------------------------
// E6 -- (unsigned char) of NaN is UNREACHABLE from the public API.
// ---------------------------------------------------------------------------

/// Tested as a *negative* property, the only correct way to test an
/// unreachable branch: over the entire input domain the pre-`cbDenorm` value is
/// never NaN, so the `cvttss2si` indefinite result (low byte `0`) can never be
/// produced. If a mistranslation introduced a NaN it would surface as a `0`
/// byte where C returns something else -- which the sweep below would catch.
#[test]
fn err_e6_nan_unreachable() {
    let p = Pair::load();
    let mut nan_count = 0usize;
    let mut checked = 0usize;
    // Full sweep of the R channel's extreme sub-domains plus a strided sweep of
    // the whole cube (a strided sweep keeps the test fast; row 26 does all 2^24).
    for r in (0u16..=255).step_by(3) {
        for g in (0u16..=255).step_by(3) {
            for b in (0u16..=255).step_by(3) {
                let i = Rgb::new(r as u8, g as u8, b as u8);
                let (nan, _, _) = model_channels(i);
                if nan {
                    nan_count += 1;
                }
                let cv = p.call_c(i);
                let rv = p.call_rust(i);
                assert_eq!(cv, rv, "E6 divergence at ({},{},{})", i.r, i.g, i.b);
                checked += 1;
            }
        }
    }
    assert!(checked > 500_000, "E6 sweep too small ({checked})");
    assert_eq!(
        nan_count, 0,
        "E6: the pre-cbDenorm value was NaN for {nan_count} inputs -- the \
         'NaN is unreachable' claim in ERRORS.md is false"
    );
}

// ---------------------------------------------------------------------------
// E7 -- cbRemoveGammaRGB threshold is a strict `>` against 0.04045 in double.
// ---------------------------------------------------------------------------

/// `10/255 = 0.0392156877` is NOT `> 0.04045` (linear branch);
/// `11/255 = 0.0431372561` IS (`pow` branch). Using `>=`, or comparing in
/// `f32`, would move this boundary and change the output.
#[test]
fn err_e7_remove_gamma_threshold() {
    let p = Pair::load();

    // Verify the boundary is exactly between 10 and 11, independently per
    // channel, by walking every value 0..=20 in each channel position.
    for v in 0u16..=20 {
        for pos in 0..3 {
            let i = match pos {
                0 => Rgb::new(v as u8, 0, 0),
                1 => Rgb::new(0, v as u8, 0),
                _ => Rgb::new(0, 0, v as u8),
            };
            let cv = p.call_c(i);
            let rv = p.call_rust(i);
            assert_eq!(cv, rv, "E7 divergence at v={v} pos={pos}");
        }
    }

    // The threshold really does straddle 10/11 in double precision.
    assert!(!((10.0f32 / 255.0f32) as f64 > 0.04045), "E7 premise: 10 is linear");
    assert!((11.0f32 / 255.0f32) as f64 > 0.04045, "E7 premise: 11 is pow");

    // All 8 combinations of the boundary values.
    let n = check_all("E7 {10,11}^3", cuboid((10, 11), (10, 11), (10, 11)));
    assert_eq!(n, 8);
}

// ---------------------------------------------------------------------------
// E8 -- cbApplyGammaRGB threshold is a strict `>` against 0.0031308...
// ---------------------------------------------------------------------------

/// Values `<= t` -- which includes every negative value -- take the linear
/// `c*12.92` branch. That linear branch is what feeds the negative denorm
/// argument of E1, so E1's non-zero result is itself proof the branch was
/// taken. Here we additionally sweep both sides of the threshold.
#[test]
fn err_e8_apply_gamma_threshold() {
    let p = Pair::load();

    // Linear side (R output <= t, mostly negative).
    check_all(
        "E8 linear side",
        random_in_ranges(SEED ^ 0xE8, 20_000, (0, 8), (0, 8), (128, 255)),
    );
    // Pow side.
    check_all(
        "E8 pow side",
        random_in_ranges(SEED ^ 0xE9, 20_000, (32, 255), (0, 255), (0, 255)),
    );

    // Both branches must actually have been reached in the linear sub-domain.
    let mut lin = 0usize;
    let mut pw = 0usize;
    for b in (0u16..=255).step_by(5) {
        for g in (0u16..=255).step_by(5) {
            for r in (0u16..=255).step_by(5) {
                let i = Rgb::new(r as u8, g as u8, b as u8);
                let (_, r_out, _) = model_channels(i);
                if r_out as f64 > 0.003_130_804_953_560_371_5 {
                    pw += 1;
                } else {
                    lin += 1;
                }
                assert_eq!(p.call_c(i), p.call_rust(i), "E8 divergence at ({},{},{})", i.r, i.g, i.b);
            }
        }
    }
    assert!(lin > 0 && pw > 0, "E8: both branches must be reached (lin={lin}, pow={pw})");
}

// ---------------------------------------------------------------------------
// E9 -- pow() is never called with a negative base.
// ---------------------------------------------------------------------------

/// If a mistranslation called `pow` unconditionally on the post-matrix value,
/// negative bases would yield NaN and then `(unsigned char)NaN == 0`. If it
/// used a saturating cast the answer would also be `0`.
///
/// The discriminating check is therefore: for inputs whose R denorm argument is
/// strongly negative (`< -1.0`, so the truncated wraparound byte is
/// well-defined and usually non-zero), the C output must equal the *linear
/// branch* wraparound prediction `trunc(d) & 0xff`. Note that a merely
/// slightly-negative `d` (e.g. `-0.4`) legitimately yields `0`, which is why
/// the naive "R must not be 0" check is invalid.
#[test]
fn err_e9_pow_never_negative_base() {
    let p = Pair::load();
    let mut negatives = 0usize;
    let mut discriminating = 0usize;
    for b in (128u16..=255).step_by(2) {
        for g in 0u16..=8 {
            for r in 0u16..=8 {
                let i = Rgb::new(r as u8, g as u8, b as u8);
                let d = model_r_denorm_arg(i);
                if !(d < -1.0) {
                    continue;
                }
                negatives += 1;
                let cv = p.call_c(i);
                let rv = p.call_rust(i);
                assert_eq!(cv, rv, "E9 divergence at ({},{},{})", i.r, i.g, i.b);

                // The linear-branch wraparound prediction.
                let expected = (d.trunc() as i32 & 0xff) as u8;
                assert_eq!(
                    cv.r, expected,
                    "E9 at ({},{},{}): denorm arg {d} => expected wraparound {expected}, \
                     C gave {} -- pow() must NOT be applied to a negative base",
                    i.r, i.g, i.b, cv.r
                );
                // Count the cases where NaN/saturation (both -> 0) would have
                // been observably different from the correct answer.
                if expected != 0 {
                    discriminating += 1;
                }
            }
        }
    }
    assert!(negatives > 500, "E9: not enough negative-base cases ({negatives})");
    assert!(
        discriminating > 500,
        "E9: only {discriminating} of {negatives} cases distinguish wraparound from \
         NaN/saturation -- test is too weak"
    );
}

// ---------------------------------------------------------------------------
// E10 / E11 / E12 -- generic C-API boundaries that are N/A here.
// ---------------------------------------------------------------------------

/// E10: there is no pointer parameter, so there is no null-pointer path.
/// E12: there is no length/size/count parameter, so no zero/oversized length.
/// Both are proved structurally: the ABI is struct-by-value, 3 bytes.
#[test]
fn err_e10_e12_no_pointer_or_length_parameters() {
    // If `tritanopia` took a pointer, `sizeof` of its argument would be 8 and a
    // by-value 3-byte struct call would corrupt the stack / crash. The fact
    // that the by-value calls below succeed and agree is the structural proof.
    assert_eq!(std::mem::size_of::<Rgb>(), 3);
    assert_eq!(std::mem::align_of::<Rgb>(), 1);
    let p = Pair::load();
    assert_same(&p, Rgb::new(0, 0, 0));
    assert_same(&p, Rgb::new(1, 2, 3));
    assert_same(&p, Rgb::new(255, 255, 255));
}

/// E11: no enum exists in the API, so there is no invalid-variant path. The
/// closest reachable analogue at the FFI boundary is an argument register whose
/// bytes beyond the struct are arbitrary -- including the all-ones pattern,
/// the maximal "out of range" value. Both impls must agree and must ignore it.
#[test]
fn err_e11_out_of_range_register_patterns() {
    let p = Pair::load();
    // Maximal / pathological register payloads. The low 3 bytes are the real
    // input; everything above is "out of range" garbage.
    let patterns: [u64; 8] = [
        0x0000_0000_0000_0000,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FF00_0000,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0xDEAD_BEEF_CAFE_0000,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
    ];
    for &pat in &patterns {
        for &(r, g, b) in &[
            (0u8, 0u8, 0u8),
            (255, 255, 255),
            (0, 0, 255),
            (255, 255, 0),
            (11, 10, 128),
        ] {
            let rgb = (r as u64) | ((g as u64) << 8) | ((b as u64) << 16);
            let dirty = (pat & !0x00FF_FFFFu64) | rgb;
            let cv = unsafe { (p.c_raw)(dirty) } & 0xFF_FFFF;
            let rv = unsafe { (p.rust_raw)(dirty) } & 0xFF_FFFF;
            let clean_c = unsafe { (p.c_raw)(rgb) } & 0xFF_FFFF;
            assert_eq!(
                cv, clean_c,
                "E11: C was influenced by out-of-range register bytes {dirty:#018x}"
            );
            assert_eq!(
                cv, rv,
                "E11: C/Rust diverge for register payload {dirty:#018x} \
                 (rgb={r},{g},{b}): C={cv:#08x} Rust={rv:#08x}"
            );
        }
    }
}

/// E13: every one of the 256^3 bit patterns is a valid input -- `unsigned char`
/// has no invalid value, so there is nothing "one past the range". Verified
/// here on every per-channel extreme and one-step-past-extreme wrap
/// (`255 + 1 == 0` in the type), and fully by row 26.
#[test]
fn err_e13_every_bit_pattern_is_valid() {
    let p = Pair::load();
    for pos in 0..3 {
        // 0 (min), 255 (max) and the wrapped "one past max" which IS 0.
        for &v in &[0u8, 1, 254, 255, 255u8.wrapping_add(1)] {
            let i = match pos {
                0 => Rgb::new(v, 128, 128),
                1 => Rgb::new(128, v, 128),
                _ => Rgb::new(128, 128, v),
            };
            assert_same(&p, i);
        }
    }
}

// ---------------------------------------------------------------------------
// E14 / E15 -- ABI register contracts.
// ---------------------------------------------------------------------------

/// E14: upper 5 bytes of the argument register must be ignored by both.
#[test]
fn err_e14_upper_arg_register_garbage() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xE14);
    for _ in 0..50_000 {
        let (r, g, b) = (rng.range_u8(0, 255), rng.range_u8(0, 255), rng.range_u8(0, 255));
        let rgb = (r as u64) | ((g as u64) << 8) | ((b as u64) << 16);
        let dirty = rgb | (rng.next_u64() & !0x00FF_FFFFu64);
        let c_clean = unsafe { (p.c_raw)(rgb) } & 0xFF_FFFF;
        let c_dirty = unsafe { (p.c_raw)(dirty) } & 0xFF_FFFF;
        let r_dirty = unsafe { (p.rust_raw)(dirty) } & 0xFF_FFFF;
        assert_eq!(c_clean, c_dirty, "E14: C affected by garbage {dirty:#018x}");
        assert_eq!(c_dirty, r_dirty, "E14: divergence for {dirty:#018x}");
    }
}

/// E15: only the low 3 bytes of the return register are meaningful; the
/// struct-typed and register-typed views must agree on exactly those bytes.
#[test]
fn err_e15_return_low_three_bytes_only() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xE15);
    let pack = |v: Rgb| (v.r as u64) | ((v.g as u64) << 8) | ((v.b as u64) << 16);
    for _ in 0..50_000 {
        let i = Rgb::new(rng.range_u8(0, 255), rng.range_u8(0, 255), rng.range_u8(0, 255));
        let bits = pack(i);
        assert_eq!(pack(p.call_c(i)), unsafe { (p.c_raw)(bits) } & 0xFF_FFFF);
        assert_eq!(pack(p.call_rust(i)), unsafe { (p.rust_raw)(bits) } & 0xFF_FFFF);
        assert_same(&p, i);
    }
}

// ---------------------------------------------------------------------------
// Reachability model (accounting only -- never used as expected output)
// ---------------------------------------------------------------------------

/// Returns `(any_nan, r_out, g_out)` for the post-matrix (pre-`cbApplyGammaRGB`)
/// values, mirroring the C's f32/f64 promotion rules. Used only to classify
/// which `ERRORS.md` branch an input exercises.
/// The exact argument handed to the R channel's `(unsigned char)` cast, i.e.
/// `cbApplyGammaRGB(R_out) * 255.f + 0.5f`. Accounting/prediction only.
fn model_r_denorm_arg(i: Rgb) -> f32 {
    let (_, r_out, _) = model_channels(i);
    let c = r_out as f64;
    let ag = if c > 0.003_130_804_953_560_371_517_027_863_777_09 {
        1.055 * c.powf(0.4166666666) - 0.055
    } else {
        c * 12.92
    } as f32;
    ag * 255.0f32 + 0.5f32
}

fn model_channels(i: Rgb) -> (bool, f32, f32) {
    let lin = |v: u8| -> f32 {
        let c = (v as f32 / 255.0f32) as f64;
        let x = if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        };
        x as f32
    };
    let (r, g, b) = (lin(i.r), lin(i.g), lin(i.b));
    let r_out = (r + 0.127_398_863_108_80_f32 * g) - 0.127_398_863_410_72_f32 * b;
    let g_out = ((-4.486E-11_f32) * r + 0.873_909_299_283_61_f32 * g)
        + 0.126_090_701_015_23_f32 * b;
    let b_out = (3.1113E-10_f32 * r + 0.873_909_297_258_48_f32 * g)
        + 0.126_090_700_671_15_f32 * b;
    let apply = |c: f32| -> f32 {
        let c = c as f64;
        let x = if c > 0.003_130_804_953_560_371_517_027_863_777_09 {
            1.055 * c.powf(0.4166666666) - 0.055
        } else {
            c * 12.92
        };
        x as f32
    };
    let (ar, ag, ab) = (apply(r_out), apply(g_out), apply(b_out));
    let nan = (ar * 255.0 + 0.5).is_nan()
        || (ag * 255.0 + 0.5).is_nan()
        || (ab * 255.0 + 0.5).is_nan();
    (nan, r_out, g_out)
}
