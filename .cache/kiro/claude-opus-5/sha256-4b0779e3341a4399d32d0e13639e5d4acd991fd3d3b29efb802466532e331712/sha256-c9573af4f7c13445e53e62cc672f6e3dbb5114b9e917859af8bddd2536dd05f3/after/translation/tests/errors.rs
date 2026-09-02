//! Phase C — error / rejection-path differential tests.
//!
//! One test (or one clearly-labelled block) per row of `ERRORS.md`. Each
//! constructs the exact invalid input or condition, calls BOTH `.so`s, and
//! asserts they return the SAME sentinel — not merely that both "failed".

mod common;
use common::*;
use std::ffi::c_void;
use std::ptr;

type FnCollided = unsafe extern "C" fn(*const c_void, i32, *const c_void, i32) -> i32;
type FnMakeProxy = unsafe extern "C" fn(*const c_void, i32, *mut C2Proxy);
type FnMetric = unsafe extern "C" fn(*mut C2Simplex) -> f32;
type FnWitness = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
type FnSimplexV = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
type FnSupport = unsafe extern "C" fn(*const C2v, i32, C2v) -> i32;
type FnVvf = unsafe extern "C" fn(C2v, f32) -> C2v;
type FnVv = unsafe extern "C" fn(C2v) -> C2v;
type FnFv = unsafe extern "C" fn(C2v) -> f32;
type FnAABBtoAABB = unsafe extern "C" fn(C2AABB, C2AABB) -> i32;
type FnAABBtoCapsule = unsafe extern "C" fn(C2AABB, C2Capsule) -> i32;
type FnCapsuletoCapsule = unsafe extern "C" fn(C2Capsule, C2Capsule) -> i32;
type FnCircletoCircle = unsafe extern "C" fn(C2Circle, C2Circle) -> i32;
type FnCircletoAABB = unsafe extern "C" fn(C2Circle, C2AABB) -> i32;
type FnCircletoCapsule = unsafe extern "C" fn(C2Circle, C2Capsule) -> i32;
type FnReverse = unsafe extern "C" fn(f32, f32, f32) -> i32;
type FnBBVerts = unsafe extern "C" fn(*mut C2v, *mut C2AABB);

/// Every `int` value that is NOT a valid `C2_TYPE`, including the ones a C enum
/// silently accepts. C enums are just `int` across the FFI boundary, so these
/// are all real inputs the C handles and the Rust must handle identically.
const BAD_TYPES: &[i32] = &[
    3, 4, 5, 99, 255, 256, 1000, -1, -2, -99, i32::MIN, i32::MAX,
    i32::MIN + 1, i32::MAX - 1, 0x0100_0000, -0x0100_0000, 0x7fff_fffe,
];

const VALID_TYPES: &[i32] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// ERRORS.md rows 1,2,3 — `c2Collided` with a valid `typeA` and an
/// out-of-range `typeB` must return 0 from the inner `default:` arm.
/// ERRORS.md row 4 — an out-of-range `typeA` must return 0 from the outer
/// `default:` arm WITHOUT dereferencing either shape pointer (proven by
/// passing NULL for both).
#[test]
fn rows1_4_collided_invalid_enum() {
    let l = libs();
    let (c, r) = l.pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(0xE0_0001);

    // rows 1,2,3: valid typeA, invalid typeB.
    for &ta in VALID_TYPES {
        let a = ShapeBlob::random(&mut rng, ta);
        for &tb in BAD_TYPES {
            // With a real B pointer...
            let b = ShapeBlob::random(&mut rng, C2_TYPE_CIRCLE);
            let (cv, rv) = unsafe { (c(a.ptr(), ta, b.ptr(), tb), r(a.ptr(), ta, b.ptr(), tb)) };
            same_i32("c2Collided invalid typeB", &(type_name(ta), tb), cv, rv);
            assert_eq!(cv, 0, "C should reject typeB={tb} with 0");

            // ...and with NULL, since the invalid arm must not dereference it.
            let (cv, rv) = unsafe {
                (
                    c(a.ptr(), ta, ptr::null(), tb),
                    r(a.ptr(), ta, ptr::null(), tb),
                )
            };
            same_i32("c2Collided invalid typeB, NULL B", &(type_name(ta), tb), cv, rv);
            assert_eq!(cv, 0);
        }
    }

    // row 4: invalid typeA, every typeB (valid and invalid), NULL shapes.
    for &ta in BAD_TYPES {
        for tb in VALID_TYPES.iter().copied().chain(BAD_TYPES.iter().copied()) {
            let (cv, rv) = unsafe {
                (
                    c(ptr::null(), ta, ptr::null(), tb),
                    r(ptr::null(), ta, ptr::null(), tb),
                )
            };
            same_i32("c2Collided invalid typeA", &(ta, tb), cv, rv);
            assert_eq!(cv, 0, "C should reject typeA={ta} with 0");
        }
    }
}

/// ERRORS.md row 5 — `c2MakeProxy` with an out-of-range type: the C `switch`
/// has no `default:`, so `*p` must be left COMPLETELY unwritten.
#[test]
fn row5_makeproxy_invalid_enum() {
    let l = libs();
    let (c, r) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0xE0_0005);

    let poison = C2Proxy {
        radius: f32::from_bits(0x5eed_face),
        count: -0x5eed,
        verts: {
            let mut v = [C2v::default(); 8];
            for (i, s) in v.iter_mut().enumerate() {
                s.x = f32::from_bits(0x4000_0000 | i as u32);
                s.y = f32::from_bits(0x4100_0000 | i as u32);
            }
            v
        },
    };

    for &bad in BAD_TYPES {
        for kind in VALID_TYPES.iter().copied() {
            let blob = ShapeBlob::random(&mut rng, kind);
            let mut cp = poison;
            let mut rp = poison;
            unsafe {
                c(blob.ptr(), bad, &mut cp);
                r(blob.ptr(), bad, &mut rp);
            }
            same("c2MakeProxy invalid type", &bad, &cp, &rp);
            // And specifically: nothing at all was written.
            same("c2MakeProxy invalid type left proxy untouched", &bad, &cp, &poison);
        }
        // Also with a NULL shape: the invalid arm never dereferences it.
        let mut cp = poison;
        let mut rp = poison;
        unsafe {
            c(ptr::null(), bad, &mut cp);
            r(ptr::null(), bad, &mut rp);
        }
        same("c2MakeProxy invalid type, NULL shape", &bad, &cp, &rp);
        same("c2MakeProxy NULL shape left proxy untouched", &bad, &cp, &poison);
    }
}

