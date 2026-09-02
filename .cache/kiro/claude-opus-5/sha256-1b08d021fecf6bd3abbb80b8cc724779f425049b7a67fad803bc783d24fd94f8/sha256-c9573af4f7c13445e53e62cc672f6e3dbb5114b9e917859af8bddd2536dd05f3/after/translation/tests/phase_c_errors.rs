//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejecting condition, calls BOTH `.so`s, and
//! asserts the same rejection: the same returned `int` **and** the same 12 out
//! bytes (so "returned 0 and left `*out` alone" is distinguished from
//! "returned 0 and clobbered `*out`"). Where the C's rejection is an IEEE
//! sentinel rather than a status code, the sentinel's exact bit pattern is
//! compared, so `+inf` vs `-inf` vs `NaN` vs `-NaN` are all distinct.

mod common;
use common::*;
use std::ffi::{c_uint, c_void};

const M: usize = 5_000;

// ===========================================================================
// Rows 1–6: c2RaytoCircle rejections
// ===========================================================================

/// Row 1: `disc < 0` — the ray's supporting line misses the circle entirely.
#[test]
fn e01_circle_disc_negative() {
    let (c, r) = pair();
    let mut d = Diff::new("E1: c2RaytoCircle disc < 0");
    let mut g = Rng::new(0xE001);
    let mut hits = 0usize;
    for _ in 0..M * 4 {
        let centre = g.v(30.0);
        let rad = 0.1 + g.unit() * 5.0;
        let u = g.dir();
        let n = c2v { x: -u.y, y: u.x };
        // offset the line by strictly more than the radius
        let off = rad * (1.5 + g.unit() * 10.0) * if g.below(2) == 0 { 1.0 } else { -1.0 };
        let dist = rad * (2.0 + g.unit() * 10.0);
        let p = c2v {
            x: centre.x - u.x * dist + n.x * off,
            y: centre.y - u.y * dist + n.y * off,
        };
        let ray = c2Ray {
            p,
            d: u,
            t: dist * 4.0,
        };
        let s = c2Circle { p: centre, r: rad };
        // sanity: this configuration really must be a miss in the C
        let mut cb = OutBuf::filled();
        if unsafe { (c.c2RaytoCircle)(ray, s, cb.as_ptr()) } == 0 && cb == OUT_FILL {
            hits += 1;
        }
        cmp_ray_circle(&mut d, c, r, ray, s);
    }
    assert!(
        hits > M * 3,
        "the disc<0 construction did not reliably reject ({hits} of {})",
        M * 4
    );
    d.finish();
}

/// Row 2: `disc >= 0` but the near root is negative (circle behind the origin,
/// or the origin strictly inside).
#[test]
fn e02_circle_root_behind_origin() {
    let (c, r) = pair();
    let mut d = Diff::new("E2: c2RaytoCircle t < 0");
    let mut g = Rng::new(0xE002);
    let mut rejected = 0usize;
    for i in 0..M * 4 {
        let centre = g.v(30.0);
        let rad = 0.1 + g.unit() * 5.0;
        let u = g.dir();
        let dist = rad * (2.0 + g.unit() * 8.0);
        // aim AWAY from the circle => both roots behind
        let (p, dir) = if i % 2 == 0 {
            (
                c2v {
                    x: centre.x - u.x * dist,
                    y: centre.y - u.y * dist,
                },
                c2v { x: -u.x, y: -u.y },
            )
        } else {
            // origin strictly inside => near root negative
            let k = g.unit() * 0.9;
            (
                c2v {
                    x: centre.x + u.x * rad * k,
                    y: centre.y + u.y * rad * k,
                },
                g.dir(),
            )
        };
        let ray = c2Ray {
            p,
            d: dir,
            t: dist * 4.0,
        };
        let s = c2Circle { p: centre, r: rad };
        let mut cb = OutBuf::filled();
        if unsafe { (c.c2RaytoCircle)(ray, s, cb.as_ptr()) } == 0 {
            rejected += 1;
        }
        cmp_ray_circle(&mut d, c, r, ray, s);
    }
    assert!(rejected > M * 3, "t<0 construction rarely rejected");
    d.finish();
}

/// Row 3: a real intersection exists but lies past `A.t`.
#[test]
fn e03_circle_hit_beyond_ray_length() {
    let (c, r) = pair();
    let mut d = Diff::new("E3: c2RaytoCircle t > A.t");
    let mut g = Rng::new(0xE003);
    let mut rejected = 0usize;
    for _ in 0..M * 4 {
        let centre = g.v(30.0);
        let rad = 0.1 + g.unit() * 5.0;
        let u = g.dir();
        let dist = rad * (3.0 + g.unit() * 8.0);
        let p = c2v {
            x: centre.x - u.x * dist,
            y: centre.y - u.y * dist,
        };
        // stop the ray well before the near surface
        let ray = c2Ray {
            p,
            d: u,
            t: (dist - rad) * g.unit() * 0.9,
        };
        let s = c2Circle { p: centre, r: rad };
        let mut cb = OutBuf::filled();
        if unsafe { (c.c2RaytoCircle)(ray, s, cb.as_ptr()) } == 0 && cb == OUT_FILL {
            rejected += 1;
        }
        cmp_ray_circle(&mut d, c, r, ray, s);
    }
    assert!(rejected > M * 3, "t>A.t construction rarely rejected");
    d.finish();
}

/// Row 4: `disc` is `NaN`, so `disc < 0` is false and `sqrtf(NaN)` is reached.
#[test]
fn e04_circle_nan_discriminant() {
    let (c, r) = pair();
    let mut d = Diff::new("E4: c2RaytoCircle NaN discriminant");
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let cases: &[(c2Ray, c2Circle)] = &[
        (
            c2Ray {
                p: c2v { x: nan, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: nan, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: inf, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: inf, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: inf, y: 0.0 },
                t: 0.0,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: inf,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: nan,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: nan,
            },
        ),
    ];
    for (ray, s) in cases {
        // The C must reject (NaN comparisons are all false).
        let mut cb = OutBuf::filled();
        let got = unsafe { (c.c2RaytoCircle)(*ray, *s, cb.as_ptr()) };
        assert_eq!(got, 0, "expected the C to reject {}", fray(*ray));
        assert_eq!(cb, OUT_FILL, "C wrote *out on a NaN rejection");
        cmp_ray_circle(&mut d, c, r, *ray, *s);
    }
    // plus randomized NaN/inf injection into every slot
    let mut g = Rng::new(0xE004);
    for _ in 0..M * 4 {
        let mut ray = c2Ray {
            p: g.v(20.0),
            d: g.dir(),
            t: g.unit() * 40.0,
        };
        let mut s = c2Circle {
            p: g.v(20.0),
            r: g.unit() * 10.0,
        };
        let poison = if g.below(2) == 0 { nan } else { inf };
        let poison = if g.below(2) == 0 { poison } else { -poison };
        match g.below(7) {
            0 => ray.p.x = poison,
            1 => ray.p.y = poison,
            2 => ray.d.x = poison,
            3 => ray.d.y = poison,
            4 => ray.t = poison,
            5 => s.p.x = poison,
            _ => s.r = poison,
        }
        cmp_ray_circle(&mut d, c, r, ray, s);
    }
    d.finish();
}

/// Row 5: negative radius — never validated, and `r*r` erases the sign.
#[test]
fn e05_circle_negative_radius() {
    let (c, r) = pair();
    let mut d = Diff::new("E5: c2RaytoCircle negative radius");
    let mut g = Rng::new(0xE005);
    for _ in 0..M * 4 {
        let centre = g.v(30.0);
        let rad = -(0.05 + g.unit() * 10.0);
        let u = g.dir();
        let dist = rad.abs() * (1.0 + g.unit() * 8.0);
        let p = c2v {
            x: centre.x - u.x * dist,
            y: centre.y - u.y * dist,
        };
        let ray = c2Ray {
            p,
            d: u,
            t: dist * (0.5 + g.unit() * 2.0),
        };
        cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: rad });
    }
    // exact -0.0 radius too
    for _ in 0..M {
        let centre = g.v(10.0);
        let ray = c2Ray {
            p: g.v(20.0),
            d: g.dir(),
            t: g.unit() * 40.0,
        };
        cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: -0.0 });
        cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: centre, r: 0.0 });
    }
    d.finish();
}

/// Row 6: negative `A.t` — no root can satisfy `t >= 0 && t <= A.t`.
#[test]
fn e06_circle_negative_ray_length() {
    let (c, r) = pair();
    let mut d = Diff::new("E6: c2RaytoCircle A.t < 0");
    let mut g = Rng::new(0xE006);
    let mut rejected = 0usize;
    for _ in 0..M * 4 {
        let centre = g.v(30.0);
        let rad = 0.1 + g.unit() * 5.0;
        let u = g.dir();
        let dist = rad * (2.0 + g.unit() * 8.0);
        let p = c2v {
            x: centre.x - u.x * dist,
            y: centre.y - u.y * dist,
        };
        let t = -(g.unit() * 100.0) - f32::MIN_POSITIVE;
        let ray = c2Ray { p, d: u, t };
        let s = c2Circle { p: centre, r: rad };
        let mut cb = OutBuf::filled();
        if unsafe { (c.c2RaytoCircle)(ray, s, cb.as_ptr()) } == 0 {
            rejected += 1;
        }
        cmp_ray_circle(&mut d, c, r, ray, s);
    }
    assert_eq!(rejected, M * 4, "a negative A.t must always reject");
    d.finish();
}

// ===========================================================================
// Rows 7–12: c2AABBtoAABB rejections
// ===========================================================================

