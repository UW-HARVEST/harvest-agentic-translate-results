//! Level 2: the raycast routines that write through `c2Raycast *out`
//! (`c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`).
//!
//! `out` is pre-filled with a sentinel before every call, so a difference in
//! *whether* the struct is written is caught as well as a difference in the
//! written values.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

type FnCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
type FnAabb = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int;
type FnCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;

/// A ray that actually points at `target`, with `t` long enough to reach it.
/// Purely random rays almost never hit anything, so the interesting branches
/// of the C code need aimed rays.
fn aimed_ray(rng: &mut Rng, target: C2v, overshoot: f32) -> C2Ray {
    let p = rng.vec();
    let dx = target.x - p.x;
    let dy = target.y - p.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 || !len.is_finite() {
        return rng.ray();
    }
    C2Ray {
        p,
        d: C2v {
            x: dx / len,
            y: dy / len,
        },
        t: len * overshoot,
    }
}

#[test]
fn c2RaytoCircle_matches() {
    let (c, r): (FnCircle, FnCircle) = syms(b"c2RaytoCircle\0");
    let mut rng = Rng::new(201);
    for i in 0..iters(60_000) {
        let circle = rng.circle();
        let over = rng.unit() * 2.0;
        let ray = match rng.below(3) {
            0 => rng.ray(),
            1 => aimed_ray(&mut rng, circle.p, 1.0),
            _ => aimed_ray(&mut rng, circle.p, over),
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let (gc, gr) = unsafe { (c(ray, circle, &mut oc), r(ray, circle, &mut or)) };
        assert_eq!(
            gc, gr,
            "c2RaytoCircle return mismatch at iter {i}\n  ray: {ray:?}\n  circle: {circle:?}"
        );
        assert!(
            cast_eq(oc, or),
            "c2RaytoCircle out mismatch at iter {i}\n  ray: {ray:?}\n  circle: {circle:?}\n  \
             C:    {}\n  Rust: {}",
            show_cast(oc),
            show_cast(or)
        );
    }
}

#[test]
fn c2RaytoAABB_matches() {
    let (c, r): (FnAabb, FnAabb) = syms(b"c2RaytoAABB\0");
    let mut rng = Rng::new(202);
    for i in 0..iters(60_000) {
        let bb = rng.aabb();
        let centre = C2v {
            x: (bb.min.x + bb.max.x) * 0.5,
            y: (bb.min.y + bb.max.y) * 0.5,
        };
        let over = rng.unit() * 2.0;
        let ray = match rng.below(4) {
            0 => rng.ray(),
            1 => aimed_ray(&mut rng, centre, 1.0),
            2 => aimed_ray(&mut rng, bb.min, over),
            _ => aimed_ray(&mut rng, centre, over),
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let (gc, gr) = unsafe { (c(ray, bb, &mut oc), r(ray, bb, &mut or)) };
        assert_eq!(
            gc, gr,
            "c2RaytoAABB return mismatch at iter {i}\n  ray: {ray:?}\n  bb: {bb:?}"
        );
        assert!(
            cast_eq(oc, or),
            "c2RaytoAABB out mismatch at iter {i}\n  ray: {ray:?}\n  bb: {bb:?}\n  \
             C:    {}\n  Rust: {}",
            show_cast(oc),
            show_cast(or)
        );
    }
}

#[test]
fn c2RaytoCapsule_matches() {
    let (c, r): (FnCapsule, FnCapsule) = syms(b"c2RaytoCapsule\0");
    let mut rng = Rng::new(203);
    for i in 0..iters(60_000) {
        let cap = rng.capsule();
        let mid = C2v {
            x: (cap.a.x + cap.b.x) * 0.5,
            y: (cap.a.y + cap.b.y) * 0.5,
        };
        let over = rng.unit() * 2.0;
        let ray = match rng.below(5) {
            0 => rng.ray(),
            1 => aimed_ray(&mut rng, mid, 1.0),
            2 => aimed_ray(&mut rng, cap.a, over),
            3 => aimed_ray(&mut rng, cap.b, over),
            // start inside the capsule: hits the early-return branches
            _ => {
                let d = rng.vec();
                C2Ray {
                    p: mid,
                    d,
                    t: over * 5.0,
                }
            }
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let (gc, gr) = unsafe { (c(ray, cap, &mut oc), r(ray, cap, &mut or)) };
        assert_eq!(
            gc, gr,
            "c2RaytoCapsule return mismatch at iter {i}\n  ray: {ray:?}\n  cap: {cap:?}"
        );
        assert!(
            cast_eq(oc, or),
            "c2RaytoCapsule out mismatch at iter {i}\n  ray: {ray:?}\n  cap: {cap:?}\n  \
             C:    {}\n  Rust: {}",
            show_cast(oc),
            show_cast(or)
        );
    }
}

/// Axis-aligned and degenerate configurations, chosen to land exactly on the
/// `<=` / `>=` boundaries and the divide-by-zero paths of the C source.
#[test]
fn raycast_degenerate_cases_match() {
    let (circle_c, circle_r): (FnCircle, FnCircle) = syms(b"c2RaytoCircle\0");
    let (aabb_c, aabb_r): (FnAabb, FnAabb) = syms(b"c2RaytoAABB\0");
    let (cap_c, cap_r): (FnCapsule, FnCapsule) = syms(b"c2RaytoCapsule\0");

    let dirs = [
        C2v { x: 1.0, y: 0.0 },
        C2v { x: -1.0, y: 0.0 },
        C2v { x: 0.0, y: 1.0 },
        C2v { x: 0.0, y: -1.0 },
        C2v { x: 0.0, y: 0.0 },
    ];
    let coords = [-2.0f32, -1.0, -0.0, 0.0, 1.0, 2.0];
    let ts = [0.0f32, 1.0, 2.0, 4.0, -1.0];
    let radii = [0.0f32, -0.0, 1.0, 2.0, -1.0];

    let mut cases = 0usize;
    for &d in &dirs {
        for &px in &coords {
            for &py in &coords {
                for &t in &ts {
                    let ray = C2Ray {
                        p: C2v { x: px, y: py },
                        d,
                        t,
                    };
                    for &rad in &radii {
                        cases += 1;

                        let circle = C2Circle {
                            p: C2v { x: 0.0, y: 0.0 },
                            r: rad,
                        };
                        let mut oc = SENTINEL;
                        let mut or = SENTINEL;
                        let (a, b) = unsafe {
                            (
                                circle_c(ray, circle, &mut oc),
                                circle_r(ray, circle, &mut or),
                            )
                        };
                        assert_eq!(a, b, "c2RaytoCircle {ray:?} {circle:?}");
                        assert!(
                            cast_eq(oc, or),
                            "c2RaytoCircle out {ray:?} {circle:?}: C {} vs Rust {}",
                            show_cast(oc),
                            show_cast(or)
                        );

                        let bb = C2AABB {
                            min: C2v { x: -rad, y: -rad },
                            max: C2v { x: rad, y: rad },
                        };
                        let mut oc = SENTINEL;
                        let mut or = SENTINEL;
                        let (a, b) =
                            unsafe { (aabb_c(ray, bb, &mut oc), aabb_r(ray, bb, &mut or)) };
                        assert_eq!(a, b, "c2RaytoAABB {ray:?} {bb:?}");
                        assert!(
                            cast_eq(oc, or),
                            "c2RaytoAABB out {ray:?} {bb:?}: C {} vs Rust {}",
                            show_cast(oc),
                            show_cast(or)
                        );

                        for cap in [
                            C2Capsule {
                                a: C2v { x: 0.0, y: -1.0 },
                                b: C2v { x: 0.0, y: 1.0 },
                                r: rad,
                            },
                            C2Capsule {
                                a: C2v { x: -1.0, y: 0.0 },
                                b: C2v { x: 1.0, y: 0.0 },
                                r: rad,
                            },
                            // degenerate: a == b, so c2Norm divides by zero
                            C2Capsule {
                                a: C2v { x: 0.0, y: 0.0 },
                                b: C2v { x: 0.0, y: 0.0 },
                                r: rad,
                            },
                        ] {
                            let mut oc = SENTINEL;
                            let mut or = SENTINEL;
                            let (a, b) =
                                unsafe { (cap_c(ray, cap, &mut oc), cap_r(ray, cap, &mut or)) };
                            assert_eq!(a, b, "c2RaytoCapsule {ray:?} {cap:?}");
                            assert!(
                                cast_eq(oc, or),
                                "c2RaytoCapsule out {ray:?} {cap:?}: C {} vs Rust {}",
                                show_cast(oc),
                                show_cast(or)
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(cases > 500, "expected a broad sweep, ran {cases}");
}
