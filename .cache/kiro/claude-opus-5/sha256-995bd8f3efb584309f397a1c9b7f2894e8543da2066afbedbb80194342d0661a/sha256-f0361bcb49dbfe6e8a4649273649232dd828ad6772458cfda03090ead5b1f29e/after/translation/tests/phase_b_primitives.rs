//! Phase B — CONFIGS.md rows 1..21: the lowest-level entry points.
//! Every call goes through `dlsym` on both `.so` files.

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 20_000;

// ---------------------------------------------------------------------- row 1/2
#[test]
fn row01_row02_scalar_vector_primitives() {
    let l = libs();
    let (c_v, r_v) = l.pair::<FnVff>("c2V");
    let (c_mulvs, r_mulvs) = l.pair::<FnVvf>("c2Mulvs");
    let (c_sub, r_sub) = l.pair::<FnVvv>("c2Sub");
    let (c_add, r_add) = l.pair::<FnVvv>("c2Add");
    let (c_neg, r_neg) = l.pair::<FnVv>("c2Neg");
    let (c_skew, r_skew) = l.pair::<FnVv>("c2Skew");
    let (c_ccw, r_ccw) = l.pair::<FnVv>("c2CCW90");
    let (c_absv, r_absv) = l.pair::<FnVv>("c2Absv");
    let (c_dot, r_dot) = l.pair::<FnFvv>("c2Dot");
    let (c_det, r_det) = l.pair::<FnFvv>("c2Det2");
    let (c_len, r_len) = l.pair::<FnFv>("c2Len");
    let (c_div, r_div) = l.pair::<FnVvf>("c2Div");

    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for i in 0..N {
        // Half the iterations use ordinary finite values (row 1), half use the
        // full "wild" set: signed zeros, subnormals, inf, NaN (row 2).
        let (a, b, s) = if i % 2 == 0 {
            (rng.vec(1e4), rng.vec(1e4), rng.f(1e3))
        } else {
            (rng.wild_vec(), rng.wild_vec(), rng.wild())
        };

        unsafe {
            assert!(veq(c_v(a.x, a.y), r_v(a.x, a.y)), "c2V {i}");
            assert!(
                veq(c_mulvs(a, s), r_mulvs(a, s)),
                "c2Mulvs {i}: {} * {} -> C {} vs R {}",
                vs(a),
                fs(s),
                vs(c_mulvs(a, s)),
                vs(r_mulvs(a, s))
            );
            assert!(veq(c_sub(a, b), r_sub(a, b)), "c2Sub {i}");
            assert!(veq(c_add(a, b), r_add(a, b)), "c2Add {i}");
            assert!(veq(c_neg(a), r_neg(a)), "c2Neg {i}: {}", vs(a));
            assert!(veq(c_skew(a), r_skew(a)), "c2Skew {i}");
            assert!(veq(c_ccw(a), r_ccw(a)), "c2CCW90 {i}");
            assert!(
                veq(c_absv(a), r_absv(a)),
                "c2Absv {i}: {} -> C {} vs R {}",
                vs(a),
                vs(c_absv(a)),
                vs(r_absv(a))
            );
            assert!(feq(c_dot(a, b), r_dot(a, b)), "c2Dot {i}");
            assert!(feq(c_det(a, b), r_det(a, b)), "c2Det2 {i}");
            assert!(
                feq(c_len(a), r_len(a)),
                "c2Len {i}: {} -> C {} vs R {}",
                vs(a),
                fs(c_len(a)),
                fs(r_len(a))
            );
            assert!(
                veq(c_div(a, s), r_div(a, s)),
                "c2Div {i}: {} / {}",
                vs(a),
                fs(s)
            );
        }
    }
}

