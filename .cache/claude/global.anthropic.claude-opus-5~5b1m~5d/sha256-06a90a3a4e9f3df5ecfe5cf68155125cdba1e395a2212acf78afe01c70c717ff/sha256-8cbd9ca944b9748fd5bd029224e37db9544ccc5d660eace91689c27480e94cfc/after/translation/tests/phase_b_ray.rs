//! Phase B -- valid-path differential tests, CONFIGS.md rows 30..=68
//! (the raycast entry points, the `c2CastRay` dispatcher and `spec_ray`).
//!
//! `c2Ray` and `c2Capsule` are 20-byte structs, i.e. SysV class MEMORY, so
//! driving these through the `.so` also exercises the stack-argument ABI of the
//! `#[no_mangle] extern "C"` wrappers.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x2545_F491_4F6C_DD1D;
/// Base iteration count for the randomized sweeps; scaled by `$DIFF_SCALE`.
fn n() -> usize { iters(8_000) }

/* ------------------------------------------------------------- generators */

/// A ray with a normalized direction, built the way a real consumer does:
/// `d = c2Norm(target - origin)`, `t = |target - origin|` (via the `.so`).
fn ray_towards(g: &mut Rng, origin: c2v, target: c2v, t_scale: f32) -> c2Ray {
    let l = libs();
    let delta = unsafe { (l.c.c2Sub)(target, origin) };
    let d = unsafe { (l.c.c2Norm)(delta) };
    let len = unsafe { (l.c.c2Len)(delta) };
    let _ = g;
    c2Ray {
        p: origin,
        d,
        t: len * t_scale,
    }
}

fn rand_ray_normalized(g: &mut Rng) -> c2Ray {
    let o = g.coord_v();
    let tgt = g.coord_v();
    ray_towards(g, o, tgt, 1.0)
}

/// A ray with a completely arbitrary (non-unit) direction -- still a perfectly
/// valid input, since `c2Rayto*` never re-normalizes `A.d`.
fn rand_ray_raw(g: &mut Rng) -> c2Ray {
    c2Ray {
        p: g.coord_v(),
        d: g.coord_v(),
        t: g.coord(),
    }
}

fn wild_ray(g: &mut Rng) -> c2Ray {
    c2Ray {
        p: g.wild_v(),
        d: g.wild_v(),
        t: g.wild(),
    }
}

fn rand_circle(g: &mut Rng) -> c2Circle {
    c2Circle {
        p: g.coord_v(),
        r: g.radius(),
    }
}

fn rand_box(g: &mut Rng) -> c2AABB {
    let (x0, y0) = (g.coord(), g.coord());
    let (w, h) = (g.radius(), g.radius());
    c2AABB {
        min: v(x0, y0),
        max: v(x0 + w, y0 + h),
    }
}

fn rand_capsule(g: &mut Rng) -> c2Capsule {
    c2Capsule {
        a: g.coord_v(),
        b: g.coord_v(),
        r: g.radius(),
    }
}

/* ==================== rows 30..36 : c2RaytoCircle ======================== */

#[test]
fn cfg30_raytocircle_property_sweep() {
    let mut g = Rng::new(SEED ^ 30);
    for i in 0..n() {
        let c = rand_circle(&mut g);
        let ray = rand_ray_normalized(&mut g);
        diff_raytocircle(&format!("row30 i={i}"), ray, c);
    }
}

#[test]
fn cfg31_raytocircle_guaranteed_hit() {
    let mut g = Rng::new(SEED ^ 31);
    for i in 0..n() {
        let c = c2Circle {
            p: g.coord_v(),
            r: 1.0 + g.uniform(5.0).abs(),
        };
        // origin well outside, aimed exactly at the centre
        let dist = c.r + 1.0 + g.uniform(20.0).abs();
        let ang = g.uniform(3.14159265);
        let origin = v(c.p.x + dist * ang.cos(), c.p.y + dist * ang.sin());
        for scale in [1.0f32, 2.0, 10.0] {
            let ray = ray_towards(&mut g, origin, c.p, scale);
            diff_raytocircle(&format!("row31 i={i} scale={scale}"), ray, c);
        }
    }
}

#[test]
fn cfg32_raytocircle_three_miss_reasons() {
    let mut g = Rng::new(SEED ^ 32);
    for i in 0..n() {
        let c = c2Circle {
            p: g.coord_v(),
            r: 1.0 + g.uniform(5.0).abs(),
        };
        let dist = c.r + 2.0 + g.uniform(20.0).abs();
        let ang = g.uniform(3.14159265);
        let origin = v(c.p.x + dist * ang.cos(), c.p.y + dist * ang.sin());

        // (a) disc < 0 : aim perpendicular, far to the side
        let side = v(
            c.p.x + (c.r * 10.0 + 5.0) * (ang + 1.5708).cos(),
            c.p.y + (c.r * 10.0 + 5.0) * (ang + 1.5708).sin(),
        );
        diff_raytocircle(
            &format!("row32 i={i} disc<0"),
            ray_towards(&mut g, origin, side, 10.0),
            c,
        );

        // (b) t < 0 : aim away from the circle (circle behind the origin)
        let away = v(
            origin.x + (origin.x - c.p.x),
            origin.y + (origin.y - c.p.y),
        );
        diff_raytocircle(
            &format!("row32 i={i} t<0"),
            ray_towards(&mut g, origin, away, 10.0),
            c,
        );

        // (c) t > A.t : correct aim but the ray is far too short
        let mut short = ray_towards(&mut g, origin, c.p, 1.0);
        short.t = (dist - c.r) * 0.25;
        diff_raytocircle(&format!("row32 i={i} t>A.t"), short, c);
    }
}

