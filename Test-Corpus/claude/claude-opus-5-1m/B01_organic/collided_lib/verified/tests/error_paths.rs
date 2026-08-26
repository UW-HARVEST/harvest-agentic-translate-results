//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E6), plus the "documented
//! non-rejection" rows (N1..N5) that must NOT be treated as errors, plus the
//! generic FFI boundary cases (null pointers, out-of-range enum values, values
//! one step past the valid range).
//!
//! Every assertion compares the exact returned sentinel from both libraries,
//! not merely "both failed".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Is this pair of tag values the only thing the C `switch`es accept?
fn is_valid_pair(ta: c_int, tb: c_int) -> bool {
    matches!(ta, C2_TYPE_CIRCLE | C2_TYPE_AABB) && matches!(tb, C2_TYPE_CIRCLE | C2_TYPE_AABB)
}

/// Every invalid tag value used by the error tests (nothing in `{0,1}`).
fn invalid_types() -> Vec<c_int> {
    let mut v: Vec<c_int> = EDGE_TYPES
        .iter()
        .copied()
        .filter(|t| !matches!(*t, 0 | 1))
        .collect();
    v.push(u32::MAX as c_int); // -1 via unsigned reinterpretation
    v.push((1u32 << 31) as c_int); // i32::MIN via unsigned reinterpretation
    v.push(2); // first value one step past the documented range
    v.push(-1); // first value one step before it
    v.dedup();
    v
}

// ===========================================================================
// E1 — outer `default:`  (typeA invalid, typeB valid)
// ===========================================================================

#[test]
fn err_e1_outer_default_bad_typea() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE1);
    for ta in invalid_types() {
        for &tb in [C2_TYPE_CIRCLE, C2_TYPE_AABB].iter() {
            for _ in 0..16 {
                // real, fully-initialised payloads: the C must return 0 without
                // ever looking at them
                let ab = aabb_bytes(&rng.aabb_grid());
                let cb = circle_bytes(&rng.circle_grid());
                let pa = ab.as_ptr() as *const c_void;
                let pb = cb.as_ptr() as *const c_void;
                let rc = unsafe { (c.collided)(pa, ta, pb, tb) };
                let rr = unsafe { (r.collided)(pa, ta, pb, tb) };
                assert_eq!(rc, 0, "C should reject typeA={ta} with 0, got {rc}");
                same("collided E1", (ta, tb), rc, rr);
            }
        }
    }
}

// ===========================================================================
// E2 — inner `default:` under `case C2_TYPE_CIRCLE`
// ===========================================================================

#[test]
fn err_e2_circle_bad_typeb() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE2);
    for tb in invalid_types() {
        for _ in 0..32 {
            let cb = circle_bytes(&rng.circle_grid());
            let other = circle_bytes(&rng.circle_grid());
            let pa = cb.as_ptr() as *const c_void;
            let pb = other.as_ptr() as *const c_void;
            let rc = unsafe { (c.collided)(pa, C2_TYPE_CIRCLE, pb, tb) };
            let rr = unsafe { (r.collided)(pa, C2_TYPE_CIRCLE, pb, tb) };
            assert_eq!(rc, 0, "C should reject CIRCLE/typeB={tb} with 0, got {rc}");
            same("collided E2", tb, rc, rr);
        }
    }
}

// ===========================================================================
// E3 — inner `default:` under `case C2_TYPE_AABB`
// ===========================================================================

#[test]
fn err_e3_aabb_bad_typeb() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE3);
    for tb in invalid_types() {
        for _ in 0..32 {
            let ab = aabb_bytes(&rng.aabb_grid());
            let other = aabb_bytes(&rng.aabb_grid());
            let pa = ab.as_ptr() as *const c_void;
            let pb = other.as_ptr() as *const c_void;
            let rc = unsafe { (c.collided)(pa, C2_TYPE_AABB, pb, tb) };
            let rr = unsafe { (r.collided)(pa, C2_TYPE_AABB, pb, tb) };
            assert_eq!(rc, 0, "C should reject AABB/typeB={tb} with 0, got {rc}");
            same("collided E3", tb, rc, rr);
        }
    }
}

// ===========================================================================
// E4 — both tags invalid
// ===========================================================================

