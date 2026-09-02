//! Phase B — CONFIGS.md rows 1..14: the pure vector-math entry points.
//!
//! These are the lowest level of the call hierarchy, so they are verified first.
//! Every call goes through `dlsym` on both `.so`s; results are compared bit-for-bit.

mod common;
use common::*;

const N: usize = 20_000;

type FnVff = unsafe extern "C" fn(f32, f32) -> C2v;
type FnVvf = unsafe extern "C" fn(C2v, f32) -> C2v;
type FnVvv = unsafe extern "C" fn(C2v, C2v) -> C2v;
type FnVvvv = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type FnFvv = unsafe extern "C" fn(C2v, C2v) -> f32;
type FnFv = unsafe extern "C" fn(C2v) -> f32;
type FnVv = unsafe extern "C" fn(C2v) -> C2v;
type FnR = unsafe extern "C" fn() -> C2r;
type FnX = unsafe extern "C" fn() -> C2x;
type FnVrv = unsafe extern "C" fn(C2r, C2v) -> C2v;
type FnVxv = unsafe extern "C" fn(C2x, C2v) -> C2v;

/// row 1 — c2V
#[test]
fn row01_c2v() {
    let l = libs();
    let (c, r) = l.pair::<FnVff>("c2V");
    let mut rng = Rng::new(0xC2_0001);
    for i in 0..N {
        let (x, y) = if i < SPECIAL.len() * SPECIAL.len() {
            (SPECIAL[i / SPECIAL.len()], SPECIAL[i % SPECIAL.len()])
        } else {
            (rng.f32_mixed(), rng.f32_mixed())
        };
        let (cv, rv) = unsafe { (c(x, y), r(x, y)) };
        same("c2V", &(x, y), &cv, &rv);
    }
}

/// row 2 — c2Mulvs
#[test]
fn row02_c2mulvs() {
    let l = libs();
    let (c, r) = l.pair::<FnVvf>("c2Mulvs");
    let mut rng = Rng::new(0xC2_0002);
    for i in 0..N {
        let a = if i % 3 == 0 { rng.v_any() } else { rng.v_mixed() };
        let s = if i % 4 == 0 {
            SPECIAL[rng.below(SPECIAL.len() as u32) as usize]
        } else {
            rng.f32_mixed()
        };
        let (cv, rv) = unsafe { (c(a, s), r(a, s)) };
        same("c2Mulvs", &(a, s), &cv, &rv);
    }
}

/// row 3 — c2Add / c2Sub
#[test]
fn row03_add_sub() {
    let l = libs();
    let (ca, ra) = l.pair::<FnVvv>("c2Add");
    let (cs, rs) = l.pair::<FnVvv>("c2Sub");
    let mut rng = Rng::new(0xC2_0003);
    for i in 0..N {
        let a = if i % 3 == 0 { rng.v_any() } else { rng.v_mixed() };
        // Every 5th case makes b == a to force exact cancellation / inf-inf.
        let b = if i % 5 == 0 { a } else if i % 3 == 1 { rng.v_any() } else { rng.v_mixed() };
        let (x, y) = unsafe { (ca(a, b), ra(a, b)) };
        same("c2Add", &(a, b), &x, &y);
        let (x, y) = unsafe { (cs(a, b), rs(a, b)) };
        same("c2Sub", &(a, b), &x, &y);
    }
}

/// row 4 — c2Dot,  row 5 — c2Det2
#[test]
fn row04_05_dot_det2() {
    let l = libs();
    let (cd, rd) = l.pair::<FnFvv>("c2Dot");
    let (ce, re) = l.pair::<FnFvv>("c2Det2");
    let mut rng = Rng::new(0xC2_0004);
    for i in 0..N {
        let a = if i % 3 == 0 { rng.v_any() } else { rng.v_mixed() };
        let b = match i % 7 {
            0 => a,                                                  // det == 0
            1 => C2v { x: -a.y, y: a.x },                             // dot == 0
            2 => C2v { x: a.x * 2.0, y: a.y * 2.0 },                  // collinear
            3 => rng.v_any(),
            _ => rng.v_mixed(),
        };
        let (x, y) = unsafe { (cd(a, b), rd(a, b)) };
        same_f32("c2Dot", &(a, b), x, y);
        let (x, y) = unsafe { (ce(a, b), re(a, b)) };
        same_f32("c2Det2", &(a, b), x, y);
    }
}

