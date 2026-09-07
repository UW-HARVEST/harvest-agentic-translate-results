//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every case constructs the exact invalid input the C checks for and asserts
//! that BOTH libraries return the identical sentinel / error result — not merely
//! that "both failed".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Out-of-range `C2_TYPE` values. C enums accept any `int`, so all of these are
/// real inputs an external caller can pass across the FFI boundary.
const BAD_TYPES: &[c_int] = &[
    3,
    4,
    5,
    -1,
    -2,
    -1000,
    255,
    256,
    0x1_0000,
    100_000,
    c_int::MAX,
    c_int::MIN,
    c_int::MAX - 1,
    c_int::MIN + 1,
];

/// Out-of-range `c2Simplex.count` values.
const BAD_COUNTS: &[c_int] = &[
    0,
    4,
    5,
    6,
    -1,
    -2,
    -100,
    255,
    c_int::MAX,
    c_int::MIN,
];

// ===========================================================================
// Row 1 — c2MakeProxy with an out-of-range type: nothing is stored at all.
// ===========================================================================
#[test]
fn err01_makeproxy_bad_type_writes_nothing() {
    let p = apis();
    let mut rng = Rng::new(0xE001);
    for &ty in BAD_TYPES {
        for &fill in &[0x00u8, 0x5A, 0xAA, 0xFF] {
            for _ in 0..64 {
                let shape = rng.capsule();
                let mut pc: c2Proxy = unsafe { std::mem::zeroed() };
                unsafe { std::ptr::write_bytes(&mut pc as *mut c2Proxy as *mut u8, fill, 1) };
                let before = pc;
                let mut pr = pc;
                unsafe {
                    (p.c.c2MakeProxy)(&shape as *const c2Capsule as *const c_void, ty, &mut pc);
                    (p.r.c2MakeProxy)(&shape as *const c2Capsule as *const c_void, ty, &mut pr);
                }
                eq_bytes("c2MakeProxy/bad-type", &(ty, fill), &pc, &pr);
                // and the documented behaviour: the buffer is untouched
                eq_bytes("c2MakeProxy/bad-type/untouched", &(ty, fill), &before, &pc);
            }
        }
    }
}

// ===========================================================================
// Row 2 — c2GJKSimplexMetric: `default:` falls into `case 1:` -> 0.0f
// ===========================================================================
#[test]
fn err02_gjksimplexmetric_bad_count_returns_zero() {
    let p = apis();
    let mut rng = Rng::new(0xE002);
    for &count in BAD_COUNTS {
        for _ in 0..200 {
            let mut sc = rng.simplex(count);
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2GJKSimplexMetric)(&mut sc);
                let rv = (p.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32("c2GJKSimplexMetric/bad-count", &count, cv, rv);
                assert_eq!(cv.to_bits(), 0u32, "C should return +0.0 for count={count}");
            }
            eq_simplex("c2GJKSimplexMetric/bad-count/untouched", &count, &sc, &sr);
        }
    }
}

// ===========================================================================
// Row 3 — c2D: `default:` shares `case 3:` -> c2V(0,0)
// ===========================================================================
#[test]
fn err03_c2D_bad_count_returns_zero_vector() {
    let p = apis();
    let mut rng = Rng::new(0xE003);
    let mut counts: Vec<c_int> = BAD_COUNTS.to_vec();
    counts.push(3); // `case 3:` shares the `default:` label
    for &count in &counts {
        for _ in 0..200 {
            let mut sc = rng.simplex(count);
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2D)(&mut sc);
                let rv = (p.r.c2D)(&mut sr);
                eq_v("c2D/bad-count", &count, cv, rv);
                assert_eq!(
                    (cv.x.to_bits(), cv.y.to_bits()),
                    (0, 0),
                    "C should return (+0,+0) for count={count}"
                );
            }
            eq_simplex("c2D/bad-count/untouched", &count, &sc, &sr);
        }
    }
}

