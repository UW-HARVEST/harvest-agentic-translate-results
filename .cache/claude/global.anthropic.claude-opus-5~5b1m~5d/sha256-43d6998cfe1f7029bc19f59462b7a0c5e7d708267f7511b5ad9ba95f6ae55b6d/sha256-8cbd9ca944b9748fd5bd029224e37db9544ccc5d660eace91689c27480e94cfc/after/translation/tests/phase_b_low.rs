//! Phase B — low-level differential tests (CONFIGS.md rows 1..24).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

const N: usize = 4000;

macro_rules! sym {
    ($p:expr, $t:ty, $n:literal) => {
        (($p).c.get::<$t>($n), ($p).r.get::<$t>($n))
    };
}

// --- row 1 / 2 : unary + binary vector helpers -----------------------------

#[test]
fn row01_row02_vector_helpers() {
    fresh(|| {
        let p = pair();
        let (cv_c, cv_r) = sym!(p, unsafe extern "C" fn(f32, f32) -> c2v, "c2V");
        let (mv_c, mv_r) = sym!(p, FnVfV, "c2Mulvs");
        let (sub_c, sub_r) = sym!(p, FnVVV, "c2Sub");
        let (add_c, add_r) = sym!(p, FnVVV, "c2Add");
        let (neg_c, neg_r) = sym!(p, FnVV, "c2Neg");
        let (sk_c, sk_r) = sym!(p, FnVV, "c2Skew");
        let (cw_c, cw_r) = sym!(p, FnVV, "c2CCW90");
        let (ab_c, ab_r) = sym!(p, FnVV, "c2Absv");

        let mut rng = Rng::new(0xC0FFEE_01);
        for i in 0..N {
            // row 1 uses `nice`, row 2 uses `wild` (non-finite)
            let wild = i % 2 == 1;
            let (a, b) = if wild {
                (rng.wild_v(), rng.wild_v())
            } else {
                (rng.v(), rng.v())
            };
            let s = if wild { rng.wild() } else { rng.nice() };
            unsafe {
                assert!(v_eq(cv_c(a.x, a.y), cv_r(a.x, a.y)), "c2V {:?}", a);
                let (x, y) = (mv_c(a, s), mv_r(a, s));
                assert!(v_eq(x, y), "c2Mulvs({}, {}) C={} R={}", fmt_v(a), fmt_f32(s), fmt_v(x), fmt_v(y));
                let (x, y) = (sub_c(a, b), sub_r(a, b));
                assert!(v_eq(x, y), "c2Sub({},{}) C={} R={}", fmt_v(a), fmt_v(b), fmt_v(x), fmt_v(y));
                let (x, y) = (add_c(a, b), add_r(a, b));
                assert!(v_eq(x, y), "c2Add({},{}) C={} R={}", fmt_v(a), fmt_v(b), fmt_v(x), fmt_v(y));
                for (name, f, g) in [
                    ("c2Neg", neg_c, neg_r),
                    ("c2Skew", sk_c, sk_r),
                    ("c2CCW90", cw_c, cw_r),
                    ("c2Absv", ab_c, ab_r),
                ] {
                    let (x, y) = (f(a), g(a));
                    assert!(v_eq(x, y), "{}({}) C={} R={}", name, fmt_v(a), fmt_v(x), fmt_v(y));
                }
            }
        }
    });
}

// --- row 3 / 4 : min / max / clamp ----------------------------------------

