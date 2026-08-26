//! Phase C — ERRORS.md rows 1-70: the error / rejection / degenerate-input surface.
//!
//! `c_src/src/lib.c` has NO error codes, NO asserts and NO `errno` use (verified
//! by grep), so "the same error" here means "the same sentinel / degenerate
//! result / same untouched-output behaviour", compared bit-for-bit — never
//! merely "both failed somehow".
//!
//! Test names match the `test` column of ERRORS.md one-for-one.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_void;

fn shapes() -> (Circle, AABB, Capsule) {
    (
        Circle { p: V::new(1.5, -2.5), r: 1.25 },
        AABB { min: V::new(-2.0, -3.0), max: V::new(4.0, 5.0) },
        Capsule { a: V::new(6.0, 1.0), b: V::new(9.0, -4.0), r: 0.75 },
    )
}

/// All nine (typeA, typeB) shape-pointer/type pairs from one shape set.
fn all_pairs<'a>(
    c: &'a Circle,
    bb: &'a AABB,
    cap: &'a Capsule,
) -> Vec<(*const c_void, i32, *const c_void, i32)> {
    let s: [(*const c_void, i32); 3] = [
        (c as *const Circle as *const c_void, C2_TYPE_CIRCLE),
        (bb as *const AABB as *const c_void, C2_TYPE_AABB),
        (cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE),
    ];
    let mut v = Vec::new();
    for &(pa, ta) in s.iter() {
        for &(pb, tb) in s.iter() {
            v.push((pa, ta, pb, tb));
        }
    }
    v
}

// ===========================================================================
// Rows 1-3 — NULL transforms
// ===========================================================================

#[test]
fn err_gjk_null_ax() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    let bx = X { p: V::new(3.0, -1.0), r: R { c: 0.6, s: 0.8 } };
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, Some(&bx), ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, Some(&bx), ur, true, true, true, None) };
            gjk_same("row1 null ax_ptr", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            assert!(co.dist.is_finite(), "expected a finite distance, got {}", co.dist);
        }
    }
}

#[test]
fn err_gjk_null_bx() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    let ax = X { p: V::new(-2.0, 4.0), r: R { c: 0.8, s: -0.6 } };
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, ta, Some(&ax), pb, tb, None, ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, ta, Some(&ax), pb, tb, None, ur, true, true, true, None) };
            gjk_same("row2 null bx_ptr", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
        }
    }
}

#[test]
fn err_gjk_null_both_x() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    let ident = X::IDENTITY;
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
            gjk_same("row3 both transforms NULL", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            // must be indistinguishable from passing an explicit identity
            let ce = unsafe {
                gjk_call(&c, pa, ta, Some(&ident), pb, tb, Some(&ident), ur, true, true, true, None)
            };
            gjk_same("row3 NULL == explicit identity", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ce);
        }
    }
}

// ===========================================================================
// Rows 4-7 — NULL out-parameters
// ===========================================================================

#[test]
fn err_gjk_null_outA() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 1, false, true, true, None) };
        let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, 1, false, true, true, None) };
        gjk_same("row4 outA NULL", &format!("ta={ta} tb={tb}"), &co, &ro);
        assert!(co.a_untouched && ro.a_untouched, "outA must remain untouched");
        assert!(!co.b_untouched && !ro.b_untouched, "outB must still be written");
        assert!(!co.it_untouched && !ro.it_untouched, "iterations must still be written");
    }
}

#[test]
fn err_gjk_null_outB() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, false, true, None) };
        let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, 1, true, false, true, None) };
        gjk_same("row5 outB NULL", &format!("ta={ta} tb={tb}"), &co, &ro);
        assert!(co.b_untouched && ro.b_untouched, "outB must remain untouched");
        assert!(!co.a_untouched && !ro.a_untouched, "outA must still be written");
    }
}

#[test]
fn err_gjk_null_iterations() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, false, None) };
        let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, 1, true, true, false, None) };
        gjk_same("row6 iterations NULL", &format!("ta={ta} tb={tb}"), &co, &ro);
        assert!(co.it_untouched && ro.it_untouched, "iterations must remain untouched");
    }
}

#[test]
fn err_gjk_all_null_outputs() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, false, false, false, None) };
            let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, false, false, false, None) };
            gjk_same("row7 every out-param NULL", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            assert!(co.a_untouched && co.b_untouched && co.it_untouched);
            // the float return must still be the real distance
            let full = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
            assert_eq!(
                co.dist.to_bits(), full.dist.to_bits(),
                "return value must not depend on which out-params are NULL"
            );
        }
    }
}

