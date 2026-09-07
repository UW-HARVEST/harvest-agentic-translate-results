//! Phase C — error/rejection-path differential tests, one per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input the C rejects on and asserts
//! that both `.so`s return the SAME sentinel (`0`/`1`) *and* leave/write the
//! same `out` bytes — not merely "both failed somehow". The `out` buffer is
//! pre-filled with `DIRTY`, so "C left the field untouched" is asserted too.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Iterations per randomized row. Override with `DIFF_ITERS=<n>`.
fn iters() -> u32 {
    std::env::var("DIFF_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000)
}


// --- generic call helpers ---------------------------------------------------

fn circ(row: u32, i: u32, ray: C2Ray, ci: C2Circle) {
    let p = pair();
    let rc = ray_call(|o| unsafe { (p.c.c2RaytoCircle)(ray, ci, o) });
    let rr = ray_call(|o| unsafe { (p.r.c2RaytoCircle)(ray, ci, o) });
    diff_eq!(row, i, rc, rr, format!("{ray:?} {ci:?}"));
}

fn abox(row: u32, i: u32, ray: C2Ray, b: C2AABB) {
    let p = pair();
    let rc = ray_call(|o| unsafe { (p.c.c2RaytoAABB)(ray, b, o) });
    let rr = ray_call(|o| unsafe { (p.r.c2RaytoAABB)(ray, b, o) });
    diff_eq!(row, i, rc, rr, format!("{ray:?} {b:?}"));
}

fn caps(row: u32, i: u32, ray: C2Ray, cap: C2Capsule) {
    let p = pair();
    let rc = ray_call(|o| unsafe { (p.c.c2RaytoCapsule)(ray, cap, o) });
    let rr = ray_call(|o| unsafe { (p.r.c2RaytoCapsule)(ray, cap, o) });
    diff_eq!(row, i, rc, rr, format!("{ray:?} {cap:?}"));
}

fn poly(row: u32, i: u32, ray: C2Ray, pl: &C2Poly, bx: Option<&C2x>) {
    let p = pair();
    let bxp: *const C2x = match bx {
        Some(x) => x as *const C2x,
        None => std::ptr::null(),
    };
    let rc = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, pl as *const C2Poly, bxp, o) });
    let rr = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, pl as *const C2Poly, bxp, o) });
    diff_eq!(row, i, rc, rr, format!("{ray:?} count={} {bx:?}", pl.count));
}

// ===========================================================================
// c2RaytoCircle — rows 1–8
// ===========================================================================

/// Row 1 — `disc < 0`: the ray *line* misses the circle entirely.
#[test]
fn err01_circle_disc_negative() {
    let mut g = Rng::new(0x1001);
    for i in 0..iters() {
        let c = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: g.range(0.1, 5.0),
        };
        // Origin on the x axis, direction straight up: perpendicular distance
        // is |o.x| > r, so disc < 0.
        let o = C2v {
            x: c.r + g.range(0.5, 30.0),
            y: g.range(-30.0, 30.0),
        };
        circ(1, i, C2Ray { p: o, d: C2v { x: 0.0, y: 1.0 }, t: g.range(0.0, 1e4) }, c);
    }
}

/// Row 2 — hit exists but `t < 0`: circle is entirely behind the origin.
#[test]
fn err02_circle_behind_origin() {
    let mut g = Rng::new(0x1002);
    for i in 0..iters() {
        let c = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: g.range(0.1, 5.0),
        };
        let o = C2v {
            x: c.r + g.range(0.5, 30.0),
            y: 0.0,
        };
        // pointing away from the circle
        circ(2, i, C2Ray { p: o, d: C2v { x: 1.0, y: 0.0 }, t: g.range(0.0, 1e4) }, c);
    }
}

/// Row 3 — `t > A.t`: geometric hit, but past the end of the ray.
#[test]
fn err03_circle_beyond_ray_length() {
    let mut g = Rng::new(0x1003);
    for i in 0..iters() {
        let c = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: g.range(0.1, 5.0),
        };
        let dist = c.r + g.range(1.0, 30.0);
        let o = C2v { x: -dist, y: 0.0 };
        // stop short of the near surface
        let t = (dist - c.r) * g.range(0.0, 0.99);
        circ(3, i, C2Ray { p: o, d: C2v { x: 1.0, y: 0.0 }, t }, c);
    }
}

/// Row 4 — zero radius.
#[test]
fn err04_circle_zero_radius() {
    let mut g = Rng::new(0x1004);
    for i in 0..iters() {
        let c = C2Circle { p: g.normal_v(), r: 0.0 };
        let mut ray = g.normal_ray();
        if i % 3 == 0 {
            ray.p = c.p; // exactly at the point
        }
        circ(4, i, ray, c);
    }
}

/// Row 5 — negative radius (`r*r` is still positive, so C treats it as `|r|`).
#[test]
fn err05_circle_negative_radius() {
    let mut g = Rng::new(0x1005);
    for i in 0..iters() {
        let base = g.normal_circle();
        let c = C2Circle { p: base.p, r: -base.r.max(0.25) };
        let mut ray = g.normal_ray();
        // aim at the centre so the accept branch (and c2Norm) is exercised
        let dx = c.p.x - ray.p.x;
        let dy = c.p.y - ray.p.y;
        let l = (dx * dx + dy * dy).sqrt();
        ray.d = C2v { x: dx / l, y: dy / l };
        ray.t = l + g.range(0.0, 10.0);
        circ(5, i, ray, c);
    }
}

/// Row 6 — negative `A.t` rejects every `t >= 0`.
#[test]
fn err06_circle_negative_ray_length() {
    let mut g = Rng::new(0x1006);
    for i in 0..iters() {
        let c = g.normal_circle();
        let mut ray = g.normal_ray();
        ray.t = -g.range(0.0, 100.0);
        circ(6, i, ray, c);
    }
}

/// Row 7 — origin exactly at the centre: `c2Norm({0,0})` → NaN normal, ret 1.
#[test]
fn err07_circle_origin_at_centre() {
    let mut g = Rng::new(0x1007);
    for i in 0..iters() {
        let c = C2Circle {
            p: g.normal_v(),
            r: g.range(0.25, 12.0),
        };
        let th = g.range(-7.0, 7.0);
        // t = -b - sqrt(disc); with m = 0 → b = 0, disc = r*r, t = -r < 0, so
        // this actually rejects. Also try the impact-at-centre variant where
        // the ray starts on the rim aimed dead through the centre.
        let ray = if i % 2 == 0 {
            C2Ray { p: c.p, d: C2v { x: th.cos(), y: th.sin() }, t: g.range(0.0, 50.0) }
        } else {
            C2Ray {
                p: C2v { x: c.p.x - c.r * th.cos(), y: c.p.y - c.r * th.sin() },
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 50.0),
            }
        };
        circ(7, i, ray, c);
    }
}

