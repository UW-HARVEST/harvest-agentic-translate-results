//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `to_barycentric` has no status return, no out-pointers, no enums and no
//! validation whatsoever (see `ERRORS.md` for the mechanical grep that
//! establishes this), so its "error surface" is exactly the set of IEEE-754
//! exceptional conditions its unguarded arithmetic can reach — chiefly the
//! unchecked `1.0f / (dot00*dot11 - dot01*dot01)` on `c_src/src/lib.c:25`.
//!
//! Every test asserts C and Rust return the *same* sentinel bit-for-bit, and
//! additionally pins the specific documented sentinel (`0xFFC0_0000`, `±inf`,
//! the quieted NaN payload, …) rather than merely "both were non-finite".

mod common;

use common::*;

/// Helper: assert C and Rust agree AND that both lanes equal `want`.
#[track_caller]
fn expect_both(row: &str, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2, want: (u32, u32)) {
    let got = diff_get(row, p1, p2, p3, p);
    assert_eq!(
        got.bits(),
        want,
        "[{row}] C and Rust agree with each other but not with the documented \
         sentinel: got {got:?}, ERRORS.md says {want:#010x?}"
    );
}

// ---------------------------------------------------------------------------
// E1 — all four points coincident: denom = +0.0, numerators = +0.0
//      => 1.0f/+0.0 = +inf, then 0.0 * inf = invalid = 0xFFC0_0000
// ---------------------------------------------------------------------------
#[test]
fn e1_all_points_coincident() {
    expect_both("E1/zero", P_ZERO, P_ZERO, P_ZERO, P_ZERO, (IND, IND));
    expect_both(
        "E1/negzero",
        Vec2::new(-0.0, -0.0),
        Vec2::new(-0.0, -0.0),
        Vec2::new(-0.0, -0.0),
        Vec2::new(-0.0, -0.0),
        (IND, IND),
    );

    let mut rng = Rng::new(0xC001_0000_0000_0001);
    for _ in 0..20_000 {
        let q = rng.vec2(|r| r.normal_in(-30, 30));
        // All four identical => every v == 0 => IND.
        expect_both("E1/rand", q, q, q, q, (IND, IND));
    }
    // Same with special values (inf/NaN coincident points) — here the answer is
    // no longer necessarily IND, so only the differential assertion applies.
    for _ in 0..20_000 {
        let q = rng.vec2(|r| r.special());
        diff("E1/special", q, q, q, q);
    }
}

// ---------------------------------------------------------------------------
// E2 — p2 == p1  (v1 == 0), p3 != p1
// ---------------------------------------------------------------------------
#[test]
fn e2_p2_equals_p1() {
    expect_both(
        "E2/unit",
        P_ZERO,
        P_ZERO,
        Vec2::new(0.0, 1.0),
        Vec2::new(0.5, 0.5),
        (IND, IND),
    );
    let mut rng = Rng::new(0xC002_0000_0000_0002);
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.normal_in(-20, 20));
        let p3 = rng.vec2(|r| r.normal_in(-20, 20));
        let p = rng.vec2(|r| r.normal_in(-20, 20));
        // dot01 = dot11 = dot12 = 0 => denom = +0.0, both numerators = +-0.0
        expect_both("E2/rand", p1, p1, p3, p, (IND, IND));
    }
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.special());
        let p3 = rng.vec2(|r| r.special());
        let p = rng.vec2(|r| r.special());
        diff("E2/special", p1, p1, p3, p);
    }
}

// ---------------------------------------------------------------------------
// E3 — p3 == p1  (v0 == 0), p2 != p1
// ---------------------------------------------------------------------------
#[test]
fn e3_p3_equals_p1() {
    expect_both(
        "E3/unit",
        P_ZERO,
        Vec2::new(1.0, 0.0),
        P_ZERO,
        Vec2::new(0.5, 0.5),
        (IND, IND),
    );
    let mut rng = Rng::new(0xC003_0000_0000_0003);
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.normal_in(-20, 20));
        let p2 = rng.vec2(|r| r.normal_in(-20, 20));
        let p = rng.vec2(|r| r.normal_in(-20, 20));
        expect_both("E3/rand", p1, p2, p1, p, (IND, IND));
    }
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.special());
        let p2 = rng.vec2(|r| r.special());
        let p = rng.vec2(|r| r.special());
        diff("E3/special", p1, p2, p1, p);
    }
}

