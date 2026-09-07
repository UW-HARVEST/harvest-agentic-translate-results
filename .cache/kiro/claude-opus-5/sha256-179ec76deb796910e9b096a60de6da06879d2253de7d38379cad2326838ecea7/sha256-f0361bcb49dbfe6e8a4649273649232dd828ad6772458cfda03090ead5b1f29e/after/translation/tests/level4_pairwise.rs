//! Phase B — CONFIGS.md rows 76-84: the pairwise boolean predicates.
//!
//! Rows 79-82 target the three branches of `c2CircletoCapsule` individually
//! (`da < 0`, `da>=0 && db<0`, `da>=0 && db>=0`) and assert coverage, because
//! the middle branch is the only place in the library that divides by
//! `c2Dot(n,n)` — zero for a degenerate capsule.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 40_000;

/// Row 76 — `c2AABBtoAABB` across overlap, touch, all four separation axes,
/// inverted boxes and NaN.
#[test]
fn row76_aabb_to_aabb() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 76);
    for i in 0..N {
        let a = g.aabb();
        let b = g.aabb();
        eq_int(
            &format!("row76 random #{i} {a:?} {b:?}"),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
    // Deliberate coverage of each of the four separating-axis tests.
    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for &(dx, dy) in &[
        (3.0f32, 0.0f32), // A.max.x < B.min.x
        (-3.0, 0.0),      // B.max.x < A.min.x
        (0.0, 3.0),       // A.max.y < B.min.y
        (0.0, -3.0),      // B.max.y < A.min.y
        (2.0, 0.0),       // exactly touching on x
        (-2.0, 0.0),
        (0.0, 2.0),
        (0.0, -2.0),
        (1.9999, 0.0), // barely overlapping
        (2.0001, 0.0), // barely apart
        (0.0, 0.0),    // coincident
    ] {
        let b = c2AABB {
            min: c2v {
                x: -1.0 + dx,
                y: -1.0 + dy,
            },
            max: c2v {
                x: 1.0 + dx,
                y: 1.0 + dy,
            },
        };
        eq_int(
            &format!("row76 axis dx={dx} dy={dy}"),
            (c.c2AABBtoAABB)(unit, b),
            (r.c2AABBtoAABB)(unit, b),
        );
        eq_int(
            &format!("row76 axis rev dx={dx} dy={dy}"),
            (c.c2AABBtoAABB)(b, unit),
            (r.c2AABBtoAABB)(b, unit),
        );
    }
    // Wild boxes.
    for i in 0..N {
        let a = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let b = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        eq_int(
            &format!("row76 wild #{i}"),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
}

/// Row 77 — `c2CircletoCircle`, including the exactly-touching case (the C uses
/// a strict `<`, so touching is a miss) and negative radii.
#[test]
fn row77_circle_to_circle() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 77);
    for i in 0..N {
        let a = g.circle();
        let b = g.circle();
        eq_int(
            &format!("row77 random #{i} {a:?} {b:?}"),
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
        );
    }
    // Exact-touch sweep: d == rA + rB must be a MISS (strict `<`).
    for k in 1..200i32 {
        let ra = k as f32 * 0.25;
        let rb = 1.0 + k as f32 * 0.125;
        for &scale in &[0.999_9f32, 1.0, 1.000_1] {
            let a = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: ra,
            };
            let b = c2Circle {
                p: c2v {
                    x: (ra + rb) * scale,
                    y: 0.0,
                },
                r: rb,
            };
            eq_int(
                &format!("row77 touch k={k} scale={scale}"),
                (c.c2CircletoCircle)(a, b),
                (r.c2CircletoCircle)(a, b),
            );
        }
    }
    for i in 0..N {
        let a = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let b = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        eq_int(
            &format!("row77 wild #{i}"),
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
        );
    }
}

/// Row 78 — `c2CircletoAABB`: centre inside, on an edge, on a corner, outside;
/// zero radius; inverted AABB (where `c2Clampv` produces a nonsense point).
#[test]
fn row78_circle_to_aabb() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 78);
    for i in 0..N {
        let a = g.circle();
        let b = g.aabb();
        eq_int(
            &format!("row78 random #{i} {a:?} {b:?}"),
            (c.c2CircletoAABB)(a, b),
            (r.c2CircletoAABB)(a, b),
        );
    }
    let bb = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    // Grid sweep: inside, edges, corners, just outside.
    for ix in -6..=6i32 {
        for iy in -6..=6i32 {
            for &rad in &[0.0f32, 1e-7, 0.25, 0.5, 1.0, 2.0] {
                let a = c2Circle {
                    p: c2v {
                        x: ix as f32 * 0.5,
                        y: iy as f32 * 0.5,
                    },
                    r: rad,
                };
                eq_int(
                    &format!("row78 grid {ix},{iy} r={rad}"),
                    (c.c2CircletoAABB)(a, bb),
                    (r.c2CircletoAABB)(a, bb),
                );
            }
        }
    }
    // Inverted / degenerate AABBs.
    for bad in [
        c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: -1.0, y: -1.0 },
        },
        c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
    ] {
        for i in 0..2000 {
            let a = g.circle();
            eq_int(
                &format!("row78 inverted #{i}"),
                (c.c2CircletoAABB)(a, bad),
                (r.c2CircletoAABB)(a, bad),
            );
        }
    }
    for i in 0..N {
        let a = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let b = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        eq_int(
            &format!("row78 wild #{i}"),
            (c.c2CircletoAABB)(a, b),
            (r.c2CircletoAABB)(a, b),
        );
    }
}