/// ERRORS.md rows 6,7 — `c2GJKSimplexMetric` returns 0 for `count == 1` and for
/// every out-of-range count (the `default:` label falls into `case 1:`).
#[test]
fn rows6_7_metric_out_of_range_count() {
    let l = libs();
    let (c, r) = l.pair::<FnMetric>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0xE0_0006);

    for &count in &[1i32, 0, -1, 4, 5, 99, -99, i32::MIN, i32::MAX] {
        for _ in 0..500 {
            let mut s = C2Simplex::default();
            s.count = count;
            s.div = rng.f32_mixed();
            for k in 0..4 {
                s.v[k].p = rng.v_mixed();
                s.v[k].u = rng.f32_mixed();
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            same_f32("c2GJKSimplexMetric out-of-range count", &count, cv, rv);
            if count != 2 && count != 3 {
                assert_eq!(cv.to_bits(), 0f32.to_bits(), "expected +0.0 for count={count}");
            }
        }
    }
}

/// ERRORS.md rows 8,9,10 — `c2Witness` `default:` arm, and `div == +-0`.
#[test]
fn rows8_10_witness_rejections() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0xE0_0008);
    let poison = C2v { x: f32::from_bits(0x5eed_face), y: f32::from_bits(0x5eed_beef) };

    // row 8: out-of-range count -> both witnesses forced to (0,0).
    for &count in &[0i32, 4, 5, 99, -1, -99, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let mut s = C2Simplex::default();
            s.count = count;
            s.div = rng.f32_mixed();
            for k in 0..4 {
                s.v[k].sA = rng.v_mixed();
                s.v[k].sB = rng.v_mixed();
                s.v[k].u = rng.f32_mixed();
            }
            let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            same("c2Witness default a", &count, &ca, &ra);
            same("c2Witness default b", &count, &cb, &rb);
            assert_eq!(ca.x.to_bits(), 0, "expected (0,0) for count={count}");
            assert_eq!(cb.y.to_bits(), 0);
        }
    }

    // rows 9,10: div == +0 / -0 with counts 2 and 3 -> 1/div is +-inf.
    for &count in &[1i32, 2, 3] {
        for &div in &[0.0f32, -0.0, f32::MIN_POSITIVE, f32::from_bits(1), f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            for _ in 0..300 {
                let mut s = C2Simplex::default();
                s.count = count;
                s.div = div;
                for k in 0..4 {
                    s.v[k].sA = rng.v_mixed();
                    s.v[k].sB = rng.v_mixed();
                    s.v[k].u = match k % 3 {
                        0 => 0.0,
                        1 => rng.f32_mixed(),
                        _ => rng.range(-5.0, 5.0),
                    };
                }
                let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
                let mut cs = s;
                let mut rs = s;
                unsafe {
                    c(&mut cs, &mut ca, &mut cb);
                    r(&mut rs, &mut ra, &mut rb);
                }
                same("c2Witness div=0 a", &(count, div), &ca, &ra);
                same("c2Witness div=0 b", &(count, div), &cb, &rb);
            }
        }
    }
}

/// ERRORS.md rows 11,12 — `c2L` `default:` arm and `div == 0`.
/// ERRORS.md row 13 — `c2D` at `count == 3` and out of range.
#[test]
fn rows11_13_c2l_c2d_rejections() {
    let l = libs();
    let (cl, rl) = l.pair::<FnSimplexV>("c2L");
    let (cd, rd) = l.pair::<FnSimplexV>("c2D");
    let mut rng = Rng::new(0xE0_0011);

    for &count in &[0i32, 3, 4, 5, 99, -1, -99, i32::MIN, i32::MAX] {
        for _ in 0..400 {
            let mut s = C2Simplex::default();
            s.count = count;
            s.div = rng.f32_mixed();
            for k in 0..4 {
                s.v[k].p = rng.v_mixed();
                s.v[k].u = rng.f32_mixed();
            }
            let mut cs = s;
            let mut rs = s;
            let (a, b) = unsafe { (cl(&mut cs), rl(&mut rs)) };
            same("c2L out-of-range count", &count, &a, &b);
            assert_eq!((a.x.to_bits(), a.y.to_bits()), (0, 0), "c2L count={count}");

            let mut cs = s;
            let mut rs = s;
            let (a, b) = unsafe { (cd(&mut cs), rd(&mut rs)) };
            same("c2D out-of-range count", &count, &a, &b);
            assert_eq!((a.x.to_bits(), a.y.to_bits()), (0, 0), "c2D count={count}");
        }
    }

    // row 12: div == 0 with count == 2.
    for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::from_bits(1)] {
        for _ in 0..400 {
            let mut s = C2Simplex::default();
            s.count = 2;
            s.div = div;
            for k in 0..4 {
                s.v[k].p = rng.v_mixed();
                s.v[k].u = if k == 0 { 0.0 } else { rng.f32_mixed() };
            }
            let mut cs = s;
            let mut rs = s;
            let (a, b) = unsafe { (cl(&mut cs), rl(&mut rs)) };
            same("c2L div=0", &div, &a, &b);
        }
    }
}

/// ERRORS.md rows 14,15 — `c2Support` with `count <= 0` (returns 0 but still
/// reads `verts[0]`), and with all-equal / NaN dots (no `dot > dmax` ever true).
#[test]
fn rows14_15_support_rejections() {
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0xE0_0014);

    for &count in &[0i32, -1, -2, -99, i32::MIN, i32::MIN + 1] {
        for _ in 0..500 {
            let mut verts = [C2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.v_mixed();
            }
            let d = rng.v_mixed();
            let (cv, rv) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            same_i32("c2Support count<=0", &(count, d), cv, rv);
            assert_eq!(cv, 0, "expected index 0 for count={count}");
        }
    }

    // row 15: ties and NaN dots -> the first index must win.
    for &count in &[1i32, 2, 4, 8] {
        for k in 0..500 {
            let verts = match k % 3 {
                0 => [rng.v_tame(); 8],                          // all equal
                1 => [C2v { x: f32::NAN, y: f32::NAN }; 8],       // all NaN
                _ => {
                    let mut v = [C2v::default(); 8];
                    for s in v.iter_mut() {
                        s.x = f32::NAN;
                        s.y = rng.f32_tame();
                    }
                    v
                }
            };
            let d = if k % 2 == 0 { C2v { x: 1.0, y: 0.0 } } else { rng.v_mixed() };
            let (cv, rv) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            same_i32("c2Support ties/NaN", &(count, d), cv, rv);
            assert_eq!(cv, 0, "first index must win on ties");
        }
    }
}

