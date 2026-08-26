//! Phase B, `CONFIGS.md` rows 24–39: the simplex tier
//! (`c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2L`, `c2Witness`).
//!
//! `c22` has 3 branches and `c23` has 7.  Rather than *hoping* random inputs
//! reach them, every branch is classified up front (using the C library's own
//! `c2Dot`/`c2Sub`/`c2Det2` so the predicates are evaluated in exactly the same
//! f32 arithmetic as the code under test), the hit counts are accumulated, and
//! the test FAILS if any branch was never exercised.
//!
//! The comparison is over the whole 152-byte `c2Simplex`, so the vertex
//! shuffling (`s->a = s->b`, `s->b = s->c`, …) is verified field by field —
//! including `sA`/`sB`/`iA`/`iB`, which the solvers only move around.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::c_int;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// helpers: build randomised simplexes, and classify the solver branches using
// the C library's own arithmetic.
// ---------------------------------------------------------------------------

fn rand_sv(rng: &mut Rng, p: c2v) -> c2sv {
    c2sv {
        sA: rng.v(),
        sB: rng.v(),
        p,
        u: rng.coord(),
        iA: (rng.below(8)) as c_int,
        iB: (rng.below(8)) as c_int,
    }
}

fn simplex(rng: &mut Rng, pts: &[c2v], div: f32) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div,
        count: pts.len() as c_int,
    };
    for (i, v) in s.verts.iter_mut().enumerate() {
        let p = if i < pts.len() { pts[i] } else { rng.v() };
        *v = rand_sv(rng, p);
    }
    s
}

/// Point generator biased to reach every solver branch.
fn gen_point(rng: &mut Rng) -> c2v {
    match rng.below(7) {
        0 => c2v {
            x: (rng.below(9) as f32) - 4.0,
            y: (rng.below(9) as f32) - 4.0,
        },
        1 => c2v { x: 0.0, y: 0.0 },
        2 => c2v {
            x: rng.range(-1.0, 1.0),
            y: rng.range(-1.0, 1.0),
        },
        3 => {
            let th = rng.range(0.0, 6.283_185_5);
            let rr = rng.range(0.1, 5.0);
            c2v {
                x: rr * th.cos(),
                y: rr * th.sin(),
            }
        }
        4 => c2v {
            x: rng.range(-1e4, 1e4),
            y: rng.range(-1e4, 1e4),
        },
        5 => c2v {
            x: rng.range(-1e-4, 1e-4),
            y: rng.range(-1e-4, 1e-4),
        },
        _ => rng.v(),
    }
}

/// A triangle that (usually) encloses the origin — needed for `c23` branch 7.
fn gen_enclosing_triangle(rng: &mut Rng) -> [c2v; 3] {
    let base = rng.range(0.0, 2.094_395_2);
    let mut out = [c2v::default(); 3];
    for (k, o) in out.iter_mut().enumerate() {
        let th = base + (k as f32) * 2.094_395_2 + rng.range(-0.5, 0.5);
        let rr = rng.range(0.5, 8.0);
        *o = c2v {
            x: rr * th.cos(),
            y: rr * th.sin(),
        };
    }
    out
}

struct Ops {
    dot: FnFvv,
    sub: FnVvv,
    det2: FnFvv,
}

impl Ops {
    fn load() -> Self {
        let (dot, _) = fnpair!("c2Dot", FnFvv);
        let (sub, _) = fnpair!("c2Sub", FnVvv);
        let (det2, _) = fnpair!("c2Det2", FnFvv);
        Ops { dot, sub, det2 }
    }

    /// Which of `c22`'s 3 branches the given (a,b) takes.
    fn c22_branch(&self, a: c2v, b: c2v) -> usize {
        let u = (self.dot)(b, (self.sub)(b, a));
        let v = (self.dot)(a, (self.sub)(a, b));
        if v <= 0.0 {
            0
        } else if u <= 0.0 {
            1
        } else {
            2
        }
    }