/// Rows 79-82 — `c2CircletoCapsule`, with explicit coverage of all three
/// branches plus the degenerate `a == b` capsule (`n == 0` → `da/0` → NaN).
#[test]
fn rows79_82_circle_to_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 79);
    // branch: 0 = da<0, 1 = da>=0 && db<0, 2 = da>=0 && db>=0
    let mut hits = [0usize; 3];
    let classify = |a: c2Circle, b: c2Capsule| -> usize {
        let n = c2v {
            x: b.b.x - b.a.x,
            y: b.b.y - b.a.y,
        };
        let ap = c2v {
            x: a.p.x - b.a.x,
            y: a.p.y - b.a.y,
        };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            return 0;
        }
        let bp = c2v {
            x: a.p.x - b.b.x,
            y: a.p.y - b.b.y,
        };
        let db = bp.x * n.x + bp.y * n.y;
        if db < 0.0 { 1 } else { 2 }
    };

    for i in 0..N {
        let a = g.circle();
        let b = g.capsule();
        eq_int(
            &format!("row79-82 random #{i} {a:?} {b:?}"),
            (c.c2CircletoCapsule)(a, b),
            (r.c2CircletoCapsule)(a, b),
        );
        hits[classify(a, b)] += 1;
    }

    // Deliberate branch placement along a horizontal capsule from (0,0) to (4,0).
    let cap = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 4.0, y: 0.0 },
        r: 0.5,
    };
    for ix in -8..=24i32 {
        for iy in -4..=4i32 {
            for &rad in &[0.0f32, 0.25, 0.5, 1.0] {
                let circ = c2Circle {
                    p: c2v {
                        x: ix as f32 * 0.5,
                        y: iy as f32 * 0.5,
                    },
                    r: rad,
                };
                eq_int(
                    &format!("row79-82 grid {ix},{iy} r={rad}"),
                    (c.c2CircletoCapsule)(circ, cap),
                    (r.c2CircletoCapsule)(circ, cap),
                );
                hits[classify(circ, cap)] += 1;
            }
        }
    }
    for (b, &n) in hits.iter().enumerate() {
        assert!(
            n > 0,
            "c2CircletoCapsule branch {b} never exercised (hits: {hits:?})"
        );
    }
    println!("c2CircletoCapsule branch hits: {hits:?}");

    // Row 82: degenerate capsule (a == b) => n == (0,0) => da == 0 (not < 0)
    // => db == 0 (not < 0) => the third branch. And with a.p == b.a the
    // division `da / c2Dot(n,n)` would be 0/0; verify both agree.
    for i in 0..5000 {
        let p = g.v();
        let deg = c2Capsule {
            a: p,
            b: p,
            r: g.radius(),
        };
        let circ = g.circle();
        eq_int(
            &format!("row82 degenerate #{i}"),
            (c.c2CircletoCapsule)(circ, deg),
            (r.c2CircletoCapsule)(circ, deg),
        );
        // circle centred exactly on the degenerate capsule
        let on = c2Circle { p, r: g.radius() };
        eq_int(
            &format!("row82 coincident #{i}"),
            (c.c2CircletoCapsule)(on, deg),
            (r.c2CircletoCapsule)(on, deg),
        );
    }

    for i in 0..N {
        let a = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let b = c2Capsule {
            a: g.wild_v(),
            b: g.wild_v(),
            r: g.wild(),
        };
        eq_int(
            &format!("row79-82 wild #{i}"),
            (c.c2CircletoCapsule)(a, b),
            (r.c2CircletoCapsule)(a, b),
        );
    }
}

