//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row, plus the generic C-API boundary cases.
//!
//! `c_src` has **no** error surface (no pointers, no lengths, no enums, no
//! branches, no asserts — see `ERRORS.md` for the mechanical derivation), so
//! "the same error/rejection" is asserted as "the same returned bit pattern",
//! which is strictly stronger than comparing an error code: it pins the exact
//! `NaN` payload / `inf` sign the C produces, not merely "both failed somehow".
//!
//! Each test additionally asserts the *class* of the C result (NaN vs infinite
//! vs finite) so that a row cannot silently stop exercising its condition.

mod common;

use common::*;

const N: u32 = 20_000;

/// Assert that C and Rust agree bit-for-bit and report both results.
fn both(d: &Dual, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    let c = d.call_c(p1, p2, p3, p);
    let r = d.call_rust(p1, p2, p3, p);
    assert_eq!(
        c.bits(),
        r.bits(),
        "divergence for p1=({:#010x},{:#010x}) p2=({:#010x},{:#010x}) \
         p3=({:#010x},{:#010x}) p=({:#010x},{:#010x}): C=({:#010x},{:#010x}) rust=({:#010x},{:#010x})",
        p1.x.to_bits(), p1.y.to_bits(), p2.x.to_bits(), p2.y.to_bits(),
        p3.x.to_bits(), p3.y.to_bits(), p.x.to_bits(), p.y.to_bits(),
        c.x.to_bits(), c.y.to_bits(), r.x.to_bits(), r.y.to_bits(),
    );
    c
}

/// E1 — fully degenerate triangle `p1 == p2 == p3`: `denom == 0` and both
/// numerators are `0`, so `0 * inf` yields NaN in both components.
#[test]
fn phase_c_e01_all_vertices_coincident() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE1);
    let mut diff = Diff::new(&d, "E1 all vertices coincident");
    for _ in 0..N {
        let a = rng.vec2_wide();
        let p = rng.vec2_wide();
        let out = both(&d, a, a, a, p);
        assert!(
            out.x.is_nan() && out.y.is_nan(),
            "E1 expected NaN from the C, got ({:e},{:e})",
            out.x,
            out.y
        );
        diff.check(a, a, a, p);
    }
    // the canonical case
    let z = Vec2::new(0.0, 0.0);
    let out = both(&d, z, z, z, Vec2::new(1.0, 2.0));
    assert_eq!(out.x.to_bits(), out.y.to_bits());
    assert!(out.x.is_nan(), "E1 origin case must be NaN");
    diff.finish();
}

/// E2 — `p2 == p1` (`v1 == 0`) with `p3 != p1`: `dot11 == dot01 == dot12 == 0`.
#[test]
fn phase_c_e02_p2_equals_p1() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE2);
    let mut diff = Diff::new(&d, "E2 p2 == p1");
    let mut saw_nan = false;
    for _ in 0..N {
        let a = rng.vec2_unit();
        let mut c3 = rng.vec2_unit();
        if c3 == a {
            c3 = Vec2::new(a.x + 1.0, a.y);
        }
        let p = rng.vec2_unit();
        let out = both(&d, a, a, c3, p);
        saw_nan |= out.x.is_nan() || out.y.is_nan();
        diff.check(a, a, c3, p);
    }
    assert!(saw_nan, "E2 never produced the degenerate result");
    diff.finish();
}

/// E3 — `p3 == p1` (`v0 == 0`): `dot00 == dot01 == dot02 == 0`.
#[test]
fn phase_c_e03_p3_equals_p1() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE3);
    let mut diff = Diff::new(&d, "E3 p3 == p1");
    let mut saw_nonfinite = false;
    for _ in 0..N {
        let a = rng.vec2_unit();
        let mut b = rng.vec2_unit();
        if b == a {
            b = Vec2::new(a.x + 1.0, a.y);
        }
        let p = rng.vec2_unit();
        let out = both(&d, a, b, a, p);
        saw_nonfinite |= !out.x.is_finite() || !out.y.is_finite();
        diff.check(a, b, a, p);
    }
    assert!(saw_nonfinite, "E3 never produced the degenerate result");
    diff.finish();
}

