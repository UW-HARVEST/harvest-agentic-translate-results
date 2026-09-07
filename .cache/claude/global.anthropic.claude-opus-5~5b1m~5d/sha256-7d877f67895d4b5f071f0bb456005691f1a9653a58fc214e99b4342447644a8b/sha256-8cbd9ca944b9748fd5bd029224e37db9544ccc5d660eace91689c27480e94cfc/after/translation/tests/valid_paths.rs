//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Both implementations are loaded from their `.so` via libloading; every call
//! crosses the FFI boundary exactly as an external consumer's would.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 4000; // randomized inputs per row

// ===========================================================================
// Rows 1-11: scalar / vector helpers
// ===========================================================================

#[test]
fn row01_c2V() {
    let (c, r) = sym::<FnffV>("c2V");
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let (x, y) = (rng.wild(), rng.wild());
        unsafe { same("row01 c2V", &format!("{x:?},{y:?}"), c(x, y), r(x, y)) }
    }
}

#[test]
fn row02_c2Mulvs() {
    let (c, r) = sym::<FnVfV>("c2Mulvs");
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let a = rng.wild_vec();
        let b = rng.wild();
        unsafe {
            same(
                "row02 c2Mulvs",
                &format!("{a:?} * {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
}

#[test]
fn row03_c2Maxv_c2Minv() {
    let (cmax, rmax) = sym::<FnVVV>("c2Maxv");
    let (cmin, rmin) = sym::<FnVVV>("c2Minv");
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let mut a = rng.wild_vec();
        let mut b = rng.wild_vec();
        if rng.below(4) == 0 {
            b = a; // equal components
        }
        if rng.below(8) == 0 {
            a.y = b.y;
        }
        unsafe {
            same("row03 c2Maxv", &format!("{a:?} {b:?}"), cmax(a, b), rmax(a, b));
            same("row03 c2Maxv rev", &format!("{b:?} {a:?}"), cmax(b, a), rmax(b, a));
            same("row03 c2Minv", &format!("{a:?} {b:?}"), cmin(a, b), rmin(a, b));
            same("row03 c2Minv rev", &format!("{b:?} {a:?}"), cmin(b, a), rmin(b, a));
        }
    }
}

#[test]
fn row04_c2Clampv() {
    let (c, r) = sym::<FnVVVV>("c2Clampv");
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let a = rng.wild_vec();
        let bb = rng.aabb(); // includes inverted and zero-extent boxes
        unsafe {
            same(
                "row04 c2Clampv",
                &format!("{a:?} lo={:?} hi={:?}", bb.min, bb.max),
                c(a, bb.min, bb.max),
                r(a, bb.min, bb.max),
            )
        }
    }
}

#[test]
fn row05_c2Sub_c2Add() {
    let (cs, rs) = sym::<FnVVV>("c2Sub");
    let (ca, ra) = sym::<FnVVV>("c2Add");
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        let a = rng.wild_vec();
        let b = if rng.below(4) == 0 { a } else { rng.wild_vec() };
        unsafe {
            same("row05 c2Sub", &format!("{a:?} {b:?}"), cs(a, b), rs(a, b));
            same("row05 c2Add", &format!("{a:?} {b:?}"), ca(a, b), ra(a, b));
        }
    }
}

#[test]
fn row06_c2Dot() {
    let (c, r) = sym::<FnVVf>("c2Dot");
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        let a = rng.wild_vec();
        let b = rng.wild_vec();
        unsafe { same("row06 c2Dot", &format!("{a:?} {b:?}"), c(a, b), r(a, b)) }
    }
    // deliberate overflow to +/-inf
    for &m in &[1e20f32, 1e30, f32::MAX, -f32::MAX] {
        let a = c2v { x: m, y: m };
        unsafe { same("row06 c2Dot overflow", &format!("{m}"), c(a, a), r(a, a)) }
    }
}

#[test]
fn row07_c2Det2() {
    let (c, r) = sym::<FnVVf>("c2Det2");
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        let a = rng.wild_vec();
        // 1/4 collinear -> det == 0
        let b = if rng.below(4) == 0 {
            let k = rng.coord();
            c2v { x: a.x * k, y: a.y * k }
        } else {
            rng.wild_vec()
        };
        unsafe { same("row07 c2Det2", &format!("{a:?} {b:?}"), c(a, b), r(a, b)) }
    }
}

#[test]
fn row08_c2Len() {
    let (c, r) = sym::<FnVf>("c2Len");
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        let a = rng.wild_vec();
        unsafe { same("row08 c2Len", &format!("{a:?}"), c(a), r(a)) }
    }
    for &v in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: 0.0 },
    ] {
        unsafe { same("row08 c2Len special", &format!("{v:?}"), c(v), r(v)) }
    }
}

#[test]
fn row09_c2Div() {
    let (c, r) = sym::<FnVfV>("c2Div");
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let a = rng.wild_vec();
        let mut b = rng.wild();
        if b == 0.0 {
            b = 1.0;
        }
        unsafe { same("row09 c2Div", &format!("{a:?} / {b:?}"), c(a, b), r(a, b)) }
    }
}

#[test]
fn row10_c2Norm() {
    let (c, r) = sym::<FnVV>("c2Norm");
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let mut a = rng.wild_vec();
        if a.x == 0.0 && a.y == 0.0 {
            a.x = 1.0;
        }
        unsafe { same("row10 c2Norm", &format!("{a:?}"), c(a), r(a)) }
    }
    for &v in &[
        c2v { x: f32::MIN_POSITIVE, y: f32::MIN_POSITIVE },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
        c2v { x: 1e30, y: 1e30 },
    ] {
        unsafe { same("row10 c2Norm subnormal", &format!("{v:?}"), c(v), r(v)) }
    }
}

#[test]
fn row11_c2Neg_c2Skew_c2CCW90() {
    let (cn, rn) = sym::<FnVV>("c2Neg");
    let (cs, rs) = sym::<FnVV>("c2Skew");
    let (cc, rc) = sym::<FnVV>("c2CCW90");
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        let a = rng.wild_vec();
        unsafe {
            same("row11 c2Neg", &format!("{a:?}"), cn(a), rn(a));
            same("row11 c2Skew", &format!("{a:?}"), cs(a), rs(a));
            same("row11 c2CCW90", &format!("{a:?}"), cc(a), rc(a));
        }
    }
    for &v in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        // sign of zero must survive the negation
        unsafe {
            same("row11 c2Neg zero", &format!("{v:?}"), cn(v), rn(v));
            same("row11 c2Skew zero", &format!("{v:?}"), cs(v), rs(v));
            same("row11 c2CCW90 zero", &format!("{v:?}"), cc(v), rc(v));
        }
    }
}

