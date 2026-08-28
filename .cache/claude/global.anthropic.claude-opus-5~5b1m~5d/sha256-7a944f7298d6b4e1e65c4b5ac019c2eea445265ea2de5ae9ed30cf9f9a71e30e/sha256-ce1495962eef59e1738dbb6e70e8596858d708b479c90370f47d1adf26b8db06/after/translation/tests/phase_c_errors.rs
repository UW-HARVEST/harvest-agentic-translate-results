//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! Each test constructs the exact invalid input or boundary condition the C
//! code checks for, calls BOTH shared objects, and asserts they produce the
//! same sentinel / error value (not merely "both failed").

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Out-of-range values for the `C2_TYPE` parameters. A C `enum` argument is an
/// `int` at the ABI level, so any of these is a real input the C must handle.
const BAD_TYPES: [c_int; 10] = [
    3,
    4,
    -1,
    -2,
    255,
    256,
    1 << 16,
    c_int::MAX,
    c_int::MIN,
    c_int::MIN + 1,
];

/// The vertex count `c2MakeProxy` writes for each shape type. Cache indices at
/// or above this value read uninitialised memory in the C (ERRORS.md row U4).
fn proxy_count(ty: c_int) -> c_int {
    match ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    }
}

fn some_circle() -> c2Circle {
    c2Circle { p: c2v { x: 1.5, y: -2.5 }, r: 3.0 }
}
fn some_aabb() -> c2AABB {
    c2AABB { min: c2v { x: -1.0, y: -2.0 }, max: c2v { x: 4.0, y: 5.0 } }
}
fn some_capsule() -> c2Capsule {
    c2Capsule { a: c2v { x: -3.0, y: 0.5 }, b: c2v { x: 2.0, y: 6.5 }, r: 1.25 }
}

// ===========================================================================
// Rows 1-4 — c2Collided with out-of-range enum values
// ===========================================================================

#[test]
fn err_collided_bad_typeA() {
    let l = libs();
    let (c, r) = l.pair::<FnCollided>("c2Collided");
    let circle = some_circle();
    let aabb = some_aabb();
    let capsule = some_capsule();
    let shapes: [(*const c_void, c_int); 3] = [
        (&circle as *const _ as *const c_void, C2_TYPE_CIRCLE),
        (&aabb as *const _ as *const c_void, C2_TYPE_AABB),
        (&capsule as *const _ as *const c_void, C2_TYPE_CAPSULE),
    ];
    for &bad in &BAD_TYPES {
        for &(bp, bt) in &shapes {
            // Any A pointer works: the C never dereferences it on this path.
            let cv = unsafe { c(shapes[0].0, bad, bp, bt) };
            let rv = unsafe { r(shapes[0].0, bad, bp, bt) };
            assert_eq!(cv, rv, "c2Collided(typeA={bad}, typeB={bt})");
            assert_eq!(cv, 0, "C must reject typeA={bad} with 0");
        }
        // Also both types invalid at once.
        let cv = unsafe { c(shapes[0].0, bad, shapes[0].0, bad) };
        let rv = unsafe { r(shapes[0].0, bad, shapes[0].0, bad) };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);
    }
}

fn collided_bad_typeB(good: c_int, ptr: *const c_void) {
    let l = libs();
    let (c, r) = l.pair::<FnCollided>("c2Collided");
    for &bad in &BAD_TYPES {
        let cv = unsafe { c(ptr, good, ptr, bad) };
        let rv = unsafe { r(ptr, good, ptr, bad) };
        assert_eq!(cv, rv, "c2Collided(typeA={good}, typeB={bad})");
        assert_eq!(cv, 0, "C must reject typeB={bad} with 0");
    }
}

#[test]
fn err_collided_bad_typeB_circle() {
    let circle = some_circle();
    collided_bad_typeB(C2_TYPE_CIRCLE, &circle as *const _ as *const c_void);
}

#[test]
fn err_collided_bad_typeB_aabb() {
    let aabb = some_aabb();
    collided_bad_typeB(C2_TYPE_AABB, &aabb as *const _ as *const c_void);
}

#[test]
fn err_collided_bad_typeB_capsule() {
    let capsule = some_capsule();
    collided_bad_typeB(C2_TYPE_CAPSULE, &capsule as *const _ as *const c_void);
}

// ===========================================================================
// Row 5 — c2MakeProxy with an out-of-range type leaves *p untouched
// ===========================================================================

#[test]
fn err_makeproxy_bad_type_leaves_proxy_untouched() {
    let l = libs();
    let (c, r) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let circle = some_circle();
    let mut rng = Rng::new(0x2005);
    for &bad in &BAD_TYPES {
        for _ in 0..64 {
            // Identical non-zero garbage in both buffers.
            let mut dirty = c2Proxy::default();
            dirty.radius = rng.range(-100.0, 100.0);
            dirty.count = rng.next_u32() as c_int;
            for k in 0..8 {
                dirty.verts[k] = rng.vec_coord(64.0);
            }
            let mut cp = dirty;
            let mut rp = dirty;
            unsafe { c(&circle as *const _ as *const c_void, bad, &mut cp) };
            unsafe { r(&circle as *const _ as *const c_void, bad, &mut rp) };
            assert!(
                proxy_eq(&cp, &rp),
                "c2MakeProxy(type={bad})\nC:\n{}\nRUST:\n{}",
                proxy_desc(&cp),
                proxy_desc(&rp)
            );
            assert!(
                proxy_eq(&cp, &dirty),
                "the C switch has no `default:` so nothing may be written (type={bad})"
            );
        }
    }
}

// ===========================================================================
// Rows 6-11 — c2GJK null-pointer arguments
// ===========================================================================

type GjkCall = (Option<c2x>, Option<c2x>, bool, bool, bool, bool);

fn gjk_raw(
    f: FnGJK,
    a: &Shape,
    b: &Shape,
    call: &GjkCall,
    cache_init: c2GJKCache,
) -> (f32, c2v, c2v, c_int, c2GJKCache) {
    let (ax, bx, want_a, want_b, want_it, want_cache) = *call;
    let mut oa = c2v { x: -7.5e8, y: 6.25e8 };
    let mut ob = c2v { x: 3.125e8, y: -1.5e8 };
    let mut it: c_int = -424242;
    let mut cache = cache_init;
    let dist = unsafe {
        f(
            a.as_ptr(),
            a.ty(),
            ax.as_ref().map_or(std::ptr::null(), |v| v as *const c2x),
            b.as_ptr(),
            b.ty(),
            bx.as_ref().map_or(std::ptr::null(), |v| v as *const c2x),
            if want_a { &mut oa } else { std::ptr::null_mut() },
            if want_b { &mut ob } else { std::ptr::null_mut() },
            1,
            if want_it { &mut it } else { std::ptr::null_mut() },
            if want_cache { &mut cache } else { std::ptr::null_mut() },
        )
    };
    (dist, oa, ob, it, cache)
}

fn assert_raw_eq(ctx: &str, c: &(f32, c2v, c2v, c_int, c2GJKCache), r: &(f32, c2v, c2v, c_int, c2GJKCache)) {
    assert!(
        feq(c.0, r.0) && veq(c.1, r.1) && veq(c.2, r.2) && c.3 == r.3 && cache_eq(&c.4, &r.4),
        "{ctx}\n  C   : dist={} a={} b={} it={} cache={}\n  RUST: dist={} a={} b={} it={} cache={}",
        fdesc(c.0), vdesc(c.1), vdesc(c.2), c.3, cache_desc(&c.4),
        fdesc(r.0), vdesc(r.1), vdesc(r.2), r.3, cache_desc(&r.4),
    );
}

fn gjk_pairs(rng: &mut Rng) -> Vec<(Shape, Shape)> {
    let mut out = Vec::new();
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..12 {
                let scale = SCALES[rng.below(SCALES.len())];
                out.push((gen_shape(rng, tyA, scale), gen_shape(rng, tyB, scale)));
            }
        }
    }
    out
}

#[test]
fn err_gjk_null_ax() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let mut rng = Rng::new(0x2006);
    for (a, b) in gjk_pairs(&mut rng) {
        let null_call: GjkCall = (None, None, true, true, true, false);
        let ident_call: GjkCall = (Some(ident), None, true, true, true, false);
        let cn = gjk_raw(cf, &a, &b, &null_call, c2GJKCache::default());
        let rn = gjk_raw(rf, &a, &b, &null_call, c2GJKCache::default());
        assert_raw_eq("ax_ptr = NULL", &cn, &rn);
        // NULL must be exactly equivalent to c2xIdentity() in both libraries.
        let ci = gjk_raw(cf, &a, &b, &ident_call, c2GJKCache::default());
        let ri = gjk_raw(rf, &a, &b, &ident_call, c2GJKCache::default());
        assert_raw_eq("ax_ptr = identity", &ci, &ri);
        assert_raw_eq("NULL ax must equal identity ax (C)", &cn, &ci);
    }
}

#[test]
fn err_gjk_null_bx() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let mut rng = Rng::new(0x2007);
    for (a, b) in gjk_pairs(&mut rng) {
        let null_call: GjkCall = (None, None, true, true, true, false);
        let ident_call: GjkCall = (None, Some(ident), true, true, true, false);
        let cn = gjk_raw(cf, &a, &b, &null_call, c2GJKCache::default());
        let rn = gjk_raw(rf, &a, &b, &null_call, c2GJKCache::default());
        assert_raw_eq("bx_ptr = NULL", &cn, &rn);
        let ci = gjk_raw(cf, &a, &b, &ident_call, c2GJKCache::default());
        let ri = gjk_raw(rf, &a, &b, &ident_call, c2GJKCache::default());
        assert_raw_eq("bx_ptr = identity", &ci, &ri);
        assert_raw_eq("NULL bx must equal identity bx (C)", &cn, &ci);
    }
}

#[test]
fn err_gjk_null_outputs() {
    // Rows 8, 9, 10, 11 — every subset of {outA, outB, iterations, cache}.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2008);
    let pairs = gjk_pairs(&mut rng);
    for mask in 0..16u32 {
        let call: GjkCall = (
            None,
            None,
            mask & 1 != 0,
            mask & 2 != 0,
            mask & 4 != 0,
            mask & 8 != 0,
        );
        for (a, b) in &pairs {
            let cv = gjk_raw(cf, a, b, &call, c2GJKCache::default());
            let rv = gjk_raw(rf, a, b, &call, c2GJKCache::default());
            assert_raw_eq(&format!("null-output mask {mask}"), &cv, &rv);
            // Sentinels must survive when the store is skipped.
            if !call.2 {
                assert!(veq(cv.1, c2v { x: -7.5e8, y: 6.25e8 }), "outA=NULL still wrote");
            }
            if !call.3 {
                assert!(veq(cv.2, c2v { x: 3.125e8, y: -1.5e8 }), "outB=NULL still wrote");
            }
            if !call.4 {
                assert_eq!(cv.3, -424242, "iterations=NULL still wrote");
            }
            if !call.5 {
                assert!(cache_eq(&cv.4, &c2GJKCache::default()), "cache=NULL still wrote");
                assert!(cache_eq(&rv.4, &c2GJKCache::default()), "cache=NULL still wrote (rust)");
            }
        }
    }
}

// ===========================================================================
// Rows 12-14, U3 — cache validation
// ===========================================================================

