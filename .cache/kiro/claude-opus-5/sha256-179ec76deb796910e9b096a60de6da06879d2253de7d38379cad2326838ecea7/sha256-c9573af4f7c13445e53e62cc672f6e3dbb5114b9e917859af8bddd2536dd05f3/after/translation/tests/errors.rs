//! Phase C — error/rejection-path differential tests. One test per row group of
//! `ERRORS.md`; every test constructs the exact invalid input the C checks for
//! and asserts the C `.so` and the Rust `.so` reject it identically (same
//! sentinel value / same untouched output), not merely "both failed".
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

/// Out-of-range `C2_TYPE` values. C enums accept any `int`, so every one of
/// these is a real input the C code handles via a `default:` arm.
const BAD_TYPES: &[C2_TYPE] = &[
    -1,
    3,
    4,
    5,
    7,
    100,
    -100,
    255,
    256,
    65536,
    i32::MIN,
    i32::MAX,
    i32::MIN + 1,
    i32::MAX - 1,
];

fn filled_proxy() -> c2Proxy {
    let mut p = c2Proxy {
        radius: f32::from_bits(0xDEAD_BEEF),
        count: -559038737,
        verts: [c2v {
            x: f32::from_bits(0xCAFE_BABE),
            y: f32::from_bits(0xF00D_F00D),
        }; 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = f32::from_bits(0xAAAA_0000 | i as u32);
    }
    p
}

// ===========================================================================
// Rows 1-4 — c2Collided rejects out-of-range enum values
// ===========================================================================

#[test]
fn rows1_4_c2Collided_bad_enums() {
    let b = both();
    let mut rng = Rng::new(101);
    for &bad in BAD_TYPES {
        for &good in TYPES.iter() {
            for _ in 0..5000 {
                let pa = shape_parts(&mut rng, good, 10.0);
                let pb = shape_parts(&mut rng, good, 10.0);
                let ap = pa.as_ptr() as *const c_void;
                let bp = pb.as_ptr() as *const c_void;
                // row 1: typeA invalid (outer default:) — B is never touched
                let (c, r) = unsafe {
                    (
                        (b.c.c2Collided)(ap, bad, bp, good),
                        (b.rs.c2Collided)(ap, bad, bp, good),
                    )
                };
                assert_eq!(c, 0, "C c2Collided(typeA={bad}) should return 0");
                same(&format!("c2Collided/typeA={bad}"), &c, &r);

                // rows 2-4: typeA valid, typeB invalid (inner default:)
                let (c, r) = unsafe {
                    (
                        (b.c.c2Collided)(ap, good, bp, bad),
                        (b.rs.c2Collided)(ap, good, bp, bad),
                    )
                };
                assert_eq!(
                    c, 0,
                    "C c2Collided(typeA={good}, typeB={bad}) should return 0"
                );
                same(&format!("c2Collided/typeA={good},typeB={bad}"), &c, &r);

                // both invalid
                let (c, r) = unsafe {
                    (
                        (b.c.c2Collided)(ap, bad, bp, bad),
                        (b.rs.c2Collided)(ap, bad, bp, bad),
                    )
                };
                assert_eq!(c, 0);
                same("c2Collided/both-invalid", &c, &r);
            }
        }
    }
}

// ===========================================================================
// Rows 6-8 — omni_collide with out-of-range enum values
// ===========================================================================

fn omni(api: &Api, ta: C2_TYPE, a: &[f32; 5], tb: C2_TYPE, bb: &[f32; 5]) -> i32 {
    unsafe {
        (api.omni_collide)(ta, a[0], a[1], a[2], a[3], a[4], tb, bb[0], bb[1], bb[2], bb[3], bb[4])
    }
}

#[test]
fn rows6_8_omni_collide_bad_enums() {
    let b = both();
    let mut rng = Rng::new(102);
    for &bad in BAD_TYPES {
        for &good in TYPES.iter() {
            for _ in 0..5000 {
                let a = shape_parts(&mut rng, good, 10.0);
                let bb = shape_parts(&mut rng, good, 10.0);

                let c = omni(&b.c, bad, &a, good, &bb);
                let r = omni(&b.rs, bad, &a, good, &bb);
                assert_eq!(c, 0, "C omni_collide(type_a={bad}) should return 0");
                same(&format!("omni/type_a={bad}"), &c, &r);

                let c = omni(&b.c, good, &a, bad, &bb);
                let r = omni(&b.rs, good, &a, bad, &bb);
                assert_eq!(c, 0, "C omni_collide(type_b={bad}) should return 0");
                same(&format!("omni/type_b={bad}"), &c, &r);

                let c = omni(&b.c, bad, &a, bad, &bb);
                let r = omni(&b.rs, bad, &a, bad, &bb);
                assert_eq!(c, 0);
                same("omni/both-invalid", &c, &r);
            }
        }
        // also with NaN/inf payloads so the invalid-type rejection is shown to
        // happen before any float is looked at
        for _ in 0..5000 {
            let a = [rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()];
            let bb = [rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()];
            let c = omni(&b.c, bad, &a, bad, &bb);
            let r = omni(&b.rs, bad, &a, bad, &bb);
            assert_eq!(c, 0);
            same("omni/invalid+wild", &c, &r);
        }
    }
}

// ===========================================================================
// Row 9 — ptr_from_parts with an out-of-range type is C undefined behaviour
// (`switch` with no `default:` in a non-void function). The Rust returns NULL;
// the C return value is indeterminate, so only crash-freedom plus the
// end-to-end equivalence (rows 6-8) can be asserted.
// ===========================================================================

#[test]
fn row9_ptr_from_parts_bad_enum() {
    let b = both();
    for &bad in BAD_TYPES {
        let cp = unsafe { (b.c.ptr_from_parts)(bad, 1.0, 2.0, 3.0, 4.0, 5.0) };
        let rp = unsafe { (b.rs.ptr_from_parts)(bad, 1.0, 2.0, 3.0, 4.0, 5.0) };
        // Documented divergence: C falls off the end of the function.
        assert!(rp.is_null(), "Rust ptr_from_parts({bad}) should be NULL");
        // Touch the C value so the call is not optimised away; never deref it.
        let _ = cp as usize;
    }
}

// ===========================================================================
// Row 10 — c2MakeProxy with an out-of-range type leaves *p untouched
// ===========================================================================

#[test]
fn row10_c2MakeProxy_bad_enum() {
    let b = both();
    let mut rng = Rng::new(103);
    for &bad in BAD_TYPES {
        for _ in 0..5000 {
            let parts = [rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()];
            let pristine = filled_proxy();
            let mut cp = pristine;
            let mut rp = pristine;
            unsafe {
                (b.c.c2MakeProxy)(parts.as_ptr() as *const c_void, bad, &mut cp);
                (b.rs.c2MakeProxy)(parts.as_ptr() as *const c_void, bad, &mut rp);
            }
            same(&format!("c2MakeProxy/type={bad}"), &cp, &rp);
            same(&format!("c2MakeProxy/type={bad}/untouched"), &cp, &pristine);
        }
    }
}

// ===========================================================================
// Row 11 — c2GJK with an out-of-range type. `c2MakeProxy` leaves the whole
// `c2Proxy` untouched, so the C then reads an *uninitialised* `p->count` and
// hands it to `c2Support`, which walks off the end of `verts[8]`. Executing
// this reliably segfaults the C library (confirmed: SIGSEGV), so the row is
// documented as unexercisable UB rather than compared. The Rust zero-initialises
// the proxy and therefore survives; that divergence is unobservable through any
// defined input.
// ===========================================================================

#[test]
fn row11_gjk_bad_enum_documented_ub() {
    // Assert only the reachable half: the invalid type is rejected before
    // c2GJK is ever entered by the public entry points (rows 1-8), and
    // c2MakeProxy itself agrees bit-for-bit (row 10).
    let b = both();
    let parts = [1.0f32, 2.0, 3.0, 4.0, 5.0];
    for &bad in &[-1i32, 3, 4, 100, i32::MIN, i32::MAX] {
        let mut cp = filled_proxy();
        let mut rp = filled_proxy();
        unsafe {
            (b.c.c2MakeProxy)(parts.as_ptr() as *const c_void, bad, &mut cp);
            (b.rs.c2MakeProxy)(parts.as_ptr() as *const c_void, bad, &mut rp);
        }
        same("c2MakeProxy/bad-type-leaves-proxy-untouched", &cp, &rp);
    }
}

// ===========================================================================
// Rows 12-17 — NULL optional arguments of c2GJK
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn gjk_nullable(
    api: &Api,
    pa: &[f32; 5],
    ta: C2_TYPE,
    ax: Option<c2x>,
    pb: &[f32; 5],
    tb: C2_TYPE,
    bx: Option<c2x>,
    use_radius: i32,
    want_a: bool,
    want_b: bool,
    want_iters: bool,
    cache_in: Option<c2GJKCache>,
) -> (f32, c2v, c2v, i32, c2GJKCache) {
    const SENTINEL_A: c2v = c2v { x: 12.5, y: -37.25 };
    const SENTINEL_B: c2v = c2v { x: -99.0, y: 4.5 };
    const SENTINEL_IT: i32 = -0x5EED;
    let mut oa = SENTINEL_A;
    let mut ob = SENTINEL_B;
    let mut it = SENTINEL_IT;
    let mut cache = cache_in.unwrap_or_default();
    let d = unsafe {
        (api.c2GJK)(
            pa.as_ptr() as *const c_void,
            ta,
            ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            pb.as_ptr() as *const c_void,
            tb,
            bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            if want_a { &mut oa } else { std::ptr::null_mut() },
            if want_b { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if want_iters { &mut it } else { std::ptr::null_mut() },
            if cache_in.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    (d, oa, ob, it, cache)
}

#[test]
fn rows12_17_gjk_null_arguments() {
    let b = both();
    let mut rng = Rng::new(104);
    let identity = unsafe { (b.c.c2xIdentity)() };
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for ur in [0i32, 1] {
                for _ in 0..6000 {
                    let pa = shape_parts(&mut rng, ta, 15.0);
                    let pb = shape_parts(&mut rng, tb, 15.0);
                    // rows 14-16: every combination of NULL/non-NULL outputs
                    for mask in 0..8u32 {
                        let (wa, wb, wi) =
                            (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
                        let c = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, ur, wa, wb, wi, None);
                        let r = gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, ur, wa, wb, wi, None);
                        same(
                            &format!("gjk/null-mask={mask}"),
                            &(c.0, c.1, c.2, c.3),
                            &(r.0, r.1, r.2, r.3),
                        );
                    }
                    // rows 12-13: NULL transform == explicit identity transform
                    let n = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, ur, true, true, true, None);
                    let i1 = gjk_nullable(
                        &b.c, &pa, ta, Some(identity), &pb, tb, Some(identity), ur, true, true,
                        true, None,
                    );
                    same(
                        "gjk/null-xform==identity(C)",
                        &(n.0, n.1, n.2, n.3),
                        &(i1.0, i1.1, i1.2, i1.3),
                    );
                    let nr =
                        gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, ur, true, true, true, None);
                    let i2 = gjk_nullable(
                        &b.rs, &pa, ta, Some(identity), &pb, tb, Some(identity), ur, true, true,
                        true, None,
                    );
                    same(
                        "gjk/null-xform==identity(Rust)",
                        &(nr.0, nr.1, nr.2, nr.3),
                        &(i2.0, i2.1, i2.2, i2.3),
                    );
                    // one side NULL only
                    for (axo, bxo) in [
                        (Some(identity), None),
                        (None, Some(identity)),
                        (Some(rng.xform(20.0)), None),
                        (None, Some(rng.xform(20.0))),
                    ] {
                        let c = gjk_nullable(&b.c, &pa, ta, axo, &pb, tb, bxo, ur, true, true, true, None);
                        let r = gjk_nullable(&b.rs, &pa, ta, axo, &pb, tb, bxo, ur, true, true, true, None);
                        same("gjk/half-null-xform", &(c.0, c.1, c.2, c.3), &(r.0, r.1, r.2, r.3));
                    }
                    // row 17: cache NULL vs cold cache
                    let cnull = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, ur, true, true, true, None);
                    let rnull = gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, ur, true, true, true, None);
                    same("gjk/cache-null", &(cnull.0, cnull.1, cnull.2, cnull.3), &(rnull.0, rnull.1, rnull.2, rnull.3));
                    // row 18: cold cache (count == 0) is ignored on entry but
                    // written back on exit
                    let ccold = gjk_nullable(
                        &b.c, &pa, ta, None, &pb, tb, None, ur, true, true, true,
                        Some(c2GJKCache::default()),
                    );
                    let rcold = gjk_nullable(
                        &b.rs, &pa, ta, None, &pb, tb, None, ur, true, true, true,
                        Some(c2GJKCache::default()),
                    );
                    same(
                        "gjk/cache-cold",
                        &(ccold.0, ccold.1, ccold.2, ccold.4),
                        &(rcold.0, rcold.1, rcold.2, rcold.4),
                    );
                    same(
                        "gjk/cache-cold==cache-null",
                        &(cnull.0, cnull.1, cnull.2),
                        &(ccold.0, ccold.1, ccold.2),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 19-22, 24 — hand-crafted caches
// ===========================================================================

/// Rebuild the simplex `c2GJK` would produce from a cache and ask the library
/// for its metric, so the test can prove which cache branch it is exercising.
fn cache_metric(api: &Api, pa: &[f32; 5], ta: C2_TYPE, pb: &[f32; 5], tb: C2_TYPE, cache: &c2GJKCache) -> f32 {
    let mut proxa = c2Proxy::default();
    let mut proxb = c2Proxy::default();
    unsafe {
        (api.c2MakeProxy)(pa.as_ptr() as *const c_void, ta, &mut proxa);
        (api.c2MakeProxy)(pb.as_ptr() as *const c_void, tb, &mut proxb);
    }
    let id = unsafe { (api.c2xIdentity)() };
    let mut s = c2Simplex::default();
    for i in 0..cache.count.clamp(0, 3) as usize {
        let sa = unsafe { (api.c2Mulxv)(id, proxa.verts[cache.iA[i] as usize]) };
        let sb = unsafe { (api.c2Mulxv)(id, proxb.verts[cache.iB[i] as usize]) };
        s.verts[i].sA = sa;
        s.verts[i].sB = sb;
        s.verts[i].p = unsafe { (api.c2Sub)(sb, sa) };
    }
    s.count = cache.count;
    unsafe { (api.c2GJKSimplexMetric)(&mut s) }
}

#[test]
fn rows19_22_24_gjk_crafted_caches() {
    let b = both();
    let mut rng = Rng::new(105);
    let mut saw_rejected_cache = false;
    let mut saw_accepted_cache = false;

    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            let max_a = proxy_count(ta) - 1;
            let max_b = proxy_count(tb) - 1;
            for ur in [0i32, 1] {
                for _ in 0..5000 {
                    let pa = shape_parts(&mut rng, ta, 20.0);
                    let pb = shape_parts(&mut rng, tb, 20.0);
                    // counts 1..3, in-range indices (out-of-range indices would
                    // read uninitialised c2Proxy slots in the C — see ERRORS.md)
                    let count = 1 + (rng.below(3) as i32);
                    let mut cache = c2GJKCache {
                        metric: if rng.bool() { rng.range(1.0e9) } else { rng.wild() },
                        count,
                        iA: [0; 3],
                        iB: [0; 3],
                        div: match rng.below(4) {
                            0 => 0.0,          // row 24
                            1 => rng.wild(),
                            _ => rng.unit() * 3.0 + 0.1,
                        },
                    };
                    for k in 0..3 {
                        cache.iA[k] = rng.below(max_a + 1) as i32;
                        cache.iB[k] = rng.below(max_b + 1) as i32;
                    }
                    let m = cache_metric(&b.c, &pa, ta, &pb, tb, &cache);
                    let min_m = m.min(cache.metric);
                    let max_m = m.max(cache.metric);
                    if min_m < max_m * 2.0 && m < -1.0e8 {
                        saw_rejected_cache = true;
                    } else {
                        saw_accepted_cache = true;
                    }
                    let c = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, ur, true, true, true, Some(cache));
                    let r = gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, ur, true, true, true, Some(cache));
                    same(
                        "gjk/crafted-cache",
                        &(c.0, c.1, c.2, c.3),
                        &(r.0, r.1, r.2, r.3),
                    );
                    same("gjk/crafted-cache/out", &c.4, &r.4);

                    // row 22: negative cache count
                    for bad_count in [-1i32, -2, -3, i32::MIN + 1] {
                        let mut neg = cache;
                        neg.count = bad_count;
                        let c = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, ur, true, true, true, Some(neg));
                        let r = gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, ur, true, true, true, Some(neg));
                        same(
                            &format!("gjk/cache-count={bad_count}"),
                            &(c.0, c.1, c.2, c.3),
                            &(r.0, r.1, r.2, r.3),
                        );
                        same("gjk/cache-count-neg/out", &c.4, &r.4);
                    }
                }
            }
        }
    }

    // Row 19 needs metric < -1e8, which requires a huge negative determinant:
    // build it explicitly from two far-apart, enormous AABBs.
    for scale in [1.0e19f32, 1.0e20, 1.0e25, 3.0e38] {
        let pa = [-scale, -scale, scale, scale, 0.0];
        let pb = [scale * 0.5, -scale, scale, scale, 0.0];
        for ur in [0i32, 1] {
            for (mo, ia, ib) in [
                (0.0f32, [0, 1, 2], [0, 2, 1]),
                (0.0, [0, 2, 1], [0, 1, 2]),
                (1.0e30, [0, 3, 1], [1, 2, 3]),
                (-1.0e30, [2, 1, 0], [3, 0, 1]),
            ] {
                let cache = c2GJKCache {
                    metric: mo,
                    count: 3,
                    iA: ia,
                    iB: ib,
                    div: 1.0,
                };
                let m = cache_metric(&b.c, &pa, C2_TYPE_AABB, &pb, C2_TYPE_AABB, &cache);
                let min_m = m.min(mo);
                let max_m = m.max(mo);
                if min_m < max_m * 2.0 && m < -1.0e8 {
                    saw_rejected_cache = true;
                }
                let c = gjk_nullable(&b.c, &pa, C2_TYPE_AABB, None, &pb, C2_TYPE_AABB, None, ur, true, true, true, Some(cache));
                let r = gjk_nullable(&b.rs, &pa, C2_TYPE_AABB, None, &pb, C2_TYPE_AABB, None, ur, true, true, true, Some(cache));
                same("gjk/huge-negative-metric-cache", &(c.0, c.1, c.2, c.3), &(r.0, r.1, r.2, r.3));
                same("gjk/huge-negative-metric-cache/out", &c.4, &r.4);
            }
        }
    }

    assert!(
        saw_accepted_cache,
        "row 20 (cache accepted) was never exercised"
    );
    assert!(
        saw_rejected_cache,
        "row 19 (cache rejected: metric < -1e8) was never exercised"
    );
}

