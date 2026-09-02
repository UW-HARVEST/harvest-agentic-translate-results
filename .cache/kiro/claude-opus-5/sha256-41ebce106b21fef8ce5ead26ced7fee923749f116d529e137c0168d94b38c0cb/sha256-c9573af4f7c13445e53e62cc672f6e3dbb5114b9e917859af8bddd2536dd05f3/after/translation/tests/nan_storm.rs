//! NaN / infinity storm.
//!
//! Rationale: several arithmetic sites inside the composite raycasts multiply
//! or add two *independently derived* values, e.g. `out->t = t * A.t` in
//! `c2RaytoAABB` and `y = yAp.y + (yAe.y - yAp.y) * t` in `c2RaytoCapsule`.
//! When both operands are NaN, the surviving sign/payload depends on which one
//! the compiler put in the SSE destination register, which is a codegen
//! decision, not a language one. The mostly-finite generators used elsewhere
//! reach those states only rarely, so this test drives every entry point with
//! inputs drawn EXCLUSIVELY from the pathological pool.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const ROUNDS: usize = 120_000;

#[test]
fn nan_storm_circle() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5A1701);
    unsafe {
        for _ in 0..ROUNDS {
            let A = c2Ray {
                p: rng.v_weird(),
                d: rng.v_weird(),
                t: rng.f_weird(),
            };
            let B = c2Circle { p: rng.v_weird(), r: rng.f_weird() };
            d.ray("nan-storm c2RaytoCircle", call_circle(&p.c, A, B), call_circle(&p.rs, A, B));
        }
    }
    d.finish("NaN storm: c2RaytoCircle");
}

#[test]
fn nan_storm_aabb() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5A1702);
    unsafe {
        for _ in 0..ROUNDS {
            let A = c2Ray {
                p: rng.v_weird(),
                d: rng.v_weird(),
                t: rng.f_weird(),
            };
            let B = c2AABB { min: rng.v_weird(), max: rng.v_weird() };
            d.ray("nan-storm c2RaytoAABB", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
        }
        // half-weird: finite geometry but a pathological A.t, which is exactly
        // the `out->t = tK * A.t` ambiguity
        for _ in 0..ROUNDS {
            let bx = rng.sym(6.0);
            let by = rng.sym(6.0);
            let B = c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v { x: bx + rng.range(0.0, 6.0), y: by + rng.range(0.0, 6.0) },
            };
            let A = c2Ray {
                p: if rng.bool() { rng.v_small() } else { rng.v_weird() },
                d: if rng.bool() { rng.v_dir() } else { rng.v_weird() },
                t: rng.f_weird(),
            };
            d.ray("nan-storm c2RaytoAABB(half)", call_aabb(&p.c, A, B), call_aabb(&p.rs, A, B));
        }
    }
    d.finish("NaN storm: c2RaytoAABB");
}

#[test]
fn nan_storm_capsule() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5A1703);
    unsafe {
        for _ in 0..ROUNDS {
            let A = c2Ray {
                p: rng.v_weird(),
                d: rng.v_weird(),
                t: rng.f_weird(),
            };
            let B = c2Capsule { a: rng.v_weird(), b: rng.v_weird(), r: rng.f_weird() };
            d.ray("nan-storm c2RaytoCapsule", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
        }
        // half-weird: a real capsule reached by a pathological ray, which is
        // what drives `y = yAp.y + (yAe.y - yAp.y) * t` into mixed NaN states
        for _ in 0..ROUNDS {
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.2, 12.0);
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
                r: if rng.bool() { rng.range(0.05, 5.0) } else { rng.f_weird() },
            };
            let A = c2Ray {
                p: if rng.bool() { rng.v_small() } else { rng.v_weird() },
                d: if rng.bool() { rng.v_dir() } else { rng.v_weird() },
                t: if rng.bool() { rng.range(0.0, 40.0) } else { rng.f_weird() },
            };
            d.ray("nan-storm c2RaytoCapsule(half)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
        }
    }
    d.finish("NaN storm: c2RaytoCapsule");
}

