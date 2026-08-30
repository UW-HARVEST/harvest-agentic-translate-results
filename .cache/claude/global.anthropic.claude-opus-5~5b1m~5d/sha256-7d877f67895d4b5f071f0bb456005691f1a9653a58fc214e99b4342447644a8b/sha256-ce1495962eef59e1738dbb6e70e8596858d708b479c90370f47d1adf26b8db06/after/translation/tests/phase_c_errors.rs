#![allow(non_snake_case)]
//! Phase C — error-path / rejection differential tests.
//! One test per row of ERRORS.md.

mod common;
use common::*;

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

const NASTY: [f32; 12] = [
    f32::NAN,
    -f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    0.0,
    -0.0,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    FLT_EPSILON,
    -1.0,
];

fn nan_variants() -> Vec<f32> {
    vec![
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_0001), // another quiet NaN payload
        f32::from_bits(0x7f80_0001), // signalling NaN
    ]
}

// ============================================================ rows 1..4
// Out-of-range C enum values crossing the FFI boundary.

fn collided_bad(tag: &str, ta_list: &[C2_TYPE], tb_list: &[C2_TYPE]) {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut g = Rng::new(0x3001);
    for &ta in ta_list {
        for &tb in tb_list {
            for _ in 0..200 {
                // blobs are 32 B so even a bogus type cannot read out of bounds
                let a = Blob::of_capsule(g.capsule());
                let b = Blob::of_capsule(g.capsule());
                let cv = unsafe { cf(a.ptr(), ta, b.ptr(), tb) };
                let rv = unsafe { rf(a.ptr(), ta, b.ptr(), tb) };
                same(&format!("{tag} ta={ta} tb={tb}"), cv, rv);
                assert_eq!(cv, 0, "{tag}: C must reject with 0 (ta={ta} tb={tb})");
            }
        }
    }
}

#[test]
fn err_collided_bad_typeA() {
    // row 1: outer default: -> 0, for every typeB (valid or not)
    let mut tb: Vec<C2_TYPE> = ALL_TYPES.to_vec();
    tb.extend_from_slice(&BAD_TYPES);
    collided_bad("row1 bad typeA", &BAD_TYPES, &tb);
}

#[test]
fn err_collided_bad_typeB_circle() {
    collided_bad("row2 bad typeB/circle", &[C2_TYPE_CIRCLE], &BAD_TYPES);
}

#[test]
fn err_collided_bad_typeB_aabb() {
    collided_bad("row3 bad typeB/aabb", &[C2_TYPE_AABB], &BAD_TYPES);
}

#[test]
fn err_collided_bad_typeB_capsule() {
    collided_bad("row4 bad typeB/capsule", &[C2_TYPE_CAPSULE], &BAD_TYPES);
}

// ================================================================= row 5
/// `c2MakeProxy` has NO `default:` arm — the output struct must be left
/// COMPLETELY untouched for an out-of-range type.
#[test]
fn err_makeproxy_bad_type() {
    let (cf, rf) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut g = Rng::new(0x3002);
    let sentinel = c2Proxy {
        radius: f32::from_bits(0xABCD_1234),
        count: -424_242,
        verts: [v(f32::from_bits(0x1111_1111), f32::from_bits(0x2222_2222)); 8],
    };
    for &ty in &BAD_TYPES {
        for _ in 0..200 {
            let blob = Blob::of_capsule(g.capsule());
            let mut cp = sentinel;
            let mut rp = sentinel;
            unsafe {
                cf(blob.ptr(), ty, &mut cp);
                rf(blob.ptr(), ty, &mut rp);
            }
            same(&format!("c2MakeProxy bad type {ty}"), cp, rp);
            same("c2MakeProxy left output untouched", sentinel, cp);
        }
    }
}

// ================================================================= row 6
#[test]
fn err_simplex_metric_bad_count() {
    let (cf, rf) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut g = Rng::new(0x3003);
    for count in [0i32, 1, 4, 5, -1, -99, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = g.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&mut cs) };
            let rv = unsafe { rf(&mut rs) };
            same(&format!("c2GJKSimplexMetric count={count}"), (cv, cs), (rv, rs));
            assert_eq!(cv.to_bits(), 0.0f32.to_bits(), "count={count} must give +0.0");
        }
    }
}

// ================================================================= row 7
#[test]
fn err_c2d_bad_count() {
    let (cf, rf) = pair::<FnSimplexV>("c2D");
    let mut g = Rng::new(0x3004);
    for count in [0i32, 3, 4, 5, -1, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = g.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&mut cs) };
            let rv = unsafe { rf(&mut rs) };
            same(&format!("c2D count={count}"), (cv, cs), (rv, rs));
            same("c2D fallback is (0,0)", v(0.0, 0.0), cv);
        }
    }
}

// ================================================================= row 8
#[test]
fn err_witness_bad_count() {
    let (cf, rf) = pair::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x3005);
    for count in [0i32, 4, 5, -1, -7, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = g.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let mut ca = v(1.0, 2.0);
            let mut cb = v(3.0, 4.0);
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                cf(&mut cs, &mut ca, &mut cb);
                rf(&mut rs, &mut ra, &mut rb);
            }
            same(
                &format!("c2Witness count={count}"),
                (ca, cb, cs),
                (ra, rb, rs),
            );
            same("c2Witness fallback", (v(0.0, 0.0), v(0.0, 0.0)), (ca, cb));
        }
    }
}