// ===========================================================================
// Rows 8-11, 68-70 — the cache
// ===========================================================================

#[test]
fn err_gjk_null_cache() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, true, None) };
        let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, 1, true, true, true, None) };
        gjk_same("row8 cache NULL", &format!("ta={ta} tb={tb}"), &co, &ro);
        assert!(co.cache.is_none());
    }
}

#[test]
fn err_gjk_cache_count_zero() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    // count == 0 but every other field poisoned: `!!count` must short-circuit
    // BEFORE any of them is used.
    let poisoned_cold = GJKCache {
        metric: f32::NAN,
        count: 0,
        iA: [99, -7, i32::MAX],
        iB: [-99, 7, i32::MIN],
        div: 0.0,
    };
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let co = unsafe {
            gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, true, Some(poisoned_cold))
        };
        let ro = unsafe {
            gjk_call(&r, pa, ta, None, pb, tb, None, 1, true, true, true, Some(poisoned_cold))
        };
        gjk_same("row9 cache count==0", &format!("ta={ta} tb={tb}"), &co, &ro);
        // and it must equal the no-cache result
        let nc = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, true, None) };
        assert_eq!(
            co.dist.to_bits(), nc.dist.to_bits(),
            "a count==0 cache must behave exactly like no cache"
        );
    }
}

#[test]
fn err_gjk_cache_metric_guard_reuse() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    // Ordinary warm cache: metric >= -1e8, so `metric < -1e8f` is FALSE, the
    // whole `&&` is false, `!(...)` is true -> cache_was_read = 1 (reuse).
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        for count in 1..=3i32 {
            for &metric_old in &[0.0f32, 1.0, -1.0, 12.5, -12.5, 1e7, -1e7, f32::NAN] {
                let cache = GJKCache {
                    metric: metric_old,
                    count,
                    iA: [0, 0, 0],
                    iB: [0, 0, 0],
                    div: 1.0,
                };
                let co = unsafe {
                    gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, true, Some(cache))
                };
                let ro = unsafe {
                    gjk_call(&r, pa, ta, None, pb, tb, None, 1, true, true, true, Some(cache))
                };
                gjk_same(
                    "row10 cache reuse branch",
                    &format!("ta={ta} tb={tb} count={count} metric_old={metric_old:?}"),
                    &co,
                    &ro,
                );
            }
        }
    }
}

#[test]
fn err_gjk_cache_metric_guard_reject() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // To take the FALSE side of `!(min_metric < max_metric*2 && metric < -1e8f)`
    // we need BOTH:  simplex metric < -1e8   AND   min_metric < max_metric*2.
    // Huge coordinates make the count==3 determinant metric hugely negative, and
    // a large POSITIVE cached metric_old makes max_metric*2 dominate.
    let bb = AABB { min: V::new(-1.0e5, -1.0e5), max: V::new(1.0e5, 1.0e5) };
    let bb2 = AABB { min: V::new(-2.0e5, -2.0e5), max: V::new(2.0e5, 2.0e5) };
    let pa = &bb as *const AABB as *const c_void;
    let pb = &bb2 as *const AABB as *const c_void;

    let mut hit_reject = 0usize;
    // every ordering of 3 distinct AABB corner indices, both signs of area
    for ia in 0..4i32 {
        for ib in 0..4i32 {
            for ic in 0..4i32 {
                for ja in 0..4i32 {
                    for &metric_old in &[1.0e9f32, 1.0e12, 3.4e38] {
                        let cache = GJKCache {
                            metric: metric_old,
                            count: 3,
                            iA: [ia, ib, ic],
                            iB: [ja, (ja + 1) % 4, (ja + 2) % 4],
                            div: 1.0,
                        };
                        let co = unsafe {
                            gjk_call(&c, pa, C2_TYPE_AABB, None, pb, C2_TYPE_AABB, None, 1, true, true, true, Some(cache))
                        };
                        let ro = unsafe {
                            gjk_call(&r, pa, C2_TYPE_AABB, None, pb, C2_TYPE_AABB, None, 1, true, true, true, Some(cache))
                        };
                        gjk_same(
                            "row11 cache reject branch",
                            &format!("iA=[{ia},{ib},{ic}] iB0={ja} metric_old={metric_old:?}"),
                            &co,
                            &ro,
                        );
                        // If the cache was REJECTED, the run is a cold start and
                        // must match the count==0 result exactly.
                        let cold = unsafe {
                            gjk_call(&c, pa, C2_TYPE_AABB, None, pb, C2_TYPE_AABB, None, 1, true, true, true,
                                     Some(GJKCache::default()))
                        };
                        if co.dist.to_bits() == cold.dist.to_bits()
                            && co.cache.map(|x| x.count) == cold.cache.map(|x| x.count)
                        {
                            hit_reject += 1;
                        }
                    }
                }
            }
        }
    }
    // Sanity: the configuration space above really does contain cache rejections.
    assert!(hit_reject > 0, "expected at least one cache-reject (cold restart) case");
    eprintln!("row11: {hit_reject} configurations behaved as a cold restart");
}