/// E4 — collinear but non-coincident vertices: `denom == 0` exactly while the
/// numerators are non-zero, so the result is `±inf` and the *sign* matters.
#[test]
fn phase_c_e04_collinear_infinite_result() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE4);
    let mut diff = Diff::new(&d, "E4 collinear -> inf");
    // hand-built exact cases: integer coordinates make the cancellation exact
    let cases: &[(Vec2, Vec2, Vec2, Vec2)] = &[
        (
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 1.0),
        ),
        (
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
        ),
        (
            Vec2::new(1.0, 1.0),
            Vec2::new(3.0, 3.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(-4.0, 7.0),
        ),
        (
            Vec2::new(0.0, 0.0),
            Vec2::new(-1.0, -2.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(3.0, -1.0),
        ),
    ];
    let mut saw_inf = false;
    for &(a, b, c, p) in cases {
        let out = both(&d, a, b, c, p);
        saw_inf |= out.x.is_infinite() || out.y.is_infinite() || out.x.is_nan();
        diff.check(a, b, c, p);
    }
    assert!(saw_inf, "E4 hand-built cases produced no non-finite result");
    // randomized exactly-parallel edges over small integers
    for _ in 0..N {
        let p1 = rng.vec2_small_int(8);
        let dir = rng.vec2_small_int(4);
        let s = rng.small_int(4);
        let t = rng.small_int(4);
        diff.check(
            p1,
            add2(p1, scale2(dir, t)),
            add2(p1, scale2(dir, s)),
            rng.vec2_small_int(8),
        );
    }
    diff.finish();
}

/// E5 — `denom` underflows to zero (or a subnormal) although the vertices are
/// distinct: coordinates around `1e-25`.
#[test]
fn phase_c_e05_denominator_underflow() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE5);
    let mut diff = Diff::new(&d, "E5 denom underflow");
    let scales = [1e-22f32, 1e-24, 1e-25, 1e-26, 1e-30, 1e-38];
    let mut saw_nonfinite = false;
    for _ in 0..N {
        let s = scales[rng.below(scales.len() as u32) as usize];
        let p1 = scale2(rng.vec2_unit(), s);
        let p2 = scale2(rng.vec2_unit(), s);
        let p3 = scale2(rng.vec2_unit(), s);
        let p = scale2(rng.vec2_unit(), s);
        let out = both(&d, p1, p2, p3, p);
        saw_nonfinite |= !out.x.is_finite() || !out.y.is_finite();
        diff.check(p1, p2, p3, p);
    }
    assert!(
        saw_nonfinite,
        "E5 never drove denom to zero — row not exercised"
    );
    diff.finish();
}