/// ERRORS.md rows 16,17 — `c2Div` by `+0` and `-0`.
/// ERRORS.md rows 18,19 — `c2Norm` of the zero vector / non-finite input.
/// ERRORS.md rows 20,21 — `c2Len` of non-finite input / overflow to +inf.
#[test]
fn rows16_21_division_and_length_degeneracies() {
    let l = libs();
    let (cdiv, rdiv) = l.pair::<FnVvf>("c2Div");
    let (cn, rn) = l.pair::<FnVv>("c2Norm");
    let (clen, rlen) = l.pair::<FnFv>("c2Len");
    let mut rng = Rng::new(0xE0_0016);

    // rows 16,17
    for &b in &[0.0f32, -0.0] {
        let mut vs: Vec<C2v> = vec![
            C2v { x: 0.0, y: 0.0 },
            C2v { x: -0.0, y: 0.0 },
            C2v { x: 1.0, y: -1.0 },
            C2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            C2v { x: f32::NAN, y: 1.0 },
        ];
        for _ in 0..500 {
            vs.push(rng.v_mixed());
        }
        for a in vs {
            let (p, q) = unsafe { (cdiv(a, b), rdiv(a, b)) };
            same("c2Div by zero", &(a, b), &p, &q);
        }
    }

    // rows 18,19,20,21
    let mut vs: Vec<C2v> = vec![
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: f32::MAX, y: f32::MAX },     // dot overflows to +inf
        C2v { x: f32::MIN, y: f32::MIN },
        C2v { x: f32::INFINITY, y: 0.0 },
        C2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        C2v { x: f32::NAN, y: 0.0 },
        C2v { x: 0.0, y: f32::NAN },
        C2v { x: f32::from_bits(1), y: f32::from_bits(1) }, // underflows to 0
        C2v { x: f32::MIN_POSITIVE, y: 0.0 },
    ];
    for _ in 0..2000 {
        vs.push(rng.v_mixed());
    }
    for _ in 0..2000 {
        vs.push(rng.v_any());
    }
    for a in vs {
        let (p, q) = unsafe { (cn(a), rn(a)) };
        same("c2Norm degenerate", &a, &p, &q);
        let (p, q) = unsafe { (clen(a), rlen(a)) };
        same_f32("c2Len degenerate", &a, p, q);
    }
    // Spot-check the documented outcomes.
    let z = C2v { x: 0.0, y: 0.0 };
    assert!(unsafe { cn(z) }.x.is_nan(), "c2Norm((0,0)).x should be NaN");
    let big = C2v { x: f32::MAX, y: f32::MAX };
    assert_eq!(unsafe { clen(big) }, f32::INFINITY, "c2Len should overflow to +inf");
}

/// ERRORS.md rows 22..28 — every NULL-pointer guard in `c2GJK`, in every
/// combination, and the cold-cache (`count == 0`) rejection.
#[test]
fn rows22_28_gjk_null_guards() {
    let mut rng = Rng::new(0xE0_0022);
    let ident = C2x { p: C2v { x: 0.0, y: 0.0 }, r: C2r { c: 1.0, s: 0.0 } };

    for (ta, tb) in TYPE_PAIRS {
        for ur in [0, 1] {
            for i in 0..24 {
                let a = ShapeBlob::random(&mut rng, ta);
                let b = ShapeBlob::random(&mut rng, tb);

                // rows 22,23: NULL transform must behave exactly like identity.
                let null_ax = gjk_diff("row22 NULL ax", &a, ta, &b, tb,
                    &GjkOpts { use_radius: ur, bx: Some(ident), ..Default::default() });
                let ident_ax = gjk_diff("row22 identity ax", &a, ta, &b, tb,
                    &GjkOpts { use_radius: ur, ax: Some(ident), bx: Some(ident), ..Default::default() });
                same("row22 NULL ax == identity ax", &(type_name(ta), type_name(tb)), &null_ax, &ident_ax);

                let null_bx = gjk_diff("row23 NULL bx", &a, ta, &b, tb,
                    &GjkOpts { use_radius: ur, ax: Some(ident), ..Default::default() });
                same("row23 NULL bx == identity bx", &(type_name(ta), type_name(tb)), &null_bx, &ident_ax);

                let both_null = gjk_diff("rows22+23 both NULL", &a, ta, &b, tb,
                    &GjkOpts { use_radius: ur, ..Default::default() });
                same("both NULL == both identity", &(type_name(ta), type_name(tb)), &both_null, &ident_ax);

                // rows 24,25,26: every subset of the out-params NULL.
                for mask in 0..8u8 {
                    let o = gjk_diff("rows24-26 NULL out-params", &a, ta, &b, tb,
                        &GjkOpts {
                            use_radius: ur,
                            want_a: mask & 1 != 0,
                            want_b: mask & 2 != 0,
                            want_iters: mask & 4 != 0,
                            ..Default::default()
                        });
                    same_f32("dist unaffected by NULL out-params",
                             &(type_name(ta), type_name(tb), mask), both_null.dist, o.dist);
                }

                // row 27: cache == NULL (already the default) vs
                // row 28: cache != NULL with count == 0 -> not read, cold start.
                let cold = C2GJKCache {
                    metric: if i % 2 == 0 { rng.f32_mixed() } else { -1.0e30 },
                    count: 0,
                    iA: [7, -2, 31],
                    iB: [-5, 99, 1],
                    div: rng.f32_mixed(),
                };
                let with_cold = gjk_diff("row28 cold cache", &a, ta, &b, tb,
                    &GjkOpts { use_radius: ur, cache: Some(cold), ..Default::default() });
                same_f32("row28 cold cache dist == no-cache dist",
                         &(type_name(ta), type_name(tb)), both_null.dist, with_cold.dist);
                same("row28 cold cache outA", &(type_name(ta), type_name(tb)), &both_null.a, &with_cold.a);
                same("row28 cold cache outB", &(type_name(ta), type_name(tb)), &both_null.b, &with_cold.b);
                same_i32("row28 cold cache iters", &(type_name(ta), type_name(tb)),
                         both_null.iters, with_cold.iters);
                let cl = classify(&a, ta, &b, tb,
                    &GjkOpts { use_radius: ur, cache: Some(cold), ..Default::default() });
                assert!(!cl.cache_read, "count==0 cache must not be read");
            }
        }
    }
}

