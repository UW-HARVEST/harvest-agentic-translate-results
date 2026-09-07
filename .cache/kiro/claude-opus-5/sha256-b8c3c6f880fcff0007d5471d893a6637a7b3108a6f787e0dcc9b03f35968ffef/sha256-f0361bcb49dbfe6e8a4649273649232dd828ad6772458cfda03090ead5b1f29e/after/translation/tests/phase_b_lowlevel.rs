//! Phase B — valid-path differential tests for the low-level entry points.
//! CONFIGS.md rows 1–22.
//!
//! Every call goes through a `.so` export loaded with `libloading`: the C one
//! and the Rust one. Nothing is called directly.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_int, c_void};

// --- row 1 & 2: elementwise vector math -----------------------------------

#[test]
fn row01_row02_basic_vector_math() {
    let (c, r) = load();

    let c2V_c: libloading::Symbol<FnV> = c.sym("c2V");
    let c2V_r: libloading::Symbol<FnV> = r.sym("c2V");
    let mulvs_c: libloading::Symbol<FnVvf> = c.sym("c2Mulvs");
    let mulvs_r: libloading::Symbol<FnVvf> = r.sym("c2Mulvs");
    let sub_c: libloading::Symbol<FnVvv> = c.sym("c2Sub");
    let sub_r: libloading::Symbol<FnVvv> = r.sym("c2Sub");
    let add_c: libloading::Symbol<FnVvv> = c.sym("c2Add");
    let add_r: libloading::Symbol<FnVvv> = r.sym("c2Add");
    let dot_c: libloading::Symbol<FnFvv> = c.sym("c2Dot");
    let dot_r: libloading::Symbol<FnFvv> = r.sym("c2Dot");
    let det_c: libloading::Symbol<FnFvv> = c.sym("c2Det2");
    let det_r: libloading::Symbol<FnFvv> = r.sym("c2Det2");
    let neg_c: libloading::Symbol<FnVv> = c.sym("c2Neg");
    let neg_r: libloading::Symbol<FnVv> = r.sym("c2Neg");
    let skew_c: libloading::Symbol<FnVv> = c.sym("c2Skew");
    let skew_r: libloading::Symbol<FnVv> = r.sym("c2Skew");
    let ccw_c: libloading::Symbol<FnVv> = c.sym("c2CCW90");
    let ccw_r: libloading::Symbol<FnVv> = r.sym("c2CCW90");

    let mut rng = Rng::new(SEED);
    // pass 0 = row 1 (finite, wide exponent range); pass 1 = row 2 (NaN/inf/±0)
    for pass in 0..2 {
        for i in 0..N * 4 {
            let (a, b, s) = if pass == 0 {
                (rng.wide_v(), rng.wide_v(), rng.wide_f32())
            } else {
                (rng.any_v(), rng.any_v(), rng.any_f32())
            };
            unsafe {
                assert!(veq(c2V_c(a.x, a.y), c2V_r(a.x, a.y)), "c2V pass{pass} i{i}");
                assert!(
                    veq(mulvs_c(a, s), mulvs_r(a, s)),
                    "c2Mulvs pass{pass} i{i}: a={} s={} C={} R={}",
                    vdesc(a), fdesc(s), vdesc(mulvs_c(a, s)), vdesc(mulvs_r(a, s))
                );
                assert!(veq(sub_c(a, b), sub_r(a, b)), "c2Sub pass{pass} i{i}");
                assert!(veq(add_c(a, b), add_r(a, b)), "c2Add pass{pass} i{i}");
                assert!(
                    feq(dot_c(a, b), dot_r(a, b)),
                    "c2Dot pass{pass} i{i}: a={} b={} C={} R={}",
                    vdesc(a), vdesc(b), fdesc(dot_c(a, b)), fdesc(dot_r(a, b))
                );
                assert!(feq(det_c(a, b), det_r(a, b)), "c2Det2 pass{pass} i{i}");
                assert!(veq(neg_c(a), neg_r(a)), "c2Neg pass{pass} i{i}: a={}", vdesc(a));
                assert!(veq(skew_c(a), skew_r(a)), "c2Skew pass{pass} i{i}");
                assert!(veq(ccw_c(a), ccw_r(a)), "c2CCW90 pass{pass} i{i}");
            }
        }
    }
}

// --- rows 3 & 4: min / max / clamp ----------------------------------------

