//! Phase B — CONFIGS.md rows 21-49: the simplex primitives.
//!
//! These are the lowest-level entry points that take a `c2Simplex*`. They are
//! driven directly with hand-forged simplex structs (not only via `c2GJK`), so
//! every `count` value and every branch of `c22` / `c23` is reached on purpose,
//! including the `default:` arms that `c2GJK` alone can never produce.
//!
//! `c22` / `c23` mutate the simplex in place, so each test compares the whole
//! 152-byte struct afterwards, not just the return value.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 20_000;

/// Every `count` value worth probing, including the out-of-range ones that only
/// a direct call can produce.
const COUNTS: [i32; 9] = [0, 1, 2, 3, 4, 5, -1, -1000, 999];

fn forge(g: &mut Rng, count: i32) -> c2Simplex {
    c2Simplex {
        verts: [
            g.simplex_vert(),
            g.simplex_vert(),
            g.simplex_vert(),
            g.simplex_vert(),
        ],
        div: match g.below(6) {
            0 => 0.0,
            1 => 1.0,
            2 => g.wild(),
            _ => g.coord(),
        },
        count,
    }
}

/// Build a simplex whose `p` values are exactly the three given points.
fn simplex_from_points(pts: &[c2v], div: f32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for (i, p) in pts.iter().enumerate() {
        s.verts[i].p = *p;
        // distinct sA/sB so the witness maths is observable
        s.verts[i].sA = c2v {
            x: p.x * 0.5,
            y: p.y - 1.0,
        };
        s.verts[i].sB = c2v {
            x: p.x + 2.0,
            y: p.y * 0.25,
        };
        s.verts[i].u = 1.0;
        s.verts[i].iA = i as i32;
        s.verts[i].iB = (i as i32 + 1) % 4;
    }
    s.div = div;
    s.count = pts.len() as i32;
    s
}

