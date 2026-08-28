//! Phase B, Group 3 — CONFIGS.md rows C21..C45.
//!
//! These are the LOW-LEVEL entry points `c2GJK` composes.  They are exported,
//! so an external caller can drive them directly — and a bug here is invisible
//! to a test that only calls the `capsule()` convenience wrapper.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 6000;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Runs a `void f(c2Simplex*)` in both libraries and compares the WHOLE
/// 152-byte simplex afterwards (not just the fields the arm happens to set).
fn diff_void(p: &Pair, name: &str, s: c2Simplex, ctx: &str) {
    let c: FnSimplexVoid = p.c.sym(name);
    let r: FnSimplexVoid = p.rs.sym(name);
    let mut cs = s;
    let mut rs_ = s;
    unsafe {
        c(&mut cs);
        r(&mut rs_);
    }
    if raw(&cs) != raw(&rs_) {
        panic!(
            "DIVERGENCE {name} {ctx}\n  input : {}\n  C     : {}\n  Rust  : {}",
            simplex_hex(&s),
            simplex_hex(&cs),
            simplex_hex(&rs_)
        );
    }
}

/// Runs a `float f(c2Simplex*)` in both libraries; compares result AND the
/// simplex (proving neither side mutates it).
fn diff_f32(p: &Pair, name: &str, s: c2Simplex, ctx: &str) {
    let c: FnSimplexf = p.c.sym(name);
    let r: FnSimplexf = p.rs.sym(name);
    let mut cs = s;
    let mut rs_ = s;
    let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs_)) };
    if cv.to_bits() != rv.to_bits() || raw(&cs) != raw(&rs_) {
        panic!(
            "DIVERGENCE {name} {ctx}\n  input : {}\n  C  = {} / {}\n  Rust= {} / {}",
            simplex_hex(&s),
            f32_hex(cv),
            simplex_hex(&cs),
            f32_hex(rv),
            simplex_hex(&rs_)
        );
    }
}

/// Runs a `c2v f(c2Simplex*)` in both libraries.
fn diff_v(p: &Pair, name: &str, s: c2Simplex, ctx: &str) {
    let c: FnSimplexv = p.c.sym(name);
    let r: FnSimplexv = p.rs.sym(name);
    let mut cs = s;
    let mut rs_ = s;
    let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs_)) };
    if raw(&cv) != raw(&rv) || raw(&cs) != raw(&rs_) {
        panic!(
            "DIVERGENCE {name} {ctx}\n  input : {}\n  C  = {} / {}\n  Rust= {} / {}",
            simplex_hex(&s),
            v_hex(&cv),
            simplex_hex(&cs),
            v_hex(&rv),
            simplex_hex(&rs_)
        );
    }
}

/// A simplex whose `p` fields are the given points; everything else randomised.
fn simplex_with_points(rng: &mut Rng, pts: &[c2v], div: f32) -> c2Simplex {
    let mut s = rng.simplex(pts.len() as c_int);
    for (i, q) in pts.iter().enumerate() {
        s.verts[i].p = *q;
    }
    s.div = div;
    s
}

// ---------------------------------------------------------------------------
// C21..C23 — c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn c21_c22_c23_metric() {
    let p = load();
    let mut rng = Rng::new(0x21);
    for count in [1, 2, 3] {
        for _ in 0..N {
            let s = rng.simplex(count);
            diff_f32(&p, "c2GJKSimplexMetric", s, &format!("count={count}"));
        }
    }
    // C22: equal points => metric 0 ; C23: collinear => det == +-0
    for _ in 0..N {
        let a = rng.v();
        let s2 = simplex_with_points(&mut rng, &[a, a], 1.0);
        diff_f32(&p, "c2GJKSimplexMetric", s2, "count=2 a.p==b.p");
        let d = rng.v();
        let k = rng.coord();
        let col = c2v {
            x: a.x + d.x * k,
            y: a.y + d.y * k,
        };
        let s3 = simplex_with_points(
            &mut rng,
            &[a, c2v { x: a.x + d.x, y: a.y + d.y }, col],
            1.0,
        );
        diff_f32(&p, "c2GJKSimplexMetric", s3, "count=3 collinear");
    }
    // wild points
    for count in [1, 2, 3] {
        for _ in 0..N {
            let pts: Vec<c2v> = (0..count).map(|_| rng.v_wild()).collect();
            let dv = rng.wild();
            let s = simplex_with_points(&mut rng, &pts, dv);
            diff_f32(&p, "c2GJKSimplexMetric", s, "wild");
        }
    }
}

