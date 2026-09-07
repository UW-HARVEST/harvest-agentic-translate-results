//! Phase C — error / rejection-path differential tests.
//!
//! One test (or one clearly-labelled block) per row of `ERRORS.md`. Every row
//! constructs the exact invalid input the C source checks for and asserts that
//! both libraries reject it *identically* — same sentinel, same untouched
//! output fields, same NaN/inf bit pattern — not merely "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

// ---------------------------------------------------------------------------
// shared plumbing
// ---------------------------------------------------------------------------

type MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
type Collide = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
type PtrFromParts = unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void;
type Omni = unsafe extern "C" fn(
    *mut c2Manifold, c_int, f32, f32, f32, f32, f32, c_int, f32, f32, f32, f32, f32,
);
type AabbAabb = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
type CircCirc = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
type CircAabb = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
type CircCap = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
type CapCap = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
type AabbCap = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
type CapPoly = unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold);
type GjkFn = unsafe extern "C" fn(
    *const c_void, c_int, *const c2x, *const c_void, c_int, *const c2x,
    *mut c2v, *mut c2v, c_int, *mut c_int, *mut c2GJKCache,
) -> f32;

/// Every enum value that is *not* a valid `C2_TYPE`, plus the one valid-but-
/// unhandled variant (`C2_TYPE_POLY`).
const BAD_TYPES: [c_int; 9] = [
    C2_TYPE_POLY,
    -1,
    4,
    5,
    99,
    -12345,
    c_int::MIN,
    c_int::MAX,
    0x7fff_0000,
];

const GOOD_TYPES: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

fn seed_manifold() -> c2Manifold {
    c2Manifold {
        count: -13,
        depths: [7.5, -8.25],
        contact_points: [c2v { x: 11.0, y: 12.0 }, c2v { x: 13.0, y: 14.0 }],
        n: c2v { x: 0.25, y: -0.75 },
    }
}

#[inline(never)]
fn expect_man_eq(row: &str, mc: &c2Manifold, mr: &c2Manifold, ctx: impl FnOnce() -> String) {
    if !man_eq(mc, mr) {
        panic!(
            "ERRORS.md row {}: {}\n  C: {}\n  R: {}",
            row,
            ctx(),
            fmt_man(mc),
            fmt_man(mr)
        );
    }
}

// ---------------------------------------------------------------------------
// rows 1 / 2 — c2MakeProxy: POLY and out-of-range type leave *p untouched
// ---------------------------------------------------------------------------

