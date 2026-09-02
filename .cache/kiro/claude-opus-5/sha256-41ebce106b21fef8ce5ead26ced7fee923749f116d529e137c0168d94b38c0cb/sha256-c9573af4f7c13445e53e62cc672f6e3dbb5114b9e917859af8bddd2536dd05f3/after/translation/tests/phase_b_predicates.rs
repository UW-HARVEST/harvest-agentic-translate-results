//! Phase B — CONFIGS.md rows 21..25: the boolean predicate layer.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 6000;

/// Rows 21-23: overlapping, exactly-touching, inverted and degenerate boxes.
#[test]
fn row21_row22_row23_c2AABBtoAABB() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x21);
    unsafe {
        // row 21 — random boxes, well-formed, deliberately overlapping often
        for _ in 0..N {
            let ax = rng.sym(10.0);
            let ay = rng.sym(10.0);
            let A = c2AABB {
                min: c2v { x: ax, y: ay },
                max: c2v {
                    x: ax + rng.range(0.0, 8.0),
                    y: ay + rng.range(0.0, 8.0),
                },
            };
            let bx = ax + rng.sym(8.0);
            let by = ay + rng.sym(8.0);
            let B = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v {
                    x: bx + rng.range(0.0, 8.0),
                    y: by + rng.range(0.0, 8.0),
                },
            };
            d.int("c2AABBtoAABB(overlap)", (p.c.c2AABBtoAABB)(A, B), (p.rs.c2AABBtoAABB)(A, B));
            d.int("c2AABBtoAABB(swap)", (p.c.c2AABBtoAABB)(B, A), (p.rs.c2AABBtoAABB)(B, A));
        }

        // row 22 — exactly touching on each of the four sides (`<` is strict,
        // so touching counts as overlapping)
        for _ in 0..N {
            let ax = rng.sym(10.0);
            let ay = rng.sym(10.0);
            let w = rng.range(0.5, 5.0);
            let h = rng.range(0.5, 5.0);
            let A = c2AABB {
                min: c2v { x: ax, y: ay },
                max: c2v { x: ax + w, y: ay + h },
            };
            let shifts = [
                (w, 0.0),
                (-w, 0.0),
                (0.0, h),
                (0.0, -h),
                (w, h),
                (-w, -h),
            ];
            for (sx, sy) in shifts {
                let B = c2AABB {
                    min: c2v { x: A.min.x + sx, y: A.min.y + sy },
                    max: c2v { x: A.max.x + sx, y: A.max.y + sy },
                };
                d.int("c2AABBtoAABB(touch)", (p.c.c2AABBtoAABB)(A, B), (p.rs.c2AABBtoAABB)(A, B));
            }
        }

        // row 23 — inverted (min > max), zero-area, and NaN/inf boxes
        for _ in 0..N {
            let A = c2AABB { min: rng.v_mixed(), max: rng.v_mixed() };
            let B = c2AABB { min: rng.v_mixed(), max: rng.v_mixed() };
            d.int("c2AABBtoAABB(mixed)", (p.c.c2AABBtoAABB)(A, B), (p.rs.c2AABBtoAABB)(A, B));
        }
        for &q in WEIRD {
            let A = c2AABB {
                min: c2v { x: q, y: q },
                max: c2v { x: q, y: q },
            };
            for &z in WEIRD {
                let B = c2AABB {
                    min: c2v { x: z, y: q },
                    max: c2v { x: q, y: z },
                };
                d.int("c2AABBtoAABB(weird)", (p.c.c2AABBtoAABB)(A, B), (p.rs.c2AABBtoAABB)(A, B));
            }
        }
    }
    d.finish("rows 21-23: c2AABBtoAABB");
}