#[test]
fn cfg33_raytocircle_tangent_and_exact_boundaries() {
    let mut g = Rng::new(SEED ^ 33);
    for i in 0..n() {
        let ctr = v(
            (g.below(21) as f32) - 10.0,
            (g.below(21) as f32) - 10.0,
        );
        let r = (g.below(6) + 1) as f32;
        let c = c2Circle { p: ctr, r };

        // exact tangent: axis-aligned ray offset by exactly r
        let ray = c2Ray {
            p: v(ctr.x - 100.0, ctr.y + r),
            d: v(1.0, 0.0),
            t: 1000.0,
        };
        diff_raytocircle(&format!("row33 i={i} tangent"), ray, c);

        // origin exactly on the rim, aimed inward => t == 0
        let ray = c2Ray {
            p: v(ctr.x - r, ctr.y),
            d: v(1.0, 0.0),
            t: 100.0,
        };
        diff_raytocircle(&format!("row33 i={i} t==0"), ray, c);

        // impact exactly at t == A.t (boundary of `t <= A.t`)
        let dist = r + 10.0;
        let ray = c2Ray {
            p: v(ctr.x - dist, ctr.y),
            d: v(1.0, 0.0),
            t: dist - r,
        };
        diff_raytocircle(&format!("row33 i={i} t==A.t"), ray, c);
        // one ULP short / one ULP long
        for delta in [-1i32, 1] {
            let mut r2 = ray;
            r2.t = f32::from_bits((ray.t.to_bits() as i32 + delta) as u32);
            diff_raytocircle(&format!("row33 i={i} t==A.t{delta:+}"), r2, c);
        }
    }
}

#[test]
fn cfg34_raytocircle_origin_inside() {
    let mut g = Rng::new(SEED ^ 34);
    for i in 0..n() {
        let c = c2Circle {
            p: g.coord_v(),
            r: 1.0 + g.uniform(10.0).abs(),
        };
        // origin strictly inside
        let f = 0.9 * (g.below(100) as f32 / 100.0);
        let ang = g.uniform(3.14159265);
        let origin = v(
            c.p.x + c.r * f * ang.cos(),
            c.p.y + c.r * f * ang.sin(),
        );
        let tgt = g.coord_v();
        diff_raytocircle(
            &format!("row34 i={i}"),
            ray_towards(&mut g, origin, tgt, 5.0),
            c,
        );
        // origin exactly at the centre
        diff_raytocircle(
            &format!("row34 i={i} centre"),
            ray_towards(&mut g, c.p, tgt, 5.0),
            c,
        );
    }
}

#[test]
fn cfg35_raytocircle_non_unit_direction_and_degenerate_t() {
    let mut g = Rng::new(SEED ^ 35);
    for i in 0..n() {
        let c = rand_circle(&mut g);
        diff_raytocircle(&format!("row35 i={i} raw-d"), rand_ray_raw(&mut g), c);
        for t in [
            0.0f32,
            -0.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
        ] {
            let mut r = rand_ray_normalized(&mut g);
            r.t = t;
            diff_raytocircle(&format!("row35 i={i} t={t:e}"), r, c);
        }
        let mut r = rand_ray_normalized(&mut g);
        r.d = v(0.0, 0.0);
        diff_raytocircle(&format!("row35 i={i} d=0"), r, c);
        r.d = v(-0.0, -0.0);
        diff_raytocircle(&format!("row35 i={i} d=-0"), r, c);
    }
}

#[test]
fn cfg36_raytocircle_degenerate_radii_and_nan_normal() {
    let mut g = Rng::new(SEED ^ 36);
    for i in 0..n() {
        let ctr = g.coord_v();
        for r in [
            0.0f32,
            -0.0,
            -1.0,
            -5.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MIN_POSITIVE,
            1e30,
            f32::MAX,
        ] {
            let c = c2Circle { p: ctr, r };
            // aimed exactly at the centre from a distance
            let origin = v(ctr.x - 10.0, ctr.y);
            diff_raytocircle(
                &format!("row36 i={i} r={r:e} aimed"),
                c2Ray {
                    p: origin,
                    d: v(1.0, 0.0),
                    t: 100.0,
                },
                c,
            );
            diff_raytocircle(&format!("row36 i={i} r={r:e} rand"), rand_ray_raw(&mut g), c);
        }
        // r == 0 with the impact exactly at B.p => c2Norm((0,0)) => NaN normal
        diff_raytocircle(
            &format!("row36 i={i} nan-normal"),
            c2Ray {
                p: v(ctr.x - 4.0, ctr.y),
                d: v(1.0, 0.0),
                t: 100.0,
            },
            c2Circle { p: ctr, r: 0.0 },
        );
        // fully wild
        diff_raytocircle(
            &format!("row36 i={i} wild"),
            wild_ray(&mut g),
            c2Circle {
                p: g.wild_v(),
                r: g.wild(),
            },
        );
    }
}

/* ==================== rows 37..43 : c2RaytoAABB ========================== */

