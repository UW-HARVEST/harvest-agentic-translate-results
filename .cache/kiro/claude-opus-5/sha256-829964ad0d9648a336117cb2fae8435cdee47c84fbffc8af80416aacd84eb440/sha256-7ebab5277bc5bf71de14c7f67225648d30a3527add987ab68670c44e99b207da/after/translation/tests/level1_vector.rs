//! Level 1: the leaf vector/scalar helpers.
//!
//! Every call goes through `dlsym` on both shared objects; results are compared
//! bit-for-bit.

#![allow(non_snake_case)]

mod common;
use common::*;

const RANGE: f32 = 250.0;

#[test]
fn t_c2V() {
    let (c, r) = both::<FnVff>("c2V");
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            unsafe { assert_v(&format!("c2V({x:?},{y:?})"), c(x, y), r(x, y)) };
        }
    }
    let mut g = Rng::new(1);
    for _ in 0..20_000 {
        let (x, y) = (g.f_spicy(RANGE), g.f_spicy(RANGE));
        unsafe { assert_v(&format!("c2V({x:?},{y:?})"), c(x, y), r(x, y)) };
    }
}

#[test]
fn t_c2Mulvs() {
    let (c, r) = both::<FnVvf>("c2Mulvs");
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            for &s in EDGE_F32 {
                let a = c2v { x, y };
                unsafe {
                    assert_v(&format!("c2Mulvs({x:?},{y:?},{s:?})"), c(a, s), r(a, s));
                }
            }
        }
    }
    let mut g = Rng::new(2);
    for _ in 0..20_000 {
        let a = g.v_spicy(RANGE);
        let s = g.f_spicy(RANGE);
        unsafe { assert_v(&format!("c2Mulvs({a:?},{s:?})"), c(a, s), r(a, s)) };
    }
}

/// c2Maxv / c2Minv use raw ternaries in C, so signed zeros and NaN matter.
#[test]
fn t_c2Maxv_c2Minv() {
    for sym in ["c2Maxv", "c2Minv"] {
        let (c, r) = both::<FnVvv>(sym);
        for &ax in EDGE_F32 {
            for &ay in EDGE_F32 {
                for &bx in EDGE_F32 {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: ax };
                    unsafe {
                        assert_v(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b));
                    }
                }
            }
        }
        let mut g = Rng::new(3);
        for _ in 0..20_000 {
            let (a, b) = (g.v_spicy(RANGE), g.v_spicy(RANGE));
            unsafe { assert_v(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
        }
    }
}

#[test]
fn t_c2Clampv() {
    let (c, r) = both::<FnVvvv>("c2Clampv");
    let mut g = Rng::new(4);
    for _ in 0..40_000 {
        let a = g.v_spicy(RANGE);
        let lo = g.v_spicy(RANGE);
        let hi = g.v_spicy(RANGE);
        unsafe {
            assert_v(
                &format!("c2Clampv({a:?},{lo:?},{hi:?})"),
                c(a, lo, hi),
                r(a, lo, hi),
            )
        };
    }
    // Deliberately inverted / degenerate bounds, plus NaN bounds.
    for &lo in EDGE_F32 {
        for &hi in EDGE_F32 {
            let a = c2v { x: 1.5, y: -2.5 };
            let l = c2v { x: lo, y: hi };
            let h = c2v { x: hi, y: lo };
            unsafe {
                assert_v(
                    &format!("c2Clampv({a:?},{l:?},{h:?})"),
                    c(a, l, h),
                    r(a, l, h),
                )
            };
        }
    }
}