/// Rows 7–10: each of `d0`..`d3` in isolation.
#[test]
fn e07_10_aabb_aabb_each_separating_axis() {
    let (c, r) = pair();
    let mut g = Rng::new(0xE007);
    for which in 0..4u32 {
        let mut d = Diff::new(format!("E{}: c2AABBtoAABB d{which}", 7 + which));
        let mut rejected = 0usize;
        for _ in 0..M * 2 {
            let a = g.aabb(10.0);
            let gap = f32::MIN_POSITIVE + g.unit() * 5.0;
            let b = match which {
                0 => c2AABB {
                    min: c2v { x: a.min.x - gap - 1.0, y: a.min.y - 1.0 },
                    max: c2v { x: a.min.x - gap, y: a.max.y + 1.0 },
                },
                1 => c2AABB {
                    min: c2v { x: a.max.x + gap, y: a.min.y - 1.0 },
                    max: c2v { x: a.max.x + gap + 1.0, y: a.max.y + 1.0 },
                },
                2 => c2AABB {
                    min: c2v { x: a.min.x - 1.0, y: a.min.y - gap - 1.0 },
                    max: c2v { x: a.max.x + 1.0, y: a.min.y - gap },
                },
                _ => c2AABB {
                    min: c2v { x: a.min.x - 1.0, y: a.max.y + gap },
                    max: c2v { x: a.max.x + 1.0, y: a.max.y + gap + 1.0 },
                },
            };
            let cv = unsafe { (c.c2AABBtoAABB)(a, b) };
            let rv = unsafe { (r.c2AABBtoAABB)(a, b) };
            if cv == 0 {
                rejected += 1;
            }
            d.eq(|| format!("d{which} {} {}", fbox(a), fbox(b)), cv, rv);
        }
        assert!(
            rejected > M * 2 - M / 5,
            "d{which} construction rarely rejected ({rejected})"
        );
        d.finish();
    }
}

/// Row 11: any `NaN` coordinate makes all four `<` false, so the C *accepts*.
#[test]
fn e11_aabb_aabb_nan_accepts() {
    let (c, r) = pair();
    let mut d = Diff::new("E11: c2AABBtoAABB NaN accepts");
    let nan = f32::NAN;
    let base = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let far = c2AABB {
        min: c2v { x: 100.0, y: 100.0 },
        max: c2v { x: 101.0, y: 101.0 },
    };
    // Poison one coordinate at a time in each of the 8 slots, on each side.
    for slot in 0..8 {
        for swap in [false, true] {
            let mut a = base;
            let mut b = far;
            {
                let t = if swap { &mut b } else { &mut a };
                match slot {
                    0 => t.min.x = nan,
                    1 => t.min.y = nan,
                    2 => t.max.x = nan,
                    3 => t.max.y = nan,
                    4 => t.min.x = -nan,
                    5 => t.min.y = -nan,
                    6 => t.max.x = -nan,
                    _ => t.max.y = -nan,
                }
            }
            let cv = unsafe { (c.c2AABBtoAABB)(a, b) };
            let rv = unsafe { (r.c2AABBtoAABB)(a, b) };
            d.eq(|| format!("slot{slot} swap{swap} {} {}", fbox(a), fbox(b)), cv, rv);
        }
    }
    // Full NaN boxes.
    let allnan = c2AABB {
        min: c2v { x: nan, y: nan },
        max: c2v { x: nan, y: nan },
    };
    for (a, b) in [(allnan, base), (base, allnan), (allnan, allnan)] {
        d.eq(
            || format!("allnan {} {}", fbox(a), fbox(b)),
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
        );
    }
    d.finish();
}

/// Row 12: inverted boxes (`min > max`) are never validated.
#[test]
fn e12_aabb_aabb_inverted() {
    let (c, r) = pair();
    let mut d = Diff::new("E12: c2AABBtoAABB inverted boxes");
    let mut g = Rng::new(0xE012);
    for i in 0..M * 6 {
        let p = g.aabb(10.0);
        let q = g.aabb(10.0);
        let (a, b) = match i % 3 {
            0 => (c2AABB { min: p.max, max: p.min }, q),
            1 => (p, c2AABB { min: q.max, max: q.min }),
            _ => (
                c2AABB { min: p.max, max: p.min },
                c2AABB { min: q.max, max: q.min },
            ),
        };
        d.eq(
            || format!("inv {} {}", fbox(a), fbox(b)),
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
        );
    }
    d.finish();
}

// ===========================================================================
// Rows 13–18: c2AABBtoPoint rejections
// ===========================================================================

/// Rows 13–16: each of `d0`..`d3` in isolation.
#[test]
fn e13_16_aabb_point_each_side() {
    let (c, r) = pair();
    let mut g = Rng::new(0xE013);
    for which in 0..4u32 {
        let mut d = Diff::new(format!("E{}: c2AABBtoPoint d{which}", 13 + which));
        let mut rejected = 0usize;
        for _ in 0..M * 2 {
            let b = g.aabb(10.0);
            let gap = f32::MIN_POSITIVE + g.unit() * 5.0;
            let p = match which {
                0 => c2v { x: b.min.x - gap, y: b.min.y },
                1 => c2v { x: b.min.x, y: b.min.y - gap },
                2 => c2v { x: b.max.x + gap, y: b.max.y },
                _ => c2v { x: b.max.x, y: b.max.y + gap },
            };
            let cv = unsafe { (c.c2AABBtoPoint)(b, p) };
            let rv = unsafe { (r.c2AABBtoPoint)(b, p) };
            if cv == 0 {
                rejected += 1;
            }
            d.eq(|| format!("d{which} {} {}", fbox(b), fv(p)), cv, rv);
        }
        assert!(rejected > M, "d{which} rarely rejected ({rejected})");
        d.finish();
    }
}

/// Row 17: a point exactly on a face / corner is INSIDE (strict comparisons).
#[test]
fn e17_aabb_point_on_boundary_accepts() {
    let (c, r) = pair();
    let mut d = Diff::new("E17: c2AABBtoPoint boundary accepts");
    let mut g = Rng::new(0xE017);
    let mut accepted = 0usize;
    for i in 0..M * 4 {
        let b = g.aabb(10.0);
        let p = match i % 6 {
            0 => b.min,
            1 => b.max,
            2 => c2v { x: b.min.x, y: b.max.y },
            3 => c2v { x: b.max.x, y: b.min.y },
            4 => c2v {
                x: b.min.x,
                y: b.min.y + (b.max.y - b.min.y) * g.unit(),
            },
            _ => c2v {
                x: b.min.x + (b.max.x - b.min.x) * g.unit(),
                y: b.max.y,
            },
        };
        let cv = unsafe { (c.c2AABBtoPoint)(b, p) };
        let rv = unsafe { (r.c2AABBtoPoint)(b, p) };
        if cv == 1 {
            accepted += 1;
        }
        d.eq(|| format!("boundary {} {}", fbox(b), fv(p)), cv, rv);
    }
    assert!(accepted > M * 3, "boundary points should be accepted");
    d.finish();
}

/// Row 18: `NaN` point coordinates make all four comparisons false → accept.
#[test]
fn e18_aabb_point_nan_accepts() {
    let (c, r) = pair();
    let mut d = Diff::new("E18: c2AABBtoPoint NaN accepts");
    let b = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for p in [
        c2v { x: f32::NAN, y: 0.5 },
        c2v { x: 0.5, y: f32::NAN },
        c2v { x: f32::NAN, y: f32::NAN },
        c2v { x: -f32::NAN, y: -f32::NAN },
        c2v { x: f32::from_bits(0x7F80_0001), y: 0.5 },
    ] {
        d.eq(
            || format!("nanpoint {} {}", fbox(b), fv(p)),
            unsafe { (c.c2AABBtoPoint)(b, p) },
            unsafe { (r.c2AABBtoPoint)(b, p) },
        );
    }
    d.finish();
}

// ===========================================================================
// Rows 19–23: c2CircleToPoint rejections
// ===========================================================================

#[test]
fn e19_20_circle_point_outside_and_on() {
    let (c, r) = pair();
    let mut d = Diff::new("E19/E20: c2CircleToPoint outside / exactly on (strict <)");
    let mut g = Rng::new(0xE019);
    let mut rejected = 0usize;
    for i in 0..M * 4 {
        let cir = c2Circle {
            p: g.v(20.0),
            r: 0.1 + g.unit() * 8.0,
        };
        let ang = g.unit() * std::f32::consts::TAU;
        let k = if i % 2 == 0 { 1.0 } else { 1.0 + g.unit() * 4.0 };
        let p = c2v {
            x: cir.p.x + ang.cos() * cir.r * k,
            y: cir.p.y + ang.sin() * cir.r * k,
        };
        let cv = unsafe { (c.c2CircleToPoint)(cir, p) };
        let rv = unsafe { (r.c2CircleToPoint)(cir, p) };
        if cv == 0 {
            rejected += 1;
        }
        d.eq(|| format!("{} {}", fcircle(cir), fv(p)), cv, rv);
    }
    assert!(rejected > M * 2, "outside/on points should mostly reject");

    // A construction where d2 == r*r exactly: axis-aligned offset by r.
    for _ in 0..M {
        let cir = c2Circle {
            p: g.v(20.0),
            r: 0.5 + g.unit() * 8.0,
        };
        for p in [
            c2v { x: cir.p.x + cir.r, y: cir.p.y },
            c2v { x: cir.p.x - cir.r, y: cir.p.y },
            c2v { x: cir.p.x, y: cir.p.y + cir.r },
            c2v { x: cir.p.x, y: cir.p.y - cir.r },
        ] {
            d.eq(
                || format!("exact-on {} {}", fcircle(cir), fv(p)),
                unsafe { (c.c2CircleToPoint)(cir, p) },
                unsafe { (r.c2CircleToPoint)(cir, p) },
            );
        }
    }
    d.finish();
}