#[test]
fn err_gjk_cache_count_zero_rejected() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2012);
    for (a, b) in gjk_pairs(&mut rng) {
        // `count == 0` => `cache_was_good` is false: everything else in the
        // cache (metric, indices, div) must be ignored.
        let poison = c2GJKCache {
            metric: 12345.678,
            count: 0,
            iA: [2, 1, 3],
            iB: [3, 2, 1],
            div: -98765.4,
        };
        let call: GjkCall = (None, None, true, true, true, true);
        let cv = gjk_raw(cf, &a, &b, &call, poison);
        let rv = gjk_raw(rf, &a, &b, &call, poison);
        assert_raw_eq("cache->count == 0", &cv, &rv);
        // Proof that the poisoned fields were ignored on the way *in*: the
        // result must be identical to the zeroed-cache call. Note the C only
        // writes back `iA[i]`/`iB[i]` for `i < s.count`, so the trailing index
        // slots keep their poisoned values and are excluded from this
        // comparison (the C-vs-Rust check above already covers them).
        let cz = gjk_raw(cf, &a, &b, &call, c2GJKCache::default());
        let n = cv.4.count.clamp(0, 3) as usize;
        assert!(
            feq(cv.0, cz.0)
                && veq(cv.1, cz.1)
                && veq(cv.2, cz.2)
                && cv.3 == cz.3
                && feq(cv.4.metric, cz.4.metric)
                && cv.4.count == cz.4.count
                && feq(cv.4.div, cz.4.div)
                && cv.4.iA[..n] == cz.4.iA[..n]
                && cv.4.iB[..n] == cz.4.iB[..n],
            "a poisoned count==0 cache must behave exactly like a zeroed cache in the C\n  poisoned: dist={} cache={}\n  zeroed  : dist={} cache={}",
            fdesc(cv.0), cache_desc(&cv.4), fdesc(cz.0), cache_desc(&cz.4)
        );
        // The trailing slots really are left alone by the C.
        for k in n..3 {
            assert_eq!(cv.4.iA[k], poison.iA[k], "iA[{k}] must be untouched");
            assert_eq!(cv.4.iB[k], poison.iB[k], "iB[{k}] must be untouched");
        }
    }
}

#[test]
fn err_gjk_cache_reuse_always_accepted() {
    // The `metric < -1.0e8f` conjunct makes the staleness test false for every
    // finite metric, so a non-empty cache is always replayed. Feed back a cache
    // from a *different* shape pair to exercise that quirk.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2013);
    let call: GjkCall = (None, None, true, true, true, true);
    let mut replayed = 0usize;
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..60 {
                let scale = SCALES[rng.below(SCALES.len())];
                let (a0, b0) = (gen_shape(&mut rng, tyA, scale), gen_shape(&mut rng, tyB, scale));
                let seed_c = gjk_raw(cf, &a0, &b0, &call, c2GJKCache::default());
                let seed_r = gjk_raw(rf, &a0, &b0, &call, c2GJKCache::default());
                assert_raw_eq("cache seeding call", &seed_c, &seed_r);
                // Now a *different* pair of the same types, replaying the cache.
                let (a1, b1) = (gen_shape(&mut rng, tyA, scale), gen_shape(&mut rng, tyB, scale));
                let cv = gjk_raw(cf, &a1, &b1, &call, seed_c.4);
                let rv = gjk_raw(rf, &a1, &b1, &call, seed_r.4);
                assert_raw_eq("stale cache replay", &cv, &rv);
                if seed_c.4.count != 0 {
                    replayed += 1;
                }
            }
        }
    }
    assert!(replayed > 0, "no cache was ever non-empty");
}

#[test]
fn err_gjk_cache_nonfinite_metric() {
    // metric = NaN / +-inf / huge negative: the `?:` min/max picks and the
    // `metric < -1.0e8f` test must behave identically.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2014);
    let metrics = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_dead),
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0e8,
        -1.0e9,
        -1.000_000_1e8,
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
    ];
    let divs = [1.0f32, 0.0, -0.0, -1.0, f32::NAN, f32::INFINITY, 1.0e-30, 1.0e30];
    let call: GjkCall = (None, None, true, true, true, true);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for &metric in &metrics {
                for &div in &divs {
                    let scale = SCALES[rng.below(SCALES.len())];
                    let a = gen_shape(&mut rng, tyA, scale);
                    let b = gen_shape(&mut rng, tyB, scale);
                    for count in 1..=3 {
                        // Cache indices MUST stay inside `[0, proxy.count)`:
                        // the C reads `pA.verts[iA]` and anything at or above
                        // `c2MakeProxy`'s vertex count is uninitialised stack
                        // memory there (documented UB row U4).
                        let (na, nb) = (proxy_count(tyA), proxy_count(tyB));
                        let cache = c2GJKCache {
                            metric,
                            count,
                            iA: [0, 1 % na, 0],
                            iB: [0, 1 % nb, 0],
                            div,
                        };
                        let cv = gjk_raw(cf, &a, &b, &call, cache);
                        let rv = gjk_raw(rf, &a, &b, &call, cache);
                        assert_raw_eq(
                            &format!("cache metric={} div={} count={count}", fdesc(metric), fdesc(div)),
                            &cv, &rv,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn err_gjk_cache_count_four() {
    // Row U3: `cache->count == 4` makes the C loop read `cache->iA[3]` and
    // `cache->iB[3]`, which are one past the end of the two `int[3]` arrays:
    // `iA[3]` aliases `iB[0]` and `iB[3]` aliases the *float* `div` reinterpreted
    // as an `int`. Both are then used as indices into the 8-element proxy vertex
    // array, so an arbitrary `div` makes the C read wildly out of bounds (a real
    // SIGSEGV). To probe the reachable part of this row we keep every index --
    // including the two aliased ones -- inside `[0, 8)`, which means `div` must
    // have a bit pattern in `0..8` (i.e. +0.0 or a tiny subnormal).
    //
    // Under that constraint the observable outputs are provably independent of
    // the vertex values read, because a 4-vertex simplex takes the `default:`
    // arms of `c2L` / `c2D` / `c2Witness` and breaks on the epsilon test during
    // the first iteration.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2015);
    let call: GjkCall = (None, None, true, true, true, true);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for div_bits in 0u32..8 {
                for _ in 0..20 {
                    let scale = SCALES[rng.below(SCALES.len())];
                    let a = gen_shape(&mut rng, tyA, scale);
                    let b = gen_shape(&mut rng, tyB, scale);
                    let cache = c2GJKCache {
                        metric: rng.range(-1.0e3, 1.0e3),
                        count: 4,
                        iA: [0, 1, 0],
                        // iB[0] is also read as iA[3]; keep it in range.
                        iB: [1, 0, 1],
                        // div is also read as iB[3]; keep its bit pattern in range.
                        div: f32::from_bits(div_bits),
                    };
                    let cv = gjk_raw(cf, &a, &b, &call, cache);
                    let rv = gjk_raw(rf, &a, &b, &call, cache);
                    assert_raw_eq("cache->count == 4", &cv, &rv);
                    assert_eq!(cv.4.count, 4, "the C keeps count == 4");
                    assert!(feq(cv.0, 0.0), "a 4-vertex simplex always yields dist 0");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 15-20 — c2GJK use_radius / hit / iteration cap
// ===========================================================================

#[test]
fn err_gjk_use_radius_zero() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2016);
    // Radius-bearing shapes only, so the skipped block matters.
    for _ in 0..600 {
        let scale = SCALES[rng.below(SCALES.len())];
        for &(tyA, tyB) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_CAPSULE),
            (C2_TYPE_CAPSULE, C2_TYPE_CAPSULE),
        ] {
            let a = gen_shape(&mut rng, tyA, scale);
            let b = gen_shape(&mut rng, tyB, scale);
            let mut oa0 = c2v::default();
            let mut ob0 = c2v::default();
            let mut it0: c_int = 0;
            let mut oa1 = c2v::default();
            let mut ob1 = c2v::default();
            let mut it1: c_int = 0;
            let d_c = unsafe {
                cf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
                   &mut oa0, &mut ob0, 0, &mut it0, std::ptr::null_mut())
            };
            let d_r = unsafe {
                rf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
                   &mut oa1, &mut ob1, 0, &mut it1, std::ptr::null_mut())
            };
            assert!(
                feq(d_c, d_r) && veq(oa0, oa1) && veq(ob0, ob1) && it0 == it1,
                "use_radius = 0: A={a:?} B={b:?} C=(dist={} a={} b={} it={it0}) R=(dist={} a={} b={} it={it1})",
                fdesc(d_c), vdesc(oa0), vdesc(ob0), fdesc(d_r), vdesc(oa1), vdesc(ob1)
            );
        }
    }
}

#[test]
fn err_gjk_use_radius_truthy_values() {
    // `else if (use_radius)` is a truthiness test: 2, -1, INT_MIN must behave
    // exactly like 1.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2017);
    let truthy: [c_int; 7] = [1, 2, -1, 255, 1 << 30, c_int::MAX, c_int::MIN];
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..25 {
                let scale = SCALES[rng.below(SCALES.len())];
                let a = gen_shape(&mut rng, tyA, scale);
                let b = gen_shape(&mut rng, tyB, scale);
                let mut reference: Option<(u32, c2v, c2v)> = None;
                for &ur in &truthy {
                    let mut ca = c2v::default();
                    let mut cb = c2v::default();
                    let mut ra = c2v::default();
                    let mut rb = c2v::default();
                    let dc = unsafe {
                        cf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
                           &mut ca, &mut cb, ur, std::ptr::null_mut(), std::ptr::null_mut())
                    };
                    let dr = unsafe {
                        rf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
                           &mut ra, &mut rb, ur, std::ptr::null_mut(), std::ptr::null_mut())
                    };
                    assert!(
                        feq(dc, dr) && veq(ca, ra) && veq(cb, rb),
                        "use_radius = {ur}: A={a:?} B={b:?} C dist={} R dist={}",
                        fdesc(dc), fdesc(dr)
                    );
                    match reference {
                        None => reference = Some((dc.to_bits(), ca, cb)),
                        Some((d, x, y)) => assert!(
                            d == dc.to_bits() && veq(x, ca) && veq(y, cb),
                            "use_radius = {ur} differs from use_radius = 1 in the C itself"
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn err_gjk_radius_else_branch_midpoint() {
    // Deeply overlapping radius shapes: `!(dist > rA+rB && dist > eps)` so the
    // midpoint / dist = 0 branch is taken.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2018);
    let mut hits = 0usize;
    for _ in 0..1200 {
        // Concentric-ish circles with large radii: the core distance is always
        // smaller than rA + rB.
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rng.range(5.0, 50.0) });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: rng.range(-2.0, 2.0), y: rng.range(-2.0, 2.0) },
            r: rng.range(5.0, 50.0),
        });
        let mut ca = c2v::default();
        let mut cb = c2v::default();
        let mut ra = c2v::default();
        let mut rb = c2v::default();
        let dc = unsafe {
            cf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
               &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut())
        };
        let dr = unsafe {
            rf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
               &mut ra, &mut rb, 1, std::ptr::null_mut(), std::ptr::null_mut())
        };
        assert!(feq(dc, dr) && veq(ca, ra) && veq(cb, rb), "midpoint branch: A={a:?} B={b:?}");
        if dc == 0.0 && veq(ca, cb) {
            hits += 1;
        }
    }
    assert!(hits > 0, "midpoint branch never taken");
}

