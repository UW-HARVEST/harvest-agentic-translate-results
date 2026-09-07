//! Phase C — error / rejection-path differential tests.
//!
//! One test (or one clearly labelled block) per row of `ERRORS.md`. Each
//! constructs the exact invalid input or condition, calls BOTH libraries through
//! their `.so` exports, and asserts they reject identically — same sentinel, same
//! returned value, same side effects — not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_void;

type CollidedFn = unsafe extern "C" fn(*const c_void, i32, *const c_void, i32) -> i32;
type MakeProxyFn = unsafe extern "C" fn(*const c_void, i32, *mut c2Proxy) -> ();
type SimplexF32Fn = unsafe extern "C" fn(*mut c2Simplex) -> f32;
type SimplexVecFn = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
type WitnessFn = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v) -> ();
type SupportFn = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;

#[allow(clippy::type_complexity)]
type GjkFn = unsafe extern "C" fn(
    *const c_void,
    i32,
    *const c2x,
    *const c_void,
    i32,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;

/// Counts, including out-of-range and one-past-valid values, for the `switch`
/// dispatch inside the simplex helpers.
const BAD_COUNTS: &[i32] = &[
    0,
    -1,
    -2,
    4,
    5,
    100,
    1 << 20,
    i32::MAX,
    i32::MIN,
    i32::MIN + 1,
];

// ===========================================================================
// Rows 1, 2, 3, 4 — c2Collided rejects out-of-range C2_TYPE values
// ===========================================================================

#[test]
fn rows1_4_collided_out_of_range_enum() {
    let (c_f, r_f) = pair::<CollidedFn>("c2Collided");
    let mut rng = Rng::new(0xC001);

    for i in 0..2000 {
        let circ = rng.circle(30.0);
        let aabb = rng.aabb(30.0);
        let cap = rng.capsule(30.0);
        let pc = &circ as *const c2Circle as *const c_void;
        let pa = &aabb as *const c2AABB as *const c_void;
        let pp = &cap as *const c2Capsule as *const c_void;

        for &bad in BAD_TYPES {
            unsafe {
                // Row 1: typeA is out of range -> outer `default:` -> 0.
                for (p, t) in [(pc, C2_TYPE_CIRCLE), (pa, C2_TYPE_AABB), (pp, C2_TYPE_CAPSULE)] {
                    let (cv, rv) = (c_f(pc, bad, p, t), r_f(pc, bad, p, t));
                    assert_eq!(cv, rv, "row1 typeA={bad} typeB={t} #{i}");
                    assert_eq!(cv, 0, "row1 typeA={bad} must reject with 0, got {cv}");
                }

                // Rows 2, 3, 4: typeA valid, typeB out of range -> inner
                // `default:` of the corresponding arm -> 0.
                for (p, t, row) in [
                    (pc, C2_TYPE_CIRCLE, 2),
                    (pa, C2_TYPE_AABB, 3),
                    (pp, C2_TYPE_CAPSULE, 4),
                ] {
                    let (cv, rv) = (c_f(p, t, pc, bad), r_f(p, t, pc, bad));
                    assert_eq!(cv, rv, "row{row} typeA={t} typeB={bad} #{i}");
                    assert_eq!(cv, 0, "row{row} typeB={bad} must reject with 0, got {cv}");
                }

                // Both out of range.
                for &bad2 in BAD_TYPES {
                    let (cv, rv) = (c_f(pc, bad, pc, bad2), r_f(pc, bad, pc, bad2));
                    assert_eq!(cv, rv, "rows1-4 both bad ({bad},{bad2}) #{i}");
                    assert_eq!(cv, 0, "both-bad must reject with 0");
                }
            }
        }
        if i > 40 {
            break; // the enum sweep above is already exhaustive per iteration
        }
    }

    // Null shape pointers combined with an out-of-range type: the C never
    // dereferences the shape on the `default:` path, so this must be safe and
    // return 0 in both.
    for &bad in BAD_TYPES {
        unsafe {
            let n = std::ptr::null::<c_void>();
            assert_eq!(c_f(n, bad, n, bad), r_f(n, bad, n, bad), "null+bad {bad}");
            assert_eq!(c_f(n, bad, n, bad), 0, "null+bad {bad} must be 0");
        }
    }
}

// ===========================================================================
// Row 5 — c2MakeProxy with an out-of-range type leaves *p untouched
// ===========================================================================

#[test]
fn row5_make_proxy_out_of_range_enum_writes_nothing() {
    let (c_f, r_f) = pair::<MakeProxyFn>("c2MakeProxy");
    let mut rng = Rng::new(0xC005);

    for i in 0..500 {
        let circ = rng.circle(30.0);
        let shape = &circ as *const c2Circle as *const c_void;
        // Distinctive pre-existing contents; the C `switch` has no `default:`
        // label, so every byte must survive verbatim.
        let seed = c2Proxy {
            radius: 1.5 + i as f32,
            count: -777 - i as i32,
            verts: std::array::from_fn(|k| c2v {
                x: (i * 8 + k) as f32,
                y: -((i * 8 + k) as f32) - 0.5,
            }),
        };
        for &bad in BAD_TYPES {
            unsafe {
                let mut cp = seed;
                let mut rp = seed;
                c_f(shape, bad, &mut cp);
                r_f(shape, bad, &mut rp);
                assert!(
                    proxy_bits_eq(&cp, &rp),
                    "row5 type={bad} #{i}\n  C={cp:?}\n Rs={rp:?}"
                );
                assert!(
                    proxy_bits_eq(&cp, &seed),
                    "row5 type={bad} #{i}: C wrote to the proxy but must not have"
                );
            }
        }
    }
}

// ===========================================================================
// Row 6 — c2GJKSimplexMetric with a count outside {2,3} returns 0
// ===========================================================================

#[test]
fn row6_simplex_metric_bad_count() {
    let (c_f, r_f) = pair::<SimplexF32Fn>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0xC006);

    for &count in BAD_COUNTS.iter().chain([1i32].iter()) {
        for i in 0..500 {
            let mut s = rng.simplex(100.0, count);
            s.count = count;
            unsafe {
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = (c_f(&mut cs), r_f(&mut rs));
                assert_f32_bits!(cv, rv, "row6 count={count} #{i}");
                assert_eq!(
                    cv.to_bits(),
                    0.0f32.to_bits(),
                    "row6 count={count} must return +0.0, got {cv}"
                );
                assert!(
                    simplex_bits_eq(&cs, &rs),
                    "row6 count={count} #{i} simplex mutated differently"
                );
            }
        }
    }
}