/// Row 21: `r == 0` rejects every point, including the exact centre.
#[test]
fn e21_circle_point_zero_radius() {
    let (c, r) = pair();
    let mut d = Diff::new("E21: c2CircleToPoint r == 0 rejects everything");
    let mut g = Rng::new(0xE021);
    for _ in 0..M * 2 {
        let q = g.v(20.0);
        for rad in [0.0f32, -0.0] {
            let cir = c2Circle { p: q, r: rad };
            for p in [q, g.v(20.0), c2v { x: q.x, y: q.y }] {
                let cv = unsafe { (c.c2CircleToPoint)(cir, p) };
                let rv = unsafe { (r.c2CircleToPoint)(cir, p) };
                assert_eq!(cv, 0, "zero radius must reject in C");
                d.eq(|| format!("{} {}", fcircle(cir), fv(p)), cv, rv);
            }
        }
    }
    d.finish();
}

/// Row 22: a negative radius behaves like its magnitude (`r*r > 0`).
#[test]
fn e22_circle_point_negative_radius() {
    let (c, r) = pair();
    let mut d = Diff::new("E22: c2CircleToPoint r < 0 behaves like |r|");
    let mut g = Rng::new(0xE022);
    let mut accepted = 0usize;
    for _ in 0..M * 4 {
        let cir = c2Circle {
            p: g.v(20.0),
            r: -(0.1 + g.unit() * 8.0),
        };
        let ang = g.unit() * std::f32::consts::TAU;
        let p = c2v {
            x: cir.p.x + ang.cos() * cir.r.abs() * g.unit() * 0.9,
            y: cir.p.y + ang.sin() * cir.r.abs() * g.unit() * 0.9,
        };
        let cv = unsafe { (c.c2CircleToPoint)(cir, p) };
        let rv = unsafe { (r.c2CircleToPoint)(cir, p) };
        if cv == 1 {
            accepted += 1;
        }
        d.eq(|| format!("{} {}", fcircle(cir), fv(p)), cv, rv);
    }
    assert!(accepted > M, "negative radius should still accept interiors");
    d.finish();
}

/// Row 23: `NaN` anywhere makes `d2 < r*r` false → reject.
#[test]
fn e23_circle_point_nan() {
    let (c, r) = pair();
    let mut d = Diff::new("E23: c2CircleToPoint NaN rejects");
    let nan = f32::NAN;
    let cases = [
        (
            c2Circle { p: c2v { x: nan, y: 0.0 }, r: 1.0 },
            c2v { x: 0.0, y: 0.0 },
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: nan },
            c2v { x: 0.0, y: 0.0 },
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2v { x: nan, y: 0.0 },
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2v { x: 0.0, y: nan },
        ),
        (
            c2Circle { p: c2v { x: f32::INFINITY, y: 0.0 }, r: f32::INFINITY },
            c2v { x: f32::INFINITY, y: 0.0 },
        ),
    ];
    for (cir, p) in cases {
        let cv = unsafe { (c.c2CircleToPoint)(cir, p) };
        let rv = unsafe { (r.c2CircleToPoint)(cir, p) };
        d.eq(|| format!("{} {}", fcircle(cir), fv(p)), cv, rv);
    }
    d.finish();
}

// ===========================================================================
// Rows 24–26: the static helper's three rejecting arms, reached through
// c2RaytoAABB. `c2RayToPlane_OneDimensional` is `static` and therefore not
// exported, so each arm is driven by the geometry that forces it.
// ===========================================================================

/// Row 24 (`da < 0`) and row 26 (`da == db`, the guarded division).
/// A ray exactly parallel to a pair of planes makes `da == db` for that pair,
/// so `d == 0` and the helper must return `0.0f` rather than dividing.
#[test]
fn e24_26_ray_to_plane_parallel_and_negative_da() {
    let (c, r) = pair();
    let mut d = Diff::new("E24/E26: c2RayToPlane_OneDimensional da<0 and d==0 arms");
    let mut g = Rng::new(0xE024);
    for i in 0..M * 8 {
        let b = c2AABB {
            min: g.v(10.0),
            max: c2v { x: 0.0, y: 0.0 },
        };
        let b = c2AABB {
            min: b.min,
            max: c2v {
                x: b.min.x + 0.5 + g.unit() * 8.0,
                y: b.min.y + 0.5 + g.unit() * 8.0,
            },
        };
        // Perfectly axis-parallel rays: p1 == p0 in one coordinate, so
        // da == db for that axis' two planes and the d==0 arm is taken.
        let dir = [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
        ][i % 4];
        // start below/left of the box so some da are negative too
        let p = match i % 3 {
            0 => c2v { x: b.min.x - 3.0, y: (b.min.y + b.max.y) * 0.5 },
            1 => c2v { x: (b.min.x + b.max.x) * 0.5, y: b.min.y - 3.0 },
            _ => c2v {
                x: b.min.x + (b.max.x - b.min.x) * g.unit(),
                y: b.min.y + (b.max.y - b.min.y) * g.unit(),
            },
        };
        for t in [0.0f32, 1e-20, 1.0, 6.0, 100.0] {
            cmp_ray_aabb(&mut d, c, r, c2Ray { p, d: dir, t }, b);
        }
    }
    d.finish();
}

/// Row 25 (`da*db > 0`): both swept endpoints strictly on the same side of a
/// plane, which is the common case for a ray that stops short of the box.
#[test]
fn e25_ray_to_plane_same_side() {
    let (c, r) = pair();
    let mut d = Diff::new("E25: c2RayToPlane_OneDimensional da*db > 0 arm");
    let mut g = Rng::new(0xE025);
    for _ in 0..M * 8 {
        let b = g.aabb(10.0);
        // A short ray entirely on one side of the box in x, but overlapping in y.
        let p = c2v {
            x: b.min.x - 5.0 - g.unit() * 5.0,
            y: b.min.y + (b.max.y - b.min.y) * g.unit(),
        };
        let dir = c2v { x: 1.0, y: g.sym(0.2) };
        let t = g.unit() * 4.0;
        cmp_ray_aabb(&mut d, c, r, c2Ray { p, d: dir, t }, b);
    }
    d.finish();
}

// ===========================================================================
// Rows 27–33: c2RaytoAABB rejections
// ===========================================================================

