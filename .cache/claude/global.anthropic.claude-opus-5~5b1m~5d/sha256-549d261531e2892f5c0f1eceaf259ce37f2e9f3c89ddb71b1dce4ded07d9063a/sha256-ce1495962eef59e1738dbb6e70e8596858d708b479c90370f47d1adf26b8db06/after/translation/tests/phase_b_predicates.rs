//! Phase B — valid-path differential tests for the overlap predicates.
//! Covers `CONFIGS.md` rows 19–23.
//!
//! These are the lowest-level *branching* entry points: every one of them is a
//! disjunction of four (or one) float comparisons, so the tests deliberately
//! generate inputs that land exactly on each boundary as well as randomly.

#![allow(non_snake_case)]

mod common;

use common::*;

const N: usize = 4096;

/// Boxes engineered so each separating axis is hit, plus exact-touch cases.
fn boundary_box_pairs() -> Vec<(c2AABB, c2AABB)> {
    let mk = |a: f32, b: f32, c: f32, d: f32| c2AABB {
        min: c2v { x: a, y: b },
        max: c2v { x: c, y: d },
    };
    let a = mk(0.0, 0.0, 2.0, 2.0);
    vec![
        // overlapping
        (a, mk(1.0, 1.0, 3.0, 3.0)),
        // A contains B / B contains A
        (a, mk(0.5, 0.5, 1.5, 1.5)),
        (a, mk(-1.0, -1.0, 3.0, 3.0)),
        // identical
        (a, a),
        // exact edge touch on each of the 4 axes
        (a, mk(2.0, 0.0, 4.0, 2.0)),  // B.min.x == A.max.x
        (a, mk(-2.0, 0.0, 0.0, 2.0)), // B.max.x == A.min.x
        (a, mk(0.0, 2.0, 2.0, 4.0)),  // B.min.y == A.max.y
        (a, mk(0.0, -2.0, 2.0, 0.0)), // B.max.y == A.min.y
        // one float past the touch → separated on each axis (d0..d3)
        (a, mk(-4.0, 0.0, -0.000001, 2.0)), // d0: B.max.x < A.min.x
        (a, mk(2.000001, 0.0, 4.0, 2.0)),   // d1: A.max.x < B.min.x
        (a, mk(0.0, -4.0, 2.0, -0.000001)), // d2: B.max.y < A.min.y
        (a, mk(0.0, 2.000001, 2.0, 4.0)),   // d3: A.max.y < B.min.y
        // degenerate (min == max)
        (mk(1.0, 1.0, 1.0, 1.0), a),
        (a, mk(1.0, 1.0, 1.0, 1.0)),
        (mk(1.0, 1.0, 1.0, 1.0), mk(1.0, 1.0, 1.0, 1.0)),
        // inverted (min > max) — no validation in the C
        (mk(2.0, 2.0, 0.0, 0.0), a),
        (a, mk(2.0, 2.0, 0.0, 0.0)),
        (mk(2.0, 2.0, 0.0, 0.0), mk(3.0, 3.0, 1.0, 1.0)),
        // signed zeros
        (mk(-0.0, -0.0, 0.0, 0.0), mk(0.0, 0.0, -0.0, -0.0)),
    ]
}

// ---------------------------------------------------------------------------
// Rows 19, 20 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn row19_c2AABBtoAABB_proper() {
    let l = libs();
    for (i, (a, b)) in boundary_box_pairs().into_iter().enumerate() {
        diff_eq!(
            format!("row19 boundary #{i} A={:?} B={:?}", a, b),
            (l.c.c2AABBtoAABB)(a, b),
            (l.rs.c2AABBtoAABB)(a, b)
        );
        // ...and the (deliberately non-symmetric) swapped call.
        diff_eq!(
            format!("row19 boundary swapped #{i}"),
            (l.c.c2AABBtoAABB)(b, a),
            (l.rs.c2AABBtoAABB)(b, a)
        );
    }

    let mut rng = Rng::new(SEED ^ 19);
    // Gridded coordinates make exact ties (the `<` boundaries) common.
    for i in 0..(N * 4) {
        let a0 = rng.vec_grid(6);
        let a1 = c2v {
            x: a0.x + rng.gridded(4).abs(),
            y: a0.y + rng.gridded(4).abs(),
        };
        let b0 = rng.vec_grid(6);
        let b1 = c2v {
            x: b0.x + rng.gridded(4).abs(),
            y: b0.y + rng.gridded(4).abs(),
        };
        let a = c2AABB { min: a0, max: a1 };
        let b = c2AABB { min: b0, max: b1 };
        diff_eq!(
            format!("row19 rand #{i} A={:?} B={:?}", a, b),
            (l.c.c2AABBtoAABB)(a, b),
            (l.rs.c2AABBtoAABB)(a, b)
        );
    }
}

