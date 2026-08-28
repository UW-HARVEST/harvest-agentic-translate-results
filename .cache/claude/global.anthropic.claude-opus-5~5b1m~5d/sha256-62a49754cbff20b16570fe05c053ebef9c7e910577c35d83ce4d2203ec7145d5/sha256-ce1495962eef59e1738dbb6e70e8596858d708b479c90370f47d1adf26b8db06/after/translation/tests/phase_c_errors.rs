//! Phase C — error-path differential tests, one per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input / rejection condition the C
//! source checks for, calls BOTH `.so`s through their exported symbols, and
//! asserts they return the SAME sentinel (not merely "both failed").

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(align(4))]
struct Buf([u8; 20]);

// ===========================================================================
// Rows 1..=4 — c2Collided: out-of-range C2_TYPE at both positions
// ===========================================================================

#[test]
fn err_row01_collided_invalid_type_a() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0300);
        for &bad in INVALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..200 {
                    let sb = Shape::random(&mut rng, tb, 10.0);
                    // A is a real, fully-initialised buffer; the C must never
                    // dereference it because the outer switch rejects `bad`.
                    let sa = Shape::random(&mut rng, C2_TYPE_CIRCLE, 10.0);
                    let ba = Buf(sa.bytes());
                    let bb = Buf(sb.bytes());
                    let (cv, rv) = unsafe {
                        (
                            (c.c2Collided)(
                                ba.0.as_ptr() as *const c_void,
                                bad,
                                bb.0.as_ptr() as *const c_void,
                                tb,
                            ),
                            (r.c2Collided)(
                                ba.0.as_ptr() as *const c_void,
                                bad,
                                bb.0.as_ptr() as *const c_void,
                                tb,
                            ),
                        )
                    };
                    assert_eq!(
                        cv, rv,
                        "{label} err1 #{i} typeA={bad} typeB={tb}: C {cv} vs R {rv}"
                    );
                    assert_eq!(cv, 0, "{label} err1: C must reject typeA={bad} with 0");
                }
            }
        }
    });
}

#[test]
fn err_row02to04_collided_invalid_type_b() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0301);
        for &ta in VALID_TYPES.iter() {
            for &bad in INVALID_TYPES.iter() {
                for i in 0..200 {
                    let sa = Shape::random(&mut rng, ta, 10.0);
                    let sb = Shape::random(&mut rng, C2_TYPE_AABB, 10.0);
                    let ba = Buf(sa.bytes());
                    let bb = Buf(sb.bytes());
                    let (cv, rv) = unsafe {
                        (
                            (c.c2Collided)(
                                ba.0.as_ptr() as *const c_void,
                                ta,
                                bb.0.as_ptr() as *const c_void,
                                bad,
                            ),
                            (r.c2Collided)(
                                ba.0.as_ptr() as *const c_void,
                                ta,
                                bb.0.as_ptr() as *const c_void,
                                bad,
                            ),
                        )
                    };
                    assert_eq!(
                        cv, rv,
                        "{label} err2-4 #{i} typeA={ta} typeB={bad}: C {cv} vs R {rv}"
                    );
                    assert_eq!(cv, 0, "{label} err2-4: C must reject typeB={bad} with 0");
                }
            }
        }
    });
}

// ===========================================================================
// Row 5 — NULL shape pointers (documented, deliberately NOT exercised)
// ===========================================================================

#[test]
fn err_row05_null_shape_pointers_documented() {
    // `c2Collided` dereferences `A`/`B` unconditionally once the type is valid
    // (`*(c2Circle *)A`), so a NULL pointer is an immediate SIGSEGV in BOTH
    // implementations. There is no return value to compare, so this row is
    // documented rather than executed. What IS asserted is that the type
    // rejection happens *before* any dereference: a NULL pointer paired with an
    // out-of-range type must still return 0 rather than crash.
    for_each_pair(|c, r, label| {
        for &bad in INVALID_TYPES.iter() {
            let (cv, rv) = unsafe {
                (
                    (c.c2Collided)(std::ptr::null(), bad, std::ptr::null(), bad),
                    (r.c2Collided)(std::ptr::null(), bad, std::ptr::null(), bad),
                )
            };
            assert_eq!(cv, rv, "{label} err5 typeA=typeB={bad}");
            assert_eq!(cv, 0, "{label} err5: must reject before dereferencing NULL");
        }
        // Valid typeA + invalid typeB: A is dereferenced?  No — the inner
        // switch's `default:` returns before `*(c2Circle *)A` is evaluated.
        for &ta in VALID_TYPES.iter() {
            let (cv, rv) = unsafe {
                (
                    (c.c2Collided)(std::ptr::null(), ta, std::ptr::null(), 12345),
                    (r.c2Collided)(std::ptr::null(), ta, std::ptr::null(), 12345),
                )
            };
            assert_eq!(cv, rv, "{label} err5 typeA={ta} typeB=12345");
            assert_eq!(cv, 0, "{label} err5: inner default must return before deref");
        }
    });
}

// ===========================================================================
// Row 6 — ptr_from_parts with an unrecognised type (falls off the end)
// ===========================================================================

#[test]
fn err_row06_ptr_from_parts_invalid_type_documented() {
    // `ptr_from_parts` has no `return` for an unrecognised `typ`: control reaches
    // the closing brace of a non-`void` function, so the returned value is
    // indeterminate (whatever happens to be in `rax`). There is nothing
    // deterministic to compare, so the value is NOT asserted. The Rust returns
    // NULL. What IS asserted (rows 7/8) is the only observable consequence:
    // `omni_collide` never dereferences the result for an invalid type.
    //
    // Calling it is still useful as a "does not crash / does not allocate"
    // smoke check.
    for_each_pair(|c, r, label| {
        for &bad in INVALID_TYPES.iter() {
            unsafe {
                let _cp = (c.ptr_from_parts)(bad, 1.0, 2.0, 3.0, 4.0, 5.0);
                let rp = (r.ptr_from_parts)(bad, 1.0, 2.0, 3.0, 4.0, 5.0);
                assert!(
                    rp.is_null(),
                    "{label} err6: Rust should produce NULL for typ={bad}"
                );
                // `_cp` is indeterminate — never dereferenced, never freed.
            }
        }
        // And the valid types must produce a non-NULL, freeable pointer in both.
        for &ty in VALID_TYPES.iter() {
            unsafe {
                let cp = (c.ptr_from_parts)(ty, 1.0, 2.0, 3.0, 4.0, 5.0);
                let rp = (r.ptr_from_parts)(ty, 1.0, 2.0, 3.0, 4.0, 5.0);
                assert!(!cp.is_null() && !rp.is_null(), "{label} err6 ty={ty}");
                free(cp);
                free(rp);
            }
        }
    });
}

// ===========================================================================
// Rows 7 / 8 — omni_collide with out-of-range types
// ===========================================================================

#[test]
fn err_row07_omni_invalid_type_a() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0302);
        for &bad in INVALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..200 {
                    let pb = Shape::random(&mut rng, tb, 10.0).parts();
                    let pa = [
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                    ];
                    let cv = unsafe {
                        (c.omni_collide)(
                            bad, pa[0], pa[1], pa[2], pa[3], pa[4], tb, pb[0], pb[1], pb[2],
                            pb[3], pb[4],
                        )
                    };
                    let rv = unsafe {
                        (r.omni_collide)(
                            bad, pa[0], pa[1], pa[2], pa[3], pa[4], tb, pb[0], pb[1], pb[2],
                            pb[3], pb[4],
                        )
                    };
                    assert_eq!(cv, rv, "{label} err7 #{i} type_a={bad} type_b={tb}");
                    assert_eq!(cv, 0, "{label} err7: must reject type_a={bad}");
                }
            }
        }
    });
}

#[test]
fn err_row08_omni_invalid_type_b() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0303);
        for &ta in VALID_TYPES.iter() {
            for &bad in INVALID_TYPES.iter() {
                for i in 0..200 {
                    let pa = Shape::random(&mut rng, ta, 10.0).parts();
                    let pb = [
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                        rng.ordinary(10.0),
                    ];
                    let cv = unsafe {
                        (c.omni_collide)(
                            ta, pa[0], pa[1], pa[2], pa[3], pa[4], bad, pb[0], pb[1], pb[2],
                            pb[3], pb[4],
                        )
                    };
                    let rv = unsafe {
                        (r.omni_collide)(
                            ta, pa[0], pa[1], pa[2], pa[3], pa[4], bad, pb[0], pb[1], pb[2],
                            pb[3], pb[4],
                        )
                    };
                    assert_eq!(cv, rv, "{label} err8 #{i} type_a={ta} type_b={bad}");
                    assert_eq!(cv, 0, "{label} err8: must reject type_b={bad}");
                }
            }
        }
        // Both invalid.
        for &bad in INVALID_TYPES.iter() {
            let cv = unsafe {
                (c.omni_collide)(bad, 1.0, 2.0, 3.0, 4.0, 5.0, bad, 6.0, 7.0, 8.0, 9.0, 10.0)
            };
            let rv = unsafe {
                (r.omni_collide)(bad, 1.0, 2.0, 3.0, 4.0, 5.0, bad, 6.0, 7.0, 8.0, 9.0, 10.0)
            };
            assert_eq!(cv, rv, "{label} err8 both={bad}");
            assert_eq!(cv, 0);
        }
    });
}

// ===========================================================================
// Row 9 — c2MakeProxy with an unrecognised type leaves *p untouched
// ===========================================================================

#[test]
fn err_row09_makeproxy_invalid_type_leaves_proxy_untouched() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0304);
        for &bad in INVALID_TYPES.iter() {
            for i in 0..100 {
                let sa = Shape::random(&mut rng, C2_TYPE_CAPSULE, 10.0);
                let buf = Buf(sa.bytes());
                // Distinctive pattern so an untouched proxy is identifiable.
                let mut base = c2Proxy {
                    radius: f32::from_bits(0x0BAD_F00D),
                    count: 0x7EED_1234,
                    verts: [c2v::default(); 8],
                };
                for (k, v) in base.verts.iter_mut().enumerate() {
                    v.x = f32::from_bits(0x1111_0000 + k as u32);
                    v.y = f32::from_bits(0x2222_0000 + k as u32);
                }
                let mut cp = base;
                let mut rp = base;
                unsafe {
                    (c.c2MakeProxy)(buf.0.as_ptr() as *const c_void, bad, &mut cp);
                    (r.c2MakeProxy)(buf.0.as_ptr() as *const c_void, bad, &mut rp);
                }
                assert!(
                    proxy_same(&cp, &rp),
                    "{label} err9 #{i} bad={bad}:\n  C: {}\n  R: {}",
                    fmt_proxy(&cp),
                    fmt_proxy(&rp)
                );
                assert!(
                    proxy_same(&cp, &base),
                    "{label} err9: the C must leave the proxy completely untouched"
                );
            }
        }
    });
}