#[test]
fn err_gjk_cache_warm_indices() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    // Row 68: warm cache with count 1..3 and EVERY in-range index combination
    // for each proxy's vertex count (1 / 4 / 2).
    let sets: [(*const c_void, i32, i32); 3] = [
        (&ci as *const Circle as *const c_void, C2_TYPE_CIRCLE, 1),
        (&bb as *const AABB as *const c_void, C2_TYPE_AABB, 4),
        (&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE, 2),
    ];
    for &(pa, ta, na) in sets.iter() {
        for &(pb, tb, nb) in sets.iter() {
            for count in 1..=3i32 {
                for i0 in 0..na {
                    for j0 in 0..nb {
                        let cache = GJKCache {
                            metric: 1.0,
                            count,
                            iA: [i0, (i0 + 1) % na, (i0 + 2) % na],
                            iB: [j0, (j0 + 1) % nb, (j0 + 2) % nb],
                            div: 2.0,
                        };
                        for &ur in &[0i32, 1] {
                            let co = unsafe {
                                gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, Some(cache))
                            };
                            let ro = unsafe {
                                gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, Some(cache))
                            };
                            gjk_same(
                                "row68 warm cache in-range indices",
                                &format!("ta={ta} tb={tb} count={count} i0={i0} j0={j0} ur={ur}"),
                                &co,
                                &ro,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn err_gjk_cache_negative_count() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    // `!!count` is TRUE for a negative count, so the C enters the warm branch,
    // runs `for (i = 0; i < count; ...)` zero times, and sets s.count = count.
    // Every later `switch` then falls to its default arm.
    for &count in &[-1i32, -2, -3, i32::MIN + 1] {
        for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
            for &div in &[1.0f32, 0.0, -3.0, f32::NAN] {
                let cache = GJKCache { metric: 5.0, count, iA: [0; 3], iB: [0; 3], div };
                let co = unsafe {
                    gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, true, Some(cache))
                };
                let ro = unsafe {
                    gjk_call(&r, pa, ta, None, pb, tb, None, 1, true, true, true, Some(cache))
                };
                gjk_same(
                    "row69 negative cache count",
                    &format!("count={count} ta={ta} tb={tb} div={div:?}"),
                    &co,
                    &ro,
                );
            }
        }
    }
}

#[test]
fn err_gjk_cache_zero_div() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e-45] {
        for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
            for count in 1..=3i32 {
                let cache = GJKCache { metric: 1.0, count, iA: [0; 3], iB: [0; 3], div };
                for &ur in &[0i32, 1] {
                    let co = unsafe {
                        gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, Some(cache))
                    };
                    let ro = unsafe {
                        gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, Some(cache))
                    };
                    gjk_same(
                        "row70 cache div == 0 / NaN / Inf",
                        &format!("div={div:?} ta={ta} tb={tb} count={count} ur={ur}"),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 12-15 — out-of-range enum values (the classic FFI blind spot)
// ===========================================================================

/// Out-of-range C enum values. A C enum accepts any `int`, so these are all
/// real inputs the library must handle identically.
const BAD_TYPES: &[i32] = &[
    3,
    4,
    -1,
    -2,
    99,
    255,
    256,
    1000,
    i32::MAX,
    i32::MIN,
    i32::MIN + 1,
    0x7fff_ffff,
    -0x8000_0000i64 as i32,
];

#[test]
fn err_makeproxy_invalid_type() {
    let l = libs();
    let (c, r) = l.get::<FnMakeProxy>("c2MakeProxy");
    let (ci, bb, cap) = shapes();
    let shape_ptrs: [*const c_void; 3] = [
        &ci as *const Circle as *const c_void,
        &bb as *const AABB as *const c_void,
        &cap as *const Capsule as *const c_void,
    ];
    // Row 14: with a CALLER-OWNED proxy the behaviour is fully defined — the
    // switch has no `default:`, so the buffer must be left byte-for-byte intact.
    for &t in BAD_TYPES {
        for &sp in shape_ptrs.iter() {
            let mut cp: Proxy = poisoned();
            let mut rp: Proxy = poisoned();
            let before: Proxy = poisoned();
            unsafe {
                c(sp, t, &mut cp);
                r(sp, t, &mut rp);
            }
            ck_bytes!("row14 c2MakeProxy invalid type", cp, rp, "type={t}");
            ck_bytes!("row14 proxy must be untouched", cp, before, "type={t}");
        }
    }
}

#[test]
fn err_makeproxy_partial_write() {
    let l = libs();
    let (c, r) = l.get::<FnMakeProxy>("c2MakeProxy");
    let (ci, bb, cap) = shapes();
    // Row 15: for each VALID type, only the fields that arm assigns may change;
    // the remaining `verts[count..8]` must keep their poison bytes.
    let cases: [(*const c_void, i32, usize); 3] = [
        (&ci as *const Circle as *const c_void, C2_TYPE_CIRCLE, 1),
        (&bb as *const AABB as *const c_void, C2_TYPE_AABB, 4),
        (&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE, 2),
    ];
    for &(sp, t, n) in cases.iter() {
        let mut cp: Proxy = poisoned();
        let mut rp: Proxy = poisoned();
        unsafe {
            c(sp, t, &mut cp);
            r(sp, t, &mut rp);
        }
        ck_bytes!("row15 c2MakeProxy partial write", cp, rp, "type={t}");
        assert_eq!(cp.count as usize, n, "type {t} should give count {n}");
        for k in n..8 {
            assert_eq!(
                cp.verts[k].x.to_bits(), 0xA5A5_A5A5,
                "type {t}: verts[{k}] must keep its poison (C never writes it)"
            );
            assert_eq!(rp.verts[k].x.to_bits(), 0xA5A5_A5A5);
        }
    }
}

/// Rows 12/13 — out-of-range `typeA`/`typeB` passed to `c2GJK`.
///
/// NOTE: inside `c2GJK` the proxy is the uninitialised local `c2Proxy pA;`
/// (L371) and `c2MakeProxy` leaves it untouched for an unknown type, so the C
/// then reads INDETERMINATE stack values. That is C undefined behaviour, so the
/// numeric outputs are deliberately NOT asserted equal here — only that neither
/// library crashes, and that the *defined* observable (which out-params get
/// written) still agrees. The byte-exact version of this behaviour is row 14.
#[test]
fn err_gjk_invalid_typeA() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    let pb = &bb as *const AABB as *const c_void;
    for &t in BAD_TYPES {
        for &pa in [
            &ci as *const Circle as *const c_void,
            &cap as *const Capsule as *const c_void,
        ]
        .iter()
        {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, t, None, pb, C2_TYPE_AABB, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, t, None, pb, C2_TYPE_AABB, None, ur, true, true, true, None) };
                assert_eq!(
                    co.a_untouched, ro.a_untouched,
                    "typeA={t}: outA written-ness must agree"
                );
                assert_eq!(co.b_untouched, ro.b_untouched, "typeA={t}");
                assert_eq!(co.it_untouched, ro.it_untouched, "typeA={t}");
                assert!(
                    (0..=20).contains(&co.iters) && (0..=20).contains(&ro.iters),
                    "typeA={t}: iteration count must stay in range (C={}, Rust={})",
                    co.iters, ro.iters
                );
            }
        }
    }
}