// ---------------------------------------------------------------------------
// C24..C26 — c22   (all three arms)
// ---------------------------------------------------------------------------

#[test]
fn c24_c25_c26_c22() {
    let p = load();
    let cf: FnSimplexVoid = p.c.sym("c22");
    let mut rng = Rng::new(0x22);
    // Which arm fired?  Determined from the resulting count / which vertex won.
    let mut arms = [0usize; 3];
    let classify = |s: c2Simplex| -> usize {
        let mut cs = s;
        unsafe { cf(&mut cs) };
        if cs.count == 2 {
            2
        } else if raw(&cs.verts[0].p) == raw(&s.verts[1].p) && raw(&s.verts[0].p) != raw(&s.verts[1].p) {
            1
        } else {
            0
        }
    };

    for _ in 0..(N * 3) {
        let s = rng.simplex(2);
        arms[classify(s)] += 1;
        diff_void(&p, "c22", s, "random count=2");
    }
    // C25: a.p == b.p  (u == v == 0 -> first arm)
    for _ in 0..N {
        let a = rng.v();
        let s = simplex_with_points(&mut rng, &[a, a], 1.0);
        diff_void(&p, "c22", s, "a.p == b.p");
    }
    // C26: origin strictly inside segment ab -> else arm, count stays 2
    for _ in 0..N {
        let d = c2v {
            x: rng.uniform(0.5, 20.0),
            y: rng.uniform(0.5, 20.0),
        };
        let t = rng.uniform(0.2, 0.8);
        let a = c2v { x: -d.x * t, y: -d.y * t };
        let b = c2v {
            x: d.x * (1.0 - t),
            y: d.y * (1.0 - t),
        };
        let s = simplex_with_points(&mut rng, &[a, b], 1.0);
        assert_eq!(classify(s), 2, "expected the else arm for origin-inside-ab");
        diff_void(&p, "c22", s, "origin inside ab");
    }
    // wild inputs (NaN u/v -> else arm)
    for _ in 0..N {
        let (wa, wb, dv) = (rng.v_wild(), rng.v_wild(), rng.wild());
        let s = simplex_with_points(&mut rng, &[wa, wb], dv);
        diff_void(&p, "c22", s, "wild");
    }
    // c22 called with an out-of-range count still only looks at a/b
    for count in [0, 1, 3, 4, -1, i32::MIN, i32::MAX] {
        for _ in 0..64 {
            let mut s = rng.simplex(2);
            s.count = count;
            diff_void(&p, "c22", s, &format!("count={count}"));
        }
    }
    assert!(
        arms.iter().all(|&n| n > 0),
        "c22 arm coverage incomplete: {arms:?}"
    );
    eprintln!("c22 arm hit counts (v<=0, u<=0, else) = {arms:?}");
}

// ---------------------------------------------------------------------------
// C27..C30 — c23   (all seven arms)
// ---------------------------------------------------------------------------

