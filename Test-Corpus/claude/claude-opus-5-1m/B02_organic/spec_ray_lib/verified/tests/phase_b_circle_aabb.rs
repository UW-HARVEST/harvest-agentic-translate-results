//! Phase B — valid-path differential tests, rows 18..35 of `CONFIGS.md`
//! (`c2RaytoCircle` and `c2RaytoAABB`, driven directly through the `.so`
//! exports, not through the `spec_ray` convenience wrapper).

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 2000;

fn perp(u: C2v) -> C2v {
    v(-u.y, u.x)
}

/* ======================= c2RaytoCircle, rows 18..24 ======================= */

/// Ray aimed at a circle from the outside, hit within `A.t`.
fn gen_circle_hit(rng: &mut Rng) -> (C2Ray, C2Circle) {
    let center = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
    let r = rng.range(0.05, 15.0);
    let u = rng.dir();
    let s = rng.range(-0.94, 0.94) * r; // lateral offset, inside the disc
    let dist = rng.range(r + 0.01, r + 60.0);
    let q = perp(u);
    let origin = vadd(vadd(center, vscale(q, s)), vscale(u, -dist));
    (
        C2Ray {
            p: origin,
            d: u,
            t: dist + 2.0 * r + rng.range(0.0, 10.0),
        },
        C2Circle { p: center, r },
    )
}

#[test]
fn row18_raytocircle_hit() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x18_0001);
    let mut d = Diff::new("18: c2RaytoCircle hit (0 <= t <= A.t)");
    for _ in 0..N {
        let (ray, circ) = gen_circle_hit(&mut rng);
        d.check_call(
            || format!("{} {}", rayshow(&ray), circshow(&circ)),
            call_raytocircle(c, ray, circ),
            call_raytocircle(r, ray, circ),
        );
    }
    d.require_hits(N * 9 / 10);
    d.finish();
}

#[test]
fn row19_raytocircle_line_miss_disc_negative() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x19_0001);
    let mut d = Diff::new("19: c2RaytoCircle disc < 0");
    for _ in 0..N {
        let center = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let rad = rng.range(0.05, 15.0);
        let u = rng.dir();
        let s = rng.range(1.06, 5.0) * rad * if rng.chance(2) { 1.0 } else { -1.0 };
        let dist = rng.range(0.1, 60.0);
        let origin = vadd(vadd(center, vscale(perp(u), s)), vscale(u, -dist));
        let ray = C2Ray {
            p: origin,
            d: u,
            t: dist + 4.0 * rad,
        };
        let circ = C2Circle { p: center, r: rad };
        d.check_call(
            || format!("{} {}", rayshow(&ray), circshow(&circ)),
            call_raytocircle(c, ray, circ),
            call_raytocircle(r, ray, circ),
        );
    }
    d.require_misses(N * 9 / 10);
    d.finish();
}

#[test]
fn row20_raytocircle_tangent_disc_zero() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x20_0001);
    let mut d = Diff::new("20: c2RaytoCircle tangent (disc == 0 +- 1ulp)");
    for _ in 0..N {
        let center = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let rad = rng.range(0.05, 15.0);
        let u = rng.dir();
        let dist = rng.range(0.1, 60.0);
        for s in [rad, -rad, next_up(rad), next_down(rad)] {
            let origin = vadd(vadd(center, vscale(perp(u), s)), vscale(u, -dist));
            let ray = C2Ray {
                p: origin,
                d: u,
                t: dist + 4.0 * rad,
            };
            let circ = C2Circle { p: center, r: rad };
            d.check_call(
                || format!("{} {}", rayshow(&ray), circshow(&circ)),
                call_raytocircle(c, ray, circ),
                call_raytocircle(r, ray, circ),
            );
        }
    }
    d.finish();
}

