//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input / rejection condition the C
//! source checks for, calls BOTH libraries, and asserts they produce the same
//! sentinel (`0`, `(0,0)`, "out-param untouched", a specific IEEE value), not
//! merely "both did something".

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::os::raw::c_int;

// ===========================================================================
// Row 1 — c2MakeProxy with a `type` outside {0,1,2}: the switch has no
//         `default:`, so the function is a no-op and `*p` is left untouched.
// ===========================================================================

fn pattern_proxy() -> c2Proxy {
    let mut p = c2Proxy {
        radius: -12.5,
        count: 0x1234_5678,
        verts: [c2v::default(); 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = 100.0 + i as f32;
        v.y = -100.0 - i as f32;
    }
    p
}

#[test]
fn c01_makeproxy_invalid_type_is_a_noop() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC01);
    unsafe {
        for &bad in BAD_TYPES.iter() {
            // Any shape payload; the callee must not look at it.
            let mut shape = rng.circle(10.0);
            let untouched = pattern_proxy();
            let mut pc = untouched;
            let mut pr = untouched;
            (c.c2MakeProxy)(
                &mut shape as *mut c2Circle as *const c_void,
                bad,
                &mut pc,
            );
            (r.c2MakeProxy)(
                &mut shape as *mut c2Circle as *const c_void,
                bad,
                &mut pr,
            );
            eq_proxy(&format!("c2MakeProxy type={bad}"), &pc, &pr);
            eq_proxy(
                &format!("c2MakeProxy type={bad} must be a no-op"),
                &untouched,
                &pc,
            );
        }
    }
}

// ===========================================================================
// Rows 2 / 3 / 4 / 5 — the `default:` labels of the simplex accessors.
//         `count` values one step past every valid range.
// ===========================================================================

const BAD_COUNTS: [c_int; 8] = [-1, -2, 0, 4, 5, 100, c_int::MIN, c_int::MAX];

#[test]
fn c02_gjksimplexmetric_default_returns_zero() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC02);
    unsafe {
        for &count in BAD_COUNTS.iter().chain([1i32].iter()) {
            for _ in 0..64 {
                let mut sc = rng.simplex(count, 100.0);
                let mut sr = sc;
                let cv = (c.c2GJKSimplexMetric)(&mut sc);
                let rv = (r.c2GJKSimplexMetric)(&mut sr);
                eq_f32(&format!("c2GJKSimplexMetric count={count}"), cv, rv);
                eq_f32(
                    &format!("c2GJKSimplexMetric count={count} must be +0.0"),
                    0.0,
                    cv,
                );
                eq_simplex("simplex untouched", &sc, &sr);
            }
        }
    }
}

#[test]
fn c03_cD_default_returns_zero_vector() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC03);
    unsafe {
        for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
            for _ in 0..64 {
                let mut sc = rng.simplex(count, 100.0);
                let mut sr = sc;
                let cv = (c.c2D)(&mut sc);
                let rv = (r.c2D)(&mut sr);
                eq_v(&format!("c2D count={count}"), cv, rv);
                eq_v(
                    &format!("c2D count={count} must be (+0,+0)"),
                    c2v { x: 0.0, y: 0.0 },
                    cv,
                );
                eq_simplex("simplex untouched", &sc, &sr);
            }
        }
    }
}

#[test]
fn c04_witness_default_writes_zero_vectors() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC04);
    unsafe {
        for &count in BAD_COUNTS.iter() {
            for _ in 0..64 {
                let mut sc = rng.simplex(count, 100.0);
                let mut sr = sc;
                let mut ac = c2v { x: 9.0, y: 9.0 };
                let mut bc = c2v { x: 8.0, y: 8.0 };
                let mut ar = ac;
                let mut br = bc;
                (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v(&format!("c2Witness a count={count}"), ac, ar);
                eq_v(&format!("c2Witness b count={count}"), bc, br);
                eq_v("must be (+0,+0)", c2v { x: 0.0, y: 0.0 }, ac);
                eq_v("must be (+0,+0)", c2v { x: 0.0, y: 0.0 }, bc);
                eq_simplex("simplex untouched", &sc, &sr);
            }
        }
    }
}

#[test]
fn c05_cL_default_returns_zero_vector() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC05);
    unsafe {
        for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
            for _ in 0..64 {
                let mut sc = rng.simplex(count, 100.0);
                // `1.0f / div` is still evaluated in the default branch; make
                // sure a zero / NaN `div` does not change the sentinel.
                sc.div = [0.0f32, -0.0, f32::NAN, f32::INFINITY, 1.0][(count.unsigned_abs() % 5) as usize];
                let mut sr = sc;
                let cv = (c.c2L)(&mut sc);
                let rv = (r.c2L)(&mut sr);
                eq_v(&format!("c2L count={count}"), cv, rv);
                eq_v(
                    &format!("c2L count={count} must be (+0,+0)"),
                    c2v { x: 0.0, y: 0.0 },
                    cv,
                );
                eq_simplex("simplex untouched", &sc, &sr);
            }
        }
    }
}

// ===========================================================================
// c2GJK plumbing shared by rows 6 … 21
// ===========================================================================

#[repr(C, align(8))]
#[derive(Copy, Clone, PartialEq, Debug)]
struct Buf([u8; 32]);