#[test]
fn err_e4_both_types_invalid() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE4);
    for ta in invalid_types() {
        for tb in invalid_types() {
            let ab = aabb_bytes(&rng.aabb_grid());
            let cb = circle_bytes(&rng.circle_grid());
            let pa = ab.as_ptr() as *const c_void;
            let pb = cb.as_ptr() as *const c_void;
            let rc = unsafe { (c.collided)(pa, ta, pb, tb) };
            let rr = unsafe { (r.collided)(pa, ta, pb, tb) };
            assert_eq!(rc, 0, "C should reject ({ta},{tb}) with 0, got {rc}");
            same("collided E4", (ta, tb), rc, rr);
        }
    }
}

// ===========================================================================
// E5 — null pointers together with an invalid tag (the only defined null case)
// ===========================================================================

#[test]
fn err_e5_null_pointers_with_invalid_type() {
    let (c, r) = apis();
    let nul: *const c_void = std::ptr::null();
    let payload = aabb_bytes(&C2Aabb::default());
    let good = payload.as_ptr() as *const c_void;

    for ta in invalid_types() {
        for &tb in [C2_TYPE_CIRCLE, C2_TYPE_AABB, 7, -1].iter() {
            // A NULL, tag invalid -> A never dereferenced
            let rc = unsafe { (c.collided)(nul, ta, good, tb) };
            let rr = unsafe { (r.collided)(nul, ta, good, tb) };
            assert_eq!(rc, 0);
            same("collided E5 A=NULL", (ta, tb), rc, rr);

            // both NULL, tag invalid
            let rc = unsafe { (c.collided)(nul, ta, nul, tb) };
            let rr = unsafe { (r.collided)(nul, ta, nul, tb) };
            assert_eq!(rc, 0);
            same("collided E5 A=B=NULL", (ta, tb), rc, rr);
        }
    }

    // B NULL with a valid typeA but an invalid typeB: the inner `default:`
    // returns before B is dereferenced, for both CIRCLE and AABB arms.
    for &ta in [C2_TYPE_CIRCLE, C2_TYPE_AABB].iter() {
        for tb in invalid_types() {
            let rc = unsafe { (c.collided)(good, ta, nul, tb) };
            let rr = unsafe { (r.collided)(good, ta, nul, tb) };
            assert_eq!(rc, 0);
            same("collided E5 B=NULL", (ta, tb), rc, rr);
        }
    }
}

// ===========================================================================
// E6 — exhaustive enum cross-product sweep across the FFI boundary
// ===========================================================================

#[test]
fn err_e6_enum_cross_product_sweep() {
    let (c, r) = apis();
    let mut types: Vec<c_int> = (-8..=8).collect();
    types.extend_from_slice(&[
        100,
        255,
        256,
        65_536,
        c_int::MAX,
        c_int::MIN,
        u32::MAX as c_int,
        (u32::MAX - 1) as c_int,
        (1u32 << 31) as c_int,
    ]);

    let mut rng = Rng::new(0xE6);
    for &ta in types.iter() {
        for &tb in types.iter() {
            // 16 bytes each, so both a c2Circle and a c2AABB read is in-bounds
            let ab = aabb_bytes(&rng.aabb_grid());
            let bb = aabb_bytes(&rng.aabb_grid());
            let pa = ab.as_ptr() as *const c_void;
            let pb = bb.as_ptr() as *const c_void;
            let rc = unsafe { (c.collided)(pa, ta, pb, tb) };
            let rr = unsafe { (r.collided)(pa, ta, pb, tb) };
            same("collided E6", (ta, tb), rc, rr);
            if !is_valid_pair(ta, tb) {
                assert_eq!(rc, 0, "C must return 0 for invalid tag pair ({ta},{tb})");
                assert_eq!(rr, 0, "Rust must return 0 for invalid tag pair ({ta},{tb})");
            }
            // whatever the value, both libs must return a plain 0/1 boolean
            assert!(rc == 0 || rc == 1, "C returned non-boolean {rc}");
            assert!(rr == 0 || rr == 1, "Rust returned non-boolean {rr}");
        }
    }
}

// ===========================================================================
// N1..N5 — documented NON-rejections (must behave identically, not error)
// ===========================================================================

