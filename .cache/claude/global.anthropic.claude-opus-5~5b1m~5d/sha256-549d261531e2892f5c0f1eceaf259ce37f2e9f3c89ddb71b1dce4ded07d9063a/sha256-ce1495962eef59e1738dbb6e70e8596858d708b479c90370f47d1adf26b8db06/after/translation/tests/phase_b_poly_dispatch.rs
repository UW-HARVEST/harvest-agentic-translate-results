//! Phase B — valid-path differential tests for `c2RaytoPoly`, the `c2CastRay`
//! dispatcher, and the `poly_ray` one-shot wrapper.
//! Covers `CONFIGS.md` rows 34–41.
//!
//! `c2RaytoPoly` is driven through an explicit 512-byte backing buffer
//! (`PolyBuf`) rather than a bare `c2Poly`, so that when `count > 8` makes the
//! C index past `verts[8]`/`norms[8]` both libraries read *byte-identical*
//! memory and the comparison stays meaningful.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 4096;

macro_rules! cmp_poly {
    ($l:expr, $ctx:expr, $a:expr, $buf:expr, $bx:expr) => {{
        let a = $a;
        let buf = $buf;
        let bx = $bx;
        let cr = run_poly_raw(&$l.c, a, buf, bx);
        let rr = run_poly_raw(&$l.rs, a, buf, bx);
        assert!(
            cr == rr,
            "DIVERGENCE [{}]\n  ray   = p={} d={} t={}\n  count = {}\n  bx    = {:?}\n  C    = {:?}\n  RUST = {:?}",
            $ctx,
            showv(a.p),
            showv(a.d),
            show(a.t),
            unsafe { (*buf.as_ptr()).count },
            bx.map(|x| xb(*x)),
            cr,
            rr
        );
    }};
}

macro_rules! cmp_cast {
    ($l:expr, $ctx:expr, $a:expr, $shape:expr, $bx:expr, $ty:expr) => {{
        let a = $a;
        let shape = $shape;
        let bx = $bx;
        let ty = $ty;
        let cr = run_cast(&$l.c, a, shape, bx, ty);
        let rr = run_cast(&$l.rs, a, shape, bx, ty);
        assert!(
            cr == rr,
            "DIVERGENCE [{}]\n  ray = p={} d={} t={}\n  typeB = {}\n  bx = {:?}\n  C    = {:?}\n  RUST = {:?}",
            $ctx, showv(a.p), showv(a.d), show(a.t), ty, bx.map(|x| xb(*x)), cr, rr
        );
    }};
}

// ===========================================================================
// Row 34 — c2RaytoPoly, bx == NULL, axis-aligned box poly
// ===========================================================================

#[test]
fn row34_c2RaytoPoly_box_null_transform() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 34);
    let mut hits = 0usize;
    for i in 0..(N * 4) {
        let p = box_poly(&mut rng);
        let buf = PolyBuf::from_poly(&p);
        let a = sane_ray(&mut rng);
        if run_poly_raw(&l.c, a, &buf, None).ret != 0 {
            hits += 1;
        }
        cmp_poly!(l, format!("row34 rand #{i}"), a, &buf, None);
    }
    assert!(hits > 256, "row34 only produced {hits} hits");

    // The exact `poly_ray` box, swept finely so faces, corners, and the
    // origin-inside case are all reached.
    let p = poly_ray_box();
    let buf = PolyBuf::from_poly(&p);
    for i in 0..2048 {
        let y = -14.0 + 28.0 * (i as f32) / 2048.0;
        for &d in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 1.0, y: 1.0 },
        ] {
            for &t in &[0.0f32, 4.0, 40.0, -4.0, f32::INFINITY] {
                cmp_poly!(
                    l,
                    format!("row34 sweep y={} t={}", show(y), show(t)),
                    c2Ray {
                        p: c2v { x: -3.869416, y },
                        d,
                        t
                    },
                    &buf,
                    None
                );
                cmp_poly!(
                    l,
                    format!("row34 sweepx x={} t={}", show(y), show(t)),
                    c2Ray {
                        p: c2v { x: y, y: 13.0693407 },
                        d,
                        t
                    },
                    &buf,
                    None
                );
            }
        }
    }
    // Exactly on each face / vertex of the box.
    for &pt in &[
        c2v { x: 0.875, y: 0.0 },
        c2v { x: -0.875, y: 0.0 },
        c2v { x: 0.0, y: 11.5 },
        c2v { x: 0.0, y: -11.5 },
        c2v { x: 0.875, y: 11.5 },
        c2v { x: -0.875, y: -11.5 },
        c2v { x: 0.0, y: 0.0 },
    ] {
        for &d in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
        ] {
            for &t in &[0.0f32, 1.0, 4.0, 100.0] {
                cmp_poly!(l, "row34 exact", c2Ray { p: pt, d, t }, &buf, None);
            }
        }
    }
}

