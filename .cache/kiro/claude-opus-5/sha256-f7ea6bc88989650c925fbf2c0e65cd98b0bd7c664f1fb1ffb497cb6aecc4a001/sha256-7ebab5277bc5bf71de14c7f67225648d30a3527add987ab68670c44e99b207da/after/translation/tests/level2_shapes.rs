//! Level 2: the shape-vs-shape predicates
//! (`c2CircletoCircle`, `c2CircletoAABB`, `c2CircletoCapsule`).
//!
//! These take structs by value, which also exercises the SysV classification
//! of the arguments: `c2Circle` (12 bytes) and `c2AABB` (16 bytes) go in SSE
//! registers, while `c2Capsule` (20 bytes) is MEMORY-class and passed on the
//! stack. A mismatch here would show up as a wrong answer, so the tests double
//! as an ABI check on the `#[no_mangle]` wrappers.

#![allow(non_snake_case)]

mod common;

use common::*;

macro_rules! check_i {
    ($what:expr, $c:expr, $rs:expr, $($ctx:tt)*) => {{
        let (c, rs) = ($c, $rs);
        if c != rs {
            assert_int($what, &format!($($ctx)*), c, rs);
        }
    }};
}

/// Radii spanning degenerate, normal, and non-finite values.
const RADII: &[f32] = &[
    0.0,
    -0.0,
    -1.0,
    -20.0,
    1e-45,
    f32::MIN_POSITIVE,
    0.5,
    1.0,
    10.0,
    20.0,
    100.0,
    f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

const COORDS: &[f32] = &[
    0.0,
    -0.0,
    -15.0,
    15.0,
    -40.0,
    40.0,
    -70.0,
    100.0,
    1e-45,
    1e30,
    -1e30,
    f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

#[test]
fn circle_to_circle_grid() {
    let p = pair();
    for &ax in COORDS {
        for &ay in COORDS {
            for &ar in RADII {
                for &bx in COORDS {
                    for &br in RADII {
                        let A = c2Circle {
                            p: c2v { x: ax, y: ay },
                            r: ar,
                        };
                        let B = c2Circle {
                            p: c2v { x: bx, y: ay },
                            r: br,
                        };
                        check_i!(
                            "c2CircletoCircle",
                            unsafe { (p.c.c2CircletoCircle)(A, B) },
                            unsafe { (p.rs.c2CircletoCircle)(A, B) },
                            "c2CircletoCircle({A:?}, {B:?})"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn circle_to_circle_random() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..1_000_000u32 {
        let A = c2Circle {
            p: rng.v(150.0),
            r: rng.range(60.0),
        };
        let B = c2Circle {
            p: rng.v(150.0),
            r: rng.range(60.0),
        };
        check_i!(
            "c2CircletoCircle",
            unsafe { (p.c.c2CircletoCircle)(A, B) },
            unsafe { (p.rs.c2CircletoCircle)(A, B) },
            "iter {i}: {A:?} vs {B:?}"
        );
    }
    // ...and again with arbitrary bit patterns, so NaN/Inf paths are covered.
    for i in 0..1_000_000u32 {
        let A = c2Circle {
            p: rng.any_v(),
            r: rng.any_f32(),
        };
        let B = c2Circle {
            p: rng.any_v(),
            r: rng.any_f32(),
        };
        check_i!(
            "c2CircletoCircle",
            unsafe { (p.c.c2CircletoCircle)(A, B) },
            unsafe { (p.rs.c2CircletoCircle)(A, B) },
            "bit iter {i}: {A:?} vs {B:?}"
        );
    }
}

/// Exactly-touching circles: `d2 < r2` is a strict comparison, so the boundary
/// must resolve identically on both sides.
#[test]
fn circle_to_circle_exact_boundary() {
    let p = pair();
    for &(dx, ra, rb) in &[
        (3.0f32, 1.0f32, 2.0f32),   // d2 == r2 exactly
        (5.0, 3.0, 4.0),
        (2.0, 1.0, 1.0),
        (0.0, 0.0, 0.0),
        (1.0, 0.5, 0.5),
        (4.0, 2.0, 2.0),
        (40.0, 20.0, 20.0),
        (25.0, 5.0, 20.0),
    ] {
        for eps in [-1.0f32, 0.0, 1.0] {
            let shifted = f32::from_bits(
                (dx.to_bits() as i64 + eps as i64) as u32,
            );
            let A = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: ra,
            };
            let B = c2Circle {
                p: c2v { x: shifted, y: 0.0 },
                r: rb,
            };
            check_i!(
                "c2CircletoCircle",
                unsafe { (p.c.c2CircletoCircle)(A, B) },
                unsafe { (p.rs.c2CircletoCircle)(A, B) },
                "boundary dx={dx} eps={eps} {A:?} vs {B:?}"
            );
        }
    }
}

#[test]
fn circle_to_aabb_grid() {
    let p = pair();
    const BOX_EDGES: &[f32] = &[-40.0, -15.0, 0.0, 15.0, 40.0, f32::NAN, f32::INFINITY];
    for &ax in COORDS {
        for &ay in COORDS {
            for &ar in RADII {
                for &minx in BOX_EDGES {
                    for &maxx in BOX_EDGES {
                        for &miny in BOX_EDGES {
                            for &maxy in BOX_EDGES {
                                let A = c2Circle {
                                    p: c2v { x: ax, y: ay },
                                    r: ar,
                                };
                                let B = c2AABB {
                                    min: c2v { x: minx, y: miny },
                                    max: c2v { x: maxx, y: maxy },
                                };
                                check_i!(
                                    "c2CircletoAABB",
                                    unsafe { (p.c.c2CircletoAABB)(A, B) },
                                    unsafe { (p.rs.c2CircletoAABB)(A, B) },
                                    "c2CircletoAABB({A:?}, {B:?})"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn circle_to_aabb_random() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..1_000_000u32 {
        let A = c2Circle {
            p: rng.v(120.0),
            r: rng.range(40.0),
        };
        // Deliberately do not normalise min/max: inverted boxes are legal input.
        let B = c2AABB {
            min: rng.v(60.0),
            max: rng.v(60.0),
        };
        check_i!(
            "c2CircletoAABB",
            unsafe { (p.c.c2CircletoAABB)(A, B) },
            unsafe { (p.rs.c2CircletoAABB)(A, B) },
            "iter {i}: {A:?} vs {B:?}"
        );
    }
    for i in 0..1_000_000u32 {
        let A = c2Circle {
            p: rng.any_v(),
            r: rng.any_f32(),
        };
        let B = c2AABB {
            min: rng.any_v(),
            max: rng.any_v(),
        };
        check_i!(
            "c2CircletoAABB",
            unsafe { (p.c.c2CircletoAABB)(A, B) },
            unsafe { (p.rs.c2CircletoAABB)(A, B) },
            "bit iter {i}: {A:?} vs {B:?}"
        );
    }
}

/// Circle centre exactly on each edge/corner of the AABB used by
/// `circle_collide`, plus one-ULP steps either side.
#[test]
fn circle_to_aabb_edges_and_corners() {
    let p = pair();
    let B = c2AABB {
        min: c2v { x: -40.0, y: -40.0 },
        max: c2v { x: -15.0, y: -15.0 },
    };
    let anchors = [-40.0f32, -15.0, -27.5];
    for &x in &anchors {
        for &y in &anchors {
            for dx in -2i64..=2 {
                for dy in -2i64..=2 {
                    let px = f32::from_bits((x.to_bits() as i64 + dx) as u32);
                    let py = f32::from_bits((y.to_bits() as i64 + dy) as u32);
                    for &r in &[0.0f32, 1e-45, f32::MIN_POSITIVE, 0.001, 1.0, 12.5, 25.0] {
                        let A = c2Circle {
                            p: c2v { x: px, y: py },
                            r,
                        };
                        check_i!(
                            "c2CircletoAABB",
                            unsafe { (p.c.c2CircletoAABB)(A, B) },
                            unsafe { (p.rs.c2CircletoAABB)(A, B) },
                            "edge case {A:?} vs {B:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn circle_to_capsule_grid() {
    let p = pair();
    // Includes a degenerate capsule (a == b) which makes c2Dot(n, n) zero and
    // therefore da/0 -> NaN or 0/0 -> NaN inside the middle branch.
    let capsules = [
        c2Capsule {
            a: c2v { x: -40.0, y: 40.0 },
            b: c2v { x: -20.0, y: 100.0 },
            r: 10.0,
        },
        c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: 5.0,
        },
        c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        },
        c2Capsule {
            a: c2v { x: -10.0, y: 0.0 },
            b: c2v { x: 10.0, y: 0.0 },
            r: 1.0,
        },
        c2Capsule {
            a: c2v { x: 10.0, y: 0.0 },
            b: c2v { x: -10.0, y: 0.0 },
            r: 1.0,
        },
        c2Capsule {
            a: c2v { x: 0.0, y: -0.0 },
            b: c2v { x: -0.0, y: 0.0 },
            r: -3.0,
        },
        c2Capsule {
            a: c2v { x: f32::NAN, y: 0.0 },
            b: c2v { x: 1.0, y: 2.0 },
            r: 4.0,
        },
        c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: f32::INFINITY, y: 0.0 },
            r: 4.0,
        },
        c2Capsule {
            a: c2v {
                x: f32::NEG_INFINITY,
                y: f32::INFINITY,
            },
            b: c2v { x: f32::INFINITY, y: 0.0 },
            r: f32::NAN,
        },
        c2Capsule {
            a: c2v { x: 1e30, y: -1e30 },
            b: c2v { x: -1e30, y: 1e30 },
            r: 1e30,
        },
        c2Capsule {
            a: c2v { x: 1e-45, y: 0.0 },
            b: c2v { x: 2e-45, y: 0.0 },
            r: 1e-45,
        },
    ];
    for &ax in COORDS {
        for &ay in COORDS {
            for &ar in RADII {
                for B in capsules {
                    let A = c2Circle {
                        p: c2v { x: ax, y: ay },
                        r: ar,
                    };
                    check_i!(
                        "c2CircletoCapsule",
                        unsafe { (p.c.c2CircletoCapsule)(A, B) },
                        unsafe { (p.rs.c2CircletoCapsule)(A, B) },
                        "c2CircletoCapsule({A:?}, {B:?})"
                    );
                }
            }
        }
    }
}

#[test]
fn circle_to_capsule_random() {
    let p = pair();
    let mut rng = Rng::new();
    // Finite sweep: hits all three branches (before `a`, on the segment, past `b`).
    for i in 0..1_000_000u32 {
        let A = c2Circle {
            p: rng.v(150.0),
            r: rng.range(30.0),
        };
        let B = c2Capsule {
            a: rng.v(80.0),
            b: rng.v(80.0),
            r: rng.range(20.0),
        };
        check_i!(
            "c2CircletoCapsule",
            unsafe { (p.c.c2CircletoCapsule)(A, B) },
            unsafe { (p.rs.c2CircletoCapsule)(A, B) },
            "iter {i}: {A:?} vs {B:?}"
        );
    }
    for i in 0..1_000_000u32 {
        let A = c2Circle {
            p: rng.any_v(),
            r: rng.any_f32(),
        };
        let B = c2Capsule {
            a: rng.any_v(),
            b: rng.any_v(),
            r: rng.any_f32(),
        };
        check_i!(
            "c2CircletoCapsule",
            unsafe { (p.c.c2CircletoCapsule)(A, B) },
            unsafe { (p.rs.c2CircletoCapsule)(A, B) },
            "bit iter {i}: {A:?} vs {B:?}"
        );
    }
}

/// Drives the `da == 0` / `db == 0` branch boundaries: the C tests `da < 0` and
/// `db < 0`, so a point exactly at an endpoint takes the *middle* branch.
#[test]
fn circle_to_capsule_branch_boundaries() {
    let p = pair();
    let B = c2Capsule {
        a: c2v { x: -40.0, y: 40.0 },
        b: c2v { x: -20.0, y: 100.0 },
        r: 10.0,
    };
    let anchors = [
        c2v { x: -40.0, y: 40.0 },   // exactly at a  => da == 0
        c2v { x: -20.0, y: 100.0 },  // exactly at b  => db == 0
        c2v { x: -30.0, y: 70.0 },   // midpoint
        c2v { x: -45.0, y: 25.0 },   // before a
        c2v { x: -15.0, y: 115.0 },  // past b
    ];
    for anchor in anchors {
        for dx in -3i64..=3 {
            for dy in -3i64..=3 {
                let px = f32::from_bits((anchor.x.to_bits() as i64 + dx) as u32);
                let py = f32::from_bits((anchor.y.to_bits() as i64 + dy) as u32);
                for &r in &[0.0f32, 1e-45, 0.5, 10.0, 30.0, -5.0] {
                    let A = c2Circle {
                        p: c2v { x: px, y: py },
                        r,
                    };
                    check_i!(
                        "c2CircletoCapsule",
                        unsafe { (p.c.c2CircletoCapsule)(A, B) },
                        unsafe { (p.rs.c2CircletoCapsule)(A, B) },
                        "branch boundary {A:?} vs {B:?}"
                    );
                }
            }
        }
    }
}
