//! Phase B, level 4 — boolean shape-vs-shape tests. CONFIGS.md rows 47..=52.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 6000;

/// Generate an AABB pair in one of the interesting relationships.
fn aabb_pair(rng: &mut Rng) -> (C2AABB, C2AABB) {
    fn mk(x: f32, y: f32, w: f32, h: f32) -> C2AABB {
        C2AABB {
            min: C2v { x, y },
            max: C2v { x: x + w, y: y + h },
        }
    }
    match rng.below(9) {
        // disjoint on x
        0 => {
            let dx = 2.0 + rng.unit();
            (mk(0.0, 0.0, 1.0, 1.0), mk(dx, 0.0, 1.0, 1.0))
        }
        // exactly touching on x (B.min.x == A.max.x): `<` is strict, so touching collides
        1 => (mk(0.0, 0.0, 1.0, 1.0), mk(1.0, 0.0, 1.0, 1.0)),
        // exactly touching on y
        2 => (mk(0.0, 0.0, 1.0, 1.0), mk(0.0, 1.0, 1.0, 1.0)),
        // overlapping
        3 => {
            let (ox, oy) = (rng.sym(1.5), rng.sym(1.5));
            (mk(0.0, 0.0, 2.0, 2.0), mk(ox, oy, 2.0, 2.0))
        }
        // nested
        4 => {
            let (ox, oy) = (rng.sym(1.0), rng.sym(1.0));
            (mk(-5.0, -5.0, 10.0, 10.0), mk(ox, oy, 1.0, 1.0))
        }
        // identical
        5 => {
            let a = aabb(rng, 5.0);
            (a, a)
        }
        // inverted
        6 => {
            let a = C2AABB { min: C2v { x: 1.0, y: 1.0 }, max: C2v { x: -1.0, y: -1.0 } };
            (a, aabb(rng, 5.0))
        }
        // degenerate point boxes
        7 => {
            let p = C2v { x: rng.grid(), y: rng.grid() };
            let q = C2v { x: rng.grid(), y: rng.grid() };
            (C2AABB { min: p, max: p }, C2AABB { min: q, max: q })
        }
        // anything, incl. NaN
        _ => (
            C2AABB { min: vec_mixed(rng, 5.0), max: vec_mixed(rng, 5.0) },
            C2AABB { min: vec_mixed(rng, 5.0), max: vec_mixed(rng, 5.0) },
        ),
    }
}

#[test]
fn row47_c2AABBtoAABB() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnAABBtoAABB>("c2AABBtoAABB") };
    let mut rng = Rng::new(0x4001);
    let mut d = Diff::new("47: c2AABBtoAABB");
    let mut yes = 0;
    let mut no = 0;
    for _ in 0..N * 4 {
        let (A, B) = aabb_pair(&mut rng);
        let (cv, rv) = unsafe { (c(A, B), r(A, B)) };
        d.check((A, B), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    d.finish();
    assert!(yes > 0 && no > 0, "one-sided results: yes={yes} no={no}");
}

#[test]
fn row48_c2CircletoCircle() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCircletoCircle>("c2CircletoCircle") };
    let mut rng = Rng::new(0x4002);
    let mut d = Diff::new("48: c2CircletoCircle");
    let mut yes = 0;
    let mut no = 0;
    for _ in 0..N * 4 {
        let (A, B) = match rng.below(6) {
            // exactly tangent: d2 == r2, and `<` is strict -> must be 0
            0 => {
                let rA = (rng.below(6) + 1) as f32;
                let rB = (rng.below(6) + 1) as f32;
                (
                    C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: rA },
                    C2Circle { p: C2v { x: rA + rB, y: 0.0 }, r: rB },
                )
            }
            // zero radii
            1 => (
                C2Circle { p: vec_tame(&mut rng, 3.0), r: 0.0 },
                C2Circle { p: vec_tame(&mut rng, 3.0), r: 0.0 },
            ),
            // negative radii
            2 => (
                C2Circle { p: vec_tame(&mut rng, 3.0), r: -rng.unit() * 3.0 },
                C2Circle { p: vec_tame(&mut rng, 3.0), r: -rng.unit() * 3.0 },
            ),
            // coincident centres
            3 => {
                let p = vec_tame(&mut rng, 3.0);
                (
                    C2Circle { p, r: rng.unit() * 3.0 },
                    C2Circle { p, r: rng.unit() * 3.0 },
                )
            }
            // wild values incl. NaN / inf
            4 => (
                C2Circle { p: vec_mixed(&mut rng, 3.0), r: mixed(&mut rng, 3.0) },
                C2Circle { p: vec_mixed(&mut rng, 3.0), r: mixed(&mut rng, 3.0) },
            ),
            _ => (circle(&mut rng, 3.0), circle(&mut rng, 3.0)),
        };
        let (cv, rv) = unsafe { (c(A, B), r(A, B)) };
        d.check((A, B), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    d.finish();
    assert!(yes > 0 && no > 0, "one-sided results: yes={yes} no={no}");
}

