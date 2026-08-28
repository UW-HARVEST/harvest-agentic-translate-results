//! Level 1: leaf vector-math functions (`c2V` .. `c2MulmvT`).

#![allow(non_snake_case)]

mod common;
use common::*;

#[test]
fn t_c2V() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnV>("c2V");
    for &x in EDGE_SCALARS {
        for &y in EDGE_SCALARS {
            let ctx = format!("x={x:?} y={y:?}");
            unsafe { assert_v_eq("c2V", &ctx, c(x, y), r(x, y)) };
        }
    }
    let mut rng = Rng::new(1);
    for _ in 0..20_000 {
        let (x, y) = (rng.float(), rng.float());
        let ctx = format!("x={x:?} y={y:?}");
        unsafe { assert_v_eq("c2V", &ctx, c(x, y), r(x, y)) };
    }
}

/// Drive a `(c2v, c2v) -> c2v` pair over the edge grid + random inputs.
fn check_vv_v(p: &Pair, name: &str) {
    let (c, r) = p.sym::<FnVV_V>(name);
    for &ax in EDGE_SCALARS {
        for &ay in EDGE_SCALARS {
            for &bx in EDGE_SCALARS {
                for &by in EDGE_SCALARS {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    let ctx = format!("a=({ax:?},{ay:?}) b=({bx:?},{by:?})");
                    unsafe { assert_v_eq(name, &ctx, c(a, b), r(a, b)) };
                }
            }
        }
    }
    let mut rng = Rng::new(0xABCD);
    for _ in 0..50_000 {
        let a = rng.vec_wild();
        let b = rng.vec_wild();
        let ctx = format!("a=({:?},{:?}) b=({:?},{:?})", a.x, a.y, b.x, b.y);
        unsafe { assert_v_eq(name, &ctx, c(a, b), r(a, b)) };
    }
}

#[test]
fn t_c2Add() {
    check_vv_v(&Pair::load(), "c2Add");
}

#[test]
fn t_c2Sub() {
    check_vv_v(&Pair::load(), "c2Sub");
}

#[test]
fn t_c2Minv() {
    check_vv_v(&Pair::load(), "c2Minv");
}

#[test]
fn t_c2Maxv() {
    check_vv_v(&Pair::load(), "c2Maxv");
}

#[test]
fn t_c2Dot() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnVV_f>("c2Dot");
    for &ax in EDGE_SCALARS {
        for &ay in EDGE_SCALARS {
            for &bx in EDGE_SCALARS {
                for &by in EDGE_SCALARS {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    let ctx = format!("a=({ax:?},{ay:?}) b=({bx:?},{by:?})");
                    unsafe { assert_f_eq("c2Dot", &ctx, c(a, b), r(a, b)) };
                }
            }
        }
    }
    let mut rng = Rng::new(7);
    for _ in 0..100_000 {
        let a = rng.vec_wild();
        let b = rng.vec_wild();
        let ctx = format!("a=({:?},{:?}) b=({:?},{:?})", a.x, a.y, b.x, b.y);
        unsafe { assert_f_eq("c2Dot", &ctx, c(a, b), r(a, b)) };
    }
    // and again with tame magnitudes, where rounding differences (e.g. an
    // unwanted FMA contraction) would be the only possible divergence.
    let mut rng = Rng::new(8);
    for _ in 0..100_000 {
        let a = c2v {
            x: rng.sym(1.0),
            y: rng.sym(1.0),
        };
        let b = c2v {
            x: rng.sym(1.0),
            y: rng.sym(1.0),
        };
        let ctx = format!("a=({:?},{:?}) b=({:?},{:?})", a.x, a.y, b.x, b.y);
        unsafe { assert_f_eq("c2Dot", &ctx, c(a, b), r(a, b)) };
    }
}

fn check_v_f(p: &Pair, name: &str) {
    let (c, r) = p.sym::<FnV_f>(name);
    for &ax in EDGE_SCALARS {
        for &ay in EDGE_SCALARS {
            let a = c2v { x: ax, y: ay };
            let ctx = format!("a=({ax:?},{ay:?})");
            unsafe { assert_f_eq(name, &ctx, c(a), r(a)) };
        }
    }
    let mut rng = Rng::new(0x1234_5678);
    for _ in 0..100_000 {
        let a = rng.vec_wild();
        let ctx = format!("a=({:?},{:?})", a.x, a.y);
        unsafe { assert_f_eq(name, &ctx, c(a), r(a)) };
    }
}

#[test]
fn t_c2Len() {
    check_v_f(&Pair::load(), "c2Len");
}

