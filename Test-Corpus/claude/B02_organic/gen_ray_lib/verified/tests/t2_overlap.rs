//! Phase B rows B12..B18 and Phase C rows E06..E10, E16..E19:
//! the three overlap predicates `c2AABBtoAABB`, `c2AABBtoPoint`,
//! `c2CircleToPoint`.

mod common;
use common::*;

/// B12: random overlapping box pairs.
#[test]
fn b12_aabb_overlapping() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB12);
    let mut d = Diff::new();
    for _ in 0..20_000 {
        // Two boxes around nearby centres, so they usually overlap.
        let a = rng.aabb_proper();
        let cx = (a.min.x + a.max.x) * 0.5 + rng.uniform(3.0);
        let cy = (a.min.y + a.max.y) * 0.5 + rng.uniform(3.0);
        let b = c2AABB {
            min: c2v {
                x: cx - 2.0,
                y: cy - 2.0,
            },
            max: c2v {
                x: cx + 2.0,
                y: cy + 2.0,
            },
        };
        d.ints(
            "c2AABBtoAABB/overlap",
            || format!("{:?} {:?}", a, b),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
    d.finish("B12 c2AABBtoAABB overlapping");
}

/// B13 + E06..E09: separated along each of the four axes individually.
#[test]
fn b13_e06_e09_aabb_separated_each_axis() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB13);
    let mut d = Diff::new();
    for i in 0..20_000 {
        let a = rng.aabb_proper();
        let gap = (rng.uniform(5.0)).abs() + 1e-6;
        let w = 1.0 + (rng.uniform(4.0)).abs();
        // Place `b` strictly outside along exactly one axis.
        let b = match i % 4 {
            // d0: B.max.x < A.min.x
            0 => c2AABB {
                min: c2v {
                    x: a.min.x - gap - w,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.min.x - gap,
                    y: a.max.y,
                },
            },
            // d1: A.max.x < B.min.x
            1 => c2AABB {
                min: c2v {
                    x: a.max.x + gap,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.max.x + gap + w,
                    y: a.max.y,
                },
            },
            // d2: B.max.y < A.min.y
            2 => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.min.y - gap - w,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.min.y - gap,
                },
            },
            // d3: A.max.y < B.min.y
            _ => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.max.y + gap,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.max.y + gap + w,
                },
            },
        };
        let cr = (c.c2AABBtoAABB)(a, b);
        let rr = (r.c2AABBtoAABB)(a, b);
        d.ints(
            "c2AABBtoAABB/separated",
            || format!("axis={} {:?} {:?}", i % 4, a, b),
            cr,
            rr,
        );
        // Sanity: the C really does reject here (guards the test's intent).
        d.check(cr == 0, || {
            format!("expected C to reject axis={} {:?} {:?}", i % 4, a, b)
        });
    }
    d.finish("B13/E06-E09 c2AABBtoAABB separated");
}

/// B14: exactly touching edges - the `<` boundary.
#[test]
fn b14_aabb_touching() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB14);
    let mut d = Diff::new();
    for i in 0..20_000 {
        let a = rng.aabb_proper();
        let b = match i % 4 {
            0 => c2AABB {
                min: c2v {
                    x: a.min.x - 1.0,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.min.x,
                    y: a.max.y,
                },
            },
            1 => c2AABB {
                min: c2v {
                    x: a.max.x,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.max.x + 1.0,
                    y: a.max.y,
                },
            },
            2 => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.min.y - 1.0,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.min.y,
                },
            },
            _ => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.max.y,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.max.y + 1.0,
                },
            },
        };
        d.ints(
            "c2AABBtoAABB/touching",
            || format!("{:?} {:?}", a, b),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
    d.finish("B14 c2AABBtoAABB touching");
}