fn put<T: Copy>(v: &T) -> Buf {
    let mut b = Buf([0xA5; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

struct GjkOut {
    dist: f32,
    oa: c2v,
    ob: c2v,
    it: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
fn gjk(
    api: &Api,
    abuf: &Buf,
    ta: c_int,
    ax: Option<c2x>,
    bbuf: &Buf,
    tb: c_int,
    bx: Option<c2x>,
    want_oa: bool,
    want_ob: bool,
    use_radius: c_int,
    want_it: bool,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    let mut oa = c2v { x: -1.5, y: 2.5 };
    let mut ob = c2v { x: 3.5, y: -4.5 };
    let mut it: c_int = -999;
    let mut cache = cache_in.unwrap_or_default();
    let dist = unsafe {
        (api.c2GJK)(
            abuf.0.as_ptr() as *const c_void,
            ta,
            ax.as_ref().map_or(std::ptr::null(), |v| v as *const c2x),
            bbuf.0.as_ptr() as *const c_void,
            tb,
            bx.as_ref().map_or(std::ptr::null(), |v| v as *const c2x),
            if want_oa { &mut oa } else { std::ptr::null_mut() },
            if want_ob { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if want_it { &mut it } else { std::ptr::null_mut() },
            if cache_in.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOut {
        dist,
        oa,
        ob,
        it,
        cache,
    }
}

#[track_caller]
fn eq_gjk(ctx: &str, a: &GjkOut, b: &GjkOut) {
    eq_f32(&format!("{ctx} dist"), a.dist, b.dist);
    eq_v(&format!("{ctx} outA"), a.oa, b.oa);
    eq_v(&format!("{ctx} outB"), a.ob, b.ob);
    eq_int(&format!("{ctx} iterations"), a.it, b.it);
    eq_cache(&format!("{ctx} cache"), &a.cache, &b.cache);
}

// ===========================================================================
// Rows 6 / 7 — NULL transforms are replaced by `c2xIdentity()`
// ===========================================================================

#[test]
fn c06_c07_null_transforms_equal_explicit_identity() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC06);
    let id = unsafe { (c.c2xIdentity)() };
    for i in 0..600 {
        let a = put(&rng.circle(20.0));
        let b = put(&rng.aabb(20.0));
        for (ax, bx, label) in [
            (None, None, "both NULL"),
            (Some(id), None, "ax explicit id, bx NULL"),
            (None, Some(id), "ax NULL, bx explicit id"),
            (Some(id), Some(id), "both explicit id"),
        ] {
            let oc = gjk(
                c,
                &a,
                C2_TYPE_CIRCLE,
                ax,
                &b,
                C2_TYPE_AABB,
                bx,
                true,
                true,
                1,
                true,
                Some(c2GJKCache::default()),
            );
            let or = gjk(
                r,
                &a,
                C2_TYPE_CIRCLE,
                ax,
                &b,
                C2_TYPE_AABB,
                bx,
                true,
                true,
                1,
                true,
                Some(c2GJKCache::default()),
            );
            eq_gjk(&format!("c2GJK {label} #{i}"), &oc, &or);
        }
        // NULL must be *equivalent* to the explicit identity, in both libs.
        for api in [c, r] {
            let n = gjk(
                api,
                &a,
                C2_TYPE_CIRCLE,
                None,
                &b,
                C2_TYPE_AABB,
                None,
                true,
                true,
                1,
                true,
                Some(c2GJKCache::default()),
            );
            let e = gjk(
                api,
                &a,
                C2_TYPE_CIRCLE,
                Some(id),
                &b,
                C2_TYPE_AABB,
                Some(id),
                true,
                true,
                1,
                true,
                Some(c2GJKCache::default()),
            );
            eq_gjk(&format!("{} NULL == identity #{i}", api.tag), &n, &e);
        }
    }
}

// ===========================================================================
// Rows 8 / 12 / 13 / 14 — NULL `cache`, `outA`, `outB`, `iterations`
// ===========================================================================

#[test]
fn c08_c12_c13_c14_null_out_params_are_skipped() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC08);
    for i in 0..400 {
        let a = put(&rng.capsule(20.0));
        let b = put(&rng.capsule(20.0));
        for mask in 0u32..16 {
            let want_oa = mask & 1 != 0;
            let want_ob = mask & 2 != 0;
            let want_it = mask & 4 != 0;
            let cache = (mask & 8 != 0).then(c2GJKCache::default);
            let oc = gjk(
                c,
                &a,
                C2_TYPE_CAPSULE,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                want_oa,
                want_ob,
                1,
                want_it,
                cache,
            );
            let or = gjk(
                r,
                &a,
                C2_TYPE_CAPSULE,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                want_oa,
                want_ob,
                1,
                want_it,
                cache,
            );
            eq_f32(&format!("c2GJK mask={mask} #{i} dist"), oc.dist, or.dist);
            // The skipped writes must leave the sentinels the caller planted.
            if !want_oa {
                eq_v("outA NULL sentinel", c2v { x: -1.5, y: 2.5 }, oc.oa);
                eq_v("outA NULL sentinel", c2v { x: -1.5, y: 2.5 }, or.oa);
            } else {
                eq_v("outA", oc.oa, or.oa);
            }
            if !want_ob {
                eq_v("outB NULL sentinel", c2v { x: 3.5, y: -4.5 }, oc.ob);
                eq_v("outB NULL sentinel", c2v { x: 3.5, y: -4.5 }, or.ob);
            } else {
                eq_v("outB", oc.ob, or.ob);
            }
            if !want_it {
                eq_int("iterations NULL sentinel", -999, oc.it);
                eq_int("iterations NULL sentinel", -999, or.it);
            } else {
                eq_int("iterations", oc.it, or.it);
            }
            if cache.is_none() {
                eq_cache("cache NULL sentinel", &c2GJKCache::default(), &oc.cache);
                eq_cache("cache NULL sentinel", &c2GJKCache::default(), &or.cache);
            } else {
                eq_cache("cache", &oc.cache, &or.cache);
            }
        }
    }
}

// ===========================================================================
// Row 9 — `cache->count == 0` is rejected as "not good" (cold start), even
//         when the other cache fields hold garbage.
// ===========================================================================

#[test]
fn c09_cache_count_zero_is_rejected() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC09);
    for i in 0..600 {
        let a = put(&rng.aabb(20.0));
        let b = put(&rng.capsule(20.0));
        let garbage = c2GJKCache {
            metric: rng.wild_f32(),
            count: 0,
            iA: [3, -7, 1000],
            iB: [-1, 2, i32::MIN],
            div: rng.wild_f32(),
        };
        // Same index array (the C only overwrites `iA[0..count)`, so the tail is
        // caller-visible state that must be preserved), but zeroed metric/div:
        // with `count == 0` neither is read, so the outcome must be identical.
        let clean = c2GJKCache {
            metric: 0.0,
            count: 0,
            iA: garbage.iA,
            iB: garbage.iB,
            div: 0.0,
        };
        let oc = gjk(
            c,
            &a,
            C2_TYPE_AABB,
            None,
            &b,
            C2_TYPE_CAPSULE,
            None,
            true,
            true,
            1,
            true,
            Some(garbage),
        );
        let or = gjk(
            r,
            &a,
            C2_TYPE_AABB,
            None,
            &b,
            C2_TYPE_CAPSULE,
            None,
            true,
            true,
            1,
            true,
            Some(garbage),
        );
        eq_gjk(&format!("garbage-but-count-0 cache #{i}"), &oc, &or);
        // …and it must be indistinguishable from an all-zero cache.
        for api in [c, r] {
            let g = gjk(
                api,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                true,
                true,
                1,
                true,
                Some(garbage),
            );
            let z = gjk(
                api,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                true,
                true,
                1,
                true,
                Some(clean),
            );
            eq_gjk(&format!("{} count==0 == clean #{i}", api.tag), &g, &z);
        }
    }
}

// ===========================================================================
// Row 10 — the cache-invalidation guard
//          `!(min_metric < max_metric*2 && metric < -1.0e8f)`
// ===========================================================================

#[test]
fn c10_cache_metric_guard_rejects_the_cache() {
    let (c, r) = libs();
    // A large AABB against a point-circle at the origin makes the recomputed
    // 3-simplex metric a big signed area; the index order below makes it
    // negative (-4e10), which trips `metric < -1.0e8f`.
    let big = c2AABB {
        min: c2v { x: -1.0e5, y: -1.0e5 },
        max: c2v { x: 1.0e5, y: 1.0e5 },
    };
    let dot = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    let a = put(&big);
    let b = put(&dot);

    // metric_old = 0  -> min = metric (-4e10), max = 0, max*2 = 0
    //              -> min < max*2 AND metric < -1e8  -> cache REJECTED
    let rejected = c2GJKCache {
        metric: 0.0,
        count: 3,
        iA: [0, 2, 1],
        iB: [0, 0, 0],
        div: 1.0,
    };
    // metric_old = -4e10 -> min = max = -4e10, max*2 = -8e10
    //              -> min < max*2 is FALSE -> cache ACCEPTED
    let accepted = c2GJKCache {
        metric: -4.0e10,
        count: 3,
        iA: [0, 2, 1],
        iB: [0, 0, 0],
        div: 1.0,
    };

    let mut outs = Vec::new();
    for (label, cache) in [("rejected", rejected), ("accepted", accepted)] {
        let oc = gjk(
            c,
            &a,
            C2_TYPE_AABB,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            0,
            true,
            Some(cache),
        );
        let or = gjk(
            r,
            &a,
            C2_TYPE_AABB,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            0,
            true,
            Some(cache),
        );
        eq_gjk(&format!("metric guard {label}"), &oc, &or);
        outs.push((label, oc.dist, oc.it, oc.cache.count, oc.cache.metric));
    }
    println!("metric-guard outcomes: {outs:?}");
    // Prove the guard is actually observable: the two calls differ only in
    // `cache->metric`, yet take different paths.
    assert!(
        outs[0].1.to_bits() != outs[1].1.to_bits()
            || outs[0].2 != outs[1].2
            || outs[0].3 != outs[1].3
            || outs[0].4.to_bits() != outs[1].4.to_bits(),
        "the metric guard produced identical results for both branches — \
         the row is not actually exercised: {outs:?}"
    );

    // Also sweep a range of metric_old values across the guard boundary.
    for k in 0..200 {
        let m = -1.0e11 + (k as f32) * 1.0e9;
        for count in 1..=3i32 {
            let cache = c2GJKCache {
                metric: m,
                count,
                iA: [0, 2, 1],
                iB: [0, 0, 0],
                div: 1.0,
            };
            let oc = gjk(
                c,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CIRCLE,
                None,
                true,
                true,
                0,
                true,
                Some(cache),
            );
            let or = gjk(
                r,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CIRCLE,
                None,
                true,
                true,
                0,
                true,
                Some(cache),
            );
            eq_gjk(&format!("metric guard sweep m={m} count={count}"), &oc, &or);
        }
    }
}

// ===========================================================================
// Row 11 — `cache->count < 0`
// ===========================================================================

#[test]
fn c11_negative_cache_count() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC11);
    for &count in [-1i32, -2, -3, -100, c_int::MIN].iter() {
        for i in 0..200 {
            let a = put(&rng.aabb(20.0));
            let b = put(&rng.capsule(20.0));
            let cache = c2GJKCache {
                metric: rng.wild_f32(),
                count,
                iA: [0, 1, 2],
                iB: [0, 1, 0],
                div: match rng.below(4) {
                    0 => 0.0,
                    1 => 1.0,
                    2 => rng.uniform(-10.0, 10.0),
                    _ => rng.wild_f32(),
                },
            };
            let oc = gjk(
                c,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                true,
                true,
                (i & 1) as c_int,
                true,
                Some(cache),
            );
            let or = gjk(
                r,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                true,
                true,
                (i & 1) as c_int,
                true,
                Some(cache),
            );
            eq_gjk(&format!("negative cache count={count} #{i}"), &oc, &or);
            // documented sentinel behaviour
            eq_f32("dist must be +0", 0.0, oc.dist);
            eq_v("outA must be (0,0)", c2v { x: 0.0, y: 0.0 }, oc.oa);
            eq_v("outB must be (0,0)", c2v { x: 0.0, y: 0.0 }, oc.ob);
            eq_int("iterations must be 0", 0, oc.it);
            eq_int("cache.count preserved", count, oc.cache.count);
            eq_f32("cache.metric must be +0", 0.0, oc.cache.metric);
        }
    }
}