#[test]
fn row03_row04_min_max_clamp() {
    let (c, r) = load();
    let max_c: libloading::Symbol<FnVvv> = c.sym("c2Maxv");
    let max_r: libloading::Symbol<FnVvv> = r.sym("c2Maxv");
    let min_c: libloading::Symbol<FnVvv> = c.sym("c2Minv");
    let min_r: libloading::Symbol<FnVvv> = r.sym("c2Minv");
    let cl_c: libloading::Symbol<FnVvvv> = c.sym("c2Clampv");
    let cl_r: libloading::Symbol<FnVvvv> = r.sym("c2Clampv");

    let mut rng = Rng::new(SEED ^ 3);
    for pass in 0..2 {
        for i in 0..N * 4 {
            let (a, mut lo, mut hi) = if pass == 0 {
                (rng.wide_v(), rng.wide_v(), rng.wide_v())
            } else {
                (rng.any_v(), rng.any_v(), rng.any_v())
            };
            if pass == 0 {
                // row 3: ordered range lo <= hi
                if lo.x > hi.x {
                    std::mem::swap(&mut lo.x, &mut hi.x);
                }
                if lo.y > hi.y {
                    std::mem::swap(&mut lo.y, &mut hi.y);
                }
            }
            unsafe {
                assert!(
                    veq(max_c(a, lo), max_r(a, lo)),
                    "c2Maxv pass{pass} i{i}: a={} b={} C={} R={}",
                    vdesc(a), vdesc(lo), vdesc(max_c(a, lo)), vdesc(max_r(a, lo))
                );
                assert!(veq(min_c(a, lo), min_r(a, lo)), "c2Minv pass{pass} i{i}");
                // also with the NaN operand on the left
                assert!(veq(max_c(lo, a), max_r(lo, a)), "c2Maxv rev pass{pass} i{i}");
                assert!(veq(min_c(lo, a), min_r(lo, a)), "c2Minv rev pass{pass} i{i}");
                assert!(
                    veq(cl_c(a, lo, hi), cl_r(a, lo, hi)),
                    "c2Clampv pass{pass} i{i}: a={} lo={} hi={}",
                    vdesc(a), vdesc(lo), vdesc(hi)
                );
                // inverted range (row 4)
                assert!(veq(cl_c(a, hi, lo), cl_r(a, hi, lo)), "c2Clampv inv pass{pass} i{i}");
            }
        }
    }
}

// --- rows 5 & 6: length / divide / normalize -------------------------------

#[test]
fn row05_row06_len_div_norm() {
    let (c, r) = load();
    let len_c: libloading::Symbol<FnFv> = c.sym("c2Len");
    let len_r: libloading::Symbol<FnFv> = r.sym("c2Len");
    let div_c: libloading::Symbol<FnVvf> = c.sym("c2Div");
    let div_r: libloading::Symbol<FnVvf> = r.sym("c2Div");
    let norm_c: libloading::Symbol<FnVv> = c.sym("c2Norm");
    let norm_r: libloading::Symbol<FnVv> = r.sym("c2Norm");

    // Row 6 explicit edge inputs: zero, huge (Dot overflows), subnormal, NaN.
    let edge: [(c2v, f32); 10] = [
        (c2v { x: 0.0, y: 0.0 }, 0.0),
        (c2v { x: -0.0, y: 0.0 }, -0.0),
        (c2v { x: f32::MAX, y: f32::MAX }, 1.0),
        (c2v { x: f32::MIN_POSITIVE, y: f32::MIN_POSITIVE }, f32::MIN_POSITIVE),
        (c2v { x: f32::from_bits(1), y: 0.0 }, f32::from_bits(1)),
        (c2v { x: f32::NAN, y: 1.0 }, f32::NAN),
        (c2v { x: f32::INFINITY, y: 0.0 }, f32::INFINITY),
        (c2v { x: f32::NEG_INFINITY, y: f32::INFINITY }, f32::NEG_INFINITY),
        (c2v { x: 3.0, y: 4.0 }, 5.0),
        (c2v { x: -1e-30, y: 1e-30 }, 1e30),
    ];
    unsafe {
        for (i, (v, d)) in edge.iter().enumerate() {
            assert!(feq(len_c(*v), len_r(*v)), "c2Len edge{i}: v={}", vdesc(*v));
            assert!(
                veq(div_c(*v, *d), div_r(*v, *d)),
                "c2Div edge{i}: v={} d={} C={} R={}",
                vdesc(*v), fdesc(*d), vdesc(div_c(*v, *d)), vdesc(div_r(*v, *d))
            );
            assert!(
                veq(norm_c(*v), norm_r(*v)),
                "c2Norm edge{i}: v={} C={} R={}",
                vdesc(*v), vdesc(norm_c(*v)), vdesc(norm_r(*v))
            );
        }
    }

    let mut rng = Rng::new(SEED ^ 5);
    for pass in 0..2 {
        for i in 0..N * 4 {
            let (v, d) = if pass == 0 {
                (rng.wide_v(), rng.wide_f32())
            } else {
                (rng.any_v(), rng.any_f32())
            };
            unsafe {
                assert!(feq(len_c(v), len_r(v)), "c2Len pass{pass} i{i}: v={}", vdesc(v));
                assert!(veq(div_c(v, d), div_r(v, d)), "c2Div pass{pass} i{i}");
                assert!(veq(norm_c(v), norm_r(v)), "c2Norm pass{pass} i{i}: v={}", vdesc(v));
            }
        }
    }
}