// ---------------------------------------------------------------------------
// Rows 21-24 — c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn row21_24_gjk_simplex_metric() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 21);
    for &count in &COUNTS {
        for i in 0..N / 3 {
            let s = forge(&mut g, count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
            let rv = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
            eq_f32(&format!("row21-24 metric count={count} #{i}"), cv, rv);
            // Must not mutate the simplex.
            eq_simplex(&format!("row21-24 unchanged count={count} #{i}"), &cs, &rs);
            eq_bytes(&format!("row21-24 input intact count={count} #{i}"), &cs, &s);
        }
    }
    // Row 21/24: the `default:` and `case 1:` arms both return exactly 0.
    for &count in &[0i32, 1, 4, 5, -1, 999] {
        let mut s = forge(&mut g, count);
        let mut s2 = s;
        eq_f32_bits(
            &format!("row21/24 zero for count={count}"),
            unsafe { (c.c2GJKSimplexMetric)(&mut s) },
            unsafe { (r.c2GJKSimplexMetric)(&mut s2) },
        );
        assert_eq!(
            unsafe { (c.c2GJKSimplexMetric)(&mut s) }.to_bits(),
            0f32.to_bits(),
            "C metric for count={count} should be +0.0"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 25-29 — c22 (all three branches)
// ---------------------------------------------------------------------------

fn diff_c22(c: &Api, r: &Api, ctx: &str, s: c2Simplex) {
    let mut cs = s;
    let mut rs = s;
    unsafe { (c.c22)(&mut cs) };
    unsafe { (r.c22)(&mut rs) };
    eq_simplex(ctx, &cs, &rs);
}

/// Row 25 — branch `v <= 0`: the origin is beyond `a`, keep vertex A.
#[test]
fn row25_c22_branch_v_le_0() {
    let (c, r) = apis();
    // a and b on the same ray from the origin with |a| < |b| makes
    // v = dot(a, a-b) <= 0.
    for k in 1..200i32 {
        let t = k as f32 * 0.37;
        let s = simplex_from_points(
            &[
                c2v { x: t, y: t },
                c2v {
                    x: t * 3.0,
                    y: t * 3.0,
                },
            ],
            1.0,
        );
        diff_c22(c, r, &format!("row25 k={k}"), s);
        let mut chk = s;
        unsafe { (c.c22)(&mut chk) };
        assert_eq!(chk.count, 1, "row25 k={k}: expected the count=1 branch");
    }
    // Degenerate: a == b makes u == v == 0, so the first branch wins.
    let p = c2v { x: 2.0, y: -3.0 };
    let s = simplex_from_points(&[p, p], 1.0);
    diff_c22(c, r, "row25 a==b", s);
}

/// Row 26 — branch `u <= 0`: the origin is beyond `b`, vertex A is replaced by
/// vertex B (so the `sA`/`sB`/`iA`/`iB` copy is observable).
#[test]
fn row26_c22_branch_u_le_0() {
    let (c, r) = apis();
    for k in 1..200i32 {
        let t = k as f32 * 0.37;
        let s = simplex_from_points(
            &[
                c2v {
                    x: t * 3.0,
                    y: t * 3.0,
                },
                c2v { x: t, y: t },
            ],
            1.0,
        );
        diff_c22(c, r, &format!("row26 k={k}"), s);
        let mut chk = s;
        unsafe { (c.c22)(&mut chk) };
        assert_eq!(chk.count, 1, "row26 k={k}: expected the count=1 branch");
        assert_eq!(
            chk.verts[0].iA, s.verts[1].iA,
            "row26: vertex A must have been replaced by vertex B"
        );
    }
}

/// Row 27 — the `else` branch: the origin projects inside the segment.
#[test]
fn row27_c22_branch_interior() {
    let (c, r) = apis();
    for k in 1..200i32 {
        let t = k as f32 * 0.11 + 0.5;
        // Segment straddling the origin along x.
        let s = simplex_from_points(&[c2v { x: -t, y: 1.0 }, c2v { x: t, y: 1.0 }], 1.0);
        diff_c22(c, r, &format!("row27 k={k}"), s);
        let mut chk = s;
        unsafe { (c.c22)(&mut chk) };
        assert_eq!(chk.count, 2, "row27 k={k}: expected the count=2 branch");
    }
}

/// Rows 28-29 — randomized `c22` over every float class and every `count`
/// value (the C reads only `a.p`/`b.p` regardless of `count`).
#[test]
fn row28_29_c22_randomized() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 28);
    for i in 0..N {
        let s = forge(&mut g, *&COUNTS[(i % COUNTS.len()) as usize]);
        diff_c22(c, r, &format!("row28 random #{i}"), s);
    }
    // Wild `p` values, incl. inf/NaN/subnormal.
    for i in 0..N {
        let mut s = forge(&mut g, 2);
        s.verts[0].p = g.wild_v();
        s.verts[1].p = g.wild_v();
        diff_c22(c, r, &format!("row28 wild #{i}"), s);
    }
    // Row 29: exactly-equal points, and points differing by one ULP.
    for i in 0..2000u32 {
        let mut g2 = Rng::new(SEED ^ 29 ^ i as u64);
        let p = g2.v();
        let q = c2v {
            x: f32::from_bits(p.x.to_bits() ^ 1),
            y: p.y,
        };
        diff_c22(c, r, &format!("row29 equal #{i}"), simplex_from_points(&[p, p], 1.0));
        diff_c22(c, r, &format!("row29 ulp #{i}"), simplex_from_points(&[p, q], 1.0));
    }
}

// ---------------------------------------------------------------------------
// Rows 30-37 — c23 (all seven branches)
// ---------------------------------------------------------------------------

fn diff_c23(c: &Api, r: &Api, ctx: &str, s: c2Simplex) -> c2Simplex {
    let mut cs = s;
    let mut rs = s;
    unsafe { (c.c23)(&mut cs) };
    unsafe { (r.c23)(&mut rs) };
    eq_simplex(ctx, &cs, &rs);
    cs
}

/// Rows 30-36 — every one of the seven `c23` branches, reached deliberately by
/// placing the origin in each Voronoi region of a triangle, then verified to
/// have actually been taken (via the resulting `count` and vertex shuffle).
///
/// A large randomized sweep records which branches were hit so the test fails
/// loudly if any branch is left unexercised.
#[test]
fn row30_36_c23_all_seven_branches() {
    let (c, r) = apis();
    let mut hits = [0usize; 7];

    // Classify by replicating the C's branch conditions (used only to assert
    // coverage; correctness itself is decided by the differential compare).
    let classify = |a: c2v, b: c2v, cc: c2v| -> usize {
        let dot = |p: c2v, q: c2v| p.x * q.x + p.y * q.y;
        let sub = |p: c2v, q: c2v| c2v {
            x: p.x - q.x,
            y: p.y - q.y,
        };
        let det = |p: c2v, q: c2v| p.x * q.y - p.y * q.x;
        let uab = dot(b, sub(b, a));
        let vab = dot(a, sub(a, b));
        let ubc = dot(cc, sub(cc, b));
        let vbc = dot(b, sub(b, cc));
        let uca = dot(a, sub(a, cc));
        let vca = dot(cc, sub(cc, a));
        let area = det(sub(b, a), sub(cc, a));
        let uabc = det(b, cc) * area;
        let vabc = det(cc, a) * area;
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
    };

    // Hand-built cases: a triangle, translated so the origin lands in each of
    // its 7 feature regions (3 vertices, 3 edges, interior).
    let tri = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 4.0, y: 0.0 },
        c2v { x: 0.0, y: 3.0 },
    ];
    let offsets = [
        c2v { x: -2.0, y: -2.0 },  // beyond vertex A
        c2v { x: 6.0, y: -2.0 },   // beyond vertex B
        c2v { x: -2.0, y: 5.0 },   // beyond vertex C
        c2v { x: 2.0, y: -2.0 },   // edge AB
        c2v { x: 4.0, y: 3.0 },    // edge BC
        c2v { x: -2.0, y: 1.5 },   // edge CA
        c2v { x: 1.0, y: 1.0 },    // interior
    ];
    for (oi, off) in offsets.iter().enumerate() {
        let pts: Vec<c2v> = tri
            .iter()
            .map(|p| c2v {
                x: p.x - off.x,
                y: p.y - off.y,
            })
            .collect();
        let s = simplex_from_points(&pts, 1.0);
        let out = diff_c23(c, r, &format!("row30-36 offset#{oi}"), s);
        let br = classify(pts[0], pts[1], pts[2]);
        hits[br] += 1;
        // Sanity: branches 0..3 collapse to count 1, 3..6 to 2, 6 to 3.
        let expect_count = match br {
            0 | 1 | 2 => 1,
            3 | 4 | 5 => 2,
            _ => 3,
        };
        assert_eq!(
            out.count, expect_count,
            "row30-36 offset#{oi}: branch {br} should yield count {expect_count}"
        );
    }

    // Randomized sweep across many triangle shapes and origin placements.
    let mut g = Rng::new(SEED ^ 30);
    for i in 0..N {
        let pts = [g.v(), g.v(), g.v()];
        let s = simplex_from_points(&pts, 1.0);
        diff_c23(c, r, &format!("row30-36 random #{i}"), s);
        hits[classify(pts[0], pts[1], pts[2])] += 1;
    }
    // Dense grid sweep around a fixed triangle to guarantee full coverage.
    for ix in -12..=12i32 {
        for iy in -12..=12i32 {
            let off = c2v {
                x: ix as f32 * 0.7,
                y: iy as f32 * 0.7,
            };
            let pts: Vec<c2v> = tri
                .iter()
                .map(|p| c2v {
                    x: p.x - off.x,
                    y: p.y - off.y,
                })
                .collect();
            let s = simplex_from_points(&pts, 1.0);
            diff_c23(c, r, &format!("row30-36 grid {ix},{iy}"), s);
            hits[classify(pts[0], pts[1], pts[2])] += 1;
        }
    }

    for (b, &n) in hits.iter().enumerate() {
        assert!(n > 0, "c23 branch {b} was never exercised (hits: {hits:?})");
    }
    println!("c23 branch hit counts: {hits:?}");
}

