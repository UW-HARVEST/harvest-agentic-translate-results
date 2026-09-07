//! Phase B — CONFIGS.md rows 20..47 (the lowest-level GJK entry points, driven
//! directly rather than through the `c2GJK` / `c2Collided` wrappers).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

const N: usize = 3000;

// ---------------------------------------------------------------------------
// Simplex construction helpers
// ---------------------------------------------------------------------------

/// Fill every field of a simplex with recognisable junk so that any field the C
/// leaves alone is still part of the byte comparison.
fn seeded_simplex(r: &mut Rng, count: c_int) -> c2Simplex {
    let mut s = c2Simplex::default();
    for i in 0..4 {
        s.verts[i].sA = r.geo_v();
        s.verts[i].sB = r.geo_v();
        s.verts[i].p = r.geo_v();
        s.verts[i].u = r.sym(3.0);
        s.verts[i].iA = r.below(8) as c_int;
        s.verts[i].iB = r.below(8) as c_int;
    }
    s.div = match r.below(6) {
        0 => 0.0,
        1 => 1.0,
        2 => s.verts[0].u + s.verts[1].u,
        _ => r.sym(10.0),
    };
    s.count = count;
    s
}

/// Which branch of the C `c23` a given `(a,b,c)` triple takes — used only to
/// prove the randomized corpus actually reaches every branch.
fn c23_branch(a: c2v, b: c2v, c: c2v) -> usize {
    let dot = |u: c2v, v: c2v| u.x * v.x + u.y * v.y;
    let det = |u: c2v, v: c2v| u.x * v.y - u.y * v.x;
    let sub = |u: c2v, v: c2v| c2v { x: u.x - v.x, y: u.y - v.y };
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uABC = det(b, c) * area;
    let vABC = det(c, a) * area;
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

fn c22_branch(a: c2v, b: c2v) -> usize {
    let dot = |u: c2v, v: c2v| u.x * v.x + u.y * v.y;
    let sub = |u: c2v, v: c2v| c2v { x: u.x - v.x, y: u.y - v.y };
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

// ---------------------------------------------------------------------------
// rows 20-22: c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn row20_22_c2GJKSimplexMetric() {
    let a = api();
    let mut d = Diff::new("rows 20-22: c2GJKSimplexMetric count=1/2/3");
    let mut r = Rng::new(2022);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let mut s = seeded_simplex(&mut r, count);
            // occasionally force degeneracies
            match i % 5 {
                0 => s.verts[1].p = s.verts[0].p,          // zero-length edge
                1 => {
                    // collinear triangle -> area == 0
                    let base = s.verts[0].p;
                    let dir = c2v { x: r.sym(10.0), y: r.sym(10.0) };
                    s.verts[1].p = c2v { x: base.x + dir.x, y: base.y + dir.y };
                    s.verts[2].p = c2v { x: base.x + dir.x * 2.0, y: base.y + dir.y * 2.0 };
                }
                _ => {}
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe {
                ((a.c2GJKSimplexMetric.0)(&mut cs), (a.c2GJKSimplexMetric.1)(&mut rs))
            };
            d.check(feq(cv, rv), || {
                format!("c2GJKSimplexMetric count={count} {s:?}\n C={} R={}", fmt_f(cv), fmt_f(rv))
            });
            // the function must not mutate the simplex in either build
            d.check(simplexeq(&cs, &rs), || format!("c2GJKSimplexMetric mutated differently\n C={cs:?}\n R={rs:?}"));
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 23-26: c22
// ---------------------------------------------------------------------------

#[test]
fn row23_26_c22() {
    let a = api();
    let mut d = Diff::new("rows 23-26: c22 all three branches + a.p == b.p");
    let mut r = Rng::new(2326);
    let mut hits = [0usize; 3];
    for i in 0..N * 3 {
        let mut s = seeded_simplex(&mut r, 2);
        if i % 7 == 0 {
            s.verts[1].p = s.verts[0].p; // row 26
        }
        hits[c22_branch(s.verts[0].p, s.verts[1].p)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            (a.c22.0)(&mut cs);
            (a.c22.1)(&mut rs);
        }
        d.check(bytes_of(&cs) == bytes_of(&rs), || {
            format!("c22 in={s:?}\n C={cs:?}\n R={rs:?}")
        });
    }
    assert!(hits.iter().all(|&h| h > 0), "c22 branch coverage incomplete: {hits:?}");
    eprintln!("c22 branch hits: {hits:?}");
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 27-35: c23
// ---------------------------------------------------------------------------

#[test]
fn row27_35_c23() {
    let a = api();
    let mut d = Diff::new("rows 27-35: c23 all seven branches, both windings, degenerate area");
    let mut r = Rng::new(2735);
    let mut hits = [0usize; 7];
    for i in 0..N * 6 {
        let mut s = seeded_simplex(&mut r, 3);
        match i % 9 {
            0 => {
                // row 34: collinear -> area == 0 -> final else with div == 0
                let base = r.geo_v();
                let dir = c2v { x: r.sym(20.0), y: r.sym(20.0) };
                s.verts[0].p = base;
                s.verts[1].p = c2v { x: base.x + dir.x, y: base.y + dir.y };
                s.verts[2].p = c2v { x: base.x + dir.x * 2.0, y: base.y + dir.y * 2.0 };
            }
            1 => {
                // row 35: reverse the winding of a random triangle
                let t = s.verts[1];
                s.verts[1] = s.verts[2];
                s.verts[2] = t;
            }
            2 => {
                // small triangle straddling the origin -> interior branch
                s.verts[0].p = c2v { x: -1.0 - r.unit(), y: -1.0 - r.unit() };
                s.verts[1].p = c2v { x: 1.0 + r.unit(), y: -1.0 - r.unit() };
                s.verts[2].p = c2v { x: r.sym(0.5), y: 1.0 + r.unit() };
            }
            3 => {
                // duplicated vertex
                s.verts[2].p = s.verts[1].p;
            }
            _ => {}
        }
        hits[c23_branch(s.verts[0].p, s.verts[1].p, s.verts[2].p)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            (a.c23.0)(&mut cs);
            (a.c23.1)(&mut rs);
        }
        d.check(bytes_of(&cs) == bytes_of(&rs), || {
            format!("c23 in={s:?}\n C={cs:?}\n R={rs:?}")
        });
    }
    assert!(hits.iter().all(|&h| h > 0), "c23 branch coverage incomplete: {hits:?}");
    eprintln!("c23 branch hits: {hits:?}");
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 36-39: c2D
// ---------------------------------------------------------------------------

#[test]
fn row36_39_c2D() {
    let a = api();
    let mut d = Diff::new("rows 36-39: c2D count=1 / 2(skew) / 2(ccw90) / 3");
    let mut r = Rng::new(3639);
    let mut skew = 0usize;
    let mut ccw = 0usize;
    for count in [1i32, 2, 3] {
        for _ in 0..N {
            let s = seeded_simplex(&mut r, count);
            if count == 2 {
                let det = {
                    let ab = c2v { x: s.verts[1].p.x - s.verts[0].p.x, y: s.verts[1].p.y - s.verts[0].p.y };
                    let na = c2v { x: -s.verts[0].p.x, y: -s.verts[0].p.y };
                    ab.x * na.y - ab.y * na.x
                };
                if det > 0.0 {
                    skew += 1;
                } else {
                    ccw += 1;
                }
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((a.c2D.0)(&mut cs), (a.c2D.1)(&mut rs)) };
            d.check(veq(cv, rv), || {
                format!("c2D count={count} in={s:?}\n C={} R={}", fmt_v(cv), fmt_v(rv))
            });
            d.check(bytes_of(&cs) == bytes_of(&rs), || "c2D mutated the simplex differently".into());
        }
    }
    assert!(skew > 0 && ccw > 0, "c2D count==2 branch coverage incomplete: skew={skew} ccw={ccw}");
    d.finish();
}

// ---------------------------------------------------------------------------
// row 40: c2L
// ---------------------------------------------------------------------------

#[test]
fn row40_c2L() {
    let a = api();
    let mut d = Diff::new("row 40: c2L count=1/2 x random div and weights");
    let mut r = Rng::new(40);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let mut s = seeded_simplex(&mut r, count);
            if i % 4 == 0 {
                // make `div` the true weight sum (the physically meaningful case)
                s.verts[0].u = r.unit();
                s.verts[1].u = r.unit();
                s.div = s.verts[0].u + s.verts[1].u;
            }
            if i % 11 == 0 {
                s.div = 0.0;
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((a.c2L.0)(&mut cs), (a.c2L.1)(&mut rs)) };
            d.check(veq(cv, rv), || {
                format!("c2L count={count} div={} in={s:?}\n C={} R={}", fmt_f(s.div), fmt_v(cv), fmt_v(rv))
            });
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 41-43: c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row41_43_c2Witness() {
    let a = api();
    let mut d = Diff::new("rows 41-43: c2Witness count=1/2/3 x random barycentrics");
    let mut r = Rng::new(4143);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let mut s = seeded_simplex(&mut r, count);
            if i % 3 == 0 {
                let (u0, u1, u2) = (r.unit(), r.unit(), r.unit());
                s.verts[0].u = u0;
                s.verts[1].u = u1;
                s.verts[2].u = u2;
                s.div = u0 + u1 + u2;
            }
            if i % 13 == 0 {
                s.div = 0.0;
            }
            let mut cs = s;
            let mut rs = s;
            let mut ca = c2v { x: 12.5, y: -7.0 };
            let mut cb = c2v { x: -3.0, y: 9.0 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                (a.c2Witness.0)(&mut cs, &mut ca, &mut cb);
                (a.c2Witness.1)(&mut rs, &mut ra, &mut rb);
            }
            d.check(veq(ca, ra) && veq(cb, rb), || {
                format!(
                    "c2Witness count={count} div={} in={s:?}\n C=({}, {})\n R=({}, {})",
                    fmt_f(s.div),
                    fmt_v(ca),
                    fmt_v(cb),
                    fmt_v(ra),
                    fmt_v(rb)
                )
            });
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 44-47: c2Support
// ---------------------------------------------------------------------------

#[test]
fn row44_47_c2Support() {
    let a = api();
    let mut d = Diff::new("rows 44-47: c2Support count=1/2/4/8 x random and tying directions");
    let r = Rng::new(4447);
    for count in [1i32, 2, 4, 8] {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = r.geo_v();
            }
            if i % 9 == 0 {
                // force all vertices identical -> every dot ties
                let v0 = verts[0];
                verts = [v0; 8];
            }
            if i % 9 == 1 {
                // AABB-style corners so axis-aligned directions tie on two verts
                verts[0] = c2v { x: -1.0, y: -1.0 };
                verts[1] = c2v { x: 1.0, y: -1.0 };
                verts[2] = c2v { x: 1.0, y: 1.0 };
                verts[3] = c2v { x: -1.0, y: 1.0 };
            }
            let dir = match i % 7 {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => c2v { x: 1.0, y: 0.0 },
                2 => c2v { x: 0.0, y: 1.0 },
                3 => c2v { x: -1.0, y: 0.0 },
                4 => c2v { x: 0.0, y: -1.0 },
                _ => r.wild_v(),
            };
            let (cv, rv) = unsafe {
                ((a.c2Support.0)(verts.as_ptr(), count, dir), (a.c2Support.1)(verts.as_ptr(), count, dir))
            };
            d.check(cv == rv, || {
                format!("c2Support(count={count}, d={}) verts={verts:?} C={cv} R={rv}", fmt_v(dir))
            });
        }
    }
    d.finish();
}