#[test]
fn row49_c2CircletoAABB() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCircletoAABB>("c2CircletoAABB") };
    let mut rng = Rng::new(0x4003);
    let mut d = Diff::new("49: c2CircletoAABB");
    let mut yes = 0;
    let mut no = 0;
    let unit = C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } };
    for _ in 0..N * 4 {
        let (A, B) = match rng.below(8) {
            // centre strictly inside
            0 => (C2Circle { p: vec_tame(&mut rng, 0.9), r: rng.unit() }, unit),
            // centre exactly on an edge
            1 => {
                let t = rng.sym(1.0);
                let p = [
                    C2v { x: -1.0, y: t },
                    C2v { x: 1.0, y: t },
                    C2v { x: t, y: -1.0 },
                    C2v { x: t, y: 1.0 },
                ][rng.below(4)];
                (C2Circle { p, r: rng.unit() }, unit)
            }
            // centre exactly on a corner
            2 => {
                let p = [
                    C2v { x: -1.0, y: -1.0 },
                    C2v { x: 1.0, y: -1.0 },
                    C2v { x: 1.0, y: 1.0 },
                    C2v { x: -1.0, y: 1.0 },
                ][rng.below(4)];
                (C2Circle { p, r: rng.unit() }, unit)
            }
            // exactly tangent to an edge: d2 == r2 -> 0
            3 => (
                C2Circle { p: C2v { x: 1.0 + 2.0, y: 0.0 }, r: 2.0 },
                unit,
            ),
            // zero radius
            4 => (C2Circle { p: vec_tame(&mut rng, 3.0), r: 0.0 }, unit),
            // inverted AABB
            5 => (
                circle(&mut rng, 3.0),
                C2AABB { min: C2v { x: 1.0, y: 1.0 }, max: C2v { x: -1.0, y: -1.0 } },
            ),
            // wild
            6 => (
                C2Circle { p: vec_mixed(&mut rng, 3.0), r: mixed(&mut rng, 3.0) },
                C2AABB { min: vec_mixed(&mut rng, 3.0), max: vec_mixed(&mut rng, 3.0) },
            ),
            _ => (circle(&mut rng, 3.0), aabb(&mut rng, 3.0)),
        };
        let (cv, rv) = unsafe { (c(A, B), r(A, B)) };
        d.check((A, B), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    d.finish();
    assert!(yes > 0 && no > 0, "one-sided results: yes={yes} no={no}");
}

#[test]
fn row50_c2CircletoCapsule_all_branches() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCircletoCapsule>("c2CircletoCapsule") };
    let mut rng = Rng::new(0x4004);
    let mut d = Diff::new("50: c2CircletoCapsule");
    // branch counters: da<0, db<0 (perpendicular), else (b end)
    let mut br = [0usize; 3];
    let mut yes = 0;
    let mut no = 0;
    for _ in 0..N * 4 {
        let (A, B) = match rng.below(7) {
            // horizontal capsule, circle placed before `a` (da < 0)
            0 => (
                C2Circle { p: C2v { x: -1.0 - rng.unit() * 2.0, y: rng.sym(1.0) }, r: rng.unit() * 2.0 },
                C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 4.0, y: 0.0 }, r: rng.unit() },
            ),
            // circle beside the middle (db < 0 -> perpendicular projection)
            1 => (
                C2Circle { p: C2v { x: 2.0 + rng.sym(1.0), y: rng.sym(2.0) }, r: rng.unit() * 2.0 },
                C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 4.0, y: 0.0 }, r: rng.unit() },
            ),
            // circle past `b` (else branch)
            2 => (
                C2Circle { p: C2v { x: 5.0 + rng.unit() * 2.0, y: rng.sym(1.0) }, r: rng.unit() * 2.0 },
                C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 4.0, y: 0.0 }, r: rng.unit() },
            ),
            // degenerate capsule a == b (n == (0,0), avoids the 0/0 divide)
            3 => {
                let p = vec_tame(&mut rng, 3.0);
                (circle(&mut rng, 3.0), C2Capsule { a: p, b: p, r: rng.unit() * 2.0 })
            }
            // exactly at the endpoints (da == 0 / db == 0)
            4 => (
                C2Circle { p: [C2v { x: 0.0, y: 0.0 }, C2v { x: 4.0, y: 0.0 }][rng.below(2)], r: rng.unit() },
                C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 4.0, y: 0.0 }, r: rng.unit() },
            ),
            // wild / NaN
            5 => (
                C2Circle { p: vec_mixed(&mut rng, 3.0), r: mixed(&mut rng, 3.0) },
                C2Capsule { a: vec_mixed(&mut rng, 3.0), b: vec_mixed(&mut rng, 3.0), r: mixed(&mut rng, 3.0) },
            ),
            _ => (circle(&mut rng, 3.0), capsule(&mut rng, 3.0)),
        };
        let (cv, rv) = unsafe { (c(A, B), r(A, B)) };
        d.check((A, B), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
        // classify the branch the C would take
        let n = C2v { x: B.b.x - B.a.x, y: B.b.y - B.a.y };
        let ap = C2v { x: A.p.x - B.a.x, y: A.p.y - B.a.y };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            br[0] += 1;
        } else {
            let db = (A.p.x - B.b.x) * n.x + (A.p.y - B.b.y) * n.y;
            if db < 0.0 { br[1] += 1 } else { br[2] += 1 }
        }
    }
    d.finish();
    assert!(br.iter().all(|&x| x > 0), "branch coverage gap: {br:?}");
    assert!(yes > 0 && no > 0, "one-sided results: yes={yes} no={no}");
}