#[test]
fn row21_raytocircle_origin_inside_or_behind() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x21_0001);
    let mut d = Diff::new("21: c2RaytoCircle t < 0 (inside / behind)");
    for i in 0..N {
        let center = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let rad = rng.range(0.05, 15.0);
        let u = rng.dir();
        let ray = if i % 2 == 0 {
            // origin strictly inside the circle
            let off = rng.range(0.0, 0.98) * rad;
            C2Ray {
                p: vadd(center, vscale(rng.dir(), off)),
                d: u,
                t: rng.range(0.0, 50.0),
            }
        } else {
            // circle entirely behind the ray origin
            let dist = rad + rng.range(0.01, 40.0);
            C2Ray {
                p: vadd(center, vscale(u, dist)),
                d: u,
                t: rng.range(0.0, 50.0),
            }
        };
        let circ = C2Circle { p: center, r: rad };
        d.check_call(
            || format!("{} {}", rayshow(&ray), circshow(&circ)),
            call_raytocircle(c, ray, circ),
            call_raytocircle(r, ray, circ),
        );
    }
    d.require_misses(N * 9 / 10);
    d.finish();
}

#[test]
fn row22_raytocircle_t_vs_A_t_boundary() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x22_0001);
    let mut d = Diff::new("22: c2RaytoCircle t == A.t boundary");
    for _ in 0..N {
        let (ray, circ) = gen_circle_hit(&mut rng);
        // First find the exact hit distance with a generous A.t ...
        let (rc, out) = call_raytocircle(c, ray, circ);
        if rc != 1 {
            continue;
        }
        // ... then probe A.t exactly at, just below and just above it.
        for at in [out.t, next_down(out.t), next_up(out.t), out.t * 0.5] {
            let probe = C2Ray { t: at, ..ray };
            d.check_call(
                || format!("{} {}", rayshow(&probe), circshow(&circ)),
                call_raytocircle(c, probe, circ),
                call_raytocircle(r, probe, circ),
            );
        }
    }
    d.require_hits(100);
    d.require_misses(100);
    d.finish();
}

#[test]
fn row23_raytocircle_direction_and_length_shapes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x23_0001);
    let mut d = Diff::new("23: c2RaytoCircle non-unit dir, A.t = 0/inf/negative");
    for i in 0..N {
        let (base, circ) = gen_circle_hit(&mut rng);
        let ray = match i % 8 {
            0 => C2Ray {
                d: vscale(base.d, rng.range(0.01, 100.0)),
                ..base
            },
            1 => C2Ray {
                d: vscale(base.d, -1.0),
                ..base
            },
            2 => C2Ray { t: 0.0, ..base },
            3 => C2Ray { t: -0.0, ..base },
            4 => C2Ray {
                t: f32::INFINITY,
                ..base
            },
            5 => C2Ray {
                t: -rng.range(0.0, 50.0),
                ..base
            },
            6 => C2Ray {
                d: v(0.0, 0.0),
                ..base
            },
            _ => C2Ray {
                d: vscale(base.d, rng.range(1e-30, 1e-20)),
                t: rng.range(1e20, 1e30),
                ..base
            },
        };
        d.check_call(
            || format!("{} {}", rayshow(&ray), circshow(&circ)),
            call_raytocircle(c, ray, circ),
            call_raytocircle(r, ray, circ),
        );
    }
    d.finish();
}

#[test]
fn row24_raytocircle_radius_and_nan_shapes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x24_0001);
    let mut d = Diff::new("24: c2RaytoCircle r = 0/negative/huge/NaN fields");
    for i in 0..N {
        let (base, circ) = gen_circle_hit(&mut rng);
        let (ray, circ) = match i % 8 {
            0 => (base, C2Circle { r: 0.0, ..circ }),
            1 => (base, C2Circle { r: -circ.r, ..circ }),
            2 => (
                base,
                C2Circle {
                    r: rng.range(1e30, 3e38),
                    ..circ
                },
            ),
            3 => (
                base,
                C2Circle {
                    r: f32::from_bits(rng.below(0x0080_0000)),
                    ..circ
                },
            ),
            4 => (
                base,
                C2Circle {
                    r: rng.special(),
                    ..circ
                },
            ),
            5 => (
                C2Ray {
                    p: rng.v_special(),
                    ..base
                },
                circ,
            ),
            6 => (
                C2Ray {
                    d: rng.v_special(),
                    t: rng.special(),
                    ..base
                },
                circ,
            ),
            _ => (
                C2Ray {
                    p: rng.v_mixed(),
                    d: rng.v_mixed(),
                    t: rng.mixed(),
                },
                C2Circle {
                    p: rng.v_mixed(),
                    r: rng.mixed(),
                },
            ),
        };
        d.check_call(
            || format!("{} {}", rayshow(&ray), circshow(&circ)),
            call_raytocircle(c, ray, circ),
            call_raytocircle(r, ray, circ),
        );
    }
    d.finish();
}