#[test]
fn cfg37_raytoaabb_property_sweep() {
    let mut g = Rng::new(SEED ^ 37);
    for i in 0..n() {
        let bb = rand_box(&mut g);
        diff_raytoaabb(&format!("row37 i={i} norm"), rand_ray_normalized(&mut g), bb);
        diff_raytoaabb(&format!("row37 i={i} raw"), rand_ray_raw(&mut g), bb);
    }
}

#[test]
fn cfg38_raytoaabb_each_face_branch() {
    let mut g = Rng::new(SEED ^ 38);
    for i in 0..n() {
        let cx = g.coord();
        let cy = g.coord();
        let hw = 1.0 + g.uniform(5.0).abs();
        let hh = 1.0 + g.uniform(5.0).abs();
        let bb = c2AABB {
            min: v(cx - hw, cy - hh),
            max: v(cx + hw, cy + hh),
        };
        let ctr = v(cx, cy);
        let far = 3.0 * (hw + hh) + 10.0;
        // approach from each of the 4 sides, aimed at the centre
        for (k, o) in [
            v(cx - far, cy),
            v(cx + far, cy),
            v(cx, cy - far),
            v(cx, cy + far),
        ]
        .iter()
        .enumerate()
        {
            for scale in [1.0f32, 2.0] {
                diff_raytoaabb(
                    &format!("row38 i={i} side={k} scale={scale}"),
                    ray_towards(&mut g, *o, ctr, scale),
                    bb,
                );
            }
        }
        // diagonal approaches from each corner
        for (k, o) in [
            v(cx - far, cy - far),
            v(cx + far, cy - far),
            v(cx - far, cy + far),
            v(cx + far, cy + far),
        ]
        .iter()
        .enumerate()
        {
            diff_raytoaabb(
                &format!("row38 i={i} corner={k}"),
                ray_towards(&mut g, *o, ctr, 2.0),
                bb,
            );
        }
    }
}

#[test]
fn cfg39_raytoaabb_tied_t_values() {
    let mut g = Rng::new(SEED ^ 39);
    for i in 0..n() {
        let cx = (g.below(21) as f32) - 10.0;
        let cy = (g.below(21) as f32) - 10.0;
        let h = (g.below(4) + 1) as f32;
        let bb = c2AABB {
            min: v(cx - h, cy - h),
            max: v(cx + h, cy + h),
        };
        // ray origin exactly at the centre: every da is inside => several
        // c2RayToPlane results are 0, so t0..t3 tie at 0.
        for d in [v(1.0, 0.0), v(0.0, 1.0), v(1.0, 1.0), v(-1.0, -1.0)] {
            diff_raytoaabb(
                &format!("row39 i={i} centre d={d:?}"),
                c2Ray { p: v(cx, cy), d, t: 4.0 * h },
                bb,
            );
        }
        // exact corner-to-corner sweep (symmetric t values)
        diff_raytoaabb(
            &format!("row39 i={i} corner-sweep"),
            c2Ray {
                p: v(cx - h, cy - h),
                d: v(0.70710678, 0.70710678),
                t: 2.0 * h * 1.4142135,
            },
            bb,
        );
        // zero-length ray at the centre => all t == 0
        diff_raytoaabb(
            &format!("row39 i={i} zero-len"),
            c2Ray {
                p: v(cx, cy),
                d: v(1.0, 0.0),
                t: 0.0,
            },
            bb,
        );
    }
}

#[test]
fn cfg40_raytoaabb_three_miss_reasons() {
    let mut g = Rng::new(SEED ^ 40);
    for i in 0..n() {
        let bb = rand_box(&mut g);
        let w = (bb.max.x - bb.min.x).abs() + 1.0;
        let h = (bb.max.y - bb.min.y).abs() + 1.0;
        // (a) swept box entirely to one side => c2AABBtoAABB rejects
        let o = v(bb.min.x - 10.0 * w, bb.min.y);
        let tgt = v(bb.min.x - 5.0 * w, bb.max.y);
        diff_raytoaabb(
            &format!("row40 i={i} sweep-disjoint"),
            ray_towards(&mut g, o, tgt, 1.0),
            bb,
        );
        // (b) SAT reject: long diagonal ray that straddles the box's bounds but
        // passes outside the box itself
        let o = v(bb.min.x - w, bb.max.y + h);
        let tgt = v(bb.max.x + w, bb.max.y + h * 0.5 + h);
        diff_raytoaabb(
            &format!("row40 i={i} sat"),
            ray_towards(&mut g, o, tgt, 1.0),
            bb,
        );
        // (c) short ray aimed correctly but stopping before the box
        let ctr = v((bb.min.x + bb.max.x) * 0.5, (bb.min.y + bb.max.y) * 0.5);
        let o = v(bb.min.x - 10.0 * w, ctr.y);
        let mut r = ray_towards(&mut g, o, ctr, 1.0);
        r.t *= 0.1;
        diff_raytoaabb(&format!("row40 i={i} short"), r, bb);
    }
}