// ===========================================================================
// Row 4 — c2Witness: `default:` writes (0,0)/(0,0) but STILL evaluates 1/div.
// ===========================================================================
#[test]
fn err04_c2Witness_bad_count_writes_zero_vectors() {
    let p = apis();
    let mut rng = Rng::new(0xE004);
    for &count in BAD_COUNTS {
        for &div in &[0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
            for _ in 0..40 {
                let mut sc = rng.simplex(count);
                sc.div = div;
                let mut sr = sc;
                let mut ac = c2v { x: 99.0, y: -99.0 };
                let mut bc = c2v { x: -7.0, y: 7.0 };
                let (mut ar, mut br) = (ac, bc);
                unsafe {
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                eq_v("c2Witness/bad-count/a", &(count, div), ac, ar);
                eq_v("c2Witness/bad-count/b", &(count, div), bc, br);
                assert_eq!(
                    (ac.x.to_bits(), ac.y.to_bits(), bc.x.to_bits(), bc.y.to_bits()),
                    (0, 0, 0, 0),
                    "C should zero both out-vectors for count={count}"
                );
                eq_simplex("c2Witness/bad-count/untouched", &(count, div), &sc, &sr);
            }
        }
    }
}

// ===========================================================================
// Row 5 — c2L: `default:` -> c2V(0,0), 1/div still evaluated.
// ===========================================================================
#[test]
fn err05_c2L_bad_count_returns_zero_vector() {
    let p = apis();
    let mut rng = Rng::new(0xE005);
    let mut counts: Vec<c_int> = BAD_COUNTS.to_vec();
    counts.push(3);
    for &count in &counts {
        for &div in &[0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY] {
            for _ in 0..40 {
                let mut sc = rng.simplex(count);
                sc.div = div;
                let mut sr = sc;
                unsafe {
                    let cv = (p.c.c2L)(&mut sc);
                    let rv = (p.r.c2L)(&mut sr);
                    eq_v("c2L/bad-count", &(count, div), cv, rv);
                    assert_eq!((cv.x.to_bits(), cv.y.to_bits()), (0, 0));
                }
                eq_simplex("c2L/bad-count/untouched", &(count, div), &sc, &sr);
            }
        }
    }
}

// ===========================================================================
// Rows 6, 7, 8, 9 — c2Collided with out-of-range enum values -> 0
// ===========================================================================
#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct Buf([u8; 32]);

fn buf_of<T: Copy>(v: &T) -> Buf {
    let mut b = Buf([0u8; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

fn collided(x: *const c_void, tx: c_int, y: *const c_void, ty: c_int, ctx: &dyn std::fmt::Debug) -> c_int {
    let p = apis();
    unsafe {
        let cv = (p.c.c2Collided)(x, tx, y, ty);
        let rv = (p.r.c2Collided)(x, tx, y, ty);
        eq_int("c2Collided", ctx, cv, rv);
        cv
    }
}

#[test]
fn err06_err08_collided_bad_typeB_returns_zero() {
    let mut rng = Rng::new(0xE006);
    for _ in 0..200 {
        let ci = rng.circle();
        let bb = rng.aabb();
        let ca = rng.capsule();
        let bufs: [(Buf, c_int); 3] = [
            (buf_of(&ci), C2_TYPE_CIRCLE),
            (buf_of(&bb), C2_TYPE_AABB),
            (buf_of(&ca), C2_TYPE_CAPSULE),
        ];
        let other = buf_of(&ca);
        // rows 6/7/8: typeA valid, typeB out of range
        for (a, ta) in &bufs {
            for &tb in BAD_TYPES {
                let got = collided(
                    a.0.as_ptr() as *const c_void,
                    *ta,
                    other.0.as_ptr() as *const c_void,
                    tb,
                    &(*ta, tb),
                );
                assert_eq!(got, 0, "typeA={ta} typeB={tb} must yield 0");
            }
        }
    }
}

#[test]
fn err09_collided_bad_typeA_returns_zero_without_touching_B() {
    let mut rng = Rng::new(0xE009);
    for _ in 0..200 {
        let ca = rng.capsule();
        let b = buf_of(&ca);
        for &ta in BAD_TYPES {
            // typeB valid ...
            for &tb in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                let got = collided(
                    b.0.as_ptr() as *const c_void,
                    ta,
                    b.0.as_ptr() as *const c_void,
                    tb,
                    &(ta, tb),
                );
                assert_eq!(got, 0, "typeA={ta} must yield 0");
            }
            // ... typeB also invalid
            for &tb in BAD_TYPES {
                let got = collided(
                    b.0.as_ptr() as *const c_void,
                    ta,
                    b.0.as_ptr() as *const c_void,
                    tb,
                    &(ta, tb),
                );
                assert_eq!(got, 0);
            }
            // `B` is never dereferenced on this path: a NULL B must not crash
            let got = collided(
                b.0.as_ptr() as *const c_void,
                ta,
                std::ptr::null(),
                C2_TYPE_CIRCLE,
                &(ta, "null-B"),
            );
            assert_eq!(got, 0);
            // and a NULL A too (also never dereferenced)
            let got = collided(std::ptr::null(), ta, std::ptr::null(), ta, &(ta, "null-both"));
            assert_eq!(got, 0);
        }
    }
}

// ===========================================================================
// Rows 10, 11 — c2GJK NULL transforms substitute c2xIdentity()
// ===========================================================================
#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct SB([u8; 32]);

fn sb<T: Copy>(v: &T) -> SB {
    let mut b = SB([0u8; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

#[derive(Copy, Clone, Debug, Default)]
struct Out {
    dist: f32,
    a: c2v,
    b: c2v,
    it: c_int,
    cache: c2GJKCache,
    cache_used: bool,
}

#[allow(clippy::too_many_arguments)]
unsafe fn gjk(
    f: FnGJK,
    a: &SB,
    ta: c_int,
    ax: *const c2x,
    b: &SB,
    tb: c_int,
    bx: *const c2x,
    outs: bool,
    ur: c_int,
    iters: bool,
    cache: Option<&mut c2GJKCache>,
) -> Out {
    let mut oa = c2v { x: 42.5, y: -13.25 };
    let mut ob = c2v { x: -77.0, y: 3.5 };
    let mut it: c_int = -999;
    let used = cache.is_some();
    let cp = match cache {
        Some(c) => c as *mut c2GJKCache,
        None => std::ptr::null_mut(),
    };
    let dist = f(
        a.0.as_ptr() as *const c_void,
        ta,
        ax,
        b.0.as_ptr() as *const c_void,
        tb,
        bx,
        if outs { &mut oa } else { std::ptr::null_mut() },
        if outs { &mut ob } else { std::ptr::null_mut() },
        ur,
        if iters { &mut it } else { std::ptr::null_mut() },
        cp,
    );
    Out {
        dist,
        a: oa,
        b: ob,
        it,
        cache: if cp.is_null() { c2GJKCache::default() } else { *cp },
        cache_used: used,
    }
}

#[track_caller]
fn cmp(what: &str, ctx: &dyn std::fmt::Debug, c: &Out, r: &Out) {
    eq_f32(&format!("{what}.dist"), ctx, c.dist, r.dist);
    eq_v(&format!("{what}.a"), ctx, c.a, r.a);
    eq_v(&format!("{what}.b"), ctx, c.b, r.b);
    eq_int(&format!("{what}.it"), ctx, c.it, r.it);
    if c.cache_used {
        eq_bytes(&format!("{what}.cache"), ctx, &c.cache, &r.cache);
    }
}

const IDENT: c2x = c2x {
    p: c2v { x: 0.0, y: 0.0 },
    r: c2r { c: 1.0, s: 0.0 },
};

#[test]
fn err10_err11_gjk_null_transforms_equal_identity() {
    let p = apis();
    let mut rng = Rng::new(0xE010);
    for _ in 0..1500 {
        for &(ta, tb) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CAPSULE),
            (C2_TYPE_CAPSULE, C2_TYPE_CAPSULE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            let mk = |rng: &mut Rng, t: c_int| match t {
                C2_TYPE_CIRCLE => sb(&rng.circle()),
                C2_TYPE_AABB => sb(&rng.aabb()),
                _ => sb(&rng.capsule()),
            };
            let a = mk(&mut rng, ta);
            let b = mk(&mut rng, tb);
            let nul: *const c2x = std::ptr::null();
            let idp: *const c2x = &IDENT;
            for &ur in &[0i32, 1] {
                for &(axp, bxp) in &[(nul, nul), (idp, nul), (nul, idp), (idp, idp)] {
                    unsafe {
                        let oc = gjk(p.c.c2GJK, &a, ta, axp, &b, tb, bxp, true, ur, true, None);
                        let or_ = gjk(p.r.c2GJK, &a, ta, axp, &b, tb, bxp, true, ur, true, None);
                        cmp("c2GJK/null-xform", &(ta, tb, ur), &oc, &or_);
                    }
                }
                // NULL must be *equivalent* to passing the identity transform
                unsafe {
                    let n = gjk(p.c.c2GJK, &a, ta, nul, &b, tb, nul, true, ur, true, None);
                    let i = gjk(p.c.c2GJK, &a, ta, idp, &b, tb, idp, true, ur, true, None);
                    assert_eq!(
                        n.dist.to_bits(),
                        i.dist.to_bits(),
                        "C: NULL xform != identity xform for ({ta},{tb},{ur})"
                    );
                    let n = gjk(p.r.c2GJK, &a, ta, nul, &b, tb, nul, true, ur, true, None);
                    let i = gjk(p.r.c2GJK, &a, ta, idp, &b, tb, idp, true, ur, true, None);
                    assert_eq!(n.dist.to_bits(), i.dist.to_bits(), "Rust: same");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 12, 13, 14, 15 — c2GJK NULL out-params / NULL cache
// ===========================================================================
#[test]
fn err12_err15_gjk_null_outparams() {
    let p = apis();
    let mut rng = Rng::new(0xE012);
    for _ in 0..1200 {
        for &(ta, tb) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_CAPSULE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
            (C2_TYPE_CAPSULE, C2_TYPE_AABB),
        ] {
            let mk = |rng: &mut Rng, t: c_int| match t {
                C2_TYPE_CIRCLE => sb(&rng.circle()),
                C2_TYPE_AABB => sb(&rng.aabb()),
                _ => sb(&rng.capsule()),
            };
            let a = mk(&mut rng, ta);
            let b = mk(&mut rng, tb);
            for &outs in &[false, true] {
                for &iters in &[false, true] {
                    for &ur in &[0i32, 1] {
                        unsafe {
                            // row 15: NULL cache
                            let oc = gjk(p.c.c2GJK, &a, ta, std::ptr::null(), &b, tb, std::ptr::null(), outs, ur, iters, None);
                            let or_ = gjk(p.r.c2GJK, &a, ta, std::ptr::null(), &b, tb, std::ptr::null(), outs, ur, iters, None);
                            cmp("c2GJK/null-out", &(ta, tb, outs, iters, ur), &oc, &or_);
                            // sentinel values must survive when the pointer is NULL
                            if !outs {
                                assert_eq!(oc.a.x.to_bits(), 42.5f32.to_bits());
                                assert_eq!(or_.a.x.to_bits(), 42.5f32.to_bits());
                            }
                            if !iters {
                                assert_eq!(oc.it, -999);
                                assert_eq!(or_.it, -999);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 16 — non-NULL cache with count == 0: read skipped, write-back happens
// ===========================================================================
#[test]
fn err16_gjk_cache_count_zero() {
    let p = apis();
    let mut rng = Rng::new(0xE016);
    for _ in 0..2000 {
        let a = sb(&rng.capsule());
        let b = sb(&rng.aabb());
        // count == 0 but every other field is garbage
        let dirty = c2GJKCache {
            metric: rng.f32_in(-1.0e9, 1.0e9),
            count: 0,
            iA: [7, -3, 11],
            iB: [-1, 99, 4],
            div: rng.f32_in(-9.0, 9.0),
        };
        let mut cc = dirty;
        let mut rc = dirty;
        for &ur in &[0i32, 1] {
            unsafe {
                let oc = gjk(p.c.c2GJK, &a, C2_TYPE_CAPSULE, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, ur, true, Some(&mut cc));
                let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_CAPSULE, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, ur, true, Some(&mut rc));
                cmp("c2GJK/cache0", &(dirty, ur), &oc, &or_);
            }
            eq_bytes("c2GJK/cache0/writeback", &(dirty, ur), &cc, &rc);
            // documented behaviour: the write-back always overwrites count
            assert!(cc.count >= 1 && cc.count <= 3, "count={} ", cc.count);
        }
    }
}

// ===========================================================================
// Row 17 — the inverted validity test: a warm cache is (almost) always trusted
// ===========================================================================
#[test]
fn err17_gjk_inverted_cache_validity_test() {
    let p = apis();
    let mut rng = Rng::new(0xE017);
    for _ in 0..1500 {
        let a = sb(&rng.aabb());
        let b = sb(&rng.aabb());
        // a cache whose metric is wildly wrong in every direction
        for &metric in &[
            0.0f32,
            1.0,
            -1.0,
            -1.0e9,
            -1.0e30,
            f32::MAX,
            f32::MIN,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            for &count in &[1i32, 2, 3] {
                let h = c2GJKCache {
                    metric,
                    count,
                    iA: [0, 1, 2],
                    iB: [3, 0, 1],
                    div: rng.f32_in(-4.0, 4.0),
                };
                let mut cc = h;
                let mut rc = h;
                unsafe {
                    let oc = gjk(p.c.c2GJK, &a, C2_TYPE_AABB, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, 1, true, Some(&mut cc));
                    let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_AABB, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, 1, true, Some(&mut rc));
                    cmp("c2GJK/inverted-validity", &(metric, count), &oc, &or_);
                }
                eq_bytes("c2GJK/inverted-validity/cache", &(metric, count), &cc, &rc);
            }
        }
    }
}

// ===========================================================================
// Row 18 — cache->count > 3 walks past cache->iA[3]
// ===========================================================================
#[test]
fn err18_gjk_cache_count_above_three() {
    let p = apis();
    let mut rng = Rng::new(0xE018);
    // The loop reads cache->iA[3] (== iB[0]) and cache->iB[3] (== the `div`
    // bits reinterpreted as int). Craft the struct so both land on vertex 0,
    // keeping the read inside the initialised part of the proxy.
    for _ in 0..800 {
        let a = sb(&rng.aabb());
        let b = sb(&rng.aabb());
        let h = c2GJKCache {
            metric: rng.f32_in(-50.0, 50.0),
            count: 4,
            iA: [0, 1, 2],
            iB: [0, 0, 0],           // iA[3] aliases iB[0] == 0
            div: f32::from_bits(0),  // iB[3] aliases div == 0 -> index 0
        };
        let mut cc = h;
        let mut rc = h;
        for &ur in &[0i32, 1] {
            let mut c1 = cc;
            let mut r1 = rc;
            unsafe {
                let oc = gjk(p.c.c2GJK, &a, C2_TYPE_AABB, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, ur, true, Some(&mut c1));
                let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_AABB, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, ur, true, Some(&mut r1));
                cmp("c2GJK/cache-count4", &(h, ur), &oc, &or_);
            }
            eq_bytes("c2GJK/cache-count4/cache", &(h, ur), &c1, &r1);
        }
        cc = h;
        rc = h;
        let _ = (cc, rc);
    }
}

// ===========================================================================
// Row 19 — cache indices at the edge of the *initialised* proxy vertices
// ===========================================================================
#[test]
fn err19_gjk_cache_indices_at_proxy_bounds() {
    let p = apis();
    let mut rng = Rng::new(0xE019);
    // AABB proxies fill verts[0..4]; index 3 is the last valid one. Indices
    // >= count read `c2Proxy` bytes the C never initialises (genuine C UB,
    // see ERRORS.md), so the comparable range is [0, count).
    for _ in 0..2000 {
        let a = sb(&rng.aabb());
        let b = sb(&rng.aabb());
        for &count in &[1i32, 2, 3] {
            for ia in 0..4i32 {
                for ib in 0..4i32 {
                    let h = c2GJKCache {
                        metric: rng.f32_in(-20.0, 20.0),
                        count,
                        iA: [ia, (ia + 1) % 4, (ia + 2) % 4],
                        iB: [ib, (ib + 3) % 4, (ib + 1) % 4],
                        div: rng.f32_in(-4.0, 4.0),
                    };
                    let mut cc = h;
                    let mut rc = h;
                    unsafe {
                        let oc = gjk(p.c.c2GJK, &a, C2_TYPE_AABB, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, 1, true, Some(&mut cc));
                        let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_AABB, std::ptr::null(), &b, C2_TYPE_AABB, std::ptr::null(), true, 1, true, Some(&mut rc));
                        cmp("c2GJK/cache-idx", &(count, ia, ib), &oc, &or_);
                    }
                    eq_bytes("c2GJK/cache-idx/cache", &(count, ia, ib), &cc, &rc);
                }
            }
        }
    }
}

// ===========================================================================
// Row 20 — the iteration cap (`while (iter < 20)`)
// ===========================================================================
#[test]
fn err20_gjk_iteration_cap() {
    let p = apis();
    let mut rng = Rng::new(0xE020);
    let mut max_seen = 0;
    for _ in 0..30_000 {
        let ta = *pick(&[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE], &mut rng);
        let tb = *pick(&[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE], &mut rng);
        let mk = |rng: &mut Rng, t: c_int| match t {
            C2_TYPE_CIRCLE => sb(&rng.circle()),
            C2_TYPE_AABB => sb(&rng.aabb()),
            _ => sb(&rng.capsule()),
        };
        let a = mk(&mut rng, ta);
        let b = mk(&mut rng, tb);
        unsafe {
            let oc = gjk(p.c.c2GJK, &a, ta, std::ptr::null(), &b, tb, std::ptr::null(), true, 1, true, None);
            let or_ = gjk(p.r.c2GJK, &a, ta, std::ptr::null(), &b, tb, std::ptr::null(), true, 1, true, None);
            cmp("c2GJK/iter-cap", &(ta, tb), &oc, &or_);
            assert!(oc.it >= 0 && oc.it <= 20, "C iterations out of range: {}", oc.it);
            assert_eq!(oc.it, or_.it);
            max_seen = max_seen.max(oc.it);
        }
    }
    assert!(max_seen >= 1, "iteration loop never ran");
}

// ===========================================================================
// Rows 21, 22, 23 — the radius/hit fix-ups at the end of c2GJK
// ===========================================================================
#[test]
fn err21_err23_gjk_radius_and_hit_fixups() {
    let p = apis();
    let mut rng = Rng::new(0xE021);
    // Row 23 (hit): heavily overlapping shapes.
    // Row 21 (dist <= rA+rB or dist <= FLT_EPSILON): huge radii on tiny gaps.
    // Row 22 (a == b after the shrink): symmetric configurations.
    for _ in 0..8000 {
        let c = rng.v();
        let big = c2Circle { p: c, r: rng.f32_in(50.0, 200.0) };
        let small = c2Circle {
            p: c2v { x: c.x + rng.f32_in(-2.0, 2.0), y: c.y + rng.f32_in(-2.0, 2.0) },
            r: rng.f32_in(0.0, 3.0),
        };
        let a = sb(&big);
        let b = sb(&small);
        for &ur in &[0i32, 1] {
            unsafe {
                let oc = gjk(p.c.c2GJK, &a, C2_TYPE_CIRCLE, std::ptr::null(), &b, C2_TYPE_CIRCLE, std::ptr::null(), true, ur, true, None);
                let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_CIRCLE, std::ptr::null(), &b, C2_TYPE_CIRCLE, std::ptr::null(), true, ur, true, None);
                cmp("c2GJK/overlap", &(big, small, ur), &oc, &or_);
            }
        }
        // exactly coincident circles -> dist == 0 <= FLT_EPSILON -> row 21 else-branch
        let same = sb(&big);
        for &ur in &[0i32, 1] {
            unsafe {
                let oc = gjk(p.c.c2GJK, &a, C2_TYPE_CIRCLE, std::ptr::null(), &same, C2_TYPE_CIRCLE, std::ptr::null(), true, ur, true, None);
                let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_CIRCLE, std::ptr::null(), &same, C2_TYPE_CIRCLE, std::ptr::null(), true, ur, true, None);
                cmp("c2GJK/coincident", &(big, ur), &oc, &or_);
                assert_eq!(oc.dist.to_bits(), 0u32, "coincident shapes must give +0.0");
            }
        }
        // gap just above/below FLT_EPSILON
        for &g in &[0.0f32, f32::EPSILON * 0.5, f32::EPSILON, f32::EPSILON * 2.0, 1.0e-6] {
            let n1 = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 };
            let n2 = c2Circle { p: c2v { x: g, y: 0.0 }, r: 0.0 };
            let (x, y) = (sb(&n1), sb(&n2));
            for &ur in &[0i32, 1] {
                unsafe {
                    let oc = gjk(p.c.c2GJK, &x, C2_TYPE_CIRCLE, std::ptr::null(), &y, C2_TYPE_CIRCLE, std::ptr::null(), true, ur, true, None);
                    let or_ = gjk(p.r.c2GJK, &x, C2_TYPE_CIRCLE, std::ptr::null(), &y, C2_TYPE_CIRCLE, std::ptr::null(), true, ur, true, None);
                    cmp("c2GJK/eps-gap", &(g, ur), &oc, &or_);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 24, 25 — NaN / Inf coordinates propagate instead of being rejected
// ===========================================================================
#[test]
fn err24_err25_gjk_nan_inf_shapes() {
    let p = apis();
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let shapes: [(SB, c_int); 3] = [
                (sb(&c2Circle { p: c2v { x: s, y: t }, r: t }), C2_TYPE_CIRCLE),
                (
                    sb(&c2AABB { min: c2v { x: s, y: t }, max: c2v { x: t, y: s } }),
                    C2_TYPE_AABB,
                ),
                (
                    sb(&c2Capsule { a: c2v { x: s, y: t }, b: c2v { x: t, y: s }, r: s }),
                    C2_TYPE_CAPSULE,
                ),
            ];
            let plain: [(SB, c_int); 3] = [
                (sb(&c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 }), C2_TYPE_CIRCLE),
                (
                    sb(&c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
                    C2_TYPE_AABB,
                ),
                (
                    sb(&c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 5.0, y: 5.0 }, r: 1.0 }),
                    C2_TYPE_CAPSULE,
                ),
            ];
            for (a, ta) in &shapes {
                for (b, tb) in &plain {
                    for &ur in &[0i32, 1] {
                        let z = c2GJKCache::default();
                        let mut cc = z;
                        let mut rc = z;
                        unsafe {
                            let oc = gjk(p.c.c2GJK, a, *ta, std::ptr::null(), b, *tb, std::ptr::null(), true, ur, true, Some(&mut cc));
                            let or_ = gjk(p.r.c2GJK, a, *ta, std::ptr::null(), b, *tb, std::ptr::null(), true, ur, true, Some(&mut rc));
                            cmp("c2GJK/nan-shape", &(s, t, ta, tb, ur), &oc, &or_);
                        }
                        eq_bytes("c2GJK/nan-shape/cache", &(s, t, ta, tb, ur), &cc, &rc);
                    }
                }
                // NaN shape against itself
                for &ur in &[0i32, 1] {
                    unsafe {
                        let oc = gjk(p.c.c2GJK, a, *ta, std::ptr::null(), a, *ta, std::ptr::null(), true, ur, true, None);
                        let or_ = gjk(p.r.c2GJK, a, *ta, std::ptr::null(), a, *ta, std::ptr::null(), true, ur, true, None);
                        cmp("c2GJK/nan-self", &(s, t, ta, ur), &oc, &or_);
                    }
                }
            }
            // NaN / Inf inside the transforms
            let bx = c2x { p: c2v { x: s, y: t }, r: c2r { c: t, s } };
            for (a, ta) in &plain {
                for (b, tb) in &plain {
                    unsafe {
                        let oc = gjk(p.c.c2GJK, a, *ta, &bx, b, *tb, &bx, true, 1, true, None);
                        let or_ = gjk(p.r.c2GJK, a, *ta, &bx, b, *tb, &bx, true, 1, true, None);
                        cmp("c2GJK/nan-xform", &(s, t, ta, tb), &oc, &or_);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 26, 27 — c2Support with count <= 0 and count > 8
// ===========================================================================
#[test]
fn err26_err27_c2Support_bad_counts() {
    let p = apis();
    let mut rng = Rng::new(0xE026);
    // A 32-slot array owned by the test, so even "past the proxy" reads stay
    // inside memory both libraries see identically.
    for _ in 0..4000 {
        let mut verts = [c2v::default(); 32];
        for v in verts.iter_mut() {
            *v = rng.v();
        }
        let d = rng.any_v();
        // count <= 0: verts[0] is still dereferenced, loop body never runs -> 0
        for &count in &[0i32, -1, -2, -100, c_int::MIN + 1] {
            unsafe {
                let cv = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rv = (p.r.c2Support)(verts.as_ptr(), count, d);
                eq_int("c2Support/count<=0", &(count, d), cv, rv);
                assert_eq!(cv, 0, "count={count} must return 0");
            }
        }
        // count > 8 (past what c2Proxy would hold) but inside our array
        for &count in &[9i32, 12, 16, 24, 32] {
            unsafe {
                let cv = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rv = (p.r.c2Support)(verts.as_ptr(), count, d);
                eq_int("c2Support/count>8", &(count, d), cv, rv);
                assert!(cv >= 0 && cv < count);
            }
        }
        // NaN direction: `dot > dmax` never true -> index 0
        let nd = c2v { x: f32::NAN, y: f32::NAN };
        unsafe {
            let cv = (p.c.c2Support)(verts.as_ptr(), 32, nd);
            let rv = (p.r.c2Support)(verts.as_ptr(), 32, nd);
            eq_int("c2Support/nan-dir", &nd, cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

// ===========================================================================
// Row 28 — c2Witness / c2L with div == 0 (1/0 == Inf, 0*Inf == NaN)
// ===========================================================================
#[test]
fn err28_div_zero_in_witness_and_L() {
    let p = apis();
    let mut rng = Rng::new(0xE028);
    for &div in &[0.0f32, -0.0] {
        for &count in &[1i32, 2, 3, 0, 4, -1] {
            for _ in 0..500 {
                let mut sc = rng.simplex(count);
                sc.div = div;
                let mut sr = sc;
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let (mut ar, mut br) = (ac, bc);
                unsafe {
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                    eq_v("c2Witness/div0/a", &(div, count), ac, ar);
                    eq_v("c2Witness/div0/b", &(div, count), bc, br);
                    let mut s2c = sc;
                    let mut s2r = sr;
                    let cv = (p.c.c2L)(&mut s2c);
                    let rv = (p.r.c2L)(&mut s2r);
                    eq_v("c2L/div0", &(div, count), cv, rv);
                }
            }
        }
    }
}

// ===========================================================================
// Row 29 — c2Div / c2Norm with a zero divisor / zero-length vector
// ===========================================================================
#[test]
fn err29_div_and_norm_by_zero() {
    let p = apis();
    let mut rng = Rng::new(0xE029);
    for &b in &[0.0f32, -0.0] {
        for _ in 0..2000 {
            let a = rng.any_v();
            unsafe {
                eq_v("c2Div/zero", &(a, b), (p.c.c2Div)(a, b), (p.r.c2Div)(a, b));
            }
        }
        for &x in SPECIAL_F32 {
            for &y in SPECIAL_F32 {
                let a = c2v { x, y };
                unsafe {
                    eq_v("c2Div/zero/sp", &(a, b), (p.c.c2Div)(a, b), (p.r.c2Div)(a, b));
                }
            }
        }
    }
    for &(x, y) in &[(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        let a = c2v { x, y };
        unsafe {
            let cv = (p.c.c2Norm)(a);
            let rv = (p.r.c2Norm)(a);
            eq_v("c2Norm/zero", &a, cv, rv);
            assert!(cv.x.is_nan() && cv.y.is_nan(), "expected NaN, got {cv:?}");
        }
    }
}

// ===========================================================================
// Row 30 — c2Len of a vector whose dot product is negative / NaN
// ===========================================================================
#[test]
fn err30_c2Len_negative_or_nan_dot() {
    let p = apis();
    // Inf components make Dot == Inf; NaN components make Dot == NaN.
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = c2v { x, y };
            unsafe {
                eq_f32("c2Len/sp", &a, (p.c.c2Len)(a), (p.r.c2Len)(a));
            }
        }
    }
    // +Inf and -Inf in the same vector -> Inf + Inf, or Inf*0 -> NaN
    for &(x, y) in &[
        (f32::INFINITY, f32::NEG_INFINITY),
        (f32::NEG_INFINITY, f32::INFINITY),
        (f32::INFINITY, 0.0),
        (0.0, f32::NEG_INFINITY),
        (f32::MAX, f32::MAX),
        (f32::MIN, f32::MAX),
    ] {
        let a = c2v { x, y };
        unsafe {
            eq_f32("c2Len/inf", &a, (p.c.c2Len)(a), (p.r.c2Len)(a));
            eq_v("c2Norm/inf", &a, (p.c.c2Norm)(a), (p.r.c2Norm)(a));
        }
    }
}

// ===========================================================================
// Row 31 — degenerate capsule in c2CircletoCapsule (dot(n,n) == 0)
// ===========================================================================
#[test]
fn err31_circletocapsule_degenerate_capsule() {
    let p = apis();
    let mut rng = Rng::new(0xE031);
    for _ in 0..4000 {
        let a = rng.v();
        let B = c2Capsule { a, b: a, r: rng.radius() };
        for pt in [a, rng.v(), c2v { x: 0.0, y: 0.0 }] {
            let A = c2Circle { p: pt, r: rng.radius() };
            unsafe {
                eq_int(
                    "c2CircletoCapsule/degen",
                    &(A, B),
                    (p.c.c2CircletoCapsule)(A, B),
                    (p.r.c2CircletoCapsule)(A, B),
                );
            }
        }
    }
    // n == 0 with da > 0 is impossible, but n == 0 with NaN coordinates is not
    for &s in SPECIAL_F32 {
        let a = c2v { x: s, y: s };
        let B = c2Capsule { a, b: a, r: s };
        for &t in SPECIAL_F32 {
            let A = c2Circle { p: c2v { x: t, y: -t }, r: t };
            unsafe {
                eq_int(
                    "c2CircletoCapsule/degen-sp",
                    &(A, B),
                    (p.c.c2CircletoCapsule)(A, B),
                    (p.r.c2CircletoCapsule)(A, B),
                );
            }
        }
    }
}

// ===========================================================================
// Row 32 — inverted AABBs are silently accepted
// ===========================================================================
#[test]
fn err32_inverted_aabbs() {
    let p = apis();
    let mut rng = Rng::new(0xE032);
    for _ in 0..8000 {
        let lo = rng.v();
        let hi = rng.v();
        // deliberately min > max on both axes
        let inv = c2AABB {
            min: c2v { x: lo.x.max(hi.x), y: lo.y.max(hi.y) },
            max: c2v { x: lo.x.min(hi.x), y: lo.y.min(hi.y) },
        };
        let ok = c2AABB {
            min: c2v { x: lo.x.min(hi.x), y: lo.y.min(hi.y) },
            max: c2v { x: lo.x.max(hi.x), y: lo.y.max(hi.y) },
        };
        let ci = rng.circle();
        let ca = rng.capsule();
        unsafe {
            eq_int("inv/AABBtoAABB", &(inv, ok), (p.c.c2AABBtoAABB)(inv, ok), (p.r.c2AABBtoAABB)(inv, ok));
            eq_int("inv/AABBtoAABB2", &(inv, inv), (p.c.c2AABBtoAABB)(inv, inv), (p.r.c2AABBtoAABB)(inv, inv));
            eq_int("inv/CircletoAABB", &(ci, inv), (p.c.c2CircletoAABB)(ci, inv), (p.r.c2CircletoAABB)(ci, inv));
            eq_int("inv/AABBtoCapsule", &(inv, ca), (p.c.c2AABBtoCapsule)(inv, ca), (p.r.c2AABBtoCapsule)(inv, ca));
            // and via c2Collided / c2GJK
            let ib = sb(&inv);
            let cb = sb(&ca);
            let cv = (p.c.c2Collided)(ib.0.as_ptr() as *const c_void, C2_TYPE_AABB, cb.0.as_ptr() as *const c_void, C2_TYPE_CAPSULE);
            let rv = (p.r.c2Collided)(ib.0.as_ptr() as *const c_void, C2_TYPE_AABB, cb.0.as_ptr() as *const c_void, C2_TYPE_CAPSULE);
            eq_int("inv/Collided", &(inv, ca), cv, rv);
            let oc = gjk(p.c.c2GJK, &ib, C2_TYPE_AABB, std::ptr::null(), &cb, C2_TYPE_CAPSULE, std::ptr::null(), true, 1, true, None);
            let or_ = gjk(p.r.c2GJK, &ib, C2_TYPE_AABB, std::ptr::null(), &cb, C2_TYPE_CAPSULE, std::ptr::null(), true, 1, true, None);
            cmp("inv/GJK", &(inv, ca), &oc, &or_);
        }
    }
}

// ===========================================================================
// Row 33 — a NaN distance is truthy in C, so the wrappers must return 0
// ===========================================================================
#[test]
fn err33_nan_distance_is_truthy() {
    let p = apis();
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let A = c2AABB { min: c2v { x: s, y: t }, max: c2v { x: t, y: s } };
            let B = c2Capsule { a: c2v { x: t, y: s }, b: c2v { x: s, y: t }, r: s };
            let K = c2Capsule { a: c2v { x: s, y: s }, b: c2v { x: t, y: t }, r: t };
            unsafe {
                eq_int("nan/AABBtoCapsule", &(A, B), (p.c.c2AABBtoCapsule)(A, B), (p.r.c2AABBtoCapsule)(A, B));
                eq_int("nan/CapsuletoCapsule", &(K, B), (p.c.c2CapsuletoCapsule)(K, B), (p.r.c2CapsuletoCapsule)(K, B));
                eq_int("nan/CapsuletoCapsule2", &(B, B), (p.c.c2CapsuletoCapsule)(B, B), (p.r.c2CapsuletoCapsule)(B, B));
            }
        }
    }
}

// ===========================================================================
// Row 34 — negative radii are not validated
// ===========================================================================
#[test]
fn err34_negative_radii() {
    let p = apis();
    let mut rng = Rng::new(0xE034);
    for _ in 0..6000 {
        let rA = -rng.f32_in(0.0, 100.0);
        let rB = -rng.f32_in(0.0, 100.0);
        let ci = c2Circle { p: rng.v(), r: rA };
        let cj = c2Circle { p: rng.v(), r: rB };
        let ka = c2Capsule { a: rng.v(), b: rng.v(), r: rA };
        let kb = c2Capsule { a: rng.v(), b: rng.v(), r: rB };
        let bb = rng.aabb();
        unsafe {
            eq_int("neg-r/CircletoCircle", &(ci, cj), (p.c.c2CircletoCircle)(ci, cj), (p.r.c2CircletoCircle)(ci, cj));
            eq_int("neg-r/CircletoAABB", &(ci, bb), (p.c.c2CircletoAABB)(ci, bb), (p.r.c2CircletoAABB)(ci, bb));
            eq_int("neg-r/CircletoCapsule", &(ci, kb), (p.c.c2CircletoCapsule)(ci, kb), (p.r.c2CircletoCapsule)(ci, kb));
            eq_int("neg-r/AABBtoCapsule", &(bb, kb), (p.c.c2AABBtoCapsule)(bb, kb), (p.r.c2AABBtoCapsule)(bb, kb));
            eq_int("neg-r/CapsuletoCapsule", &(ka, kb), (p.c.c2CapsuletoCapsule)(ka, kb), (p.r.c2CapsuletoCapsule)(ka, kb));
            // negative rA+rB *grows* the distance inside c2GJK
            let (x, y) = (sb(&ci), sb(&kb));
            for &ur in &[0i32, 1] {
                let oc = gjk(p.c.c2GJK, &x, C2_TYPE_CIRCLE, std::ptr::null(), &y, C2_TYPE_CAPSULE, std::ptr::null(), true, ur, true, None);
                let or_ = gjk(p.r.c2GJK, &x, C2_TYPE_CIRCLE, std::ptr::null(), &y, C2_TYPE_CAPSULE, std::ptr::null(), true, ur, true, None);
                cmp("neg-r/GJK", &(ci, kb, ur), &oc, &or_);
            }
        }
    }
}

// ===========================================================================
// Row 35 — capsule() never fails; returns a 3-bit mask
// ===========================================================================
#[test]
fn err35_capsule_never_fails() {
    let p = apis();
    let mut rng = Rng::new(0xE035);
    for _ in 0..50_000 {
        let a = rng.any_f32();
        let b = rng.any_f32();
        let c = rng.any_f32();
        let d = rng.any_f32();
        let e = rng.any_f32();
        unsafe {
            let cv = (p.c.capsule)(a, b, c, d, e);
            let rv = (p.r.capsule)(a, b, c, d, e);
            eq_int("capsule/any", &(a, b, c, d, e), cv, rv);
            assert!((0..8).contains(&cv), "capsule returned {cv}");
        }
    }
    // exhaustive over the special table for the two most influential args
    for &a in SPECIAL_F32 {
        for &b in SPECIAL_F32 {
            for &r in SPECIAL_F32 {
                unsafe {
                    let cv = (p.c.capsule)(a, b, -a, -b, r);
                    let rv = (p.r.capsule)(a, b, -a, -b, r);
                    eq_int("capsule/sp", &(a, b, r), cv, rv);
                }
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary checks (beyond the ERRORS.md rows)
// ===========================================================================
#[test]
fn generic_out_of_range_enums_everywhere() {
    let p = apis();
    let mut rng = Rng::new(0xEFFF);
    // c2MakeProxy and c2Collided are the only functions taking a C2_TYPE that
    // do not dereference the shape on the invalid path; both are covered above.
    // Here we additionally sweep *every* invalid enum through c2Collided while
    // varying which side is valid, and confirm the sentinel is exactly 0.
    for _ in 0..500 {
        let ci = rng.circle();
        let bb = rng.aabb();
        let ca = rng.capsule();
        let bufs = [
            (buf_of(&ci), C2_TYPE_CIRCLE),
            (buf_of(&bb), C2_TYPE_AABB),
            (buf_of(&ca), C2_TYPE_CAPSULE),
        ];
        let mut tys: Vec<c_int> = BAD_TYPES.to_vec();
        tys.extend_from_slice(&[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE]);
        for &ta in &tys {
            for &tb in &tys {
                let (a, _) = &bufs[(ta.rem_euclid(3)) as usize];
                let (b, _) = &bufs[(tb.rem_euclid(3)) as usize];
                let got = collided(
                    a.0.as_ptr() as *const c_void,
                    ta,
                    b.0.as_ptr() as *const c_void,
                    tb,
                    &(ta, tb),
                );
                let valid = (0..=2).contains(&ta) && (0..=2).contains(&tb);
                if !valid {
                    assert_eq!(got, 0, "({ta},{tb}) must be rejected with 0");
                }
            }
        }
    }
}

#[test]
fn generic_simplex_count_one_past_valid_range() {
    // count == 4 is exactly one past the largest valid value for every routine
    // that switches on it, and count == 0 exactly one before the smallest.
    let p = apis();
    let mut rng = Rng::new(0xEFFE);
    for &count in &[-1i32, 0, 4] {
        for _ in 0..2000 {
            let mut a = rng.simplex(count);
            let mut b = a;
            unsafe {
                eq_f32("metric/edge", &count, (p.c.c2GJKSimplexMetric)(&mut a), (p.r.c2GJKSimplexMetric)(&mut b));
            }
            let (mut a, mut b) = (a, b);
            unsafe {
                eq_v("c2D/edge", &count, (p.c.c2D)(&mut a), (p.r.c2D)(&mut b));
                eq_v("c2L/edge", &count, (p.c.c2L)(&mut a), (p.r.c2L)(&mut b));
            }
            let mut ac = c2v::default();
            let mut bc = c2v::default();
            let (mut ar, mut br) = (ac, bc);
            unsafe {
                (p.c.c2Witness)(&mut a, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut b, &mut ar, &mut br);
            }
            eq_v("c2Witness/edge-a", &count, ac, ar);
            eq_v("c2Witness/edge-b", &count, bc, br);
            eq_simplex("edge/simplex", &count, &a, &b);
            // c22 / c23 have no `default:` guard at all: they operate on
            // verts[0..2] / verts[0..3] regardless of `count`.
            let mut a2 = a;
            let mut b2 = b;
            unsafe {
                (p.c.c22)(&mut a2);
                (p.r.c22)(&mut b2);
            }
            eq_simplex("c22/edge", &count, &a2, &b2);
            let mut a3 = a;
            let mut b3 = b;
            unsafe {
                (p.c.c23)(&mut a3);
                (p.r.c23)(&mut b3);
            }
            eq_simplex("c23/edge", &count, &a3, &b3);
        }
    }
}

#[test]
fn generic_null_pointers_on_guarded_paths() {
    let p = apis();
    let mut rng = Rng::new(0xEFFD);
    let a = sb(&rng.circle());
    let b = sb(&rng.capsule());
    // Every pointer c2GJK actually null-checks, in all 2^4 combinations.
    for mask in 0u32..16 {
        let axp: *const c2x = if mask & 1 != 0 { &IDENT } else { std::ptr::null() };
        let bxp: *const c2x = if mask & 2 != 0 { &IDENT } else { std::ptr::null() };
        let outs = mask & 4 != 0;
        let iters = mask & 8 != 0;
        for &ur in &[0i32, 1] {
            for cache in [false, true] {
                let z = c2GJKCache::default();
                let mut cc = z;
                let mut rc = z;
                unsafe {
                    let oc = gjk(p.c.c2GJK, &a, C2_TYPE_CIRCLE, axp, &b, C2_TYPE_CAPSULE, bxp, outs, ur, iters, if cache { Some(&mut cc) } else { None });
                    let or_ = gjk(p.r.c2GJK, &a, C2_TYPE_CIRCLE, axp, &b, C2_TYPE_CAPSULE, bxp, outs, ur, iters, if cache { Some(&mut rc) } else { None });
                    cmp("null-mask", &(mask, ur, cache), &oc, &or_);
                }
                if cache {
                    eq_bytes("null-mask/cache", &(mask, ur), &cc, &rc);
                }
            }
        }
    }
    // c2Witness with only one of the two out-pointers is NOT guarded by the C,
    // so we only exercise the guarded surface here.
}

#[test]
fn generic_zero_and_oversized_lengths() {
    // c2Support is the only length-taking function; covered by err26_err27.
    // c2BBVerts always writes exactly 4 vertices -- verify with a tight buffer.
    let p = apis();
    let mut rng = Rng::new(0xEFFC);
    for _ in 0..4000 {
        let mut bbc = rng.aabb();
        let mut bbr = bbc;
        let mut oc = [c2v { x: f32::NAN, y: f32::NAN }; 4];
        let mut or_ = oc;
        unsafe {
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
            (p.r.c2BBVerts)(or_.as_mut_ptr(), &mut bbr);
        }
        eq_bytes("c2BBVerts/tight", &bbc, &oc, &or_);
        for v in oc.iter() {
            assert!(!(v.x.is_nan() && v.y.is_nan()) || bbc.min.x.is_nan() || bbc.max.x.is_nan());
        }
    }
}