#[test]
fn row03_row04_min_max_clamp() {
    fresh(|| {
        let p = pair();
        let (mx_c, mx_r) = sym!(p, FnVVV, "c2Maxv");
        let (mn_c, mn_r) = sym!(p, FnVVV, "c2Minv");
        let (cl_c, cl_r) = sym!(p, FnVVVV, "c2Clampv");

        let mut rng = Rng::new(0xC0FFEE_03);
        for i in 0..N {
            let wild = i % 3 == 2;
            let (a, b, c) = if wild {
                (rng.wild_v(), rng.wild_v(), rng.wild_v())
            } else {
                (rng.v(), rng.v(), rng.v())
            };
            unsafe {
                let (x, y) = (mx_c(a, b), mx_r(a, b));
                assert!(v_eq(x, y), "c2Maxv({},{}) C={} R={}", fmt_v(a), fmt_v(b), fmt_v(x), fmt_v(y));
                let (x, y) = (mn_c(a, b), mn_r(a, b));
                assert!(v_eq(x, y), "c2Minv({},{}) C={} R={}", fmt_v(a), fmt_v(b), fmt_v(x), fmt_v(y));
                // both lo<=hi and deliberately lo>hi orderings
                for (lo, hi) in [(b, c), (c, b)] {
                    let (x, y) = (cl_c(a, lo, hi), cl_r(a, lo, hi));
                    assert!(
                        v_eq(x, y),
                        "c2Clampv({},{},{}) C={} R={}",
                        fmt_v(a), fmt_v(lo), fmt_v(hi), fmt_v(x), fmt_v(y)
                    );
                }
            }
        }
        // Explicit ±0 and NaN pairs.
        let specials = [0.0f32, -0.0, 1.0, -1.0, f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
        for &ax in &specials {
            for &bx in &specials {
                let a = c2v { x: ax, y: bx };
                let b = c2v { x: bx, y: ax };
                unsafe {
                    assert!(v_eq(mx_c(a, b), mx_r(a, b)), "c2Maxv special {} {}", fmt_v(a), fmt_v(b));
                    assert!(v_eq(mn_c(a, b), mn_r(a, b)), "c2Minv special {} {}", fmt_v(a), fmt_v(b));
                }
            }
        }
    });
}

// --- row 5 / 6 : dot / det / len / div / norm -----------------------------

#[test]
fn row05_row06_dot_det_len_div_norm() {
    fresh(|| {
        let p = pair();
        let (dot_c, dot_r) = sym!(p, FnVVf, "c2Dot");
        let (det_c, det_r) = sym!(p, FnVVf, "c2Det2");
        let (len_c, len_r) = sym!(p, FnVf, "c2Len");
        let (div_c, div_r) = sym!(p, FnVfV, "c2Div");
        let (nrm_c, nrm_r) = sym!(p, FnVV, "c2Norm");

        let mut rng = Rng::new(0xC0FFEE_05);
        for i in 0..N {
            let wild = i % 2 == 1;
            let (a, b) = if wild {
                (rng.wild_v(), rng.wild_v())
            } else {
                (rng.v(), rng.v())
            };
            let s = if wild { rng.wild() } else { rng.nice() };
            unsafe {
                let (x, y) = (dot_c(a, b), dot_r(a, b));
                assert!(bits_eq_f32(x, y), "c2Dot({},{}) C={} R={}", fmt_v(a), fmt_v(b), fmt_f32(x), fmt_f32(y));
                let (x, y) = (det_c(a, b), det_r(a, b));
                assert!(bits_eq_f32(x, y), "c2Det2({},{}) C={} R={}", fmt_v(a), fmt_v(b), fmt_f32(x), fmt_f32(y));
                let (x, y) = (len_c(a), len_r(a));
                assert!(bits_eq_f32(x, y), "c2Len({}) C={} R={}", fmt_v(a), fmt_f32(x), fmt_f32(y));
                let (x, y) = (div_c(a, s), div_r(a, s));
                assert!(v_eq(x, y), "c2Div({},{}) C={} R={}", fmt_v(a), fmt_f32(s), fmt_v(x), fmt_v(y));
                let (x, y) = (nrm_c(a), nrm_r(a));
                assert!(v_eq(x, y), "c2Norm({}) C={} R={}", fmt_v(a), fmt_v(x), fmt_v(y));
            }
        }
        // zero vector -> NaN, and division by zero
        for a in [c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: 0.0 }, c2v { x: 1e30, y: 1e30 }] {
            unsafe {
                assert!(v_eq(nrm_c(a), nrm_r(a)), "c2Norm degenerate {}", fmt_v(a));
                for b in [0.0f32, -0.0, f32::INFINITY, f32::NAN] {
                    assert!(v_eq(div_c(a, b), div_r(a, b)), "c2Div {} / {}", fmt_v(a), fmt_f32(b));
                }
            }
        }
    });
}

// --- row 7 : c2Dist / c2PlaneAt -------------------------------------------