#[test]
fn cfg41_raytoaabb_axis_aligned_da_equals_db() {
    let mut g = Rng::new(SEED ^ 41);
    for i in 0..n() {
        let bb = rand_box(&mut g);
        let ctr = v((bb.min.x + bb.max.x) * 0.5, (bb.min.y + bb.max.y) * 0.5);
        // purely horizontal / vertical rays: for the perpendicular axis
        // da == db, so the `d != 0` guard in c2RayToPlane fires.
        for (k, d) in [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0)]
            .iter()
            .enumerate()
        {
            for t in [0.0f32, 1.0, 1e3] {
                diff_raytoaabb(
                    &format!("row41 i={i} k={k} t={t}"),
                    c2Ray { p: ctr, d: *d, t },
                    bb,
                );
                diff_raytoaabb(
                    &format!("row41 i={i} k={k} t={t} outside"),
                    c2Ray {
                        p: v(bb.min.x - 1.0, bb.min.y - 1.0),
                        d: *d,
                        t,
                    },
                    bb,
                );
            }
        }
        // ray exactly along the lower face
        diff_raytoaabb(
            &format!("row41 i={i} along-face"),
            c2Ray {
                p: v(bb.min.x - 5.0, bb.min.y),
                d: v(1.0, 0.0),
                t: 20.0,
            },
            bb,
        );
    }
}

#[test]
fn cfg42_raytoaabb_origin_inside_and_degenerate_t() {
    let mut g = Rng::new(SEED ^ 42);
    for i in 0..n() {
        let bb = rand_box(&mut g);
        let inside = v(
            (bb.min.x + bb.max.x) * 0.5,
            (bb.min.y + bb.max.y) * 0.5,
        );
        for t in [
            0.0f32,
            -0.0,
            -1.0,
            1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
        ] {
            diff_raytoaabb(
                &format!("row42 i={i} inside t={t:e}"),
                c2Ray {
                    p: inside,
                    d: v(0.6, 0.8),
                    t,
                },
                bb,
            );
            diff_raytoaabb(
                &format!("row42 i={i} d0 t={t:e}"),
                c2Ray {
                    p: inside,
                    d: v(0.0, 0.0),
                    t,
                },
                bb,
            );
        }
    }
}

#[test]
fn cfg43_raytoaabb_degenerate_boxes_and_nan() {
    let mut g = Rng::new(SEED ^ 43);
    for i in 0..n() {
        let p = g.coord_v();
        let zero = c2AABB { min: p, max: p };
        let b = rand_box(&mut g);
        let inv = c2AABB {
            min: b.max,
            max: b.min,
        };
        let huge = c2AABB {
            min: v(-1e30, -1e30),
            max: v(1e30, 1e30),
        };
        let infb = c2AABB {
            min: v(f32::NEG_INFINITY, f32::NEG_INFINITY),
            max: v(f32::INFINITY, f32::INFINITY),
        };
        let nanb = c2AABB {
            min: v(f32::NAN, 0.0),
            max: v(1.0, f32::NAN),
        };
        for (k, bb) in [zero, inv, huge, infb, nanb].iter().enumerate() {
            diff_raytoaabb(
                &format!("row43 i={i} box={k} norm"),
                rand_ray_normalized(&mut g),
                *bb,
            );
            diff_raytoaabb(&format!("row43 i={i} box={k} raw"), rand_ray_raw(&mut g), *bb);
            diff_raytoaabb(&format!("row43 i={i} box={k} wild"), wild_ray(&mut g), *bb);
        }
        diff_raytoaabb(
            &format!("row43 i={i} wild-box"),
            wild_ray(&mut g),
            c2AABB {
                min: g.wild_v(),
                max: g.wild_v(),
            },
        );
    }
}

/* ==================== rows 44..53 : c2RaytoCapsule ======================= */

#[test]
fn cfg44_raytocapsule_property_sweep() {
    let mut g = Rng::new(SEED ^ 44);
    for i in 0..n() {
        let cap = rand_capsule(&mut g);
        diff_raytocapsule(&format!("row44 i={i} norm"), rand_ray_normalized(&mut g), cap);
        diff_raytocapsule(&format!("row44 i={i} raw"), rand_ray_raw(&mut g), cap);
    }
}

/// A well-formed vertical-ish capsule plus its local frame.
fn nice_capsule(g: &mut Rng) -> c2Capsule {
    let a = g.coord_v();
    let ang = g.uniform(3.14159265);
    let len = 1.0 + g.uniform(20.0).abs();
    c2Capsule {
        a,
        b: v(a.x + len * ang.cos(), a.y + len * ang.sin()),
        r: 0.25 + g.uniform(4.0).abs(),
    }
}

#[test]
fn cfg45_raytocapsule_origin_in_slab() {
    let mut g = Rng::new(SEED ^ 45);
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        // midpoint of the segment is always inside the slab
        let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        for d in [v(1.0, 0.0), v(0.0, 1.0), v(0.6, -0.8)] {
            diff_raytocapsule(
                &format!("row45 i={i} mid d={d:?}"),
                c2Ray { p: mid, d, t: 10.0 },
                cap,
            );
        }
        // a point offset along the segment but within r of it
        let f = g.below(100) as f32 / 100.0;
        let on = v(
            cap.a.x + (cap.b.x - cap.a.x) * f,
            cap.a.y + (cap.b.y - cap.a.y) * f,
        );
        diff_raytocapsule(
            &format!("row45 i={i} on-seg"),
            c2Ray {
                p: on,
                d: v(1.0, 0.0),
                t: 10.0,
            },
            cap,
        );
    }
}