// ===========================================================================
// Rows 10..=18, 26 — c2GJK NULL handling and use_radius
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn gjk(
    api: &Api,
    a: &Shape,
    b: &Shape,
    ax: Option<&c2x>,
    bx: Option<&c2x>,
    want_a: bool,
    want_b: bool,
    want_it: bool,
    use_radius: c_int,
    cache: Option<&mut c2GJKCache>,
) -> (f32, c2v, c2v, c_int) {
    let ba = Buf(a.bytes());
    let bb = Buf(b.bytes());
    let poison = c2v { x: -8.5e-11, y: 6.25e17 };
    let mut oa = poison;
    let mut ob = poison;
    let mut it: c_int = -0x0BAD;
    let dist = unsafe {
        (api.c2GJK)(
            ba.0.as_ptr() as *const c_void,
            a.ty(),
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            bb.0.as_ptr() as *const c_void,
            b.ty(),
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            if want_a { &mut oa } else { std::ptr::null_mut() },
            if want_b { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if want_it { &mut it } else { std::ptr::null_mut() },
            cache.map_or(std::ptr::null_mut(), |k| k as *mut c2GJKCache),
        )
    };
    (dist, oa, ob, it)
}

#[test]
fn err_row10to12_gjk_null_transforms_and_cache() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0305);
        let ident = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        };
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..150 {
                    let sa = Shape::random(&mut rng, ta, 10.0);
                    let sb = Shape::random(&mut rng, tb, 10.0);
                    // Row 10/11: NULL ax/bx must behave exactly like identity.
                    for (ax, bx) in [
                        (None, None),
                        (Some(&ident), None),
                        (None, Some(&ident)),
                        (Some(&ident), Some(&ident)),
                    ] {
                        let co = gjk(c, &sa, &sb, ax, bx, true, true, true, 1, None);
                        let ro = gjk(r, &sa, &sb, ax, bx, true, true, true, 1, None);
                        assert!(
                            f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                                && co.3 == ro.3,
                            "{label} err10-12 #{i} ta={ta} tb={tb} ax={} bx={}:\n  C dist={} a={} b={} it={}\n  R dist={} a={} b={} it={}",
                            ax.is_some(),
                            bx.is_some(),
                            fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2), co.3,
                            fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2), ro.3,
                        );
                        // The C's own NULL == identity guarantee.
                        let null_both = gjk(c, &sa, &sb, None, None, true, true, true, 1, None);
                        let ident_both =
                            gjk(c, &sa, &sb, Some(&ident), Some(&ident), true, true, true, 1, None);
                        assert!(
                            f32_same(null_both.0, ident_both.0),
                            "{label} err10-12: NULL transform != identity in the C itself"
                        );
                    }
                }
            }
        }
    });
}

#[test]
fn err_row13_gjk_cold_cache_count_zero() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0306);
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..200 {
                    let sa = Shape::random(&mut rng, ta, 10.0);
                    let sb = Shape::random(&mut rng, tb, 10.0);
                    // count == 0 with *garbage* in every other field: the C must
                    // ignore the whole cache because `cache_was_good` is false.
                    let seed_cache = c2GJKCache {
                        metric: f32::from_bits(0xDEAD_BEEF),
                        count: 0,
                        iA: [7, -3, 99],
                        iB: [-1, 5, 12345],
                        div: f32::from_bits(0xFEED_FACE),
                    };
                    let mut ck = seed_cache;
                    let mut rk = seed_cache;
                    let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
                    let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
                    assert!(
                        f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                            && co.3 == ro.3,
                        "{label} err13 #{i} ta={ta} tb={tb}: dist C {} vs R {}",
                        fmt_f32(co.0),
                        fmt_f32(ro.0)
                    );
                    assert!(
                        cache_same(&ck, &rk),
                        "{label} err13 #{i}: cache write-back\n  C: {}\n  R: {}",
                        fmt_cache(&ck),
                        fmt_cache(&rk)
                    );
                    // A cold cache must have been *written*, i.e. count != 0.
                    assert_ne!(ck.count, 0, "{label} err13: cache not written back");
                }
            }
        }
    });
}

#[test]
fn err_row14_gjk_warm_cache_is_read() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0307);
        let mut reused = 0usize;
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..200 {
                    let sa = Shape::random(&mut rng, ta, 10.0);
                    let sb = Shape::random(&mut rng, tb, 10.0);
                    let mut ck = c2GJKCache::default();
                    let mut rk = c2GJKCache::default();
                    // First call primes the cache.
                    let _ = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
                    let _ = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
                    assert!(cache_same(&ck, &rk), "{label} err14 #{i}: primed cache differs");
                    if ck.count != 0 {
                        reused += 1;
                    }
                    // Second call reads it (`cache_was_read == 1`, because the
                    // `metric < -1.0e8f` half of the L400 test is essentially
                    // never true).
                    let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
                    let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
                    assert!(
                        f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                            && co.3 == ro.3,
                        "{label} err14 #{i} ta={ta} tb={tb}: dist C {} vs R {}, it C {} vs R {}",
                        fmt_f32(co.0),
                        fmt_f32(ro.0),
                        co.3,
                        ro.3
                    );
                    assert!(cache_same(&ck, &rk), "{label} err14 #{i}: cache after reuse differs");
                }
            }
        }
        assert!(reused > 0, "{label} err14: the warm-cache path was never entered");
        println!("{label} err14: warm cache entries={reused}");
    });
}

#[test]
fn err_row15_gjk_cache_metric_test_fails() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0308);
        // The L400 test `min_metric < max_metric * 2.0f && metric < -1.0e8f`
        // can only hold when the *reloaded* simplex metric is below -1e8, which
        // needs count == 3 and a hugely negative determinant. Feed exactly that:
        // a primed 3-vertex cache from enormous shapes.
        for i in 0..600 {
            let big = 1.0e20f32 * if rng.bool() { 1.0 } else { -1.0 };
            let sa = Shape::Aabb(c2AABB {
                min: c2v { x: -big.abs(), y: -big.abs() },
                max: c2v { x: big.abs(), y: big.abs() },
            });
            let sb = Shape::Aabb(c2AABB {
                min: c2v { x: -big.abs() * 0.5, y: -big.abs() * 0.5 },
                max: c2v { x: big.abs() * 0.5, y: big.abs() * 0.5 },
            });
            let mut ck = c2GJKCache::default();
            let mut rk = c2GJKCache::default();
            let _ = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
            let _ = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
            assert!(cache_same(&ck, &rk), "{label} err15 #{i}: primed cache differs");
            // Now poison `metric` / `div` in the cache directly and re-call, so
            // both the "test holds" and "test fails" sides of L400 are visited.
            for (m, d) in [
                (-1.0e30f32, ck.div),
                (-1.0e9f32, 1.0f32),
                (0.0f32, 0.0f32),
                (f32::MAX, ck.div),
                (f32::NEG_INFINITY, ck.div),
            ] {
                let mut ck2 = ck;
                let mut rk2 = rk;
                ck2.metric = m;
                ck2.div = d;
                rk2.metric = m;
                rk2.div = d;
                let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck2));
                let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk2));
                assert!(
                    f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                        && co.3 == ro.3,
                    "{label} err15 #{i} metric={} div={}: dist C {} vs R {}",
                    fmt_f32(m),
                    fmt_f32(d),
                    fmt_f32(co.0),
                    fmt_f32(ro.0)
                );
                assert!(
                    cache_same(&ck2, &rk2),
                    "{label} err15 #{i} metric={} div={}: cache\n  C: {}\n  R: {}",
                    fmt_f32(m),
                    fmt_f32(d),
                    fmt_cache(&ck2),
                    fmt_cache(&rk2)
                );
            }
        }
    });
}

