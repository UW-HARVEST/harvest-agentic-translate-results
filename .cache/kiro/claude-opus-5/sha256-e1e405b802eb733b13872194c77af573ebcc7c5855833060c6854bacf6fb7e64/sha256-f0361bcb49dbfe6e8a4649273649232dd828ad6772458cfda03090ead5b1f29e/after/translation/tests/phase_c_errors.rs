//! Phase C — one differential test per row of `ERRORS.md` (rows 1..52).
//!
//! Each test constructs the exact rejecting/degenerate condition, calls BOTH
//! `.so`s, and asserts the *same* sentinel/error value comes back — not merely
//! "both did something".

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Enum values that are out of range for `C2_TYPE` but perfectly legal `int`s
/// to pass across the FFI boundary.
const BAD_TYPES: [c_int; 10] = [
    3,
    4,
    -1,
    -2,
    100,
    255,
    256,
    65536,
    c_int::MAX,
    c_int::MIN,
];

fn sample_shapes(rng: &mut Rng) -> (Shape, Shape, Shape) {
    (
        Shape::Circle(rng.circle()),
        Shape::Aabb(rng.aabb()),
        Shape::Capsule(rng.capsule()),
    )
}

// =========================================================================
// Rows 1..4 — c2Collided: out-of-range C2_TYPE => return 0
// =========================================================================

#[test]
fn err01_collided_bad_typeA() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(1);
    for _ in 0..200 {
        let (c, b, k) = sample_shapes(&mut rng);
        for a in [&c, &b, &k] {
            for bt in BAD_TYPES {
                for (_, tb) in TYPE_PAIRS {
                    let bs = rand_shape(&mut rng, tb);
                    let cv = unsafe { cf(a.ptr(), bt, bs.ptr(), tb) };
                    let rv = unsafe { rf(a.ptr(), bt, bs.ptr(), tb) };
                    assert_same(&format!("c2Collided typeA={bt}"), cv, rv);
                    assert_eq!(cv, 0, "C must reject typeA={bt} with 0");
                }
            }
        }
    }
}

#[test]
fn err02_collided_circle_bad_typeB() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(2);
    for _ in 0..400 {
        let a = Shape::Circle(rng.circle());
        let b = Shape::Circle(rng.circle());
        for bt in BAD_TYPES {
            let cv = unsafe { cf(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), bt) };
            let rv = unsafe { rf(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), bt) };
            assert_same(&format!("c2Collided CIRCLE/typeB={bt}"), cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

#[test]
fn err03_collided_aabb_bad_typeB() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(3);
    for _ in 0..400 {
        let a = Shape::Aabb(rng.aabb());
        let b = Shape::Aabb(rng.aabb());
        for bt in BAD_TYPES {
            let cv = unsafe { cf(a.ptr(), C2_TYPE_AABB, b.ptr(), bt) };
            let rv = unsafe { rf(a.ptr(), C2_TYPE_AABB, b.ptr(), bt) };
            assert_same(&format!("c2Collided AABB/typeB={bt}"), cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

#[test]
fn err04_collided_capsule_bad_typeB() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(4);
    for _ in 0..400 {
        let a = Shape::Capsule(rng.capsule());
        let b = Shape::Capsule(rng.capsule());
        for bt in BAD_TYPES {
            let cv = unsafe { cf(a.ptr(), C2_TYPE_CAPSULE, b.ptr(), bt) };
            let rv = unsafe { rf(a.ptr(), C2_TYPE_CAPSULE, b.ptr(), bt) };
            assert_same(&format!("c2Collided CAPSULE/typeB={bt}"), cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

// =========================================================================
// Row 5 — c2MakeProxy with an out-of-range type leaves *p untouched
// =========================================================================

#[test]
fn err05_makeproxy_bad_type() {
    let (cf, rf) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(5);
    let seed = |i: u32| c2Proxy {
        radius: f32::from_bits(0x4242_0000 + i),
        count: -31337,
        verts: std::array::from_fn(|k| c2v {
            x: f32::from_bits(0x3333_0000 + k as u32),
            y: f32::from_bits(0x4444_0000 + k as u32),
        }),
    };
    for i in 0..200u32 {
        let (c, b, k) = sample_shapes(&mut rng);
        for s in [&c, &b, &k] {
            for bt in BAD_TYPES {
                let mut cp = seed(i);
                let mut rp = seed(i);
                unsafe {
                    cf(s.ptr(), bt, &raw mut cp);
                    rf(s.ptr(), bt, &raw mut rp);
                }
                assert_same(&format!("c2MakeProxy type={bt}"), cp, rp);
                // The C `switch` has no `default:` arm: nothing is written.
                assert_same("proxy must be untouched", cp, seed(i));
            }
        }
    }
}

// =========================================================================
// Rows 6..7 — NULL transform pointers substitute c2xIdentity
// =========================================================================

#[test]
fn err06_gjk_null_ax() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(6);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..100 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                let c_null = unsafe {
                    call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, Some(&ident), ur, None)
                };
                let r_null = unsafe {
                    call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, Some(&ident), ur, None)
                };
                assert_same("c2GJK ax=NULL", c_null, r_null);
                // NULL must behave exactly like an explicit identity.
                let c_id = unsafe {
                    call_gjk(&cf, a.ptr(), ta, Some(&ident), b.ptr(), tb, Some(&ident), ur, None)
                };
                assert_same("ax=NULL == ax=identity (C)", c_null, c_id);
            }
        }
    }
}

#[test]
fn err07_gjk_null_bx() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(7);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..100 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                let c_null = unsafe {
                    call_gjk(&cf, a.ptr(), ta, Some(&ident), b.ptr(), tb, None, ur, None)
                };
                let r_null = unsafe {
                    call_gjk(&rf, a.ptr(), ta, Some(&ident), b.ptr(), tb, None, ur, None)
                };
                assert_same("c2GJK bx=NULL", c_null, r_null);
                let c_id = unsafe {
                    call_gjk(&cf, a.ptr(), ta, Some(&ident), b.ptr(), tb, Some(&ident), ur, None)
                };
                assert_same("bx=NULL == bx=identity (C)", c_null, c_id);
            }
        }
    }
}

// =========================================================================
// Rows 8..11 — NULL out-parameters must not be written
// =========================================================================

/// Call `c2GJK` with a chosen subset of out-parameters set to NULL. The
/// non-NULL ones are pre-seeded so an unexpected write is visible; the NULL
/// ones must simply not be touched (verified by the call returning normally
/// and the remaining outputs still matching).
#[allow(clippy::too_many_arguments)]
unsafe fn gjk_with_nulls(
    f: &FnGJK,
    a: &Shape,
    b: &Shape,
    ur: c_int,
    null_outA: bool,
    null_outB: bool,
    null_iters: bool,
    null_cache: bool,
) -> (f32, c2v, c2v, c_int, c2GJKCache) {
    let mut outA = c2v {
        x: f32::from_bits(0xDEAD_0001),
        y: f32::from_bits(0xDEAD_0002),
    };
    let mut outB = c2v {
        x: f32::from_bits(0xDEAD_0003),
        y: f32::from_bits(0xDEAD_0004),
    };
    let mut iters: c_int = -777;
    let mut cache = c2GJKCache {
        metric: 1.5,
        count: 0,
        iA: [7, 8, 9],
        iB: [10, 11, 12],
        div: 2.5,
    };
    let dist = unsafe {
        f(
            a.ptr(),
            a.ty(),
            std::ptr::null(),
            b.ptr(),
            b.ty(),
            std::ptr::null(),
            if null_outA { std::ptr::null_mut() } else { &raw mut outA },
            if null_outB { std::ptr::null_mut() } else { &raw mut outB },
            ur,
            if null_iters { std::ptr::null_mut() } else { &raw mut iters },
            if null_cache { std::ptr::null_mut() } else { &raw mut cache },
        )
    };
    (dist, outA, outB, iters, cache)
}