#[test]
fn cfg46_raytocapsule_origin_in_end_caps() {
    let mut g = Rng::new(SEED ^ 46);
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let ux = (cap.b.x - cap.a.x) / ((cap.b.x - cap.a.x).hypot(cap.b.y - cap.a.y));
        let uy = (cap.b.y - cap.a.y) / ((cap.b.x - cap.a.x).hypot(cap.b.y - cap.a.y));
        // just beyond cap a along -u, but still inside the circle of radius r
        let f = cap.r * 0.5;
        let in_a = v(cap.a.x - ux * f, cap.a.y - uy * f);
        let in_b = v(cap.b.x + ux * f, cap.b.y + uy * f);
        for (k, o) in [in_a, in_b].iter().enumerate() {
            for d in [v(1.0, 0.0), v(0.0, -1.0), v(ux, uy)] {
                diff_raytocapsule(
                    &format!("row46 i={i} cap={k} d={d:?}"),
                    c2Ray { p: *o, d, t: 10.0 },
                    cap,
                );
            }
        }
        // exactly at the end points
        for (k, o) in [cap.a, cap.b].iter().enumerate() {
            diff_raytocapsule(
                &format!("row46 i={i} endpoint={k}"),
                c2Ray {
                    p: *o,
                    d: v(1.0, 0.0),
                    t: 10.0,
                },
                cap,
            );
        }
    }
}

#[test]
fn cfg47_48_raytocapsule_side_wall_both_sides() {
    let mut g = Rng::new(SEED ^ 47);
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let len = (cap.b.x - cap.a.x).hypot(cap.b.y - cap.a.y);
        let (ux, uy) = ((cap.b.x - cap.a.x) / len, (cap.b.y - cap.a.y) / len);
        let (px, py) = (-uy, ux); // perpendicular
        // aim at the middle of the segment from both perpendicular sides
        let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        for (k, s) in [1.0f32, -1.0].iter().enumerate() {
            let dist = cap.r + 1.0 + g.uniform(20.0).abs();
            let o = v(mid.x + px * s * dist, mid.y + py * s * dist);
            for scale in [1.0f32, 2.0] {
                diff_raytocapsule(
                    &format!("row47/48 i={i} side={k} scale={scale}"),
                    ray_towards(&mut g, o, mid, scale),
                    cap,
                );
            }
            // slightly off-centre along the segment (still a side-wall hit)
            let f = 0.25 + (g.below(50) as f32) / 100.0;
            let tgt = v(cap.a.x + ux * len * f, cap.a.y + uy * len * f);
            diff_raytocapsule(
                &format!("row47/48 i={i} side={k} off-centre"),
                ray_towards(&mut g, o, tgt, 2.0),
                cap,
            );
        }
    }
}

#[test]
fn cfg49_50_raytocapsule_cap_delegation() {
    let mut g = Rng::new(SEED ^ 49);
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let len = (cap.b.x - cap.a.x).hypot(cap.b.y - cap.a.y);
        let (ux, uy) = ((cap.b.x - cap.a.x) / len, (cap.b.y - cap.a.y) / len);
        let (px, py) = (-uy, ux);
        // Aim just past each end cap: y <= 0 -> Ca, y >= yBb.y -> Cb
        for (k, (ex, ey, s)) in [
            (cap.a.x, cap.a.y, -1.0f32),
            (cap.b.x, cap.b.y, 1.0f32),
        ]
        .iter()
        .enumerate()
        {
            let tgt = v(ex + ux * s * cap.r * 0.5, ey + uy * s * cap.r * 0.5);
            let dist = cap.r + 2.0 + g.uniform(20.0).abs();
            for side in [1.0f32, -1.0] {
                let o = v(tgt.x + px * side * dist, tgt.y + py * side * dist);
                // hitting the cap (long enough ray)
                diff_raytocapsule(
                    &format!("row49/50 i={i} end={k} side={side} hit"),
                    ray_towards(&mut g, o, tgt, 2.0),
                    cap,
                );
                // delegated c2RaytoCircle returns 0 (ray far too short)
                let mut short = ray_towards(&mut g, o, tgt, 1.0);
                short.t *= 0.05;
                diff_raytocapsule(
                    &format!("row49/50 i={i} end={k} side={side} short"),
                    short,
                    cap,
                );
            }
        }
        // |yAp.x| < B.r branch: origin laterally within r but axially outside
        for (k, s) in [-1.0f32, 1.0].iter().enumerate() {
            let base = if *s < 0.0 { cap.a } else { cap.b };
            let o = v(
                base.x + ux * s * (cap.r + 2.0) + px * cap.r * 0.5,
                base.y + uy * s * (cap.r + 2.0) + py * cap.r * 0.5,
            );
            diff_raytocapsule(
                &format!("row50 i={i} lateral={k}"),
                c2Ray {
                    p: o,
                    d: v(-ux * s, -uy * s),
                    t: len + 4.0 * cap.r + 8.0,
                },
                cap,
            );
        }
    }
}

#[test]
fn cfg51_raytocapsule_miss_still_writes_out() {
    let mut g = Rng::new(SEED ^ 51);
    let l = libs();
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let len = (cap.b.x - cap.a.x).hypot(cap.b.y - cap.a.y);
        let (ux, uy) = ((cap.b.x - cap.a.x) / len, (cap.b.y - cap.a.y) / len);
        let (px, py) = (-uy, ux);
        // ray running parallel to the capsule, far off to one side
        let off = cap.r * 20.0 + 50.0;
        let o = v(cap.a.x + px * off, cap.a.y + py * off);
        let ray = c2Ray {
            p: o,
            d: v(ux, uy),
            t: len,
        };
        diff_raytocapsule(&format!("row51 i={i}"), ray, cap);
        // and confirm the C library really did overwrite `out` on that miss
        let mut co = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
        if rc == 0 {
            assert_ne!(
                co.bits(),
                SENTINEL.bits(),
                "C c2RaytoCapsule is expected to write *out even on a miss"
            );
        }
    }
}