#[test]
fn nonerr_n1_negative_radii() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1111);
    let cir = |x: f32, y: f32, r: f32| C2Circle { p: C2v { x, y }, r };
    let cases = [
        (cir(0.0, 0.0, -1.0), cir(1.0, 0.0, -1.0)),
        (cir(0.0, 0.0, -2.0), cir(1.0, 0.0, 0.0)),
        (cir(0.0, 0.0, -1.0), cir(1.0, 0.0, 1.0)), // rA+rB == 0 -> r2 == 0
        (cir(0.0, 0.0, -5.0), cir(1.0, 0.0, 1.0)),
        (cir(0.0, 0.0, f32::MIN), cir(1.0, 0.0, f32::MAX)),
    ];
    for (a, b) in cases {
        same(
            "N1 c2CircletoCircle negative r",
            (a, b),
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
        );
    }
    for _ in 0..N {
        let mut a = rng.circle_grid();
        let mut b = rng.circle_grid();
        a.r = -a.r.abs();
        b.r = -b.r.abs();
        same(
            "N1 c2CircletoCircle negative r (random)",
            (a, b),
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
        );
        let bx = rng.aabb_grid();
        same(
            "N1 c2CircletoAABB negative r (random)",
            (a, bx),
            (c.c2CircletoAABB)(a, bx),
            (r.c2CircletoAABB)(a, bx),
        );
    }
}

#[test]
fn nonerr_n2_inverted_aabb() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2222);
    for _ in 0..N * 2 {
        let a = rng.circle_grid();
        // deliberately inverted: min strictly greater than max
        let lo = rng.vec_grid();
        let b = C2Aabb {
            min: C2v { x: lo.x + 1.0, y: lo.y + 1.0 },
            max: C2v { x: lo.x - 1.0, y: lo.y - 1.0 },
        };
        same(
            "N2 c2CircletoAABB inverted",
            (a, b),
            (c.c2CircletoAABB)(a, b),
            (r.c2CircletoAABB)(a, b),
        );
        // and through the dispatcher, both tag orders
        collided_bytes_both(&circle_bytes(&a), C2_TYPE_CIRCLE, &aabb_bytes(&b), C2_TYPE_AABB, 0, 0, (a, b));
        collided_bytes_both(&aabb_bytes(&b), C2_TYPE_AABB, &circle_bytes(&a), C2_TYPE_CIRCLE, 0, 0, (b, a));
    }
}

#[test]
fn nonerr_n3_inverted_degenerate_aabb() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3333);
    for _ in 0..N * 2 {
        let p = rng.vec_grid();
        let q = rng.vec_grid();
        let degenerate = C2Aabb { min: p, max: p };
        let inverted = C2Aabb {
            min: C2v { x: q.x + 1.0, y: q.y + 1.0 },
            max: C2v { x: q.x - 1.0, y: q.y - 1.0 },
        };
        for &(a, b) in [
            (degenerate, inverted),
            (inverted, degenerate),
            (degenerate, degenerate),
            (inverted, inverted),
        ]
        .iter()
        {
            same(
                "N3 c2AABBtoAABB degenerate/inverted",
                (a, b),
                (c.c2AABBtoAABB)(a, b),
                (r.c2AABBtoAABB)(a, b),
            );
            collided_bytes_both(&aabb_bytes(&a), C2_TYPE_AABB, &aabb_bytes(&b), C2_TYPE_AABB, 0, 0, (a, b));
        }
    }
}