// ===========================================================================
// Row 7 — c2D with a count outside {1,2} returns (0,0)
// ===========================================================================

#[test]
fn row7_c2d_bad_count() {
    let (c_f, r_f) = pair::<SimplexVecFn>("c2D");
    let mut rng = Rng::new(0xC007);

    for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
        for i in 0..500 {
            let mut s = rng.simplex(100.0, count);
            s.count = count;
            unsafe {
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = (c_f(&mut cs), r_f(&mut rs));
                assert_v_bits!(cv, rv, "row7 count={count} #{i}");
                assert!(
                    cv.x.to_bits() == 0 && cv.y.to_bits() == 0,
                    "row7 count={count} must return (+0,+0), got {cv:?}"
                );
            }
        }
    }
}

// ===========================================================================
// Row 8 — c2Witness with a count outside {1,2,3} writes (0,0) to both outputs
// ===========================================================================

#[test]
fn row8_witness_bad_count() {
    let (c_f, r_f) = pair::<WitnessFn>("c2Witness");
    let mut rng = Rng::new(0xC008);

    for &count in BAD_COUNTS {
        for i in 0..500 {
            let mut s = rng.simplex(100.0, count);
            s.count = count;
            let sent = c2v {
                x: 4242.5,
                y: -8484.25,
            };
            unsafe {
                let (mut ca, mut cb) = (sent, sent);
                let (mut ra, mut rb) = (sent, sent);
                let mut cs = s;
                let mut rs = s;
                c_f(&mut cs, &mut ca, &mut cb);
                r_f(&mut rs, &mut ra, &mut rb);
                assert_v_bits!(ca, ra, "row8 count={count} #{i} outA");
                assert_v_bits!(cb, rb, "row8 count={count} #{i} outB");
                assert!(
                    ca.x.to_bits() == 0
                        && ca.y.to_bits() == 0
                        && cb.x.to_bits() == 0
                        && cb.y.to_bits() == 0,
                    "row8 count={count} must zero both outputs, got {ca:?} {cb:?}"
                );
            }
        }
    }
}

// ===========================================================================
// Row 9 — c2L with a count outside {1,2} returns (0,0)
// ===========================================================================

