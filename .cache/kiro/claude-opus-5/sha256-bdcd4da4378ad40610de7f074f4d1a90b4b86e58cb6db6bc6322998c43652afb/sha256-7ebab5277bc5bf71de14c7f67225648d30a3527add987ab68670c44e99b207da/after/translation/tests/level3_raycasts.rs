//! Level 3: raycast routines (`c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`,
//! `c2RaytoPoly`). Both the integer return value *and* the fully-written
//! `c2Raycast` out-parameter are compared.
//!
//! The out-parameter is pre-filled with an identical poison pattern on both
//! sides, so a function that leaves it untouched (or partially written) is
//! still compared field-for-field.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_int;

/// Poison so that "not written" is distinguishable from any plausible result.
fn poison() -> c2Raycast {
    c2Raycast {
        t: f32::from_bits(0xDEAD_BEEF),
        n: c2v {
            x: f32::from_bits(0xCAFE_BABE),
            y: f32::from_bits(0xFEED_FACE),
        },
    }
}

#[track_caller]
fn cmp(name: &str, ctx: &str, cr: (c_int, c2Raycast), rr: (c_int, c2Raycast)) {
    assert_bits(&format!("{name} ret"), ctx, &cr.0, &rr.0);
    assert_bits(&format!("{name} out"), ctx, &cr.1, &rr.1);
}

// ---------------------------------------------------------------------------
// c2RaytoCircle
// ---------------------------------------------------------------------------

fn run_circle(f: FnRayCircle_i, A: c2Ray, B: c2Circle) -> (c_int, c2Raycast) {
    let mut out = poison();
    let ret = unsafe { f(A, B, &mut out) };
    (ret, out)
}

