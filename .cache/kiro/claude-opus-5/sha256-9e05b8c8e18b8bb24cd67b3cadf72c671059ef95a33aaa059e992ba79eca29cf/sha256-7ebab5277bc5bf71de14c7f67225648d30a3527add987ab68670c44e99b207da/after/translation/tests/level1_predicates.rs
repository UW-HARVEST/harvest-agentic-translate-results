//! Level 1: the integer-returning predicates
//! (`c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`).
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn c2AABBtoAABB_matches() {
    type F = extern "C" fn(C2AABB, C2AABB) -> c_int;
    let (c, r): (F, F) = syms(b"c2AABBtoAABB\0");
    let mut rng = Rng::new(101);
    for i in 0..iters(50_000) {
        let (a, b) = (rng.aabb(), rng.aabb());
        let (gc, gr) = (c(a, b), r(a, b));
        assert_eq!(
            gc, gr,
            "c2AABBtoAABB mismatch at iter {i}\n  A: {a:?}\n  B: {b:?}"
        );
    }
}

#[test]
fn c2AABBtoPoint_matches() {
    type F = extern "C" fn(C2AABB, C2v) -> c_int;
    let (c, r): (F, F) = syms(b"c2AABBtoPoint\0");
    let mut rng = Rng::new(102);
    for i in 0..iters(50_000) {
        let a = rng.aabb();
        // Half the points are snapped onto a box corner/edge so the strict
        // `<` / `>` boundaries in the C code are exercised.
        let b = match rng.below(4) {
            0 => C2v { x: a.min.x, y: a.min.y },
            1 => C2v { x: a.max.x, y: a.max.y },
            2 => C2v { x: a.min.x, y: a.max.y },
            _ => rng.vec(),
        };
        let (gc, gr) = (c(a, b), r(a, b));
        assert_eq!(
            gc, gr,
            "c2AABBtoPoint mismatch at iter {i}\n  A: {a:?}\n  B: {}",
            show_v(b)
        );
    }
}

#[test]
fn c2CircleToPoint_matches() {
    type F = extern "C" fn(C2Circle, C2v) -> c_int;
    let (c, r): (F, F) = syms(b"c2CircleToPoint\0");
    let mut rng = Rng::new(103);
    for i in 0..iters(50_000) {
        let a = rng.circle();
        // Sometimes place the point exactly on the centre or on the circle's
        // axis at radius distance, where `d2 < r*r` is a tie.
        let b = match rng.below(4) {
            0 => a.p,
            1 => C2v { x: a.p.x + a.r, y: a.p.y },
            2 => C2v { x: a.p.x, y: a.p.y - a.r },
            _ => rng.vec(),
        };
        let (gc, gr) = (c(a, b), r(a, b));
        assert_eq!(
            gc, gr,
            "c2CircleToPoint mismatch at iter {i}\n  A: {a:?}\n  B: {}",
            show_v(b)
        );
    }
}

#[test]
fn predicate_edge_values_match() {
    let edges: [f32; 8] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
    ];

    type FAabb = extern "C" fn(C2AABB, C2AABB) -> c_int;
    type FPoint = extern "C" fn(C2AABB, C2v) -> c_int;
    type FCircle = extern "C" fn(C2Circle, C2v) -> c_int;
    let (aa_c, aa_r): (FAabb, FAabb) = syms(b"c2AABBtoAABB\0");
    let (ap_c, ap_r): (FPoint, FPoint) = syms(b"c2AABBtoPoint\0");
    let (cp_c, cp_r): (FCircle, FCircle) = syms(b"c2CircleToPoint\0");

    for &a in &edges {
        for &b in &edges {
            for &d in &edges {
                let box1 = C2AABB {
                    min: C2v { x: a, y: b },
                    max: C2v { x: b, y: d },
                };
                let box2 = C2AABB {
                    min: C2v { x: d, y: a },
                    max: C2v { x: a, y: b },
                };
                assert_eq!(
                    aa_c(box1, box2),
                    aa_r(box1, box2),
                    "c2AABBtoAABB mismatch for {box1:?} {box2:?}"
                );
                let p = C2v { x: d, y: b };
                assert_eq!(
                    ap_c(box1, p),
                    ap_r(box1, p),
                    "c2AABBtoPoint mismatch for {box1:?} {}",
                    show_v(p)
                );
                let circle = C2Circle {
                    p: C2v { x: a, y: b },
                    r: d,
                };
                assert_eq!(
                    cp_c(circle, p),
                    cp_r(circle, p),
                    "c2CircleToPoint mismatch for {circle:?} {}",
                    show_v(p)
                );
            }
        }
    }
}