// ===========================================================================
// Rows 12-14: rotations and transforms
// ===========================================================================

#[test]
fn row12_identities() {
    let (cr, rr) = sym::<FnR>("c2RotIdentity");
    let (cx, rx) = sym::<FnX>("c2xIdentity");
    unsafe {
        same("row12 c2RotIdentity", "()", cr(), rr());
        same("row12 c2xIdentity", "()", cx(), rx());
    }
}

#[test]
fn row13_c2Mulrv_c2MulrvT() {
    let (cm, rm) = sym::<FnRVV>("c2Mulrv");
    let (ct, rt) = sym::<FnRVV>("c2MulrvT");
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..N {
        let rot = rng.rot();
        let v = rng.wild_vec();
        unsafe {
            same("row13 c2Mulrv", &format!("{rot:?} {v:?}"), cm(rot, v), rm(rot, v));
            same("row13 c2MulrvT", &format!("{rot:?} {v:?}"), ct(rot, v), rt(rot, v));
        }
    }
}

#[test]
fn row14_c2Mulxv() {
    let (c, r) = sym::<FnXVV>("c2Mulxv");
    let mut rng = Rng::new(SEED ^ 14);
    let fixed = [
        c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }, // identity
        c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 0.0, s: 1.0 } }, // rotation only
        c2x { p: c2v { x: 3.5, y: -7.0 }, r: c2r { c: 1.0, s: 0.0 } }, // translation only
        c2x { p: c2v { x: -2.0, y: 11.0 }, r: c2r { c: 0.6, s: 0.8 } }, // both
    ];
    for x in fixed {
        for _ in 0..64 {
            let v = rng.wild_vec();
            unsafe { same("row14 c2Mulxv fixed", &format!("{x:?} {v:?}"), c(x, v), r(x, v)) }
        }
    }
    for _ in 0..N {
        let x = rng.xform();
        let v = rng.wild_vec();
        unsafe { same("row14 c2Mulxv", &format!("{x:?} {v:?}"), c(x, v), r(x, v)) }
    }
}

// ===========================================================================
// Rows 15-18: proxies
// ===========================================================================

#[test]
fn row15_c2BBVerts() {
    let (c, r) = sym::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let mut bb = rng.aabb();
        let mut co = [c2v { x: 1.0, y: 2.0 }; 4];
        let mut ro = co;
        unsafe {
            c(co.as_mut_ptr(), &mut bb);
            r(ro.as_mut_ptr(), &mut bb);
        }
        for i in 0..4 {
            same(
                "row15 c2BBVerts",
                &format!("{bb:?} vert {i}"),
                co[i],
                ro[i],
            );
        }
    }
}

fn proxy_of(f: FnMakeProxy, shape: &Shape) -> c2Proxy {
    // Seeded with a recognisable pattern so an un-written field is visible.
    let mut p = c2Proxy {
        radius: -99.0,
        count: -99,
        verts: [c2v { x: -99.0, y: -99.0 }; 8],
    };
    unsafe { f(shape.as_ptr(), shape.ty(), &mut p) };
    p
}

#[test]
fn row16_makeproxy_circle() {
    let (c, r) = sym::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..N {
        let s = Shape::Circle(rng.circle());
        same(
            "row16 c2MakeProxy CIRCLE",
            &s.show(),
            proxy_of(*c, &s),
            proxy_of(*r, &s),
        );
    }
}

#[test]
fn row17_makeproxy_aabb() {
    let (c, r) = sym::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        let s = Shape::Aabb(rng.aabb());
        same(
            "row17 c2MakeProxy AABB",
            &s.show(),
            proxy_of(*c, &s),
            proxy_of(*r, &s),
        );
    }
}

#[test]
fn row18_makeproxy_capsule() {
    let (c, r) = sym::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..N {
        let s = Shape::Capsule(rng.capsule());
        same(
            "row18 c2MakeProxy CAPSULE",
            &s.show(),
            proxy_of(*c, &s),
            proxy_of(*r, &s),
        );
    }
}

// ===========================================================================
// Rows 19-22: c2Support at every proxy vertex count
// ===========================================================================

fn support_row(row: &str, count: i32, seed: u64) {
    let (c, r) = sym::<FnSupport>("c2Support");
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let mut verts = [c2v::default(); 8];
        for i in 0..8 {
            verts[i] = rng.vec();
        }
        // 1/4 of the time make several verts identical to force `dot > dmax` ties
        if rng.below(4) == 0 {
            let v = verts[0];
            for i in 0..count as usize {
                verts[i] = v;
            }
        }
        let d = match rng.below(6) {
            0 => c2v { x: 1.0, y: 0.0 },
            1 => c2v { x: 0.0, y: 1.0 },
            2 => c2v { x: -1.0, y: 0.0 },
            3 => c2v { x: 0.0, y: -1.0 },
            4 => c2v { x: 0.0, y: 0.0 },
            _ => rng.vec(),
        };
        let input = format!("count={count} d={d:?} verts={verts:?}");
        unsafe {
            same(
                row,
                &input,
                c(verts.as_ptr(), count, d),
                r(verts.as_ptr(), count, d),
            )
        }
    }
}

#[test]
fn row19_support_count1() {
    support_row("row19 c2Support count=1", 1, SEED ^ 19);
}

#[test]
fn row20_support_count2() {
    support_row("row20 c2Support count=2", 2, SEED ^ 20);
}

#[test]
fn row21_support_count4() {
    support_row("row21 c2Support count=4", 4, SEED ^ 21);
}

#[test]
fn row22_support_count8() {
    support_row("row22 c2Support count=8", 8, SEED ^ 22);
}

// ===========================================================================
// Rows 23-25: c2GJKSimplexMetric
// ===========================================================================