// --- row 7: identity constructors (struct-return ABI) ---------------------

#[test]
fn row07_identities() {
    let (c, r) = load();
    let rot_c: libloading::Symbol<FnR> = c.sym("c2RotIdentity");
    let rot_r: libloading::Symbol<FnR> = r.sym("c2RotIdentity");
    let x_c: libloading::Symbol<FnX> = c.sym("c2xIdentity");
    let x_r: libloading::Symbol<FnX> = r.sym("c2xIdentity");
    unsafe {
        assert!(req(rot_c(), rot_r()), "c2RotIdentity");
        let (a, b) = (x_c(), x_r());
        assert!(veq(a.p, b.p) && req(a.r, b.r), "c2xIdentity");
    }
}

// --- rows 8 & 9: rotations and transforms ---------------------------------

#[test]
fn row08_row09_rotations() {
    let (c, r) = load();
    let mulrv_c: libloading::Symbol<FnVrv> = c.sym("c2Mulrv");
    let mulrv_r: libloading::Symbol<FnVrv> = r.sym("c2Mulrv");
    let mulrvT_c: libloading::Symbol<FnVrv> = c.sym("c2MulrvT");
    let mulrvT_r: libloading::Symbol<FnVrv> = r.sym("c2MulrvT");
    let mulxv_c: libloading::Symbol<FnVxv> = c.sym("c2Mulxv");
    let mulxv_r: libloading::Symbol<FnVxv> = r.sym("c2Mulxv");

    let mut rng = Rng::new(SEED ^ 8);
    for pass in 0..2 {
        for i in 0..N * 4 {
            let (rot, v, p) = if pass == 0 {
                // row 8: proper unit rotations
                let t = rng.unit() * std::f32::consts::TAU;
                (c2r { c: t.cos(), s: t.sin() }, rng.geo_v(1000.0), rng.geo_v(1000.0))
            } else {
                // row 9: unvalidated / zero / NaN rotations
                (c2r { c: rng.any_f32(), s: rng.any_f32() }, rng.any_v(), rng.any_v())
            };
            let x = c2x { p, r: rot };
            unsafe {
                assert!(
                    veq(mulrv_c(rot, v), mulrv_r(rot, v)),
                    "c2Mulrv pass{pass} i{i}: r=({},{}) v={}",
                    fdesc(rot.c), fdesc(rot.s), vdesc(v)
                );
                assert!(
                    veq(mulrvT_c(rot, v), mulrvT_r(rot, v)),
                    "c2MulrvT pass{pass} i{i}: r=({},{}) v={}",
                    fdesc(rot.c), fdesc(rot.s), vdesc(v)
                );
                assert!(
                    veq(mulxv_c(x, v), mulxv_r(x, v)),
                    "c2Mulxv pass{pass} i{i}: p={} r=({},{}) v={}",
                    vdesc(p), fdesc(rot.c), fdesc(rot.s), vdesc(v)
                );
            }
        }
    }
}

// --- rows 10 & 11: c2BBVerts ----------------------------------------------