/// E6 — `denom` overflows to `+inf`: `invDenom == 0` and the numerator may be
/// `inf - inf == NaN`.
#[test]
fn phase_c_e06_denominator_overflow() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE6);
    let mut diff = Diff::new(&d, "E6 denom overflow");
    let scales = [1e20f32, 1e22, 1e25, 1e30, 1e35, 1e38];
    let mut saw_nan = false;
    for _ in 0..N {
        let s = scales[rng.below(scales.len() as u32) as usize];
        let p1 = scale2(rng.vec2_unit(), s);
        let p2 = scale2(rng.vec2_unit(), s);
        let p3 = scale2(rng.vec2_unit(), s);
        let p = scale2(rng.vec2_unit(), s);
        let out = both(&d, p1, p2, p3, p);
        saw_nan |= out.x.is_nan() || out.y.is_nan();
        diff.check(p1, p2, p3, p);
    }
    assert!(saw_nan, "E6 never produced NaN from inf - inf");

    // `invDenom == 0` with a *finite* numerator needs dot00 and dot11 each
    // finite but their product overflowing: |v0| = |v1| = 1e15 gives
    // dot00 = dot11 = 1e30, denom = 1e60 -> +inf, invDenom = +0. Keeping `p`
    // near `p1` keeps dot02/dot12 small so the numerators stay finite and the
    // result is an exact (signed) zero rather than NaN.
    let e = 1e15f32;
    let tiny = 1e-10f32;
    let mut saw_zero = false;
    for &(sx, sy) in &[(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let p1 = Vec2::new(0.0, 0.0);
        let p2 = Vec2::new(0.0, e * sy);
        let p3 = Vec2::new(e * sx, 0.0);
        let p = Vec2::new(tiny * sx, tiny * sy);
        let out = both(&d, p1, p2, p3, p);
        saw_zero |= out.x == 0.0 || out.y == 0.0;
        diff.check(p1, p2, p3, p);
    }
    assert!(
        saw_zero,
        "E6 never produced the invDenom == 0 case with a finite numerator"
    );
    // randomized variants of the same shape
    for _ in 0..N {
        let s = 1e14f32 * (1.0 + rng.unit() * 9.0);
        let p1 = Vec2::new(0.0, 0.0);
        let p2 = scale2(rng.vec2_unit(), s);
        let p3 = scale2(rng.vec2_unit(), s);
        let p = scale2(rng.vec2_unit(), 1e-10);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// E7 — `±inf` input components: `inf - inf == NaN` propagation.
#[test]
fn phase_c_e07_infinite_inputs() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE7);
    let mut diff = Diff::new(&d, "E7 infinite inputs");
    // every single slot set to +inf, then to -inf, over random surroundings
    for _ in 0..N {
        let mut base = [0f32; 8];
        for s in base.iter_mut() {
            *s = rng.signed_unit();
        }
        for slot in 0..8usize {
            for inf in [f32::INFINITY, f32::NEG_INFINITY] {
                let mut c = base;
                c[slot] = inf;
                let (p1, p2, p3, p) = from_components(c);
                diff.check(p1, p2, p3, p);
            }
        }
    }
    // all eight slots infinite, full 2^8 sign sweep
    for m in 0u32..256 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            *s = if m >> i & 1 == 1 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            };
        }
        let (p1, p2, p3, p) = from_components(c);
        let out = both(&d, p1, p2, p3, p);
        assert!(
            out.x.is_nan() && out.y.is_nan(),
            "E7 all-inf case should be NaN, got ({:e},{:e})",
            out.x,
            out.y
        );
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// E8 — quiet-NaN inputs with distinct payloads in every slot combination.
#[test]
fn phase_c_e08_quiet_nan_inputs() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE8);
    let mut diff = Diff::new(&d, "E8 quiet NaN inputs");
    // exhaustive over which slots are NaN (2^8 masks), unique payload per slot
    for m in 0u32..256 {
        for round in 0..8u32 {
            let mut c = [0f32; 8];
            for (i, s) in c.iter_mut().enumerate() {
                *s = if m >> i & 1 == 1 {
                    let payload = 1 + round * 8 + i as u32;
                    f32::from_bits(0x7FC0_0000 | payload)
                } else {
                    rng.signed_unit()
                };
            }
            let (p1, p2, p3, p) = from_components(c);
            let out = both(&d, p1, p2, p3, p);
            if m != 0 {
                assert!(out.x.is_nan() && out.y.is_nan(), "E8 expected NaN output");
            }
            diff.check(p1, p2, p3, p);
        }
    }
    // random payloads / signs
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = if rng.bool() {
                rng.quiet_nan()
            } else {
                rng.wide_normal()
            };
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// E9 — signalling-NaN inputs, quieted by the first arithmetic operation.
#[test]
fn phase_c_e09_signalling_nan_inputs() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xE9);
    let mut diff = Diff::new(&d, "E9 signalling NaN inputs");
    for m in 0u32..256 {
        for round in 0..8u32 {
            let mut c = [0f32; 8];
            for (i, s) in c.iter_mut().enumerate() {
                *s = if m >> i & 1 == 1 {
                    let payload = 1 + round * 8 + i as u32;
                    f32::from_bits(0x7F80_0000 | payload)
                } else {
                    rng.signed_unit()
                };
            }
            let (p1, p2, p3, p) = from_components(c);
            let out = both(&d, p1, p2, p3, p);
            if m != 0 {
                assert!(out.x.is_nan() && out.y.is_nan(), "E9 expected NaN output");
                // sNaN must have been quieted by the hardware
                assert_ne!(
                    out.x.to_bits() & 0x0040_0000,
                    0,
                    "E9 result should be a quiet NaN"
                );
            }
            diff.check(p1, p2, p3, p);
        }
    }
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = if rng.bool() {
                rng.signalling_nan()
            } else {
                rng.wide_normal()
            };
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// E10 — `-0.0` inputs: sign of zero must survive identically.
#[test]
fn phase_c_e10_negative_zero() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xEA);
    let mut diff = Diff::new(&d, "E10 negative zero");
    // exhaustive: every slot is +0.0 or -0.0
    for m in 0u32..256 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            *s = if m >> i & 1 == 1 { 0.0 } else { -0.0 };
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    // exhaustive over which slots are signed zero, rest random
    for m in 0u32..256 {
        for _ in 0..8 {
            let mut c = [0f32; 8];
            for (i, s) in c.iter_mut().enumerate() {
                *s = if m >> i & 1 == 1 {
                    if rng.bool() {
                        -0.0
                    } else {
                        0.0
                    }
                } else {
                    rng.signed_unit()
                };
            }
            let (p1, p2, p3, p) = from_components(c);
            diff.check(p1, p2, p3, p);
        }
    }
    // a case where the C really does return a signed zero
    let out = both(
        &d,
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(-0.0, -0.0),
    );
    assert_eq!(
        (out.x.to_bits(), out.y.to_bits()),
        (d.call_rust(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-0.0, -0.0)
        ))
        .bits()
    );
    diff.finish();
}

