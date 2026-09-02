//! Phase B — CONFIGS.md rows 26..36: `c2RaytoCircle` and `c2RaytoAABB`,
//! called directly (not through `c2CastRay`).

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4000;

// ---------------------------------------------------------------- circle

/// Row 26: fully random rays vs random circles.
#[test]
fn row26_circle_random() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x26);
    unsafe {
        for _ in 0..(N * 4) {
            let A = c2Ray {
                p: rng.v_small(),
                d: rng.v_dir(),
                t: rng.range(0.0, 40.0),
            };
            let B = c2Circle {
                p: rng.v_small(),
                r: rng.range(0.0, 10.0),
            };
            d.ray("c2RaytoCircle(rand)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
        }
    }
    d.finish("row 26: c2RaytoCircle random");
}

/// Row 27: ray origin strictly inside the circle → `c < 0` → `t < 0` reject.
#[test]
fn row27_circle_origin_inside() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x27);
    unsafe {
        for _ in 0..(N * 2) {
            let B = c2Circle {
                p: rng.v_small(),
                r: rng.range(0.1, 10.0),
            };
            let ang = rng.range(-7.0, 7.0);
            let frac = rng.range(0.0, 0.999);
            let A = c2Ray {
                p: c2v {
                    x: B.p.x + B.r * frac * ang.cos(),
                    y: B.p.y + B.r * frac * ang.sin(),
                },
                d: rng.v_dir(),
                t: rng.range(0.0, 40.0),
            };
            d.ray("c2RaytoCircle(inside)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
        }
        // origin exactly at the centre
        for _ in 0..N {
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.0, 10.0) };
            let A = c2Ray { p: B.p, d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            d.ray("c2RaytoCircle(centre)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
        }
    }
    d.finish("row 27: c2RaytoCircle origin inside");
}

/// Row 28: `A.t` placed exactly at (and one ULP either side of) the analytic
/// hit distance, so the `t <= A.t` boundary is hit from both directions.
#[test]
fn row28_circle_t_boundary() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x28);
    unsafe {
        for _ in 0..(N * 2) {
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.1, 8.0) };
            let dir = rng.v_dir();
            // start outside, aimed at the centre-ish
            let dist = B.r + rng.range(0.1, 20.0);
            let A0 = c2Ray {
                p: c2v {
                    x: B.p.x - dir.x * dist,
                    y: B.p.y - dir.y * dist,
                },
                d: dir,
                t: 1.0e30,
            };
            // learn the exact hit t from the C implementation, then pin A.t to it
            let (hit, out) = call_circle(&p.c, A0, B);
            let ts: Vec<f32> = if hit != 0 {
                vec![out.t, ulp_down(out.t), ulp_up(out.t), 0.0, -out.t]
            } else {
                vec![0.0, dist, 1.0e30]
            };
            for t in ts {
                let A = c2Ray { p: A0.p, d: A0.d, t };
                d.ray("c2RaytoCircle(t-bound)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
            }
        }
    }
    d.finish("row 28: c2RaytoCircle A.t boundary");
}

/// Row 29: unnormalised and zero direction vectors.
#[test]
fn row29_circle_unnormalised_dir() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x29);
    unsafe {
        for _ in 0..(N * 2) {
            let scale = 10f32.powf(rng.range(-6.0, 6.0));
            let dir = rng.v_dir();
            let A = c2Ray {
                p: rng.v_small(),
                d: c2v { x: dir.x * scale, y: dir.y * scale },
                t: rng.range(0.0, 40.0),
            };
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.0, 10.0) };
            d.ray("c2RaytoCircle(unnorm)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
        }
        for _ in 0..N {
            let A = c2Ray {
                p: rng.v_small(),
                d: c2v { x: 0.0, y: 0.0 },
                t: rng.range(0.0, 40.0),
            };
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.0, 10.0) };
            d.ray("c2RaytoCircle(zero-dir)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
        }
        // axis-aligned directions
        for _ in 0..N {
            for dir in AXIS_DIRS {
                let A = c2Ray { p: rng.v_small(), d: dir, t: rng.range(0.0, 40.0) };
                let B = c2Circle { p: rng.v_small(), r: rng.range(0.0, 10.0) };
                d.ray("c2RaytoCircle(axis)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
            }
        }
    }
    d.finish("row 29: c2RaytoCircle unnormalised / zero / axis dir");
}