#[test]
fn nonerr_n4_nan_operands() {
    let (c, r) = apis();
    // Every NaN encoding we can think of, in every operand slot.
    let nans: [f32; 6] = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // sNaN
        f32::from_bits(0xff80_0001),
        f32::from_bits(0x7fff_ffff),
        f32::from_bits(0xffc0_dead),
    ];
    for &n in nans.iter() {
        for slot in 0..4usize {
            let mut a = C2v { x: 1.0, y: 2.0 };
            let mut b = C2v { x: 3.0, y: 4.0 };
            match slot {
                0 => a.x = n,
                1 => a.y = n,
                2 => b.x = n,
                _ => b.y = n,
            }
            same("N4 c2Maxv", (a, b, slot), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            same("N4 c2Minv", (a, b, slot), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
            same("N4 c2Sub", (a, b, slot), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            same("N4 c2Dot", (a, b, slot), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
            let mid = C2v { x: 0.0, y: 0.0 };
            same("N4 c2Clampv/a", (a, b, slot), (c.c2Clampv)(a, mid, b), (r.c2Clampv)(a, mid, b));
            same("N4 c2Clampv/lo", (a, b, slot), (c.c2Clampv)(mid, a, b), (r.c2Clampv)(mid, a, b));
            same("N4 c2Clampv/hi", (a, b, slot), (c.c2Clampv)(mid, b, a), (r.c2Clampv)(mid, b, a));

            let ca = C2Circle { p: a, r: n };
            let cb = C2Circle { p: b, r: 1.0 };
            same("N4 circle/circle", (ca, cb), (c.c2CircletoCircle)(ca, cb), (r.c2CircletoCircle)(ca, cb));
            let bx = C2Aabb { min: a, max: b };
            same("N4 circle/aabb", (cb, bx), (c.c2CircletoAABB)(cb, bx), (r.c2CircletoAABB)(cb, bx));
            let by = C2Aabb { min: b, max: a };
            same("N4 aabb/aabb", (bx, by), (c.c2AABBtoAABB)(bx, by), (r.c2AABBtoAABB)(bx, by));
            collided_bytes_both(&circle_bytes(&ca), C2_TYPE_CIRCLE, &circle_bytes(&cb), C2_TYPE_CIRCLE, 0, 0, (ca, cb));
            collided_bytes_both(&circle_bytes(&ca), C2_TYPE_CIRCLE, &aabb_bytes(&bx), C2_TYPE_AABB, 0, 0, (ca, bx));
            collided_bytes_both(&aabb_bytes(&bx), C2_TYPE_AABB, &circle_bytes(&ca), C2_TYPE_CIRCLE, 0, 0, (bx, ca));
            collided_bytes_both(&aabb_bytes(&bx), C2_TYPE_AABB, &aabb_bytes(&by), C2_TYPE_AABB, 0, 0, (bx, by));
        }
    }
}

#[test]
fn nonerr_n5_infinities() {
    let (c, r) = apis();
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;
    let vals = [inf, ninf, 0.0f32, -0.0f32, f32::MAX, f32::MIN, 1e30, -1e30];
    for &ax in vals.iter() {
        for &bx in vals.iter() {
            for &ay in vals.iter() {
                for &by in vals.iter() {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    same("N5 c2Sub", (a, b), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                    same("N5 c2Dot", (a, b), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                    same("N5 c2Maxv", (a, b), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                    same("N5 c2Minv", (a, b), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                    let ca = C2Circle { p: a, r: bx };
                    let cb = C2Circle { p: b, r: ax };
                    same("N5 circle/circle", (ca, cb), (c.c2CircletoCircle)(ca, cb), (r.c2CircletoCircle)(ca, cb));
                    let bx2 = C2Aabb { min: a, max: b };
                    same("N5 circle/aabb", (ca, bx2), (c.c2CircletoAABB)(ca, bx2), (r.c2CircletoAABB)(ca, bx2));
                    same("N5 aabb/aabb", (bx2, bx2), (c.c2AABBtoAABB)(bx2, bx2), (r.c2AABBtoAABB)(bx2, bx2));
                }
            }
        }
    }
}

// ===========================================================================
// Generic FFI-boundary sanity: results are always exactly 0 or 1
// ===========================================================================

#[test]
fn boundary_return_values_are_boolean() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xB007);
    for _ in 0..N {
        let a = rng.circle_wild();
        let b = rng.circle_wild();
        let ba = rng.aabb_wild();
        let bb = rng.aabb_wild();
        for (n, cv, rv) in [
            ("c2CircletoCircle", (c.c2CircletoCircle)(a, b), (r.c2CircletoCircle)(a, b)),
            ("c2CircletoAABB", (c.c2CircletoAABB)(a, bb), (r.c2CircletoAABB)(a, bb)),
            ("c2AABBtoAABB", (c.c2AABBtoAABB)(ba, bb), (r.c2AABBtoAABB)(ba, bb)),
        ] {
            assert!(cv == 0 || cv == 1, "{n}: C returned {cv}");
            assert!(rv == 0 || rv == 1, "{n}: Rust returned {rv}");
            assert_eq!(cv, rv, "{n} diverged for {a:?} {b:?} {ba:?} {bb:?}");
        }
    }
}