// ================================================================= row 9
#[test]
fn err_c2l_bad_count() {
    let (cf, rf) = pair::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x3006);
    for count in [0i32, 3, 4, -1, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = g.simplex(count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { cf(&mut cs) };
            let rv = unsafe { rf(&mut rs) };
            same(&format!("c2L count={count}"), (cv, cs), (rv, rs));
            same("c2L fallback is (0,0)", v(0.0, 0.0), cv);
        }
    }
}

// ============================================================ rows 10, 11
#[test]
fn err_witness_zero_div() {
    let (cf, rf) = pair::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x3007);
    let divs = [
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
    ];
    for count in [1i32, 2, 3, 0, 4] {
        for d in divs {
            for _ in 0..80 {
                let mut s = g.simplex(count);
                s.div = d;
                let mut cs = s;
                let mut rs = s;
                let mut ca = v(9.0, 9.0);
                let mut cb = ca;
                let mut ra = ca;
                let mut rb = ca;
                unsafe {
                    cf(&mut cs, &mut ca, &mut cb);
                    rf(&mut rs, &mut ra, &mut rb);
                }
                same(
                    &format!("c2Witness div={d:e} count={count}"),
                    (ca, cb, cs),
                    (ra, rb, rs),
                );
            }
        }
    }
}

#[test]
fn err_c2l_zero_div() {
    let (cf, rf) = pair::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x3008);
    let divs = [
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for count in [1i32, 2, 3] {
        for d in divs {
            for _ in 0..100 {
                let mut s = g.simplex(count);
                s.div = d;
                let mut cs = s;
                let mut rs = s;
                let cv = unsafe { cf(&mut cs) };
                let rv = unsafe { rf(&mut rs) };
                same(&format!("c2L div={d:e} count={count}"), (cv, cs), (rv, rs));
            }
        }
    }
}

// ============================================================ rows 12, 13
#[test]
fn err_div_by_zero() {
    let (cf, rf) = pair::<FnV_vf>("c2Div");
    let mut g = Rng::new(0x3009);
    for b in NASTY {
        for _ in 0..200 {
            for a in [g.vec(), g.nasty_vec(), v(0.0, 0.0), v(-0.0, 0.0)] {
                same(&format!("c2Div b={b:e}"), cf(a, b), rf(a, b));
            }
        }
    }
    for b in nan_variants() {
        let a = v(1.0, -1.0);
        same("c2Div NaN divisor", cf(a, b), rf(a, b));
    }
}

// ================================================================ row 14
#[test]
fn err_norm_zero() {
    let (cf, rf) = pair::<FnV_v>("c2Norm");
    let zeros = [v(0.0, 0.0), v(-0.0, 0.0), v(0.0, -0.0), v(-0.0, -0.0)];
    for a in zeros {
        let cv = cf(a);
        same("c2Norm(0,0)", cv, rf(a));
        assert!(cv.x.is_nan() && cv.y.is_nan(), "expected NaN, got {cv:?}");
    }
    // subnormal magnitudes: x*x underflows to 0 -> also NaN
    for a in [
        v(f32::from_bits(1), 0.0),
        v(f32::from_bits(1), f32::from_bits(1)),
        v(f32::MIN_POSITIVE, 0.0),
        v(f32::MAX, f32::MAX),
        v(f32::INFINITY, 0.0),
        v(f32::NAN, 1.0),
    ] {
        same(&format!("c2Norm {a:?}"), cf(a), rf(a));
    }
}

// ================================================================ row 15
#[test]
fn err_len_nonfinite() {
    let (cf, rf) = pair::<FnF_v>("c2Len");
    for x in NASTY {
        for y in NASTY {
            let a = v(x, y);
            same(&format!("c2Len({x:e},{y:e})"), cf(a), rf(a));
        }
    }
    for x in nan_variants() {
        let a = v(x, 1.0);
        same("c2Len NaN", cf(a), rf(a));
    }
    // overflow of x*x -> +inf -> sqrtf(inf) = inf
    let a = v(1e30, 1e30);
    let cv = cf(a);
    same("c2Len overflow", cv, rf(a));
    assert!(cv.is_infinite());
}

// ============================================================ rows 16, 17
#[test]
fn err_support_nonpositive_count() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut g = Rng::new(0x300a);
    for count in [0i32, -1, -5, i32::MIN] {
        for _ in 0..400 {
            let mut verts = [c2v::default(); 8];
            for x in verts.iter_mut() {
                *x = g.nasty_vec();
            }
            let d = g.vec();
            let cv = unsafe { cf(verts.as_ptr(), count, d) };
            let rv = unsafe { rf(verts.as_ptr(), count, d) };
            same(&format!("c2Support count={count}"), cv, rv);
            assert_eq!(cv, 0, "count={count} must return 0");
        }
    }
}

#[test]
fn err_support_degenerate_d() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut g = Rng::new(0x300b);
    let dirs = [
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(f32::NAN, f32::NAN),
        v(f32::NAN, 0.0),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(f32::MAX, f32::MAX),
        v(f32::from_bits(1), f32::from_bits(1)),
    ];
    for count in [1i32, 2, 3, 4, 8] {
        for d in dirs {
            for _ in 0..100 {
                let mut verts = [c2v::default(); 8];
                for x in verts.iter_mut() {
                    *x = if d.x.is_nan() { g.nasty_vec() } else { g.vec() };
                }
                let cv = unsafe { cf(verts.as_ptr(), count, d) };
                let rv = unsafe { rf(verts.as_ptr(), count, d) };
                same(&format!("c2Support d={d:?} count={count}"), cv, rv);
            }
        }
    }
    // all-NaN verts: `dot > dmax` never true -> index 0
    let verts = [v(f32::NAN, f32::NAN); 8];
    let cv = unsafe { cf(verts.as_ptr(), 8, v(1.0, 1.0)) };
    same("c2Support all-NaN verts", cv, unsafe {
        rf(verts.as_ptr(), 8, v(1.0, 1.0))
    });
    assert_eq!(cv, 0);
}