#[test]
fn row9_c2l_bad_count() {
    let (c_f, r_f) = pair::<SimplexVecFn>("c2L");
    let mut rng = Rng::new(0xC009);

    for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
        for i in 0..500 {
            let mut s = rng.simplex(100.0, count);
            s.count = count;
            unsafe {
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = (c_f(&mut cs), r_f(&mut rs));
                assert_v_bits!(cv, rv, "row9 count={count} #{i}");
                assert!(
                    cv.x.to_bits() == 0 && cv.y.to_bits() == 0,
                    "row9 count={count} must return (+0,+0), got {cv:?}"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 10, 11 — div == 0 / -0 makes den infinite in c2Witness and c2L
// ===========================================================================

#[test]
fn rows10_11_zero_and_negative_zero_div() {
    let (c_l, r_l) = pair::<SimplexVecFn>("c2L");
    let (c_w, r_w) = pair::<WitnessFn>("c2Witness");
    let mut rng = Rng::new(0xC010);

    let divs: &[f32] = &[
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN,
    ];

    for &div in divs {
        for count in [1i32, 2, 3] {
            for i in 0..400 {
                let mut s = rng.simplex(50.0, count);
                s.div = div;
                // also exercise u == 0 and u == inf
                if i % 3 == 0 {
                    for v in s.verts.iter_mut() {
                        v.u = 0.0;
                    }
                } else if i % 3 == 1 {
                    s.verts[0].u = f32::INFINITY;
                }
                unsafe {
                    let mut cs = s;
                    let mut rs = s;
                    let (cv, rv) = (c_l(&mut cs), r_l(&mut rs));
                    assert_v_bits!(cv, rv, "rows10-11 c2L div={div} count={count} #{i}");

                    let (mut ca, mut cb) = (c2v::default(), c2v::default());
                    let (mut ra, mut rb) = (c2v::default(), c2v::default());
                    let mut cs = s;
                    let mut rs = s;
                    c_w(&mut cs, &mut ca, &mut cb);
                    r_w(&mut rs, &mut ra, &mut rb);
                    assert_v_bits!(ca, ra, "rows10-11 witness A div={div} count={count} #{i}");
                    assert_v_bits!(cb, rb, "rows10-11 witness B div={div} count={count} #{i}");
                }
            }
        }
    }
}

// ===========================================================================
// Row 12 — c2Support with count <= 0 still reads verts[0] and returns 0
// ===========================================================================

#[test]
fn row12_support_nonpositive_count() {
    let (c_f, r_f) = pair::<SupportFn>("c2Support");
    let mut rng = Rng::new(0xC012);

    for &count in &[0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        for i in 0..500 {
            // verts[0] must be readable: the C dereferences it unconditionally.
            let verts: [c2v; 8] = std::array::from_fn(|_| rng.vec(100.0));
            let d = rng.vec(10.0);
            unsafe {
                let (cv, rv) = (
                    c_f(verts.as_ptr(), count, d),
                    r_f(verts.as_ptr(), count, d),
                );
                assert_eq!(cv, rv, "row12 count={count} #{i}");
                assert_eq!(cv, 0, "row12 count={count} must return index 0, got {cv}");
            }
        }
    }
    // count == 1 is the boundary just inside the valid range.
    let verts = [c2v { x: 7.0, y: -7.0 }; 8];
    for &dx in EDGE_F32 {
        let d = c2v { x: dx, y: dx };
        unsafe {
            assert_eq!(
                c_f(verts.as_ptr(), 1, d),
                r_f(verts.as_ptr(), 1, d),
                "row12 boundary count=1 d={d:?}"
            );
        }
    }
}

// ===========================================================================
// Rows 13, 14 — c2Div by zero and c2Norm of the zero vector
// ===========================================================================

#[test]
fn rows13_14_div_by_zero_and_norm_of_zero() {
    type VS = unsafe extern "C" fn(c2v, f32) -> c2v;
    type V1 = unsafe extern "C" fn(c2v) -> c2v;
    let (c_div, r_div) = pair::<VS>("c2Div");
    let (c_norm, r_norm) = pair::<V1>("c2Norm");

    // Row 13: every combination of edge numerator with a zero / subnormal /
    // infinite denominator.
    for &b in &[
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ] {
        for &x in EDGE_F32 {
            for &y in EDGE_F32 {
                let a = c2v { x, y };
                unsafe {
                    assert_v_bits!(c_div(a, b), r_div(a, b), "row13 c2Div {a:?} / {b}");
                }
            }
        }
    }

    // Row 14: c2Norm of exactly zero, of -0, and of vectors whose length
    // underflows or overflows.
    let zeros: &[c2v] = &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v {
            x: 1e-30,
            y: 1e-30,
        },
        c2v {
            x: f32::MIN_POSITIVE,
            y: 0.0,
        },
        c2v {
            x: f32::MAX,
            y: f32::MAX,
        },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
        c2v {
            x: f32::NAN,
            y: 0.0,
        },
    ];
    for &a in zeros {
        unsafe {
            assert_v_bits!(c_norm(a), r_norm(a), "row14 c2Norm {a:?}");
        }
    }
}

// ===========================================================================
// Rows 15–21, 29 — c2GJK pointer-nullness and cache-presence rejections
// ===========================================================================

#[test]
fn rows15_21_gjk_null_pointers_and_cache_presence() {
    let (c_f, r_f) = pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0xC015);

    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..400 {
                let circ = rng.circle(30.0);
                let aabb = rng.aabb(30.0);
                let cap = rng.capsule(30.0);
                let sel = |t: i32| -> *const c_void {
                    match t {
                        C2_TYPE_CIRCLE => &circ as *const c2Circle as *const c_void,
                        C2_TYPE_AABB => &aabb as *const c2AABB as *const c_void,
                        _ => &cap as *const c2Capsule as *const c_void,
                    }
                };
                let (pa, pb) = (sel(ta), sel(tb));
                let use_radius = (i % 2) as i32;
                let _xf = rng.xform(20.0);

                unsafe {
                    // Rows 15, 16: NULL transform == explicit identity transform,
                    // in every nullness combination.
                    let variants: [(*const c2x, *const c2x, &str); 4] = [
                        (std::ptr::null(), std::ptr::null(), "both-null"),
                        (&ident, std::ptr::null(), "ax-ident"),
                        (std::ptr::null(), &ident, "bx-ident"),
                        (&ident, &ident, "both-ident"),
                    ];
                    let mut results = Vec::new();
                    for (axp, bxp, tag) in variants {
                        let (mut ca, mut cb, mut ci) = (c2v::default(), c2v::default(), 0i32);
                        let (mut ra, mut rb, mut ri) = (c2v::default(), c2v::default(), 0i32);
                        let cd = c_f(
                            pa, ta, axp, pb, tb, bxp, &mut ca, &mut cb, use_radius, &mut ci,
                            std::ptr::null_mut(),
                        );
                        let rd = r_f(
                            pa, ta, axp, pb, tb, bxp, &mut ra, &mut rb, use_radius, &mut ri,
                            std::ptr::null_mut(),
                        );
                        assert_f32_bits!(cd, rd, "rows15-16 {tag} ({ta},{tb}) #{i}");
                        assert_v_bits!(ca, ra, "rows15-16 {tag} outA ({ta},{tb}) #{i}");
                        assert_v_bits!(cb, rb, "rows15-16 {tag} outB ({ta},{tb}) #{i}");
                        assert_eq!(ci, ri, "rows15-16 {tag} iters ({ta},{tb}) #{i}");
                        results.push(cd.to_bits());
                    }
                    // NULL must be exactly equivalent to an identity transform.
                    assert!(
                        results.windows(2).all(|w| w[0] == w[1]),
                        "rows15-16: NULL transform diverged from explicit identity \
                         ({ta},{tb}) #{i}: {results:?}"
                    );

                    // Rows 17, 18, 19: each out pointer NULL individually, with
                    // sentinels proving the other outputs are still written and
                    // the NULL one is skipped rather than faulting.
                    let sent_v = c2v {
                        x: -1234.5,
                        y: 6789.0,
                    };
                    let sent_i = -31337i32;
                    for which in 0..3 {
                        let (mut ca, mut cb, mut ci) = (sent_v, sent_v, sent_i);
                        let (mut ra, mut rb, mut ri) = (sent_v, sent_v, sent_i);
                        let (oa_c, ob_c, it_c): (*mut c2v, *mut c2v, *mut i32) = match which {
                            0 => (std::ptr::null_mut(), &mut cb, &mut ci),
                            1 => (&mut ca, std::ptr::null_mut(), &mut ci),
                            _ => (&mut ca, &mut cb, std::ptr::null_mut()),
                        };
                        let (oa_r, ob_r, it_r): (*mut c2v, *mut c2v, *mut i32) = match which {
                            0 => (std::ptr::null_mut(), &mut rb, &mut ri),
                            1 => (&mut ra, std::ptr::null_mut(), &mut ri),
                            _ => (&mut ra, &mut rb, std::ptr::null_mut()),
                        };
                        let cd = c_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            oa_c,
                            ob_c,
                            use_radius,
                            it_c,
                            std::ptr::null_mut(),
                        );
                        let rd = r_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            oa_r,
                            ob_r,
                            use_radius,
                            it_r,
                            std::ptr::null_mut(),
                        );
                        assert_f32_bits!(cd, rd, "rows17-19 which={which} ({ta},{tb}) #{i}");
                        assert_v_bits!(ca, ra, "rows17-19 which={which} A ({ta},{tb}) #{i}");
                        assert_v_bits!(cb, rb, "rows17-19 which={which} B ({ta},{tb}) #{i}");
                        assert_eq!(ci, ri, "rows17-19 which={which} it ({ta},{tb}) #{i}");
                        // the NULLed slot must remain at its sentinel in both
                        match which {
                            0 => assert!(
                                v_bits_eq(ca, sent_v) && v_bits_eq(ra, sent_v),
                                "row17: outA=NULL but a value was written"
                            ),
                            1 => assert!(
                                v_bits_eq(cb, sent_v) && v_bits_eq(rb, sent_v),
                                "row18: outB=NULL but a value was written"
                            ),
                            _ => assert!(
                                ci == sent_i && ri == sent_i,
                                "row19: iterations=NULL but a value was written"
                            ),
                        }
                    }

                    // Row 20 vs Row 21: cache == NULL must give the same distance
                    // as a cold cache (count == 0), and the cold cache must still
                    // be written back identically by both libraries.
                    let mut c_cache = c2GJKCache {
                        metric: 12345.0,
                        count: 0,
                        iA: [9, 9, 9],
                        iB: [9, 9, 9],
                        div: -4242.0,
                    };
                    let mut r_cache = c_cache;
                    let d_nocache = c_f(
                        pa,
                        ta,
                        std::ptr::null(),
                        pb,
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        use_radius,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    let cd = c_f(
                        pa,
                        ta,
                        std::ptr::null(),
                        pb,
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        use_radius,
                        std::ptr::null_mut(),
                        &mut c_cache,
                    );
                    let rd = r_f(
                        pa,
                        ta,
                        std::ptr::null(),
                        pb,
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        use_radius,
                        std::ptr::null_mut(),
                        &mut r_cache,
                    );
                    assert_f32_bits!(cd, rd, "rows20-21 cold-cache dist ({ta},{tb}) #{i}");
                    assert_f32_bits!(
                        d_nocache,
                        cd,
                        "row21: cold cache changed the C result vs no cache"
                    );
                    assert!(
                        cache_bits_eq(&c_cache, &r_cache),
                        "rows20-21 writeback ({ta},{tb}) #{i}\n  C={c_cache:?}\n Rs={r_cache:?}"
                    );
                    assert!(
                        c_cache.count >= 1 && c_cache.count <= 3,
                        "row21: cache must be written back with a live count, got {}",
                        c_cache.count
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 22 — the cache-validity predicate `!(min < max*2 && metric < -1.0e8f)`
// Row 30 — warm cache with in-range indices, including a stale `div`
// ===========================================================================

#[test]
fn rows22_30_cache_validity_predicate_and_stale_div() {
    let (c_f, r_f) = pair::<GjkFn>("c2GJK");

    // An AABB with huge extents against a point circle at the origin makes the
    // recomputed simplex metric enormous; ordering the cached indices as
    // [0,2,1] makes the determinant NEGATIVE, so `metric < -1.0e8f` holds and
    // the second half of the predicate becomes reachable. `metric_old` (which
    // the caller controls via `cache->metric`) then decides the outcome.
    let big = c2AABB {
        min: c2v { x: -1e10, y: -1e10 },
        max: c2v { x: 1e10, y: 1e10 },
    };
    let pt = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    let pa = &big as *const c2AABB as *const c_void;
    let pb = &pt as *const c2Circle as *const c_void;

    let metric_olds: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        1e8,
        -1e8,
        -1e9,
        -1e20,
        -4e20,
        -8e20,
        1e20,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    // index triples that are all inside the AABB proxy's valid 0..=3 range
    let index_sets: &[[i32; 3]] = &[
        [0, 1, 2],
        [0, 2, 1],
        [1, 2, 3],
        [3, 2, 1],
        [2, 0, 1],
        [0, 0, 0],
        [1, 1, 1],
        [3, 3, 3],
        [0, 3, 1],
        [2, 3, 0],
    ];
    let divs: &[f32] = &[1.0, 0.0, -0.0, 2.0, -3.5, 1e20, -1e20, f32::NAN, f32::INFINITY];

    let mut both_predicate_outcomes = std::collections::BTreeSet::new();

    for &count in &[1i32, 2, 3] {
        for ia in index_sets {
            for &metric in metric_olds {
                for &div in divs {
                    for use_radius in [0i32, 1] {
                        let seed = c2GJKCache {
                            metric,
                            count,
                            iA: *ia,
                            // circle proxy has exactly one vertex, so iB must be 0
                            iB: [0, 0, 0],
                            div,
                        };
                        unsafe {
                            let mut cc = seed;
                            let mut rc = seed;
                            let (mut ca, mut cb, mut ci) =
                                (c2v::default(), c2v::default(), 0i32);
                            let (mut ra, mut rb, mut ri) =
                                (c2v::default(), c2v::default(), 0i32);
                            let cd = c_f(
                                pa,
                                C2_TYPE_AABB,
                                std::ptr::null(),
                                pb,
                                C2_TYPE_CIRCLE,
                                std::ptr::null(),
                                &mut ca,
                                &mut cb,
                                use_radius,
                                &mut ci,
                                &mut cc,
                            );
                            let rd = r_f(
                                pa,
                                C2_TYPE_AABB,
                                std::ptr::null(),
                                pb,
                                C2_TYPE_CIRCLE,
                                std::ptr::null(),
                                &mut ra,
                                &mut rb,
                                use_radius,
                                &mut ri,
                                &mut rc,
                            );
                            let ctx = format!(
                                "count={count} iA={ia:?} metric_old={metric} div={div} \
                                 ur={use_radius}"
                            );
                            assert_f32_bits!(cd, rd, "rows22/30 dist :: {ctx}");
                            assert_v_bits!(ca, ra, "rows22/30 outA :: {ctx}");
                            assert_v_bits!(cb, rb, "rows22/30 outB :: {ctx}");
                            assert_eq!(ci, ri, "rows22/30 iters :: {ctx}");
                            assert!(
                                cache_bits_eq(&cc, &rc),
                                "rows22/30 cache :: {ctx}\n  C={cc:?}\n Rs={rc:?}"
                            );
                            both_predicate_outcomes.insert(ci);
                        }
                    }
                }
            }
        }
    }
    // A cold seed (predicate false -> cache_was_read == 0) and a trusted warm
    // seed (predicate true) lead to different iteration counts; seeing more than
    // one distinct value proves both sides of the predicate were exercised.
    assert!(
        both_predicate_outcomes.len() > 1,
        "row22: only one predicate outcome observed: {both_predicate_outcomes:?}"
    );

    // Same again with a capsule as B so iB has two valid indices.
    let cap = c2Capsule {
        a: c2v { x: -3.0, y: 1.0 },
        b: c2v { x: 5.0, y: -2.0 },
        r: 0.75,
    };
    let pb2 = &cap as *const c2Capsule as *const c_void;
    for &count in &[1i32, 2, 3] {
        for ia in index_sets {
            for ib in &[[0i32, 0, 0], [1, 1, 1], [0, 1, 0], [1, 0, 1]] {
                for &metric in &[0.0f32, -1e20, 1e20, f32::NAN] {
                    for &div in &[1.0f32, 0.0, -2.0] {
                        let seed = c2GJKCache {
                            metric,
                            count,
                            iA: *ia,
                            iB: *ib,
                            div,
                        };
                        unsafe {
                            let mut cc = seed;
                            let mut rc = seed;
                            let (mut ca, mut cb, mut ci) =
                                (c2v::default(), c2v::default(), 0i32);
                            let (mut ra, mut rb, mut ri) =
                                (c2v::default(), c2v::default(), 0i32);
                            let cd = c_f(
                                pa,
                                C2_TYPE_AABB,
                                std::ptr::null(),
                                pb2,
                                C2_TYPE_CAPSULE,
                                std::ptr::null(),
                                &mut ca,
                                &mut cb,
                                1,
                                &mut ci,
                                &mut cc,
                            );
                            let rd = r_f(
                                pa,
                                C2_TYPE_AABB,
                                std::ptr::null(),
                                pb2,
                                C2_TYPE_CAPSULE,
                                std::ptr::null(),
                                &mut ra,
                                &mut rb,
                                1,
                                &mut ri,
                                &mut rc,
                            );
                            let ctx =
                                format!("cap count={count} iA={ia:?} iB={ib:?} m={metric} d={div}");
                            assert_f32_bits!(cd, rd, "row30 dist :: {ctx}");
                            assert_v_bits!(ca, ra, "row30 outA :: {ctx}");
                            assert_v_bits!(cb, rb, "row30 outB :: {ctx}");
                            assert_eq!(ci, ri, "row30 iters :: {ctx}");
                            assert!(cache_bits_eq(&cc, &rc), "row30 cache :: {ctx}");
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 23, 24, 25 — iteration cap, epsilon direction break, `d1 > d0` break
// ===========================================================================

#[test]
fn rows23_24_25_loop_exit_conditions() {
    let (c_f, r_f) = pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0xC023);

    let mut max_iter_c = i32::MIN;
    let mut seen = std::collections::BTreeSet::new();

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..3000 {
                // Coincident and near-coincident shapes drive `c2Dot(d,d)` below
                // FLT_EPSILON^2 (row 24) and make `d1 > d0` fire (row 25).
                let base = rng.vec(10.0);
                let jitter = match i % 5 {
                    0 => c2v { x: 0.0, y: 0.0 },
                    1 => c2v {
                        x: FLT_EPSILON,
                        y: 0.0,
                    },
                    2 => c2v {
                        x: FLT_EPSILON * FLT_EPSILON,
                        y: -FLT_EPSILON * FLT_EPSILON,
                    },
                    3 => c2v { x: 1e-30, y: 1e-30 },
                    _ => rng.vec(0.001),
                };
                let mk = |t: i32, p: c2v| -> (c2Circle, c2AABB, c2Capsule, i32) {
                    (
                        c2Circle { p, r: 0.0 },
                        c2AABB { min: p, max: p },
                        c2Capsule { a: p, b: p, r: 0.0 },
                        t,
                    )
                };
                let (ca_, aa_, pa_, _) = mk(ta, base);
                let (cb_, ab_, pb_, _) = mk(
                    tb,
                    c2v {
                        x: base.x + jitter.x,
                        y: base.y + jitter.y,
                    },
                );
                let sel = |t: i32,
                           c: &c2Circle,
                           a: &c2AABB,
                           p: &c2Capsule|
                 -> *const c_void {
                    match t {
                        C2_TYPE_CIRCLE => c as *const c2Circle as *const c_void,
                        C2_TYPE_AABB => a as *const c2AABB as *const c_void,
                        _ => p as *const c2Capsule as *const c_void,
                    }
                };
                let pa = sel(ta, &ca_, &aa_, &pa_);
                let pb = sel(tb, &cb_, &ab_, &pb_);

                for use_radius in [0i32, 1] {
                    unsafe {
                        let (mut xa, mut xb, mut ci) = (c2v::default(), c2v::default(), -1i32);
                        let (mut ya, mut yb, mut ri) = (c2v::default(), c2v::default(), -1i32);
                        let cd = c_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            &mut xa,
                            &mut xb,
                            use_radius,
                            &mut ci,
                            std::ptr::null_mut(),
                        );
                        let rd = r_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            &mut ya,
                            &mut yb,
                            use_radius,
                            &mut ri,
                            std::ptr::null_mut(),
                        );
                        let ctx = format!("({ta},{tb}) ur={use_radius} #{i} jitter={jitter:?}");
                        assert_f32_bits!(cd, rd, "rows23-25 dist :: {ctx}");
                        assert_v_bits!(xa, ya, "rows23-25 outA :: {ctx}");
                        assert_v_bits!(xb, yb, "rows23-25 outB :: {ctx}");
                        assert_eq!(ci, ri, "rows23-25 iters :: {ctx}");
                        // Row 23: the hard cap must hold in both.
                        assert!(
                            (0..=20).contains(&ci),
                            "row23: iteration count {ci} outside [0,20] :: {ctx}"
                        );
                        max_iter_c = max_iter_c.max(ci);
                        seen.insert(ci);
                    }
                }
            }
        }
    }
    // The degenerate point-shapes above all terminate immediately, which
    // exercises rows 24/25 but pins `iter` at 0. Add non-degenerate shapes so the
    // loop actually advances and row 23's cap is observed against live counts.
    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..3000 {
                let a = match ta {
                    C2_TYPE_CIRCLE => (Some(rng.circle(20.0)), None, None),
                    C2_TYPE_AABB => (None, Some(rng.aabb(20.0)), None),
                    _ => (None, None, Some(rng.capsule(20.0))),
                };
                let b = match tb {
                    C2_TYPE_CIRCLE => (Some(rng.circle(20.0)), None, None),
                    C2_TYPE_AABB => (None, Some(rng.aabb(20.0)), None),
                    _ => (None, None, Some(rng.capsule(20.0))),
                };
                let pick = |t: (Option<c2Circle>, Option<c2AABB>, Option<c2Capsule>)| t;
                let a = pick(a);
                let b = pick(b);
                let ptr_of = |t: &(Option<c2Circle>, Option<c2AABB>, Option<c2Capsule>)| {
                    if let Some(c) = t.0.as_ref() {
                        c as *const c2Circle as *const c_void
                    } else if let Some(c) = t.1.as_ref() {
                        c as *const c2AABB as *const c_void
                    } else {
                        t.2.as_ref().unwrap() as *const c2Capsule as *const c_void
                    }
                };
                let (pa, pb) = (ptr_of(&a), ptr_of(&b));
                for use_radius in [0i32, 1] {
                    unsafe {
                        let (mut xa, mut xb, mut ci) = (c2v::default(), c2v::default(), -1i32);
                        let (mut ya, mut yb, mut ri) = (c2v::default(), c2v::default(), -1i32);
                        let cd = c_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            &mut xa,
                            &mut xb,
                            use_radius,
                            &mut ci,
                            std::ptr::null_mut(),
                        );
                        let rd = r_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            &mut ya,
                            &mut yb,
                            use_radius,
                            &mut ri,
                            std::ptr::null_mut(),
                        );
                        let ctx = format!("live ({ta},{tb}) ur={use_radius} #{i}");
                        assert_f32_bits!(cd, rd, "row23 dist :: {ctx}");
                        assert_v_bits!(xa, ya, "row23 outA :: {ctx}");
                        assert_v_bits!(xb, yb, "row23 outB :: {ctx}");
                        assert_eq!(ci, ri, "row23 iters :: {ctx}");
                        assert!(
                            (0..=20).contains(&ci),
                            "row23: iteration count {ci} outside [0,20] :: {ctx}"
                        );
                        max_iter_c = max_iter_c.max(ci);
                        seen.insert(ci);
                    }
                }
            }
        }
    }

    assert!(
        seen.len() > 1,
        "rows23-25: only one iteration count observed ({seen:?}); the loop-exit \
         conditions were not differentiated"
    );
    assert!(max_iter_c <= 20, "row23: cap violated, saw {max_iter_c}");
    assert!(
        seen.contains(&0),
        "row24/25: the immediate-break paths were never taken"
    );
}

// ===========================================================================
// Rows 26, 27, 28, 29 — the post-loop radius / hit handling
// ===========================================================================

#[test]
fn rows26_29_radius_and_hit_paths() {
    let (c_f, r_f) = pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0xC026);

    let mut zero_dist = 0usize;
    let mut pos_dist = 0usize;

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..2500 {
                // Sweep the separation across the radius sum so `dist > rA+rB`
                // flips, hitting both the shrink branch (rows 27, 29) and the
                // midpoint-collapse branch (row 26).
                let ra = rng.radius(5.0);
                let rb = rng.radius(5.0);
                let sep = (ra + rb) * rng.range(0.0, 2.5);
                let p0 = rng.vec(10.0);
                let p1 = c2v {
                    x: p0.x + sep,
                    y: p0.y,
                };
                let shapes_a = (
                    c2Circle { p: p0, r: ra },
                    c2AABB { min: p0, max: p0 },
                    c2Capsule {
                        a: p0,
                        b: p0,
                        r: ra,
                    },
                );
                let shapes_b = (
                    c2Circle { p: p1, r: rb },
                    c2AABB { min: p1, max: p1 },
                    c2Capsule {
                        a: p1,
                        b: p1,
                        r: rb,
                    },
                );
                let sel = |t: i32, s: &(c2Circle, c2AABB, c2Capsule)| -> *const c_void {
                    match t {
                        C2_TYPE_CIRCLE => &s.0 as *const c2Circle as *const c_void,
                        C2_TYPE_AABB => &s.1 as *const c2AABB as *const c_void,
                        _ => &s.2 as *const c2Capsule as *const c_void,
                    }
                };
                let pa = sel(ta, &shapes_a);
                let pb = sel(tb, &shapes_b);

                for use_radius in [0i32, 1] {
                    unsafe {
                        let (mut xa, mut xb, mut ci) = (c2v::default(), c2v::default(), -1i32);
                        let (mut ya, mut yb, mut ri) = (c2v::default(), c2v::default(), -1i32);
                        let cd = c_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            &mut xa,
                            &mut xb,
                            use_radius,
                            &mut ci,
                            std::ptr::null_mut(),
                        );
                        let rd = r_f(
                            pa,
                            ta,
                            std::ptr::null(),
                            pb,
                            tb,
                            std::ptr::null(),
                            &mut ya,
                            &mut yb,
                            use_radius,
                            &mut ri,
                            std::ptr::null_mut(),
                        );
                        let ctx = format!(
                            "({ta},{tb}) ur={use_radius} #{i} rA={ra} rB={rb} sep={sep}"
                        );
                        assert_f32_bits!(cd, rd, "rows26-29 dist :: {ctx}");
                        assert_v_bits!(xa, ya, "rows26-29 outA :: {ctx}");
                        assert_v_bits!(xb, yb, "rows26-29 outB :: {ctx}");
                        assert_eq!(ci, ri, "rows26-29 iters :: {ctx}");
                        if cd == 0.0 {
                            zero_dist += 1;
                        } else {
                            pos_dist += 1;
                        }
                        // Row 28: when the C reports a hit it must set a == b.
                        if use_radius == 0 && cd == 0.0 {
                            assert!(
                                v_bits_eq(xa, xb) == v_bits_eq(ya, yb),
                                "row28: witness-point collapse disagreed :: {ctx}"
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(
        zero_dist > 0 && pos_dist > 0,
        "rows26-29: needed both zero and positive distances, got {zero_dist}/{pos_dist}"
    );
}

// ===========================================================================
// Rows 31, 32 — the C float->bool truthiness in the GJK-backed wrappers
// ===========================================================================

#[test]
fn rows31_32_wrapper_truthiness() {
    type AC = unsafe extern "C" fn(c2AABB, c2Capsule) -> i32;
    type CC = unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32;
    let (c_ac, r_ac) = pair::<AC>("c2AABBtoCapsule");
    let (c_cc, r_cc) = pair::<CC>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0xC031);

    let mut ones = 0usize;
    let mut zeros = 0usize;

    for i in 0..8000 {
        // Sweep separation through the exact touching point so the underlying
        // c2GJK return crosses 0.0, plus NaN-producing inputs where C's
        // `if (float)` is TRUE (NaN != 0) and must yield 0.
        let r = rng.radius(4.0);
        let p = rng.vec(15.0);
        let sep = match i % 6 {
            0 => 0.0,
            1 => r,
            2 => r * 2.0,
            3 => FLT_EPSILON,
            4 => -FLT_EPSILON,
            _ => rng.range(0.0, 12.0),
        };
        let bb = c2AABB {
            min: p,
            max: c2v {
                x: p.x + rng.radius(6.0),
                y: p.y + rng.radius(6.0),
            },
        };
        let cap = c2Capsule {
            a: c2v {
                x: bb.max.x + sep,
                y: p.y,
            },
            b: c2v {
                x: bb.max.x + sep,
                y: p.y + rng.radius(6.0),
            },
            r,
        };
        unsafe {
            let (cv, rv) = (c_ac(bb, cap), r_ac(bb, cap));
            assert_eq!(cv, rv, "row31 #{i} {bb:?} {cap:?}");
            assert!(cv == 0 || cv == 1, "row31: non-boolean result {cv}");
            if cv == 1 {
                ones += 1
            } else {
                zeros += 1
            }

            let cap2 = c2Capsule {
                a: p,
                b: c2v {
                    x: p.x + 1.0,
                    y: p.y,
                },
                r,
            };
            let (cv, rv) = (c_cc(cap2, cap), r_cc(cap2, cap));
            assert_eq!(cv, rv, "row32 #{i} {cap2:?} {cap:?}");
        }
    }
    assert!(
        ones > 0 && zeros > 0,
        "rows31-32: both truthiness outcomes required, got {ones}/{zeros}"
    );

    // NaN / inf inputs: the C's `if (c2GJK(...))` is TRUE for NaN, so these must
    // return 0 from both, not 1.
    for &a in EDGE_F32 {
        for &b in EDGE_F32 {
            let bb = c2AABB {
                min: c2v { x: a, y: b },
                max: c2v { x: b, y: a },
            };
            let cap = c2Capsule {
                a: c2v { x: b, y: a },
                b: c2v { x: a, y: b },
                r: a,
            };
            let cap2 = c2Capsule {
                a: c2v { x: a, y: a },
                b: c2v { x: b, y: b },
                r: b,
            };
            unsafe {
                assert_eq!(c_ac(bb, cap), r_ac(bb, cap), "row31 edge {bb:?} {cap:?}");
                assert_eq!(
                    c_cc(cap2, cap),
                    r_cc(cap2, cap),
                    "row32 edge {cap2:?} {cap:?}"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 33, 34, 35, 36, 37 — degenerate / inverted / negative-radius shapes
// ===========================================================================

#[test]
fn rows33_37_degenerate_shape_handling() {
    type CIRCF = unsafe extern "C" fn(c2Circle, c2Circle) -> i32;
    type CABF = unsafe extern "C" fn(c2Circle, c2AABB) -> i32;
    type CCAP = unsafe extern "C" fn(c2Circle, c2Capsule) -> i32;
    type AAF = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
    let (c_cc, r_cc) = pair::<CIRCF>("c2CircletoCircle");
    let (c_ca, r_ca) = pair::<CABF>("c2CircletoAABB");
    let (c_cp, r_cp) = pair::<CCAP>("c2CircletoCapsule");
    let (c_aa, r_aa) = pair::<AAF>("c2AABBtoAABB");
    let mut rng = Rng::new(0xC033);

    for i in 0..8000 {
        // Row 33: negative radii, singly and doubly.
        let a = c2Circle {
            p: rng.vec(20.0),
            r: -rng.radius(20.0),
        };
        let b = c2Circle {
            p: rng.vec(20.0),
            r: if i % 2 == 0 {
                -rng.radius(20.0)
            } else {
                rng.radius(20.0)
            },
        };
        unsafe {
            assert_eq!(c_cc(a, b), r_cc(a, b), "row33 #{i} {a:?} {b:?}");
        }

        // Row 34: inverted AABB (min > max on one or both axes).
        let p = rng.vec(20.0);
        let inv = c2AABB {
            min: c2v {
                x: p.x + 5.0,
                y: p.y + 5.0,
            },
            max: p,
        };
        let half_inv = c2AABB {
            min: c2v {
                x: p.x + 5.0,
                y: p.y,
            },
            max: c2v {
                x: p.x,
                y: p.y + 5.0,
            },
        };
        let circ = rng.circle(20.0);
        unsafe {
            assert_eq!(c_ca(circ, inv), r_ca(circ, inv), "row34 #{i} {circ:?} {inv:?}");
            assert_eq!(
                c_ca(circ, half_inv),
                r_ca(circ, half_inv),
                "row34 half #{i}"
            );
            // Row 37: inverted AABB vs AABB.
            assert_eq!(c_aa(inv, half_inv), r_aa(inv, half_inv), "row37 #{i}");
            assert_eq!(c_aa(half_inv, inv), r_aa(half_inv, inv), "row37 rev #{i}");
        }

        // Rows 35, 36: degenerate capsule a == b -> the `da/c2Dot(n,n)` division
        // is unreachable because da == 0 is not < 0 and db == 0 is not < 0.
        let q = rng.vec(20.0);
        let degen = c2Capsule {
            a: q,
            b: q,
            r: rng.radius(10.0),
        };
        let degen_neg = c2Capsule {
            a: q,
            b: q,
            r: -rng.radius(10.0),
        };
        unsafe {
            assert_eq!(
                c_cp(circ, degen),
                r_cp(circ, degen),
                "row35 #{i} {circ:?} {degen:?}"
            );
            assert_eq!(
                c_cp(circ, degen_neg),
                r_cp(circ, degen_neg),
                "row35-neg #{i}"
            );
            // circle centre exactly on the degenerate capsule's point
            let on_point = c2Circle { p: q, r: circ.r };
            assert_eq!(
                c_cp(on_point, degen),
                r_cp(on_point, degen),
                "row35-coincident #{i}"
            );
        }
    }

    // Row 37: NaN-bearing AABBs make all four `<` comparisons false, so the C
    // reports a collision (returns 1). Verify identically, exhaustively.
    for &a in EDGE_F32 {
        for &b in EDGE_F32 {
            for &c in EDGE_F32 {
                let x = c2AABB {
                    min: c2v { x: a, y: b },
                    max: c2v { x: c, y: a },
                };
                let y = c2AABB {
                    min: c2v { x: b, y: c },
                    max: c2v { x: a, y: b },
                };
                unsafe {
                    assert_eq!(c_aa(x, y), r_aa(x, y), "row37 edge {x:?} {y:?}");
                }
            }
        }
    }
}

// ===========================================================================
// Row 38 — NaN / inf propagation through every c2v-taking leaf function
// ===========================================================================

#[test]
fn row38_nan_inf_propagation() {
    type V1 = unsafe extern "C" fn(c2v) -> c2v;
    type V2 = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type V3 = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
    type F1 = unsafe extern "C" fn(c2v) -> f32;
    type F2 = unsafe extern "C" fn(c2v, c2v) -> f32;

    let v1: &[(&str, _)] = &[
        ("c2Neg", pair::<V1>("c2Neg")),
        ("c2Skew", pair::<V1>("c2Skew")),
        ("c2CCW90", pair::<V1>("c2CCW90")),
        ("c2Norm", pair::<V1>("c2Norm")),
    ];
    let v2: &[(&str, _)] = &[
        ("c2Sub", pair::<V2>("c2Sub")),
        ("c2Add", pair::<V2>("c2Add")),
        ("c2Maxv", pair::<V2>("c2Maxv")),
        ("c2Minv", pair::<V2>("c2Minv")),
    ];
    let f1: &[(&str, _)] = &[("c2Len", pair::<F1>("c2Len"))];
    let f2: &[(&str, _)] = &[
        ("c2Dot", pair::<F2>("c2Dot")),
        ("c2Det2", pair::<F2>("c2Det2")),
    ];
    let (c_clamp, r_clamp) = pair::<V3>("c2Clampv");

    let specials: &[f32] = &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f800001), // signalling NaN pattern
        f32::from_bits(0xff800001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
    ];

    for &ax in specials {
        for &ay in specials {
            let a = c2v { x: ax, y: ay };
            for (name, (c, r)) in v1 {
                unsafe {
                    assert_v_bits!(c(a), r(a), "row38 {name} {a:?}");
                }
            }
            for (name, (c, r)) in f1 {
                unsafe {
                    assert_f32_bits!(c(a), r(a), "row38 {name} {a:?}");
                }
            }
            for &bx in specials {
                for &by in specials {
                    let b = c2v { x: bx, y: by };
                    for (name, (c, r)) in v2 {
                        unsafe {
                            assert_v_bits!(c(a, b), r(a, b), "row38 {name} {a:?} {b:?}");
                        }
                    }
                    for (name, (c, r)) in f2 {
                        unsafe {
                            assert_f32_bits!(c(a, b), r(a, b), "row38 {name} {a:?} {b:?}");
                        }
                    }
                    unsafe {
                        assert_v_bits!(
                            c_clamp(a, b, a),
                            r_clamp(a, b, a),
                            "row38 c2Clampv {a:?} {b:?}"
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 39, 40 — reverse_collide has no input validation
// ===========================================================================

#[test]
fn rows39_40_reverse_collide_no_validation() {
    type F = unsafe extern "C" fn(f32, f32, f32) -> i32;
    let (c_f, r_f) = pair::<F>("reverse_collide");

    // Row 39: non-positive / non-finite / huge radii and coordinates.
    let vals: &[f32] = &[
        0.0,
        -0.0,
        -1.0,
        -1e30,
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        1e38,
        -1e38,
        20.0,
        -70.0,
        -40.0,
        -15.0,
        100.0,
    ];
    for &x in vals {
        for &y in vals {
            for &r in vals {
                unsafe {
                    let (cv, rv) = (c_f(x, y, r), r_f(x, y, r));
                    assert_eq!(cv, rv, "rows39-40 reverse_collide({x}, {y}, {r})");
                    assert!(
                        (0..=7).contains(&cv),
                        "reverse_collide returned {cv}, outside the 3-bit mask range"
                    );
                }
            }
        }
    }

    // Row 40: -0.0 must behave exactly like +0.0 in every position.
    for &r in &[0.0f32, 1.0, 20.0, 100.0] {
        for (x, y) in [(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
            unsafe {
                assert_eq!(
                    c_f(x, y, r),
                    r_f(x, y, r),
                    "row40 reverse_collide({x}, {y}, {r})"
                );
                assert_eq!(
                    c_f(x, y, r),
                    c_f(x.abs(), y.abs(), r),
                    "row40: C treated -0.0 differently from +0.0"
                );
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary sweep: out-of-range enum + zero/oversized counts
// applied to every entry point that takes a type tag or a count.
// ===========================================================================

#[test]
fn generic_ffi_boundary_sweep() {
    let (c_collide, r_collide) = pair::<CollidedFn>("c2Collided");
    let (c_proxy, r_proxy) = pair::<MakeProxyFn>("c2MakeProxy");
    let (c_sup, r_sup) = pair::<SupportFn>("c2Support");
    let (c_met, r_met) = pair::<SimplexF32Fn>("c2GJKSimplexMetric");
    let (c_d, r_d) = pair::<SimplexVecFn>("c2D");
    let (c_l, r_l) = pair::<SimplexVecFn>("c2L");
    let (c_w, r_w) = pair::<WitnessFn>("c2Witness");

    let shape = c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    };
    let sp = &shape as *const c2Circle as *const c_void;

    // Every int, valid and invalid, one step past each boundary of the enum.
    let all_type_values: Vec<i32> = ALL_TYPES
        .iter()
        .copied()
        .chain(BAD_TYPES.iter().copied())
        .collect();

    for &ta in &all_type_values {
        for &tb in &all_type_values {
            unsafe {
                assert_eq!(
                    c_collide(sp, ta, sp, tb),
                    r_collide(sp, ta, sp, tb),
                    "ffi sweep c2Collided ({ta},{tb})"
                );
            }
        }
        // c2MakeProxy: only the three valid tags write; everything else is a
        // no-op that must leave the caller's buffer bit-identical.
        let seed = c2Proxy {
            radius: 9.5,
            count: 1234,
            verts: std::array::from_fn(|k| c2v {
                x: k as f32,
                y: -(k as f32),
            }),
        };
        unsafe {
            let mut cp = seed;
            let mut rp = seed;
            c_proxy(sp, ta, &mut cp);
            r_proxy(sp, ta, &mut rp);
            assert!(proxy_bits_eq(&cp, &rp), "ffi sweep c2MakeProxy type={ta}");
            if !ALL_TYPES.contains(&ta) {
                assert!(
                    proxy_bits_eq(&cp, &seed),
                    "ffi sweep: invalid type {ta} must not write to the proxy"
                );
            }
        }
    }

    // Counts: 0, every valid count, one past the maximum, and extremes.
    let verts = [c2v { x: 1.0, y: -1.0 }; 8];
    for &n in &[0i32, 1, 2, 3, 4, 5, 8, 9, -1, i32::MAX, i32::MIN] {
        // c2Support only reads verts[0..n]; clamp to the buffer so the C stays
        // in bounds (n > 8 would be a C-side overrun, excluded in ERRORS.md).
        if (0..=8).contains(&n) {
            unsafe {
                let d = c2v { x: 0.5, y: -0.25 };
                assert_eq!(
                    c_sup(verts.as_ptr(), n, d),
                    r_sup(verts.as_ptr(), n, d),
                    "ffi sweep c2Support count={n}"
                );
            }
        }
        // The simplex helpers only switch on `count`; any int is a real input.
        let mut rng = Rng::new(0xC0FF);
        for _ in 0..50 {
            let mut s = rng.simplex(25.0, n);
            s.count = n;
            unsafe {
                let (mut cs, mut rs) = (s, s);
                assert_f32_bits!(c_met(&mut cs), r_met(&mut rs), "ffi metric count={n}");
                let (mut cs, mut rs) = (s, s);
                assert_v_bits!(c_d(&mut cs), r_d(&mut rs), "ffi c2D count={n}");
                let (mut cs, mut rs) = (s, s);
                assert_v_bits!(c_l(&mut cs), r_l(&mut rs), "ffi c2L count={n}");
                let (mut cs, mut rs) = (s, s);
                let (mut ca, mut cb) = (c2v::default(), c2v::default());
                let (mut ra, mut rb) = (c2v::default(), c2v::default());
                c_w(&mut cs, &mut ca, &mut cb);
                r_w(&mut rs, &mut ra, &mut rb);
                assert_v_bits!(ca, ra, "ffi witness A count={n}");
                assert_v_bits!(cb, rb, "ffi witness B count={n}");
            }
        }
    }
}