/// ERRORS.md row 29 — the cache metric guard
/// (`min_metric < max_metric*2 && metric < -1.0e8f`) rejects the cached simplex
/// and forces a cold restart.
/// ERRORS.md row 30 — `cache->metric = NaN` makes both comparisons false, so the
/// cache IS read.
#[test]
fn rows29_30_cache_metric_guard() {
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    let mut rng = Rng::new(0xE0_0029);

    // A very negative 3-simplex metric needs a large-coordinate AABB (4 verts,
    // so cache counts up to 3 with in-range indices) and an index ordering that
    // makes c2Det2 negative.
    let scale = 3.0e5f32;
    let a = ShapeBlob::aabb(C2AABB {
        min: C2v { x: -scale, y: -scale },
        max: C2v { x: scale, y: scale },
    });
    let b = ShapeBlob::aabb(C2AABB {
        min: C2v { x: -scale * 0.5, y: -scale * 0.5 },
        max: C2v { x: scale * 2.0, y: scale * 0.25 },
    });

    let mut cases: Vec<(C2GJKCache, i32)> = Vec::new();
    for ia0 in 0..4i32 {
        for ia1 in 0..4i32 {
            for ia2 in 0..4i32 {
                for ib in 0..4i32 {
                    for &m in &[0.0f32, 1.0, -1.0, -1.0e9, 1.0e9, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                        cases.push((
                            C2GJKCache {
                                metric: m,
                                count: 3,
                                iA: [ia0, ia1, ia2],
                                iB: [ib, (ib + 1) % 4, (ib + 2) % 4],
                                div: 1.0,
                            },
                            0,
                        ));
                    }
                }
            }
        }
    }
    for _ in 0..2000 {
        cases.push((
            C2GJKCache {
                metric: rng.f32_mixed(),
                count: (rng.below(3) + 1) as i32,
                iA: [rng.below(4) as i32, rng.below(4) as i32, rng.below(4) as i32],
                iB: [rng.below(4) as i32, rng.below(4) as i32, rng.below(4) as i32],
                div: rng.f32_mixed(),
            },
            0,
        ));
    }

    let mut nan_metric_accepted = 0usize;
    for (cache, _) in cases {
        for ur in [0, 1] {
            let opts = GjkOpts { use_radius: ur, cache: Some(cache), ..Default::default() };
            gjk_diff("row29 cache metric guard", &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &opts);
            let cl = classify(&a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &opts);
            if cl.cache_read {
                accepted += 1;
                if cache.metric.is_nan() {
                    nan_metric_accepted += 1;
                }
            } else {
                rejected += 1;
            }
        }
    }
    assert!(rejected > 0, "row 29: the metric guard never rejected a cache");
    assert!(accepted > 0, "row 29: no cache was ever accepted");
    assert!(nan_metric_accepted > 0, "row 30: NaN metric never took the accept path");
    eprintln!("cache metric guard: rejected={rejected} accepted={accepted} (NaN-metric accepts={nan_metric_accepted})");
}