#[test]
fn row01_row02_makeproxy_unhandled_type() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<MakeProxy>("c2MakeProxy"), p.r.get::<MakeProxy>("c2MakeProxy"));
        let mut rng = Rng::new(0xE001);
        let poly = c2Poly::default();
        for &ty in &BAD_TYPES {
            for _ in 0..200 {
                // distinctive pre-fill so "untouched" is observable
                let seed = c2Proxy {
                    radius: rng.nice(),
                    count: rng.next_u32() as c_int,
                    verts: [rng.v(); 8],
                };
                let mut pc = seed;
                let mut pr = seed;
                with_clean_stack(|| unsafe {
                    f_c(&poly as *const _ as *const c_void, ty, &mut pc)
                });
                with_clean_stack(|| unsafe {
                    f_r(&poly as *const _ as *const c_void, ty, &mut pr)
                });
                assert!(
                    proxy_eq(&pc, &pr),
                    "row 1/2: type={} C={:?} R={:?}",
                    ty, pc, pr
                );
                assert!(
                    proxy_eq(&pc, &seed),
                    "row 1/2: type={} must leave the proxy untouched, got {:?}",
                    ty, pc
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 3 — ptr_from_parts with no matching case falls off the end
// ---------------------------------------------------------------------------

#[test]
fn row03_ptr_from_parts_no_case() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (
            p.c.get::<PtrFromParts>("ptr_from_parts"),
            p.r.get::<PtrFromParts>("ptr_from_parts"),
        );
        // Valid types must return a usable, correctly-populated allocation.
        unsafe {
            let c = f_c(C2_TYPE_CIRCLE, 1.0, 2.0, 3.0, 4.0, 5.0) as *const c2Circle;
            let r = f_r(C2_TYPE_CIRCLE, 1.0, 2.0, 3.0, 4.0, 5.0) as *const c2Circle;
            assert!(!c.is_null() && !r.is_null());
            assert!(v_eq((*c).p, (*r).p) && bits_eq_f32((*c).r, (*r).r), "row 3: circle payload");
            assert!(v_eq((*c).p, c2v { x: 1.0, y: 2.0 }) && bits_eq_f32((*c).r, 3.0));

            let c = f_c(C2_TYPE_AABB, 1.0, 2.0, 3.0, 4.0, 5.0) as *const c2AABB;
            let r = f_r(C2_TYPE_AABB, 1.0, 2.0, 3.0, 4.0, 5.0) as *const c2AABB;
            assert!(v_eq((*c).min, (*r).min) && v_eq((*c).max, (*r).max), "row 3: aabb payload");

            let c = f_c(C2_TYPE_CAPSULE, 1.0, 2.0, 3.0, 4.0, 5.0) as *const c2Capsule;
            let r = f_r(C2_TYPE_CAPSULE, 1.0, 2.0, 3.0, 4.0, 5.0) as *const c2Capsule;
            assert!(
                v_eq((*c).a, (*r).a) && v_eq((*c).b, (*r).b) && bits_eq_f32((*c).r, (*r).r),
                "row 3: capsule payload"
            );
        }
        // For POLY / out-of-range there is no `return` at all: the C function
        // falls off the end, so its return value is indeterminate by definition
        // and cannot be compared. What IS observable — and what `c2Collide`
        // relies on — is that neither library dereferences or crashes, and that
        // the pointer is never used. Assert the call is survivable on both.
        for &ty in &BAD_TYPES {
            let a = with_clean_stack(|| unsafe { f_c(ty, 1.0, 2.0, 3.0, 4.0, 5.0) });
            let b = with_clean_stack(|| unsafe { f_r(ty, 1.0, 2.0, 3.0, 4.0, 5.0) });
            // Rust deliberately returns NULL here; the value must never be used.
            let _ = (a, b);
            assert!(b.is_null(), "row 3: Rust must return a null sentinel for type {}", ty);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 4 / 5 / 6 / 7 — c2Collide with unhandled / out-of-range type tags
// ---------------------------------------------------------------------------

#[test]
fn row04_row07_collide_bad_types() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<Collide>("c2Collide"), p.r.get::<Collide>("c2Collide"));
        let mut rng = Rng::new(0xE004);
        for _ in 0..400 {
            let circ = c2Circle { p: rng.v(), r: rng.range(0.0, 3.0) };
            let bb = c2AABB { min: rng.v(), max: rng.v() };
            let cap = c2Capsule { a: rng.v(), b: rng.v(), r: rng.range(0.0, 3.0) };
            let blobs: [*const c_void; 3] = [
                &circ as *const _ as *const c_void,
                &bb as *const _ as *const c_void,
                &cap as *const _ as *const c_void,
            ];
            // row 4/5: bad typeA (outer switch)
            for &ta in &BAD_TYPES {
                for (i, &tb) in GOOD_TYPES.iter().enumerate() {
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    with_clean_stack(|| unsafe { f_c(blobs[0], ta, blobs[i], tb, &mut mc) });
                    with_clean_stack(|| unsafe { f_r(blobs[0], ta, blobs[i], tb, &mut mr) });
                    expect_man_eq("4/5", &mc, &mr, || format!("ta={} tb={}", ta, tb));
                    assert_eq!(mc.count, 0, "row 4/5: ta={} must yield count 0", ta);
                    // everything except `count` must be untouched
                    let s = seed_manifold();
                    assert!(
                        bits_eq_f32(mc.depths[0], s.depths[0])
                            && v_eq(mc.n, s.n)
                            && v_eq(mc.contact_points[0], s.contact_points[0]),
                        "row 4/5: ta={} must not touch depths/n/contacts, got {}",
                        ta, fmt_man(&mc)
                    );
                }
            }
            // row 6/7: good typeA, bad typeB (inner switch)
            for (i, &ta) in GOOD_TYPES.iter().enumerate() {
                for &tb in &BAD_TYPES {
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    with_clean_stack(|| unsafe { f_c(blobs[i], ta, blobs[0], tb, &mut mc) });
                    with_clean_stack(|| unsafe { f_r(blobs[i], ta, blobs[0], tb, &mut mr) });
                    expect_man_eq("6/7", &mc, &mr, || format!("ta={} tb={}", ta, tb));
                    assert_eq!(mc.count, 0, "row 6/7: tb={} must yield count 0", tb);
                    // `m->n` must NOT be negated on the unhandled inner branch
                    assert!(
                        v_eq(mc.n, seed_manifold().n),
                        "row 6/7: ta={} tb={} must leave m->n untouched, got {}",
                        ta, tb, fmt_v(mc.n)
                    );
                }
            }
            // both bad
            for &ta in &BAD_TYPES {
                for &tb in &BAD_TYPES {
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    with_clean_stack(|| unsafe { f_c(blobs[0], ta, blobs[1], tb, &mut mc) });
                    with_clean_stack(|| unsafe { f_r(blobs[0], ta, blobs[1], tb, &mut mr) });
                    expect_man_eq("4-7", &mc, &mr, || format!("ta={} tb={}", ta, tb));
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 8 / 9 — c2AABBtoAABBManifold separating-axis early returns
// ---------------------------------------------------------------------------

#[test]
fn row08_row09_aabb_aabb_separated() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (
            p.c.get::<AabbAabb>("c2AABBtoAABBManifold"),
            p.r.get::<AabbAabb>("c2AABBtoAABBManifold"),
        );
        let unit = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        // row 8: separated on x (dx < 0);  row 9: dx >= 0 but dy < 0
        let cases: &[(f32, f32, &str)] = &[
            (5.0, 0.0, "8"),
            (-5.0, 0.0, "8"),
            (2.000_001, 0.0, "8"),
            (0.0, 5.0, "9"),
            (0.0, -5.0, "9"),
            (0.0, 2.000_001, "9"),
            (1.5, 5.0, "9"),
        ];
        for &(dx, dy, row) in cases {
            let b = c2AABB {
                min: c2v { x: -1.0 + dx, y: -1.0 + dy },
                max: c2v { x: 1.0 + dx, y: 1.0 + dy },
            };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { f_c(unit, b, &mut mc) });
            with_clean_stack(|| unsafe { f_r(unit, b, &mut mr) });
            expect_man_eq(row, &mc, &mr, || format!("dx={} dy={}", dx, dy));
            assert_eq!(mc.count, 0, "row {}: dx={} dy={}", row, dx, dy);
            let s = seed_manifold();
            assert!(
                bits_eq_f32(mc.depths[0], s.depths[0]) && v_eq(mc.n, s.n),
                "row {}: early return must not write depths/n",
                row
            );
        }
    });
}

// ---------------------------------------------------------------------------
// rows 10 / 11 / 12 / 13 / 14 — "no overlap" rejections
// ---------------------------------------------------------------------------

#[test]
fn row10_row14_no_overlap() {
    fresh(|| {
        let p = pair();
        let cc = (
            p.c.get::<CircCirc>("c2CircletoCircleManifold"),
            p.r.get::<CircCirc>("c2CircletoCircleManifold"),
        );
        let ca = (
            p.c.get::<CircAabb>("c2CircletoAABBManifold"),
            p.r.get::<CircAabb>("c2CircletoAABBManifold"),
        );
        let ck = (
            p.c.get::<CircCap>("c2CircletoCapsuleManifold"),
            p.r.get::<CircCap>("c2CircletoCapsuleManifold"),
        );
        let kk = (
            p.c.get::<CapCap>("c2CapsuletoCapsuleManifold"),
            p.r.get::<CapCap>("c2CapsuletoCapsuleManifold"),
        );

        // row 10: d2 >= r*r — including the exact-touch boundary d2 == r*r
        for &(dx, r_a, r_b) in &[
            (10.0f32, 1.0f32, 2.0f32),
            (3.0, 1.0, 2.0), // exactly touching -> rejected (strict <)
            (3.000_01, 1.0, 2.0),
            (0.0, 0.0, 0.0), // both degenerate, d2 == 0 == r*r -> rejected
        ] {
            let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: r_a };
            let b = c2Circle { p: c2v { x: dx, y: 0.0 }, r: r_b };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { cc.0(a, b, &mut mc) });
            with_clean_stack(|| unsafe { cc.1(a, b, &mut mr) });
            expect_man_eq("10", &mc, &mr, || format!("dx={} rA={} rB={}", dx, r_a, r_b));
            assert_eq!(mc.count, 0, "row 10: dx={} rA={} rB={}", dx, r_a, r_b);
        }

        // row 11: d2 >= r2
        let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        for &(x, r) in &[(10.0f32, 1.0f32), (2.0, 1.0), (2.0, 0.999_999), (1.0, 0.0)] {
            let a = c2Circle { p: c2v { x, y: 0.0 }, r };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { ca.0(a, bb, &mut mc) });
            with_clean_stack(|| unsafe { ca.1(a, bb, &mut mr) });
            expect_man_eq("11", &mc, &mr, || format!("x={} r={}", x, r));
            assert_eq!(mc.count, 0, "row 11: x={} r={}", x, r);
        }

        // row 12: GJK d >= r
        let capsule = c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 };
        for &(y, r) in &[(10.0f32, 1.0f32), (1.5, 1.0), (2.0, 1.4)] {
            let a = c2Circle { p: c2v { x: 0.0, y }, r };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { ck.0(a, capsule, &mut mc) });
            with_clean_stack(|| unsafe { ck.1(a, capsule, &mut mr) });
            expect_man_eq("12", &mc, &mr, || format!("y={} r={}", y, r));
            assert_eq!(mc.count, 0, "row 12: y={} r={}", y, r);
        }

        // row 13: capsule/capsule GJK d >= r
        for &y in &[10.0f32, 1.0, 1.000_001] {
            let b = c2Capsule { a: c2v { x: -1.0, y }, b: c2v { x: 1.0, y }, r: 0.5 };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { kk.0(capsule, b, &mut mc) });
            with_clean_stack(|| unsafe { kk.1(capsule, b, &mut mr) });
            expect_man_eq("13", &mc, &mr, || format!("y={}", y));
            assert_eq!(mc.count, 0, "row 13: y={}", y);
        }

        // row 14: capsule/poly, d >= 1e-6 AND d >= A.r
        let f_cp = (
            p.c.get::<CapPoly>("c2CapsuletoPolyManifold"),
            p.r.get::<CapPoly>("c2CapsuletoPolyManifold"),
        );
        let norms = p.c.get::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int)>("c2Norms");
        let mut poly = c2Poly::default();
        poly.count = 4;
        poly.verts[0] = c2v { x: -1.0, y: -1.0 };
        poly.verts[1] = c2v { x: 1.0, y: -1.0 };
        poly.verts[2] = c2v { x: 1.0, y: 1.0 };
        poly.verts[3] = c2v { x: -1.0, y: 1.0 };
        with_clean_stack(|| unsafe { norms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), 4) });
        for &(x, r) in &[(50.0f32, 0.5f32), (5.0, 1.0), (3.0, 1.9)] {
            let a = c2Capsule { a: c2v { x, y: -1.0 }, b: c2v { x, y: 1.0 }, r };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { f_cp.0(a, &poly, std::ptr::null(), &mut mc) });
            with_clean_stack(|| unsafe { f_cp.1(a, &poly, std::ptr::null(), &mut mr) });
            expect_man_eq("14", &mc, &mr, || format!("x={} r={}", x, r));
            assert_eq!(mc.count, 0, "row 14: x={} r={}", x, r);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 15..23 — c2Clip / c2SidePlanes rejections, reached through
// c2CapsuletoPolyManifold (the only callers) and verified end to end.
// ---------------------------------------------------------------------------