#[test]
fn row07_dist_planeat() {
    fresh(|| {
        let p = pair();
        let (dist_c, dist_r) = sym!(p, unsafe extern "C" fn(c2h, c2v) -> f32, "c2Dist");
        let (pa_c, pa_r) =
            sym!(p, unsafe extern "C" fn(*const c2Poly, c_int) -> c2h, "c2PlaneAt");

        let mut rng = Rng::new(0xC0FFEE_07);
        for i in 0..N {
            let wild = i % 4 == 3;
            let h = c2h {
                n: if wild { rng.wild_v() } else { rng.v() },
                d: if wild { rng.wild() } else { rng.nice() },
            };
            let q = if wild { rng.wild_v() } else { rng.v() };
            unsafe {
                let (x, y) = (dist_c(h, q), dist_r(h, q));
                assert!(bits_eq_f32(x, y), "c2Dist C={} R={}", fmt_f32(x), fmt_f32(y));
            }
            let mut poly = c2Poly::default();
            poly.count = 8;
            for k in 0..8 {
                poly.verts[k] = if wild { rng.wild_v() } else { rng.v() };
                poly.norms[k] = if wild { rng.wild_v() } else { rng.v() };
            }
            for k in 0..8 {
                unsafe {
                    let (x, y) = (pa_c(&poly, k), pa_r(&poly, k));
                    assert!(h_eq(x, y), "c2PlaneAt[{}] C={} {} R={} {}", k, fmt_v(x.n), fmt_f32(x.d), fmt_v(y.n), fmt_f32(y.d));
                }
            }
        }
    });
}

// --- row 8 : identities ---------------------------------------------------

#[test]
fn row08_identities() {
    fresh(|| {
        let p = pair();
        let (ri_c, ri_r) = sym!(p, unsafe extern "C" fn() -> c2r, "c2RotIdentity");
        let (xi_c, xi_r) = sym!(p, unsafe extern "C" fn() -> c2x, "c2xIdentity");
        unsafe {
            let (a, b) = (ri_c(), ri_r());
            assert!(bits_eq_f32(a.c, b.c) && bits_eq_f32(a.s, b.s), "c2RotIdentity");
            let (a, b) = (xi_c(), xi_r());
            assert!(x_eq(a, b), "c2xIdentity");
        }
    });
}

// --- rows 9 / 10 / 11 : rotations & transforms ----------------------------

#[test]
fn row09_row10_row11_rot_xform() {
    fresh(|| {
        let p = pair();
        let (mrv_c, mrv_r) = sym!(p, unsafe extern "C" fn(c2r, c2v) -> c2v, "c2Mulrv");
        let (mrvt_c, mrvt_r) = sym!(p, unsafe extern "C" fn(c2r, c2v) -> c2v, "c2MulrvT");
        let (mxv_c, mxv_r) = sym!(p, unsafe extern "C" fn(c2x, c2v) -> c2v, "c2Mulxv");
        let (mxvt_c, mxvt_r) = sym!(p, unsafe extern "C" fn(c2x, c2v) -> c2v, "c2MulxvT");

        let mut rng = Rng::new(0xC0FFEE_09);
        for i in 0..N {
            let mode = i % 4;
            let r = match mode {
                0 => rng.rot(),                               // unit rotation
                1 => c2r { c: rng.nice(), s: rng.nice() },    // arbitrary non-unit
                2 => c2r { c: 1.0, s: 0.0 },                  // identity
                _ => c2r { c: rng.wild(), s: rng.wild() },    // NaN / inf (row 10)
            };
            let b = if mode == 3 { rng.wild_v() } else { rng.v() };
            unsafe {
                let (x, y) = (mrv_c(r, b), mrv_r(r, b));
                assert!(v_eq(x, y), "c2Mulrv(({},{}),{}) C={} R={}", fmt_f32(r.c), fmt_f32(r.s), fmt_v(b), fmt_v(x), fmt_v(y));
                let (x, y) = (mrvt_c(r, b), mrvt_r(r, b));
                assert!(v_eq(x, y), "c2MulrvT(({},{}),{}) C={} R={}", fmt_f32(r.c), fmt_f32(r.s), fmt_v(b), fmt_v(x), fmt_v(y));
            }
            // row 11: identity / translation only / rotation only / both
            let xs = [
                c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } },
                c2x { p: rng.v(), r: c2r { c: 1.0, s: 0.0 } },
                c2x { p: c2v { x: 0.0, y: 0.0 }, r },
                c2x { p: rng.v(), r },
            ];
            for xf in xs {
                unsafe {
                    let (u, v) = (mxv_c(xf, b), mxv_r(xf, b));
                    assert!(v_eq(u, v), "c2Mulxv C={} R={}", fmt_v(u), fmt_v(v));
                    let (u, v) = (mxvt_c(xf, b), mxvt_r(xf, b));
                    assert!(v_eq(u, v), "c2MulxvT C={} R={}", fmt_v(u), fmt_v(v));
                }
            }
        }
    });
}