/// E11 — subnormal inputs.
#[test]
fn phase_c_e11_subnormal_inputs() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xEB);
    let mut diff = Diff::new(&d, "E11 subnormal inputs");
    // the two extreme subnormals in every slot mask
    let extremes = [f32::from_bits(1), f32::from_bits(0x807F_FFFF)];
    for m in 0u32..256 {
        for &sub in extremes.iter() {
            let mut c = [0f32; 8];
            for (i, s) in c.iter_mut().enumerate() {
                *s = if m >> i & 1 == 1 { sub } else { rng.signed_unit() };
            }
            let (p1, p2, p3, p) = from_components(c);
            diff.check(p1, p2, p3, p);
        }
    }
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.subnormal();
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// E12 — maximum-magnitude finite inputs (`±FLT_MAX`).
#[test]
fn phase_c_e12_flt_max_inputs() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xEC);
    let mut diff = Diff::new(&d, "E12 FLT_MAX inputs");
    // exhaustive sign sweep with every slot at ±FLT_MAX
    for m in 0u32..256 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            *s = if m >> i & 1 == 1 { f32::MAX } else { -f32::MAX };
        }
        let (p1, p2, p3, p) = from_components(c);
        let out = both(&d, p1, p2, p3, p);
        assert!(
            !out.x.is_finite() || !out.y.is_finite() || out.x == 0.0 || out.y == 0.0,
            "E12 expected overflow-affected result, got ({:e},{:e})",
            out.x,
            out.y
        );
        diff.check(p1, p2, p3, p);
    }
    // FLT_MAX spliced into random surroundings
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.signed_unit();
        }
        let k = 1 + rng.below(8);
        for _ in 0..k {
            c[rng.below(8) as usize] = if rng.bool() { f32::MAX } else { -f32::MAX };
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// E13 — `p` far outside the triangle: the C performs **no** range check, so
/// the result is a normal finite pair outside `[0, 1]`. This row exists to
/// prove the absence of a rejection.
#[test]
fn phase_c_e13_no_range_rejection() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xED);
    let mut diff = Diff::new(&d, "E13 no range rejection");
    let p1 = Vec2::new(0.0, 0.0);
    let p2 = Vec2::new(1.0, 0.0);
    let p3 = Vec2::new(0.0, 1.0);
    let mut saw_out_of_unit_range = false;
    for _ in 0..N {
        // barycentric coordinates well outside [0,1]
        let p = Vec2::new(rng.range(-1000.0, 1000.0), rng.range(-1000.0, 1000.0));
        let out = both(&d, p1, p2, p3, p);
        assert!(
            out.x.is_finite() && out.y.is_finite(),
            "E13 must stay finite: ({:e},{:e})",
            out.x,
            out.y
        );
        saw_out_of_unit_range |= out.x < 0.0 || out.x > 1.0 || out.y < 0.0 || out.y > 1.0;
        diff.check(p1, p2, p3, p);
    }
    assert!(
        saw_out_of_unit_range,
        "E13 never left [0,1] — the row is not exercising the condition"
    );
    // and the C returns a value (never a sentinel) for a wildly out-of-range p
    let out = both(&d, p1, p2, p3, Vec2::new(1e30, -1e30));
    assert!(out.x.is_finite() && out.y.is_finite());
    diff.finish();
}