/// row 6 — c2Maxv / c2Minv (incl. NaN operands and +-0)
#[test]
fn row06_maxv_minv() {
    let l = libs();
    let (cx, rx) = l.pair::<FnVvv>("c2Maxv");
    let (cn, rn) = l.pair::<FnVvv>("c2Minv");
    let mut rng = Rng::new(0xC2_0006);
    // Exhaustive over the special pool first: this is where NaN ternary
    // semantics and +0/-0 sign selection show up.
    for &ax in SPECIAL {
        for &bx in SPECIAL {
            for &ay in SPECIAL {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: ay };
                let (p, q) = unsafe { (cx(a, b), rx(a, b)) };
                same("c2Maxv", &(a, b), &p, &q);
                let (p, q) = unsafe { (cn(a, b), rn(a, b)) };
                same("c2Minv", &(a, b), &p, &q);
            }
        }
    }
    for i in 0..N {
        let a = if i % 3 == 0 { rng.v_any() } else { rng.v_mixed() };
        let b = if i % 3 == 1 { rng.v_any() } else { rng.v_mixed() };
        let (p, q) = unsafe { (cx(a, b), rx(a, b)) };
        same("c2Maxv", &(a, b), &p, &q);
        let (p, q) = unsafe { (cn(a, b), rn(a, b)) };
        same("c2Minv", &(a, b), &p, &q);
    }
}

/// row 7 — c2Clampv (valid lo<=hi, inverted lo>hi, NaN in each argument)
#[test]
fn row07_clampv() {
    let l = libs();
    let (c, r) = l.pair::<FnVvvv>("c2Clampv");
    let mut rng = Rng::new(0xC2_0007);
    for i in 0..N {
        let a = if i % 4 == 0 { rng.v_any() } else { rng.v_mixed() };
        let (lo, hi) = match i % 5 {
            0 => {
                // inverted range
                let x = rng.v_mixed();
                let y = rng.v_mixed();
                (C2v { x: x.x.max(y.x), y: x.y.max(y.y) }, C2v { x: x.x.min(y.x), y: x.y.min(y.y) })
            }
            1 => (C2v { x: f32::NAN, y: 0.0 }, rng.v_mixed()),
            2 => (rng.v_mixed(), C2v { x: 0.0, y: f32::NAN }),
            _ => {
                let x = rng.v_mixed();
                let y = rng.v_mixed();
                (C2v { x: x.x.min(y.x), y: x.y.min(y.y) }, C2v { x: x.x.max(y.x), y: x.y.max(y.y) })
            }
        };
        let (p, q) = unsafe { (c(a, lo, hi), r(a, lo, hi)) };
        same("c2Clampv", &(a, lo, hi), &p, &q);
    }
}

/// row 8 — c2Neg / c2Skew / c2CCW90
#[test]
fn row08_neg_skew_ccw90() {
    let l = libs();
    let ops: Vec<(&str, (_, _))> = vec![
        ("c2Neg", l.pair::<FnVv>("c2Neg")),
        ("c2Skew", l.pair::<FnVv>("c2Skew")),
        ("c2CCW90", l.pair::<FnVv>("c2CCW90")),
    ];
    let mut rng = Rng::new(0xC2_0008);
    for &x in SPECIAL {
        for &y in SPECIAL {
            let a = C2v { x, y };
            for (name, (c, r)) in &ops {
                let (p, q) = unsafe { (c(a), r(a)) };
                same(name, &a, &p, &q);
            }
        }
    }
    for i in 0..N {
        let a = if i % 2 == 0 { rng.v_any() } else { rng.v_mixed() };
        for (name, (c, r)) in &ops {
            let (p, q) = unsafe { (c(a), r(a)) };
            same(name, &a, &p, &q);
        }
    }
}

/// row 9 — c2Len,  row 21 (ERRORS) overflow to inf
#[test]
fn row09_len() {
    let l = libs();
    let (c, r) = l.pair::<FnFv>("c2Len");
    let mut rng = Rng::new(0xC2_0009);
    for &x in SPECIAL {
        for &y in SPECIAL {
            let a = C2v { x, y };
            let (p, q) = unsafe { (c(a), r(a)) };
            same_f32("c2Len", &a, p, q);
        }
    }
    for i in 0..N {
        let a = if i % 3 == 0 { rng.v_any() } else { rng.v_mixed() };
        let (p, q) = unsafe { (c(a), r(a)) };
        same_f32("c2Len", &a, p, q);
    }
}