// ------------------------------------------------------------------------ row 3
#[test]
fn row03_min_max_clamp() {
    let l = libs();
    let (c_max, r_max) = l.pair::<FnVvv>("c2Maxv");
    let (c_min, r_min) = l.pair::<FnVvv>("c2Minv");
    let (c_clamp, r_clamp) = l.pair::<FnVvvv>("c2Clampv");

    let mut rng = Rng::new(0xAAAA_BBBB_CCCC_DDDD);
    for i in 0..N {
        let (a, lo, hi) = match i % 4 {
            0 => (rng.vec(100.0), rng.vec(100.0), rng.vec(100.0)),
            // equal components
            1 => {
                let x = rng.vec(10.0);
                (x, x, x)
            }
            // inverted range: lo > hi
            2 => (rng.vec(50.0), v(10.0, 10.0), v(-10.0, -10.0)),
            // NaN / inf / signed zero operands
            _ => (rng.wild_vec(), rng.wild_vec(), rng.wild_vec()),
        };
        unsafe {
            assert!(
                veq(c_max(a, lo), r_max(a, lo)),
                "c2Maxv {i}: {} {} -> C {} vs R {}",
                vs(a),
                vs(lo),
                vs(c_max(a, lo)),
                vs(r_max(a, lo))
            );
            assert!(
                veq(c_min(a, lo), r_min(a, lo)),
                "c2Minv {i}: {} {} -> C {} vs R {}",
                vs(a),
                vs(lo),
                vs(c_min(a, lo)),
                vs(r_min(a, lo))
            );
            assert!(
                veq(c_clamp(a, lo, hi), r_clamp(a, lo, hi)),
                "c2Clampv {i}: {} {} {} -> C {} vs R {}",
                vs(a),
                vs(lo),
                vs(hi),
                vs(c_clamp(a, lo, hi)),
                vs(r_clamp(a, lo, hi))
            );
        }
    }
}

// ------------------------------------------------------------------------ row 4
#[test]
fn row04_norm() {
    let l = libs();
    let (c_norm, r_norm) = l.pair::<FnVv>("c2Norm");
    let mut rng = Rng::new(0x0F0F_0F0F_0F0F_0F0F);
    for i in 0..N {
        let a = match i % 5 {
            0 => rng.vec(1.0),
            1 => rng.vec(1e6),
            2 => rng.vec(1e-30), // near-zero magnitude
            3 => rng.vec(1e30),  // squares overflow to inf
            _ => rng.wild_vec(),
        };
        unsafe {
            assert!(
                veq(c_norm(a), r_norm(a)),
                "c2Norm {i}: {} -> C {} vs R {}",
                vs(a),
                vs(c_norm(a)),
                vs(r_norm(a))
            );
        }
    }
}

// ------------------------------------------------------------------------ row 5
#[test]
fn row05_intersect() {
    let l = libs();
    let (c_i, r_i) = l.pair::<FnIntersect>("c2Intersect");
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for i in 0..N {
        let a = rng.vec(100.0);
        let b = rng.vec(100.0);
        let (da, db) = match i % 4 {
            0 => (rng.f(10.0), rng.f(10.0)),
            1 => {
                let x = rng.f(10.0);
                (x, x) // da == db -> division by zero
            }
            2 => (0.0, rng.f(10.0)),
            _ => (rng.wild(), rng.wild()),
        };
        unsafe {
            assert!(
                veq(c_i(a, b, da, db), r_i(a, b, da, db)),
                "c2Intersect {i}: {} {} {} {} -> C {} vs R {}",
                vs(a),
                vs(b),
                fs(da),
                fs(db),
                vs(c_i(a, b, da, db)),
                vs(r_i(a, b, da, db))
            );
        }
    }
}