fn null_outparam_row(seed: u64, na: bool, nb: bool, ni: bool, nc: bool, label: &str) {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(seed);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..150 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                let cv = unsafe { gjk_with_nulls(&cf, &a, &b, ur, na, nb, ni, nc) };
                let rv = unsafe { gjk_with_nulls(&rf, &a, &b, ur, na, nb, ni, nc) };
                assert_same(&format!("{label} dist"), cv.0, rv.0);
                assert_same(&format!("{label} outA"), cv.1, rv.1);
                assert_same(&format!("{label} outB"), cv.2, rv.2);
                assert_same(&format!("{label} iters"), cv.3, rv.3);
                assert_same(&format!("{label} cache"), cv.4, rv.4);
                // The NULL-ed slots must still hold their sentinels.
                if na {
                    assert_eq!(cv.1.x.to_bits(), 0xDEAD_0001);
                    assert_eq!(rv.1.x.to_bits(), 0xDEAD_0001);
                }
                if nb {
                    assert_eq!(cv.2.x.to_bits(), 0xDEAD_0003);
                    assert_eq!(rv.2.x.to_bits(), 0xDEAD_0003);
                }
                if ni {
                    assert_eq!(cv.3, -777);
                    assert_eq!(rv.3, -777);
                }
                if nc {
                    assert_eq!(cv.4.iA, [7, 8, 9]);
                    assert_eq!(rv.4.iA, [7, 8, 9]);
                }
            }
        }
    }
}

#[test]
fn err08_gjk_null_outA() {
    null_outparam_row(8, true, false, false, false, "outA=NULL");
}

#[test]
fn err09_gjk_null_outB() {
    null_outparam_row(9, false, true, false, false, "outB=NULL");
}

#[test]
fn err10_gjk_null_iterations() {
    null_outparam_row(10, false, false, true, false, "iterations=NULL");
}

#[test]
fn err11_gjk_null_cache() {
    null_outparam_row(11, false, false, false, true, "cache=NULL");
    // All four NULL at once.
    null_outparam_row(11, true, true, true, true, "all outputs NULL");
}

// =========================================================================
// Rows 12..15 — cache-validity guard in c2GJK
// =========================================================================

fn cache_row(seed: u64, metric: f32, count: c_int, label: &str) {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(seed);
    let vc = |t: c_int| match t {
        C2_TYPE_CIRCLE => 1u32,
        C2_TYPE_CAPSULE => 2,
        _ => 4,
    };
    for (ta, tb) in TYPE_PAIRS {
        let (na, nb) = (vc(ta), vc(tb));
        for _ in 0..120 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            let mut cache = c2GJKCache {
                metric,
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: rng.range(-4.0, 4.0),
            };
            for k in 0..3 {
                cache.iA[k] = (rng.next_u32() % na) as c_int;
                cache.iB[k] = (rng.next_u32() % nb) as c_int;
            }
            for ur in [0, 1] {
                let cv = unsafe {
                    call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, ur, Some(cache))
                };
                let rv = unsafe {
                    call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, None, ur, Some(cache))
                };
                assert_same(label, cv, rv);
            }
        }
    }
}

#[test]
fn err12_gjk_zero_count_cache() {
    // cache_was_good == 0 -> the cache is NOT read, but IS written back.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(12);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..150 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            // count == 0 but every other field garbage: must be ignored.
            let cache = c2GJKCache {
                metric: rng.range(-1e9, 1e9),
                count: 0,
                iA: [111, 222, 333],
                iB: [-1, -2, -3],
                div: rng.range(-1e9, 1e9),
            };
            for ur in [0, 1] {
                let cv = unsafe {
                    call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, ur, Some(cache))
                };
                let rv = unsafe {
                    call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, None, ur, Some(cache))
                };
                assert_same("cache count=0", cv, rv);
                // Must match the NULL-cache result apart from the write-back.
                let cn = unsafe {
                    call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None)
                };
                assert_same("count=0 cache == NULL cache dist", cv.dist, cn.dist);
                assert_same("count=0 cache == NULL cache outA", cv.outA, cn.outA);
                assert_same("count=0 cache == NULL cache outB", cv.outB, cn.outB);
                assert_same("count=0 cache == NULL cache iters", cv.iterations, cn.iterations);
            }
        }
    }
}

#[test]
fn err13_gjk_warm_cache() {
    for count in [1, 2, 3] {
        cache_row(1300 + count as u64, 0.0, count, "warm cache metric=0");
        cache_row(1310 + count as u64, 12.5, count, "warm cache metric=12.5");
        cache_row(1320 + count as u64, -12.5, count, "warm cache metric=-12.5");
    }
}

#[test]
fn err14_gjk_nan_cache_metric() {
    for count in [1, 2, 3] {
        cache_row(1400 + count as u64, f32::NAN, count, "cache metric=NaN");
        cache_row(1410 + count as u64, f32::INFINITY, count, "cache metric=+Inf");
        cache_row(1420 + count as u64, f32::NEG_INFINITY, count, "cache metric=-Inf");
    }
}

#[test]
fn err15_gjk_huge_negative_cache_metric() {
    for count in [1, 2, 3] {
        cache_row(1500 + count as u64, -1e30, count, "cache metric=-1e30");
        cache_row(1510 + count as u64, -1e8 - 1.0, count, "metric just past -1e8");
        cache_row(1520 + count as u64, -1e8 + 1.0, count, "metric just under -1e8");
        cache_row(1530 + count as u64, f32::MAX, count, "cache metric=FLT_MAX");
        cache_row(1540 + count as u64, -f32::MAX, count, "cache metric=-FLT_MAX");
    }
}

// =========================================================================
// Rows 16..17 — c2Witness
// =========================================================================

#[test]
fn err16_witness_bad_count() {
    let (cf, rf) = pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(16);
    for count in [0, 4, -1, -1000, 5, 99, c_int::MAX, c_int::MIN] {
        for _ in 0..400 {
            let s = rng.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let sentinel = c2v {
                x: f32::from_bits(0xABCD_0001),
                y: f32::from_bits(0xABCD_0002),
            };
            let (mut ca, mut cb, mut ra, mut rb) = (sentinel, sentinel, sentinel, sentinel);
            unsafe {
                cf(&raw mut cs, &raw mut ca, &raw mut cb);
                rf(&raw mut rs, &raw mut ra, &raw mut rb);
            }
            assert_same(&format!("c2Witness count={count} a"), ca, ra);
            assert_same(&format!("c2Witness count={count} b"), cb, rb);
            // `default:` writes exactly c2V(0,0) to both.
            assert_eq!(ca.x.to_bits(), 0, "C default arm must write +0.0");
            assert_eq!(cb.y.to_bits(), 0);
        }
    }
}

#[test]
fn err17_witness_zero_div() {
    let (cf, rf) = pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(17);
    for count in [1, 2, 3] {
        for div in [0.0f32, -0.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE, f32::NAN] {
            for _ in 0..400 {
                let mut s = rng.simplex(count);
                s.div = div;
                let mut cs = s;
                let mut rs = s;
                let (mut ca, mut cb, mut ra, mut rb) =
                    (c2v::default(), c2v::default(), c2v::default(), c2v::default());
                unsafe {
                    cf(&raw mut cs, &raw mut ca, &raw mut cb);
                    rf(&raw mut rs, &raw mut ra, &raw mut rb);
                }
                assert_same(&format!("c2Witness div={div} a"), ca, ra);
                assert_same(&format!("c2Witness div={div} b"), cb, rb);
            }
        }
    }
}