#[test]
fn err_gjk_invalid_typeB() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    let pa = &bb as *const AABB as *const c_void;
    for &t in BAD_TYPES {
        for &pb in [
            &ci as *const Circle as *const c_void,
            &cap as *const Capsule as *const c_void,
        ]
        .iter()
        {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, C2_TYPE_AABB, None, pb, t, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, C2_TYPE_AABB, None, pb, t, None, ur, true, true, true, None) };
                assert_eq!(co.a_untouched, ro.a_untouched, "typeB={t}");
                assert_eq!(co.b_untouched, ro.b_untouched, "typeB={t}");
                assert_eq!(co.it_untouched, ro.it_untouched, "typeB={t}");
                assert!(
                    (0..=20).contains(&co.iters) && (0..=20).contains(&ro.iters),
                    "typeB={t}: iteration count out of range"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 16-21 — use_radius and the radius-correction branches
// ===========================================================================

#[test]
fn err_gjk_use_radius_zero() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 0, true, true, true, None) };
        let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, 0, true, true, true, None) };
        gjk_same("row16 use_radius == 0", &format!("ta={ta} tb={tb}"), &co, &ro);
    }
}

#[test]
fn err_gjk_use_radius_truthy() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let (ci, bb, cap) = shapes();
    // C tests `if (use_radius)`, so every nonzero int must behave like 1.
    let truthy: &[i32] = &[1, 2, -1, 7, 0x100, i32::MAX, i32::MIN, -0x8000];
    for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
        let one = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, 1, true, true, true, None) };
        for &ur in truthy {
            let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
            gjk_same("row17 truthy use_radius", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            assert_eq!(
                co.dist.to_bits(), one.dist.to_bits(),
                "use_radius={ur} must behave exactly like use_radius=1"
            );
        }
    }
}

