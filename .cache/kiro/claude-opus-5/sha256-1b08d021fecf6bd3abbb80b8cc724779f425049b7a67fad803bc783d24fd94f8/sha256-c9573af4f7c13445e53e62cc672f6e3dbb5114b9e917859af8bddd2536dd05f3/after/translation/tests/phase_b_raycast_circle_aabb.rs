//! Phase B rows 25–42: `c2RaytoCircle` and `c2RaytoAABB`.
//!
//! Every row compares the returned `int` *and* all 12 bytes of the
//! `c2Raycast *out` buffer (pre-filled with `0xA5`), so "left untouched" is
//! distinguished from "written with the same value".

mod common;
use common::*;

const N: usize = 20_000;

// ===========================================================================
// c2RaytoCircle — rows 25–30
// ===========================================================================

/// Recompute the C's own root using the C's own exported primitives, so the
/// value is bit-identical to what `c2RaytoCircle` will compute internally.
fn c_root(c: &Impl, ray: c2Ray, s: c2Circle) -> f32 {
    let m = unsafe { (c.c2Sub)(ray.p, s.p) };
    let cc = unsafe { (c.c2Dot)(m, m) } - s.r * s.r;
    let b = unsafe { (c.c2Dot)(m, ray.d) };
    let disc = b * b - cc;
    -b - disc.sqrt()
}

#[test]
fn row25_circle_hit() {
    let (c, r) = pair();
    let mut d = Diff::new("25: c2RaytoCircle hit, origin outside, normalised d");
    let mut g = Rng::new(0x2501);
    for _ in 0..N {
        let centre = g.v(50.0);
        let rad = 0.01 + g.unit() * 20.0;
        // Aim from a point outside towards a random point inside the circle.
        let dir = g.dir();
        let dist = rad * (1.5 + g.unit() * 10.0);
        let origin = c2v {
            x: centre.x - dir.x * dist,
            y: centre.y - dir.y * dist,
        };
        // perturb the aim so some rays miss
        let jitter = g.sym(rad * 1.5);
        let aim = c2v {
            x: dir.x - dir.y * jitter / dist,
            y: dir.y + dir.x * jitter / dist,
        };
        let nd = unsafe { (c.c2Norm)(aim) };
        let ray = c2Ray {
            p: origin,
            d: nd,
            t: dist * (0.5 + g.unit() * 1.5),
        };
        cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: rad });
    }
    d.finish();
}

#[test]
fn row26_circle_t_exactly_at_ray_length() {
    let (c, r) = pair();
    let mut d = Diff::new("26: c2RaytoCircle with A.t == the computed root (inclusive <=)");
    let mut g = Rng::new(0x2601);
    let mut exercised = 0usize;
    for _ in 0..N {
        let centre = g.v(50.0);
        let rad = 0.01 + g.unit() * 20.0;
        let dir = g.dir();
        let dist = rad * (1.5 + g.unit() * 10.0);
        let origin = c2v {
            x: centre.x - dir.x * dist,
            y: centre.y - dir.y * dist,
        };
        let probe = c2Ray {
            p: origin,
            d: dir,
            t: 0.0,
        };
        let s = c2Circle { p: centre, r: rad };
        let t = c_root(c, probe, s);
        if !t.is_finite() {
            continue;
        }
        exercised += 1;
        // exactly at, one ULP below, one ULP above
        for tt in [
            t,
            f32::from_bits(t.to_bits().wrapping_sub(1)),
            f32::from_bits(t.to_bits().wrapping_add(1)),
            0.0,
            -0.0,
        ] {
            cmp_ray_circle(&mut d, c, r, c2Ray { p: origin, d: dir, t: tt }, s);
        }
    }
    assert!(exercised > N / 2, "boundary construction rarely applied");
    d.finish();
}

