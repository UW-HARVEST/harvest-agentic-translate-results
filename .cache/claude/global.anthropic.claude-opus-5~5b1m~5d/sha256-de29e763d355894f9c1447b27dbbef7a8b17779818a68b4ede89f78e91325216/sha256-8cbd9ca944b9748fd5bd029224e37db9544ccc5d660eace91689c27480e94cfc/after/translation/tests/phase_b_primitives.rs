//! Phase B — CONFIGS.md rows 1..14: the scalar / vector primitives.
//!
//! Every function is reached through `dlopen` + `dlsym` on BOTH shared objects;
//! results are compared bit-for-bit (`f32::to_bits`).

mod common;
use common::*;

const N: usize = 4000;

// ---------------------------------------------------------------------- row 1
#[test]
fn row01_c2V() {
    let p = apis();
    let mut rng = Rng::new(0x0101);
    for _ in 0..n_cases(N) {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        unsafe {
            eq_v("c2V", &(x, y), (p.c.c2V)(x, y), (p.r.c2V)(x, y));
        }
    }
    // exhaustive over the special table
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            unsafe {
                eq_v("c2V/special", &(x, y), (p.c.c2V)(x, y), (p.r.c2V)(x, y));
            }
        }
    }
}

// ---------------------------------------------------------------------- row 2
#[test]
fn row02_c2Mulvs() {
    let p = apis();
    let mut rng = Rng::new(0x0202);
    for _ in 0..n_cases(N) {
        let a = rng.any_v();
        let b = rng.any_f32();
        unsafe {
            eq_v("c2Mulvs", &(a, b), (p.c.c2Mulvs)(a, b), (p.r.c2Mulvs)(a, b));
        }
    }
    for &x in SPECIAL_F32 {
        for &s in SPECIAL_F32 {
            let a = c2v { x, y: -x };
            unsafe {
                eq_v(
                    "c2Mulvs/special",
                    &(a, s),
                    (p.c.c2Mulvs)(a, s),
                    (p.r.c2Mulvs)(a, s),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------- row 3
#[test]
fn row03_c2Maxv_c2Minv() {
    let p = apis();
    let mut rng = Rng::new(0x0303);
    for _ in 0..n_cases(N) {
        let (a, b) = (rng.any_v(), rng.any_v());
        unsafe {
            eq_v("c2Maxv", &(a, b), (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
            eq_v("c2Minv", &(a, b), (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
            // reversed argument order (the ternaries are not symmetric on ties/NaN)
            eq_v("c2Maxv/rev", &(b, a), (p.c.c2Maxv)(b, a), (p.r.c2Maxv)(b, a));
            eq_v("c2Minv/rev", &(b, a), (p.c.c2Minv)(b, a), (p.r.c2Minv)(b, a));
        }
    }
    // exhaustive special x special, incl. ties, +-0.0 and NaN in either operand
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            unsafe {
                eq_v("c2Maxv/sp", &(a, b), (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
                eq_v("c2Minv/sp", &(a, b), (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
            }
        }
    }
}

// ---------------------------------------------------------------------- row 4
#[test]
fn row04_c2Clampv() {
    let p = apis();
    let mut rng = Rng::new(0x0404);
    for _ in 0..n_cases(N) {
        let a = rng.any_v();
        let bb = rng.aabb();
        unsafe {
            eq_v(
                "c2Clampv",
                &(a, bb),
                (p.c.c2Clampv)(a, bb.min, bb.max),
                (p.r.c2Clampv)(a, bb.min, bb.max),
            );
            // deliberately inverted box
            eq_v(
                "c2Clampv/inv",
                &(a, bb),
                (p.c.c2Clampv)(a, bb.max, bb.min),
                (p.r.c2Clampv)(a, bb.max, bb.min),
            );
        }
    }
    for &s in SPECIAL_F32 {
        let a = c2v { x: s, y: s };
        for &lo in SPECIAL_F32 {
            let l = c2v { x: lo, y: -lo };
            let h = c2v { x: -lo, y: lo };
            unsafe {
                eq_v(
                    "c2Clampv/sp",
                    &(a, l, h),
                    (p.c.c2Clampv)(a, l, h),
                    (p.r.c2Clampv)(a, l, h),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------- row 5
#[test]
fn row05_c2Sub_c2Add() {
    let p = apis();
    let mut rng = Rng::new(0x0505);
    for _ in 0..n_cases(N) {
        let (a, b) = (rng.any_v(), rng.any_v());
        unsafe {
            eq_v("c2Sub", &(a, b), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
            eq_v("c2Add", &(a, b), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
            eq_v("c2Sub/rev", &(b, a), (p.c.c2Sub)(b, a), (p.r.c2Sub)(b, a));
            eq_v("c2Add/rev", &(b, a), (p.c.c2Add)(b, a), (p.r.c2Add)(b, a));
        }
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            unsafe {
                eq_v("c2Sub/sp", &(a, b), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
                eq_v("c2Add/sp", &(a, b), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
                // exact cancellation -> sign of zero
                eq_v("c2Sub/self", &a, (p.c.c2Sub)(a, a), (p.r.c2Sub)(a, a));
            }
        }
    }
}

// ------------------------------------------------------------------ rows 6, 7
#[test]
fn row06_row07_c2Dot_c2Det2() {
    let p = apis();
    let mut rng = Rng::new(0x0607);
    for _ in 0..n_cases(N) {
        let (a, b) = (rng.any_v(), rng.any_v());
        unsafe {
            eq_f32("c2Dot", &(a, b), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
            eq_f32("c2Dot/rev", &(b, a), (p.c.c2Dot)(b, a), (p.r.c2Dot)(b, a));
            eq_f32("c2Det2", &(a, b), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
            eq_f32("c2Det2/rev", &(b, a), (p.c.c2Det2)(b, a), (p.r.c2Det2)(b, a));
            // parallel / antiparallel -> det == +-0
            let k = rng.f32_in(-3.0, 3.0);
            let ap = c2v { x: a.x * k, y: a.y * k };
            eq_f32("c2Det2/par", &(a, ap), (p.c.c2Det2)(a, ap), (p.r.c2Det2)(a, ap));
        }
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            unsafe {
                eq_f32("c2Dot/sp", &(a, b), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
                eq_f32("c2Dot/self", &a, (p.c.c2Dot)(a, a), (p.r.c2Dot)(a, a));
                eq_f32("c2Det2/sp", &(a, b), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
                eq_f32("c2Det2/self", &a, (p.c.c2Det2)(a, a), (p.r.c2Det2)(a, a));
            }
        }
    }
}

// ---------------------------------------------------------------------- row 8
#[test]
fn row08_c2Len() {
    let p = apis();
    let mut rng = Rng::new(0x0808);
    for _ in 0..n_cases(N) {
        let a = rng.any_v();
        unsafe {
            eq_f32("c2Len", &a, (p.c.c2Len)(a), (p.r.c2Len)(a));
        }
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            unsafe {
                eq_f32("c2Len/sp", &a, (p.c.c2Len)(a), (p.r.c2Len)(a));
            }
        }
    }
}

// ---------------------------------------------------------------------- row 9
#[test]
fn row09_c2Div_c2Norm() {
    let p = apis();
    let mut rng = Rng::new(0x0909);
    for _ in 0..n_cases(N) {
        let a = rng.any_v();
        let b = rng.any_f32();
        unsafe {
            eq_v("c2Div", &(a, b), (p.c.c2Div)(a, b), (p.r.c2Div)(a, b));
            eq_v("c2Norm", &a, (p.c.c2Norm)(a), (p.r.c2Norm)(a));
        }
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            unsafe {
                eq_v("c2Norm/sp", &a, (p.c.c2Norm)(a), (p.r.c2Norm)(a));
                eq_v("c2Div/sp", &(a, y), (p.c.c2Div)(a, y), (p.r.c2Div)(a, y));
            }
        }
    }
    // zero vector -> 1/0 == inf, 0*inf == NaN
    let z = c2v { x: 0.0, y: 0.0 };
    unsafe {
        eq_v("c2Norm/zero", &z, (p.c.c2Norm)(z), (p.r.c2Norm)(z));
        eq_v("c2Div/zero", &z, (p.c.c2Div)(z, 0.0), (p.r.c2Div)(z, 0.0));
        eq_v("c2Div/-zero", &z, (p.c.c2Div)(z, -0.0), (p.r.c2Div)(z, -0.0));
    }
}

// --------------------------------------------------------------------- row 10
#[test]
fn row10_c2Neg_c2Skew_c2CCW90() {
    let p = apis();
    let mut rng = Rng::new(0x1010);
    for _ in 0..n_cases(N) {
        let a = rng.any_v();
        unsafe {
            eq_v("c2Neg", &a, (p.c.c2Neg)(a), (p.r.c2Neg)(a));
            eq_v("c2Skew", &a, (p.c.c2Skew)(a), (p.r.c2Skew)(a));
            eq_v("c2CCW90", &a, (p.c.c2CCW90)(a), (p.r.c2CCW90)(a));
        }
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            unsafe {
                eq_v("c2Neg/sp", &a, (p.c.c2Neg)(a), (p.r.c2Neg)(a));
                eq_v("c2Skew/sp", &a, (p.c.c2Skew)(a), (p.r.c2Skew)(a));
                eq_v("c2CCW90/sp", &a, (p.c.c2CCW90)(a), (p.r.c2CCW90)(a));
            }
        }
    }
}

// --------------------------------------------------------------------- row 11
#[test]
fn row11_identities() {
    let p = apis();
    unsafe {
        eq_r(
            "c2RotIdentity",
            &(),
            (p.c.c2RotIdentity)(),
            (p.r.c2RotIdentity)(),
        );
        eq_x("c2xIdentity", &(), (p.c.c2xIdentity)(), (p.r.c2xIdentity)());
        // and byte-compare the whole returned structs
        let (cx, rx) = ((p.c.c2xIdentity)(), (p.r.c2xIdentity)());
        eq_bytes("c2xIdentity/bytes", &(), &cx, &rx);
    }
}

// --------------------------------------------------------------------- row 12
#[test]
fn row12_c2Mulrv_c2MulrvT() {
    let p = apis();
    let mut rng = Rng::new(0x1212);
    for _ in 0..n_cases(N) {
        let r = rng.rot();
        let v = rng.any_v();
        unsafe {
            eq_v("c2Mulrv", &(r, v), (p.c.c2Mulrv)(r, v), (p.r.c2Mulrv)(r, v));
            eq_v(
                "c2MulrvT",
                &(r, v),
                (p.c.c2MulrvT)(r, v),
                (p.r.c2MulrvT)(r, v),
            );
        }
    }
    // NaN / Inf in the rotation itself
    for &c in SPECIAL_F32 {
        for &s in SPECIAL_F32 {
            let r = c2r { c, s };
            let v = c2v { x: 3.0, y: -7.5 };
            unsafe {
                eq_v("c2Mulrv/sp", &(r, v), (p.c.c2Mulrv)(r, v), (p.r.c2Mulrv)(r, v));
                eq_v(
                    "c2MulrvT/sp",
                    &(r, v),
                    (p.c.c2MulrvT)(r, v),
                    (p.r.c2MulrvT)(r, v),
                );
            }
            let v2 = c2v { x: 0.0, y: -0.0 };
            unsafe {
                eq_v("c2Mulrv/z", &(r, v2), (p.c.c2Mulrv)(r, v2), (p.r.c2Mulrv)(r, v2));
                eq_v(
                    "c2MulrvT/z",
                    &(r, v2),
                    (p.c.c2MulrvT)(r, v2),
                    (p.r.c2MulrvT)(r, v2),
                );
            }
        }
    }
    // special vector, identity-ish rotation
    let r = c2r { c: 1.0, s: 0.0 };
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let v = c2v { x, y };
            unsafe {
                eq_v("c2Mulrv/id", &(r, v), (p.c.c2Mulrv)(r, v), (p.r.c2Mulrv)(r, v));
                eq_v(
                    "c2MulrvT/id",
                    &(r, v),
                    (p.c.c2MulrvT)(r, v),
                    (p.r.c2MulrvT)(r, v),
                );
            }
        }
    }
}

// --------------------------------------------------------------------- row 13
#[test]
fn row13_c2Mulxv() {
    let p = apis();
    let mut rng = Rng::new(0x1313);
    for _ in 0..n_cases(N) {
        let x = rng.xform();
        let v = rng.any_v();
        unsafe {
            eq_v("c2Mulxv", &(x, v), (p.c.c2Mulxv)(x, v), (p.r.c2Mulxv)(x, v));
        }
    }
    // identity transform
    let idx = unsafe { (p.c.c2xIdentity)() };
    for &a in SPECIAL_F32 {
        for &b in SPECIAL_F32 {
            let v = c2v { x: a, y: b };
            unsafe {
                eq_v(
                    "c2Mulxv/id",
                    &(idx, v),
                    (p.c.c2Mulxv)(idx, v),
                    (p.r.c2Mulxv)(idx, v),
                );
            }
            // pure translation
            let t = c2x {
                p: c2v { x: a, y: b },
                r: c2r { c: 1.0, s: 0.0 },
            };
            let w = c2v { x: 2.5, y: -3.25 };
            unsafe {
                eq_v(
                    "c2Mulxv/trans",
                    &(t, w),
                    (p.c.c2Mulxv)(t, w),
                    (p.r.c2Mulxv)(t, w),
                );
            }
        }
    }
}

// --------------------------------------------------------------------- row 14
#[test]
fn row14_c2BBVerts() {
    let p = apis();
    let mut rng = Rng::new(0x1414);
    for _ in 0..n_cases(N) {
        let mut bbc = rng.aabb();
        let mut bbr = bbc;
        // 8 slots so an accidental overrun would show up
        let mut oc = [c2v { x: 7.0, y: -7.0 }; 8];
        let mut or_ = oc;
        unsafe {
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
            (p.r.c2BBVerts)(or_.as_mut_ptr(), &mut bbr);
        }
        eq_bytes("c2BBVerts/out", &bbc, &oc, &or_);
        eq_bytes("c2BBVerts/in", &bbc, &bbc, &bbr); // input must be untouched
    }
    // degenerate + special corners
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let mut bbc = c2AABB {
                min: c2v { x: s, y: t },
                max: c2v { x: t, y: s },
            };
            let mut bbr = bbc;
            let mut oc = [c2v::default(); 8];
            let mut or_ = oc;
            unsafe {
                (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
                (p.r.c2BBVerts)(or_.as_mut_ptr(), &mut bbr);
            }
            eq_bytes("c2BBVerts/sp", &bbc, &oc, &or_);
        }
    }
}