/// E14 — unrestricted bit-pattern fuzz: proves there is no input the C rejects
/// and pins the exact result for every IEEE class combination.
#[test]
fn phase_c_e14_no_input_is_rejected() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xEE);
    let mut diff = Diff::new(&d, "E14 unrestricted fuzz");
    for _ in 0..200_000 {
        let p1 = rng.vec2_any();
        let p2 = rng.vec2_any();
        let p3 = rng.vec2_any();
        let p = rng.vec2_any();
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

// ---------------------------------------------------------------------------
// Generic C-API boundary cases required by Phase C even though `ERRORS.md`
// shows they are structurally inapplicable to this signature.
// ---------------------------------------------------------------------------

/// The public API has no pointer parameters, so a "null pointer" input cannot
/// be constructed. This test proves that mechanically from the exported
/// signature instead of silently skipping the case: the bit pattern that would
/// be a null pointer (all-zero words) is passed as the argument values, and the
/// nearest analogue (all-zero `lm_vec2`s) is compared.
#[test]
fn phase_c_boundary_no_pointer_parameters() {
    let d = Dual::load();
    let zero = Vec2::new(f32::from_bits(0), f32::from_bits(0));
    // "all arguments are the null bit pattern"
    let out = both(&d, zero, zero, zero, zero);
    assert!(out.x.is_nan() && out.y.is_nan());
    // one slot at a time zeroed, rest normal
    let mut rng = Rng::new(SEED ^ 0xF0);
    let mut diff = Diff::new(&d, "boundary null-analogue");
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.wide_normal();
        }
        for slot in 0..8usize {
            let mut cc = c;
            cc[slot] = f32::from_bits(0);
            let (p1, p2, p3, p) = from_components(cc);
            diff.check(p1, p2, p3, p);
        }
    }
    diff.finish();
}

/// The public API has no length/size/count parameter, so "zero and oversized
/// lengths" cannot be constructed. The closest reachable analogue is a
/// zero-length edge (`v0 == 0` and/or `v1 == 0`, i.e. a zero-extent triangle)
/// and an "oversized" extent (`FLT_MAX`-scale edges), both of which are
/// asserted bit-exactly here.
#[test]
fn phase_c_boundary_zero_and_oversized_extent() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xF1);
    let mut diff = Diff::new(&d, "boundary zero/oversized extent");
    for _ in 0..N {
        let a = rng.vec2_unit();
        let p = rng.vec2_unit();
        // zero extent in one or both edges
        diff.check(a, a, a, p);
        diff.check(a, a, add2(a, rng.vec2_unit()), p);
        diff.check(a, add2(a, rng.vec2_unit()), a, p);
        // oversized extent
        let huge = Vec2::new(f32::MAX, f32::MAX);
        diff.check(a, add2(a, huge), a, p);
        diff.check(
            Vec2::new(-f32::MAX, -f32::MAX),
            Vec2::new(f32::MAX, -f32::MAX),
            Vec2::new(-f32::MAX, f32::MAX),
            p,
        );
    }
    diff.finish();
}

