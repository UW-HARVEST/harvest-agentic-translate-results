//! Exhaustive small-grid sweeps.
//!
//! Random sampling can miss the exact-equality branches that dominate this
//! code (`den == 0`, `d != 0`, `t0 >= t1 && t0 >= t2 && t0 >= t3`,
//! `t <= A.t`, `y >= yBb.y`, ...). These tests enumerate every combination
//! over small integer/half-integer grids so those branches are guaranteed to
//! be exercised on both sides.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_int, c_void};

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

/// Coordinate grid: negatives, zero (and -0.0), halves, small integers.
const COORDS: [f32; 9] = [-2.0, -1.0, -0.5, -0.0, 0.0, 0.5, 1.0, 2.0, 3.0];
/// Direction grid, including the degenerate zero direction.
const DIRS: [f32; 5] = [-1.0, -0.5, 0.0, 0.5, 1.0];
/// Ray lengths, including 0 and a negative (which the C code does not reject).
const TS: [f32; 5] = [0.0, 0.5, 1.0, 4.0, -1.0];

fn grid_vecs(vals: &[f32]) -> Vec<c2v> {
    let mut v = Vec::with_capacity(vals.len() * vals.len());
    for &x in vals {
        for &y in vals {
            v.push(c2v { x, y });
        }
    }
    v
}

#[test]
fn c2RaytoAABB_exhaustive_grid() {
    let (c, r) = libs().sym::<FnRayAABB_i>("c2RaytoAABB");
    let ps = grid_vecs(&COORDS);
    let ds = grid_vecs(&DIRS);
    // A handful of boxes, including degenerate and inverted ones.
    let boxes = [
        (-1.0, -1.0, 1.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (-1.0, -1.0, 0.0, 0.0),
        (0.0, 0.0, 0.0, 0.0),   // degenerate point
        (1.0, 1.0, -1.0, -1.0), // inverted
        (-2.0, -0.5, 2.0, 0.5), // wide
        (-0.5, -2.0, 0.5, 2.0), // tall
    ];

    let mut n = 0u64;
    for p in &ps {
        for d in &ds {
            for &t in &TS {
                let A = c2Ray { p: *p, d: *d, t };
                for &(mnx, mny, mxx, mxy) in &boxes {
                    let B = c2AABB {
                        min: c2v { x: mnx, y: mny },
                        max: c2v { x: mxx, y: mxy },
                    };
                    let mut o1 = poison();
                    let mut o2 = poison();
                    let r1 = unsafe { c(A, B, &mut o1) };
                    let r2 = unsafe { r(A, B, &mut o2) };
                    cmp(
                        "c2RaytoAABB",
                        &format!("{A:?} {B:?}"),
                        (r1, o1),
                        (r2, o2),
                    );
                    n += 1;
                }
            }
        }
    }
    assert!(n > 50_000, "grid too small: {n}");
}

#[test]
fn c2RaytoCircle_exhaustive_grid() {
    let (c, r) = libs().sym::<FnRayCircle_i>("c2RaytoCircle");
    let ps = grid_vecs(&COORDS);
    let ds = grid_vecs(&DIRS);
    let centres = grid_vecs(&[-1.0, 0.0, 1.0]);
    let radii = [0.0, -0.0, 0.5, 1.0, 2.0, -1.0];

    for p in &ps {
        for d in &ds {
            for &t in &TS {
                let A = c2Ray { p: *p, d: *d, t };
                for cp in &centres {
                    for &rad in &radii {
                        let B = c2Circle { p: *cp, r: rad };
                        let mut o1 = poison();
                        let mut o2 = poison();
                        let r1 = unsafe { c(A, B, &mut o1) };
                        let r2 = unsafe { r(A, B, &mut o2) };
                        cmp("c2RaytoCircle", &format!("{A:?} {B:?}"), (r1, o1), (r2, o2));
                    }
                }
            }
        }
    }
}

#[test]
fn c2RaytoCapsule_exhaustive_grid() {
    let (c, r) = libs().sym::<FnRayCapsule_i>("c2RaytoCapsule");
    let ps = grid_vecs(&COORDS);
    let ds = grid_vecs(&DIRS);
    // Vertical, horizontal, diagonal, reversed and degenerate capsules.
    let caps = [
        ((0.0, -1.0), (0.0, 1.0)),
        ((0.0, 1.0), (0.0, -1.0)),
        ((-1.0, 0.0), (1.0, 0.0)),
        ((-1.0, -1.0), (1.0, 1.0)),
        ((0.0, 0.0), (0.0, 0.0)), // degenerate -> NaN axis
        ((0.0, 0.0), (0.0, 2.0)),
    ];
    let radii = [0.0, 0.5, 1.0, -1.0];

    for p in &ps {
        for d in &ds {
            for &t in &TS {
                let A = c2Ray { p: *p, d: *d, t };
                for &((ax, ay), (bx, by)) in &caps {
                    for &rad in &radii {
                        let B = c2Capsule {
                            a: c2v { x: ax, y: ay },
                            b: c2v { x: bx, y: by },
                            r: rad,
                        };
                        let mut o1 = poison();
                        let mut o2 = poison();
                        let r1 = unsafe { c(A, B, &mut o1) };
                        let r2 = unsafe { r(A, B, &mut o2) };
                        cmp(
                            "c2RaytoCapsule",
                            &format!("{A:?} {B:?}"),
                            (r1, o1),
                            (r2, o2),
                        );
                    }
                }
            }
        }
    }
}

fn box_poly(hw: f32, hh: f32) -> c2Poly {
    let mut p = c2Poly::default();
    p.verts[0] = c2v { x: hw, y: -hh };
    p.verts[1] = c2v { x: hw, y: hh };
    p.verts[2] = c2v { x: -hw, y: hh };
    p.verts[3] = c2v { x: -hw, y: -hh };
    p.norms[0] = c2v { x: 1.0, y: 0.0 };
    p.norms[1] = c2v { x: 0.0, y: 1.0 };
    p.norms[2] = c2v { x: -1.0, y: 0.0 };
    p.norms[3] = c2v { x: 0.0, y: -1.0 };
    p.count = 4;
    p
}

fn tri_poly() -> c2Poly {
    let mut p = c2Poly::default();
    p.count = 3;
    p.verts[0] = c2v { x: 1.0, y: -1.0 };
    p.verts[1] = c2v { x: 0.0, y: 1.0 };
    p.verts[2] = c2v { x: -1.0, y: -1.0 };
    p.norms[0] = c2v { x: 0.5, y: 0.5 };
    p.norms[1] = c2v { x: -0.5, y: 0.5 };
    p.norms[2] = c2v { x: 0.0, y: -1.0 };
    p
}

#[test]
fn c2RaytoPoly_exhaustive_grid() {
    let (c, r) = libs().sym::<FnRayPoly_i>("c2RaytoPoly");
    let ps = grid_vecs(&COORDS);
    let ds = grid_vecs(&DIRS);

    let polys = [
        box_poly(1.0, 1.0),
        box_poly(0.875, 11.5),
        box_poly(0.0, 0.0), // degenerate
        tri_poly(),
    ];
    // Null, identity, translated, 90-degree rotation, mirrored, garbage.
    let xforms = [
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: 1.0, y: -1.0 },
            r: c2r { c: 1.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 0.0, s: 1.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 0.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: -2.0, y: 2.0 },
            r: c2r { c: -1.0, s: 0.0 },
        },
    ];

    for p in &ps {
        for d in &ds {
            for &t in &TS {
                let A = c2Ray { p: *p, d: *d, t };
                for poly in &polys {
                    // null transform
                    let mut o1 = poison();
                    let mut o2 = poison();
                    let r1 = unsafe {
                        c(A, poly as *const c2Poly, std::ptr::null(), &mut o1)
                    };
                    let r2 = unsafe {
                        r(A, poly as *const c2Poly, std::ptr::null(), &mut o2)
                    };
                    cmp("c2RaytoPoly/null-bx", &format!("{A:?}"), (r1, o1), (r2, o2));

                    for bx in &xforms {
                        let mut o1 = poison();
                        let mut o2 = poison();
                        let r1 = unsafe {
                            c(A, poly as *const c2Poly, bx as *const c2x, &mut o1)
                        };
                        let r2 = unsafe {
                            r(A, poly as *const c2Poly, bx as *const c2x, &mut o2)
                        };
                        cmp("c2RaytoPoly", &format!("{A:?} {bx:?}"), (r1, o1), (r2, o2));
                    }
                }
            }
        }
    }
}