#[test]
fn row20_c2AABBtoAABB_degenerate_and_nan() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..(N * 4) {
        let a = any_aabb(&mut rng);
        let b = any_aabb(&mut rng);
        diff_eq!(
            format!("row20 rand #{i} A={:?} B={:?}", a, b),
            (l.c.c2AABBtoAABB)(a, b),
            (l.rs.c2AABBtoAABB)(a, b)
        );
    }
    // NaN in every single coordinate position, one at a time.
    let base = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 2.0, y: 2.0 },
    };
    let other = c2AABB {
        min: c2v { x: 1.0, y: 1.0 },
        max: c2v { x: 3.0, y: 3.0 },
    };
    for &nanbits in NANS {
        let nan = f32::from_bits(nanbits);
        for slot in 0..8 {
            let mut a = base;
            let mut b = other;
            match slot {
                0 => a.min.x = nan,
                1 => a.min.y = nan,
                2 => a.max.x = nan,
                3 => a.max.y = nan,
                4 => b.min.x = nan,
                5 => b.min.y = nan,
                6 => b.max.x = nan,
                _ => b.max.y = nan,
            }
            diff_eq!(
                format!("row20 nan slot{slot} bits={nanbits:#010x}"),
                (l.c.c2AABBtoAABB)(a, b),
                (l.rs.c2AABBtoAABB)(a, b)
            );
        }
    }
    // Infinities in every position.
    for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
        for slot in 0..8 {
            let mut a = base;
            let mut b = other;
            match slot {
                0 => a.min.x = inf,
                1 => a.min.y = inf,
                2 => a.max.x = inf,
                3 => a.max.y = inf,
                4 => b.min.x = inf,
                5 => b.min.y = inf,
                6 => b.max.x = inf,
                _ => b.max.y = inf,
            }
            diff_eq!(
                format!("row20 inf slot{slot} {}", show(inf)),
                (l.c.c2AABBtoAABB)(a, b),
                (l.rs.c2AABBtoAABB)(a, b)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 21, 22 — c2AABBtoPoint
// ---------------------------------------------------------------------------

#[test]
fn row21_c2AABBtoPoint_proper() {
    let l = libs();
    let box_ = c2AABB {
        min: c2v { x: -1.0, y: -2.0 },
        max: c2v { x: 3.0, y: 4.0 },
    };
    // Inside, exactly on each edge, each corner, and one step outside each side.
    let pts = [
        c2v { x: 1.0, y: 1.0 },   // inside
        c2v { x: -1.0, y: 1.0 },  // on min.x edge
        c2v { x: 3.0, y: 1.0 },   // on max.x edge
        c2v { x: 1.0, y: -2.0 },  // on min.y edge
        c2v { x: 1.0, y: 4.0 },   // on max.y edge
        c2v { x: -1.0, y: -2.0 }, // corner
        c2v { x: 3.0, y: 4.0 },   // corner
        c2v { x: -1.0, y: 4.0 },  // corner
        c2v { x: 3.0, y: -2.0 },  // corner
        c2v {
            x: -1.000001,
            y: 1.0,
        }, // d0
        c2v {
            x: 1.0,
            y: -2.000001,
        }, // d1
        c2v {
            x: 3.000001,
            y: 1.0,
        }, // d2
        c2v {
            x: 1.0,
            y: 4.000001,
        }, // d3
    ];
    for (i, &p) in pts.iter().enumerate() {
        diff_eq!(
            format!("row21 boundary #{i} p={}", showv(p)),
            (l.c.c2AABBtoPoint)(box_, p),
            (l.rs.c2AABBtoPoint)(box_, p)
        );
    }
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..(N * 4) {
        let a0 = rng.vec_grid(6);
        let a1 = c2v {
            x: a0.x + rng.gridded(4).abs(),
            y: a0.y + rng.gridded(4).abs(),
        };
        let a = c2AABB { min: a0, max: a1 };
        let p = rng.vec_grid(8);
        diff_eq!(
            format!("row21 rand #{i} A={:?} p={}", a, showv(p)),
            (l.c.c2AABBtoPoint)(a, p),
            (l.rs.c2AABBtoPoint)(a, p)
        );
    }
}

#[test]
fn row22_c2AABBtoPoint_degenerate_and_nan() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..(N * 4) {
        let a = any_aabb(&mut rng);
        let p = rng.any_vec();
        diff_eq!(
            format!("row22 rand #{i} A={:?} p={}", a, showv(p)),
            (l.c.c2AABBtoPoint)(a, p),
            (l.rs.c2AABBtoPoint)(a, p)
        );
    }
    let sp = special_wide();
    let a = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for &x in &sp {
        for &y in &sp {
            let p = c2v { x, y };
            diff_eq!(
                format!("row22 sp point p={}", showv(p)),
                (l.c.c2AABBtoPoint)(a, p),
                (l.rs.c2AABBtoPoint)(a, p)
            );
        }
    }
    // Special values in the box itself.
    let p = c2v { x: 0.0, y: 0.0 };
    for &v in &sp {
        for slot in 0..4 {
            let mut b = a;
            match slot {
                0 => b.min.x = v,
                1 => b.min.y = v,
                2 => b.max.x = v,
                _ => b.max.y = v,
            }
            diff_eq!(
                format!("row22 sp box slot{slot} {}", show(v)),
                (l.c.c2AABBtoPoint)(b, p),
                (l.rs.c2AABBtoPoint)(b, p)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 — c2CircleToPoint
// ---------------------------------------------------------------------------

#[test]
fn row23_c2CircleToPoint() {
    let l = libs();

    // Exactly-on-the-rim cases: the C uses a strict `<`, so these must MISS.
    // Pythagorean triples give exact float arithmetic.
    let exact = [
        (c2v { x: 0.0, y: 0.0 }, 5.0f32, c2v { x: 3.0, y: 4.0 }),
        (c2v { x: 0.0, y: 0.0 }, 5.0f32, c2v { x: -3.0, y: -4.0 }),
        (c2v { x: 1.0, y: 2.0 }, 13.0f32, c2v { x: 6.0, y: 14.0 }),
        (c2v { x: 0.0, y: 0.0 }, 1.0f32, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 0.0, y: 0.0 }, 1.0f32, c2v { x: 0.0, y: -1.0 }),
        // strictly inside / outside by one step
        (c2v { x: 0.0, y: 0.0 }, 5.0f32, c2v { x: 2.9999, y: 4.0 }),
        (c2v { x: 0.0, y: 0.0 }, 5.0f32, c2v { x: 3.0001, y: 4.0 }),
        // centre == point
        (c2v { x: 7.0, y: -3.0 }, 2.0f32, c2v { x: 7.0, y: -3.0 }),
        // zero and negative radius
        (c2v { x: 0.0, y: 0.0 }, 0.0f32, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: 0.0, y: 0.0 }, -0.0f32, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: 0.0, y: 0.0 }, -5.0f32, c2v { x: 1.0, y: 1.0 }),
        (c2v { x: 0.0, y: 0.0 }, -5.0f32, c2v { x: 3.0, y: 4.0 }),
    ];
    for (i, &(p, r, b)) in exact.iter().enumerate() {
        let a = c2Circle { p, r };
        diff_eq!(
            format!("row23 exact #{i} c=({},{}) r={} b={}", show(p.x), show(p.y), show(r), showv(b)),
            (l.c.c2CircleToPoint)(a, b),
            (l.rs.c2CircleToPoint)(a, b)
        );
    }

    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..(N * 4) {
        let a = c2Circle {
            p: rng.vec_grid(6),
            r: rng.gridded(5),
        };
        let b = rng.vec_grid(8);
        diff_eq!(
            format!("row23 grid #{i} r={} b={}", show(a.r), showv(b)),
            (l.c.c2CircleToPoint)(a, b),
            (l.rs.c2CircleToPoint)(a, b)
        );
    }
    for i in 0..(N * 4) {
        let a = any_circle(&mut rng);
        let b = rng.any_vec();
        diff_eq!(
            format!("row23 wild #{i} c={} r={} b={}", showv(a.p), show(a.r), showv(b)),
            (l.c.c2CircleToPoint)(a, b),
            (l.rs.c2CircleToPoint)(a, b)
        );
    }
    // Special values in every slot.
    let sp = special_wide();
    for &v in &sp {
        for slot in 0..5 {
            let mut a = c2Circle {
                p: c2v { x: 1.0, y: 2.0 },
                r: 3.0,
            };
            let mut b = c2v { x: 2.0, y: 2.0 };
            match slot {
                0 => a.p.x = v,
                1 => a.p.y = v,
                2 => a.r = v,
                3 => b.x = v,
                _ => b.y = v,
            }
            diff_eq!(
                format!("row23 sp slot{slot} {}", show(v)),
                (l.c.c2CircleToPoint)(a, b),
                (l.rs.c2CircleToPoint)(a, b)
            );
        }
    }
}