/// Recomputes the C's arm selection in f64-free, bit-faithful f32 so the test
/// can assert full arm coverage.  (Only used for *coverage accounting*; the
/// pass/fail decision is always the byte comparison of the two `.so`s.)
fn c23_arm(s: &c2Simplex) -> usize {
    #[inline]
    fn dot(a: c2v, b: c2v) -> f32 {
        a.y * b.y + a.x * b.x
    }
    #[inline]
    fn sub(a: c2v, b: c2v) -> c2v {
        c2v { x: a.x - b.x, y: a.y - b.y }
    }
    #[inline]
    fn det(a: c2v, b: c2v) -> f32 {
        b.y * a.x - b.x * a.y
    }
    let (a, b, c) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
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
fn c27_c28_c29_c30_c23() {
    let p = load();
    let mut rng = Rng::new(0x23);
    let mut arms = [0usize; 7];

    for _ in 0..(N * 6) {
        let s = rng.simplex(3);
        arms[c23_arm(&s)] += 1;
        diff_void(&p, "c23", s, "random count=3");
    }
    // C28: origin strictly inside the triangle -> the `else` arm (count = 3)
    for _ in 0..N {
        let r0 = rng.uniform(2.0, 30.0);
        let base = rng.uniform(0.0, 6.28);
        let pts: Vec<c2v> = (0..3)
            .map(|k| {
                let ang = base + (k as f32) * 2.0943951;
                c2v {
                    x: r0 * ang.cos(),
                    y: r0 * ang.sin(),
                }
            })
            .collect();
        let s = simplex_with_points(&mut rng, &pts, 1.0);
        arms[c23_arm(&s)] += 1;
        diff_void(&p, "c23", s, "origin inside triangle");
    }
    // C29: degenerate (collinear) triangle -> area == +-0
    for _ in 0..N {
        let a = rng.v();
        let d = c2v { x: rng.coord(), y: rng.coord() };
        let mk = |t: f32| c2v { x: a.x + d.x * t, y: a.y + d.y * t };
        let t3 = rng.coord();
        let s = simplex_with_points(&mut rng, &[a, mk(1.0), mk(t3)], 1.0);
        arms[c23_arm(&s)] += 1;
        diff_void(&p, "c23", s, "collinear");
    }
    // C30: all three points identical
    for _ in 0..N {
        let a = rng.v();
        let s = simplex_with_points(&mut rng, &[a, a, a], 1.0);
        arms[c23_arm(&s)] += 1;
        diff_void(&p, "c23", s, "a==b==c");
    }
    // wild points (all-NaN -> else arm)
    for _ in 0..N {
        let (wa, wb, wc, dv) = (rng.v_wild(), rng.v_wild(), rng.v_wild(), rng.wild());
        let s = simplex_with_points(&mut rng, &[wa, wb, wc], dv);
        arms[c23_arm(&s)] += 1;
        diff_void(&p, "c23", s, "wild");
    }
    // out-of-range count
    for count in [0, 1, 2, 4, -1, i32::MIN, i32::MAX] {
        for _ in 0..64 {
            let mut s = rng.simplex(3);
            s.count = count;
            diff_void(&p, "c23", s, &format!("count={count}"));
        }
    }
    assert!(
        arms.iter().all(|&n| n > 0),
        "c23 arm coverage incomplete: {arms:?}"
    );
    eprintln!("c23 arm hit counts = {arms:?}");
}

// ---------------------------------------------------------------------------
// C31..C33 — c2D
// ---------------------------------------------------------------------------

#[test]
fn c31_c32_c33_c2D() {
    let p = load();
    let mut rng = Rng::new(0x2d);
    for count in [1, 2, 3] {
        for _ in 0..(N * 2) {
            let s = rng.simplex(count);
            diff_v(&p, "c2D", s, &format!("count={count}"));
        }
    }
    // C32: force det > 0 and det <= 0 and det == 0
    for _ in 0..N {
        let a = rng.v();
        let b = rng.v();
        diff_v(&p, "c2D", simplex_with_points(&mut rng, &[a, b], 1.0), "count=2 ab");
        diff_v(&p, "c2D", simplex_with_points(&mut rng, &[b, a], 1.0), "count=2 ba");
        // collinear with the origin => det == +-0
        let k = rng.coord();
        let col = c2v { x: a.x * k, y: a.y * k };
        diff_v(&p, "c2D", simplex_with_points(&mut rng, &[a, col], 1.0), "count=2 det==0");
    }
    for _ in 0..N {
        for count in [1, 2, 3] {
            let pts: Vec<c2v> = (0..count).map(|_| rng.v_wild()).collect();
            let dv = rng.wild();
            let s = simplex_with_points(&mut rng, &pts, dv);
            diff_v(&p, "c2D", s, "wild");
        }
    }
}

// ---------------------------------------------------------------------------
// C34..C38 — c2Support
// ---------------------------------------------------------------------------

#[test]
fn c34_to_c38_c2Support() {
    let p = load();
    let c: FnSupport = p.c.sym("c2Support");
    let r: FnSupport = p.rs.sym("c2Support");
    let mut rng = Rng::new(0x34);
    unsafe {
        for count in [1i32, 2, 4, 8] {
            for _ in 0..N {
                let mut verts = [c2v::default(); 8];
                for i in 0..8 {
                    verts[i] = if rng.below(6) == 0 { rng.v_wild() } else { rng.v() };
                }
                let d = if rng.below(6) == 0 { rng.v_wild() } else { rng.v() };
                let cv = c(verts.as_ptr(), count, d);
                let rv = r(verts.as_ptr(), count, d);
                assert_eq!(
                    cv, rv,
                    "c2Support(count={count}, d={}) C={cv} Rust={rv}",
                    v_hex(&d)
                );
            }
        }
        // C36: axis-aligned AABB corners with axis-aligned d -> exact ties
        let bb = [
            c2v { x: -1.0, y: -1.0 },
            c2v { x: 1.0, y: -1.0 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: -1.0, y: 1.0 },
        ];
        for d in [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: f32::NAN, y: 0.0 },
        ] {
            let cv = c(bb.as_ptr(), 4, d);
            let rv = r(bb.as_ptr(), 4, d);
            assert_eq!(cv, rv, "c2Support AABB tie d={}", v_hex(&d));
        }
        // C38: all vertices equal -> index 0 must win in both
        for _ in 0..N {
            let a = rng.v();
            let verts = [a; 8];
            for count in [1, 2, 4, 8] {
                let d = rng.v();
                let cv = c(verts.as_ptr(), count, d);
                let rv = r(verts.as_ptr(), count, d);
                assert_eq!(cv, rv, "c2Support all-equal count={count}");
            }
        }
        // full special sweep on d with a fixed 8-vertex ring
        let mut ring = [c2v::default(); 8];
        for i in 0..8 {
            let a = (i as f32) * 0.7853982;
            ring[i] = c2v { x: a.cos() * 10.0, y: a.sin() * 10.0 };
        }
        for dx in specials() {
            for dy in specials() {
                let d = c2v { x: dx, y: dy };
                for count in [1, 2, 4, 8] {
                    let cv = c(ring.as_ptr(), count, d);
                    let rv = r(ring.as_ptr(), count, d);
                    assert_eq!(cv, rv, "c2Support special d={} count={count}", v_hex(&d));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C39..C42 — c2Witness
// ---------------------------------------------------------------------------

#[test]
fn c39_to_c42_c2Witness() {
    let p = load();
    let c: FnWitness = p.c.sym("c2Witness");
    let r: FnWitness = p.rs.sym("c2Witness");
    let mut rng = Rng::new(0x39);
    unsafe {
        let run = |s: c2Simplex, ctx: &str| {
            let mut cs = s;
            let mut rs_ = s;
            // sentinel-filled outputs so a missing store is caught
            let mut ca = c2v { x: 12.5, y: -3.25 };
            let mut cb = c2v { x: -8.0, y: 6.0 };
            let mut ra = ca;
            let mut rb = cb;
            c(&mut cs, &mut ca, &mut cb);
            r(&mut rs_, &mut ra, &mut rb);
            if raw(&ca) != raw(&ra) || raw(&cb) != raw(&rb) || raw(&cs) != raw(&rs_) {
                panic!(
                    "DIVERGENCE c2Witness {ctx}\n  input: {}\n  C   a={} b={}\n  Rust a={} b={}",
                    simplex_hex(&s),
                    v_hex(&ca),
                    v_hex(&cb),
                    v_hex(&ra),
                    v_hex(&rb)
                );
            }
        };
        for count in [1i32, 2, 3] {
            for _ in 0..(N * 2) {
                run(rng.simplex(count), &format!("count={count}"));
            }
        }
        // C42: div == +-0 / NaN / inf at each count
        for count in [0i32, 1, 2, 3, 4, -1, i32::MIN, i32::MAX] {
            for divb in specials() {
                let mut s = rng.simplex(if count.clamp(0, 3) == 0 { 1 } else { count.clamp(1, 3) });
                s.count = count;
                s.div = divb;
                run(s, &format!("count={count} div={}", f32_hex(divb)));
            }
        }
        // wild u / sA / sB
        for count in [1i32, 2, 3] {
            for _ in 0..N {
                let mut s = rng.simplex(count);
                for i in 0..4 {
                    s.verts[i].sA = rng.v_wild();
                    s.verts[i].sB = rng.v_wild();
                    s.verts[i].u = rng.wild();
                }
                s.div = rng.wild();
                run(s, "wild");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C43..C45 — c2L
// ---------------------------------------------------------------------------

#[test]
fn c43_c44_c45_c2L() {
    let p = load();
    let mut rng = Rng::new(0x4c);
    for count in [0i32, 1, 2, 3, 4, -1, i32::MIN, i32::MAX] {
        for _ in 0..N {
            let mut s = rng.simplex(count.clamp(1, 3).max(1));
            s.count = count;
            diff_v(&p, "c2L", s, &format!("count={count}"));
        }
        for divb in specials() {
            let mut s = rng.simplex(1);
            s.count = count;
            s.div = divb;
            diff_v(&p, "c2L", s, &format!("count={count} div={}", f32_hex(divb)));
        }
    }
    // wild p / u
    for count in [1i32, 2, 3] {
        for _ in 0..N {
            let mut s = rng.simplex(count);
            for i in 0..4 {
                s.verts[i].p = rng.v_wild();
                s.verts[i].u = rng.wild();
            }
            s.div = rng.wild();
            diff_v(&p, "c2L", s, "wild");
        }
    }
}