#[test]
fn err_gjk_hit_zero_dist() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // Deeply overlapping shapes drive `s.count == 3` -> hit -> a = b, dist = 0.
    let big = AABB { min: V::new(-10.0, -10.0), max: V::new(10.0, 10.0) };
    let mut g = Rng::new(0x18);
    let mut hits = 0usize;
    for _ in 0..20_000 {
        let inner = Circle { p: V::new(g.range(-5.0, 5.0), g.range(-5.0, 5.0)), r: g.range(0.1, 3.0) };
        let icap = Capsule {
            a: V::new(g.range(-5.0, 5.0), g.range(-5.0, 5.0)),
            b: V::new(g.range(-5.0, 5.0), g.range(-5.0, 5.0)),
            r: g.range(0.0, 2.0),
        };
        let pa = &big as *const AABB as *const c_void;
        for &(pb, tb) in [
            (&inner as *const Circle as *const c_void, C2_TYPE_CIRCLE),
            (&icap as *const Capsule as *const c_void, C2_TYPE_CAPSULE),
        ]
        .iter()
        {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, C2_TYPE_AABB, None, pb, tb, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, C2_TYPE_AABB, None, pb, tb, None, ur, true, true, true, None) };
                gjk_same("row18 hit path", &format!("tb={tb} ur={ur}"), &co, &ro);
                if co.dist == 0.0 && co.a.bits() == co.b.bits() {
                    hits += 1;
                }
            }
        }
    }
    assert!(hits > 0, "expected the hit (dist==0, a==b) path to be reached");
    eprintln!("row18: {hits} hit-path results");
}

#[test]
fn err_gjk_radius_overlap_midpoint() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // dist <= rA + rB with use_radius -> midpoint branch, a == b, dist == 0.
    let mut g = Rng::new(0x19);
    let mut mids = 0usize;
    for _ in 0..20_000 {
        // two circles whose radii sum exceeds their centre distance
        let d = g.range(0.0, 10.0);
        let c1 = Circle { p: V::new(0.0, 0.0), r: g.range(d * 0.5, d * 2.0 + 1.0) };
        let c2 = Circle { p: V::new(d, 0.0), r: g.range(d * 0.5, d * 2.0 + 1.0) };
        let pa = &c1 as *const Circle as *const c_void;
        let pb = &c2 as *const Circle as *const c_void;
        let co = unsafe { gjk_call(&c, pa, C2_TYPE_CIRCLE, None, pb, C2_TYPE_CIRCLE, None, 1, true, true, true, None) };
        let ro = unsafe { gjk_call(&r, pa, C2_TYPE_CIRCLE, None, pb, C2_TYPE_CIRCLE, None, 1, true, true, true, None) };
        gjk_same("row19 radius-overlap midpoint", &format!("d={d:?} r1={:?} r2={:?}", c1.r, c2.r), &co, &ro);
        if co.dist == 0.0 && co.a.bits() == co.b.bits() {
            mids += 1;
        }
    }
    assert!(mids > 0, "expected the midpoint branch to be reached");
    eprintln!("row19: {mids} midpoint results");
}