// =========================================================================
// Rows 18..21 — c2GJKSimplexMetric / c2L / c2D out-of-range count
// =========================================================================

#[test]
fn err18_metric_bad_count() {
    let (cf, rf) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(18);
    for count in [0, 1, 4, 5, -1, -99, c_int::MAX, c_int::MIN] {
        for _ in 0..500 {
            let s = rng.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&raw mut cs) };
            let rv = unsafe { rf(&raw mut rs) };
            assert_same(&format!("metric count={count}"), cv, rv);
            assert_eq!(cv.to_bits(), 0, "default/case 1 must return +0.0f");
        }
    }
}

#[test]
fn err19_c2L_bad_count() {
    let (cf, rf) = pair::<FnSimplexV>("c2L");
    let mut rng = Rng::new(19);
    for count in [0, 3, 4, 5, -1, -99, c_int::MAX, c_int::MIN] {
        for _ in 0..500 {
            let s = rng.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&raw mut cs) };
            let rv = unsafe { rf(&raw mut rs) };
            assert_same(&format!("c2L count={count}"), cv, rv);
            assert_eq!(cv.x.to_bits(), 0);
            assert_eq!(cv.y.to_bits(), 0);
        }
    }
}

#[test]
fn err20_c2L_zero_div() {
    let (cf, rf) = pair::<FnSimplexV>("c2L");
    let mut rng = Rng::new(20);
    for div in [0.0f32, -0.0, f32::NAN, f32::MIN_POSITIVE] {
        for _ in 0..800 {
            let mut s = rng.simplex(2);
            s.div = div;
            let mut cs = s;
            let mut rs = s;
            assert_same(
                &format!("c2L div={div}"),
                unsafe { cf(&raw mut cs) },
                unsafe { rf(&raw mut rs) },
            );
        }
    }
}

#[test]
fn err21_c2D_bad_count() {
    let (cf, rf) = pair::<FnSimplexV>("c2D");
    let mut rng = Rng::new(21);
    for count in [0, 3, 4, 5, -1, -99, c_int::MAX, c_int::MIN] {
        for _ in 0..500 {
            let s = rng.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&raw mut cs) };
            let rv = unsafe { rf(&raw mut rs) };
            assert_same(&format!("c2D count={count}"), cv, rv);
            assert_eq!(cv.x.to_bits(), 0);
            assert_eq!(cv.y.to_bits(), 0);
        }
    }
}

// =========================================================================
// Rows 22..24 — c2Support
// =========================================================================

#[test]
fn err22_support_nonpositive_count() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(22);
    for count in [0, -1, -2, -1000, c_int::MIN] {
        for _ in 0..500 {
            let verts: [c2v; 8] = std::array::from_fn(|_| rng.vec());
            let d = rng.vec();
            let cv = unsafe { cf(verts.as_ptr(), count, d) };
            let rv = unsafe { rf(verts.as_ptr(), count, d) };
            assert_same(&format!("c2Support count={count}"), cv, rv);
            assert_eq!(cv, 0, "must return index 0");
        }
    }
}

#[test]
fn err23_support_tie() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(23);
    for _ in 0..500 {
        let v = rng.vec();
        let verts = [v; 8];
        for count in 1..=8 {
            for d in [rng.vec(), c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }] {
                let cv = unsafe { cf(verts.as_ptr(), count, d) };
                let rv = unsafe { rf(verts.as_ptr(), count, d) };
                assert_same("c2Support all-equal verts", cv, rv);
                assert_eq!(cv, 0, "ties resolve to the first index");
            }
        }
    }
}

#[test]
fn err24_support_nan() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(24);
    let nanv = c2v { x: f32::NAN, y: f32::NAN };
    for _ in 0..500 {
        let mut verts: [c2v; 8] = std::array::from_fn(|_| rng.vec());
        for count in 1..=8 {
            // NaN direction: every `dot > dmax` is false -> index 0.
            let cv = unsafe { cf(verts.as_ptr(), count, nanv) };
            let rv = unsafe { rf(verts.as_ptr(), count, nanv) };
            assert_same("c2Support NaN direction", cv, rv);
            assert_eq!(cv, 0);
        }
        // NaN in verts[0] makes dmax NaN, so nothing can beat it either.
        verts[0] = nanv;
        for count in 1..=8 {
            let d = rng.vec();
            let cv = unsafe { cf(verts.as_ptr(), count, d) };
            let rv = unsafe { rf(verts.as_ptr(), count, d) };
            assert_same("c2Support NaN verts[0]", cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

// =========================================================================
// Rows 25..27 — division / sqrt degeneracies
// =========================================================================

#[test]
fn err25_div_by_zero() {
    let (cf, rf) = pair::<extern "C" fn(c2v, f32) -> c2v>("c2Div");
    let mut rng = Rng::new(25);
    for b in [0.0f32, -0.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE, 1e-45, -1e-45] {
        for _ in 0..500 {
            let a = rng.vec();
            assert_same(&format!("c2Div b={b}"), cf(a, b), rf(a, b));
        }
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            c2v { x: f32::NAN, y: 1.0 },
            c2v { x: f32::MAX, y: -f32::MAX },
        ] {
            assert_same("c2Div edge", cf(a, b), rf(a, b));
        }
    }
}

#[test]
fn err26_norm_zero_vector() {
    let (cf, rf) = pair::<FnVV>("c2Norm");
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
        c2v { x: 1e-30, y: 1e-30 },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NAN, y: f32::NAN },
    ] {
        assert_same("c2Norm degenerate", cf(a), rf(a));
    }
    let mut rng = Rng::new(26);
    for _ in 0..2000 {
        let a = rng.spicy_vec();
        assert_same("c2Norm spicy", cf(a), rf(a));
    }
}

#[test]
fn err27_len_nonfinite() {
    let (cf, rf) = pair::<FnVf>("c2Len");
    for a in [
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: 0.0, y: f32::NAN },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: 1e30, y: 1e30 },
    ] {
        assert_same("c2Len nonfinite", cf(a), rf(a));
    }
    let mut rng = Rng::new(27);
    for _ in 0..2000 {
        let a = rng.spicy_vec();
        assert_same("c2Len spicy", cf(a), rf(a));
    }
}

// =========================================================================
// Rows 28..29 — ternary min/max and inverted clamp ranges
// =========================================================================

#[test]
fn err28_minmax_nan() {
    let (c_max, r_max) = pair::<FnVVV>("c2Maxv");
    let (c_min, r_min) = pair::<FnVVV>("c2Minv");
    let nan = f32::NAN;
    let cases: [(c2v, c2v); 6] = [
        (c2v { x: nan, y: nan }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: nan, y: nan }),
        (c2v { x: nan, y: 2.0 }, c2v { x: 3.0, y: nan }),
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        (c2v { x: f32::INFINITY, y: nan }, c2v { x: nan, y: f32::NEG_INFINITY }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
    ];
    for (a, b) in cases {
        // `c2Maxv(a,b)` is `a.x > b.x ? a.x : b.x`, so a NaN comparison yields b.
        assert_same("c2Maxv NaN", c_max(a, b), r_max(a, b));
        assert_same("c2Minv NaN", c_min(a, b), r_min(a, b));
    }
    // Signed zero: `0.0 > -0.0` is false, so c2Maxv(0.0,-0.0) == -0.0.
    let pz = c2v { x: 0.0, y: 0.0 };
    let nz = c2v { x: -0.0, y: -0.0 };
    let z = c_max(pz, nz);
    assert_eq!(z.x.to_bits(), 0x8000_0000, "C must return -0.0 here");
    assert_same("c2Maxv signed zero", z, r_max(pz, nz));
}

