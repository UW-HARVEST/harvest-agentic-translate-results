//! Level 3: the raycast routines, which write through an out-parameter.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Input generators
// ---------------------------------------------------------------------------

/// Fully unconstrained ray (NaN / inf / denormal directions included).
fn wild_ray(rng: &mut Rng) -> c2Ray {
    c2Ray {
        p: rng.vec_wild(),
        d: rng.vec_wild(),
        t: rng.float(),
    }
}

/// A ray shaped the way `spec_ray` builds them: unit direction, positive t.
fn tame_ray(rng: &mut Rng) -> c2Ray {
    let ang = rng.unit() * 6.283_185_5;
    c2Ray {
        p: c2v {
            x: rng.sym(20.0),
            y: rng.sym(20.0),
        },
        d: c2v {
            x: ang.cos(),
            y: ang.sin(),
        },
        t: rng.unit() * 40.0,
    }
}

/// A ray aimed at `target`, built exactly like `spec_ray` does (normalised
/// difference, `t` from the dot-product projection).
fn aimed_ray(rng: &mut Rng, target: c2v) -> c2Ray {
    let p = c2v {
        x: rng.sym(30.0),
        y: rng.sym(30.0),
    };
    let dx = target.x - p.x;
    let dy = target.y - p.y;
    let len = (dx * dx + dy * dy).sqrt();
    let inv = 1.0f32 / len;
    let d = c2v {
        x: dx * inv,
        y: dy * inv,
    };
    let t = (target.x * d.x + target.y * d.y) - (p.x * d.x + p.y * d.y);
    c2Ray { p, d, t }
}

fn wild_circle(rng: &mut Rng) -> c2Circle {
    c2Circle {
        p: rng.vec_wild(),
        r: rng.float(),
    }
}

fn tame_circle(rng: &mut Rng) -> c2Circle {
    c2Circle {
        p: c2v {
            x: rng.sym(20.0),
            y: rng.sym(20.0),
        },
        r: rng.unit() * 10.0,
    }
}

fn wild_aabb(rng: &mut Rng) -> c2AABB {
    c2AABB {
        min: rng.vec_wild(),
        max: rng.vec_wild(),
    }
}

fn tame_aabb(rng: &mut Rng) -> c2AABB {
    let cx = rng.sym(15.0);
    let cy = rng.sym(15.0);
    let ex = rng.unit() * 10.0;
    let ey = rng.unit() * 10.0;
    c2AABB {
        min: c2v {
            x: cx - ex,
            y: cy - ey,
        },
        max: c2v {
            x: cx + ex,
            y: cy + ey,
        },
    }
}

fn wild_capsule(rng: &mut Rng) -> c2Capsule {
    c2Capsule {
        a: rng.vec_wild(),
        b: rng.vec_wild(),
        r: rng.float(),
    }
}

fn tame_capsule(rng: &mut Rng) -> c2Capsule {
    let a = c2v {
        x: rng.sym(15.0),
        y: rng.sym(15.0),
    };
    let ang = rng.unit() * 6.283_185_5;
    let len = 0.5 + rng.unit() * 20.0;
    c2Capsule {
        a,
        b: c2v {
            x: a.x + len * ang.cos(),
            y: a.y + len * ang.sin(),
        },
        r: 0.01 + rng.unit() * 6.0,
    }
}

/// Degenerate capsule (a == b, so `c2Norm` divides by zero) -- kept because
/// the C code has no guard and the exact NaN propagation must match.
fn degenerate_capsule(rng: &mut Rng) -> c2Capsule {
    let a = c2v {
        x: rng.sym(5.0),
        y: rng.sym(5.0),
    };
    c2Capsule {
        a,
        b: a,
        r: rng.unit() * 4.0,
    }
}

// ---------------------------------------------------------------------------
// Comparison driver
// ---------------------------------------------------------------------------

