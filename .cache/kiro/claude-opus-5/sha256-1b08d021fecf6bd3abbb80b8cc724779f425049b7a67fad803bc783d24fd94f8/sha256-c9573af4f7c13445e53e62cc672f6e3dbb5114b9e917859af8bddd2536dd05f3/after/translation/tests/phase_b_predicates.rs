//! Phase B rows 18–24: the overlap / containment predicates.

mod common;
use common::*;

const N: usize = 40_000;

#[test]
fn row18_20_21_aabb_to_aabb() {
    let (c, r) = pair();
    let mut d = Diff::new("18/20/21: c2AABBtoAABB");
    let mut g = Rng::new(0x1801);

    // 18: random pairs, mostly overlapping (drawn from the same small region).
    for _ in 0..N {
        let a = g.aabb(10.0);
        let b = g.aabb(10.0);
        d.eq(
            || format!("overlap {} {}", fbox(a), fbox(b)),
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
        );
    }

    // 20: separated on exactly one axis, cycling which of d0..d3 fires.
    for i in 0..N {
        let a = g.aabb(5.0);
        let gap = g.unit() * 3.0;
        let b = match i % 4 {
            // d0: b.max.x < a.min.x
            0 => c2AABB {
                min: c2v {
                    x: a.min.x - gap - 2.0,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.min.x - gap,
                    y: a.max.y,
                },
            },
            // d1: a.max.x < b.min.x
            1 => c2AABB {
                min: c2v {
                    x: a.max.x + gap,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.max.x + gap + 2.0,
                    y: a.max.y,
                },
            },
            // d2: b.max.y < a.min.y
            2 => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.min.y - gap - 2.0,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.min.y - gap,
                },
            },
            // d3: a.max.y < b.min.y
            _ => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.max.y + gap,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.max.y + gap + 2.0,
                },
            },
        };
        d.eq(
            || format!("separated{} {} {}", i % 4, fbox(a), fbox(b)),
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
        );
    }

    // 21: degenerate, inverted, infinite, NaN, fully random bit patterns.
    for _ in 0..N {
        let mk = |g: &mut Rng| -> c2AABB {
            match g.below(5) {
                0 => {
                    let p = g.v(5.0);
                    c2AABB { min: p, max: p } // degenerate
                }
                1 => {
                    let a = g.aabb(5.0);
                    c2AABB { min: a.max, max: a.min } // inverted
                }
                2 => c2AABB {
                    min: g.v_special(),
                    max: g.v_special(),
                },
                3 => c2AABB {
                    min: c2v {
                        x: g.any_bits_f32(),
                        y: g.any_bits_f32(),
                    },
                    max: c2v {
                        x: g.any_bits_f32(),
                        y: g.any_bits_f32(),
                    },
                },
                _ => g.aabb(1e30),
            }
        };
        let a = mk(&mut g);
        let b = mk(&mut g);
        d.eq(
            || format!("degenerate {} {}", fbox(a), fbox(b)),
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
        );
    }
    d.finish();
}

#[test]
fn row19_aabb_to_aabb_exact_touch() {
    let (c, r) = pair();
    let mut d = Diff::new("19: c2AABBtoAABB exact face contact (strict <)");
    let mut g = Rng::new(0x1901);
    for i in 0..N {
        let a = g.aabb(10.0);
        let w = g.unit() * 4.0;
        let b = match i % 4 {
            0 => c2AABB {
                min: c2v { x: a.max.x, y: a.min.y },
                max: c2v { x: a.max.x + w, y: a.max.y },
            },
            1 => c2AABB {
                min: c2v { x: a.min.x - w, y: a.min.y },
                max: c2v { x: a.min.x, y: a.max.y },
            },
            2 => c2AABB {
                min: c2v { x: a.min.x, y: a.max.y },
                max: c2v { x: a.max.x, y: a.max.y + w },
            },
            _ => c2AABB {
                min: c2v { x: a.min.x, y: a.min.y - w },
                max: c2v { x: a.max.x, y: a.min.y },
            },
        };
        d.eq(
            || format!("touch{} {} {}", i % 4, fbox(a), fbox(b)),
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
        );
    }
    d.finish();
}

