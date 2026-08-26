//! Phase B — CONFIGS.md rows 1-13: the pure vector/scalar helpers.
//!
//! Every call goes through `dlsym` on both `.so`s; results are compared
//! bit-for-bit (NaN payloads and the sign of zero included).

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;

const N: usize = 200_000;

/// Row 1 — `c2V`
#[test]
fn row01_c2V() {
    let l = libs();
    let (c, r) = l.get::<FnV>("c2V");
    let mut g = Rng::new(0x0101);
    for i in 0..N {
        let (x, y) = (g.mixed(), g.mixed());
        let (cv, rv) = unsafe { (c(x, y), r(x, y)) };
        ck_v!("c2V", cv, rv, "i={i} x={x:?} y={y:?}");
    }
    // exhaustive-ish over raw bit patterns too
    let mut g = Rng::new(0x0102);
    for i in 0..N {
        let (x, y) = (g.any_f32(), g.any_f32());
        let (cv, rv) = unsafe { (c(x, y), r(x, y)) };
        ck_v!("c2V/anybits", cv, rv, "i={i} xb={:#010x}", x.to_bits());
    }
}

/// Row 2 — `c2Sub`, `c2Add`
#[test]
fn row02_add_sub() {
    let l = libs();
    let (cadd, radd) = l.get::<FnVVV>("c2Add");
    let (csub, rsub) = l.get::<FnVVV>("c2Sub");
    let mut g = Rng::new(0x0201);
    for i in 0..N {
        let (a, b) = (g.v_mixed(), g.v_mixed());
        ck_v!("c2Add", unsafe { cadd(a, b) }, unsafe { radd(a, b) }, "i={i} a={a:?} b={b:?}");
        ck_v!("c2Sub", unsafe { csub(a, b) }, unsafe { rsub(a, b) }, "i={i} a={a:?} b={b:?}");
    }
    let mut g = Rng::new(0x0202);
    for i in 0..N {
        let (a, b) = (g.v_any(), g.v_any());
        ck_v!("c2Add/anybits", unsafe { cadd(a, b) }, unsafe { radd(a, b) }, "i={i} a={a:?} b={b:?}");
        ck_v!("c2Sub/anybits", unsafe { csub(a, b) }, unsafe { rsub(a, b) }, "i={i} a={a:?} b={b:?}");
    }
}

/// Row 3 — `c2Mulvs`, `c2Div`
#[test]
fn row03_mulvs_div() {
    let l = libs();
    let (cm, rm) = l.get::<FnVsV>("c2Mulvs");
    let (cd, rd) = l.get::<FnVsV>("c2Div");
    let mut g = Rng::new(0x0301);
    for i in 0..N {
        let a = g.v_mixed();
        let s = g.mixed();
        ck_v!("c2Mulvs", unsafe { cm(a, s) }, unsafe { rm(a, s) }, "i={i} a={a:?} s={s:?}");
        ck_v!("c2Div", unsafe { cd(a, s) }, unsafe { rd(a, s) }, "i={i} a={a:?} s={s:?}");
    }
    let mut g = Rng::new(0x0302);
    for i in 0..N {
        let a = g.v_any();
        let s = g.any_f32();
        ck_v!("c2Mulvs/anybits", unsafe { cm(a, s) }, unsafe { rm(a, s) }, "i={i} a={a:?} s={s:?}");
        ck_v!("c2Div/anybits", unsafe { cd(a, s) }, unsafe { rd(a, s) }, "i={i} a={a:?} s={s:?}");
    }
}

