//! Phase B, level 1 — CONFIGS.md rows B11 … B37.
//!
//! `c2MakeProxy`, `c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2L`, `c2Witness`.
//! These take `c2Simplex*` / `c2Proxy*` out-parameters, so the *whole* struct is
//! compared byte-for-byte after the call (the structs are padding-free).
//!
//! For the multi-way branch functions (`c22`: 3 branches, `c23`: 7 branches,
//! `c2D`: 4 branches) the test classifies which branch each random input takes
//! and asserts at the end that **every** branch was exercised — so the
//! CONFIGS.md rows are provably covered rather than hopefully covered.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::os::raw::c_int;

const N: usize = 4096;

// ---------------------------------------------------------------------------
// B11 / B12 / B13 — c2MakeProxy for each shape type
// ---------------------------------------------------------------------------

/// Pre-fill the proxy with a recognizable pattern so that "the C left it
/// untouched" is distinguishable from "the C zeroed it".
fn dirty_proxy(seed: f32) -> c2Proxy {
    let mut p = c2Proxy {
        radius: seed,
        count: 0x5555_5555,
        verts: [c2v {
            x: seed,
            y: -seed,
        }; 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = seed + i as f32;
        v.y = seed - i as f32;
    }
    p
}

#[test]
fn b11_makeproxy_circle() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB11);
    unsafe {
        for i in 0..N {
            let mut shape = if rng.below(4) == 0 {
                c2Circle {
                    p: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.circle(500.0)
            };
            let mut pc = dirty_proxy(7.25);
            let mut pr = pc;
            (c.c2MakeProxy)(
                &mut shape as *mut c2Circle as *const c_void,
                C2_TYPE_CIRCLE,
                &mut pc,
            );
            (r.c2MakeProxy)(
                &mut shape as *mut c2Circle as *const c_void,
                C2_TYPE_CIRCLE,
                &mut pr,
            );
            eq_proxy(&format!("c2MakeProxy CIRCLE #{i}"), &pc, &pr);
            eq_int("count", pc.count, 1);
        }
    }
}

#[test]
fn b12_makeproxy_aabb() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB12);
    unsafe {
        for i in 0..N {
            let mut shape = if rng.below(4) == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb(500.0)
            };
            let mut pc = dirty_proxy(-3.5);
            let mut pr = pc;
            (c.c2MakeProxy)(
                &mut shape as *mut c2AABB as *const c_void,
                C2_TYPE_AABB,
                &mut pc,
            );
            (r.c2MakeProxy)(
                &mut shape as *mut c2AABB as *const c_void,
                C2_TYPE_AABB,
                &mut pr,
            );
            eq_proxy(&format!("c2MakeProxy AABB #{i}"), &pc, &pr);
            eq_int("count", pc.count, 4);
        }
    }
}

#[test]
fn b13_makeproxy_capsule() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB13);
    unsafe {
        for i in 0..N {
            let mut shape = if rng.below(4) == 0 {
                c2Capsule {
                    a: rng.wild_v(),
                    b: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.capsule(500.0)
            };
            let mut pc = dirty_proxy(0.0);
            let mut pr = pc;
            (c.c2MakeProxy)(
                &mut shape as *mut c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                &mut pc,
            );
            (r.c2MakeProxy)(
                &mut shape as *mut c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                &mut pr,
            );
            eq_proxy(&format!("c2MakeProxy CAPSULE #{i}"), &pc, &pr);
            eq_int("count", pc.count, 2);
        }
    }
}

// ---------------------------------------------------------------------------
// B14 / B15 / B16 — c2GJKSimplexMetric for count 1 / 2 / 3
// ---------------------------------------------------------------------------