#[test]
fn row10_row11_bbverts() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnBBVerts> = c.sym("c2BBVerts");
    let f_r: libloading::Symbol<FnBBVerts> = r.sym("c2BBVerts");

    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N * 4 {
        // pass 0: min<max, pass 1: degenerate min==max, pass 2: inverted, pass 3: anything
        let mut bb = match i % 4 {
            0 => {
                let a = rng.geo_v(500.0);
                let d = c2v { x: rng.unit() * 100.0, y: rng.unit() * 100.0 };
                c2AABB { min: a, max: c2v { x: a.x + d.x, y: a.y + d.y } }
            }
            1 => {
                let a = rng.geo_v(500.0);
                c2AABB { min: a, max: a }
            }
            2 => {
                let a = rng.geo_v(500.0);
                let b = rng.geo_v(500.0);
                c2AABB {
                    min: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
                    max: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                }
            }
            _ => c2AABB { min: rng.any_v(), max: rng.any_v() },
        };
        let mut oc = [c2v { x: 7.5, y: -7.5 }; 4];
        let mut or_ = oc;
        unsafe {
            f_c(oc.as_mut_ptr(), &mut bb);
            f_r(or_.as_mut_ptr(), &mut bb);
        }
        for k in 0..4 {
            assert!(
                veq(oc[k], or_[k]),
                "c2BBVerts i{i} vert{k}: min={} max={} C={} R={}",
                vdesc(bb.min), vdesc(bb.max), vdesc(oc[k]), vdesc(or_[k])
            );
        }
    }
}

// --- rows 12, 13, 14: c2MakeProxy for each shape type ---------------------

#[test]
fn row12_row13_row14_make_proxy() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnMakeProxy> = c.sym("c2MakeProxy");
    let f_r: libloading::Symbol<FnMakeProxy> = r.sym("c2MakeProxy");

    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..N * 3 {
        // Poison the proxies identically so "unwritten" fields are comparable.
        let poison = c2Proxy {
            radius: -123.5,
            count: -9,
            verts: [c2v { x: 1.25, y: -1.25 }; 8],
        };
        let mut pc = poison;
        let mut pr = poison;

        // row 12: circle
        let circ = c2Circle {
            p: rng.geo_v(200.0),
            r: match i % 4 {
                0 => 0.0,
                1 => -rng.unit() * 10.0,
                2 => rng.any_f32(),
                _ => rng.unit() * 50.0,
            },
        };
        unsafe {
            f_c(&circ as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pc);
            f_r(&circ as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
        }
        assert!(proxy_eq(&pc, &pr), "c2MakeProxy CIRCLE i{i}: {:?} vs {:?}", pc, pr);

        // row 13: aabb
        let mut pc = poison;
        let mut pr = poison;
        let bb = match i % 3 {
            0 => {
                let a = rng.geo_v(200.0);
                c2AABB { min: a, max: a }
            }
            1 => c2AABB { min: rng.geo_v(200.0), max: rng.geo_v(200.0) },
            _ => c2AABB { min: rng.any_v(), max: rng.any_v() },
        };
        unsafe {
            f_c(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
            f_r(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
        }
        assert!(proxy_eq(&pc, &pr), "c2MakeProxy AABB i{i}: {:?} vs {:?}", pc, pr);

        // row 14: capsule
        let mut pc = poison;
        let mut pr = poison;
        let a = rng.geo_v(200.0);
        let cap = c2Capsule {
            a,
            b: if i % 5 == 0 { a } else { rng.geo_v(200.0) },
            r: if i % 7 == 0 { 0.0 } else { rng.unit() * 30.0 },
        };
        unsafe {
            f_c(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pc);
            f_r(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
        }
        assert!(proxy_eq(&pc, &pr), "c2MakeProxy CAPSULE i{i}: {:?} vs {:?}", pc, pr);
    }
}

// --- simplex construction helper ------------------------------------------

/// Builds a random simplex. `count` is stored verbatim (may be out of range).
pub fn rand_simplex(rng: &mut Rng, count: c_int, wild: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    for k in 0..4 {
        s.verts[k] = c2sv {
            sA: if wild { rng.any_v() } else { rng.geo_v(100.0) },
            sB: if wild { rng.any_v() } else { rng.geo_v(100.0) },
            p: if wild { rng.any_v() } else { rng.geo_v(100.0) },
            u: if wild { rng.any_f32() } else { rng.unit() * 10.0 },
            iA: rng.below(8) as c_int,
            iB: rng.below(8) as c_int,
        };
    }
    s.div = if wild {
        rng.any_f32()
    } else {
        match rng.below(4) {
            0 => 1.0,
            1 => 0.0,
            2 => rng.unit() * 1e6,
            _ => rng.unit() * 30.0 + 0.5,
        }
    };
    s.count = count;
    s
}

// --- row 15: c2GJKSimplexMetric -------------------------------------------

#[test]
fn row15_simplex_metric() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexF> = c.sym("c2GJKSimplexMetric");
    let f_r: libloading::Symbol<FnSimplexF> = r.sym("c2GJKSimplexMetric");

    let mut rng = Rng::new(SEED ^ 15);
    for count in [1, 2, 3] {
        for i in 0..N * 2 {
            let mut sc = rand_simplex(&mut rng, count, i % 3 == 0);
            let mut sr = sc;
            let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
            assert!(
                feq(vc, vr),
                "c2GJKSimplexMetric count{count} i{i}: C={} R={}",
                fdesc(vc), fdesc(vr)
            );
            assert!(simplex_eq(&sc, &sr), "metric mutated simplex differently count{count} i{i}");
        }
    }
}

// --- row 16: c2L ----------------------------------------------------------

#[test]
fn row16_c2L() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexV> = c.sym("c2L");
    let f_r: libloading::Symbol<FnSimplexV> = r.sym("c2L");

    let mut rng = Rng::new(SEED ^ 16);
    for count in [1, 2] {
        for i in 0..N * 2 {
            let mut sc = rand_simplex(&mut rng, count, i % 3 == 0);
            let mut sr = sc;
            let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
            assert!(
                veq(vc, vr),
                "c2L count{count} i{i}: div={} u0={} u1={} C={} R={}",
                fdesc(sc.div), fdesc(sc.verts[0].u), fdesc(sc.verts[1].u), vdesc(vc), vdesc(vr)
            );
        }
    }
}

// --- row 17: c2D ----------------------------------------------------------

#[test]
fn row17_c2D() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexV> = c.sym("c2D");
    let f_r: libloading::Symbol<FnSimplexV> = r.sym("c2D");
    let det_c: libloading::Symbol<FnFvv> = c.sym("c2Det2");

    let mut rng = Rng::new(SEED ^ 17);
    let mut skew_hits = 0usize;
    let mut ccw_hits = 0usize;
    for count in [1, 2] {
        for i in 0..N * 3 {
            let mut sc = rand_simplex(&mut rng, count, i % 4 == 0);
            let mut sr = sc;
            if count == 2 {
                let ab = c2v {
                    x: sc.verts[1].p.x - sc.verts[0].p.x,
                    y: sc.verts[1].p.y - sc.verts[0].p.y,
                };
                let na = c2v { x: -sc.verts[0].p.x, y: -sc.verts[0].p.y };
                let d = unsafe { det_c(ab, na) };
                if d > 0.0 {
                    skew_hits += 1;
                } else {
                    ccw_hits += 1;
                }
            }
            let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
            assert!(veq(vc, vr), "c2D count{count} i{i}: C={} R={}", vdesc(vc), vdesc(vr));
        }
    }
    assert!(skew_hits > 20 && ccw_hits > 20, "c2D branches not both covered: {skew_hits}/{ccw_hits}");
}