#[test]
fn err_row16to18_gjk_null_out_params() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0309);
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..120 {
                    let sa = Shape::random(&mut rng, ta, 10.0);
                    let sb = Shape::random(&mut rng, tb, 10.0);
                    for (wa, wb, wi) in [
                        (false, true, true),
                        (true, false, true),
                        (true, true, false),
                        (false, false, false),
                        (true, true, true),
                    ] {
                        let co = gjk(c, &sa, &sb, None, None, wa, wb, wi, 1, None);
                        let ro = gjk(r, &sa, &sb, None, None, wa, wb, wi, 1, None);
                        assert!(
                            f32_same(co.0, ro.0),
                            "{label} err16-18 #{i} ({wa},{wb},{wi}): dist C {} vs R {}",
                            fmt_f32(co.0),
                            fmt_f32(ro.0)
                        );
                        // Skipped writes must leave the poison in place -- and
                        // identically so in both implementations.
                        assert!(v_same(co.1, ro.1), "{label} err16 outA");
                        assert!(v_same(co.2, ro.2), "{label} err17 outB");
                        assert_eq!(co.3, ro.3, "{label} err18 iterations");
                        if !wa {
                            assert!(
                                v_same(co.1, c2v { x: -8.5e-11, y: 6.25e17 }),
                                "{label} err16: C wrote through a NULL outA"
                            );
                        }
                        if !wi {
                            assert_eq!(co.3, -0x0BAD, "{label} err18: C wrote through NULL iterations");
                        }
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Rows 19..=26 — c2GJK internal early-outs, hit path, radius branches
// ===========================================================================

#[test]
fn err_row19to26_gjk_internal_earlyouts() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x030A);
        let mut iter_hist = [0usize; 22];
        let mut zero_dist_radius = 0usize;
        let mut zero_dist_noradius = 0usize;
        let mut nonzero = 0usize;
        // A wide sweep whose only purpose is to visit as many of the internal
        // break conditions as possible; every visit is compared.
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..900 {
                    let (sa, sb) = match rng.below(6) {
                        0 => (
                            Shape::random(&mut rng, ta, 1.0),
                            Shape::random(&mut rng, tb, 1.0),
                        ),
                        1 => (
                            Shape::random(&mut rng, ta, 1.0e6),
                            Shape::random(&mut rng, tb, 1.0e6),
                        ),
                        2 => (
                            Shape::random_degenerate(&mut rng, ta, 10.0),
                            Shape::random_degenerate(&mut rng, tb, 10.0),
                        ),
                        3 => (
                            Shape::random_extreme(&mut rng, ta),
                            Shape::random_extreme(&mut rng, tb),
                        ),
                        4 => {
                            // identical shapes -> witness points coincide
                            let s = Shape::random(&mut rng, ta, 10.0);
                            (s, s)
                        }
                        _ => (
                            Shape::random(&mut rng, ta, 1.0e-6),
                            Shape::random(&mut rng, tb, 1.0e-6),
                        ),
                    };
                    for ur in [0, 1] {
                        let co = gjk(c, &sa, &sb, None, None, true, true, true, ur, None);
                        let ro = gjk(r, &sa, &sb, None, None, true, true, true, ur, None);
                        assert!(
                            f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                                && co.3 == ro.3,
                            "{label} err19-26 #{i} ta={ta} tb={tb} ur={ur}\n  A={sa:?}\n  B={sb:?}\n  C dist={} a={} b={} it={}\n  R dist={} a={} b={} it={}",
                            fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2), co.3,
                            fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2), ro.3,
                        );
                        if (0..=20).contains(&co.3) {
                            iter_hist[co.3 as usize] += 1;
                        } else {
                            iter_hist[21] += 1;
                        }
                        if co.0 == 0.0 {
                            if ur == 0 {
                                zero_dist_noradius += 1;
                            } else {
                                zero_dist_radius += 1;
                            }
                        } else {
                            nonzero += 1;
                        }
                    }
                }
            }
        }
        println!(
            "{label} err19-26: iter histogram {iter_hist:?}\n  zero(ur=1)={zero_dist_radius} zero(ur=0)={zero_dist_noradius} nonzero={nonzero}"
        );
        // Row 23: the `hit` path (s.count == 3) is the only way `use_radius == 0`
        // can produce dist == 0 for non-coincident shapes, so this proves it was
        // reached. Rows 24/25 need `use_radius == 1` overlap.
        assert!(zero_dist_noradius > 0, "{label} row23: the hit path was never reached");
        assert!(zero_dist_radius > 0, "{label} row24: the radius-overlap path was never reached");
        assert!(nonzero > 0, "{label} row26: the separated path was never reached");
        // Rows 19..=22: multiple distinct iteration counts prove the loop exits
        // via several different break conditions.
        let distinct = iter_hist.iter().filter(|&&n| n > 0).count();
        assert!(
            distinct >= 3,
            "{label} rows19-22: only {distinct} distinct iteration counts observed: {iter_hist:?}"
        );
    });
}

#[test]
fn err_row25_gjk_radius_shrink_collapses_to_zero() {
    // Row 25: separated shapes whose witness points become *equal* after being
    // pulled in by rA / rB, so the C sets dist = 0 even though the `dist > rA+rB`
    // test passed. Reachable when rA + rB is a hair below `dist`.
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x030B);
        for i in 0..2000 {
            let d = 1.0 + rng.unit().abs() * 100.0;
            // radii summing to just under (or just over) the gap
            let eps = match rng.below(4) {
                0 => 0.0,
                1 => f32::EPSILON * d,
                2 => -f32::EPSILON * d,
                _ => rng.unit() * 1.0e-5 * d,
            };
            let ra = (d - eps) * 0.5;
            let rb = d - eps - ra;
            let sa = Shape::Circle(c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: ra,
            });
            let sb = Shape::Circle(c2Circle {
                p: c2v { x: d, y: 0.0 },
                r: rb,
            });
            let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, None);
            let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, None);
            assert!(
                f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2) && co.3 == ro.3,
                "{label} err25 #{i} d={} ra={} rb={}:\n  C dist={} a={} b={}\n  R dist={} a={} b={}",
                fmt_f32(d), fmt_f32(ra), fmt_f32(rb),
                fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2),
                fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2),
            );
        }
        // Also the `dist > FLT_EPSILON` half of the L480 test: shapes almost
        // exactly touching, with zero radii.
        for k in 0..2000 {
            let d = (k as f32) * FLT_EPSILON * 0.5;
            let sa = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 });
            let sb = Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: 0.0 });
            let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, None);
            let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, None);
            assert!(
                f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2),
                "{label} err25 eps #{k} d={}: C dist={} vs R dist={}",
                fmt_f32(d),
                fmt_f32(co.0),
                fmt_f32(ro.0)
            );
        }
    });
}

// ===========================================================================
// Row 27 / 35 / 62 — indeterminate-in-C conditions (documented, not asserted)
// ===========================================================================

#[test]
fn err_row27_35_62_indeterminate_documented() {
    // Row 27: `cache->iA[i]` >= `pA.count` makes the C read a `c2Proxy.verts[]`
    //         slot that `c2MakeProxy` never initialised (the C's `c2Proxy` is an
    //         uninitialised automatic).
    // Row 35: `c2Support` with a `count` larger than the real array reads past
    //         the end of the caller's buffer.
    // Row 62 (c2GJK part): an out-of-range `C2_TYPE` makes `c2MakeProxy` leave
    //         the uninitialised `c2Proxy` untouched, after which the C reads
    //         `pA.count` garbage and `c2Support` loops over it -- an out-of-
    //         bounds read that can and does segfault.
    //
    // All three are undefined behaviour in the C, so there is no defined result
    // for the Rust to match and no value is asserted. They are recorded here so
    // the row is accounted for. What IS asserted is that the *reachable* part
    // of the surface behaves identically:
    for_each_pair(|c, r, label| {
        // In-range cache indices only -- the defined half of row 27.
        let sa = Shape::Aabb(c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        });
        let sb = Shape::Aabb(c2AABB {
            min: c2v { x: 3.0, y: 3.0 },
            max: c2v { x: 5.0, y: 5.0 },
        });
        for count in 1..=3 {
            for i0 in 0..4 {
                for i1 in 0..4 {
                    let mut ck = c2GJKCache {
                        metric: 0.0,
                        count,
                        iA: [i0, i1, 0],
                        iB: [i1, i0, 0],
                        div: 1.0,
                    };
                    let mut rk = ck;
                    let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
                    let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
                    assert!(
                        f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                            && co.3 == ro.3,
                        "{label} err27 count={count} iA=[{i0},{i1}]: C dist={} vs R dist={}",
                        fmt_f32(co.0),
                        fmt_f32(ro.0)
                    );
                    assert!(cache_same(&ck, &rk), "{label} err27: cache differs");
                }
            }
        }
        // `c2Support` with an exact, in-range count -- the defined half of row 35.
        let verts = [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: -1.0 },
        ];
        for n in 1..=4 {
            let d = c2v { x: 0.3, y: -0.7 };
            let ci = unsafe { (c.c2Support)(verts.as_ptr(), n, d) };
            let ri = unsafe { (r.c2Support)(verts.as_ptr(), n, d) };
            assert_eq!(ci, ri, "{label} err35 n={n}");
        }
    });
}

// ===========================================================================
// Row 28 — c2GJKSimplexMetric default branch
// ===========================================================================

#[test]
fn err_row28_simplex_metric_out_of_range_count() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x030C);
        for &count in &[0, 1, 4, 5, -1, -2, 1000, c_int::MAX, c_int::MIN] {
            for i in 0..200 {
                let mut cs = c2Simplex {
                    verts: [c2sv::default(); 4],
                    div: rng.ordinary(10.0),
                    count,
                };
                for v in cs.verts.iter_mut() {
                    v.p = rng.v_ordinary(50.0);
                    v.sA = rng.v_ordinary(50.0);
                    v.sB = rng.v_ordinary(50.0);
                    v.u = rng.ordinary(10.0);
                }
                let mut rs = cs;
                let cm = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
                let rm = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
                assert!(
                    f32_same(cm, rm),
                    "{label} err28 #{i} count={count}: C {} vs R {}",
                    fmt_f32(cm),
                    fmt_f32(rm)
                );
                assert!(
                    f32_same(cm, 0.0),
                    "{label} err28: count={count} must give exactly 0, got {}",
                    fmt_f32(cm)
                );
                assert!(simplex_same(&cs, &rs), "{label} err28: simplex mutated");
            }
        }
    });
}

// ===========================================================================
// Row 29 — c2D default branch
// ===========================================================================

#[test]
fn err_row29_c2d_out_of_range_count() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x030D);
        for &count in &[3, 0, 4, 5, -1, 1000, c_int::MAX, c_int::MIN] {
            for i in 0..200 {
                let mut cs = c2Simplex {
                    verts: [c2sv::default(); 4],
                    div: rng.ordinary(10.0),
                    count,
                };
                for v in cs.verts.iter_mut() {
                    v.p = rng.v_ordinary(50.0);
                }
                let mut rs = cs;
                let cd = unsafe { (c.c2D)(&mut cs) };
                let rd = unsafe { (r.c2D)(&mut rs) };
                assert!(
                    v_same(cd, rd),
                    "{label} err29 #{i} count={count}: C {} vs R {}",
                    fmt_v(cd),
                    fmt_v(rd)
                );
                assert!(
                    f32_same(cd.x, 0.0) && f32_same(cd.y, 0.0),
                    "{label} err29: count={count} must give (0,0), got {}",
                    fmt_v(cd)
                );
            }
        }
    });
}

// ===========================================================================
// Rows 30 / 31 — c2Witness default branch, and div == 0
// ===========================================================================

#[test]
fn err_row30_witness_out_of_range_count() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x030E);
        for &count in &[0, 4, 5, -1, 1000, c_int::MAX, c_int::MIN] {
            for i in 0..200 {
                let mut cs = c2Simplex {
                    verts: [c2sv::default(); 4],
                    div: rng.ordinary(10.0),
                    count,
                };
                for v in cs.verts.iter_mut() {
                    v.sA = rng.v_ordinary(50.0);
                    v.sB = rng.v_ordinary(50.0);
                    v.u = rng.ordinary(10.0);
                }
                let mut rs = cs;
                let poison = c2v { x: 1.5e-9, y: -2.5e11 };
                let (mut ca, mut cb) = (poison, poison);
                let (mut ra, mut rb) = (poison, poison);
                unsafe {
                    (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                    (r.c2Witness)(&mut rs, &mut ra, &mut rb);
                }
                assert!(
                    v_same(ca, ra) && v_same(cb, rb),
                    "{label} err30 #{i} count={count}: C a={} b={} vs R a={} b={}",
                    fmt_v(ca),
                    fmt_v(cb),
                    fmt_v(ra),
                    fmt_v(rb)
                );
                assert!(
                    f32_same(ca.x, 0.0) && f32_same(ca.y, 0.0) && f32_same(cb.x, 0.0)
                        && f32_same(cb.y, 0.0),
                    "{label} err30: count={count} must give (0,0)/(0,0)"
                );
            }
        }
    });
}