#[test]
fn row22_23_aabb_to_point() {
    let (c, r) = pair();
    let mut d = Diff::new("22/23: c2AABBtoPoint");
    let mut g = Rng::new(0x2201);

    for i in 0..N {
        let bx = g.aabb(10.0);
        let p = match i % 8 {
            0 => c2v {
                // strictly inside
                x: bx.min.x + (bx.max.x - bx.min.x) * g.unit(),
                y: bx.min.y + (bx.max.y - bx.min.y) * g.unit(),
            },
            1 => c2v { x: bx.min.x, y: bx.min.y }, // corner
            2 => c2v { x: bx.max.x, y: bx.max.y }, // corner
            3 => c2v {
                x: bx.min.x,
                y: bx.min.y + (bx.max.y - bx.min.y) * g.unit(),
            }, // on a face
            4 => c2v {
                x: bx.min.x - 1.0 - g.unit(),
                y: bx.min.y,
            }, // d0
            5 => c2v {
                x: bx.min.x,
                y: bx.min.y - 1.0 - g.unit(),
            }, // d1
            6 => c2v {
                x: bx.max.x + 1.0 + g.unit(),
                y: bx.max.y,
            }, // d2
            _ => c2v {
                x: bx.max.x,
                y: bx.max.y + 1.0 + g.unit(),
            }, // d3
        };
        d.eq(
            || format!("case{} {} {}", i % 8, fbox(bx), fv(p)),
            unsafe { (c.c2AABBtoPoint)(bx, p) },
            unsafe { (r.c2AABBtoPoint)(bx, p) },
        );
    }

    // degenerate box + point exactly at it, plus fully random bit patterns
    for _ in 0..N {
        let (bx, p) = match g.below(4) {
            0 => {
                let q = g.v(5.0);
                (c2AABB { min: q, max: q }, q)
            }
            1 => {
                let a = g.aabb(5.0);
                (c2AABB { min: a.max, max: a.min }, g.v(5.0))
            }
            2 => (
                c2AABB {
                    min: g.v_special(),
                    max: g.v_special(),
                },
                g.v_special(),
            ),
            _ => (
                c2AABB {
                    min: c2v {
                        x: g.any_bits_f32(),
                        y: g.any_bits_f32(),
                    },
                    max: c2v {
                        x: g.any_bits_f32(),
                        y: g.any_bits_f32(),
                    },
                },
                c2v {
                    x: g.any_bits_f32(),
                    y: g.any_bits_f32(),
                },
            ),
        };
        d.eq(
            || format!("edge {} {}", fbox(bx), fv(p)),
            unsafe { (c.c2AABBtoPoint)(bx, p) },
            unsafe { (r.c2AABBtoPoint)(bx, p) },
        );
    }
    d.finish();
}

#[test]
fn row24_circle_to_point() {
    let (c, r) = pair();
    let mut d = Diff::new("24: c2CircleToPoint");
    let mut g = Rng::new(0x2401);

    const RADII: &[f32] = &[
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1.0,
        -1.0,
        1e-6,
        1e6,
        1e18,
        -1e18,
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
    ];

    for i in 0..N {
        let rad = RADII[i % RADII.len()];
        let cir = c2Circle { p: g.v(10.0), r: rad };
        let ang = g.unit() * std::f32::consts::TAU;
        // Points strictly inside, exactly on, and outside.
        let scale = match i % 4 {
            0 => g.unit() * 0.999,
            1 => 1.0, // exactly on -> reject (strict <)
            2 => 1.0 + g.unit(),
            _ => g.unit() * 4.0,
        };
        let rr = if rad.is_finite() { rad } else { 1.0 };
        let p = c2v {
            x: cir.p.x + ang.cos() * rr * scale,
            y: cir.p.y + ang.sin() * rr * scale,
        };
        d.eq(
            || format!("case{} {} {}", i % 4, fcircle(cir), fv(p)),
            unsafe { (c.c2CircleToPoint)(cir, p) },
            unsafe { (r.c2CircleToPoint)(cir, p) },
        );
    }

    // centre of a zero-radius circle, and hostile bit patterns
    for _ in 0..N {
        let (cir, p) = match g.below(4) {
            0 => {
                let q = g.v(5.0);
                (c2Circle { p: q, r: 0.0 }, q)
            }
            1 => (
                c2Circle {
                    p: g.v_special(),
                    r: g.special_f32(),
                },
                g.v_special(),
            ),
            2 => (
                c2Circle {
                    p: c2v {
                        x: g.any_bits_f32(),
                        y: g.any_bits_f32(),
                    },
                    r: g.any_bits_f32(),
                },
                c2v {
                    x: g.any_bits_f32(),
                    y: g.any_bits_f32(),
                },
            ),
            _ => (
                c2Circle {
                    p: g.v(1e20),
                    r: g.sym(1e20),
                },
                g.v(1e20),
            ),
        };
        d.eq(
            || format!("edge {} {}", fcircle(cir), fv(p)),
            unsafe { (c.c2CircleToPoint)(cir, p) },
            unsafe { (r.c2CircleToPoint)(cir, p) },
        );
    }
    d.finish();
}