/// B15: degenerate (min == max) and inverted (min > max) boxes.
#[test]
fn b15_aabb_degenerate_inverted() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB15);
    let mut d = Diff::new();
    for i in 0..20_000 {
        let p = rng.vec_nice();
        let q = rng.vec_nice();
        let (a, b) = match i % 4 {
            // degenerate x degenerate
            0 => (c2AABB { min: p, max: p }, c2AABB { min: q, max: q }),
            // inverted x proper
            1 => (
                c2AABB { min: q, max: p },
                c2AABB {
                    min: c2v {
                        x: p.x - 1.0,
                        y: p.y - 1.0,
                    },
                    max: c2v {
                        x: p.x + 1.0,
                        y: p.y + 1.0,
                    },
                },
            ),
            // proper x inverted
            2 => (
                c2AABB {
                    min: c2v {
                        x: q.x - 1.0,
                        y: q.y - 1.0,
                    },
                    max: c2v {
                        x: q.x + 1.0,
                        y: q.y + 1.0,
                    },
                },
                c2AABB { min: p, max: q },
            ),
            // both random (frequently inverted)
            _ => (
                c2AABB { min: p, max: q },
                c2AABB { min: q, max: p },
            ),
        };
        d.ints(
            "c2AABBtoAABB/degenerate",
            || format!("{:?} {:?}", a, b),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
    d.finish("B15 c2AABBtoAABB degenerate/inverted");
}

/// B16 + E10: NaN / infinite coordinates (unordered compares).
#[test]
fn b16_e10_aabb_nan_inf() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB16);
    let mut d = Diff::new();

    // A NaN anywhere makes every `<` false, so the C ACCEPTS (returns 1).
    let nan_box = c2AABB {
        min: c2v {
            x: f32::NAN,
            y: 0.0,
        },
        max: c2v {
            x: f32::NAN,
            y: 1.0,
        },
    };
    let far = c2AABB {
        min: c2v {
            x: 1e30,
            y: 1e30,
        },
        max: c2v {
            x: 2e30,
            y: 2e30,
        },
    };
    let cr = (c.c2AABBtoAABB)(nan_box, far);
    d.ints("c2AABBtoAABB/nan", || "nan_box vs far".into(), cr, (r.c2AABBtoAABB)(nan_box, far));

    for _ in 0..20_000 {
        let a = rng.aabb_hostile();
        let b = rng.aabb_hostile();
        d.ints(
            "c2AABBtoAABB/hostile",
            || format!("{:?} {:?}", a, b),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
    // Exhaustive: all four coordinates of A drawn from SPECIALS, B fixed proper.
    let bproper = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for &v0 in &SPECIALS {
        for &v1 in &SPECIALS {
            for &v2 in &SPECIALS {
                for &v3 in &SPECIALS {
                    let a = c2AABB {
                        min: c2v { x: v0, y: v1 },
                        max: c2v { x: v2, y: v3 },
                    };
                    d.ints(
                        "c2AABBtoAABB/specials",
                        || format!("{:?}", a),
                        (c.c2AABBtoAABB)(a, bproper),
                        (r.c2AABBtoAABB)(a, bproper),
                    );
                    d.ints(
                        "c2AABBtoAABB/specials-rev",
                        || format!("{:?}", a),
                        (c.c2AABBtoAABB)(bproper, a),
                        (r.c2AABBtoAABB)(bproper, a),
                    );
                }
            }
        }
    }
    d.finish("B16/E10 c2AABBtoAABB NaN/inf");
}

/// B17 + E16: point-in-box, all rejection axes, edges, corners, NaN.
#[test]
fn b17_e16_aabb_to_point() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB17);
    let mut d = Diff::new();

    for i in 0..20_000 {
        let a = rng.aabb_proper();
        let eps = 1e-3;
        let p = match i % 10 {
            // inside
            0 => c2v {
                x: (a.min.x + a.max.x) * 0.5,
                y: (a.min.y + a.max.y) * 0.5,
            },
            // d0: B.x < A.min.x
            1 => c2v {
                x: a.min.x - eps,
                y: (a.min.y + a.max.y) * 0.5,
            },
            // d1: B.y < A.min.y
            2 => c2v {
                x: (a.min.x + a.max.x) * 0.5,
                y: a.min.y - eps,
            },
            // d2: B.x > A.max.x
            3 => c2v {
                x: a.max.x + eps,
                y: (a.min.y + a.max.y) * 0.5,
            },
            // d3: B.y > A.max.y
            4 => c2v {
                x: (a.min.x + a.max.x) * 0.5,
                y: a.max.y + eps,
            },
            // exactly on each edge / corner
            5 => c2v { x: a.min.x, y: a.min.y },
            6 => c2v { x: a.max.x, y: a.max.y },
            7 => c2v { x: a.min.x, y: a.max.y },
            8 => c2v { x: a.max.x, y: a.min.y },
            // fully random
            _ => rng.vec_nice(),
        };
        d.ints(
            "c2AABBtoPoint",
            || format!("case={} {:?} {:?}", i % 10, a, p),
            (c.c2AABBtoPoint)(a, p),
            (r.c2AABBtoPoint)(a, p),
        );
    }

    // Hostile boxes/points, plus the exhaustive specials cross product.
    for _ in 0..20_000 {
        let a = rng.aabb_hostile();
        let p = rng.vec_hostile();
        d.ints(
            "c2AABBtoPoint/hostile",
            || format!("{:?} {:?}", a, p),
            (c.c2AABBtoPoint)(a, p),
            (r.c2AABBtoPoint)(a, p),
        );
    }
    let boxes = [
        c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        },
        // inverted
        c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: -1.0, y: -1.0 },
        },
        // degenerate
        c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
        // signed zero corner
        c2AABB {
            min: c2v { x: -0.0, y: -0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
    ];
    for b in boxes {
        for &x in &SPECIALS {
            for &y in &SPECIALS {
                let p = c2v { x, y };
                d.ints(
                    "c2AABBtoPoint/specials",
                    || format!("{:?} {:?}", b, p),
                    (c.c2AABBtoPoint)(b, p),
                    (r.c2AABBtoPoint)(b, p),
                );
            }
        }
    }
    d.finish("B17/E16 c2AABBtoPoint");
}

