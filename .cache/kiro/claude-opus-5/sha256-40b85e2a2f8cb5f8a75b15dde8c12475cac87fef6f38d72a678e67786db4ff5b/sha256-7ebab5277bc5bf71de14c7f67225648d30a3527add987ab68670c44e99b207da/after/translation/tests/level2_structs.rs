//! Level 2: functions that read/write the aggregate types
//! (`c2Poly`, `c2AABB`, `c2Proxy`, `c2Simplex`).
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 20_000;

/// Fills a value with pseudo-random bytes.
fn fill<T>(rng: &mut Rng, v: &mut T) {
    let n = std::mem::size_of::<T>();
    let p = v as *mut T as *mut u8;
    for i in 0..n {
        unsafe { *p.add(i) = rng.next_u32() as u8 };
    }
}

/// A simplex whose float fields come from `wild()`/`tame()` rather than raw bytes,
/// so that the values are floats a caller might realistically produce.
fn make_simplex(rng: &mut Rng, wild: bool) -> c2Simplex {
    let f = |r: &mut Rng| if wild { r.wild() } else { r.tame() };
    let sv = |r: &mut Rng| c2sv {
        sA: c2v { x: f(r), y: f(r) },
        sB: c2v { x: f(r), y: f(r) },
        p: c2v { x: f(r), y: f(r) },
        u: f(r),
        iA: (r.below(8)) as c_int,
        iB: (r.below(8)) as c_int,
    };
    c2Simplex {
        a: sv(rng),
        b: sv(rng),
        c: sv(rng),
        d: sv(rng),
        div: f(rng),
        count: rng.below(6) as c_int - 1,
    }
}

fn make_poly(rng: &mut Rng, wild: bool) -> c2Poly {
    let mut p = c2Poly::default();
    p.count = 1 + rng.below(8) as c_int;
    for i in 0..8 {
        p.verts[i] = if wild { rng.vec_wild() } else { rng.vec_tame() };
        p.norms[i] = if wild { rng.vec_wild() } else { rng.vec_tame() };
    }
    p
}

#[test]
fn c2PlaneAt_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(*const c2Poly, c_int) -> c2h>("c2PlaneAt");
    let mut rng = Rng::new(101);
    for _ in 0..N {
        let wild = rng.below(3) == 0;
        let p = make_poly(&mut rng, wild);
        for i in 0..8 {
            unsafe {
                let (cv, rv) = (c(&p, i), r(&p, i));
                assert_same_lazy(&cv, &rv, || format!("c2PlaneAt(i={i}, {p:?})"));
            }
        }
    }
}

#[test]
fn c2BBVerts_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2v, *mut c2AABB) -> ()>("c2BBVerts");
    let mut rng = Rng::new(103);
    for _ in 0..N {
        let mut bb = c2AABB {
            min: rng.vec_wild(),
            max: rng.vec_wild(),
        };
        let mut oc = [c2v { x: 7.5, y: -3.25 }; 4];
        let mut or_ = oc;
        unsafe {
            c(oc.as_mut_ptr(), &mut bb);
            r(or_.as_mut_ptr(), &mut bb);
        }
        assert_same_lazy(&oc, &or_, || format!("c2BBVerts({bb:?})"));
    }
}

#[test]
fn c2MakeProxy_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(*const c_void, C2_TYPE, *mut c2Proxy) -> ()>("c2MakeProxy");
    let mut rng = Rng::new(107);
    for _ in 0..N {
        // The proxy the C sees must start from a known state, otherwise the
        // (deliberately absent) C2_TYPE_POLY case would read our stack garbage.
        let seed = {
            let mut p = c2Proxy::default();
            fill(&mut rng, &mut p);
            p
        };
        let circle = c2Circle {
            p: rng.vec_wild(),
            r: rng.wild(),
        };
        let aabb = c2AABB {
            min: rng.vec_wild(),
            max: rng.vec_wild(),
        };
        let capsule = c2Capsule {
            a: rng.vec_wild(),
            b: rng.vec_wild(),
            r: rng.wild(),
        };
        let cases: [(C2_TYPE, *const c_void); 4] = [
            (C2_TYPE_CIRCLE, &circle as *const _ as *const c_void),
            (C2_TYPE_AABB, &aabb as *const _ as *const c_void),
            (C2_TYPE_CAPSULE, &capsule as *const _ as *const c_void),
            (C2_TYPE_POLY, &circle as *const _ as *const c_void),
        ];
        for (t, shape) in cases {
            let mut pc = seed;
            let mut pr = seed;
            unsafe {
                c(shape, t, &mut pc);
                r(shape, t, &mut pr);
            }
            assert_same_lazy(&pc, &pr, || format!("c2MakeProxy(type={t})"));
        }
    }
}