// ===========================================================================
// Row 15 — the hard `while (iter < 20)` cap
// ===========================================================================

#[test]
fn c15_iteration_cap() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC15);
    let mut max_seen = 0;
    for i in 0..4000 {
        // AABB x AABB gives the largest vertex sets, so the longest walks.
        let a = put(&rng.aabb(1.0e6));
        let b = put(&rng.aabb(1.0e6));
        let oc = gjk(
            c,
            &a,
            C2_TYPE_AABB,
            Some(rng.xform(1.0e6)),
            &b,
            C2_TYPE_AABB,
            Some(rng.xform(1.0e6)),
            true,
            true,
            (i & 1) as c_int,
            true,
            Some(c2GJKCache::default()),
        );
        let or = gjk(
            r,
            &a,
            C2_TYPE_AABB,
            None,
            &b,
            C2_TYPE_AABB,
            None,
            true,
            true,
            (i & 1) as c_int,
            true,
            Some(c2GJKCache::default()),
        );
        let _ = &or; // (transform args differ above; recompute properly below)
        let oc2 = gjk(
            c,
            &a,
            C2_TYPE_AABB,
            None,
            &b,
            C2_TYPE_AABB,
            None,
            true,
            true,
            (i & 1) as c_int,
            true,
            Some(c2GJKCache::default()),
        );
        eq_gjk(&format!("iteration cap #{i}"), &oc2, &or);
        assert!(
            (0..=20).contains(&oc.it) && (0..=20).contains(&or.it),
            "iterations out of the documented 0..=20 range: C={} RUST={}",
            oc.it,
            or.it
        );
        max_seen = max_seen.max(oc2.it);
    }
    println!("c15: max iterations observed = {max_seen} (hard cap is 20)");
}