// ===========================================================================
// Row 35 — every count in 1..=8, convex n-gon, bx == NULL
// ===========================================================================

#[test]
fn row35_c2RaytoPoly_all_counts_1_to_8() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 35);
    for count in 1..=8i32 {
        let mut hits = 0usize;
        for i in 0..(N * 2) {
            let p = convex_ngon(&mut rng, count);
            let buf = PolyBuf::from_poly(&p);
            let a = sane_ray(&mut rng);
            if run_poly_raw(&l.c, a, &buf, None).ret != 0 {
                hits += 1;
            }
            cmp_poly!(l, format!("row35 count={count} #{i}"), a, &buf, None);
        }
        // count == 1 can still hit (one back-facing plane is enough to set
        // `index`), so require *some* hits for every count.
        assert!(
            hits > 0,
            "row35 count={count} produced no hits at all — generator problem"
        );
    }
    // Gridded rays over gridded polygons, which makes `den == 0` and exact
    // `num == lo*den` ties far more likely.
    for count in 1..=8i32 {
        for i in 0..(N * 2) {
            let mut p = convex_ngon(&mut rng, count);
            for v in p.verts.iter_mut() {
                v.x = v.x.round();
                v.y = v.y.round();
            }
            for n in p.norms.iter_mut() {
                n.x = n.x.round();
                n.y = n.y.round();
            }
            let buf = PolyBuf::from_poly(&p);
            let a = c2Ray {
                p: rng.vec_grid(10),
                d: rng.vec_grid(2),
                t: rng.gridded(10),
            };
            cmp_poly!(l, format!("row35 grid count={count} #{i}"), a, &buf, None);
        }
    }
}

// ===========================================================================
// Row 36 — bx != NULL: identity, translation, rotation, both, non-unit, zero
// ===========================================================================

#[test]
fn row36_c2RaytoPoly_transforms() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 36);
    let ident = (l.rs.c2xIdentity)();

    // An identity-valued non-NULL bx must behave exactly like bx == NULL.
    for count in 3..=8i32 {
        for i in 0..512 {
            let p = convex_ngon(&mut rng, count);
            let buf = PolyBuf::from_poly(&p);
            let a = sane_ray(&mut rng);
            cmp_poly!(l, format!("row36 ident count={count} #{i}"), a, &buf, Some(&ident));
            let with_null = run_poly_raw(&l.c, a, &buf, None);
            let with_ident = run_poly_raw(&l.c, a, &buf, Some(&ident));
            assert!(
                with_null == with_ident,
                "row36: NULL bx and identity bx disagree in the C: {with_null:?} vs {with_ident:?}"
            );
        }
    }

    let transforms: Vec<c2x> = {
        let mut v = vec![
            ident,
            c2x {
                p: c2v { x: 3.0, y: -2.0 },
                r: c2r { c: 1.0, s: 0.0 },
            }, // pure translation
            c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 2.0, s: -3.0 },
            }, // non-normalised
            c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 0.0, s: 0.0 },
            }, // zero rotation
            c2x {
                p: c2v { x: -0.0, y: -0.0 },
                r: c2r { c: -0.0, s: -0.0 },
            },
        ];
        // Pure rotations over 64 angles + rotation with translation.
        for k in 0..64u32 {
            let ang = std::f32::consts::TAU * (k as f32) / 64.0;
            v.push(c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r {
                    c: ang.cos(),
                    s: ang.sin(),
                },
            });
            v.push(c2x {
                p: c2v {
                    x: (k as f32) * 0.25 - 8.0,
                    y: 4.0 - (k as f32) * 0.125,
                },
                r: c2r {
                    c: ang.cos(),
                    s: ang.sin(),
                },
            });
        }
        v
    };

    for count in 3..=8i32 {
        for (xi, bx) in transforms.iter().enumerate() {
            for i in 0..16 {
                let p = convex_ngon(&mut rng, count);
                let buf = PolyBuf::from_poly(&p);
                let a = sane_ray(&mut rng);
                cmp_poly!(
                    l,
                    format!("row36 count={count} x{xi} #{i}"),
                    a,
                    &buf,
                    Some(bx)
                );
            }
        }
    }
    // Fully arbitrary transforms including non-finite components.
    for i in 0..(N * 4) {
        let count = 1 + (rng.below(8) as i32);
        let p = convex_ngon(&mut rng, count);
        let buf = PolyBuf::from_poly(&p);
        let bx = rng.any_x();
        let a = any_ray(&mut rng);
        cmp_poly!(l, format!("row36 wild-x #{i}"), a, &buf, Some(&bx));
    }
}