/// Row 27: the swept-AABB pre-test rejects.
#[test]
fn e27_aabb_swept_box_disjoint() {
    let (c, r) = pair();
    let mut d = Diff::new("E27: c2RaytoAABB swept-AABB disjoint");
    let mut g = Rng::new(0xE027);
    let mut rejected = 0usize;
    for _ in 0..M * 4 {
        let b = g.aabb(5.0);
        // A short ray far away from the box.
        let p = c2v {
            x: b.max.x + 100.0 + g.unit() * 50.0,
            y: b.max.y + 100.0 + g.unit() * 50.0,
        };
        let ray = c2Ray {
            p,
            d: g.dir(),
            t: g.unit() * 5.0,
        };
        let mut cb = OutBuf::filled();
        if unsafe { (c.c2RaytoAABB)(ray, b, cb.as_ptr()) } == 0 && cb == OUT_FILL {
            rejected += 1;
        }
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    assert_eq!(rejected, M * 4, "far-away short rays must all reject");
    d.finish();
}

/// Row 28: the separating-axis test rejects (`d > 0`) even though the swept
/// AABBs overlap — a diagonal ray passing outside a corner.
#[test]
fn e28_aabb_separating_axis() {
    let (c, r) = pair();
    let mut d = Diff::new("E28: c2RaytoAABB separating-axis reject");
    let mut g = Rng::new(0xE028);
    let mut rejected = 0usize;
    let mut swept_overlapped = 0usize;
    for i in 0..M * 6 {
        let b = g.aabb(6.0);
        let corner = match i % 4 {
            0 => c2v { x: b.min.x, y: b.min.y },
            1 => c2v { x: b.max.x, y: b.min.y },
            2 => c2v { x: b.min.x, y: b.max.y },
            _ => c2v { x: b.max.x, y: b.max.y },
        };
        let sx = if corner.x == b.min.x { -1.0f32 } else { 1.0 };
        let sy = if corner.y == b.min.y { -1.0f32 } else { 1.0 };
        // A 45-degree line offset outward past the corner.
        let off = 0.3 + g.unit() * 3.0;
        let k = 4.0 + g.unit() * 4.0;
        let start = c2v {
            x: corner.x + sx * (k + off),
            y: corner.y + sy * (k - off),
        };
        let dir = unsafe { (c.c2Norm)(c2v { x: -sx, y: -sy }) };
        let ray = c2Ray {
            p: start,
            d: dir,
            t: k * 3.0,
        };
        // Confirm the swept AABB DOES overlap (so we really are past the first
        // early-out and testing the second one).
        let p1 = unsafe { (c.c2Add)(ray.p, (c.c2Mulvs)(ray.d, ray.t)) };
        let sweep = c2AABB {
            min: unsafe { (c.c2Minv)(ray.p, p1) },
            max: unsafe { (c.c2Maxv)(ray.p, p1) },
        };
        if unsafe { (c.c2AABBtoAABB)(sweep, b) } != 0 {
            swept_overlapped += 1;
            let mut cb = OutBuf::filled();
            if unsafe { (c.c2RaytoAABB)(ray, b, cb.as_ptr()) } == 0 && cb == OUT_FILL {
                rejected += 1;
            }
        }
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    assert!(
        swept_overlapped > M,
        "construction did not reach the separating-axis test ({swept_overlapped})"
    );
    assert!(
        rejected > M / 2,
        "separating-axis reject rarely observed ({rejected} of {swept_overlapped})"
    );
    d.finish();
}

/// Row 29: `hit == 0` — every `t_i > 1.0f`.
#[test]
fn e29_aabb_all_t_beyond_one() {
    let (c, r) = pair();
    let mut d = Diff::new("E29: c2RaytoAABB hit == 0");
    let mut g = Rng::new(0xE029);
    // This arm is reached by rays whose infinite line hits the box but whose
    // parametrised plane crossings all land past 1; drive it by volume and by
    // stopping the ray short.
    for _ in 0..M * 8 {
        let b = g.aabb(5.0);
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        let u = g.dir();
        let dist = 5.0 + g.unit() * 20.0;
        let p = c2v {
            x: cx - u.x * dist,
            y: cy - u.y * dist,
        };
        // ray far too short to reach the box, but whose swept AABB may still
        // overlap for shallow angles
        let ray = c2Ray {
            p,
            d: u,
            t: dist * g.unit() * 0.5,
        };
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    d.finish();
}

/// Row 30: `A.t == 0` — a zero-length ray still reports a hit when its origin
/// is inside the box, with `out->t == t_i * 0`.
#[test]
fn e30_aabb_zero_length_ray() {
    let (c, r) = pair();
    let mut d = Diff::new("E30: c2RaytoAABB A.t == 0");
    let mut g = Rng::new(0xE030);
    let mut inside_hits = 0usize;
    for i in 0..M * 4 {
        let b = g.aabb(8.0);
        let p = if i % 2 == 0 {
            c2v {
                x: b.min.x + (b.max.x - b.min.x) * g.unit(),
                y: b.min.y + (b.max.y - b.min.y) * g.unit(),
            }
        } else {
            g.v(30.0)
        };
        for (t, dir) in [
            (0.0f32, g.dir()),
            (-0.0f32, g.dir()),
            (0.0f32, c2v { x: 0.0, y: 0.0 }),
            (0.0f32, c2v { x: 1.0, y: 0.0 }),
        ] {
            let ray = c2Ray { p, d: dir, t };
            let mut cb = OutBuf::filled();
            if unsafe { (c.c2RaytoAABB)(ray, b, cb.as_ptr()) } == 1 {
                inside_hits += 1;
            }
            cmp_ray_aabb(&mut d, c, r, ray, b);
        }
    }
    assert!(
        inside_hits > M,
        "zero-length rays inside the box should hit ({inside_hits})"
    );
    d.finish();
}

/// Row 31: unnormalised and zero direction vectors.
#[test]
fn e31_aabb_unnormalised_direction() {
    let (c, r) = pair();
    let mut d = Diff::new("E31: c2RaytoAABB unnormalised / zero direction");
    let mut g = Rng::new(0xE031);
    for i in 0..M * 6 {
        let b = g.aabb(8.0);
        let dir = match i % 6 {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => c2v { x: -0.0, y: -0.0 },
            2 => c2v { x: 1e-30, y: 1e-30 },
            3 => c2v { x: 1e30, y: -1e30 },
            4 => g.v(1e-6),
            _ => g.v(1e6),
        };
        let ray = c2Ray {
            p: g.v(20.0),
            d: dir,
            t: g.mixed_f32(100.0),
        };
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    d.finish();
}

/// Row 32: `NaN` anywhere → `hit == 0` → reject.
#[test]
fn e32_aabb_nan_rejects() {
    let (c, r) = pair();
    let mut d = Diff::new("E32: c2RaytoAABB NaN input");
    let mut g = Rng::new(0xE032);
    let nan = f32::NAN;
    for i in 0..M * 8 {
        let mut b = g.aabb(8.0);
        let mut ray = c2Ray {
            p: g.v(20.0),
            d: g.dir(),
            t: g.unit() * 40.0,
        };
        let poison = if i % 2 == 0 { nan } else { -nan };
        match i % 9 {
            0 => ray.p.x = poison,
            1 => ray.p.y = poison,
            2 => ray.d.x = poison,
            3 => ray.d.y = poison,
            4 => ray.t = poison,
            5 => b.min.x = poison,
            6 => b.min.y = poison,
            7 => b.max.x = poison,
            _ => b.max.y = poison,
        }
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    d.finish();
}

/// Row 33: inverted `B` (negative half-extents).
#[test]
fn e33_aabb_inverted_box() {
    let (c, r) = pair();
    let mut d = Diff::new("E33: c2RaytoAABB inverted box");
    let mut g = Rng::new(0xE033);
    for _ in 0..M * 6 {
        let q = g.aabb(8.0);
        let b = c2AABB { min: q.max, max: q.min };
        let ray = c2Ray {
            p: g.v(20.0),
            d: if g.below(2) == 0 { g.dir() } else { g.v(2.0) },
            t: g.unit() * 40.0,
        };
        cmp_ray_aabb(&mut d, c, r, ray, b);
    }
    d.finish();
}

// ===========================================================================
// Rows 34–40: c2RaytoCapsule rejections
// ===========================================================================

/// Row 34: the final `return 0`, with `out` already overwritten by
/// `c2Norm(b-a)` / `0` before the branch. The byte comparison is what makes
/// that pre-write observable.
#[test]
fn e34_capsule_fall_through_miss() {
    let (c, r) = pair();
    let mut d = Diff::new("E34: c2RaytoCapsule fall-through miss (out pre-written)");
    let mut g = Rng::new(0xE034);
    let mut clean_misses = 0usize;
    for _ in 0..M * 6 {
        let a = g.v(15.0);
        let len = 1.0 + g.unit() * 15.0;
        let u = g.dir();
        let cap = c2Capsule {
            a,
            b: c2v {
                x: a.x + u.x * len,
                y: a.y + u.y * len,
            },
            r: 0.05 + g.unit() * 2.0,
        };
        let perp = c2v { x: -u.y, y: u.x };
        // Far to one side, travelling parallel: yAp.x and yAe.x share a sign
        // and both exceed r, so nothing matches.
        let off = cap.r * (10.0 + g.unit() * 50.0) * if g.below(2) == 0 { 1.0 } else { -1.0 };
        let p = c2v {
            x: a.x + u.x * (len * g.unit()) + perp.x * off,
            y: a.y + u.y * (len * g.unit()) + perp.y * off,
        };
        let ray = c2Ray {
            p,
            d: u,
            t: len * (0.1 + g.unit()),
        };
        let mut cb = OutBuf::filled();
        let got = unsafe { (c.c2RaytoCapsule)(ray, cap, cb.as_ptr()) };
        if got == 0 && cb != OUT_FILL {
            clean_misses += 1;
        }
        cmp_ray_capsule(&mut d, c, r, ray, cap);
    }
    assert!(
        clean_misses > M * 4,
        "expected misses that still overwrite *out ({clean_misses})"
    );
    d.finish();
}

/// Row 35: degenerate capsule `a == b` → `c2Norm(0,0)` → NaN everywhere.
#[test]
fn e35_capsule_degenerate_zero_length() {
    let (c, r) = pair();
    let mut d = Diff::new("E35: c2RaytoCapsule a == b (c2Norm of the zero vector)");
    let mut g = Rng::new(0xE035);
    for _ in 0..M * 4 {
        let q = g.v(20.0);
        for cap in [
            c2Capsule { a: q, b: q, r: 0.5 + g.unit() * 5.0 },
            c2Capsule { a: q, b: q, r: 0.0 },
            c2Capsule { a: q, b: q, r: -1.0 },
            c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: -0.0, y: -0.0 },
                r: 1.0,
            },
        ] {
            let ray = c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.unit() * 40.0,
            };
            cmp_ray_capsule(&mut d, c, r, ray, cap);
        }
    }
    // Confirm the C really does produce NaN in out->n here (the condition the
    // row is about), so the row is not vacuous.
    let q = c2v { x: 3.0, y: 4.0 };
    let cap = c2Capsule { a: q, b: q, r: 1.0 };
    let ray = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let mut cb = OutBuf::filled();
    unsafe { (c.c2RaytoCapsule)(ray, cap, cb.as_ptr()) };
    let w = cb.words();
    assert!(
        f32::from_bits(w[1]).is_nan() && f32::from_bits(w[2]).is_nan(),
        "expected NaN normal from the degenerate capsule, got {cb:?}"
    );
    d.finish();
}

/// Row 36: the UNGUARDED division `t = (c - yAp.x) / (yAe.x - yAp.x)`.
///
/// Two distinct claims are tested here.
///
/// 1. An *exact* zero denominator is **unreachable**. A ray travelling exactly
///    along the capsule axis gives `yAd.x == 0`, hence `yAe.x == yAp.x`, hence
///    `d == 0` — but that also forces the outer test to succeed only via
///    `min(|yAe.x|,|yAp.x|) < B.r`, i.e. `|yAp.x| < B.r`, which routes to the
///    circle-delegation arms *before* the division. So the C never actually
///    divides by zero here. The first loop constructs exactly that geometry and
///    asserts the arm is never entered with a zero denominator, in the C.
/// 2. A *non-finite* denominator (`±inf`) and a non-finite quotient ARE
///    reachable, via `inf` in `A.d` / `A.t` / `B`. The second loop searches for
///    those and diffs them.
#[test]
fn e36_capsule_unguarded_division_by_zero() {
    let (c, r) = pair();
    let mut d = Diff::new("E36: c2RaytoCapsule slab division, zero and non-finite denominators");
    let mut g = Rng::new(0xE036);

    // --- claim 1: exactly-parallel rays never reach the division -----------
    let mut zero_denominator_in_arm = 0usize;
    for _ in 0..M * 4 {
        let a = g.v(15.0);
        let len = 1.0 + g.unit() * 10.0;
        let cap = c2Capsule {
            a,
            b: c2v { x: a.x, y: a.y + len },
            r: 0.2 + g.unit() * 2.0,
        };
        for off_scale in [1.0f32, 0.5, 1.0 + g.unit() * 5.0, 0.999] {
            for sign in [1.0f32, -1.0] {
                let p = c2v {
                    x: a.x + sign * cap.r * off_scale,
                    y: a.y - len * (0.5 + g.unit()),
                };
                let ray = c2Ray {
                    p,
                    d: c2v { x: 0.0, y: 1.0 },
                    t: len * (1.0 + g.unit() * 3.0),
                };
                if slab_denominator(c, ray, cap) == Some(0.0) {
                    zero_denominator_in_arm += 1;
                }
                cmp_ray_capsule(&mut d, c, r, ray, cap);
            }
        }
    }
    assert_eq!(
        zero_denominator_in_arm, 0,
        "an exact 0 denominator was reached; the ERRORS.md row-36 reachability \
         claim is wrong and needs revising"
    );

    // --- claim 2: non-finite denominators / quotients are reachable --------
    let mut nonfinite_t = 0usize;
    let mut inf_denominator = 0usize;
    let mut g2 = Rng::new(0xE0362);
    for _ in 0..400_000u32 {
        let cap = match g2.below(3) {
            0 => c2Capsule {
                a: g2.v_special(),
                b: g2.v_special(),
                r: g2.special_f32(),
            },
            1 => c2Capsule {
                a: c2v { x: g2.any_bits_f32(), y: g2.any_bits_f32() },
                b: c2v { x: g2.any_bits_f32(), y: g2.any_bits_f32() },
                r: g2.any_bits_f32(),
            },
            _ => c2Capsule {
                a: g2.v_mixed(1e3),
                b: g2.v_mixed(1e3),
                r: g2.mixed_f32(1e3),
            },
        };
        let ray = match g2.below(3) {
            0 => c2Ray {
                p: g2.v_special(),
                d: g2.v_special(),
                t: g2.special_f32(),
            },
            1 => c2Ray {
                p: c2v { x: g2.any_bits_f32(), y: g2.any_bits_f32() },
                d: c2v { x: g2.any_bits_f32(), y: g2.any_bits_f32() },
                t: g2.any_bits_f32(),
            },
            _ => c2Ray {
                p: g2.v_mixed(1e3),
                d: g2.v_mixed(1e3),
                t: g2.mixed_f32(1e3),
            },
        };
        match slab_denominator(c, ray, cap) {
            None => continue,
            Some(dd) => {
                if dd.is_infinite() {
                    inf_denominator += 1;
                }
                // Only diff the interesting ones, to keep the test fast.
                if !dd.is_finite() || dd == 0.0 {
                    nonfinite_t += 1;
                    cmp_ray_capsule(&mut d, c, r, ray, cap);
                } else if nonfinite_t < 50 {
                    cmp_ray_capsule(&mut d, c, r, ray, cap);
                }
            }
        }
    }
    assert!(
        inf_denominator > 100,
        "expected the inf-denominator case to be reachable, saw {inf_denominator}"
    );
    println!(
        "E36: inf denominators={inf_denominator}, non-finite-denominator cases diffed={nonfinite_t}"
    );
    d.finish();
}

/// If `(ray, cap)` reaches `c2RaytoCapsule`'s slab arm, return the denominator
/// `yAe.x - yAp.x` that the C would divide by; otherwise `None`. Recomputed
/// entirely with the C's own exported primitives.
fn slab_denominator(c: &Impl, a: c2Ray, b: c2Capsule) -> Option<f32> {
    unsafe {
        let my = (c.c2Norm)((c.c2Sub)(b.b, b.a));
        let mx = (c.c2CCW90)(my);
        let m = c2m { x: mx, y: my };
        let cap_n = (c.c2Sub)(b.b, b.a);
        let y_bb = (c.c2MulmvT)(m, cap_n);
        let y_ap = (c.c2MulmvT)(m, (c.c2Sub)(a.p, b.a));
        let y_ad = (c.c2MulmvT)(m, a.d);
        let y_ae = (c.c2Add)(y_ap, (c.c2Mulvs)(y_ad, a.t));
        let bb = c2AABB {
            min: (c.c2V)(-b.r, 0.0),
            max: (c.c2V)(b.r, y_bb.y),
        };
        if (c.c2AABBtoPoint)(bb, y_ap) != 0 {
            return None;
        }
        if (c.c2CircleToPoint)(c2Circle { p: b.a, r: b.r }, a.p) != 0 {
            return None;
        }
        if (c.c2CircleToPoint)(c2Circle { p: b.b, r: b.r }, a.p) != 0 {
            return None;
        }
        let tabs = |v: f32| if v < 0.0 { -v } else { v };
        let tmin = |x: f32, y: f32| if x < y { x } else { y };
        if !(y_ae.x * y_ap.x < 0.0 || tmin(tabs(y_ae.x), tabs(y_ap.x)) < b.r) {
            return None;
        }
        if tabs(y_ap.x) < b.r {
            return None;
        }
        Some(y_ae.x - y_ap.x)
    }
}

/// Rows 37–38: zero and negative capsule radius.
#[test]
fn e37_38_capsule_zero_and_negative_radius() {
    let (c, r) = pair();
    let mut d = Diff::new("E37/E38: c2RaytoCapsule r == 0 and r < 0");
    let mut g = Rng::new(0xE037);
    const RS: &[f32] = &[0.0, -0.0, -1e-6, -0.5, -1.0, -20.0, f32::from_bits(1)];
    for i in 0..M * 8 {
        let a = g.v(15.0);
        let len = 0.5 + g.unit() * 12.0;
        let u = g.dir();
        let cap = c2Capsule {
            a,
            b: c2v {
                x: a.x + u.x * len,
                y: a.y + u.y * len,
            },
            r: RS[i % RS.len()],
        };
        let ray = c2Ray {
            p: g.v(25.0),
            d: g.dir(),
            t: g.unit() * 50.0,
        };
        cmp_ray_capsule(&mut d, c, r, ray, cap);
        // and a ray aimed exactly at the axis (the only way a zero-radius
        // capsule can be hit at all)
        let mid = c2v {
            x: a.x + u.x * len * 0.5,
            y: a.y + u.y * len * 0.5,
        };
        let start = c2v {
            x: mid.x - u.y * 5.0,
            y: mid.y + u.x * 5.0,
        };
        let dir = unsafe { (c.c2Norm)(c2v { x: mid.x - start.x, y: mid.y - start.y }) };
        cmp_ray_capsule(
            &mut d,
            c,
            r,
            c2Ray { p: start, d: dir, t: 10.0 },
            cap,
        );
    }
    d.finish();
}

/// Row 39: zero and negative `A.t`.
#[test]
fn e39_capsule_zero_and_negative_t() {
    let (c, r) = pair();
    let mut d = Diff::new("E39: c2RaytoCapsule A.t <= 0");
    let mut g = Rng::new(0xE039);
    for i in 0..M * 6 {
        let a = g.v(15.0);
        let len = 0.5 + g.unit() * 12.0;
        let u = g.dir();
        let cap = c2Capsule {
            a,
            b: c2v {
                x: a.x + u.x * len,
                y: a.y + u.y * len,
            },
            r: 0.05 + g.unit() * 3.0,
        };
        let t = [0.0f32, -0.0, -1e-30, -1.0, -1e30, f32::MIN][i % 6];
        let ray = c2Ray {
            p: g.v(25.0),
            d: g.dir(),
            t,
        };
        cmp_ray_capsule(&mut d, c, r, ray, cap);
    }
    d.finish();
}

/// Row 40: `NaN` / `inf` in every slot of `A` and `B`.
#[test]
fn e40_capsule_nan_inf_slots() {
    let (c, r) = pair();
    let mut d = Diff::new("E40: c2RaytoCapsule NaN/inf in each slot");
    let mut g = Rng::new(0xE040);
    for i in 0..M * 10 {
        let a = g.v(15.0);
        let len = 0.5 + g.unit() * 12.0;
        let u = g.dir();
        let mut cap = c2Capsule {
            a,
            b: c2v {
                x: a.x + u.x * len,
                y: a.y + u.y * len,
            },
            r: 0.05 + g.unit() * 3.0,
        };
        let mut ray = c2Ray {
            p: g.v(25.0),
            d: g.dir(),
            t: g.unit() * 50.0,
        };
        let poison = [
            f32::NAN,
            -f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7F80_0001),
        ][i % 5];
        match i % 10 {
            0 => ray.p.x = poison,
            1 => ray.p.y = poison,
            2 => ray.d.x = poison,
            3 => ray.d.y = poison,
            4 => ray.t = poison,
            5 => cap.a.x = poison,
            6 => cap.a.y = poison,
            7 => cap.b.x = poison,
            8 => cap.b.y = poison,
            _ => cap.r = poison,
        }
        cmp_ray_capsule(&mut d, c, r, ray, cap);
    }
    d.finish();
}

// ===========================================================================
// Rows 41–44: c2CastRay — the out-of-range enum (UB) and payload handling
// ===========================================================================

/// Rows 41–42: out-of-range `C2_TYPE`.
///
/// The C's compiled default edge (`ja <epilogue>` / `jmp <epilogue>`) never
/// writes `%eax`, so it returns the caller's incoming `%eax`. The Rust export
/// must do the same.
///
/// Comparing two ordinary call sites is ill-posed here, because the value under
/// test is the *caller's* register state and the two call sites are not
/// obliged to agree on it (an unoptimised caller loads each callee's own
/// address into `rax` right before `call rax`). So the incoming `%eax` is
/// pinned to a chosen value by `cast_ray_with_eax`, and BOTH libraries are then
/// required to return exactly that value, for every out-of-range tag, while
/// leaving `*out` untouched.
#[test]
fn e41_42_cast_ray_out_of_range_enum() {
    let (c, r) = pair();
    let mut d = Diff::new("E41/E42: c2CastRay out-of-range C2_TYPE (controlled %eax)");
    let s = c2Circle {
        p: c2v { x: 5.0, y: 0.0 },
        r: 1.0,
    };
    let mut g = Rng::new(0xE041);
    const TAGS: &[c_uint] = &[
        3,
        4,
        5,
        7,
        8,
        16,
        255,
        256,
        1000,
        0x0000_FFFF,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        0xFFFF_FFFF, // == (C2_TYPE)-1
    ];
    const EAXS: &[u32] = &[
        0,
        1,
        2,
        3,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0xDEAD_BEEF,
        0x0000_00FF,
        0x42C8_0000,
    ];
    let mut observed = std::collections::BTreeSet::new();
    for tag in TAGS.iter().copied() {
        for eax in EAXS.iter().copied() {
            let ray = c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.mixed_f32(1e4),
            };
            // Both must return exactly the injected %eax.
            let mut cb = OutBuf::filled();
            let cv = unsafe {
                cast_ray_with_eax(
                    c.c2CastRay,
                    eax,
                    ray,
                    &s as *const c2Circle as *const c_void,
                    tag,
                    cb.as_ptr(),
                )
            };
            assert_eq!(
                cv as u32, eax,
                "the C did not return the caller's %eax for tag={tag:#x}"
            );
            assert_eq!(cb, OUT_FILL, "the C wrote *out on the UB edge");
            observed.insert(cv);
            cmp_cast_ray_eax(&mut d, c, r, eax, ray, as_bytes(&s), tag);
        }
        // and many randomized %eax values per tag
        for _ in 0..300 {
            let eax = g.next_u32();
            let ray = c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.mixed_f32(1e4),
            };
            observed.insert(unsafe {
                cast_ray_with_eax(
                    c.c2CastRay,
                    eax,
                    ray,
                    &s as *const c2Circle as *const c_void,
                    tag,
                    OutBuf::filled().as_ptr(),
                )
            });
            cmp_cast_ray_eax(&mut d, c, r, eax, ray, as_bytes(&s), tag);
        }
    }
    assert!(
        observed.len() > 1000,
        "the UB return value barely varied ({}); the row would be passing for a \
         trivial reason",
        observed.len()
    );
    println!(
        "E41/E42: {} distinct UB return values injected and matched",
        observed.len()
    );

    // Valid tags must be unaffected by the incoming %eax.
    for tag in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        for eax in EAXS.iter().copied() {
            let ray = c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.unit() * 40.0,
            };
            cmp_cast_ray_eax(&mut d, c, r, eax, ray, as_bytes(&s), tag);
        }
    }
    d.finish();
}