// ===========================================================================
// Row 16 — the `if (d1 > d0) break;` non-monotonic-progress bail-out
// ===========================================================================

#[test]
fn c16_non_monotonic_progress_break() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC16);
    // Huge / near-degenerate geometry is what makes `d1 > d0` fire.
    for i in 0..4000 {
        let m = [1.0e15f32, 1.0e20, 1.0e30, FLT_MAX][(i % 4) as usize];
        let a = put(&c2AABB {
            min: c2v { x: -m, y: -m },
            max: c2v { x: m, y: m },
        });
        let b = put(&c2Capsule {
            a: c2v {
                x: rng.uniform(-1.0, 1.0) * m,
                y: rng.uniform(-1.0, 1.0) * m,
            },
            b: c2v {
                x: rng.uniform(-1.0, 1.0) * m,
                y: rng.uniform(-1.0, 1.0) * m,
            },
            r: rng.uniform(0.0, 1.0) * m,
        });
        for ur in [0, 1] {
            let oc = gjk(
                c,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                true,
                true,
                ur,
                true,
                Some(c2GJKCache::default()),
            );
            let or = gjk(
                r,
                &a,
                C2_TYPE_AABB,
                None,
                &b,
                C2_TYPE_CAPSULE,
                None,
                true,
                true,
                ur,
                true,
                Some(c2GJKCache::default()),
            );
            eq_gjk(&format!("d1>d0 break #{i} ur={ur}"), &oc, &or);
        }
    }
}

// ===========================================================================
// Row 17 — the degenerate-direction break: identical shapes give `d == 0`,
//          so the loop exits with `*iterations == 0`.
// ===========================================================================

#[test]
fn c17_degenerate_direction_break() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC17);
    for i in 0..600 {
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let s = match ty {
                C2_TYPE_CIRCLE => put(&rng.circle(20.0)),
                C2_TYPE_AABB => put(&rng.aabb(20.0)),
                _ => put(&rng.capsule(20.0)),
            };
            let oc = gjk(
                c, &s, ty, None, &s, ty, None, true, true, 0, true,
                Some(c2GJKCache::default()),
            );
            let or = gjk(
                r, &s, ty, None, &s, ty, None, true, true, 0, true,
                Some(c2GJKCache::default()),
            );
            eq_gjk(&format!("identical shapes ty={ty} #{i}"), &oc, &or);
            eq_int("identical shapes must break at iteration 0", 0, oc.it);
        }
    }
}

// ===========================================================================
// Row 18 — the duplicate-support-index break.  A circle proxy has a single
//          vertex, so the very first support point always duplicates.
// ===========================================================================

#[test]
fn c18_duplicate_support_break() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC18);
    for i in 0..2000 {
        let a = put(&rng.circle(20.0));
        let b = put(&rng.circle(20.0));
        let oc = gjk(
            c,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            (i & 1) as c_int,
            true,
            Some(c2GJKCache::default()),
        );
        let or = gjk(
            r,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            (i & 1) as c_int,
            true,
            Some(c2GJKCache::default()),
        );
        eq_gjk(&format!("circle x circle dup break #{i}"), &oc, &or);
        eq_int("circle x circle must exit at iteration 0", 0, oc.it);
        eq_int("final simplex is a point", 1, oc.cache.count);
    }
}

// ===========================================================================
// Row 19 — `use_radius` with overlapping shapes: `dist` is forced to 0 and
//          both witness points collapse onto the midpoint.
// ===========================================================================

#[test]
fn c19_radius_midpoint_collapse() {
    let (c, r) = libs();
    // Concentric circles with generous radii: core distance 0 <= rA+rB.
    for k in 0..200 {
        let ra = 1.0 + k as f32;
        let a = put(&c2Circle {
            p: c2v { x: 5.0, y: -3.0 },
            r: ra,
        });
        let b = put(&c2Circle {
            p: c2v { x: 6.0, y: -3.0 },
            r: ra,
        });
        let oc = gjk(
            c,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            1,
            true,
            Some(c2GJKCache::default()),
        );
        let or = gjk(
            r,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            1,
            true,
            Some(c2GJKCache::default()),
        );
        eq_gjk(&format!("radius midpoint collapse k={k}"), &oc, &or);
        eq_f32("dist forced to 0", 0.0, oc.dist);
        eq_v("outA == outB (midpoint)", oc.oa, oc.ob);
        eq_v("outA == outB (midpoint)", or.oa, or.ob);
    }
    // `use_radius` values other than 0/1 must behave like 1 (row G3).
    for ur in [2i32, -1, 7, c_int::MIN, c_int::MAX] {
        let a = put(&c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 5.0,
        });
        let b = put(&c2Circle {
            p: c2v { x: 1.0, y: 0.0 },
            r: 5.0,
        });
        let oc = gjk(
            c,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            ur,
            true,
            Some(c2GJKCache::default()),
        );
        let or = gjk(
            r,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            ur,
            true,
            Some(c2GJKCache::default()),
        );
        eq_gjk(&format!("use_radius={ur}"), &oc, &or);
        let one = gjk(
            c,
            &a,
            C2_TYPE_CIRCLE,
            None,
            &b,
            C2_TYPE_CIRCLE,
            None,
            true,
            true,
            1,
            true,
            Some(c2GJKCache::default()),
        );
        eq_f32("non-zero use_radius behaves like 1", one.dist, oc.dist);
        eq_f32("non-zero use_radius behaves like 1", one.dist, or.dist);
    }
}

// ===========================================================================
// Row 20 — the radius shrink collapses the two witness points, so `dist` is
//          forced to 0 even though the raw distance exceeded `rA + rB`.
//          Built with `rA = 1e38`, `rB = -1e38`  =>  `rA + rB == 0`, and the
//          1e38-long shift swallows the 1-unit separation.
// ===========================================================================