// ===========================================================================
// Row 37 — out-of-range `count`: <= 0 and 9..=16 (out-of-bounds indexing)
// ===========================================================================

#[test]
fn row37_c2RaytoPoly_out_of_range_count() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 37);

    // Non-positive counts: the loop body never runs.
    for &count in &[0i32, -1, -2, -8, -100, i32::MIN, i32::MIN + 1] {
        for i in 0..256 {
            let mut p = convex_ngon(&mut rng, 8);
            p.count = count;
            let buf = PolyBuf::from_poly(&p);
            let a = any_ray(&mut rng);
            cmp_poly!(l, format!("row37 count={count} #{i}"), a, &buf, None);
            let bx = rng.any_x();
            cmp_poly!(l, format!("row37 count={count} bx #{i}"), a, &buf, Some(&bx));
        }
    }

    // Counts 9..=16 read past `verts[8]` / `norms[8]`. Both libraries index the
    // same 512-byte buffer, so the out-of-bounds bytes are identical.
    for count in 9..=16i32 {
        for i in 0..1024 {
            let mut p = convex_ngon(&mut rng, 8);
            p.count = count;
            let buf = PolyBuf::from_poly(&p);
            let a = sane_ray(&mut rng);
            cmp_poly!(l, format!("row37 oob count={count} #{i}"), a, &buf, None);
            let bx = rng.any_x();
            cmp_poly!(
                l,
                format!("row37 oob count={count} bx #{i}"),
                a,
                &buf,
                Some(&bx)
            );
        }
        // And with a *known* tail, so the out-of-range normals are ordinary
        // finite unit vectors rather than garbage — a different code path
        // through the lo/hi updates.
        let tail: Vec<f32> = (0..64)
            .map(|k| match k % 4 {
                0 => 1.0,
                1 => 0.0,
                2 => -1.0,
                _ => 0.5,
            })
            .collect();
        for i in 0..512 {
            let mut p = convex_ngon(&mut rng, 8);
            p.count = count;
            let buf = PolyBuf::from_poly_with_tail(&p, &tail);
            let a = sane_ray(&mut rng);
            cmp_poly!(
                l,
                format!("row37 oob-tail count={count} #{i}"),
                a,
                &buf,
                None
            );
        }
    }

    // Mutating `count` in place on one buffer, exercising the C's per-iteration
    // re-read of `B->count`.
    let mut buf = PolyBuf::from_poly(&poly_ray_box());
    for count in -2..=16i32 {
        buf.set_count(count);
        for i in 0..128 {
            let a = any_ray(&mut rng);
            cmp_poly!(l, format!("row37 mutated count={count} #{i}"), a, &buf, None);
        }
    }
}