#[test]
fn c2Norms_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int) -> ()>("c2Norms");
    let mut rng = Rng::new(109);
    for _ in 0..N {
        let n = rng.below(9) as c_int;
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = if rng.below(3) == 0 {
                rng.vec_wild()
            } else {
                rng.vec_tame()
            };
        }
        let mut nc = [c2v { x: 1.5, y: 2.5 }; 8];
        let mut nr = nc;
        let mut vc = verts;
        let mut vr = verts;
        unsafe {
            c(vc.as_mut_ptr(), nc.as_mut_ptr(), n);
            r(vr.as_mut_ptr(), nr.as_mut_ptr(), n);
        }
        assert_same_lazy(&nc, &nr, || format!("c2Norms(n={n}) norms"));
        assert_same_lazy(&vc, &vr, || format!("c2Norms(n={n}) verts"));
    }
}

#[test]
fn c2GJKSimplexMetric_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> f32>("c2GJKSimplexMetric");
    let mut rng = Rng::new(113);
    for _ in 0..N {
        for wild in [false, true] {
            let s = make_simplex(&mut rng, wild);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                let a = c(&mut sc);
                let b = r(&mut sr);
                assert_f32_lazy(a, b, || format!("c2GJKSimplexMetric(count={})", s.count));
            }
            assert_same("c2GJKSimplexMetric side effects", &sc, &sr);
        }
    }
}

#[test]
fn c22_and_c23_match() {
    let _serial = serialize();
    let l = Libs::load();
    let mut rng = Rng::new(127);
    for name in ["c22", "c23"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> ()>(name);
        for _ in 0..N {
            for wild in [false, true] {
                let s = make_simplex(&mut rng, wild);
                let mut sc = s;
                let mut sr = s;
                unsafe {
                    c(&mut sc);
                    r(&mut sr);
                }
                assert_same_lazy(&sc, &sr, || format!("{name}(count={})", s.count));
            }
        }
    }
}

#[test]
fn c2D_and_c2L_match() {
    let _serial = serialize();
    let l = Libs::load();
    let mut rng = Rng::new(131);
    for name in ["c2D", "c2L"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>(name);
        for _ in 0..N {
            for wild in [false, true] {
                let s = make_simplex(&mut rng, wild);
                let mut sc = s;
                let mut sr = s;
                unsafe {
                    let a = c(&mut sc);
                    let b = r(&mut sr);
                    assert_same_lazy(&a, &b, || format!("{name}(count={})", s.count));
                }
                assert_same_lazy(&sc, &sr, || format!("{name} side effects"));
            }
        }
    }
}

#[test]
fn c2Witness_matches() {
    let _serial = serialize();
    let l = Libs::load();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v) -> ()>("c2Witness");
    let mut rng = Rng::new(137);
    for _ in 0..N {
        for wild in [false, true] {
            let s = make_simplex(&mut rng, wild);
            let mut sc = s;
            let mut sr = s;
            let mut ac = c2v { x: 9.0, y: -9.0 };
            let mut bc = c2v { x: -8.0, y: 8.0 };
            let mut ar = ac;
            let mut br = bc;
            unsafe {
                c(&mut sc, &mut ac, &mut bc);
                r(&mut sr, &mut ar, &mut br);
            }
            assert_same_lazy(&ac, &ar, || format!("c2Witness a (count={})", s.count));
            assert_same_lazy(&bc, &br, || format!("c2Witness b (count={})", s.count));
            assert_same("c2Witness side effects", &sc, &sr);
        }
    }
}