#[test]
fn c20_radius_shrink_collapse() {
    let (c, r) = libs();
    let a = put(&c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 1.0e38,
    });
    let b = put(&c2Circle {
        p: c2v { x: 1.0, y: 0.0 },
        r: -1.0e38,
    });
    let oc = gjk(
        c,
        &a,
        C2_TYPE_CIRCLE,
        None,
        &b,
        C2_TYPE_CIRCLE,
        None,
        true,
        true,
        1,
        true,
        Some(c2GJKCache::default()),
    );
    let or = gjk(
        r,
        &a,
        C2_TYPE_CIRCLE,
        None,
        &b,
        C2_TYPE_CIRCLE,
        None,
        true,
        true,
        1,
        true,
        Some(c2GJKCache::default()),
    );
    eq_gjk("radius shrink collapse", &oc, &or);
    println!(
        "c20: dist={:?} outA={:?} outB={:?}",
        oc.dist, oc.oa, oc.ob
    );
    // The collapse branch is only reached when the shrink really did make the
    // two witness points identical; assert that, then assert the sentinel.
    eq_v("witness points collapsed", oc.oa, oc.ob);
    eq_f32("dist forced to 0 by the collapse", 0.0, oc.dist);

    // Sweep the construction so the row is covered across many values.
    for k in 0..64 {
        let big = 1.0e38f32 / (1.0 + k as f32);
        let a = put(&c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: big,
        });
        let b = put(&c2Circle {
            p: c2v {
                x: 1.0 + k as f32,
                y: 0.0,
            },
            r: -big,
        });
        let oc = gjk(
            c, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true,
            Some(c2GJKCache::default()),
        );
        let or = gjk(
            r, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true,
            Some(c2GJKCache::default()),
        );
        eq_gjk(&format!("radius shrink collapse sweep k={k}"), &oc, &or);
    }
}

// ===========================================================================
// Row 21 — `hit` (simplex reached 3 vertices) ignores the radii entirely.
// ===========================================================================

#[test]
fn c21_hit_ignores_radii() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC21);
    let mut hits = 0usize;
    for i in 0..2000 {
        // Crossing, *asymmetric* capsules and overlapping AABBs: the origin ends
        // up strictly inside the Minkowski difference, so `count == 3` and
        // `hit == 1`.  (A perfectly symmetric cross puts the origin exactly on an
        // edge of the simplex, which takes the `c23` edge branch instead — hence
        // the deliberate jitter.)
        let jitter = |rng: &mut Rng| rng.uniform(-6.0, 6.0);
        let a = put(&c2Capsule {
            a: c2v {
                x: -10.0 + jitter(&mut rng),
                y: jitter(&mut rng),
            },
            b: c2v {
                x: 10.0 + jitter(&mut rng),
                y: jitter(&mut rng),
            },
            r: 5.0 + rng.uniform(0.0, 20.0),
        });
        let b = put(&c2Capsule {
            a: c2v {
                x: jitter(&mut rng),
                y: -10.0 + jitter(&mut rng),
            },
            b: c2v {
                x: jitter(&mut rng),
                y: 10.0 + jitter(&mut rng),
            },
            r: 5.0 + rng.uniform(0.0, 20.0),
        });
        let bb1 = put(&c2AABB {
            min: c2v {
                x: -10.0 + jitter(&mut rng),
                y: -10.0 + jitter(&mut rng),
            },
            max: c2v {
                x: 10.0 + jitter(&mut rng),
                y: 10.0 + jitter(&mut rng),
            },
        });
        let bb2 = put(&c2AABB {
            min: c2v {
                x: -8.0 + jitter(&mut rng),
                y: -8.0 + jitter(&mut rng),
            },
            max: c2v {
                x: 12.0 + jitter(&mut rng),
                y: 12.0 + jitter(&mut rng),
            },
        });
        for (pa, ta, pb, tb) in [
            (&a, C2_TYPE_CAPSULE, &b, C2_TYPE_CAPSULE),
            (&bb1, C2_TYPE_AABB, &bb2, C2_TYPE_AABB),
            (&bb1, C2_TYPE_AABB, &b, C2_TYPE_CAPSULE),
            (&a, C2_TYPE_CAPSULE, &bb2, C2_TYPE_AABB),
        ] {
            for ur in [0, 1] {
                let oc = gjk(
                    c, pa, ta, None, pb, tb, None, true, true, ur, true,
                    Some(c2GJKCache::default()),
                );
                let or = gjk(
                    r, pa, ta, None, pb, tb, None, true, true, ur, true,
                    Some(c2GJKCache::default()),
                );
                eq_gjk(&format!("hit path #{i} {ta}x{tb} ur={ur}"), &oc, &or);
                if oc.cache.count == 3 {
                    hits += 1;
                    eq_f32("hit forces dist to 0", 0.0, oc.dist);
                    eq_v("hit sets a = b", oc.oa, oc.ob);
                }
            }
        }
    }
    assert!(hits > 0, "row 21 was never exercised (no `hit` reached)");
    println!("c21: {hits} hit-path cases");
}

// ===========================================================================
// Rows 22 / 23 / 24 / 25 — every `default: return 0;` of c2Collided,
//          including out-of-range enum values (row G1).
// ===========================================================================

#[test]
fn c22_c23_c24_c25_collided_invalid_types_return_zero() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC22);
    unsafe {
        for i in 0..200 {
            let bufs = [
                put(&rng.circle(20.0)),
                put(&rng.aabb(20.0)),
                put(&rng.capsule(20.0)),
            ];
            let pa = bufs[(i % 3) as usize].0.as_ptr() as *const c_void;
            let pb = bufs[((i + 1) % 3) as usize].0.as_ptr() as *const c_void;

            // Row 22/23/24: valid typeA, invalid typeB.
            for ta in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                for &tb in BAD_TYPES.iter() {
                    let cv = (c.c2Collided)(pa, ta, pb, tb);
                    let rv = (r.c2Collided)(pa, ta, pb, tb);
                    eq_int(&format!("c2Collided({ta},{tb})"), cv, rv);
                    eq_int(&format!("c2Collided({ta},{tb}) must be 0"), 0, cv);
                }
            }
            // Row 25: invalid typeA (any typeB, including invalid).  `B` is
            // never dereferenced, so a NULL `B` is also legal here.
            for &ta in BAD_TYPES.iter() {
                for tb in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE]
                    .into_iter()
                    .chain(BAD_TYPES)
                {
                    let cv = (c.c2Collided)(pa, ta, pb, tb);
                    let rv = (r.c2Collided)(pa, ta, pb, tb);
                    eq_int(&format!("c2Collided({ta},{tb})"), cv, rv);
                    eq_int(&format!("c2Collided({ta},{tb}) must be 0"), 0, cv);
                    // and with a NULL B pointer
                    let cv = (c.c2Collided)(pa, ta, std::ptr::null(), tb);
                    let rv = (r.c2Collided)(pa, ta, std::ptr::null(), tb);
                    eq_int(&format!("c2Collided({ta},{tb}) NULL B"), cv, rv);
                    eq_int("must be 0", 0, cv);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 26 / 27 — c2Support with `count <= 0`, and the first-maximum tie rule.
// ===========================================================================

#[test]
fn c26_support_nonpositive_count_returns_zero() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC26);
    unsafe {
        for &count in [0i32, -1, -2, -100, c_int::MIN].iter() {
            for _ in 0..200 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = rng.wild_v();
                }
                let d = rng.wild_v();
                let cv = (c.c2Support)(verts.as_ptr(), count, d);
                let rv = (r.c2Support)(verts.as_ptr(), count, d);
                eq_int(&format!("c2Support count={count}"), cv, rv);
                eq_int(&format!("c2Support count={count} must be 0"), 0, cv);
            }
        }
    }
}