#[test]
fn cfg52_raytocapsule_orientations() {
    let mut g = Rng::new(SEED ^ 52);
    for i in 0..n() {
        let a = g.coord_v();
        let len = 1.0 + g.uniform(20.0).abs();
        let r = 0.25 + g.uniform(4.0).abs();
        let caps = [
            c2Capsule { a, b: v(a.x + len, a.y), r },              // horizontal
            c2Capsule { a, b: v(a.x - len, a.y), r },              // reversed
            c2Capsule { a, b: v(a.x, a.y + len), r },              // vertical up
            c2Capsule { a, b: v(a.x, a.y - len), r },              // "b below a"
            c2Capsule { a: v(0.0, 0.0), b: v(0.0, 1.0), r: 1.0 },  // unit
        ];
        for (k, cap) in caps.iter().enumerate() {
            diff_raytocapsule(
                &format!("row52 i={i} cap={k} norm"),
                rand_ray_normalized(&mut g),
                *cap,
            );
            diff_raytocapsule(&format!("row52 i={i} cap={k} raw"), rand_ray_raw(&mut g), *cap);
            // aimed at the segment midpoint
            let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            let o = v(mid.x + 30.0, mid.y + 17.0);
            diff_raytocapsule(
                &format!("row52 i={i} cap={k} aimed"),
                ray_towards(&mut g, o, mid, 2.0),
                *cap,
            );
        }
    }
}

#[test]
fn cfg53_raytocapsule_degenerate() {
    let mut g = Rng::new(SEED ^ 53);
    for i in 0..n() {
        let p = g.coord_v();
        // a == b  => c2Norm((0,0)) => NaN frame
        let degen = c2Capsule { a: p, b: p, r: g.radius() };
        diff_raytocapsule(&format!("row53 i={i} a==b"), rand_ray_normalized(&mut g), degen);
        diff_raytocapsule(&format!("row53 i={i} a==b raw"), rand_ray_raw(&mut g), degen);

        let base = nice_capsule(&mut g);
        for r in [
            0.0f32,
            -0.0,
            -1.0,
            -5.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MIN_POSITIVE,
            1e30,
        ] {
            let cap = c2Capsule { r, ..base };
            diff_raytocapsule(
                &format!("row53 i={i} r={r:e}"),
                rand_ray_normalized(&mut g),
                cap,
            );
            let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            diff_raytocapsule(
                &format!("row53 i={i} r={r:e} aimed"),
                ray_towards(&mut g, v(mid.x + 25.0, mid.y - 11.0), mid, 2.0),
                cap,
            );
        }
        // yAe.x == yAp.x  =>  d == 0  =>  division by zero in the `t` formula.
        // A ray parallel to the capsule axis has a constant local x.
        let len = (base.b.x - base.a.x).hypot(base.b.y - base.a.y);
        let (ux, uy) = ((base.b.x - base.a.x) / len, (base.b.y - base.a.y) / len);
        let (px, py) = (-uy, ux);
        for off in [base.r * 2.0, base.r * 10.0, base.r + 1.0] {
            diff_raytocapsule(
                &format!("row53 i={i} parallel off={off}"),
                c2Ray {
                    p: v(base.a.x + px * off - ux * 5.0, base.a.y + py * off - uy * 5.0),
                    d: v(ux, uy),
                    t: len + 10.0,
                },
                base,
            );
        }
        // fully wild
        diff_raytocapsule(
            &format!("row53 i={i} wild"),
            wild_ray(&mut g),
            c2Capsule {
                a: g.wild_v(),
                b: g.wild_v(),
                r: g.wild(),
            },
        );
    }
}

/* ==================== rows 54..58 : c2CastRay ============================ */

#[test]
fn cfg54_castray_circle() {
    let mut g = Rng::new(SEED ^ 54);
    for i in 0..n() {
        diff_castray_circle(
            &format!("row54 i={i} norm"),
            rand_ray_normalized(&mut g),
            rand_circle(&mut g),
        );
        diff_castray_circle(
            &format!("row54 i={i} raw"),
            rand_ray_raw(&mut g),
            rand_circle(&mut g),
        );
    }
}

#[test]
fn cfg55_castray_aabb() {
    let mut g = Rng::new(SEED ^ 55);
    for i in 0..n() {
        diff_castray_aabb(
            &format!("row55 i={i} norm"),
            rand_ray_normalized(&mut g),
            rand_box(&mut g),
        );
        diff_castray_aabb(
            &format!("row55 i={i} raw"),
            rand_ray_raw(&mut g),
            rand_box(&mut g),
        );
    }
}

#[test]
fn cfg56_castray_capsule() {
    let mut g = Rng::new(SEED ^ 56);
    for i in 0..n() {
        diff_castray_capsule(
            &format!("row56 i={i} nice"),
            rand_ray_normalized(&mut g),
            nice_capsule(&mut g),
        );
        diff_castray_capsule(
            &format!("row56 i={i} raw"),
            rand_ray_raw(&mut g),
            rand_capsule(&mut g),
        );
    }
}