/// Row 43: `B == NULL` together with an out-of-range tag. The C never
/// dereferences `B` on that edge, so this must not crash, and (with a
/// controlled `%eax`) both must return the same value.
#[test]
fn e43_cast_ray_null_payload_invalid_tag() {
    let (c, r) = pair();
    let mut d = Diff::new("E43: c2CastRay NULL payload + invalid tag");
    let mut g = Rng::new(0xE043);
    for tag in [3u32, 4, 99, 0x8000_0000, 0xFFFF_FFFF] {
        for _ in 0..200 {
            let eax = g.next_u32();
            let ray = c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.mixed_f32(1e4),
            };
            let mut cb = OutBuf::filled();
            let mut rb = OutBuf::filled();
            let cv = unsafe {
                cast_ray_with_eax(c.c2CastRay, eax, ray, std::ptr::null(), tag, cb.as_ptr())
            };
            let rv = unsafe {
                cast_ray_with_eax(r.c2CastRay, eax, ray, std::ptr::null(), tag, rb.as_ptr())
            };
            d.eq(
                || format!("NULL tag={tag:#x} eax={eax:#010x} {}", fray(ray)),
                RayResult { ret: cv, out: cb },
                RayResult { ret: rv, out: rb },
            );
        }
    }
    // NULL `out` as well: still never dereferenced on this edge.
    for tag in [3u32, 0xFFFF_FFFF] {
        for eax in [0u32, 7, 0xFFFF_FFFF] {
            let ray = c2Ray {
                p: c2v { x: 1.0, y: 2.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 7.0,
            };
            let cv = unsafe {
                cast_ray_with_eax(
                    c.c2CastRay,
                    eax,
                    ray,
                    std::ptr::null(),
                    tag,
                    std::ptr::null_mut(),
                )
            };
            let rv = unsafe {
                cast_ray_with_eax(
                    r.c2CastRay,
                    eax,
                    ray,
                    std::ptr::null(),
                    tag,
                    std::ptr::null_mut(),
                )
            };
            d.eq(|| format!("NULL/NULL tag={tag:#x} eax={eax:#010x}"), cv, rv);
        }
    }
    d.finish();
}