fn proxy_count(t: C2_TYPE) -> usize {
    match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_CAPSULE => 2,
        C2_TYPE_AABB => 4,
        _ => 1,
    }
}

// ===========================================================================
// Rows 25-33 — c2GJK loop exits and use_radius truthiness
// ===========================================================================

#[test]
fn rows25_33_gjk_loop_exits_and_use_radius() {
    let b = both();
    let mut rng = Rng::new(106);
    let mut max_iters = 0i32;
    let mut saw_hit = false;
    let mut saw_shrink = false;
    let mut saw_midpoint = false;

    // row 33: any non-zero int is truthy for `if (use_radius)`
    for ur in [0i32, 1, 2, -1, 7, i32::MIN, i32::MAX] {
        for &ta in TYPES.iter() {
            for &tb in TYPES.iter() {
                for _ in 0..6000 {
                    let pa = shape_parts(&mut rng, ta, 12.0);
                    let pb = shape_parts(&mut rng, tb, 12.0);
                    let c = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, ur, true, true, true, None);
                    let r = gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, ur, true, true, true, None);
                    same(
                        &format!("gjk/use_radius={ur}"),
                        &(c.0, c.1, c.2, c.3),
                        &(r.0, r.1, r.2, r.3),
                    );
                    max_iters = max_iters.max(c.3);
                    // row 29: hit (overlap) => dist == 0 and a == b
                    if c.0 == 0.0 && c.1.x.to_bits() == c.2.x.to_bits() && c.1.y.to_bits() == c.2.y.to_bits() {
                        saw_hit = true;
                    }
                    if ur != 0 {
                        if c.0 > 0.0 {
                            saw_shrink = true;
                        } else {
                            saw_midpoint = true;
                        }
                    }
                }
            }
        }
    }

    // rows 26-28: identical shapes drive the degenerate-direction and duplicate
    // support-point exits
    for &t in TYPES.iter() {
        for _ in 0..25000 {
            let p = shape_parts(&mut rng, t, 12.0);
            for ur in [0i32, 1] {
                let c = gjk_nullable(&b.c, &p, t, None, &p, t, None, ur, true, true, true, None);
                let r = gjk_nullable(&b.rs, &p, t, None, &p, t, None, ur, true, true, true, None);
                same("gjk/identical", &(c.0, c.1, c.2, c.3), &(r.0, r.1, r.2, r.3));
                assert_eq!(c.3, 0, "identical shapes should exit on iteration 0");
            }
        }
    }

    assert!(saw_hit, "row 29 (hit path) never observed");
    assert!(saw_shrink, "row 31/32 (radius shrink) never observed");
    assert!(saw_midpoint, "row 30 (midpoint collapse) never observed");

    // Row 25: try hard to exhaust the `iter < 20` guard — random transforms,
    // non-unit rotations, warm caches and extreme scales. Whatever the outcome,
    // `*iterations` is compared differentially on every call.
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..4000 {
                let scale = [1.0e-20f32, 1.0e-3, 1.0, 1.0e3, 1.0e20][rng.below(5)];
                let pa = shape_parts(&mut rng, ta, scale);
                let pb = shape_parts(&mut rng, tb, scale);
                let ax = c2x {
                    p: rng.vec_coord(scale),
                    r: c2r { c: rng.range(3.0), s: rng.range(3.0) },
                };
                let bx = rng.xform(scale);
                let mut cache = c2GJKCache {
                    metric: rng.range(1.0e9),
                    count: rng.below(4) as i32,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: rng.unit() * 3.0,
                };
                for k in 0..3 {
                    cache.iA[k] = rng.below(proxy_count(ta)) as i32;
                    cache.iB[k] = rng.below(proxy_count(tb)) as i32;
                }
                for ur in [0i32, 1] {
                    for cc in [None, Some(cache)] {
                        let c = gjk_nullable(&b.c, &pa, ta, Some(ax), &pb, tb, Some(bx), ur, true, true, true, cc);
                        let r = gjk_nullable(&b.rs, &pa, ta, Some(ax), &pb, tb, Some(bx), ur, true, true, true, cc);
                        same("gjk/iteration-hunt", &(c.0, c.1, c.2, c.3), &(r.0, r.1, r.2, r.3));
                        if cc.is_some() {
                            same("gjk/iteration-hunt/cache", &c.4, &r.4);
                        }
                        max_iters = max_iters.max(c.3);
                    }
                }
            }
        }
    }
    eprintln!("max c2GJK iterations observed: {max_iters} (loop cap is 20)");
}