#[test]
fn row27_circle_origin_on_and_inside() {
    let (c, r) = pair();
    let mut d = Diff::new("27: c2RaytoCircle origin on / inside the circle");
    let mut g = Rng::new(0x2701);
    for i in 0..N {
        let centre = g.v(50.0);
        let rad = 0.01 + g.unit() * 20.0;
        let ang = g.unit() * std::f32::consts::TAU;
        let scale = match i % 3 {
            0 => 1.0,               // exactly on the surface
            1 => g.unit() * 0.999,  // strictly inside
            _ => 0.0,               // at the centre
        };
        let origin = c2v {
            x: centre.x + ang.cos() * rad * scale,
            y: centre.y + ang.sin() * rad * scale,
        };
        let dir = g.dir();
        let ray = c2Ray {
            p: origin,
            d: dir,
            t: rad * (0.1 + g.unit() * 5.0),
        };
        cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: rad });
    }
    d.finish();
}

#[test]
fn row28_circle_tangent_sweep() {
    let (c, r) = pair();
    let mut d = Diff::new("28: c2RaytoCircle tangent sweep (disc straddles 0)");
    let mut g = Rng::new(0x2801);
    for _ in 0..N / 4 {
        let centre = g.v(20.0);
        let rad = 0.5 + g.unit() * 5.0;
        let ang = g.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: ang.cos(),
            y: ang.sin(),
        };
        let normal = c2v { x: -dir.y, y: dir.x };
        let dist = rad * (2.0 + g.unit() * 5.0);
        // Sweep the perpendicular offset through exactly +-rad.
        for k in -4i32..=4 {
            let off = rad * (1.0 + k as f32 * f32::EPSILON * 4.0);
            let origin = c2v {
                x: centre.x - dir.x * dist + normal.x * off,
                y: centre.y - dir.y * dist + normal.y * off,
            };
            let ray = c2Ray {
                p: origin,
                d: dir,
                t: dist * 2.0,
            };
            cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: rad });
        }
    }
    d.finish();
}

#[test]
fn row29_circle_non_normalised_direction() {
    let (c, r) = pair();
    let mut d = Diff::new("29: c2RaytoCircle non-normalised / zero direction");
    let mut g = Rng::new(0x2901);
    for i in 0..N {
        let centre = g.v(30.0);
        let rad = 0.1 + g.unit() * 10.0;
        let origin = g.v(60.0);
        let dir = match i % 5 {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => c2v { x: -0.0, y: -0.0 },
            2 => {
                let u = g.dir();
                let k = 1e-6 + g.unit() * 1e3;
                c2v { x: u.x * k, y: u.y * k }
            }
            3 => g.v(1e6),
            _ => g.v(1e-6),
        };
        let ray = c2Ray {
            p: origin,
            d: dir,
            t: g.sym(1e3),
        };
        cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: rad });
    }
    d.finish();
}

#[test]
fn row30_circle_random_and_hostile() {
    let (c, r) = pair();
    let mut d = Diff::new("30: c2RaytoCircle fully random incl. special classes");
    let mut g = Rng::new(0x3001);
    for _ in 0..N * 3 {
        let (ray, s) = match g.below(4) {
            0 => (
                c2Ray {
                    p: g.v(20.0),
                    d: g.dir(),
                    t: g.unit() * 60.0,
                },
                c2Circle {
                    p: g.v(20.0),
                    r: g.unit() * 15.0,
                },
            ),
            1 => (
                c2Ray {
                    p: g.v_special(),
                    d: g.v_special(),
                    t: g.special_f32(),
                },
                c2Circle {
                    p: g.v_special(),
                    r: g.special_f32(),
                },
            ),
            2 => (
                c2Ray {
                    p: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                    d: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                    t: g.any_bits_f32(),
                },
                c2Circle {
                    p: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                    r: g.any_bits_f32(),
                },
            ),
            _ => (
                c2Ray {
                    p: g.v_mixed(1e3),
                    d: g.v_mixed(1e3),
                    t: g.mixed_f32(1e3),
                },
                c2Circle {
                    p: g.v_mixed(1e3),
                    r: g.mixed_f32(1e3),
                },
            ),
        };
        cmp_ray_circle(&mut d, c, r, ray, s);
    }
    d.finish();
}

// ===========================================================================
// c2RaytoAABB — rows 31–42
// ===========================================================================

fn unit_box(g: &mut Rng) -> c2AABB {
    let centre = g.v(20.0);
    let hx = 0.1 + g.unit() * 8.0;
    let hy = 0.1 + g.unit() * 8.0;
    c2AABB {
        min: c2v {
            x: centre.x - hx,
            y: centre.y - hy,
        },
        max: c2v {
            x: centre.x + hx,
            y: centre.y + hy,
        },
    }
}