/// Row 41/42 addendum: an ordinary (uncontrolled) call site. The return value
/// is caller-state-dependent and therefore not asserted; what *is* asserted is
/// that neither library crashes, neither touches `*out`, and both agree
/// whenever the harness did leave the same `%eax` before the two calls.
#[test]
fn e41_42b_cast_ray_uncontrolled_call_site() {
    let (c, r) = pair();
    let s = c2Circle {
        p: c2v { x: 5.0, y: 0.0 },
        r: 1.0,
    };
    let mut g = Rng::new(0xE041B);
    let mut agreed = 0usize;
    let mut total = 0usize;
    for tag in [3u32, 4, 0x8000_0000, 0xFFFF_FFFF] {
        for _ in 0..500 {
            let ray = c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.mixed_f32(1e4),
            };
            let p = &s as *const c2Circle as *const c_void;
            let mut cb = OutBuf::filled();
            let mut rb = OutBuf::filled();
            let cv = unsafe { (c.c2CastRay)(ray, p, tag, cb.as_ptr()) };
            let rv = unsafe { (r.c2CastRay)(ray, p, tag, rb.as_ptr()) };
            assert_eq!(cb, OUT_FILL, "C wrote *out on the UB edge");
            assert_eq!(rb, OUT_FILL, "Rust wrote *out on the UB edge");
            total += 1;
            if cv == rv {
                agreed += 1;
            }
        }
    }
    println!(
        "E41/E42b: uncontrolled call site agreed on {agreed}/{total} \
         (caller-state dependent by construction; see e41_42 for the \
         well-posed test)"
    );
}

/// Row 44: a valid tag with an over-sized payload buffer — both sides read the
/// same bytes, and the C cannot detect the mismatch.
#[test]
fn e44_cast_ray_oversized_payload() {
    let (c, r) = pair();
    let mut d = Diff::new("E44: c2CastRay valid tag, over-sized payload buffer");
    let mut g = Rng::new(0xE044);
    for i in 0..M * 3 {
        // 20 bytes of random-but-identical payload; every tag reads a prefix.
        let mut payload = [0u8; 20];
        for w in 0..5 {
            let bits = if g.below(2) == 0 {
                g.sym(20.0).to_bits()
            } else {
                g.special_f32().to_bits()
            };
            payload[w * 4..w * 4 + 4].copy_from_slice(&bits.to_ne_bytes());
        }
        let ray = c2Ray {
            p: g.v(20.0),
            d: g.dir(),
            t: g.unit() * 40.0,
        };
        let tag = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][i % 3];
        cmp_cast_ray(&mut d, c, r, ray, &payload, tag, "oversized");
    }
    d.finish();
}

// ===========================================================================
// Rows 45–46: NULL out-parameter on paths that never write it
// ===========================================================================