/// B18 + E17..E19: point-in-circle, incl. r == 0, r < 0, on-circumference.
#[test]
fn b18_e17_e19_circle_to_point() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB18);
    let mut d = Diff::new();

    for i in 0..20_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(20.0)).abs() + 1e-4;
        let dir = rng.unit();
        let (circ, p) = match i % 8 {
            // strictly inside
            0 => (
                c2Circle { p: centre, r: rad },
                c2v {
                    x: centre.x + dir.x * rad * 0.5,
                    y: centre.y + dir.y * rad * 0.5,
                },
            ),
            // strictly outside
            1 => (
                c2Circle { p: centre, r: rad },
                c2v {
                    x: centre.x + dir.x * rad * 2.0,
                    y: centre.y + dir.y * rad * 2.0,
                },
            ),
            // exactly on the circumference (d2 == r*r boundary)
            2 => (
                c2Circle { p: centre, r: rad },
                c2v {
                    x: centre.x + dir.x * rad,
                    y: centre.y + dir.y * rad,
                },
            ),
            // E18: r == 0
            3 => (c2Circle { p: centre, r: 0.0 }, centre),
            // E19: negative radius still "contains" points (r*r > 0)
            4 => (
                c2Circle { p: centre, r: -rad },
                c2v {
                    x: centre.x + dir.x * rad * 0.5,
                    y: centre.y + dir.y * rad * 0.5,
                },
            ),
            // centre == point
            5 => (c2Circle { p: centre, r: rad }, centre),
            // -0.0 radius
            6 => (c2Circle { p: centre, r: -0.0 }, centre),
            _ => (rng.circle_nice(), rng.vec_nice()),
        };
        d.ints(
            "c2CircleToPoint",
            || format!("case={} {:?} {:?}", i % 8, circ, p),
            (c.c2CircleToPoint)(circ, p),
            (r.c2CircleToPoint)(circ, p),
        );
    }

    for _ in 0..20_000 {
        let circ = rng.circle_hostile();
        let p = rng.vec_hostile();
        d.ints(
            "c2CircleToPoint/hostile",
            || format!("{:?} {:?}", circ, p),
            (c.c2CircleToPoint)(circ, p),
            (r.c2CircleToPoint)(circ, p),
        );
    }
    for &rad in &SPECIALS {
        for &x in &SPECIALS {
            for &y in &SPECIALS {
                let circ = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: rad,
                };
                let p = c2v { x, y };
                d.ints(
                    "c2CircleToPoint/specials",
                    || format!("{:?} {:?}", circ, p),
                    (c.c2CircleToPoint)(circ, p),
                    (r.c2CircleToPoint)(circ, p),
                );
            }
        }
    }
    d.finish("B18/E17-E19 c2CircleToPoint");
}