/// Rows 31–34: enter through each of the four faces. The C's tie-break chain
/// picks `(-1,0)` / `(1,0)` / `(0,-1)` / `(0,1)`; we do not assert which, only
/// that C and Rust agree, but the four setups guarantee all four arms run.
#[test]
fn row31_34_aabb_each_face() {
    let (c, r) = pair();
    let mut d = Diff::new("31-34: c2RaytoAABB entry through each face");
    let mut g = Rng::new(0x3101);
    for i in 0..N * 2 {
        let b = unit_box(&mut g);
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        // a random point on the target face, slightly inset so the hit is
        // unambiguous, plus a small chance of landing outside
        let u = g.unit() * 1.2 - 0.1;
        let (start, dir) = match i % 4 {
            0 => (
                c2v {
                    x: b.min.x - w * (0.5 + g.unit()),
                    y: b.min.y + h * u,
                },
                c2v { x: 1.0, y: 0.0 },
            ),
            1 => (
                c2v {
                    x: b.max.x + w * (0.5 + g.unit()),
                    y: b.min.y + h * u,
                },
                c2v { x: -1.0, y: 0.0 },
            ),
            2 => (
                c2v {
                    x: b.min.x + w * u,
                    y: b.min.y - h * (0.5 + g.unit()),
                },
                c2v { x: 0.0, y: 1.0 },
            ),
            _ => (
                c2v {
                    x: b.min.x + w * u,
                    y: b.max.y + h * (0.5 + g.unit()),
                },
                c2v { x: 0.0, y: -1.0 },
            ),
        };
        let _ = (cx, cy);
        // also try slightly off-axis directions
        let dir = if g.below(2) == 0 {
            dir
        } else {
            unsafe { (c.c2Norm)(c2v { x: dir.x + g.sym(0.3), y: dir.y + g.sym(0.3) }) }
        };
        let len = (w + h) * (1.0 + g.unit() * 3.0);
        cmp_ray_aabb(&mut d, c, r, c2Ray { p: start, d: dir, t: len }, b);
    }
    d.finish();
}

#[test]
fn row35_aabb_exact_corner() {
    let (c, r) = pair();
    let mut d = Diff::new("35: c2RaytoAABB exact corner hit (tie in the chain)");
    let mut g = Rng::new(0x3501);
    for i in 0..N {
        let b = unit_box(&mut g);
        let corner = match i % 4 {
            0 => c2v { x: b.min.x, y: b.min.y },
            1 => c2v { x: b.max.x, y: b.min.y },
            2 => c2v { x: b.min.x, y: b.max.y },
            _ => c2v { x: b.max.x, y: b.max.y },
        };
        // aim exactly at the corner from a symmetric diagonal offset
        let k = 1.0 + g.unit() * 4.0;
        let sx = if corner.x == b.min.x { -1.0 } else { 1.0 };
        let sy = if corner.y == b.min.y { -1.0 } else { 1.0 };
        let start = c2v {
            x: corner.x + sx * k,
            y: corner.y + sy * k,
        };
        let dir = unsafe { (c.c2Norm)(c2v { x: -sx, y: -sy }) };
        for t in [k * 1.5, k * std::f32::consts::SQRT_2, k * 0.5, k * 4.0] {
            cmp_ray_aabb(&mut d, c, r, c2Ray { p: start, d: dir, t }, b);
        }
    }
    d.finish();
}

#[test]
fn row36_aabb_axis_aligned() {
    let (c, r) = pair();
    let mut d = Diff::new("36: c2RaytoAABB perfectly axis-aligned rays (d==0 plane branch)");
    let mut g = Rng::new(0x3601);
    const DIRS: &[c2v] = &[
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
    ];
    for i in 0..N * 2 {
        let b = unit_box(&mut g);
        let dir = DIRS[i % DIRS.len()];
        // start somewhere on the axis lines through the box, and off them
        let start = match g.below(3) {
            0 => c2v {
                x: b.min.x - 3.0,
                y: (b.min.y + b.max.y) * 0.5,
            },
            1 => c2v {
                x: (b.min.x + b.max.x) * 0.5,
                y: b.min.y - 3.0,
            },
            _ => g.v(30.0),
        };
        let t = [0.0f32, 1.0, 5.0, 50.0, -1.0][i % 5];
        cmp_ray_aabb(&mut d, c, r, c2Ray { p: start, d: dir, t }, b);
    }
    d.finish();
}

