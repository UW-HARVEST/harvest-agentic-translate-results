//! Phase B, NaN-payload parity.
//!
//! The C library compiles to SSE scalar arithmetic, whose NaN-propagation rule
//! is *defined*: `MULSS`/`ADDSS`/`SUBSS`/`DIVSS` return the quieted destination
//! operand when it is a NaN, else the quieted source operand.  Which operand
//! gcc places in the destination register is therefore observable across the FFI
//! boundary whenever two operands are different NaNs — and NaNs are perfectly
//! legal `float` inputs.  This file pins that behaviour with the full
//! cross-product of every distinct NaN class (sign x quiet/signalling x payload)
//! against every leaf and composite arithmetic export.

#![allow(non_snake_case)]
mod common;
use common::*;

/// Every NaN payload class that a caller can hand us, paired against itself and
/// against every other, for every leaf arithmetic function.
const NANS: [u32; 8] = [
    0x7fc0_0000, // +qNaN default
    0xffc0_0000, // -qNaN default
    0x7f80_0001, // +sNaN low payload
    0xff80_0001, // -sNaN low payload
    0x7fbf_ffff, // +sNaN high payload
    0xffbf_ffff, // -sNaN high payload
    0x7fff_ffff, // +qNaN all-ones payload
    0xffff_ffff, // -qNaN all-ones payload
];

#[test]
fn bnan1_nan_payload_matrix_leaf_functions() {
    let (c, r) = libs();
    let extra: [u32; 6] = [
        0x0000_0000,
        0x8000_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x7f80_0000,
        0xff80_0000,
    ];
    let mut pool: Vec<f32> = Vec::new();
    for b in NANS.iter().chain(extra.iter()) {
        pool.push(f32::from_bits(*b));
    }

    unsafe {
        for &ax in &pool {
            for &ay in &pool {
                for &bx in &pool {
                    for &by in &pool {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        let ctx = format!(
                            "a=(0x{:08x},0x{:08x}) b=(0x{:08x},0x{:08x})",
                            ax.to_bits(),
                            ay.to_bits(),
                            bx.to_bits(),
                            by.to_bits()
                        );
                        eq_f32(&format!("c2Dot {ctx}"), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                        eq_f32(&format!("c2Det2 {ctx}"), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
                        eq_v(&format!("c2Add {ctx}"), (c.c2Add)(a, b), (r.c2Add)(a, b));
                        eq_v(&format!("c2Sub {ctx}"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                        eq_v(&format!("c2Maxv {ctx}"), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                        eq_v(&format!("c2Minv {ctx}"), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                        let rr = c2r { c: ax, s: ay };
                        eq_v(
                            &format!("c2Mulrv {ctx}"),
                            (c.c2Mulrv)(rr, b),
                            (r.c2Mulrv)(rr, b),
                        );
                        eq_v(
                            &format!("c2MulrvT {ctx}"),
                            (c.c2MulrvT)(rr, b),
                            (r.c2MulrvT)(rr, b),
                        );
                        eq_v(
                            &format!("c2Mulvs {ctx}"),
                            (c.c2Mulvs)(a, bx),
                            (r.c2Mulvs)(a, bx),
                        );
                        eq_v(&format!("c2Div {ctx}"), (c.c2Div)(a, bx), (r.c2Div)(a, bx));
                    }
                    let a = c2v { x: ax, y: ay };
                    eq_f32(&format!("c2Len {ax:e},{ay:e}"), (c.c2Len)(a), (r.c2Len)(a));
                    eq_v(&format!("c2Norm"), (c.c2Norm)(a), (r.c2Norm)(a));
                    eq_v(&format!("c2Neg"), (c.c2Neg)(a), (r.c2Neg)(a));
                    eq_v(&format!("c2Skew"), (c.c2Skew)(a), (r.c2Skew)(a));
                    eq_v(&format!("c2CCW90"), (c.c2CCW90)(a), (r.c2CCW90)(a));
                    let _ = bx;
                }
            }
        }
    }
}

#[test]
fn bnan2_nan_payload_matrix_composites() {
    let (c, r) = libs();
    let pool: Vec<f32> = NANS
        .iter()
        .map(|b| f32::from_bits(*b))
        .chain([0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY])
        .collect();

    unsafe {
        // c2Clampv / c2Mulxv over the NaN pool
        for &ax in &pool {
            for &ay in &pool {
                for &bx in &pool {
                    let a = c2v { x: ax, y: ay };
                    let lo = c2v { x: bx, y: ay };
                    let hi = c2v { x: ay, y: bx };
                    eq_v("c2Clampv", (c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
                    let x = c2x {
                        p: c2v { x: bx, y: ax },
                        r: c2r { c: ay, s: bx },
                    };
                    eq_v("c2Mulxv", (c.c2Mulxv)(x, a), (r.c2Mulxv)(x, a));
                    // circle/circle and circle/capsule radius sums
                    let ca = c2Circle { p: a, r: ax };
                    let cb = c2Circle { p: lo, r: bx };
                    eq_int(
                        "c2CircletoCircle",
                        (c.c2CircletoCircle)(ca, cb),
                        (r.c2CircletoCircle)(ca, cb),
                    );
                    let cap = c2Capsule {
                        a: lo,
                        b: hi,
                        r: bx,
                    };
                    eq_int(
                        "c2CircletoCapsule",
                        (c.c2CircletoCapsule)(ca, cap),
                        (r.c2CircletoCapsule)(ca, cap),
                    );
                    let bb = c2AABB { min: lo, max: hi };
                    eq_int(
                        "c2CircletoAABB",
                        (c.c2CircletoAABB)(ca, bb),
                        (r.c2CircletoAABB)(ca, bb),
                    );
                    eq_int(
                        "c2AABBtoAABB",
                        (c.c2AABBtoAABB)(bb, bb),
                        (r.c2AABBtoAABB)(bb, bb),
                    );
                }
            }
        }
    }
}