#[test]
fn err_gjk_radius_shrink_exact_equal() {
    // After the shrink, `a == b` forces `dist = 0` even though `dist -= rA+rB`
    // produced a non-zero value. Reachable when `dist` is only epsilon-above
    // `rA + rB`.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2019);
    let mut zeroed = 0usize;
    for _ in 0..4000 {
        let r = ((rng.next_u32() % 64) as f32) / 8.0;
        let gap = match rng.below(4) {
            0 => 0.0,
            1 => f32::from_bits((2.0f32 * r).to_bits().wrapping_add(1)) - 2.0 * r,
            2 => 1.0e-6,
            _ => rng.range(0.0, 1.0e-3),
        };
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r });
        let b = Shape::Circle(c2Circle { p: c2v { x: 2.0 * r + gap, y: 0.0 }, r });
        let mut ca = c2v::default();
        let mut cb = c2v::default();
        let mut ra = c2v::default();
        let mut rb = c2v::default();
        let dc = unsafe {
            cf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
               &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut())
        };
        let dr = unsafe {
            rf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
               &mut ra, &mut rb, 1, std::ptr::null_mut(), std::ptr::null_mut())
        };
        assert!(
            feq(dc, dr) && veq(ca, ra) && veq(cb, rb),
            "shrink-equal: r={r} gap={} C dist={} R dist={}",
            fdesc(gap), fdesc(dc), fdesc(dr)
        );
        if dc == 0.0 {
            zeroed += 1;
        }
    }
    assert!(zeroed > 0, "shrink path never produced dist == 0");
}

#[test]
fn err_gjk_hit_overrides_radius() {
    // When the simplex encloses the origin the `hit` branch wins and the
    // `use_radius` block is never entered (`else if`).
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x201a);
    let mut hits = 0usize;
    for _ in 0..2000 {
        // Heavily overlapping AABBs -> the 3-simplex encloses the origin.
        let a = Shape::Aabb(c2AABB { min: c2v { x: -10.0, y: -10.0 }, max: c2v { x: 10.0, y: 10.0 } });
        let b = Shape::Aabb(c2AABB {
            min: c2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) },
            max: c2v { x: rng.range(5.0, 9.0), y: rng.range(5.0, 9.0) },
        });
        for &ur in &[0, 1] {
            let mut ca = c2v::default();
            let mut cb = c2v::default();
            let mut ra = c2v::default();
            let mut rb = c2v::default();
            let mut cit: c_int = 0;
            let mut rit: c_int = 0;
            let dc = unsafe {
                cf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
                   &mut ca, &mut cb, ur, &mut cit, std::ptr::null_mut())
            };
            let dr = unsafe {
                rf(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(), std::ptr::null(),
                   &mut ra, &mut rb, ur, &mut rit, std::ptr::null_mut())
            };
            assert!(
                feq(dc, dr) && veq(ca, ra) && veq(cb, rb) && cit == rit,
                "hit branch: A={a:?} B={b:?} ur={ur}"
            );
            if dc == 0.0 && veq(ca, cb) {
                hits += 1;
            }
        }
    }
    assert!(hits > 0, "hit branch never taken");
}

#[test]
fn err_gjk_iteration_cap() {
    // Sweep hard shapes across many transforms and assert `*iterations` (which
    // is capped at 20) always agrees.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x201b);
    let mut max_it = 0;
    let call: GjkCall = (None, None, true, true, true, true);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..400 {
                let scale = SCALES[rng.below(SCALES.len())];
                let a = gen_shape(&mut rng, tyA, scale);
                let b = gen_shape(&mut rng, tyB, scale);
                let cv = gjk_raw(cf, &a, &b, &call, c2GJKCache::default());
                let rv = gjk_raw(rf, &a, &b, &call, c2GJKCache::default());
                assert_raw_eq("iteration cap", &cv, &rv);
                assert!(cv.3 >= 0 && cv.3 <= 20, "iterations out of range: {}", cv.3);
                max_it = max_it.max(cv.3);
            }
        }
    }
    assert!(max_it >= 1, "the loop never iterated");
}

#[test]
fn err_gjk_break_paths_reachable() {
    // Rows 21 and 23: `d1 > d0` and duplicate-support breaks. Both are only
    // reachable through the composed pipeline, so drive many transforms and
    // shapes and demand a spread of iteration counts (each distinct count
    // corresponds to a different exit point being taken).
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x201c);
    let mut counts = std::collections::BTreeSet::new();
    let call_variants: [GjkCall; 3] = [
        (None, None, true, true, true, true),
        (None, None, true, true, true, false),
        (None, None, false, false, true, true),
    ];
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..500 {
                let scale = SCALES[rng.below(SCALES.len())];
                let a = gen_shape(&mut rng, tyA, scale);
                let b = gen_shape(&mut rng, tyB, scale);
                for call in &call_variants {
                    let mut call = *call;
                    call.0 = if rng.boolean() { Some(gen_x(&mut rng, scale)) } else { None };
                    call.1 = if rng.boolean() { Some(gen_x(&mut rng, scale)) } else { None };
                    let cv = gjk_raw(cf, &a, &b, &call, c2GJKCache::default());
                    let rv = gjk_raw(rf, &a, &b, &call, c2GJKCache::default());
                    assert_raw_eq("break paths", &cv, &rv);
                    if call.4 {
                        counts.insert(cv.3);
                    }
                }
            }
        }
    }
    assert!(counts.len() >= 3, "only these iteration counts seen: {counts:?}");
}

#[test]
fn err_gjk_break_degenerate_direction() {
    // Row 22: `c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON`.
    // With a 1-vertex simplex exactly at the origin, `c2D` returns -(0,0) and
    // the loop breaks on iteration 0.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let call: GjkCall = (None, None, true, true, true, true);
    let mut rng = Rng::new(0x201d);
    let mut zero_iters = 0usize;
    for _ in 0..2000 {
        let p = c2v { x: rng.coord(8.0), y: rng.coord(8.0) };
        // Two zero-radius circles at exactly the same place: sB - sA == (0,0).
        let a = Shape::Circle(c2Circle { p, r: 0.0 });
        let b = Shape::Circle(c2Circle { p, r: 0.0 });
        let cv = gjk_raw(cf, &a, &b, &call, c2GJKCache::default());
        let rv = gjk_raw(rf, &a, &b, &call, c2GJKCache::default());
        assert_raw_eq("degenerate direction", &cv, &rv);
        if cv.3 == 0 {
            zero_iters += 1;
        }
        // And with sub-epsilon separation, which also trips the same test.
        let tiny = c2v { x: p.x + 1.0e-9, y: p.y };
        let b2 = Shape::Circle(c2Circle { p: tiny, r: 0.0 });
        let cv = gjk_raw(cf, &a, &b2, &call, c2GJKCache::default());
        let rv = gjk_raw(rf, &a, &b2, &call, c2GJKCache::default());
        assert_raw_eq("sub-epsilon direction", &cv, &rv);
    }
    assert!(zero_iters > 0, "epsilon break on iteration 0 never taken");
}

#[test]
fn err_gjk_coincident_points() {
    // Row 24.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let call: GjkCall = (None, None, true, true, true, true);
    for &r in &[0.0f32, 1.0, 1.0e-8, 1.0e8, -1.0] {
        for &ur in &[0, 1] {
            let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r });
            let b = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r });
            let mut call = call;
            call.2 = ur != 0;
            let cv = gjk_raw(cf, &a, &b, &call, c2GJKCache::default());
            let rv = gjk_raw(rf, &a, &b, &call, c2GJKCache::default());
            assert_raw_eq(&format!("coincident r={r} ur={ur}"), &cv, &rv);
            assert_eq!(cv.3, 0, "coincident points must break on iteration 0");
        }
    }
}

#[test]
fn err_gjk_nonfinite_shapes() {
    // Row 25: NaN / +-inf shape data. All the `<`/`>` tests become false so the
    // control flow must match exactly and the NaN/inf must propagate the same.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x201e);
    let call_base: GjkCall = (None, None, true, true, true, true);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..300 {
                let a = match tyA {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.vec_wild(), r: rng.wild() }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.vec_wild(), max: rng.vec_wild() }),
                    _ => Shape::Capsule(c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() }),
                };
                let b = match tyB {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.vec_wild(), r: rng.wild() }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.vec_wild(), max: rng.vec_wild() }),
                    _ => Shape::Capsule(c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() }),
                };
                for &ur in &[false, true] {
                    let mut call = call_base;
                    call.2 = ur;
                    call.0 = if rng.boolean() {
                        Some(c2x { p: rng.vec_wild(), r: c2r { c: rng.wild(), s: rng.wild() } })
                    } else {
                        None
                    };
                    let cv = gjk_raw(cf, &a, &b, &call, c2GJKCache::default());
                    let rv = gjk_raw(rf, &a, &b, &call, c2GJKCache::default());
                    assert_raw_eq("non-finite shapes", &cv, &rv);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 26-32 — simplex helpers with out-of-range `count` / zero `div`
// ===========================================================================

/// `count` values that hit the `default:` arms.
const BAD_COUNTS: [c_int; 10] = [0, 4, 5, 6, 7, -1, -2, 100, c_int::MAX, c_int::MIN];

#[test]
fn err_metric_out_of_range_count() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0x2026);
    for &count in &BAD_COUNTS {
        for _ in 0..200 {
            let wild = rng.below(3) == 0;
            let s = gen_simplex(&mut rng, count, 64.0, wild);
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
            assert!(
                feq(x, y),
                "c2GJKSimplexMetric(count={count}) C={} R={}",
                fdesc(x), fdesc(y)
            );
            assert!(feq(x, 0.0), "the `default:` arm falls into `case 1:` => 0.0f");
        }
    }
    // `count == 1` also returns 0 (the labelled case).
    for _ in 0..200 {
        let s = gen_simplex(&mut rng, 1, 64.0, false);
        let mut cs = s;
        let mut rs = s;
        let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
        assert!(feq(x, y) && feq(x, 0.0));
    }
}

#[test]
fn err_c2d_out_of_range_count() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexV>("c2D");
    let mut rng = Rng::new(0x2027);
    for &count in &BAD_COUNTS.iter().copied().chain([3]).collect::<Vec<_>>() {
        for _ in 0..200 {
            let wild = rng.below(3) == 0;
            let s = gen_simplex(&mut rng, count, 64.0, wild);
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
            assert!(veq(x, y), "c2D(count={count}) C={} R={}", vdesc(x), vdesc(y));
            assert!(veq(x, c2v { x: 0.0, y: 0.0 }), "case 3 / default must give (0,0)");
        }
    }
}