#[test]
fn row37_38_aabb_origin_inside_and_zero_length() {
    let (c, r) = pair();
    let mut d = Diff::new("37/38: c2RaytoAABB origin inside, and A.t == 0");
    let mut g = Rng::new(0x3701);
    for i in 0..N * 2 {
        let b = unit_box(&mut g);
        let inside = c2v {
            x: b.min.x + (b.max.x - b.min.x) * g.unit(),
            y: b.min.y + (b.max.y - b.min.y) * g.unit(),
        };
        let p = if i % 3 == 0 { g.v(30.0) } else { inside };
        let t = [0.0f32, -0.0, 1e-30, 1.0, 20.0][i % 5];
        let dir = if g.below(2) == 0 { g.dir() } else { g.v(2.0) };
        cmp_ray_aabb(&mut d, c, r, c2Ray { p, d: dir, t }, b);
    }
    d.finish();
}

#[test]
fn row39_aabb_corner_skim_reject() {
    let (c, r) = pair();
    let mut d = Diff::new("39: c2RaytoAABB separating-axis reject (d > 0)");
    let mut g = Rng::new(0x3901);
    for i in 0..N {
        let b = unit_box(&mut g);
        let corner = match i % 4 {
            0 => c2v { x: b.min.x, y: b.min.y },
            1 => c2v { x: b.max.x, y: b.min.y },
            2 => c2v { x: b.min.x, y: b.max.y },
            _ => c2v { x: b.max.x, y: b.max.y },
        };
        // A diagonal ray whose supporting line passes just outside the corner,
        // but whose swept AABB still overlaps the box.
        let dir = unsafe { (c.c2Norm)(c2v { x: 1.0, y: 1.0 }) };
        let n = c2v { x: -dir.y, y: dir.x };
        let eps = g.sym(0.5) + if g.below(2) == 0 { 0.5 } else { -0.5 };
        let k = 2.0 + g.unit() * 3.0;
        let start = c2v {
            x: corner.x - dir.x * k + n.x * eps,
            y: corner.y - dir.y * k + n.y * eps,
        };
        cmp_ray_aabb(
            &mut d,
            c,
            r,
            c2Ray { p: start, d: dir, t: k * 2.0 },
            b,
        );
    }
    d.finish();
}

#[test]
fn row40_aabb_fully_random() {
    let (c, r) = pair();
    let mut d = Diff::new("40: c2RaytoAABB uniform random over a region larger than B");
    let mut g = Rng::new(0x4001);
    for _ in 0..N * 4 {
        let b = unit_box(&mut g);
        let ray = c2Ray {
            p: g.v(40.0),
            d: if g.below(2) == 0 { g.dir() } else { g.v(3.0) },
            t: g.unit() * 80.0,
        };
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    d.finish();
}

#[test]
fn row41_42_aabb_degenerate_and_hostile() {
    let (c, r) = pair();
    let mut d = Diff::new("41/42: c2RaytoAABB degenerate / inverted / inf / NaN boxes");
    let mut g = Rng::new(0x4101);
    for _ in 0..N * 3 {
        let b = match g.below(6) {
            0 => {
                let q = g.v(10.0);
                c2AABB { min: q, max: q }
            }
            1 => {
                let q = g.v(10.0);
                c2AABB {
                    min: q,
                    max: c2v { x: q.x, y: q.y + 5.0 },
                }
            }
            2 => {
                let a = unit_box(&mut g);
                c2AABB { min: a.max, max: a.min }
            }
            3 => c2AABB {
                min: g.v_special(),
                max: g.v_special(),
            },
            4 => c2AABB {
                min: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                max: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
            },
            _ => g.aabb(1e30),
        };
        let ray = match g.below(3) {
            0 => c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.unit() * 40.0,
            },
            1 => c2Ray {
                p: g.v_special(),
                d: g.v_special(),
                t: g.special_f32(),
            },
            _ => c2Ray {
                p: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                d: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                t: g.any_bits_f32(),
            },
        };
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    d.finish();
}