#[test]
fn err29_clampv_inverted() {
    let (cf, rf) = pair::<extern "C" fn(c2v, c2v, c2v) -> c2v>("c2Clampv");
    let mut rng = Rng::new(29);
    for _ in 0..3000 {
        let a = rng.vec();
        let lo = rng.vec();
        let hi = rng.vec();
        // Deliberately unordered: lo may exceed hi.
        assert_same("c2Clampv unordered", cf(a, lo, hi), rf(a, lo, hi));
        assert_same("c2Clampv reversed", cf(a, hi, lo), rf(a, hi, lo));
    }
    for _ in 0..2000 {
        let a = rng.spicy_vec();
        let lo = rng.spicy_vec();
        let hi = rng.spicy_vec();
        assert_same("c2Clampv spicy", cf(a, lo, hi), rf(a, lo, hi));
    }
}

// =========================================================================
// Rows 30..32 — inverted boxes / negative radii in the boolean helpers
// =========================================================================

#[test]
fn err30_circle_aabb_inverted() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2AABB) -> c_int>("c2CircletoAABB");
    let mut rng = Rng::new(30);
    for _ in 0..3000 {
        let bb = rng.aabb();
        let inv = c2AABB { min: bb.max, max: bb.min };
        let a = rng.circle();
        assert_same("c2CircletoAABB inverted", cf(a, inv), rf(a, inv));
        let deg = c2AABB { min: bb.min, max: bb.min };
        assert_same("c2CircletoAABB degenerate", cf(a, deg), rf(a, deg));
        let mixed = c2AABB {
            min: c2v { x: bb.max.x, y: bb.min.y },
            max: c2v { x: bb.min.x, y: bb.max.y },
        };
        assert_same("c2CircletoAABB half-inverted", cf(a, mixed), rf(a, mixed));
    }
}

#[test]
fn err31_aabb_aabb_inverted() {
    let (cf, rf) = pair::<extern "C" fn(c2AABB, c2AABB) -> c_int>("c2AABBtoAABB");
    let mut rng = Rng::new(31);
    for _ in 0..4000 {
        let a = rng.aabb();
        let b = rng.aabb();
        let ia = c2AABB { min: a.max, max: a.min };
        let ib = c2AABB { min: b.max, max: b.min };
        assert_same("both inverted", cf(ia, ib), rf(ia, ib));
        assert_same("A inverted", cf(ia, b), rf(ia, b));
        assert_same("B inverted", cf(a, ib), rf(a, ib));
    }
}

#[test]
fn err32_circle_circle_negative_radius() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2Circle) -> c_int>("c2CircletoCircle");
    let mut rng = Rng::new(32);
    for _ in 0..4000 {
        let a = c2Circle { p: rng.vec(), r: -rng.radius() };
        let b = c2Circle { p: rng.vec(), r: -rng.radius() };
        assert_same("both negative r", cf(a, b), rf(a, b));
        let c = c2Circle { p: rng.vec(), r: rng.radius() };
        assert_same("mixed sign r", cf(a, c), rf(a, c));
        // rA + rB == 0 exactly: r2 == 0 so `d2 < 0` is always false.
        let d = c2Circle { p: rng.vec(), r: -a.r };
        assert_same("radii cancel", cf(a, d), rf(a, d));
        assert_eq!(cf(a, d), 0, "r2 == 0 must reject");
    }
}

// =========================================================================
// Rows 33..34 — degenerate capsule axis
// =========================================================================

#[test]
fn err33_circle_capsule_degenerate() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2Capsule) -> c_int>("c2CircletoCapsule");
    let mut rng = Rng::new(33);
    for _ in 0..4000 {
        let p = rng.vec();
        let cap = c2Capsule { a: p, b: p, r: rng.radius() };
        let a = rng.circle();
        assert_same("degenerate capsule", cf(a, cap), rf(a, cap));
        let zero = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: -0.0, y: -0.0 },
            r: 0.0,
        };
        assert_same("zero capsule", cf(a, zero), rf(a, zero));
    }
}

#[test]
fn err34_circle_capsule_tiny_axis() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2Capsule) -> c_int>("c2CircletoCapsule");
    let mut rng = Rng::new(34);
    for eps in [1e-22f32, 1e-30, 1e-38, f32::MIN_POSITIVE, 1e-45] {
        for _ in 0..800 {
            let p = rng.vec();
            let cap = c2Capsule {
                a: p,
                b: c2v { x: p.x + eps, y: p.y + eps },
                r: rng.radius(),
            };
            let a = rng.circle();
            assert_same(&format!("tiny axis eps={eps}"), cf(a, cap), rf(a, cap));
        }
    }
    // Huge axis: dot(n,n) overflows to +Inf.
    for _ in 0..800 {
        let cap = c2Capsule {
            a: c2v { x: -1e30, y: -1e30 },
            b: c2v { x: 1e30, y: 1e30 },
            r: rng.radius(),
        };
        let a = rng.circle();
        assert_same("huge axis", cf(a, cap), rf(a, cap));
    }
    // Non-finite capsule axis.
    for _ in 0..2000 {
        let cap = c2Capsule {
            a: rng.spicy_vec(),
            b: rng.spicy_vec(),
            r: rng.spicy_f32(),
        };
        let a = c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() };
        assert_same("nonfinite capsule", cf(a, cap), rf(a, cap));
    }
}

// =========================================================================
// Rows 35..45 — c2GJK loop-termination and radius branches
// =========================================================================

#[test]
fn err35_gjk_iteration_cap() {
    // `while (iter < 20)`: the loop can never report more than 20 iterations,
    // and both libraries must report the SAME count for the same input.
    //
    // The three shape kinds give proxies of at most 4 vertices, so the number
    // of distinct support points the loop can add before the `dup` break fires
    // is small; the measured maximum below is 3 and the `iter == 20` cap is
    // unreachable through the public API. The differential assertion still
    // covers the cap expression itself (both libraries must agree on the count
    // for every input), and the search below is deliberately broad —
    // transforms, warm caches, extreme scales — so that any input that COULD
    // drive the count higher is exercised.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(35);
    let mut max_iter = 0;
    let mut hist = [0usize; 21];
    let vc = |t: c_int| match t {
        C2_TYPE_CIRCLE => 1u32,
        C2_TYPE_CAPSULE => 2,
        _ => 4,
    };
    for (ta, tb) in TYPE_PAIRS {
        let (na, nb) = (vc(ta), vc(tb));
        for _ in 0..800 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            let ax = rng.xform();
            let bx = rng.xform();
            let mut warm = c2GJKCache {
                metric: rng.range(-1e6, 1e6),
                count: 1 + (rng.next_u32() % 3) as c_int,
                iA: [0; 3],
                iB: [0; 3],
                div: rng.range(-5.0, 5.0),
            };
            for k in 0..3 {
                warm.iA[k] = (rng.next_u32() % na) as c_int;
                warm.iB[k] = (rng.next_u32() % nb) as c_int;
            }
            for (axo, bxo) in [(None, None), (Some(&ax), Some(&bx))] {
                for cache in [None, Some(c2GJKCache::default()), Some(warm)] {
                    for ur in [0, 1] {
                        let cv = unsafe {
                            call_gjk(&cf, a.ptr(), ta, axo, b.ptr(), tb, bxo, ur, cache)
                        };
                        let rv = unsafe {
                            call_gjk(&rf, a.ptr(), ta, axo, b.ptr(), tb, bxo, ur, cache)
                        };
                        assert_same("iteration cap", cv, rv);
                        assert!(
                            (0..=20).contains(&cv.iterations),
                            "C reported iterations={} outside [0,20]",
                            cv.iterations
                        );
                        max_iter = max_iter.max(cv.iterations);
                        hist[cv.iterations as usize] += 1;
                    }
                }
            }
        }
    }
    eprintln!("err35 iteration histogram {hist:?} (max observed {max_iter})");
    assert!(max_iter >= 1, "the loop never iterated at all");
}