/// Row 4 — `c2Dot`
#[test]
fn row04_dot() {
    let l = libs();
    let (c, r) = l.get::<FnVVf>("c2Dot");
    let mut g = Rng::new(0x0401);
    for i in 0..N {
        let (a, b) = (g.v_mixed(), g.v_mixed());
        ck_f32!("c2Dot", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
    }
    let mut g = Rng::new(0x0402);
    for i in 0..N {
        let (a, b) = (g.v_any(), g.v_any());
        ck_f32!("c2Dot/anybits", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
    }
    // exact cancellation: a.x*b.x == -(a.y*b.y)
    let mut g = Rng::new(0x0403);
    for i in 0..20_000 {
        let t = g.coord();
        let a = V::new(t, t);
        let b = V::new(1.0, -1.0);
        ck_f32!("c2Dot/cancel", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} t={t:?}");
        // Inf * 0
        let a2 = V::new(f32::INFINITY, 0.0);
        let b2 = V::new(0.0, f32::INFINITY);
        ck_f32!("c2Dot/inf0", unsafe { c(a2, b2) }, unsafe { r(a2, b2) }, "i={i}");
    }
}

/// Row 5 — `c2Det2`
#[test]
fn row05_det2() {
    let l = libs();
    let (c, r) = l.get::<FnVVf>("c2Det2");
    let mut g = Rng::new(0x0501);
    for i in 0..N {
        let (a, b) = (g.v_mixed(), g.v_mixed());
        ck_f32!("c2Det2", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
    }
    let mut g = Rng::new(0x0502);
    for i in 0..N {
        let (a, b) = (g.v_any(), g.v_any());
        ck_f32!("c2Det2/anybits", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
    }
    // collinear -> exactly zero area
    let mut g = Rng::new(0x0503);
    for i in 0..20_000 {
        let a = g.v_grid();
        let k = g.grid();
        let b = V::new(a.x * k, a.y * k);
        ck_f32!("c2Det2/collinear", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} k={k:?}");
    }
}

/// Row 6 — `c2Len`, `c2Norm`
#[test]
fn row06_len_norm() {
    let l = libs();
    let (cl, rl) = l.get::<FnVf>("c2Len");
    let (cn, rn) = l.get::<FnVV>("c2Norm");
    let mut g = Rng::new(0x0601);
    for i in 0..N {
        let a = g.v_mixed();
        ck_f32!("c2Len", unsafe { cl(a) }, unsafe { rl(a) }, "i={i} a={a:?}");
        ck_v!("c2Norm", unsafe { cn(a) }, unsafe { rn(a) }, "i={i} a={a:?}");
    }
    let mut g = Rng::new(0x0602);
    for i in 0..N {
        let a = g.v_any();
        ck_f32!("c2Len/anybits", unsafe { cl(a) }, unsafe { rl(a) }, "i={i} a={a:?}");
        ck_v!("c2Norm/anybits", unsafe { cn(a) }, unsafe { rn(a) }, "i={i} a={a:?}");
    }
    // every edge-value pair
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = V::new(x, y);
            ck_f32!("c2Len/edge", unsafe { cl(a) }, unsafe { rl(a) }, "a={a:?}");
            ck_v!("c2Norm/edge", unsafe { cn(a) }, unsafe { rn(a) }, "a={a:?}");
        }
    }
}

/// Row 7 — `c2Neg`, `c2Skew`, `c2CCW90`
#[test]
fn row07_neg_skew_ccw() {
    let l = libs();
    let (cneg, rneg) = l.get::<FnVV>("c2Neg");
    let (csk, rsk) = l.get::<FnVV>("c2Skew");
    let (cc, rc) = l.get::<FnVV>("c2CCW90");
    let mut g = Rng::new(0x0701);
    for i in 0..N {
        let a = if i % 2 == 0 { g.v_mixed() } else { g.v_any() };
        ck_v!("c2Neg", unsafe { cneg(a) }, unsafe { rneg(a) }, "i={i} a={a:?}");
        ck_v!("c2Skew", unsafe { csk(a) }, unsafe { rsk(a) }, "i={i} a={a:?}");
        ck_v!("c2CCW90", unsafe { cc(a) }, unsafe { rc(a) }, "i={i} a={a:?}");
    }
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = V::new(x, y);
            ck_v!("c2Neg/edge", unsafe { cneg(a) }, unsafe { rneg(a) }, "a={a:?}");
            ck_v!("c2Skew/edge", unsafe { csk(a) }, unsafe { rsk(a) }, "a={a:?}");
            ck_v!("c2CCW90/edge", unsafe { cc(a) }, unsafe { rc(a) }, "a={a:?}");
        }
    }
}

/// Row 8 — `c2Maxv`, `c2Minv`
#[test]
fn row08_maxv_minv() {
    let l = libs();
    let (cx, rx) = l.get::<FnVVV>("c2Maxv");
    let (cn, rn) = l.get::<FnVVV>("c2Minv");
    let mut g = Rng::new(0x0801);
    for i in 0..N {
        let (a, b) = if i % 3 == 0 {
            (g.v_any(), g.v_any())
        } else {
            (g.v_mixed(), g.v_mixed())
        };
        ck_v!("c2Maxv", unsafe { cx(a, b) }, unsafe { rx(a, b) }, "i={i} a={a:?} b={b:?}");
        ck_v!("c2Minv", unsafe { cn(a, b) }, unsafe { rn(a, b) }, "i={i} a={a:?} b={b:?}");
        // equal operands, and +0 vs -0
        ck_v!("c2Maxv/eq", unsafe { cx(a, a) }, unsafe { rx(a, a) }, "i={i} a={a:?}");
        ck_v!("c2Minv/eq", unsafe { cn(a, a) }, unsafe { rn(a, a) }, "i={i} a={a:?}");
    }
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = V::new(x, y);
            let b = V::new(y, x);
            ck_v!("c2Maxv/edge", unsafe { cx(a, b) }, unsafe { rx(a, b) }, "a={a:?} b={b:?}");
            ck_v!("c2Minv/edge", unsafe { cn(a, b) }, unsafe { rn(a, b) }, "a={a:?} b={b:?}");
        }
    }
}