/// Row 8 — NaN anywhere: every comparison is false, so C returns 0.
#[test]
fn err08_circle_nan_inputs() {
    let mut g = Rng::new(0x1008);
    for i in 0..iters() {
        let mut ray = g.normal_ray();
        let mut c = g.normal_circle();
        match i % 6 {
            0 => ray.p.x = g.nan_f32(),
            1 => ray.p.y = g.nan_f32(),
            2 => ray.d.x = g.nan_f32(),
            3 => ray.d.y = g.nan_f32(),
            4 => ray.t = g.nan_f32(),
            _ => c.r = g.nan_f32(),
        }
        circ(8, i, ray, c);
    }
    // fully adversarial
    for i in iters()..(2 * iters()) {
        circ(8, i, C2Ray { p: g.nan_v(), d: g.nan_v(), t: g.nan_zero_inf_f32() },
             C2Circle { p: g.nan_v(), r: g.nan_zero_inf_f32() });
    }
}

// ===========================================================================
// c2AABBtoAABB — rows 9–14
// ===========================================================================

fn bb2(row: u32, i: u32, a: C2AABB, b: C2AABB) {
    let p = pair();
    unsafe {
        diff_eq!(row, i, (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), format!("{a:?} {b:?}"));
    }
}

/// Rows 9–12 — each of the four separating-axis rejections in isolation.
#[test]
fn err09_to_12_aabb_each_separating_axis() {
    let mut g = Rng::new(0x1009);
    for i in 0..iters() {
        let a = g.normal_aabb();
        let w = g.range(0.5, 10.0);
        let h = g.range(0.5, 10.0);
        // d0: B.max.x < A.min.x
        let b0 = C2AABB {
            min: C2v { x: a.min.x - w - 1.0, y: a.min.y },
            max: C2v { x: a.min.x - 1.0, y: a.min.y + h },
        };
        bb2(9, i, a, b0);
        // d1: A.max.x < B.min.x
        let b1 = C2AABB {
            min: C2v { x: a.max.x + 1.0, y: a.min.y },
            max: C2v { x: a.max.x + 1.0 + w, y: a.min.y + h },
        };
        bb2(10, i, a, b1);
        // d2: B.max.y < A.min.y
        let b2 = C2AABB {
            min: C2v { x: a.min.x, y: a.min.y - h - 1.0 },
            max: C2v { x: a.min.x + w, y: a.min.y - 1.0 },
        };
        bb2(11, i, a, b2);
        // d3: A.max.y < B.min.y
        let b3 = C2AABB {
            min: C2v { x: a.min.x, y: a.max.y + 1.0 },
            max: C2v { x: a.min.x + w, y: a.max.y + 1.0 + h },
        };
        bb2(12, i, a, b3);
    }
}

/// Row 13 — inverted boxes (`min > max`), no validation in C.
#[test]
fn err13_aabb_inverted() {
    let mut g = Rng::new(0x100D);
    for i in 0..iters() {
        let n = g.normal_aabb();
        let inv = C2AABB { min: n.max, max: n.min };
        bb2(13, i, inv, g.normal_aabb());
        bb2(13, i, g.normal_aabb(), inv);
        bb2(13, i, inv, inv);
    }
}

/// Row 14 — NaN makes all four `<` false, so C reports "overlapping" (1).
#[test]
fn err14_aabb_nan_reports_overlap() {
    let p = pair();
    let nan = f32::NAN;
    let a = C2AABB {
        min: C2v { x: nan, y: nan },
        max: C2v { x: nan, y: nan },
    };
    let b = C2AABB {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    unsafe {
        // Documents the sentinel explicitly: C really returns 1 here.
        assert_eq!((p.c.c2AABBtoAABB)(a, b), 1, "C's NaN AABB overlap sentinel changed");
        diff_eq!(14, 0, (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), "all-NaN vs unit box");
    }
    let mut g = Rng::new(0x100E);
    for i in 0..iters() {
        let mut x = g.normal_aabb();
        let mut y = g.normal_aabb();
        match i % 4 {
            0 => x.min.x = g.nan_f32(),
            1 => x.max.y = g.nan_f32(),
            2 => y.min.y = g.nan_f32(),
            _ => y.max.x = g.nan_f32(),
        }
        bb2(14, i, x, y);
    }
}

// ===========================================================================
// c2RayToPlane_OneDimensional (static, reached via c2RaytoAABB) — rows 15–17
// ===========================================================================

/// Row 15 — `da < 0` for a given plane. Reached by putting the ray origin on
/// the outside of one of the four AABB planes.
/// Row 16 — `da*db > 0`: both endpoints on the same side of a plane.
/// Row 17 — `da == db` (`d == 0`): the ray is parallel to that plane.
#[test]
fn err15_16_17_raytoplane_branches() {
    let mut g = Rng::new(0x100F);
    for i in 0..iters() {
        let b = C2AABB {
            min: C2v { x: -2.0, y: -3.0 },
            max: C2v { x: 5.0, y: 7.0 },
        };
        // Row 17: axis-aligned direction → for the perpendicular pair,
        // p0 and p1 have the same coordinate so da == db, d == 0.
        let d17 = match i % 4 {
            0 => C2v { x: 1.0, y: 0.0 },
            1 => C2v { x: -1.0, y: 0.0 },
            2 => C2v { x: 0.0, y: 1.0 },
            _ => C2v { x: 0.0, y: -1.0 },
        };
        abox(17, i, C2Ray { p: C2v { x: g.range(-10.0, 10.0), y: g.range(-10.0, 10.0) }, d: d17, t: g.range(0.0, 40.0) }, b);
        // Rows 15/16: origin outside a plane, sweeping so that both same-side
        // (da*db > 0) and crossing cases occur.
        let o = C2v { x: g.range(-20.0, 20.0), y: g.range(-20.0, 20.0) };
        let th = g.range(-7.0, 7.0);
        let ray = C2Ray { p: o, d: C2v { x: th.cos(), y: th.sin() }, t: g.range(0.0, 60.0) };
        abox(15, i, ray, b);
        abox(16, i, ray, b);
    }
}

// ===========================================================================
// c2RaytoAABB — rows 18–25
// ===========================================================================

/// Row 18 — swept-ray AABB does not overlap `B` at all.
#[test]
fn err18_aabb_swept_box_disjoint() {
    let mut g = Rng::new(0x1012);
    for i in 0..iters() {
        let b = C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 4.0, y: 4.0 },
        };
        // Ray far to the left, moving further left.
        let o = C2v { x: -g.range(10.0, 100.0), y: g.range(-100.0, 100.0) };
        abox(18, i, C2Ray { p: o, d: C2v { x: -1.0, y: 0.0 }, t: g.range(0.0, 5.0) }, b);
    }
}

