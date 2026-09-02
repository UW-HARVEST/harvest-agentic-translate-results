//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded from their `.so` and driven through their
//! exported symbols only. All comparisons are bit-exact (`f32::to_bits`).

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 4000;

// ===========================================================================
// L0 — pure vector maths (rows 1-10)
// ===========================================================================

#[test]
fn row01_c2v_neg_skew_ccw90() {
    let p = load_pair();
    let mut rng = Rng::new(0x0101);
    unsafe {
        for i in 0..N {
            let v = rng.v();
            eq_v(
                &format!("row01/c2V #{i}"),
                (p.c.c2V)(v.x, v.y),
                (p.r.c2V)(v.x, v.y),
            );
            eq_v(&format!("row01/c2Neg #{i}"), (p.c.c2Neg)(v), (p.r.c2Neg)(v));
            eq_v(
                &format!("row01/c2Skew #{i}"),
                (p.c.c2Skew)(v),
                (p.r.c2Skew)(v),
            );
            eq_v(
                &format!("row01/c2CCW90 #{i}"),
                (p.c.c2CCW90)(v),
                (p.r.c2CCW90)(v),
            );
        }
    }
}

#[test]
fn row02_c2add_c2sub() {
    let p = load_pair();
    let mut rng = Rng::new(0x0202);
    unsafe {
        for i in 0..N {
            let a = rng.v();
            let b = if i % 7 == 0 { a } else { rng.v() };
            eq_v(&format!("row02/c2Add #{i}"), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
            eq_v(&format!("row02/c2Sub #{i}"), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
            eq_v(&format!("row02/c2Sub self #{i}"), (p.c.c2Sub)(a, a), (p.r.c2Sub)(a, a));
        }
    }
}

#[test]
fn row03_c2mulvs_c2div() {
    let p = load_pair();
    let mut rng = Rng::new(0x0303);
    let fixed = [1.0f32, -1.0, 0.5, -0.5, 2.0, 1.0e30, 1.0e-30, f32::MIN_POSITIVE];
    unsafe {
        for i in 0..N {
            let a = rng.v();
            let s = if i % 5 == 0 { fixed[i % fixed.len()] } else { rng.finite() };
            eq_v(&format!("row03/c2Mulvs #{i} s={s:e}"), (p.c.c2Mulvs)(a, s), (p.r.c2Mulvs)(a, s));
            eq_v(&format!("row03/c2Div #{i} s={s:e}"), (p.c.c2Div)(a, s), (p.r.c2Div)(a, s));
        }
    }
}

#[test]
fn row04_c2dot_c2det2() {
    let p = load_pair();
    let mut rng = Rng::new(0x0404);
    unsafe {
        for i in 0..N {
            let a = rng.v();
            // orthogonal / parallel / cancelling variants
            let b = match i % 4 {
                0 => c2v { x: -a.y, y: a.x },
                1 => c2v { x: a.x, y: a.y },
                2 => c2v { x: -a.x, y: -a.y },
                _ => rng.v(),
            };
            eq_f32(&format!("row04/c2Dot #{i}"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
            eq_f32(&format!("row04/c2Det2 #{i}"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
        }
    }
}

#[test]
fn row05_c2len_c2norm() {
    let p = load_pair();
    let mut rng = Rng::new(0x0505);
    unsafe {
        for i in 0..N {
            let v = match i % 5 {
                0 => c2v { x: 1.0, y: 0.0 },
                1 => c2v { x: 0.0, y: -1.0 },
                2 => c2v { x: rng.scaled(1.0e-30), y: rng.scaled(1.0e-30) },
                3 => c2v { x: rng.scaled(1.0e18), y: rng.scaled(1.0e18) },
                _ => rng.v(),
            };
            eq_f32(&format!("row05/c2Len #{i}"), (p.c.c2Len)(v), (p.r.c2Len)(v));
            eq_v(&format!("row05/c2Norm #{i}"), (p.c.c2Norm)(v), (p.r.c2Norm)(v));
        }
    }
}

#[test]
fn row06_c2maxv_c2minv() {
    let p = load_pair();
    let mut rng = Rng::new(0x0606);
    let zeros = [0.0f32, -0.0];
    unsafe {
        for i in 0..N {
            let mut a = rng.v();
            let mut b = rng.v();
            if i % 11 == 0 {
                a.x = zeros[i % 2];
                b.x = zeros[(i + 1) % 2];
            }
            if i % 13 == 0 {
                b.y = a.y; // exact tie
            }
            eq_v(&format!("row06/c2Maxv #{i}"), (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
            eq_v(&format!("row06/c2Minv #{i}"), (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
        }
    }
}

#[test]
fn row07_c2clampv() {
    let p = load_pair();
    let mut rng = Rng::new(0x0707);
    unsafe {
        for i in 0..N {
            let lo = rng.v_coord();
            let hi = match i % 4 {
                0 => lo,                                                    // lo == hi
                1 => c2v { x: lo.x - 5.0, y: lo.y - 5.0 },                  // inverted
                _ => c2v { x: lo.x + rng.unit().abs() * 10.0, y: lo.y + rng.unit().abs() * 10.0 },
            };
            let a = match i % 3 {
                0 => c2v { x: lo.x - 1.0, y: hi.y + 1.0 },
                1 => rng.v_coord(),
                _ => rng.v(),
            };
            eq_v(
                &format!("row07/c2Clampv #{i}"),
                (p.c.c2Clampv)(a, lo, hi),
                (p.r.c2Clampv)(a, lo, hi),
            );
        }
    }
}

#[test]
fn row08_identities() {
    let p = load_pair();
    unsafe {
        eq_r("row08/c2RotIdentity", (p.c.c2RotIdentity)(), (p.r.c2RotIdentity)());
        eq_x("row08/c2xIdentity", (p.c.c2xIdentity)(), (p.r.c2xIdentity)());
    }
}

#[test]
fn row09_c2mulrv_c2mulrvt() {
    let p = load_pair();
    let mut rng = Rng::new(0x0909);
    unsafe {
        for i in 0..N {
            let r = match i % 4 {
                0 => c2r { c: 1.0, s: 0.0 },
                1 => c2r { c: 0.0, s: 0.0 },
                2 => c2r { c: rng.finite(), s: rng.finite() },
                _ => rng.rot(),
            };
            let v = rng.v();
            eq_v(&format!("row09/c2Mulrv #{i}"), (p.c.c2Mulrv)(r, v), (p.r.c2Mulrv)(r, v));
            eq_v(&format!("row09/c2MulrvT #{i}"), (p.c.c2MulrvT)(r, v), (p.r.c2MulrvT)(r, v));
        }
    }
}

#[test]
fn row10_c2mulxv() {
    let p = load_pair();
    let mut rng = Rng::new(0x1010);
    unsafe {
        for i in 0..N {
            let x = match i % 4 {
                0 => (p.c.c2xIdentity)(),
                1 => c2x { p: rng.v_coord(), r: c2r { c: 1.0, s: 0.0 } },
                2 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() },
                _ => rng.x_transform(),
            };
            let v = rng.v();
            eq_v(&format!("row10/c2Mulxv #{i}"), (p.c.c2Mulxv)(x, v), (p.r.c2Mulxv)(x, v));
        }
    }
}

// ===========================================================================
// L1 — shape / proxy (rows 11-19)
// ===========================================================================

#[test]
fn row11_c2bbverts() {
    let p = load_pair();
    let mut rng = Rng::new(0x1111);
    unsafe {
        for i in 0..N {
            let mut bb = match i % 5 {
                0 => c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
                1 => {
                    let m = rng.v_coord();
                    c2AABB { min: m, max: c2v { x: m.x, y: m.y + 3.0 } }
                }
                2 => {
                    let m = rng.v_coord();
                    c2AABB { min: c2v { x: m.x + 5.0, y: m.y + 5.0 }, max: m } // inverted
                }
                3 => c2AABB { min: c2v { x: -1.0e18, y: -1.0e18 }, max: c2v { x: 1.0e18, y: 1.0e18 } },
                _ => {
                    let m = rng.v();
                    c2AABB { min: m, max: rng.v() }
                }
            };
            let mut oc = [c2v { x: 1.5, y: 2.5 }; 4];
            let mut orr = oc;
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bb);
            (p.r.c2BBVerts)(orr.as_mut_ptr(), &mut bb);
            for k in 0..4 {
                eq_v(&format!("row11/c2BBVerts #{i}[{k}]"), oc[k], orr[k]);
            }
        }
    }
}

fn poisoned_proxy() -> c2Proxy {
    let mut px = c2Proxy {
        radius: -123.456,
        count: -999,
        verts: [c2v { x: 0.0, y: 0.0 }; 8],
    };
    for (k, v) in px.verts.iter_mut().enumerate() {
        v.x = 100.0 + k as f32;
        v.y = -100.0 - k as f32;
    }
    px
}

fn make_proxy_row(seed: u64, ty: c_int, label: &str) {
    let p = load_pair();
    let mut rng = Rng::new(seed);
    unsafe {
        for i in 0..N {
            let ctr = rng.v();
            let ext = rng.finite().abs();
            let shape = gen_shape(&mut rng, ty, ctr, ext);
            let mut pc = poisoned_proxy();
            let mut pr = poisoned_proxy();
            (p.c.c2MakeProxy)(shape.as_ptr(), ty, &mut pc);
            (p.r.c2MakeProxy)(shape.as_ptr(), ty, &mut pr);
            eq_proxy(&format!("{label} #{i}"), &pc, &pr);
        }
    }
}

#[test]
fn row12_makeproxy_circle() {
    make_proxy_row(0x1212, C2_TYPE_CIRCLE, "row12/circle");
}

#[test]
fn row13_makeproxy_aabb() {
    make_proxy_row(0x1313, C2_TYPE_AABB, "row13/aabb");
}

#[test]
fn row14_makeproxy_capsule() {
    make_proxy_row(0x1414, C2_TYPE_CAPSULE, "row14/capsule");
}

#[test]
fn row15_makeproxy_untouched_tail() {
    // Poisoned destination: the bytes the C never writes (verts[count..8]) must
    // also be identical, which is what catches an over-eager Rust translation
    // that zeroes the whole proxy.
    let p = load_pair();
    let mut rng = Rng::new(0x1515);
    unsafe {
        for i in 0..N {
            let ty = ALL_TYPES[i % 3];
            let ctr = rng.v_coord();
            let shape = gen_shape(&mut rng, ty, ctr, 5.0);
            let mut pc = poisoned_proxy();
            let mut pr = poisoned_proxy();
            (p.c.c2MakeProxy)(shape.as_ptr(), ty, &mut pc);
            (p.r.c2MakeProxy)(shape.as_ptr(), ty, &mut pr);
            eq_proxy(&format!("row15 #{i} ty={ty}"), &pc, &pr);
            // and the tail really is the poison value
            let keep = match ty {
                C2_TYPE_CIRCLE => 1,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            for k in keep..8 {
                eq_f32(&format!("row15 #{i} tail[{k}].x"), 100.0 + k as f32, pc.verts[k].x);
            }
        }
    }
}

fn support_row(seed: u64, count: c_int, label: &str) {
    let p = load_pair();
    let mut rng = Rng::new(seed);
    unsafe {
        for i in 0..N {
            let mut verts = [c2v { x: 0.0, y: 0.0 }; 8];
            for k in 0..8 {
                verts[k] = rng.v_coord();
            }
            // inject duplicates and exact ties
            if i % 5 == 0 && count > 1 {
                verts[(i % count as usize)] = verts[0];
            }
            let d = match i % 6 {
                0 => c2v { x: 1.0, y: 0.0 },
                1 => c2v { x: 0.0, y: 1.0 },
                2 => c2v { x: 1.0, y: 1.0 },
                3 => c2v { x: 0.0, y: 0.0 },
                4 => rng.v(),
                _ => rng.v_coord(),
            };
            eq_i(
                &format!("{label} #{i}"),
                (p.c.c2Support)(verts.as_ptr(), count, d),
                (p.r.c2Support)(verts.as_ptr(), count, d),
            );
        }
    }
}

#[test]
fn row16_support_count1() {
    support_row(0x1616, 1, "row16/support c=1");
}

#[test]
fn row17_support_count2() {
    support_row(0x1717, 2, "row17/support c=2");
}

#[test]
fn row18_support_count4() {
    support_row(0x1818, 4, "row18/support c=4");
}

#[test]
fn row19_support_count8() {
    support_row(0x1919, 8, "row19/support c=8");
}

// ===========================================================================
// L2 — simplex routines (rows 20-27)
// ===========================================================================

/// Random simplex with `count` meaningful vertices; all 4 slots filled so the
/// untouched-slot comparison is meaningful.
fn rand_simplex(rng: &mut Rng, count: c_int, coord_only: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    for k in 0..4 {
        let (sa, sb) = if coord_only {
            (rng.v_coord(), rng.v_coord())
        } else {
            (rng.v(), rng.v())
        };
        s.verts[k] = c2sv {
            sA: sa,
            sB: sb,
            p: c2v { x: sb.x - sa.x, y: sb.y - sa.y },
            u: rng.finite(),
            iA: (rng.below(4)) as c_int,
            iB: (rng.below(4)) as c_int,
        };
    }
    s.div = if rng.below(8) == 0 { 0.0 } else { rng.finite() };
    s.count = count;
    s
}

#[test]
fn row20_simplex_metric_count2() {
    let p = load_pair();
    let mut rng = Rng::new(0x2020);
    unsafe {
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 2, i % 2 == 0);
            let mut sr = sc;
            let rc = (p.c.c2GJKSimplexMetric)(&mut sc);
            let rr = (p.r.c2GJKSimplexMetric)(&mut sr);
            eq_f32(&format!("row20 #{i}"), rc, rr);
            eq_simplex(&format!("row20 #{i} struct"), &sc, &sr);
        }
    }
}

#[test]
fn row21_simplex_metric_count3() {
    let p = load_pair();
    let mut rng = Rng::new(0x2121);
    unsafe {
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 3, i % 2 == 0);
            let mut sr = sc;
            let rc = (p.c.c2GJKSimplexMetric)(&mut sc);
            let rr = (p.r.c2GJKSimplexMetric)(&mut sr);
            eq_f32(&format!("row21 #{i}"), rc, rr);
            eq_simplex(&format!("row21 #{i} struct"), &sc, &sr);
        }
    }
}

#[test]
fn row22_c22_all_branches() {
    let p = load_pair();
    let mut rng = Rng::new(0x2222);
    let mut branch_counts = [0usize; 3];
    unsafe {
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 2, i % 3 != 0);
            // force the degenerate a == b case sometimes
            if i % 17 == 0 {
                sc.verts[1].p = sc.verts[0].p;
            }
            let mut sr = sc;
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
            eq_simplex(&format!("row22 #{i}"), &sc, &sr);
            // classify for coverage reporting
            let a = sc.verts[0].p;
            let _ = a;
            branch_counts[(sc.count as usize).min(2)] += 1;
        }
    }
    // both count==1 and count==2 outcomes must have been produced
    assert!(branch_counts[1] > 0, "c22 never collapsed to count=1");
    assert!(branch_counts[2] > 0, "c22 never kept count=2");
}

#[test]
fn row23_c23_all_branches() {
    let p = load_pair();
    let mut rng = Rng::new(0x2323);
    let mut seen = [0usize; 4];
    unsafe {
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 3, i % 3 != 0);
            match i % 23 {
                0 => sc.verts[1].p = sc.verts[0].p,            // duplicate -> area 0
                1 => sc.verts[2].p = sc.verts[0].p,
                2 => {
                    // collinear
                    let a = sc.verts[0].p;
                    let d = c2v { x: 1.0, y: 2.0 };
                    sc.verts[1].p = c2v { x: a.x + d.x, y: a.y + d.y };
                    sc.verts[2].p = c2v { x: a.x + 2.0 * d.x, y: a.y + 2.0 * d.y };
                }
                _ => {}
            }
            let mut sr = sc;
            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
            eq_simplex(&format!("row23 #{i}"), &sc, &sr);
            seen[(sc.count.clamp(0, 3)) as usize] += 1;
        }
    }
    assert!(seen[1] > 0 && seen[2] > 0 && seen[3] > 0, "c23 branch coverage: {seen:?}");
}

#[test]
fn row24_c23_from_real_gjk_geometry() {
    // Simplices built from actual Minkowski-difference support points of two
    // real shapes, so the barycentric branches are hit with realistic values.
    let p = load_pair();
    let mut rng = Rng::new(0x2424);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let ca = rng.v_coord();
            let sa = gen_shape(&mut rng, ta, ca, 4.0);
            let cb = rng.v_coord();
            let sb = gen_shape(&mut rng, tb, cb, 4.0);
            let mut pa = c2Proxy::default();
            let mut pb = c2Proxy::default();
            (p.c.c2MakeProxy)(sa.as_ptr(), ta, &mut pa);
            (p.c.c2MakeProxy)(sb.as_ptr(), tb, &mut pb);
            let mut s = c2Simplex::default();
            for k in 0..3 {
                let ia = (rng.below(pa.count.max(1) as u32)) as usize;
                let ib = (rng.below(pb.count.max(1) as u32)) as usize;
                let va = pa.verts[ia];
                let vb = pb.verts[ib];
                s.verts[k] = c2sv {
                    sA: va,
                    sB: vb,
                    p: c2v { x: vb.x - va.x, y: vb.y - va.y },
                    u: 0.0,
                    iA: ia as c_int,
                    iB: ib as c_int,
                };
            }
            s.div = 1.0;
            s.count = 3;
            let mut sc = s;
            let mut sr = s;
            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
            eq_simplex(&format!("row24 #{i}"), &sc, &sr);
        }
    }
}

