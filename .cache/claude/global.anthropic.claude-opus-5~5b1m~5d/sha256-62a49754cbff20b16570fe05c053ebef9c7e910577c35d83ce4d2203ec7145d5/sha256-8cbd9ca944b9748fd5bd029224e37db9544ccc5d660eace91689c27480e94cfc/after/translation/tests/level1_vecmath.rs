//! Phase B, level 1 — leaf vector math. CONFIGS.md rows 1..=14.
//!
//! Every call goes through both `.so`s via libloading.

mod common;
use common::*;

const N: usize = 4000;

#[test]
fn row01_c2V() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnFFV>("c2V") };
    let mut rng = Rng::new(0x1001);
    let mut d = Diff::new("1: c2V");
    for i in 0..N {
        let (x, y) = if i < SPECIALS.len() * SPECIALS.len() {
            (SPECIALS[i / SPECIALS.len()], SPECIALS[i % SPECIALS.len()])
        } else {
            (mixed(&mut rng, 100.0), mixed(&mut rng, 100.0))
        };
        unsafe { d.check((x, y), c(x, y), r(x, y)) };
    }
    d.finish();
}

#[test]
fn row02_c2Mulvs() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVfV>("c2Mulvs") };
    let mut rng = Rng::new(0x1002);
    let mut d = Diff::new("2: c2Mulvs");
    for _ in 0..N {
        let a = vec_mixed(&mut rng, 100.0);
        let b = mixed(&mut rng, 100.0);
        unsafe { d.check((a, b), c(a, b), r(a, b)) };
    }
    d.finish();
}

#[test]
fn row03_c2Maxv_c2Minv() {
    let l = libs();
    let (cmax, rmax) = unsafe { l.pair::<FnVVV>("c2Maxv") };
    let (cmin, rmin) = unsafe { l.pair::<FnVVV>("c2Minv") };
    let mut rng = Rng::new(0x1003);
    let mut dmax = Diff::new("3a: c2Maxv");
    let mut dmin = Diff::new("3b: c2Minv");
    // exhaustive over the specials cross-product, then randomized
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            for k in 0..SPECIALS.len() {
                let a = C2v {
                    x: SPECIALS[i],
                    y: SPECIALS[j],
                };
                let b = C2v {
                    x: SPECIALS[k],
                    y: SPECIALS[(k + i) % SPECIALS.len()],
                };
                unsafe {
                    dmax.check((a, b), cmax(a, b), rmax(a, b));
                    dmin.check((a, b), cmin(a, b), rmin(a, b));
                }
            }
        }
    }
    for _ in 0..N {
        let a = vec_mixed(&mut rng, 10.0);
        // frequently make them equal or share a component to hit exact ties
        let b = match rng.below(4) {
            0 => a,
            1 => C2v { x: a.x, y: mixed(&mut rng, 10.0) },
            2 => C2v { x: mixed(&mut rng, 10.0), y: a.y },
            _ => vec_mixed(&mut rng, 10.0),
        };
        unsafe {
            dmax.check((a, b), cmax(a, b), rmax(a, b));
            dmin.check((a, b), cmin(a, b), rmin(a, b));
        }
    }
    dmax.finish();
    dmin.finish();
}

#[test]
fn row04_c2Clampv() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVVVV>("c2Clampv") };
    let mut rng = Rng::new(0x1004);
    let mut d = Diff::new("4: c2Clampv");
    for _ in 0..N * 2 {
        let a = vec_mixed(&mut rng, 10.0);
        let p = vec_mixed(&mut rng, 10.0);
        let q = vec_mixed(&mut rng, 10.0);
        // half the time proper bounds, half the time inverted
        let (lo, hi) = if rng.boolean() {
            (
                C2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                C2v { x: p.x.max(q.x), y: p.y.max(q.y) },
            )
        } else {
            (p, q)
        };
        unsafe { d.check((a, lo, hi), c(a, lo, hi), r(a, lo, hi)) };
    }
    d.finish();
}