#[test]
fn err_row31_witness_zero_div() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x030F);
        for &div in &[0.0f32, -0.0f32, f32::MIN_POSITIVE, -f32::MIN_POSITIVE, 1e-45] {
            for count in [1, 2, 3] {
                for i in 0..300 {
                    let mut cs = c2Simplex {
                        verts: [c2sv::default(); 4],
                        div,
                        count,
                    };
                    for v in cs.verts.iter_mut() {
                        v.sA = rng.v_ordinary(50.0);
                        v.sB = rng.v_ordinary(50.0);
                        v.u = match rng.below(4) {
                            0 => 0.0,
                            1 => -0.0,
                            _ => rng.ordinary(10.0),
                        };
                    }
                    let mut rs = cs;
                    let poison = c2v { x: 1.5e-9, y: -2.5e11 };
                    let (mut ca, mut cb) = (poison, poison);
                    let (mut ra, mut rb) = (poison, poison);
                    unsafe {
                        (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                        (r.c2Witness)(&mut rs, &mut ra, &mut rb);
                    }
                    assert!(
                        v_same(ca, ra) && v_same(cb, rb),
                        "{label} err31 #{i} div={} count={count}: C a={} b={} vs R a={} b={}",
                        fmt_f32(div),
                        fmt_v(ca),
                        fmt_v(cb),
                        fmt_v(ra),
                        fmt_v(rb)
                    );
                }
            }
        }
    });
}

// ===========================================================================
// Rows 32 / 33 — c2L default branch, and div == 0
// ===========================================================================

#[test]
fn err_row32_c2l_out_of_range_count() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0310);
        for &count in &[3, 0, 4, 5, -1, 1000, c_int::MAX, c_int::MIN] {
            for i in 0..200 {
                let mut cs = c2Simplex {
                    verts: [c2sv::default(); 4],
                    div: rng.ordinary(10.0),
                    count,
                };
                for v in cs.verts.iter_mut() {
                    v.p = rng.v_ordinary(50.0);
                    v.u = rng.ordinary(10.0);
                }
                let mut rs = cs;
                let cd = unsafe { (c.c2L)(&mut cs) };
                let rd = unsafe { (r.c2L)(&mut rs) };
                assert!(
                    v_same(cd, rd),
                    "{label} err32 #{i} count={count}: C {} vs R {}",
                    fmt_v(cd),
                    fmt_v(rd)
                );
                assert!(
                    f32_same(cd.x, 0.0) && f32_same(cd.y, 0.0),
                    "{label} err32: count={count} must give (0,0), got {}",
                    fmt_v(cd)
                );
            }
        }
    });
}

#[test]
fn err_row33_c2l_zero_div() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0311);
        for &div in &[0.0f32, -0.0f32, f32::MIN_POSITIVE, 1e-45, f32::INFINITY] {
            for count in [1, 2] {
                for i in 0..300 {
                    let mut cs = c2Simplex {
                        verts: [c2sv::default(); 4],
                        div,
                        count,
                    };
                    for v in cs.verts.iter_mut() {
                        v.p = rng.v_ordinary(50.0);
                        v.u = match rng.below(4) {
                            0 => 0.0,
                            1 => -0.0,
                            _ => rng.ordinary(10.0),
                        };
                    }
                    let mut rs = cs;
                    let cd = unsafe { (c.c2L)(&mut cs) };
                    let rd = unsafe { (r.c2L)(&mut rs) };
                    assert!(
                        v_same(cd, rd),
                        "{label} err33 #{i} div={} count={count}: C {} vs R {}",
                        fmt_f32(div),
                        fmt_v(cd),
                        fmt_v(rd)
                    );
                }
            }
        }
    });
}

// ===========================================================================
// Row 34 — c2Support with count <= 1
// ===========================================================================

#[test]
fn err_row34_support_nonpositive_count() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0312);
        for &count in &[1, 0, -1, -2, -1000, c_int::MIN] {
            for i in 0..300 {
                // `verts[0]` IS dereferenced even for count == 0, so a real
                // one-element buffer must exist.
                let verts = [
                    rng.v_special(),
                    rng.v_special(),
                    rng.v_special(),
                    rng.v_special(),
                ];
                let d = rng.v_special();
                let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
                let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
                assert_eq!(
                    ci, ri,
                    "{label} err34 #{i} count={count} d={}: C {ci} vs R {ri}",
                    fmt_v(d)
                );
                assert_eq!(ci, 0, "{label} err34: count={count} must return 0");
            }
        }
    });
}

// ===========================================================================
// Rows 36..=39 — c2Div / c2Norm / c2Len divisions and NaN propagation
// ===========================================================================

#[test]
fn err_row36to38_div_and_norm_by_zero() {
    for_each_pair(|c, r, label| {
        let vecs = [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 2.0 },
            c2v { x: f32::MAX, y: f32::MIN },
            c2v { x: f32::MIN_POSITIVE, y: 1e-45 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ];
        // Row 36 / 37: c2Div with +0.0 and -0.0 (and inf).
        for &b in &[0.0f32, -0.0f32, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE] {
            for &a in vecs.iter() {
                let cd = (c.c2Div)(a, b);
                let rd = (r.c2Div)(a, b);
                assert!(
                    v_same(cd, rd),
                    "{label} err36-37 c2Div a={} b={}: C {} vs R {}",
                    fmt_v(a),
                    fmt_f32(b),
                    fmt_v(cd),
                    fmt_v(rd)
                );
            }
        }
        // Row 38: c2Norm of the zero vector.
        for &a in vecs.iter() {
            let cn = (c.c2Norm)(a);
            let rn = (r.c2Norm)(a);
            assert!(
                v_same(cn, rn),
                "{label} err38 c2Norm a={}: C {} vs R {}",
                fmt_v(a),
                fmt_v(cn),
                fmt_v(rn)
            );
        }
        assert!(
            (c.c2Norm)(c2v { x: 0.0, y: 0.0 }).x.is_nan(),
            "{label} err38: c2Norm((0,0)) should be NaN in the C"
        );
    });
}

#[test]
fn err_row39_len_inf_nan() {
    for_each_pair(|c, r, label| {
        let vecs = [
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: f32::NEG_INFINITY, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: 0.0, y: f32::NAN },
            c2v { x: f32::NAN, y: f32::NAN },
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: 1e20, y: 1e20 },
            c2v { x: 1e-45, y: 1e-45 },
        ];
        for &a in vecs.iter() {
            let cl = (c.c2Len)(a);
            let rl = (r.c2Len)(a);
            assert!(
                f32_same(cl, rl),
                "{label} err39 c2Len a={}: C {} vs R {}",
                fmt_v(a),
                fmt_f32(cl),
                fmt_f32(rl)
            );
        }
    });
}

// ===========================================================================
// Rows 40 / 41 — c22 collapse branches
// ===========================================================================

#[test]
fn err_row40to41_c22_collapse_branches() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0313);
        let mut seen = [0usize; 3];
        for i in 0..12000 {
            let mut cs = c2Simplex {
                verts: [c2sv::default(); 4],
                div: rng.ordinary(10.0),
                count: 2,
            };
            for v in cs.verts.iter_mut() {
                v.sA = rng.v_ordinary(20.0);
                v.sB = rng.v_ordinary(20.0);
                v.u = rng.ordinary(10.0);
                v.iA = rng.below(4) as c_int;
                v.iB = rng.below(4) as c_int;
            }
            // Deliberately place the pair so the origin is beyond a, beyond b,
            // or between them.
            let dir = c2v { x: rng.unit(), y: rng.unit() };
            let (t0, t1) = match rng.below(3) {
                0 => (1.0f32, 4.0f32),   // v <= 0 (origin before a)
                1 => (-4.0f32, -1.0f32), // u <= 0 (origin past b)
                _ => (-2.0f32, 2.0f32),  // interior
            };
            cs.verts[0].p = c2v { x: dir.x * t0, y: dir.y * t0 };
            cs.verts[1].p = c2v { x: dir.x * t1, y: dir.y * t1 };
            // Classify from the C's own formulas.
            let a = cs.verts[0].p;
            let b = cs.verts[1].p;
            let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
            let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
            let br = if v <= 0.0 { 0 } else if u <= 0.0 { 1 } else { 2 };
            seen[br] += 1;
            let mut rs = cs;
            unsafe {
                (c.c22)(&mut cs);
                (r.c22)(&mut rs);
            }
            assert!(
                simplex_same(&cs, &rs),
                "{label} err40-41 #{i} branch={br}:\n  C: {}\n  R: {}",
                fmt_simplex(&cs),
                fmt_simplex(&rs)
            );
            // Pin the documented result of each collapse.
            match br {
                0 | 1 => {
                    assert_eq!(cs.count, 1, "{label} err40-41: collapse must set count=1");
                    assert!(f32_same(cs.div, 1.0), "{label} err40-41: collapse div");
                    assert!(f32_same(cs.verts[0].u, 1.0), "{label} err40-41: collapse u");
                }
                _ => assert_eq!(cs.count, 2, "{label} err40-41: interior count"),
            }
        }
        assert!(
            seen.iter().all(|&n| n > 500),
            "{label} err40-41: branch coverage {seen:?}"
        );
        println!("{label} err40-41 c22 branch coverage: {seen:?}");
    });
}

// ===========================================================================
// Rows 42..=48 — the seven c23 region branches
// ===========================================================================

