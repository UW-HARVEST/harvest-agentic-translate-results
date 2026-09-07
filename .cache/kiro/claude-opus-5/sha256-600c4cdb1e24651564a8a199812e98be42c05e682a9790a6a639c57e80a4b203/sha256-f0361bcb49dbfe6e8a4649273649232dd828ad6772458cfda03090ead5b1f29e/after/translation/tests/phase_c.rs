//! Phase C — error / rejection-path differential tests. One test per
//! `ERRORS.md` row, plus the generic FFI boundaries (null pointers, zero and
//! oversized lengths, out-of-range `C2_TYPE` enum values).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

fn pair() -> Pair {
    load_pair()
}

/// Every `int` that is *not* a valid `C2_TYPE` and is worth pushing across the
/// FFI boundary. C enums accept any `int`, so these are real inputs.
const BAD_TYPES: [c_int; 12] = [
    3,
    4,
    -1,
    -2,
    99,
    255,
    256,
    1000,
    c_int::MAX,
    c_int::MIN,
    0x1_0000,
    -0x1_0000,
];

/// Every simplex `count` that is not 1, 2 or 3.
const BAD_COUNTS: [c_int; 10] = [0, -1, -2, 4, 5, 8, 100, -100, c_int::MAX, c_int::MIN];

// ---------------------------------------------------------------------------
// Row 1 — c2MakeProxy with an out-of-range C2_TYPE leaves *p untouched
// ---------------------------------------------------------------------------

#[test]
fn err01_makeproxy_bad_type_leaves_proxy_untouched() {
    let p = pair();
    let mut g = Rng::new(0x1001);
    for ty in BAD_TYPES {
        for i in 0..64 {
            // A caller-initialised proxy makes the "no write at all" behaviour
            // observable and deterministic on both sides.
            let seed = c2Proxy {
                radius: g.coord(),
                count: g.below(9) as c_int,
                verts: [
                    g.vec(), g.vec(), g.vec(), g.vec(),
                    g.vec(), g.vec(), g.vec(), g.vec(),
                ],
            };
            let mut shape = g.capsule();
            let ctx = format!("err01 ty={ty} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut pr = seed;
                let mut sh = shape;
                (x.c2MakeProxy)(&mut sh as *mut _ as *const c_void, ty, &mut pr);
                pr
            });
            // The shape pointer is never dereferenced for an unknown type, so a
            // NULL shape is also a valid input here.
            let ctx = format!("err01 ty={ty} #{i} NULL shape");
            diff(&ctx, &p, |x| unsafe {
                let mut pr = seed;
                (x.c2MakeProxy)(core::ptr::null(), ty, &mut pr);
                pr
            });
            let _ = &mut shape;
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 2-5 — c2Collided with out-of-range types returns 0 without dereferencing
// ---------------------------------------------------------------------------

#[test]
fn err02_collided_bad_typeA() {
    let p = pair();
    let mut g = Rng::new(0x1002);
    for ty in BAD_TYPES {
        for tb in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            for i in 0..16 {
                let a = g.capsule();
                let b = g.capsule();
                let ctx = format!("err02 typeA={ty} typeB={tb} #{i}");
                diff(&ctx, &p, |x| unsafe {
                    let mut a = a;
                    let mut b = b;
                    (x.c2Collided)(
                        &mut a as *mut _ as *const c_void,
                        ty,
                        &mut b as *mut _ as *const c_void,
                        tb,
                    )
                });
            }
            // Neither pointer is dereferenced when typeA is unknown.
            let ctx = format!("err02 typeA={ty} typeB={tb} NULL");
            diff(&ctx, &p, |x| unsafe {
                (x.c2Collided)(core::ptr::null(), ty, core::ptr::null(), tb)
            });
        }
        // Both types invalid.
        let ctx = format!("err02 typeA={ty} typeB={ty} NULL");
        diff(&ctx, &p, |x| unsafe {
            (x.c2Collided)(core::ptr::null(), ty, core::ptr::null(), ty)
        });
    }
}

#[test]
fn err03_collided_circle_bad_typeB() {
    let p = pair();
    let mut g = Rng::new(0x1003);
    for ty in BAD_TYPES {
        for i in 0..32 {
            let a = g.circle();
            let ctx = format!("err03 typeB={ty} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut a = a;
                (x.c2Collided)(
                    &mut a as *mut _ as *const c_void,
                    C2_TYPE_CIRCLE,
                    core::ptr::null(),
                    ty,
                )
            });
        }
    }
}