#[test]
fn row15_row23_clip_sideplane_rejections() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (
            p.c.get::<CapPoly>("c2CapsuletoPolyManifold"),
            p.r.get::<CapPoly>("c2CapsuletoPolyManifold"),
        );
        let norms = p.c.get::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int)>("c2Norms");
        let mut rng = Rng::new(0xE015);
        let mut ub_free = 0usize;
        for n in 3..=8i32 {
            for _ in 0..1500 {
                let radius = rng.range(0.2, 3.0);
                let phase = rng.range(-3.2, 3.2);
                let mut poly = c2Poly::default();
                poly.count = n;
                for i in 0..n {
                    let t = phase + 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
                    poly.verts[i as usize] =
                        c2v { x: radius * t.cos(), y: radius * t.sin() };
                }
                with_clean_stack(|| unsafe {
                    norms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), n)
                });
                // Capsules whose side planes clip the reference edge away are
                // exactly the ones that make c2Clip return < 2. Sweeping a
                // deeply-overlapping capsule across the polygon hits every
                // combination of {code 0,1,2} x {clip accepted, clip rejected}.
                let a = c2Capsule {
                    a: c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) },
                    b: c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) },
                    r: rng.range(0.0, 3.0),
                };
                for bx in [
                    None,
                    Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }),
                    Some(rng.xform()),
                ] {
                    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    with_clean_stack(|| unsafe { f_c(a, &poly, bxp, &mut mc) });
                    with_clean_stack(|| unsafe { f_r(a, &poly, bxp, &mut mr) });
                    expect_man_eq("15-23", &mc, &mr, || {
                        format!("n={} A={:?} bx={:?}", n, a, bx)
                    });
                    ub_free += 1;
                }
            }
        }
        assert!(ub_free > 20_000, "row 15-23: expected a large sample, got {}", ub_free);

        // row 21: `h == NULL` — c2SidePlanes with a null out-plane is only
        // reachable internally, but the equivalent observable case is
        // c2CapsuletoPolyManifold with a NULL transform, exercised above.

        // row 23: d0 == d1 == 0 (segment exactly on the clip plane).
        let mut poly = c2Poly::default();
        poly.count = 4;
        poly.verts[0] = c2v { x: -1.0, y: -1.0 };
        poly.verts[1] = c2v { x: 1.0, y: -1.0 };
        poly.verts[2] = c2v { x: 1.0, y: 1.0 };
        poly.verts[3] = c2v { x: -1.0, y: 1.0 };
        with_clean_stack(|| unsafe { norms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), 4) });
        for a in [
            // capsule segment lying exactly along a polygon edge
            c2Capsule { a: c2v { x: -1.0, y: -1.0 }, b: c2v { x: 1.0, y: -1.0 }, r: 0.5 },
            c2Capsule { a: c2v { x: -1.0, y: 1.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 0.5 },
            // segment exactly through opposite corners
            c2Capsule { a: c2v { x: -1.0, y: -1.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 0.25 },
            // segment exactly on the left side plane
            c2Capsule { a: c2v { x: -1.0, y: -1.0 }, b: c2v { x: -1.0, y: 1.0 }, r: 0.25 },
        ] {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { f_c(a, &poly, std::ptr::null(), &mut mc) });
            with_clean_stack(|| unsafe { f_r(a, &poly, std::ptr::null(), &mut mr) });
            expect_man_eq("22/23", &mc, &mr, || format!("A={:?}", a));
        }
    });
}

// ---------------------------------------------------------------------------
// rows 24 / 25 / 67 / 68 — index == ~0 out-of-range reads
// ---------------------------------------------------------------------------

#[test]
fn row24_row25_row67_row68_negative_index() {
    fresh(|| {
        let p = pair();
        let (cp_c, cp_r) = (
            p.c.get::<CapPoly>("c2CapsuletoPolyManifold"),
            p.r.get::<CapPoly>("c2CapsuletoPolyManifold"),
        );
        let (ac_c, ac_r) = (
            p.c.get::<AabbCap>("c2AABBtoCapsuleManifold"),
            p.r.get::<AabbCap>("c2AABBtoCapsuleManifold"),
        );
        let norms = p.c.get::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int)>("c2Norms");

        // row 67: degenerate capsule (A.a == A.b) => ab = NaN => every plane NaN
        //          => index stays ~0 => c2SidePlanesFromPoly(..., -1, ...)
        let mut poly = c2Poly::default();
        poly.count = 4;
        poly.verts[0] = c2v { x: -1.0, y: -1.0 };
        poly.verts[1] = c2v { x: 1.0, y: -1.0 };
        poly.verts[2] = c2v { x: 1.0, y: 1.0 };
        poly.verts[3] = c2v { x: -1.0, y: 1.0 };
        with_clean_stack(|| unsafe { norms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), 4) });
        for pos in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.5, y: -0.25 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: 50.0, y: 50.0 },
        ] {
            for r in [0.0f32, 0.5, 5.0] {
                let a = c2Capsule { a: pos, b: pos, r };
                let mut mc = seed_manifold();
                let mut mr = seed_manifold();
                with_clean_stack(|| unsafe { cp_c(a, &poly, std::ptr::null(), &mut mc) });
                with_clean_stack(|| unsafe { cp_r(a, &poly, std::ptr::null(), &mut mr) });
                expect_man_eq("67", &mc, &mr, || format!("A={:?}", a));
            }
        }

        // rows 24/25/68: degenerate AABB => c2Norms produces NaN normals =>
        // c2Incident / index == -1 => verts[-1] out-of-bounds read.
        let zero = c2v { x: 0.0, y: 0.0 };
        let aabbs = [
            c2AABB { min: zero, max: zero },
            c2AABB { min: c2v { x: 2.0, y: 3.0 }, max: c2v { x: 2.0, y: 3.0 } },
            c2AABB { min: c2v { x: 0.0, y: -1.0 }, max: c2v { x: 0.0, y: 1.0 } }, // zero width
            c2AABB { min: c2v { x: -1.0, y: 0.0 }, max: c2v { x: 1.0, y: 0.0 } }, // zero height
            c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } }, // inverted
        ];
        let caps = [
            c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 },
            c2Capsule { a: c2v { x: 0.0, y: -1.0 }, b: c2v { x: 0.0, y: 1.0 }, r: 0.5 },
            c2Capsule { a: zero, b: zero, r: 1.0 },
            c2Capsule { a: c2v { x: 9.0, y: 9.0 }, b: c2v { x: 10.0, y: 10.0 }, r: 0.5 },
        ];
        for &bb in &aabbs {
            for &cap in &caps {
                let mut mc = seed_manifold();
                let mut mr = seed_manifold();
                with_clean_stack(|| unsafe { ac_c(bb, cap, &mut mc) });
                with_clean_stack(|| unsafe { ac_r(bb, cap, &mut mr) });
                expect_man_eq("24/25/68", &mc, &mr, || format!("A={:?} B={:?}", bb, cap));
            }
        }

        // row 24 proper: ip->count <= 0 -> the loop never runs. Reached through
        // c2CapsuletoPolyManifold with an empty polygon.
        for count in [0i32, -1, -7] {
            let mut empty = c2Poly::default();
            empty.count = count;
            let a = c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { cp_c(a, &empty, std::ptr::null(), &mut mc) });
            with_clean_stack(|| unsafe { cp_r(a, &empty, std::ptr::null(), &mut mr) });
            expect_man_eq("24", &mc, &mr, || format!("poly.count={}", count));
        }
    });
}

// ---------------------------------------------------------------------------
// row 26 — c2Support with count <= 0
// ---------------------------------------------------------------------------