/// Row 45: `c2RaytoCircle(out = NULL)` on a guaranteed miss. `out` is only
/// dereferenced inside the `if`, so this must return 0 without faulting.
#[test]
fn e45_circle_null_out_on_miss() {
    let (c, r) = pair();
    let mut d = Diff::new("E45: c2RaytoCircle out == NULL on a miss");
    let mut g = Rng::new(0xE045);
    for _ in 0..M * 2 {
        // Certain miss: circle far away, short ray.
        let ray = c2Ray {
            p: g.v(5.0),
            d: g.dir(),
            t: g.unit(),
        };
        let s = c2Circle {
            p: c2v {
                x: 1.0e6 + g.unit(),
                y: 1.0e6 + g.unit(),
            },
            r: 1.0,
        };
        // Verify with a real buffer first that this is indeed a miss.
        let mut probe = OutBuf::filled();
        assert_eq!(
            unsafe { (c.c2RaytoCircle)(ray, s, probe.as_ptr()) },
            0,
            "setup error: expected a miss"
        );
        let cv = unsafe { (c.c2RaytoCircle)(ray, s, std::ptr::null_mut()) };
        let rv = unsafe { (r.c2RaytoCircle)(ray, s, std::ptr::null_mut()) };
        d.eq(|| format!("NULL out {} {}", fray(ray), fcircle(s)), cv, rv);
    }
    // Also the disc<0 and NaN edges specifically.
    let cases: &[(c2Ray, c2Circle)] = &[
        (
            c2Ray {
                p: c2v { x: 0.0, y: 100.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 1000.0,
            },
            c2Circle {
                p: c2v { x: 50.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: f32::NAN, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: -1.0,
            },
            c2Circle {
                p: c2v { x: 5.0, y: 0.0 },
                r: 1.0,
            },
        ),
    ];
    for (ray, s) in cases {
        let cv = unsafe { (c.c2RaytoCircle)(*ray, *s, std::ptr::null_mut()) };
        let rv = unsafe { (r.c2RaytoCircle)(*ray, *s, std::ptr::null_mut()) };
        d.eq(|| format!("NULL edge {} {}", fray(*ray), fcircle(*s)), cv, rv);
    }
    d.finish();
}

/// Row 46: `c2RaytoAABB(out = NULL)` on a guaranteed miss.
#[test]
fn e46_aabb_null_out_on_miss() {
    let (c, r) = pair();
    let mut d = Diff::new("E46: c2RaytoAABB out == NULL on a miss");
    let mut g = Rng::new(0xE046);
    for _ in 0..M * 2 {
        let ray = c2Ray {
            p: g.v(5.0),
            d: g.dir(),
            t: g.unit(),
        };
        let b = c2AABB {
            min: c2v { x: 1.0e6, y: 1.0e6 },
            max: c2v { x: 1.0e6 + 1.0, y: 1.0e6 + 1.0 },
        };
        let mut probe = OutBuf::filled();
        assert_eq!(
            unsafe { (c.c2RaytoAABB)(ray, b, probe.as_ptr()) },
            0,
            "setup error: expected a miss"
        );
        let cv = unsafe { (c.c2RaytoAABB)(ray, b, std::ptr::null_mut()) };
        let rv = unsafe { (r.c2RaytoAABB)(ray, b, std::ptr::null_mut()) };
        d.eq(|| format!("NULL out {} {}", fray(ray), fbox(b)), cv, rv);
    }
    // The `d > 0` separating-axis edge with a NULL out.
    // Box is [-1,1]^2 (half-extents (1,1)); the ray runs at exactly 45 deg, so
    // `n = skew(ab)` gives `|dot(n, p0-centre)| = t*|p0.y-p0.x|/sqrt(2)` and
    // `dot(|n|, half_extents) = t*sqrt(2)`. The reject needs
    // `|p0.y - p0.x| > 2`; `(-6, -3)` gives 3.
    let b = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let ray = c2Ray {
        p: c2v { x: -6.0, y: -3.0 },
        d: c2v {
            x: std::f32::consts::FRAC_1_SQRT_2,
            y: std::f32::consts::FRAC_1_SQRT_2,
        },
        t: 20.0,
    };
    // The swept AABB must overlap (so the FIRST early-out is passed and we
    // really are exercising the separating-axis one).
    let p1 = unsafe { (c.c2Add)(ray.p, (c.c2Mulvs)(ray.d, ray.t)) };
    let sweep = c2AABB {
        min: unsafe { (c.c2Minv)(ray.p, p1) },
        max: unsafe { (c.c2Maxv)(ray.p, p1) },
    };
    assert_eq!(
        unsafe { (c.c2AABBtoAABB)(sweep, b) },
        1,
        "setup error: swept AABB does not overlap, wrong early-out under test"
    );
    let mut probe = OutBuf::filled();
    let got = unsafe { (c.c2RaytoAABB)(ray, b, probe.as_ptr()) };
    assert_eq!(got, 0, "setup error: expected a separating-axis reject");
    assert_eq!(probe, OUT_FILL, "setup error: C wrote *out on this reject");
    let cv = unsafe { (c.c2RaytoAABB)(ray, b, std::ptr::null_mut()) };
    let rv = unsafe { (r.c2RaytoAABB)(ray, b, std::ptr::null_mut()) };
    d.eq(|| "NULL out, d>0 edge".to_string(), cv, rv);
    d.finish();
}

// ===========================================================================
// Rows 47–52: unguarded IEEE sentinels in the vector helpers
// ===========================================================================

/// Rows 47–48: `c2Div` by zero / by `inf` / by `NaN` — no guard exists, and
/// the C multiplies by the reciprocal rather than dividing.
#[test]
fn e47_48_div_by_degenerate_scalar() {
    let (c, r) = pair();
    let mut d = Diff::new("E47/E48: c2Div by 0 / inf / NaN / subnormal");
    const SCALARS: &[f32] = &[
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        1e-45,
        1e38,
        3.0,
    ];
    let mut g = Rng::new(0xE047);
    for s in SCALARS.iter().copied() {
        for _ in 0..400 {
            let a = match g.below(3) {
                0 => g.v_special(),
                1 => c2v {
                    x: g.any_bits_f32(),
                    y: g.any_bits_f32(),
                },
                _ => g.v(1e6),
            };
            d.v_bits(
                || format!("c2Div({}, {:#010x})", fv(a), s.to_bits()),
                unsafe { (c.c2Div)(a, s) },
                unsafe { (r.c2Div)(a, s) },
            );
        }
        // and the exact zero / inf vectors
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            c2v { x: f32::NAN, y: 1.0 },
        ] {
            d.v_bits(
                || format!("c2Div edge({}, {:#010x})", fv(a), s.to_bits()),
                unsafe { (c.c2Div)(a, s) },
                unsafe { (r.c2Div)(a, s) },
            );
        }
    }
    d.finish();
}

/// Rows 49–50: `c2Norm` of the zero vector and of vectors containing `inf`.
#[test]
fn e49_50_norm_degenerate() {
    let (c, r) = pair();
    let mut d = Diff::new("E49/E50: c2Norm zero vector / inf component");
    let inf = f32::INFINITY;
    let nan = f32::NAN;
    let cases = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: inf, y: 0.0 },
        c2v { x: 0.0, y: inf },
        c2v { x: -inf, y: -inf },
        c2v { x: inf, y: -inf },
        c2v { x: nan, y: 0.0 },
        c2v { x: 0.0, y: nan },
        c2v { x: -nan, y: -nan },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN, y: f32::MIN },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
        c2v { x: f32::from_bits(1), y: 0.0 },
        c2v { x: 1e-30, y: 1e-30 },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: 1e20, y: 1e20 },
    ];
    for a in cases {
        d.v_bits(
            || format!("c2Norm({})", fv(a)),
            unsafe { (c.c2Norm)(a) },
            unsafe { (r.c2Norm)(a) },
        );
    }
    // Confirm the zero vector really does yield NaN, so the row is not vacuous.
    let z = unsafe { (c.c2Norm)(c2v { x: 0.0, y: 0.0 }) };
    assert!(
        z.x.is_nan() && z.y.is_nan(),
        "expected c2Norm((0,0)) to be NaN in the C, got {z:?}"
    );
    d.finish();
}