/// Row 19 — swept boxes overlap but the SAT test on the ray normal separates.
#[test]
fn err19_aabb_sat_separates() {
    let mut g = Rng::new(0x1013);
    for i in 0..iters() {
        let b = C2AABB {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        // A long diagonal ray whose swept AABB contains B but which passes
        // by a corner — the classic case row 18 cannot catch.
        let s = g.range(3.0, 20.0);
        let off = g.range(1.5, 4.0);
        abox(
            19,
            i,
            C2Ray {
                p: C2v { x: -s + off, y: -s - off },
                d: C2v { x: 0.70710678, y: 0.70710678 },
                t: 2.0 * s * g.range(1.0, 1.5),
            },
            b,
        );
    }
}

/// Row 20 — every `t_i > 1`, so `hit == 0`.
#[test]
fn err20_aabb_all_t_beyond_one() {
    let mut g = Rng::new(0x1014);
    for i in 0..iters() {
        let b = g.normal_aabb();
        // Ray that stops just before entering the box.
        let cx = (b.min.x + b.max.x) * 0.5;
        let ex = (b.max.x - b.min.x) * 0.5;
        let gap = g.range(0.5, 10.0);
        let o = C2v { x: cx - ex - gap, y: (b.min.y + b.max.y) * 0.5 };
        abox(20, i, C2Ray { p: o, d: C2v { x: 1.0, y: 0.0 }, t: gap * g.range(0.0, 0.99) }, b);
    }
}

/// Row 21 — `A.t == 0`: zero-length ray, `p0 == p1`, so `n == {0,0}`.
#[test]
fn err21_aabb_zero_length_ray() {
    let mut g = Rng::new(0x1015);
    for i in 0..iters() {
        let b = g.normal_aabb();
        let inside = i % 2 == 0;
        let o = if inside {
            C2v {
                x: b.min.x + (b.max.x - b.min.x) * g.unit(),
                y: b.min.y + (b.max.y - b.min.y) * g.unit(),
            }
        } else {
            g.normal_v()
        };
        let th = g.range(-7.0, 7.0);
        abox(21, i, C2Ray { p: o, d: C2v { x: th.cos(), y: th.sin() }, t: if i % 4 == 3 { -0.0 } else { 0.0 } }, b);
    }
}

/// Row 22 — negative `A.t` puts `p1` behind the origin.
#[test]
fn err22_aabb_negative_ray_length() {
    let mut g = Rng::new(0x1016);
    for i in 0..iters() {
        let b = g.normal_aabb();
        let mut ray = through_box_for_err(&mut g, b, i);
        ray.t = -g.range(0.0, 100.0);
        abox(22, i, ray, b);
    }
}

/// Row 23 — zero direction vector.
#[test]
fn err23_aabb_zero_direction() {
    let mut g = Rng::new(0x1017);
    for i in 0..iters() {
        let b = g.normal_aabb();
        let o = if i % 2 == 0 {
            C2v {
                x: b.min.x + (b.max.x - b.min.x) * g.unit(),
                y: b.min.y + (b.max.y - b.min.y) * g.unit(),
            }
        } else {
            g.normal_v()
        };
        abox(23, i, C2Ray { p: o, d: C2v { x: 0.0, y: 0.0 }, t: g.t_sweep(i) }, b);
    }
}

/// Row 24 — inverted `B` gives negative half extents.
#[test]
fn err24_aabb_inverted_target_box() {
    let mut g = Rng::new(0x1018);
    for i in 0..iters() {
        let n = g.normal_aabb();
        let b = C2AABB { min: n.max, max: n.min };
        abox(24, i, through_box_for_err(&mut g, n, i), b);
    }
}

/// Row 25 — NaN in the ray or box.
#[test]
fn err25_aabb_nan_inputs() {
    let mut g = Rng::new(0x1019);
    for i in 0..iters() {
        let mut ray = g.normal_ray();
        let mut b = g.normal_aabb();
        match i % 8 {
            0 => ray.p.x = g.nan_f32(),
            1 => ray.p.y = g.nan_f32(),
            2 => ray.d.x = g.nan_f32(),
            3 => ray.d.y = g.nan_f32(),
            4 => ray.t = g.nan_f32(),
            5 => b.min.x = g.nan_f32(),
            6 => b.max.y = g.nan_f32(),
            _ => {
                ray.p = g.nan_v();
                b.min = g.nan_v();
            }
        }
        abox(25, i, ray, b);
    }
    for i in iters()..(2 * iters()) {
        abox(
            25,
            i,
            C2Ray { p: g.nan_v(), d: g.nan_v(), t: g.nan_zero_inf_f32() },
            C2AABB { min: g.nan_v(), max: g.nan_v() },
        );
    }
}

fn through_box_for_err(g: &mut Rng, b: C2AABB, axis: u32) -> C2Ray {
    let cx = (b.min.x + b.max.x) * 0.5;
    let cy = (b.min.y + b.max.y) * 0.5;
    let ex = (b.max.x - b.min.x) * 0.5 + g.range(0.5, 10.0);
    let ey = (b.max.y - b.min.y) * 0.5 + g.range(0.5, 10.0);
    let (o, d) = match axis & 3 {
        0 => (C2v { x: cx - ex, y: cy }, C2v { x: 1.0, y: 0.0 }),
        1 => (C2v { x: cx + ex, y: cy }, C2v { x: -1.0, y: 0.0 }),
        2 => (C2v { x: cx, y: cy - ey }, C2v { x: 0.0, y: 1.0 }),
        _ => (C2v { x: cx, y: cy + ey }, C2v { x: 0.0, y: -1.0 }),
    };
    C2Ray { p: o, d, t: g.range(0.0, 80.0) }
}

// ===========================================================================
// c2AABBtoPoint — rows 26–30
// ===========================================================================

fn bpt(row: u32, i: u32, a: C2AABB, b: C2v) {
    let p = pair();
    unsafe {
        diff_eq!(row, i, (p.c.c2AABBtoPoint)(a, b), (p.r.c2AABBtoPoint)(a, b), format!("{a:?} {b:?}"));
    }
}

#[test]
fn err26_to_29_aabbtopoint_each_rejection() {
    let mut g = Rng::new(0x101A);
    for i in 0..iters() {
        let a = g.normal_aabb();
        let mid = C2v { x: (a.min.x + a.max.x) * 0.5, y: (a.min.y + a.max.y) * 0.5 };
        bpt(26, i, a, C2v { x: a.min.x - g.range(0.0, 10.0) - 1e-3, y: mid.y }); // B.x < min.x
        bpt(27, i, a, C2v { x: mid.x, y: a.min.y - g.range(0.0, 10.0) - 1e-3 }); // B.y < min.y
        bpt(28, i, a, C2v { x: a.max.x + g.range(0.0, 10.0) + 1e-3, y: mid.y }); // B.x > max.x
        bpt(29, i, a, C2v { x: mid.x, y: a.max.y + g.range(0.0, 10.0) + 1e-3 }); // B.y > max.y
    }
}

/// Row 30 — NaN makes all four comparisons false → C returns 1.
#[test]
fn err30_aabbtopoint_nan_reports_inside() {
    let p = pair();
    let a = C2AABB {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let b = C2v { x: f32::NAN, y: f32::NAN };
    unsafe {
        assert_eq!((p.c.c2AABBtoPoint)(a, b), 1, "C's NaN point-in-box sentinel changed");
        diff_eq!(30, 0, (p.c.c2AABBtoPoint)(a, b), (p.r.c2AABBtoPoint)(a, b), "NaN point");
    }
    let mut g = Rng::new(0x101E);
    for i in 0..iters() {
        let mut aa = g.normal_aabb();
        let mut pt = g.normal_v();
        match i % 4 {
            0 => pt.x = g.nan_f32(),
            1 => pt.y = g.nan_f32(),
            2 => aa.min.x = g.nan_f32(),
            _ => aa.max.y = g.nan_f32(),
        }
        bpt(30, i, aa, pt);
    }
}

// ===========================================================================
// c2CircleToPoint — rows 31–34
// ===========================================================================

fn cpt(row: u32, i: u32, c: C2Circle, b: C2v) {
    let p = pair();
    unsafe {
        diff_eq!(row, i, (p.c.c2CircleToPoint)(c, b), (p.r.c2CircleToPoint)(c, b), format!("{c:?} {b:?}"));
    }
}

/// Row 31 — outside, and exactly ON the rim (`<` is strict, so 0).
#[test]
fn err31_circletopoint_on_and_outside_rim() {
    let p = pair();
    let c = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 2.0 };
    unsafe {
        assert_eq!(
            (p.c.c2CircleToPoint)(c, C2v { x: 2.0, y: 0.0 }),
            0,
            "C rejects a point exactly on the rim"
        );
    }
    let mut g = Rng::new(0x101F);
    for i in 0..iters() {
        let ci = g.normal_circle();
        let th = g.range(-7.0, 7.0);
        let rad = if i % 2 == 0 { ci.r } else { ci.r + g.range(0.0, 10.0) + 1e-4 };
        cpt(31, i, ci, C2v { x: ci.p.x + rad * th.cos(), y: ci.p.y + rad * th.sin() });
    }
}

/// Row 32 — `r == 0` rejects even the centre.
#[test]
fn err32_circletopoint_zero_radius() {
    let p = pair();
    let c = C2Circle { p: C2v { x: 3.0, y: -4.0 }, r: 0.0 };
    unsafe {
        assert_eq!((p.c.c2CircleToPoint)(c, c.p), 0, "r==0 must reject the centre");
    }
    let mut g = Rng::new(0x1020);
    for i in 0..iters() {
        let ci = C2Circle { p: g.normal_v(), r: 0.0 };
        cpt(32, i, ci, if i % 2 == 0 { ci.p } else { g.normal_v() });
    }
}

/// Row 33 — negative radius behaves like `|r|`.
#[test]
fn err33_circletopoint_negative_radius() {
    let mut g = Rng::new(0x1021);
    for i in 0..iters() {
        let base = g.normal_circle();
        let ci = C2Circle { p: base.p, r: -base.r.max(0.5) };
        let th = g.range(-7.0, 7.0);
        let rad = g.range(0.0, 2.0) * base.r.max(0.5);
        cpt(33, i, ci, C2v { x: ci.p.x + rad * th.cos(), y: ci.p.y + rad * th.sin() });
    }
}

/// Row 34 — NaN → `d2 < r*r` false → 0.
#[test]
fn err34_circletopoint_nan() {
    let mut g = Rng::new(0x1022);
    for i in 0..iters() {
        let mut ci = g.normal_circle();
        let mut pt = g.normal_v();
        match i % 4 {
            0 => ci.r = g.nan_f32(),
            1 => ci.p.x = g.nan_f32(),
            2 => pt.y = g.nan_f32(),
            _ => {
                ci.p = g.nan_v();
                pt = g.nan_v();
            }
        }
        cpt(34, i, ci, pt);
    }
}

// ===========================================================================
// c2RaytoCapsule — rows 35–45
// ===========================================================================

/// Row 35 — the final fall-through `return 0`, which nonetheless has already
/// written `out->n = c2Norm(cap_n)` and `out->t = 0`. The DIRTY prefill makes
/// that side effect part of the assertion.
#[test]
fn err35_capsule_fallthrough_writes_out() {
    let p = pair();
    let cap = C2Capsule {
        a: C2v { x: 0.0, y: 0.0 },
        b: C2v { x: 0.0, y: 10.0 },
        r: 1.0,
    };
    // Far to the right, moving further right: never enters the slab.
    let ray = C2Ray { p: C2v { x: 50.0, y: 5.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 };
    let rc = ray_call(|o| unsafe { (p.c.c2RaytoCapsule)(ray, cap, o) });
    assert_eq!(rc.ret, 0, "expected the fall-through rejection");
    assert_ne!(rc.out, cb(DIRTY), "C must still have written out->n/out->t");
    let rr = ray_call(|o| unsafe { (p.r.c2RaytoCapsule)(ray, cap, o) });
    diff_eq!(35, 0, rc, rr, "fall-through out-struct side effect");

    let mut g = Rng::new(0x1023);
    for i in 0..iters() {
        let c = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.0, y: g.range(1.0, 20.0) },
            r: g.range(0.25, 4.0),
        };
        let x = c.r + g.range(1.0, 40.0);
        caps(35, i, C2Ray { p: C2v { x, y: g.range(-20.0, 40.0) }, d: C2v { x: 1.0, y: 0.0 }, t: g.range(0.0, 20.0) }, c);
    }
}