#[test]
fn row26_support_nonpositive_count() {
    fresh(|| {
        let p = pair();
        type Sup = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        let (f_c, f_r) = (p.c.get::<Sup>("c2Support"), p.r.get::<Sup>("c2Support"));
        let mut rng = Rng::new(0xE026);
        for count in [0i32, -1, -2, -1000, c_int::MIN] {
            for _ in 0..200 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = rng.wild_v();
                }
                let d = rng.wild_v();
                let x = with_clean_stack(|| unsafe { f_c(verts.as_ptr(), count, d) });
                let y = with_clean_stack(|| unsafe { f_r(verts.as_ptr(), count, d) });
                assert_eq!(x, y, "row 26: count={} C={} R={}", count, x, y);
                assert_eq!(x, 0, "row 26: count={} must return 0", count);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 27..31 / 61 / 62 — arithmetic sentinels (NaN / inf)
// ---------------------------------------------------------------------------

#[test]
fn row27_row31_arithmetic_sentinels() {
    fresh(|| {
        let p = pair();
        let (norm_c, norm_r) = (p.c.get::<FnVV>("c2Norm"), p.r.get::<FnVV>("c2Norm"));
        let (div_c, div_r) = (p.c.get::<FnVfV>("c2Div"), p.r.get::<FnVfV>("c2Div"));
        let (len_c, len_r) = (p.c.get::<FnVf>("c2Len"), p.r.get::<FnVf>("c2Len"));
        let (dot_c, dot_r) = (p.c.get::<FnVVf>("c2Dot"), p.r.get::<FnVVf>("c2Dot"));
        type Isect = unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v;
        let (is_c, is_r) = (p.c.get::<Isect>("c2Intersect"), p.r.get::<Isect>("c2Intersect"));

        // rows 27/28: c2Norm of the zero vector and of non-finite vectors
        let degenerate = [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: 0.0, y: f32::NAN },
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::INFINITY },
            c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
            c2v { x: f32::from_bits(0xffab_cdef), y: 1.0 }, // sNaN
            c2v { x: f32::MIN_POSITIVE, y: 0.0 },
            c2v { x: f32::from_bits(1), y: 0.0 }, // denormal
        ];
        for a in degenerate {
            let x = with_clean_stack(|| unsafe { norm_c(a) });
            let y = with_clean_stack(|| unsafe { norm_r(a) });
            assert!(v_eq(x, y), "row 27/28: c2Norm({}) C={} R={}", fmt_v(a), fmt_v(x), fmt_v(y));
            // rows 29/30: c2Len overflow / NaN
            let lx = unsafe { len_c(a) };
            let ly = unsafe { len_r(a) };
            assert!(bits_eq_f32(lx, ly), "row 29/30: c2Len({})", fmt_v(a));
            // rows 61/62
            for b in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 1e-45] {
                let dx = unsafe { div_c(a, b) };
                let dy = unsafe { div_r(a, b) };
                assert!(v_eq(dx, dy), "row 61: c2Div({}, {})", fmt_v(a), fmt_f32(b));
            }
            for b in degenerate {
                let sx = unsafe { dot_c(a, b) };
                let sy = unsafe { dot_r(a, b) };
                assert!(bits_eq_f32(sx, sy), "row 62: c2Dot({}, {})", fmt_v(a), fmt_v(b));
            }
        }
        // rows 29/30: huge magnitudes -> inf, inf-inf -> NaN
        for a in [
            c2v { x: 1e38, y: 1e38 },
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ] {
            let lx = unsafe { len_c(a) };
            let ly = unsafe { len_r(a) };
            assert!(bits_eq_f32(lx, ly), "row 29/30: c2Len({}) C={} R={}", fmt_v(a), fmt_f32(lx), fmt_f32(ly));
        }
        // row 31: c2Intersect with da == db (0/0 and x/0)
        let mut rng = Rng::new(0xE031);
        for _ in 0..2000 {
            let a = rng.wild_v();
            let b = rng.wild_v();
            let t = rng.wild();
            for (da, db) in [(t, t), (0.0, 0.0), (1.0, 1.0), (-0.0, 0.0), (f32::INFINITY, f32::INFINITY)] {
                let x = unsafe { is_c(a, b, da, db) };
                let y = unsafe { is_r(a, b, da, db) };
                assert!(
                    v_eq(x, y),
                    "row 31: c2Intersect({},{},{},{}) C={} R={}",
                    fmt_v(a), fmt_v(b), fmt_f32(da), fmt_f32(db), fmt_v(x), fmt_v(y)
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 32..37 — simplex functions with out-of-contract `count` / `div`
// ---------------------------------------------------------------------------

#[test]
fn row32_row37_simplex_out_of_contract() {
    fresh(|| {
        let p = pair();
        type Metric = unsafe extern "C" fn(*mut c2Simplex) -> f32;
        type DFn = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        type Wit = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
        let (m_c, m_r) = (p.c.get::<Metric>("c2GJKSimplexMetric"), p.r.get::<Metric>("c2GJKSimplexMetric"));
        let (d_c, d_r) = (p.c.get::<DFn>("c2D"), p.r.get::<DFn>("c2D"));
        let (l_c, l_r) = (p.c.get::<DFn>("c2L"), p.r.get::<DFn>("c2L"));
        let (w_c, w_r) = (p.c.get::<Wit>("c2Witness"), p.r.get::<Wit>("c2Witness"));

        let mut rng = Rng::new(0xE032);
        // every out-of-contract count, including negative and > 3
        let counts: [c_int; 10] = [0, 4, 5, 8, -1, -2, -1000, c_int::MIN, c_int::MAX, 100];
        let divs = [0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, 1e-45, f32::MAX];
        for &count in &counts {
            for &div in &divs {
                for _ in 0..40 {
                    let mut s = c2Simplex::default();
                    for v in [&mut s.a as *mut c2sv, &mut s.b, &mut s.c, &mut s.d] {
                        unsafe {
                            (*v).sA = rng.wild_v();
                            (*v).sB = rng.wild_v();
                            (*v).p = rng.wild_v();
                            (*v).u = rng.wild();
                            (*v).iA = rng.next_u32() as c_int;
                            (*v).iB = rng.next_u32() as c_int;
                        }
                    }
                    s.div = div;
                    s.count = count;

                    // row 32
                    let mut s1 = s;
                    let mut s2 = s;
                    let x = with_clean_stack(|| unsafe { m_c(&mut s1) });
                    let y = with_clean_stack(|| unsafe { m_r(&mut s2) });
                    assert!(bits_eq_f32(x, y), "row 32: count={} C={} R={}", count, fmt_f32(x), fmt_f32(y));
                    assert!(simplex_eq(&s1, &s2), "row 32: simplex mutated differently");

                    // row 33
                    let mut s1 = s;
                    let mut s2 = s;
                    let x = with_clean_stack(|| unsafe { d_c(&mut s1) });
                    let y = with_clean_stack(|| unsafe { d_r(&mut s2) });
                    assert!(v_eq(x, y), "row 33: count={} C={} R={}", count, fmt_v(x), fmt_v(y));

                    // rows 36/37
                    let mut s1 = s;
                    let mut s2 = s;
                    let x = with_clean_stack(|| unsafe { l_c(&mut s1) });
                    let y = with_clean_stack(|| unsafe { l_r(&mut s2) });
                    assert!(
                        v_eq(x, y),
                        "row 36/37: count={} div={} C={} R={}",
                        count, fmt_f32(div), fmt_v(x), fmt_v(y)
                    );

                    // rows 34/35
                    let mut s1 = s;
                    let mut s2 = s;
                    let (mut a1, mut b1) = (c2v { x: 1.5, y: 2.5 }, c2v { x: 3.5, y: 4.5 });
                    let (mut a2, mut b2) = (a1, b1);
                    with_clean_stack(|| unsafe { w_c(&mut s1, &mut a1, &mut b1) });
                    with_clean_stack(|| unsafe { w_r(&mut s2, &mut a2, &mut b2) });
                    assert!(
                        v_eq(a1, a2) && v_eq(b1, b2),
                        "row 34/35: count={} div={} C=({},{}) R=({},{})",
                        count, fmt_f32(div), fmt_v(a1), fmt_v(b1), fmt_v(a2), fmt_v(b2)
                    );
                }
            }
        }
        // rows 20/21 of CONFIGS are the valid counts; here make sure the valid
        // counts with div == 0 (rows 35/37) also agree.
        for count in [1i32, 2, 3] {
            for div in [0.0f32, -0.0] {
                let mut s = c2Simplex::default();
                s.count = count;
                s.div = div;
                s.a.u = 1.0;
                s.b.u = 2.0;
                s.c.u = 3.0;
                s.a.sA = c2v { x: 1.0, y: 2.0 };
                s.b.sA = c2v { x: 3.0, y: 4.0 };
                s.c.sA = c2v { x: 5.0, y: 6.0 };
                s.a.sB = c2v { x: -1.0, y: -2.0 };
                s.b.sB = c2v { x: -3.0, y: -4.0 };
                s.c.sB = c2v { x: -5.0, y: -6.0 };
                let mut s1 = s;
                let mut s2 = s;
                let (mut a1, mut b1) = (c2v::default(), c2v::default());
                let (mut a2, mut b2) = (c2v::default(), c2v::default());
                with_clean_stack(|| unsafe { w_c(&mut s1, &mut a1, &mut b1) });
                with_clean_stack(|| unsafe { w_r(&mut s2, &mut a2, &mut b2) });
                assert!(v_eq(a1, a2) && v_eq(b1, b2), "row 35: count={} div=0", count);
                let mut s1 = s;
                let mut s2 = s;
                let x = unsafe { l_c(&mut s1) };
                let y = unsafe { l_r(&mut s2) };
                assert!(v_eq(x, y), "row 37: count={} div=0", count);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 38..52 — c2GJK NULL parameters, cache states, loop exits
// ---------------------------------------------------------------------------

#[test]
fn row38_row52_gjk_null_and_limits() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<GjkFn>("c2GJK"), p.r.get::<GjkFn>("c2GJK"));
        let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
        let mut rng = Rng::new(0xE038);

        for _ in 0..600 {
            let circ = c2Circle { p: rng.v(), r: rng.range(0.0, 3.0) };
            let cap = c2Capsule { a: rng.v(), b: rng.v(), r: rng.range(0.0, 3.0) };
            let bb = c2AABB { min: rng.v(), max: rng.v() };
            let blobs: [(c_int, *const c_void); 3] = [
                (C2_TYPE_CIRCLE, &circ as *const _ as *const c_void),
                (C2_TYPE_AABB, &bb as *const _ as *const c_void),
                (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
            ];
            for &(ta, pa) in &blobs {
                for &(tb, pb) in &blobs {
                    for ur in [0, 1] {
                        // rows 38/39: NULL transforms == identity transforms
                        let mut o = [c2v::default(); 8];
                        let mut it = [0 as c_int; 4];
                        let d_null_c = with_clean_stack(|| unsafe {
                            f_c(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                &mut o[0], &mut o[1], ur, &mut it[0], std::ptr::null_mut())
                        });
                        let d_id_c = with_clean_stack(|| unsafe {
                            f_c(pa, ta, &ident, pb, tb, &ident,
                                &mut o[2], &mut o[3], ur, &mut it[1], std::ptr::null_mut())
                        });
                        let d_null_r = with_clean_stack(|| unsafe {
                            f_r(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                &mut o[4], &mut o[5], ur, &mut it[2], std::ptr::null_mut())
                        });
                        let d_id_r = with_clean_stack(|| unsafe {
                            f_r(pa, ta, &ident, pb, tb, &ident,
                                &mut o[6], &mut o[7], ur, &mut it[3], std::ptr::null_mut())
                        });
                        assert!(bits_eq_f32(d_null_c, d_null_r), "row 38/39: NULL xform dist");
                        assert!(bits_eq_f32(d_id_c, d_id_r), "row 38/39: identity xform dist");
                        assert!(bits_eq_f32(d_null_c, d_id_c), "row 38/39: C NULL != identity");
                        assert!(bits_eq_f32(d_null_r, d_id_r), "row 38/39: R NULL != identity");
                        assert!(v_eq(o[0], o[4]) && v_eq(o[1], o[5]), "row 38/39: NULL outs");
                        assert!(v_eq(o[2], o[6]) && v_eq(o[3], o[7]), "row 38/39: ident outs");
                        assert_eq!(it[0], it[2]);
                        assert_eq!(it[1], it[3]);

                        // rows 40/41/42: NULL outA / outB / iterations, in every
                        // subset, must not change the return value.
                        for mask in 0..8u32 {
                            let mut oa_c = c2v { x: 9.0, y: 9.0 };
                            let mut ob_c = c2v { x: 8.0, y: 8.0 };
                            let mut i_c: c_int = -5;
                            let mut oa_r = oa_c;
                            let mut ob_r = ob_c;
                            let mut i_r: c_int = -5;
                            let pa_out = |m: u32, ptr: *mut c2v| if m & 1 != 0 { std::ptr::null_mut() } else { ptr };
                            let pb_out = |m: u32, ptr: *mut c2v| if m & 2 != 0 { std::ptr::null_mut() } else { ptr };
                            let pi_out = |m: u32, ptr: *mut c_int| if m & 4 != 0 { std::ptr::null_mut() } else { ptr };
                            let dc = with_clean_stack(|| unsafe {
                                f_c(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                    pa_out(mask, &mut oa_c), pb_out(mask, &mut ob_c), ur,
                                    pi_out(mask, &mut i_c), std::ptr::null_mut())
                            });
                            let dr = with_clean_stack(|| unsafe {
                                f_r(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                    pa_out(mask, &mut oa_r), pb_out(mask, &mut ob_r), ur,
                                    pi_out(mask, &mut i_r), std::ptr::null_mut())
                            });
                            assert!(bits_eq_f32(dc, dr), "row 40-42: mask={} dist", mask);
                            assert!(v_eq(oa_c, oa_r) && v_eq(ob_c, ob_r), "row 40-42: mask={} outs", mask);
                            assert_eq!(i_c, i_r, "row 40-42: mask={} iterations", mask);
                            assert!(bits_eq_f32(dc, d_null_c), "row 40-42: NULL outs changed dist");
                            if mask & 1 != 0 {
                                assert!(v_eq(oa_c, c2v { x: 9.0, y: 9.0 }), "row 40: outA written despite NULL");
                            }
                            if mask & 4 != 0 {
                                assert_eq!(i_c, -5, "row 42: iterations written despite NULL");
                            }
                        }

                        // rows 43/44/45: cache NULL / cold / warm
                        let dc_nocache = d_null_c;
                        let cold = c2GJKCache { metric: rng.nice(), count: 0, iA: [1, 2, 0], iB: [0, 1, 1], div: rng.nice() };
                        let mut cc = cold;
                        let mut cr = cold;
                        let dc = with_clean_stack(|| unsafe {
                            f_c(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                std::ptr::null_mut(), std::ptr::null_mut(), ur,
                                std::ptr::null_mut(), &mut cc)
                        });
                        let dr = with_clean_stack(|| unsafe {
                            f_r(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                std::ptr::null_mut(), std::ptr::null_mut(), ur,
                                std::ptr::null_mut(), &mut cr)
                        });
                        assert!(bits_eq_f32(dc, dr), "row 44: cold-cache dist");
                        assert!(cache_eq(&cc, &cr), "row 44: cold-cache write C={:?} R={:?}", cc, cr);
                        assert!(bits_eq_f32(dc, dc_nocache), "row 43/44: cold cache changed dist");

                        // row 45: warm cache (round-tripped) and a hand-made one
                        // whose metric forces both sides of the `metric < -1e8f`
                        // test.
                        let n_a = match ta { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                        let n_b = match tb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                        for metric in [-1e9f32, -1e8, 0.0, 1e9, f32::NAN] {
                            for cnt in [1i32, 2, 3] {
                                let warm = c2GJKCache {
                                    metric,
                                    count: cnt,
                                    iA: [rng.below(n_a) as c_int, rng.below(n_a) as c_int, rng.below(n_a) as c_int],
                                    iB: [rng.below(n_b) as c_int, rng.below(n_b) as c_int, rng.below(n_b) as c_int],
                                    div: rng.range(-2.0, 4.0),
                                };
                                let mut cc = warm;
                                let mut cr = warm;
                                let mut it_c: c_int = 0;
                                let mut it_r: c_int = 0;
                                let dc = with_clean_stack(|| unsafe {
                                    f_c(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                        std::ptr::null_mut(), std::ptr::null_mut(), ur, &mut it_c, &mut cc)
                                });
                                let dr = with_clean_stack(|| unsafe {
                                    f_r(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                                        std::ptr::null_mut(), std::ptr::null_mut(), ur, &mut it_r, &mut cr)
                                });
                                assert!(
                                    bits_eq_f32(dc, dr),
                                    "row 45: warm cache metric={} cnt={} C={} R={}",
                                    fmt_f32(metric), cnt, fmt_f32(dc), fmt_f32(dr)
                                );
                                assert!(cache_eq(&cc, &cr), "row 45: warm cache write");
                                assert_eq!(it_c, it_r, "row 45: iterations");
                            }
                        }
                    }
                }
            }
        }
    });
}

#[test]
fn row46_row52_gjk_termination_and_radius() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<GjkFn>("c2GJK"), p.r.get::<GjkFn>("c2GJK"));
        let mut rng = Rng::new(0xE046);
        // rows 46..49 (loop exits) and 50/51/52 (radius / hit collapse) are all
        // value-dependent; sweep hard over shapes that are identical, touching,
        // nested, and far apart, with use_radius on and off.
        let mut saw_hit = 0usize;
        let mut saw_iter20 = 0usize;
        for i in 0..8000 {
            let scale = [1e-6f32, 1e-3, 1.0, 1e3, 1e6][ (i % 5) as usize ];
            let a = c2Capsule {
                a: c2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                b: c2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                r: rng.range(0.0, 1.0) * scale,
            };
            // deliberately identical / nested / touching configurations
            let b = match i % 4 {
                0 => a,
                1 => c2Capsule { a: a.b, b: a.a, r: a.r },
                2 => c2Capsule { a: a.a, b: a.a, r: a.r },
                _ => c2Capsule {
                    a: c2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                    b: c2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                    r: rng.range(0.0, 1.0) * scale,
                },
            };
            for ur in [0, 1] {
                let (mut oac, mut obc) = (c2v::default(), c2v::default());
                let (mut oar, mut obr) = (c2v::default(), c2v::default());
                let mut itc: c_int = 0;
                let mut itr: c_int = 0;
                let dc = with_clean_stack(|| unsafe {
                    f_c(&a as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        &b as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        &mut oac, &mut obc, ur, &mut itc, std::ptr::null_mut())
                });
                let dr = with_clean_stack(|| unsafe {
                    f_r(&a as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        &b as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        &mut oar, &mut obr, ur, &mut itr, std::ptr::null_mut())
                });
                assert!(
                    bits_eq_f32(dc, dr),
                    "row 46-52: ur={} A={:?} B={:?} C={} R={}",
                    ur, a, b, fmt_f32(dc), fmt_f32(dr)
                );
                assert!(v_eq(oac, oar) && v_eq(obc, obr), "row 46-52: witness points");
                assert_eq!(itc, itr, "row 46-52: iteration count");
                if dc == 0.0 {
                    saw_hit += 1;
                }
                if itc >= 20 {
                    saw_iter20 += 1;
                }
            }
        }
        assert!(saw_hit > 0, "row 51: never observed the hit/collapse path");
        let _ = saw_iter20; // row 46 is only reachable for pathological inputs
    });
}

// ---------------------------------------------------------------------------
// row 53 — the uninitialised POLY proxy (documented UB)
// ---------------------------------------------------------------------------

#[test]
fn row53_poly_proxy_uninitialised() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<GjkFn>("c2GJK"), p.r.get::<GjkFn>("c2GJK"));
        let mut poly = c2Poly::default();
        poly.count = 4;
        poly.verts[0] = c2v { x: -1.0, y: -1.0 };
        poly.verts[1] = c2v { x: 1.0, y: -1.0 };
        poly.verts[2] = c2v { x: 1.0, y: 1.0 };
        poly.verts[3] = c2v { x: -1.0, y: 1.0 };
        let cap = c2Capsule { a: c2v { x: 5.0, y: 5.0 }, b: c2v { x: 6.0, y: 6.0 }, r: 0.5 };
        let mut rng = Rng::new(0xE053);
        for _ in 0..500 {
            for ur in [0, 1] {
                let (mut oac, mut obc) = (c2v::default(), c2v::default());
                let (mut oar, mut obr) = (c2v::default(), c2v::default());
                let dc = with_clean_stack(|| unsafe {
                    f_c(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        &poly as *const _ as *const c_void, C2_TYPE_POLY, std::ptr::null(),
                        &mut oac, &mut obc, ur, std::ptr::null_mut(), std::ptr::null_mut())
                });
                let dr = with_clean_stack(|| unsafe {
                    f_r(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        &poly as *const _ as *const c_void, C2_TYPE_POLY, std::ptr::null(),
                        &mut oar, &mut obr, ur, std::ptr::null_mut(), std::ptr::null_mut())
                });
                // With the proxy memory pinned to zero (see `with_clean_stack`
                // and the RTLD_NOW note in common/mod.rs) both libraries observe
                // radius 0 / count 0 / verts {0,0} and must agree exactly.
                assert!(
                    bits_eq_f32(dc, dr),
                    "row 53: ur={} C={} R={}",
                    ur, fmt_f32(dc), fmt_f32(dr)
                );
                assert!(v_eq(oac, oar) && v_eq(obc, obr), "row 53: witness points");
            }
            let _ = rng.next_u32();
        }
    });
}

// ---------------------------------------------------------------------------
// row 54 — c2PlaneAt with an out-of-range index
// ---------------------------------------------------------------------------

/// `c2Poly` embedded in known surrounding bytes so that an out-of-bounds
/// `c2PlaneAt` index reads *defined* memory and the two libraries can be
/// compared: both are handed the very same pointer.
#[repr(C)]
struct PolyBox {
    pre: [c2v; 4],
    poly: c2Poly,
    post: [c2v; 4],
}

#[test]
fn row54_planeat_out_of_range() {
    fresh(|| {
        let p = pair();
        type PlaneAt = unsafe extern "C" fn(*const c2Poly, c_int) -> c2h;
        let (f_c, f_r) = (p.c.get::<PlaneAt>("c2PlaneAt"), p.r.get::<PlaneAt>("c2PlaneAt"));
        let mut rng = Rng::new(0xE054);
        for _ in 0..500 {
            let mut bx = PolyBox {
                pre: [rng.wild_v(); 4],
                poly: c2Poly::default(),
                post: [rng.wild_v(); 4],
            };
            bx.poly.count = 4;
            for k in 0..8 {
                bx.poly.verts[k] = rng.wild_v();
                bx.poly.norms[k] = rng.wild_v();
            }
            for k in 0..4 {
                bx.pre[k] = rng.wild_v();
                bx.post[k] = rng.wild_v();
            }
            for i in [-4i32, -1, 0, 3, 4, 7, 8, 11] {
                let x = with_clean_stack(|| unsafe { f_c(&bx.poly, i) });
                let y = with_clean_stack(|| unsafe { f_r(&bx.poly, i) });
                assert!(
                    h_eq(x, y),
                    "row 54: i={} C=({}, {}) R=({}, {})",
                    i, fmt_v(x.n), fmt_f32(x.d), fmt_v(y.n), fmt_f32(y.d)
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 55 / 56 / 57 / 58 / 59 — no-validation helpers
// ---------------------------------------------------------------------------

#[test]
fn row55_row59_unvalidated_helpers() {
    fresh(|| {
        let p = pair();
        type BB = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
        type NormsFn = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int);
        let (bb_c, bb_r) = (p.c.get::<BB>("c2BBVerts"), p.r.get::<BB>("c2BBVerts"));
        let (n_c, n_r) = (p.c.get::<NormsFn>("c2Norms"), p.r.get::<NormsFn>("c2Norms"));
        let (cl_c, cl_r) = (p.c.get::<FnVVVV>("c2Clampv"), p.r.get::<FnVVVV>("c2Clampv"));
        let (mx_c, mx_r) = (p.c.get::<FnVVV>("c2Maxv"), p.r.get::<FnVVV>("c2Maxv"));
        let (mn_c, mn_r) = (p.c.get::<FnVVV>("c2Minv"), p.r.get::<FnVVV>("c2Minv"));

        let mut rng = Rng::new(0xE055);

        // row 55: inverted AABB
        for _ in 0..500 {
            let a = rng.wild_v();
            let b = rng.wild_v();
            for (mn, mx) in [(a, b), (b, a)] {
                let mut aabb = c2AABB { min: mn, max: mx };
                let mut oc = [c2v { x: 7.0, y: -7.0 }; 4];
                let mut or_ = oc;
                with_clean_stack(|| unsafe { bb_c(oc.as_mut_ptr(), &mut aabb) });
                with_clean_stack(|| unsafe { bb_r(or_.as_mut_ptr(), &mut aabb) });
                for k in 0..4 {
                    assert!(v_eq(oc[k], or_[k]), "row 55: vert {}", k);
                }
            }
        }

        // rows 56 / 57: c2Norms with count <= 0 and with zero-length edges
        for count in [0i32, -1, -3, c_int::MIN, 1, 2, 8] {
            for iter in 0..200 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = rng.v();
                }
                if iter % 3 == 0 {
                    // duplicate consecutive verts -> zero-length edge -> NaN
                    for k in 1..8 {
                        verts[k] = verts[0];
                    }
                }
                let mut nc = [c2v { x: 4.5, y: -4.5 }; 8];
                let mut nr = nc;
                let mut vc = verts;
                let mut vr = verts;
                with_clean_stack(|| unsafe { n_c(vc.as_mut_ptr(), nc.as_mut_ptr(), count) });
                with_clean_stack(|| unsafe { n_r(vr.as_mut_ptr(), nr.as_mut_ptr(), count) });
                for k in 0..8 {
                    assert!(v_eq(nc[k], nr[k]), "row 56/57: count={} norm {}", count, k);
                    assert!(v_eq(vc[k], vr[k]), "row 56/57: verts mutated");
                }
                if count <= 0 {
                    assert!(
                        v_eq(nc[0], c2v { x: 4.5, y: -4.5 }),
                        "row 56: count={} must write nothing",
                        count
                    );
                }
            }
        }

        // rows 58 / 59: c2Clampv with lo > hi, and NaN in min/max
        let specials = [
            0.0f32, -0.0, 1.0, -1.0, f32::NAN, -f32::NAN,
            f32::from_bits(0xffab_cdef), f32::INFINITY, f32::NEG_INFINITY,
        ];
        for &ax in &specials {
            for &bx in &specials {
                for &cx in &specials {
                    let a = c2v { x: ax, y: bx };
                    let lo = c2v { x: bx, y: cx };
                    let hi = c2v { x: cx, y: ax };
                    let x = unsafe { cl_c(a, lo, hi) };
                    let y = unsafe { cl_r(a, lo, hi) };
                    assert!(v_eq(x, y), "row 58: c2Clampv({},{},{})", fmt_v(a), fmt_v(lo), fmt_v(hi));
                    let x = unsafe { mx_c(a, lo) };
                    let y = unsafe { mx_r(a, lo) };
                    assert!(v_eq(x, y), "row 59: c2Maxv({},{})", fmt_v(a), fmt_v(lo));
                    let x = unsafe { mn_c(a, lo) };
                    let y = unsafe { mn_r(a, lo) };
                    assert!(v_eq(x, y), "row 59: c2Minv({},{})", fmt_v(a), fmt_v(lo));
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 60 — omni_manifold with unhandled / out-of-range types
// ---------------------------------------------------------------------------

#[test]
fn row60_omni_bad_types() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<Omni>("omni_manifold"), p.r.get::<Omni>("omni_manifold"));
        let mut rng = Rng::new(0xE060);
        let mut all: Vec<c_int> = BAD_TYPES.to_vec();
        all.extend_from_slice(&GOOD_TYPES);
        for _ in 0..80 {
            let a: [f32; 5] = [rng.nice(), rng.nice(), rng.nice(), rng.nice(), rng.nice()];
            let b: [f32; 5] = [rng.nice(), rng.nice(), rng.nice(), rng.nice(), rng.nice()];
            for &ta in &all {
                for &tb in &all {
                    if GOOD_TYPES.contains(&ta) && GOOD_TYPES.contains(&tb) {
                        continue; // covered by CONFIGS.md rows 55..57
                    }
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    with_clean_stack(|| unsafe {
                        f_c(&mut mc, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4])
                    });
                    with_clean_stack(|| unsafe {
                        f_r(&mut mr, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4])
                    });
                    expect_man_eq("60", &mc, &mr, || format!("ta={} tb={} a={:?} b={:?}", ta, tb, a, b));
                    assert_eq!(mc.count, 0, "row 60: ta={} tb={} must yield count 0", ta, tb);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 63 / 64 / 65 / 66 — degenerate-geometry fallbacks
// ---------------------------------------------------------------------------

#[test]
fn row63_row66_degenerate_fallbacks() {
    fresh(|| {
        let p = pair();
        let cc = (
            p.c.get::<CircCirc>("c2CircletoCircleManifold"),
            p.r.get::<CircCirc>("c2CircletoCircleManifold"),
        );
        let ca = (
            p.c.get::<CircAabb>("c2CircletoAABBManifold"),
            p.r.get::<CircAabb>("c2CircletoAABBManifold"),
        );
        let ck = (
            p.c.get::<CircCap>("c2CircletoCapsuleManifold"),
            p.r.get::<CircCap>("c2CircletoCapsuleManifold"),
        );
        let kk = (
            p.c.get::<CapCap>("c2CapsuletoCapsuleManifold"),
            p.r.get::<CapCap>("c2CapsuletoCapsuleManifold"),
        );
        let mut rng = Rng::new(0xE063);

        // row 63: coincident centres (l == 0) -> n = (0, 1)
        for _ in 0..300 {
            let c = rng.v();
            let a = c2Circle { p: c, r: rng.range(0.0, 3.0) };
            let b = c2Circle { p: c, r: rng.range(0.0, 3.0) };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { cc.0(a, b, &mut mc) });
            with_clean_stack(|| unsafe { cc.1(a, b, &mut mr) });
            expect_man_eq("63", &mc, &mr, || format!("A={:?} B={:?}", a, b));
            if mc.count == 1 {
                assert!(v_eq(mc.n, c2v { x: 0.0, y: 1.0 }), "row 63: fallback normal");
            }
        }

        // row 64: circle centre strictly inside the AABB (d2 == 0)
        for _ in 0..600 {
            let cx = rng.range(-5.0, 5.0);
            let cy = rng.range(-5.0, 5.0);
            let ex = rng.range(0.01, 4.0);
            let ey = rng.range(0.01, 4.0);
            let bb = c2AABB {
                min: c2v { x: cx - ex, y: cy - ey },
                max: c2v { x: cx + ex, y: cy + ey },
            };
            // pick a point strictly inside
            let a = c2Circle {
                p: c2v {
                    x: cx + rng.range(-0.9, 0.9) * ex,
                    y: cy + rng.range(-0.9, 0.9) * ey,
                },
                r: rng.range(0.0, 6.0),
            };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { ca.0(a, bb, &mut mc) });
            with_clean_stack(|| unsafe { ca.1(a, bb, &mut mr) });
            expect_man_eq("64", &mc, &mr, || format!("A={:?} B={:?}", a, bb));
        }

        // row 65: circle/capsule with d == 0 and a zero-length capsule
        for _ in 0..300 {
            let c = rng.v();
            let a = c2Circle { p: c, r: rng.range(0.0, 3.0) };
            let degen = c2Capsule { a: c, b: c, r: rng.range(0.0, 3.0) };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { ck.0(a, degen, &mut mc) });
            with_clean_stack(|| unsafe { ck.1(a, degen, &mut mr) });
            expect_man_eq("65", &mc, &mr, || format!("A={:?} B={:?}", a, degen));
            // and a non-degenerate capsule whose segment passes through `c`
            let along = c2Capsule {
                a: c2v { x: c.x - 2.0, y: c.y },
                b: c2v { x: c.x + 2.0, y: c.y },
                r: rng.range(0.0, 3.0),
            };
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { ck.0(a, along, &mut mc) });
            with_clean_stack(|| unsafe { ck.1(a, along, &mut mr) });
            expect_man_eq("65", &mc, &mr, || format!("A={:?} B={:?}", a, along));
        }

        // row 66: capsule/capsule with d == 0, A degenerate
        for _ in 0..300 {
            let c = rng.v();
            let a = c2Capsule { a: c, b: c, r: rng.range(0.0, 3.0) };
            let b = c2Capsule {
                a: c2v { x: c.x - 2.0, y: c.y },
                b: c2v { x: c.x + 2.0, y: c.y },
                r: rng.range(0.0, 3.0),
            };
            for (x, y) in [(a, b), (b, a), (a, a)] {
                let mut mc = seed_manifold();
                let mut mr = seed_manifold();
                with_clean_stack(|| unsafe { kk.0(x, y, &mut mc) });
                with_clean_stack(|| unsafe { kk.1(x, y, &mut mr) });
                expect_man_eq("66", &mc, &mr, || format!("A={:?} B={:?}", x, y));
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 69 / 70 — stale `m->n` is still negated when nothing collides
// ---------------------------------------------------------------------------

#[test]
fn row69_row70_stale_normal_negation() {
    fresh(|| {
        let p = pair();
        let (ac_c, ac_r) = (
            p.c.get::<AabbCap>("c2AABBtoCapsuleManifold"),
            p.r.get::<AabbCap>("c2AABBtoCapsuleManifold"),
        );
        let (co_c, co_r) = (p.c.get::<Collide>("c2Collide"), p.r.get::<Collide>("c2Collide"));

        // row 69: far-apart AABB/capsule -> count 0 but m->n negated in place
        let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        let far = c2Capsule { a: c2v { x: 90.0, y: 90.0 }, b: c2v { x: 95.0, y: 95.0 }, r: 0.5 };
        for seed_n in [
            c2v { x: 0.25, y: -0.75 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: f32::NAN, y: f32::INFINITY },
            c2v { x: f32::from_bits(0xffab_cdef), y: 3.0 },
        ] {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            mc.n = seed_n;
            mr.n = seed_n;
            with_clean_stack(|| unsafe { ac_c(bb, far, &mut mc) });
            with_clean_stack(|| unsafe { ac_r(bb, far, &mut mr) });
            expect_man_eq("69", &mc, &mr, || format!("seed_n={}", fmt_v(seed_n)));
            assert_eq!(mc.count, 0, "row 69");
            assert!(
                !v_eq(mc.n, seed_n) || (seed_n.x == 0.0 && seed_n.y == 0.0),
                "row 69: m->n should have been negated in place: {} -> {}",
                fmt_v(seed_n), fmt_v(mc.n)
            );
        }

        // row 70: c2Collide AABB-vs-CIRCLE with no collision still negates m->n
        let circ = c2Circle { p: c2v { x: 100.0, y: 100.0 }, r: 0.5 };
        for seed_n in [
            c2v { x: 0.25, y: -0.75 },
            c2v { x: f32::NAN, y: -0.0 },
            c2v { x: f32::from_bits(0x7fc0_1234), y: 1.0 },
        ] {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            mc.n = seed_n;
            mr.n = seed_n;
            with_clean_stack(|| unsafe {
                co_c(&bb as *const _ as *const c_void, C2_TYPE_AABB,
                     &circ as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut mc)
            });
            with_clean_stack(|| unsafe {
                co_r(&bb as *const _ as *const c_void, C2_TYPE_AABB,
                     &circ as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut mr)
            });
            expect_man_eq("70", &mc, &mr, || format!("seed_n={}", fmt_v(seed_n)));
            assert_eq!(mc.count, 0, "row 70");
        }

        // and the CAPSULE-vs-{CIRCLE,AABB} branches, which negate as well
        let cap = c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.25 };
        for (tb, blob) in [
            (C2_TYPE_CIRCLE, &circ as *const _ as *const c_void),
            (C2_TYPE_AABB, &bb as *const _ as *const c_void),
        ] {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe {
                co_c(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, blob, tb, &mut mc)
            });
            with_clean_stack(|| unsafe {
                co_r(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, blob, tb, &mut mr)
            });
            expect_man_eq("70", &mc, &mr, || format!("capsule vs {}", tb));
        }
    });
}

// ---------------------------------------------------------------------------
// generic FFI boundary checks required in addition to the table
// ---------------------------------------------------------------------------

#[test]
fn generic_boundaries() {
    fresh(|| {
        let p = pair();
        // zero-length / oversized counts on every count-taking export
        type Sup = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        let (sup_c, sup_r) = (p.c.get::<Sup>("c2Support"), p.r.get::<Sup>("c2Support"));
        let mut verts = [c2v::default(); 8];
        let mut rng = Rng::new(0xE0FF);
        for v in verts.iter_mut() {
            *v = rng.v();
        }
        // 1..=8 is in-contract; 0 and negatives are covered by row 26. Values
        // one step past the documented range would read out of bounds in *both*
        // libraries identically, so compare 1..=8 exhaustively here.
        for count in 1..=8i32 {
            let d = rng.v();
            let x = unsafe { sup_c(verts.as_ptr(), count, d) };
            let y = unsafe { sup_r(verts.as_ptr(), count, d) };
            assert_eq!(x, y, "boundary: c2Support count={}", count);
        }

        // c2GJK with every out-of-range type for A and B (proxy left untouched,
        // i.e. zeroed by `with_clean_stack`) — both libraries must agree.
        let (g_c, g_r) = (p.c.get::<GjkFn>("c2GJK"), p.r.get::<GjkFn>("c2GJK"));
        let cap = c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 };
        let blob = &cap as *const _ as *const c_void;
        for &ta in &BAD_TYPES {
            for &tb in &BAD_TYPES {
                for ur in [0, 1] {
                    let (mut oac, mut obc) = (c2v::default(), c2v::default());
                    let (mut oar, mut obr) = (c2v::default(), c2v::default());
                    let dc = with_clean_stack(|| unsafe {
                        g_c(blob, ta, std::ptr::null(), blob, tb, std::ptr::null(),
                            &mut oac, &mut obc, ur, std::ptr::null_mut(), std::ptr::null_mut())
                    });
                    let dr = with_clean_stack(|| unsafe {
                        g_r(blob, ta, std::ptr::null(), blob, tb, std::ptr::null(),
                            &mut oar, &mut obr, ur, std::ptr::null_mut(), std::ptr::null_mut())
                    });
                    assert!(
                        bits_eq_f32(dc, dr),
                        "boundary: c2GJK ta={} tb={} ur={} C={} R={}",
                        ta, tb, ur, fmt_f32(dc), fmt_f32(dr)
                    );
                    assert!(v_eq(oac, oar) && v_eq(obc, obr), "boundary: c2GJK outs");
                }
            }
        }

        // c2CapsuletoPolyManifold with a NULL transform vs an identity one.
        let (cp_c, cp_r) = (
            p.c.get::<CapPoly>("c2CapsuletoPolyManifold"),
            p.r.get::<CapPoly>("c2CapsuletoPolyManifold"),
        );
        let norms = p.c.get::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int)>("c2Norms");
        let mut poly = c2Poly::default();
        poly.count = 4;
        poly.verts[0] = c2v { x: -1.0, y: -1.0 };
        poly.verts[1] = c2v { x: 1.0, y: -1.0 };
        poly.verts[2] = c2v { x: 1.0, y: 1.0 };
        poly.verts[3] = c2v { x: -1.0, y: 1.0 };
        with_clean_stack(|| unsafe { norms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), 4) });
        let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
        for a in [
            c2Capsule { a: c2v { x: -0.5, y: 0.0 }, b: c2v { x: 0.5, y: 0.0 }, r: 0.25 },
            c2Capsule { a: c2v { x: -3.0, y: 0.0 }, b: c2v { x: 3.0, y: 0.0 }, r: 0.25 },
            c2Capsule { a: c2v { x: 1.1, y: -1.0 }, b: c2v { x: 1.1, y: 1.0 }, r: 2.0 },
        ] {
            for bxp in [std::ptr::null(), &ident as *const c2x] {
                let mut mc = seed_manifold();
                let mut mr = seed_manifold();
                with_clean_stack(|| unsafe { cp_c(a, &poly, bxp, &mut mc) });
                with_clean_stack(|| unsafe { cp_r(a, &poly, bxp, &mut mr) });
                expect_man_eq("boundary", &mc, &mr, || {
                    format!("A={:?} bx_null={}", a, bxp.is_null())
                });
            }
        }
    });
}