// --- row 12 : c2Intersect -------------------------------------------------

#[test]
fn row12_intersect() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v,
            "c2Intersect"
        );
        let mut rng = Rng::new(0xC0FFEE_12);
        for i in 0..N {
            let wild = i % 3 == 2;
            let (a, b) = if wild { (rng.wild_v(), rng.wild_v()) } else { (rng.v(), rng.v()) };
            let (da, db) = match i % 5 {
                0 => {
                    let t = rng.range(0.1, 5.0);
                    (t, -rng.range(0.1, 5.0))
                }
                1 => {
                    let t = rng.nice();
                    (t, t) // da == db -> 0/0 or x/0
                }
                2 => (0.0, rng.nice()),
                3 => (rng.wild(), rng.wild()),
                _ => (rng.nice(), rng.nice()),
            };
            unsafe {
                let (x, y) = (f_c(a, b, da, db), f_r(a, b, da, db));
                assert!(
                    v_eq(x, y),
                    "c2Intersect({},{},{},{}) C={} R={}",
                    fmt_v(a), fmt_v(b), fmt_f32(da), fmt_f32(db), fmt_v(x), fmt_v(y)
                );
            }
        }
    });
}

// --- row 13 : c2BBVerts ---------------------------------------------------

#[test]
fn row13_bbverts() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(*mut c2v, *mut c2AABB),
            "c2BBVerts"
        );
        let mut rng = Rng::new(0xC0FFEE_13);
        for i in 0..N {
            let mut bb = match i % 4 {
                0 => {
                    let a = rng.v();
                    let b = rng.v();
                    c2AABB { min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) }, max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) } }
                }
                1 => {
                    let a = rng.v();
                    c2AABB { min: a, max: a } // degenerate
                }
                2 => {
                    let a = rng.v();
                    let b = rng.v();
                    c2AABB { min: c2v { x: a.x.max(b.x), y: a.y.max(b.y) }, max: c2v { x: a.x.min(b.x), y: a.y.min(b.y) } } // inverted
                }
                _ => c2AABB { min: rng.wild_v(), max: rng.wild_v() },
            };
            let mut oc = [c2v { x: 7.0, y: 7.0 }; 4];
            let mut or_ = [c2v { x: 7.0, y: 7.0 }; 4];
            scrub_stack();
            unsafe {
                f_c(oc.as_mut_ptr(), &mut bb);
                f_r(or_.as_mut_ptr(), &mut bb);
            }
            for k in 0..4 {
                assert!(v_eq(oc[k], or_[k]), "c2BBVerts[{}] C={} R={}", k, fmt_v(oc[k]), fmt_v(or_[k]));
            }
        }
    });
}

// --- row 14 : c2Norms -----------------------------------------------------

#[test]
fn row14_norms() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(*mut c2v, *mut c2v, c_int),
            "c2Norms"
        );
        let mut rng = Rng::new(0xC0FFEE_14);
        for count in [0i32, 1, 2, 3, 4, 5, 6, 7, 8] {
            for iter in 0..300 {
                let mut verts = [c2v::default(); 8];
                for k in 0..8 {
                    verts[k] = rng.v();
                }
                if iter % 5 == 0 && count >= 2 {
                    // duplicate consecutive vertices -> zero-length edge -> NaN normal
                    verts[1] = verts[0];
                }
                let mut nc = [c2v { x: 3.5, y: -3.5 }; 8];
                let mut nr = [c2v { x: 3.5, y: -3.5 }; 8];
                let mut vc = verts;
                let mut vr = verts;
                scrub_stack();
                unsafe {
                    f_c(vc.as_mut_ptr(), nc.as_mut_ptr(), count);
                    f_r(vr.as_mut_ptr(), nr.as_mut_ptr(), count);
                }
                for k in 0..8 {
                    assert!(v_eq(nc[k], nr[k]), "c2Norms count={} norm[{}] C={} R={}", count, k, fmt_v(nc[k]), fmt_v(nr[k]));
                    assert!(v_eq(vc[k], vr[k]), "c2Norms verts mutated differently");
                }
            }
        }
    });
}