// ============================================================ rows 18, 19
#[test]
fn err_gjk_null_transforms() {
    let (cf, rf) = gjk_pair();
    let ident = c2x {
        p: v(0.0, 0.0),
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut g = Rng::new(0x300c);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..200 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                // NULL ax only / NULL bx only / both NULL
                for (ax, bx) in [
                    (None, Some(&ident)),
                    (Some(&ident), None),
                    (None, None),
                    (Some(&ident), Some(&ident)),
                ] {
                    let co = call_gjk(cf, &a, ta, ax, &b, tb, bx, 1, None);
                    let ro = call_gjk(rf, &a, ta, ax, &b, tb, bx, 1, None);
                    same("c2GJK NULL transform -> identity", co, ro);
                    // C substitutes c2xIdentity(), so all four must agree
                    let base = call_gjk(cf, &a, ta, None, &b, tb, None, 1, None);
                    same("all four NULL/identity combos equal", base, co);
                }
            }
        }
    }
}

// ================================================== rows 20, 21, 22, 23
#[test]
fn err_gjk_null_outputs() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x300d);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..300 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                // everything NULL: only the return value is observable
                let cd = unsafe {
                    cf(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        1,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                let rd = unsafe {
                    rf(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        1,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                same("c2GJK all-NULL outputs", cd, rd);
                // and with use_radius = 0
                let cd0 = unsafe {
                    cf(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                let rd0 = unsafe {
                    rf(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                same("c2GJK all-NULL outputs ur=0", cd0, rd0);
            }
        }
    }
}

// ================================================================ row 24
#[test]
fn err_gjk_cache_count_zero() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x300e);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..300 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                // count == 0 but every OTHER field is garbage: must be ignored
                let cache = c2GJKCache {
                    metric: g.nasty(),
                    count: 0,
                    iA: [999, -7, 31],
                    iB: [-1, 4242, 6],
                    div: g.nasty(),
                };
                let co = call_gjk(cf, &a, ta, None, &b, tb, None, 1, Some(cache));
                let ro = call_gjk(rf, &a, ta, None, &b, tb, None, 1, Some(cache));
                same("c2GJK cache.count == 0 ignores garbage", co, ro);
                // must equal the NULL-cache result apart from the written cache
                let base = call_gjk(cf, &a, ta, None, &b, tb, None, 1, None);
                same(
                    "cold cache == NULL cache (dist/a/b/iters)",
                    (base.dist, base.a, base.b, base.iters),
                    (co.dist, co.a, co.b, co.iters),
                );
            }
        }
    }
}

// ================================================================ row 25
#[test]
fn err_gjk_cache_reject_metric() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x300f);
    // `metric < -1.0e8f` is essentially never true, so cache_was_read is
    // (almost) always 1.  Drive both sides of the guard with extreme metrics.
    let metrics = [
        0.0f32,
        1.0,
        -1.0,
        -1.0e8,
        -1.0e9,
        -f32::MAX,
        f32::MAX,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for m in metrics {
                for _ in 0..40 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    // build a legal warm cache first, then override the metric
                    let warm =
                        call_gjk(cf, &a, ta, None, &b, tb, None, 1, Some(c2GJKCache::default()));
                    let mut cache = warm.cache.unwrap();
                    if cache.count == 0 {
                        cache.count = 1;
                    }
                    cache.metric = m;
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, 1, Some(cache));
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, 1, Some(cache));
                    same(&format!("c2GJK cache metric={m:e}"), co, ro);
                }
            }
        }
    }
}

// ================================================================ row 26
#[test]
fn err_gjk_cache_count_negative() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3010);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for count in [-1i32, -3, -1000, i32::MIN] {
                for ur in [0i32, 1] {
                    for _ in 0..40 {
                        let a = rand_shape(&mut g, ta);
                        let b = rand_shape(&mut g, tb);
                        let cache = c2GJKCache {
                            metric: g.nasty(),
                            count,
                            iA: [0, 1, 2],
                            iB: [0, 1, 2],
                            div: g.nasty(),
                        };
                        let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, Some(cache));
                        let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, Some(cache));
                        same(
                            &format!("c2GJK cache.count={count} ta={ta} tb={tb} ur={ur}"),
                            co,
                            ro,
                        );
                    }
                }
            }
        }
    }
}