#[test]
fn row05_c2Sub_c2Add() {
    let l = libs();
    let (cs, rs) = unsafe { l.pair::<FnVVV>("c2Sub") };
    let (ca, ra) = unsafe { l.pair::<FnVVV>("c2Add") };
    let mut rng = Rng::new(0x1005);
    let mut dsub = Diff::new("5a: c2Sub");
    let mut dadd = Diff::new("5b: c2Add");
    for _ in 0..N * 2 {
        let a = vec_mixed(&mut rng, 1e20);
        let b = if rng.below(4) == 0 { a } else { vec_mixed(&mut rng, 1e20) };
        unsafe {
            dsub.check((a, b), cs(a, b), rs(a, b));
            dadd.check((a, b), ca(a, b), ra(a, b));
        }
    }
    dsub.finish();
    dadd.finish();
}

#[test]
fn row06_c2Dot() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVVF>("c2Dot") };
    let mut rng = Rng::new(0x1006);
    let mut d = Diff::new("6: c2Dot");
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            let b = C2v { x: SPECIALS[j], y: SPECIALS[i] };
            unsafe { d.check((a, b), c(a, b), r(a, b)) };
        }
    }
    for _ in 0..N * 2 {
        let a = vec_mixed(&mut rng, 1e18);
        // orthogonal / anti-parallel cases produce cancellation
        let b = match rng.below(4) {
            0 => C2v { x: -a.y, y: a.x },
            1 => C2v { x: -a.x, y: -a.y },
            _ => vec_mixed(&mut rng, 1e18),
        };
        unsafe { d.check((a, b), c(a, b), r(a, b)) };
    }
    d.finish();
}

#[test]
fn row07_c2Det2() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVVF>("c2Det2") };
    let mut rng = Rng::new(0x1007);
    let mut d = Diff::new("7: c2Det2");
    for _ in 0..N * 2 {
        let a = vec_mixed(&mut rng, 1e18);
        let b = match rng.below(4) {
            0 => a,                                                   // det == 0
            1 => C2v { x: a.x * 2.0, y: a.y * 2.0 },                  // collinear
            2 => C2v { x: -a.x, y: -a.y },                            // anti-collinear
            _ => vec_mixed(&mut rng, 1e18),
        };
        unsafe { d.check((a, b), c(a, b), r(a, b)) };
    }
    d.finish();
}

#[test]
fn row08_c2Len_and_row17_overflow_nan() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVF>("c2Len") };
    let mut rng = Rng::new(0x1008);
    let mut d = Diff::new("8/17: c2Len");
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            unsafe { d.check(a, c(a), r(a)) };
        }
    }
    for _ in 0..N * 2 {
        let a = vec_mixed(&mut rng, 1e20);
        unsafe { d.check(a, c(a), r(a)) };
    }
    d.finish();
}

#[test]
fn row09_c2Div_and_row15_div_by_zero() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVfV>("c2Div") };
    let mut rng = Rng::new(0x1009);
    let mut d = Diff::new("9/15: c2Div");
    for s in SPECIALS {
        for i in 0..SPECIALS.len() {
            let a = C2v { x: s, y: SPECIALS[i] };
            for b in SPECIALS {
                unsafe { d.check((a, b), c(a, b), r(a, b)) };
            }
        }
    }
    for _ in 0..N {
        let a = vec_mixed(&mut rng, 1e10);
        let b = if rng.below(5) == 0 { 0.0 } else { mixed(&mut rng, 1e10) };
        unsafe { d.check((a, b), c(a, b), r(a, b)) };
    }
    d.finish();
}

#[test]
fn row10_c2Norm_and_row16_zero_vector() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVV>("c2Norm") };
    let mut rng = Rng::new(0x100a);
    let mut d = Diff::new("10/16: c2Norm");
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            unsafe { d.check(a, c(a), r(a)) };
        }
    }
    // explicit zero vector -> NaN
    let z = C2v { x: 0.0, y: 0.0 };
    unsafe { d.check(z, c(z), r(z)) };
    let nz = C2v { x: -0.0, y: -0.0 };
    unsafe { d.check(nz, c(nz), r(nz)) };
    for _ in 0..N * 2 {
        let a = vec_mixed(&mut rng, 1e20);
        unsafe { d.check(a, c(a), r(a)) };
    }
    d.finish();
}