#[test]
fn b14_b15_b16_simplex_metric() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB14);
    unsafe {
        for count in [1i32, 2, 3] {
            for i in 0..N {
                let mut sc = rng.simplex(count, 100.0);
                // Special shapes: coincident points, collinear triples, huge coords.
                match rng.below(6) {
                    0 => sc.verts[1].p = sc.verts[0].p,
                    1 => {
                        sc.verts[1].p = sc.verts[0].p;
                        sc.verts[2].p = sc.verts[0].p;
                    }
                    2 => {
                        // collinear: c = a + 2*(b-a)
                        sc.verts[2].p = c2v {
                            x: sc.verts[0].p.x + 2.0 * (sc.verts[1].p.x - sc.verts[0].p.x),
                            y: sc.verts[0].p.y + 2.0 * (sc.verts[1].p.y - sc.verts[0].p.y),
                        };
                    }
                    3 => {
                        sc.verts[0].p = c2v { x: 1.0e30, y: -1.0e30 };
                        sc.verts[1].p = c2v { x: -1.0e30, y: 1.0e30 };
                        sc.verts[2].p = rng.wild_v();
                    }
                    _ => {}
                }
                let mut sr = sc;
                let cv = (c.c2GJKSimplexMetric)(&mut sc);
                let rv = (r.c2GJKSimplexMetric)(&mut sr);
                eq_f32(&format!("c2GJKSimplexMetric count={count} #{i}"), cv, rv);
                eq_simplex(&format!("simplex unchanged count={count} #{i}"), &sc, &sr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B17 … B20 — c22, with proven coverage of all 3 branches
// ---------------------------------------------------------------------------

/// Mirrors the C branch selection of `c22` using the C library's own helpers so
/// the classification cannot drift from the implementation under test.
unsafe fn c22_branch(c: &Api, s: &c2Simplex) -> usize {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = (c.c2Dot)(b, (c.c2Sub)(b, a));
    let v = (c.c2Dot)(a, (c.c2Sub)(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

#[test]
fn b17_b18_b19_b20_c22() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB22);
    let mut hits = [0usize; 3];
    unsafe {
        for i in 0..N * 8 {
            let mut sc = match rng.below(8) {
                // Tight coordinates around the origin so all three Voronoi
                // regions of segment AB are reached.
                0..=4 => rng.simplex(2, 4.0),
                5 => {
                    let mut s = rng.simplex(2, 4.0);
                    s.verts[1].p = s.verts[0].p; // degenerate segment
                    s
                }
                6 => rng.simplex(2, 1.0e18),
                _ => {
                    let mut s = rng.simplex(2, 4.0);
                    s.verts[0].p = rng.wild_v();
                    s.verts[1].p = rng.wild_v();
                    s
                }
            };
            sc.count = [0i32, 1, 2, 3, 4][rng.below(5) as usize];
            let mut sr = sc;
            hits[c22_branch(c, &sc)] += 1;
            (c.c22)(&mut sc);
            (r.c22)(&mut sr);
            eq_simplex(&format!("c22 #{i}"), &sc, &sr);
        }
        // Hand-built inputs that pin each branch even if the fuzz got unlucky.
        let mk = |ap: c2v, bp: c2v| {
            let mut s = c2Simplex::default();
            s.verts[0].p = ap;
            s.verts[1].p = bp;
            s.verts[0].sA = c2v { x: 1.0, y: 2.0 };
            s.verts[0].sB = c2v { x: 3.0, y: 4.0 };
            s.verts[1].sA = c2v { x: 5.0, y: 6.0 };
            s.verts[1].sB = c2v { x: 7.0, y: 8.0 };
            s.verts[0].iA = 1;
            s.verts[1].iA = 2;
            s.div = 3.0;
            s.count = 2;
            s
        };
        let pinned = [
            // origin "behind" A  -> v <= 0
            mk(c2v { x: 1.0, y: 0.0 }, c2v { x: 3.0, y: 0.0 }),
            // origin "past" B    -> u <= 0
            mk(c2v { x: -3.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }),
            // origin projects inside AB
            mk(c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }),
        ];
        for (k, mut sc) in pinned.into_iter().enumerate() {
            let mut sr = sc;
            assert_eq!(c22_branch(c, &sc), k, "pinned c22 case {k} took another branch");
            hits[k] += 1;
            (c.c22)(&mut sc);
            (r.c22)(&mut sr);
            eq_simplex(&format!("c22 pinned {k}"), &sc, &sr);
        }
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c22 branch coverage incomplete: {hits:?}"
    );
    println!("c22 branch hits: {hits:?}");
}

// ---------------------------------------------------------------------------
// B21 … B28 — c23, with proven coverage of all 7 branches
// ---------------------------------------------------------------------------

unsafe fn c23_branch(c: &Api, s: &c2Simplex) -> usize {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let cc = s.verts[2].p;
    let dot = c.c2Dot;
    let sub = c.c2Sub;
    let det = c.c2Det2;
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(cc, sub(cc, b));
    let vBC = dot(b, sub(b, cc));
    let uCA = dot(a, sub(a, cc));
    let vCA = dot(cc, sub(cc, a));
    let area = det(sub(b, a), sub(cc, a));
    let uABC = det(b, cc) * area;
    let vABC = det(cc, a) * area;
    let wABC = det(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn b21_to_b28_c23() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB23);
    let mut hits = [0usize; 7];
    unsafe {
        for i in 0..N * 16 {
            let mut sc = match rng.below(10) {
                0..=5 => rng.simplex(3, 4.0),
                6 => {
                    // triangle guaranteed to contain the origin
                    let mut s = rng.simplex(3, 4.0);
                    s.verts[0].p = c2v {
                        x: rng.uniform(-4.0, -1.0),
                        y: rng.uniform(-4.0, -1.0),
                    };
                    s.verts[1].p = c2v {
                        x: rng.uniform(1.0, 4.0),
                        y: rng.uniform(-4.0, -1.0),
                    };
                    s.verts[2].p = c2v {
                        x: rng.uniform(-1.0, 1.0),
                        y: rng.uniform(1.0, 4.0),
                    };
                    s
                }
                7 => {
                    // degenerate: two or three coincident vertices
                    let mut s = rng.simplex(3, 4.0);
                    s.verts[2].p = s.verts[1].p;
                    if rng.bool() {
                        s.verts[1].p = s.verts[0].p;
                    }
                    s
                }
                8 => {
                    // collinear
                    let mut s = rng.simplex(3, 4.0);
                    let t = rng.uniform(-3.0, 3.0);
                    s.verts[2].p = c2v {
                        x: s.verts[0].p.x + t * (s.verts[1].p.x - s.verts[0].p.x),
                        y: s.verts[0].p.y + t * (s.verts[1].p.y - s.verts[0].p.y),
                    };
                    s
                }
                _ => {
                    let mut s = rng.simplex(3, 4.0);
                    s.verts[0].p = rng.wild_v();
                    s.verts[1].p = rng.wild_v();
                    s.verts[2].p = rng.wild_v();
                    s
                }
            };
            sc.count = [0i32, 1, 2, 3, 4][rng.below(5) as usize];
            let mut sr = sc;
            hits[c23_branch(c, &sc)] += 1;
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
            eq_simplex(&format!("c23 #{i}"), &sc, &sr);
        }
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c23 branch coverage incomplete: {hits:?} (need all 7 non-zero)"
    );
    println!("c23 branch hits: {hits:?}");
}

// ---------------------------------------------------------------------------
// B29 … B32 — c2D for every `count` and both `count == 2` sub-branches
// ---------------------------------------------------------------------------

#[test]
fn b29_to_b32_cD() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB2D);
    // [count1, skew, ccw90, other-count]
    let mut hits = [0usize; 4];
    unsafe {
        for i in 0..N * 8 {
            let count = [-1i32, 0, 1, 2, 3, 4][rng.below(6) as usize];
            let mut sc = rng.simplex(count, 4.0);
            if rng.below(4) == 0 {
                sc.verts[0].p = rng.wild_v();
                sc.verts[1].p = rng.wild_v();
            }
            if rng.below(6) == 0 {
                // force c2Det2(ab, -a) == 0 exactly: origin collinear with AB
                sc.verts[0].p = c2v { x: 1.0, y: 2.0 };
                sc.verts[1].p = c2v { x: 3.0, y: 6.0 };
            }
            let mut sr = sc;
            match count {
                1 => hits[0] += 1,
                2 => {
                    let ab = (c.c2Sub)(sc.verts[1].p, sc.verts[0].p);
                    let det = (c.c2Det2)(ab, (c.c2Neg)(sc.verts[0].p));
                    if det > 0.0 {
                        hits[1] += 1
                    } else {
                        hits[2] += 1
                    }
                }
                _ => hits[3] += 1,
            }
            let cv = (c.c2D)(&mut sc);
            let rv = (r.c2D)(&mut sr);
            eq_v(&format!("c2D count={count} #{i}"), cv, rv);
            eq_simplex(&format!("c2D simplex count={count} #{i}"), &sc, &sr);
        }
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c2D branch coverage incomplete: {hits:?}"
    );
    println!("c2D branch hits: {hits:?}");
}

// ---------------------------------------------------------------------------
// B33 / B34 — c2L
// ---------------------------------------------------------------------------

#[test]
fn b33_b34_cL() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB2C);
    unsafe {
        for i in 0..N * 8 {
            let count = [-1i32, 0, 1, 2, 3, 4][rng.below(6) as usize];
            let mut sc = rng.simplex(count, 100.0);
            // stress `div`: 0, denormal, huge, negative, NaN, inf
            sc.div = match rng.below(8) {
                0 => 0.0,
                1 => -0.0,
                2 => FLT_MIN_POS,
                3 => 1.0e-45,
                4 => FLT_MAX,
                5 => f32::INFINITY,
                6 => f32::NAN,
                _ => sc.div,
            };
            if rng.below(4) == 0 {
                sc.verts[0].u = rng.wild_f32();
                sc.verts[1].u = rng.wild_f32();
                sc.verts[0].p = rng.wild_v();
                sc.verts[1].p = rng.wild_v();
            }
            let mut sr = sc;
            let cv = (c.c2L)(&mut sc);
            let rv = (r.c2L)(&mut sr);
            eq_v(&format!("c2L count={count} #{i}"), cv, rv);
            eq_simplex(&format!("c2L simplex count={count} #{i}"), &sc, &sr);
        }
    }
}