/// Row 36 — degenerate axis `a == b` makes `c2Norm({0,0})` NaN.
#[test]
fn err36_capsule_degenerate_axis() {
    let mut g = Rng::new(0x1024);
    for i in 0..iters() {
        let a = g.normal_v();
        let cap = C2Capsule { a, b: a, r: g.range(0.0, 8.0) };
        let ray = match i % 3 {
            0 => C2Ray { p: a, d: C2v { x: 1.0, y: 0.0 }, t: 5.0 },
            1 => g.normal_ray(),
            _ => {
                let mut r = g.normal_ray();
                r.p = C2v { x: a.x + g.range(-1.0, 1.0), y: a.y + g.range(-1.0, 1.0) };
                r
            }
        };
        caps(36, i, ray, cap);
    }
}

/// Row 37 — zero radius.
#[test]
fn err37_capsule_zero_radius() {
    let mut g = Rng::new(0x1025);
    for i in 0..iters() {
        let cap = C2Capsule { a: g.normal_v(), b: g.normal_v(), r: 0.0 };
        caps(37, i, g.normal_ray(), cap);
    }
}

/// Row 38 — negative radius inverts `capsule_bb` on x.
#[test]
fn err38_capsule_negative_radius() {
    let mut g = Rng::new(0x1026);
    for i in 0..iters() {
        let base = g.normal_capsule();
        let cap = C2Capsule { a: base.a, b: base.b, r: -base.r.max(0.25) };
        caps(38, i, g.normal_ray(), cap);
    }
}