#[test]
fn err36_gjk_duplicate_support() {
    // circle-vs-circle: both proxies have a single vertex, so the support
    // point on the first pass is necessarily (iA,iB) == (0,0), which is
    // already in saveA/saveB -> the `dup` break fires with iter == 0.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(36);
    for _ in 0..3000 {
        let mut a = rng.circle();
        let mut b = rng.circle();
        if a.p.x == b.p.x && a.p.y == b.p.y {
            b.p.x += 1.0;
        }
        a.r = 0.0;
        b.r = 0.0;
        let sa = Shape::Circle(a);
        let sb = Shape::Circle(b);
        let cv = unsafe {
            call_gjk(&cf, sa.ptr(), C2_TYPE_CIRCLE, None, sb.ptr(), C2_TYPE_CIRCLE, None, 0, None)
        };
        let rv = unsafe {
            call_gjk(&rf, sa.ptr(), C2_TYPE_CIRCLE, None, sb.ptr(), C2_TYPE_CIRCLE, None, 0, None)
        };
        assert_same("dup-support break", cv, rv);
        assert_eq!(cv.iterations, 0, "dup break must happen on the first pass");
    }
    // Repeated identical vertices in a capsule likewise force duplicates.
    for _ in 0..3000 {
        let p = rng.vec();
        let cap = c2Capsule { a: p, b: p, r: rng.radius() };
        let sa = Shape::Capsule(cap);
        let sb = Shape::Circle(rng.circle());
        for ur in [0, 1] {
            let cv = unsafe {
                call_gjk(&cf, sa.ptr(), C2_TYPE_CAPSULE, None, sb.ptr(), C2_TYPE_CIRCLE, None, ur, None)
            };
            let rv = unsafe {
                call_gjk(&rf, sa.ptr(), C2_TYPE_CAPSULE, None, sb.ptr(), C2_TYPE_CIRCLE, None, ur, None)
            };
            assert_same("dup-support degenerate capsule", cv, rv);
        }
    }
}

#[test]
fn err37_gjk_d1_gt_d0() {
    // `if (d1 > d0) break;` — reachable only when the barycentric distance
    // stops decreasing, which needs adversarial / non-finite geometry. Both
    // libraries must agree over the whole sweep regardless of which break wins.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(37);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..600 {
            // Extreme and near-degenerate shapes maximise the chance of a
            // non-monotone distance sequence.
            let scale = [1e-30f32, 1e-8, 1.0, 1e8, 1e30][(rng.next_u32() % 5) as usize];
            let mk = |ty: c_int, rng: &mut Rng| -> Shape {
                match ty {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                        p: c2v { x: rng.coord() * scale, y: rng.coord() * scale },
                        r: rng.radius() * scale,
                    }),
                    C2_TYPE_AABB => {
                        let p = c2v { x: rng.coord() * scale, y: rng.coord() * scale };
                        Shape::Aabb(c2AABB {
                            min: p,
                            max: c2v { x: p.x + rng.radius() * scale, y: p.y },
                        })
                    }
                    _ => {
                        let p = c2v { x: rng.coord() * scale, y: rng.coord() * scale };
                        Shape::Capsule(c2Capsule { a: p, b: p, r: rng.radius() * scale })
                    }
                }
            };
            let a = mk(ta, &mut rng);
            let b = mk(tb, &mut rng);
            for ur in [0, 1] {
                let cv = unsafe { call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None) };
                let rv = unsafe { call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None) };
                assert_same("d1 > d0 sweep", cv, rv);
            }
        }
    }
}

#[test]
fn err38_gjk_tiny_direction() {
    // Coincident shapes give s.a.p == (0,0), so c2D returns (0,0) and
    // `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` breaks immediately.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(38);
    for _ in 0..2000 {
        let p = rng.vec();
        let shapes = [
            Shape::Circle(c2Circle { p, r: rng.radius() }),
            Shape::Aabb(c2AABB { min: p, max: c2v { x: p.x + 10.0, y: p.y + 10.0 } }),
            Shape::Capsule(c2Capsule { a: p, b: c2v { x: p.x + 5.0, y: p.y }, r: rng.radius() }),
        ];
        for s in &shapes {
            for ur in [0, 1] {
                // A vs A: identical proxies -> p == (0,0) on the first vertex.
                let cv = unsafe {
                    call_gjk(&cf, s.ptr(), s.ty(), None, s.ptr(), s.ty(), None, ur, None)
                };
                let rv = unsafe {
                    call_gjk(&rf, s.ptr(), s.ty(), None, s.ptr(), s.ty(), None, ur, None)
                };
                assert_same("tiny-direction break", cv, rv);
                assert_eq!(cv.iterations, 0);
                assert_eq!(cv.dist.to_bits(), 0, "coincident shapes must give dist 0");
            }
        }
    }
}

#[test]
fn err39_gjk_hit_path() {
    // Overlapping polygons drive s.count to 3 -> `hit`, so `a = b` and
    // `dist = 0` regardless of use_radius.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(39);
    let mut hits = 0usize;
    for _ in 0..4000 {
        let c = rng.vec();
        let a = Shape::Aabb(c2AABB {
            min: c2v { x: c.x - 10.0, y: c.y - 10.0 },
            max: c2v { x: c.x + 10.0, y: c.y + 10.0 },
        });
        let off = c2v { x: rng.range(-8.0, 8.0), y: rng.range(-8.0, 8.0) };
        let b = Shape::Aabb(c2AABB {
            min: c2v { x: c.x + off.x - 6.0, y: c.y + off.y - 6.0 },
            max: c2v { x: c.x + off.x + 6.0, y: c.y + off.y + 6.0 },
        });
        for ur in [0, 1] {
            let cv = unsafe {
                call_gjk(&cf, a.ptr(), C2_TYPE_AABB, None, b.ptr(), C2_TYPE_AABB, None, ur, None)
            };
            let rv = unsafe {
                call_gjk(&rf, a.ptr(), C2_TYPE_AABB, None, b.ptr(), C2_TYPE_AABB, None, ur, None)
            };
            assert_same("hit path", cv, rv);
            if cv.dist == 0.0 && same(&cv.outA, &cv.outB) {
                hits += 1;
            }
        }
    }
    assert!(hits > 0, "the hit path was never taken");
    eprintln!("err39 hit-path observations: {hits}");
}

#[test]
fn err40_gjk_radius_overlap() {
    // use_radius != 0 and dist <= rA + rB -> a = b = midpoint, dist = 0.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(40);
    let mut midpoints = 0usize;
    for _ in 0..4000 {
        let ra = rng.range(5.0, 40.0);
        let rb = rng.range(5.0, 40.0);
        let d = rng.range(0.1, ra + rb); // strictly inside the combined radius
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
        let b = Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: rb });
        let cv = unsafe {
            call_gjk(&cf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None)
        };
        let rv = unsafe {
            call_gjk(&rf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None)
        };
        assert_same("radius overlap", cv, rv);
        assert_eq!(cv.dist.to_bits(), 0, "overlap must give +0.0");
        if same(&cv.outA, &cv.outB) {
            midpoints += 1;
        }
    }
    assert_eq!(midpoints, 4000, "outA must equal outB on the midpoint branch");
}