/// Rows 51–52: `c2Len` overflow and `NaN` (libm `sqrtf` vs the `sqrtss`
/// instruction, including the NaN payload).
#[test]
fn e51_52_len_overflow_and_nan() {
    let (c, r) = pair();
    let mut d = Diff::new("E51/E52: c2Len overflow to inf, and NaN payloads");
    let cases = [
        c2v { x: 1e20, y: 0.0 },
        c2v { x: f32::MAX, y: 0.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN, y: f32::MIN },
        c2v { x: 2e19, y: 2e19 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NEG_INFINITY, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: -f32::NAN, y: 0.0 },
        c2v { x: f32::from_bits(0x7F80_0001), y: 0.0 }, // signalling NaN
        c2v { x: f32::from_bits(0xFF80_0001), y: 0.0 },
        c2v { x: f32::from_bits(0x7FC0_1234), y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NAN },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
    ];
    for a in cases {
        d.f32_bits(
            || format!("c2Len({})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
    }
    // A large exhaustive-ish sweep of NaN payloads through c2Len.
    let mut g = Rng::new(0xE051);
    for _ in 0..M * 8 {
        let bits = 0x7F80_0000u32 | (g.next_u32() & 0x807F_FFFF);
        let a = c2v {
            x: f32::from_bits(bits),
            y: g.mixed_f32(1e6),
        };
        d.f32_bits(
            || format!("c2Len(nan-sweep {})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
    }
    assert!(
        unsafe { (c.c2Len)(c2v { x: f32::MAX, y: f32::MAX }) }.is_infinite(),
        "expected c2Len overflow to inf"
    );
    d.finish();
}

// ===========================================================================
// Rows 53–57: spec_ray rejections
// ===========================================================================

/// Row 53: `mp == ray.p` → `c2Norm(0,0)` → NaN direction → miss.
#[test]
fn e53_spec_ray_degenerate_direction() {
    let (c, r) = pair();
    let mut d = Diff::new("E53: spec_ray mp == ray origin");
    let mut g = Rng::new(0xE053);
    let mut rejected = 0usize;
    for _ in 0..M * 4 {
        let p = g.v(50.0);
        let centre = g.v(50.0);
        let rad = g.unit() * 20.0;
        let mut cb = OutBuf::filled();
        if unsafe { (c.spec_ray)(cb.as_ptr(), p.x, p.y, centre.x, centre.y, rad, p.x, p.y) } == 0 {
            rejected += 1;
        }
        cmp_spec_ray(&mut d, c, r, p, centre, rad, p);
    }
    assert_eq!(rejected, M * 4, "a NaN direction must always reject");
    // and the signed-zero variants of "the same point"
    for (a, b) in [
        (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: -0.0, y: 0.0 }, c2v { x: 0.0, y: -0.0 }),
    ] {
        cmp_spec_ray(&mut d, c, r, a, c2v { x: 5.0, y: 5.0 }, 1.0, b);
        cmp_spec_ray(&mut d, c, r, b, c2v { x: 5.0, y: 5.0 }, 1.0, a);
    }
    d.finish();
}

/// Row 54: zero and negative circle radius.
#[test]
fn e54_spec_ray_degenerate_radius() {
    let (c, r) = pair();
    let mut d = Diff::new("E54: spec_ray c_r == 0 / c_r < 0");
    let mut g = Rng::new(0xE054);
    const RS: &[f32] = &[0.0, -0.0, -1e-30, -1.0, -100.0, f32::MIN, f32::from_bits(1)];
    for i in 0..M * 8 {
        let centre = g.v(50.0);
        let origin = g.v(50.0);
        let mp = g.v(50.0);
        cmp_spec_ray(&mut d, c, r, mp, centre, RS[i % RS.len()], origin);
    }
    // aimed exactly at the centre of a zero-radius circle
    for _ in 0..M {
        let centre = g.v(50.0);
        let origin = g.v(50.0);
        cmp_spec_ray(&mut d, c, r, centre, centre, 0.0, origin);
        cmp_spec_ray(&mut d, c, r, centre, centre, -0.0, origin);
    }
    d.finish();
}

/// Row 55: `NaN` / `±inf` in every one of the seven float parameters.
#[test]
fn e55_spec_ray_nan_inf_each_argument() {
    let (c, r) = pair();
    let mut d = Diff::new("E55: spec_ray NaN/inf in each of the 7 float arguments");
    let mut g = Rng::new(0xE055);
    const POISON: &[f32] = &[
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_ABCD),
    ];
    for i in 0..M * 14 {
        let mut a = [
            g.sym(50.0), // mp.x
            g.sym(50.0), // mp.y
            g.sym(50.0), // c.p.x
            g.sym(50.0), // c.p.y
            g.unit() * 20.0, // c.r
            g.sym(50.0), // r.p.x
            g.sym(50.0), // r.p.y
        ];
        a[i % 7] = POISON[(i / 7) % POISON.len()];
        // sometimes poison two slots
        if i % 3 == 0 {
            a[(i + 3) % 7] = POISON[(i / 5) % POISON.len()];
        }
        cmp_spec_ray(
            &mut d,
            c,
            r,
            c2v { x: a[0], y: a[1] },
            c2v { x: a[2], y: a[3] },
            a[4],
            c2v { x: a[5], y: a[6] },
        );
    }
    d.finish();
}

/// Row 56: `cast == NULL` with a guaranteed miss — only
/// `c2RaytoCircle`'s early `return 0` is reached, so nothing dereferences it.
#[test]
fn e56_spec_ray_null_out_on_miss() {
    let (c, r) = pair();
    let mut d = Diff::new("E56: spec_ray cast == NULL on a guaranteed miss");
    let mut g = Rng::new(0xE056);
    for _ in 0..M * 2 {
        // Mouse point and ray origin near the origin; circle enormously far.
        let mp = c2v {
            x: g.sym(1.0),
            y: g.sym(1.0),
        };
        let rp = c2v {
            x: 10.0 + g.unit(),
            y: 10.0 + g.unit(),
        };
        let cp = c2v { x: 1.0e7, y: -1.0e7 };
        let cr = 1.0;
        let mut probe = OutBuf::filled();
        let got = unsafe { (c.spec_ray)(probe.as_ptr(), mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y) };
        assert_eq!(got, 0, "setup error: expected a miss");
        assert_eq!(probe, OUT_FILL, "setup error: C wrote *cast on a miss");
        let cv = unsafe {
            (c.spec_ray)(std::ptr::null_mut(), mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y)
        };
        let rv = unsafe {
            (r.spec_ray)(std::ptr::null_mut(), mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y)
        };
        d.eq(|| format!("NULL cast mp={} rp={}", fv(mp), fv(rp)), cv, rv);
    }
    // NaN-direction miss with a NULL out, too.
    let p = c2v { x: 3.0, y: 4.0 };
    let cv = unsafe { (c.spec_ray)(std::ptr::null_mut(), p.x, p.y, 0.0, 0.0, 1.0, p.x, p.y) };
    let rv = unsafe { (r.spec_ray)(std::ptr::null_mut(), p.x, p.y, 0.0, 0.0, 1.0, p.x, p.y) };
    d.eq(|| "NULL cast, NaN direction".to_string(), cv, rv);
    d.finish();
}

/// Row 57: the circle behind the mouse point, and the ray origin inside it.
#[test]
fn e57_spec_ray_circle_behind_or_containing_origin() {
    let (c, r) = pair();
    let mut d = Diff::new("E57: spec_ray circle behind mp / origin inside the circle");
    let mut g = Rng::new(0xE057);
    let mut rejected = 0usize;
    for i in 0..M * 4 {
        let centre = g.v(40.0);
        let rad = 0.5 + g.unit() * 8.0;
        let u = g.dir();
        let dist = rad * (3.0 + g.unit() * 6.0);
        if i % 2 == 0 {
            // circle strictly behind the ray origin
            let origin = c2v {
                x: centre.x + u.x * dist,
                y: centre.y + u.y * dist,
            };
            let mp = c2v {
                x: origin.x + u.x * dist,
                y: origin.y + u.y * dist,
            };
            let mut cb = OutBuf::filled();
            if unsafe {
                (c.spec_ray)(cb.as_ptr(), mp.x, mp.y, centre.x, centre.y, rad, origin.x, origin.y)
            } == 0
            {
                rejected += 1;
            }
            cmp_spec_ray(&mut d, c, r, mp, centre, rad, origin);
        } else {
            // ray origin strictly inside the circle
            let k = g.unit() * 0.9;
            let origin = c2v {
                x: centre.x + u.x * rad * k,
                y: centre.y + u.y * rad * k,
            };
            let mp = c2v {
                x: origin.x + u.x * dist,
                y: origin.y + u.y * dist,
            };
            cmp_spec_ray(&mut d, c, r, mp, centre, rad, origin);
        }
    }
    assert!(rejected > M, "behind-the-origin circles should reject");
    d.finish();
}

// ===========================================================================
// Row 58: signed zeros and subnormals across every float entry point.
// ===========================================================================

#[test]
fn e58_signed_zero_and_subnormal_everywhere() {
    let (c, r) = pair();
    let mut d = Diff::new("E58: signed zeros / subnormals across all float entry points");
    const Z: &[f32] = &[
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    for &x in Z {
        for &y in Z {
            let a = c2v { x, y };
            for &p in Z {
                for &q in Z {
                    let b = c2v { x: p, y: q };
                    d.v_bits(|| format!("c2Add({},{})", fv(a), fv(b)), unsafe {
                        (c.c2Add)(a, b)
                    }, unsafe { (r.c2Add)(a, b) });
                    d.v_bits(|| format!("c2Sub({},{})", fv(a), fv(b)), unsafe {
                        (c.c2Sub)(a, b)
                    }, unsafe { (r.c2Sub)(a, b) });
                    d.v_bits(|| format!("c2Minv({},{})", fv(a), fv(b)), unsafe {
                        (c.c2Minv)(a, b)
                    }, unsafe { (r.c2Minv)(a, b) });
                    d.v_bits(|| format!("c2Maxv({},{})", fv(a), fv(b)), unsafe {
                        (c.c2Maxv)(a, b)
                    }, unsafe { (r.c2Maxv)(a, b) });
                    d.f32_bits(|| format!("c2Dot({},{})", fv(a), fv(b)), unsafe {
                        (c.c2Dot)(a, b)
                    }, unsafe { (r.c2Dot)(a, b) });
                }
                d.v_bits(|| format!("c2Mulvs({},{:#010x})", fv(a), p.to_bits()), unsafe {
                    (c.c2Mulvs)(a, p)
                }, unsafe { (r.c2Mulvs)(a, p) });
                d.v_bits(|| format!("c2Div({},{:#010x})", fv(a), p.to_bits()), unsafe {
                    (c.c2Div)(a, p)
                }, unsafe { (r.c2Div)(a, p) });
            }
            d.v_bits(|| format!("c2Absv({})", fv(a)), unsafe { (c.c2Absv)(a) }, unsafe {
                (r.c2Absv)(a)
            });
            d.v_bits(|| format!("c2Skew({})", fv(a)), unsafe { (c.c2Skew)(a) }, unsafe {
                (r.c2Skew)(a)
            });
            d.v_bits(|| format!("c2CCW90({})", fv(a)), unsafe { (c.c2CCW90)(a) }, unsafe {
                (r.c2CCW90)(a)
            });
            d.v_bits(|| format!("c2Norm({})", fv(a)), unsafe { (c.c2Norm)(a) }, unsafe {
                (r.c2Norm)(a)
            });
            d.f32_bits(|| format!("c2Len({})", fv(a)), unsafe { (c.c2Len)(a) }, unsafe {
                (r.c2Len)(a)
            });
            d.v_bits(|| format!("c2V({:#010x},{:#010x})", x.to_bits(), y.to_bits()), unsafe {
                (c.c2V)(x, y)
            }, unsafe { (r.c2V)(x, y) });
        }
    }
    // and through the raycasts / spec_ray
    for &x in Z {
        for &y in Z {
            let ray = c2Ray {
                p: c2v { x, y },
                d: c2v { x: y, y: x },
                t: x,
            };
            cmp_ray_circle(&mut d, c, r, ray, c2Circle { p: c2v { x: y, y: x }, r: x });
            cmp_ray_aabb(
                &mut d,
                c,
                r,
                ray,
                c2AABB {
                    min: c2v { x, y },
                    max: c2v { x: y, y: x },
                },
            );
            cmp_ray_capsule(
                &mut d,
                c,
                r,
                ray,
                c2Capsule {
                    a: c2v { x, y },
                    b: c2v { x: y, y: x },
                    r: x,
                },
            );
            cmp_spec_ray(&mut d, c, r, c2v { x, y }, c2v { x: y, y: x }, x, c2v { x: y, y });
        }
    }
    d.finish();
}