// ------------------------------------------------------------------------ row 6
#[test]
fn row06_dist_and_plane_at() {
    let l = libs();
    let (c_dist, r_dist) = l.pair::<FnFhv>("c2Dist");
    let (c_pa, r_pa) = l.pair::<FnHpi>("c2PlaneAt");
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0001);
    for i in 0..N {
        let h = c2h {
            n: if i % 3 == 0 { rng.wild_vec() } else { rng.vec(1.0) },
            d: if i % 3 == 0 { rng.wild() } else { rng.f(10.0) },
        };
        let p = if i % 3 == 0 { rng.wild_vec() } else { rng.vec(100.0) };
        unsafe {
            assert!(
                feq(c_dist(h, p), r_dist(h, p)),
                "c2Dist {i}: n={} d={} p={}",
                vs(h.n),
                fs(h.d),
                vs(p)
            );
        }

        let mut poly = c2Poly::default();
        poly.count = 3 + (i % 6) as c_int;
        for k in 0..8 {
            poly.verts[k] = rng.vec(10.0);
            poly.norms[k] = rng.vec(1.0);
        }
        // Indices 0..7 are all in bounds of the fixed 8-element arrays.
        let idx = (i % 8) as c_int;
        unsafe {
            let a = c_pa(&poly, idx);
            let b = r_pa(&poly, idx);
            assert!(
                heq(a, b),
                "c2PlaneAt {i} idx={idx}: C n={} d={} vs R n={} d={}",
                vs(a.n),
                fs(a.d),
                vs(b.n),
                fs(b.d)
            );
        }
    }
}

// ------------------------------------------------------------------------ row 7
#[test]
fn row07_identities() {
    let l = libs();
    let (c_ri, r_ri) = l.pair::<FnR>("c2RotIdentity");
    let (c_xi, r_xi) = l.pair::<FnX>("c2xIdentity");
    unsafe {
        let (a, b) = (c_ri(), r_ri());
        assert!(feq(a.c, b.c) && feq(a.s, b.s), "c2RotIdentity");
        let (a, b) = (c_xi(), r_xi());
        assert!(
            veq(a.p, b.p) && feq(a.r.c, b.r.c) && feq(a.r.s, b.r.s),
            "c2xIdentity"
        );
    }
}

// ------------------------------------------------------------------ rows 8 / 9
#[test]
fn row08_row09_rotations_and_transforms() {
    let l = libs();
    let (c_mrv, r_mrv) = l.pair::<FnVrv>("c2Mulrv");
    let (c_mrvt, r_mrvt) = l.pair::<FnVrv>("c2MulrvT");
    let (c_mxv, r_mxv) = l.pair::<FnVxv>("c2Mulxv");
    let (c_mxvt, r_mxvt) = l.pair::<FnVxv>("c2MulxvT");
    let mut rng = Rng::new(0x5555_6666_7777_8888);

    for i in 0..N {
        let rot = match i % 5 {
            0 => rng.rot(),                                   // unit
            1 => c2r { c: 1.0, s: 0.0 },                       // identity
            2 => c2r { c: 0.0, s: 0.0 },                       // zero
            3 => c2r { c: rng.f(10.0), s: rng.f(10.0) },        // non-unit
            _ => c2r { c: rng.wild(), s: rng.wild() },         // wild
        };
        let p = match i % 4 {
            0 => v(0.0, 0.0), // pure rotation
            1 => rng.vec(100.0),
            2 => rng.vec(1e6),
            _ => rng.wild_vec(),
        };
        let x = c2x { p, r: rot };
        let b = if i % 7 == 0 { rng.wild_vec() } else { rng.vec(100.0) };

        unsafe {
            assert!(veq(c_mrv(rot, b), r_mrv(rot, b)), "c2Mulrv {i}");
            assert!(veq(c_mrvt(rot, b), r_mrvt(rot, b)), "c2MulrvT {i}");
            assert!(
                veq(c_mxv(x, b), r_mxv(x, b)),
                "c2Mulxv {i}: C {} vs R {}",
                vs(c_mxv(x, b)),
                vs(r_mxv(x, b))
            );
            assert!(
                veq(c_mxvt(x, b), r_mxvt(x, b)),
                "c2MulxvT {i}: C {} vs R {}",
                vs(c_mxvt(x, b)),
                vs(r_mxvt(x, b))
            );
        }
    }
}