/// Run one shape/ray pair through both libraries and compare the return value
/// and the *entire* out-struct (pre-filled with a sentinel so that "not
/// written" is itself compared).
fn drive<S: Copy>(
    name: &str,
    cf: &unsafe extern "C" fn(c2Ray, S, *mut c2Raycast) -> c_int,
    rf: &unsafe extern "C" fn(c2Ray, S, *mut c2Raycast) -> c_int,
    ray: c2Ray,
    shape: S,
    ctx: &str,
) -> c_int {
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { cf(ray, shape, &mut co) };
    let rr = unsafe { rf(ray, shape, &mut ro) };
    assert_i_eq(name, ctx, cr, rr);
    assert_cast_eq(name, ctx, &co, &ro);
    cr
}

fn ray_ctx(ray: &c2Ray) -> String {
    format!(
        "ray p=({:?},{:?}) d=({:?},{:?}) t={:?}",
        ray.p.x, ray.p.y, ray.d.x, ray.d.y, ray.t
    )
}

// ---------------------------------------------------------------------------
// c2RaytoCircle
// ---------------------------------------------------------------------------

#[test]
fn t_c2RaytoCircle() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnRayCircle_i>("c2RaytoCircle");
    let (cf, rf) = (&*c, &*r);

    let mut hits = 0u64;
    let mut misses = 0u64;

    let mut rng = Rng::new(0x5001);
    for _ in 0..200_000 {
        let ray = wild_ray(&mut rng);
        let s = wild_circle(&mut rng);
        let ctx = format!("{} circle p=({:?},{:?}) r={:?}", ray_ctx(&ray), s.p.x, s.p.y, s.r);
        if drive("c2RaytoCircle", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    let mut rng = Rng::new(0x5002);
    for _ in 0..300_000 {
        let ray = tame_ray(&mut rng);
        let s = tame_circle(&mut rng);
        let ctx = format!("{} circle p=({:?},{:?}) r={:?}", ray_ctx(&ray), s.p.x, s.p.y, s.r);
        if drive("c2RaytoCircle", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Rays aimed straight at the circle centre: guarantees the hit branch and
    // the `t <= A.t` boundary get exercised heavily.
    let mut rng = Rng::new(0x5003);
    for _ in 0..300_000 {
        let s = tame_circle(&mut rng);
        let ray = aimed_ray(&mut rng, s.p);
        let ctx = format!("{} circle p=({:?},{:?}) r={:?}", ray_ctx(&ray), s.p.x, s.p.y, s.r);
        if drive("c2RaytoCircle", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Grazing rays: t lands right at the tangent, so `disc` hovers around 0.
    let mut rng = Rng::new(0x5004);
    for _ in 0..200_000 {
        let s = tame_circle(&mut rng);
        let ang = rng.unit() * 6.283_185_5;
        let off = s.r * (0.999 + rng.unit() * 0.002);
        let target = c2v {
            x: s.p.x + off * ang.cos(),
            y: s.p.y + off * ang.sin(),
        };
        let ray = aimed_ray(&mut rng, target);
        let ctx = format!("{} circle p=({:?},{:?}) r={:?}", ray_ctx(&ray), s.p.x, s.p.y, s.r);
        if drive("c2RaytoCircle", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    assert!(hits > 1000, "too few hit-branch cases ({hits})");
    assert!(misses > 1000, "too few miss-branch cases ({misses})");
}

// ---------------------------------------------------------------------------
// c2RaytoAABB
// ---------------------------------------------------------------------------

#[test]
fn t_c2RaytoAABB() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnRayAABB_i>("c2RaytoAABB");
    let (cf, rf) = (&*c, &*r);

    let mut hits = 0u64;
    let mut misses = 0u64;

    let mut rng = Rng::new(0x6001);
    for _ in 0..200_000 {
        let ray = wild_ray(&mut rng);
        let s = wild_aabb(&mut rng);
        let ctx = format!(
            "{} aabb ({:?},{:?})-({:?},{:?})",
            ray_ctx(&ray), s.min.x, s.min.y, s.max.x, s.max.y
        );
        if drive("c2RaytoAABB", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    let mut rng = Rng::new(0x6002);
    for _ in 0..300_000 {
        let ray = tame_ray(&mut rng);
        let s = tame_aabb(&mut rng);
        let ctx = format!(
            "{} aabb ({:?},{:?})-({:?},{:?})",
            ray_ctx(&ray), s.min.x, s.min.y, s.max.x, s.max.y
        );
        if drive("c2RaytoAABB", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Rays aimed at the box centre and at box corners -- forces all four
    // face-normal branches plus the `hitN` ties.
    let mut rng = Rng::new(0x6003);
    for _ in 0..300_000 {
        let s = tame_aabb(&mut rng);
        let pick = rng.next_u32() % 5;
        let target = match pick {
            0 => c2v {
                x: (s.min.x + s.max.x) * 0.5,
                y: (s.min.y + s.max.y) * 0.5,
            },
            1 => s.min,
            2 => s.max,
            3 => c2v { x: s.min.x, y: s.max.y },
            _ => c2v { x: s.max.x, y: s.min.y },
        };
        let ray = aimed_ray(&mut rng, target);
        let ctx = format!(
            "{} aabb ({:?},{:?})-({:?},{:?}) pick={pick}",
            ray_ctx(&ray), s.min.x, s.min.y, s.max.x, s.max.y
        );
        if drive("c2RaytoAABB", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Axis-aligned rays: `n` becomes (0, k) or (k, 0) so `abs_n` has a zero
    // component and `c2RayToPlane_OneDimensional` hits its `d == 0` arm.
    let mut rng = Rng::new(0x6004);
    for _ in 0..300_000 {
        let s = tame_aabb(&mut rng);
        let d = match rng.next_u32() % 4 {
            0 => c2v { x: 1.0, y: 0.0 },
            1 => c2v { x: -1.0, y: 0.0 },
            2 => c2v { x: 0.0, y: 1.0 },
            _ => c2v { x: 0.0, y: -1.0 },
        };
        let ray = c2Ray {
            p: c2v {
                x: rng.sym(30.0),
                y: rng.sym(30.0),
            },
            d,
            t: rng.unit() * 60.0,
        };
        let ctx = format!(
            "{} aabb ({:?},{:?})-({:?},{:?})",
            ray_ctx(&ray), s.min.x, s.min.y, s.max.x, s.max.y
        );
        if drive("c2RaytoAABB", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Zero-length rays (t == 0 => p0 == p1, ab == 0, n == 0).
    let mut rng = Rng::new(0x6005);
    for _ in 0..100_000 {
        let s = tame_aabb(&mut rng);
        let ray = c2Ray {
            p: c2v {
                x: rng.sym(20.0),
                y: rng.sym(20.0),
            },
            d: c2v {
                x: rng.sym(1.0),
                y: rng.sym(1.0),
            },
            t: 0.0,
        };
        let ctx = format!(
            "{} aabb ({:?},{:?})-({:?},{:?})",
            ray_ctx(&ray), s.min.x, s.min.y, s.max.x, s.max.y
        );
        if drive("c2RaytoAABB", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    assert!(hits > 1000, "too few hit-branch cases ({hits})");
    assert!(misses > 1000, "too few miss-branch cases ({misses})");
}

// ---------------------------------------------------------------------------
// c2RaytoCapsule
// ---------------------------------------------------------------------------

#[test]
fn t_c2RaytoCapsule() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnRayCapsule_i>("c2RaytoCapsule");
    let (cf, rf) = (&*c, &*r);

    let mut hits = 0u64;
    let mut misses = 0u64;

    let cap_ctx = |ray: &c2Ray, s: &c2Capsule| {
        format!(
            "{} cap a=({:?},{:?}) b=({:?},{:?}) r={:?}",
            ray_ctx(ray), s.a.x, s.a.y, s.b.x, s.b.y, s.r
        )
    };

    let mut rng = Rng::new(0x7001);
    for _ in 0..200_000 {
        let ray = wild_ray(&mut rng);
        let s = wild_capsule(&mut rng);
        let ctx = cap_ctx(&ray, &s);
        if drive("c2RaytoCapsule", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    let mut rng = Rng::new(0x7002);
    for _ in 0..300_000 {
        let ray = tame_ray(&mut rng);
        let s = tame_capsule(&mut rng);
        let ctx = cap_ctx(&ray, &s);
        if drive("c2RaytoCapsule", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Aim at points on / around the capsule: covers the early-out AABB and
    // circle-containment arms, the two `c2RaytoCircle` delegations, and the
    // side-wall branch.
    let mut rng = Rng::new(0x7003);
    for _ in 0..400_000 {
        let s = tame_capsule(&mut rng);
        let u = rng.unit();
        let mid = c2v {
            x: s.a.x + (s.b.x - s.a.x) * u,
            y: s.a.y + (s.b.y - s.a.y) * u,
        };
        let ang = rng.unit() * 6.283_185_5;
        let off = s.r * (rng.unit() * 1.5);
        let target = c2v {
            x: mid.x + off * ang.cos(),
            y: mid.y + off * ang.sin(),
        };
        let ray = aimed_ray(&mut rng, target);
        let ctx = cap_ctx(&ray, &s);
        if drive("c2RaytoCapsule", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Ray origins placed inside the capsule (early `return 1` paths).
    let mut rng = Rng::new(0x7004);
    for _ in 0..200_000 {
        let s = tame_capsule(&mut rng);
        let u = rng.unit();
        let ang = rng.unit() * 6.283_185_5;
        let off = s.r * rng.unit() * 0.9;
        let origin = c2v {
            x: s.a.x + (s.b.x - s.a.x) * u + off * ang.cos(),
            y: s.a.y + (s.b.y - s.a.y) * u + off * ang.sin(),
        };
        let a2 = rng.unit() * 6.283_185_5;
        let ray = c2Ray {
            p: origin,
            d: c2v {
                x: a2.cos(),
                y: a2.sin(),
            },
            t: rng.unit() * 30.0,
        };
        let ctx = cap_ctx(&ray, &s);
        if drive("c2RaytoCapsule", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Degenerate capsules: a == b makes c2Norm produce NaN/inf throughout.
    let mut rng = Rng::new(0x7005);
    for _ in 0..100_000 {
        let s = degenerate_capsule(&mut rng);
        let ray = tame_ray(&mut rng);
        let ctx = cap_ctx(&ray, &s);
        if drive("c2RaytoCapsule", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    // Axis-aligned capsules with axis-aligned rays: yAe.x == yAp.x makes
    // `d == 0` in the side-wall branch, so `t` is +/-inf or NaN.
    let mut rng = Rng::new(0x7006);
    for _ in 0..200_000 {
        let a = c2v {
            x: rng.sym(10.0),
            y: rng.sym(10.0),
        };
        let s = c2Capsule {
            a,
            b: c2v {
                x: a.x,
                y: a.y + 1.0 + rng.unit() * 10.0,
            },
            r: 0.5 + rng.unit() * 3.0,
        };
        let d = match rng.next_u32() % 4 {
            0 => c2v { x: 0.0, y: 1.0 },
            1 => c2v { x: 0.0, y: -1.0 },
            2 => c2v { x: 1.0, y: 0.0 },
            _ => c2v { x: -1.0, y: 0.0 },
        };
        let ray = c2Ray {
            p: c2v {
                x: rng.sym(15.0),
                y: rng.sym(15.0),
            },
            d,
            t: rng.unit() * 30.0,
        };
        let ctx = cap_ctx(&ray, &s);
        if drive("c2RaytoCapsule", cf, rf, ray, s, &ctx) != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    assert!(hits > 1000, "too few hit-branch cases ({hits})");
    assert!(misses > 1000, "too few miss-branch cases ({misses})");
}