// ================================================================ row 28
/// Cache indices inside `[0, proxy.count)` — every byte read is one that
/// `c2MakeProxy` actually wrote, so this must match bit-for-bit.
#[test]
fn err_gjk_cache_index_out_of_range() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3011);
    let proxy_count = |t: C2_TYPE| match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    };
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let (na, nb) = (proxy_count(ta), proxy_count(tb));
            for count in [1i32, 2, 3] {
                for _ in 0..150 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let mut iA = [0i32; 3];
                    let mut iB = [0i32; 3];
                    for k in 0..3 {
                        // deliberately NOT the index GJK would have chosen,
                        // but still within the initialised proxy slots
                        iA[k] = g.below(na) as i32;
                        iB[k] = g.below(nb) as i32;
                    }
                    let cache = c2GJKCache {
                        metric: g.range(-50.0, 50.0),
                        count,
                        iA,
                        iB,
                        div: g.range(0.1, 5.0),
                    };
                    for ur in [0i32, 1] {
                        let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, Some(cache));
                        let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, Some(cache));
                        same(
                            &format!(
                                "c2GJK crafted cache count={count} iA={iA:?} iB={iB:?} \
                                 ta={ta} tb={tb} ur={ur}"
                            ),
                            co,
                            ro,
                        );
                    }
                }
            }
        }
    }
}

// ================================================================ row 30
#[test]
fn err_gjk_iteration_cap() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3012);
    let mut max_it = 0i32;
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..1000 {
                // pathological: near-identical, near-degenerate shapes
                let s = 1e-6f32;
                let a = match ta {
                    C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: v(0.0, 0.0), r: s }),
                    C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                        min: v(0.0, 0.0),
                        max: v(s, s),
                    }),
                    _ => Blob::of_capsule(c2Capsule {
                        a: v(0.0, 0.0),
                        b: v(s, 0.0),
                        r: s,
                    }),
                };
                let o = v(g.range(-s * 4.0, s * 4.0), g.range(-s * 4.0, s * 4.0));
                let b = match tb {
                    C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: o, r: s }),
                    C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                        min: o,
                        max: v(o.x + s, o.y + s),
                    }),
                    _ => Blob::of_capsule(c2Capsule {
                        a: o,
                        b: v(o.x + s, o.y),
                        r: s,
                    }),
                };
                for ur in [0i32, 1] {
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                    same("c2GJK iteration cap", co, ro);
                    assert!(co.iters <= 20, "cap violated: {}", co.iters);
                    max_it = max_it.max(co.iters);
                }
            }
        }
    }
    eprintln!("max observed GJK iterations = {max_it}");
}

// ============================================================ rows 32, 33
#[test]
fn err_gjk_degenerate_shapes() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3013);
    // point-like shapes: the search direction collapses (dot(d,d) < eps^2)
    // and support points repeat (dup -> break).
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..400 {
                    let p = g.vec();
                    let mk = |t: C2_TYPE, q: c2v| match t {
                        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: q, r: 0.0 }),
                        C2_TYPE_AABB => Blob::of_aabb(c2AABB { min: q, max: q }),
                        _ => Blob::of_capsule(c2Capsule { a: q, b: q, r: 0.0 }),
                    };
                    let a = mk(ta, p);
                    // exactly coincident, and one-ULP apart
                    for q in [
                        p,
                        v(f32::from_bits(p.x.to_bits() ^ 1), p.y),
                        v(p.x + FLT_EPSILON, p.y),
                    ] {
                        let b = mk(tb, q);
                        let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                        let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                        same("c2GJK collapsed direction / dup support", co, ro);
                    }
                }
            }
        }
    }
}

// ============================================================ rows 34, 35
#[test]
fn err_gjk_radius_shrink_to_zero() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3014);
    let mut zeroed = 0usize;
    let mut shrunk = 0usize;
    for _ in 0..4000 {
        let ra = g.range(0.0, 20.0);
        let rb = g.range(0.0, 20.0);
        // sweep the centre distance right across `rA + rB`
        for k in -3i32..=3 {
            let d = ra + rb + (k as f32) * FLT_EPSILON * (ra + rb + 1.0);
            let a = Blob::of_circle(c2Circle { p: v(0.0, 0.0), r: ra });
            let b = Blob::of_circle(c2Circle { p: v(d, 0.0), r: rb });
            let co = call_gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, 1, None);
            let ro = call_gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, 1, None);
            same("c2GJK radius shrink boundary", co, ro);
            if co.dist == 0.0 {
                zeroed += 1;
            } else {
                shrunk += 1;
            }
        }
        // distance below FLT_EPSILON with use_radius: forced midpoint branch
        let a = Blob::of_circle(c2Circle { p: v(0.0, 0.0), r: 0.0 });
        let b = Blob::of_circle(c2Circle {
            p: v(FLT_EPSILON * 0.5, 0.0),
            r: 0.0,
        });
        let co = call_gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, 1, None);
        let ro = call_gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, 1, None);
        same("c2GJK midpoint branch", co, ro);
        assert_eq!(co.dist.to_bits(), 0.0f32.to_bits());
    }
    assert!(zeroed > 0 && shrunk > 0, "{zeroed} / {shrunk}");
}