// --- row 18: c2Witness ----------------------------------------------------

#[test]
fn row18_witness() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnWitness> = c.sym("c2Witness");
    let f_r: libloading::Symbol<FnWitness> = r.sym("c2Witness");

    let mut rng = Rng::new(SEED ^ 18);
    for count in [1, 2, 3] {
        for i in 0..N * 2 {
            let mut sc = rand_simplex(&mut rng, count, i % 3 == 0);
            let mut sr = sc;
            let (mut ac, mut bc) = (c2v { x: 9.0, y: 9.0 }, c2v { x: -9.0, y: -9.0 });
            let (mut ar, mut br) = (ac, bc);
            unsafe {
                f_c(&mut sc, &mut ac, &mut bc);
                f_r(&mut sr, &mut ar, &mut br);
            }
            assert!(
                veq(ac, ar) && veq(bc, br),
                "c2Witness count{count} i{i}: div={} C=({},{}) R=({},{})",
                fdesc(sc.div), vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br)
            );
        }
    }
}

// --- row 19: c22 ----------------------------------------------------------

#[test]
fn row19_c22() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplex> = c.sym("c22");
    let f_r: libloading::Symbol<FnSimplex> = r.sym("c22");
    let dot: libloading::Symbol<FnFvv> = c.sym("c2Dot");
    let sub: libloading::Symbol<FnVvv> = c.sym("c2Sub");

    let mut rng = Rng::new(SEED ^ 19);
    let mut arms = [0usize; 3];
    for i in 0..N * 8 {
        let wild = i % 6 == 0;
        let mut sc = rand_simplex(&mut rng, 2, wild);
        if !wild {
            // Bias the p values so all three arms of c22 get hit.
            let scale = match i % 3 {
                0 => 1.0,
                1 => 1e-4,
                _ => 1e4,
            };
            sc.verts[0].p = c2v { x: rng.sym(scale), y: rng.sym(scale) };
            sc.verts[1].p = c2v { x: rng.sym(scale), y: rng.sym(scale) };
            if i % 7 == 0 {
                sc.verts[1].p = sc.verts[0].p; // degenerate: a == b
            }
        }
        let mut sr = sc;
        unsafe {
            let a = sc.verts[0].p;
            let b = sc.verts[1].p;
            let u = dot(b, sub(b, a));
            let v = dot(a, sub(a, b));
            if v <= 0.0 {
                arms[0] += 1;
            } else if u <= 0.0 {
                arms[1] += 1;
            } else {
                arms[2] += 1;
            }
            f_c(&mut sc);
            f_r(&mut sr);
        }
        assert!(simplex_eq(&sc, &sr), "c22 i{i}: C={:?} R={:?}", sc, sr);
    }
    assert!(arms.iter().all(|&n| n > 10), "c22 arms not all covered: {arms:?}");
}

