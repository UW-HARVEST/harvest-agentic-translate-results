//! Level 1: leaf scalar / vector helpers.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

type FnVV = unsafe extern "C" fn(c2v) -> c2v;
type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
type FnVf = unsafe extern "C" fn(c2v) -> f32;
type FnVsV = unsafe extern "C" fn(c2v, f32) -> c2v;

const N: usize = 40_000;

#[test]
fn c2V_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(f32, f32) -> c2v>("c2V");
    let mut rng = Rng::new(1);
    for i in 0..N {
        let (x, y) = if i < NOTABLE.len() * NOTABLE.len() {
            (NOTABLE[i / NOTABLE.len()], NOTABLE[i % NOTABLE.len()])
        } else {
            (rng.wild(), rng.wild())
        };
        let (cv, rv) = unsafe { (c(x, y), r(x, y)) };
        assert_same_lazy(&cv, &rv, || format!("c2V({x},{y})"));
    }
}

/// Drives a `c2v -> c2v` pair over notable and random inputs.
fn sweep_vv(l: &Libs, name: &str) {
    let (c, r) = l.pair::<FnVV>(name);
    let mut rng = Rng::new(0x5eed);
    for &x in NOTABLE {
        for &y in NOTABLE {
            let a = c2v { x, y };
            let (cv, rv) = unsafe { (c(a), r(a)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({x},{y})"));
        }
    }
    for _ in 0..N {
        let a = rng.vec_wild();
        let (cv, rv) = unsafe { (c(a), r(a)) };
        assert_same_lazy(&cv, &rv, || format!("{name}({a:?})"));
    }
}

fn sweep_vvv(l: &Libs, name: &str) {
    let (c, r) = l.pair::<FnVVV>(name);
    let mut rng = Rng::new(0xabcd);
    for &x in NOTABLE {
        for &y in NOTABLE {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({a:?},{b:?})"));
            let z = c2v { x: 0.0, y: 0.0 };
            let (cv, rv) = unsafe { (c(a, z), r(a, z)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({a:?},{z:?})"));
            let (cv, rv) = unsafe { (c(z, a), r(z, a)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({z:?},{a:?})"));
        }
    }
    for _ in 0..N {
        let a = rng.vec_wild();
        let b = rng.vec_wild();
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_same_lazy(&cv, &rv, || format!("{name}({a:?},{b:?})"));
    }
}

fn sweep_vvf(l: &Libs, name: &str) {
    let (c, r) = l.pair::<FnVVf>(name);
    let mut rng = Rng::new(0x1234_5678);
    for &x in NOTABLE {
        for &y in NOTABLE {
            for &z in NOTABLE {
                let a = c2v { x, y };
                let b = c2v { x: z, y };
                let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
                assert_f32_lazy(cv, rv, || format!("{name}({a:?},{b:?})"));
            }
        }
    }
    for _ in 0..N {
        let a = rng.vec_wild();
        let b = rng.vec_wild();
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_f32_lazy(cv, rv, || format!("{name}({a:?},{b:?})"));
    }
}

#[test]
fn unary_vector_ops_match() {
    let _serial = serialize();
    let l = Libs::load();
    for name in ["c2Neg", "c2CCW90", "c2Skew", "c2Absv", "c2Norm"] {
        sweep_vv(&l, name);
    }
}

#[test]
fn binary_vector_ops_match() {
    let _serial = serialize();
    let l = Libs::load();
    for name in ["c2Maxv", "c2Minv", "c2Sub", "c2Add"] {
        sweep_vvv(&l, name);
    }
}

#[test]
fn dot_and_det_match() {
    let _serial = serialize();
    let l = Libs::load();
    sweep_vvf(&l, "c2Dot");
    sweep_vvf(&l, "c2Det2");
}

#[test]
fn c2Len_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<FnVf>("c2Len");
    let mut rng = Rng::new(7);
    for &x in NOTABLE {
        for &y in NOTABLE {
            let a = c2v { x, y };
            let (cv, rv) = unsafe { (c(a), r(a)) };
            assert_f32_lazy(cv, rv, || format!("c2Len({a:?})"));
        }
    }
    for _ in 0..N {
        let a = rng.vec_wild();
        let (cv, rv) = unsafe { (c(a), r(a)) };
        assert_f32_lazy(cv, rv, || format!("c2Len({a:?})"));
    }
}

#[test]
fn scaling_ops_match() {
    let _serial = serialize();
    let l = Libs::load();
    let mut rng = Rng::new(11);
    for name in ["c2Mulvs", "c2Div"] {
        let (c, r) = l.pair::<FnVsV>(name);
        for &x in NOTABLE {
            for &s in NOTABLE {
                let a = c2v { x, y: -x };
                let (cv, rv) = unsafe { (c(a, s), r(a, s)) };
                assert_same_lazy(&cv, &rv, || format!("{name}({a:?},{s})"));
            }
        }
        for _ in 0..N {
            let a = rng.vec_wild();
            let s = rng.wild();
            let (cv, rv) = unsafe { (c(a, s), r(a, s)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({a:?},{s})"));
        }
    }
}

#[test]
fn c2Clampv_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v>("c2Clampv");
    let mut rng = Rng::new(13);
    for _ in 0..N {
        let a = rng.vec_wild();
        let lo = rng.vec_wild();
        let hi = rng.vec_wild();
        unsafe {
            assert_same(
                &format!("c2Clampv({a:?},{lo:?},{hi:?})"),
                &c(a, lo, hi),
                &r(a, lo, hi),
            )
        };
    }
    // Ordered lo <= hi cases, which is how the library actually calls it.
    for _ in 0..N {
        let a = rng.vec_tame();
        let p = rng.vec_tame();
        let q = rng.vec_tame();
        let lo = c2v {
            x: p.x.min(q.x),
            y: p.y.min(q.y),
        };
        let hi = c2v {
            x: p.x.max(q.x),
            y: p.y.max(q.y),
        };
        unsafe {
            assert_same(
                &format!("c2Clampv({a:?},{lo:?},{hi:?})"),
                &c(a, lo, hi),
                &r(a, lo, hi),
            )
        };
    }
}

#[test]
fn c2Dist_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2h, c2v) -> f32>("c2Dist");
    let mut rng = Rng::new(17);
    for _ in 0..N {
        let h = c2h {
            n: rng.vec_wild(),
            d: rng.wild(),
        };
        let p = rng.vec_wild();
        let (cv, rv) = unsafe { (c(h, p), r(h, p)) };
        assert_f32_lazy(cv, rv, || format!("c2Dist({h:?},{p:?})"));
    }
}

#[test]
fn c2Intersect_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v>("c2Intersect");
    let mut rng = Rng::new(19);
    for _ in 0..N {
        let a = rng.vec_wild();
        let b = rng.vec_wild();
        let da = rng.wild();
        let db = rng.wild();
        unsafe {
            assert_same(
                &format!("c2Intersect({a:?},{b:?},{da},{db})"),
                &c(a, b, da, db),
                &r(a, b, da, db),
            )
        };
    }
    // Equal da/db forces a 0/0 division.
    for _ in 0..2000 {
        let a = rng.vec_tame();
        let b = rng.vec_tame();
        let d = rng.tame();
        unsafe {
            assert_same(
                &format!("c2Intersect({a:?},{b:?},{d},{d})"),
                &c(a, b, d, d),
                &r(a, b, d, d),
            )
        };
    }
}

#[test]
fn identities_match() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn() -> c2r>("c2RotIdentity");
    unsafe { assert_same("c2RotIdentity", &c(), &r()) };
    let (c, r) = l.pair::<unsafe extern "C" fn() -> c2x>("c2xIdentity");
    unsafe { assert_same("c2xIdentity", &c(), &r()) };
}

#[test]
fn rotation_ops_match() {
    let _serial = serialize();
    let l = Libs::load();
    let mut rng = Rng::new(23);
    for name in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2r, c2v) -> c2v>(name);
        for &x in NOTABLE {
            for &y in NOTABLE {
                let rot = c2r { c: x, s: y };
                let v = c2v { x: y, y: x };
                let (cv, rv) = unsafe { (c(rot, v), r(rot, v)) };
                assert_same_lazy(&cv, &rv, || format!("{name}({rot:?},{v:?})"));
            }
        }
        for _ in 0..N {
            let rot = c2r {
                c: rng.wild(),
                s: rng.wild(),
            };
            let v = rng.vec_wild();
            let (cv, rv) = unsafe { (c(rot, v), r(rot, v)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({rot:?},{v:?})"));
        }
    }
}

#[test]
fn transform_ops_match() {
    let _serial = serialize();
    let l = Libs::load();
    let mut rng = Rng::new(29);
    for name in ["c2Mulxv", "c2MulxvT"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2x, c2v) -> c2v>(name);
        for _ in 0..N {
            let x = c2x {
                p: rng.vec_wild(),
                r: c2r {
                    c: rng.wild(),
                    s: rng.wild(),
                },
            };
            let v = rng.vec_wild();
            let (cv, rv) = unsafe { (c(x, v), r(x, v)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({x:?},{v:?})"));
        }
        // Realistic rotations (unit c/s) with tame translations.
        for _ in 0..N {
            let ang = (rng.next_u32() as f64 / u32::MAX as f64) as f32 * 6.2831855;
            let x = c2x {
                p: rng.vec_tame(),
                r: c2r {
                    c: ang.cos(),
                    s: ang.sin(),
                },
            };
            let v = rng.vec_tame();
            let (cv, rv) = unsafe { (c(x, v), r(x, v)) };
            assert_same_lazy(&cv, &rv, || format!("{name}({x:?},{v:?})"));
        }
    }
}

#[test]
fn c2Support_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int>("c2Support");
    let mut rng = Rng::new(31);
    for _ in 0..N {
        let n = 1 + rng.below(8) as usize;
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut().take(n) {
            *v = if rng.below(4) == 0 {
                rng.vec_wild()
            } else {
                rng.vec_tame()
            };
        }
        let d = rng.vec_tame();
        unsafe {
            let a = c(verts.as_ptr(), n as c_int, d);
            let b = r(verts.as_ptr(), n as c_int, d);
            assert_eq!(a, b, "c2Support(n={n}, verts={verts:?}, d={d:?})");
        }
    }
}
