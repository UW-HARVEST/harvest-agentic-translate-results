//! Coverage guard: asserts that the generators in these tests actually reach
//! the distinctive outcomes of each C branch, so a passing suite cannot be the
//! result of every case bailing out early.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

type FnAabb = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int;
type FnCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
type FnCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;

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

/// `c2RaytoAABB` selects one of four face normals; all four must be observed,
/// and the two early `return 0` paths must be taken too.
#[test]
fn aabb_branches_are_reached() {
    let (c, r): (FnAabb, FnAabb) = syms(b"c2RaytoAABB\0");
    let mut rng = Rng::new(202);
    let mut normals = [0usize; 4];
    let mut misses = 0usize;
    let mut hits = 0usize;

    for _ in 0..40_000 {
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
        assert_eq!(gc, gr);
        assert!(cast_eq(oc, or));
        if gc == 0 {
            misses += 1;
            continue;
        }
        hits += 1;
        let idx = match (oc.n.x, oc.n.y) {
            (-1.0, 0.0) => 0,
            (1.0, 0.0) => 1,
            (0.0, -1.0) => 2,
            (0.0, 1.0) => 3,
            other => panic!("unexpected AABB normal {other:?}"),
        };
        normals[idx] += 1;
    }

    assert!(hits > 100, "too few AABB hits: {hits}");
    assert!(misses > 100, "too few AABB misses: {misses}");
    assert!(
        normals.iter().all(|&n| n > 0),
        "not every AABB face normal was produced: {normals:?}"
    );
}

/// `c2RaytoCapsule` has five distinguishable outcomes: the two "ray origin is
/// already inside" early returns, the two end-cap circle casts, and the flat
/// side hit that writes `M.x` / `c2Skew(M.y)`.
#[test]
fn capsule_branches_are_reached() {
    let (c, r): (FnCapsule, FnCapsule) = syms(b"c2RaytoCapsule\0");
    let mut rng = Rng::new(203);

    let mut inside = 0usize; // returned 1 with t == 0
    let mut side_hit = 0usize; // returned 1 with t != 0 and a skewed normal
    let mut endcap_hit = 0usize; // returned 1 via c2RaytoCircle
    let mut miss = 0usize;

    for _ in 0..60_000 {
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
        assert_eq!(gc, gr);
        assert!(cast_eq(oc, or));

        if gc == 0 {
            miss += 1;
        } else if oc.t == 0.0 {
            inside += 1;
        } else {
            // A side hit keeps `out->t = t * A.t` with a normal derived from
            // the capsule axis; an end-cap hit gets its normal from
            // c2RaytoCircle, i.e. the impact point minus the cap centre.
            let axis_len =
                ((cap.b.x - cap.a.x).powi(2) + (cap.b.y - cap.a.y).powi(2)).sqrt();
            let nx = (cap.b.x - cap.a.x) / axis_len;
            let ny = (cap.b.y - cap.a.y) / axis_len;
            let is_axis_normal = (oc.n.x - ny).abs() < 1e-6 && (oc.n.y + nx).abs() < 1e-6
                || (oc.n.x + ny).abs() < 1e-6 && (oc.n.y - nx).abs() < 1e-6;
            if is_axis_normal {
                side_hit += 1;
            } else {
                endcap_hit += 1;
            }
        }
    }

    assert!(miss > 100, "too few capsule misses: {miss}");
    assert!(inside > 50, "the capsule early-return path was barely hit: {inside}");
    assert!(side_hit > 20, "the capsule side-hit path was barely hit: {side_hit}");
    assert!(
        endcap_hit > 20,
        "the capsule end-cap path was barely hit: {endcap_hit}"
    );
}

/// `c2RaytoCircle` must be seen both accepting (`0 <= t <= A.t`) and rejecting
/// (`disc < 0`, or `t` out of range) intersections.
#[test]
fn circle_branches_are_reached() {
    let (c, r): (FnCircle, FnCircle) = syms(b"c2RaytoCircle\0");
    let mut rng = Rng::new(201);
    let mut hits = 0usize;
    let mut misses = 0usize;

    for _ in 0..40_000 {
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
        assert_eq!(gc, gr);
        assert!(cast_eq(oc, or));
        if gc == 0 {
            misses += 1;
        } else {
            hits += 1;
        }
    }

    assert!(hits > 500, "too few circle hits: {hits}");
    assert!(misses > 500, "too few circle misses: {misses}");
}