#[test]
fn err04_collided_aabb_bad_typeB() {
    let p = pair();
    let mut g = Rng::new(0x1004);
    for ty in BAD_TYPES {
        for i in 0..32 {
            let a = g.aabb();
            let ctx = format!("err04 typeB={ty} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut a = a;
                (x.c2Collided)(
                    &mut a as *mut _ as *const c_void,
                    C2_TYPE_AABB,
                    core::ptr::null(),
                    ty,
                )
            });
        }
    }
}

#[test]
fn err05_collided_capsule_bad_typeB() {
    let p = pair();
    let mut g = Rng::new(0x1005);
    for ty in BAD_TYPES {
        for i in 0..32 {
            let a = g.capsule();
            let ctx = format!("err05 typeB={ty} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut a = a;
                (x.c2Collided)(
                    &mut a as *mut _ as *const c_void,
                    C2_TYPE_CAPSULE,
                    core::ptr::null(),
                    ty,
                )
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6-9 — out-of-range simplex counts hit the `default:` sentinels
// ---------------------------------------------------------------------------

#[test]
fn err06_simplex_metric_bad_count() {
    let p = pair();
    let mut g = Rng::new(0x1006);
    for count in BAD_COUNTS {
        for i in 0..64 {
            let s = g.simplex(count);
            let ctx = format!("err06 count={count} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut s = s;
                let m = (x.c2GJKSimplexMetric)(&mut s);
                (m, s)
            });
        }
    }
}

#[test]
fn err07_c2D_bad_count() {
    let p = pair();
    let mut g = Rng::new(0x1007);
    for count in [3, 0, -1, 4, 7, c_int::MAX, c_int::MIN] {
        for i in 0..64 {
            let s = g.simplex(count);
            let ctx = format!("err07 count={count} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut s = s;
                let d = (x.c2D)(&mut s);
                (d, s)
            });
        }
    }
}

#[test]
fn err08_witness_bad_count() {
    let p = pair();
    let mut g = Rng::new(0x1008);
    for count in BAD_COUNTS {
        for i in 0..64 {
            let s = g.simplex(count);
            let ctx = format!("err08 count={count} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut s = s;
                // Pre-loaded sentinels: the C's `default:` writes (0,0) to both.
                let mut a = c2v { x: 7.5, y: -8.5 };
                let mut b = c2v { x: -9.5, y: 10.5 };
                (x.c2Witness)(&mut s, &mut a, &mut b);
                (a, b, s)
            });
        }
    }
}

#[test]
fn err09_c2L_bad_count() {
    let p = pair();
    let mut g = Rng::new(0x1009);
    for count in BAD_COUNTS {
        for i in 0..64 {
            let s = g.simplex(count);
            let ctx = format!("err09 count={count} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut s = s;
                let l = (x.c2L)(&mut s);
                (l, s)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — c2Support with count <= 0 (and oversized counts)
// ---------------------------------------------------------------------------

#[test]
fn err10_support_zero_negative_and_oversized_count() {
    let p = pair();
    let mut g = Rng::new(0x100A);
    // 32 readable verts, so "oversized" counts stay inside memory the caller
    // owns — exactly what the C would read.
    for i in 0..600 {
        let mut verts = [c2v::default(); 32];
        for k in 0..32 {
            verts[k] = if i % 4 == 0 { g.vec_wide() } else { g.vec() };
        }
        let d = if i % 5 == 0 { g.vec_wide() } else { g.vec() };
        for count in [0i32, -1, -2, -100, c_int::MIN, 1, 9, 16, 32] {
            let ctx = format!("err10 #{i} count={count} d={d:?}");
            diff(&ctx, &p, |x| unsafe {
                (x.c2Support)(verts.as_ptr(), count, d)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 11-16 — c2GJK NULL-pointer handling
// ---------------------------------------------------------------------------

/// Calls `c2GJK` with a fully explicit pointer configuration and returns every
/// observable output (sentinels reveal a skipped write).
#[allow(clippy::too_many_arguments)]
fn gjk_raw(
    x: &Api,
    a: &c2Capsule,
    ta: c_int,
    b: &c2AABB,
    tb: c_int,
    ax: Option<c2x>,
    bx: Option<c2x>,
    want_a: bool,
    want_b: bool,
    want_it: bool,
    cache: Option<c2GJKCache>,
    use_radius: c_int,
) -> (f32, c2v, c2v, c_int, Option<c2GJKCache>) {
    unsafe {
        let mut sa = *a;
        let mut sb = *b;
        let mut axv = ax;
        let mut bxv = bx;
        let mut cv = cache;
        let mut oa = c2v { x: -1.5, y: 2.5 };
        let mut ob = c2v { x: 3.5, y: -4.5 };
        let mut it: c_int = -12345;
        let dist = (x.c2GJK)(
            &mut sa as *mut _ as *const c_void,
            ta,
            match axv.as_mut() {
                Some(t) => t as *const c2x,
                None => core::ptr::null(),
            },
            &mut sb as *mut _ as *const c_void,
            tb,
            match bxv.as_mut() {
                Some(t) => t as *const c2x,
                None => core::ptr::null(),
            },
            if want_a {
                &mut oa as *mut c2v
            } else {
                core::ptr::null_mut()
            },
            if want_b {
                &mut ob as *mut c2v
            } else {
                core::ptr::null_mut()
            },
            use_radius,
            if want_it {
                &mut it as *mut c_int
            } else {
                core::ptr::null_mut()
            },
            match cv.as_mut() {
                Some(c) => c as *mut c2GJKCache,
                None => core::ptr::null_mut(),
            },
        );
        (dist, oa, ob, it, cv)
    }
}

#[test]
fn err11_12_gjk_null_transforms_use_identity() {
    let p = pair();
    let mut g = Rng::new(0x100B);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    for i in 0..600 {
        let a = g.capsule();
        let b = g.aabb();
        let ur = (i % 2) as c_int;
        // NULL ax / NULL bx / both NULL must equal explicitly passing identity.
        for (ax, bx) in [
            (None, None),
            (Some(ident), None),
            (None, Some(ident)),
            (Some(ident), Some(ident)),
        ] {
            let ctx = format!("err11_12 #{i} ax={ax:?} bx={bx:?} ur={ur}");
            diff(&ctx, &p, |x| {
                let (d, oa, ob, it, _) = gjk_raw(
                    x,
                    &a,
                    C2_TYPE_CAPSULE,
                    &b,
                    C2_TYPE_AABB,
                    ax,
                    bx,
                    true,
                    true,
                    true,
                    None,
                    ur,
                );
                (d, oa, ob, it)
            });
        }
        // And the identity substitution really is equivalent, in both impls.
        for impl_ in [&p.c, &p.r] {
            let n = gjk_raw(
                impl_, &a, C2_TYPE_CAPSULE, &b, C2_TYPE_AABB, None, None, true, true, true, None,
                ur,
            );
            let e = gjk_raw(
                impl_,
                &a,
                C2_TYPE_CAPSULE,
                &b,
                C2_TYPE_AABB,
                Some(ident),
                Some(ident),
                true,
                true,
                true,
                None,
                ur,
            );
            assert_eq!(
                (n.0.to_bits(), n.1.fingerprint(), n.2.fingerprint(), n.3),
                (e.0.to_bits(), e.1.fingerprint(), e.2.fingerprint(), e.3),
                "{}: NULL transform != identity transform (#{i})",
                impl_.tag
            );
        }
    }
}

#[test]
fn err13_14_15_16_gjk_null_outparams_and_cache() {
    let p = pair();
    let mut g = Rng::new(0x100C);
    for i in 0..400 {
        let a = g.capsule();
        let b = g.aabb();
        for ur in [0, 1] {
            for want_a in [false, true] {
                for want_b in [false, true] {
                    for want_it in [false, true] {
                        for cache in [None, Some(c2GJKCache::default())] {
                            let ctx = format!(
                                "err13-16 #{i} ur={ur} a={want_a} b={want_b} it={want_it} \
                                 cache={}",
                                cache.is_some()
                            );
                            diff(&ctx, &p, |x| {
                                let (d, oa, ob, it, cv) = gjk_raw(
                                    x,
                                    &a,
                                    C2_TYPE_CAPSULE,
                                    &b,
                                    C2_TYPE_AABB,
                                    None,
                                    None,
                                    want_a,
                                    want_b,
                                    want_it,
                                    cache,
                                    ur,
                                );
                                // Sentinels are included so a *skipped* write is
                                // as observable as a performed one.
                                ((d, oa), (ob, it), cv)
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17-18, 21 — cache count edge values
// ---------------------------------------------------------------------------

#[test]
fn err17_gjk_cache_count_zero_is_ignored() {
    let p = pair();
    let mut g = Rng::new(0x100D);
    for i in 0..600 {
        let a = g.capsule();
        let b = g.aabb();
        let ur = (i % 2) as c_int;
        // count == 0 => cache_was_good false, every other field must be ignored.
        let junk = c2GJKCache {
            metric: g.coord_wide(),
            count: 0,
            iA: [7, 7, 7],
            iB: [7, 7, 7],
            div: g.coord_wide(),
        };
        let ctx = format!("err17 #{i} junk={junk:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let (d, oa, ob, it, cv) = gjk_raw(
                x,
                &a,
                C2_TYPE_CAPSULE,
                &b,
                C2_TYPE_AABB,
                None,
                None,
                true,
                true,
                true,
                Some(junk),
                ur,
            );
            ((d, oa), (ob, it), cv)
        });
        // ... and must give the same answer as passing a NULL cache.
        for impl_ in [&p.c, &p.r] {
            let with = gjk_raw(
                impl_,
                &a,
                C2_TYPE_CAPSULE,
                &b,
                C2_TYPE_AABB,
                None,
                None,
                true,
                true,
                true,
                Some(junk),
                ur,
            );
            let without = gjk_raw(
                impl_, &a, C2_TYPE_CAPSULE, &b, C2_TYPE_AABB, None, None, true, true, true, None,
                ur,
            );
            assert_eq!(
                with.0.to_bits(),
                without.0.to_bits(),
                "{}: count==0 cache changed the result (#{i})",
                impl_.tag
            );
        }
    }
}

#[test]
fn err18_gjk_cache_metric_guard_is_always_taken() {
    let p = pair();
    let mut g = Rng::new(0x100E);
    // The guard is `!(min_metric < max_metric*2 && metric < -1.0e8f)`. The
    // second conjunct is essentially never true, so `cache_was_read` is set
    // even for wildly wrong cached metrics. Feed metrics designed to probe both
    // sides of the comparison, including -inf / +inf / NaN / -1e8 exactly.
    let metrics = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        -1.0e8,
        -1.000_000_1e8,
        -1.0e9,
        -1.0e30,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN,
    ];
    for (mi, m) in metrics.iter().enumerate() {
        for i in 0..40 {
            let a = g.capsule();
            let b = g.capsule();
            let cache = c2GJKCache {
                metric: *m,
                count: [1, 2, 3][i % 3],
                iA: [0, 1, 0],
                iB: [0, 1, 1],
                div: [1.0f32, 0.0, -1.0, 2.5][i % 4],
            };
            let ur = (i % 2) as c_int;
            let ctx = format!("err18 m#{mi}={m:?} #{i} cache={cache:?} ur={ur}");
            diff(&ctx, &p, |x| unsafe {
                let mut sa = a;
                let mut sb = b;
                let mut cv = cache;
                let mut oa = c2v { x: -1.5, y: 2.5 };
                let mut ob = c2v { x: 3.5, y: -4.5 };
                let mut it: c_int = -12345;
                let d = (x.c2GJK)(
                    &mut sa as *mut _ as *const c_void,
                    C2_TYPE_CAPSULE,
                    core::ptr::null(),
                    &mut sb as *mut _ as *const c_void,
                    C2_TYPE_CAPSULE,
                    core::ptr::null(),
                    &mut oa,
                    &mut ob,
                    ur,
                    &mut it,
                    &mut cv,
                );
                ((d, oa), (ob, it), cv)
            });
        }
    }
}

#[test]
fn err21_gjk_cache_count_four_and_negative() {
    let p = pair();
    let mut g = Rng::new(0x100F);
    // NEGATIVE counts are fully defined and reproducible: `i < cache->count` is
    // false immediately, so the cache-read loop and the cache-write loop are
    // both skipped, `s.count` becomes the negative value, and every `switch`
    // falls to its `default:` sentinel.
    //
    // count >= 4 is NOT asserted, because the C crashes on it: `cache->iA[3]`
    // aliases `iB[0]` and `cache->iB[3]` aliases the FLOAT `div` reinterpreted
    // as an int (e.g. `1.0f` => 1065353216), which is then used to index
    // `pB.verts[]`. Verified directly against the C .so: a plain C caller with
    // `cache.count = 4` dies with SIGSEGV. See ERRORS.md row 21.
    for count in [-1i32, -2, -100, c_int::MIN] {
        for i in 0..200 {
            let a = g.capsule();
            let b = g.aabb();
            let cache = c2GJKCache {
                metric: g.coord(),
                count,
                iA: [g.below(3) as c_int, g.below(3) as c_int, g.below(3) as c_int],
                iB: [g.below(2) as c_int, g.below(2) as c_int, g.below(2) as c_int],
                div: [1.0f32, 0.0, 2.0][i % 3],
            };
            let ur = (i % 2) as c_int;
            let ctx = format!("err21 count={count} #{i} cache={cache:?} ur={ur}");
            diff(&ctx, &p, |x| {
                let (d, oa, ob, it, cv) = gjk_raw(
                    x,
                    &a,
                    C2_TYPE_CAPSULE,
                    &b,
                    C2_TYPE_AABB,
                    None,
                    None,
                    true,
                    true,
                    true,
                    Some(cache),
                    ur,
                );
                ((d, oa), (ob, it), cv)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — cached indices in range for the 8-slot vert array
// ---------------------------------------------------------------------------

#[test]
fn err20_gjk_cache_indices_within_proxy_count() {
    let p = pair();
    let mut g = Rng::new(0x1010);
    // Cached indices that are valid for the CURRENT proxy (AABB => 4 verts) are
    // fully defined and must agree. Indices >= the proxy's count would make the
    // C read an uninitialised `c2Proxy` stack local (ERRORS.md rows 19/20) and
    // are therefore not asserted.
    for i in 0..800 {
        let a = g.aabb();
        let b = g.aabb();
        let count = [1i32, 2, 3][i % 3];
        let cache = c2GJKCache {
            metric: g.coord(),
            count,
            iA: [g.below(4) as c_int, g.below(4) as c_int, g.below(4) as c_int],
            iB: [g.below(4) as c_int, g.below(4) as c_int, g.below(4) as c_int],
            div: [1.0f32, 0.0, 3.0, -2.0][i % 4],
        };
        let ur = (i % 2) as c_int;
        let ctx = format!("err20 #{i} cache={cache:?} ur={ur}");
        diff(&ctx, &p, |x| unsafe {
            let mut sa = a;
            let mut sb = b;
            let mut cv = cache;
            let mut oa = c2v { x: -1.5, y: 2.5 };
            let mut ob = c2v { x: 3.5, y: -4.5 };
            let mut it: c_int = -12345;
            let d = (x.c2GJK)(
                &mut sa as *mut _ as *const c_void,
                C2_TYPE_AABB,
                core::ptr::null(),
                &mut sb as *mut _ as *const c_void,
                C2_TYPE_AABB,
                core::ptr::null(),
                &mut oa,
                &mut ob,
                ur,
                &mut it,
                &mut cv,
            );
            ((d, oa), (ob, it), cv)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 19 — out-of-range C2_TYPE inside c2GJK is undefined in the C
// ---------------------------------------------------------------------------

#[test]
fn err19_gjk_bad_type_documented_ub_no_crash() {
    let p = pair();
    // `c2MakeProxy` writes nothing for an unknown type, so the C then reads
    // `c2Proxy pA;` — an UNINITIALISED stack local. `pA.count` is whatever the
    // previous frame left behind, which makes the result unreproducible (and
    // potentially a wild read). Byte-equality is therefore NOT asserted; what
    // IS asserted is that the Rust export survives the same call, so a caller
    // that passes a bogus enum cannot take the Rust library down where the C
    // survives.
    let mut g = Rng::new(0x1011);
    let mut rust_results = 0usize;
    for ty in [3i32, -1, 99, 12345] {
        for i in 0..32 {
            let a = g.capsule();
            let b = g.aabb();
            for (ta, tb) in [(ty, C2_TYPE_AABB), (C2_TYPE_CAPSULE, ty), (ty, ty)] {
                let (d, _, _, _, _) = gjk_raw(
                    &p.r,
                    &a,
                    ta,
                    &b,
                    tb,
                    None,
                    None,
                    true,
                    true,
                    true,
                    None,
                    (i % 2) as c_int,
                );
                assert!(d.is_nan() || d.is_finite() || d.is_infinite());
                rust_results += 1;
            }
        }
    }
    assert_eq!(rust_results, 4 * 32 * 3);
}

// ---------------------------------------------------------------------------
// Rows 22-28 — the c2GJK loop-exit and radius-collapse paths
// ---------------------------------------------------------------------------

#[test]
fn err22_to_28_gjk_loop_exits_and_radius_collapse() {
    let p = pair();
    let mut g = Rng::new(0x1012);
    // Constructions aimed at each exit: identical shapes (hit / origin
    // enclosed), coincident points (degenerate search direction), enormous and
    // tiny separations (d1 > d0 and the FLT_EPSILON guards), and radii chosen
    // to sit exactly on `dist == rA + rB`.
    let mut saw_iter = [0usize; 21];
    for i in 0..N {
        let (sa, sb, ur): (Shape, Shape, c_int) = match i % 8 {
            0 => {
                let c = g.circle();
                (Shape::Circle(c), Shape::Circle(c), 1) // identical => hit
            }
            1 => {
                let k = g.capsule();
                (Shape::Capsule(k), Shape::Capsule(k), 1)
            }
            2 => {
                let bb = g.aabb();
                (Shape::Aabb(bb), Shape::Aabb(bb), 0)
            }
            3 => {
                // dist == rA + rB exactly: two circles r apart.
                let r = g.below(10) as f32;
                let a = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r,
                };
                let b = c2Circle {
                    p: c2v { x: r + r, y: 0.0 },
                    r,
                };
                (Shape::Circle(a), Shape::Circle(b), 1)
            }
            4 => {
                // separation just under / just over FLT_EPSILON
                let e = [
                    1.192_092_895_507_812_5e-7f32,
                    1.0e-8,
                    1.0e-30,
                    0.0,
                    2.4e-7,
                ][g.below(5) as usize];
                let a = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: 0.0,
                };
                let b = c2Circle {
                    p: c2v { x: e, y: 0.0 },
                    r: 0.0,
                };
                (Shape::Circle(a), Shape::Circle(b), 1)
            }
            5 => {
                let s = 1.0e30f32;
                (
                    Shape::Aabb(c2AABB {
                        min: c2v { x: -s, y: -s },
                        max: c2v { x: s, y: s },
                    }),
                    Shape::Capsule(c2Capsule {
                        a: c2v { x: -s, y: s },
                        b: c2v { x: s, y: -s },
                        r: s,
                    }),
                    1,
                )
            }
            6 => {
                let v = g.vec();
                (
                    Shape::Capsule(c2Capsule { a: v, b: v, r: 0.0 }),
                    Shape::Capsule(c2Capsule { a: v, b: v, r: 0.0 }),
                    1,
                )
            }
            _ => {
                let (a, b) = (g.capsule(), g.aabb());
                (Shape::Capsule(a), Shape::Aabb(b), (i % 2) as c_int)
            }
        };
        let ctx = format!("err22-28 #{i} A={sa:?} B={sb:?} ur={ur}");
        diff(&ctx, &p, |x| {
            let o = GjkOpts {
                use_radius: ur,
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            call_gjk(x, &sa, &sb, &o)
        });
        // Track how deep the iteration counter goes (row 22).
        let o = GjkOpts {
            use_radius: ur,
            ..Default::default()
        };
        let it = call_gjk(&p.c, &sa, &sb, &o).iters.unwrap();
        if (0..=20).contains(&it) {
            saw_iter[it as usize] += 1;
        }
    }
    // The hard cap is 20; whatever depths occur must occur identically in both,
    // which the diff above already established.
    assert!(
        saw_iter.iter().sum::<usize>() > 0,
        "err22: no iteration counts observed"
    );
}

// ---------------------------------------------------------------------------
// Rows 29-31 — division by zero and NaN propagation in c2Div / c2Norm / c2Len
// ---------------------------------------------------------------------------

#[test]
fn err29_30_31_div_norm_len_degenerate() {
    let p = pair();
    let mut g = Rng::new(0x1013);
    let specials = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0xFF80_0001), // negative signalling NaN
        f32::from_bits(0xFFC0_0000), // QNaN indefinite
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        1.0,
        -1.0,
    ];
    for a in specials {
        for b in specials {
            let v = c2v { x: a, y: b };
            let ctx = format!("err29-31 v=({a:e},{b:e})");
            diff(&ctx, &p, |x| unsafe {
                ((x.c2Len)(v), (x.c2Norm)(v))
            });
            for s in specials {
                let ctx = format!("err29-31 div v=({a:e},{b:e}) s={s:e}");
                diff(&ctx, &p, |x| unsafe { (x.c2Div)(v, s) });
            }
        }
    }
    for i in 0..N {
        let v = g.vec_wide();
        let s = g.coord_wide();
        let ctx = format!("err29-31 rand #{i} v={v:?} s={s:e}");
        diff(&ctx, &p, |x| unsafe {
            ((x.c2Len)(v), (x.c2Norm)(v), (x.c2Div)(v, s))
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 32-33 — c2CircletoCapsule degenerate capsule and negative radii
// ---------------------------------------------------------------------------

#[test]
fn err32_33_circle_to_capsule_degenerate() {
    let p = pair();
    let mut g = Rng::new(0x1014);
    for i in 0..N {
        // a == b: `n == (0,0)`, so neither `da < 0` nor `db < 0` holds and the
        // `c2Dot(n,n)` division is never reached.
        let v = g.vec();
        let k = c2Capsule {
            a: v,
            b: v,
            r: if i % 3 == 0 {
                g.range(-10.0, 0.0)
            } else {
                g.radius()
            },
        };
        let c = match i % 4 {
            0 => c2Circle { p: v, r: g.radius() },
            1 => c2Circle {
                p: g.vec_wide(),
                r: g.coord_wide(),
            },
            2 => c2Circle {
                p: v,
                r: g.range(-10.0, 0.0),
            },
            _ => g.circle(),
        };
        let ctx = format!("err32-33 #{i} A={c:?} B={k:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2CircletoCapsule)(c, k) });
    }
}

// ---------------------------------------------------------------------------
// Rows 34-36 — negative radii, inverted AABBs, NaN coordinates
// ---------------------------------------------------------------------------

#[test]
fn err34_circle_to_circle_negative_radius() {
    let p = pair();
    let mut g = Rng::new(0x1015);
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -5.0,
        5.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
    ];
    for ra in radii {
        for rb in radii {
            for i in 0..24 {
                let a = c2Circle {
                    p: if i % 3 == 0 { g.vec_wide() } else { g.vec() },
                    r: ra,
                };
                let b = c2Circle {
                    p: if i % 4 == 0 { g.vec_wide() } else { g.vec() },
                    r: rb,
                };
                let ctx = format!("err34 ra={ra:e} rb={rb:e} #{i}");
                diff(&ctx, &p, |x| unsafe { (x.c2CircletoCircle)(a, b) });
            }
        }
    }
}

#[test]
fn err35_circle_to_aabb_inverted_box() {
    let p = pair();
    let mut g = Rng::new(0x1016);
    for i in 0..N {
        let lo = g.vec();
        let hi = g.vec();
        // Deliberately inverted (min > max) and NaN-cornered boxes.
        let bb = match i % 4 {
            0 => c2AABB {
                min: c2v {
                    x: lo.x.max(hi.x),
                    y: lo.y.max(hi.y),
                },
                max: c2v {
                    x: lo.x.min(hi.x),
                    y: lo.y.min(hi.y),
                },
            },
            1 => c2AABB {
                min: g.vec_wide(),
                max: g.vec_wide(),
            },
            2 => c2AABB { min: lo, max: lo },
            _ => c2AABB { min: lo, max: hi },
        };
        let c = if i % 5 == 0 {
            c2Circle {
                p: g.vec_wide(),
                r: g.coord_wide(),
            }
        } else {
            g.circle()
        };
        let ctx = format!("err35 #{i} A={c:?} B={bb:?}");
        diff(&ctx, &p, |x| unsafe {
            // c2Clampv itself is the mechanism, so check it too.
            ((x.c2CircletoAABB)(c, bb), (x.c2Clampv)(c.p, bb.min, bb.max))
        });
    }
}

#[test]
fn err36_aabb_to_aabb_nan_and_inverted() {
    let p = pair();
    let mut g = Rng::new(0x1017);
    let nan = f32::NAN;
    let fixed = [
        (
            c2AABB {
                min: c2v { x: nan, y: nan },
                max: c2v { x: nan, y: nan },
            },
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            },
        ),
        (
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            },
            c2AABB {
                min: c2v { x: nan, y: nan },
                max: c2v { x: nan, y: nan },
            },
        ),
        (
            c2AABB {
                min: c2v { x: 5.0, y: 5.0 },
                max: c2v { x: 0.0, y: 0.0 },
            },
            c2AABB {
                min: c2v { x: 1.0, y: 1.0 },
                max: c2v { x: 2.0, y: 2.0 },
            },
        ),
    ];
    for (i, (a, b)) in fixed.iter().enumerate() {
        let (a, b) = (*a, *b);
        let ctx = format!("err36 fixed #{i}");
        diff(&ctx, &p, |x| unsafe { (x.c2AABBtoAABB)(a, b) });
    }
    for i in 0..N {
        let a = c2AABB {
            min: g.vec_wide(),
            max: g.vec_wide(),
        };
        let b = c2AABB {
            min: g.vec_wide(),
            max: g.vec_wide(),
        };
        let ctx = format!("err36 #{i} A={a:?} B={b:?}");
        diff(&ctx, &p, |x| unsafe { (x.c2AABBtoAABB)(a, b) });
    }
}

// ---------------------------------------------------------------------------
// Row 37 — `c2GJK(...) != 0` in the GJK-backed boolean wrappers
// ---------------------------------------------------------------------------

#[test]
fn err37_gjk_backed_bools_nan_distance() {
    let p = pair();
    let mut g = Rng::new(0x1018);
    for i in 0..N {
        // NaN / Inf geometry makes the GJK distance NaN, which the C's raw
        // `!= 0` test reads as "no collision".
        let mk_cap = |g: &mut Rng| c2Capsule {
            a: g.vec_wide(),
            b: g.vec_wide(),
            r: g.coord_wide(),
        };
        let mk_bb = |g: &mut Rng| c2AABB {
            min: g.vec_wide(),
            max: g.vec_wide(),
        };
        let bb = mk_bb(&mut g);
        let k1 = mk_cap(&mut g);
        let k2 = mk_cap(&mut g);
        let ctx = format!("err37 #{i} bb={bb:?} k1={k1:?} k2={k2:?}");
        diff(&ctx, &p, |x| unsafe {
            (
                (x.c2AABBtoCapsule)(bb, k1),
                (x.c2CapsuletoCapsule)(k1, k2),
                (x.c2CapsuletoCapsule)(k1, k1),
            )
        });
    }
}

// ---------------------------------------------------------------------------
// Row 38 — the public entry point has no validation whatsoever
// ---------------------------------------------------------------------------

#[test]
fn err38_capsule_entry_point_unvalidated() {
    let p = pair();
    let mut g = Rng::new(0x1019);
    for i in 0..N * 2 {
        let a = g.coord_wide();
        let b = g.coord_wide();
        let c = g.coord_wide();
        let d = g.coord_wide();
        let e = g.coord_wide();
        let ctx = format!("err38 #{i} ({a:e},{b:e},{c:e},{d:e},{e:e})");
        diff(&ctx, &p, |x| unsafe { (x.capsule)(a, b, c, d, e) });
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep: every function that tolerates a NULL, plus one
// step past every documented range.
// ---------------------------------------------------------------------------

#[test]
fn generic_boundary_sweep() {
    let p = pair();
    let mut g = Rng::new(0x101A);

    // Out-of-range enum values one step past the valid range, in both
    // directions, on every entry point that takes a C2_TYPE.
    for ty in [-1i32, 3] {
        diff(&format!("generic collided ty={ty}"), &p, |x| unsafe {
            (x.c2Collided)(core::ptr::null(), ty, core::ptr::null(), ty)
        });
        let seed = c2Proxy {
            radius: 1.25,
            count: 5,
            verts: [c2v { x: 9.0, y: -9.0 }; 8],
        };
        diff(&format!("generic makeproxy ty={ty}"), &p, |x| unsafe {
            let mut pr = seed;
            (x.c2MakeProxy)(core::ptr::null(), ty, &mut pr);
            pr
        });
    }

    // Counts one step past the valid simplex range.
    for count in [0i32, 4] {
        for i in 0..64 {
            let s = g.simplex(count);
            let ctx = format!("generic simplex count={count} #{i}");
            diff(&ctx, &p, |x| unsafe {
                let mut s1 = s;
                let mut s2 = s;
                let mut s3 = s;
                let mut a = c2v { x: 1.0, y: 2.0 };
                let mut b = c2v { x: 3.0, y: 4.0 };
                (x.c2Witness)(&mut s1, &mut a, &mut b);
                (
                    (x.c2GJKSimplexMetric)(&mut s2),
                    (x.c2D)(&mut s3),
                    (x.c2L)(&mut s3),
                    (a, b),
                )
            });
        }
    }

    // Zero and one-past-the-end vertex counts for c2Support.
    let verts = [c2v { x: 1.0, y: 2.0 }; 8];
    for count in [0i32, 1, 8, 9] {
        let d = c2v { x: 1.0, y: 1.0 };
        let ctx = format!("generic support count={count}");
        // count == 9 is still inside `verts` only for 8 slots; use a 16-slot
        // buffer so both sides read the same owned memory.
        let mut big = [c2v::default(); 16];
        big[..8].copy_from_slice(&verts);
        diff(&ctx, &p, |x| unsafe { (x.c2Support)(big.as_ptr(), count, d) });
    }

    // Zero-length / degenerate geometry through every boolean entry point.
    let z = c2v { x: 0.0, y: 0.0 };
    let c0 = c2Circle { p: z, r: 0.0 };
    let b0 = c2AABB { min: z, max: z };
    let k0 = c2Capsule { a: z, b: z, r: 0.0 };
    diff("generic zero geometry", &p, |x| unsafe {
        (
            (
                (x.c2CircletoCircle)(c0, c0),
                (x.c2CircletoAABB)(c0, b0),
                (x.c2CircletoCapsule)(c0, k0),
            ),
            (
                (x.c2AABBtoAABB)(b0, b0),
                (x.c2AABBtoCapsule)(b0, k0),
                (x.c2CapsuletoCapsule)(k0, k0),
            ),
        )
    });
}