#[test]
fn err_c2d_det_not_positive() {
    // `if (c2Det2(ab, -a) > 0)` is false for `== 0` and for NaN, giving
    // `c2CCW90` instead of `c2Skew`.
    let l = libs();
    let (c, r) = l.pair::<FnSimplexV>("c2D");
    let mut rng = Rng::new(0x2028);
    let mut ccw = 0usize;
    for _ in 0..3000 {
        let mut s = c2Simplex::default();
        s.count = 2;
        s.div = 1.0;
        // Collinear with the origin => det == 0 exactly.
        let t = rng.coord(8.0);
        let u = rng.coord(8.0);
        s.verts[0].p = c2v { x: t, y: t };
        s.verts[1].p = c2v { x: u, y: u };
        let mut cs = s;
        let mut rs = s;
        let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
        assert!(veq(x, y), "collinear c2D C={} R={}", vdesc(x), vdesc(y));
        let abx = u - t;
        let aby = u - t;
        if feq(x.x, aby) && feq(x.y, -abx) {
            ccw += 1;
        }
        // NaN determinant.
        s.verts[0].p = c2v { x: f32::NAN, y: rng.coord(8.0) };
        s.verts[1].p = rng.vec_wild();
        let mut cs = s;
        let mut rs = s;
        let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
        assert!(veq(x, y), "NaN c2D C={} R={}", vdesc(x), vdesc(y));
    }
    assert!(ccw > 0, "the c2CCW90 arm was never taken on a zero determinant");
}

#[test]
fn err_c2l_out_of_range_count() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexV>("c2L");
    let mut rng = Rng::new(0x2029);
    // Note `case 3:` is NOT handled by c2L: it falls into `default:`.
    for &count in &BAD_COUNTS.iter().copied().chain([3]).collect::<Vec<_>>() {
        for _ in 0..200 {
            let wild = rng.below(3) == 0;
            let s = gen_simplex(&mut rng, count, 64.0, wild);
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
            assert!(veq(x, y), "c2L(count={count}) C={} R={}", vdesc(x), vdesc(y));
            assert!(veq(x, c2v { x: 0.0, y: 0.0 }), "default must give (0,0) for count={count}");
        }
    }
}

#[test]
fn err_c2l_zero_div() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexV>("c2L");
    let mut rng = Rng::new(0x202a);
    let divs = [0.0f32, -0.0, f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE];
    for &div in &divs {
        for count in [1, 2] {
            for _ in 0..200 {
                let mut s = gen_simplex(&mut rng, count, 64.0, false);
                s.div = div;
                let mut cs = s;
                let mut rs = s;
                let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
                assert!(
                    veq(x, y),
                    "c2L(div={}, count={count}) C={} R={}",
                    fdesc(div), vdesc(x), vdesc(y)
                );
            }
        }
    }
}

#[test]
fn err_witness_out_of_range_count() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0x202b);
    for &count in &BAD_COUNTS {
        for _ in 0..200 {
            let wild = rng.below(3) == 0;
            let s = gen_simplex(&mut rng, count, 64.0, wild);
            let mut cs = s;
            let mut rs = s;
            let sent = c2v { x: 9.75e8, y: -8.5e8 };
            let (mut ca, mut cb) = (sent, sent);
            let (mut ra, mut rb) = (sent, sent);
            unsafe { c(&mut cs, &mut ca, &mut cb) };
            unsafe { r(&mut rs, &mut ra, &mut rb) };
            assert!(
                veq(ca, ra) && veq(cb, rb),
                "c2Witness(count={count}) C=({}, {}) R=({}, {})",
                vdesc(ca), vdesc(cb), vdesc(ra), vdesc(rb)
            );
            let zero = c2v { x: 0.0, y: 0.0 };
            assert!(veq(ca, zero) && veq(cb, zero), "default arm must give (0,0)/(0,0)");
        }
    }
}

#[test]
fn err_witness_zero_div() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0x202c);
    let divs = [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE, f32::MAX];
    for &div in &divs {
        for count in [1, 2, 3] {
            for _ in 0..200 {
                let wild = rng.below(4) == 0;
                let mut s = gen_simplex(&mut rng, count, 64.0, wild);
                s.div = div;
                let mut cs = s;
                let mut rs = s;
                let (mut ca, mut cb) = (c2v::default(), c2v::default());
                let (mut ra, mut rb) = (c2v::default(), c2v::default());
                unsafe { c(&mut cs, &mut ca, &mut cb) };
                unsafe { r(&mut rs, &mut ra, &mut rb) };
                assert!(
                    veq(ca, ra) && veq(cb, rb),
                    "c2Witness(div={}, count={count}) C=({}, {}) R=({}, {})",
                    fdesc(div), vdesc(ca), vdesc(cb), vdesc(ra), vdesc(rb)
                );
            }
        }
    }
}

// ===========================================================================
// Rows 33-43 — c22 / c23 branch selection
// ===========================================================================

fn run_c22(s: &c2Simplex) -> c2Simplex {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexVoid>("c22");
    let mut cs = *s;
    let mut rs = *s;
    unsafe { c(&mut cs) };
    unsafe { r(&mut rs) };
    assert!(
        simplex_eq(&cs, &rs),
        "c22 diverged\nINPUT:\n{}\nC:\n{}\nRUST:\n{}",
        simplex_desc(s), simplex_desc(&cs), simplex_desc(&rs)
    );
    cs
}

fn run_c23(s: &c2Simplex) -> c2Simplex {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexVoid>("c23");
    let mut cs = *s;
    let mut rs = *s;
    unsafe { c(&mut cs) };
    unsafe { r(&mut rs) };
    assert!(
        simplex_eq(&cs, &rs),
        "c23 diverged\nINPUT:\n{}\nC:\n{}\nRUST:\n{}",
        simplex_desc(s), simplex_desc(&cs), simplex_desc(&rs)
    );
    cs
}

fn tagged(rng: &mut Rng, pts: &[c2v]) -> c2Simplex {
    let mut s = c2Simplex::default();
    for (i, &p) in pts.iter().enumerate() {
        s.verts[i].p = p;
        s.verts[i].sA = rng.vec_coord(16.0);
        s.verts[i].sB = rng.vec_coord(16.0);
        s.verts[i].u = rng.range(-2.0, 2.0);
        s.verts[i].iA = i as c_int;
        s.verts[i].iB = 10 + i as c_int;
    }
    s.div = 1.0;
    s.count = pts.len() as c_int;
    s
}

#[test]
fn err_c22_all_branches() {
    // Rows 33, 34 and the edge arm: construct each region explicitly.
    let mut rng = Rng::new(0x2033);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..3000 {
        // v <= 0 : origin beyond `a` (a is between origin and b, pointing away)
        let d = rng.range(1.0, 20.0);
        let e = d + rng.range(0.5, 20.0);
        let s = tagged(&mut rng, &[c2v { x: d, y: 0.0 }, c2v { x: e, y: 0.0 }]);
        let out = run_c22(&s);
        seen.insert((out.count, out.verts[0].iA));
        assert_eq!(out.count, 1);
        assert_eq!(out.verts[0].iA, 0, "must keep vertex a");
        assert!(feq(out.verts[0].u, 1.0) && feq(out.div, 1.0));

        // u <= 0 : origin beyond `b`
        let s = tagged(&mut rng, &[c2v { x: e, y: 0.0 }, c2v { x: d, y: 0.0 }]);
        let out = run_c22(&s);
        seen.insert((out.count, out.verts[0].iA));
        assert_eq!(out.count, 1);
        assert_eq!(out.verts[0].iA, 1, "must copy vertex b into slot a");
        assert!(feq(out.verts[0].u, 1.0) && feq(out.div, 1.0));

        // edge arm : origin projects inside the segment
        let s = tagged(&mut rng, &[c2v { x: -d, y: 3.0 }, c2v { x: e, y: 3.0 }]);
        let out = run_c22(&s);
        seen.insert((out.count, out.verts[0].iA));
        assert_eq!(out.count, 2);
    }
    assert_eq!(seen.len(), 3, "all three c22 arms: {seen:?}");
}

#[test]
fn err_c22_degenerate_equal_points() {
    // Row 35: a == b => u == v == 0 => the *first* test (`v <= 0`) wins.
    let mut rng = Rng::new(0x2035);
    for _ in 0..2000 {
        let p = match rng.below(4) {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => c2v { x: -0.0, y: 0.0 },
            2 => rng.vec_wild(),
            _ => rng.vec_coord(64.0),
        };
        let s = tagged(&mut rng, &[p, p]);
        let out = run_c22(&s);
        if p.x.is_finite() && p.y.is_finite() {
            assert_eq!(out.count, 1);
            assert_eq!(out.verts[0].iA, 0, "the `v <= 0` arm must win for a == b");
        }
    }
}

#[test]
fn err_c23_all_branches() {
    // Rows 36-41 and 43: every one of the seven arms.
    let mut rng = Rng::new(0x2036);
    let mut seen: std::collections::BTreeMap<(c_int, c_int, c_int), usize> = Default::default();

    for _ in 0..4000 {
        // Vertex regions: a tight cluster far from the origin, with a chosen
        // member pulled closest.
        let ang = rng.range(-3.15, 3.15);
        let dist = rng.range(40.0, 300.0);
        let (cx, cy) = (ang.cos() * dist, ang.sin() * dist);
        let near = rng.below(3);
        let mut pts = [c2v::default(); 3];
        for k in 0..3 {
            let pull = if k == near { 0.75f32 } else { 1.0 };
            pts[k] = c2v {
                x: (cx + rng.range(-8.0, 8.0)) * pull,
                y: (cy + rng.range(-8.0, 8.0)) * pull,
            };
        }
        let s = tagged(&mut rng, &pts);
        let out = run_c23(&s);
        *seen.entry((out.count, out.verts[0].iA, out.verts[1].iA)).or_default() += 1;

        // Edge regions: two points straddle the origin ray, third far away.
        let (dx, dy) = (ang.cos(), ang.sin());
        let (px, py) = (-dy, dx);
        let d = rng.range(4.0, 60.0);
        let w = rng.range(4.0, 60.0);
        let far = rng.range(200.0, 900.0);
        let mut pts = [
            c2v { x: dx * d + px * w, y: dy * d + py * w },
            c2v { x: dx * d - px * w, y: dy * d - py * w },
            c2v { x: dx * far, y: dy * far },
        ];
        pts.rotate_left(rng.below(3));
        let s = tagged(&mut rng, &pts);
        let out = run_c23(&s);
        *seen.entry((out.count, out.verts[0].iA, out.verts[1].iA)).or_default() += 1;

        // Interior: three radii ~120 degrees apart.
        let step = 2.0 * std::f32::consts::PI / 3.0;
        let mut pts = [c2v::default(); 3];
        for k in 0..3 {
            let a = ang + step * k as f32 + rng.range(-0.3, 0.3);
            let rr = rng.range(2.0, 80.0);
            pts[k] = c2v { x: a.cos() * rr, y: a.sin() * rr };
        }
        let s = tagged(&mut rng, &pts);
        let out = run_c23(&s);
        *seen.entry((out.count, out.verts[0].iA, out.verts[1].iA)).or_default() += 1;
    }

    let vertex_arms: Vec<_> = seen.keys().filter(|k| k.0 == 1).collect();
    let edge_arms: Vec<_> = seen.keys().filter(|k| k.0 == 2).collect();
    let interior: usize = seen.iter().filter(|(k, _)| k.0 == 3).map(|(_, v)| *v).sum();
    assert!(vertex_arms.len() >= 3, "vertex arms: {vertex_arms:?}");
    assert!(edge_arms.len() >= 3, "edge arms: {edge_arms:?}");
    assert!(interior > 0, "interior arm never taken; seen = {seen:?}");
}