#[test]
fn err41_gjk_radius_epsilon() {
    // dist <= FLT_EPSILON with use_radius != 0 -> same midpoint branch.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(41);
    for _ in 0..3000 {
        let p = rng.vec();
        for eps in [0.0f32, 1e-9, 1.1920929e-7, 5e-8] {
            let a = Shape::Circle(c2Circle { p, r: 0.0 });
            let b = Shape::Circle(c2Circle {
                p: c2v { x: p.x + eps, y: p.y },
                r: 0.0,
            });
            let cv = unsafe {
                call_gjk(&cf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None)
            };
            let rv = unsafe {
                call_gjk(&rf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None)
            };
            assert_same("radius epsilon branch", cv, rv);
            assert_eq!(cv.dist.to_bits(), 0);
        }
    }
}

#[test]
fn err42_gjk_radius_ab_equal() {
    // After shrinking by rA / rB the two witness points can land on the same
    // coordinate, and the C then forces dist = 0 while keeping a and b.
    // Sweep a dense range of radii/separations looking for that collapse and
    // require both libraries to agree everywhere.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(42);
    let mut collapses = 0usize;
    for _ in 0..8000 {
        let ra = rng.range(0.0, 30.0);
        let rb = rng.range(0.0, 30.0);
        // Separation just barely above rA + rB, so the shrink almost cancels.
        let extra = [0.0f32, 1e-6, 1e-5, 1e-4, 1e-3][(rng.next_u32() % 5) as usize];
        let d = ra + rb + extra;
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
        let b = Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: rb });
        let cv = unsafe {
            call_gjk(&cf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None)
        };
        let rv = unsafe {
            call_gjk(&rf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None)
        };
        assert_same("radius shrink collapse", cv, rv);
        if same(&cv.outA, &cv.outB) && cv.dist == 0.0 {
            collapses += 1;
        }
    }
    eprintln!("err42 a==b collapses observed: {collapses}");
    assert!(collapses > 0, "the a == b collapse was never reached");
}

#[test]
fn err43_gjk_no_radius() {
    // use_radius == 0: NO radius adjustment, so the raw witness distance is
    // returned even for shapes whose radii overlap.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(43);
    for _ in 0..4000 {
        let ra = rng.range(1.0, 40.0);
        let rb = rng.range(1.0, 40.0);
        let d = rng.range(0.5, ra + rb);
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
        let b = Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: rb });
        let cv = unsafe {
            call_gjk(&cf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 0, None)
        };
        let rv = unsafe {
            call_gjk(&rf, a.ptr(), C2_TYPE_CIRCLE, None, b.ptr(), C2_TYPE_CIRCLE, None, 0, None)
        };
        assert_same("use_radius=0", cv, rv);
        // The radii are ignored entirely: dist is the centre separation.
        assert_eq!(cv.dist.to_bits(), d.to_bits(), "radius must not be applied");
    }
}

#[test]
fn err44_gjk_use_radius_nonbool() {
    // C truthiness: any non-zero int enables the radius path.
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(44);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..200 {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            let one =
                unsafe { call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, 1, None) };
            for ur in [2, -1, 255, 65536, c_int::MAX, c_int::MIN, -2147483647] {
                let cv =
                    unsafe { call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None) };
                let rv =
                    unsafe { call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None) };
                assert_same(&format!("use_radius={ur}"), cv, rv);
                assert_same(&format!("use_radius={ur} == 1"), cv, one);
            }
            // And 0 really is the only false value.
            let zero =
                unsafe { call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, 0, None) };
            let zero_r =
                unsafe { call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, None, 0, None) };
            assert_same("use_radius=0", zero, zero_r);
        }
    }
}

#[test]
fn err45_gjk_nonfinite_shapes() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(45);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..500 {
            let a = match ta {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() }),
                _ => Shape::Capsule(c2Capsule {
                    a: rng.spicy_vec(),
                    b: rng.spicy_vec(),
                    r: rng.spicy_f32(),
                }),
            };
            let b = match tb {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() }),
                _ => Shape::Capsule(c2Capsule {
                    a: rng.spicy_vec(),
                    b: rng.spicy_vec(),
                    r: rng.spicy_f32(),
                }),
            };
            for ur in [0, 1] {
                let cv = unsafe { call_gjk(&cf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None) };
                let rv = unsafe { call_gjk(&rf, a.ptr(), ta, None, b.ptr(), tb, None, ur, None) };
                if !same(&cv, &rv) {
                    eprintln!("nonfinite A={a:?} B={b:?} ur={ur}");
                }
                assert_same("nonfinite shapes", cv, rv);
            }
            // Also with non-finite transforms.
            let ax = c2x { p: rng.spicy_vec(), r: c2r { c: rng.spicy_f32(), s: rng.spicy_f32() } };
            let cv = unsafe {
                call_gjk(&cf, a.ptr(), ta, Some(&ax), b.ptr(), tb, None, 1, None)
            };
            let rv = unsafe {
                call_gjk(&rf, a.ptr(), ta, Some(&ax), b.ptr(), tb, None, 1, None)
            };
            assert_same("nonfinite transform", cv, rv);
        }
    }
}

// =========================================================================
// Row 46 — `if (float)` truthiness in the two GJK-backed boolean wrappers
// =========================================================================