#[test]
fn row11_c2Neg_c2Skew_c2CCW90() {
    let l = libs();
    let (cn, rn) = unsafe { l.pair::<FnVV>("c2Neg") };
    let (ck, rk) = unsafe { l.pair::<FnVV>("c2Skew") };
    let (cw, rw) = unsafe { l.pair::<FnVV>("c2CCW90") };
    let mut rng = Rng::new(0x100b);
    let mut dn = Diff::new("11a: c2Neg");
    let mut dk = Diff::new("11b: c2Skew");
    let mut dw = Diff::new("11c: c2CCW90");
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            unsafe {
                dn.check(a, cn(a), rn(a));
                dk.check(a, ck(a), rk(a));
                dw.check(a, cw(a), rw(a));
            }
        }
    }
    for _ in 0..N {
        let a = vec_mixed(&mut rng, 1e12);
        unsafe {
            dn.check(a, cn(a), rn(a));
            dk.check(a, ck(a), rk(a));
            dw.check(a, cw(a), rw(a));
        }
    }
    dn.finish();
    dk.finish();
    dw.finish();
}

#[test]
fn row12_identities() {
    let l = libs();
    let (cr, rr) = unsafe { l.pair::<FnR>("c2RotIdentity") };
    let (cx, rx) = unsafe { l.pair::<FnX>("c2xIdentity") };
    let mut d = Diff::new("12: identities");
    unsafe {
        d.check("c2RotIdentity", cr(), rr());
        d.check("c2xIdentity", cx(), rx());
    }
    // also verify the exact expected constants
    unsafe {
        let v = cr();
        assert!(v.c.to_bits() == 1.0f32.to_bits() && v.s.to_bits() == 0.0f32.to_bits());
    }
    d.finish();
}

#[test]
fn row13_c2Mulrv_c2MulrvT() {
    let l = libs();
    let (cm, rm) = unsafe { l.pair::<FnRVV>("c2Mulrv") };
    let (ct, rt) = unsafe { l.pair::<FnRVV>("c2MulrvT") };
    let mut rng = Rng::new(0x100d);
    let mut dm = Diff::new("13a: c2Mulrv");
    let mut dt = Diff::new("13b: c2MulrvT");
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2r { c: SPECIALS[i], s: SPECIALS[j] };
            let b = C2v { x: SPECIALS[j], y: SPECIALS[i] };
            unsafe {
                dm.check((a, b), cm(a, b), rm(a, b));
                dt.check((a, b), ct(a, b), rt(a, b));
            }
        }
    }
    for _ in 0..N * 2 {
        let a = rot_mixed(&mut rng);
        let b = vec_mixed(&mut rng, 1e6);
        unsafe {
            dm.check((a, b), cm(a, b), rm(a, b));
            dt.check((a, b), ct(a, b), rt(a, b));
        }
    }
    dm.finish();
    dt.finish();
}

#[test]
fn row14_c2Mulxv() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnXVV>("c2Mulxv") };
    let mut rng = Rng::new(0x100e);
    let mut d = Diff::new("14: c2Mulxv");
    for _ in 0..N * 2 {
        let a = x_mixed(&mut rng, 1e4);
        let b = vec_mixed(&mut rng, 1e4);
        unsafe { d.check((a, b), c(a, b), r(a, b)) };
    }
    // identity transform must be a no-op modulo -0.0 quirks
    let idn = C2x {
        p: C2v { x: 0.0, y: 0.0 },
        r: C2r { c: 1.0, s: 0.0 },
    };
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let b = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            unsafe { d.check((idn, b), c(idn, b), r(idn, b)) };
        }
    }
    d.finish();
}