fn metric_row(row: &str, count: i32, seed: u64) {
    let (c, r) = sym::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let mut s1 = rng.simplex(count);
        if rng.below(6) == 0 {
            s1.verts[1].p = s1.verts[0].p; // degenerate
        }
        let mut s2 = s1;
        unsafe {
            let (rc, rr) = (c(&mut s1), r(&mut s2));
            same(row, &s1.show(), rc, rr);
            // the function must not mutate the simplex
            same(row, "simplex unchanged", s1, s2);
        }
    }
}

#[test]
fn row23_metric_count1() {
    metric_row("row23 c2GJKSimplexMetric count=1", 1, SEED ^ 23);
}

#[test]
fn row24_metric_count2() {
    metric_row("row24 c2GJKSimplexMetric count=2", 2, SEED ^ 24);
}

#[test]
fn row25_metric_count3() {
    metric_row("row25 c2GJKSimplexMetric count=3", 3, SEED ^ 25);
}

// ===========================================================================
// Rows 26-31: the c22 / c23 simplex solvers
// ===========================================================================

/// Run `c22`/`c23` on both libs, compare the whole mutated simplex.
fn solver_diff(name: &str, s: c2Simplex) {
    let (c, r) = sym::<FnSimplexVoid>(name);
    let mut sc = s;
    let mut sr = s;
    unsafe {
        c(&mut sc);
        r(&mut sr);
    }
    same(name, &s.show(), sc, sr);
}

/// Which arm of `c22` does this simplex take? (mirrors the C conditions)
fn c22_arm(s: &c2Simplex) -> u32 {
    let (a, b) = (s.verts[0].p, s.verts[1].p);
    let dot = |p: c2v, q: c2v| p.x * q.x + p.y * q.y;
    let sub = |p: c2v, q: c2v| c2v { x: p.x - q.x, y: p.y - q.y };
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

#[test]
fn row26_28_29_c22_all_arms() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut hits = [0usize; 3];
    for _ in 0..N * 3 {
        let mut s = rng.simplex(2);
        match rng.below(5) {
            0 => s.verts[1].p = s.verts[0].p,          // a == b
            1 => s.verts[0].p = c2v { x: 0.0, y: 0.0 }, // origin at a
            2 => s.verts[1].p = c2v { x: 0.0, y: 0.0 }, // origin at b
            _ => {}
        }
        hits[c22_arm(&s) as usize] += 1;
        solver_diff("c22", s);
    }
    assert!(
        hits.iter().all(|&h| h > 20),
        "rows 26/27/28: c22 arm coverage too thin: {hits:?}"
    );
}

#[test]
fn row27_c22_arm_u_le_zero() {
    // Targeted construction of the `u <= 0` arm (origin beyond b): pick a, then
    // b on the far side so that dot(b, b-a) <= 0.
    let mut rng = Rng::new(SEED ^ 27);
    let mut n = 0;
    for _ in 0..N * 4 {
        let a = rng.vec();
        let t = rng.range(0.0, 1.0);
        let b = c2v { x: a.x * t, y: a.y * t }; // b between origin and a
        let mut s = rng.simplex(2);
        s.verts[0].p = a;
        s.verts[1].p = b;
        if c22_arm(&s) == 1 {
            n += 1;
        }
        solver_diff("c22", s);
    }
    assert!(n > 50, "row27: `u <= 0` arm hit only {n} times");
}