/// Row 37 — randomized `c23` including degenerate triangles (collinear points,
/// duplicated vertices, zero area) and every float class, plus every `count`.
#[test]
fn row37_c23_degenerate_and_wild() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 37);

    // Collinear / zero-area triangles: `area == 0` makes uABC/vABC/wABC all 0.
    for i in 0..N / 2 {
        let base = g.v();
        let dir = g.v();
        let t = [g.coord(), g.coord(), g.coord()];
        let pts: Vec<c2v> = t
            .iter()
            .map(|&k| c2v {
                x: base.x + dir.x * k,
                y: base.y + dir.y * k,
            })
            .collect();
        diff_c23(
            c,
            r,
            &format!("row37 collinear #{i}"),
            simplex_from_points(&pts, 1.0),
        );
    }
    // Duplicated vertices.
    for i in 0..N / 4 {
        let p = g.v();
        let q = g.v();
        for pts in [
            vec![p, p, q],
            vec![p, q, p],
            vec![q, p, p],
            vec![p, p, p],
        ] {
            diff_c23(
                c,
                r,
                &format!("row37 dup #{i} {pts:?}"),
                simplex_from_points(&pts, 1.0),
            );
        }
    }
    // Wild float classes and arbitrary `count` values.
    for i in 0..N {
        let mut s = forge(&mut g, COUNTS[i % COUNTS.len()]);
        s.verts[0].p = g.wild_v();
        s.verts[1].p = g.wild_v();
        s.verts[2].p = g.wild_v();
        diff_c23(c, r, &format!("row37 wild #{i}"), s);
    }
}