#[test]
fn c27_support_tie_picks_lowest_index() {
    let (c, r) = libs();
    unsafe {
        // All projections equal -> index 0 must win (strict `>` in the C).
        for count in 1..=8i32 {
            let verts = [c2v { x: 1.0, y: 1.0 }; 8];
            for d in [
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -1.0, y: -1.0 },
                c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
            ] {
                let cv = (c.c2Support)(verts.as_ptr(), count, d);
                let rv = (r.c2Support)(verts.as_ptr(), count, d);
                eq_int(&format!("c2Support tie count={count} d={d:?}"), cv, rv);
                eq_int("first maximum must win", 0, cv);
            }
        }
        // A tie between two *later* indices: the earlier of the two wins.
        let verts = [
            c2v { x: -5.0, y: 0.0 },
            c2v { x: 3.0, y: 0.0 },
            c2v { x: 3.0, y: 0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 3.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
        ];
        let d = c2v { x: 1.0, y: 0.0 };
        for count in 1..=8i32 {
            let cv = (c.c2Support)(verts.as_ptr(), count, d);
            let rv = (r.c2Support)(verts.as_ptr(), count, d);
            eq_int(&format!("c2Support later tie count={count}"), cv, rv);
        }
        eq_int(
            "the first of the tied maxima",
            1,
            (c.c2Support)(verts.as_ptr(), 8, d),
        );
        // Every projection is NaN -> `dot > dmax` is always false -> 0.
        let nans = [c2v {
            x: f32::NAN,
            y: f32::NAN,
        }; 8];
        for count in 1..=8i32 {
            let cv = (c.c2Support)(nans.as_ptr(), count, d);
            let rv = (r.c2Support)(nans.as_ptr(), count, d);
            eq_int(&format!("c2Support all-NaN count={count}"), cv, rv);
            eq_int("must be 0", 0, cv);
        }
    }
}

// ===========================================================================
// Rows 28 / 29 / 30 — unchecked divisions and the sqrt domain.
// ===========================================================================

#[test]
fn c28_div_by_zero() {
    let (c, r) = libs();
    unsafe {
        for &b in [0.0f32, -0.0].iter() {
            for &ax in GRID.iter() {
                for &ay in GRID.iter() {
                    let a = c2v { x: ax, y: ay };
                    let cv = (c.c2Div)(a, b);
                    let rv = (r.c2Div)(a, b);
                    eq_v(&format!("c2Div({a:?}, {b:?})"), cv, rv);
                }
            }
        }
        // The documented sentinels.
        let one = c2v { x: 1.0, y: -1.0 };
        eq_v(
            "1/(+0) -> +inf/-inf",
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            (c.c2Div)(one, 0.0),
        );
        eq_v(
            "1/(+0) -> +inf/-inf (rust)",
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            (r.c2Div)(one, 0.0),
        );
        let zero = c2v { x: 0.0, y: 0.0 };
        let cz = (c.c2Div)(zero, 0.0);
        let rz = (r.c2Div)(zero, 0.0);
        eq_v("0/0 -> NaN", cz, rz);
        assert!(cz.x.is_nan() && cz.y.is_nan(), "expected NaN, got {cz:?}");
    }
}

#[test]
fn c29_norm_of_zero_vector() {
    let (c, r) = libs();
    unsafe {
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: -0.0, y: -0.0 },
        ] {
            let cv = (c.c2Norm)(a);
            let rv = (r.c2Norm)(a);
            eq_v(&format!("c2Norm({a:?})"), cv, rv);
            assert!(cv.x.is_nan() && cv.y.is_nan(), "expected NaN, got {cv:?}");
        }
        // denormal input: c2Dot underflows to 0 -> same NaN sentinel
        for a in [
            c2v {
                x: 1.0e-45,
                y: 1.0e-45,
            },
            c2v {
                x: 1.0e-30,
                y: 0.0,
            },
        ] {
            eq_v(&format!("c2Norm({a:?})"), (c.c2Norm)(a), (r.c2Norm)(a));
        }
    }
}

#[test]
fn c30_len_overflow_and_nan() {
    let (c, r) = libs();
    unsafe {
        let cases = [
            (c2v { x: 1.0e30, y: 0.0 }, None),
            (
                c2v {
                    x: 1.0e30,
                    y: 1.0e30,
                },
                Some(f32::INFINITY),
            ),
            (c2v { x: FLT_MAX, y: FLT_MAX }, Some(f32::INFINITY)),
            (
                c2v {
                    x: f32::INFINITY,
                    y: 0.0,
                },
                Some(f32::INFINITY),
            ),
            (
                c2v {
                    x: f32::NEG_INFINITY,
                    y: 0.0,
                },
                Some(f32::INFINITY),
            ),
        ];
        for (a, expect) in cases {
            let cv = (c.c2Len)(a);
            let rv = (r.c2Len)(a);
            eq_f32(&format!("c2Len({a:?})"), cv, rv);
            if let Some(e) = expect {
                eq_f32(&format!("c2Len({a:?}) sentinel"), e, cv);
            }
        }
        // NaN in either component propagates.
        for a in [
            c2v {
                x: f32::NAN,
                y: 0.0,
            },
            c2v {
                x: 0.0,
                y: f32::NAN,
            },
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
        ] {
            let cv = (c.c2Len)(a);
            let rv = (r.c2Len)(a);
            eq_f32(&format!("c2Len({a:?})"), cv, rv);
        }
    }
}

// ===========================================================================
// Rows 31 / 32 — `1.0f / s->div` with `div == 0` in c2Witness / c2L.
// ===========================================================================