#[test]
fn err_c23_degenerate_triangle() {
    // Row 42: area == 0 (collinear or repeated points) => uABC/vABC/wABC are 0
    // (or +-0), so which arm wins is decided purely by source order.
    let mut rng = Rng::new(0x2042);
    for _ in 0..4000 {
        let a = rng.vec_coord(32.0);
        let b = rng.vec_coord(32.0);
        let pts = match rng.below(6) {
            0 => [a, a, a],
            1 => [a, b, a],
            2 => [a, a, b],
            3 => [a, b, b],
            4 => [
                a,
                b,
                c2v { x: a.x + (b.x - a.x) * 2.0, y: a.y + (b.y - a.y) * 2.0 },
            ],
            _ => [
                a,
                b,
                c2v { x: a.x + (b.x - a.x) * 0.5, y: a.y + (b.y - a.y) * 0.5 },
            ],
        };
        let s = tagged(&mut rng, &pts);
        run_c23(&s);
    }
}

// ===========================================================================
// Rows 44-47 — c2Support
// ===========================================================================

#[test]
fn err_support_nonpositive_count() {
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0x2044);
    // The C dereferences verts[0] unconditionally, so a valid array is needed.
    for &count in &[0i32, -1, -2, -100, c_int::MIN, c_int::MIN + 1] {
        for _ in 0..200 {
            let verts = [rng.vec_wild(), rng.vec_wild(), rng.vec_wild(), rng.vec_wild()];
            let d = rng.vec_wild();
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            assert_eq!(cv, rv, "c2Support(count={count})");
            assert_eq!(cv, 0, "non-positive count must return index 0");
        }
    }
}

#[test]
fn err_support_ties_keep_first() {
    // Row 45: `dot > dmax` is strict, so an exact tie keeps the earlier index.
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0x2045);
    for _ in 0..2000 {
        let v = rng.vec_coord(8.0);
        // All identical vertices => every dot is equal.
        let verts = [v, v, v, v, v, v, v, v];
        for count in 1..=8 {
            let d = rng.vec_coord(8.0);
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            assert_eq!(cv, rv);
            assert_eq!(cv, 0, "an all-tie array must return index 0");
        }
        // A tie between slots 0 and 1, with every later vertex strictly
        // smaller along `d` -> the FIRST of the two tied maxima must win.
        let d = c2v { x: 1.0, y: 0.0 };
        let big = c2v { x: 1.0e6, y: 0.0 };
        let small = c2v { x: -1.0e6, y: rng.coord(8.0) };
        let verts = [big, big, small, small];
        let cv = unsafe { c(verts.as_ptr(), 4, d) };
        let rv = unsafe { r(verts.as_ptr(), 4, d) };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0, "the first of two tied maxima must win");
        // A tie between slots 1 and 2 with slot 0 smaller -> index 1 wins.
        let verts = [small, big, big, small];
        let cv = unsafe { c(verts.as_ptr(), 4, d) };
        let rv = unsafe { r(verts.as_ptr(), 4, d) };
        assert_eq!(cv, rv);
        assert_eq!(cv, 1, "the first of two tied maxima must win");
        // Fully random arrays (ties arise naturally from the quantised coords).
        let verts: [c2v; 8] = std::array::from_fn(|_| {
            c2v { x: ((rng.next_u32() % 5) as f32) - 2.0, y: ((rng.next_u32() % 5) as f32) - 2.0 }
        });
        for count in 1..=8 {
            let d = c2v { x: ((rng.next_u32() % 3) as f32) - 1.0, y: ((rng.next_u32() % 3) as f32) - 1.0 };
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            assert_eq!(cv, rv, "tie-heavy c2Support(count={count}, d={})", vdesc(d));
        }
    }
}

#[test]
fn err_support_zero_direction() {
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0x2046);
    for &d in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        for _ in 0..500 {
            let verts: [c2v; 8] = std::array::from_fn(|_| rng.vec_coord(64.0));
            for count in 1..=8 {
                let cv = unsafe { c(verts.as_ptr(), count, d) };
                let rv = unsafe { r(verts.as_ptr(), count, d) };
                assert_eq!(cv, rv, "c2Support(d={})", vdesc(d));
                assert_eq!(cv, 0, "a zero direction makes every dot 0 => index 0");
            }
        }
    }
}

#[test]
fn err_support_nan() {
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0x2047);
    for _ in 0..3000 {
        let verts: [c2v; 8] = std::array::from_fn(|_| match rng.below(3) {
            0 => rng.vec_wild(),
            _ => rng.vec_coord(64.0),
        });
        let d = match rng.below(3) {
            0 => c2v { x: f32::NAN, y: rng.coord(8.0) },
            1 => rng.vec_wild(),
            _ => rng.vec_coord(64.0),
        };
        for count in 1..=8 {
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            assert_eq!(cv, rv, "c2Support(count={count}, d={}) verts={verts:?}", vdesc(d));
        }
    }
}

// ===========================================================================
// Rows 48-54 — degenerate scalar / vector helpers
// ===========================================================================

#[test]
fn err_norm_zero_vector() {
    let l = libs();
    let (c, r) = l.pair::<FnVec>("c2Norm");
    for &a in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(veq(x, y), "c2Norm({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
        assert!(x.x.is_nan() && x.y.is_nan(), "1/0 * 0 must be NaN, got {}", vdesc(x));
    }
}

#[test]
fn err_div_degenerate_denominators() {
    let l = libs();
    let (c, r) = l.pair::<FnVecScalar>("c2Div");
    let mut rng = Rng::new(0x2049);
    let denoms = [
        0.0f32,
        -0.0,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_beef),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        1.0,
        -1.0,
    ];
    for &b in &denoms {
        for _ in 0..400 {
            let a = if rng.boolean() { rng.vec_wild() } else { rng.vec_coord(64.0) };
            let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
            assert!(
                veq(x, y),
                "c2Div({}, {}) C={} R={}",
                vdesc(a), fdesc(b), vdesc(x), vdesc(y)
            );
        }
    }
}

#[test]
fn err_norm_nonfinite() {
    let l = libs();
    let (c, r) = l.pair::<FnVec>("c2Norm");
    let mut rng = Rng::new(0x204a);
    let picks = [
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: 0.0, y: f32::NAN },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
    ];
    for &a in &picks {
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(veq(x, y), "c2Norm({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
    }
    for _ in 0..4000 {
        let a = rng.vec_wild();
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(veq(x, y), "c2Norm({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
    }
}

#[test]
fn err_len_edge_values() {
    let l = libs();
    let (c, r) = l.pair::<FnVecF>("c2Len");
    let mut rng = Rng::new(0x204b);
    let picks = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::NAN, y: f32::NAN },
        c2v { x: -f32::NAN, y: 1.0 },
        c2v { x: f32::from_bits(0x7f80_0001), y: 0.0 }, // signalling NaN
        c2v { x: f32::from_bits(0xff80_0001), y: 0.0 }, // negative signalling NaN
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: 0.0 },
    ];
    for &a in &picks {
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(feq(x, y), "c2Len({}) C={} R={}", vdesc(a), fdesc(x), fdesc(y));
    }
    for _ in 0..8000 {
        let a = rng.vec_wild();
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(feq(x, y), "c2Len({}) C={} R={}", vdesc(a), fdesc(x), fdesc(y));
    }
}

#[test]
fn err_minmax_nan_asymmetry() {
    // Row 52: C's `?:` returns the SECOND operand when the comparison is false,
    // which for a NaN operand differs from fmaxf/fminf and from Rust's
    // f32::max / f32::min.
    let l = libs();
    let (cmax, rmax) = l.pair::<FnVecVec>("c2Maxv");
    let (cmin, rmin) = l.pair::<FnVecVec>("c2Minv");
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_4321),
        f32::from_bits(0x7f80_0001),
    ];
    let others = [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX];
    for &n in &nans {
        for &o in &others {
            for (a, b) in [
                (c2v { x: n, y: o }, c2v { x: o, y: n }),
                (c2v { x: o, y: n }, c2v { x: n, y: o }),
                (c2v { x: n, y: n }, c2v { x: o, y: o }),
                (c2v { x: o, y: o }, c2v { x: n, y: n }),
            ] {
                let (x, y) = (unsafe { cmax(a, b) }, unsafe { rmax(a, b) });
                assert!(veq(x, y), "c2Maxv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
                let (x, y) = (unsafe { cmin(a, b) }, unsafe { rmin(a, b) });
                assert!(veq(x, y), "c2Minv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
            }
        }
    }
    // The C really does return operand `b` for a NaN comparison.
    let a = c2v { x: f32::NAN, y: f32::NAN };
    let b = c2v { x: 5.0, y: -5.0 };
    let m = unsafe { cmax(a, b) };
    assert!(veq(m, b), "c2Maxv(NaN, b) must be b, got {}", vdesc(m));
}

#[test]
fn err_minmax_signed_zero() {
    // Row 53: neither `>` nor `<` holds for +0 vs -0, so `b` is returned.
    let l = libs();
    let (cmax, rmax) = l.pair::<FnVecVec>("c2Maxv");
    let (cmin, rmin) = l.pair::<FnVecVec>("c2Minv");
    for (a, b) in [
        (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
    ] {
        let (x, y) = (unsafe { cmax(a, b) }, unsafe { rmax(a, b) });
        assert!(veq(x, y), "c2Maxv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
        assert!(veq(x, b), "the second operand must be returned, got {}", vdesc(x));
        let (x, y) = (unsafe { cmin(a, b) }, unsafe { rmin(a, b) });
        assert!(veq(x, y), "c2Minv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
        assert!(veq(x, b), "the second operand must be returned, got {}", vdesc(x));
    }
}

#[test]
fn err_clampv_inverted_box() {
    let l = libs();
    let (c, r) = l.pair::<FnVecVecVec>("c2Clampv");
    let mut rng = Rng::new(0x2054);
    for _ in 0..4000 {
        let lo = rng.vec_coord(64.0);
        let hi = c2v { x: lo.x - rng.range(0.0, 64.0), y: lo.y - rng.range(0.0, 64.0) };
        let a = if rng.boolean() { rng.vec_wild() } else { rng.vec_coord(64.0) };
        let (x, y) = (unsafe { c(a, lo, hi) }, unsafe { r(a, lo, hi) });
        assert!(
            veq(x, y),
            "c2Clampv({}, lo={}, hi={}) C={} R={}",
            vdesc(a), vdesc(lo), vdesc(hi), vdesc(x), vdesc(y)
        );
    }
    // Fully degenerate: lo == hi == NaN.
    let n = c2v { x: f32::NAN, y: f32::NAN };
    let (x, y) = (unsafe { c(n, n, n) }, unsafe { r(n, n, n) });
    assert!(veq(x, y));
}

// ===========================================================================
// Rows 55-62 — degenerate shapes through the public predicates
// ===========================================================================

#[test]
fn err_circle_aabb_degenerate() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoAABB>("c2CircletoAABB");
    let mut rng = Rng::new(0x2055);
    for _ in 0..4000 {
        let circle = c2Circle {
            p: if rng.boolean() { rng.vec_wild() } else { rng.vec_coord(16.0) },
            r: match rng.below(5) {
                0 => 0.0,
                1 => -0.0,
                2 => -4.0,
                3 => rng.wild(),
                _ => rng.range(0.0, 8.0),
            },
        };
        let base = rng.vec_coord(16.0);
        let bb = match rng.below(4) {
            0 => c2AABB { min: base, max: base },                       // point box
            1 => c2AABB { min: c2v { x: base.x + 4.0, y: base.y + 4.0 }, max: base }, // inverted
            2 => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
            _ => c2AABB { min: base, max: c2v { x: base.x + 6.0, y: base.y + 3.0 } },
        };
        let (x, y) = (unsafe { c(circle, bb) }, unsafe { r(circle, bb) });
        assert_eq!(x, y, "c2CircletoAABB({circle:?}, {bb:?})");
    }
    // r == 0 can never collide: `d2 < 0` is false even at the exact centre.
    let z = c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 0.0 };
    let bb = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 2.0, y: 2.0 } };
    assert_eq!(unsafe { c(z, bb) }, 0);
    assert_eq!(unsafe { r(z, bb) }, 0);
}