/// ERRORS.md rows 31..38 — every loop exit and every `use_radius` arm of
/// `c2GJK`, with the branch actually taken recorded via `classify()`.
#[test]
fn rows31_38_gjk_exits_and_radius_arms() {
    use std::collections::HashSet;
    let mut brks: HashSet<Brk> = HashSet::new();
    let mut arms: HashSet<RadiusArm> = HashSet::new();
    let mut rng = Rng::new(0xE0_0031);
    let mut max_iter = 0i32;

    // Deterministic constructions for the exits that are easy to force.
    let mut fixed: Vec<(ShapeBlob, i32, ShapeBlob, i32)> = Vec::new();
    // row 33 (dup): two circles -> 1-vertex proxies, so the 2nd support point
    // always duplicates the 1st.
    fixed.push((
        ShapeBlob::circle(C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 3.0 }),
        C2_TYPE_CIRCLE,
        ShapeBlob::circle(C2Circle { p: C2v { x: 50.0, y: 0.0 }, r: 2.0 }),
        C2_TYPE_CIRCLE,
    ));
    // row 32 (degenerate direction): coincident shapes -> d == (0,0).
    for k in VALID_TYPES.iter().copied() {
        let s = ShapeBlob::near(&mut rng, k, C2v { x: 5.0, y: -3.0 }, 4.0);
        fixed.push((s, k, s, k));
    }
    // row 38 (hit): heavily overlapping boxes.
    fixed.push((
        ShapeBlob::aabb(C2AABB { min: C2v { x: -10.0, y: -10.0 }, max: C2v { x: 10.0, y: 10.0 } }),
        C2_TYPE_AABB,
        ShapeBlob::aabb(C2AABB { min: C2v { x: -5.0, y: -5.0 }, max: C2v { x: 5.0, y: 5.0 } }),
        C2_TYPE_AABB,
    ));
    // rows 35,36 (midpoint arm): exactly-touching circles, and coincident ones.
    fixed.push((
        ShapeBlob::circle(C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 2.0 }),
        C2_TYPE_CIRCLE,
        ShapeBlob::circle(C2Circle { p: C2v { x: 4.0, y: 0.0 }, r: 2.0 }),
        C2_TYPE_CIRCLE,
    ));
    // row 37 (shrink collapses to a == b): needs the radius shrink to round
    // exactly onto the other witness point. A zero-extent AABB (rA = 0) at
    // x = 1e7 against a circle one float-step away with r = 0.5: dist == 1.0 so
    // `dist > rA+rB` holds, but b - n*0.5 == 1e7 + 0.5 ties-to-even back to
    // 1e7 == a, so the C forces dist to 0.
    fixed.push((
        ShapeBlob::aabb(C2AABB {
            min: C2v { x: 1.0e7, y: 0.0 },
            max: C2v { x: 1.0e7, y: 0.0 },
        }),
        C2_TYPE_AABB,
        ShapeBlob::circle(C2Circle { p: C2v { x: 1.0e7 + 1.0, y: 0.0 }, r: 0.5 }),
        C2_TYPE_CIRCLE,
    ));
    // row 39 (negative radii): never validated by the C.
    fixed.push((
        ShapeBlob::circle(C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: -10.0 }),
        C2_TYPE_CIRCLE,
        ShapeBlob::capsule(C2Capsule { a: C2v { x: 30.0, y: 0.0 }, b: C2v { x: 40.0, y: 0.0 }, r: -5.0 }),
        C2_TYPE_CAPSULE,
    ));
    // row 40 (non-finite coordinates).
    for k in VALID_TYPES.iter().copied() {
        for i in 0..6 {
            fixed.push((ShapeBlob::degenerate(k, i), k, ShapeBlob::degenerate(k, i + 3), k));
        }
    }

    for (a, ta, b, tb) in &fixed {
        for ur in [0, 1] {
            let opts = GjkOpts { use_radius: ur, ..Default::default() };
            gjk_diff("rows31-40 fixed", a, *ta, b, *tb, &opts);
            let cl = classify(a, *ta, b, *tb, &opts);
            brks.insert(cl.brk);
            arms.insert(cl.radius);
            max_iter = max_iter.max(cl.iters);
        }
    }

    // Broad randomized sweep to reach the remaining exits (notably d1 > d0).
    for (ta, tb) in TYPE_PAIRS {
        for ur in [0, 1] {
            for i in 0..4000 {
                let (a, b) = match i % 5 {
                    0 => {
                        let at = rng.v_tame();
                        let s = rng.range(0.0, 40.0);
                        (ShapeBlob::near(&mut rng, ta, at, s), ShapeBlob::near(&mut rng, tb, at, s))
                    }
                    1 => {
                        let (p1, s1) = (rng.v_tame(), rng.range(0.0, 1.0e-3));
                        let (p2, s2) = (rng.v_tame(), rng.range(0.0, 1.0e-3));
                        (
                            ShapeBlob::near(&mut rng, ta, p1, s1),
                            ShapeBlob::near(&mut rng, tb, p2, s2),
                        )
                    }
                    2 => {
                        let s = 1.0e28;
                        let p1 = C2v { x: rng.range(-1e30, 1e30), y: rng.range(-1e30, 1e30) };
                        let p2 = C2v { x: rng.range(-1e30, 1e30), y: rng.range(-1e30, 1e30) };
                        (
                            ShapeBlob::near(&mut rng, ta, p1, s),
                            ShapeBlob::near(&mut rng, tb, p2, s),
                        )
                    }
                    3 => (ShapeBlob::degenerate(ta, i), ShapeBlob::degenerate(tb, i / 3)),
                    _ => (ShapeBlob::random(&mut rng, ta), ShapeBlob::random(&mut rng, tb)),
                };
                let opts = GjkOpts {
                    use_radius: ur,
                    ax: if i % 3 == 0 { Some(rng.xform_unit()) } else { None },
                    bx: if i % 4 == 0 { Some(rng.xform_nonunit()) } else { None },
                    ..Default::default()
                };
                let out = gjk_diff("rows31-40 sweep", &a, ta, &b, tb, &opts);
                let cl = classify(&a, ta, &b, tb, &opts);
                brks.insert(cl.brk);
                arms.insert(cl.radius);
                max_iter = max_iter.max(cl.iters);
                // row 34: the iteration cap can never be exceeded.
                assert!(
                    (0..=20).contains(&out.iters),
                    "iterations outside [0,20]: {}",
                    out.iters
                );
            }
        }
    }

    eprintln!("GJK loop exits reached: {brks:?}");
    eprintln!("use_radius arms reached: {arms:?}");
    eprintln!("max iterations observed: {max_iter}");
    for want in [Brk::Hit, Brk::NoProgress, Brk::DegenerateDir, Brk::Dup] {
        assert!(brks.contains(&want), "GJK loop exit never reached: {want:?} (got {brks:?})");
    }
    // `Brk::IterCap` is unreachable: the widest proxy has 4 vertices, so the
    // support point must repeat (Brk::Dup) or the simplex must close
    // (Brk::Hit) long before `iter` reaches 20. Row 34 is verified by the
    // `0..=20` bound asserted on every call above instead.
    assert!(
        !brks.contains(&Brk::IterCap),
        "the 20-iteration cap became reachable; ERRORS.md row 34 needs updating"
    );
    for want in [
        RadiusArm::SkippedByHit,
        RadiusArm::Disabled,
        RadiusArm::Shrink,
        RadiusArm::ShrinkCollapsed,
        RadiusArm::Midpoint,
    ] {
        assert!(arms.contains(&want), "use_radius arm never reached: {want:?} (got {arms:?})");
    }
}