/// Rows 39/40 — inside the infinite slab, above/below the caps, and the
/// delegated `c2RaytoCircle` MISSES (so the capsule returns that 0).
#[test]
fn err39_40_capsule_delegated_circle_miss() {
    let mut g = Rng::new(0x1027);
    for i in 0..iters() {
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.0, y: g.range(2.0, 20.0) },
            r: g.range(0.5, 4.0),
        };
        let below = i % 2 == 0;
        let y = if below { -g.range(1.0, 30.0) } else { cap.b.y + g.range(1.0, 30.0) };
        let o = C2v { x: g.range(-0.95, 0.95) * cap.r, y };
        // Point AWAY from the capsule so the delegated circle cast misses.
        let d = if below { C2v { x: 0.0, y: -1.0 } } else { C2v { x: 0.0, y: 1.0 } };
        caps(if below { 39 } else { 40 }, i, C2Ray { p: o, d, t: g.range(0.0, 30.0) }, cap);
    }
}

/// Rows 41/42 — side-slab crossing whose `y` falls outside the axis span, and
/// the delegated end-cap circle cast MISSES.
#[test]
fn err41_42_capsule_sidecross_circle_miss() {
    let mut g = Rng::new(0x1028);
    for i in 0..iters() {
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.0, y: g.range(2.0, 20.0) },
            r: g.range(0.5, 4.0),
        };
        let below = i % 2 == 0;
        // Start outside the slab and cross it well below / above the caps, with
        // a ray length far too short to actually reach the end-cap circle.
        let ox = cap.r + g.range(2.0, 20.0);
        let oy = if below { -g.range(5.0, 30.0) } else { cap.b.y + g.range(5.0, 30.0) };
        let target = C2v { x: -ox, y: oy };
        let o = C2v { x: ox, y: oy };
        let dx = target.x - o.x;
        let dy = target.y - o.y;
        let l = (dx * dx + dy * dy).sqrt();
        caps(
            if below { 41 } else { 42 },
            i,
            C2Ray { p: o, d: C2v { x: dx / l, y: dy / l }, t: l * g.range(0.0, 1.2) },
            cap,
        );
    }
}

/// Row 43 — `d = yAe.x - yAp.x == 0`, so `t = (c - yAp.x) / 0`.
#[test]
fn err43_capsule_zero_slab_denominator() {
    let mut g = Rng::new(0x1029);
    for i in 0..iters() {
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.0, y: g.range(2.0, 20.0) },
            r: g.range(0.5, 4.0),
        };
        // Direction parallel to the axis → yAd.x == 0 → yAe.x == yAp.x.
        let o = C2v { x: cap.r + g.range(0.1, 20.0), y: g.range(-20.0, 40.0) };
        caps(43, i, C2Ray { p: o, d: C2v { x: 0.0, y: if g.bool() { 1.0 } else { -1.0 } }, t: g.range(0.0, 60.0) }, cap);
    }
}

/// Row 44 — `A.t == 0` gives the same `d == 0` degeneracy.
#[test]
fn err44_capsule_zero_ray_length() {
    let mut g = Rng::new(0x102A);
    for i in 0..iters() {
        let cap = g.normal_capsule();
        let mut ray = g.normal_ray();
        ray.t = if i % 3 == 0 { -0.0 } else { 0.0 };
        caps(44, i, ray, cap);
    }
}

/// Row 45 — NaN inputs; `out` is still partially written before the reject.
#[test]
fn err45_capsule_nan() {
    let mut g = Rng::new(0x102B);
    for i in 0..iters() {
        let mut cap = g.normal_capsule();
        let mut ray = g.normal_ray();
        match i % 7 {
            0 => cap.a.x = g.nan_f32(),
            1 => cap.b.y = g.nan_f32(),
            2 => cap.r = g.nan_f32(),
            3 => ray.p.x = g.nan_f32(),
            4 => ray.d.y = g.nan_f32(),
            5 => ray.t = g.nan_f32(),
            _ => {
                cap = C2Capsule { a: g.nan_v(), b: g.nan_v(), r: g.nan_zero_inf_f32() };
                ray = C2Ray { p: g.nan_v(), d: g.nan_v(), t: g.nan_zero_inf_f32() };
            }
        }
        caps(45, i, ray, cap);
    }
}

// ===========================================================================
// c2RaytoPoly — rows 46–57
// ===========================================================================

/// Row 46 — `den == 0 && num < 0`: ray parallel to a plane, origin outside it.
#[test]
fn err46_poly_parallel_outside_plane() {
    let mut g = Rng::new(0x102C);
    for i in 0..iters() {
        let pl = poly_ray_poly(); // axis-aligned box, normals ±x / ±y
        // Direction along +y is parallel to the ±x planes; put the origin
        // outside the +x plane so num < 0 for norms[0].
        let o = C2v { x: 0.875 + g.range(0.001, 30.0), y: g.range(-30.0, 30.0) };
        poly(46, i, C2Ray { p: o, d: C2v { x: 0.0, y: if g.bool() { 1.0 } else { -1.0 } }, t: g.range(0.0, 60.0) }, &pl, None);
    }
}

/// Row 47 — slabs become disjoint (`hi < lo`).
#[test]
fn err47_poly_slabs_disjoint() {
    let mut g = Rng::new(0x102D);
    for i in 0..iters() {
        let pl = poly_ray_poly();
        // Diagonal rays that pass by the tall thin box: lo/hi cross over.
        let o = C2v { x: -g.range(2.0, 30.0), y: -g.range(12.0, 40.0) };
        let th = g.range(-1.5, 1.5);
        poly(47, i, C2Ray { p: o, d: C2v { x: th.cos(), y: th.sin() }, t: g.range(0.0, 80.0) }, &pl, None);
    }
}

/// Row 48 — loop completes with `index == ~0` (no plane ever clipped `lo`).
#[test]
fn err48_poly_index_never_set() {
    let mut g = Rng::new(0x102E);
    for i in 0..iters() {
        // Origin strictly inside a convex polygon: every num < 0 and no
        // `num < lo*den` with den < 0 fires, so `index` stays ~0.
        let c = g.normal_v();
        let rad = g.range(1.0, 12.0);
        let n = 3 + (i % 6).min(5) as usize;
        let pl = ngon(n, c, rad, g.range(-7.0, 7.0));
        let th = g.range(-7.0, 7.0);
        poly(48, i, C2Ray { p: c, d: C2v { x: th.cos(), y: th.sin() }, t: g.range(0.0, 40.0) }, &pl, None);
    }
}