/* ======================== c2RaytoAABB, rows 25..35 ======================== */

fn rand_proper_box(rng: &mut Rng) -> C2AABB {
    let x0 = rng.range(-40.0, 40.0);
    let y0 = rng.range(-40.0, 40.0);
    C2AABB {
        min: v(x0, y0),
        max: v(x0 + rng.range(0.05, 30.0), y0 + rng.range(0.05, 30.0)),
    }
}

/// Ray entering the box from `side` (0 = -x, 1 = +x, 2 = -y, 3 = +y).
fn gen_aabb_from_side(rng: &mut Rng, side: u32) -> (C2Ray, C2AABB) {
    let b = rand_proper_box(rng);
    let w = b.max.x - b.min.x;
    let h = b.max.y - b.min.y;
    let (origin, dir) = match side {
        0 => (
            v(b.min.x - rng.range(0.1, 30.0), b.min.y + rng.range(0.05, 0.95) * h),
            v(1.0, 0.0),
        ),
        1 => (
            v(b.max.x + rng.range(0.1, 30.0), b.min.y + rng.range(0.05, 0.95) * h),
            v(-1.0, 0.0),
        ),
        2 => (
            v(b.min.x + rng.range(0.05, 0.95) * w, b.min.y - rng.range(0.1, 30.0)),
            v(0.0, 1.0),
        ),
        _ => (
            v(b.min.x + rng.range(0.05, 0.95) * w, b.max.y + rng.range(0.1, 30.0)),
            v(0.0, -1.0),
        ),
    };
    // slight tilt so the ray is not perfectly axis parallel half of the time
    let dir = if rng.chance(2) {
        vnorm(vadd(dir, vscale(perp(dir), rng.range(-0.2, 0.2))))
    } else {
        dir
    };
    let t = 30.0 + w + h + rng.range(0.0, 30.0);
    (C2Ray { p: origin, d: dir, t }, b)
}

fn aabb_row(row: &str, seed: u64, side: u32, want: AabbBranch) {
    let (c, r) = apis();
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N {
        let (ray, b) = gen_aabb_from_side(&mut rng, side);
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_tag(want.name(), N / 3);
    d.finish();
}

#[test]
fn row25_raytoaabb_face_neg_x() {
    aabb_row("25: c2RaytoAABB -x face", 0x25_0001, 0, AabbBranch::FaceNegX);
}

#[test]
fn row26_raytoaabb_face_pos_x() {
    aabb_row("26: c2RaytoAABB +x face", 0x26_0001, 1, AabbBranch::FacePosX);
}

#[test]
fn row27_raytoaabb_face_neg_y() {
    aabb_row("27: c2RaytoAABB -y face", 0x27_0001, 2, AabbBranch::FaceNegY);
}

#[test]
fn row28_raytoaabb_face_pos_y() {
    aabb_row("28: c2RaytoAABB +y face", 0x28_0001, 3, AabbBranch::FacePosY);
}

#[test]
fn row29_raytoaabb_ties_and_corners() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x29_0001);
    let mut d = Diff::new("29: c2RaytoAABB t0==t1==t2==t3 ties / corner hits");
    for i in 0..N {
        let b = rand_proper_box(&mut rng);
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        let ray = match i % 4 {
            // origin inside the box: every plane test yields t = 0
            0 => C2Ray {
                p: v(
                    b.min.x + rng.range(0.05, 0.95) * w,
                    b.min.y + rng.range(0.05, 0.95) * h,
                ),
                d: rng.dir(),
                t: rng.range(0.0, 50.0),
            },
            // exact diagonal through the min corner
            1 => C2Ray {
                p: v(b.min.x - w, b.min.y - h),
                d: vnorm(v(w, h)),
                t: (w * w + h * h).sqrt() * 2.0,
            },
            // exact diagonal through the max corner
            2 => C2Ray {
                p: v(b.max.x + w, b.max.y + h),
                d: vnorm(v(-w, -h)),
                t: (w * w + h * h).sqrt() * 2.0,
            },
            // starting exactly on a corner
            _ => C2Ray {
                p: b.min,
                d: rng.dir(),
                t: rng.range(0.0, 50.0),
            },
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_tag("face_-x", 100);
    d.finish();
}

#[test]
fn row30_raytoaabb_bbox_reject() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x30_0001);
    let mut d = Diff::new("30: c2RaytoAABB swept-bbox reject");
    for _ in 0..N {
        let b = rand_proper_box(&mut rng);
        // ray far away on one side, pointing away from the box
        let away = match rng.below(4) {
            0 => v(-1.0, 0.0),
            1 => v(1.0, 0.0),
            2 => v(0.0, -1.0),
            _ => v(0.0, 1.0),
        };
        let center = v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);
        let origin = vadd(center, vscale(away, 100.0 + rng.range(0.0, 100.0)));
        let ray = C2Ray {
            p: origin,
            d: away,
            t: rng.range(0.0, 50.0),
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_tag("bbox_reject", N * 9 / 10);
    d.finish();
}