/// The public API has no `enum` (or any integer) parameter, so an
/// "out-of-range enum value" cannot be constructed. `float` is the only
/// parameter type and it has **no** invalid bit pattern: all 2^32 words are
/// valid `float`s. This test proves the claim by walking the entire exponent
/// field plus the boundary mantissas of every IEEE class through every one of
/// the eight argument slots — the exhaustive analogue of "one step past the
/// documented valid range".
#[test]
fn phase_c_boundary_every_ieee_class_in_every_slot() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 0xF2);
    let mut diff = Diff::new(&d, "boundary IEEE class sweep");

    // Class boundaries: one step below / at / one step above every transition.
    let mut probes: Vec<u32> = Vec::new();
    for &base in &[
        0x0000_0000u32, // +0
        0x0000_0001,    // smallest subnormal
        0x007F_FFFF,    // largest subnormal
        0x0080_0000,    // smallest normal (FLT_MIN)
        0x0080_0001,
        0x3F7F_FFFF, // just below 1.0
        0x3F80_0000, // 1.0
        0x3F80_0001, // just above 1.0
        0x7F7F_FFFE,
        0x7F7F_FFFF, // FLT_MAX
        0x7F80_0000, // +inf
        0x7F80_0001, // smallest sNaN
        0x7FBF_FFFF, // largest sNaN
        0x7FC0_0000, // smallest qNaN
        0x7FFF_FFFF, // largest qNaN
    ] {
        probes.push(base);
        probes.push(base | 0x8000_0000); // negative counterpart
    }
    // full exponent walk at a fixed mantissa
    for exp in 0..=255u32 {
        probes.push((exp << 23) | 0x0012_3456);
        probes.push(0x8000_0000 | (exp << 23) | 0x0012_3456);
    }

    for &bits in &probes {
        let v = f32::from_bits(bits);
        for slot in 0..8usize {
            // against a fixed well-conditioned background
            let mut c = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
            c[slot] = v;
            let (p1, p2, p3, p) = from_components(c);
            diff.check(p1, p2, p3, p);
            // and against randomized backgrounds
            for _ in 0..4 {
                let mut r = [0f32; 8];
                for s in r.iter_mut() {
                    *s = rng.wide_normal();
                }
                r[slot] = v;
                let (q1, q2, q3, q) = from_components(r);
                diff.check(q1, q2, q3, q);
            }
        }
        // and with every slot set to the probe value at once
        let c = [v; 8];
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// Pairwise sweep: every ordered pair of class-boundary values placed in every
/// ordered pair of argument slots. This is the case that catches operand-order
/// bugs in commutative operations, because it is the only way to get two
/// *different* NaNs into the two operands of a single `mulss`/`addss`.
#[test]
fn phase_c_boundary_pairwise_class_sweep() {
    let d = Dual::load();
    let mut diff = Diff::new(&d, "boundary pairwise class sweep");
    let probes: [u32; 16] = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x0080_0000,
        0x3F80_0000,
        0xBF80_0000,
        0x7F7F_FFFF,
        0xFF7F_FFFF,
        0x7F80_0000,
        0xFF80_0000,
        0x7F80_0001,
        0x7FC0_0000,
        0xFFC0_0000,
        0x7FC1_2345,
        0xFFDE_ADBE,
    ];
    for &a in &probes {
        for &b in &probes {
            let va = f32::from_bits(a);
            let vb = f32::from_bits(b);
            for i in 0..8usize {
                for j in 0..8usize {
                    if i == j {
                        continue;
                    }
                    let mut c = [1.0f32, 2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0];
                    c[i] = va;
                    c[j] = vb;
                    let (p1, p2, p3, p) = from_components(c);
                    diff.check(p1, p2, p3, p);
                }
            }
        }
    }
    diff.finish();
}