#[test]
fn err_gjk_dist_below_epsilon() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // rA + rB == 0 and dist <= FLT_EPSILON -> still the midpoint branch.
    const EPS: f32 = 1.1920929e-7;
    for &sep in &[
        0.0f32, EPS * 0.25, EPS * 0.5, EPS, EPS * 1.0000001, EPS * 2.0, 1e-40, 1e-45, -0.0,
    ] {
        let c1 = Circle { p: V::new(0.0, 0.0), r: 0.0 };
        let c2 = Circle { p: V::new(sep, 0.0), r: 0.0 };
        let pa = &c1 as *const Circle as *const c_void;
        let pb = &c2 as *const Circle as *const c_void;
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, C2_TYPE_CIRCLE, None, pb, C2_TYPE_CIRCLE, None, ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, C2_TYPE_CIRCLE, None, pb, C2_TYPE_CIRCLE, None, ur, true, true, true, None) };
            gjk_same("row20 dist <= FLT_EPSILON", &format!("sep={sep:?} ur={ur}"), &co, &ro);
        }
    }
}

#[test]
fn err_gjk_shrink_collapse() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // After `dist -= rA + rB` and the two shrink steps, a and b can land on the
    // exact same point, which forces `dist = 0` (L486). Sweep separations right
    // at rA + rB to straddle it.
    let mut g = Rng::new(0x21);
    for _ in 0..40_000 {
        let ra = g.range(0.0, 5.0);
        let rb = g.range(0.0, 5.0);
        let sum = ra + rb;
        let sep = sum * g.range(0.999_99, 1.000_01);
        let c1 = Circle { p: V::new(0.0, 0.0), r: ra };
        let c2 = Circle { p: V::new(sep, 0.0), r: rb };
        let pa = &c1 as *const Circle as *const c_void;
        let pb = &c2 as *const Circle as *const c_void;
        let co = unsafe { gjk_call(&c, pa, C2_TYPE_CIRCLE, None, pb, C2_TYPE_CIRCLE, None, 1, true, true, true, None) };
        let ro = unsafe { gjk_call(&r, pa, C2_TYPE_CIRCLE, None, pb, C2_TYPE_CIRCLE, None, 1, true, true, true, None) };
        gjk_same("row21 shrink collapse", &format!("ra={ra:?} rb={rb:?} sep={sep:?}"), &co, &ro);
    }
}

// ===========================================================================
// Rows 22-25 — loop exits
// ===========================================================================

#[test]
fn err_gjk_no_progress_break() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // `d1 > d0` with d0 starting at FLT_MAX needs d1 to be +Inf, which huge
    // coordinates produce inside c2Dot.
    let mut g = Rng::new(0x22);
    for _ in 0..20_000 {
        let s = 1e38f32;
        let c1 = Circle { p: V::new(g.range(-s, s), g.range(-s, s)), r: g.range(0.0, 1e30) };
        let c2 = Circle { p: V::new(g.range(-s, s), g.range(-s, s)), r: g.range(0.0, 1e30) };
        let bb = AABB { min: V::new(-s, -s), max: V::new(s, s) };
        let pa = &c1 as *const Circle as *const c_void;
        for &(pb, tb) in [
            (&c2 as *const Circle as *const c_void, C2_TYPE_CIRCLE),
            (&bb as *const AABB as *const c_void, C2_TYPE_AABB),
        ]
        .iter()
        {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, C2_TYPE_CIRCLE, None, pb, tb, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, C2_TYPE_CIRCLE, None, pb, tb, None, ur, true, true, true, None) };
                gjk_same("row22 d1 > d0 early break", &format!("tb={tb} ur={ur}"), &co, &ro);
            }
        }
    }
}

#[test]
fn err_gjk_degenerate_direction() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // c2Dot(d,d) < FLT_EPSILON^2 : the search direction collapses, which happens
    // when the simplex point sits (almost) exactly on the origin — i.e. the two
    // support points coincide.
    let mut g = Rng::new(0x23);
    for _ in 0..20_000 {
        let p = V::new(g.range(-10.0, 10.0), g.range(-10.0, 10.0));
        // identical shapes -> Minkowski difference contains the origin exactly
        let c1 = Circle { p, r: g.range(0.0, 3.0) };
        let c2 = Circle { p, r: g.range(0.0, 3.0) };
        let cap1 = Capsule { a: p, b: V::new(p.x + 1.0, p.y), r: 0.5 };
        let cap2 = cap1;
        let pairs: [(*const c_void, i32, *const c_void, i32); 2] = [
            (
                &c1 as *const Circle as *const c_void, C2_TYPE_CIRCLE,
                &c2 as *const Circle as *const c_void, C2_TYPE_CIRCLE,
            ),
            (
                &cap1 as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
                &cap2 as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
            ),
        ];
        for &(pa, ta, pb, tb) in pairs.iter() {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
                gjk_same("row23 degenerate direction", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            }
        }
    }
}