fn check_v_v(p: &Pair, name: &str) {
    let (c, r) = p.sym::<FnV_V>(name);
    for &ax in EDGE_SCALARS {
        for &ay in EDGE_SCALARS {
            let a = c2v { x: ax, y: ay };
            let ctx = format!("a=({ax:?},{ay:?})");
            unsafe { assert_v_eq(name, &ctx, c(a), r(a)) };
        }
    }
    let mut rng = Rng::new(0xFEED_BEEF);
    for _ in 0..100_000 {
        let a = rng.vec_wild();
        let ctx = format!("a=({:?},{:?})", a.x, a.y);
        unsafe { assert_v_eq(name, &ctx, c(a), r(a)) };
    }
    // A second pass biased towards near-unit vectors: this is the regime
    // `c2Norm` is actually used in and where the reciprocal-multiply quirk
    // shows up.
    let mut rng = Rng::new(0xC0FF_EE00);
    for _ in 0..200_000 {
        let a = c2v {
            x: rng.sym(3.0),
            y: rng.sym(3.0),
        };
        let ctx = format!("a=({:?},{:?})", a.x, a.y);
        unsafe { assert_v_eq(name, &ctx, c(a), r(a)) };
    }
}

#[test]
fn t_c2Skew() {
    check_v_v(&Pair::load(), "c2Skew");
}

#[test]
fn t_c2Absv() {
    check_v_v(&Pair::load(), "c2Absv");
}

#[test]
fn t_c2CCW90() {
    check_v_v(&Pair::load(), "c2CCW90");
}

#[test]
fn t_c2Norm() {
    check_v_v(&Pair::load(), "c2Norm");
}

fn check_vf_v(p: &Pair, name: &str) {
    let (c, r) = p.sym::<FnVf_V>(name);
    for &ax in EDGE_SCALARS {
        for &ay in EDGE_SCALARS {
            for &b in EDGE_SCALARS {
                let a = c2v { x: ax, y: ay };
                let ctx = format!("a=({ax:?},{ay:?}) b={b:?}");
                unsafe { assert_v_eq(name, &ctx, c(a, b), r(a, b)) };
            }
        }
    }
    let mut rng = Rng::new(0x5EED);
    for _ in 0..100_000 {
        let a = rng.vec_wild();
        let b = rng.float();
        let ctx = format!("a=({:?},{:?}) b={b:?}", a.x, a.y);
        unsafe { assert_v_eq(name, &ctx, c(a, b), r(a, b)) };
    }
    // Tame pass: exercises the `1/b` rounding path of c2Div densely.
    let mut rng = Rng::new(0x5EED_2);
    for _ in 0..200_000 {
        let a = c2v {
            x: rng.sym(10.0),
            y: rng.sym(10.0),
        };
        let b = rng.sym(10.0);
        let ctx = format!("a=({:?},{:?}) b={b:?}", a.x, a.y);
        unsafe { assert_v_eq(name, &ctx, c(a, b), r(a, b)) };
    }
}

#[test]
fn t_c2Mulvs() {
    check_vf_v(&Pair::load(), "c2Mulvs");
}

#[test]
fn t_c2Div() {
    check_vf_v(&Pair::load(), "c2Div");
}

#[test]
fn t_c2MulmvT() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnMV_V>("c2MulmvT");
    let mut rng = Rng::new(0x99);
    for _ in 0..100_000 {
        let m = c2m {
            x: rng.vec_wild(),
            y: rng.vec_wild(),
        };
        let b = rng.vec_wild();
        let ctx = format!(
            "M=(({:?},{:?}),({:?},{:?})) b=({:?},{:?})",
            m.x.x, m.x.y, m.y.x, m.y.y, b.x, b.y
        );
        unsafe { assert_v_eq("c2MulmvT", &ctx, c(m, b), r(m, b)) };
    }
    let mut rng = Rng::new(0x9A);
    for _ in 0..100_000 {
        let m = c2m {
            x: rng.vec_tame(),
            y: rng.vec_tame(),
        };
        let b = rng.vec_tame();
        let ctx = format!(
            "M=(({:?},{:?}),({:?},{:?})) b=({:?},{:?})",
            m.x.x, m.x.y, m.y.x, m.y.y, b.x, b.y
        );
        unsafe { assert_v_eq("c2MulmvT", &ctx, c(m, b), r(m, b)) };
    }
    // Rotation-matrix shaped inputs (how c2RaytoCapsule uses it).
    let mut rng = Rng::new(0x9B);
    for _ in 0..100_000 {
        let ang = rng.unit() * 6.283_185_5;
        let y = c2v {
            x: ang.cos(),
            y: ang.sin(),
        };
        let m = c2m {
            x: c2v { x: y.y, y: -y.x },
            y,
        };
        let b = c2v {
            x: rng.sym(20.0),
            y: rng.sym(20.0),
        };
        let ctx = format!("ang={ang:?} b=({:?},{:?})", b.x, b.y);
        unsafe { assert_v_eq("c2MulmvT", &ctx, c(m, b), r(m, b)) };
    }
}