#[test]
fn cfg57_castray_miss_paths_leave_out_untouched() {
    let mut g = Rng::new(SEED ^ 57);
    let l = libs();
    for i in 0..n() {
        // A ray that certainly misses everything placed far away.
        let far = c2Ray {
            p: v(1e6, 1e6),
            d: v(1.0, 0.0),
            t: 1.0,
        };
        let circle = c2Circle {
            p: g.coord_v(),
            r: 1.0,
        };
        diff_castray_circle(&format!("row57 i={i} circle"), far, circle);
        let bb = rand_box(&mut g);
        diff_castray_aabb(&format!("row57 i={i} aabb"), far, bb);

        // and check the sentinel really survived in the C library
        let mut co = SENTINEL;
        let rc = unsafe {
            (l.c.c2CastRay)(
                far,
                &circle as *const c2Circle as *const std::os::raw::c_void,
                C2_TYPE_CIRCLE,
                &mut co,
            )
        };
        assert_eq!(rc, 0, "the far-away ray should miss");
        assert_eq!(
            co.bits(),
            SENTINEL.bits(),
            "C c2CastRay/CIRCLE must not touch *out on a miss"
        );
    }
}

#[test]
fn cfg58_castray_abi_stress_all_types() {
    let mut g = Rng::new(SEED ^ 58);
    for i in 0..n() {
        match g.below(3) {
            0 => diff_castray_circle(
                &format!("row58 i={i}"),
                wild_ray(&mut g),
                c2Circle {
                    p: g.wild_v(),
                    r: g.wild(),
                },
            ),
            1 => diff_castray_aabb(
                &format!("row58 i={i}"),
                wild_ray(&mut g),
                c2AABB {
                    min: g.wild_v(),
                    max: g.wild_v(),
                },
            ),
            _ => diff_castray_capsule(
                &format!("row58 i={i}"),
                wild_ray(&mut g),
                c2Capsule {
                    a: g.wild_v(),
                    b: g.wild_v(),
                    r: g.wild(),
                },
            ),
        }
    }
}

/* ==================== rows 59..68 : spec_ray ============================= */

#[test]
fn cfg59_spec_ray_property_sweep() {
    let mut g = Rng::new(SEED ^ 59);
    for i in 0..n() * 4 {
        let a = [
            g.coord(),
            g.coord(),
            g.coord(),
            g.coord(),
            g.radius(),
            g.coord(),
            g.coord(),
        ];
        diff_spec_ray(&format!("row59 i={i}"), a);
    }
}

#[test]
fn cfg60_spec_ray_guaranteed_hit() {
    let mut g = Rng::new(SEED ^ 60);
    for i in 0..n() {
        let (cx, cy) = (g.coord(), g.coord());
        let r = 1.0 + g.uniform(10.0).abs();
        // ray origin outside, mouse point at the centre
        let dist = r + 1.0 + g.uniform(30.0).abs();
        let ang = g.uniform(3.14159265);
        let (rx, ry) = (cx + dist * ang.cos(), cy + dist * ang.sin());
        diff_spec_ray(&format!("row60 i={i}"), [cx, cy, cx, cy, r, rx, ry]);
        // mouse point beyond the circle (still on the same line)
        let (mx, my) = (cx + dist * ang.cos() * -1.0, cy + dist * ang.sin() * -1.0);
        diff_spec_ray(&format!("row60 i={i} beyond"), [mx, my, cx, cy, r, rx, ry]);
    }
}

#[test]
fn cfg61_spec_ray_guaranteed_miss() {
    let mut g = Rng::new(SEED ^ 61);
    for i in 0..n() {
        let (cx, cy) = (g.coord(), g.coord());
        let r = 0.5 + g.uniform(5.0).abs();
        let dist = r + 5.0 + g.uniform(30.0).abs();
        let ang = g.uniform(3.14159265);
        let (rx, ry) = (cx + dist * ang.cos(), cy + dist * ang.sin());
        // (a) mouse point far to the side => ray direction misses the circle
        let perp = ang + 1.5707963;
        let (mx, my) = (rx + 1000.0 * perp.cos(), ry + 1000.0 * perp.sin());
        diff_spec_ray(&format!("row61 i={i} aside"), [mx, my, cx, cy, r, rx, ry]);
        // (b) mouse point on the far side of the origin => circle behind
        let (mx, my) = (
            rx + (rx - cx) * 2.0,
            ry + (ry - cy) * 2.0,
        );
        diff_spec_ray(&format!("row61 i={i} behind"), [mx, my, cx, cy, r, rx, ry]);
    }
}

#[test]
fn cfg62_spec_ray_origin_inside_circle() {
    let mut g = Rng::new(SEED ^ 62);
    for i in 0..n() {
        let (cx, cy) = (g.coord(), g.coord());
        let r = 1.0 + g.uniform(10.0).abs();
        let f = 0.9 * (g.below(100) as f32) / 100.0;
        let ang = g.uniform(3.14159265);
        let (rx, ry) = (cx + r * f * ang.cos(), cy + r * f * ang.sin());
        let (mx, my) = (g.coord(), g.coord());
        diff_spec_ray(&format!("row62 i={i}"), [mx, my, cx, cy, r, rx, ry]);
        // origin exactly at the centre
        diff_spec_ray(&format!("row62 i={i} centre"), [mx, my, cx, cy, r, cx, cy]);
    }
}