#[test]
fn row51_c2CapsuletoCapsule() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule") };
    let mut rng = Rng::new(0x4005);
    let mut d = Diff::new("51: c2CapsuletoCapsule");
    let mut yes = 0;
    let mut no = 0;
    for _ in 0..N * 2 {
        let (A, B) = match rng.below(7) {
            // parallel
            0 => {
                let off = rng.sym(3.0);
                (
                    C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 4.0, y: 0.0 }, r: rng.unit() },
                    C2Capsule { a: C2v { x: 0.0, y: off }, b: C2v { x: 4.0, y: off }, r: rng.unit() },
                )
            }
            // crossing
            1 => (
                C2Capsule { a: C2v { x: -2.0, y: 0.0 }, b: C2v { x: 2.0, y: 0.0 }, r: rng.unit() * 0.5 },
                C2Capsule { a: C2v { x: rng.sym(3.0), y: -2.0 }, b: C2v { x: rng.sym(3.0), y: 2.0 }, r: rng.unit() * 0.5 },
            ),
            // collinear (overlapping or not)
            2 => (
                C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 2.0, y: 0.0 }, r: rng.unit() * 0.5 },
                C2Capsule { a: C2v { x: rng.grid(), y: 0.0 }, b: C2v { x: rng.grid() + 2.0, y: 0.0 }, r: rng.unit() * 0.5 },
            ),
            // far apart
            3 => (
                C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 1.0, y: 0.0 }, r: 0.25 },
                C2Capsule { a: C2v { x: 100.0, y: 100.0 }, b: C2v { x: 101.0, y: 100.0 }, r: 0.25 },
            ),
            // both degenerate (points)
            4 => {
                let p = vec_tame(&mut rng, 3.0);
                let q = vec_tame(&mut rng, 3.0);
                (
                    C2Capsule { a: p, b: p, r: rng.unit() },
                    C2Capsule { a: q, b: q, r: rng.unit() },
                )
            }
            // negative radii
            5 => (
                C2Capsule { a: vec_tame(&mut rng, 3.0), b: vec_tame(&mut rng, 3.0), r: -rng.unit() * 2.0 },
                C2Capsule { a: vec_tame(&mut rng, 3.0), b: vec_tame(&mut rng, 3.0), r: -rng.unit() * 2.0 },
            ),
            _ => (capsule(&mut rng, 3.0), capsule(&mut rng, 3.0)),
        };
        let (cv, rv) = unsafe { (c(A, B), r(A, B)) };
        d.check((A, B), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    d.finish();
    assert!(yes > 0 && no > 0, "one-sided results: yes={yes} no={no}");
}

#[test]
fn row52_c2AABBtoCapsule() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule") };
    let mut rng = Rng::new(0x4006);
    let mut d = Diff::new("52: c2AABBtoCapsule");
    let mut yes = 0;
    let mut no = 0;
    let unit = C2AABB { min: C2v { x: -2.0, y: -2.0 }, max: C2v { x: 2.0, y: 2.0 } };
    for _ in 0..N * 2 {
        let (A, B) = match rng.below(7) {
            // capsule fully inside
            0 => (unit, C2Capsule { a: vec_tame(&mut rng, 1.0), b: vec_tame(&mut rng, 1.0), r: rng.unit() * 0.5 }),
            // capsule crossing an edge
            1 => (unit, C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 4.0, y: 0.0 }, r: rng.unit() * 0.5 }),
            // capsule crossing a corner diagonally
            2 => (unit, C2Capsule { a: C2v { x: 1.0, y: 1.0 }, b: C2v { x: 5.0, y: 5.0 }, r: rng.unit() * 0.5 }),
            // disjoint
            3 => (unit, C2Capsule { a: C2v { x: 10.0, y: 10.0 }, b: C2v { x: 12.0, y: 12.0 }, r: 0.5 }),
            // exactly tangent to the right edge
            4 => (unit, C2Capsule { a: C2v { x: 3.0, y: -1.0 }, b: C2v { x: 3.0, y: 1.0 }, r: 1.0 }),
            // degenerate: point AABB and/or point capsule
            5 => {
                let p = vec_tame(&mut rng, 3.0);
                let q = vec_tame(&mut rng, 3.0);
                (C2AABB { min: p, max: p }, C2Capsule { a: q, b: q, r: rng.unit() })
            }
            _ => (aabb(&mut rng, 3.0), capsule(&mut rng, 3.0)),
        };
        let (cv, rv) = unsafe { (c(A, B), r(A, B)) };
        d.check((A, B), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    d.finish();
    assert!(yes > 0 && no > 0, "one-sided results: yes={yes} no={no}");
}