/// Which arm of `c23` does this simplex take? (mirrors the C conditions)
fn c23_arm(s: &c2Simplex) -> u32 {
    let (a, b, c) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
    let dot = |p: c2v, q: c2v| p.x * q.x + p.y * q.y;
    let sub = |p: c2v, q: c2v| c2v { x: p.x - q.x, y: p.y - q.y };
    let det = |p: c2v, q: c2v| p.x * q.y - p.y * q.x;
    let uab = dot(b, sub(b, a));
    let vab = dot(a, sub(a, b));
    let ubc = dot(c, sub(c, b));
    let vbc = dot(b, sub(b, c));
    let uca = dot(a, sub(a, c));
    let vca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uabc = det(b, c) * area;
    let vabc = det(c, a) * area;
    let wabc = det(a, b) * area;
    if vab <= 0.0 && uca <= 0.0 {
        0
    } else if uab <= 0.0 && vbc <= 0.0 {
        1
    } else if ubc <= 0.0 && vca <= 0.0 {
        2
    } else if uab > 0.0 && vab > 0.0 && wabc <= 0.0 {
        3
    } else if ubc > 0.0 && vbc > 0.0 && uabc <= 0.0 {
        4
    } else if uca > 0.0 && vca > 0.0 && vabc <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn row30_c23_all_seven_arms() {
    let mut rng = Rng::new(SEED ^ 30);
    let mut hits = [0usize; 7];
    for _ in 0..N * 6 {
        let mut s = rng.simplex(3);
        // Mix in triangles that actually surround the origin so arm 6 is reached.
        if rng.below(3) == 0 {
            let scale = rng.range(0.5, 20.0);
            let base = rng.range(0.0, 6.28);
            for k in 0..3 {
                let ang = base + k as f32 * 2.094_395_1;
                s.verts[k].p = c2v {
                    x: ang.cos() * scale,
                    y: ang.sin() * scale,
                };
            }
        }
        hits[c23_arm(&s) as usize] += 1;
        solver_diff("c23", s);
    }
    assert!(
        hits.iter().all(|&h| h > 5),
        "row30: c23 arm coverage too thin: {hits:?}"
    );
}

#[test]
fn row31_c23_degenerate() {
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..N {
        let mut s = rng.simplex(3);
        match rng.below(5) {
            0 => s.verts[1].p = s.verts[0].p,
            1 => s.verts[2].p = s.verts[0].p,
            2 => s.verts[2].p = s.verts[1].p,
            3 => {
                s.verts[1].p = s.verts[0].p;
                s.verts[2].p = s.verts[0].p;
            }
            _ => {
                // collinear: c = a + t*(b-a)  => area == 0
                let t = rng.range(-3.0, 3.0);
                s.verts[2].p = c2v {
                    x: s.verts[0].p.x + t * (s.verts[1].p.x - s.verts[0].p.x),
                    y: s.verts[0].p.y + t * (s.verts[1].p.y - s.verts[0].p.y),
                };
            }
        }
        solver_diff("c23", s);
    }
}

// ===========================================================================
// Rows 32-34: c2D / c2L / c2Witness at every count
// ===========================================================================

#[test]
fn row32_c2D() {
    let (c, r) = sym::<FnSimplexV>("c2D");
    let mut rng = Rng::new(SEED ^ 32);
    let mut pos = 0usize;
    let mut neg = 0usize;
    for count in [1i32, 2, 3] {
        for _ in 0..N {
            let mut s1 = rng.simplex(count);
            if rng.below(5) == 0 {
                s1.verts[1].p = s1.verts[0].p;
            }
            if count == 2 {
                let ab = c2v {
                    x: s1.verts[1].p.x - s1.verts[0].p.x,
                    y: s1.verts[1].p.y - s1.verts[0].p.y,
                };
                let det = ab.x * -s1.verts[0].p.y - ab.y * -s1.verts[0].p.x;
                if det > 0.0 {
                    pos += 1
                } else {
                    neg += 1
                }
            }
            let mut s2 = s1;
            unsafe {
                same(
                    "row32 c2D",
                    &format!("count={count} {}", s1.show()),
                    c(&mut s1),
                    r(&mut s2),
                );
            }
            same("row32 c2D no-mutate", "", s1, s2);
        }
    }
    assert!(pos > 50 && neg > 50, "row32: det sign coverage {pos}/{neg}");
}

#[test]
fn row33_c2L() {
    let (c, r) = sym::<FnSimplexV>("c2L");
    let mut rng = Rng::new(SEED ^ 33);
    for count in [1i32, 2] {
        for _ in 0..N {
            let mut s1 = rng.simplex(count);
            let mut s2 = s1;
            unsafe {
                same(
                    "row33 c2L",
                    &format!("count={count} {}", s1.show()),
                    c(&mut s1),
                    r(&mut s2),
                )
            }
        }
    }
}

#[test]
fn row34_c2Witness() {
    let (c, r) = sym::<FnWitness>("c2Witness");
    let mut rng = Rng::new(SEED ^ 34);
    for count in [1i32, 2, 3] {
        for _ in 0..N {
            let mut s1 = rng.simplex(count);
            let mut s2 = s1;
            let mut ca = c2v { x: 7.0, y: 7.0 };
            let mut cb = c2v { x: 8.0, y: 8.0 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                c(&mut s1, &mut ca, &mut cb);
                r(&mut s2, &mut ra, &mut rb);
            }
            same(
                "row34 c2Witness",
                &format!("count={count} {}", s1.show()),
                (ca, cb),
                (ra, rb),
            );
        }
    }
}

// ===========================================================================
// Rows 35-48: c2GJK over every type pair x option combination
// ===========================================================================

fn gjk_pair_row(row: &str, ta: C2_TYPE, tb: C2_TYPE, seed: u64, mk: fn(&mut Rng) -> GjkOpts) {
    let mut rng = Rng::new(seed);
    for _ in 0..(N / 2) {
        let a = rand_shape(&mut rng, ta);
        let b = rand_shape(&mut rng, tb);
        let o = mk(&mut rng);
        gjk_same(row, &a, &b, &o);
    }
}

fn opts_plain(_r: &mut Rng) -> GjkOpts {
    GjkOpts::default()
}
fn opts_no_radius(_r: &mut Rng) -> GjkOpts {
    GjkOpts { use_radius: 0, ..Default::default() }
}
fn opts_ax(r: &mut Rng) -> GjkOpts {
    GjkOpts { ax: Some(r.xform()), ..Default::default() }
}
fn opts_bx(r: &mut Rng) -> GjkOpts {
    GjkOpts { bx: Some(r.xform()), ..Default::default() }
}
fn opts_both(r: &mut Rng) -> GjkOpts {
    GjkOpts { ax: Some(r.xform()), bx: Some(r.xform()), ..Default::default() }
}
fn opts_both_no_radius(r: &mut Rng) -> GjkOpts {
    GjkOpts {
        ax: Some(r.xform()),
        bx: Some(r.xform()),
        use_radius: 0,
        ..Default::default()
    }
}

#[test]
fn row35_gjk_circle_circle() {
    gjk_pair_row("row35 GJK CIRCLE x CIRCLE", C2_TYPE_CIRCLE, C2_TYPE_CIRCLE, SEED ^ 35, opts_plain);
    // hand-built separated / touching / overlapping / concentric cases
    let cases = [
        ((0.0f32, 0.0f32, 1.0f32), (10.0f32, 0.0f32, 1.0f32)), // separated
        ((0.0, 0.0, 1.0), (2.0, 0.0, 1.0)),                    // exactly touching
        ((0.0, 0.0, 5.0), (2.0, 0.0, 5.0)),                    // overlapping
        ((0.0, 0.0, 3.0), (0.0, 0.0, 3.0)),                    // concentric identical
        ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0)),                    // both degenerate points
        ((0.0, 0.0, 0.0), (1.0, 1.0, 0.0)),                    // zero radius separated
    ];
    for (a, b) in cases {
        let sa = Shape::Circle(c2Circle { p: c2v { x: a.0, y: a.1 }, r: a.2 });
        let sb = Shape::Circle(c2Circle { p: c2v { x: b.0, y: b.1 }, r: b.2 });
        for ur in [0, 1] {
            gjk_same(
                "row35 GJK CIRCLE x CIRCLE fixed",
                &sa,
                &sb,
                &GjkOpts { use_radius: ur, ..Default::default() },
            );
        }
    }
}

#[test]
fn row36_gjk_circle_aabb() {
    gjk_pair_row("row36 GJK CIRCLE x AABB", C2_TYPE_CIRCLE, C2_TYPE_AABB, SEED ^ 36, opts_plain);
}

#[test]
fn row37_gjk_circle_capsule() {
    gjk_pair_row("row37 GJK CIRCLE x CAPSULE", C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, SEED ^ 37, opts_plain);
}

#[test]
fn row38_gjk_aabb_circle() {
    gjk_pair_row("row38 GJK AABB x CIRCLE", C2_TYPE_AABB, C2_TYPE_CIRCLE, SEED ^ 38, opts_plain);
}

#[test]
fn row39_gjk_aabb_aabb() {
    gjk_pair_row("row39 GJK AABB x AABB", C2_TYPE_AABB, C2_TYPE_AABB, SEED ^ 39, opts_plain);
}

#[test]
fn row40_gjk_aabb_capsule() {
    gjk_pair_row("row40 GJK AABB x CAPSULE", C2_TYPE_AABB, C2_TYPE_CAPSULE, SEED ^ 40, opts_plain);
}