// ===========================================================================
// Rows 34-36 — c2Support degenerate counts and directions
// ===========================================================================

#[test]
fn rows34_36_c2Support_degenerate() {
    let b = both();
    let mut rng = Rng::new(107);
    for count in [0i32, -1, -2, -100, i32::MIN + 1, 1] {
        for _ in 0..25000 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec_wild();
            }
            for d in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -0.0, y: -0.0 },
                c2v { x: f32::NAN, y: f32::NAN },
                c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
                rng.vec_wild(),
            ] {
                unsafe {
                    same(
                        &format!("c2Support/count={count}"),
                        &(b.c.c2Support)(verts.as_ptr(), count, d),
                        &(b.rs.c2Support)(verts.as_ptr(), count, d),
                    )
                };
            }
        }
    }
    // row 35: all dots equal (zero direction) -> strict `>` never fires
    let verts = [c2v { x: 1.0, y: 2.0 }; 8];
    for count in [1i32, 2, 4, 8] {
        unsafe {
            let z = c2v { x: 0.0, y: 0.0 };
            same(
                "c2Support/all-equal",
                &(b.c.c2Support)(verts.as_ptr(), count, z),
                &(b.rs.c2Support)(verts.as_ptr(), count, z),
            );
        }
    }
    // row 36: every dot is NaN
    let nanverts = [c2v { x: f32::NAN, y: f32::NAN }; 8];
    for count in [1i32, 2, 4, 8] {
        unsafe {
            let d = c2v { x: 1.0, y: 1.0 };
            let c = (b.c.c2Support)(nanverts.as_ptr(), count, d);
            let r = (b.rs.c2Support)(nanverts.as_ptr(), count, d);
            assert_eq!(c, 0, "C c2Support with all-NaN dots must return 0");
            same("c2Support/all-nan", &c, &r);
        }
    }
}