/// Row 24: inside / on each edge / at each corner / outside, plus NaN.
#[test]
fn row24_c2AABBtoPoint() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x24);
    unsafe {
        for _ in 0..N {
            let ax = rng.sym(10.0);
            let ay = rng.sym(10.0);
            let w = rng.range(0.0, 8.0);
            let h = rng.range(0.0, 8.0);
            let A = c2AABB {
                min: c2v { x: ax, y: ay },
                max: c2v { x: ax + w, y: ay + h },
            };
            // corners + edge midpoints + centre + just-outside probes
            let pts = [
                c2v { x: A.min.x, y: A.min.y },
                c2v { x: A.max.x, y: A.min.y },
                c2v { x: A.min.x, y: A.max.y },
                c2v { x: A.max.x, y: A.max.y },
                c2v { x: A.min.x, y: (A.min.y + A.max.y) * 0.5 },
                c2v { x: A.max.x, y: (A.min.y + A.max.y) * 0.5 },
                c2v { x: (A.min.x + A.max.x) * 0.5, y: A.min.y },
                c2v { x: (A.min.x + A.max.x) * 0.5, y: A.max.y },
                c2v { x: (A.min.x + A.max.x) * 0.5, y: (A.min.y + A.max.y) * 0.5 },
                c2v { x: nextafter_down(A.min.x), y: A.min.y },
                c2v { x: nextafter_up(A.max.x), y: A.max.y },
                c2v { x: A.min.x, y: nextafter_down(A.min.y) },
                c2v { x: A.max.x, y: nextafter_up(A.max.y) },
                rng.v_small(),
                rng.v_mixed(),
            ];
            for q in pts {
                d.int("c2AABBtoPoint", (p.c.c2AABBtoPoint)(A, q), (p.rs.c2AABBtoPoint)(A, q));
            }
        }
        // exhaustive weird cross product
        for &q in WEIRD {
            for &z in WEIRD {
                let A = c2AABB {
                    min: c2v { x: q, y: z },
                    max: c2v { x: z, y: q },
                };
                let pt = c2v { x: z, y: q };
                d.int("c2AABBtoPoint(weird)", (p.c.c2AABBtoPoint)(A, pt), (p.rs.c2AABBtoPoint)(A, pt));
            }
        }
    }
    d.finish("row 24: c2AABBtoPoint");
}

/// Row 25: strict `<` means a point exactly on the rim is OUTSIDE. Also
/// covers `r == 0`, `r < 0`, huge/tiny `r` and NaN.
#[test]
fn row25_c2CircleToPoint() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x25);
    unsafe {
        for _ in 0..N {
            let A = c2Circle {
                p: rng.v_small(),
                r: rng.range(-4.0, 8.0),
            };
            // exactly on the rim, just inside, just outside, centre, random
            let ang = rng.range(-7.0, 7.0);
            let on = c2v {
                x: A.p.x + A.r * ang.cos(),
                y: A.p.y + A.r * ang.sin(),
            };
            let pts = [
                A.p,
                on,
                c2v {
                    x: A.p.x + A.r * 0.999_9 * ang.cos(),
                    y: A.p.y + A.r * 0.999_9 * ang.sin(),
                },
                c2v {
                    x: A.p.x + A.r * 1.000_1 * ang.cos(),
                    y: A.p.y + A.r * 1.000_1 * ang.sin(),
                },
                rng.v_small(),
                rng.v_mixed(),
            ];
            for q in pts {
                d.int("c2CircleToPoint", (p.c.c2CircleToPoint)(A, q), (p.rs.c2CircleToPoint)(A, q));
            }
        }
        // exact-rim on the axes with integer-friendly radii (no rounding slack)
        for r in [0.0f32, 1.0, 2.0, 4.0, 8.0, 0.5, -1.0, -4.0] {
            let A = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r };
            for q in [
                c2v { x: r, y: 0.0 },
                c2v { x: -r, y: 0.0 },
                c2v { x: 0.0, y: r },
                c2v { x: 0.0, y: -r },
                c2v { x: 0.0, y: 0.0 },
            ] {
                d.int("c2CircleToPoint(rim)", (p.c.c2CircleToPoint)(A, q), (p.rs.c2CircleToPoint)(A, q));
            }
        }
        for &cr in WEIRD {
            for &z in WEIRD {
                let A = c2Circle { p: c2v { x: z, y: cr }, r: cr };
                let q = c2v { x: cr, y: z };
                d.int("c2CircleToPoint(weird)", (p.c.c2CircleToPoint)(A, q), (p.rs.c2CircleToPoint)(A, q));
            }
        }
    }
    d.finish("row 25: c2CircleToPoint");
}

fn nextafter_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

fn nextafter_down(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() - 1)
    } else {
        f32::from_bits(x.to_bits() + 1)
    }
}