#[test]
fn c2CastRay_exhaustive_grid() {
    let (c, r) = libs().sym::<FnCastRay_i>("c2CastRay");
    let ps = grid_vecs(&COORDS);
    let ds = grid_vecs(&DIRS);

    let circle = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let aabb = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let capsule = c2Capsule {
        a: c2v { x: 0.0, y: -1.0 },
        b: c2v { x: 0.0, y: 1.0 },
        r: 0.5,
    };
    let poly = box_poly(1.0, 1.0);
    let bx = c2x {
        p: c2v { x: 0.5, y: -0.5 },
        r: c2r { c: 0.0, s: 1.0 },
    };

    let shapes: [(c_int, *const c_void); 4] = [
        (C2_TYPE_CIRCLE, &circle as *const _ as *const c_void),
        (C2_TYPE_AABB, &aabb as *const _ as *const c_void),
        (C2_TYPE_CAPSULE, &capsule as *const _ as *const c_void),
        (C2_TYPE_POLY, &poly as *const _ as *const c_void),
    ];

    for p in &ps {
        for d in &ds {
            for &t in &TS {
                let A = c2Ray { p: *p, d: *d, t };
                for &(ty, ptr) in &shapes {
                    for bp in [&bx as *const c2x, std::ptr::null()] {
                        let mut o1 = poison();
                        let mut o2 = poison();
                        let r1 = unsafe { c(A, ptr, bp, ty, &mut o1) };
                        let r2 = unsafe { r(A, ptr, bp, ty, &mut o2) };
                        cmp(
                            "c2CastRay",
                            &format!("type {ty} {A:?}"),
                            (r1, o1),
                            (r2, o2),
                        );
                    }
                }
            }
        }
    }
}