    /// Which of `c23`'s 7 branches the given (a,b,c) takes.
    fn c23_branch(&self, a: c2v, b: c2v, c: c2v) -> usize {
        let uAB = (self.dot)(b, (self.sub)(b, a));
        let vAB = (self.dot)(a, (self.sub)(a, b));
        let uBC = (self.dot)(c, (self.sub)(c, b));
        let vBC = (self.dot)(b, (self.sub)(b, c));
        let uCA = (self.dot)(a, (self.sub)(a, c));
        let vCA = (self.dot)(c, (self.sub)(c, a));
        let area = (self.det2)((self.sub)(b, a), (self.sub)(c, a));
        let uABC = (self.det2)(b, c) * area;
        let vABC = (self.det2)(c, a) * area;
        let wABC = (self.det2)(a, b) * area;
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
}

#[track_caller]
fn assert_all_hit(name: &str, hits: &[usize], min: usize) {
    for (i, &h) in hits.iter().enumerate() {
        assert!(
            h >= min,
            "{name}: branch {i} was hit only {h} time(s) (need >= {min}); hits = {hits:?}"
        );
    }
    eprintln!("[coverage] {name} branch hits = {hits:?}");
}

// ---------------------------------------------------------------------------
// row 24 — c2GJKSimplexMetric for count 1 / 2 / 3
// ---------------------------------------------------------------------------

#[test]
fn row24_c2GJKSimplexMetric() {
    let (c, r) = fnpair!("c2GJKSimplexMetric", FnSimplexF);
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..N {
        for count in 1..=3usize {
            let pts: Vec<c2v> = (0..count).map(|_| gen_point(&mut rng)).collect();
            let dv = rng.coord();
            let s = simplex(&mut rng, &pts, dv);
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            eq_f32(&format!("metric #{i} count={count} {pts:?}"), cv, rv);
            // the function must not modify the simplex
            eq_raw(&format!("metric-nomod #{i} count={count}"), &cs, &rs);
        }
    }
    // extreme / special points
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let pt = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            for count in 1..=3usize {
                let s = simplex(&mut rng, &vec![pt; count], 1.0);
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
                eq_f32(&format!("metric odd count={count} {pt:?}"), cv, rv);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 25–28 — c22, all three branches
// ---------------------------------------------------------------------------

#[test]
fn rows25to28_c22_all_branches() {
    let (c, r) = fnpair!("c22", FnSimplexVoid);
    let ops = Ops::load();
    let mut rng = Rng::new(SEED ^ 22_00);
    let mut hits = [0usize; 3];
    for i in 0..N {
        let a = gen_point(&mut rng);
        let b = if rng.below(6) == 0 { a } else { gen_point(&mut rng) };
        let dv = rng.coord();
        let s = simplex(&mut rng, &[a, b], dv);
        let br = ops.c22_branch(a, b);
        hits[br] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        eq_raw(
            &format!("c22 #{i} branch={br} a={a:?} b={b:?} in={s:?}"),
            &cs,
            &rs,
        );
    }
    assert_all_hit("c22", &hits, 200);
}

/// row 28 continued: `c22` on NaN / inf points (all `<=` tests false → the
/// `else` branch with a NaN `div`).
#[test]
fn row28_c22_specials() {
    let (c, r) = fnpair!("c22", FnSimplexVoid);
    let mut rng = Rng::new(SEED ^ 22_01);
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            for &t in ODDBALLS.iter() {
                let a = c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                };
                let b = c2v {
                    x: f32::from_bits(t),
                    y: f32::from_bits(p),
                };
                let s = simplex(&mut rng, &[a, b], 1.0);
                let mut cs = s;
                let mut rs = s;
                unsafe {
                    c(&mut cs);
                    r(&mut rs);
                }
                eq_raw(&format!("c22 odd a={a:?} b={b:?}"), &cs, &rs);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 29–36 — c23, all seven branches
// ---------------------------------------------------------------------------

#[test]
fn rows29to36_c23_all_branches() {
    let (c, r) = fnpair!("c23", FnSimplexVoid);
    let ops = Ops::load();
    let mut rng = Rng::new(SEED ^ 23_00);
    let mut hits = [0usize; 7];
    for i in 0..(N * 4) {
        // half fully random, half origin-enclosing (to reach branch 7)
        let pts: [c2v; 3] = if rng.bool() {
            gen_enclosing_triangle(&mut rng)
        } else {
            let a = gen_point(&mut rng);
            let b = if rng.below(10) == 0 { a } else { gen_point(&mut rng) };
            let cc = match rng.below(10) {
                0 => a,
                1 => b,
                _ => gen_point(&mut rng),
            };
            [a, b, cc]
        };
        let dv = rng.coord();
        let s = simplex(&mut rng, &pts, dv);
        let br = ops.c23_branch(pts[0], pts[1], pts[2]);
        hits[br] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        eq_raw(&format!("c23 #{i} branch={br} pts={pts:?} in={s:?}"), &cs, &rs);
    }
    assert_all_hit("c23", &hits, 100);
}

/// row 36 continued: degenerate and special-value triangles.
#[test]
fn row36_c23_degenerate_and_specials() {
    let (c, r) = fnpair!("c23", FnSimplexVoid);
    let mut rng = Rng::new(SEED ^ 23_01);

    let mut run = |pts: [c2v; 3], ctx: String| {
        let s = simplex(&mut rng, &pts, 1.0);
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        eq_raw(&format!("c23 {ctx}"), &cs, &rs);
    };

    // all three equal (area == 0)
    for &s in SPECIALS.iter() {
        let p = c2v { x: s, y: s };
        run([p, p, p], format!("equal {s:?}"));
    }
    // collinear
    for k in 0..32 {
        let d = c2v {
            x: (k as f32) * 0.25,
            y: (k as f32) * -0.5,
        };
        run(
            [
                c2v { x: 0.0, y: 0.0 },
                d,
                c2v {
                    x: d.x * 2.0,
                    y: d.y * 2.0,
                },
            ],
            format!("collinear {k}"),
        );
    }
    // one vertex at the origin
    for k in 0..32 {
        let th = (k as f32) * 0.2;
        run(
            [
                c2v { x: 0.0, y: 0.0 },
                c2v {
                    x: th.cos(),
                    y: th.sin(),
                },
                c2v {
                    x: -th.sin(),
                    y: th.cos(),
                },
            ],
            format!("origin-vertex {k}"),
        );
    }
    // oddball bit patterns
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            let b = c2v {
                x: f32::from_bits(q),
                y: f32::from_bits(p),
            };
            let cc = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(p),
            };
            run([a, b, cc], format!("odd {a:?} {b:?} {cc:?}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 37 — c2D for count 1 / 2 (both sub-branches) / 3
// ---------------------------------------------------------------------------

#[test]
fn row37_c2D() {
    let (c, r) = fnpair!("c2D", FnSimplexV);
    let (det2, _) = fnpair!("c2Det2", FnFvv);
    let (sub, _) = fnpair!("c2Sub", FnVvv);
    let (neg, _) = fnpair!("c2Neg", FnVv);
    let mut rng = Rng::new(SEED ^ 37);
    let mut hits = [0usize; 4]; // count1, count2-skew, count2-ccw90, count3
    for i in 0..N {
        for count in 1..=3usize {
            let pts: Vec<c2v> = (0..count).map(|_| gen_point(&mut rng)).collect();
            let dv = rng.coord();
            let s = simplex(&mut rng, &pts, dv);
            match count {
                1 => hits[0] += 1,
                2 => {
                    let ab = sub(pts[1], pts[0]);
                    if det2(ab, neg(pts[0])) > 0.0 {
                        hits[1] += 1;
                    } else {
                        hits[2] += 1;
                    }
                }
                _ => hits[3] += 1,
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            eq_raw(&format!("c2D #{i} count={count} {pts:?}"), &cv, &rv);
            eq_raw(&format!("c2D-nomod #{i} count={count}"), &cs, &rs);
        }
    }
    assert_all_hit("c2D", &hits, 100);
    // specials
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            let b = c2v {
                x: f32::from_bits(q),
                y: f32::from_bits(p),
            };
            for count in 1..=3usize {
                let s = simplex(&mut rng, &[a, b, a][..count], 1.0);
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
                eq_raw(&format!("c2D odd count={count} {a:?} {b:?}"), &cv, &rv);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 38 — c2L for count 1 / 2 (barycentric blend, random u and div)
// ---------------------------------------------------------------------------

#[test]
fn row38_c2L() {
    let (c, r) = fnpair!("c2L", FnSimplexV);
    let mut rng = Rng::new(SEED ^ 38);
    for i in 0..N {
        for count in 1..=2usize {
            let pts: Vec<c2v> = (0..count).map(|_| gen_point(&mut rng)).collect();
            let div = match rng.below(6) {
                0 => 1.0,
                1 => rng.range(1e-6, 1.0),
                2 => rng.range(1.0, 1e6),
                _ => rng.coord(),
            };
            let mut s = simplex(&mut rng, &pts, div);
            // make the u's a genuine barycentric split of div sometimes
            if rng.bool() && count == 2 {
                let t = rng.range(0.0, 1.0);
                s.verts[0].u = div * t;
                s.verts[1].u = div * (1.0 - t);
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            eq_raw(
                &format!("c2L #{i} count={count} div={div:?} {pts:?}"),
                &cv,
                &rv,
            );
            eq_raw(&format!("c2L-nomod #{i} count={count}"), &cs, &rs);
        }
    }
    // div == 0 / ±inf / NaN, u special
    for &d in SPECIALS.iter() {
        for &uu in SPECIALS.iter() {
            for count in 1..=2usize {
                let mut s = simplex(&mut rng, &vec![c2v { x: 1.5, y: -2.5 }; count], d);
                s.verts[0].u = uu;
                s.verts[1].u = -uu;
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
                eq_raw(
                    &format!("c2L special count={count} div={d:?} u={uu:?}"),
                    &cv,
                    &rv,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 39 — c2Witness for count 1 / 2 / 3
// ---------------------------------------------------------------------------

#[test]
fn row39_c2Witness() {
    let (c, r) = fnpair!("c2Witness", FnWitness);
    let mut rng = Rng::new(SEED ^ 39);
    let poison = c2v {
        x: f32::from_bits(0x1234_5678),
        y: f32::from_bits(0x8765_4321),
    };
    for i in 0..N {
        for count in 1..=3usize {
            let pts: Vec<c2v> = (0..count).map(|_| gen_point(&mut rng)).collect();
            let div = match rng.below(5) {
                0 => 1.0,
                1 => rng.range(1e-6, 1e6),
                _ => rng.coord(),
            };
            let s = simplex(&mut rng, &pts, div);
            let mut cs = s;
            let mut rs = s;
            let (mut ca, mut cb) = (poison, poison);
            let (mut ra, mut rb) = (poison, poison);
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            let ctx = format!("c2Witness #{i} count={count} div={div:?}");
            eq_raw(&format!("{ctx} a"), &ca, &ra);
            eq_raw(&format!("{ctx} b"), &cb, &rb);
            eq_raw(&format!("{ctx} nomod"), &cs, &rs);
        }
    }
    // div == 0 / inf / NaN
    for &d in SPECIALS.iter() {
        for count in 1..=3usize {
            let mut s = simplex(&mut rng, &vec![c2v { x: 2.0, y: 3.0 }; count], d);
            for k in 0..3 {
                s.verts[k].u = (k as f32) + 1.0;
                s.verts[k].sA = c2v {
                    x: (k as f32) * 1.5,
                    y: -(k as f32),
                };
                s.verts[k].sB = c2v {
                    x: -(k as f32) * 0.5,
                    y: (k as f32) * 2.0,
                };
            }
            let mut cs = s;
            let mut rs = s;
            let (mut ca, mut cb) = (poison, poison);
            let (mut ra, mut rb) = (poison, poison);
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            let ctx = format!("c2Witness special count={count} div={d:?}");
            eq_raw(&format!("{ctx} a"), &ca, &ra);
            eq_raw(&format!("{ctx} b"), &cb, &rb);
        }
    }
}