// ----------------------------------------------------------------------- row 10
#[test]
fn row10_bb_verts() {
    let l = libs();
    let (c_bb, r_bb) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0x9999_1111_2222_3333);
    for i in 0..N {
        let mut bb = match i % 4 {
            0 => {
                let a = rng.vec(100.0);
                let b = rng.vec(100.0);
                c2AABB {
                    min: v(a.x.min(b.x), a.y.min(b.y)),
                    max: v(a.x.max(b.x), a.y.max(b.y)),
                }
            }
            1 => {
                let a = rng.vec(10.0);
                c2AABB { min: a, max: a } // zero extent
            }
            2 => c2AABB {
                min: rng.vec(10.0),
                max: rng.vec(-10.0),
            }, // inverted
            _ => c2AABB {
                min: rng.wild_vec(),
                max: rng.wild_vec(),
            },
        };
        let mut co = [v(-7.0, -7.0); 4];
        let mut ro = [v(-7.0, -7.0); 4];
        unsafe {
            c_bb(co.as_mut_ptr(), &mut bb);
            r_bb(ro.as_mut_ptr(), &mut bb);
        }
        for k in 0..4 {
            assert!(
                veq(co[k], ro[k]),
                "c2BBVerts {i}[{k}]: C {} vs R {}",
                vs(co[k]),
                vs(ro[k])
            );
        }
    }
}

// ----------------------------------------------------------------------- row 11
#[test]
fn row11_norms() {
    let l = libs();
    let (c_n, r_n) = l.pair::<FnNorms>("c2Norms");
    let mut rng = Rng::new(0x0102_0304_0506_0708);
    for i in 0..4000 {
        let count = match i % 8 {
            0 => 0, // row 59 of ERRORS.md too: writes nothing
            k => (1 + k) as c_int,
        };
        let mut verts = [v(0.0, 0.0); 8];
        for k in 0..8 {
            verts[k] = rng.vec(10.0);
        }
        if i % 5 == 0 && count >= 2 {
            // duplicate consecutive verts -> c2Norm(0,0) -> NaN
            verts[1] = verts[0];
        }
        let mut cn = [v(-3.0, -3.0); 8];
        let mut rn = [v(-3.0, -3.0); 8];
        unsafe {
            c_n(verts.as_mut_ptr(), cn.as_mut_ptr(), count);
            r_n(verts.as_mut_ptr(), rn.as_mut_ptr(), count);
        }
        for k in 0..8 {
            assert!(
                veq(cn[k], rn[k]),
                "c2Norms {i} count={count} [{k}]: C {} vs R {}",
                vs(cn[k]),
                vs(rn[k])
            );
        }
    }
}

// ----------------------------------------------------------------------- row 12
#[test]
fn row12_support() {
    let l = libs();
    let (c_s, r_s) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0x1A2B_3C4D_5E6F_7080);
    for i in 0..N {
        let count = match i % 5 {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 8,
            _ => 0, // ERRORS.md row 29
        };
        let mut verts = [v(0.0, 0.0); 8];
        for k in 0..8 {
            verts[k] = if i % 11 == 0 {
                // deliberate ties in the dot product
                v(1.0, 1.0)
            } else {
                rng.vec(10.0)
            };
        }
        let d = if i % 9 == 0 { rng.wild_vec() } else { rng.vec(1.0) };
        unsafe {
            let a = c_s(verts.as_ptr(), count, d);
            let b = r_s(verts.as_ptr(), count, d);
            assert_eq!(a, b, "c2Support {i} count={count} d={}", vs(d));
        }
    }
}