#[test]
fn cfg63_spec_ray_impact_exactly_at_ray_t() {
    let mut g = Rng::new(SEED ^ 63);
    for i in 0..n() {
        let cx = (g.below(41) as f32) - 20.0;
        let cy = (g.below(41) as f32) - 20.0;
        let r = (g.below(6) + 1) as f32;
        let dist = r + (g.below(20) + 1) as f32;
        // axis-aligned: origin left of the circle, mouse point on the near rim
        diff_spec_ray(
            &format!("row63 i={i} near-rim"),
            [cx - r, cy, cx, cy, r, cx - dist, cy],
        );
        // mouse point on the far rim
        diff_spec_ray(
            &format!("row63 i={i} far-rim"),
            [cx + r, cy, cx, cy, r, cx - dist, cy],
        );
        // mouse point exactly at the centre
        diff_spec_ray(
            &format!("row63 i={i} centre"),
            [cx, cy, cx, cy, r, cx - dist, cy],
        );
    }
}

#[test]
fn cfg64_spec_ray_mp_equals_origin() {
    let mut g = Rng::new(SEED ^ 64);
    for i in 0..n() {
        let (px, py) = (g.coord(), g.coord());
        let (cx, cy) = (g.coord(), g.coord());
        let r = g.radius();
        diff_spec_ray(&format!("row64 i={i}"), [px, py, cx, cy, r, px, py]);
        // and with the circle centred on that same point
        diff_spec_ray(&format!("row64 i={i} same"), [px, py, px, py, r, px, py]);
        // signed-zero variants of "equal"
        diff_spec_ray(&format!("row64 i={i} zeros"), [0.0, 0.0, cx, cy, r, -0.0, -0.0]);
    }
}

#[test]
fn cfg65_spec_ray_degenerate_radii() {
    let mut g = Rng::new(SEED ^ 65);
    for i in 0..n() {
        let (cx, cy) = (g.coord(), g.coord());
        let (rx, ry) = (cx - 10.0, cy);
        for r in [
            0.0f32,
            -0.0,
            -1.0,
            -7.5,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MIN_POSITIVE,
            1e-30,
            1e30,
            f32::MAX,
        ] {
            diff_spec_ray(&format!("row65 i={i} r={r:e} aimed"), [cx, cy, cx, cy, r, rx, ry]);
            diff_spec_ray(
                &format!("row65 i={i} r={r:e} rand"),
                [g.coord(), g.coord(), cx, cy, r, rx, ry],
            );
        }
    }
}

#[test]
fn cfg66_spec_ray_wild_arguments() {
    let mut g = Rng::new(SEED ^ 66);
    for i in 0..n() * 8 {
        let a = [
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
        ];
        diff_spec_ray(&format!("row66 i={i}"), a);
    }
    // one argument wild at a time, the rest a sane hit configuration
    for slot in 0..7 {
        for _ in 0..3000 {
            let mut a = [5.0f32, 5.0, 5.0, 5.0, 2.0, -10.0, 5.0];
            a[slot] = g.wild();
            diff_spec_ray(&format!("row66 slot={slot}"), a);
        }
    }
}

#[test]
fn cfg67_spec_ray_pixel_grid() {
    let mut g = Rng::new(SEED ^ 67);
    for i in 0..n() * 2 {
        let a = [
            g.below(1920) as f32,
            g.below(1080) as f32,
            g.below(1920) as f32,
            g.below(1080) as f32,
            g.below(200) as f32,
            g.below(1920) as f32,
            g.below(1080) as f32,
        ];
        diff_spec_ray(&format!("row67 i={i}"), a);
    }
}

#[test]
fn cfg68_spec_ray_equals_manual_pipeline() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 68);
    for i in 0..n() * 2 {
        let a = [
            g.coord(),
            g.coord(),
            g.coord(),
            g.coord(),
            g.radius(),
            g.coord(),
            g.coord(),
        ];
        // Recompose spec_ray out of the low-level exports of the *Rust* .so and
        // compare against the C .so's one-shot spec_ray.
        let mp = unsafe { (l.r.c2V)(a[0], a[1]) };
        let circle = c2Circle {
            p: unsafe { (l.r.c2V)(a[2], a[3]) },
            r: a[4],
        };
        let rp = unsafe { (l.r.c2V)(a[5], a[6]) };
        let d = unsafe { (l.r.c2Norm)((l.r.c2Sub)(mp, rp)) };
        let t = {
            // ray.t = c2Dot(mp, d) - c2Dot(rp, d)
            let d1 = unsafe { (l.r.c2Dot)(mp, d) };
            let d2 = unsafe { (l.r.c2Dot)(rp, d) };
            d1 - d2
        };
        let ray = c2Ray { p: rp, d, t };
        let mut manual = SENTINEL;
        let mr = unsafe {
            (l.r.c2CastRay)(
                ray,
                &circle as *const c2Circle as *const std::os::raw::c_void,
                C2_TYPE_CIRCLE,
                &mut manual,
            )
        };
        let mut c_out = SENTINEL;
        let cr =
            unsafe { (l.c.spec_ray)(&mut c_out, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        same(
            &format!("row68 i={i} args={a:?} (manual Rust pipeline vs C spec_ray)"),
            (cr, c_out),
            (mr, manual),
        );
        // ... and the plain differential check as well
        diff_spec_ray(&format!("row68 i={i}"), a);
    }
}