#[test]
fn c31_c32_zero_div_in_witness_and_L() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC31);
    unsafe {
        for &div in [0.0f32, -0.0].iter() {
            for count in [1i32, 2, 3] {
                for _ in 0..200 {
                    let mut sc = rng.simplex(count, 100.0);
                    sc.div = div;
                    let mut sr = sc;
                    let cv = (c.c2L)(&mut sc);
                    let rv = (r.c2L)(&mut sr);
                    eq_v(&format!("c2L div={div:?} count={count}"), cv, rv);
                    eq_simplex("simplex untouched", &sc, &sr);

                    let mut sc2 = sc;
                    let mut sr2 = sc;
                    let mut ac = c2v { x: 1.0, y: 2.0 };
                    let mut bc = c2v { x: 3.0, y: 4.0 };
                    let mut ar = ac;
                    let mut br = bc;
                    (c.c2Witness)(&mut sc2, &mut ac, &mut bc);
                    (r.c2Witness)(&mut sr2, &mut ar, &mut br);
                    eq_v(&format!("c2Witness a div={div:?} count={count}"), ac, ar);
                    eq_v(&format!("c2Witness b div={div:?} count={count}"), bc, br);
                }
            }
        }
        // With count == 1 the `den` is computed but unused: the result must be
        // the raw stored vertex, not a NaN.
        let mut s = rng.simplex(1, 10.0);
        s.div = 0.0;
        let mut s2 = s;
        let cv = (c.c2L)(&mut s);
        let rv = (r.c2L)(&mut s2);
        eq_v("c2L count=1 div=0 uses no den", cv, rv);
        eq_v("c2L count=1 returns a.p verbatim", s.verts[0].p, cv);
    }
}

// ===========================================================================
// Row 33 — a degenerate capsule (`B.a == B.b`) makes `n == (0,0)`.  Because
//          `da == 0` is NOT `< 0` and `db == 0` is NOT `< 0`, the C skips the
//          perpendicular branch entirely and takes the `bp` branch, so the
//          `da / c2Dot(n,n)` division-by-zero is UNREACHABLE.  This test pins
//          that (both libs agree, and the result equals a plain circle test
//          against the point `B.b`).
// ===========================================================================

#[test]
fn c33_degenerate_capsule_takes_the_bp_branch() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC33);
    unsafe {
        for i in 0..4000 {
            let p = if rng.below(4) == 0 {
                rng.wild_v()
            } else {
                rng.v(50.0)
            };
            let q = if rng.below(8) == 0 {
                rng.wild_v()
            } else {
                rng.v(50.0)
            };
            let cir = c2Circle {
                p,
                r: rng.radius(20.0),
            };
            let cap = c2Capsule {
                a: q,
                b: q,
                r: rng.radius(20.0),
            };
            let cv = (c.c2CircletoCapsule)(cir, cap);
            let rv = (r.c2CircletoCapsule)(cir, cap);
            eq_int(&format!("degenerate capsule #{i}"), cv, rv);

            // Equivalent to a circle-vs-circle test at B.b …
            let equiv = (c.c2CircletoCircle)(
                c2Circle { p: q, r: cap.r },
                c2Circle { p, r: cir.r },
            );
            eq_int(
                &format!("degenerate capsule == circle at B.b #{i}"),
                equiv,
                cv,
            );
        }
        // Also: a *near*-degenerate capsule whose `c2Dot(n,n)` underflows to 0.
        for k in 0..64 {
            let e = 1.0e-23f32 * (1.0 + k as f32);
            let cap = c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: e, y: e },
                r: 1.0,
            };
            for p in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: e * 0.5, y: e * 0.5 },
                c2v { x: 1.0e20, y: -1.0e20 },
                c2v { x: -1.0e20, y: 1.0e20 },
                c2v { x: 0.5, y: 0.5 },
            ] {
                let cir = c2Circle { p, r: 2.0 };
                eq_int(
                    &format!("underflow capsule k={k} p={p:?}"),
                    (c.c2CircletoCapsule)(cir, cap),
                    (r.c2CircletoCapsule)(cir, cap),
                );
            }
        }
    }
}

// ===========================================================================
// Row 34 — strict `<`: exactly-touching shapes are NOT a collision.
// ===========================================================================

#[test]
fn c34_exact_touch_is_not_a_collision() {
    let (c, r) = libs();
    unsafe {
        // circle/circle: d2 == r2 exactly
        for r1 in 1..12i32 {
            for r2 in 1..12i32 {
                let a = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: r1 as f32,
                };
                let b = c2Circle {
                    p: c2v {
                        x: (r1 + r2) as f32,
                        y: 0.0,
                    },
                    r: r2 as f32,
                };
                let cv = (c.c2CircletoCircle)(a, b);
                eq_int("touch c/c", cv, (r.c2CircletoCircle)(a, b));
                eq_int("touching circles must NOT collide", 0, cv);
            }
        }
        // circle/aabb: the circle grazes an edge
        let bb = c2AABB {
            min: c2v { x: -4.0, y: -4.0 },
            max: c2v { x: 4.0, y: 4.0 },
        };
        for rad in 1..12i32 {
            let a = c2Circle {
                p: c2v {
                    x: 4.0 + rad as f32,
                    y: 0.0,
                },
                r: rad as f32,
            };
            let cv = (c.c2CircletoAABB)(a, bb);
            eq_int("touch c/bb", cv, (r.c2CircletoAABB)(a, bb));
            eq_int("grazing circle must NOT collide", 0, cv);
        }
        // circle/capsule: the circle grazes the side of the capsule
        let cap = c2Capsule {
            a: c2v { x: -8.0, y: 0.0 },
            b: c2v { x: 8.0, y: 0.0 },
            r: 2.0,
        };
        for rad in 1..12i32 {
            let a = c2Circle {
                p: c2v {
                    x: 0.0,
                    y: 2.0 + rad as f32,
                },
                r: rad as f32,
            };
            let cv = (c.c2CircletoCapsule)(a, cap);
            eq_int("touch c/cap", cv, (r.c2CircletoCapsule)(a, cap));
            eq_int("grazing circle must NOT collide", 0, cv);
        }
    }
}

// ===========================================================================
// Row 35 — c2AABBtoAABB with NaN / inverted boxes reports a HIT.
// ===========================================================================