// --- row 15 : c2Support ---------------------------------------------------

#[test]
fn row15_support() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int,
            "c2Support"
        );
        let mut rng = Rng::new(0xC0FFEE_15);
        for count in [1i32, 2, 3, 4, 8] {
            for iter in 0..400 {
                let mut verts = [c2v::default(); 8];
                for k in 0..8 {
                    verts[k] = rng.v();
                }
                if iter % 7 == 0 {
                    // ties
                    for k in 0..8 {
                        verts[k] = verts[0];
                    }
                }
                let d = if iter % 5 == 4 { rng.wild_v() } else { rng.v() };
                unsafe {
                    let (x, y) = (f_c(verts.as_ptr(), count, d), f_r(verts.as_ptr(), count, d));
                    assert_eq!(x, y, "c2Support count={} d={} C={} R={}", count, fmt_v(d), x, y);
                }
            }
        }
    });
}

// --- rows 16 / 17 / 18 : c2MakeProxy --------------------------------------

#[test]
fn row16_row17_row18_makeproxy() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy),
            "c2MakeProxy"
        );
        let mut rng = Rng::new(0xC0FFEE_16);
        for _ in 0..N {
            // CIRCLE
            let circ = c2Circle { p: rng.v(), r: rng.nice() };
            // AABB
            let mut bb = c2AABB { min: rng.v(), max: rng.v() };
            // CAPSULE
            let cap = c2Capsule { a: rng.v(), b: rng.v(), r: rng.nice() };

            let seed = c2Proxy { radius: 9.25, count: -7, verts: [c2v { x: 1.5, y: -2.5 }; 8] };

            for (ty, ptr) in [
                (C2_TYPE_CIRCLE, &circ as *const _ as *const c_void),
                (C2_TYPE_AABB, &mut bb as *mut _ as *const c_void),
                (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
            ] {
                let mut pc = seed;
                let mut pr = seed;
                scrub_stack();
                unsafe {
                    f_c(ptr, ty, &mut pc);
                    f_r(ptr, ty, &mut pr);
                }
                assert!(proxy_eq(&pc, &pr), "c2MakeProxy type={} C={:?} R={:?}", ty, pc, pr);
            }
        }
    });
}

// --- rows 19..24 : simplex functions --------------------------------------

/// Builds a simplex with random support vertices; `p` is derived as `sB - sA`
/// the way `c2GJK` does, so the geometry is realistic.
fn rand_simplex(rng: &mut Rng, count: c_int) -> c2Simplex {
    let mut s = c2Simplex::default();
    let vs = [&mut s.a as *mut c2sv, &mut s.b, &mut s.c, &mut s.d];
    for (i, v) in vs.into_iter().enumerate() {
        unsafe {
            (*v).sA = rng.v();
            (*v).sB = rng.v();
            (*v).p = c2v { x: (*v).sB.x - (*v).sA.x, y: (*v).sB.y - (*v).sA.y };
            (*v).u = rng.range(-2.0, 2.0);
            (*v).iA = (i as c_int) % 4;
            (*v).iB = ((i + 1) as c_int) % 4;
        }
    }
    s.div = rng.range(-2.0, 4.0);
    s.count = count;
    s
}

#[test]
fn row19_gjk_simplex_metric() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(*mut c2Simplex) -> f32,
            "c2GJKSimplexMetric"
        );
        let mut rng = Rng::new(0xC0FFEE_19);
        for count in [1i32, 2, 3] {
            for _ in 0..600 {
                let mut sc = rand_simplex(&mut rng, count);
                let mut sr = sc;
                unsafe {
                    let (x, y) = (f_c(&mut sc), f_r(&mut sr));
                    assert!(bits_eq_f32(x, y), "c2GJKSimplexMetric count={} C={} R={}", count, fmt_f32(x), fmt_f32(y));
                }
                assert!(simplex_eq(&sc, &sr), "metric mutated simplex differently");
            }
        }
    });
}