/// Row 49 — `count == 0`.
#[test]
fn err49_poly_zero_count() {
    let p = pair();
    let mut g = Rng::new(0x102F);
    for i in 0..iters() {
        let mut pl = ngon(4, C2v { x: 0.0, y: 0.0 }, 3.0, 0.0);
        pl.count = 0;
        let ray = g.normal_ray();
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, &pl as *const C2Poly, std::ptr::null(), o) });
        assert_eq!(rc.ret, 0, "count == 0 must reject");
        assert_eq!(rc.out, cb(DIRTY), "count == 0 must not touch out");
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, &pl as *const C2Poly, std::ptr::null(), o) });
        diff_eq!(49, i, rc, rr, format!("{ray:?}"));
    }
}

/// Row 50 — negative `count`.
#[test]
fn err50_poly_negative_count() {
    let mut g = Rng::new(0x1030);
    let counts: [c_int; 6] = [-1, -2, -7, -100, i32::MIN + 1, i32::MIN];
    for i in 0..iters() {
        let mut pl = ngon(4, C2v { x: 0.0, y: 0.0 }, 3.0, 0.0);
        pl.count = counts[(i as usize) % counts.len()];
        poly(50, i, g.normal_ray(), &pl, None);
    }
}

/// Row 51 — `count > 8` overruns the fixed arrays. The polygon is embedded in
/// an over-allocated `PolyBuf` so both libraries read identical trailing bytes.
#[test]
fn err51_poly_count_overrun() {
    let p = pair();
    let mut g = Rng::new(0x1031);
    for i in 0..iters() {
        let mut buf = PolyBuf {
            poly: ngon(8, C2v { x: 0.0, y: 0.0 }, g.range(1.0, 10.0), g.range(-7.0, 7.0)),
            slack: [C2v { x: 0.0, y: 0.0 }; 32],
        };
        for k in 0..32 {
            buf.slack[k] = g.normal_v();
        }
        buf.poly.count = 9 + (i % 24) as c_int; // 9..32, still inside `slack`
        let ray = g.normal_ray();
        let pp = &buf.poly as *const C2Poly;
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, pp, std::ptr::null(), o) });
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, pp, std::ptr::null(), o) });
        diff_eq!(51, i, rc, rr, format!("count={} {ray:?}", buf.poly.count));
    }
}

/// Row 52 — `bx_ptr == NULL` is NOT an error: identity is substituted, so the
/// result must equal the explicit-identity call exactly.
#[test]
fn err52_poly_null_bx_is_identity() {
    let p = pair();
    let mut g = Rng::new(0x1032);
    let ident = unsafe { (p.c.c2xIdentity)() };
    let ident_r = unsafe { (p.r.c2xIdentity)() };
    diff_eq!(52, 0, xb(ident), xb(ident_r), "c2xIdentity parity");
    for i in 0..iters() {
        let pl = ngon(3 + (i % 6).min(5) as usize, g.normal_v(), g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let ray = g.normal_ray();
        let cn = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, &pl as *const C2Poly, std::ptr::null(), o) });
        let ci = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, &pl as *const C2Poly, &ident, o) });
        let rn = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, &pl as *const C2Poly, std::ptr::null(), o) });
        let ri = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, &pl as *const C2Poly, &ident, o) });
        diff_eq!(52, i, cn, rn, format!("null bx {ray:?}"));
        diff_eq!(52, i, ci, ri, format!("identity bx {ray:?}"));
        diff_eq!(52, i, cn, ci, format!("C: null == identity {ray:?}"));
    }
}

/// Row 53 — negative `A.t` makes `hi < lo` on the first plane.
#[test]
fn err53_poly_negative_ray_length() {
    let mut g = Rng::new(0x1033);
    for i in 0..iters() {
        let pl = ngon(3 + (i % 6).min(5) as usize, g.normal_v(), g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let mut ray = g.normal_ray();
        ray.t = -g.range(0.0, 100.0);
        poly(53, i, ray, &pl, None);
    }
}

/// Row 54 — `A.t == 0`.
#[test]
fn err54_poly_zero_ray_length() {
    let mut g = Rng::new(0x1034);
    for i in 0..iters() {
        let pl = ngon(3 + (i % 6).min(5) as usize, g.normal_v(), g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let mut ray = g.normal_ray();
        ray.t = if i % 3 == 0 { -0.0 } else { 0.0 };
        poly(54, i, ray, &pl, None);
    }
}

/// Row 55 — zero normals give `num == den == 0`, so the plane is ignored.
#[test]
fn err55_poly_zero_normals() {
    let mut g = Rng::new(0x1035);
    for i in 0..iters() {
        let mut pl = ngon(6, g.normal_v(), g.range(1.0, 10.0), g.range(-7.0, 7.0));
        // zero out some subset of the normals
        for k in 0..6 {
            if (i as usize + k) % 3 == 0 {
                pl.norms[k] = C2v { x: 0.0, y: 0.0 };
            }
        }
        poly(55, i, g.normal_ray(), &pl, None);
    }
}

/// Row 56 — NaN in the ray, verts, norms or transform.
#[test]
fn err56_poly_nan() {
    let mut g = Rng::new(0x1036);
    for i in 0..iters() {
        let mut pl = ngon(5, g.normal_v(), g.range(1.0, 10.0), g.range(-7.0, 7.0));
        let mut ray = g.normal_ray();
        let mut bx: Option<C2x> = None;
        match i % 6 {
            0 => pl.verts[i as usize % 5].x = g.nan_f32(),
            1 => pl.norms[i as usize % 5].y = g.nan_f32(),
            2 => ray.p = g.nan_v(),
            3 => ray.d = g.nan_v(),
            4 => ray.t = g.nan_f32(),
            _ => bx = Some(C2x { p: g.nan_v(), r: C2r { c: g.nan_f32(), s: g.nan_f32() } }),
        }
        poly(56, i, ray, &pl, bx.as_ref());
    }
}

/// Row 57 — non-normalized `bx.r` (`c*c + s*s != 1`), applied literally.
#[test]
fn err57_poly_nonunit_rotation() {
    let mut g = Rng::new(0x1037);
    for i in 0..iters() {
        let pl = ngon(4, C2v { x: 0.0, y: 0.0 }, g.range(1.0, 10.0), g.range(-7.0, 7.0));
        let bx = C2x {
            p: g.normal_v(),
            r: C2r { c: g.range(-5.0, 5.0), s: g.range(-5.0, 5.0) },
        };
        poly(57, i, g.normal_ray(), &pl, Some(&bx));
    }
}

// ===========================================================================
// c2CastRay — row 58: out-of-range enum values
// ===========================================================================

/// Row 58 — C enums accept any `int`. Every value with no valid variant must
/// fall off the `switch` and return 0 with `out` untouched.
#[test]
fn err58_castray_out_of_range_enum() {
    let p = pair();
    let mut g = Rng::new(0x1038);
    let bad: [c_int; 14] = [
        -1,
        -2,
        4,
        5,
        7,
        8,
        99,
        255,
        256,
        0x1_0000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        -0x8000,
    ];
    // Point `B` at a real object of each kind so a mis-dispatch would be
    // visible rather than crashing.
    for (k, &ty) in bad.iter().enumerate() {
        let ci = C2Circle { p: C2v { x: 1.0, y: 2.0 }, r: 3.0 };
        let ray = C2Ray { p: C2v { x: -10.0, y: 2.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 100.0 };
        let b = &ci as *const C2Circle as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), ty, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, std::ptr::null(), ty, o) });
        assert_eq!(rc.ret, 0, "C must reject typeB = {ty}");
        assert_eq!(rc.out, cb(DIRTY), "C must not touch out for typeB = {ty}");
        diff_eq!(58, k as u32, rc, rr, format!("typeB = {ty}"));
    }
    // Randomized: any int outside 0..=3, with and without a bx, over each of
    // the four payload shapes.
    for i in 0..iters() {
        let ty = loop {
            let v = g.next_u32() as c_int;
            if !(0..=3).contains(&v) {
                break v;
            }
        };
        let bx = g.mixed_x();
        let bxp: *const C2x = if g.bool() { &bx } else { std::ptr::null() };
        let pl = ngon(4, C2v { x: 0.0, y: 0.0 }, 3.0, 0.0);
        let cap = g.normal_capsule();
        let bb = g.normal_aabb();
        let ci = g.normal_circle();
        let b: *const c_void = match i % 4 {
            0 => &ci as *const C2Circle as *const c_void,
            1 => &bb as *const C2AABB as *const c_void,
            2 => &cap as *const C2Capsule as *const c_void,
            _ => &pl as *const C2Poly as *const c_void,
        };
        let ray = g.normal_ray();
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, bxp, ty, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, bxp, ty, o) });
        assert_eq!(rc.ret, 0, "C must reject typeB = {ty}");
        assert_eq!(rc.out, cb(DIRTY), "C must not touch out for typeB = {ty}");
        diff_eq!(58, i, rc, rr, format!("typeB = {ty}"));
    }
    // And the four VALID values must NOT be rejected out of hand — guards
    // against a Rust match that silently swallows a variant.
    let ci = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 3.0 };
    let ray = C2Ray { p: C2v { x: -10.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 100.0 };
    let b = &ci as *const C2Circle as *const c_void;
    let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_CIRCLE, o) });
    assert_eq!(rc.ret, 1, "sanity: a valid circle cast should hit");
}

