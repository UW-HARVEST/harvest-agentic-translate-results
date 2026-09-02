//! Phase B — CONFIGS.md rows 63..70: the `c2CastRay` dispatcher (every
//! `C2_TYPE` arm, with and without `bx`) and the `poly_ray` driver.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 3000;

fn rand_ray(rng: &mut Rng) -> c2Ray {
    c2Ray {
        p: rng.v_small(),
        d: if rng.below(4) == 0 { AXIS_DIRS[rng.below(4)] } else { rng.v_dir() },
        t: rng.range(0.0, 40.0),
    }
}

fn rand_bx(rng: &mut Rng) -> c2x {
    let k = rng.below(4);
    match k {
        0 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } },
        1 => c2x { p: rng.v_small(), r: c2r { c: 1.0, s: 0.0 } },
        2 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot_unit() },
        _ => c2x { p: rng.v_small(), r: rng.rot_unit() },
    }
}

/// Row 63 + row 68: `C2_TYPE_CIRCLE` through the dispatcher, with `bx = NULL`
/// and `bx = &random` (the C ignores `bx` on this arm), cross-checked against
/// the direct `c2RaytoCircle` call.
#[test]
fn row63_row68_cast_circle() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x63);
    unsafe {
        for _ in 0..(N * 4) {
            let A = rand_ray(&mut rng);
            let B = c2Circle { p: rng.v_small(), r: rng.range(-2.0, 10.0) };
            let bx = rand_bx(&mut rng);
            let sp = &B as *const c2Circle as *const c_void;
            for b in [None, Some(&bx)] {
                let cr = call_cast(&p.c, A, sp, b, C2_TYPE_CIRCLE);
                let rr = call_cast(&p.rs, A, sp, b, C2_TYPE_CIRCLE);
                d.ray("c2CastRay(CIRCLE)", cr, rr);
                // row 68: dispatcher must equal the direct low-level call
                d.ray("c2CastRay(CIRCLE) == c2RaytoCircle [C]", cr, call_circle(&p.c, A, B));
                d.ray("c2CastRay(CIRCLE) == c2RaytoCircle [RS]", rr, call_circle(&p.rs, A, B));
            }
        }
    }
    d.finish("rows 63,68: c2CastRay C2_TYPE_CIRCLE");
}

/// Row 64 + row 68: `C2_TYPE_AABB`.
#[test]
fn row64_row68_cast_aabb() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x64);
    unsafe {
        for _ in 0..(N * 4) {
            let A = rand_ray(&mut rng);
            let bx0 = rng.sym(10.0);
            let by0 = rng.sym(10.0);
            let B = c2AABB {
                min: c2v { x: bx0, y: by0 },
                max: c2v {
                    x: bx0 + rng.range(0.0, 10.0),
                    y: by0 + rng.range(0.0, 10.0),
                },
            };
            let bx = rand_bx(&mut rng);
            let sp = &B as *const c2AABB as *const c_void;
            for b in [None, Some(&bx)] {
                let cr = call_cast(&p.c, A, sp, b, C2_TYPE_AABB);
                let rr = call_cast(&p.rs, A, sp, b, C2_TYPE_AABB);
                d.ray("c2CastRay(AABB)", cr, rr);
                d.ray("c2CastRay(AABB) == c2RaytoAABB [C]", cr, call_aabb(&p.c, A, B));
                d.ray("c2CastRay(AABB) == c2RaytoAABB [RS]", rr, call_aabb(&p.rs, A, B));
            }
        }
    }
    d.finish("rows 64,68: c2CastRay C2_TYPE_AABB");
}

/// Row 65 + row 68: `C2_TYPE_CAPSULE`.
#[test]
fn row65_row68_cast_capsule() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x65);
    unsafe {
        for _ in 0..(N * 4) {
            let A = rand_ray(&mut rng);
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.0, 16.0);
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
                r: rng.range(-1.0, 6.0),
            };
            let bx = rand_bx(&mut rng);
            let sp = &B as *const c2Capsule as *const c_void;
            for b in [None, Some(&bx)] {
                let cr = call_cast(&p.c, A, sp, b, C2_TYPE_CAPSULE);
                let rr = call_cast(&p.rs, A, sp, b, C2_TYPE_CAPSULE);
                d.ray("c2CastRay(CAPSULE)", cr, rr);
                d.ray("c2CastRay(CAPSULE) == c2RaytoCapsule [C]", cr, call_capsule(&p.c, A, B));
                d.ray("c2CastRay(CAPSULE) == c2RaytoCapsule [RS]", rr, call_capsule(&p.rs, A, B));
            }
        }
    }
    d.finish("rows 65,68: c2CastRay C2_TYPE_CAPSULE");
}