#[test]
fn row41_gjk_capsule_circle() {
    gjk_pair_row("row41 GJK CAPSULE x CIRCLE", C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, SEED ^ 41, opts_plain);
}

#[test]
fn row42_gjk_capsule_aabb() {
    gjk_pair_row("row42 GJK CAPSULE x AABB", C2_TYPE_CAPSULE, C2_TYPE_AABB, SEED ^ 42, opts_plain);
}

#[test]
fn row43_gjk_capsule_capsule() {
    gjk_pair_row("row43 GJK CAPSULE x CAPSULE", C2_TYPE_CAPSULE, C2_TYPE_CAPSULE, SEED ^ 43, opts_plain);
}

#[test]
fn row44_gjk_all_pairs_use_radius_zero() {
    for (i, &ta) in ALL_TYPES.iter().enumerate() {
        for (j, &tb) in ALL_TYPES.iter().enumerate() {
            gjk_pair_row(
                "row44 GJK use_radius=0",
                ta,
                tb,
                SEED ^ 4400 ^ ((i * 3 + j) as u64),
                opts_no_radius,
            );
        }
    }
}

#[test]
fn row45_gjk_all_pairs_ax_only() {
    for (i, &ta) in ALL_TYPES.iter().enumerate() {
        for (j, &tb) in ALL_TYPES.iter().enumerate() {
            gjk_pair_row("row45 GJK ax only", ta, tb, SEED ^ 4500 ^ ((i * 3 + j) as u64), opts_ax);
        }
    }
}

#[test]
fn row46_gjk_all_pairs_bx_only() {
    for (i, &ta) in ALL_TYPES.iter().enumerate() {
        for (j, &tb) in ALL_TYPES.iter().enumerate() {
            gjk_pair_row("row46 GJK bx only", ta, tb, SEED ^ 4600 ^ ((i * 3 + j) as u64), opts_bx);
        }
    }
}

#[test]
fn row47_gjk_all_pairs_both_transforms() {
    for (i, &ta) in ALL_TYPES.iter().enumerate() {
        for (j, &tb) in ALL_TYPES.iter().enumerate() {
            gjk_pair_row("row47 GJK both xforms", ta, tb, SEED ^ 4700 ^ ((i * 3 + j) as u64), opts_both);
        }
    }
}

#[test]
fn row48_gjk_all_pairs_both_transforms_no_radius() {
    for (i, &ta) in ALL_TYPES.iter().enumerate() {
        for (j, &tb) in ALL_TYPES.iter().enumerate() {
            gjk_pair_row(
                "row48 GJK both xforms use_radius=0",
                ta,
                tb,
                SEED ^ 4800 ^ ((i * 3 + j) as u64),
                opts_both_no_radius,
            );
        }
    }
}

// ===========================================================================
// Rows 49-52: the c2GJKCache paths
// ===========================================================================

#[test]
fn row49_gjk_cold_cache() {
    let mut rng = Rng::new(SEED ^ 49);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..300 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                let o = GjkOpts {
                    cache: Some(c2GJKCache::default()), // count == 0 -> cold
                    ..Default::default()
                };
                gjk_same("row49 GJK cold cache", &a, &b, &o);
            }
        }
    }
}

/// Run a sequence of `c2GJK` calls threading one cache object through them, on
/// both libraries, comparing every intermediate result.
fn cache_sequence(row: &str, steps: &[(Shape, Shape, GjkOpts)]) {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut ccache = c2GJKCache::default();
    let mut rcache = c2GJKCache::default();
    for (i, (a, b, o)) in steps.iter().enumerate() {
        let mut oc = *o;
        oc.cache = Some(ccache);
        let mut or = *o;
        or.cache = Some(rcache);
        let (rc, rr) = unsafe { (call_gjk(*cf, a, b, &oc), call_gjk(*rf, a, b, &or)) };
        same(
            row,
            &format!("step {i}: A={} B={} [{}]", a.show(), b.show(), oc.show()),
            rc,
            rr,
        );
        ccache = rc.cache.unwrap();
        rcache = rr.cache.unwrap();
    }
}

#[test]
fn row50_gjk_warm_cache_same_shapes() {
    let mut rng = Rng::new(SEED ^ 50);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..120 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                let o = GjkOpts::default();
                cache_sequence(
                    "row50 GJK warm cache (unchanged shapes)",
                    &[(a, b, o), (a, b, o), (a, b, o)],
                );
            }
        }
    }
}

fn translate(s: &Shape, dx: f32, dy: f32) -> Shape {
    let mv = |v: c2v| c2v { x: v.x + dx, y: v.y + dy };
    match *s {
        Shape::Circle(c) => Shape::Circle(c2Circle { p: mv(c.p), r: c.r }),
        Shape::Aabb(a) => Shape::Aabb(c2AABB { min: mv(a.min), max: mv(a.max) }),
        Shape::Capsule(c) => Shape::Capsule(c2Capsule { a: mv(c.a), b: mv(c.b), r: c.r }),
    }
}

#[test]
fn row51_gjk_warm_cache_moved_shapes() {
    let mut rng = Rng::new(SEED ^ 51);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..120 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                let o = GjkOpts::default();
                let a2 = translate(&a, rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
                let b2 = translate(&b, rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
                cache_sequence(
                    "row51 GJK warm cache (moved shapes)",
                    &[(a, b, o), (a2, b2, o), (a, b2, o), (a2, b, o)],
                );
            }
        }
    }
}

#[test]
fn row52_gjk_cache_sweep() {
    let mut rng = Rng::new(SEED ^ 52);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..60 {
                let a = rand_shape(&mut rng, ta);
                let b0 = rand_shape(&mut rng, tb);
                let ax = rng.xform();
                let bx = rng.xform();
                let o = GjkOpts {
                    ax: Some(ax),
                    bx: Some(bx),
                    use_radius: 1,
                    ..Default::default()
                };
                // sweep B past A in small steps -> progressively warm cache
                let steps: Vec<(Shape, Shape, GjkOpts)> = (0..12)
                    .map(|k| (a, translate(&b0, -20.0 + k as f32 * 4.0, 0.0), o))
                    .collect();
                cache_sequence("row52 GJK cache sweep", &steps);
            }
        }
    }
}