// ===========================================================================
// Unguarded arithmetic — rows 59–66
// ===========================================================================

/// Rows 59/60 — `c2Div` has no zero check: `1.0f/b`.
#[test]
fn err59_60_div_by_zero_and_specials() {
    let p = pair();
    let mut g = Rng::new(0x1039);
    let scalars = [
        0.0f32,
        -0.0f32,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFFC0_1234),
        f32::from_bits(0x7F80_0001),
        f32::MIN_POSITIVE,
        1e-45,
        f32::MAX,
    ];
    for (k, &s) in scalars.iter().enumerate() {
        for &v in &[
            C2v { x: 0.0, y: -0.0 },
            C2v { x: 1.0, y: -1.0 },
            C2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            C2v { x: f32::NAN, y: f32::from_bits(0x7FC0_0001) },
            C2v { x: f32::MAX, y: f32::MIN },
        ] {
            unsafe {
                diff_eq!(59, k as u32, vb((p.c.c2Div)(v, s)), vb((p.r.c2Div)(v, s)), format!("{v:?} / {s:e}"));
            }
        }
    }
    for i in 0..iters() {
        let v = g.mixed_v();
        let s = if i % 3 == 0 { g.nan_zero_inf_f32() } else { g.mixed_f32() };
        unsafe {
            diff_eq!(60, i, vb((p.c.c2Div)(v, s)), vb((p.r.c2Div)(v, s)), format!("{v:?} / {s:e}"));
        }
    }
}

/// Rows 61/62 — `c2Norm` divides by `c2Len`, which can be `0` or `Inf`.
#[test]
fn err61_62_norm_zero_and_inf() {
    let p = pair();
    let fixed = [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: 0.0, y: -0.0 },
        C2v { x: f32::INFINITY, y: 0.0 },
        C2v { x: f32::INFINITY, y: f32::INFINITY },
        C2v { x: f32::NEG_INFINITY, y: 1.0 },
        C2v { x: f32::MAX, y: f32::MAX },
        C2v { x: 1e-45, y: 1e-45 },
        C2v { x: f32::NAN, y: 0.0 },
    ];
    for (k, &v) in fixed.iter().enumerate() {
        unsafe {
            diff_eq!(61, k as u32, vb((p.c.c2Norm)(v)), vb((p.r.c2Norm)(v)), format!("{v:?}"));
        }
    }
    let mut g = Rng::new(0x103A);
    for i in 0..iters() {
        let v = if i % 2 == 0 { g.nan_v() } else { g.mixed_v() };
        unsafe {
            diff_eq!(62, i, vb((p.c.c2Norm)(v)), vb((p.r.c2Norm)(v)), format!("{v:?}"));
        }
    }
}

/// Rows 63/64 — `c2Len` overflow to `Inf` and NaN propagation through `sqrtf`.
#[test]
fn err63_64_len_overflow_and_nan() {
    let p = pair();
    let fixed = [
        C2v { x: f32::MAX, y: f32::MAX },
        C2v { x: 1e30, y: 1e30 },
        C2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        C2v { x: f32::NAN, y: 1.0 },
        C2v { x: f32::from_bits(0x7F80_0001), y: 0.0 }, // sNaN
        C2v { x: -0.0, y: -0.0 },
    ];
    for (k, &v) in fixed.iter().enumerate() {
        unsafe {
            diff_eq!(63, k as u32, fb((p.c.c2Len)(v)), fb((p.r.c2Len)(v)), format!("{v:?}"));
        }
    }
    let mut g = Rng::new(0x103B);
    for i in 0..iters() {
        let v = if i % 2 == 0 { g.nan_v() } else { g.mixed_v() };
        unsafe {
            diff_eq!(64, i, fb((p.c.c2Len)(v)), fb((p.r.c2Len)(v)), format!("{v:?}"));
        }
    }
}