#[test]
fn err_row42to48_c23_all_regions() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0314);
        let mut seen = [0usize; 7];
        let mut tested = 0usize;
        while tested < 200_000 && seen.iter().any(|&n| n < 500) {
            let mut cs = c2Simplex {
                verts: [c2sv::default(); 4],
                div: rng.ordinary(10.0),
                count: 3,
            };
            for v in cs.verts.iter_mut() {
                v.sA = rng.v_ordinary(20.0);
                v.sB = rng.v_ordinary(20.0);
                v.u = rng.ordinary(10.0);
                v.iA = rng.below(4) as c_int;
                v.iB = rng.below(4) as c_int;
            }
            let off = match rng.below(3) {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => c2v { x: rng.unit() * 3.0, y: rng.unit() * 3.0 },
                _ => c2v { x: rng.unit() * 12.0, y: rng.unit() * 12.0 },
            };
            let t = rng.unit() * std::f32::consts::PI;
            let rad = 0.5 + rng.unit().abs() * 4.0;
            for k in 0..3 {
                let ang = t + k as f32 * 2.094_395_1;
                cs.verts[k].p = c2v {
                    x: off.x + rad * ang.cos(),
                    y: off.y + rad * ang.sin(),
                };
            }
            // Same classification the C performs.
            let dot = |p: c2v, q: c2v| p.x * q.x + p.y * q.y;
            let sub = |p: c2v, q: c2v| c2v { x: p.x - q.x, y: p.y - q.y };
            let det = |p: c2v, q: c2v| p.x * q.y - p.y * q.x;
            let (a, b, cc) = (cs.verts[0].p, cs.verts[1].p, cs.verts[2].p);
            let uab = dot(b, sub(b, a));
            let vab = dot(a, sub(a, b));
            let ubc = dot(cc, sub(cc, b));
            let vbc = dot(b, sub(b, cc));
            let uca = dot(a, sub(a, cc));
            let vca = dot(cc, sub(cc, a));
            let area = det(sub(b, a), sub(cc, a));
            let uabc = det(b, cc) * area;
            let vabc = det(cc, a) * area;
            let wabc = det(a, b) * area;
            let br = if vab <= 0.0 && uca <= 0.0 {
                0
            } else if uab <= 0.0 && vbc <= 0.0 {
                1
            } else if ubc <= 0.0 && vca <= 0.0 {
                2
            } else if uab > 0.0 && vab > 0.0 && wabc <= 0.0 {
                3
            } else if ubc > 0.0 && vbc > 0.0 && uabc <= 0.0 {
                4
            } else if uca > 0.0 && vca > 0.0 && vabc <= 0.0 {
                5
            } else {
                6
            };
            seen[br] += 1;
            tested += 1;
            let mut rs = cs;
            unsafe {
                (c.c23)(&mut cs);
                (r.c23)(&mut rs);
            }
            assert!(
                simplex_same(&cs, &rs),
                "{label} err42-48 branch={br}:\n  C: {}\n  R: {}",
                fmt_simplex(&cs),
                fmt_simplex(&rs)
            );
            let expect_count = match br {
                0..=2 => 1,
                3..=5 => 2,
                _ => 3,
            };
            assert_eq!(
                cs.count, expect_count,
                "{label} err42-48: branch {br} must give count={expect_count}"
            );
        }
        assert!(
            seen.iter().all(|&n| n >= 500),
            "{label} err42-48: branch coverage {seen:?} after {tested} samples"
        );
        println!("{label} err42-48 c23 branch coverage: {seen:?}");
    });
}

// ===========================================================================
// Rows 49..=51 — c2AABBtoAABB rejections
// ===========================================================================

#[test]
fn err_row49to51_aabb_rejections() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0315);
        // Row 49: each of the four separating-axis tests, isolated.
        let base = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 2.0, y: 2.0 },
        };
        let cases = [
            // B.max.x < A.min.x
            c2AABB { min: c2v { x: -5.0, y: 0.0 }, max: c2v { x: -1.0, y: 2.0 } },
            // A.max.x < B.min.x
            c2AABB { min: c2v { x: 3.0, y: 0.0 }, max: c2v { x: 5.0, y: 2.0 } },
            // B.max.y < A.min.y
            c2AABB { min: c2v { x: 0.0, y: -5.0 }, max: c2v { x: 2.0, y: -1.0 } },
            // A.max.y < B.min.y
            c2AABB { min: c2v { x: 0.0, y: 3.0 }, max: c2v { x: 2.0, y: 5.0 } },
        ];
        for (k, b) in cases.into_iter().enumerate() {
            let cv = (c.c2AABBtoAABB)(base, b);
            let rv = (r.c2AABBtoAABB)(base, b);
            assert_eq!(cv, rv, "{label} err49 case {k}");
            assert_eq!(cv, 0, "{label} err49: separating axis {k} must reject");
        }
        // Row 50: NaN => every `<` false => returns 1.
        let n = f32::NAN;
        let nanboxes = [
            c2AABB { min: c2v { x: n, y: 0.0 }, max: c2v { x: 2.0, y: 2.0 } },
            c2AABB { min: c2v { x: 0.0, y: n }, max: c2v { x: 2.0, y: 2.0 } },
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: n, y: 2.0 } },
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 2.0, y: n } },
            c2AABB { min: c2v { x: n, y: n }, max: c2v { x: n, y: n } },
        ];
        for (k, nb) in nanboxes.into_iter().enumerate() {
            for (a, b) in [(nb, base), (base, nb), (nb, nb)] {
                let cv = (c.c2AABBtoAABB)(a, b);
                let rv = (r.c2AABBtoAABB)(a, b);
                assert_eq!(cv, rv, "{label} err50 case {k}: A={a:?} B={b:?}");
            }
        }
        // The all-NaN pair is the pure form of the quirk.
        let allnan = c2AABB {
            min: c2v { x: n, y: n },
            max: c2v { x: n, y: n },
        };
        assert_eq!((c.c2AABBtoAABB)(allnan, allnan), 1, "{label} err50 sanity");
        assert_eq!(
            (c.c2AABBtoAABB)(allnan, allnan),
            (r.c2AABBtoAABB)(allnan, allnan)
        );
        // Row 51: inverted boxes, randomized.
        for i in 0..N {
            let p = rng.v_special();
            let q = rng.v_special();
            let a = c2AABB { min: p, max: q };
            let b = c2AABB { min: q, max: p };
            let cv = (c.c2AABBtoAABB)(a, b);
            let rv = (r.c2AABBtoAABB)(a, b);
            assert_eq!(cv, rv, "{label} err51 #{i}: A={a:?} B={b:?}");
        }
    });
}

// ===========================================================================
// Rows 52..=59 — predicate rejections
// ===========================================================================

#[test]
fn err_row52to53_circle_circle_rejections() {
    for_each_pair(|c, r, label| {
        // Row 52: d2 >= r2 (including exactly tangent).
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let tangent = c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: 2.0 };
        assert_eq!((c.c2CircletoCircle)(a, tangent), (r.c2CircletoCircle)(a, tangent));
        assert_eq!(
            (c.c2CircletoCircle)(a, tangent),
            0,
            "{label} err52: exact tangency must be rejected (d2 < r2 is strict)"
        );
        // negative total radius behaves like its magnitude
        let na = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -1.0 };
        let nb = c2Circle { p: c2v { x: 1.5, y: 0.0 }, r: -1.0 };
        assert_eq!((c.c2CircletoCircle)(na, nb), (r.c2CircletoCircle)(na, nb));
        assert_eq!(
            (c.c2CircletoCircle)(na, nb),
            1,
            "{label} err52: (-1)+(-1) squared is 4, so they 'collide'"
        );
        // Row 53: rA + rB == 0 with coincident centres.
        let z0 = c2Circle { p: c2v { x: 5.0, y: -5.0 }, r: 0.0 };
        assert_eq!((c.c2CircletoCircle)(z0, z0), (r.c2CircletoCircle)(z0, z0));
        assert_eq!((c.c2CircletoCircle)(z0, z0), 0, "{label} err53");
        let cancel_a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 4.0 };
        let cancel_b = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -4.0 };
        assert_eq!(
            (c.c2CircletoCircle)(cancel_a, cancel_b),
            (r.c2CircletoCircle)(cancel_a, cancel_b)
        );
        assert_eq!((c.c2CircletoCircle)(cancel_a, cancel_b), 0, "{label} err53 cancel");
        // NaN inputs.
        let nanc = c2Circle { p: c2v { x: f32::NAN, y: 0.0 }, r: 1.0 };
        for (x, y) in [(nanc, a), (a, nanc), (nanc, nanc)] {
            assert_eq!(
                (c.c2CircletoCircle)(x, y),
                (r.c2CircletoCircle)(x, y),
                "{label} err52 NaN"
            );
        }
    });
}

#[test]
fn err_row54to55_circle_aabb_rejections() {
    for_each_pair(|c, r, label| {
        let bb = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 2.0, y: 2.0 },
        };
        // Row 54: r == 0 -> r2 == 0 -> always rejected, even inside the box.
        let inside = c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 0.0 };
        assert_eq!((c.c2CircletoAABB)(inside, bb), (r.c2CircletoAABB)(inside, bb));
        assert_eq!(
            (c.c2CircletoAABB)(inside, bb),
            0,
            "{label} err54: r==0 always rejects (d2==0 is not < 0)"
        );
        // negative radius acts like |r|
        let neg = c2Circle { p: c2v { x: 3.0, y: 1.0 }, r: -2.0 };
        assert_eq!((c.c2CircletoAABB)(neg, bb), (r.c2CircletoAABB)(neg, bb));
        assert_eq!((c.c2CircletoAABB)(neg, bb), 1, "{label} err54 negative r");
        // exact tangency at a face
        let tangent = c2Circle { p: c2v { x: 4.0, y: 1.0 }, r: 2.0 };
        assert_eq!((c.c2CircletoAABB)(tangent, bb), (r.c2CircletoAABB)(tangent, bb));
        assert_eq!((c.c2CircletoAABB)(tangent, bb), 0, "{label} err54 tangency");
        // Row 55: inverted AABB.
        let inv = c2AABB {
            min: c2v { x: 2.0, y: 2.0 },
            max: c2v { x: 0.0, y: 0.0 },
        };
        let mut rng = Rng::new(0x0316);
        for i in 0..N {
            let a = match rng.below(3) {
                0 => c2Circle { p: rng.v_ordinary(5.0), r: rng.radius(3.0) },
                1 => c2Circle { p: rng.v_special(), r: rng.special() },
                _ => c2Circle { p: rng.v_ordinary(5.0), r: -rng.radius(3.0) },
            };
            assert_eq!(
                (c.c2CircletoAABB)(a, inv),
                (r.c2CircletoAABB)(a, inv),
                "{label} err55 #{i}: A={a:?}"
            );
            let degen = c2AABB { min: a.p, max: a.p };
            assert_eq!(
                (c.c2CircletoAABB)(a, degen),
                (r.c2CircletoAABB)(a, degen),
                "{label} err55 degen #{i}: A={a:?}"
            );
        }
    });
}