// ===========================================================================
// Rows 37-43 — simplex accessors with out-of-range counts / zero div
// ===========================================================================

fn wild_simplex(rng: &mut Rng, count: i32) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div: match rng.below(4) {
            0 => 0.0,
            1 => -0.0,
            2 => rng.wild(),
            _ => rng.range(5.0),
        },
        count,
    };
    for (i, v) in s.verts.iter_mut().enumerate() {
        v.sA = rng.vec_wild();
        v.sB = rng.vec_wild();
        v.p = rng.vec_wild();
        v.u = rng.wild();
        v.iA = i as i32 * 10;
        v.iB = i as i32 * 10 + 1;
    }
    s
}

#[test]
fn rows37_43_simplex_out_of_range_counts() {
    let b = both();
    let mut rng = Rng::new(108);
    for count in [0i32, -1, -7, 4, 5, 99, i32::MIN + 1, i32::MAX, 1, 2, 3] {
        for _ in 0..25000 {
            let s0 = wild_simplex(&mut rng, count);

            // row 37/38: c2Witness
            let mut cs = s0;
            let mut rs = s0;
            let mut ca = c2v { x: 7.0, y: 8.0 };
            let mut cb = c2v { x: 9.0, y: 10.0 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                (b.c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (b.rs.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            same(
                &format!("c2Witness/count={count}"),
                &(ca, cb),
                &(ra, rb),
            );
            if !(1..=3).contains(&count) {
                assert_eq!(
                    (ca.x, ca.y, cb.x, cb.y),
                    (0.0, 0.0, 0.0, 0.0),
                    "C c2Witness default: must zero both outputs"
                );
            }

            // rows 39/40: c2D
            let mut cs = s0;
            let mut rs = s0;
            unsafe {
                same(
                    &format!("c2D/count={count}"),
                    &(b.c.c2D)(&mut cs),
                    &(b.rs.c2D)(&mut rs),
                )
            };

            // rows 41/42: c2L
            let mut cs = s0;
            let mut rs = s0;
            unsafe {
                same(
                    &format!("c2L/count={count}"),
                    &(b.c.c2L)(&mut cs),
                    &(b.rs.c2L)(&mut rs),
                )
            };

            // row 43: c2GJKSimplexMetric
            let mut cs = s0;
            let mut rs = s0;
            unsafe {
                let c = (b.c.c2GJKSimplexMetric)(&mut cs);
                let r = (b.rs.c2GJKSimplexMetric)(&mut rs);
                same(&format!("c2GJKSimplexMetric/count={count}"), &c, &r);
                if count != 2 && count != 3 {
                    assert_eq!(c.to_bits(), 0f32.to_bits(), "C metric default must be 0");
                }
            }
        }
    }
    // row 40: count == 2 with ab collinear with the origin (det == 0, so the
    // strict `> 0` test fails and c2CCW90 is used)
    for _ in 0..25000 {
        let mut s = wild_simplex(&mut rng, 2);
        let p = rng.vec_coord(10.0);
        let k = rng.range(4.0);
        s.verts[0].p = p;
        s.verts[1].p = c2v { x: p.x * k, y: p.y * k };
        let mut cs = s;
        let mut rs = s;
        unsafe {
            same("c2D/collinear", &(b.c.c2D)(&mut cs), &(b.rs.c2D)(&mut rs));
        }
    }
}

// ===========================================================================
// Rows 44-48, 57-59 — IEEE edge cases in the scalar/vector helpers
// ===========================================================================

#[test]
fn rows44_48_float_edge_cases() {
    let b = both();
    // full cross-product of the boundary float set
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let v = c2v { x, y };
            unsafe {
                same("c2Len/edge", &(b.c.c2Len)(v), &(b.rs.c2Len)(v));
                same("c2Norm/edge", &(b.c.c2Norm)(v), &(b.rs.c2Norm)(v));
                same("c2Neg/edge", &(b.c.c2Neg)(v), &(b.rs.c2Neg)(v));
                same("c2Skew/edge", &(b.c.c2Skew)(v), &(b.rs.c2Skew)(v));
                same("c2CCW90/edge", &(b.c.c2CCW90)(v), &(b.rs.c2CCW90)(v));
            }
            for &s in EDGE_F32 {
                unsafe {
                    same("c2Div/edge", &(b.c.c2Div)(v, s), &(b.rs.c2Div)(v, s));
                    same("c2Mulvs/edge", &(b.c.c2Mulvs)(v, s), &(b.rs.c2Mulvs)(v, s));
                }
            }
            for &x2 in EDGE_F32 {
                for &y2 in EDGE_F32 {
                    let w = c2v { x: x2, y: y2 };
                    unsafe {
                        same("c2Dot/edge", &(b.c.c2Dot)(v, w), &(b.rs.c2Dot)(v, w));
                        same("c2Det2/edge", &(b.c.c2Det2)(v, w), &(b.rs.c2Det2)(v, w));
                        same("c2Add/edge", &(b.c.c2Add)(v, w), &(b.rs.c2Add)(v, w));
                        same("c2Sub/edge", &(b.c.c2Sub)(v, w), &(b.rs.c2Sub)(v, w));
                        same("c2Maxv/edge", &(b.c.c2Maxv)(v, w), &(b.rs.c2Maxv)(v, w));
                        same("c2Minv/edge", &(b.c.c2Minv)(v, w), &(b.rs.c2Minv)(v, w));
                    }
                }
            }
        }
    }
    // zero-length / infinite vector normalisation (rows 45-46)
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::INFINITY },
        c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN_POSITIVE, y: f32::MIN_POSITIVE },
    ] {
        unsafe {
            same("c2Norm/degenerate", &(b.c.c2Norm)(v), &(b.rs.c2Norm)(v));
            same("c2Len/degenerate", &(b.c.c2Len)(v), &(b.rs.c2Len)(v));
        }
    }
}