#[test]
fn c2RaytoCircle_matches() {
    let (c, r) = libs().sym::<FnRayCircle_i>("c2RaytoCircle");

    // Hand-picked cases covering each branch: disc<0, t<0, t>A.t, t in range,
    // ray origin inside the circle, degenerate radius, zero direction.
    let mut cases: Vec<(c2Ray, c2Circle)> = vec![
        // straight-on hit
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // miss (disc < 0)
        (
            c2Ray {
                p: c2v { x: -5.0, y: 5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // hit exactly at t == A.t boundary
        (
            c2Ray {
                p: c2v { x: -2.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 1.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // out of range (t > A.t)
        (
            c2Ray {
                p: c2v { x: -20.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 1.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // origin inside -> t negative
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // exactly tangent (disc == 0)
        (
            c2Ray {
                p: c2v { x: -5.0, y: 1.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // zero radius, ray passes through the centre -> c2Norm(0,0)
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 0.0,
            },
        ),
        // zero direction vector
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 0.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // negative radius
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: -1.0,
            },
        ),
        // NaN / inf plumbing
        (
            c2Ray {
                p: c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Ray {
                p: c2v {
                    x: f32::NEG_INFINITY,
                    y: 0.0,
                },
                d: c2v { x: 1.0, y: 0.0 },
                t: f32::INFINITY,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
    ];

    // Random coarse inputs -- small integers/halves land on the exact
    // equality boundaries (`t >= 0`, `t <= A.t`, `disc == 0`).
    let mut rng = Rng::new(0x4444);
    for _ in 0..40_000 {
        cases.push((
            c2Ray {
                p: rng.vec_coarse(),
                d: rng.vec_coarse(),
                t: rng.f32_coarse(),
            },
            c2Circle {
                p: rng.vec_coarse(),
                r: rng.f32_coarse(),
            },
        ));
    }
    // Random continuous inputs, including normalised directions.
    for _ in 0..40_000 {
        let ang = rng.f32_range(4.0);
        cases.push((
            c2Ray {
                p: rng.vec_range(20.0),
                d: c2v {
                    x: ang.cos(),
                    y: ang.sin(),
                },
                t: rng.f32_range(50.0),
            },
            c2Circle {
                p: rng.vec_range(20.0),
                r: rng.f32_range(10.0),
            },
        ));
    }

    for (A, B) in cases {
        cmp(
            "c2RaytoCircle",
            &format!("{A:?} {B:?}"),
            run_circle(c, A, B),
            run_circle(r, A, B),
        );
    }
}

// ---------------------------------------------------------------------------
// c2RaytoAABB
// ---------------------------------------------------------------------------

fn run_aabb(f: FnRayAABB_i, A: c2Ray, B: c2AABB) -> (c_int, c2Raycast) {
    let mut out = poison();
    let ret = unsafe { f(A, B, &mut out) };
    (ret, out)
}

#[test]
fn c2RaytoAABB_matches() {
    let (c, r) = libs().sym::<FnRayAABB_i>("c2RaytoAABB");

    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let mut cases: Vec<(c2Ray, c2AABB)> = vec![
        // hits from each of the four sides
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            unit,
        ),
        (
            c2Ray {
                p: c2v { x: 5.0, y: 0.0 },
                d: c2v { x: -1.0, y: 0.0 },
                t: 10.0,
            },
            unit,
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: -5.0 },
                d: c2v { x: 0.0, y: 1.0 },
                t: 10.0,
            },
            unit,
        ),
        (
            c2Ray {
                p: c2v { x: 0.0, y: 5.0 },
                d: c2v { x: 0.0, y: -1.0 },
                t: 10.0,
            },
            unit,
        ),
        // early-out via the a_box overlap test
        (
            c2Ray {
                p: c2v { x: -5.0, y: 5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 1.0,
            },
            unit,
        ),
        // early-out via the separating-axis `d > 0` test
        (
            c2Ray {
                p: c2v { x: -5.0, y: 3.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 20.0,
            },
            unit,
        ),
        // exact corner hit
        (
            c2Ray {
                p: c2v { x: -3.0, y: -3.0 },
                d: c2v { x: 1.0, y: 1.0 },
                t: 10.0,
            },
            unit,
        ),
        // degenerate ray (zero length) inside the box -> ab == 0, n == 0
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 0.0, y: 0.0 },
                t: 0.0,
            },
            unit,
        ),
        // degenerate box
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 0.0, y: 0.0 },
            },
        ),
        // inverted box
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2AABB {
                min: c2v { x: 1.0, y: 1.0 },
                max: c2v { x: -1.0, y: -1.0 },
            },
        ),
        // infinities / NaN
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: f32::INFINITY,
            },
            unit,
        ),
        (
            c2Ray {
                p: c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            unit,
        ),
    ];

    let mut rng = Rng::new(0x5555);
    // Coarse inputs are essential here: the `t0 >= t1 && ...` tie-break chain
    // and the `da * db > 0` / `d != 0` branches only trigger on exact values.
    for _ in 0..60_000 {
        cases.push((
            c2Ray {
                p: rng.vec_coarse(),
                d: rng.vec_coarse(),
                t: rng.f32_coarse(),
            },
            c2AABB {
                min: rng.vec_coarse(),
                max: rng.vec_coarse(),
            },
        ));
    }
    for _ in 0..40_000 {
        let ang = rng.f32_range(4.0);
        let lo = rng.vec_range(10.0);
        let ext = c2v {
            x: rng.f32_range(5.0).abs(),
            y: rng.f32_range(5.0).abs(),
        };
        cases.push((
            c2Ray {
                p: rng.vec_range(15.0),
                d: c2v {
                    x: ang.cos(),
                    y: ang.sin(),
                },
                t: rng.f32_range(30.0),
            },
            c2AABB {
                min: lo,
                max: c2v {
                    x: lo.x + ext.x,
                    y: lo.y + ext.y,
                },
            },
        ));
    }

    for (A, B) in cases {
        cmp(
            "c2RaytoAABB",
            &format!("{A:?} {B:?}"),
            run_aabb(c, A, B),
            run_aabb(r, A, B),
        );
    }
}

// ---------------------------------------------------------------------------
// c2RaytoCapsule
// ---------------------------------------------------------------------------

fn run_capsule(f: FnRayCapsule_i, A: c2Ray, B: c2Capsule) -> (c_int, c2Raycast) {
    let mut out = poison();
    let ret = unsafe { f(A, B, &mut out) };
    (ret, out)
}

#[test]
fn c2RaytoCapsule_matches() {
    let (c, r) = libs().sym::<FnRayCapsule_i>("c2RaytoCapsule");

    let vert = c2Capsule {
        a: c2v { x: 0.0, y: -2.0 },
        b: c2v { x: 0.0, y: 2.0 },
        r: 1.0,
    };
    let mut cases: Vec<(c2Ray, c2Capsule)> = vec![
        // side hit
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // side hit from the other side (exercises the `c2Skew(M.y)` normal)
        (
            c2Ray {
                p: c2v { x: 5.0, y: 0.0 },
                d: c2v { x: -1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // origin inside the capsule body -> early return 1
        (
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // origin inside end-cap a
        (
            c2Ray {
                p: c2v { x: 0.0, y: -2.5 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // origin inside end-cap b
        (
            c2Ray {
                p: c2v { x: 0.0, y: 2.5 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // hits the lower cap (y <= 0 branch)
        (
            c2Ray {
                p: c2v { x: -5.0, y: -2.5 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // hits the upper cap (y >= yBb.y branch)
        (
            c2Ray {
                p: c2v { x: -5.0, y: 2.5 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // complete miss
        (
            c2Ray {
                p: c2v { x: -5.0, y: 10.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
        // degenerate capsule (a == b) -> c2Norm(0,0) NaN axis
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ),
        // zero radius
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: -2.0 },
                b: c2v { x: 0.0, y: 2.0 },
                r: 0.0,
            },
        ),
        // negative radius
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: -2.0 },
                b: c2v { x: 0.0, y: 2.0 },
                r: -1.0,
            },
        ),
        // b below a (capsule pointing down -> negative yBb.y)
        (
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: 2.0 },
                b: c2v { x: 0.0, y: -2.0 },
                r: 1.0,
            },
        ),
        // ray parallel to the capsule axis (d == 0 in the transformed x)
        (
            c2Ray {
                p: c2v { x: 3.0, y: -5.0 },
                d: c2v { x: 0.0, y: 1.0 },
                t: 10.0,
            },
            vert,
        ),
        // NaN
        (
            c2Ray {
                p: c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            vert,
        ),
    ];

    let mut rng = Rng::new(0x6666);
    for _ in 0..60_000 {
        cases.push((
            c2Ray {
                p: rng.vec_coarse(),
                d: rng.vec_coarse(),
                t: rng.f32_coarse(),
            },
            c2Capsule {
                a: rng.vec_coarse(),
                b: rng.vec_coarse(),
                r: rng.f32_coarse(),
            },
        ));
    }
    for _ in 0..40_000 {
        let ang = rng.f32_range(4.0);
        cases.push((
            c2Ray {
                p: rng.vec_range(15.0),
                d: c2v {
                    x: ang.cos(),
                    y: ang.sin(),
                },
                t: rng.f32_range(30.0),
            },
            c2Capsule {
                a: rng.vec_range(10.0),
                b: rng.vec_range(10.0),
                r: rng.f32_range(4.0),
            },
        ));
    }

    for (A, B) in cases {
        cmp(
            "c2RaytoCapsule",
            &format!("{A:?} {B:?}"),
            run_capsule(c, A, B),
            run_capsule(r, A, B),
        );
    }
}
