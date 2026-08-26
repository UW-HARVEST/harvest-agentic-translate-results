//! Phase B — CONFIGS.md rows 20..27: the simplex primitives, driven DIRECTLY
//! (not through `c2GJK`), because the composed pipeline hides per-arm bugs.
//!
//! `c22` has 3 arms and `c23` has 7; the tests below count which arm each
//! random input landed in and assert that every arm was exercised, so a row is
//! only "covered" if the randomised inputs actually reached all of its branches.

mod common;
use common::*;
use std::ffi::c_int;

const N: usize = 4096;

// ---------------------------------------------------------------------------
// Row 20: c2GJKSimplexMetric — count 1 / 2 / 3
// ---------------------------------------------------------------------------

#[test]
fn row20_gjk_simplex_metric() {
    let p = pair();
    let mut rng = Rng::new(0x2020);
    unsafe {
        for count in [1i32, 2, 3] {
            for i in 0..N {
                let mut sc = rng.simplex_any(count);
                // occasionally force collinear / duplicate points
                match rng.below(6) {
                    0 => sc.verts[1].p = sc.verts[0].p,
                    1 => sc.verts[2].p = sc.verts[0].p,
                    2 => {
                        let d = c2v { x: sc.verts[1].p.x - sc.verts[0].p.x,
                                      y: sc.verts[1].p.y - sc.verts[0].p.y };
                        sc.verts[2].p = c2v { x: sc.verts[0].p.x + d.x * 2.0,
                                              y: sc.verts[0].p.y + d.y * 2.0 };
                    }
                    _ => {}
                }
                let mut sr = sc;
                let mc = (p.c.c2GJKSimplexMetric)(&mut sc);
                let mr = (p.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32(&format!("row20 count={count}[{i}] metric"), mc, mr);
                // the function must not mutate the simplex
                eq_simplex(&format!("row20 count={count}[{i}] simplex untouched"), &sc, &sr);
            }
        }
        // extreme but NaN-free magnitudes -> STRICT
        for i in 0..N {
            for count in [1i32, 2, 3] {
                let mut sc = c2Simplex::default();
                for k in 0..4 {
                    sc.verts[k].p = rng.vec_nasty_no_nan();
                }
                sc.count = count;
                let mut sr = sc;
                eq_f32(
                    &format!("row20 extreme count={count}[{i}]"),
                    (p.c.c2GJKSimplexMetric)(&mut sc),
                    (p.r.c2GJKSimplexMetric)(&mut sr),
                );
            }
        }
        // NaN inputs -> SOFT
        for i in 0..N {
            for count in [1i32, 2, 3] {
                let mut sc = c2Simplex::default();
                for k in 0..4 {
                    sc.verts[k].p = rng.vec_nasty();
                }
                sc.count = count;
                let mut sr = sc;
                eq_f32_soft(
                    &format!("row20 soft count={count}[{i}]"),
                    (p.c.c2GJKSimplexMetric)(&mut sc),
                    (p.r.c2GJKSimplexMetric)(&mut sr),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21: c22 — all three arms
// ---------------------------------------------------------------------------

fn same(a: c2v, b: c2v) -> bool {
    a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
}

/// Classifies the arm `c22` took: 0 = vertex A, 1 = vertex B, 2 = edge AB.
fn classify_c22(orig: &c2Simplex, out: &c2Simplex) -> usize {
    if out.count == 1 {
        if same(out.verts[0].p, orig.verts[0].p) && !same(orig.verts[0].p, orig.verts[1].p) {
            0
        } else if same(out.verts[0].p, orig.verts[1].p) {
            1
        } else {
            0
        }
    } else {
        2
    }
}

#[test]
fn row21_c22_all_arms() {
    let p = pair();
    let mut rng = Rng::new(0x2121);
    let mut arms = [0usize; 3];
    unsafe {
        for i in 0..N * 4 {
            let scale = rng.scale_choice();
            let mut sc = rand_simplex(&mut rng, 2, scale);
            // Bias the segment so all three Voronoi regions of the origin get hit:
            // sometimes centre it on the origin, sometimes push it far away.
            match rng.below(4) {
                0 => {
                    // origin well inside the slab -> edge arm
                    sc.verts[0].p = c2v { x: -scale, y: rng.scaled(scale) };
                    sc.verts[1].p = c2v { x: scale, y: rng.scaled(scale) };
                }
                1 => {
                    // both points on the +x side -> vertex arm
                    sc.verts[0].p = c2v { x: scale + rng.scaled(scale).abs(), y: rng.scaled(scale) };
                    sc.verts[1].p =
                        c2v { x: 2.0 * scale + rng.scaled(scale).abs(), y: rng.scaled(scale) };
                }
                2 => {
                    // duplicate points (degenerate segment)
                    sc.verts[1].p = sc.verts[0].p;
                }
                _ => {}
            }
            let mut sr = sc;
            let orig = sc;
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
            eq_simplex(&format!("row21[{i}]"), &sc, &sr);
            arms[classify_c22(&orig, &sc)] += 1;
        }
        // extreme, NaN-free
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 2, 1.0);
            sc.verts[0].p = rng.vec_nasty_no_nan();
            sc.verts[1].p = rng.vec_nasty_no_nan();
            let mut sr = sc;
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
            eq_simplex(&format!("row21 extreme[{i}]"), &sc, &sr);
        }
        // NaN inputs -> SOFT (every comparison in c22 is false, so the else arm)
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 2, 1.0);
            sc.verts[0].p = rng.vec_nasty();
            sc.verts[1].p = rng.vec_nasty();
            let mut sr = sc;
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
            eq_simplex_soft(&format!("row21 soft[{i}]"), &sc, &sr);
        }
    }
    assert!(arms.iter().all(|&n| n > 0), "row21 did not cover all 3 c22 arms: {arms:?}");
    eprintln!("row21 c22 arm coverage (A, B, AB) = {arms:?}");
}

// ---------------------------------------------------------------------------
// Row 22: c23 — all seven arms
// ---------------------------------------------------------------------------

/// 0=vertA 1=vertB 2=vertC 3=edgeAB 4=edgeBC 5=edgeCA 6=interior
fn classify_c23(orig: &c2Simplex, out: &c2Simplex) -> usize {
    let (a, b, c) = (orig.verts[0].p, orig.verts[1].p, orig.verts[2].p);
    match out.count {
        1 => {
            let v = out.verts[0].p;
            if same(v, a) {
                0
            } else if same(v, b) {
                1
            } else if same(v, c) {
                2
            } else {
                0
            }
        }
        2 => {
            let (v0, v1) = (out.verts[0].p, out.verts[1].p);
            if same(v0, a) && same(v1, b) {
                3
            } else if same(v0, b) && same(v1, c) {
                4
            } else if same(v0, c) && same(v1, a) {
                5
            } else {
                3
            }
        }
        _ => 6,
    }
}

#[test]
fn row22_c23_all_arms() {
    let p = pair();
    let mut rng = Rng::new(0x2222);
    let mut arms = [0usize; 7];
    unsafe {
        for i in 0..N * 8 {
            let scale = rng.scale_choice();
            let mut sc = rand_simplex(&mut rng, 3, scale);
            match rng.below(8) {
                0 => {
                    // triangle containing the origin -> interior arm
                    sc.verts[0].p = c2v { x: -scale, y: -scale };
                    sc.verts[1].p = c2v { x: scale, y: -scale };
                    sc.verts[2].p = c2v { x: 0.0, y: scale };
                }
                1 => {
                    // same but opposite winding (area sign flips)
                    sc.verts[0].p = c2v { x: scale, y: -scale };
                    sc.verts[1].p = c2v { x: -scale, y: -scale };
                    sc.verts[2].p = c2v { x: 0.0, y: scale };
                }
                2 => {
                    // pushed far into +x -> a vertex/edge arm
                    let off = 4.0 * scale;
                    sc.verts[0].p = c2v { x: off, y: -scale };
                    sc.verts[1].p = c2v { x: off + scale, y: 0.0 };
                    sc.verts[2].p = c2v { x: off, y: scale };
                }
                3 => {
                    // collinear (degenerate, area == 0)
                    let d = rng.vec_scaled(scale);
                    let o = rng.vec_scaled(scale);
                    sc.verts[0].p = o;
                    sc.verts[1].p = c2v { x: o.x + d.x, y: o.y + d.y };
                    sc.verts[2].p = c2v { x: o.x + 2.0 * d.x, y: o.y + 2.0 * d.y };
                }
                4 => {
                    // duplicate vertices
                    sc.verts[2].p = sc.verts[1].p;
                }
                5 => {
                    // rotate a containing triangle by a random angle
                    let ang = rng.unit() * std::f32::consts::PI;
                    let (co, si) = (ang.cos(), ang.sin());
                    let base = [
                        c2v { x: -scale, y: -scale },
                        c2v { x: scale, y: -scale },
                        c2v { x: 0.0, y: scale },
                    ];
                    for k in 0..3 {
                        sc.verts[k].p = c2v {
                            x: co * base[k].x - si * base[k].y,
                            y: si * base[k].x + co * base[k].y,
                        };
                    }
                }
                _ => {}
            }
            let mut sr = sc;
            let orig = sc;
            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
            eq_simplex(&format!("row22[{i}]"), &sc, &sr);
            arms[classify_c23(&orig, &sc)] += 1;
        }
        // extreme, NaN-free
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 3, 1.0);
            for k in 0..3 {
                sc.verts[k].p = rng.vec_nasty_no_nan();
            }
            let mut sr = sc;
            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
            eq_simplex(&format!("row22 extreme[{i}]"), &sc, &sr);
        }
        // NaN -> SOFT
        for i in 0..N {
            let mut sc = rand_simplex(&mut rng, 3, 1.0);
            for k in 0..3 {
                sc.verts[k].p = rng.vec_nasty();
            }
            let mut sr = sc;
            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
            eq_simplex_soft(&format!("row22 soft[{i}]"), &sc, &sr);
        }
    }
    assert!(
        arms.iter().all(|&n| n > 0),
        "row22 did not cover all 7 c23 arms: {arms:?} (A,B,C,AB,BC,CA,interior)"
    );
    eprintln!("row22 c23 arm coverage (A,B,C,AB,BC,CA,int) = {arms:?}");
}