#[test]
fn err_row56to59_circle_capsule_rejections() {
    for_each_pair(|c, r, label| {
        let cap = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        // Row 56: da < 0 -> nearest is end a.
        let before = c2Circle { p: c2v { x: -5.0, y: 0.0 }, r: 1.0 };
        assert_eq!((c.c2CircletoCapsule)(before, cap), (r.c2CircletoCapsule)(before, cap));
        assert_eq!((c.c2CircletoCapsule)(before, cap), 0, "{label} err56");
        let touch_a = c2Circle { p: c2v { x: -1.5, y: 0.0 }, r: 1.0 };
        assert_eq!((c.c2CircletoCapsule)(touch_a, cap), (r.c2CircletoCapsule)(touch_a, cap));
        assert_eq!((c.c2CircletoCapsule)(touch_a, cap), 1, "{label} err56 hit");
        // Row 57: da >= 0, db < 0 -> segment projection.
        let mid = c2Circle { p: c2v { x: 5.0, y: 1.5 }, r: 1.0 };
        assert_eq!((c.c2CircletoCapsule)(mid, cap), (r.c2CircletoCapsule)(mid, cap));
        assert_eq!((c.c2CircletoCapsule)(mid, cap), 1, "{label} err57");
        let mid_far = c2Circle { p: c2v { x: 5.0, y: 5.0 }, r: 1.0 };
        assert_eq!((c.c2CircletoCapsule)(mid_far, cap), (r.c2CircletoCapsule)(mid_far, cap));
        assert_eq!((c.c2CircletoCapsule)(mid_far, cap), 0, "{label} err57 miss");
        // Row 58: da >= 0, db >= 0 -> nearest is end b.
        let past = c2Circle { p: c2v { x: 20.0, y: 0.0 }, r: 1.0 };
        assert_eq!((c.c2CircletoCapsule)(past, cap), (r.c2CircletoCapsule)(past, cap));
        assert_eq!((c.c2CircletoCapsule)(past, cap), 0, "{label} err58");
        // Row 59: d2 >= r*r, including exact tangency.
        let tangent = c2Circle { p: c2v { x: 5.0, y: 2.0 }, r: 1.0 };
        assert_eq!((c.c2CircletoCapsule)(tangent, cap), (r.c2CircletoCapsule)(tangent, cap));
        assert_eq!((c.c2CircletoCapsule)(tangent, cap), 0, "{label} err59 tangency");
        // Degenerate capsule (a == b): n == (0,0) so da == db == 0, the
        // `db >= 0` branch is taken and d2 = |A.p - B.b|^2.
        let p = c2v { x: 3.0, y: 4.0 };
        let degen = c2Capsule { a: p, b: p, r: 2.0 };
        for circ in [
            c2Circle { p, r: 1.0 },
            c2Circle { p: c2v { x: 5.9, y: 4.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 6.0, y: 4.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 6.1, y: 4.0 }, r: 1.0 },
        ] {
            assert_eq!(
                (c.c2CircletoCapsule)(circ, degen),
                (r.c2CircletoCapsule)(circ, degen),
                "{label} err57 degenerate capsule: A={circ:?}"
            );
        }
        // NaN and randomized sweeps.
        let mut rng = Rng::new(0x0317);
        for i in 0..N {
            let a = c2Circle { p: rng.v_special(), r: rng.special() };
            let b = c2Capsule {
                a: rng.v_special(),
                b: rng.v_special(),
                r: rng.special(),
            };
            assert_eq!(
                (c.c2CircletoCapsule)(a, b),
                (r.c2CircletoCapsule)(a, b),
                "{label} err56-59 #{i}: A={a:?} B={b:?}"
            );
        }
    });
}

// ===========================================================================
// Rows 60 / 61 — the two GJK-backed predicates and the NaN truth test
// ===========================================================================

#[test]
fn err_row60to61_gjk_backed_predicates() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0318);
        // NaN geometry makes c2GJK return NaN, which C's `if (float)` treats as
        // TRUE, so the predicates return 0.
        let nan_cap = c2Capsule {
            a: c2v { x: f32::NAN, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: 1.0,
        };
        let bb = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        assert_eq!(
            (c.c2AABBtoCapsule)(bb, nan_cap),
            (r.c2AABBtoCapsule)(bb, nan_cap),
            "{label} err60 NaN capsule"
        );
        assert_eq!(
            (c.c2CapsuletoCapsule)(nan_cap, nan_cap),
            (r.c2CapsuletoCapsule)(nan_cap, nan_cap),
            "{label} err61 NaN capsules"
        );
        // Exactly-touching (dist == 0) and just-separated pairs.
        let a = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 4.0, y: 0.0 },
            r: 1.0,
        };
        for dy in [1.9f32, 2.0, 2.0000005, 2.1] {
            let b = c2Capsule {
                a: c2v { x: 0.0, y: dy },
                b: c2v { x: 4.0, y: dy },
                r: 1.0,
            };
            assert_eq!(
                (c.c2CapsuletoCapsule)(a, b),
                (r.c2CapsuletoCapsule)(a, b),
                "{label} err61 dy={dy}"
            );
        }
        for i in 0..N_SLOW * 4 {
            let bb = c2AABB { min: rng.v_special(), max: rng.v_special() };
            let cap = c2Capsule {
                a: rng.v_special(),
                b: rng.v_special(),
                r: rng.special(),
            };
            assert_eq!(
                (c.c2AABBtoCapsule)(bb, cap),
                (r.c2AABBtoCapsule)(bb, cap),
                "{label} err60 #{i}: A={bb:?} B={cap:?}"
            );
            let cap2 = c2Capsule {
                a: rng.v_special(),
                b: rng.v_special(),
                r: rng.special(),
            };
            assert_eq!(
                (c.c2CapsuletoCapsule)(cap, cap2),
                (r.c2CapsuletoCapsule)(cap, cap2),
                "{label} err61 #{i}: A={cap:?} B={cap2:?}"
            );
        }
    });
}

// ===========================================================================
// Rows 62..=64 — generic FFI boundaries
// ===========================================================================

#[test]
fn err_row62_out_of_range_enum_everywhere() {
    for_each_pair(|c, r, label| {
        // Every entry point that takes a C2_TYPE, with every invalid value.
        // (`c2GJK` is excluded: an invalid type leaves the C's *uninitialised*
        // `c2Proxy` untouched, after which `c2Support` loops over a garbage
        // `count` -- an out-of-bounds read that can segfault. See row 27/62 in
        // ERRORS.md.)
        let sa = Shape::Circle(c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 });
        let buf = Buf(sa.bytes());
        for &bad in INVALID_TYPES.iter() {
            // c2MakeProxy: proxy untouched.
            let base = c2Proxy {
                radius: 1.25,
                count: 42,
                verts: [c2v { x: -1.0, y: -2.0 }; 8],
            };
            let mut cp = base;
            let mut rp = base;
            unsafe {
                (c.c2MakeProxy)(buf.0.as_ptr() as *const c_void, bad, &mut cp);
                (r.c2MakeProxy)(buf.0.as_ptr() as *const c_void, bad, &mut rp);
            }
            assert!(proxy_same(&cp, &rp), "{label} err62 c2MakeProxy bad={bad}");
            assert!(proxy_same(&cp, &base), "{label} err62: proxy must be untouched");

            // c2Collided: 0.
            let cv = unsafe {
                (c.c2Collided)(
                    buf.0.as_ptr() as *const c_void,
                    bad,
                    buf.0.as_ptr() as *const c_void,
                    bad,
                )
            };
            let rv = unsafe {
                (r.c2Collided)(
                    buf.0.as_ptr() as *const c_void,
                    bad,
                    buf.0.as_ptr() as *const c_void,
                    bad,
                )
            };
            assert_eq!(cv, rv, "{label} err62 c2Collided bad={bad}");
            assert_eq!(cv, 0);

            // omni_collide: 0.
            let cv = unsafe {
                (c.omni_collide)(bad, 1.0, 2.0, 3.0, 4.0, 5.0, bad, 1.0, 2.0, 3.0, 4.0, 5.0)
            };
            let rv = unsafe {
                (r.omni_collide)(bad, 1.0, 2.0, 3.0, 4.0, 5.0, bad, 1.0, 2.0, 3.0, 4.0, 5.0)
            };
            assert_eq!(cv, rv, "{label} err62 omni_collide bad={bad}");
            assert_eq!(cv, 0);
        }
        // One step past each end of the valid range, and every valid value, for
        // the full cross product of omni_collide's two type arguments.
        for a in -2i32..=4 {
            for b in -2i32..=4 {
                let cv = unsafe {
                    (c.omni_collide)(a, 0.0, 0.0, 1.0, 1.0, 1.0, b, 0.5, 0.5, 1.5, 1.5, 1.0)
                };
                let rv = unsafe {
                    (r.omni_collide)(a, 0.0, 0.0, 1.0, 1.0, 1.0, b, 0.5, 0.5, 1.5, 1.5, 1.0)
                };
                assert_eq!(cv, rv, "{label} err62/64 omni_collide a={a} b={b}");
            }
        }
    });
}

#[test]
fn err_row63_zero_and_negative_lengths() {
    for_each_pair(|c, r, label| {
        let verts = [
            c2v { x: 1.0, y: 2.0 },
            c2v { x: 3.0, y: 4.0 },
            c2v { x: -5.0, y: 6.0 },
            c2v { x: 7.0, y: -8.0 },
        ];
        for &n in &[0, -1, -2, i32::MIN, i32::MIN + 1, 1] {
            for d in [
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: f32::NAN, y: f32::NAN },
                c2v { x: f32::INFINITY, y: 1.0 },
            ] {
                let ci = unsafe { (c.c2Support)(verts.as_ptr(), n, d) };
                let ri = unsafe { (r.c2Support)(verts.as_ptr(), n, d) };
                assert_eq!(ci, ri, "{label} err63 n={n} d={}", fmt_v(d));
                assert_eq!(ci, 0, "{label} err63: n={n} must return 0");
            }
        }
        // Oversized-but-in-bounds counts (exactly the buffer length) still work.
        for n in 1..=4 {
            let d = c2v { x: 0.5, y: -0.5 };
            assert_eq!(
                unsafe { (c.c2Support)(verts.as_ptr(), n, d) },
                unsafe { (r.c2Support)(verts.as_ptr(), n, d) },
                "{label} err63 n={n}"
            );
        }
    });
}