/// Row 30: tangent and near-tangent rays (`disc ≈ 0`, the `disc < 0` boundary).
#[test]
fn row30_circle_tangent() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x30);
    unsafe {
        for _ in 0..(N * 2) {
            let B = c2Circle { p: rng.v_small(), r: rng.range(0.1, 8.0) };
            let dir = rng.v_dir();
            let perp = c2v { x: -dir.y, y: dir.x };
            for k in [1.0f32, 0.999_99, 1.000_01, 0.99, 1.01, 0.0] {
                let off = B.r * k;
                let back = rng.range(1.0, 20.0);
                let A = c2Ray {
                    p: c2v {
                        x: B.p.x + perp.x * off - dir.x * back,
                        y: B.p.y + perp.y * off - dir.y * back,
                    },
                    d: dir,
                    t: back + B.r * 4.0,
                };
                d.ray("c2RaytoCircle(tangent)", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
            }
        }
    }
    d.finish("row 30: c2RaytoCircle tangent");
}

// ---------------------------------------------------------------- AABB

/// Row 31: fully random rays vs random boxes.
#[test]
fn row31_aabb_random() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x31);
    unsafe {
        for _ in 0..(N * 4) {
            let A = c2Ray {
                p: rng.v_small(),
                d: rng.v_dir(),
                t: rng.range(0.0, 40.0),
            };
            let bx = rng.sym(10.0);
            let by = rng.sym(10.0);
            let B = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v {
                    x: bx + rng.range(0.0, 10.0),
                    y: by + rng.range(0.0, 10.0),
                },
            };
            d.ray("c2RaytoAABB(rand)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
        }
    }
    d.finish("row 31: c2RaytoAABB random");
}

/// Row 32: axis-aligned rays — forces each of the four normal branches and
/// produces many `da - db == 0` degeneracies inside
/// `c2RayToPlane_OneDimensional`.
#[test]
fn row32_aabb_axis_aligned() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x32);
    unsafe {
        for _ in 0..(N * 2) {
            let bx = rng.sym(10.0);
            let by = rng.sym(10.0);
            let B = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v {
                    x: bx + rng.range(0.0, 10.0),
                    y: by + rng.range(0.0, 10.0),
                },
            };
            let cx = (B.min.x + B.max.x) * 0.5;
            let cy = (B.min.y + B.max.y) * 0.5;
            for dir in AXIS_DIRS {
                // aim through the centre, and offset to graze the faces
                for off in [0.0f32, 0.4, -0.4, 0.5, -0.5, 0.51, -0.51] {
                    let start = 20.0f32;
                    let (px, py) = if dir.x != 0.0 {
                        (cx - dir.x * start, cy + off * (B.max.y - B.min.y))
                    } else {
                        (cx + off * (B.max.x - B.min.x), cy - dir.y * start)
                    };
                    for t in [0.0f32, start * 0.5, start, start * 2.0, 1.0] {
                        let A = c2Ray { p: c2v { x: px, y: py }, d: dir, t };
                        d.ray("c2RaytoAABB(axis)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
                    }
                }
            }
        }
    }
    d.finish("row 32: c2RaytoAABB axis-aligned");
}

/// Row 33: ray origin inside the box.
#[test]
fn row33_aabb_origin_inside() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x33);
    unsafe {
        for _ in 0..(N * 3) {
            let bx = rng.sym(10.0);
            let by = rng.sym(10.0);
            let w = rng.range(0.1, 10.0);
            let h = rng.range(0.1, 10.0);
            let B = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v { x: bx + w, y: by + h },
            };
            let A = c2Ray {
                p: c2v {
                    x: bx + rng.range(0.0, w),
                    y: by + rng.range(0.0, h),
                },
                d: if rng.below(3) == 0 { AXIS_DIRS[rng.below(4)] } else { rng.v_dir() },
                t: rng.range(0.0, 40.0),
            };
            d.ray("c2RaytoAABB(inside)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
        }
    }
    d.finish("row 33: c2RaytoAABB origin inside");
}