// --- rows 20 & 21: c23 ----------------------------------------------------

#[test]
fn row20_row21_c23() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplex> = c.sym("c23");
    let f_r: libloading::Symbol<FnSimplex> = r.sym("c23");

    let mut rng = Rng::new(SEED ^ 20);
    let mut counts = [0usize; 4]; // resulting s.count histogram (index 0 unused)
    for i in 0..N * 10 {
        let wild = i % 8 == 0;
        let mut sc = rand_simplex(&mut rng, 3, wild);
        if !wild {
            let scale = match i % 4 {
                0 => 1.0,
                1 => 1e-3,
                2 => 1e3,
                _ => 50.0,
            };
            for k in 0..3 {
                sc.verts[k].p = c2v { x: rng.sym(scale), y: rng.sym(scale) };
            }
            match i % 11 {
                // row 21: collinear (area == 0)
                0 => {
                    let a = sc.verts[0].p;
                    let d = c2v { x: rng.sym(scale), y: rng.sym(scale) };
                    let t1 = rng.sym(3.0);
                    let t2 = rng.sym(3.0);
                    sc.verts[1].p = c2v { x: a.x + d.x * t1, y: a.y + d.y * t1 };
                    sc.verts[2].p = c2v { x: a.x + d.x * t2, y: a.y + d.y * t2 };
                }
                // row 21: duplicated points
                1 => sc.verts[2].p = sc.verts[1].p,
                2 => sc.verts[1].p = sc.verts[0].p,
                3 => {
                    sc.verts[1].p = sc.verts[0].p;
                    sc.verts[2].p = sc.verts[0].p;
                }
                _ => {}
            }
        }
        let mut sr = sc;
        unsafe {
            f_c(&mut sc);
            f_r(&mut sr);
        }
        assert!(simplex_eq(&sc, &sr), "c23 i{i}: C={:?} R={:?}", sc, sr);
        if (1..=3).contains(&sc.count) {
            counts[sc.count as usize] += 1;
        }
    }
    assert!(
        counts[1] > 10 && counts[2] > 10 && counts[3] > 10,
        "c23 outcomes not all covered: {counts:?}"
    );
}

// --- row 22: c2Support ----------------------------------------------------

#[test]
fn row22_support() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSupport> = c.sym("c2Support");
    let f_r: libloading::Symbol<FnSupport> = r.sym("c2Support");

    let mut rng = Rng::new(SEED ^ 22);
    for count in [1i32, 2, 4, 8] {
        for i in 0..N * 2 {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = if i % 5 == 0 { rng.any_v() } else { rng.geo_v(100.0) };
            }
            if i % 9 == 0 {
                // ties: all verts identical -> `dot > dmax` never true
                for k in 1..8 {
                    verts[k] = verts[0];
                }
            }
            let d = match i % 6 {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => rng.any_v(),
                _ => rng.geo_v(10.0),
            };
            let (ic, ir) = unsafe { (f_c(verts.as_ptr(), count, d), f_r(verts.as_ptr(), count, d)) };
            assert_eq!(ic, ir, "c2Support count{count} i{i}: d={}", vdesc(d));
        }
    }
}
