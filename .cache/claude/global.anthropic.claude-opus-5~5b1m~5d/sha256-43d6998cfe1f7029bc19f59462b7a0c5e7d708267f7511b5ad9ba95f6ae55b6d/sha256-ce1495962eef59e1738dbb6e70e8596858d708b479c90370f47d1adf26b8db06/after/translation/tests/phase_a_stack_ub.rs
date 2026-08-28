//! Phase A — executable documentation of the one place where the C's behaviour
//! is *not* a function of its inputs, and of the harness invariant that makes
//! the differential comparison well defined again.
//!
//! `c2MakeProxy` (lib.c:125-147) has **no `case C2_TYPE_POLY`**, so `c2GJK`'s two
//! `c2Proxy` locals stay uninitialised whenever a poly — or any out-of-range
//! type — is passed.  That path is reachable from the public API:
//!
//! ```text
//! omni_manifold(AABB, CAPSULE) -> c2Collide -> c2AABBtoCapsuleManifold
//!   -> c2CapsuletoPolyManifold -> c2GJK(..., C2_TYPE_POLY, ...)
//! ```
//!
//! In a normal C program those stack bytes are the kernel's fresh zero pages, so
//! the library behaves as if the proxy were `{radius: 0, count: 0, verts: 0}` —
//! which is exactly what the Rust translation materialises with
//! `std::mem::zeroed()`.  Verified against a standalone C `main()` linked to the
//! same `.so`: for the AABB/CAPSULE case that program prints the Rust's bytes
//! exactly.
//!
//! Inside a `libloading` harness the stack at that depth is *dirty* (it holds
//! leftover pointers whose values move with ASLR), which made the C's UB read
//! non-deterministic run to run.  `common::scrub_stack()` zeroes 32 KiB below
//! the frame immediately before every FFI call, restoring the pristine
//! condition.  This file asserts that invariant holds — if it ever regresses,
//! every poly-path test in the suite would start reporting phantom divergences.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

/// With a 1-vertex proxy for A and `C2_TYPE_POLY` for B, the loop terminates at
/// `count == 1` and `*outB` is literally `pB.verts[0]` — i.e. it *reveals* the
/// uninitialised bytes.  After scrubbing they must read as `+0.0, +0.0`.
#[test]
fn stack_is_normalised_to_zero_before_ffi() {
    let poly = make_poly(&[c2v { x: 0.0, y: 0.0 }; 8], 4);
    let ca = c2Circle {
        p: c2v { x: 100.0, y: 100.0 },
        r: 0.0,
    };
    for rep in 0..64 {
        let args = GjkArgs::default();
        let o = run_gjk_raw(
            Side::C,
            &ca as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &poly as *const c2Poly as *const c_void,
            C2_TYPE_POLY,
            &args,
        );
        assert_eq!(
            (o.b.x.to_bits(), o.b.y.to_bits()),
            (0, 0),
            "rep {rep}: the C read a NON-zero uninitialised proxy vertex ({}). \
             scrub_stack() is no longer the last thing before the FFI call — see \
             the module docs.",
            o.b.show()
        );
        let r = run_gjk_raw(
            Side::Rust,
            &ca as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &poly as *const c2Poly as *const c_void,
            C2_TYPE_POLY,
            &args,
        );
        assert!(o.bit_eq(&r), "C {} vs Rust {}", o.show(), r.show());
    }
}

/// The whole poly path, driven the way a consumer does, must be divergence-free
/// over a large randomized sweep (this is the regression guard for the
/// zeroed-proxy modelling in `c2GJK`).
#[test]
fn poly_path_is_divergence_free() {
    let mut rng = Rng::new(0xf00d_0001);
    let mut acc = DiffAccum::new("poly_path_is_divergence_free");
    for i in 0..20000 {
        let count = 1 + rng.below(8) as c_int;
        let verts = rng.convex_poly_verts(count as usize);
        let poly = make_poly(&verts, count);
        let cap = rng.capsule();
        let bx = if rng.bool() { Some(rng.xform()) } else { None };
        acc.check(format!("capsule/poly #{i}"), |s| {
            let bxp = match &bx {
                Some(x) => x as *const c2x,
                None => std::ptr::null(),
            };
            with_sentinel(|m| c2CapsuletoPolyManifold(s, cap, &poly, bxp, m))
        });
        let bb = rng.aabb();
        acc.check(format!("aabb/capsule #{i}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, bb, cap, m))
        });
        let (a1, a2, a3, a4, _) = Shape::Bb(bb).parts();
        let (b1, b2, b3, b4, b5) = Shape::Ca(cap).parts();
        acc.check(format!("omni aabb/capsule #{i}"), |s| {
            with_sentinel(|m| {
                omni_manifold(
                    s,
                    m,
                    C2_TYPE_AABB,
                    a1,
                    a2,
                    a3,
                    a4,
                    0.0,
                    C2_TYPE_CAPSULE,
                    b1,
                    b2,
                    b3,
                    b4,
                    b5,
                )
            })
        });
        acc.check(format!("omni capsule/aabb #{i}"), |s| {
            with_sentinel(|m| {
                omni_manifold(
                    s,
                    m,
                    C2_TYPE_CAPSULE,
                    b1,
                    b2,
                    b3,
                    b4,
                    b5,
                    C2_TYPE_AABB,
                    a1,
                    a2,
                    a3,
                    a4,
                    0.0,
                )
            })
        });
    }
    acc.finish();
}

/// Records the reachable range of `c2GJK`'s iteration counter, so that the
/// `iter < 20` claim in CONFIGS.md row 78 stays honest.  The largest proxy
/// `c2MakeProxy` builds has 4 vertices (AABB), so the duplicate-support test
/// always fires within a handful of iterations.
#[test]
fn gjk_iteration_bound_is_four() {
    let mut rng = Rng::new(0xf00d_0002);
    let mut hist = [0usize; 25];
    for _ in 0..200000 {
        let ka = rng.below(3);
        let kb = rng.below(3);
        let sa = if rng.bool() {
            rng.nice_shape(ka)
        } else {
            rng.shape(ka)
        };
        let sb = if rng.bool() {
            rng.nice_shape(kb)
        } else {
            rng.shape(kb)
        };
        let cache = if rng.bool() {
            Some(c2GJKCache {
                metric: rng.special(),
                count: 1 + rng.below(3) as c_int,
                iA: [
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                ],
                iB: [
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                ],
                div: rng.coord(),
            })
        } else {
            None
        };
        let args = GjkArgs {
            ax: if rng.bool() { Some(rng.xform()) } else { None },
            bx: if rng.bool() { Some(rng.xform()) } else { None },
            use_radius: rng.below(2) as c_int,
            cache,
            ..Default::default()
        };
        let o = run_gjk(Side::C, &sa, &sb, &args);
        hist[o.iter.clamp(0, 24) as usize] += 1;
    }
    eprintln!("gjk_iteration_bound_is_four: histogram = {hist:?}");
    for k in 0..=4 {
        assert!(hist[k] > 0, "iteration count {k} unreachable: {hist:?}");
    }
    assert!(
        hist[5..].iter().all(|&n| n == 0),
        "iteration count > 4 IS reachable — CONFIGS.md row 78 needs updating: {hist:?}"
    );
}