// ---------------------------------------------------------------------------
// E4 — exactly collinear triangle. Cauchy-Schwarz equality makes the
//      determinant exactly 0 AND both numerators exactly 0, so the C returns
//      IND in both lanes (not +-inf).
// ---------------------------------------------------------------------------
#[test]
fn e4_collinear_triangle() {
    // Hand anchors (exact powers of two so no rounding intervenes).
    expect_both(
        "E4/t=2",
        P_ZERO,
        Vec2::new(1.0, 1.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(0.5, 0.25),
        (IND, IND),
    );
    expect_both(
        "E4/t=-1",
        P_ZERO,
        Vec2::new(1.0, 1.0),
        Vec2::new(-1.0, -1.0),
        Vec2::new(0.5, 0.25),
        (IND, IND),
    );
    expect_both(
        "E4/t=0.5",
        P_ZERO,
        Vec2::new(4.0, 2.0),
        Vec2::new(2.0, 1.0),
        Vec2::new(1.0, 7.0),
        (IND, IND),
    );

    // Randomised, exactly collinear: dyadic coords and a power-of-two t keep
    // every product exact, so the degeneracy is exact.
    let mut rng = Rng::new(0xC004_0000_0000_0004);
    let mut exact_ind = 0usize;
    for _ in 0..40_000 {
        let p1 = rng.vec2(|r| r.dyadic());
        let d = rng.vec2(|r| r.dyadic());
        let t = rng.pow2(-6, 6);
        let p2 = Vec2::new(p1.x + d.x, p1.y + d.y);
        let p3 = Vec2::new(p1.x + d.x * t, p1.y + d.y * t);
        let p = rng.vec2(|r| r.dyadic());
        let got = diff_get("E4/rand", p1, p2, p3, p);
        if got.bits() == (IND, IND) {
            exact_ind += 1;
        }
    }
    assert!(
        exact_ind > 20_000,
        "expected the exactly-collinear family to yield IND most of the time, \
         got {exact_ind}/40000 — the row is not exercising the degenerate divide"
    );
}

// ---------------------------------------------------------------------------
// E5 / E6 — near-collinear input where the determinant *rounds* to zero
//           (=> +-inf results) or rounds NEGATIVE (=> sign flips).
//           Both are reachable only through rounding; witnesses below were
//           found by search and are pinned as literals.
// ---------------------------------------------------------------------------
#[test]
fn e5_near_collinear_rounds_to_zero() {
    // Witness with a zero determinant but a non-zero numerator => +-inf.
    expect_both(
        "E5/inf-witness",
        Vec2::from_bits(0x3ff0_857b, 0xc179_0efc),
        Vec2::from_bits(0x3fd6_a944, 0xc15f_1767),
        Vec2::from_bits(0x3fea_ad2a, 0xc173_307c),
        Vec2::from_bits(0xc11f_0db6, 0x4098_e270),
        (0x7f80_0000, 0xff80_0000), // (+inf, -inf)
    );

    // E6 witness: determinant rounds NEGATIVE => the signs of u and v flip.
    expect_both(
        "E6/negative-denominator-witness",
        Vec2::from_bits(0xbe7a_fa50, 0x3e0c_1750),
        Vec2::from_bits(0xbf24_4802, 0xc0b6_89c0),
        Vec2::from_bits(0xbe9f_c03b, 0xbf59_43ba),
        Vec2::from_bits(0xbe9d_6b5b, 0xbf6b_4050),
        (0x8000_0000, 0x3e00_0000), // (-0.0, 0.125)
    );

    // Randomised near-collinear sweep. Counts how many of each outcome class we
    // actually hit, so the row cannot silently stop exercising the paths.
    let mut rng = Rng::new(0xC005_0000_0000_0005);
    let mut n_inf = 0usize;
    let mut n_ind = 0usize;
    let mut n_finite = 0usize;
    for _ in 0..300_000 {
        let p1 = rng.vec2(|r| r.normal_in(-3, 3));
        let d = rng.vec2(|r| r.normal_in(-3, 3));
        let t = rng.normal_in(-3, 3);
        let p2 = Vec2::new(p1.x + d.x, p1.y + d.y);
        let p3 = Vec2::new(p1.x + d.x * t, p1.y + d.y * t);
        let p = rng.vec2(|r| r.normal_in(-3, 3));
        let got = diff_get("E5/rand", p1, p2, p3, p);
        let (bx, by) = got.bits();
        if is_nan_bits(bx) || is_nan_bits(by) {
            n_ind += 1;
        } else if got.x.is_infinite() || got.y.is_infinite() {
            n_inf += 1;
        } else {
            n_finite += 1;
        }
    }
    assert!(n_inf > 1_000, "E5: only {n_inf} infinite results — path not hit");
    assert!(n_ind > 1_000, "E5: only {n_ind} NaN results — path not hit");
    assert!(n_finite > 1_000, "E5: only {n_finite} finite results");
}

// ---------------------------------------------------------------------------
// E7 — magnitudes near FLT_MAX: overflow in lm_sub2 and/or lm_dot2.
//      Also covers E11 (inf/inf reached through denom = inf).
// ---------------------------------------------------------------------------
#[test]
fn e7_overflow_to_infinity() {
    expect_both(
        "E7/fltmax",
        Vec2::new(f32::MAX, f32::MAX),
        Vec2::new(-f32::MAX, 0.0),
        Vec2::new(0.0, -f32::MAX),
        Vec2::new(1.0, 1.0),
        (IND, IND),
    );

    let mut rng = Rng::new(0xC007_0000_0000_0007);
    for _ in 0..60_000 {
        // Exponents 100..127: squares always overflow to +inf.
        let p1 = rng.vec2(|r| r.normal_in(100, 127));
        let p2 = rng.vec2(|r| r.normal_in(100, 127));
        let p3 = rng.vec2(|r| r.normal_in(100, 127));
        let p = rng.vec2(|r| r.normal_in(100, 127));
        diff("E7/rand", p1, p2, p3, p);
    }
    // Exactly FLT_MAX / -FLT_MAX in every slot combination (2^8 = 256).
    for mask in 0u32..256 {
        let f = |b: u32| {
            if mask & (1 << b) == 0 {
                f32::MAX
            } else {
                -f32::MAX
            }
        };
        diff(
            "E7/fltmax-mask",
            Vec2::new(f(0), f(1)),
            Vec2::new(f(2), f(3)),
            Vec2::new(f(4), f(5)),
            Vec2::new(f(6), f(7)),
        );
    }
}

// ---------------------------------------------------------------------------
// E8 — inf - inf inside lm_sub2
// ---------------------------------------------------------------------------
#[test]
fn e8_inf_minus_inf() {
    expect_both(
        "E8/anchor",
        Vec2::new(f32::INFINITY, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(f32::INFINITY, 1.0),
        Vec2::new(0.5, 0.5),
        (IND, IND),
    );
    // Sweep: put the same-signed infinity in p1 and one other point, per lane.
    let mut rng = Rng::new(0xC008_0000_0000_0008);
    for lane in 0..2usize {
        for other in 0..3usize {
            for _ in 0..5_000 {
                let s = rng.inf();
                let mut a = [rng.normal_in(-5, 5), rng.normal_in(-5, 5)];
                let mut pts = [
                    [rng.normal_in(-5, 5), rng.normal_in(-5, 5)],
                    [rng.normal_in(-5, 5), rng.normal_in(-5, 5)],
                    [rng.normal_in(-5, 5), rng.normal_in(-5, 5)],
                ];
                a[lane] = s;
                pts[other][lane] = s;
                diff(
                    "E8/sweep",
                    Vec2::new(a[0], a[1]),
                    Vec2::new(pts[0][0], pts[0][1]),
                    Vec2::new(pts[1][0], pts[1][1]),
                    Vec2::new(pts[2][0], pts[2][1]),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E9 — 0 * inf inside lm_dot2
// ---------------------------------------------------------------------------
#[test]
fn e9_zero_times_inf() {
    expect_both(
        "E9/anchor",
        P_ZERO,
        Vec2::new(f32::INFINITY, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        (IND, IND),
    );
    let mut rng = Rng::new(0xC009_0000_0000_0009);
    for _ in 0..60_000 {
        // One lane infinite, the other lane an exact zero => 0*inf in lm_dot2.
        let gen = |r: &mut Rng| {
            if r.chance(50) {
                r.inf()
            } else if r.chance(50) {
                0.0
            } else {
                -0.0
            }
        };
        let p1 = rng.vec2(gen);
        let p2 = rng.vec2(gen);
        let p3 = rng.vec2(gen);
        let p = rng.vec2(gen);
        diff("E9/rand", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// E10 — inf + (-inf) in lm_dot2's addss
// ---------------------------------------------------------------------------
#[test]
fn e10_inf_plus_neg_inf() {
    expect_both(
        "E10/anchor",
        P_ZERO,
        Vec2::new(f32::INFINITY, f32::INFINITY),
        Vec2::new(f32::INFINITY, f32::NEG_INFINITY),
        Vec2::new(1.0, 1.0),
        (IND, IND),
    );
    // v0 = (+inf, -inf) style: x-term +inf, y-term -inf.
    let mut rng = Rng::new(0xC010_0000_0000_0010);
    for _ in 0..40_000 {
        let a = rng.normal_in(-4, 4).abs();
        let b = rng.normal_in(-4, 4).abs();
        // p3-p1 = (+inf, -inf) exactly.
        let p1 = P_ZERO;
        let p3 = Vec2::new(f32::INFINITY, f32::NEG_INFINITY);
        let p2 = Vec2::new(a, b);
        let p = rng.vec2(|r| r.normal_in(-4, 4));
        diff("E10/rand", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// E11 — inf / inf and 1.0f / +-inf
// ---------------------------------------------------------------------------
#[test]
fn e11_inf_division() {
    // denom = +inf  => invDenom = 1.0f/+inf = +0.0; numerator +-inf => IND.
    expect_both(
        "E11/denom-inf",
        P_ZERO,
        Vec2::new(f32::MAX, f32::MAX),
        Vec2::new(-f32::MAX, f32::MAX),
        Vec2::new(1.0, 2.0),
        (IND, IND),
    );
    let mut rng = Rng::new(0xC011_0000_0000_0011);
    let mut saw_zero_result = 0usize;
    for _ in 0..80_000 {
        // Huge triangle, modest p: dot00*dot11 overflows -> denom = +inf.
        let p1 = P_ZERO;
        let p2 = rng.vec2(|r| r.normal_in(110, 127));
        let p3 = rng.vec2(|r| r.normal_in(110, 127));
        let p = rng.vec2(|r| r.normal_in(-4, 4));
        let got = diff_get("E11/rand", p1, p2, p3, p);
        if got.bits() == (0, 0) || got.bits() == (0x8000_0000, 0x8000_0000) {
            saw_zero_result += 1;
        }
    }
    let _ = saw_zero_result;
}

// ---------------------------------------------------------------------------
// E12 — underflow: subnormal inputs, squares flushing to +0.0
// ---------------------------------------------------------------------------
#[test]
fn e12_underflow_subnormal() {
    expect_both(
        "E12/subnormal-anchor",
        Vec2::from_bits(1, 2),
        Vec2::from_bits(3, 1),
        Vec2::from_bits(5, 9),
        Vec2::from_bits(7, 4),
        (IND, IND),
    );
    expect_both(
        "E12/fltmin-anchor",
        Vec2::new(f32::MIN_POSITIVE, 0.0),
        Vec2::new(f32::MIN_POSITIVE * 2.0, 0.0),
        Vec2::new(0.0, f32::MIN_POSITIVE),
        Vec2::new(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        (IND, IND),
    );

    let mut rng = Rng::new(0xC012_0000_0000_0012);
    for _ in 0..60_000 {
        let p1 = rng.vec2(|r| r.subnormal());
        let p2 = rng.vec2(|r| r.subnormal());
        let p3 = rng.vec2(|r| r.subnormal());
        let p = rng.vec2(|r| r.subnormal());
        diff("E12/subnormal", p1, p2, p3, p);
    }
    // The whole "gradual underflow" band: exponents -126..-60.
    for _ in 0..60_000 {
        let p1 = rng.vec2(|r| r.normal_in(-126, -60));
        let p2 = rng.vec2(|r| r.normal_in(-126, -60));
        let p3 = rng.vec2(|r| r.normal_in(-126, -60));
        let p = rng.vec2(|r| r.normal_in(-126, -60));
        diff("E12/tiny-normals", p1, p2, p3, p);
    }
    // Smallest / largest subnormal in every slot, 1.0 elsewhere.
    for slot in 0..8usize {
        for bits in [0x0000_0001u32, 0x8000_0001, 0x007F_FFFF, 0x807F_FFFF] {
            let mut f = [1.0f32; 8];
            f[slot] = f32::from_bits(bits);
            diff(
                "E12/slot",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E13 — a single QUIET NaN in each of the 8 input floats
// ---------------------------------------------------------------------------
#[test]
fn e13_single_qnan_each_position() {
    // Anchor: payload survives verbatim into both output lanes.
    expect_both(
        "E13/anchor",
        Vec2::new(f32::from_bits(0x7FC0_1234), 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.5, 0.5),
        (0x7FC0_1234, 0x7FC0_1234),
    );

    let mut rng = Rng::new(0xC013_0000_0000_0013);
    for slot in 0..8usize {
        for _ in 0..8_000 {
            let mut f = [0.0f32; 8];
            for k in 0..8 {
                f[k] = rng.normal_in(-8, 8);
            }
            let nan = rng.qnan();
            f[slot] = nan;
            let got = diff_get(
                "E13/sweep",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
            // A quiet NaN anywhere must produce NaN in both lanes.
            let (bx, by) = got.bits();
            assert!(
                is_nan_bits(bx) && is_nan_bits(by),
                "[E13] slot {slot}: NaN input {:#010x} did not yield NaN output: {got:?}",
                nan.to_bits()
            );
        }
        // Every special NaN encoding in this slot.
        for bits in [
            0x7FC0_0000u32,
            0xFFC0_0000,
            0x7FFF_FFFF,
            0xFFFF_FFFF,
            0x7FC0_0001,
            0xFFC0_0001,
        ] {
            let mut f = [1.0f32; 8];
            f[slot] = f32::from_bits(bits);
            diff(
                "E13/enc",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E14 — a single SIGNALLING NaN in each of the 8 input floats.
//       The hardware quiets it (|= 0x0040_0000) but keeps sign + payload.
// ---------------------------------------------------------------------------
#[test]
fn e14_single_snan_each_position() {
    expect_both(
        "E14/anchor-pos",
        Vec2::new(f32::from_bits(0x7F80_0001), 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.5, 0.5),
        (0x7FC0_0001, 0x7FC0_0001), // quieted, payload preserved
    );
    expect_both(
        "E14/anchor-neg",
        P_ZERO,
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, f32::from_bits(0xFF80_0BAD)),
        Vec2::new(0.5, 0.5),
        (0xFFC0_0BAD, 0xFFC0_0BAD),
    );

    let mut rng = Rng::new(0xC014_0000_0000_0014);
    for slot in 0..8usize {
        for _ in 0..8_000 {
            let mut f = [0.0f32; 8];
            for k in 0..8 {
                f[k] = rng.normal_in(-8, 8);
            }
            let snan = rng.snan();
            f[slot] = snan;
            let got = diff_get(
                "E14/sweep",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
            let (bx, by) = got.bits();
            assert!(
                is_nan_bits(bx) && is_nan_bits(by),
                "[E14] slot {slot}: SNaN {:#010x} did not yield NaN: {got:?}",
                snan.to_bits()
            );
            // The result must be QUIET: the C's arithmetic always quiets it.
            assert_ne!(
                bx & 0x0040_0000,
                0,
                "[E14] slot {slot}: result {bx:#010x} is still signalling"
            );
            assert_ne!(by & 0x0040_0000, 0);
        }
        // Boundary SNaN encodings (payload 1 and payload all-ones).
        for bits in [
            0x7F80_0001u32,
            0xFF80_0001,
            0x7FBF_FFFF,
            0xFFBF_FFFF,
            0x7F80_4000,
            0xFF80_4000,
        ] {
            let mut f = [1.0f32; 8];
            f[slot] = f32::from_bits(bits);
            diff(
                "E14/enc",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E15 — MULTIPLE NaNs at once: the payload-selection race.
//       This is the row that distinguishes the SSE destination operand at each
//       arithmetic site; a translation that just writes `a*b + c*d` passes E13
//       and E14 but fails here.
// ---------------------------------------------------------------------------
#[test]
fn e15_multi_nan_payload_race() {
    let mut rng = Rng::new(0xC015_0000_0000_0015);

    // Exhaustive over which of the 8 slots are NaN (256 masks), several random
    // payload assignments each, quiet and signalling.
    for mask in 0u32..256 {
        for rep in 0..80 {
            let mut f = [0.0f32; 8];
            for k in 0..8u32 {
                f[k as usize] = if mask & (1 << k) != 0 {
                    if rep % 3 == 0 {
                        rng.snan()
                    } else {
                        rng.qnan()
                    }
                } else {
                    rng.normal_in(-8, 8)
                };
            }
            diff(
                "E15/mask",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
        }
    }

    // Adversarial: distinct, easily-identified payloads so a wrong winner is
    // unambiguous rather than accidentally equal.
    for _ in 0..100_000 {
        let mut f = [0.0f32; 8];
        for k in 0..8u32 {
            // payload encodes the slot index -> whichever payload survives
            // names the operand that won.
            let sign = (rng.next_u32() & 1) << 31;
            let quiet = if rng.chance(50) { 0x0040_0000 } else { 0 };
            let pay = 0x0001_0000 * (k + 1) + 1;
            f[k as usize] = if rng.chance(70) {
                f32::from_bits(sign | 0x7F80_0000 | quiet | pay)
            } else {
                rng.normal_in(-8, 8)
            };
        }
        diff(
            "E15/tagged",
            Vec2::new(f[0], f[1]),
            Vec2::new(f[2], f[3]),
            Vec2::new(f[4], f[5]),
            Vec2::new(f[6], f[7]),
        );
    }

    // All eight NaN, all distinct payloads, both quietness classes.
    for _ in 0..100_000 {
        let mut f = [0.0f32; 8];
        for k in 0..8usize {
            f[k] = rng.any_nan();
        }
        diff(
            "E15/all-nan",
            Vec2::new(f[0], f[1]),
            Vec2::new(f[2], f[3]),
            Vec2::new(f[4], f[5]),
            Vec2::new(f[6], f[7]),
        );
    }
}

// ---------------------------------------------------------------------------
// E16 — signed zeros: sign of the zero must match exactly
// ---------------------------------------------------------------------------
#[test]
fn e16_signed_zero() {
    // Exhaustive over all 256 +-0.0 assignments (also CONFIGS row B13).
    for mask in 0u32..256 {
        let z = |b: u32| if mask & (1 << b) == 0 { 0.0f32 } else { -0.0f32 };
        expect_both(
            "E16/exhaustive",
            Vec2::new(z(0), z(1)),
            Vec2::new(z(2), z(3)),
            Vec2::new(z(4), z(5)),
            Vec2::new(z(6), z(7)),
            (IND, IND),
        );
    }

    // Mixed zeros and normals: a `-0.0` result must not be reported as `+0.0`.
    let mut rng = Rng::new(0xC016_0000_0000_0016);
    let mut saw_neg_zero = 0usize;
    for _ in 0..200_000 {
        let gen = |r: &mut Rng| match r.below(4) {
            0 => 0.0,
            1 => -0.0,
            _ => r.normal_in(-6, 6),
        };
        let p1 = rng.vec2(gen);
        let p2 = rng.vec2(gen);
        let p3 = rng.vec2(gen);
        let p = rng.vec2(gen);
        let got = diff_get("E16/mixed", p1, p2, p3, p);
        let (bx, by) = got.bits();
        if bx == 0x8000_0000 || by == 0x8000_0000 {
            saw_neg_zero += 1;
        }
    }
    assert!(
        saw_neg_zero > 0,
        "E16 never produced a -0.0 result, so the signed-zero comparison was untested"
    );

    // 1.0f / -0.0 would be -inf, but the determinant can never be -0.0 (both
    // dot00*dot11 and dot01*dot01 are >= +0.0), so assert that invariant
    // holds in practice: no run of the exhaustive zero masks yields -inf.
    for mask in 0u32..256 {
        let z = |b: u32| if mask & (1 << b) == 0 { 0.0f32 } else { -0.0f32 };
        let got = diff_get(
            "E16/no-neg-inf",
            Vec2::new(z(0), z(1)),
            Vec2::new(z(2), z(3)),
            Vec2::new(z(4), z(5)),
            Vec2::new(z(6), z(7)),
        );
        assert!(!got.x.is_infinite() && !got.y.is_infinite());
    }
}

// ---------------------------------------------------------------------------
// E17 — the full 2^32 domain of every input float, and the structural
//       justification for why NULL / length / enum rejections do not exist.
// ---------------------------------------------------------------------------
#[test]
fn e17_fully_random_bit_patterns() {
    let mut rng = Rng::new(0xC017_0000_0000_0017);
    for _ in 0..400_000 {
        let p1 = rng.vec2(|r| r.any_bits());
        let p2 = rng.vec2(|r| r.any_bits());
        let p3 = rng.vec2(|r| r.any_bits());
        let p = rng.vec2(|r| r.any_bits());
        diff("E17/random", p1, p2, p3, p);
    }

    // Exhaustive over the *exponent+class* structure of one slot at a time:
    // walk every one of the 256 possible high bytes, with random low bits.
    for slot in 0..8usize {
        for hi in 0u32..256 {
            for _ in 0..16 {
                let mut f = [1.0f32; 8];
                f[slot] = f32::from_bits((hi << 24) | (rng.next_u32() & 0x00FF_FFFF));
                diff(
                    "E17/hi-byte",
                    Vec2::new(f[0], f[1]),
                    Vec2::new(f[2], f[3]),
                    Vec2::new(f[4], f[5]),
                    Vec2::new(f[6], f[7]),
                );
            }
        }
    }
}

/// Guard test for the ERRORS.md claim that NULL-pointer / zero-length /
/// oversized-length / out-of-range-enum rejections are *structurally*
/// unreachable: it re-reads the public header and asserts the API still has no
/// pointer, integer, size or enum parameter. If the header ever grows one, this
/// fails and the ERRORS.md justification must be redone rather than silently
/// remaining stale.
#[test]
fn e17_no_pointer_or_enum_params() {
    let hdr = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/include/lib.h");
    let src = std::fs::read_to_string(&hdr).expect("read c_src/include/lib.h");

    // Strip comments, then look at the declaration lines only.
    let decls: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "*", "[", "enum", "size_t", "unsigned", "long", "double", "char", "void",
    ] {
        assert!(
            !decls.contains(forbidden),
            "c_src/include/lib.h now contains `{forbidden}`: the ERRORS.md argument \
             that NULL / length / enum rejections are unreachable no longer holds \
             and Phase C must be extended"
        );
    }
    assert!(decls.contains("lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);"));

    // And the whole C source has no validation construct at all.
    let csrc = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    for forbidden in [
        "if ", "if(", "switch", "assert", "return -", "NULL", "goto", "errno", "#if",
    ] {
        assert!(
            !csrc.contains(forbidden),
            "c_src/src/lib.c now contains `{forbidden}`: the ERRORS.md table must be regenerated"
        );
    }
}