/// ERRORS.md rows 41,42 — `c2AABBtoCapsule` / `c2CapsuletoCapsule` return 0
/// exactly when `c2GJK != 0.0f`.
///
/// Both wrappers pass `use_radius = 1`, and in that mode a `NaN` distance always
/// fails `dist > rA + rB` and takes the midpoint arm, which forces `dist` to
/// `0.0f`. So `c2GJK` can never hand these wrappers a `NaN`, and the test
/// asserts that invariant on BOTH implementations rather than pretending to
/// exercise an unreachable branch. The `NaN` distance that `use_radius = 0` does
/// produce is checked separately below.
#[test]
fn rows41_42_gjk_wrapper_sentinels() {
    let l = libs();
    let (cac, rac) = l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let (ccc, rccc) = l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule");
    let (gjk_c, gjk_r) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xE0_0041);

    /// Raw `c2GJK` distance for a shape pair at a given `use_radius`.
    unsafe fn dist(
        f: &libloading::Symbol<'_, FnGJK>,
        a: &ShapeBlob,
        ta: i32,
        b: &ShapeBlob,
        tb: i32,
        ur: i32,
    ) -> f32 {
        unsafe {
            f(
                a.ptr(), ta, ptr::null(),
                b.ptr(), tb, ptr::null(),
                ptr::null_mut(), ptr::null_mut(), ur, ptr::null_mut(), ptr::null_mut(),
            )
        }
    }

    let mut zeros = 0usize;
    let mut ones = 0usize;
    let mut nan_ur0 = 0usize;
    let mut nan_ur1 = 0usize;

    // --- c2AABBtoCapsule -------------------------------------------------
    let mut cases: Vec<(C2AABB, C2Capsule)> = Vec::new();
    for a in degenerate_aabbs() {
        for b in degenerate_capsules() {
            cases.push((a, b));
        }
    }
    for _ in 0..20_000 {
        cases.push((
            C2AABB { min: rng.v_mixed(), max: rng.v_mixed() },
            C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
        ));
    }
    for (a, b) in cases {
        let (cv, rv) = unsafe { (cac(a, b), rac(a, b)) };
        same_i32("c2AABBtoCapsule sentinel", &(a, b), cv, rv);
        if cv == 0 { zeros += 1 } else { ones += 1 }

        let ab = ShapeBlob::aabb(a);
        let cb = ShapeBlob::capsule(b);
        // The sentinel must be exactly `dist != 0.0`, in BOTH libraries.
        let dc = unsafe { dist(&gjk_c, &ab, C2_TYPE_AABB, &cb, C2_TYPE_CAPSULE, 1) };
        let dr = unsafe { dist(&gjk_r, &ab, C2_TYPE_AABB, &cb, C2_TYPE_CAPSULE, 1) };
        same_f32("c2AABBtoCapsule underlying dist", &(a, b), dc, dr);
        assert_eq!(cv, if dc != 0.0 { 0 } else { 1 }, "sentinel disagrees with dist={dc}");
        assert!(!dc.is_nan(), "use_radius=1 must never yield NaN (got {dc} for {a:?} {b:?})");
        if dc.is_nan() { nan_ur1 += 1 }

        // use_radius = 0 CAN yield NaN; both implementations must agree there too.
        let dc0 = unsafe { dist(&gjk_c, &ab, C2_TYPE_AABB, &cb, C2_TYPE_CAPSULE, 0) };
        let dr0 = unsafe { dist(&gjk_r, &ab, C2_TYPE_AABB, &cb, C2_TYPE_CAPSULE, 0) };
        same_f32("raw c2GJK use_radius=0 dist", &(a, b), dc0, dr0);
        if dc0.is_nan() {
            nan_ur0 += 1;
            assert!(dr0.is_nan(), "Rust lost the NaN the C produced");
        }
    }
    assert!(zeros > 0 && ones > 0, "one-sided: zeros={zeros} ones={ones}");
    assert_eq!(nan_ur1, 0, "use_radius=1 produced a NaN distance");
    assert!(nan_ur0 > 0, "no NaN distance was produced even at use_radius=0");

    // --- c2CapsuletoCapsule ----------------------------------------------
    let (mut zeros, mut ones) = (0usize, 0usize);
    let mut nan_ur0b = 0usize;
    let mut cases: Vec<(C2Capsule, C2Capsule)> = Vec::new();
    for a in degenerate_capsules() {
        for b in degenerate_capsules() {
            cases.push((a, b));
        }
    }
    for _ in 0..20_000 {
        cases.push((
            C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
            C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
        ));
    }
    for (a, b) in cases {
        let (cv, rv) = unsafe { (ccc(a, b), rccc(a, b)) };
        same_i32("c2CapsuletoCapsule sentinel", &(a, b), cv, rv);
        if cv == 0 { zeros += 1 } else { ones += 1 }
        let ab = ShapeBlob::capsule(a);
        let bb = ShapeBlob::capsule(b);
        let dc = unsafe { dist(&gjk_c, &ab, C2_TYPE_CAPSULE, &bb, C2_TYPE_CAPSULE, 1) };
        let dr = unsafe { dist(&gjk_r, &ab, C2_TYPE_CAPSULE, &bb, C2_TYPE_CAPSULE, 1) };
        same_f32("c2CapsuletoCapsule underlying dist", &(a, b), dc, dr);
        assert_eq!(cv, if dc != 0.0 { 0 } else { 1 });
        assert!(!dc.is_nan(), "use_radius=1 must never yield NaN");
        let dc0 = unsafe { dist(&gjk_c, &ab, C2_TYPE_CAPSULE, &bb, C2_TYPE_CAPSULE, 0) };
        let dr0 = unsafe { dist(&gjk_r, &ab, C2_TYPE_CAPSULE, &bb, C2_TYPE_CAPSULE, 0) };
        same_f32("raw c2GJK use_radius=0 dist", &(a, b), dc0, dr0);
        if dc0.is_nan() {
            nan_ur0b += 1;
        }
    }
    assert!(zeros > 0 && ones > 0, "one-sided: zeros={zeros} ones={ones}");
    assert!(nan_ur0b > 0, "no NaN distance at use_radius=0 for capsule/capsule");
    eprintln!(
        "wrapper sentinels: NaN at use_radius=1 -> 0 (unreachable, as documented); \
         NaN at use_radius=0 -> aabb/capsule {nan_ur0}, capsule/capsule {nan_ur0b}"
    );
}