#[test]
fn err_circle_circle_negative_radii() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCircle>("c2CircletoCircle");
    let mut rng = Rng::new(0x2056);
    for _ in 0..4000 {
        let ra = match rng.below(4) {
            0 => -rng.range(0.0, 8.0),
            1 => 0.0,
            2 => rng.wild(),
            _ => rng.range(0.0, 8.0),
        };
        let rb = match rng.below(4) {
            0 => -rng.range(0.0, 8.0),
            1 => -0.0,
            2 => rng.wild(),
            _ => rng.range(0.0, 8.0),
        };
        let a = c2Circle { p: rng.vec_coord(16.0), r: ra };
        let b = c2Circle { p: rng.vec_coord(16.0), r: rb };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "c2CircletoCircle({a:?}, {b:?})");
    }
    // A negative sum still squares positive => behaves like a positive radius.
    let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -3.0 };
    let b = c2Circle { p: c2v { x: 1.0, y: 0.0 }, r: -3.0 };
    assert_eq!(unsafe { c(a, b) }, 1);
    assert_eq!(unsafe { r(a, b) }, 1);
}

#[test]
fn err_circle_capsule_degenerate() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(0x2057);
    for _ in 0..4000 {
        let p = rng.vec_coord(16.0);
        let capsule = c2Capsule {
            a: p,
            b: p, // a == b: n == (0,0), da == 0, db == 0 => the `bp` branch
            r: match rng.below(4) {
                0 => 0.0,
                1 => -2.0,
                2 => rng.wild(),
                _ => rng.range(0.0, 8.0),
            },
        };
        let circle = c2Circle { p: rng.vec_coord(16.0), r: rng.range(-4.0, 8.0) };
        let (x, y) = (unsafe { c(circle, capsule) }, unsafe { r(circle, capsule) });
        assert_eq!(x, y, "c2CircletoCapsule({circle:?}, {capsule:?})");
    }
}

#[test]
fn err_circle_capsule_nonfinite() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(0x2058);
    for _ in 0..6000 {
        let circle = c2Circle { p: rng.vec_wild(), r: rng.wild() };
        let capsule = c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() };
        let (x, y) = (unsafe { c(circle, capsule) }, unsafe { r(circle, capsule) });
        assert_eq!(x, y, "c2CircletoCapsule({circle:?}, {capsule:?})");
    }
    // inf ends => c2Dot(n, n) can be inf/NaN, exercising the da/0 division.
    for &(ax, bx) in &[
        (f32::INFINITY, f32::INFINITY),
        (f32::INFINITY, f32::NEG_INFINITY),
        (f32::NAN, 0.0),
    ] {
        let capsule = c2Capsule { a: c2v { x: ax, y: 0.0 }, b: c2v { x: bx, y: 0.0 }, r: 1.0 };
        let circle = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
        assert_eq!(unsafe { c(circle, capsule) }, unsafe { r(circle, capsule) });
    }
}

#[test]
fn err_aabb_aabb_degenerate() {
    let l = libs();
    let (c, r) = l.pair::<FnAABBtoAABB>("c2AABBtoAABB");
    let mut rng = Rng::new(0x2059);
    for _ in 0..8000 {
        let mk = |rng: &mut Rng| match rng.below(4) {
            0 => {
                let p = rng.vec_coord(16.0);
                c2AABB { min: p, max: p }
            }
            1 => {
                let p = rng.vec_coord(16.0);
                c2AABB { min: c2v { x: p.x + 4.0, y: p.y + 4.0 }, max: p }
            }
            2 => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
            _ => {
                let p = rng.vec_coord(16.0);
                c2AABB { min: p, max: c2v { x: p.x + 6.0, y: p.y + 6.0 } }
            }
        };
        let a = mk(&mut rng);
        let b = mk(&mut rng);
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "c2AABBtoAABB({a:?}, {b:?})");
    }
    // All-NaN: every `<` is false => `!(0|0|0|0)` => 1.
    let n = c2AABB { min: c2v { x: f32::NAN, y: f32::NAN }, max: c2v { x: f32::NAN, y: f32::NAN } };
    assert_eq!(unsafe { c(n, n) }, 1);
    assert_eq!(unsafe { r(n, n) }, 1);
}

#[test]
fn err_gjk_wrappers_nan_distance() {
    // Row 60: `if (c2GJK(...)) return 0;` — a NaN distance is non-zero, so a
    // NaN-poisoned shape reports "no collision".
    let l = libs();
    let (cac, rac) = l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let (ccc, rcc) = l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x2060);
    for _ in 0..4000 {
        let bb = c2AABB { min: rng.vec_wild(), max: rng.vec_wild() };
        let cap = c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() };
        assert_eq!(unsafe { cac(bb, cap) }, unsafe { rac(bb, cap) }, "c2AABBtoCapsule({bb:?}, {cap:?})");
        let cap2 = c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() };
        assert_eq!(
            unsafe { ccc(cap, cap2) },
            unsafe { rcc(cap, cap2) },
            "c2CapsuletoCapsule({cap:?}, {cap2:?})"
        );
    }
    // Explicit NaN capsule: both must answer 0.
    let nan_cap = c2Capsule {
        a: c2v { x: f32::NAN, y: 0.0 },
        b: c2v { x: 1.0, y: 1.0 },
        r: 1.0,
    };
    let good = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 };
    assert_eq!(unsafe { ccc(nan_cap, good) }, unsafe { rcc(nan_cap, good) });
    let bb = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    assert_eq!(unsafe { cac(bb, nan_cap) }, unsafe { rac(bb, nan_cap) });
}

#[test]
fn err_bbverts_inverted() {
    let l = libs();
    let (c, r) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0x2061);
    for _ in 0..4000 {
        let base = rng.vec_coord(16.0);
        let bb = match rng.below(4) {
            0 => c2AABB { min: base, max: base },
            1 => c2AABB { min: c2v { x: base.x + 8.0, y: base.y + 8.0 }, max: base },
            2 => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
            _ => c2AABB { min: base, max: c2v { x: base.x + 5.0, y: base.y + 7.0 } },
        };
        let sent = c2v { x: -3.5e8, y: 7.25e8 };
        let mut co = [sent; 5];
        let mut ro = [sent; 5];
        let mut bc = bb;
        let mut br = bb;
        unsafe { c(co.as_mut_ptr(), &mut bc) };
        unsafe { r(ro.as_mut_ptr(), &mut br) };
        for k in 0..5 {
            assert!(veq(co[k], ro[k]), "c2BBVerts({bb:?}) out[{k}]: C={} R={}", vdesc(co[k]), vdesc(ro[k]));
        }
        assert!(veq(co[4], sent), "c2BBVerts must write exactly 4 vertices");
    }
}

#[test]
fn err_aabb_entry_degenerate() {
    let l = libs();
    let (c, r) = l.pair::<FnAabb>("aabb");
    let mut rng = Rng::new(0x2062);
    let picks: [(f32, f32, f32, f32); 10] = [
        (f32::NAN, f32::NAN, f32::NAN, f32::NAN),
        (0.0, 0.0, -0.0, -0.0),
        (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY),
        (10.0, 10.0, -10.0, -10.0), // inverted
        (-70.0, 0.0, -70.0, 0.0),   // exactly the circle's centre
        (-40.0, -40.0, -15.0, -15.0), // exactly the hard-coded AABB
        (-40.0, 40.0, -20.0, 100.0), // exactly the capsule's spine
        (f32::MAX, f32::MAX, f32::MAX, f32::MAX),
        (f32::MIN, f32::MIN, f32::MAX, f32::MAX),
    ];
    for &(a, b, cc, d) in &picks {
        let cv = unsafe { c(a, b, cc, d) };
        let rv = unsafe { r(a, b, cc, d) };
        assert_eq!(cv, rv, "aabb({}, {}, {}, {})", fdesc(a), fdesc(b), fdesc(cc), fdesc(d));
    }
    for _ in 0..8000 {
        let (a, b, cc, d) = (rng.wild(), rng.wild(), rng.wild(), rng.wild());
        let cv = unsafe { c(a, b, cc, d) };
        let rv = unsafe { r(a, b, cc, d) };
        assert_eq!(cv, rv, "aabb({}, {}, {}, {})", fdesc(a), fdesc(b), fdesc(cc), fdesc(d));
    }
}

// ===========================================================================
// Row 63 — every scalar/vector op with non-finite operands
// ===========================================================================