// ================================================================ row 36
#[test]
fn err_gjk_negative_radius() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3015);
    for &ta in &[C2_TYPE_CIRCLE, C2_TYPE_CAPSULE] {
        for &tb in &[C2_TYPE_CIRCLE, C2_TYPE_CAPSULE] {
            for ur in [0i32, 1] {
                for _ in 0..500 {
                    let neg = |t: C2_TYPE, g: &mut Rng| match t {
                        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle {
                            p: g.vec(),
                            r: -g.range(0.0, 30.0),
                        }),
                        _ => Blob::of_capsule(c2Capsule {
                            a: g.vec(),
                            b: g.vec(),
                            r: -g.range(0.0, 30.0),
                        }),
                    };
                    let a = neg(ta, &mut g);
                    let b = neg(tb, &mut g);
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                    same(&format!("c2GJK negative radius ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
    // and through the boolean predicates
    let (ccc, rcc) = pair::<FnI_Cir_Cir>("c2CircletoCircle");
    let (ccap, rcap) = pair::<FnI_Cir_Cap>("c2CircletoCapsule");
    let (cac, rac) = pair::<FnI_AABB_Cap>("c2AABBtoCapsule");
    let (ccc2, rcc2) = pair::<FnI_Cap_Cap>("c2CapsuletoCapsule");
    for _ in 0..3000 {
        let a = c2Circle {
            p: g.vec(),
            r: -g.range(0.0, 30.0),
        };
        let b = c2Circle {
            p: g.vec(),
            r: -g.range(0.0, 30.0),
        };
        same("c2CircletoCircle neg r", ccc(a, b), rcc(a, b));
        let cap = c2Capsule {
            a: g.vec(),
            b: g.vec(),
            r: -g.range(0.0, 20.0),
        };
        same("c2CircletoCapsule neg r", ccap(a, cap), rcap(a, cap));
        let bb = g.aabb();
        same("c2AABBtoCapsule neg r", cac(bb, cap), rac(bb, cap));
        let cap2 = c2Capsule {
            a: g.vec(),
            b: g.vec(),
            r: -g.range(0.0, 20.0),
        };
        same("c2CapsuletoCapsule neg r", ccc2(cap, cap2), rcc2(cap, cap2));
    }
}

// ================================================================ row 37
#[test]
fn err_gjk_hit_returns_zero() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3016);
    let mut hits = 0usize;
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for _ in 0..400 {
                    let ctr = g.vec();
                    let big = g.range(20.0, 60.0);
                    let a = match ta {
                        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: ctr, r: big }),
                        C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                            min: v(ctr.x - big, ctr.y - big),
                            max: v(ctr.x + big, ctr.y + big),
                        }),
                        _ => Blob::of_capsule(c2Capsule {
                            a: v(ctr.x - big, ctr.y),
                            b: v(ctr.x + big, ctr.y),
                            r: big,
                        }),
                    };
                    let b = match tb {
                        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle { p: ctr, r: big * 0.5 }),
                        C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                            min: v(ctr.x - big * 0.5, ctr.y - big * 0.5),
                            max: v(ctr.x + big * 0.5, ctr.y + big * 0.5),
                        }),
                        _ => Blob::of_capsule(c2Capsule {
                            a: v(ctr.x - big * 0.5, ctr.y),
                            b: v(ctr.x + big * 0.5, ctr.y),
                            r: big * 0.5,
                        }),
                    };
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, None);
                    same("c2GJK hit path", co, ro);
                    if co.dist == 0.0 {
                        hits += 1;
                        assert_eq!(co.dist.to_bits(), 0.0f32.to_bits(), "must be +0.0");
                        same("hit sets a = b", co.a, co.b);
                    }
                }
            }
        }
    }
    assert!(hits > 0, "never reached the hit path");
}