#[test]
fn err_row64_one_step_past_every_range() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0319);
        // simplex count: 0 and 4 are one step outside {1,2,3} for every solver.
        for &count in &[0, 1, 2, 3, 4] {
            for i in 0..300 {
                let mut cs = c2Simplex {
                    verts: [c2sv::default(); 4],
                    div: rng.ordinary(10.0),
                    count,
                };
                for v in cs.verts.iter_mut() {
                    v.p = rng.v_ordinary(20.0);
                    v.sA = rng.v_ordinary(20.0);
                    v.sB = rng.v_ordinary(20.0);
                    v.u = rng.ordinary(10.0);
                }
                // c2GJKSimplexMetric / c2D / c2L / c2Witness all switch on count.
                let (mut c1, mut r1) = (cs, cs);
                assert!(
                    f32_same(
                        unsafe { (c.c2GJKSimplexMetric)(&mut c1) },
                        unsafe { (r.c2GJKSimplexMetric)(&mut r1) }
                    ),
                    "{label} err64 metric count={count} #{i}"
                );
                let (mut c2_, mut r2) = (cs, cs);
                assert!(
                    v_same(unsafe { (c.c2D)(&mut c2_) }, unsafe { (r.c2D)(&mut r2) }),
                    "{label} err64 c2D count={count} #{i}"
                );
                let (mut c3, mut r3) = (cs, cs);
                assert!(
                    v_same(unsafe { (c.c2L)(&mut c3) }, unsafe { (r.c2L)(&mut r3) }),
                    "{label} err64 c2L count={count} #{i}"
                );
                let (mut c4, mut r4) = (cs, cs);
                let poison = c2v { x: 2.5e-8, y: -1.75e13 };
                let (mut ca, mut cb) = (poison, poison);
                let (mut ra, mut rb) = (poison, poison);
                unsafe {
                    (c.c2Witness)(&mut c4, &mut ca, &mut cb);
                    (r.c2Witness)(&mut r4, &mut ra, &mut rb);
                }
                assert!(
                    v_same(ca, ra) && v_same(cb, rb),
                    "{label} err64 c2Witness count={count} #{i}"
                );
            }
        }
        // Iteration-cap neighbourhood: 19 / 20 / 21 iterations cannot be forced
        // directly, but every observed `iterations` value must agree, which the
        // sweep in err19-26 covers. Here we pin the invariant that the C never
        // reports more than 20.
        let sa = Shape::Aabb(c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        });
        for i in 0..500 {
            let sb = Shape::random(&mut rng, VALID_TYPES[(i % 3) as usize], 50.0);
            let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, None);
            let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, None);
            assert_eq!(co.3, ro.3, "{label} err64 iterations #{i}");
            assert!(
                (0..=20).contains(&co.3),
                "{label} err64: C reported {} iterations",
                co.3
            );
        }
    });
}

// ===========================================================================
// Row 15b — the exact `min_metric < max_metric * 2.0f` equality boundary
// ===========================================================================
//
// L400 is `if (!(min_metric < max_metric * 2.0f && metric < -1.0e8f))
//              cache_was_read = 1;`
//
// Reaching the side where the `&&` HOLDS (so `cache_was_read` stays 0 and the
// cached simplex is discarded) needs the *reloaded* simplex metric below -1e8,
// which needs `cache->count == 3` and a hugely negative
// `c2Det2(b.p - a.p, c.p - a.p)`. All of that is caller-controllable, because
// the caller owns the `c2GJKCache` and the C validates none of it.
//
// Construction:
//   A = AABB min(0,0) max(1,1)        -> verts (0,0) (1,0) (1,1) (0,1)
//   B = AABB min(X,0) max(0,1)        -> verts (X,0) (0,0) (0,1) (X,1)
//       (an inverted box -- perfectly legal, the C never validates)
//   cache.count = 3, iA = [0,0,0], iB = [0,1,2]
//     => p0 = (X,0), p1 = (0,0), p2 = (0,1)
//     => metric = c2Det2(p1-p0, p2-p0) = c2Det2((-X,0), (-X,1)) = -X
//
// With `X = 2^28`, `metric = -2^28`, comfortably below -1e8. Setting
// `cache->metric` (i.e. `metric_old`) to exactly `-2^27` makes
// `max_metric * 2.0f == -2^28 == min_metric` EXACTLY, i.e. lands precisely on
// the `<` boundary. And this cache is observable: `c23` on that simplex takes
// the vertex-B branch (`uAB <= 0 && vBC <= 0`), leaving `verts[0] = {iA:0,
// iB:1, p:(0,0)}`, whereas a fresh start has `verts[0] = {iA:0, iB:0,
// p:(X,0)}` -- so the returned distance, witness points and written-back cache
// all differ between `cache_was_read == 0` and `== 1`.
#[test]
fn err_row15b_cache_metric_equality_boundary() {
    /// One f32 step further away from zero (more negative, for negative input).
    fn more_negative(v: f32) -> f32 {
        assert!(v < 0.0);
        f32::from_bits(v.to_bits() + 1)
    }
    /// One f32 step closer to zero (less negative, for negative input).
    fn less_negative(v: f32) -> f32 {
        assert!(v < 0.0);
        f32::from_bits(v.to_bits() - 1)
    }

    for_each_pair(|c, r, label| {
        let mut observable = 0usize;
        let mut checked = 0usize;
        for xe in 28..=40i32 {
            let x = (2.0f32).powi(xe);
            for ye in 0..=4i32 {
                let y = (2.0f32).powi(ye);
                let sa = Shape::Aabb(c2AABB {
                    min: c2v { x: 0.0, y: 0.0 },
                    max: c2v { x: 1.0, y: y },
                });
                // Inverted in x, so the determinant comes out negative.
                let sb = Shape::Aabb(c2AABB {
                    min: c2v { x, y: 0.0 },
                    max: c2v { x: 0.0, y },
                });
                // metric the C recomputes when it reloads this cache
                let metric = -(x * y);
                assert!(metric < -1.0e8, "construction should give metric < -1e8");
                let half = metric * 0.5; // exact: power-of-two scaling

                let mk = |metric_old: f32| c2GJKCache {
                    metric: metric_old,
                    count: 3,
                    iA: [0, 0, 0],
                    iB: [0, 1, 2],
                    div: 1.0,
                };

                // Three metric_old values straddling the `<` boundary:
                //   less_negative(half): min < max*2  =>  TRUE  => cache discarded
                //   half exactly:        min < max*2  =>  FALSE => cache USED
                //   more_negative(half): min < max*2  =>  FALSE => cache USED
                let variants = [
                    ("below", less_negative(half)),
                    ("exact", half),
                    ("above", more_negative(half)),
                ];
                let mut outs = Vec::new();
                for (tag, metric_old) in variants {
                    let seed = mk(metric_old);
                    let mut ck = seed;
                    let mut rk = seed;
                    let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
                    let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
                    assert!(
                        f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                            && co.3 == ro.3,
                        "{label} err15b [{tag}] x=2^{xe} y=2^{ye} metric_old={}:\n  C dist={} a={} b={} it={}\n  R dist={} a={} b={} it={}",
                        fmt_f32(metric_old),
                        fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2), co.3,
                        fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2), ro.3,
                    );
                    assert!(
                        cache_same(&ck, &rk),
                        "{label} err15b [{tag}] x=2^{xe} y=2^{ye} metric_old={}: cache\n  C: {}\n  R: {}",
                        fmt_f32(metric_old),
                        fmt_cache(&ck),
                        fmt_cache(&rk)
                    );
                    checked += 1;
                    outs.push((tag, co, ck));
                }
                // Self-validation: the boundary must actually be *observable*,
                // otherwise this test would not be able to detect a `<` -> `<=`
                // change. "below" (cache discarded) must differ from "above"
                // (cache used), and "exact" must agree with "above" because the
                // C uses a strict `<`.
                let (_, below, below_k) = &outs[0];
                let (_, exact, exact_k) = &outs[1];
                let (_, above, above_k) = &outs[2];
                let differs = |p: &(f32, c2v, c2v, c_int), q: &(f32, c2v, c2v, c_int), pk: &c2GJKCache, qk: &c2GJKCache| {
                    !(f32_same(p.0, q.0) && v_same(p.1, q.1) && v_same(p.2, q.2) && p.3 == q.3
                        && cache_same(pk, qk))
                };
                if differs(below, above, below_k, above_k) {
                    observable += 1;
                    assert!(
                        !differs(exact, above, exact_k, above_k),
                        "{label} err15b x=2^{xe} y=2^{ye}: `min_metric == max_metric*2.0f` must take the SAME path as `>` (strict `<`), but exact={:?}/{} differs from above={:?}/{}",
                        exact,
                        fmt_cache(exact_k),
                        above,
                        fmt_cache(above_k)
                    );
                }
            }
        }
        println!("{label} err15b: {checked} comparisons, {observable} observable boundary cases");
        assert!(
            observable > 0,
            "{label} err15b: the `min_metric < max_metric * 2.0f` boundary was never observable, so this test cannot detect a change there"
        );
    });
}