#[test]
fn err_scalar_ops_nonfinite() {
    let l = libs();
    let (cv2, rv2) = l.pair::<FnV2>("c2V");
    let (cmul, rmul) = l.pair::<FnVecScalar>("c2Mulvs");
    let (cadd, radd) = l.pair::<FnVecVec>("c2Add");
    let (csub, rsub) = l.pair::<FnVecVec>("c2Sub");
    let (cdot, rdot) = l.pair::<FnVecVecF>("c2Dot");
    let (cdet, rdet) = l.pair::<FnVecVecF>("c2Det2");
    let (cneg, rneg) = l.pair::<FnVec>("c2Neg");
    let (cskew, rskew) = l.pair::<FnVec>("c2Skew");
    let (cccw, rccw) = l.pair::<FnVec>("c2CCW90");
    let (cmr, rmr) = l.pair::<FnMulrv>("c2Mulrv");
    let (cmrt, rmrt) = l.pair::<FnMulrv>("c2MulrvT");
    let (cmx, rmx) = l.pair::<FnMulxv>("c2Mulxv");

    // Explicit special-value cross product for the binary ops.
    let specials = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_4321),
        f32::from_bits(0x7f80_0001), // sNaN
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
    ];
    for &p in &specials {
        for &q in &specials {
            let a = c2v { x: p, y: q };
            let b = c2v { x: q, y: p };
            assert!(veq(unsafe { cv2(p, q) }, unsafe { rv2(p, q) }), "c2V({}, {})", fdesc(p), fdesc(q));
            assert!(veq(unsafe { cmul(a, q) }, unsafe { rmul(a, q) }), "c2Mulvs({}, {})", vdesc(a), fdesc(q));
            assert!(veq(unsafe { cadd(a, b) }, unsafe { radd(a, b) }), "c2Add({}, {})", vdesc(a), vdesc(b));
            assert!(veq(unsafe { csub(a, b) }, unsafe { rsub(a, b) }), "c2Sub({}, {})", vdesc(a), vdesc(b));
            assert!(feq(unsafe { cdot(a, b) }, unsafe { rdot(a, b) }), "c2Dot({}, {})", vdesc(a), vdesc(b));
            assert!(feq(unsafe { cdet(a, b) }, unsafe { rdet(a, b) }), "c2Det2({}, {})", vdesc(a), vdesc(b));
            assert!(veq(unsafe { cneg(a) }, unsafe { rneg(a) }), "c2Neg({})", vdesc(a));
            assert!(veq(unsafe { cskew(a) }, unsafe { rskew(a) }), "c2Skew({})", vdesc(a));
            assert!(veq(unsafe { cccw(a) }, unsafe { rccw(a) }), "c2CCW90({})", vdesc(a));
            let rot = c2r { c: p, s: q };
            assert!(veq(unsafe { cmr(rot, b) }, unsafe { rmr(rot, b) }), "c2Mulrv(({},{}), {})", fdesc(p), fdesc(q), vdesc(b));
            assert!(veq(unsafe { cmrt(rot, b) }, unsafe { rmrt(rot, b) }), "c2MulrvT(({},{}), {})", fdesc(p), fdesc(q), vdesc(b));
            let tx = c2x { p: a, r: rot };
            assert!(veq(unsafe { cmx(tx, b) }, unsafe { rmx(tx, b) }), "c2Mulxv({tx:?}, {})", vdesc(b));
        }
    }

    // Plus randomized wild inputs.
    let mut rng = Rng::new(0x2063);
    for _ in 0..20000 {
        let a = rng.vec_wild();
        let b = rng.vec_wild();
        let s = rng.wild();
        assert!(veq(unsafe { cmul(a, s) }, unsafe { rmul(a, s) }));
        assert!(veq(unsafe { cadd(a, b) }, unsafe { radd(a, b) }));
        assert!(veq(unsafe { csub(a, b) }, unsafe { rsub(a, b) }));
        assert!(feq(unsafe { cdot(a, b) }, unsafe { rdot(a, b) }));
        assert!(feq(unsafe { cdet(a, b) }, unsafe { rdet(a, b) }));
        assert!(veq(unsafe { cneg(a) }, unsafe { rneg(a) }));
        assert!(veq(unsafe { cskew(a) }, unsafe { rskew(a) }));
        assert!(veq(unsafe { cccw(a) }, unsafe { rccw(a) }));
        let rot = c2r { c: rng.wild(), s: rng.wild() };
        assert!(veq(unsafe { cmr(rot, b) }, unsafe { rmr(rot, b) }));
        assert!(veq(unsafe { cmrt(rot, b) }, unsafe { rmrt(rot, b) }));
        let tx = c2x { p: a, r: rot };
        assert!(veq(unsafe { cmx(tx, b) }, unsafe { rmx(tx, b) }));
    }
}

// ===========================================================================
// Row 64 — cache->count < 0 (well-defined in both: every loop is skipped)
// ===========================================================================

#[test]
fn err_gjk_cache_negative_count() {
    // `cache_was_good = !!cache->count` is TRUE for a negative count, but both
    // `for (i = 0; i < cache->count; ...)` loops then execute zero times, so the
    // simplex keeps `count < 0` and every `switch` takes its `default:` arm:
    //   c2L -> (0,0), c2D -> (0,0) -> epsilon break on iteration 0,
    //   c2Witness -> (0,0)/(0,0), c2GJKSimplexMetric -> 0.
    // Nothing is read out of bounds, so this is fully defined and must match.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2064);
    let counts: [c_int; 6] = [-1, -2, -3, -100, c_int::MIN, c_int::MIN + 1];
    let call: GjkCall = (None, None, true, true, true, true);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for &count in &counts {
                for _ in 0..20 {
                    let scale = SCALES[rng.below(SCALES.len())];
                    let a = gen_shape(&mut rng, tyA, scale);
                    let b = gen_shape(&mut rng, tyB, scale);
                    let cache = c2GJKCache {
                        metric: rng.range(-1.0e3, 1.0e3),
                        count,
                        iA: [7, 6, 5],
                        iB: [5, 6, 7],
                        div: rng.range(-4.0, 4.0),
                    };
                    for &ur in &[0, 1] {
                        let mut call = call;
                        call.2 = ur != 0;
                        let cv = gjk_raw(cf, &a, &b, &call, cache);
                        let rv = gjk_raw(rf, &a, &b, &call, cache);
                        assert_raw_eq(&format!("cache->count = {count}"), &cv, &rv);
                        assert_eq!(cv.4.count, count, "count is copied through verbatim");
                        assert_eq!(cv.4.iA, cache.iA, "no index is written back");
                        assert_eq!(cv.4.iB, cache.iB, "no index is written back");
                        assert!(feq(cv.4.metric, 0.0), "the default metric arm gives 0");
                        assert!(feq(cv.4.div, cache.div), "div is copied through verbatim");
                        assert_eq!(cv.3, 0, "the epsilon break fires on iteration 0");
                        assert!(feq(cv.0, 0.0));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 65 — c2GJK with out-of-range C2_TYPE values (documented UB row U1)
// ===========================================================================

#[test]
#[ignore = "documented UB: c2MakeProxy writes nothing so `c2Proxy pA` stays uninitialised in the C"]
fn err_gjk_bad_type_is_ub() {
    // Kept as an executable record of ERRORS.md row U1. `c2MakeProxy` has no
    // `default:` label, so for an out-of-range type the C leaves the whole
    // `c2Proxy` uninitialised and then drives `c2Support` with a garbage
    // `pA.count`. The observable result is whatever happens to be on the C
    // stack; it is neither reproducible nor matchable, and it can fault.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let circle = some_circle();
    let a = Shape::Circle(circle);
    let mut oa = c2v::default();
    let mut ob = c2v::default();
    for &bad in &BAD_TYPES {
        let dc = unsafe {
            cf(a.as_ptr(), bad, std::ptr::null(), a.as_ptr(), C2_TYPE_CIRCLE, std::ptr::null(),
               &mut oa, &mut ob, 1, std::ptr::null_mut(), std::ptr::null_mut())
        };
        let dr = unsafe {
            rf(a.as_ptr(), bad, std::ptr::null(), a.as_ptr(), C2_TYPE_CIRCLE, std::ptr::null(),
               &mut oa, &mut ob, 1, std::ptr::null_mut(), std::ptr::null_mut())
        };
        eprintln!("typeA={bad}: C={} RUST={}", fdesc(dc), fdesc(dr));
    }
}

// ===========================================================================
// Row 63 (extension) — distinct-NaN-payload matrix for the COMPOSITE functions
// ===========================================================================
//
// x86 SSE arithmetic resolves a NaN operand destination-first, and gcc -O0 does
// not always put the C expression's left-hand side in the destination register.
// A wrong operand order is only observable when TWO DIFFERENT NaN payloads meet
// in the same instruction, so these tests draw every field from a small pool of
// distinct payloads to maximise the chance of two of them colliding.

/// Distinct NaN bit patterns plus the finite values that keep control flow alive.
const PAYLOADS: [u32; 12] = [
    0x7fc0_0000, // +qNaN, zero payload (the SSE "real indefinite" is 0xffc00000)
    0xffc0_0000, // -qNaN, zero payload
    0x7fc0_1234,
    0xffc0_4321,
    0x7fd0_0001,
    0xffdb_eef0,
    0x7f80_0001, // +sNaN
    0xff80_0001, // -sNaN
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x0000_0000, // +0
    0x3f80_0000, // 1.0
];

fn payload(rng: &mut Rng) -> f32 {
    f32::from_bits(PAYLOADS[rng.below(PAYLOADS.len())])
}

fn payload_vec(rng: &mut Rng) -> c2v {
    c2v { x: payload(rng), y: payload(rng) }
}

#[test]
fn err_nan_payload_scalar_matrix() {
    // Exhaustive 12x12 over the payload pool for every float/vector-returning
    // primitive, in both argument orders.
    let l = libs();
    let (cdot, rdot) = l.pair::<FnVecVecF>("c2Dot");
    let (cdet, rdet) = l.pair::<FnVecVecF>("c2Det2");
    let (clen, rlen) = l.pair::<FnVecF>("c2Len");
    let (cdiv, rdiv) = l.pair::<FnVecScalar>("c2Div");
    let (cnorm, rnorm) = l.pair::<FnVec>("c2Norm");
    let (cadd, radd) = l.pair::<FnVecVec>("c2Add");
    let (csub, rsub) = l.pair::<FnVecVec>("c2Sub");
    let (cmul, rmul) = l.pair::<FnVecScalar>("c2Mulvs");
    let (cmr, rmr) = l.pair::<FnMulrv>("c2Mulrv");
    let (cmrt, rmrt) = l.pair::<FnMulrv>("c2MulrvT");
    let (cmx, rmx) = l.pair::<FnMulxv>("c2Mulxv");
    let (cmax, rmax) = l.pair::<FnVecVec>("c2Maxv");
    let (cmin, rmin) = l.pair::<FnVecVec>("c2Minv");
    let (cclamp, rclamp) = l.pair::<FnVecVecVec>("c2Clampv");

    for &p in &PAYLOADS {
        for &q in &PAYLOADS {
            for &s in &PAYLOADS {
                let (fp, fq, fs) = (f32::from_bits(p), f32::from_bits(q), f32::from_bits(s));
                let a = c2v { x: fp, y: fq };
                let b = c2v { x: fq, y: fs };
                let c = c2v { x: fs, y: fp };
                let rot = c2r { c: fp, s: fq };
                let tx = c2x { p: c, r: rot };
                macro_rules! chk_f {
                    ($cf:expr, $rf:expr, $($arg:expr),*) => {{
                        let x = unsafe { $cf($($arg),*) };
                        let y = unsafe { $rf($($arg),*) };
                        assert!(feq(x, y), "{}: C={} R={} (p={p:#010x} q={q:#010x} s={s:#010x})",
                                stringify!($cf), fdesc(x), fdesc(y));
                    }};
                }
                macro_rules! chk_v {
                    ($cf:expr, $rf:expr, $($arg:expr),*) => {{
                        let x = unsafe { $cf($($arg),*) };
                        let y = unsafe { $rf($($arg),*) };
                        assert!(veq(x, y), "{}: C={} R={} (p={p:#010x} q={q:#010x} s={s:#010x})",
                                stringify!($cf), vdesc(x), vdesc(y));
                    }};
                }
                chk_f!(cdot, rdot, a, b);
                chk_f!(cdot, rdot, b, a);
                chk_f!(cdet, rdet, a, b);
                chk_f!(cdet, rdet, b, a);
                chk_f!(clen, rlen, a);
                chk_v!(cdiv, rdiv, a, fs);
                chk_v!(cnorm, rnorm, a);
                chk_v!(cadd, radd, a, b);
                chk_v!(cadd, radd, b, a);
                chk_v!(csub, rsub, a, b);
                chk_v!(csub, rsub, b, a);
                chk_v!(cmul, rmul, a, fs);
                chk_v!(cmr, rmr, rot, b);
                chk_v!(cmrt, rmrt, rot, b);
                chk_v!(cmx, rmx, tx, b);
                chk_v!(cmax, rmax, a, b);
                chk_v!(cmin, rmin, a, b);
                chk_v!(cclamp, rclamp, a, b, c);
            }
        }
    }
}

#[test]
fn err_nan_payload_simplex_matrix() {
    // c22 / c23 / c2L / c2Witness / c2D / c2GJKSimplexMetric with every field
    // drawn from the distinct-payload pool.
    let l = libs();
    let (c22c, c22r) = l.pair::<FnSimplexVoid>("c22");
    let (c23c, c23r) = l.pair::<FnSimplexVoid>("c23");
    let (clc, clr) = l.pair::<FnSimplexV>("c2L");
    let (cdc, cdr) = l.pair::<FnSimplexV>("c2D");
    let (cwc, cwr) = l.pair::<FnWitness>("c2Witness");
    let (cmc, cmr) = l.pair::<FnSimplexF>("c2GJKSimplexMetric");

    let mut rng = Rng::new(0x2070);
    for _ in 0..40000 {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i].sA = payload_vec(&mut rng);
            s.verts[i].sB = payload_vec(&mut rng);
            s.verts[i].p = payload_vec(&mut rng);
            s.verts[i].u = payload(&mut rng);
            s.verts[i].iA = (rng.next_u32() % 8) as c_int;
            s.verts[i].iB = (rng.next_u32() % 8) as c_int;
        }
        s.div = payload(&mut rng);
        s.count = 1 + (rng.next_u32() % 3) as c_int;

        for (name, cf, rf) in [("c22", c22c, c22r), ("c23", c23c, c23r)] {
            let mut cs = s;
            let mut rs = s;
            unsafe { cf(&mut cs) };
            unsafe { rf(&mut rs) };
            assert!(
                simplex_eq(&cs, &rs),
                "{name} NaN-payload divergence\nINPUT:\n{}\nC:\n{}\nRUST:\n{}",
                simplex_desc(&s), simplex_desc(&cs), simplex_desc(&rs)
            );
        }
        for (name, cf, rf) in [("c2L", clc, clr), ("c2D", cdc, cdr)] {
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { cf(&mut cs) }, unsafe { rf(&mut rs) });
            assert!(
                veq(x, y),
                "{name} NaN-payload divergence: C={} R={}\n{}",
                vdesc(x), vdesc(y), simplex_desc(&s)
            );
        }
        {
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { cmc(&mut cs) }, unsafe { cmr(&mut rs) });
            assert!(
                feq(x, y),
                "c2GJKSimplexMetric NaN-payload divergence: C={} R={}\n{}",
                fdesc(x), fdesc(y), simplex_desc(&s)
            );
        }
        {
            let mut cs = s;
            let mut rs = s;
            let (mut ca, mut cb) = (c2v::default(), c2v::default());
            let (mut ra, mut rb) = (c2v::default(), c2v::default());
            unsafe { cwc(&mut cs, &mut ca, &mut cb) };
            unsafe { cwr(&mut rs, &mut ra, &mut rb) };
            assert!(
                veq(ca, ra) && veq(cb, rb),
                "c2Witness NaN-payload divergence: C=({}, {}) R=({}, {})\n{}",
                vdesc(ca), vdesc(cb), vdesc(ra), vdesc(rb), simplex_desc(&s)
            );
        }
    }
}