#[test]
fn err_gjk_duplicate_support() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // Grid-snapped shapes maximise repeated support indices -> the `dup` break.
    let mut g = Rng::new(0x24);
    for _ in 0..40_000 {
        let bb = AABB { min: g.v_grid(), max: g.v_grid() };
        let bb2 = AABB { min: g.v_grid(), max: g.v_grid() };
        let pa = &bb as *const AABB as *const c_void;
        let pb = &bb2 as *const AABB as *const c_void;
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, C2_TYPE_AABB, None, pb, C2_TYPE_AABB, None, ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, C2_TYPE_AABB, None, pb, C2_TYPE_AABB, None, ur, true, true, true, None) };
            gjk_same("row24 duplicate support break", &format!("ur={ur} bb={bb:?} bb2={bb2:?}"), &co, &ro);
        }
    }
}

#[test]
fn err_gjk_iteration_cap() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    // The cap itself is unreachable (see phase_c_iteration_cap.rs: the highest
    // iteration count any input produces is 5, because a proxy has at most 4
    // verts). What IS checkable is the invariant the cap guarantees.
    let mut g = Rng::new(0x25);
    let mut maxit = -1;
    for _ in 0..50_000 {
        let bb = AABB { min: g.v_grid(), max: g.v_grid() };
        let cap = Capsule { a: g.v_grid(), b: g.v_grid(), r: g.grid().abs() };
        let pa = &bb as *const AABB as *const c_void;
        let pb = &cap as *const Capsule as *const c_void;
        for &ur in &[0i32, 1] {
            let co = unsafe { gjk_call(&c, pa, C2_TYPE_AABB, None, pb, C2_TYPE_CAPSULE, None, ur, true, true, true, None) };
            let ro = unsafe { gjk_call(&r, pa, C2_TYPE_AABB, None, pb, C2_TYPE_CAPSULE, None, ur, true, true, true, None) };
            gjk_same("row25 iteration cap", &format!("ur={ur}"), &co, &ro);
            assert!(
                (0..=20).contains(&co.iters),
                "C reported {} iterations, the cap allows at most 20",
                co.iters
            );
            maxit = maxit.max(co.iters);
        }
    }
    eprintln!("row25: max iterations observed = {maxit} (cap is 20)");
}

// ===========================================================================
// Rows 26-31 — non-finite and degenerate shape data
// ===========================================================================

#[test]
fn err_gjk_nan_inputs() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let nanp: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_0001),
        f32::from_bits(0x7f80_0001),
    ];
    for &nv in nanp {
        for slot in 0..5usize {
            let mut cvals = [1.0f32, 2.0, 3.0, 4.0, 0.5];
            cvals[slot] = nv;
            let ci = Circle { p: V::new(cvals[0], cvals[1]), r: cvals[4] };
            let bb = AABB { min: V::new(cvals[0], cvals[1]), max: V::new(cvals[2], cvals[3]) };
            let cap = Capsule {
                a: V::new(cvals[0], cvals[1]),
                b: V::new(cvals[2], cvals[3]),
                r: cvals[4],
            };
            for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
                for &ur in &[0i32, 1] {
                    let mut cc = GJKCache::default();
                    let _ = &mut cc;
                    let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, Some(cc)) };
                    let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, Some(cc)) };
                    gjk_same(
                        "row26 NaN shape data",
                        &format!("nan={:#010x} slot={slot} ta={ta} tb={tb} ur={ur}", nv.to_bits()),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

#[test]
fn err_gjk_inf_inputs() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    for &iv in &[f32::INFINITY, f32::NEG_INFINITY] {
        for slot in 0..5usize {
            let mut cvals = [1.0f32, 2.0, 3.0, 4.0, 0.5];
            cvals[slot] = iv;
            let ci = Circle { p: V::new(cvals[0], cvals[1]), r: cvals[4] };
            let bb = AABB { min: V::new(cvals[0], cvals[1]), max: V::new(cvals[2], cvals[3]) };
            let cap = Capsule {
                a: V::new(cvals[0], cvals[1]),
                b: V::new(cvals[2], cvals[3]),
                r: cvals[4],
            };
            for (pa, ta, pb, tb) in all_pairs(&ci, &bb, &cap) {
                for &ur in &[0i32, 1] {
                    let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, Some(GJKCache::default())) };
                    let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, Some(GJKCache::default())) };
                    gjk_same(
                        "row27 Inf shape data",
                        &format!("inf={iv:?} slot={slot} ta={ta} tb={tb} ur={ur}"),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

#[test]
fn err_gjk_negative_radius() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0x28);
    for _ in 0..20_000 {
        let ra = g.range(-20.0, 0.0);
        let rb = g.range(-20.0, 0.0);
        let ci = Circle { p: V::new(0.0, 0.0), r: ra };
        let ci2 = Circle { p: V::new(g.range(-20.0, 20.0), g.range(-20.0, 20.0)), r: rb };
        let cap = Capsule { a: V::new(1.0, 1.0), b: V::new(4.0, 2.0), r: ra };
        for (pa, ta, pb, tb) in [
            (
                &ci as *const Circle as *const c_void, C2_TYPE_CIRCLE,
                &ci2 as *const Circle as *const c_void, C2_TYPE_CIRCLE,
            ),
            (
                &cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
                &ci2 as *const Circle as *const c_void, C2_TYPE_CIRCLE,
            ),
        ] {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
                gjk_same("row28 negative radius", &format!("ra={ra:?} rb={rb:?} ta={ta} ur={ur}"), &co, &ro);
            }
        }
    }
}