#[test]
fn err46_bool_wrappers_float_truthiness() {
    let (c_ac, r_ac) = pair::<extern "C" fn(c2AABB, c2Capsule) -> c_int>("c2AABBtoCapsule");
    let (c_cc, r_cc) = pair::<extern "C" fn(c2Capsule, c2Capsule) -> c_int>("c2CapsuletoCapsule");
    let (c_gjk, r_gjk) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(46);
    let mut nan_cases = 0usize;
    let mut zero_cases = 0usize;
    let mut inf_cases = 0usize;
    let mut pos_cases = 0usize;

    for _ in 0..6000 {
        let bb = c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() };
        let cap = c2Capsule { a: rng.spicy_vec(), b: rng.spicy_vec(), r: rng.spicy_f32() };
        let cap2 = c2Capsule { a: rng.spicy_vec(), b: rng.spicy_vec(), r: rng.spicy_f32() };

        let cv = c_ac(bb, cap);
        assert_same("c2AABBtoCapsule truthiness", cv, r_ac(bb, cap));
        let wv = c_cc(cap, cap2);
        assert_same("c2CapsuletoCapsule truthiness", wv, r_cc(cap, cap2));

        // Cross-check against the underlying c2GJK return: `if (dist)` is
        // false only for ±0.0 and true for everything else, NaN included.
        let g = unsafe {
            let sa = Shape::Aabb(bb);
            let sb = Shape::Capsule(cap);
            call_gjk(&c_gjk, sa.ptr(), C2_TYPE_AABB, None, sb.ptr(), C2_TYPE_CAPSULE, None, 1, None)
        };
        let expect = if g.dist != 0.0 { 0 } else { 1 };
        assert_eq!(cv, expect, "C wrapper disagrees with its own c2GJK result");
        if g.dist.is_nan() {
            nan_cases += 1;
            assert_eq!(cv, 0, "NaN is truthy in C, so the wrapper must return 0");
        } else if g.dist == 0.0 {
            zero_cases += 1;
        } else if g.dist.is_infinite() {
            inf_cases += 1;
            assert_eq!(cv, 0);
        } else {
            pos_cases += 1;
            assert_eq!(cv, 0);
        }
    }
    eprintln!(
        "err46 dist classes -> NaN:{nan_cases} zero:{zero_cases} Inf:{inf_cases} finite-nonzero:{pos_cases}"
    );
    assert!(zero_cases > 0, "no zero c2GJK result was produced");
    assert!(inf_cases + pos_cases > 0, "no truthy c2GJK result was produced");
    // Both wrappers hard-code use_radius = 1, and on that path a NaN `dist`
    // fails `dist > rA + rB` and is rewritten to +0.0 by the midpoint branch.
    // A NaN can therefore never escape these two wrappers.
    assert_eq!(
        nan_cases, 0,
        "unexpected NaN escaped the use_radius=1 midpoint branch"
    );

    // The NaN-is-truthy behaviour IS reachable at the c2GJK level with
    // use_radius == 0 (no midpoint rewrite), so check that both libraries
    // produce the same NaN-ness there.
    let mut raw_nan = 0usize;
    for _ in 0..6000 {
        let bb = c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() };
        let cap = c2Capsule { a: rng.spicy_vec(), b: rng.spicy_vec(), r: rng.spicy_f32() };
        let sa = Shape::Aabb(bb);
        let sb = Shape::Capsule(cap);
        let cg = unsafe {
            call_gjk(&c_gjk, sa.ptr(), C2_TYPE_AABB, None, sb.ptr(), C2_TYPE_CAPSULE, None, 0, None)
        };
        let rg = unsafe {
            call_gjk(&r_gjk, sa.ptr(), C2_TYPE_AABB, None, sb.ptr(), C2_TYPE_CAPSULE, None, 0, None)
        };
        assert_same("use_radius=0 raw dist", cg, rg);
        assert_eq!(
            cg.dist.is_nan(),
            rg.dist.is_nan(),
            "NaN-ness of the raw distance must agree"
        );
        if cg.dist.is_nan() {
            raw_nan += 1;
        }
    }
    eprintln!("err46 raw (use_radius=0) NaN distances: {raw_nan}");
    assert!(raw_nan > 0, "no NaN raw distance was produced");

    // -0.0: the midpoint branch stores +0.0, so also check the sign explicitly
    // over a large finite sweep.
    for _ in 0..4000 {
        let bb = rng.aabb();
        let cap = rng.capsule();
        let cv = c_ac(bb, cap);
        assert_same("c2AABBtoCapsule finite", cv, r_ac(bb, cap));
        let cap2 = rng.capsule();
        assert_same("c2CapsuletoCapsule finite", c_cc(cap, cap2), r_cc(cap, cap2));
    }
}

// =========================================================================
// Rows 47..48 — public entry point and c2BBVerts with invalid geometry
// =========================================================================

#[test]
fn err47_aabb_entry_nonfinite() {
    let (cf, rf) = pair::<FnAabb>("aabb");
    let specials = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        0.0,
        -0.0,
        1e-45,
        -1e-45,
    ];
    for a in specials {
        for b in specials {
            for c in specials {
                for d in specials {
                    assert_same("aabb nonfinite", cf(a, b, c, d), rf(a, b, c, d));
                }
            }
        }
    }
    // Inverted boxes over the whole fixture neighbourhood.
    let mut rng = Rng::new(47);
    for _ in 0..20000 {
        let (x0, y0) = (rng.range(-160.0, 60.0), rng.range(-60.0, 160.0));
        let (x1, y1) = (rng.range(-160.0, 60.0), rng.range(-60.0, 160.0));
        // Deliberately swapped: min > max.
        let (mnx, mny, mxx, mxy) = (x0.max(x1), y0.max(y1), x0.min(x1), y0.min(y1));
        assert_same("aabb inverted", cf(mnx, mny, mxx, mxy), rf(mnx, mny, mxx, mxy));
    }
}

#[test]
fn err48_bbverts_inverted() {
    let (cf, rf) = pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(48);
    for _ in 0..6000 {
        let bb = rng.aabb();
        for probe in [
            c2AABB { min: bb.max, max: bb.min },
            c2AABB { min: bb.min, max: bb.min },
            c2AABB {
                min: c2v { x: f32::NAN, y: bb.min.y },
                max: c2v { x: bb.max.x, y: f32::INFINITY },
            },
            c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() },
        ] {
            let mut cout = [c2v { x: f32::from_bits(0x1111_1111), y: f32::from_bits(0x2222_2222) }; 8];
            let mut rout = cout;
            let mut cb = probe;
            let mut rb = probe;
            unsafe {
                cf(cout.as_mut_ptr(), &raw mut cb);
                rf(rout.as_mut_ptr(), &raw mut rb);
            }
            assert_same("c2BBVerts invalid box", cout, rout);
            assert_same("c2BBVerts input unmodified", cb, rb);
            // Only the first four slots may be written.
            assert_eq!(cout[4].x.to_bits(), 0x1111_1111);
            assert_eq!(cout[7].y.to_bits(), 0x2222_2222);
        }
    }
}

// =========================================================================
// Rows 49..52 — every branch of c22 and c23, constructed explicitly
// =========================================================================

