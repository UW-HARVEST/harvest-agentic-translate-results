//! Phase C — error/rejection-path differential tests, one test per row of
//! ERRORS.md. Every test builds the exact invalid input the C code checks for
//! and asserts BOTH libraries return the SAME sentinel / same rejection.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 2000;

/// Enum values with no valid variant. C enums accept any `int`, so these are
/// real inputs that must be handled identically.
const BAD_TYPES: [C2_TYPE; 8] = [3, 4, 7, 100, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF];

// ---------------------------------------------------------------------------
// Row 1 — c2MakeProxy with an out-of-range C2_TYPE leaves *p untouched
// ---------------------------------------------------------------------------

#[test]
fn err_makeproxy_bad_type_leaves_proxy_untouched() {
    let (c, r) = sym::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(SEED ^ 0x101);
    for &bad in BAD_TYPES.iter() {
        for _ in 0..200 {
            // Pre-fill with a recognisable pattern; the C `switch` has no
            // `default:` arm, so nothing may be written.
            let sentinel = c2Proxy {
                radius: rng.coord(),
                count: rng.below(1000) as i32 - 500,
                verts: [c2v { x: rng.coord(), y: rng.coord() }; 8],
            };
            let shape = rng.circle();
            let mut pc = sentinel;
            let mut pr = sentinel;
            unsafe {
                c(&shape as *const c2Circle as *const c_void, bad, &mut pc);
                r(&shape as *const c2Circle as *const c_void, bad, &mut pr);
            }
            same(
                "ERRORS row1 c2MakeProxy bad type",
                &format!("type={bad}"),
                pc,
                pr,
            );
            // and it must equal the untouched sentinel in BOTH
            same(
                "ERRORS row1 c2MakeProxy bad type (untouched)",
                &format!("type={bad}"),
                sentinel,
                pc,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 2,3,4 — c2Collided: valid typeA, out-of-range typeB -> 0
// ---------------------------------------------------------------------------

#[test]
fn err_collided_bad_typeB() {
    let (c, r) = sym::<FnCollided>("c2Collided");
    let mut rng = Rng::new(SEED ^ 0x102);
    for &ta in ALL_TYPES.iter() {
        for &bad in BAD_TYPES.iter() {
            for _ in 0..100 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, C2_TYPE_CIRCLE);
                let (cv, rv) =
                    unsafe { (c(a.as_ptr(), ta, b.as_ptr(), bad), r(a.as_ptr(), ta, b.as_ptr(), bad)) };
                same(
                    "ERRORS rows2-4 c2Collided bad typeB",
                    &format!("typeA={ta} typeB={bad}"),
                    cv,
                    rv,
                );
                assert_eq!(cv, 0, "C must return the 0 sentinel for typeB={bad}");
                assert_eq!(rv, 0, "Rust must return the 0 sentinel for typeB={bad}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — c2Collided: out-of-range typeA -> 0 (outer default arm)
// ---------------------------------------------------------------------------

#[test]
fn err_collided_bad_typeA() {
    let (c, r) = sym::<FnCollided>("c2Collided");
    let mut rng = Rng::new(SEED ^ 0x103);
    let mut all_types: Vec<C2_TYPE> = ALL_TYPES.to_vec();
    all_types.extend_from_slice(&BAD_TYPES);
    for &bad in BAD_TYPES.iter() {
        for &tb in all_types.iter() {
            for _ in 0..40 {
                let a = rand_shape(&mut rng, C2_TYPE_CIRCLE);
                let b = rand_shape(&mut rng, C2_TYPE_CIRCLE);
                let (cv, rv) = unsafe {
                    (
                        c(a.as_ptr(), bad, b.as_ptr(), tb),
                        r(a.as_ptr(), bad, b.as_ptr(), tb),
                    )
                };
                same(
                    "ERRORS row5 c2Collided bad typeA",
                    &format!("typeA={bad} typeB={tb}"),
                    cv,
                    rv,
                );
                assert_eq!(cv, 0);
                assert_eq!(rv, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6,7 — c2GJK with NULL transforms substitutes c2xIdentity()
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_null_transforms() {
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(SEED ^ 0x104);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..150 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                for &(ax, bx) in &[
                    (None, None),
                    (Some(ident), None),
                    (None, Some(ident)),
                    (Some(ident), Some(ident)),
                ] {
                    let o = GjkOpts { ax, bx, ..Default::default() };
                    gjk_same("ERRORS rows6-7 c2GJK NULL transforms", &a, &b, &o);
                }
                // NULL must give exactly the same answer as an explicit identity
                let (cf, rf) = sym::<FnGJK>("c2GJK");
                let onull = GjkOpts::default();
                let oid = GjkOpts { ax: Some(ident), bx: Some(ident), ..Default::default() };
                unsafe {
                    let cn = call_gjk(*cf, &a, &b, &onull);
                    let ci = call_gjk(*cf, &a, &b, &oid);
                    let rn = call_gjk(*rf, &a, &b, &onull);
                    let ri = call_gjk(*rf, &a, &b, &oid);
                    same("ERRORS rows6-7 NULL == identity (C)", &a.show(), cn, ci);
                    same("ERRORS rows6-7 NULL == identity (Rust)", &a.show(), rn, ri);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 8,9,10,11 — c2GJK with NULL outA / outB / iterations / cache
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_null_outputs() {
    let mut rng = Rng::new(SEED ^ 0x105);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..100 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                // All-NULL: only the return value is observable.
                let all_null = GjkOpts {
                    want_a: false,
                    want_b: false,
                    want_iters: false,
                    cache: None,
                    ..Default::default()
                };
                gjk_same("ERRORS rows8-11 c2GJK all outputs NULL", &a, &b, &all_null);
                // The distance must be independent of which outputs were asked for.
                let (cf, rf) = sym::<FnGJK>("c2GJK");
                unsafe {
                    let full = GjkOpts::default();
                    let dc_null = call_gjk(*cf, &a, &b, &all_null).dist;
                    let dc_full = call_gjk(*cf, &a, &b, &full).dist;
                    let dr_null = call_gjk(*rf, &a, &b, &all_null).dist;
                    let dr_full = call_gjk(*rf, &a, &b, &full).dist;
                    same("ERRORS rows8-11 dist(C) null vs full", &a.show(), dc_null, dc_full);
                    same("ERRORS rows8-11 dist(Rust) null vs full", &a.show(), dr_null, dr_full);
                    same("ERRORS rows8-11 dist C vs Rust", &a.show(), dc_null, dr_null);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — cache != NULL but cache->count == 0 (cold cache is not read)
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_cache_count_zero() {
    let mut rng = Rng::new(SEED ^ 0x106);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..150 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                // count == 0 but every other field is garbage: must be ignored.
                let cache = c2GJKCache {
                    metric: rng.coord(),
                    count: 0,
                    iA: [rng.below(8) as i32, 9, -3],
                    iB: [rng.below(8) as i32, -1, 12],
                    div: rng.coord(),
                };
                let o = GjkOpts { cache: Some(cache), ..Default::default() };
                gjk_same("ERRORS row12 c2GJK cache->count == 0", &a, &b, &o);
                // and it must match the NULL-cache distance
                let (cf, rf) = sym::<FnGJK>("c2GJK");
                unsafe {
                    let no_cache = GjkOpts::default();
                    same(
                        "ERRORS row12 cold cache == no cache (C)",
                        &a.show(),
                        call_gjk(*cf, &a, &b, &o).dist,
                        call_gjk(*cf, &a, &b, &no_cache).dist,
                    );
                    same(
                        "ERRORS row12 cold cache == no cache (Rust)",
                        &a.show(),
                        call_gjk(*rf, &a, &b, &o).dist,
                        call_gjk(*rf, &a, &b, &no_cache).dist,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — cache->count < 0: the read loop is skipped but s.count becomes
// negative, driving c22/c23/c2L/c2D/c2Witness into their default arms.
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_cache_count_negative() {
    let mut rng = Rng::new(SEED ^ 0x107);
    for &neg in &[-1i32, -2, -7, -1000, i32::MIN] {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                for _ in 0..40 {
                    let a = rand_shape(&mut rng, ta);
                    let b = rand_shape(&mut rng, tb);
                    let cache = c2GJKCache {
                        metric: rng.coord(),
                        count: neg,
                        iA: [0, 1, 2],
                        iB: [0, 1, 2],
                        div: if rng.below(3) == 0 { 0.0 } else { rng.range(0.5, 4.0) },
                    };
                    for ur in [0, 1] {
                        let o = GjkOpts {
                            cache: Some(cache),
                            use_radius: ur,
                            ..Default::default()
                        };
                        gjk_same("ERRORS row13 c2GJK cache->count < 0", &a, &b, &o);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — the cache metric guard: `min < max*2 && metric < -1e8f` discards it
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_cache_rejected_by_metric() {
    let mut rng = Rng::new(SEED ^ 0x108);
    // metric_old values that straddle the -1e8 threshold, incl. non-finite.
    let metrics = [
        -1.0e8f32,
        -1.000_001e8,
        -1.0e9,
        -1.0e20,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NAN,
        0.0,
        1.0e8,
    ];
    for &metric in metrics.iter() {
        for &ta in ALL_TYPES.iter() {
            for &tb in ALL_TYPES.iter() {
                for _ in 0..40 {
                    let a = rand_shape(&mut rng, ta);
                    let b = rand_shape(&mut rng, tb);
                    for count in 1i32..=3 {
                        let cache = c2GJKCache {
                            metric,
                            count,
                            // indices in range for every proxy kind (max count 4)
                            iA: [0, 0, 0],
                            iB: [0, 0, 0],
                            div: rng.range(0.25, 4.0),
                        };
                        let o = GjkOpts { cache: Some(cache), ..Default::default() };
                        gjk_same("ERRORS row14 c2GJK cache metric guard", &a, &b, &o);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — the `while (iter < 20)` iteration cap
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_iteration_cap() {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut rng = Rng::new(SEED ^ 0x109);
    let mut max_c = -1;
    let mut max_r = -1;
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..600 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                let o = GjkOpts {
                    ax: Some(rng.xform()),
                    bx: Some(rng.xform()),
                    ..Default::default()
                };
                let (rc, rr) = unsafe { (call_gjk(*cf, &a, &b, &o), call_gjk(*rf, &a, &b, &o)) };
                same("ERRORS row15 c2GJK iteration cap", &a.show(), rc, rr);
                max_c = max_c.max(rc.iters.unwrap());
                max_r = max_r.max(rr.iters.unwrap());
                assert!(rc.iters.unwrap() <= 20, "C exceeded the iteration cap");
                assert!(rr.iters.unwrap() <= 20, "Rust exceeded the iteration cap");
            }
        }
    }
    assert_eq!(max_c, max_r, "iteration counts diverge at the cap");
}

// ---------------------------------------------------------------------------
// Rows 16,17,18 — the three loop-termination guards (d1 > d0, degenerate
// direction, duplicate support point). Reaching them is observable through
// `iterations`, which must agree exactly.
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_termination_guards() {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut rng = Rng::new(SEED ^ 0x10A);
    let mut iter_hist = [0usize; 21];
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            for _ in 0..500 {
                let a = rand_shape(&mut rng, ta);
                // identical / near-identical shapes -> degenerate direction guard
                let b = match rng.below(3) {
                    0 => a,
                    1 => rand_shape(&mut rng, tb),
                    _ => rand_shape(&mut rng, tb),
                };
                for ur in [0, 1] {
                    let o = GjkOpts { use_radius: ur, ..Default::default() };
                    let (rc, rr) = unsafe { (call_gjk(*cf, &a, &b, &o), call_gjk(*rf, &a, &b, &o)) };
                    same("ERRORS rows16-18 c2GJK termination guards", &a.show(), rc, rr);
                    let it = rc.iters.unwrap();
                    if (0..=20).contains(&it) {
                        iter_hist[it as usize] += 1;
                    }
                }
            }
        }
    }
    // The guards must actually be exercised: 0 iterations (immediate break) and
    // >=1 iterations both have to occur.
    assert!(iter_hist[0] > 0, "rows16-18: the immediate-break guards were never hit");
    assert!(
        iter_hist[1..].iter().sum::<usize>() > 0,
        "rows16-18: the loop never iterated"
    );
}

// ---------------------------------------------------------------------------
// Row 19 — hit path: simplex reaches count 3 -> a = b and dist = 0
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_hit_zero_distance() {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut rng = Rng::new(SEED ^ 0x10B);
    let mut hits = 0usize;
    for _ in 0..N {
        // two large overlapping AABBs guarantee the origin is enclosed
        let a = Shape::Aabb(c2AABB {
            min: c2v { x: -10.0, y: -10.0 },
            max: c2v { x: 10.0, y: 10.0 },
        });
        let b = Shape::Aabb(c2AABB {
            min: c2v { x: rng.range(-9.0, 0.0), y: rng.range(-9.0, 0.0) },
            max: c2v { x: rng.range(0.0, 9.0), y: rng.range(0.0, 9.0) },
        });
        for ur in [0, 1] {
            let o = GjkOpts { use_radius: ur, cache: Some(c2GJKCache::default()), ..Default::default() };
            let (rc, rr) = unsafe { (call_gjk(*cf, &a, &b, &o), call_gjk(*rf, &a, &b, &o)) };
            same("ERRORS row19 c2GJK hit path", &b.show(), rc, rr);
            if rc.dist == 0.0 && rc.a.unwrap().x == rc.b.unwrap().x {
                hits += 1;
            }
        }
    }
    assert!(hits > 0, "row19: the hit path was never reached");
}

// ---------------------------------------------------------------------------
// Rows 20,21 — the use_radius midpoint collapse and the exact-equality reset
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_radius_collapse() {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut rng = Rng::new(SEED ^ 0x10C);
    let mut collapses = 0usize;
    for _ in 0..N * 2 {
        // circles whose gap is smaller than rA + rB -> the else branch
        let r1 = rng.range(0.0, 10.0);
        let r2 = rng.range(0.0, 10.0);
        let gap = rng.range(0.0, r1 + r2 + 0.5);
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: r1 });
        let b = Shape::Circle(c2Circle { p: c2v { x: gap, y: 0.0 }, r: r2 });
        let o = GjkOpts::default();
        let (rc, rr) = unsafe { (call_gjk(*cf, &a, &b, &o), call_gjk(*rf, &a, &b, &o)) };
        same("ERRORS rows20-21 c2GJK radius collapse", &format!("{r1} {r2} {gap}"), rc, rr);
        if rc.dist == 0.0 {
            collapses += 1;
        }
    }
    assert!(collapses > 0, "rows20-21: the collapse branch was never taken");

    // dist <= FLT_EPSILON with zero radii (the second half of the || guard)
    for &d in &[0.0f32, f32::EPSILON, f32::EPSILON / 2.0, 1.0e-8] {
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 });
        let b = Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: 0.0 });
        gjk_same(
            "ERRORS rows20-21 c2GJK dist <= FLT_EPSILON",
            &a,
            &b,
            &GjkOpts::default(),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 22 — use_radius == 0 must NOT subtract the radii
// ---------------------------------------------------------------------------

#[test]
fn err_gjk_use_radius_zero() {
    let (cf, rf) = sym::<FnGJK>("c2GJK");
    let mut rng = Rng::new(SEED ^ 0x10D);
    for _ in 0..N {
        let a = Shape::Circle(c2Circle {
            p: c2v { x: rng.coord(), y: rng.coord() },
            r: rng.range(0.5, 20.0),
        });
        let b = Shape::Capsule(c2Capsule {
            a: c2v { x: rng.coord(), y: rng.coord() },
            b: c2v { x: rng.coord(), y: rng.coord() },
            r: rng.range(0.5, 20.0),
        });
        for &ur in &[0i32, 1, 2, -1, i32::MIN, i32::MAX] {
            let o = GjkOpts { use_radius: ur, ..Default::default() };
            let (rc, rr) = unsafe { (call_gjk(*cf, &a, &b, &o), call_gjk(*rf, &a, &b, &o)) };
            same(
                "ERRORS row22 c2GJK use_radius variants",
                &format!("use_radius={ur} A={} B={}", a.show(), b.show()),
                rc,
                rr,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 — c2GJKSimplexMetric with a count outside {2,3}
// ---------------------------------------------------------------------------

#[test]
fn err_simplexmetric_bad_count() {
    let (c, r) = sym::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(SEED ^ 0x10E);
    for &count in &[0i32, 1, 4, 5, 100, -1, -100, i32::MIN, i32::MAX] {
        for _ in 0..200 {
            let mut s1 = rng.simplex(count);
            let mut s2 = s1;
            let (cv, rv) = unsafe { (c(&mut s1), r(&mut s2)) };
            same(
                "ERRORS row23 c2GJKSimplexMetric bad count",
                &format!("count={count}"),
                cv,
                rv,
            );
            if count != 2 && count != 3 {
                assert_eq!(cv.to_bits(), 0.0f32.to_bits(), "C must return +0 for count={count}");
                assert_eq!(rv.to_bits(), 0.0f32.to_bits(), "Rust must return +0 for count={count}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — c2D with count == 3 or any other value -> c2V(0,0)
// ---------------------------------------------------------------------------

#[test]
fn err_c2d_bad_count() {
    let (c, r) = sym::<FnSimplexV>("c2D");
    let mut rng = Rng::new(SEED ^ 0x10F);
    for &count in &[0i32, 3, 4, 9, -1, -50, i32::MIN, i32::MAX] {
        for _ in 0..200 {
            let mut s1 = rng.simplex(count);
            let mut s2 = s1;
            let (cv, rv) = unsafe { (c(&mut s1), r(&mut s2)) };
            same("ERRORS row24 c2D bad count", &format!("count={count}"), cv, rv);
            assert_eq!((cv.x.to_bits(), cv.y.to_bits()), (0, 0), "C sentinel for count={count}");
            assert_eq!((rv.x.to_bits(), rv.y.to_bits()), (0, 0), "Rust sentinel for count={count}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — c2L with count outside {1,2} -> c2V(0,0)
// ---------------------------------------------------------------------------

#[test]
fn err_c2l_bad_count() {
    let (c, r) = sym::<FnSimplexV>("c2L");
    let mut rng = Rng::new(SEED ^ 0x110);
    for &count in &[0i32, 3, 4, 17, -1, -9, i32::MIN, i32::MAX] {
        for _ in 0..200 {
            let mut s1 = rng.simplex(count);
            let mut s2 = s1;
            let (cv, rv) = unsafe { (c(&mut s1), r(&mut s2)) };
            same("ERRORS row25 c2L bad count", &format!("count={count}"), cv, rv);
            assert_eq!((cv.x.to_bits(), cv.y.to_bits()), (0, 0));
            assert_eq!((rv.x.to_bits(), rv.y.to_bits()), (0, 0));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — c2Witness with count outside {1,2,3} -> both outputs c2V(0,0)
// ---------------------------------------------------------------------------

#[test]
fn err_witness_bad_count() {
    let (c, r) = sym::<FnWitness>("c2Witness");
    let mut rng = Rng::new(SEED ^ 0x111);
    for &count in &[0i32, 4, 5, 33, -1, -4, i32::MIN, i32::MAX] {
        for _ in 0..200 {
            let mut s1 = rng.simplex(count);
            let mut s2 = s1;
            let mut ca = c2v { x: 42.0, y: 43.0 };
            let mut cb = c2v { x: 44.0, y: 45.0 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                c(&mut s1, &mut ca, &mut cb);
                r(&mut s2, &mut ra, &mut rb);
            }
            same(
                "ERRORS row26 c2Witness bad count",
                &format!("count={count}"),
                (ca, cb),
                (ra, rb),
            );
            assert_eq!(
                (ca.x.to_bits(), ca.y.to_bits(), cb.x.to_bits(), cb.y.to_bits()),
                (0, 0, 0, 0),
                "C sentinel for count={count}"
            );
            assert_eq!(
                (ra.x.to_bits(), ra.y.to_bits(), rb.x.to_bits(), rb.y.to_bits()),
                (0, 0, 0, 0),
                "Rust sentinel for count={count}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27 — c2Witness with div == 0 (den = inf, no guard)
// ---------------------------------------------------------------------------

#[test]
fn err_witness_zero_div() {
    let (c, r) = sym::<FnWitness>("c2Witness");
    let mut rng = Rng::new(SEED ^ 0x112);
    for count in 1i32..=3 {
        for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE] {
            for _ in 0..200 {
                let mut s1 = rng.simplex(count);
                s1.div = div;
                let mut s2 = s1;
                let mut ca = c2v::default();
                let mut cb = c2v::default();
                let mut ra = c2v::default();
                let mut rb = c2v::default();
                unsafe {
                    c(&mut s1, &mut ca, &mut cb);
                    r(&mut s2, &mut ra, &mut rb);
                }
                same(
                    "ERRORS row27 c2Witness div == 0",
                    &format!("count={count} div={div:?}"),
                    (ca, cb),
                    (ra, rb),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 28 — c2L with div == 0
// ---------------------------------------------------------------------------

#[test]
fn err_c2l_zero_div() {
    let (c, r) = sym::<FnSimplexV>("c2L");
    let mut rng = Rng::new(SEED ^ 0x113);
    for count in 1i32..=2 {
        for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for _ in 0..200 {
                let mut s1 = rng.simplex(count);
                s1.div = div;
                let mut s2 = s1;
                let (cv, rv) = unsafe { (c(&mut s1), r(&mut s2)) };
                same(
                    "ERRORS row28 c2L div == 0",
                    &format!("count={count} div={div:?}"),
                    cv,
                    rv,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 29 — c2Div by zero
// ---------------------------------------------------------------------------

#[test]
fn err_div_by_zero() {
    let (c, r) = sym::<FnVfV>("c2Div");
    let mut rng = Rng::new(SEED ^ 0x114);
    let divisors = [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::from_bits(1)];
    for &b in divisors.iter() {
        for _ in 0..300 {
            let a = rng.wild_vec();
            unsafe {
                same(
                    "ERRORS row29 c2Div by zero",
                    &format!("{a:?} / {b:?}"),
                    c(a, b),
                    r(a, b),
                )
            }
        }
        // exact corner cases
        for &v in &[
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 1.0, y: -1.0 },
            c2v { x: f32::MAX, y: f32::MIN },
            c2v { x: f32::NAN, y: 0.0 },
        ] {
            unsafe {
                same(
                    "ERRORS row29 c2Div corner",
                    &format!("{v:?} / {b:?}"),
                    c(v, b),
                    r(v, b),
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 30 — c2Norm of the zero vector
// ---------------------------------------------------------------------------

#[test]
fn err_norm_zero_vector() {
    let (c, r) = sym::<FnVV>("c2Norm");
    for &v in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: 0.0 },
    ] {
        unsafe {
            same(
                "ERRORS row30 c2Norm zero/degenerate",
                &format!("{v:?}"),
                c(v),
                r(v),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Row 31 — c2Len with non-finite components (sqrtf has no domain check)
// ---------------------------------------------------------------------------

#[test]
fn err_len_nonfinite() {
    let (c, r) = sym::<FnVf>("c2Len");
    let vals = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        1e30,
        -1e30,
    ];
    for &x in vals.iter() {
        for &y in vals.iter() {
            let v = c2v { x, y };
            unsafe { same("ERRORS row31 c2Len non-finite", &format!("{v:?}"), c(v), r(v)) }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 32,33 — c2Support with count <= 0 and count == 1
// ---------------------------------------------------------------------------

#[test]
fn err_support_count_le_zero() {
    let (c, r) = sym::<FnSupport>("c2Support");
    let mut rng = Rng::new(SEED ^ 0x115);
    // verts[0] is dereferenced even when count <= 0, so a valid (non-NULL)
    // buffer is still required; only `count` is invalid here.
    for &count in &[0i32, 1, -1, -7, -1000, i32::MIN] {
        for _ in 0..300 {
            let mut verts = [c2v::default(); 8];
            for i in 0..8 {
                verts[i] = rng.wild_vec();
            }
            let d = rng.wild_vec();
            let (cv, rv) =
                unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            same(
                "ERRORS rows32-33 c2Support count <= 1",
                &format!("count={count} d={d:?}"),
                cv,
                rv,
            );
            assert_eq!(cv, 0, "C must return index 0 for count={count}");
            assert_eq!(rv, 0, "Rust must return index 0 for count={count}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 34 — c2Support where `dot > dmax` is never true (all-equal or NaN dots)
// ---------------------------------------------------------------------------

#[test]
fn err_support_nan_dots() {
    let (c, r) = sym::<FnSupport>("c2Support");
    let mut rng = Rng::new(SEED ^ 0x116);
    for count in 1i32..=8 {
        for _ in 0..300 {
            let mut verts = [c2v::default(); 8];
            match rng.below(3) {
                0 => {
                    // identical verts -> all dots equal
                    let v = rng.vec();
                    verts = [v; 8];
                }
                1 => {
                    // NaN verts -> every comparison false
                    for i in 0..8 {
                        verts[i] = c2v { x: f32::NAN, y: f32::NAN };
                    }
                }
                _ => {
                    for i in 0..8 {
                        verts[i] = rng.vec();
                    }
                }
            }
            let d = if rng.below(2) == 0 {
                c2v { x: f32::NAN, y: f32::NAN }
            } else {
                c2v { x: 0.0, y: 0.0 }
            };
            let (cv, rv) =
                unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            same(
                "ERRORS row34 c2Support degenerate dots",
                &format!("count={count} d={d:?} verts[0]={:?}", verts[0]),
                cv,
                rv,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 35 — c2CircletoCapsule with a degenerate capsule (a == b)
// ---------------------------------------------------------------------------

#[test]
fn err_circletocapsule_degenerate() {
    let (c, r) = sym::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(SEED ^ 0x117);
    for _ in 0..N * 2 {
        let p = rng.vec();
        let cap = c2Capsule { a: p, b: p, r: rng.radius() };
        let circ = rng.circle();
        unsafe {
            same(
                "ERRORS row35 c2CircletoCapsule degenerate capsule",
                &format!("{circ:?} {cap:?}"),
                c(circ, cap),
                r(circ, cap),
            )
        }
    }
    // exactly coincident centres, and non-finite capsules
    for &(cx, cy, cr) in &[(0.0f32, 0.0f32, 0.0f32), (0.0, 0.0, 1.0), (1.0, 1.0, 0.0)] {
        for &(ax, ay, ar) in &[
            (0.0f32, 0.0f32, 0.0f32),
            (0.0, 0.0, 1.0),
            (f32::NAN, 0.0, 1.0),
            (f32::INFINITY, 0.0, 1.0),
        ] {
            let circ = c2Circle { p: c2v { x: cx, y: cy }, r: cr };
            let cap = c2Capsule {
                a: c2v { x: ax, y: ay },
                b: c2v { x: ax, y: ay },
                r: ar,
            };
            unsafe {
                same(
                    "ERRORS row35 degenerate fixed",
                    &format!("{circ:?} {cap:?}"),
                    c(circ, cap),
                    r(circ, cap),
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 36 — negative radii
// ---------------------------------------------------------------------------

#[test]
fn err_negative_radius() {
    let (cc, rc) = sym::<FnCircletoCircle>("c2CircletoCircle");
    let (ca, ra) = sym::<FnCircletoAABB>("c2CircletoAABB");
    let (cp, rp) = sym::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(SEED ^ 0x118);
    for _ in 0..N {
        let mut a = rng.circle();
        let mut b = rng.circle();
        a.r = -rng.range(0.0, 30.0);
        b.r = -rng.range(0.0, 30.0);
        let bb = rng.aabb();
        let mut cap = rng.capsule();
        cap.r = -rng.range(0.0, 30.0);
        unsafe {
            same(
                "ERRORS row36 c2CircletoCircle negative r",
                &format!("{a:?} {b:?}"),
                cc(a, b),
                rc(a, b),
            );
            same(
                "ERRORS row36 c2CircletoAABB negative r",
                &format!("{a:?} {bb:?}"),
                ca(a, bb),
                ra(a, bb),
            );
            same(
                "ERRORS row36 c2CircletoCapsule negative r",
                &format!("{a:?} {cap:?}"),
                cp(a, cap),
                rp(a, cap),
            );
        }
    }
    // negative radii through c2GJK / c2Collided as well
    for _ in 0..500 {
        let mut circ = rng.circle();
        circ.r = -rng.range(0.0, 20.0);
        let mut cap = rng.capsule();
        cap.r = -rng.range(0.0, 20.0);
        for (a, b) in [
            (Shape::Circle(circ), Shape::Capsule(cap)),
            (Shape::Capsule(cap), Shape::Circle(circ)),
        ] {
            for ur in [0, 1] {
                gjk_same(
                    "ERRORS row36 c2GJK negative radius",
                    &a,
                    &b,
                    &GjkOpts { use_radius: ur, ..Default::default() },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 37,38 — inverted AABBs (min > max)
// ---------------------------------------------------------------------------

#[test]
fn err_inverted_aabb() {
    let (caa, raa) = sym::<FnAABBtoAABB>("c2AABBtoAABB");
    let (cca, rca) = sym::<FnCircletoAABB>("c2CircletoAABB");
    let (cac, rac) = sym::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let (cbb, rbb) = sym::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(SEED ^ 0x119);
    for _ in 0..N * 2 {
        // deliberately inverted on one or both axes
        let p = rng.vec();
        let q = rng.vec();
        let inv = match rng.below(3) {
            0 => c2AABB { min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) }, max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) } },
            1 => c2AABB { min: c2v { x: p.x.max(q.x), y: p.y.min(q.y) }, max: c2v { x: p.x.min(q.x), y: p.y.max(q.y) } },
            _ => c2AABB { min: c2v { x: p.x.min(q.x), y: p.y.max(q.y) }, max: c2v { x: p.x.max(q.x), y: p.y.min(q.y) } },
        };
        let other = rng.aabb();
        let circ = rng.circle();
        let cap = rng.capsule();
        unsafe {
            same(
                "ERRORS row37 c2AABBtoAABB inverted",
                &format!("{inv:?} {other:?}"),
                caa(inv, other),
                raa(inv, other),
            );
            same(
                "ERRORS row37 c2AABBtoAABB inverted rev",
                &format!("{other:?} {inv:?}"),
                caa(other, inv),
                raa(other, inv),
            );
            same(
                "ERRORS row38 c2CircletoAABB inverted",
                &format!("{circ:?} {inv:?}"),
                cca(circ, inv),
                rca(circ, inv),
            );
            same(
                "ERRORS row38 c2AABBtoCapsule inverted",
                &format!("{inv:?} {cap:?}"),
                cac(inv, cap),
                rac(inv, cap),
            );
            let mut vc = [c2v { x: -1.0, y: -1.0 }; 4];
            let mut vr = vc;
            let mut bbc = inv;
            let mut bbr = inv;
            cbb(vc.as_mut_ptr(), &mut bbc);
            rbb(vr.as_mut_ptr(), &mut bbr);
            for i in 0..4 {
                same(
                    "ERRORS row38 c2BBVerts inverted",
                    &format!("{inv:?} vert {i}"),
                    vc[i],
                    vr[i],
                );
            }
        }
        // and through the GJK-backed path
        gjk_same(
            "ERRORS row37 c2GJK inverted AABB",
            &Shape::Aabb(inv),
            &Shape::Capsule(cap),
            &GjkOpts::default(),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 39 — non-finite inputs propagate identically through every entry point
// ---------------------------------------------------------------------------

#[test]
fn err_nonfinite_inputs_propagate() {
    let mut rng = Rng::new(SEED ^ 0x11A);
    let (cc, rc) = sym::<FnCircletoCircle>("c2CircletoCircle");
    let (ca, ra) = sym::<FnCircletoAABB>("c2CircletoAABB");
    let (cp, rp) = sym::<FnCircletoCapsule>("c2CircletoCapsule");
    let (caa, raa) = sym::<FnAABBtoAABB>("c2AABBtoAABB");
    let (cac, rac) = sym::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let (ccc, rcc) = sym::<FnCapsuletoCapsule>("c2CapsuletoCapsule");

    let wild_circle = |rng: &mut Rng| c2Circle {
        p: rng.wild_vec(),
        r: rng.wild(),
    };
    let wild_aabb = |rng: &mut Rng| c2AABB {
        min: rng.wild_vec(),
        max: rng.wild_vec(),
    };
    let wild_capsule = |rng: &mut Rng| c2Capsule {
        a: rng.wild_vec(),
        b: rng.wild_vec(),
        r: rng.wild(),
    };

    for _ in 0..N * 2 {
        let c1 = wild_circle(&mut rng);
        let c2 = wild_circle(&mut rng);
        let b1 = wild_aabb(&mut rng);
        let b2 = wild_aabb(&mut rng);
        let k1 = wild_capsule(&mut rng);
        let k2 = wild_capsule(&mut rng);
        unsafe {
            same("ERRORS row39 CircletoCircle wild", &format!("{c1:?} {c2:?}"), cc(c1, c2), rc(c1, c2));
            same("ERRORS row39 CircletoAABB wild", &format!("{c1:?} {b1:?}"), ca(c1, b1), ra(c1, b1));
            same("ERRORS row39 CircletoCapsule wild", &format!("{c1:?} {k1:?}"), cp(c1, k1), rp(c1, k1));
            same("ERRORS row39 AABBtoAABB wild", &format!("{b1:?} {b2:?}"), caa(b1, b2), raa(b1, b2));
            same("ERRORS row39 AABBtoCapsule wild", &format!("{b1:?} {k1:?}"), cac(b1, k1), rac(b1, k1));
            same("ERRORS row39 CapsuletoCapsule wild", &format!("{k1:?} {k2:?}"), ccc(k1, k2), rcc(k1, k2));
        }
        // through c2GJK, with non-finite transforms too
        for (a, b) in [
            (Shape::Circle(c1), Shape::Aabb(b2)),
            (Shape::Aabb(b1), Shape::Capsule(k2)),
            (Shape::Capsule(k1), Shape::Circle(c2)),
        ] {
            let o = GjkOpts {
                ax: if rng.below(2) == 0 {
                    None
                } else {
                    Some(c2x { p: rng.wild_vec(), r: c2r { c: rng.wild(), s: rng.wild() } })
                },
                bx: if rng.below(2) == 0 {
                    None
                } else {
                    Some(c2x { p: rng.wild_vec(), r: c2r { c: rng.wild(), s: rng.wild() } })
                },
                use_radius: (rng.below(2) as i32),
                cache: if rng.below(2) == 0 { None } else { Some(c2GJKCache::default()) },
                ..Default::default()
            };
            gjk_same("ERRORS row39 c2GJK wild", &a, &b, &o);
        }
        // and through c2Collided
        let (cf, rf) = sym::<FnCollided>("c2Collided");
        let shapes = [Shape::Circle(c1), Shape::Aabb(b1), Shape::Capsule(k1)];
        let shapes2 = [Shape::Circle(c2), Shape::Aabb(b2), Shape::Capsule(k2)];
        for a in shapes.iter() {
            for b in shapes2.iter() {
                unsafe {
                    same(
                        "ERRORS row39 c2Collided wild",
                        &format!("{} {}", a.show(), b.show()),
                        cf(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()),
                        rf(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()),
                    )
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 40 — reverse_collide with non-finite / extreme arguments
// ---------------------------------------------------------------------------

#[test]
fn err_reverse_collide_nonfinite() {
    let (c, r) = sym::<FnReverseCollide>("reverse_collide");
    let vals = [
        0.0f32,
        -0.0,
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::EPSILON,
        1e30,
        -1e30,
        -70.0,
        -20.0,
        20.0,
    ];
    for &x in vals.iter() {
        for &y in vals.iter() {
            for &rad in vals.iter() {
                unsafe {
                    same(
                        "ERRORS row40 reverse_collide non-finite",
                        &format!("{x:?} {y:?} {rad:?}"),
                        c(x, y, rad),
                        r(x, y, rad),
                    )
                }
            }
        }
    }
    // every NaN payload must behave the same
    for bits in [0x7FC0_0000u32, 0x7FC0_0001, 0xFFC0_0000, 0x7F80_0001, 0xFF80_0001] {
        let v = f32::from_bits(bits);
        unsafe {
            same(
                "ERRORS row40 reverse_collide NaN payloads",
                &format!("bits=0x{bits:08x}"),
                c(v, v, v),
                r(v, v, v),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks required by Phase C, beyond the table:
// out-of-range enum values everywhere they are accepted, zero/oversized
// lengths, and one-step-past-range values.
// ---------------------------------------------------------------------------

#[test]
fn err_enum_boundary_values_everywhere() {
    let (cmp, rmp) = sym::<FnMakeProxy>("c2MakeProxy");
    let (ccol, rcol) = sym::<FnCollided>("c2Collided");
    let mut rng = Rng::new(SEED ^ 0x11B);
    // one step past each end of the valid range, plus the wrap-around values
    let boundary: [C2_TYPE; 6] = [3, 0xFFFF_FFFF, 0x8000_0000, 0x7FFF_FFFF, 4, 2];
    for &t in boundary.iter() {
        for _ in 0..200 {
            let shape = rng.capsule();
            let sentinel = c2Proxy {
                radius: 3.5,
                count: 77,
                verts: [c2v { x: 1.5, y: 2.5 }; 8],
            };
            let mut pc = sentinel;
            let mut pr = sentinel;
            unsafe {
                cmp(&shape as *const c2Capsule as *const c_void, t, &mut pc);
                rmp(&shape as *const c2Capsule as *const c_void, t, &mut pr);
            }
            same("ERRORS generic c2MakeProxy enum boundary", &format!("type={t}"), pc, pr);

            let a = rng.circle();
            let b = rng.circle();
            unsafe {
                same(
                    "ERRORS generic c2Collided enum boundary",
                    &format!("type={t}"),
                    ccol(
                        &a as *const c2Circle as *const c_void,
                        t,
                        &b as *const c2Circle as *const c_void,
                        t,
                    ),
                    rcol(
                        &a as *const c2Circle as *const c_void,
                        t,
                        &b as *const c2Circle as *const c_void,
                        t,
                    ),
                )
            }
        }
    }
}

#[test]
fn err_simplex_count_exhaustive_all_consumers() {
    // Sweep the simplex `count` field one step past every documented value for
    // every function that switches on it.
    let (cmet, rmet) = sym::<FnSimplexF>("c2GJKSimplexMetric");
    let (cd, rd) = sym::<FnSimplexV>("c2D");
    let (cl, rl) = sym::<FnSimplexV>("c2L");
    let (cw, rw) = sym::<FnWitness>("c2Witness");
    let (c22c, c22r) = sym::<FnSimplexVoid>("c22");
    let (c23c, c23r) = sym::<FnSimplexVoid>("c23");
    let mut rng = Rng::new(SEED ^ 0x11C);
    for count in -3i32..=6 {
        for _ in 0..300 {
            let s = rng.simplex(count);
            unsafe {
                let (mut a, mut b) = (s, s);
                same(
                    "ERRORS generic c2GJKSimplexMetric count sweep",
                    &format!("count={count}"),
                    cmet(&mut a),
                    rmet(&mut b),
                );
                let (mut a, mut b) = (s, s);
                same("ERRORS generic c2D count sweep", &format!("count={count}"), cd(&mut a), rd(&mut b));
                let (mut a, mut b) = (s, s);
                same("ERRORS generic c2L count sweep", &format!("count={count}"), cl(&mut a), rl(&mut b));
                let (mut a, mut b) = (s, s);
                let mut wa1 = c2v { x: 5.0, y: 6.0 };
                let mut wb1 = c2v { x: 7.0, y: 8.0 };
                let mut wa2 = wa1;
                let mut wb2 = wb1;
                cw(&mut a, &mut wa1, &mut wb1);
                rw(&mut b, &mut wa2, &mut wb2);
                same(
                    "ERRORS generic c2Witness count sweep",
                    &format!("count={count}"),
                    (wa1, wb1),
                    (wa2, wb2),
                );
                // c22 / c23 read verts[0..1] / verts[0..2] regardless of count,
                // so they are safe to call at any count value.
                let (mut a, mut b) = (s, s);
                c22c(&mut a);
                c22r(&mut b);
                same("ERRORS generic c22 count sweep", &format!("count={count}"), a, b);
                let (mut a, mut b) = (s, s);
                c23c(&mut a);
                c23r(&mut b);
                same("ERRORS generic c23 count sweep", &format!("count={count}"), a, b);
            }
        }
    }
}

#[test]
fn err_support_count_oversized() {
    // `count` larger than the buffer the caller actually provides would be
    // out-of-bounds, so the buffer is sized to the largest count tested (the
    // C code has no upper bound check of its own).
    let (c, r) = sym::<FnSupport>("c2Support");
    let mut rng = Rng::new(SEED ^ 0x11D);
    let mut verts = vec![c2v::default(); 4096];
    for v in verts.iter_mut() {
        *v = rng.wild_vec();
    }
    for &count in &[1i32, 2, 3, 4, 5, 8, 9, 64, 1024, 4096] {
        for _ in 0..50 {
            let d = rng.wild_vec();
            let (cv, rv) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            same(
                "ERRORS generic c2Support oversized count",
                &format!("count={count} d={d:?}"),
                cv,
                rv,
            );
        }
    }
}