// ===========================================================================
// Row 14b — warm cache with every reachable (count, index) combination
// ===========================================================================
#[test]
fn err_row14b_cache_all_index_combinations() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x031A);
        // Sweep every cache the caller could legitimately hand back: count in
        // 1..=3 and indices in range for the corresponding proxy vertex count.
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                let na = match ta {
                    C2_TYPE_CIRCLE => 1,
                    C2_TYPE_AABB => 4,
                    _ => 2,
                };
                let nb = match tb {
                    C2_TYPE_CIRCLE => 1,
                    C2_TYPE_AABB => 4,
                    _ => 2,
                };
                for count in 1..=3 {
                    for trial in 0..120 {
                        let sa = Shape::random(&mut rng, ta, 10.0);
                        let sb = Shape::random(&mut rng, tb, 10.0);
                        let mut ia = [0i32; 3];
                        let mut ib = [0i32; 3];
                        for k in 0..3 {
                            ia[k] = rng.below(na) as c_int;
                            ib[k] = rng.below(nb) as c_int;
                        }
                        let seed = c2GJKCache {
                            metric: match rng.below(5) {
                                0 => 0.0,
                                1 => rng.ordinary(1.0e3),
                                2 => -1.0e9,
                                3 => f32::NEG_INFINITY,
                                _ => rng.special_no_nan(),
                            },
                            count,
                            iA: ia,
                            iB: ib,
                            div: match rng.below(4) {
                                0 => 1.0,
                                1 => 0.0,
                                2 => rng.ordinary(10.0),
                                _ => rng.special_no_nan(),
                            },
                        };
                        let mut ck = seed;
                        let mut rk = seed;
                        let co = gjk(c, &sa, &sb, None, None, true, true, true, 1, Some(&mut ck));
                        let ro = gjk(r, &sa, &sb, None, None, true, true, true, 1, Some(&mut rk));
                        assert!(
                            f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                                && co.3 == ro.3,
                            "{label} err14b ta={ta} tb={tb} count={count} #{trial} cache={}\n  A={sa:?} B={sb:?}\n  C dist={} a={} b={} it={}\n  R dist={} a={} b={} it={}",
                            fmt_cache(&seed),
                            fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2), co.3,
                            fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2), ro.3,
                        );
                        assert!(
                            cache_same(&ck, &rk),
                            "{label} err14b ta={ta} tb={tb} count={count} #{trial}: cache\n  in: {}\n  C: {}\n  R: {}",
                            fmt_cache(&seed),
                            fmt_cache(&ck),
                            fmt_cache(&rk)
                        );
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Row 13b — NEGATIVE cache->count
// ===========================================================================
//
// `cache_was_good = !!cache->count` is TRUE for a negative count, so the C
// enters the cache-reload block with `cache->count < 0`. Every loop bounded by
// that count simply does not execute, and every `switch` on it falls to its
// `default:`, so the whole path is well defined (no out-of-bounds access):
//
//   * reload loop `for (i = 0; i < cache->count; ...)`  -> 0 iterations
//   * `s.count = cache->count` (negative), `s.div = cache->div`
//   * `c2GJKSimplexMetric` -> `default:` -> 0
//   * main loop: `save_count` negative -> save loop skipped;
//     `switch (s.count)` -> no label matches -> no solver runs;
//     `s.count == 3` false; `c2L` -> `default:` -> (0,0); `d1 = 0`;
//     `0 > FLT_MAX` false; `c2D` -> `default:` -> (0,0);
//     `c2Dot(d,d) == 0 < FLT_EPSILON^2` -> break with iter == 0
//   * `c2Witness` -> `default:` -> a = b = (0,0); `dist = 0`
//   * write-back loop `for (i = 0; i < s.count; ...)` -> 0 iterations, so
//     `cache->iA` / `cache->iB` keep the caller's values and `cache->count`
//     stays negative.
//
// This is a genuinely reachable, fully-defined rejection path, and it is the
// ONLY thing that distinguishes `!!cache->count` from `cache->count > 0`.
#[test]
fn err_row13b_negative_cache_count() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x031B);
        for &count in &[-1, -2, -3, -4, -100, c_int::MIN, c_int::MIN + 1] {
            for &ta in VALID_TYPES.iter() {
                for &tb in VALID_TYPES.iter() {
                    for i in 0..60 {
                        let sa = Shape::random(&mut rng, ta, 10.0);
                        let sb = Shape::random(&mut rng, tb, 10.0);
                        let seed = c2GJKCache {
                            metric: match rng.below(4) {
                                0 => 0.0,
                                1 => -1.0e9,
                                2 => f32::NEG_INFINITY,
                                _ => rng.ordinary(1.0e3),
                            },
                            count,
                            iA: [3, 2, 1],
                            iB: [1, 2, 3],
                            div: match rng.below(3) {
                                0 => 0.0,
                                1 => 1.0,
                                _ => rng.ordinary(10.0),
                            },
                        };
                        for ur in [0, 1] {
                            let mut ck = seed;
                            let mut rk = seed;
                            let co =
                                gjk(c, &sa, &sb, None, None, true, true, true, ur, Some(&mut ck));
                            let ro =
                                gjk(r, &sa, &sb, None, None, true, true, true, ur, Some(&mut rk));
                            assert!(
                                f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2)
                                    && co.3 == ro.3,
                                "{label} err13b #{i} count={count} ta={ta} tb={tb} ur={ur}\n  A={sa:?} B={sb:?} cache={}\n  C dist={} a={} b={} it={}\n  R dist={} a={} b={} it={}",
                                fmt_cache(&seed),
                                fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2), co.3,
                                fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2), ro.3,
                            );
                            assert!(
                                cache_same(&ck, &rk),
                                "{label} err13b #{i} count={count} ur={ur}: cache\n  in: {}\n  C: {}\n  R: {}",
                                fmt_cache(&seed),
                                fmt_cache(&ck),
                                fmt_cache(&rk)
                            );
                            // Pin the C's documented behaviour for this path:
                            // the negative count survives the write-back and the
                            // caller's indices are untouched.
                            assert_eq!(
                                ck.count, count,
                                "{label} err13b: negative count must be written back verbatim"
                            );
                            assert_eq!(ck.iA, [3, 2, 1], "{label} err13b: iA must be untouched");
                            assert_eq!(ck.iB, [1, 2, 3], "{label} err13b: iB must be untouched");
                            assert!(
                                f32_same(ck.metric, 0.0),
                                "{label} err13b: metric must be the default-branch 0"
                            );
                            assert!(f32_same(co.0, 0.0), "{label} err13b: dist must be 0");
                            assert_eq!(co.3, 0, "{label} err13b: iterations must be 0");
                        }
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Row 19 — the `iter < 20` cap is STRUCTURALLY UNREACHABLE
// ===========================================================================
//
// `tests/search_maxiter.rs` (an `#[ignore]`d diagnostic) drives 2,000,000
// randomized `c2GJK` calls across all nine type pairs, weird rotors, extreme
// coordinates and hostile caches. The maximum `*iterations` ever observed is 5:
//
//   histogram[0..6] = [982285, 758030, 228759, 30419, 496, 11]
//
// That is inherent to the shape set: a `c2Proxy` here holds at most 4 vertices
// (AABB), so the loop always terminates first through one of the other four
// exits (`s.count == 3`, `d1 > d0`, the `FLT_EPSILON` direction test, or the
// duplicate-support test). The `iter < 20` guard is therefore dead code for
// every input this library can be given.
//
// What this test pins down is everything that IS observable about the loop
// bound: both implementations must agree on `*iterations` for every input, and
// the value must stay inside `0..=20`.
#[test]
fn err_row19_iteration_cap_unreachable_but_iterations_agree() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x031C);
        let mut hist = [0usize; 24];
        for round in 0..40_000 {
            let ta = VALID_TYPES[rng.below(3) as usize];
            let tb = VALID_TYPES[rng.below(3) as usize];
            let (sa, sb) = match rng.below(5) {
                0 => (
                    Shape::random(&mut rng, ta, 1.0),
                    Shape::random(&mut rng, tb, 1.0),
                ),
                1 => (
                    Shape::random(&mut rng, ta, 1.0e6),
                    Shape::random(&mut rng, tb, 1.0e-6),
                ),
                2 => (
                    Shape::random_degenerate(&mut rng, ta, 10.0),
                    Shape::random_degenerate(&mut rng, tb, 10.0),
                ),
                3 => (
                    Shape::random_extreme(&mut rng, ta),
                    Shape::random_extreme(&mut rng, tb),
                ),
                _ => (
                    Shape::random(&mut rng, ta, 100.0),
                    Shape::random(&mut rng, tb, 100.0),
                ),
            };
            let ax = rng.xform_weird(50.0);
            let bx = rng.xform_weird(50.0);
            let use_ax = rng.bool();
            let use_bx = rng.bool();
            // Hostile but fully DEFINED cache:
            //   * `count` in 0..=3 only -- `count >= 4` would make the C write
            //     past the end of its `int saveA[3]`.
            //   * `iA`/`iB` strictly below the proxy vertex count for the
            //     corresponding type -- a larger index makes the C read a
            //     `c2Proxy.verts[]` slot that `c2MakeProxy` never initialised
            //     (its `c2Proxy` is an uninitialised automatic). That is
            //     `ERRORS.md` row 27: indeterminate in the C, so it must not be
            //     compared.
            let nverts = |t: c_int| -> u32 {
                match t {
                    C2_TYPE_CIRCLE => 1,
                    C2_TYPE_AABB => 4,
                    _ => 2,
                }
            };
            let (na, nb) = (nverts(ta), nverts(tb));
            let seed = c2GJKCache {
                metric: rng.special_no_nan(),
                count: rng.below(4) as c_int,
                iA: [
                    rng.below(na) as c_int,
                    rng.below(na) as c_int,
                    rng.below(na) as c_int,
                ],
                iB: [
                    rng.below(nb) as c_int,
                    rng.below(nb) as c_int,
                    rng.below(nb) as c_int,
                ],
                div: rng.special_no_nan(),
            };
            let use_cache = rng.below(3) == 0;
            let ur = rng.below(2) as c_int;
            let mut ck = seed;
            let mut rk = seed;
            let co = gjk(
                c,
                &sa,
                &sb,
                if use_ax { Some(&ax) } else { None },
                if use_bx { Some(&bx) } else { None },
                true,
                true,
                true,
                ur,
                if use_cache { Some(&mut ck) } else { None },
            );
            let ro = gjk(
                r,
                &sa,
                &sb,
                if use_ax { Some(&ax) } else { None },
                if use_bx { Some(&bx) } else { None },
                true,
                true,
                true,
                ur,
                if use_cache { Some(&mut rk) } else { None },
            );
            assert!(
                f32_same(co.0, ro.0) && v_same(co.1, ro.1) && v_same(co.2, ro.2) && co.3 == ro.3,
                "{label} err19 round={round} ta={ta} tb={tb} ur={ur} ax={use_ax} bx={use_bx} cache={use_cache}\n  A={sa:?}\n  B={sb:?}\n  cache_in={}\n  C dist={} a={} b={} it={}\n  R dist={} a={} b={} it={}",
                fmt_cache(&seed),
                fmt_f32(co.0), fmt_v(co.1), fmt_v(co.2), co.3,
                fmt_f32(ro.0), fmt_v(ro.1), fmt_v(ro.2), ro.3,
            );
            assert!(cache_same(&ck, &rk), "{label} err19 round={round}: cache differs");
            assert!(
                (0..=20).contains(&co.3),
                "{label} err19: C reported {} iterations (cap is 20)",
                co.3
            );
            hist[co.3 as usize] += 1;
        }
        println!("{label} err19: iterations histogram {hist:?}");
    });
}