// ---------------------------------------------------------------------------
// B35 / B36 / B37 — c2Witness
// ---------------------------------------------------------------------------

#[test]
fn b35_b36_b37_witness() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB27);
    unsafe {
        for i in 0..N * 8 {
            let count = [-1i32, 0, 1, 2, 3, 4][rng.below(6) as usize];
            let mut sc = rng.simplex(count, 100.0);
            sc.div = match rng.below(8) {
                0 => 0.0,
                1 => -0.0,
                2 => FLT_MIN_POS,
                3 => FLT_MAX,
                4 => f32::INFINITY,
                5 => f32::NAN,
                _ => sc.div,
            };
            if rng.below(4) == 0 {
                for v in sc.verts.iter_mut() {
                    v.sA = rng.wild_v();
                    v.sB = rng.wild_v();
                    v.u = rng.wild_f32();
                }
            }
            let mut sr = sc;
            let mut ac = c2v { x: 111.0, y: 222.0 };
            let mut bc = c2v { x: 333.0, y: 444.0 };
            let mut ar = ac;
            let mut br = bc;
            (c.c2Witness)(&mut sc, &mut ac, &mut bc);
            (r.c2Witness)(&mut sr, &mut ar, &mut br);
            eq_v(&format!("c2Witness a count={count} #{i}"), ac, ar);
            eq_v(&format!("c2Witness b count={count} #{i}"), bc, br);
            eq_simplex(&format!("c2Witness simplex count={count} #{i}"), &sc, &sr);
        }
        let _: c_int = 0;
    }
}