#[test]
fn nan_storm_poly() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5A1704);
    unsafe {
        for _ in 0..ROUNDS {
            let mut poly = c2Poly::default();
            poly.count = (1 + rng.below(8)) as i32;
            for i in 0..8 {
                poly.verts[i] = rng.v_weird();
                poly.norms[i] = rng.v_weird();
            }
            let A = c2Ray {
                p: rng.v_weird(),
                d: rng.v_weird(),
                t: rng.f_weird(),
            };
            let bx = c2x {
                p: rng.v_weird(),
                r: c2r { c: rng.f_weird(), s: rng.f_weird() },
            };
            for b in [None, Some(&bx)] {
                d.ray(
                    "nan-storm c2RaytoPoly",
                    call_poly(&p.c, A, &poly, b),
                    call_poly(&p.rs, A, &poly, b),
                );
            }
        }
        // half-weird: real polygon, pathological ray / transform
        for _ in 0..ROUNDS {
            let n = 1 + rng.below(8);
            let poly = make_convex_poly(&mut rng, n);
            let A = c2Ray {
                p: if rng.bool() { rng.v_small() } else { rng.v_weird() },
                d: if rng.bool() { rng.v_dir() } else { rng.v_weird() },
                t: if rng.bool() { rng.range(0.0, 40.0) } else { rng.f_weird() },
            };
            let bx = c2x {
                p: if rng.bool() { rng.v_small() } else { rng.v_weird() },
                r: if rng.bool() {
                    rng.rot_unit()
                } else {
                    c2r { c: rng.f_weird(), s: rng.f_weird() }
                },
            };
            for b in [None, Some(&bx)] {
                d.ray(
                    "nan-storm c2RaytoPoly(half)",
                    call_poly(&p.c, A, &poly, b),
                    call_poly(&p.rs, A, &poly, b),
                );
            }
        }
    }
    d.finish("NaN storm: c2RaytoPoly");
}

#[test]
fn nan_storm_dispatch() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x5A1705);
    unsafe {
        for _ in 0..ROUNDS {
            let A = c2Ray { p: rng.v_weird(), d: rng.v_weird(), t: rng.f_weird() };
            let bx = c2x {
                p: rng.v_weird(),
                r: c2r { c: rng.f_weird(), s: rng.f_weird() },
            };
            // One 132-byte buffer reinterpreted as each shape, so every arm
            // sees identical bytes in both languages.
            let mut poly = c2Poly::default();
            poly.count = (1 + rng.below(8)) as i32;
            for i in 0..8 {
                poly.verts[i] = rng.v_weird();
                poly.norms[i] = rng.v_weird();
            }
            let sp = &poly as *const c2Poly as *const c_void;
            for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, C2_TYPE_POLY] {
                for b in [None, Some(&bx)] {
                    d.ray(
                        &format!("nan-storm c2CastRay(type {ty})"),
                        call_cast(&p.c, A, sp, b, ty),
                        call_cast(&p.rs, A, sp, b, ty),
                    );
                }
            }
        }
    }
    d.finish("NaN storm: c2CastRay");
}

/// Exhaustive sweep of the weird pool through every leaf helper — every pair
/// (and, for the three-input helpers, a diagonal slice of every triple).
#[test]
fn nan_storm_exhaustive_leaves() {
    let p = load_pair();
    let mut d = Diff::new();
    unsafe {
        for &a0 in WEIRD {
            for &a1 in WEIRD {
                for &b0 in WEIRD {
                    let a = c2v { x: a0, y: a1 };
                    let b = c2v { x: b0, y: a0 };
                    d.scalar("c2Dot", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
                    d.vec("c2Add", (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
                    d.vec("c2Sub", (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
                    d.vec("c2Minv", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
                    d.vec("c2Maxv", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
                    d.vec("c2Mulvs", (p.c.c2Mulvs)(a, b0), (p.rs.c2Mulvs)(a, b0));
                    d.vec("c2Div", (p.c.c2Div)(a, b0), (p.rs.c2Div)(a, b0));
                    let r = c2r { c: a0, s: a1 };
                    d.vec("c2Mulrv", (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
                    d.vec("c2MulrvT", (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
                    let x = c2x { p: b, r };
                    d.vec("c2MulxvT", (p.c.c2MulxvT)(x, a), (p.rs.c2MulxvT)(x, a));
                    let m = c2m { x: a, y: b };
                    d.vec("c2MulmvT", (p.c.c2MulmvT)(m, b), (p.rs.c2MulmvT)(m, b));
                    d.vec("c2MulmvT(2)", (p.c.c2MulmvT)(m, a), (p.rs.c2MulmvT)(m, a));
                }
                let a = c2v { x: a0, y: a1 };
                d.scalar("c2Len", (p.c.c2Len)(a), (p.rs.c2Len)(a));
                d.vec("c2Norm", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
                d.vec("c2Skew", (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
                d.vec("c2Absv", (p.c.c2Absv)(a), (p.rs.c2Absv)(a));
                d.vec("c2CCW90", (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));
                d.vec("c2V", (p.c.c2V)(a0, a1), (p.rs.c2V)(a0, a1));
            }
        }
    }
    d.finish("NaN storm: exhaustive leaf sweep");
}