#[test]
fn row31_raytoaabb_separating_axis_reject() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x31_0001);
    let mut d = Diff::new("31: c2RaytoAABB separating-axis reject (d > 0)");
    for _ in 0..N {
        let b = rand_proper_box(&mut rng);
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        // pick a corner and slide just outside along the outward diagonal
        let (corner, outward) = match rng.below(4) {
            0 => (b.min, vnorm(v(-1.0, -1.0))),
            1 => (b.max, vnorm(v(1.0, 1.0))),
            2 => (v(b.min.x, b.max.y), vnorm(v(-1.0, 1.0))),
            _ => (v(b.max.x, b.min.y), vnorm(v(1.0, -1.0))),
        };
        let eps = rng.range(0.001, 0.5) * (w + h) * 0.1;
        let q = vadd(corner, vscale(outward, eps));
        let dir = perp(outward);
        let l = (w + h) * rng.range(0.5, 2.0);
        let ray = C2Ray {
            p: vadd(q, vscale(dir, -l)),
            d: dir,
            t: 2.0 * l,
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_tag("sep_axis_reject", N / 2);
    d.finish();
}

#[test]
fn row32_raytoaabb_axis_parallel_zero_denominator() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x32_0001);
    let mut d = Diff::new("32: c2RaytoAABB axis-parallel (da - db == 0)");
    for i in 0..N {
        let b = rand_proper_box(&mut rng);
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        let dir = match i % 4 {
            0 => v(1.0, 0.0),
            1 => v(-1.0, 0.0),
            2 => v(0.0, 1.0),
            _ => v(0.0, -1.0),
        };
        // start outside, aligned with one axis so the other axis has da == db
        let origin = match i % 4 {
            0 => v(b.min.x - rng.range(0.1, 20.0), b.min.y + rng.range(-0.2, 1.2) * h),
            1 => v(b.max.x + rng.range(0.1, 20.0), b.min.y + rng.range(-0.2, 1.2) * h),
            2 => v(b.min.x + rng.range(-0.2, 1.2) * w, b.min.y - rng.range(0.1, 20.0)),
            _ => v(b.min.x + rng.range(-0.2, 1.2) * w, b.max.y + rng.range(0.1, 20.0)),
        };
        let ray = C2Ray {
            p: origin,
            d: dir,
            t: 40.0 + w + h,
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_hits(N / 4);
    d.require_misses(N / 8);
    d.finish();
}

#[test]
fn row33_raytoaabb_zero_length_sweep_and_inside() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x33_0001);
    let mut d = Diff::new("33: c2RaytoAABB A.t == 0 / origin inside / on face");
    for i in 0..N {
        let b = rand_proper_box(&mut rng);
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        let inside = v(
            b.min.x + rng.range(0.0, 1.0) * w,
            b.min.y + rng.range(0.0, 1.0) * h,
        );
        let ray = match i % 5 {
            0 => C2Ray {
                p: inside,
                d: rng.dir(),
                t: 0.0,
            },
            1 => C2Ray {
                p: inside,
                d: v(0.0, 0.0),
                t: rng.range(0.0, 20.0),
            },
            2 => C2Ray {
                p: v(b.min.x, b.min.y + rng.range(0.0, 1.0) * h),
                d: rng.dir(),
                t: rng.range(0.0, 20.0),
            },
            3 => C2Ray {
                p: v(b.min.x + rng.range(0.0, 1.0) * w, b.max.y),
                d: rng.dir(),
                t: rng.range(0.0, 20.0),
            },
            _ => C2Ray {
                p: vadd(inside, v(w * 2.0, h * 2.0)),
                d: rng.dir(),
                t: 0.0,
            },
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_hits(N / 4);
    d.finish();
}

#[test]
fn row34_raytoaabb_degenerate_and_inverted_boxes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x34_0001);
    let mut d = Diff::new("34: c2RaytoAABB degenerate/1-D/inverted boxes");
    for i in 0..N {
        let base = rand_proper_box(&mut rng);
        let b = match i % 4 {
            0 => C2AABB {
                min: base.min,
                max: base.min,
            },
            1 => C2AABB {
                min: base.min,
                max: v(base.min.x, base.max.y),
            },
            2 => C2AABB {
                min: base.min,
                max: v(base.max.x, base.min.y),
            },
            _ => C2AABB {
                min: base.max,
                max: base.min,
            },
        };
        let center = v((base.min.x + base.max.x) * 0.5, (base.min.y + base.max.y) * 0.5);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -rng.range(1.0, 60.0))),
            d: u,
            t: rng.range(0.0, 120.0),
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        let br = classify_aabb(c, ray, b, rc_c, &out_c);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), aabbshow(&b), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.finish();
}

