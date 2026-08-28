//! Level 4: the dispatcher `c2CastRay` and the public entry point `spec_ray`.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;

// ---------------------------------------------------------------------------
// c2CastRay
// ---------------------------------------------------------------------------
//
// Only the three enum values defined by `C2_TYPE` are exercised.  The C
// `switch` has no `default:` arm and no trailing `return`, so any other value
// falls off the end of a non-void function -- undefined behaviour, not a
// contract that can be matched.

fn cast_ctx(ray: &c2Ray) -> String {
    format!(
        "ray p=({:?},{:?}) d=({:?},{:?}) t={:?}",
        ray.p.x, ray.p.y, ray.d.x, ray.d.y, ray.t
    )
}

#[test]
fn t_c2CastRay_circle() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnCastRay>("c2CastRay");
    let mut rng = Rng::new(0x8001);
    for i in 0..300_000 {
        let ray = if i % 3 == 0 {
            c2Ray {
                p: rng.vec_wild(),
                d: rng.vec_wild(),
                t: rng.float(),
            }
        } else {
            let ang = rng.unit() * 6.283_185_5;
            c2Ray {
                p: c2v { x: rng.sym(20.0), y: rng.sym(20.0) },
                d: c2v { x: ang.cos(), y: ang.sin() },
                t: rng.unit() * 40.0,
            }
        };
        let shape = c2Circle {
            p: c2v { x: rng.sym(20.0), y: rng.sym(20.0) },
            r: rng.unit() * 10.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let (cr, rr) = unsafe {
            (
                c(ray, &shape as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut co),
                r(ray, &shape as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut ro),
            )
        };
        let ctx = format!("{} circle p=({:?},{:?}) r={:?}", cast_ctx(&ray), shape.p.x, shape.p.y, shape.r);
        assert_i_eq("c2CastRay/circle", &ctx, cr, rr);
        assert_cast_eq("c2CastRay/circle", &ctx, &co, &ro);
    }
}

#[test]
fn t_c2CastRay_aabb() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnCastRay>("c2CastRay");
    let mut rng = Rng::new(0x8002);
    for i in 0..300_000 {
        let ray = if i % 3 == 0 {
            c2Ray {
                p: rng.vec_wild(),
                d: rng.vec_wild(),
                t: rng.float(),
            }
        } else {
            let ang = rng.unit() * 6.283_185_5;
            c2Ray {
                p: c2v { x: rng.sym(20.0), y: rng.sym(20.0) },
                d: c2v { x: ang.cos(), y: ang.sin() },
                t: rng.unit() * 40.0,
            }
        };
        let cx = rng.sym(15.0);
        let cy = rng.sym(15.0);
        let ex = rng.unit() * 10.0;
        let ey = rng.unit() * 10.0;
        let shape = c2AABB {
            min: c2v { x: cx - ex, y: cy - ey },
            max: c2v { x: cx + ex, y: cy + ey },
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let (cr, rr) = unsafe {
            (
                c(ray, &shape as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut co),
                r(ray, &shape as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut ro),
            )
        };
        let ctx = format!(
            "{} aabb ({:?},{:?})-({:?},{:?})",
            cast_ctx(&ray), shape.min.x, shape.min.y, shape.max.x, shape.max.y
        );
        assert_i_eq("c2CastRay/aabb", &ctx, cr, rr);
        assert_cast_eq("c2CastRay/aabb", &ctx, &co, &ro);
    }
}

#[test]
fn t_c2CastRay_capsule() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnCastRay>("c2CastRay");
    let mut rng = Rng::new(0x8003);
    for i in 0..300_000 {
        let ray = if i % 3 == 0 {
            c2Ray {
                p: rng.vec_wild(),
                d: rng.vec_wild(),
                t: rng.float(),
            }
        } else {
            let ang = rng.unit() * 6.283_185_5;
            c2Ray {
                p: c2v { x: rng.sym(20.0), y: rng.sym(20.0) },
                d: c2v { x: ang.cos(), y: ang.sin() },
                t: rng.unit() * 40.0,
            }
        };
        let a = c2v { x: rng.sym(15.0), y: rng.sym(15.0) };
        let ang = rng.unit() * 6.283_185_5;
        let len = if i % 7 == 0 { 0.0 } else { 0.5 + rng.unit() * 20.0 };
        let shape = c2Capsule {
            a,
            b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
            r: 0.01 + rng.unit() * 6.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let (cr, rr) = unsafe {
            (
                c(ray, &shape as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut co),
                r(ray, &shape as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut ro),
            )
        };
        let ctx = format!(
            "{} cap a=({:?},{:?}) b=({:?},{:?}) r={:?}",
            cast_ctx(&ray), shape.a.x, shape.a.y, shape.b.x, shape.b.y, shape.r
        );
        assert_i_eq("c2CastRay/capsule", &ctx, cr, rr);
        assert_cast_eq("c2CastRay/capsule", &ctx, &co, &ro);
    }
}