fn dotv(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn subv(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}
fn det2v(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn run_c22(cf: &FnSimplexVoid, rf: &FnSimplexVoid, s: c2Simplex, label: &str) {
    let mut cs = s;
    let mut rs = s;
    unsafe {
        cf(&raw mut cs);
        rf(&raw mut rs);
    }
    assert_same(label, cs, rs);
}

#[test]
fn err49_c22_v_le_zero() {
    let (cf, rf) = pair::<FnSimplexVoid>("c22");
    let mut rng = Rng::new(49);
    let mut hit = 0usize;
    // v = dot(a, a-b) <= 0 means the origin is beyond `a`: put a at the origin
    // or make a-b point away from a.
    for _ in 0..4000 {
        let mut s = rng.simplex(2);
        s.verts[0].p = c2v { x: 0.0, y: 0.0 };
        s.verts[1].p = rng.vec();
        let v = dotv(s.verts[0].p, subv(s.verts[0].p, s.verts[1].p));
        if v <= 0.0 {
            hit += 1;
        }
        run_c22(&cf, &rf, s, "c22 v<=0 (a at origin)");

        // -0.0 exactly.
        let mut s2 = rng.simplex(2);
        s2.verts[0].p = c2v { x: -0.0, y: -0.0 };
        s2.verts[1].p = rng.vec();
        run_c22(&cf, &rf, s2, "c22 v==-0.0");

        // A general configuration where the origin lies outside segment at `a`.
        let d = rng.vec();
        let mut s3 = rng.simplex(2);
        s3.verts[0].p = d;
        s3.verts[1].p = c2v { x: d.x * 2.0, y: d.y * 2.0 };
        if dotv(s3.verts[0].p, subv(s3.verts[0].p, s3.verts[1].p)) <= 0.0 {
            hit += 1;
        }
        run_c22(&cf, &rf, s3, "c22 v<=0 (collinear outside)");
    }
    assert!(hit > 0, "branch v<=0 never taken");
}

#[test]
fn err50_c22_u_le_zero() {
    let (cf, rf) = pair::<FnSimplexVoid>("c22");
    let mut rng = Rng::new(50);
    let mut hit = 0usize;
    for _ in 0..4000 {
        // u = dot(b, b-a) <= 0 with v > 0: put b at the origin.
        let mut s = rng.simplex(2);
        s.verts[0].p = rng.vec();
        s.verts[1].p = c2v { x: 0.0, y: 0.0 };
        let u = dotv(s.verts[1].p, subv(s.verts[1].p, s.verts[0].p));
        let v = dotv(s.verts[0].p, subv(s.verts[0].p, s.verts[1].p));
        if v > 0.0 && u <= 0.0 {
            hit += 1;
        }
        run_c22(&cf, &rf, s, "c22 u<=0 (b at origin)");

        // Collinear with the origin beyond `b`.
        let d = rng.vec();
        let mut s2 = rng.simplex(2);
        s2.verts[0].p = c2v { x: d.x * 2.0, y: d.y * 2.0 };
        s2.verts[1].p = d;
        run_c22(&cf, &rf, s2, "c22 u<=0 (collinear)");
    }
    assert!(hit > 0, "branch u<=0 never taken");
}

#[test]
fn err51_c22_nan() {
    let (cf, rf) = pair::<FnSimplexVoid>("c22");
    let mut rng = Rng::new(51);
    // Every comparison is false with NaN, so the `else` (count = 2) arm runs
    // and `div` becomes NaN.
    for _ in 0..4000 {
        let mut s = rng.simplex(2);
        s.verts[0].p = c2v { x: f32::NAN, y: rng.coord() };
        s.verts[1].p = rng.vec();
        run_c22(&cf, &rf, s, "c22 NaN a");

        let mut s2 = rng.simplex(2);
        s2.verts[0].p = rng.vec();
        s2.verts[1].p = c2v { x: rng.coord(), y: f32::NAN };
        run_c22(&cf, &rf, s2, "c22 NaN b");

        let mut s3 = rng.simplex(2);
        s3.verts[0].p = c2v { x: f32::INFINITY, y: f32::NEG_INFINITY };
        s3.verts[1].p = c2v { x: f32::INFINITY, y: f32::INFINITY };
        run_c22(&cf, &rf, s3, "c22 Inf");

        let mut s4 = rng.simplex(2);
        s4.verts[0].p = rng.spicy_vec();
        s4.verts[1].p = rng.spicy_vec();
        run_c22(&cf, &rf, s4, "c22 spicy");
    }
    // Verify the NaN case really lands on the `else` arm in the C.
    let mut s = rng.simplex(2);
    s.verts[0].p = c2v { x: f32::NAN, y: 0.0 };
    s.verts[1].p = c2v { x: 1.0, y: 1.0 };
    let mut cs = s;
    unsafe { cf(&raw mut cs) };
    assert_eq!(cs.count, 2, "NaN must fall through to the interior arm");
    assert!(cs.div.is_nan(), "div must be NaN there");
}

#[test]
fn err52_c23_all_branches() {
    let (cf, rf) = pair::<FnSimplexVoid>("c23");
    let mut rng = Rng::new(52);
    let mut hits = [0usize; 7];

    let classify = |s: &c2Simplex| -> usize {
        let (a, b, c) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
        let uAB = dotv(b, subv(b, a));
        let vAB = dotv(a, subv(a, b));
        let uBC = dotv(c, subv(c, b));
        let vBC = dotv(b, subv(b, c));
        let uCA = dotv(a, subv(a, c));
        let vCA = dotv(c, subv(c, a));
        let area = det2v(subv(b, a), subv(c, a));
        let uABC = det2v(b, c) * area;
        let vABC = det2v(c, a) * area;
        let wABC = det2v(a, b) * area;
        if vAB <= 0.0 && uCA <= 0.0 {
            0
        } else if uAB <= 0.0 && vBC <= 0.0 {
            1
        } else if uBC <= 0.0 && vCA <= 0.0 {
            2
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            3
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            4
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            5
        } else {
            6
        }
    };

    // Triangles placed so that the origin visits every Voronoi region: each
    // vertex region, each edge region, and the interior.
    let tri = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: -1.0, y: -1.0 },
    ];
    for k in 0..3 {
        for scale in [0.1f32, 1.0, 10.0] {
            for shift in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: 3.0, y: 0.0 },
                c2v { x: -3.0, y: 0.0 },
                c2v { x: 0.0, y: 3.0 },
                c2v { x: 0.0, y: -3.0 },
                c2v { x: 3.0, y: 3.0 },
                c2v { x: -3.0, y: -3.0 },
                c2v { x: 3.0, y: -3.0 },
                c2v { x: -3.0, y: 3.0 },
            ] {
                let mut s = rng.simplex(3);
                for i in 0..3 {
                    let v = tri[(i + k) % 3];
                    s.verts[i].p = c2v {
                        x: v.x * scale + shift.x,
                        y: v.y * scale + shift.y,
                    };
                }
                hits[classify(&s)] += 1;
                let mut cs = s;
                let mut rs = s;
                unsafe {
                    cf(&raw mut cs);
                    rf(&raw mut rs);
                }
                assert_same("c23 constructed", cs, rs);
            }
        }
    }
    // Broad randomized sweep to fill in any region the fixtures missed.
    for _ in 0..40000 {
        let mut s = rng.simplex(3);
        for i in 0..3 {
            s.verts[i].p = c2v {
                x: rng.range(-4.0, 4.0),
                y: rng.range(-4.0, 4.0),
            };
        }
        hits[classify(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c23 random", cs, rs);
    }
    // Degenerate (zero-area) and NaN-driven fall-through to the count=3 arm.
    for _ in 0..4000 {
        let a = rng.vec();
        let d = rng.vec();
        let t = rng.range(-3.0, 3.0);
        let mut s = rng.simplex(3);
        s.verts[0].p = a;
        s.verts[1].p = c2v { x: a.x + d.x, y: a.y + d.y };
        s.verts[2].p = c2v { x: a.x + d.x * t, y: a.y + d.y * t };
        hits[classify(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c23 zero area", cs, rs);

        let mut sn = rng.simplex(3);
        sn.verts[0].p = c2v { x: f32::NAN, y: rng.coord() };
        sn.verts[1].p = rng.vec();
        sn.verts[2].p = rng.vec();
        let mut cs = sn;
        let mut rs = sn;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c23 NaN", cs, rs);
        assert_eq!(cs.count, 3, "NaN must fall through to the interior arm");

        let mut sp = rng.simplex(3);
        sp.verts[0].p = rng.spicy_vec();
        sp.verts[1].p = rng.spicy_vec();
        sp.verts[2].p = rng.spicy_vec();
        let mut cs = sp;
        let mut rs = sp;
        unsafe {
            cf(&raw mut cs);
            rf(&raw mut rs);
        }
        assert_same("c23 spicy", cs, rs);
    }
    eprintln!("err52 c23 branch hits: {hits:?}");
    assert!(
        hits.iter().all(|&h| h > 0),
        "c23 branch coverage incomplete: {hits:?}"
    );
}

// =========================================================================
// Documented UB: c2GJK with an out-of-range C2_TYPE.
//
// `c2MakeProxy` has no `default:` arm, so the `c2Proxy` stack object in
// `c2GJK` stays UNINITIALISED and the following `pA.verts[0]` /
// `c2Support(pA.verts, pA.count, ...)` read indeterminate — and potentially
// out-of-bounds — memory. There is no byte-identical expectation to assert
// against, and the C can fault, so this is recorded rather than enforced.
// Run explicitly with `cargo test -- --ignored ub_gjk_invalid_type`.
// =========================================================================

#[test]
#[ignore = "reads uninitialised C stack memory (undefined behaviour in the C)"]
fn ub_gjk_invalid_type() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(999);
    let a = Shape::Circle(rng.circle());
    let b = Shape::Circle(rng.circle());
    for bt in BAD_TYPES {
        let cv = unsafe { call_gjk(&cf, a.ptr(), bt, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None) };
        let rv = unsafe { call_gjk(&rf, a.ptr(), bt, None, b.ptr(), C2_TYPE_CIRCLE, None, 1, None) };
        eprintln!("typeA={bt}\n  C   ={cv:?}\n  Rust={rv:?}");
    }
}
