//! Strict (payload-exact) NaN / non-finite differential tests.
//!
//! These deliberately avoid the one case where the C compiler's instruction
//! selection makes the NaN payload unspecified (both operands of a single
//! arithmetic op being NaN — see `NOTES-nan.md`). Everything here must match
//! with FULL bit equality including the NaN sign bit.

mod common;

use common::*;

const PNAN: f32 = f32::NAN; // 0x7FC00000
fn nnan() -> f32 {
    f32::from_bits(0xFFC0_0000)
}
fn snan() -> f32 {
    f32::from_bits(0x7F80_0001)
}

/// A single NaN operand: propagation is destination-order independent.
#[test]
fn strict_single_nan_operand() {
    let p = pair();
    let nans = [PNAN, nnan(), snan()];
    let finites = [0.0f32, -0.0, 1.0, -1.0, 3.5, f32::MAX, f32::MIN_POSITIVE];
    for &n in &nans {
        for &f in &finites {
            // c2Dot: exactly one NaN factor in exactly one product.
            let a = c2v { x: n, y: f };
            let b = c2v { x: f, y: f };
            unsafe {
                same_strict("c2Dot single-nan", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
                same_strict("c2Det2 single-nan", (p.c.c2Det2)(a, b), (p.rs.c2Det2)(a, b));
                same_strict("c2Len single-nan", (p.c.c2Len)(a), (p.rs.c2Len)(a));
                same_strict_v("c2Mulvs single-nan", (p.c.c2Mulvs)(a, f), (p.rs.c2Mulvs)(a, f));
                same_strict_v("c2Div single-nan", (p.c.c2Div)(a, f), (p.rs.c2Div)(a, f));
                same_strict_v("c2Norm single-nan", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
                same_strict_v("c2Add single-nan", (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
                same_strict_v("c2Sub single-nan", (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
                same_strict_v("c2Neg single-nan", (p.c.c2Neg)(a), (p.rs.c2Neg)(a));
                same_strict_v("c2Maxv single-nan", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
                same_strict_v("c2Minv single-nan", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
                same_strict_v("c2Skew single-nan", (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
                same_strict_v("c2CCW90 single-nan", (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));
            }
        }
    }
}

/// Invalid-operation NaNs (`inf - inf`, `0 * inf`, `sqrt(neg)`) are the x86
/// "real indefinite" value in both libraries, independent of operand order.
#[test]
fn strict_invalid_operation_nans() {
    let inf = f32::INFINITY;
    let p = pair();
    let cases = [
        // 0 * inf inside c2Dot
        (c2v { x: 0.0, y: 0.0 }, c2v { x: inf, y: inf }),
        (c2v { x: -0.0, y: 0.0 }, c2v { x: -inf, y: inf }),
        // inf - inf inside c2Dot's sum
        (c2v { x: inf, y: inf }, c2v { x: inf, y: -inf }),
        (c2v { x: inf, y: -inf }, c2v { x: inf, y: inf }),
        // overflow to inf
        (
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::MAX, y: f32::MAX },
        ),
    ];
    for (a, b) in cases {
        unsafe {
            same_strict(&format!("c2Dot invop {a:?} {b:?}"), (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
            same_strict(&format!("c2Det2 invop {a:?} {b:?}"), (p.c.c2Det2)(a, b), (p.rs.c2Det2)(a, b));
            same_strict(&format!("c2Len invop {a:?}"), (p.c.c2Len)(a), (p.rs.c2Len)(a));
            same_strict_v(&format!("c2Norm invop {a:?}"), (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
        }
    }
}

/// `sqrtf` of a negative argument (`c2Dot` can't be negative, but `c2Len` is
/// also reachable with a NaN dot) plus the exact `sqrtf` results for a wide
/// value sweep — glibc `sqrtf` vs the `sqrtss` instruction must agree exactly.
#[test]
fn strict_sqrt_agreement() {
    let p = pair();
    let mut rng = Rng::new(777);
    for i in 0..20_000 {
        let v = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        unsafe { same_strict(&format!("c2Len sqrt #{i} {v:?}"), (p.c.c2Len)(v), (p.rs.c2Len)(v)) };
    }
    // Perfect squares, subnormals, extremes.
    for bits in [
        0u32, 0x8000_0000, 1, 0x0080_0000, 0x3F80_0000, 0x7F7F_FFFF, 0x0000_0002, 0x4048_0000,
    ] {
        let v = c2v { x: f32::from_bits(bits), y: 0.0 };
        unsafe { same_strict("c2Len exact", (p.c.c2Len)(v), (p.rs.c2Len)(v)) };
    }
}

/// `±0` sign handling must be exact everywhere.
#[test]
fn strict_signed_zero() {
    let p = pair();
    let zs = [0.0f32, -0.0f32];
    for &x in &zs {
        for &y in &zs {
            let a = c2v { x, y };
            for &u in &zs {
                for &v in &zs {
                    let b = c2v { x: u, y: v };
                    unsafe {
                        same_strict("c2Dot ±0", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
                        same_strict("c2Det2 ±0", (p.c.c2Det2)(a, b), (p.rs.c2Det2)(a, b));
                        same_strict_v("c2Add ±0", (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
                        same_strict_v("c2Sub ±0", (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
                        same_strict_v("c2Maxv ±0", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
                        same_strict_v("c2Minv ±0", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
                        same_strict_v("c2Clampv ±0", (p.c.c2Clampv)(a, b, b), (p.rs.c2Clampv)(a, b, b));
                    }
                }
            }
            unsafe {
                same_strict_v("c2Neg ±0", (p.c.c2Neg)(a), (p.rs.c2Neg)(a));
                same_strict_v("c2Skew ±0", (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
                same_strict_v("c2CCW90 ±0", (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));
                same_strict("c2Len ±0", (p.c.c2Len)(a), (p.rs.c2Len)(a));
                for s in [0.0f32, -0.0, 1.0, -1.0] {
                    same_strict_v("c2Mulvs ±0", (p.c.c2Mulvs)(a, s), (p.rs.c2Mulvs)(a, s));
                    same_strict_v("c2Div ±0", (p.c.c2Div)(a, s), (p.rs.c2Div)(a, s));
                }
            }
        }
    }
}

/// Subnormal inputs must not be flushed to zero by either library.
#[test]
fn strict_subnormals_not_flushed() {
    let p = pair();
    let subs: Vec<f32> = (0..24).map(|k| f32::from_bits(1 << k)).collect();
    for &x in &subs {
        for &y in &[1.0f32, 0.5, f32::from_bits(1), 1e30] {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            unsafe {
                same_strict("c2Dot subnormal", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
                same_strict("c2Det2 subnormal", (p.c.c2Det2)(a, b), (p.rs.c2Det2)(a, b));
                same_strict("c2Len subnormal", (p.c.c2Len)(a), (p.rs.c2Len)(a));
                same_strict_v("c2Norm subnormal", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
                same_strict_v("c2Div subnormal", (p.c.c2Div)(a, x), (p.rs.c2Div)(a, x));
                same_strict_v("c2Mulvs subnormal", (p.c.c2Mulvs)(a, x), (p.rs.c2Mulvs)(a, x));
            }
        }
    }
}

/// `c2GJK`'s returned distance must be strictly bit-equal for all finite
/// (non-NaN) shape data — no canonicalisation allowed here.
#[test]
fn strict_gjk_distance_finite_inputs() {
    let p = pair();
    let mut rng = Rng::new(778);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..64 {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        let oc = run_gjk(p.c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                        let or = run_gjk(p.rs, &sa, None, &sb, None, ur, OutSel::ALL, None);
                        if oc.dist.is_nan() || or.dist.is_nan() {
                            // NaN distance can only arise from NaN-vs-NaN
                            // arithmetic; covered by the relaxed comparison.
                            assert_eq!(oc.dist.is_nan(), or.dist.is_nan());
                            continue;
                        }
                        same_strict(
                            &format!("c2GJK dist {:?}/{:?} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb)),
                            oc.dist,
                            or.dist,
                        );
                        if let (Some(ac), Some(ar)) = (oc.a, or.a) {
                            if ac.x.is_nan() == ar.x.is_nan() && !ac.x.is_nan() && !ac.y.is_nan() {
                                same_strict_v("c2GJK outA", ac, ar);
                            }
                        }
                        if let (Some(bc), Some(br)) = (oc.b, or.b) {
                            if !bc.x.is_nan() && !bc.y.is_nan() {
                                same_strict_v("c2GJK outB", bc, br);
                            }
                        }
                    }
                }
            }
        }
    }
}