#[test]
fn err_gjk_inverted_aabb() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0x29);
    for _ in 0..20_000 {
        // min > max on one or both axes
        let bb = AABB { min: V::new(5.0, 5.0), max: V::new(-5.0, -5.0) };
        let bb2 = AABB { min: V::new(g.range(0.0, 5.0), -5.0), max: V::new(g.range(-5.0, 0.0), 5.0) };
        let ci = Circle { p: V::new(g.range(-10.0, 10.0), g.range(-10.0, 10.0)), r: g.range(0.0, 3.0) };
        for (pa, ta, pb, tb) in [
            (
                &bb as *const AABB as *const c_void, C2_TYPE_AABB,
                &ci as *const Circle as *const c_void, C2_TYPE_CIRCLE,
            ),
            (
                &bb2 as *const AABB as *const c_void, C2_TYPE_AABB,
                &bb as *const AABB as *const c_void, C2_TYPE_AABB,
            ),
        ] {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
                let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, None) };
                gjk_same("row29 inverted AABB", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            }
        }
    }
}

#[test]
fn err_gjk_degenerate_aabb() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0x30);
    for _ in 0..20_000 {
        let p = V::new(g.grid(), g.grid());
        let bb = AABB { min: p, max: p }; // zero extent -> 4 identical verts
        let bb2 = AABB { min: V::new(p.x, p.y), max: V::new(p.x, p.y + g.range(0.0, 5.0)) };
        let ci = Circle { p: V::new(g.grid(), g.grid()), r: g.range(0.0, 3.0) };
        for (pa, ta, pb, tb) in [
            (
                &bb as *const AABB as *const c_void, C2_TYPE_AABB,
                &ci as *const Circle as *const c_void, C2_TYPE_CIRCLE,
            ),
            (
                &bb as *const AABB as *const c_void, C2_TYPE_AABB,
                &bb2 as *const AABB as *const c_void, C2_TYPE_AABB,
            ),
        ] {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, Some(GJKCache::default())) };
                let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, Some(GJKCache::default())) };
                gjk_same("row30 degenerate AABB", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            }
        }
    }
}

#[test]
fn err_gjk_degenerate_capsule() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0x31);
    for _ in 0..20_000 {
        let p = V::new(g.grid(), g.grid());
        let cap = Capsule { a: p, b: p, r: g.range(0.0, 3.0) }; // zero length
        let cap2 = Capsule { a: p, b: p, r: 0.0 };
        let ci = Circle { p: V::new(g.grid(), g.grid()), r: g.range(0.0, 3.0) };
        for (pa, ta, pb, tb) in [
            (
                &cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
                &ci as *const Circle as *const c_void, C2_TYPE_CIRCLE,
            ),
            (
                &cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
                &cap2 as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
            ),
        ] {
            for &ur in &[0i32, 1] {
                let co = unsafe { gjk_call(&c, pa, ta, None, pb, tb, None, ur, true, true, true, Some(GJKCache::default())) };
                let ro = unsafe { gjk_call(&r, pa, ta, None, pb, tb, None, ur, true, true, true, Some(GJKCache::default())) };
                gjk_same("row31 degenerate capsule", &format!("ta={ta} tb={tb} ur={ur}"), &co, &ro);
            }
        }
    }
}