#[test]
fn row35_raytoaabb_extreme_values() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x35_0001);
    let mut d = Diff::new("35: c2RaytoAABB huge/inf/NaN coordinates");
    for i in 0..N {
        let (ray, b) = match i % 6 {
            0 => {
                let side = rng.below(4);
                let (ray, b) = gen_aabb_from_side(&mut rng, side);
                (
                    C2Ray {
                        t: f32::INFINITY,
                        ..ray
                    },
                    b,
                )
            }
            1 => {
                let side = rng.below(4);
                let (ray, b) = gen_aabb_from_side(&mut rng, side);
                (
                    C2Ray {
                        d: vscale(ray.d, 1e30),
                        t: 1e30,
                        ..ray
                    },
                    b,
                )
            }
            2 => (
                C2Ray {
                    p: rng.v_special(),
                    d: rng.v_special(),
                    t: rng.special(),
                },
                C2AABB {
                    min: rng.v_special(),
                    max: rng.v_special(),
                },
            ),
            3 => (
                C2Ray {
                    p: rng.v_mixed(),
                    d: rng.v_mixed(),
                    t: rng.mixed(),
                },
                C2AABB {
                    min: rng.v_mixed(),
                    max: rng.v_mixed(),
                },
            ),
            4 => {
                let side = rng.below(4);
                let (ray, b) = gen_aabb_from_side(&mut rng, side);
                (
                    ray,
                    C2AABB {
                        min: v(b.min.x, f32::NAN),
                        max: b.max,
                    },
                )
            }
            _ => (
                C2Ray {
                    p: v(rng.range(-3e38, 3e38), rng.range(-3e38, 3e38)),
                    d: rng.dir(),
                    t: rng.range(0.0, 3e38),
                },
                C2AABB {
                    min: v(-3e38, -3e38),
                    max: v(3e38, 3e38),
                },
            ),
        };
        let (rc_c, out_c) = call_raytoaabb(c, ray, b);
        let (rc_r, out_r) = call_raytoaabb(r, ray, b);
        d.check_call(
            || format!("{} {}", rayshow(&ray), aabbshow(&b)),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.finish();
}