// ===========================================================================
// Row 38 — property fuzz: wild polys × wild rays × wild transforms
// ===========================================================================

#[test]
fn row38_c2RaytoPoly_property_fuzz() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 38);
    for i in 0..20_000 {
        let count = (rng.below(20) as i32) - 2; // -2 ..= 17
        let p = wild_poly(&mut rng, count);
        let buf = PolyBuf::from_poly(&p);
        let a = any_ray(&mut rng);
        let use_bx = rng.below(2) == 0;
        let bx = rng.any_x();
        cmp_poly!(
            l,
            format!("row38 fuzz #{i} count={count}"),
            a,
            &buf,
            if use_bx { Some(&bx) } else { None }
        );
    }
    // Special value in each of the ray/transform slots against a fixed poly.
    let sp = special_wide();
    let buf = PolyBuf::from_poly(&poly_ray_box());
    for &v in &sp {
        for slot in 0..9 {
            let mut a = c2Ray {
                p: c2v { x: -3.869416, y: 13.0693407 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 4.0,
            };
            let mut bx = c2x {
                p: c2v { x: 0.5, y: -0.25 },
                r: c2r { c: 0.6, s: 0.8 },
            };
            match slot {
                0 => a.p.x = v,
                1 => a.p.y = v,
                2 => a.d.x = v,
                3 => a.d.y = v,
                4 => a.t = v,
                5 => bx.p.x = v,
                6 => bx.p.y = v,
                7 => bx.r.c = v,
                _ => bx.r.s = v,
            }
            cmp_poly!(l, format!("row38 sp slot{slot} {}", show(v)), a, &buf, Some(&bx));
            cmp_poly!(l, format!("row38 sp-null slot{slot} {}", show(v)), a, &buf, None);
        }
    }
    // Special values inside the polygon data itself.
    for &v in &sp {
        for slot in 0..4 {
            let mut p = poly_ray_box();
            match slot {
                0 => p.verts[0].x = v,
                1 => p.verts[2].y = v,
                2 => p.norms[0].x = v,
                _ => p.norms[3].y = v,
            }
            let b2 = PolyBuf::from_poly(&p);
            let a = c2Ray {
                p: c2v { x: -3.869416, y: 13.0693407 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 4.0,
            };
            cmp_poly!(l, format!("row38 sp-poly slot{slot} {}", show(v)), a, &b2, None);
        }
    }
}

// ===========================================================================
// Rows 39, 41 — c2CastRay dispatcher over all four valid typeB values
// ===========================================================================

#[test]
fn row39_c2CastRay_all_valid_types() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 39);
    let ident = (l.rs.c2xIdentity)();

    for i in 0..(N * 4) {
        let a = any_ray(&mut rng);
        let bx = rng.any_x();

        // CIRCLE
        let c = any_circle(&mut rng);
        let sb = ShapeBuf::from_circle(&c);
        for bxo in [None, Some(&ident), Some(&bx)] {
            cmp_cast!(l, format!("row39 circle #{i}"), a, &sb, bxo, C2_TYPE_CIRCLE);
        }
        // The dispatcher must agree with the direct low-level call.
        let direct = run_circle(&l.c, a, c);
        let via = run_cast(&l.c, a, &sb, None, C2_TYPE_CIRCLE);
        assert!(
            direct == via,
            "row39: c2CastRay(CIRCLE) != c2RaytoCircle in the C: {direct:?} vs {via:?}"
        );

        // AABB
        let bb = any_aabb(&mut rng);
        let sb = ShapeBuf::from_aabb(&bb);
        for bxo in [None, Some(&ident), Some(&bx)] {
            cmp_cast!(l, format!("row39 aabb #{i}"), a, &sb, bxo, C2_TYPE_AABB);
        }
        let direct = run_aabb(&l.c, a, bb);
        let via = run_cast(&l.c, a, &sb, Some(&bx), C2_TYPE_AABB);
        assert!(
            direct == via,
            "row39: c2CastRay(AABB) must ignore bx: {direct:?} vs {via:?}"
        );

        // CAPSULE
        let cap = any_capsule(&mut rng);
        let sb = ShapeBuf::from_capsule(&cap);
        for bxo in [None, Some(&ident), Some(&bx)] {
            cmp_cast!(l, format!("row39 capsule #{i}"), a, &sb, bxo, C2_TYPE_CAPSULE);
        }
        let direct = run_capsule(&l.c, a, cap);
        let via = run_cast(&l.c, a, &sb, Some(&bx), C2_TYPE_CAPSULE);
        assert!(
            direct == via,
            "row39: c2CastRay(CAPSULE) must ignore bx: {direct:?} vs {via:?}"
        );
    }

    // POLY through the dispatcher, over the same PolyBuf the direct tests use.
    for i in 0..(N * 4) {
        let count = (rng.below(18) as i32) - 1;
        let p = wild_poly(&mut rng, count);
        let buf = PolyBuf::from_poly(&p);
        let shape = ShapeBuf::from_bytes(&buf.0[..64]);
        let a = any_ray(&mut rng);
        let bx = rng.any_x();
        // NOTE: `ShapeBuf` is only 64 bytes, which is smaller than a c2Poly, so
        // route POLY through the real 512-byte buffer via c2CastRay's void*.
        let _ = shape;
        let mut b1 = OutBuf::poisoned();
        let mut b2 = OutBuf::poisoned();
        let (r1, r2) = unsafe {
            (
                (l.c.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    &bx,
                    C2_TYPE_POLY,
                    b1.as_ptr(),
                ),
                (l.rs.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    &bx,
                    C2_TYPE_POLY,
                    b2.as_ptr(),
                ),
            )
        };
        assert!(
            r1 == r2 && b1.bytes() == b2.bytes(),
            "row39 poly #{i}: C ret={r1} out={:02x?} vs RUST ret={r2} out={:02x?}",
            &b1.bytes()[..16],
            &b2.bytes()[..16]
        );
        // And the NULL-bx variant.
        let mut b1 = OutBuf::poisoned();
        let mut b2 = OutBuf::poisoned();
        let (r1, r2) = unsafe {
            (
                (l.c.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    std::ptr::null(),
                    C2_TYPE_POLY,
                    b1.as_ptr(),
                ),
                (l.rs.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    std::ptr::null(),
                    C2_TYPE_POLY,
                    b2.as_ptr(),
                ),
            )
        };
        assert!(
            r1 == r2 && b1.bytes() == b2.bytes(),
            "row39 poly-null #{i}: C ret={r1} vs RUST ret={r2}"
        );
    }
}