// ---------------------------------------------------------------------------
// Row 23: c2D — count 1, and both orientation arms of count 2
// ---------------------------------------------------------------------------

#[test]
fn row23_c2d() {
    let p = pair();
    let mut rng = Rng::new(0x2323);
    let mut skew_arm = 0usize;
    let mut ccw_arm = 0usize;
    unsafe {
        for i in 0..N * 2 {
            let mut sc = rng.simplex_any(1);
            let mut sr = sc;
            eq_v(&format!("row23 count1[{i}]"), (p.c.c2D)(&mut sc), (p.r.c2D)(&mut sr));
            eq_simplex(&format!("row23 count1[{i}] untouched"), &sc, &sr);

            let scale = rng.scale_choice();
            let mut s2 = rand_simplex(&mut rng, 2, scale);
            if rng.below(2) == 0 {
                // force a specific winding so both arms get hit
                s2.verts[0].p = c2v { x: -scale, y: -scale };
                s2.verts[1].p = c2v { x: scale, y: if rng.below(2) == 0 { -scale } else { scale } };
            }
            let mut s2r = s2;
            let dc = (p.c.c2D)(&mut s2);
            let dr = (p.r.c2D)(&mut s2r);
            eq_v(&format!("row23 count2[{i}]"), dc, dr);
            eq_simplex(&format!("row23 count2[{i}] untouched"), &s2, &s2r);
            // classify: c2Skew(ab) = (-ab.y, ab.x); c2CCW90(ab) = (ab.y, -ab.x)
            let ab = c2v { x: s2.verts[1].p.x - s2.verts[0].p.x,
                           y: s2.verts[1].p.y - s2.verts[0].p.y };
            if dc.x.to_bits() == (-ab.y).to_bits() && dc.y.to_bits() == ab.x.to_bits() {
                skew_arm += 1;
            } else {
                ccw_arm += 1;
            }
        }
        // boundary: det == 0 exactly (a on the line through the origin)
        for i in 0..256 {
            let mut s2 = rand_simplex(&mut rng, 2, 1.0);
            let d = rng.vec_scaled(10.0);
            s2.verts[0].p = d;
            s2.verts[1].p = c2v { x: d.x * 3.0, y: d.y * 3.0 }; // ab parallel to a -> det 0
            let mut s2r = s2;
            eq_v(&format!("row23 det0[{i}]"), (p.c.c2D)(&mut s2), (p.r.c2D)(&mut s2r));
        }
        // extreme / NaN
        for i in 0..N {
            for count in [1i32, 2, 3] {
                let mut s = c2Simplex::default();
                s.verts[0].p = rng.vec_nasty_no_nan();
                s.verts[1].p = rng.vec_nasty_no_nan();
                s.count = count;
                let mut sr = s;
                eq_v(&format!("row23 extreme c={count}[{i}]"), (p.c.c2D)(&mut s), (p.r.c2D)(&mut sr));
                let mut t = c2Simplex::default();
                t.verts[0].p = rng.vec_nasty();
                t.verts[1].p = rng.vec_nasty();
                t.count = count;
                let mut tr = t;
                eq_v_soft(&format!("row23 soft c={count}[{i}]"), (p.c.c2D)(&mut t), (p.r.c2D)(&mut tr));
            }
        }
    }
    assert!(skew_arm > 0 && ccw_arm > 0, "row23 missed a c2D arm: skew={skew_arm} ccw={ccw_arm}");
    eprintln!("row23 c2D arm coverage: skew={skew_arm} ccw90={ccw_arm}");
}

