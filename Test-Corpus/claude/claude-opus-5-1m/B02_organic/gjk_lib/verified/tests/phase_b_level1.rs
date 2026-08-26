//! Phase B — CONFIGS.md rows 14-22: pointer/buffer helpers.
//!
//! These use POISONED output buffers so the test proves the callee wrote
//! exactly the bytes the C wrote — no more, no less.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_void;

/// Row 14 — `c2BBVerts` into an 8-slot poisoned buffer (only 4 must be written).
#[test]
fn row14_bbverts() {
    let l = libs();
    let (c, r) = l.get::<FnBBVerts>("c2BBVerts");
    let mut g = Rng::new(0x1401);
    for i in 0..50_000 {
        let mut bb = match i % 5 {
            0 => AABB { min: g.v_coord(), max: g.v_coord() },
            1 => {
                // normalised
                let p = g.v_coord();
                let q = g.v_coord();
                AABB {
                    min: V::new(p.x.min(q.x), p.y.min(q.y)),
                    max: V::new(p.x.max(q.x), p.y.max(q.y)),
                }
            }
            2 => {
                let p = g.v_coord();
                AABB { min: p, max: p } // zero extent
            }
            3 => AABB { min: g.v_grid(), max: g.v_grid() },
            _ => AABB { min: g.v_mixed(), max: g.v_mixed() }, // NaN/Inf possible
        };
        let mut bb2 = bb;

        let mut co = poisoned_verts(8);
        let mut ro = poisoned_verts(8);
        unsafe {
            c(co.as_mut_ptr(), &mut bb);
            r(ro.as_mut_ptr(), &mut bb2);
        }
        for k in 0..8 {
            ck_v!("c2BBVerts out", co[k], ro[k], "i={i} k={k} bb={bb:?}");
        }
        // the input AABB must not be modified by either
        ck_bytes!("c2BBVerts input untouched", bb, bb2, "i={i}");
    }
}

/// Rows 15/16/17 — `c2MakeProxy` for each valid shape type, poisoned proxy.
#[test]
fn row15_16_17_makeproxy_valid_types() {
    let l = libs();
    let (c, r) = l.get::<FnMakeProxy>("c2MakeProxy");
    let mut g = Rng::new(0x1501);

    for i in 0..20_000 {
        // Row 15 — CIRCLE
        let circle = Circle {
            p: if i % 4 == 0 { g.v_mixed() } else { g.v_coord() },
            r: g.radius(),
        };
        let mut cp: Proxy = poisoned();
        let mut rp: Proxy = poisoned();
        unsafe {
            c(&circle as *const Circle as *const c_void, C2_TYPE_CIRCLE, &mut cp);
            r(&circle as *const Circle as *const c_void, C2_TYPE_CIRCLE, &mut rp);
        }
        ck_bytes!("c2MakeProxy CIRCLE", cp, rp, "i={i} circle={circle:?}");

        // Row 16 — AABB
        let bb = match i % 3 {
            0 => AABB { min: g.v_coord(), max: g.v_coord() },
            1 => {
                let p = g.v_coord();
                AABB { min: p, max: p }
            }
            _ => AABB { min: g.v_mixed(), max: g.v_mixed() },
        };
        let mut cp: Proxy = poisoned();
        let mut rp: Proxy = poisoned();
        unsafe {
            c(&bb as *const AABB as *const c_void, C2_TYPE_AABB, &mut cp);
            r(&bb as *const AABB as *const c_void, C2_TYPE_AABB, &mut rp);
        }
        ck_bytes!("c2MakeProxy AABB", cp, rp, "i={i} bb={bb:?}");

        // Row 17 — CAPSULE
        let cap = Capsule {
            a: if i % 5 == 0 { g.v_mixed() } else { g.v_coord() },
            b: if i % 7 == 0 { g.v_mixed() } else { g.v_coord() },
            r: g.radius(),
        };
        let mut cp: Proxy = poisoned();
        let mut rp: Proxy = poisoned();
        unsafe {
            c(&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE, &mut cp);
            r(&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE, &mut rp);
        }
        ck_bytes!("c2MakeProxy CAPSULE", cp, rp, "i={i} cap={cap:?}");
    }
}

/// Rows 18-21 — `c2Support` for counts 1, 2, 4 and 8.
#[test]
fn row18_21_support_counts() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    let mut g = Rng::new(0x1801);

    for &count in &[1i32, 2, 4, 8] {
        for i in 0..50_000 {
            let mut verts = [V::default(); 8];
            for v in verts.iter_mut() {
                *v = match i % 4 {
                    0 => g.v_coord(),
                    1 => g.v_grid(),
                    2 => g.v_mixed(),
                    _ => g.v_coord(),
                };
            }
            let d = match i % 6 {
                0 => V::new(1.0, 0.0),
                1 => V::new(0.0, 1.0),
                2 => V::new(-1.0, 0.0),
                3 => V::new(0.0, -1.0),
                4 => g.v_coord(),
                _ => g.v_mixed(),
            };
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            ck_i32!("c2Support", cv, rv, "count={count} i={i} d={d:?} verts={:?}", &verts[..count.max(1) as usize]);
        }
    }
}

/// Row 22 — `c2Support` with duplicated verts, forcing exact `dot == dmax` ties.
#[test]
fn row22_support_ties() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    let mut g = Rng::new(0x2201);
    for i in 0..20_000 {
        // Build a vert set with heavy duplication so ties are the common case.
        let base = [g.v_grid(), g.v_grid(), g.v_grid()];
        let mut verts = [V::default(); 8];
        for (k, v) in verts.iter_mut().enumerate() {
            *v = base[k % 3];
        }
        for &count in &[2i32, 3, 4, 8] {
            for d in [
                V::new(1.0, 0.0),
                V::new(0.0, 1.0),
                V::new(1.0, 1.0),
                V::new(-1.0, -1.0),
                V::new(0.0, 0.0),
                g.v_grid(),
            ] {
                let cv = unsafe { c(verts.as_ptr(), count, d) };
                let rv = unsafe { r(verts.as_ptr(), count, d) };
                ck_i32!("c2Support/tie", cv, rv, "i={i} count={count} d={d:?} verts={verts:?}");
            }
        }
        // all-identical verts: every dot equal -> index 0 must win
        let same = g.v_grid();
        let verts2 = [same; 8];
        for &count in &[1i32, 2, 4, 8] {
            let d = g.v_grid();
            let cv = unsafe { c(verts2.as_ptr(), count, d) };
            let rv = unsafe { r(verts2.as_ptr(), count, d) };
            ck_i32!("c2Support/allsame", cv, rv, "i={i} count={count} d={d:?}");
        }
    }
}