/// Row 34: `A.t == 0` (zero-length ray) — `ab` becomes zero, `n` becomes zero.
#[test]
fn row34_aabb_zero_t() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x34);
    unsafe {
        for _ in 0..(N * 3) {
            let bx = rng.sym(10.0);
            let by = rng.sym(10.0);
            let B = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v {
                    x: bx + rng.range(0.0, 10.0),
                    y: by + rng.range(0.0, 10.0),
                },
            };
            for t in [0.0f32, -0.0, -1.0, -20.0] {
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t };
                d.ray("c2RaytoAABB(t<=0)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
            }
            // origin exactly on a corner with t == 0
            for q in [B.min, B.max, c2v { x: B.min.x, y: B.max.y }, c2v { x: B.max.x, y: B.min.y }] {
                let A = c2Ray { p: q, d: rng.v_dir(), t: 0.0 };
                d.ray("c2RaytoAABB(corner,t=0)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
            }
        }
    }
    d.finish("row 34: c2RaytoAABB A.t == 0");
}

/// Row 35: diagonal rays aimed exactly at corners → ties in the four-way
/// `t0 >= t1 && t0 >= t2 && t0 >= t3` chain, where the FIRST matching arm wins.
#[test]
fn row35_aabb_corner_ties() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x35);
    unsafe {
        for _ in 0..(N * 2) {
            let s = rng.range(0.5, 8.0);
            // symmetric box centred on the origin makes exact ties reachable
            let B = c2AABB {
                min: c2v { x: -s, y: -s },
                max: c2v { x: s, y: s },
            };
            let diag = [
                c2v { x: 1.0, y: 1.0 },
                c2v { x: -1.0, y: 1.0 },
                c2v { x: 1.0, y: -1.0 },
                c2v { x: -1.0, y: -1.0 },
            ];
            for dir in diag {
                for k in [1.0f32, 2.0, 0.5, 4.0] {
                    let A = c2Ray {
                        p: c2v { x: -dir.x * s * k, y: -dir.y * s * k },
                        d: dir,
                        t: rng.range(0.0, 4.0) + k,
                    };
                    d.ray("c2RaytoAABB(diag)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
                }
            }
            // ray whose own bbox exactly equals the target box
            let A = c2Ray {
                p: B.min,
                d: c2v { x: 1.0, y: 1.0 },
                t: 2.0 * s,
            };
            d.ray("c2RaytoAABB(exact-bb)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
        }
    }
    d.finish("row 35: c2RaytoAABB corner ties");
}

/// Row 36: zero-area, inverted and non-finite boxes.
#[test]
fn row36_aabb_degenerate() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x36);
    unsafe {
        for _ in 0..(N * 2) {
            // zero-area (min == max)
            let q = rng.v_small();
            let B = c2AABB { min: q, max: q };
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            d.ray("c2RaytoAABB(zero-area)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));

            // inverted
            let Bi = c2AABB { min: rng.v_small(), max: rng.v_small() };
            d.ray("c2RaytoAABB(inverted)", call_aabb(&p.c, A, Bi), call_aabb(&p.rs, A, Bi));

            // fully mixed / non-finite
            let Bm = c2AABB { min: rng.v_mixed(), max: rng.v_mixed() };
            let Am = c2Ray { p: rng.v_mixed(), d: rng.v_mixed(), t: rng.f_mixed() };
            d.ray("c2RaytoAABB(mixed)", call_aabb(&p.c, Am, Bm), call_aabb(&p.rs, Am, Bm));
        }
    }
    d.finish("row 36: c2RaytoAABB degenerate");
}

fn ulp_up(x: f32) -> f32 {
    if !x.is_finite() {
        return x;
    }
    if x >= 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

fn ulp_down(x: f32) -> f32 {
    if !x.is_finite() {
        return x;
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() - 1)
    } else {
        f32::from_bits(x.to_bits() + 1)
    }
}