// ---------------------------------------------------------------------------
// Row 24: c2L — count 1 / 2 (+ div == 0)
// ---------------------------------------------------------------------------

#[test]
fn row24_c2l() {
    let p = pair();
    let mut rng = Rng::new(0x2424);
    unsafe {
        for count in [1i32, 2] {
            for i in 0..N {
                let mut sc = rng.simplex_any(count);
                // sometimes use a div/u pair produced by a real c22 run
                if rng.below(3) == 0 {
                    let mut t = rand_simplex(&mut rng, 2, 10.0);
                    (p.c.c22)(&mut t);
                    sc.div = t.div;
                    sc.verts[0].u = t.verts[0].u;
                    sc.verts[1].u = t.verts[1].u;
                }
                // ERRORS.md row 10: div == 0
                if rng.below(8) == 0 {
                    sc.div = 0.0;
                }
                if rng.below(16) == 0 {
                    sc.div = -0.0;
                }
                let mut sr = sc;
                eq_v(&format!("row24 count={count}[{i}]"), (p.c.c2L)(&mut sc), (p.r.c2L)(&mut sr));
                eq_simplex(&format!("row24 count={count}[{i}] untouched"), &sc, &sr);
            }
        }
        // extreme, NaN-free
        for i in 0..N {
            for count in [1i32, 2] {
                let mut s = c2Simplex::default();
                for k in 0..2 {
                    s.verts[k].p = rng.vec_nasty_no_nan();
                    s.verts[k].u = rng.nasty_no_nan();
                }
                s.div = rng.nasty_no_nan();
                s.count = count;
                let mut sr = s;
                eq_v(&format!("row24 extreme c={count}[{i}]"), (p.c.c2L)(&mut s), (p.r.c2L)(&mut sr));
            }
        }
        // NaN -> SOFT
        for i in 0..N {
            for count in [1i32, 2] {
                let mut s = c2Simplex::default();
                for k in 0..2 {
                    s.verts[k].p = rng.vec_nasty();
                    s.verts[k].u = rng.nasty();
                }
                s.div = rng.nasty();
                s.count = count;
                let mut sr = s;
                eq_v_soft(&format!("row24 soft c={count}[{i}]"), (p.c.c2L)(&mut s), (p.r.c2L)(&mut sr));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25: c2Witness — count 1 / 2 / 3 (+ div == 0)
// ---------------------------------------------------------------------------

#[test]
fn row25_c2witness() {
    let p = pair();
    let mut rng = Rng::new(0x2525);
    let poison = c2v { x: f32::from_bits(0xDEAD_BEEF), y: f32::from_bits(0xFEED_FACE) };
    unsafe {
        for count in [1i32, 2, 3] {
            for i in 0..N {
                let mut sc = rng.simplex_any(count);
                if rng.below(8) == 0 {
                    sc.div = 0.0; // ERRORS.md row 7
                }
                if rng.below(16) == 0 {
                    sc.div = -0.0;
                }
                let mut sr = sc;
                let (mut ac, mut bc) = (poison, poison);
                let (mut ar, mut br) = (poison, poison);
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v(&format!("row25 c={count}[{i}] a"), ac, ar);
                eq_v(&format!("row25 c={count}[{i}] b"), bc, br);
                eq_simplex(&format!("row25 c={count}[{i}] untouched"), &sc, &sr);
            }
        }
        // extreme, NaN-free
        for i in 0..N {
            for count in [1i32, 2, 3] {
                let mut s = c2Simplex::default();
                for k in 0..3 {
                    s.verts[k].sA = rng.vec_nasty_no_nan();
                    s.verts[k].sB = rng.vec_nasty_no_nan();
                    s.verts[k].u = rng.nasty_no_nan();
                }
                s.div = rng.nasty_no_nan();
                s.count = count;
                let mut sr = s;
                let (mut ac, mut bc) = (poison, poison);
                let (mut ar, mut br) = (poison, poison);
                (p.c.c2Witness)(&mut s, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v(&format!("row25 extreme c={count}[{i}] a"), ac, ar);
                eq_v(&format!("row25 extreme c={count}[{i}] b"), bc, br);
            }
        }
        // NaN -> SOFT
        for i in 0..N {
            for count in [1i32, 2, 3] {
                let mut s = c2Simplex::default();
                for k in 0..3 {
                    s.verts[k].sA = rng.vec_nasty();
                    s.verts[k].sB = rng.vec_nasty();
                    s.verts[k].u = rng.nasty();
                }
                s.div = rng.nasty();
                s.count = count;
                let mut sr = s;
                let (mut ac, mut bc) = (poison, poison);
                let (mut ar, mut br) = (poison, poison);
                (p.c.c2Witness)(&mut s, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v_soft(&format!("row25 soft c={count}[{i}] a"), ac, ar);
                eq_v_soft(&format!("row25 soft c={count}[{i}] b"), bc, br);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 26/27: c2Support — count 1 / 2 / 4 / 8, ties, all quadrants
// ---------------------------------------------------------------------------

#[test]
fn row26_row27_c2support() {
    let p = pair();
    let mut rng = Rng::new(0x2626);
    unsafe {
        for count in [1i32, 2, 3, 4, 8] {
            for i in 0..N {
                let mut verts = [c2v::default(); 8];
                for k in 0..8 {
                    verts[k] = match rng.below(4) {
                        0 => rng.vec_nasty_no_nan(),
                        1 => c2v { x: 0.0, y: 0.0 },
                        _ => rng.vec_any_scale(),
                    };
                }
                // ties: duplicate a vertex so `dot > dmax` must keep the FIRST
                if rng.below(3) == 0 && count > 1 {
                    let src = rng.below(count as u32) as usize;
                    let dst = rng.below(count as u32) as usize;
                    verts[dst] = verts[src];
                }
                let d = match rng.below(6) {
                    0 => c2v { x: 0.0, y: 0.0 },            // ERRORS.md row 13
                    1 => c2v { x: 1.0, y: 0.0 },
                    2 => c2v { x: 0.0, y: 1.0 },
                    3 => c2v { x: -1.0, y: 0.0 },
                    4 => c2v { x: 0.0, y: -1.0 },
                    _ => rng.vec_any_scale(),
                };
                let ic = (p.c.c2Support)(verts.as_ptr(), count, d);
                let ir = (p.r.c2Support)(verts.as_ptr(), count, d);
                eq_i(&format!("row26 count={count}[{i}] d={d:?}"), ic, ir);
                assert!(ic >= 0 && ic < count.max(1), "row26 index {ic} out of range for {count}");
            }
        }
        // AABB vertex layout specifically (the real 4-vertex proxy shape)
        for i in 0..N {
            let bb = rng.aabb_any();
            let mut verts = [c2v::default(); 8];
            let mut bbm = bb;
            (p.c.c2BBVerts)(verts.as_mut_ptr(), &mut bbm);
            for q in 0..8 {
                let ang = std::f32::consts::PI * 2.0 * (q as f32) / 8.0;
                let d = c2v { x: ang.cos(), y: ang.sin() };
                eq_i(
                    &format!("row26 aabb[{i}] q={q}"),
                    (p.c.c2Support)(verts.as_ptr(), 4, d),
                    (p.r.c2Support)(verts.as_ptr(), 4, d),
                );
            }
        }
        // NaN verts / NaN direction (ERRORS.md row 14) — the index is an int, so
        // this stays STRICT even with NaN inputs.
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = rng.vec_nasty();
            }
            let d = rng.vec_nasty();
            for count in [1i32, 2, 4, 8] {
                eq_i(
                    &format!("row26 nan[{i}] count={count}"),
                    (p.c.c2Support)(verts.as_ptr(), count, d),
                    (p.r.c2Support)(verts.as_ptr(), count, d),
                );
            }
        }
    }
}

/// The C reads `verts[0]` before the loop guard, so a `count <= 0` call is
/// valid as long as one element is readable (ERRORS.md rows 11/12).
#[test]
fn row26_c2support_nonpositive_count() {
    let p = pair();
    let mut rng = Rng::new(0x2627);
    unsafe {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = rng.vec_any_scale();
            }
            let d = rng.vec_any_scale();
            for count in [0i32, -1, -2, i32::MIN, 1] {
                eq_i(
                    &format!("row26 count={count}[{i}]"),
                    (p.c.c2Support)(verts.as_ptr(), count as c_int, d),
                    (p.r.c2Support)(verts.as_ptr(), count as c_int, d),
                );
            }
        }
    }
}