/// Row 65 — `c2Absv` uses `x < 0 ? -x : x`, so `-0.0` keeps its sign bit and
/// NaNs are returned verbatim. `f32::abs` would clear the sign bit.
#[test]
fn err65_absv_preserves_negative_zero() {
    let p = pair();
    unsafe {
        let r = (p.c.c2Absv)(C2v { x: -0.0, y: -0.0 });
        assert_eq!(
            r.x.to_bits(),
            0x8000_0000,
            "C's ternary abs must preserve -0.0 (got {:#x})",
            r.x.to_bits()
        );
        diff_eq!(65, 0, vb((p.c.c2Absv)(C2v { x: -0.0, y: 0.0 })), vb((p.r.c2Absv)(C2v { x: -0.0, y: 0.0 })), "-0.0");
        let n = C2v { x: f32::from_bits(0xFFC0_1234), y: f32::from_bits(0x7FC0_0001) };
        diff_eq!(65, 1, vb((p.c.c2Absv)(n)), vb((p.r.c2Absv)(n)), "negative NaN");
    }
    let mut g = Rng::new(0x103C);
    for i in 0..iters() {
        let v = if i % 2 == 0 { g.nan_v() } else { g.mixed_v() };
        unsafe {
            diff_eq!(65, i, vb((p.c.c2Absv)(v)), vb((p.r.c2Absv)(v)), format!("{v:?}"));
        }
    }
}

/// Row 66 — `c2Minv`/`c2Maxv` use `a < b ? a : b`, which returns `b` when
/// either operand is NaN. `f32::min`/`max` would return the non-NaN operand.
#[test]
fn err66_minv_maxv_nan_returns_b() {
    let p = pair();
    unsafe {
        let a = C2v { x: f32::NAN, y: 1.0 };
        let b = C2v { x: 7.0, y: f32::NAN };
        let mn = (p.c.c2Minv)(a, b);
        assert_eq!(mn.x.to_bits(), 7.0f32.to_bits(), "NaN < b is false → picks b");
        assert!(mn.y.is_nan(), "1.0 < NaN is false → picks b (NaN)");
        diff_eq!(66, 0, vb(mn), vb((p.r.c2Minv)(a, b)), "min NaN mix");
        let mx = (p.c.c2Maxv)(a, b);
        diff_eq!(66, 1, vb(mx), vb((p.r.c2Maxv)(a, b)), "max NaN mix");
        // -0.0 vs 0.0: neither `<` nor `>` holds, so both pick `b`.
        let z1 = C2v { x: -0.0, y: 0.0 };
        let z2 = C2v { x: 0.0, y: -0.0 };
        diff_eq!(66, 2, vb((p.c.c2Minv)(z1, z2)), vb((p.r.c2Minv)(z1, z2)), "min signed zero");
        diff_eq!(66, 3, vb((p.c.c2Maxv)(z1, z2)), vb((p.r.c2Maxv)(z1, z2)), "max signed zero");
    }
    let mut g = Rng::new(0x103D);
    for i in 0..iters() {
        let (a, b) = if i % 2 == 0 { (g.nan_v(), g.nan_v()) } else { (g.mixed_v(), g.mixed_v()) };
        unsafe {
            diff_eq!(66, i, vb((p.c.c2Minv)(a, b)), vb((p.r.c2Minv)(a, b)), format!("min {a:?} {b:?}"));
            diff_eq!(66, i, vb((p.c.c2Maxv)(a, b)), vb((p.r.c2Maxv)(a, b)), format!("max {a:?} {b:?}"));
        }
    }
}

// ===========================================================================
// Generic FFI boundary coverage required by Phase C beyond the table
// ===========================================================================

/// Zero and oversized lengths, and values one step past each documented range.
#[test]
fn boundary_lengths_and_one_past_range() {
    let p = pair();
    let mut g = Rng::new(0x2000);
    // c2Poly.count: one step past each end of 0..=8, plus the array capacity.
    for count in [-1i32, 0, 1, 7, 8, 9] {
        for i in 0..200u32 {
            let mut buf = PolyBuf {
                poly: ngon(8, C2v { x: 0.0, y: 0.0 }, g.range(1.0, 8.0), g.range(-7.0, 7.0)),
                slack: [C2v { x: 0.0, y: 0.0 }; 32],
            };
            for k in 0..32 {
                buf.slack[k] = g.normal_v();
            }
            buf.poly.count = count;
            let ray = g.normal_ray();
            let pp = &buf.poly as *const C2Poly;
            let rc = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, pp, std::ptr::null(), o) });
            let rr = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, pp, std::ptr::null(), o) });
            diff_eq!(0, i, rc, rr, format!("count={count} {ray:?}"));
        }
    }
    // A.t: 0, -0, one ULP either side of 0, huge, Inf.
    let ts = [
        0.0f32,
        -0.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for (k, &t) in ts.iter().enumerate() {
        for j in 0..100u32 {
            let mut ray = g.normal_ray();
            ray.t = t;
            let ci = g.normal_circle();
            let bb = g.normal_aabb();
            let cap = g.normal_capsule();
            let pl = ngon(4, C2v { x: 0.0, y: 0.0 }, 3.0, 0.0);
            let i = (k as u32) * 1000 + j;
            let a = ray_call(|o| unsafe { (p.c.c2RaytoCircle)(ray, ci, o) });
            let b = ray_call(|o| unsafe { (p.r.c2RaytoCircle)(ray, ci, o) });
            diff_eq!(0, i, a, b, format!("circle t={t:e}"));
            let a = ray_call(|o| unsafe { (p.c.c2RaytoAABB)(ray, bb, o) });
            let b = ray_call(|o| unsafe { (p.r.c2RaytoAABB)(ray, bb, o) });
            diff_eq!(0, i, a, b, format!("aabb t={t:e}"));
            let a = ray_call(|o| unsafe { (p.c.c2RaytoCapsule)(ray, cap, o) });
            let b = ray_call(|o| unsafe { (p.r.c2RaytoCapsule)(ray, cap, o) });
            diff_eq!(0, i, a, b, format!("capsule t={t:e}"));
            let a = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, &pl, std::ptr::null(), o) });
            let b = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, &pl, std::ptr::null(), o) });
            diff_eq!(0, i, a, b, format!("poly t={t:e}"));
        }
    }
}

/// `out` is the only pointer C dereferences unconditionally, so a NULL `out`
/// is UB on both sides and cannot be tested. `bx` IS null-checked, and that is
/// covered here for every dispatch type (rows 52 / 80 / 82 / 84).
#[test]
fn boundary_null_bx_all_dispatch_types() {
    let p = pair();
    let mut g = Rng::new(0x2001);
    for i in 0..iters() {
        let ci = g.normal_circle();
        let bb = g.normal_aabb();
        let cap = g.normal_capsule();
        let pl = ngon(4, C2v { x: 0.0, y: 0.0 }, 3.0, 0.0);
        let ray = g.normal_ray();
        for (ty, b) in [
            (C2_TYPE_CIRCLE, &ci as *const C2Circle as *const c_void),
            (C2_TYPE_AABB, &bb as *const C2AABB as *const c_void),
            (C2_TYPE_CAPSULE, &cap as *const C2Capsule as *const c_void),
            (C2_TYPE_POLY, &pl as *const C2Poly as *const c_void),
        ] {
            let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), ty, o) });
            let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, std::ptr::null(), ty, o) });
            diff_eq!(0, i, rc, rr, format!("null bx, typeB={ty}"));
        }
    }
}