/// ERRORS.md rows 43,44 — `c2AABBtoAABB` with inverted bounds and with NaN
/// bounds (all four `<` are false, so the C reports a collision).
/// ERRORS.md rows 45,46 — negative radii and NaN inputs to the circle tests.
/// ERRORS.md rows 47,48 — the degenerate-capsule and NaN arms of
/// `c2CircletoCapsule`.
/// ERRORS.md row 52 — `c2BBVerts` with inverted bounds.
#[test]
fn rows43_52_predicate_degeneracies() {
    let l = libs();
    let (caa, raa) = l.pair::<FnAABBtoAABB>("c2AABBtoAABB");
    let (ccc, rcc) = l.pair::<FnCircletoCircle>("c2CircletoCircle");
    let (cca, rca) = l.pair::<FnCircletoAABB>("c2CircletoAABB");
    let (ccp, rcp) = l.pair::<FnCircletoCapsule>("c2CircletoCapsule");
    let (cbb, rbb) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0xE0_0043);

    // row 44: NaN bounds -> c2AABBtoAABB reports 1.
    let nan_box = C2AABB {
        min: C2v { x: f32::NAN, y: f32::NAN },
        max: C2v { x: f32::NAN, y: f32::NAN },
    };
    let normal = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } };
    for (a, b) in [(nan_box, normal), (normal, nan_box), (nan_box, nan_box)] {
        let (cv, rv) = unsafe { (caa(a, b), raa(a, b)) };
        same_i32("c2AABBtoAABB NaN bounds", &(a, b), cv, rv);
        assert_eq!(cv, 1, "all-NaN comparisons are false -> C returns 1");
    }
    // row 43: inverted bounds.
    let inv = C2AABB { min: C2v { x: 5.0, y: 5.0 }, max: C2v { x: -5.0, y: -5.0 } };
    for (a, b) in [(inv, normal), (normal, inv), (inv, inv)] {
        let (cv, rv) = unsafe { (caa(a, b), raa(a, b)) };
        same_i32("c2AABBtoAABB inverted", &(a, b), cv, rv);
    }
    for _ in 0..4000 {
        let a = C2AABB { min: rng.v_mixed(), max: rng.v_mixed() };
        let b = C2AABB { min: rng.v_mixed(), max: rng.v_mixed() };
        let (cv, rv) = unsafe { (caa(a, b), raa(a, b)) };
        same_i32("c2AABBtoAABB arbitrary", &(a, b), cv, rv);
    }

    // rows 45,46: negative radii (A.r + B.r < 0 still gives r2 > 0) and NaN.
    for &(r1, r2) in &[(-3.0f32, -4.0f32), (-3.0, 1.0), (3.0, -4.0), (f32::NAN, 1.0), (1.0, f32::NAN)] {
        for &d in &[0.0f32, 1.0, 5.0, 10.0] {
            let a = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: r1 };
            let b = C2Circle { p: C2v { x: d, y: 0.0 }, r: r2 };
            let (cv, rv) = unsafe { (ccc(a, b), rcc(a, b)) };
            same_i32("c2CircletoCircle negative/NaN radius", &(a, b), cv, rv);
            if r1.is_nan() || r2.is_nan() {
                assert_eq!(cv, 0, "NaN comparison must be false");
            }
        }
    }
    for _ in 0..4000 {
        let a = C2Circle { p: rng.v_mixed(), r: rng.f32_mixed() };
        let b = C2Circle { p: rng.v_mixed(), r: rng.f32_mixed() };
        let (cv, rv) = unsafe { (ccc(a, b), rcc(a, b)) };
        same_i32("c2CircletoCircle arbitrary", &(a, b), cv, rv);
        let bb = C2AABB { min: rng.v_mixed(), max: rng.v_mixed() };
        let (cv, rv) = unsafe { (cca(a, bb), rca(a, bb)) };
        same_i32("c2CircletoAABB arbitrary", &(a, bb), cv, rv);
    }

    // rows 47,48: degenerate capsule (a == b) must NOT divide by zero, and the
    // NaN/inf coordinate case must agree.
    for &p in &[
        C2v { x: 0.0, y: 0.0 },
        C2v { x: 7.0, y: -2.0 },
        C2v { x: f32::INFINITY, y: 0.0 },
    ] {
        for &rad in &[0.0f32, 3.0, -3.0, f32::NAN] {
            let cap = C2Capsule { a: p, b: p, r: rad };
            for &cp in &[
                C2v { x: 0.0, y: 0.0 },
                C2v { x: 7.0, y: -2.0 },
                C2v { x: 100.0, y: 100.0 },
                C2v { x: f32::NAN, y: 0.0 },
            ] {
                let ci = C2Circle { p: cp, r: 5.0 };
                let (cv, rv) = unsafe { (ccp(ci, cap), rcp(ci, cap)) };
                same_i32("c2CircletoCapsule degenerate", &(ci, cap), cv, rv);
            }
        }
    }
    for _ in 0..6000 {
        let ci = C2Circle { p: rng.v_mixed(), r: rng.f32_mixed() };
        let cap = match rng.below(3) {
            0 => {
                let p = rng.v_mixed();
                C2Capsule { a: p, b: p, r: rng.f32_mixed() }
            }
            1 => C2Capsule { a: rng.v_any(), b: rng.v_any(), r: rng.any_f32() },
            _ => C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
        };
        let (cv, rv) = unsafe { (ccp(ci, cap), rcp(ci, cap)) };
        same_i32("c2CircletoCapsule arbitrary", &(ci, cap), cv, rv);
    }

    // row 52: c2BBVerts with inverted / non-finite bounds.
    for bb in degenerate_aabbs() {
        let mut cv = [C2v { x: f32::from_bits(0x5eed_0001), y: f32::from_bits(0x5eed_0002) }; 8];
        let mut rv = cv;
        let (mut cb, mut rb) = (bb, bb);
        unsafe {
            cbb(cv.as_mut_ptr(), &mut cb);
            rbb(rv.as_mut_ptr(), &mut rb);
        }
        same("c2BBVerts degenerate", &(bb.min, bb.max), &cv, &rv);
    }
}