#[test]
fn err_nan_payload_gjk_matrix() {
    // Whole-pipeline check: shapes and transforms built only from distinct NaN
    // payloads, so any operand-order slip anywhere inside c2GJK shows up in
    // `dist`, `outA`, `outB` or the written-back cache.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2071);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for _ in 0..1500 {
                let mk = |rng: &mut Rng, ty: c_int| match ty {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: payload_vec(rng), r: payload(rng) }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: payload_vec(rng), max: payload_vec(rng) }),
                    _ => Shape::Capsule(c2Capsule {
                        a: payload_vec(rng),
                        b: payload_vec(rng),
                        r: payload(rng),
                    }),
                };
                let a = mk(&mut rng, tyA);
                let b = mk(&mut rng, tyB);
                let ax = if rng.boolean() {
                    Some(c2x { p: payload_vec(&mut rng), r: c2r { c: payload(&mut rng), s: payload(&mut rng) } })
                } else {
                    None
                };
                let bx = if rng.boolean() {
                    Some(c2x { p: payload_vec(&mut rng), r: c2r { c: payload(&mut rng), s: payload(&mut rng) } })
                } else {
                    None
                };
                for &ur in &[false, true] {
                    let call: GjkCall = (ax, bx, true, true, true, true);
                    let mut call = call;
                    call.2 = ur;
                    let cache = c2GJKCache {
                        metric: payload(&mut rng),
                        count: 0,
                        iA: [0; 3],
                        iB: [0; 3],
                        div: payload(&mut rng),
                    };
                    let cv = gjk_raw(cf, &a, &b, &call, cache);
                    let rv = gjk_raw(rf, &a, &b, &call, cache);
                    assert_raw_eq("c2GJK NaN-payload matrix", &cv, &rv);
                }
            }
        }
    }
}

// ===========================================================================
// Row 63d — raw 32-bit-pattern fuzz of the float-returning primitives
// ===========================================================================
//
// The C calls glibc `sqrtf` (an actual PLT call, see `nm -D -u`) while the Rust
// lowers `f32::sqrt` to the `sqrtss` instruction. They agree for every input
// `c2Dot(a, a)` can produce (always `>= +0.0`, `+inf` or a quiet NaN — never
// negative, so glibc's `errno`-setting negative-argument path is unreachable),
// but that is worth checking against real bit patterns rather than trusting the
// argument. This also fuzzes the whole float-returning surface with completely
// unconstrained inputs, including every NaN/subnormal/infinity encoding.

#[test]
fn err_bitpattern_fuzz() {
    let l = libs();
    let (clen, rlen) = l.pair::<FnVecF>("c2Len");
    let (cdot, rdot) = l.pair::<FnVecVecF>("c2Dot");
    let (cdet, rdet) = l.pair::<FnVecVecF>("c2Det2");
    let (cdiv, rdiv) = l.pair::<FnVecScalar>("c2Div");
    let (cnorm, rnorm) = l.pair::<FnVec>("c2Norm");
    let (cmr, rmr) = l.pair::<FnMulrv>("c2Mulrv");
    let (cmrt, rmrt) = l.pair::<FnMulrv>("c2MulrvT");

    let mut rng = Rng::new(0x2072);
    for i in 0..200_000u32 {
        let bits = |rng: &mut Rng| f32::from_bits(rng.next_u32());
        let a = c2v { x: bits(&mut rng), y: bits(&mut rng) };
        let b = c2v { x: bits(&mut rng), y: bits(&mut rng) };
        let s = bits(&mut rng);

        let (x, y) = (unsafe { clen(a) }, unsafe { rlen(a) });
        assert!(feq(x, y), "iter {i}: c2Len({}) C={} R={}", vdesc(a), fdesc(x), fdesc(y));
        let (x, y) = (unsafe { cdot(a, b) }, unsafe { rdot(a, b) });
        assert!(feq(x, y), "iter {i}: c2Dot({}, {}) C={} R={}", vdesc(a), vdesc(b), fdesc(x), fdesc(y));
        let (x, y) = (unsafe { cdet(a, b) }, unsafe { rdet(a, b) });
        assert!(feq(x, y), "iter {i}: c2Det2({}, {}) C={} R={}", vdesc(a), vdesc(b), fdesc(x), fdesc(y));
        let (x, y) = (unsafe { cdiv(a, s) }, unsafe { rdiv(a, s) });
        assert!(veq(x, y), "iter {i}: c2Div({}, {}) C={} R={}", vdesc(a), fdesc(s), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { cnorm(a) }, unsafe { rnorm(a) });
        assert!(veq(x, y), "iter {i}: c2Norm({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
        let rot = c2r { c: b.x, s: b.y };
        let (x, y) = (unsafe { cmr(rot, a) }, unsafe { rmr(rot, a) });
        assert!(veq(x, y), "iter {i}: c2Mulrv C={} R={}", vdesc(x), vdesc(y));
        let (x, y) = (unsafe { cmrt(rot, a) }, unsafe { rmrt(rot, a) });
        assert!(veq(x, y), "iter {i}: c2MulrvT C={} R={}", vdesc(x), vdesc(y));
    }
}

// ===========================================================================
// Rows 66-67 — legal-C output/input ALIASING through the two writer functions
// ===========================================================================

#[test]
fn err_bbverts_output_aliases_input() {
    // Row 66. The C reloads `bb->min` / `bb->max` *after* each `out[...]` store:
    //   out[0] = bb->min;
    //   out[1] = c2V(bb->max.x, bb->min.y);   <- may overwrite bb->min
    //   out[2] = bb->max;                     <- may already be modified
    //   out[3] = c2V(bb->min.x, bb->max.y);   <- reads the modified bb->min.x
    // so an `out` buffer that overlaps `*bb` gives a cascading result. This is
    // well-defined C (and reachable: `c2MakeProxy` passes `p->verts` as `out`).
    let l = libs();
    let (c, r) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0x2066);
    // `bb` starts `off` c2v-slots into the same buffer that `out` writes.
    for off in 0..4usize {
        for _ in 0..2000 {
            let seed: [c2v; 8] = std::array::from_fn(|_| {
                if rng.below(4) == 0 { rng.vec_wild() } else { rng.vec_coord(16.0) }
            });
            let mut cbuf = seed;
            let mut rbuf = seed;
            unsafe {
                c(cbuf.as_mut_ptr(), cbuf.as_mut_ptr().add(off) as *mut c2AABB);
                r(rbuf.as_mut_ptr(), rbuf.as_mut_ptr().add(off) as *mut c2AABB);
            }
            for k in 0..8 {
                assert!(
                    veq(cbuf[k], rbuf[k]),
                    "c2BBVerts aliasing off={off} slot {k}: C={} R={}\n  seed={seed:?}\n  C  ={cbuf:?}\n  RUST={rbuf:?}",
                    vdesc(cbuf[k]), vdesc(rbuf[k])
                );
            }
        }
    }
}

#[test]
fn err_witness_output_aliases_simplex() {
    // Row 67. `*a` is stored before the `*b` expression is evaluated, so an `a`
    // that points into `*s` changes what `*b` computes. Point `a` and `b` at
    // every `c2v`-aligned slot of the simplex in turn.
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0x2067);
    // Byte offsets of every c2v field inside c2Simplex: vert i is 36 bytes,
    // fields sA=0, sB=8, p=16.
    let mut slots: Vec<usize> = Vec::new();
    for i in 0..4 {
        for f in [0usize, 8, 16] {
            slots.push(i * 36 + f);
        }
    }
    for count in [0i32, 1, 2, 3, 4] {
        for &oa in &slots {
            for &ob in &slots {
                for _ in 0..3 {
                    let wild = rng.below(3) == 0;
                    let mut s = gen_simplex(&mut rng, count, 16.0, wild);
                    if rng.below(3) == 0 {
                        s.div = 0.0;
                    }
                    let mut cs = s;
                    let mut rs = s;
                    unsafe {
                        let cp = &mut cs as *mut c2Simplex as *mut u8;
                        let rp = &mut rs as *mut c2Simplex as *mut u8;
                        c(&mut cs, cp.add(oa) as *mut c2v, cp.add(ob) as *mut c2v);
                        r(&mut rs, rp.add(oa) as *mut c2v, rp.add(ob) as *mut c2v);
                    }
                    assert!(
                        simplex_eq(&cs, &rs),
                        "c2Witness aliasing count={count} a@{oa} b@{ob}\nINPUT:\n{}\nC:\n{}\nRUST:\n{}",
                        simplex_desc(&s), simplex_desc(&cs), simplex_desc(&rs)
                    );
                }
            }
        }
    }
}