/// Row 9 — `c2Clampv`
#[test]
fn row09_clampv() {
    let l = libs();
    let (c, r) = l.get::<FnVVVV>("c2Clampv");
    let mut g = Rng::new(0x0901);
    for i in 0..N {
        let a = g.v_mixed();
        let (lo, hi) = match i % 4 {
            0 => {
                let p = g.v_coord();
                let q = g.v_coord();
                (
                    V::new(p.x.min(q.x), p.y.min(q.y)),
                    V::new(p.x.max(q.x), p.y.max(q.y)),
                )
            }
            1 => {
                let p = g.v_coord();
                (p, p) // lo == hi
            }
            2 => {
                let p = g.v_coord();
                let q = g.v_coord();
                (
                    V::new(p.x.max(q.x), p.y.max(q.y)),
                    V::new(p.x.min(q.x), p.y.min(q.y)),
                ) // inverted
            }
            _ => (g.v_mixed(), g.v_mixed()), // arbitrary, incl. NaN bounds
        };
        ck_v!("c2Clampv", unsafe { c(a, lo, hi) }, unsafe { r(a, lo, hi) },
              "i={i} a={a:?} lo={lo:?} hi={hi:?}");
    }
}

/// Row 10 — `c2RotIdentity`, `c2xIdentity` (constants, struct-return ABI)
#[test]
fn row10_identities() {
    let l = libs();
    let (cr, rr) = l.get::<FnR>("c2RotIdentity");
    let (cx, rx) = l.get::<FnX>("c2xIdentity");
    for i in 0..1000 {
        let (a, b) = unsafe { (cr(), rr()) };
        ck_bytes!("c2RotIdentity", a, b, "i={i}");
        let (a, b) = unsafe { (cx(), rx()) };
        ck_bytes!("c2xIdentity", a, b, "i={i}");
    }
}

/// Row 11 — `c2Mulrv`, `c2MulrvT`
#[test]
fn row11_mulrv() {
    let l = libs();
    let (c, r) = l.get::<FnRVV>("c2Mulrv");
    let (ct, rt) = l.get::<FnRVV>("c2MulrvT");
    let mut g = Rng::new(0x1101);
    for i in 0..N {
        let rot = g.rot();
        let v = g.v_mixed();
        ck_v!("c2Mulrv", unsafe { c(rot, v) }, unsafe { r(rot, v) }, "i={i} rot={rot:?} v={v:?}");
        ck_v!("c2MulrvT", unsafe { ct(rot, v) }, unsafe { rt(rot, v) }, "i={i} rot={rot:?} v={v:?}");
    }
    let mut g = Rng::new(0x1102);
    for i in 0..50_000 {
        let rot = R { c: g.any_f32(), s: g.any_f32() };
        let v = g.v_any();
        ck_v!("c2Mulrv/anybits", unsafe { c(rot, v) }, unsafe { r(rot, v) }, "i={i} rot={rot:?} v={v:?}");
        ck_v!("c2MulrvT/anybits", unsafe { ct(rot, v) }, unsafe { rt(rot, v) }, "i={i} rot={rot:?} v={v:?}");
    }
}

/// Row 12 — composed round trip `c2MulrvT(r, c2Mulrv(r, v))` through both libs
#[test]
fn row12_mulrv_roundtrip() {
    let l = libs();
    let (c, r) = l.get::<FnRVV>("c2Mulrv");
    let (ct, rt) = l.get::<FnRVV>("c2MulrvT");
    let mut g = Rng::new(0x1201);
    for i in 0..100_000 {
        let rot = g.rot();
        let v = g.v_mixed();
        let cout = unsafe { ct(rot, c(rot, v)) };
        let rout = unsafe { rt(rot, r(rot, v)) };
        ck_v!("mulrv roundtrip", cout, rout, "i={i} rot={rot:?} v={v:?}");
    }
}

/// Row 13 — `c2Mulxv` over all four transform modes
#[test]
fn row13_mulxv() {
    let l = libs();
    let (c, r) = l.get::<FnXVV>("c2Mulxv");
    let mut g = Rng::new(0x1301);
    for i in 0..N {
        let mode = (i % 4) as u32;
        let x = g.xform(mode);
        let v = g.v_mixed();
        ck_v!("c2Mulxv", unsafe { c(x, v) }, unsafe { r(x, v) }, "i={i} mode={mode} x={x:?} v={v:?}");
    }
    let mut g = Rng::new(0x1302);
    for i in 0..50_000 {
        let x = X {
            p: g.v_any(),
            r: R { c: g.any_f32(), s: g.any_f32() },
        };
        let v = g.v_any();
        ck_v!("c2Mulxv/anybits", unsafe { c(x, v) }, unsafe { r(x, v) }, "i={i} x={x:?} v={v:?}");
    }
}