#[test]
fn row20_c22() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(p, unsafe extern "C" fn(*mut c2Simplex), "c22");
        let mut rng = Rng::new(0xC0FFEE_20);
        let mut hit = [0usize; 3];
        for _ in 0..3000 {
            let mut sc = rand_simplex(&mut rng, 2);
            // scatter p around the origin so all three branches are reachable
            sc.a.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
            sc.b.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
            let mut sr = sc;
            scrub_stack();
            unsafe {
                f_c(&mut sc);
                f_r(&mut sr);
            }
            assert!(simplex_eq(&sc, &sr), "c22 C={:?} R={:?}", sc, sr);
            hit[(sc.count.max(1) - 1) as usize] += 1;
        }
        assert!(hit[0] > 0 && hit[1] > 0, "c22 branch coverage {:?}", hit);
    });
}

#[test]
fn row21_c23() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(p, unsafe extern "C" fn(*mut c2Simplex), "c23");
        let mut rng = Rng::new(0xC0FFEE_21);
        let mut counts = [0usize; 4];
        for _ in 0..8000 {
            let mut sc = rand_simplex(&mut rng, 3);
            let sc_ = &mut sc;
            sc_.a.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
            sc_.b.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
            sc_.c.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
            let mut sr = sc;
            scrub_stack();
            unsafe {
                f_c(&mut sc);
                f_r(&mut sr);
            }
            assert!(simplex_eq(&sc, &sr), "c23 C={:?} R={:?}", sc, sr);
            counts[sc.count.clamp(0, 3) as usize] += 1;
        }
        assert!(counts[1] > 0 && counts[2] > 0 && counts[3] > 0, "c23 branch coverage {:?}", counts);
    });
}

#[test]
fn row22_c2d() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(p, unsafe extern "C" fn(*mut c2Simplex) -> c2v, "c2D");
        let mut rng = Rng::new(0xC0FFEE_22);
        for count in [1i32, 2, 3] {
            for _ in 0..800 {
                let mut sc = rand_simplex(&mut rng, count);
                sc.a.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
                sc.b.p = c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
                let mut sr = sc;
                unsafe {
                    let (x, y) = (f_c(&mut sc), f_r(&mut sr));
                    assert!(v_eq(x, y), "c2D count={} C={} R={}", count, fmt_v(x), fmt_v(y));
                }
                assert!(simplex_eq(&sc, &sr));
            }
        }
    });
}

#[test]
fn row23_c2l() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(p, unsafe extern "C" fn(*mut c2Simplex) -> c2v, "c2L");
        let mut rng = Rng::new(0xC0FFEE_23);
        for count in [1i32, 2, 3] {
            for i in 0..800 {
                let mut sc = rand_simplex(&mut rng, count);
                if i % 11 == 0 {
                    sc.div = 0.0;
                }
                let mut sr = sc;
                unsafe {
                    let (x, y) = (f_c(&mut sc), f_r(&mut sr));
                    assert!(v_eq(x, y), "c2L count={} div={} C={} R={}", count, fmt_f32(sc.div), fmt_v(x), fmt_v(y));
                }
            }
        }
    });
}

#[test]
fn row24_witness() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = sym!(
            p,
            unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
            "c2Witness"
        );
        let mut rng = Rng::new(0xC0FFEE_24);
        for count in [1i32, 2, 3] {
            for i in 0..800 {
                let mut sc = rand_simplex(&mut rng, count);
                match i % 13 {
                    0 => sc.div = 0.0,
                    1 => sc.div = 1e-40,
                    2 => sc.div = f32::MAX,
                    _ => {}
                }
                let mut sr = sc;
                let (mut ac, mut bc) = (c2v { x: 5.0, y: 5.0 }, c2v { x: 6.0, y: 6.0 });
                let (mut ar, mut br) = (c2v { x: 5.0, y: 5.0 }, c2v { x: 6.0, y: 6.0 });
                scrub_stack();
                unsafe {
                    f_c(&mut sc, &mut ac, &mut bc);
                    f_r(&mut sr, &mut ar, &mut br);
                }
                assert!(v_eq(ac, ar), "c2Witness a count={} div={} C={} R={}", count, fmt_f32(sc.div), fmt_v(ac), fmt_v(ar));
                assert!(v_eq(bc, br), "c2Witness b count={} div={} C={} R={}", count, fmt_f32(sc.div), fmt_v(bc), fmt_v(br));
            }
        }
    });
}