// ---------------------------------------------------------------------------
// Rows 38-41 — c2D
// ---------------------------------------------------------------------------

#[test]
fn row38_41_c2D() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 38);

    // Row 38: count == 1.
    for i in 0..N / 4 {
        let s = simplex_from_points(&[g.wild_v()], 1.0);
        let mut cs = s;
        let mut rs = s;
        eq_v(
            &format!("row38 count=1 #{i}"),
            unsafe { (c.c2D)(&mut cs) },
            unsafe { (r.c2D)(&mut rs) },
        );
    }

    // Rows 39-40: count == 2, both sides of the `c2Det2(ab, -a.p) > 0` test.
    let mut pos = 0usize;
    let mut neg = 0usize;
    for i in 0..N {
        let a = g.v();
        let b = g.v();
        let s = simplex_from_points(&[a, b], 1.0);
        let mut cs = s;
        let mut rs = s;
        eq_v(
            &format!("row39-40 count=2 #{i} {a:?} {b:?}"),
            unsafe { (c.c2D)(&mut cs) },
            unsafe { (r.c2D)(&mut rs) },
        );
        let det = (b.x - a.x) * -a.y - (b.y - a.y) * -a.x;
        if det > 0.0 {
            pos += 1
        } else {
            neg += 1
        }
    }
    assert!(pos > 0 && neg > 0, "row39-40: both sides of the det test must be hit ({pos}/{neg})");

    // Row 41: count == 3 and every other value share the `c2V(0,0)` body.
    for &count in &COUNTS {
        for i in 0..256 {
            let s = forge(&mut g, count);
            let mut cs = s;
            let mut rs = s;
            eq_v(
                &format!("row41 count={count} #{i}"),
                unsafe { (c.c2D)(&mut cs) },
                unsafe { (r.c2D)(&mut rs) },
            );
            eq_bytes(&format!("row41 no mutation count={count} #{i}"), &cs, &s);
        }
    }
    // Wild `p` with count=2 (inf/NaN through c2Det2/c2Skew/c2CCW90).
    for i in 0..N {
        let mut s = forge(&mut g, 2);
        s.verts[0].p = g.wild_v();
        s.verts[1].p = g.wild_v();
        let mut cs = s;
        let mut rs = s;
        eq_v(
            &format!("row39-40 wild #{i}"),
            unsafe { (c.c2D)(&mut cs) },
            unsafe { (r.c2D)(&mut rs) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 42 — c2L
// ---------------------------------------------------------------------------

#[test]
fn row42_c2L() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 42);
    for &count in &COUNTS {
        for i in 0..N / 3 {
            let mut s = forge(&mut g, count);
            // Exercise div == 0 (=> den == inf) deliberately every 4th case.
            if i % 4 == 0 {
                s.div = 0.0;
            }
            if i % 7 == 0 {
                s.div = f32::NAN;
            }
            let mut cs = s;
            let mut rs = s;
            eq_v(
                &format!("row42 count={count} div={:?} #{i}", s.div),
                unsafe { (c.c2L)(&mut cs) },
                unsafe { (r.c2L)(&mut rs) },
            );
            eq_bytes(&format!("row42 no mutation count={count} #{i}"), &cs, &s);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 43 — c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row43_c2Witness() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 43);
    for &count in &COUNTS {
        for i in 0..N / 3 {
            let mut s = forge(&mut g, count);
            match i % 5 {
                0 => s.div = 0.0,
                1 => s.div = f32::NAN,
                2 => s.div = 1.0,
                _ => {}
            }
            let mut cs = s;
            let mut rs = s;
            // Pre-fill the outputs so the `default:` arm (which writes zeros)
            // and any arm that fails to write are both observable.
            let mut ca = c2v { x: 1234.5, y: -678.9 };
            let mut cb = c2v { x: -1.0, y: 2.0 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe { (c.c2Witness)(&mut cs, &mut ca, &mut cb) };
            unsafe { (r.c2Witness)(&mut rs, &mut ra, &mut rb) };
            eq_v(&format!("row43 outA count={count} #{i}"), ca, ra);
            eq_v(&format!("row43 outB count={count} #{i}"), cb, rb);
            eq_bytes(&format!("row43 no mutation count={count} #{i}"), &cs, &s);
        }
    }
    // Aliasing: the C is called with `&a, &b` from c2GJK, but a caller could
    // legitimately pass the same pointer twice.
    for i in 0..1000 {
        let mut s = forge(&mut g, 1 + (i % 3));
        s.div = 2.0;
        let mut cs = s;
        let mut rs = s;
        let mut cv = c2v::default();
        let mut rv = c2v::default();
        unsafe { (c.c2Witness)(&mut cs, &mut cv, &mut cv) };
        unsafe { (r.c2Witness)(&mut rs, &mut rv, &mut rv) };
        eq_v(&format!("row43 aliased #{i}"), cv, rv);
    }
}

// ---------------------------------------------------------------------------
// Rows 44-49 — c2Support
// ---------------------------------------------------------------------------

#[test]
fn row44_49_c2Support() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 44);

    // Rows 44-47: the counts real proxies use (1, 2, 4) plus the full 8 slots.
    for &count in &[1i32, 2, 3, 4, 5, 6, 7, 8] {
        for i in 0..N / 4 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = g.v();
            }
            let d = g.v();
            eq_int(
                &format!("row44-47 count={count} #{i} d={d:?}"),
                unsafe { (c.c2Support)(verts.as_ptr(), count, d) },
                unsafe { (r.c2Support)(verts.as_ptr(), count, d) },
            );
        }
    }

    // Row 48: count <= 0 still dereferences verts[0], then returns 0.
    for &count in &[0i32, -1, -100, i32::MIN] {
        for i in 0..256 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = g.v();
            }
            let d = g.v();
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            eq_int(&format!("row48 count={count} #{i}"), cv, rv);
            eq_int(&format!("row48 count={count} #{i} is 0"), cv, 0);
        }
    }

    // Row 49: zero and NaN directions => every `>` is false => index 0.
    for &d in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v {
            x: f32::NAN,
            y: f32::NAN,
        },
        c2v {
            x: f32::NAN,
            y: 1.0,
        },
        c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
    ] {
        for &count in &[1i32, 2, 4, 8] {
            for i in 0..256 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = g.wild_v();
                }
                eq_int(
                    &format!("row49 d={d:?} count={count} #{i}"),
                    unsafe { (c.c2Support)(verts.as_ptr(), count, d) },
                    unsafe { (r.c2Support)(verts.as_ptr(), count, d) },
                );
            }
        }
    }

    // Exact ties: all verts identical => strict `>` never fires => 0.
    for &count in &[1i32, 2, 4, 8] {
        let verts = [c2v { x: 1.0, y: 2.0 }; 8];
        for i in 0..64 {
            let d = g.v();
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            eq_int(&format!("row49 ties count={count} #{i}"), cv, rv);
            eq_int(&format!("row49 ties count={count} #{i} is 0"), cv, 0);
        }
    }

    // Axis-aligned AABB verts with axis-aligned directions: the tie-breaking
    // order matters and must match exactly.
    let bb_verts = [
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: -1.0, y: 1.0 },
        c2v::default(),
        c2v::default(),
        c2v::default(),
        c2v::default(),
    ];
    for &d in &[
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: -1.0, y: -1.0 },
    ] {
        eq_int(
            &format!("row46 aabb tie d={d:?}"),
            unsafe { (c.c2Support)(bb_verts.as_ptr(), 4, d) },
            unsafe { (r.c2Support)(bb_verts.as_ptr(), 4, d) },
        );
    }

    // Wild vertex values.
    for &count in &[1i32, 2, 4, 8] {
        for i in 0..N / 4 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = g.wild_v();
            }
            let d = g.wild_v();
            eq_int(
                &format!("row44-47 wild count={count} #{i}"),
                unsafe { (c.c2Support)(verts.as_ptr(), count, d) },
                unsafe { (r.c2Support)(verts.as_ptr(), count, d) },
            );
        }
    }
}