// ===========================================================================
// Row 53: every NULL combination of the output parameters
// ===========================================================================

#[test]
fn row53_gjk_all_null_output_combinations() {
    let mut rng = Rng::new(SEED ^ 53);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..40 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                for mask in 0u32..8 {
                    for cache in [None, Some(c2GJKCache::default())] {
                        let o = GjkOpts {
                            want_a: mask & 1 != 0,
                            want_b: mask & 2 != 0,
                            want_iters: mask & 4 != 0,
                            cache,
                            ..Default::default()
                        };
                        gjk_same("row53 GJK NULL out combos", &a, &b, &o);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 54-58: special input shapes for c2GJK
// ===========================================================================

#[test]
fn row54_gjk_overlapping_hit_path() {
    let mut rng = Rng::new(SEED ^ 54);
    let mut n = 0;
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..400 {
                // Force deep overlap: both shapes built around the same centre.
                let cx = rng.range(-10.0, 10.0);
                let cy = rng.range(-10.0, 10.0);
                let big = |rng: &mut Rng, t: C2_TYPE| -> Shape {
                    match t {
                        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                            p: c2v { x: cx + rng.range(-1.0, 1.0), y: cy + rng.range(-1.0, 1.0) },
                            r: rng.range(5.0, 20.0),
                        }),
                        C2_TYPE_AABB => Shape::Aabb(c2AABB {
                            min: c2v { x: cx - rng.range(3.0, 15.0), y: cy - rng.range(3.0, 15.0) },
                            max: c2v { x: cx + rng.range(3.0, 15.0), y: cy + rng.range(3.0, 15.0) },
                        }),
                        _ => Shape::Capsule(c2Capsule {
                            a: c2v { x: cx - rng.range(0.0, 10.0), y: cy - rng.range(0.0, 10.0) },
                            b: c2v { x: cx + rng.range(0.0, 10.0), y: cy + rng.range(0.0, 10.0) },
                            r: rng.range(1.0, 8.0),
                        }),
                    }
                };
                let a = big(&mut rng, ta);
                let b = big(&mut rng, tb);
                for ur in [0, 1] {
                    let o = GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    };
                    gjk_same("row54 GJK overlapping (hit path)", &a, &b, &o);
                }
                n += 1;
            }
        }
    }
    assert!(n > 0);
}

#[test]
fn row55_gjk_identical_shapes() {
    let mut rng = Rng::new(SEED ^ 55);
    for &t in ALL_TYPES.iter() {
        for _ in 0..600 {
            let a = rand_shape(&mut rng, t);
            for ur in [0, 1] {
                for cache in [None, Some(c2GJKCache::default())] {
                    let o = GjkOpts { use_radius: ur, cache, ..Default::default() };
                    gjk_same("row55 GJK identical shapes", &a, &a, &o);
                }
            }
        }
    }
}

#[test]
fn row56_gjk_zero_extent_shapes() {
    let pt = |x: f32, y: f32| Shape::Circle(c2Circle { p: c2v { x, y }, r: 0.0 });
    let zbox = |x: f32, y: f32| Shape::Aabb(c2AABB { min: c2v { x, y }, max: c2v { x, y } });
    let zcap = |x: f32, y: f32| {
        Shape::Capsule(c2Capsule { a: c2v { x, y }, b: c2v { x, y }, r: 0.0 })
    };
    let mut rng = Rng::new(SEED ^ 56);
    for _ in 0..800 {
        let (x1, y1) = (rng.coord(), rng.coord());
        let (x2, y2) = (rng.coord(), rng.coord());
        let group_a = [pt(x1, y1), zbox(x1, y1), zcap(x1, y1)];
        let group_b = [pt(x2, y2), zbox(x2, y2), zcap(x2, y2)];
        for a in group_a.iter() {
            for b in group_b.iter() {
                for ur in [0, 1] {
                    let o = GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    };
                    gjk_same("row56 GJK zero-extent shapes", a, b, &o);
                }
            }
        }
    }
}

#[test]
fn row57_gjk_degenerate_capsule() {
    let mut rng = Rng::new(SEED ^ 57);
    for &tb in ALL_TYPES.iter() {
        for _ in 0..800 {
            let p = rng.vec();
            let a = Shape::Capsule(c2Capsule { a: p, b: p, r: rng.radius() });
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                let o = GjkOpts { use_radius: ur, ..Default::default() };
                gjk_same("row57 GJK degenerate capsule (A)", &a, &b, &o);
                gjk_same("row57 GJK degenerate capsule (B)", &b, &a, &o);
            }
        }
    }
}