#[test]
fn row41_c2CastRay_forwards_bx_only_for_poly() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 41);
    let mut differing = 0usize;

    for i in 0..(N * 2) {
        let nverts = 3 + (rng.below(6) as i32);
        let p = convex_ngon(&mut rng, nverts);
        let buf = PolyBuf::from_poly(&p);
        let a = sane_ray(&mut rng);
        // A transform that actually moves things, so supplying it must matter.
        let ang = rng.unit() * std::f32::consts::TAU;
        let bx = c2x {
            p: rng.vec_sym(6.0),
            r: c2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        };

        let run = |api: &Api, use_bx: bool| -> RayResult {
            let mut b = OutBuf::poisoned();
            let ret = unsafe {
                (api.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    if use_bx {
                        &bx as *const c2x
                    } else {
                        std::ptr::null()
                    },
                    C2_TYPE_POLY,
                    b.as_ptr(),
                )
            };
            RayResult {
                ret,
                out: b.bytes(),
            }
        };

        for use_bx in [false, true] {
            let cr = run(&l.c, use_bx);
            let rr = run(&l.rs, use_bx);
            assert!(
                cr == rr,
                "row41 #{i} use_bx={use_bx}: C={cr:?} RUST={rr:?}"
            );
        }
        if run(&l.c, false) != run(&l.c, true) {
            differing += 1;
        }

        // For CIRCLE / AABB / CAPSULE the `bx` argument is dropped on the floor,
        // so passing it must make no difference at all — in either library.
        let c = any_circle(&mut rng);
        let sb = ShapeBuf::from_circle(&c);
        for api in [&l.c, &l.rs] {
            let n = run_cast(api, a, &sb, None, C2_TYPE_CIRCLE);
            let y = run_cast(api, a, &sb, Some(&bx), C2_TYPE_CIRCLE);
            assert!(n == y, "row41 {}: CIRCLE bx leaked: {n:?} vs {y:?}", api.tag);
        }
        let bb = any_aabb(&mut rng);
        let sb = ShapeBuf::from_aabb(&bb);
        for api in [&l.c, &l.rs] {
            let n = run_cast(api, a, &sb, None, C2_TYPE_AABB);
            let y = run_cast(api, a, &sb, Some(&bx), C2_TYPE_AABB);
            assert!(n == y, "row41 {}: AABB bx leaked: {n:?} vs {y:?}", api.tag);
        }
        let cap = any_capsule(&mut rng);
        let sb = ShapeBuf::from_capsule(&cap);
        for api in [&l.c, &l.rs] {
            let n = run_cast(api, a, &sb, None, C2_TYPE_CAPSULE);
            let y = run_cast(api, a, &sb, Some(&bx), C2_TYPE_CAPSULE);
            assert!(n == y, "row41 {}: CAPSULE bx leaked: {n:?} vs {y:?}", api.tag);
        }
    }
    assert!(
        differing > 0,
        "row41: supplying bx never changed a POLY result, so the forwarding \
         assertion has no teeth"
    );
}