/// ERRORS.md rows 49,50,51 — `c22` / `c23` degeneracies: coincident vertices,
/// zero area, and the final `else` arm with `div == 0` feeding `c2Witness`.
#[test]
fn rows49_51_solver_degeneracies() {
    let l = libs();
    let (c22c, c22r) = l.pair::<unsafe extern "C" fn(*mut C2Simplex)>("c22");
    let (c23c, c23r) = l.pair::<unsafe extern "C" fn(*mut C2Simplex)>("c23");
    let (cw, rw) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0xE0_0049);
    let mut div_class = [0usize; 4]; // zero, NaN, inf, finite-nonzero

    for i in 0..40_000 {
        // row 49/50: coincident and collinear point sets, plus zero vectors,
        // plus tiny/huge scales that make the barycentric products underflow
        // or overflow.
        let scale: f32 = match i % 7 {
            0 => 1.0e-25,
            1 => 1.0e-38,
            2 => 1.0e25,
            3 => 1.0e30,
            _ => 1.0,
        };
        let ps: [C2v; 4] = match i % 6 {
            0 => [C2v { x: 0.0, y: 0.0 }; 4],
            1 => {
                let p = rng.v_tame();
                [p; 4]
            }
            2 => {
                let d = rng.v_tame();
                [
                    C2v { x: 0.0, y: 0.0 },
                    d,
                    C2v { x: 2.0 * d.x, y: 2.0 * d.y },
                    C2v { x: 3.0 * d.x, y: 3.0 * d.y },
                ]
            }
            3 => [C2v { x: -0.0, y: -0.0 }; 4],
            4 => [rng.v_any(), rng.v_any(), rng.v_any(), rng.v_any()],
            _ => [
                C2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                C2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                C2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
                C2v { x: rng.range(-1.0, 1.0) * scale, y: rng.range(-1.0, 1.0) * scale },
            ],
        };
        let mut s = C2Simplex::default();
        s.div = rng.f32_mixed();
        for k in 0..4 {
            s.v[k] = C2sv {
                sA: rng.v_mixed(),
                sB: rng.v_mixed(),
                p: ps[k],
                u: rng.f32_mixed(),
                iA: k as i32,
                iB: 3 - k as i32,
            };
        }

        for (count, cf, rf) in [(2i32, &c22c, &c22r), (3, &c23c, &c23r)] {
            s.count = count;
            let mut cs = s;
            let mut rs = s;
            unsafe {
                cf(&mut cs);
                rf(&mut rs);
            }
            same("solver degenerate", &(count, ps), &cs, &rs);

            // row 51: feed the solver's (possibly degenerate) `div` straight
            // into c2Witness, exactly as c2GJK does.
            if cs.div == 0.0 {
                div_class[0] += 1;
            } else if cs.div.is_nan() {
                div_class[1] += 1;
            } else if cs.div.is_infinite() {
                div_class[2] += 1;
            } else {
                div_class[3] += 1;
            }
            let poison = C2v { x: f32::from_bits(0x5eed_dead), y: f32::from_bits(0x5eed_c0de) };
            let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
            let mut cs2 = cs;
            let mut rs2 = rs;
            unsafe {
                cw(&mut cs2, &mut ca, &mut cb);
                rw(&mut rs2, &mut ra, &mut rb);
            }
            same("solver -> witness a", &(count, ps), &ca, &ra);
            same("solver -> witness b", &(count, ps), &cb, &rb);
        }
    }
    // A degenerate `div` out of the solvers must actually have occurred and been
    // pushed through c2Witness. `div == 0` is NOT reachable from c22/c23 (see
    // ERRORS.md row 51); NaN and +-inf are, and both are covered here. The
    // `div == 0` input to c2Witness is covered directly by rows #9/#10.
    assert!(
        div_class[1] > 0 && div_class[2] > 0,
        "no degenerate solver `div` was produced: [zero, NaN, inf, finite] = {div_class:?}"
    );
    assert_eq!(
        div_class[0], 0,
        "c22/c23 unexpectedly produced div == 0; ERRORS.md row 51 needs updating"
    );
    eprintln!(
        "solver div classes [zero, NaN, inf, finite-nonzero] = {div_class:?}"
    );
}

/// ERRORS.md row 53 — `reverse_collide` performs no validation at all.
#[test]
fn row53_reverse_collide_unvalidated() {
    let l = libs();
    let (c, r) = l.pair::<FnReverse>("reverse_collide");
    let mut rng = Rng::new(0xE0_0053);

    let mut cases: Vec<(f32, f32, f32)> = Vec::new();
    for &x in SPECIAL {
        for &y in SPECIAL {
            for &r0 in SPECIAL {
                cases.push((x, y, r0));
            }
        }
    }
    for _ in 0..20_000 {
        cases.push((rng.any_f32(), rng.any_f32(), rng.any_f32()));
    }
    for _ in 0..20_000 {
        // Negative radii specifically.
        cases.push((rng.f32_tame(), rng.f32_tame(), -rng.range(0.0, 200.0)));
    }
    for (x, y, r0) in cases {
        let (cv, rv) = unsafe { (c(x, y, r0), r(x, y, r0)) };
        same_i32("reverse_collide unvalidated", &(x, y, r0), cv, rv);
        assert!((0..8).contains(&cv), "result out of range: {cv}");
    }
}

/// The `c2GJK` invalid-enum case is UNDEFINED BEHAVIOUR in C and is therefore
/// NOT differentially tested (see ERRORS.md "deliberately excluded").
///
/// `c2MakeProxy` has no `default:` arm, so for an unknown type it writes nothing
/// and `c2GJK` then reads its *uninitialised stack* `c2Proxy` — including
/// `pA.count`, which is passed straight to `c2Support` as a loop bound. Calling
/// it with a garbage count walks off the end of the 8-element vertex array; this
/// was confirmed empirically to SIGSEGV inside the C `.so`. There is no defined
/// value to compare against, so this test only pins down the part that IS
/// defined: `c2MakeProxy` itself leaves the caller's proxy untouched, which is
/// verified for both implementations in `row5_makeproxy_invalid_enum`.
#[test]
fn gjk_invalid_type_is_ub_documented() {
    let l = libs();
    let (cmp, rmp) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0xE0_0099);

    // The one defined observation: an unknown type produces no writes at all,
    // so a c2GJK caller is left reading whatever the stack held. Both
    // implementations agree on the *proxy* side; the divergence is confined to
    // what c2GJK's own uninitialised local happens to contain.
    for &bad in BAD_TYPES {
        let blob = ShapeBlob::random(&mut rng, C2_TYPE_AABB);
        let sentinel = C2Proxy {
            radius: 12.5,
            count: 4242,
            verts: [C2v { x: 1.0, y: 2.0 }; 8],
        };
        let mut cp = sentinel;
        let mut rp = sentinel;
        unsafe {
            cmp(blob.ptr(), bad, &mut cp);
            rmp(blob.ptr(), bad, &mut rp);
        }
        same("c2MakeProxy unknown type writes nothing", &bad, &cp, &rp);
        same("c2MakeProxy unknown type == sentinel", &bad, &cp, &sentinel);
    }
    eprintln!(
        "c2GJK with an out-of-range C2_TYPE is UB in C (uninitialised c2Proxy.count \
         reaches c2Support and SIGSEGVs); not differentially tested -- see ERRORS.md"
    );
}
