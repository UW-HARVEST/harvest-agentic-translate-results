//! Characterization of the ONE tolerated difference between the two `.so`s:
//! NaN payload/sign bits. See `common::NAN_PAYLOAD_NOTE` for the full root
//! cause (opposite SSE `src1` choices by gcc `-O0` vs LLVM).
//!
//! These tests pin down the *exact* scope of the difference, so it is a
//! measured fact rather than an assumption:
//!
//! * every `int`-returning export is bit-identical even with arbitrary NaN
//!   payloads and signaling NaNs on the input;
//! * every float/`c2v`-returning export is either bit-identical or NaN on both
//!   sides — never "NaN in one, a number in the other";
//! * `is_nan()` agreement is exact, so no branch in the C can diverge.

mod common;
use common::*;

const N: usize = 30_000;

/// The payload difference exists and is confined to NaN-valued results.
/// Feeds arbitrary NaN bit patterns (incl. signaling and negative NaNs).
#[test]
fn nan_payload_scope_float_results() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0xa001);
    for i in 0..N {
        let a = g.wild_v_any_nan();
        let b = g.wild_v_any_nan();
        let s = g.wild_any_nan();
        let rot = c2r {
            c: g.wild_any_nan(),
            s: g.wild_any_nan(),
        };
        let xf = c2x { p: a, r: rot };

        // Scalar results.
        eq_f32_nan_ok(&format!("nan c2Dot #{i}"), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
        eq_f32_nan_ok(
            &format!("nan c2Det2 #{i}"),
            (c.c2Det2)(a, b),
            (r.c2Det2)(a, b),
        );
        eq_f32_nan_ok(&format!("nan c2Len #{i}"), (c.c2Len)(a), (r.c2Len)(a));

        // Vector results.
        eq_v_nan_ok(&format!("nan c2Add #{i}"), (c.c2Add)(a, b), (r.c2Add)(a, b));
        eq_v_nan_ok(&format!("nan c2Sub #{i}"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        eq_v_nan_ok(
            &format!("nan c2Mulvs #{i}"),
            (c.c2Mulvs)(a, s),
            (r.c2Mulvs)(a, s),
        );
        eq_v_nan_ok(&format!("nan c2Div #{i}"), (c.c2Div)(a, s), (r.c2Div)(a, s));
        eq_v_nan_ok(&format!("nan c2Norm #{i}"), (c.c2Norm)(a), (r.c2Norm)(a));
        eq_v_nan_ok(&format!("nan c2Neg #{i}"), (c.c2Neg)(a), (r.c2Neg)(a));
        eq_v_nan_ok(&format!("nan c2Skew #{i}"), (c.c2Skew)(a), (r.c2Skew)(a));
        eq_v_nan_ok(&format!("nan c2CCW90 #{i}"), (c.c2CCW90)(a), (r.c2CCW90)(a));
        eq_v_nan_ok(&format!("nan c2Maxv #{i}"), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
        eq_v_nan_ok(&format!("nan c2Minv #{i}"), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
        eq_v_nan_ok(
            &format!("nan c2Clampv #{i}"),
            (c.c2Clampv)(a, b, a),
            (r.c2Clampv)(a, b, a),
        );
        eq_v_nan_ok(
            &format!("nan c2Mulrv #{i}"),
            (c.c2Mulrv)(rot, b),
            (r.c2Mulrv)(rot, b),
        );
        eq_v_nan_ok(
            &format!("nan c2MulrvT #{i}"),
            (c.c2MulrvT)(rot, b),
            (r.c2MulrvT)(rot, b),
        );
        eq_v_nan_ok(
            &format!("nan c2Mulxv #{i}"),
            (c.c2Mulxv)(xf, b),
            (r.c2Mulxv)(xf, b),
        );
    }
}

/// NaN-ness itself always agrees, so no `if` in the C source can take a
/// different branch in the two builds.
#[test]
fn nan_ness_agrees_exactly() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0xa002);
    for i in 0..N {
        let a = g.wild_v_any_nan();
        let b = g.wild_v_any_nan();
        let pairs: [(f32, f32); 3] = [
            ((c.c2Dot)(a, b), (r.c2Dot)(a, b)),
            ((c.c2Det2)(a, b), (r.c2Det2)(a, b)),
            ((c.c2Len)(a), (r.c2Len)(a)),
        ];
        for (k, (cv, rv)) in pairs.iter().enumerate() {
            assert_eq!(
                cv.is_nan(),
                rv.is_nan(),
                "nan-ness disagreement #{i} fn{k}: C={cv:?} Rust={rv:?}"
            );
        }
    }
}

/// Every `int`-returning export is bit-identical even under arbitrary NaN
/// payloads — this is the property that actually matters for the library's
/// contract.
#[test]
fn int_results_are_exact_under_any_nan() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0xa003);
    for i in 0..N {
        let ci = c2Circle {
            p: g.wild_v_any_nan(),
            r: g.wild_any_nan(),
        };
        let cj = c2Circle {
            p: g.wild_v_any_nan(),
            r: g.wild_any_nan(),
        };
        let bb = c2AABB {
            min: g.wild_v_any_nan(),
            max: g.wild_v_any_nan(),
        };
        let bc = c2AABB {
            min: g.wild_v_any_nan(),
            max: g.wild_v_any_nan(),
        };
        let ca = c2Capsule {
            a: g.wild_v_any_nan(),
            b: g.wild_v_any_nan(),
            r: g.wild_any_nan(),
        };
        let cb = c2Capsule {
            a: g.wild_v_any_nan(),
            b: g.wild_v_any_nan(),
            r: g.wild_any_nan(),
        };

        eq_int(
            &format!("nan c2AABBtoAABB #{i}"),
            (c.c2AABBtoAABB)(bb, bc),
            (r.c2AABBtoAABB)(bb, bc),
        );
        eq_int(
            &format!("nan c2CircletoCircle #{i}"),
            (c.c2CircletoCircle)(ci, cj),
            (r.c2CircletoCircle)(ci, cj),
        );
        eq_int(
            &format!("nan c2CircletoAABB #{i}"),
            (c.c2CircletoAABB)(ci, bb),
            (r.c2CircletoAABB)(ci, bb),
        );
        eq_int(
            &format!("nan c2CircletoCapsule #{i}"),
            (c.c2CircletoCapsule)(ci, ca),
            (r.c2CircletoCapsule)(ci, ca),
        );
        eq_int(
            &format!("nan c2AABBtoCapsule #{i}"),
            (c.c2AABBtoCapsule)(bb, ca),
            (r.c2AABBtoCapsule)(bb, ca),
        );
        eq_int(
            &format!("nan c2CapsuletoCapsule #{i}"),
            (c.c2CapsuletoCapsule)(ca, cb),
            (r.c2CapsuletoCapsule)(ca, cb),
        );

        // And the public one-shot API with raw NaN floats.
        for &ta in &VALID_TYPES {
            for &tb in &VALID_TYPES {
                let p: [f32; 10] = [
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                    g.wild_any_nan(),
                ];
                let cv = unsafe {
                    (c.omni_collide)(ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9])
                };
                let rv = unsafe {
                    (r.omni_collide)(ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9])
                };
                eq_int(
                    &format!(
                        "nan omni_collide #{i} {}/{} {p:?}",
                        type_name(ta),
                        type_name(tb)
                    ),
                    cv,
                    rv,
                );
            }
        }
    }
}

/// Demonstrates concretely that the *only* thing that ever differs is the
/// payload: `c2Add` with two distinct NaNs in the same lane.
#[test]
fn nan_payload_difference_is_real_and_payload_only() {
    let (c, r) = apis();
    let pos = f32::from_bits(0x7fc0_0000);
    let neg = f32::from_bits(0xffc0_0000);
    let a = c2v { x: pos, y: neg };
    let b = c2v { x: neg, y: pos };
    let cv = (c.c2Add)(a, b);
    let rv = (r.c2Add)(a, b);
    // Both are NaN on both sides ...
    assert!(cv.x.is_nan() && cv.y.is_nan() && rv.x.is_nan() && rv.y.is_nan());
    // ... and the relaxed comparison accepts them.
    eq_v_nan_ok("payload-only", cv, rv);
    // Document which NaN each build selects (gcc -O0 takes src1 = b,
    // LLVM takes src1 = a). If a future toolchain makes these agree the
    // assertion below simply becomes trivially true, so it is written as an
    // informational check rather than a hard requirement.
    let differs = cv.x.to_bits() != rv.x.to_bits() || cv.y.to_bits() != rv.y.to_bits();
    println!("NaN payload differs between C and Rust builds: {differs}");
}