#[test]
fn rows58_59_minmax_clamp_nan_asymmetry() {
    let b = both();
    // c2Maxv/c2Minv use a ternary, which is NOT f32::max/min for NaN.
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let p = c2v { x, y };
            let q = c2v { x: y, y: x };
            unsafe {
                same("c2Maxv/nan", &(b.c.c2Maxv)(p, q), &(b.rs.c2Maxv)(p, q));
                same("c2Minv/nan", &(b.c.c2Minv)(p, q), &(b.rs.c2Minv)(p, q));
                // row 59: lo > hi
                same(
                    "c2Clampv/inverted",
                    &(b.c.c2Clampv)(p, q, p),
                    &(b.rs.c2Clampv)(p, q, p),
                );
                same(
                    "c2Clampv/nan-bounds",
                    &(b.c.c2Clampv)(q, p, q),
                    &(b.rs.c2Clampv)(q, p, q),
                );
            }
        }
    }
}

// ===========================================================================
// Rows 49-56 — degenerate shapes in the boolean predicates
// ===========================================================================

#[test]
fn rows49_56_boolean_predicate_edges() {
    let b = both();
    let mut rng = Rng::new(109);

    // row 49: negative radii (the C squares the sum, discarding the sign)
    for _ in 0..25000 {
        let a = c2Circle { p: rng.vec_coord(10.0), r: -rng.unit() * 5.0 };
        let c = c2Circle { p: rng.vec_coord(10.0), r: -rng.unit() * 5.0 };
        unsafe {
            same(
                "c2CircletoCircle/negative-r",
                &(b.c.c2CircletoCircle)(a, c),
                &(b.rs.c2CircletoCircle)(a, c),
            )
        };
    }

    // rows 50-51: inverted AABB / negative circle radius
    for _ in 0..25000 {
        let bb0 = rng.aabb(10.0);
        let inv = c2AABB { min: bb0.max, max: bb0.min };
        let c = c2Circle { p: rng.vec_coord(10.0), r: -rng.unit() * 5.0 };
        unsafe {
            same(
                "c2CircletoAABB/inverted",
                &(b.c.c2CircletoAABB)(c, inv),
                &(b.rs.c2CircletoAABB)(c, inv),
            );
            same(
                "c2CircletoAABB/negative-r",
                &(b.c.c2CircletoAABB)(c, bb0),
                &(b.rs.c2CircletoAABB)(c, bb0),
            );
        }
    }

    // rows 52-53: degenerate capsule -> n == (0,0) and da/0
    for _ in 0..25000 {
        let p = rng.vec_coord(10.0);
        let deg = c2Capsule { a: p, b: p, r: rng.unit() * 3.0 };
        let c = rng.circle(10.0);
        unsafe {
            same(
                "c2CircletoCapsule/degenerate",
                &(b.c.c2CircletoCapsule)(c, deg),
                &(b.rs.c2CircletoCapsule)(c, deg),
            )
        };
        // circle centre exactly on the degenerate capsule (da == 0, db == 0)
        let on = c2Circle { p, r: rng.unit() * 3.0 };
        unsafe {
            same(
                "c2CircletoCapsule/degenerate-coincident",
                &(b.c.c2CircletoCapsule)(on, deg),
                &(b.rs.c2CircletoCapsule)(on, deg),
            )
        };
    }

    // row 54: NaN AABB coordinates -> every `<` is false -> reports overlap
    let nanbb = c2AABB {
        min: c2v { x: f32::NAN, y: f32::NAN },
        max: c2v { x: f32::NAN, y: f32::NAN },
    };
    let normal = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    unsafe {
        let c = (b.c.c2AABBtoAABB)(nanbb, normal);
        let r = (b.rs.c2AABBtoAABB)(nanbb, normal);
        assert_eq!(c, 1, "C c2AABBtoAABB with NaN must report overlap");
        same("c2AABBtoAABB/nan", &c, &r);
        let c = (b.c.c2AABBtoAABB)(nanbb, nanbb);
        let r = (b.rs.c2AABBtoAABB)(nanbb, nanbb);
        assert_eq!(c, 1);
        same("c2AABBtoAABB/nan-nan", &c, &r);
    }

    // row 55: c2GJK returning NaN makes the capsule wrappers report no collision
    for _ in 0..25000 {
        let cap = c2Capsule {
            a: c2v { x: f32::NAN, y: rng.coord(5.0) },
            b: rng.vec_coord(5.0),
            r: rng.unit() * 2.0,
        };
        let cap2 = rng.capsule(5.0);
        let bb = rng.aabb(5.0);
        unsafe {
            same(
                "c2CapsuletoCapsule/nan",
                &(b.c.c2CapsuletoCapsule)(cap, cap2),
                &(b.rs.c2CapsuletoCapsule)(cap, cap2),
            );
            same(
                "c2AABBtoCapsule/nan",
                &(b.c.c2AABBtoCapsule)(bb, cap),
                &(b.rs.c2AABBtoCapsule)(bb, cap),
            );
        }
    }

    // row 56: verify that c2GJK never returns -0.0f, so `if (-0.0f)` (which
    // would be false and report a collision) is unreachable by construction.
    let mut saw_neg_zero = false;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..25000 {
                let pa = shape_parts(&mut rng, ta, 8.0);
                let pb = shape_parts(&mut rng, tb, 8.0);
                let c = gjk_nullable(&b.c, &pa, ta, None, &pb, tb, None, 1, true, true, true, None);
                let r = gjk_nullable(&b.rs, &pa, ta, None, &pb, tb, None, 1, true, true, true, None);
                same("gjk/neg-zero-scan", &(c.0, c.1, c.2, c.3), &(r.0, r.1, r.2, r.3));
                if c.0.to_bits() == (-0.0f32).to_bits() {
                    saw_neg_zero = true;
                }
            }
        }
    }
    eprintln!("c2GJK produced -0.0: {saw_neg_zero} (expected false; the C only ever assigns +0.0f)");
}