// --------------------------------------------------------------- rows 13/14/15
#[test]
fn row13_row14_row15_make_proxy() {
    let l = libs();
    let (c_mp, r_mp) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0xFEED_FACE_1234_5678);
    for i in 0..6000 {
        // Seed both proxies identically with a recognizable pattern so the
        // "untouched" fields can be compared too.
        let seed = || {
            let mut p = c2Proxy::default();
            p.radius = -99.0;
            p.count = -99;
            for k in 0..8 {
                p.verts[k] = v(-99.0, -99.0);
            }
            p
        };
        let mut cp = seed();
        let mut rp = seed();

        match i % 3 {
            0 => {
                // row 13: circle -> count 1, radius r
                let c = c2Circle {
                    p: rng.vec(100.0),
                    r: rng.fpos(50.0),
                };
                unsafe {
                    c_mp(&c as *const _ as *const _, C2_TYPE_CIRCLE, &mut cp);
                    r_mp(&c as *const _ as *const _, C2_TYPE_CIRCLE, &mut rp);
                }
            }
            1 => {
                // row 14: capsule -> count 2, radius r
                let c = c2Capsule {
                    a: rng.vec(100.0),
                    b: rng.vec(100.0),
                    r: rng.fpos(50.0),
                };
                unsafe {
                    c_mp(&c as *const _ as *const _, C2_TYPE_CAPSULE, &mut cp);
                    r_mp(&c as *const _ as *const _, C2_TYPE_CAPSULE, &mut rp);
                }
            }
            _ => {
                // row 15: aabb -> count 4, radius 0
                let bb = c2AABB {
                    min: rng.vec(100.0),
                    max: rng.vec(100.0),
                };
                unsafe {
                    c_mp(&bb as *const _ as *const _, C2_TYPE_AABB, &mut cp);
                    r_mp(&bb as *const _ as *const _, C2_TYPE_AABB, &mut rp);
                }
            }
        }
        assert!(feq(cp.radius, rp.radius), "c2MakeProxy {i} radius");
        assert_eq!(cp.count, rp.count, "c2MakeProxy {i} count");
        for k in 0..8 {
            assert!(
                veq(cp.verts[k], rp.verts[k]),
                "c2MakeProxy {i} verts[{k}]: C {} vs R {}",
                vs(cp.verts[k]),
                vs(rp.verts[k])
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers — rows 16..21
// ---------------------------------------------------------------------------

/// Builds a random simplex. `count` is set explicitly; all 4 slots and both
/// `sA`/`sB` witnesses are randomized so `c2Witness` has something to mix.
fn rand_simplex(rng: &mut Rng, count: c_int, wild: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    for k in 0..4 {
        s.verts[k] = c2sv {
            sA: if wild { rng.wild_vec() } else { rng.vec(50.0) },
            sB: if wild { rng.wild_vec() } else { rng.vec(50.0) },
            p: if wild { rng.wild_vec() } else { rng.vec(50.0) },
            u: if wild { rng.wild() } else { rng.f(5.0) },
            iA: rng.below(8) as c_int,
            iB: rng.below(8) as c_int,
        };
    }
    s.div = if wild { rng.wild() } else { rng.f(5.0) };
    s.count = count;
    s
}

fn simplex_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    if a.count != b.count || !feq(a.div, b.div) {
        return false;
    }
    for k in 0..4 {
        let (x, y) = (&a.verts[k], &b.verts[k]);
        if !veq(x.sA, y.sA)
            || !veq(x.sB, y.sB)
            || !veq(x.p, y.p)
            || !feq(x.u, y.u)
            || x.iA != y.iA
            || x.iB != y.iB
        {
            return false;
        }
    }
    true
}

fn simplex_dbg(s: &c2Simplex) -> String {
    let mut out = format!("count={} div={} ", s.count, fs(s.div));
    for k in 0..4 {
        out += &format!(
            "[{}: p={} u={} sA={} sB={} iA={} iB={}] ",
            k,
            vs(s.verts[k].p),
            fs(s.verts[k].u),
            vs(s.verts[k].sA),
            vs(s.verts[k].sB),
            s.verts[k].iA,
            s.verts[k].iB
        );
    }
    out
}

// ----------------------------------------------------------------------- row 16
#[test]
fn row16_gjk_simplex_metric() {
    let l = libs();
    let (c_m, r_m) = l.pair::<FnFsimplex>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0x2222_3333_4444_5555);
    for i in 0..N {
        let count = (1 + (i % 3)) as c_int;
        let mut cs = rand_simplex(&mut rng, count, i % 7 == 0);
        let mut rs = cs;
        unsafe {
            let a = c_m(&mut cs);
            let b = r_m(&mut rs);
            assert!(
                feq(a, b),
                "c2GJKSimplexMetric {i} count={count}: C {} vs R {} :: {}",
                fs(a),
                fs(b),
                simplex_dbg(&cs)
            );
        }
        assert!(simplex_eq(&cs, &rs), "c2GJKSimplexMetric {i} mutated state");
    }
}

// ----------------------------------------------------------------------- row 17
#[test]
fn row17_c22_all_regions() {
    let l = libs();
    let (c_f, r_f) = l.pair::<FnSimplex>("c22");
    let mut rng = Rng::new(0x3333_4444_5555_6666);
    let mut hit = [0usize; 3];
    for i in 0..N {
        let mut cs = rand_simplex(&mut rng, 2, i % 13 == 0);
        // Shape the two points to steer into each Voronoi region.
        match i % 3 {
            // origin beyond a: v <= 0
            0 => {
                let dir = rng.vec(1.0);
                cs.verts[0].p = v(dir.x, dir.y);
                cs.verts[1].p = v(dir.x * 3.0, dir.y * 3.0);
            }
            // origin beyond b: u <= 0
            1 => {
                let dir = rng.vec(1.0);
                cs.verts[0].p = v(dir.x * 3.0, dir.y * 3.0);
                cs.verts[1].p = v(dir.x, dir.y);
            }
            // origin projects inside the edge
            _ => {
                let d = rng.vec(1.0);
                let n = v(-d.y, d.x);
                cs.verts[0].p = v(n.x - d.x, n.y - d.y);
                cs.verts[1].p = v(n.x + d.x, n.y + d.y);
            }
        }
        let mut rs = cs;
        unsafe {
            c_f(&mut cs);
            r_f(&mut rs);
        }
        hit[(cs.count.clamp(1, 2) - 1) as usize] += 1;
        assert!(
            simplex_eq(&cs, &rs),
            "c22 {i}:\n  C {}\n  R {}",
            simplex_dbg(&cs),
            simplex_dbg(&rs)
        );
    }
    assert!(hit[0] > 0 && hit[1] > 0, "c22 region coverage: {hit:?}");
}

// ----------------------------------------------------------------------- row 18
#[test]
fn row18_c23_all_regions() {
    let l = libs();
    let (c_f, r_f) = l.pair::<FnSimplex>("c23");
    let mut rng = Rng::new(0x4444_5555_6666_7777);
    let mut counts = [0usize; 4];
    for i in 0..N {
        let mut cs = rand_simplex(&mut rng, 3, i % 17 == 0);
        match i % 6 {
            // triangle far from the origin in a random direction (vertex/edge regions)
            0 | 1 | 2 => {
                let o = rng.vec(20.0);
                let scale = 1.0 + rng.fpos(8.0);
                cs.verts[0].p = v(o.x, o.y);
                cs.verts[1].p = v(o.x + scale, o.y);
                cs.verts[2].p = v(o.x, o.y + scale);
            }
            // triangle enclosing the origin (interior region)
            3 => {
                cs.verts[0].p = v(-1.0 - rng.fpos(3.0), -1.0 - rng.fpos(3.0));
                cs.verts[1].p = v(1.0 + rng.fpos(3.0), -1.0 - rng.fpos(3.0));
                cs.verts[2].p = v(rng.f(1.0), 1.0 + rng.fpos(3.0));
            }
            // degenerate: zero area
            4 => {
                let a = rng.vec(5.0);
                let d = rng.vec(2.0);
                cs.verts[0].p = a;
                cs.verts[1].p = v(a.x + d.x, a.y + d.y);
                cs.verts[2].p = v(a.x + 2.0 * d.x, a.y + 2.0 * d.y);
            }
            // fully random
            _ => {}
        }
        let mut rs = cs;
        unsafe {
            c_f(&mut cs);
            r_f(&mut rs);
        }
        if (0..4).contains(&cs.count) {
            counts[cs.count as usize] += 1;
        }
        assert!(
            simplex_eq(&cs, &rs),
            "c23 {i}:\n  C {}\n  R {}",
            simplex_dbg(&cs),
            simplex_dbg(&rs)
        );
    }
    assert!(
        counts[1] > 0 && counts[2] > 0 && counts[3] > 0,
        "c23 region coverage: {counts:?}"
    );
}

// ----------------------------------------------------------------------- row 19
#[test]
fn row19_c2d() {
    let l = libs();
    let (c_f, r_f) = l.pair::<FnVsimplex>("c2D");
    let mut rng = Rng::new(0x5555_7777_9999_BBBB);
    for i in 0..N {
        let count = (1 + (i % 3)) as c_int;
        let mut cs = rand_simplex(&mut rng, count, i % 11 == 0);
        // For count == 2, drive both signs of c2Det2(ab, -a).
        if count == 2 && i % 2 == 0 {
            cs.verts[0].p = v(1.0 + rng.fpos(2.0), rng.f(2.0));
            cs.verts[1].p = v(rng.f(2.0), 1.0 + rng.fpos(2.0));
        }
        let mut rs = cs;
        unsafe {
            let a = c_f(&mut cs);
            let b = r_f(&mut rs);
            assert!(
                veq(a, b),
                "c2D {i} count={count}: C {} vs R {} :: {}",
                vs(a),
                vs(b),
                simplex_dbg(&cs)
            );
        }
        assert!(simplex_eq(&cs, &rs), "c2D {i} mutated state");
    }
}

// ----------------------------------------------------------------------- row 20
#[test]
fn row20_c2l() {
    let l = libs();
    let (c_f, r_f) = l.pair::<FnVsimplex>("c2L");
    let mut rng = Rng::new(0x6666_8888_AAAA_CCCC);
    for i in 0..N {
        let count = match i % 5 {
            0 => 1,
            1 | 2 => 2,
            3 => 3,
            _ => 0,
        };
        let mut cs = rand_simplex(&mut rng, count, i % 13 == 0);
        if i % 7 == 0 {
            cs.div = 0.0; // ERRORS.md row 28
        }
        let mut rs = cs;
        unsafe {
            let a = c_f(&mut cs);
            let b = r_f(&mut rs);
            assert!(
                veq(a, b),
                "c2L {i} count={count}: C {} vs R {} :: {}",
                vs(a),
                vs(b),
                simplex_dbg(&cs)
            );
        }
        assert!(simplex_eq(&cs, &rs), "c2L {i} mutated state");
    }
}

// ----------------------------------------------------------------------- row 21
#[test]
fn row21_witness() {
    let l = libs();
    let (c_f, r_f) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0x7777_9999_BBBB_DDDD);
    for i in 0..N {
        let count = match i % 6 {
            0 => 1,
            1 => 2,
            2 | 3 => 3,
            4 => 0,
            _ => 4,
        };
        let mut cs = rand_simplex(&mut rng, count, i % 13 == 0);
        if i % 9 == 0 {
            cs.div = 0.0;
        }
        let mut rs = cs;
        let mut ca = v(-1.0, -1.0);
        let mut cb = v(-2.0, -2.0);
        let mut ra = v(-1.0, -1.0);
        let mut rb = v(-2.0, -2.0);
        unsafe {
            c_f(&mut cs, &mut ca, &mut cb);
            r_f(&mut rs, &mut ra, &mut rb);
        }
        assert!(
            veq(ca, ra) && veq(cb, rb),
            "c2Witness {i} count={count}: C a={} b={} vs R a={} b={} :: {}",
            vs(ca),
            vs(cb),
            vs(ra),
            vs(rb),
            simplex_dbg(&cs)
        );
        assert!(simplex_eq(&cs, &rs), "c2Witness {i} mutated state");
    }
}