// ================================================================ row 38
#[test]
fn err_gjk_nan_inputs() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x3017);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for k in 0..300 {
                    let bad = |t: C2_TYPE, g: &mut Rng, which: usize| -> Blob {
                        let n = |i: usize, g: &mut Rng| {
                            if i == which {
                                NASTY[g.below(NASTY.len() as u32) as usize]
                            } else {
                                g.coord()
                            }
                        };
                        match t {
                            C2_TYPE_CIRCLE => Blob::of_circle(c2Circle {
                                p: v(n(0, g), n(1, g)),
                                r: n(2, g).abs(),
                            }),
                            C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                                min: v(n(0, g), n(1, g)),
                                max: v(n(2, g), n(3, g)),
                            }),
                            _ => Blob::of_capsule(c2Capsule {
                                a: v(n(0, g), n(1, g)),
                                b: v(n(2, g), n(3, g)),
                                r: n(4, g).abs(),
                            }),
                        }
                    };
                    let a = bad(ta, &mut g, k % 5);
                    let b = bad(tb, &mut g, (k / 5) % 5);
                    let co = call_gjk(cf, &a, ta, None, &b, tb, None, ur, Some(c2GJKCache::default()));
                    let ro = call_gjk(rf, &a, ta, None, &b, tb, None, ur, Some(c2GJKCache::default()));
                    same(&format!("c2GJK nasty floats ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

// ================================================================ row 39
#[test]
fn err_capsule_nan() {
    let (cac, rac) = pair::<FnI_AABB_Cap>("c2AABBtoCapsule");
    let (ccc, rcc) = pair::<FnI_Cap_Cap>("c2CapsuletoCapsule");
    let mut g = Rng::new(0x3018);
    for x in NASTY {
        for _ in 0..300 {
            let bb = c2AABB {
                min: v(x, g.coord()),
                max: v(g.coord(), x),
            };
            let cap = c2Capsule {
                a: v(g.coord(), x),
                b: v(x, g.coord()),
                r: x.abs(),
            };
            same(&format!("c2AABBtoCapsule nasty {x:e}"), cac(bb, cap), rac(bb, cap));
            let cap2 = c2Capsule {
                a: v(x, x),
                b: g.vec(),
                r: g.range(0.0, 10.0),
            };
            same(
                &format!("c2CapsuletoCapsule nasty {x:e}"),
                ccc(cap, cap2),
                rcc(cap, cap2),
            );
        }
    }
    // pure NaN: GJK returns NaN, `if (NaN)` is TRUE in C -> 0
    let nan_bb = c2AABB {
        min: v(f32::NAN, f32::NAN),
        max: v(f32::NAN, f32::NAN),
    };
    let nan_cap = c2Capsule {
        a: v(f32::NAN, f32::NAN),
        b: v(f32::NAN, f32::NAN),
        r: f32::NAN,
    };
    same("c2AABBtoCapsule all-NaN", cac(nan_bb, nan_cap), rac(nan_bb, nan_cap));
    same(
        "c2CapsuletoCapsule all-NaN",
        ccc(nan_cap, nan_cap),
        rcc(nan_cap, nan_cap),
    );
}

// ============================================================ rows 40, 41
#[test]
fn err_aabb_inverted() {
    let (cf, rf) = pair::<FnI_AABB_AABB>("c2AABBtoAABB");
    let (cc, rc) = pair::<FnI_Cir_AABB>("c2CircletoAABB");
    let (cp, rp) = pair::<FnI_AABB_Cap>("c2AABBtoCapsule");
    let mut g = Rng::new(0x3019);
    for _ in 0..8000 {
        // min > max on one or both axes
        let a = c2AABB {
            min: v(g.coord(), g.coord()),
            max: v(g.coord(), g.coord()),
        };
        let b = c2AABB {
            min: v(g.coord(), g.coord()),
            max: v(g.coord(), g.coord()),
        };
        same("c2AABBtoAABB inverted", cf(a, b), rf(a, b));
        same("c2AABBtoAABB inverted swapped", cf(b, a), rf(b, a));
        let circ = g.circle();
        same("c2CircletoAABB inverted", cc(circ, a), rc(circ, a));
        let cap = g.capsule();
        same("c2AABBtoCapsule inverted", cp(a, cap), rp(a, cap));
    }
    // fully inverted: max strictly below min on both axes
    let a = c2AABB {
        min: v(10.0, 10.0),
        max: v(-10.0, -10.0),
    };
    same("c2AABBtoAABB self-inverted", cf(a, a), rf(a, a));
}

#[test]
fn err_aabb_nan() {
    let (cf, rf) = pair::<FnI_AABB_AABB>("c2AABBtoAABB");
    let mut g = Rng::new(0x301a);
    for nan in nan_variants() {
        for slot in 0..4 {
            for _ in 0..200 {
                let mut vals = [g.coord(), g.coord(), g.coord(), g.coord()];
                vals[slot] = nan;
                let a = c2AABB {
                    min: v(vals[0], vals[1]),
                    max: v(vals[2], vals[3]),
                };
                let b = g.aabb();
                same("c2AABBtoAABB NaN", cf(a, b), rf(a, b));
                same("c2AABBtoAABB NaN swapped", cf(b, a), rf(b, a));
            }
        }
    }
    // all-NaN: every `<` is false so the C code returns 1
    let n = c2AABB {
        min: v(f32::NAN, f32::NAN),
        max: v(f32::NAN, f32::NAN),
    };
    let cv = cf(n, n);
    same("c2AABBtoAABB all-NaN", cv, rf(n, n));
    assert_eq!(cv, 1, "C returns 1 for all-NaN AABBs");
}

// ================================================================ row 42
#[test]
fn err_circle_negative_radius() {
    let (cf, rf) = pair::<FnI_Cir_Cir>("c2CircletoCircle");
    let (ca, ra) = pair::<FnI_Cir_AABB>("c2CircletoAABB");
    let mut g = Rng::new(0x301b);
    for _ in 0..20_000 {
        let a = c2Circle {
            p: g.vec(),
            r: g.range(-40.0, 40.0),
        };
        let b = c2Circle {
            p: g.vec(),
            r: g.range(-40.0, 40.0),
        };
        same("c2CircletoCircle signed r", cf(a, b), rf(a, b));
        let bb = g.aabb();
        same("c2CircletoAABB signed r", ca(a, bb), ra(a, bb));
    }
    // exact sign-symmetry check demanded by `r2 = r2*r2`
    let a = c2Circle { p: v(0.0, 0.0), r: -5.0 };
    let b = c2Circle { p: v(7.0, 0.0), r: -3.0 };
    let cv = cf(a, b);
    same("c2CircletoCircle both neg", cv, rf(a, b));
    assert_eq!(cv, 1, "(-5)+(-3) squared = 64 > 49 -> reports a hit");
}

// ================================================ rows 43, 45, 48, 52
#[test]
fn err_circle_nan() {
    let (cf, rf) = pair::<FnI_Cir_Cir>("c2CircletoCircle");
    let (ca, ra) = pair::<FnI_Cir_AABB>("c2CircletoAABB");
    let (cp, rp) = pair::<FnI_Cir_Cap>("c2CircletoCapsule");
    let mut g = Rng::new(0x301c);
    for x in NASTY {
        for slot in 0..3 {
            for _ in 0..200 {
                let mut f = [g.coord(), g.coord(), g.range(0.0, 20.0)];
                f[slot] = x;
                let a = c2Circle {
                    p: v(f[0], f[1]),
                    r: f[2],
                };
                let b = g.circle();
                same("c2CircletoCircle nasty", cf(a, b), rf(a, b));
                same("c2CircletoCircle nasty swapped", cf(b, a), rf(b, a));
                let bb = g.aabb();
                same("c2CircletoAABB nasty circle", ca(a, bb), ra(a, bb));
                let mut bv = [g.coord(), g.coord(), g.coord(), g.coord()];
                bv[slot] = x;
                let bbn = c2AABB {
                    min: v(bv[0], bv[1]),
                    max: v(bv[2], bv[3]),
                };
                same("c2CircletoAABB nasty aabb", ca(b, bbn), ra(b, bbn));
                let cap = g.capsule();
                same("c2CircletoCapsule nasty circle", cp(a, cap), rp(a, cap));
                let capn = c2Capsule {
                    a: v(bv[0], bv[1]),
                    b: v(bv[2], bv[3]),
                    r: x.abs(),
                };
                same("c2CircletoCapsule nasty capsule", cp(b, capn), rp(b, capn));
            }
        }
    }
    let nan_c = c2Circle {
        p: v(f32::NAN, f32::NAN),
        r: f32::NAN,
    };
    let cv = cf(nan_c, nan_c);
    same("c2CircletoCircle all-NaN", cv, rf(nan_c, nan_c));
    assert_eq!(cv, 0, "NaN < NaN is false -> 0");
}

/// row 52: the min/max ternaries are NaN-order dependent.
#[test]
fn err_minmax_nan() {
    let (cx, rx) = pair::<FnV_vv>("c2Maxv");
    let (ci, ri) = pair::<FnV_vv>("c2Minv");
    let (cc, rc) = pair::<FnV_vvv>("c2Clampv");
    let mut g = Rng::new(0x301d);
    for n in nan_variants() {
        for _ in 0..500 {
            let a = v(n, g.coord());
            let b = v(g.coord(), n);
            same("c2Maxv NaN a", cx(a, b), rx(a, b));
            same("c2Maxv NaN b", cx(b, a), rx(b, a));
            same("c2Minv NaN a", ci(a, b), ri(a, b));
            same("c2Minv NaN b", ci(b, a), ri(b, a));
            let p = g.vec();
            same("c2Clampv NaN box", cc(p, a, b), rc(p, a, b));
            same("c2Clampv NaN box rev", cc(p, b, a), rc(p, b, a));
            same("c2Clampv NaN value", cc(a, p, b), rc(a, p, b));
        }
    }
    // documented asymmetry: max(NaN, x) == x but max(x, NaN) == NaN
    let nan = v(f32::NAN, f32::NAN);
    let one = v(1.0, 1.0);
    let m1 = cx(nan, one);
    same("c2Maxv(NaN, 1)", m1, rx(nan, one));
    assert_eq!(m1.x.to_bits(), 1.0f32.to_bits());
    let m2 = cx(one, nan);
    same("c2Maxv(1, NaN)", m2, rx(one, nan));
    assert!(m2.x.is_nan());
}

// ================================================================ row 44
#[test]
fn err_circle_aabb_inverted() {
    let (cf, rf) = pair::<FnI_Cir_AABB>("c2CircletoAABB");
    let mut g = Rng::new(0x301e);
    for _ in 0..20_000 {
        let bb = c2AABB {
            min: v(g.coord(), g.coord()),
            max: v(g.coord(), g.coord()),
        };
        let a = g.circle();
        same("c2CircletoAABB inverted box", cf(a, bb), rf(a, bb));
    }
    // c2Clampv(a, lo, hi) = max(lo, min(a, hi)) -> collapses to lo when lo > hi
    let bb = c2AABB {
        min: v(10.0, 10.0),
        max: v(-10.0, -10.0),
    };
    for p in [v(0.0, 0.0), v(10.0, 10.0), v(-10.0, -10.0), v(100.0, 100.0)] {
        let a = c2Circle { p, r: 5.0 };
        same("c2CircletoAABB fully inverted", cf(a, bb), rf(a, bb));
    }
}

// ============================================================ rows 46, 47
#[test]
fn err_circle_capsule_degenerate() {
    let (cf, rf) = pair::<FnI_Cir_Cap>("c2CircletoCapsule");
    let mut g = Rng::new(0x301f);
    for _ in 0..20_000 {
        // zero-length capsule: n == (0,0), da == 0 (NOT < 0), db == 0 -> `bp`
        let p = g.vec();
        let cap = c2Capsule {
            a: p,
            b: p,
            r: g.range(0.0, 25.0),
        };
        let circ = g.circle();
        same("c2CircletoCapsule zero-length", cf(circ, cap), rf(circ, cap));
        // circle centre exactly on a / b
        for q in [cap.a, cap.b] {
            let c2 = c2Circle {
                p: q,
                r: g.range(0.0, 10.0),
            };
            same("c2CircletoCapsule centre on endpoint", cf(c2, cap), rf(c2, cap));
        }
    }
    // dot(n,n) == 0 with a non-zero `da` is only reachable via inf/NaN
    for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN_POSITIVE] {
        let cap = c2Capsule {
            a: v(bad, bad),
            b: v(bad, bad),
            r: 1.0,
        };
        let circ = c2Circle { p: v(0.0, 0.0), r: 1.0 };
        same(&format!("c2CircletoCapsule inf capsule {bad:e}"), cf(circ, cap), rf(circ, cap));
        let cap2 = c2Capsule {
            a: v(0.0, 0.0),
            b: v(bad, 0.0),
            r: 1.0,
        };
        same("c2CircletoCapsule inf endpoint", cf(circ, cap2), rf(circ, cap2));
    }
}

// ================================================================ row 49
#[test]
fn err_bbverts_inverted() {
    let (cf, rf) = pair::<FnBBVerts>("c2BBVerts");
    let mut g = Rng::new(0x3020);
    for _ in 0..8000 {
        let mut bb = c2AABB {
            min: v(g.nasty(), g.nasty()),
            max: v(g.nasty(), g.nasty()),
        };
        let mut co = [v(f32::from_bits(0xFEED), 0.0); 4];
        let mut ro = co;
        unsafe {
            cf(co.as_mut_ptr(), &mut bb);
            rf(ro.as_mut_ptr(), &mut bb);
        }
        same("c2BBVerts inverted/nasty", co, ro);
        // the input must not be modified by either side
        assert_eq!(bb.min.x.to_bits(), bb.min.x.to_bits());
    }
    // fully inverted, exact
    let mut bb = c2AABB {
        min: v(5.0, 6.0),
        max: v(-5.0, -6.0),
    };
    let mut co = [c2v::default(); 4];
    let mut ro = co;
    unsafe {
        cf(co.as_mut_ptr(), &mut bb);
        rf(ro.as_mut_ptr(), &mut bb);
    }
    same("c2BBVerts fully inverted", co, ro);
    same("c2BBVerts writes min/max verbatim", [bb.min, bb.max], [co[0], co[2]]);
}

// ============================================================ rows 50, 51
#[test]
fn err_reverse_collide_negative_r() {
    let (cf, rf) = pair::<FnReverseCollide>("reverse_collide");
    let mut g = Rng::new(0x3021);
    for _ in 0..100_000 {
        let x = g.range(-170.0, 170.0);
        let y = g.range(-60.0, 170.0);
        let r = -g.range(0.0, 70.0);
        same(&format!("reverse_collide neg r ({x},{y},{r})"), cf(x, y, r), rf(x, y, r));
    }
    for r in [-0.0f32, -1.0, -10.0, -20.0, -1e30, -f32::MAX] {
        for x in [-70.0f32, -27.5, -30.0, 0.0] {
            for y in [0.0f32, -27.5, 70.0] {
                same(
                    &format!("reverse_collide neg r exact ({x},{y},{r})"),
                    cf(x, y, r),
                    rf(x, y, r),
                );
            }
        }
    }
}

#[test]
fn err_reverse_collide_nonfinite() {
    let (cf, rf) = pair::<FnReverseCollide>("reverse_collide");
    let mut vals: Vec<f32> = NASTY.to_vec();
    vals.extend(nan_variants());
    vals.extend_from_slice(&[
        f32::from_bits(1),
        f32::from_bits(0x0080_0000),
        1e-45,
        1e38,
        -1e38,
        -70.0,
        -40.0,
        -15.0,
        20.0,
        10.0,
    ]);
    for &x in &vals {
        for &y in &vals {
            for &r in &vals {
                let cv = cf(x, y, r);
                let rv = rf(x, y, r);
                same(&format!("reverse_collide({x:e},{y:e},{r:e})"), cv, rv);
                assert!(
                    (0..8).contains(&cv),
                    "bitmask out of range: {cv} for ({x:e},{y:e},{r:e})"
                );
            }
        }
    }
}

// ==================================================== generic FFI boundary
/// Out-of-range enums combined with every other axis, plus zero/oversized
/// lengths for the pointer+count API.
#[test]
fn err_generic_boundaries() {
    // c2Support: count far larger than the buffer would be UB, so probe the
    // exact buffer size and one below it (the strict `dot > dmax` boundary).
    let (cs, rs) = pair::<FnSupport>("c2Support");
    let mut g = Rng::new(0x3022);
    for count in [0i32, 1, 7, 8] {
        for _ in 0..500 {
            let mut verts = [c2v::default(); 8];
            for x in verts.iter_mut() {
                *x = g.vec();
            }
            let d = g.vec();
            same(
                &format!("c2Support boundary count={count}"),
                unsafe { cs(verts.as_ptr(), count, d) },
                unsafe { rs(verts.as_ptr(), count, d) },
            );
        }
    }

    // c2MakeProxy / c2Collided: every enum value one step past the valid range
    let (cm, rm) = pair::<FnMakeProxy>("c2MakeProxy");
    let (cc, rc) = pair::<FnCollided>("c2Collided");
    for ty in [0u32, 1, 2, 3, u32::MAX, u32::MAX - 1] {
        for _ in 0..200 {
            let blob = Blob::of_capsule(g.capsule());
            let mut cp = c2Proxy::default();
            let mut rp = c2Proxy::default();
            unsafe {
                cm(blob.ptr(), ty, &mut cp);
                rm(blob.ptr(), ty, &mut rp);
            }
            same(&format!("c2MakeProxy ty={ty}"), cp, rp);
            let b2 = Blob::of_capsule(g.capsule());
            same(
                &format!("c2Collided ty={ty}"),
                unsafe { cc(blob.ptr(), ty, b2.ptr(), ty) },
                unsafe { rc(blob.ptr(), ty, b2.ptr(), ty) },
            );
        }
    }

    // c2GJK: use_radius values other than 0/1 (C tests `if (use_radius)`)
    let (cg, rg) = gjk_pair();
    for ur in [0i32, 1, 2, -1, i32::MIN, i32::MAX] {
        for &ta in &ALL_TYPES {
            for &tb in &ALL_TYPES {
                for _ in 0..60 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let co = call_gjk(cg, &a, ta, None, &b, tb, None, ur, None);
                    let ro = call_gjk(rg, &a, ta, None, &b, tb, None, ur, None);
                    same(&format!("c2GJK use_radius={ur}"), co, ro);
                }
            }
        }
    }
}