/// Rows 66-67 + row 68: `C2_TYPE_POLY` with `bx = NULL` and with a transform.
#[test]
fn row66_row67_row68_cast_poly() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x6667);
    unsafe {
        for _ in 0..(N * 4) {
            let A = rand_ray(&mut rng);
            let count = 1 + rng.below(8);
            let poly = if rng.bool() {
                make_convex_poly(&mut rng, count)
            } else {
                make_axis_quad(&mut rng)
            };
            let bx = rand_bx(&mut rng);
            let sp = &poly as *const c2Poly as *const c_void;
            for b in [None, Some(&bx)] {
                let cr = call_cast(&p.c, A, sp, b, C2_TYPE_POLY);
                let rr = call_cast(&p.rs, A, sp, b, C2_TYPE_POLY);
                d.ray("c2CastRay(POLY)", cr, rr);
                d.ray("c2CastRay(POLY) == c2RaytoPoly [C]", cr, call_poly(&p.c, A, &poly, b));
                d.ray("c2CastRay(POLY) == c2RaytoPoly [RS]", rr, call_poly(&p.rs, A, &poly, b));
            }
        }
    }
    d.finish("rows 66-68: c2CastRay C2_TYPE_POLY");
}

/// Rows 69-70: the fixed `poly_ray` driver — return value, both out-params,
/// repeated invocation, and pre-dirtied out buffers (so that any field the C
/// leaves untouched is compared as "untouched" too).
#[test]
fn row69_row70_poly_ray() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x6970);
    unsafe {
        // clean out-params
        for _ in 0..64 {
            let (mut c1, mut c2) = (POISON, POISON);
            let (mut r1, mut r2) = (POISON, POISON);
            let cret = (p.c.poly_ray)(&mut c1, &mut c2);
            let rret = (p.rs.poly_ray)(&mut r1, &mut r2);
            d.int("poly_ray ret", cret, rret);
            d.ray("poly_ray cast1", (cret, c1), (rret, r1));
            d.ray("poly_ray cast2", (cret, c2), (rret, r2));
        }
        // row 70: arbitrary pre-existing garbage in the out buffers
        for _ in 0..2000 {
            let dirty = c2Raycast { t: rng.f_mixed(), n: rng.v_mixed() };
            let dirty2 = c2Raycast { t: rng.f_mixed(), n: rng.v_mixed() };
            let (mut c1, mut c2) = (dirty, dirty2);
            let (mut r1, mut r2) = (dirty, dirty2);
            let cret = (p.c.poly_ray)(&mut c1, &mut c2);
            let rret = (p.rs.poly_ray)(&mut r1, &mut r2);
            d.int("poly_ray(dirty) ret", cret, rret);
            d.ray("poly_ray(dirty) cast1", (cret, c1), (rret, r1));
            d.ray("poly_ray(dirty) cast2", (cret, c2), (rret, r2));
        }
        // aliasing: both out-params pointing at the same object
        for _ in 0..64 {
            let mut c1 = POISON;
            let mut r1 = POISON;
            let cret = (p.c.poly_ray)(&mut c1, &mut c1);
            let rret = (p.rs.poly_ray)(&mut r1, &mut r1);
            d.int("poly_ray(alias) ret", cret, rret);
            d.ray("poly_ray(alias) out", (cret, c1), (rret, r1));
        }
        // report the actual driver result so the expected value is on record
        let (mut c1, mut c2) = (POISON, POISON);
        let cret = (p.c.poly_ray)(&mut c1, &mut c2);
        eprintln!(
            "poly_ray C reference: ret={cret} cast1={} cast2={}",
            fmt_cast(c1),
            fmt_cast(c2)
        );
    }
    d.finish("rows 69-70: poly_ray driver");
}