#[test]
fn row58_gjk_large_magnitudes() {
    let mut rng = Rng::new(SEED ^ 58);
    let scales = [1e3f32, 1e6, 1e12, 1e18, 1e30];
    for &s in scales.iter() {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                for _ in 0..80 {
                    let big = |rng: &mut Rng, t: C2_TYPE| -> Shape {
                        let x = rng.range(-1.0, 1.0) * s;
                        let y = rng.range(-1.0, 1.0) * s;
                        match t {
                            C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                                p: c2v { x, y },
                                r: rng.range(0.0, 1.0) * s,
                            }),
                            C2_TYPE_AABB => Shape::Aabb(c2AABB {
                                min: c2v { x, y },
                                max: c2v {
                                    x: x + rng.range(0.0, 1.0) * s,
                                    y: y + rng.range(0.0, 1.0) * s,
                                },
                            }),
                            _ => Shape::Capsule(c2Capsule {
                                a: c2v { x, y },
                                b: c2v {
                                    x: x + rng.range(-1.0, 1.0) * s,
                                    y: y + rng.range(-1.0, 1.0) * s,
                                },
                                r: rng.range(0.0, 1.0) * s,
                            }),
                        }
                    };
                    let a = big(&mut rng, ta);
                    let b = big(&mut rng, tb);
                    for ur in [0, 1] {
                        let o = GjkOpts {
                            use_radius: ur,
                            cache: Some(c2GJKCache::default()),
                            ..Default::default()
                        };
                        gjk_same("row58 GJK large magnitudes", &a, &b, &o);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 59-64: the pairwise boolean collision routines
// ===========================================================================

#[test]
fn row59_aabb_to_aabb() {
    let (c, r) = sym::<FnAABBtoAABB>("c2AABBtoAABB");
    let mut rng = Rng::new(SEED ^ 59);
    for _ in 0..N * 2 {
        let a = rng.aabb();
        let b = rng.aabb();
        unsafe {
            same(
                "row59 c2AABBtoAABB",
                &format!("{a:?} {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
    // separation on each of the four axes + exact touching + containment
    let unit = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let others = [
        c2AABB { min: c2v { x: 2.0, y: 0.0 }, max: c2v { x: 3.0, y: 1.0 } }, // right
        c2AABB { min: c2v { x: -3.0, y: 0.0 }, max: c2v { x: -2.0, y: 1.0 } }, // left
        c2AABB { min: c2v { x: 0.0, y: 2.0 }, max: c2v { x: 1.0, y: 3.0 } }, // above
        c2AABB { min: c2v { x: 0.0, y: -3.0 }, max: c2v { x: 1.0, y: -2.0 } }, // below
        c2AABB { min: c2v { x: 1.0, y: 0.0 }, max: c2v { x: 2.0, y: 1.0 } }, // touching
        c2AABB { min: c2v { x: 0.25, y: 0.25 }, max: c2v { x: 0.75, y: 0.75 } }, // contained
        unit,                                                                // identical
    ];
    for o in others {
        unsafe {
            same("row59 fixed", &format!("{unit:?} {o:?}"), c(unit, o), r(unit, o));
            same("row59 fixed rev", &format!("{o:?} {unit:?}"), c(o, unit), r(o, unit));
        }
    }
}

#[test]
fn row60_aabb_to_capsule() {
    let (c, r) = sym::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let mut rng = Rng::new(SEED ^ 60);
    for _ in 0..N * 2 {
        let a = rng.aabb();
        let b = rng.capsule();
        unsafe {
            same(
                "row60 c2AABBtoCapsule",
                &format!("{a:?} {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
}

#[test]
fn row61_capsule_to_capsule() {
    let (c, r) = sym::<FnCapsuletoCapsule>("c2CapsuletoCapsule");
    let mut rng = Rng::new(SEED ^ 61);
    for _ in 0..N * 2 {
        let a = rng.capsule();
        let mut b = rng.capsule();
        match rng.below(6) {
            0 => {
                // parallel
                b.a = c2v { x: a.a.x, y: a.a.y + rng.range(-5.0, 5.0) };
                b.b = c2v { x: a.b.x, y: a.b.y + rng.range(-5.0, 5.0) };
            }
            1 => {
                // collinear overlapping
                b.a = a.a;
                b.b = a.b;
            }
            _ => {}
        }
        unsafe {
            same(
                "row61 c2CapsuletoCapsule",
                &format!("{a:?} {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
}

#[test]
fn row62_circle_to_circle() {
    let (c, r) = sym::<FnCircletoCircle>("c2CircletoCircle");
    let mut rng = Rng::new(SEED ^ 62);
    for _ in 0..N * 2 {
        let a = rng.circle();
        let b = rng.circle();
        unsafe {
            same(
                "row62 c2CircletoCircle",
                &format!("{a:?} {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
    let fixed = [
        ((0.0f32, 0.0f32, 1.0f32), (2.0f32, 0.0f32, 1.0f32)), // exactly touching -> false
        ((0.0, 0.0, 1.0), (1.9999, 0.0, 1.0)),
        ((0.0, 0.0, 1.0), (0.0, 0.0, 1.0)),   // concentric
        ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0)),   // both zero radius, same point
        ((0.0, 0.0, 5.0), (1.0, 0.0, 0.0)),   // zero-radius point inside
    ];
    for (a, b) in fixed {
        let ca = c2Circle { p: c2v { x: a.0, y: a.1 }, r: a.2 };
        let cb = c2Circle { p: c2v { x: b.0, y: b.1 }, r: b.2 };
        unsafe { same("row62 fixed", &format!("{ca:?} {cb:?}"), c(ca, cb), r(ca, cb)) }
    }
}

#[test]
fn row63_circle_to_aabb() {
    let (c, r) = sym::<FnCircletoAABB>("c2CircletoAABB");
    let mut rng = Rng::new(SEED ^ 63);
    for _ in 0..N * 2 {
        let a = rng.circle();
        let b = rng.aabb();
        unsafe {
            same(
                "row63 c2CircletoAABB",
                &format!("{a:?} {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
    // nearest point on a face, on a corner, centre inside, exactly touching
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for &(x, y, rad) in &[
        (0.0f32, 0.0f32, 0.5f32),
        (2.0, 0.0, 1.0),
        (2.0, 0.0, 1.0001),
        (2.0, 2.0, 1.4142135),
        (2.0, 2.0, 1.4142137),
        (0.0, 3.0, 2.0),
        (0.0, 0.0, 0.0),
    ] {
        let ca = c2Circle { p: c2v { x, y }, r: rad };
        unsafe { same("row63 fixed", &format!("{ca:?}"), c(ca, bb), r(ca, bb)) }
    }
}

#[test]
fn row64_circle_to_capsule() {
    let (c, r) = sym::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(SEED ^ 64);
    let mut arms = [0usize; 3];
    for _ in 0..N * 3 {
        let a = rng.circle();
        let b = rng.capsule();
        // classify which arm of the C code this input takes
        let n = c2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
        let ap = c2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            arms[0] += 1;
        } else {
            let bp = c2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
            let db = bp.x * n.x + bp.y * n.y;
            if db < 0.0 {
                arms[1] += 1;
            } else {
                arms[2] += 1;
            }
        }
        unsafe {
            same(
                "row64 c2CircletoCapsule",
                &format!("{a:?} {b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
    assert!(
        arms.iter().all(|&h| h > 50),
        "row64: arm coverage too thin: {arms:?}"
    );
}

// ===========================================================================
// Rows 65-66: c2Collided dispatcher
// ===========================================================================

#[test]
fn row65_collided_all_valid_pairs() {
    let (c, r) = sym::<FnCollided>("c2Collided");
    let mut rng = Rng::new(SEED ^ 65);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..1500 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                unsafe {
                    same(
                        "row65 c2Collided",
                        &format!("A={} ({}) B={} ({})", a.show(), ta, b.show(), tb),
                        c(a.as_ptr(), ta, b.as_ptr(), tb),
                        r(a.as_ptr(), ta, b.as_ptr(), tb),
                    )
                }
            }
        }
    }
}

#[test]
fn row66_collided_swapping_arms() {
    // AABBxCIRCLE, CAPSULExCIRCLE, CAPSULExAABB swap their arguments inside the
    // C dispatcher; verify the swap is reproduced (asymmetric shapes needed).
    let (c, r) = sym::<FnCollided>("c2Collided");
    let mut rng = Rng::new(SEED ^ 66);
    let combos: [(C2_TYPE, C2_TYPE); 3] = [
        (C2_TYPE_AABB, C2_TYPE_CIRCLE),
        (C2_TYPE_CAPSULE, C2_TYPE_CIRCLE),
        (C2_TYPE_CAPSULE, C2_TYPE_AABB),
    ];
    for (ta, tb) in combos {
        for _ in 0..N {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            unsafe {
                same(
                    "row66 c2Collided swap arm",
                    &format!("A={} ({}) B={} ({})", a.show(), ta, b.show(), tb),
                    c(a.as_ptr(), ta, b.as_ptr(), tb),
                    r(a.as_ptr(), ta, b.as_ptr(), tb),
                );
                // and the mirrored call
                same(
                    "row66 c2Collided swap arm mirrored",
                    &format!("A={} ({}) B={} ({})", b.show(), tb, a.show(), ta),
                    c(b.as_ptr(), tb, a.as_ptr(), ta),
                    r(b.as_ptr(), tb, a.as_ptr(), ta),
                );
            }
        }
    }
}

// ===========================================================================
// Rows 67-69: reverse_collide (the documented entry point)
// ===========================================================================

#[test]
fn row67_reverse_collide_grid() {
    let (c, r) = sym::<FnReverseCollide>("reverse_collide");
    let mut seen = [false; 8];
    let mut n = 0u64;
    let mut y = -160.0f32;
    while y <= 160.0 {
        let mut x = -160.0f32;
        while x <= 160.0 {
            for &rad in &[0.0f32, 1.0, 5.0, 10.0, 20.0, 60.0] {
                let (cv, rv) = unsafe { (c(x, y, rad), r(x, y, rad)) };
                same("row67 reverse_collide grid", &format!("{x} {y} {rad}"), cv, rv);
                if (0..8).contains(&cv) {
                    seen[cv as usize] = true;
                }
                n += 1;
            }
            x += 2.5;
        }
        y += 2.5;
    }
    assert!(n > 90_000, "row67: only {n} samples");
    let hit: Vec<usize> = (0..8).filter(|&i| seen[i]).collect();
    assert!(
        hit.len() >= 7,
        "row67: expected nearly every result bit pattern, saw {hit:?}"
    );
}

#[test]
fn row68_reverse_collide_random() {
    let (c, r) = sym::<FnReverseCollide>("reverse_collide");
    let mut rng = Rng::new(SEED ^ 68);
    for _ in 0..N * 20 {
        let x = rng.wild();
        let y = rng.wild();
        let rad = rng.wild();
        unsafe {
            same(
                "row68 reverse_collide random",
                &format!("{x:?} {y:?} {rad:?}"),
                c(x, y, rad),
                r(x, y, rad),
            )
        }
    }
}

#[test]
fn row69_reverse_collide_tangent_boundaries() {
    let (c, r) = sym::<FnReverseCollide>("reverse_collide");
    // The three fixed shapes in reverse_collide():
    //   circle  p=(-70,0) r=20
    //   aabb    min=(-40,-40) max=(-15,-15)
    //   capsule a=(-40,40) b=(-20,100) r=10
    let mut cases: Vec<(f32, f32, f32)> = Vec::new();
    // exactly tangent to the fixed circle along +x
    for &d in &[0.0f32, 1e-6, -1e-6, 1e-3, -1e-3] {
        cases.push((-40.0 + d, 0.0, 10.0)); // |(-70)-(-40)| = 30 = 20+10
        cases.push((0.0, 0.0, 70.0 - 20.0 + d));
    }
    // exactly on the AABB faces / corners
    for &(x, y) in &[
        (-40.0f32, -40.0f32),
        (-15.0, -15.0),
        (-40.0, -15.0),
        (-15.0, -40.0),
        (-27.5, -27.5),
    ] {
        for &rad in &[0.0f32, f32::EPSILON, 1e-6, 0.5] {
            cases.push((x, y, rad));
            cases.push((x - 1.0, y, 1.0));
            cases.push((x, y - 1.0, 1.0));
        }
    }
    // exactly tangent to the capsule's two endpoints and its side
    for &(x, y) in &[(-40.0f32, 40.0f32), (-20.0, 100.0), (-30.0, 70.0)] {
        for &rad in &[0.0f32, 10.0, 10.000001, 9.999999] {
            cases.push((x, y, rad));
            cases.push((x + 10.0, y, rad));
            cases.push((x, y + 10.0, rad));
        }
    }
    // the exact tangency distance for the capsule endpoint (dist == r + 10)
    cases.push((-40.0, 30.0, 0.0));
    cases.push((-40.0, 30.0, 1e-7));
    for (x, y, rad) in cases {
        unsafe {
            same(
                "row69 reverse_collide tangent",
                &format!("{x} {y} {rad}"),
                c(x, y, rad),
                r(x, y, rad),
            )
        }
    }
}

// ===========================================================================
// Sanity: struct sizes must agree with the C ABI (checked via c2MakeProxy,
// which writes through a caller-allocated c2Proxy).
// ===========================================================================

#[test]
fn abi_struct_sizes() {
    assert_eq!(std::mem::size_of::<c2v>(), 8);
    assert_eq!(std::mem::size_of::<c2r>(), 8);
    assert_eq!(std::mem::size_of::<c2x>(), 16);
    assert_eq!(std::mem::size_of::<c2Circle>(), 12);
    assert_eq!(std::mem::size_of::<c2AABB>(), 16);
    assert_eq!(std::mem::size_of::<c2Capsule>(), 20);
    assert_eq!(std::mem::size_of::<c2GJKCache>(), 36);
    assert_eq!(std::mem::size_of::<c2Proxy>(), 72);
    assert_eq!(std::mem::size_of::<c2sv>(), 36);
    assert_eq!(std::mem::size_of::<c2Simplex>(), 152);
    // silence the unused import warning when the c_void alias is not otherwise
    // referenced in this file
    let _: *const c_void = std::ptr::null();
}