/// row 10 — c2Div (incl. divide by 0 / -0)
#[test]
fn row10_div() {
    let l = libs();
    let (c, r) = l.pair::<FnVvf>("c2Div");
    let mut rng = Rng::new(0xC2_000A);
    for &x in SPECIAL {
        for &b in SPECIAL {
            let a = C2v { x, y: -x };
            let (p, q) = unsafe { (c(a, b), r(a, b)) };
            same("c2Div", &(a, b), &p, &q);
        }
    }
    for i in 0..N {
        let a = if i % 3 == 0 { rng.v_any() } else { rng.v_mixed() };
        let b = if i % 4 == 0 { SPECIAL[rng.below(SPECIAL.len() as u32) as usize] } else { rng.f32_mixed() };
        let (p, q) = unsafe { (c(a, b), r(a, b)) };
        same("c2Div", &(a, b), &p, &q);
    }
}

/// row 11 — c2Norm (incl. zero vector -> NaN, denormal underflow)
#[test]
fn row11_norm() {
    let l = libs();
    let (c, r) = l.pair::<FnVv>("c2Norm");
    let mut rng = Rng::new(0xC2_000B);
    for &x in SPECIAL {
        for &y in SPECIAL {
            let a = C2v { x, y };
            let (p, q) = unsafe { (c(a), r(a)) };
            same("c2Norm", &a, &p, &q);
        }
    }
    for i in 0..N {
        let a = match i % 4 {
            0 => rng.v_any(),
            1 => {
                let th = rng.range(-3.15, 3.15);
                C2v { x: th.cos(), y: th.sin() } // already unit
            }
            _ => rng.v_mixed(),
        };
        let (p, q) = unsafe { (c(a), r(a)) };
        same("c2Norm", &a, &p, &q);
    }
}

/// row 12 — c2RotIdentity / c2xIdentity
#[test]
fn row12_identities() {
    let l = libs();
    let (c, r) = l.pair::<FnR>("c2RotIdentity");
    let (cr, rr) = unsafe { (c(), r()) };
    same("c2RotIdentity", &(), &cr, &rr);
    let (c, r) = l.pair::<FnX>("c2xIdentity");
    let (cx, rx) = unsafe { (c(), r()) };
    same("c2xIdentity", &(), &cx, &rx);
}

/// row 13 — c2Mulrv / c2MulrvT (identity, unit rotations, non-unit, NaN)
#[test]
fn row13_mulrv() {
    let l = libs();
    let (cm, rm) = l.pair::<FnVrv>("c2Mulrv");
    let (ct, rt) = l.pair::<FnVrv>("c2MulrvT");
    let mut rng = Rng::new(0xC2_000D);
    for i in 0..N {
        let rot = match i % 4 {
            0 => C2r { c: 1.0, s: 0.0 },
            1 => {
                let th = rng.range(-6.2831855, 6.2831855);
                C2r { c: th.cos(), s: th.sin() }
            }
            2 => C2r { c: rng.f32_mixed(), s: rng.f32_mixed() },
            _ => C2r { c: rng.range(-3.0, 3.0), s: rng.range(-3.0, 3.0) },
        };
        let v = if i % 5 == 0 { rng.v_any() } else { rng.v_mixed() };
        let (p, q) = unsafe { (cm(rot, v), rm(rot, v)) };
        same("c2Mulrv", &((rot.c, rot.s), v), &p, &q);
        let (p, q) = unsafe { (ct(rot, v), rt(rot, v)) };
        same("c2MulrvT", &((rot.c, rot.s), v), &p, &q);
    }
}

/// row 14 — c2Mulxv over identity / rotation-only / translation-only / both
#[test]
fn row14_mulxv() {
    let l = libs();
    let (c, r) = l.pair::<FnVxv>("c2Mulxv");
    let mut rng = Rng::new(0xC2_000E);
    for i in 0..N {
        let x = match i % 5 {
            0 => C2x { p: C2v { x: 0.0, y: 0.0 }, r: C2r { c: 1.0, s: 0.0 } },
            1 => {
                let th = rng.range(-6.2831855, 6.2831855);
                C2x { p: C2v { x: 0.0, y: 0.0 }, r: C2r { c: th.cos(), s: th.sin() } }
            }
            2 => C2x { p: rng.v_mixed(), r: C2r { c: 1.0, s: 0.0 } },
            3 => rng.xform_unit(),
            _ => rng.xform_nonunit(),
        };
        let v = if i % 6 == 0 { rng.v_any() } else { rng.v_mixed() };
        let (p, q) = unsafe { (c(x, v), r(x, v)) };
        same("c2Mulxv", &((x.p.x, x.p.y, x.r.c, x.r.s), v), &p, &q);
    }
}