#[test]
fn t_c2Sub_c2Add() {
    for sym in ["c2Sub", "c2Add"] {
        let (c, r) = both::<FnVvv>(sym);
        for &ax in EDGE_F32 {
            for &bx in EDGE_F32 {
                let a = c2v { x: ax, y: bx };
                let b = c2v { x: bx, y: ax };
                unsafe { assert_v(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
            }
        }
        let mut g = Rng::new(5);
        for _ in 0..20_000 {
            let (a, b) = (g.v_spicy(RANGE), g.v_spicy(RANGE));
            unsafe { assert_v(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
        }
    }
}

#[test]
fn t_c2Dot_c2Det2() {
    for sym in ["c2Dot", "c2Det2"] {
        let (c, r) = both::<FnFvv>(sym);
        for &ax in EDGE_F32 {
            for &ay in EDGE_F32 {
                for &bx in EDGE_F32 {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: ay };
                    unsafe { assert_f32(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
                }
            }
        }
        let mut g = Rng::new(6);
        for _ in 0..40_000 {
            let (a, b) = (g.v_spicy(RANGE), g.v_spicy(RANGE));
            unsafe { assert_f32(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
        }
    }
}

#[test]
fn t_c2Len() {
    let (c, r) = both::<FnFv>("c2Len");
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = c2v { x, y };
            unsafe { assert_f32(&format!("c2Len({a:?})"), c(a), r(a)) };
        }
    }
    let mut g = Rng::new(7);
    for _ in 0..40_000 {
        let a = g.v_spicy(RANGE);
        unsafe { assert_f32(&format!("c2Len({a:?})"), c(a), r(a)) };
    }
}

#[test]
fn t_unary_vector_ops() {
    for sym in ["c2Neg", "c2Skew", "c2CCW90", "c2Norm"] {
        let (c, r) = both::<FnVv>(sym);
        for &x in EDGE_F32 {
            for &y in EDGE_F32 {
                let a = c2v { x, y };
                unsafe { assert_v(&format!("{sym}({a:?})"), c(a), r(a)) };
            }
        }
        let mut g = Rng::new(8);
        for _ in 0..40_000 {
            let a = g.v_spicy(RANGE);
            unsafe { assert_v(&format!("{sym}({a:?})"), c(a), r(a)) };
        }
    }
}

#[test]
fn t_c2Div() {
    let (c, r) = both::<FnVvf>("c2Div");
    for &x in EDGE_F32 {
        for &d in EDGE_F32 {
            let a = c2v { x, y: -x };
            unsafe { assert_v(&format!("c2Div({a:?},{d:?})"), c(a, d), r(a, d)) };
        }
    }
    let mut g = Rng::new(9);
    for _ in 0..40_000 {
        let a = g.v_spicy(RANGE);
        let d = g.f_spicy(RANGE);
        unsafe { assert_v(&format!("c2Div({a:?},{d:?})"), c(a, d), r(a, d)) };
    }
}

#[test]
fn t_identities() {
    let (c, r) = both::<FnR>("c2RotIdentity");
    unsafe {
        let (cv, rv) = (c(), r());
        assert_bytes("c2RotIdentity", &cv, &rv);
    }
    let (c, r) = both::<FnX>("c2xIdentity");
    unsafe {
        let (cv, rv) = (c(), r());
        assert_bytes("c2xIdentity", &cv, &rv);
    }
}

#[test]
fn t_c2Mulrv_c2MulrvT() {
    for sym in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = both::<FnVrv>(sym);
        for &rc in EDGE_F32 {
            for &rs in EDGE_F32 {
                for &bx in EDGE_F32 {
                    let rot = c2r { c: rc, s: rs };
                    let b = c2v { x: bx, y: rc };
                    unsafe { assert_v(&format!("{sym}({rot:?},{b:?})"), c(rot, b), r(rot, b)) };
                }
            }
        }
        let mut g = Rng::new(10);
        for _ in 0..40_000 {
            // Both arbitrary and genuine (cos, sin) rotations.
            let rot = if g.below(2) == 0 {
                c2r {
                    c: g.f_spicy(4.0),
                    s: g.f_spicy(4.0),
                }
            } else {
                let ang = g.f(std::f32::consts::PI);
                c2r {
                    c: ang.cos(),
                    s: ang.sin(),
                }
            };
            let b = g.v_spicy(RANGE);
            unsafe { assert_v(&format!("{sym}({rot:?},{b:?})"), c(rot, b), r(rot, b)) };
        }
    }
}

#[test]
fn t_c2Mulxv() {
    let (c, r) = both::<FnVxv>("c2Mulxv");
    let mut g = Rng::new(11);
    for _ in 0..60_000 {
        let ang = g.f(std::f32::consts::PI);
        let x = c2x {
            p: g.v_spicy(RANGE),
            r: if g.below(2) == 0 {
                c2r {
                    c: ang.cos(),
                    s: ang.sin(),
                }
            } else {
                c2r {
                    c: g.f_spicy(4.0),
                    s: g.f_spicy(4.0),
                }
            },
        };
        let b = g.v_spicy(RANGE);
        unsafe { assert_v(&format!("c2Mulxv({x:?},{b:?})"), c(x, b), r(x, b)) };
    }
    for &f in EDGE_F32 {
        let x = c2x {
            p: c2v { x: f, y: -f },
            r: c2r { c: f, s: 1.0 - f },
        };
        let b = c2v { x: 3.0, y: f };
        unsafe { assert_v(&format!("c2Mulxv({x:?},{b:?})"), c(x, b), r(x, b)) };
    }
}