#[test]
fn row25_c2d_counts_1_2() {
    let p = load_pair();
    let mut rng = Rng::new(0x2525);
    let mut skew = 0usize;
    let mut ccw = 0usize;
    unsafe {
        for i in 0..N {
            let count = if i % 2 == 0 { 1 } else { 2 };
            let mut sc = rand_simplex(&mut rng, count, i % 3 != 0);
            let mut sr = sc;
            let rc = (p.c.c2D)(&mut sc);
            let rr = (p.r.c2D)(&mut sr);
            eq_v(&format!("row25 #{i} count={count}"), rc, rr);
            eq_simplex(&format!("row25 #{i} struct"), &sc, &sr);
            if count == 2 {
                let ab = (p.c.c2Sub)(sc.verts[1].p, sc.verts[0].p);
                if (p.c.c2Det2)(ab, (p.c.c2Neg)(sc.verts[0].p)) > 0.0 {
                    skew += 1;
                } else {
                    ccw += 1;
                }
            }
        }
    }
    assert!(skew > 0 && ccw > 0, "c2D count=2 sub-branches: skew={skew} ccw={ccw}");
}

#[test]
fn row26_c2l_counts_1_2() {
    let p = load_pair();
    let mut rng = Rng::new(0x2626);
    unsafe {
        for i in 0..N {
            let count = if i % 2 == 0 { 1 } else { 2 };
            let mut sc = rand_simplex(&mut rng, count, i % 3 != 0);
            if i % 9 == 0 {
                sc.div = sc.verts[0].u + sc.verts[1].u;
            }
            let mut sr = sc;
            let rc = (p.c.c2L)(&mut sc);
            let rr = (p.r.c2L)(&mut sr);
            eq_v(&format!("row26 #{i} count={count}"), rc, rr);
            eq_simplex(&format!("row26 #{i} struct"), &sc, &sr);
        }
    }
}

#[test]
fn row27_c2witness_counts_1_2_3() {
    let p = load_pair();
    let mut rng = Rng::new(0x2727);
    unsafe {
        for i in 0..N {
            let count = (i % 3 + 1) as c_int;
            let mut sc = rand_simplex(&mut rng, count, i % 3 != 0);
            if i % 9 == 0 {
                sc.div = sc.verts[0].u + sc.verts[1].u + sc.verts[2].u;
            }
            let mut sr = sc;
            let mut ac = c2v { x: 7.5, y: -7.5 };
            let mut bc = c2v { x: -2.25, y: 2.25 };
            let mut ar = ac;
            let mut br = bc;
            (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
            (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
            eq_v(&format!("row27 #{i} count={count} a"), ac, ar);
            eq_v(&format!("row27 #{i} count={count} b"), bc, br);
            eq_simplex(&format!("row27 #{i} struct"), &sc, &sr);
        }
    }
}