#[test]
fn c35_aabb_nan_and_inverted_report_hit() {
    let (c, r) = libs();
    unsafe {
        let nan = f32::NAN;
        let cases = [
            c2AABB {
                min: c2v { x: nan, y: nan },
                max: c2v { x: nan, y: nan },
            },
            c2AABB {
                min: c2v { x: nan, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            },
            c2AABB {
                min: c2v { x: 0.0, y: nan },
                max: c2v { x: 1.0, y: 1.0 },
            },
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: nan, y: 1.0 },
            },
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 1.0, y: nan },
            },
        ];
        for (i, bb) in cases.iter().enumerate() {
            let cv = (c.c2AABBtoAABB)(*bb, *bb);
            eq_int(&format!("nan aabb {i}"), cv, (r.c2AABBtoAABB)(*bb, *bb));
            eq_int("all-NaN comparisons are false -> hit", 1, cv);
            // and against a far-away normal box
            let far = c2AABB {
                min: c2v {
                    x: 1000.0,
                    y: 1000.0,
                },
                max: c2v {
                    x: 1001.0,
                    y: 1001.0,
                },
            };
            eq_int(
                &format!("nan aabb {i} vs far"),
                (c.c2AABBtoAABB)(*bb, far),
                (r.c2AABBtoAABB)(*bb, far),
            );
            eq_int(
                &format!("far vs nan aabb {i}"),
                (c.c2AABBtoAABB)(far, *bb),
                (r.c2AABBtoAABB)(far, *bb),
            );
        }
        // Inverted boxes that share no space still report a hit.
        let inv1 = c2AABB {
            min: c2v { x: 10.0, y: 10.0 },
            max: c2v { x: 0.0, y: 0.0 },
        };
        let inv2 = c2AABB {
            min: c2v {
                x: 1000.0,
                y: 1000.0,
            },
            max: c2v { x: 900.0, y: 900.0 },
        };
        let cv = (c.c2AABBtoAABB)(inv1, inv2);
        eq_int("inverted x inverted", cv, (r.c2AABBtoAABB)(inv1, inv2));
        println!("c35: inverted x inverted -> {cv}");
    }
}

// ===========================================================================
// Row 36 — a NaN distance from c2GJK is reported as "no collision" by the
//          two GJK-backed booleans (`if (c2GJK(...))` is true for NaN).
// ===========================================================================

#[test]
fn c36_nan_distance_reports_no_collision() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC36);
    unsafe {
        let nan = f32::NAN;
        let inf = f32::INFINITY;
        let bad_caps = [
            c2Capsule {
                a: c2v { x: nan, y: nan },
                b: c2v { x: nan, y: nan },
                r: nan,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: nan, y: 1.0 },
                r: 1.0,
            },
            c2Capsule {
                a: c2v { x: inf, y: 0.0 },
                b: c2v { x: -inf, y: 0.0 },
                r: 1.0,
            },
            c2Capsule {
                a: c2v { x: inf, y: inf },
                b: c2v { x: inf, y: inf },
                r: inf,
            },
        ];
        let bad_bbs = [
            c2AABB {
                min: c2v { x: nan, y: nan },
                max: c2v { x: nan, y: nan },
            },
            c2AABB {
                min: c2v { x: -inf, y: -inf },
                max: c2v { x: inf, y: inf },
            },
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: nan, y: 1.0 },
            },
        ];
        for (i, cap) in bad_caps.iter().enumerate() {
            for (j, other) in bad_caps.iter().enumerate() {
                eq_int(
                    &format!("c2CapsuletoCapsule nan {i}x{j}"),
                    (c.c2CapsuletoCapsule)(*cap, *other),
                    (r.c2CapsuletoCapsule)(*cap, *other),
                );
            }
            for (j, bb) in bad_bbs.iter().enumerate() {
                eq_int(
                    &format!("c2AABBtoCapsule nan {j}x{i}"),
                    (c.c2AABBtoCapsule)(*bb, *cap),
                    (r.c2AABBtoCapsule)(*bb, *cap),
                );
            }
            // NaN capsule vs a perfectly ordinary one
            let ok = c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 1.0, y: 0.0 },
                r: 1.0,
            };
            eq_int(
                &format!("c2CapsuletoCapsule nan {i} x ok"),
                (c.c2CapsuletoCapsule)(*cap, ok),
                (r.c2CapsuletoCapsule)(*cap, ok),
            );
            eq_int(
                &format!("c2CapsuletoCapsule ok x nan {i}"),
                (c.c2CapsuletoCapsule)(ok, *cap),
                (r.c2CapsuletoCapsule)(ok, *cap),
            );
        }
        // Randomized wild inputs.
        for i in 0..4000 {
            let a = c2Capsule {
                a: rng.wild_v(),
                b: rng.wild_v(),
                r: rng.wild_f32(),
            };
            let b = c2Capsule {
                a: rng.wild_v(),
                b: rng.wild_v(),
                r: rng.wild_f32(),
            };
            let bb = c2AABB {
                min: rng.wild_v(),
                max: rng.wild_v(),
            };
            eq_int(
                &format!("c2CapsuletoCapsule wild #{i}"),
                (c.c2CapsuletoCapsule)(a, b),
                (r.c2CapsuletoCapsule)(a, b),
            );
            eq_int(
                &format!("c2AABBtoCapsule wild #{i}"),
                (c.c2AABBtoCapsule)(bb, a),
                (r.c2AABBtoCapsule)(bb, a),
            );
        }
    }
}

// ===========================================================================
// Row 37 — c2Clampv with an inverted box silently returns `lo`.
// ===========================================================================

#[test]
fn c37_clampv_inverted_box() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC37);
    unsafe {
        for i in 0..4000 {
            let lo = c2v { x: 10.0, y: 20.0 };
            let hi = c2v { x: -10.0, y: -20.0 };
            let a = if rng.below(4) == 0 {
                rng.wild_v()
            } else {
                rng.v(100.0)
            };
            let cv = (c.c2Clampv)(a, lo, hi);
            let rv = (r.c2Clampv)(a, lo, hi);
            eq_v(&format!("c2Clampv inverted #{i} a={a:?}"), cv, rv);
            if !a.x.is_nan() && !a.y.is_nan() {
                eq_v("inverted clamp collapses to lo", lo, cv);
            }
        }
    }
}

// ===========================================================================
// Row 38 — reverse_collide never errors; the result is always a 3-bit mask.
// ===========================================================================

#[test]
fn c38_reverse_collide_never_errors() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC38);
    unsafe {
        for i in 0..20000 {
            let x = rng.wild_f32();
            let y = rng.wild_f32();
            let rad = rng.wild_f32();
            let cv = (c.reverse_collide)(x, y, rad);
            let rv = (r.reverse_collide)(x, y, rad);
            eq_int(&format!("reverse_collide wild #{i} ({x:?},{y:?},{rad:?})"), cv, rv);
            assert!(
                (0..8).contains(&cv),
                "reverse_collide returned {cv}, outside the 3-bit mask range"
            );
        }
        // negative radii still collide (r*r is positive)
        for rad in [-1.0f32, -20.0, -1000.0] {
            let cv = (c.reverse_collide)(-70.0, 0.0, rad);
            eq_int("negative radius", cv, (r.reverse_collide)(-70.0, 0.0, rad));
        }
    }
}