// ---------------------------------------------------------------------------
// spec_ray
// ---------------------------------------------------------------------------

fn drive_spec_ray(
    cf: &FnSpecRay,
    rf: &FnSpecRay,
    args: [f32; 7],
) -> i32 {
    let [mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y] = args;
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { cf(&mut co, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
    let rr = unsafe { rf(&mut ro, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
    let ctx = format!(
        "mp=({mp_x:?},{mp_y:?}) c=({c_p_x:?},{c_p_y:?},{c_r:?}) rp=({r_p_x:?},{r_p_y:?})"
    );
    assert_i_eq("spec_ray", &ctx, cr, rr);
    assert_cast_eq("spec_ray", &ctx, &co, &ro);
    cr
}

#[test]
fn t_spec_ray_edge_grid() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnSpecRay>("spec_ray");
    let (cf, rf) = (&*c, &*r);
    // Exhaustive over a small but nasty grid (7^7 would be too many, so sweep
    // one axis at a time around a set of bases).
    let vals: &[f32] = &[
        0.0, -0.0, 1.0, -1.0, 5.0, -5.0, 0.5, 1.0e-30, 1.0e30, f32::INFINITY,
        f32::NEG_INFINITY, f32::NAN, f32::MAX, f32::MIN_POSITIVE,
    ];
    let bases: &[[f32; 7]] = &[
        [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        [10.0, 10.0, 5.0, 5.0, 2.0, -3.0, -3.0],
        [-2.0, 7.0, 0.0, 0.0, 3.0, 4.0, -1.0],
        [1.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.0],
    ];
    for base in bases {
        for idx in 0..7 {
            for &v in vals {
                let mut args = *base;
                args[idx] = v;
                drive_spec_ray(cf, rf, args);
            }
        }
        // and pairwise sweeps
        for i in 0..7 {
            for j in 0..7 {
                for &a in vals {
                    for &b in vals {
                        let mut args = *base;
                        args[i] = a;
                        args[j] = b;
                        drive_spec_ray(cf, rf, args);
                    }
                }
            }
        }
    }
}

#[test]
fn t_spec_ray_random() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnSpecRay>("spec_ray");
    let (cf, rf) = (&*c, &*r);

    let mut hits = 0u64;
    let mut misses = 0u64;

    // Fully wild.
    let mut rng = Rng::new(0x9001);
    for _ in 0..300_000 {
        let args = [
            rng.float(), rng.float(), rng.float(), rng.float(), rng.float(),
            rng.float(), rng.float(),
        ];
        if drive_spec_ray(cf, rf, args) != 0 { hits += 1 } else { misses += 1 }
    }

    // Realistic mouse-picking geometry.
    let mut rng = Rng::new(0x9002);
    for _ in 0..400_000 {
        let args = [
            rng.sym(400.0), rng.sym(400.0),
            rng.sym(400.0), rng.sym(400.0),
            rng.unit() * 80.0,
            rng.sym(400.0), rng.sym(400.0),
        ];
        if drive_spec_ray(cf, rf, args) != 0 { hits += 1 } else { misses += 1 }
    }

    // Mouse point deliberately placed on / near the circle: drives `disc`
    // and the `t <= A.t` comparison right up against their boundaries.
    let mut rng = Rng::new(0x9003);
    for _ in 0..400_000 {
        let cx = rng.sym(100.0);
        let cy = rng.sym(100.0);
        let rad = 0.5 + rng.unit() * 40.0;
        let ang = rng.unit() * 6.283_185_5;
        let scale = 0.98 + rng.unit() * 0.04;
        let args = [
            cx + rad * scale * ang.cos(),
            cy + rad * scale * ang.sin(),
            cx, cy, rad,
            rng.sym(200.0), rng.sym(200.0),
        ];
        if drive_spec_ray(cf, rf, args) != 0 { hits += 1 } else { misses += 1 }
    }

    // Ray origin coincident with the mouse point: `c2Norm` of a zero vector.
    let mut rng = Rng::new(0x9004);
    for _ in 0..100_000 {
        let x = rng.sym(50.0);
        let y = rng.sym(50.0);
        let args = [x, y, rng.sym(50.0), rng.sym(50.0), rng.unit() * 20.0, x, y];
        if drive_spec_ray(cf, rf, args) != 0 { hits += 1 } else { misses += 1 }
    }

    // Small integral coordinates: dense coverage of exact-tie comparisons.
    let mut rng = Rng::new(0x9005);
    for _ in 0..300_000 {
        let q = |rng: &mut Rng| ((rng.next_u32() % 21) as i32 - 10) as f32;
        let args = [
            q(&mut rng), q(&mut rng), q(&mut rng), q(&mut rng),
            (rng.next_u32() % 8) as f32,
            q(&mut rng), q(&mut rng),
        ];
        if drive_spec_ray(cf, rf, args) != 0 { hits += 1 } else { misses += 1 }
    }

    assert!(hits > 1000, "too few hit cases ({hits})");
    assert!(misses > 1000, "too few miss cases ({misses})");
}