/// Row 83 — `c2AABBtoCapsule`. Goes through `c2GJK` with `use_radius = 1` and
/// all-null optional parameters, so it also covers ERRORS.md rows 9-14 in
/// composition.
#[test]
fn row83_aabb_to_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 83);
    for i in 0..N / 2 {
        let a = g.aabb();
        let b = g.capsule();
        eq_int(
            &format!("row83 random #{i} {a:?} {b:?}"),
            (c.c2AABBtoCapsule)(a, b),
            (r.c2AABBtoCapsule)(a, b),
        );
    }
    // Structured sweep: capsule marching past a unit box.
    let bb = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for ix in -8..=8i32 {
        for iy in -8..=8i32 {
            for &rad in &[0.0f32, 0.5, 1.0] {
                for len in [0.0f32, 1.0, 4.0] {
                    let cap = c2Capsule {
                        a: c2v {
                            x: ix as f32 * 0.5,
                            y: iy as f32 * 0.5,
                        },
                        b: c2v {
                            x: ix as f32 * 0.5 + len,
                            y: iy as f32 * 0.5 + len * 0.5,
                        },
                        r: rad,
                    };
                    eq_int(
                        &format!("row83 grid {ix},{iy} r={rad} len={len}"),
                        (c.c2AABBtoCapsule)(bb, cap),
                        (r.c2AABBtoCapsule)(bb, cap),
                    );
                }
            }
        }
    }
    // Degenerate and inverted.
    for i in 0..5000 {
        let inv = c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: -1.0, y: -1.0 },
        };
        let b = g.capsule();
        eq_int(
            &format!("row83 inverted #{i}"),
            (c.c2AABBtoCapsule)(inv, b),
            (r.c2AABBtoCapsule)(inv, b),
        );
    }
    for i in 0..N / 4 {
        let a = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let b = c2Capsule {
            a: g.wild_v(),
            b: g.wild_v(),
            r: g.wild(),
        };
        eq_int(
            &format!("row83 wild #{i}"),
            (c.c2AABBtoCapsule)(a, b),
            (r.c2AABBtoCapsule)(a, b),
        );
    }
}

/// Row 84 — `c2CapsuletoCapsule`: crossing, parallel, collinear, coincident and
/// degenerate (point) capsules.
#[test]
fn row84_capsule_to_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 84);
    for i in 0..N / 2 {
        let a = g.capsule();
        let b = g.capsule();
        eq_int(
            &format!("row84 random #{i} {a:?} {b:?}"),
            (c.c2CapsuletoCapsule)(a, b),
            (r.c2CapsuletoCapsule)(a, b),
        );
    }
    let base = c2Capsule {
        a: c2v { x: -2.0, y: 0.0 },
        b: c2v { x: 2.0, y: 0.0 },
        r: 0.5,
    };
    let structured = [
        // crossing at right angles
        c2Capsule {
            a: c2v { x: 0.0, y: -2.0 },
            b: c2v { x: 0.0, y: 2.0 },
            r: 0.5,
        },
        // parallel, just touching
        c2Capsule {
            a: c2v { x: -2.0, y: 1.0 },
            b: c2v { x: 2.0, y: 1.0 },
            r: 0.5,
        },
        // parallel, just apart
        c2Capsule {
            a: c2v { x: -2.0, y: 1.001 },
            b: c2v { x: 2.0, y: 1.001 },
            r: 0.5,
        },
        // collinear, overlapping
        c2Capsule {
            a: c2v { x: 1.0, y: 0.0 },
            b: c2v { x: 5.0, y: 0.0 },
            r: 0.5,
        },
        // collinear, apart
        c2Capsule {
            a: c2v { x: 3.5, y: 0.0 },
            b: c2v { x: 7.0, y: 0.0 },
            r: 0.5,
        },
        // coincident
        base,
        // degenerate point capsules
        c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        },
        c2Capsule {
            a: c2v { x: 0.0, y: 0.6 },
            b: c2v { x: 0.0, y: 0.6 },
            r: 0.2,
        },
        c2Capsule {
            a: c2v { x: 10.0, y: 10.0 },
            b: c2v { x: 10.0, y: 10.0 },
            r: 0.0,
        },
    ];
    for (i, s) in structured.iter().enumerate() {
        eq_int(
            &format!("row84 structured #{i}"),
            (c.c2CapsuletoCapsule)(base, *s),
            (r.c2CapsuletoCapsule)(base, *s),
        );
        eq_int(
            &format!("row84 structured rev #{i}"),
            (c.c2CapsuletoCapsule)(*s, base),
            (r.c2CapsuletoCapsule)(*s, base),
        );
        for (j, t) in structured.iter().enumerate() {
            eq_int(
                &format!("row84 cross #{i}x{j}"),
                (c.c2CapsuletoCapsule)(*s, *t),
                (r.c2CapsuletoCapsule)(*s, *t),
            );
        }
    }
    // Fine separation sweep across the touch threshold.
    for k in 0..400i32 {
        let d = 0.9 + k as f32 * 0.0005;
        let other = c2Capsule {
            a: c2v { x: -2.0, y: d },
            b: c2v { x: 2.0, y: d },
            r: 0.5,
        };
        eq_int(
            &format!("row84 threshold d={d}"),
            (c.c2CapsuletoCapsule)(base, other),
            (r.c2CapsuletoCapsule)(base, other),
        );
    }
    for i in 0..N / 4 {
        let a = c2Capsule {
            a: g.wild_v(),
            b: g.wild_v(),
            r: g.wild(),
        };
        let b = c2Capsule {
            a: g.wild_v(),
            b: g.wild_v(),
            r: g.wild(),
        };
        eq_int(
            &format!("row84 wild #{i}"),
            (c.c2CapsuletoCapsule)(a, b),
            (r.c2CapsuletoCapsule)(a, b),
        );
    }
}