// ===========================================================================
// Row 40 — poly_ray, the public one-shot wrapper
// ===========================================================================

#[test]
fn row40_poly_ray() {
    let l = libs();
    for k in 0..64 {
        let (cret, c1, c2) = run_poly_ray(&l.c);
        let (rret, r1, r2) = run_poly_ray(&l.rs);
        assert_eq!(cret, rret, "row40 #{k}: return code differs");
        assert_eq!(c1, r1, "row40 #{k}: cast1 out buffer differs");
        assert_eq!(c2, r2, "row40 #{k}: cast2 out buffer differs");
        // The packed bitfield must be `hit0 + (hit1 << 1)`, so 0..=3.
        assert!(
            (0..=3).contains(&cret),
            "row40: unexpected packed hit value {cret}"
        );
    }
    // Report what the fixed scenario actually produces, and cross-check it
    // against the same scenario driven through the low-level entry point.
    let (ret, b1, b2) = run_poly_ray(&l.rs);
    let rc1 = unsafe { (b1.as_ptr() as *const c2Raycast).read_unaligned() };
    let rc2 = unsafe { (b2.as_ptr() as *const c2Raycast).read_unaligned() };
    eprintln!(
        "poly_ray => {ret}; cast1 t={} n={}; cast2 t={} n={}",
        show(rc1.t),
        showv(rc1.n),
        show(rc2.t),
        showv(rc2.n)
    );

    let buf = PolyBuf::from_poly(&poly_ray_box());
    let ray0 = c2Ray {
        p: c2v {
            x: -3.869416,
            y: 13.0693407,
        },
        d: c2v { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let ray1 = c2Ray {
        p: c2v {
            x: -3.869416,
            y: 13.0693407,
        },
        d: c2v { x: 0.0, y: -1.0 },
        t: 4.0,
    };
    for api in [&l.c, &l.rs] {
        let h0 = run_poly_raw(api, ray0, &buf, None);
        let h1 = run_poly_raw(api, ray1, &buf, None);
        let packed: c_int = h0.ret + (h1.ret << 1);
        let (wret, w1, w2) = run_poly_ray(api);
        assert_eq!(
            packed, wret,
            "row40 {}: poly_ray's packed value disagrees with the low-level calls",
            api.tag
        );
        // On a hit, the wrapper's out buffer must equal the low-level one; on a
        // miss the C leaves `*out` untouched, so both stay poisoned.
        assert_eq!(h0.out, w1, "row40 {}: cast1 payload mismatch", api.tag);
        assert_eq!(h1.out, w2, "row40 {}: cast2 payload mismatch", api.tag);
    }
}
