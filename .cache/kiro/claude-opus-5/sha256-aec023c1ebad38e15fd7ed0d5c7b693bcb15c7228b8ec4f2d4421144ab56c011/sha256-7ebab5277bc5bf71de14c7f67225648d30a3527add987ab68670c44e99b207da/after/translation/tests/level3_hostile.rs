//! Level 3: hostile and degenerate inputs.
//!
//! NaN, infinities, signed zero, denormals, degenerate capsules (`a == b`),
//! empty/inverted AABBs and negative radii. The C code guards none of these, so
//! the translation must reproduce whatever falls out.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_void;

/// Floats chosen to stress every comparison in the library.
const HOSTILE: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-45, // smallest positive denormal
    -1e-45,
    1.1920929e-7, // FLT_EPSILON, as spelled in the C source
    -1.1920929e-7,
    1e18,
    -1e18,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    // Distinct NaN payloads: propagation of these is observable via the FFI.
    f32::from_bits(0xffc0_0000),
    f32::from_bits(0x7fc0_dead),
];

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

// ---------------------------------------------------------------------------
// Predicates under hostile floats
// ---------------------------------------------------------------------------

#[test]
fn predicates_match_on_hostile_floats() {
    let l = libs();
    let (c_aa, r_aa) = l.pair::<unsafe extern "C" fn(c2AABB, c2AABB) -> i32>("c2AABBtoAABB");
    let (c_cc, r_cc) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Circle) -> i32>("c2CircletoCircle");
    let (c_ca, r_ca) = l.pair::<unsafe extern "C" fn(c2Circle, c2AABB) -> i32>("c2CircletoAABB");
    let (c_ck, r_ck) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Capsule) -> i32>("c2CircletoCapsule");

    for &a in HOSTILE {
        for &b in HOSTILE {
            for &r in HOSTILE {
                let bx1 = c2AABB { min: c2v { x: a, y: b }, max: c2v { x: b, y: a } };
                let bx2 = c2AABB { min: c2v { x: -a, y: r }, max: c2v { x: r, y: -b } };
                unsafe {
                    assert_eq!(
                        c_aa(bx1, bx2),
                        r_aa(bx1, bx2),
                        "c2AABBtoAABB {bx1:?} {bx2:?}"
                    );
                }

                let c1 = c2Circle { p: c2v { x: a, y: b }, r };
                let c2 = c2Circle { p: c2v { x: b, y: r }, r: a };
                unsafe {
                    assert_eq!(c_cc(c1, c2), r_cc(c1, c2), "c2CircletoCircle {c1:?} {c2:?}");
                    assert_eq!(c_ca(c1, bx2), r_ca(c1, bx2), "c2CircletoAABB {c1:?} {bx2:?}");
                }

                let k = c2Capsule { a: c2v { x: a, y: b }, b: c2v { x: r, y: a }, r: b };
                unsafe {
                    assert_eq!(c_ck(c1, k), r_ck(c1, k), "c2CircletoCapsule {c1:?} {k:?}");
                }
            }
        }
    }
}

/// `c2CircletoCapsule` has three distinct branches selected by `da`/`db`; drive
/// each of them, including the degenerate `a == b` capsule where `c2Dot(n, n)`
/// is zero and the division produces inf/NaN.
#[test]
fn c2CircletoCapsule_branches_match() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2Circle, c2Capsule) -> i32>("c2CircletoCapsule");

    let cases: Vec<(c2Circle, c2Capsule)> = {
        let mut v = Vec::new();
        // Degenerate capsule: a == b, so n == (0,0) and da == 0 => `db < 0` is
        // false => the `bp` branch. Also covers the 0/0 division being skipped.
        for p in [(0.0f32, 0.0f32), (1.0, 1.0), (-3.5, 7.25), (1e18, -1e18)] {
            for q in [(0.0f32, 0.0f32), (5.0, 0.0), (-2.0, -2.0)] {
                for rad in [0.0f32, 1.0, 10.0, -1.0] {
                    v.push((
                        c2Circle { p: c2v { x: p.0, y: p.1 }, r: rad },
                        c2Capsule { a: c2v { x: q.0, y: q.1 }, b: c2v { x: q.0, y: q.1 }, r: rad },
                    ));
                }
            }
        }
        // Non-degenerate capsule along +x, circle before / inside / after.
        for x in [-100.0f32, -1.0, 0.0, 0.5, 5.0, 10.0, 10.5, 100.0] {
            for y in [0.0f32, 1.0, -1.0, 50.0] {
                for rad in [0.0f32, 1.0, 5.0] {
                    v.push((
                        c2Circle { p: c2v { x, y }, r: rad },
                        c2Capsule {
                            a: c2v { x: 0.0, y: 0.0 },
                            b: c2v { x: 10.0, y: 0.0 },
                            r: rad,
                        },
                    ));
                }
            }
        }
        v
    };

    for (i, (circle, cap)) in cases.iter().enumerate() {
        let (cv, rv) = unsafe { (c(*circle, *cap), r(*circle, *cap)) };
        assert_eq!(cv, rv, "c2CircletoCapsule branch #{i} {circle:?} {cap:?}");
    }
}

// ---------------------------------------------------------------------------
// GJK under degenerate geometry
// ---------------------------------------------------------------------------

/// Shapes with zero extent: point AABBs, zero-radius circles, capsules with
/// `a == b`, plus coincident and exactly-touching placements.
#[test]
fn c2GJK_matches_on_degenerate_shapes() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");

    let pts = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: 1e-30, y: 1e-30 },
        c2v { x: 1e18, y: -1e18 },
    ];

    let mut shapes: Vec<(Vec<u8>, i32, String)> = Vec::new();
    for p in pts {
        for rad in [0.0f32, -0.0, 1.0, -1.0, 1e-30] {
            let circ = c2Circle { p, r: rad };
            shapes.push((raw(&circ), C2_TYPE_CIRCLE, format!("{circ:?}")));
            let cap = c2Capsule { a: p, b: p, r: rad };
            shapes.push((raw(&cap), C2_TYPE_CAPSULE, format!("{cap:?}")));
            let cap2 = c2Capsule { a: p, b: c2v { x: p.x + 2.0, y: p.y }, r: rad };
            shapes.push((raw(&cap2), C2_TYPE_CAPSULE, format!("{cap2:?}")));
        }
        // Point box, and an inverted box.
        let bb = c2AABB { min: p, max: p };
        shapes.push((raw(&bb), C2_TYPE_AABB, format!("{bb:?}")));
        let bb2 = c2AABB { min: c2v { x: p.x + 1.0, y: p.y + 1.0 }, max: p };
        shapes.push((raw(&bb2), C2_TYPE_AABB, format!("{bb2:?}")));
        let bb3 = c2AABB { min: p, max: c2v { x: p.x + 2.0, y: p.y + 3.0 } };
        shapes.push((raw(&bb3), C2_TYPE_AABB, format!("{bb3:?}")));
    }

    let sentinel = c2v { x: -9999.0, y: 8888.0 };
    let mut n = 0usize;
    for (ba, ta, da) in &shapes {
        for (bb, tb, db) in &shapes {
            for use_radius in [0, 1] {
                let mut oa = (sentinel, sentinel);
                let mut ob = (sentinel, sentinel);
                let mut ia = -1i32;
                let mut ib = -1i32;
                let (cd, rd) = unsafe {
                    (
                        c(
                            ba.as_ptr() as *const c_void,
                            *ta,
                            std::ptr::null(),
                            bb.as_ptr() as *const c_void,
                            *tb,
                            std::ptr::null(),
                            &mut oa.0,
                            &mut oa.1,
                            use_radius,
                            &mut ia,
                            std::ptr::null_mut(),
                        ),
                        r(
                            ba.as_ptr() as *const c_void,
                            *ta,
                            std::ptr::null(),
                            bb.as_ptr() as *const c_void,
                            *tb,
                            std::ptr::null(),
                            &mut ob.0,
                            &mut ob.1,
                            use_radius,
                            &mut ib,
                            std::ptr::null_mut(),
                        ),
                    )
                };
                let ctx = format!("c2GJK degenerate A={da} B={db} r={use_radius}");
                assert_f32_eq(cd, rd, &format!("{ctx} dist"));
                assert_bytes_eq(&oa.0, &ob.0, &format!("{ctx} outA"));
                assert_bytes_eq(&oa.1, &ob.1, &format!("{ctx} outB"));
                assert_eq!(ia, ib, "{ctx} iterations");
                n += 1;
            }
        }
    }
    assert!(n > 10000, "degenerate sweep too small: {n}");
}

/// Non-finite shape coordinates. `c2GJK`'s loop guards are all ordinary float
/// comparisons, so NaN steers control flow; the translation must follow.
#[test]
fn c2GJK_matches_on_non_finite_shapes() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let vals = [
        0.0f32,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        1e-45,
    ];

    let sentinel = c2v { x: 424242.0, y: -171717.0 };
    let mut n = 0usize;
    let mut hit_cap = 0usize;
    for &x in &vals {
        for &y in &vals {
            for &rad in &vals {
                let circ = c2Circle { p: c2v { x, y }, r: rad };
                let cap = c2Capsule { a: c2v { x: y, y: x }, b: c2v { x, y }, r: rad };
                let bb = c2AABB { min: c2v { x: -x, y: -y }, max: c2v { x, y } };
                let shapes: [(*const c_void, i32, String); 3] = [
                    (&circ as *const _ as *const c_void, C2_TYPE_CIRCLE, format!("{circ:?}")),
                    (&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, format!("{cap:?}")),
                    (&bb as *const _ as *const c_void, C2_TYPE_AABB, format!("{bb:?}")),
                ];
                for (pa, ta, da) in &shapes {
                    for (pb, tb, db) in &shapes {
                        for use_radius in [0, 1] {
                            let mut oa = (sentinel, sentinel);
                            let mut ob = (sentinel, sentinel);
                            let mut ia = -1i32;
                            let mut ib = -1i32;
                            let (cd, rd) = unsafe {
                                (
                                    c(
                                        *pa,
                                        *ta,
                                        std::ptr::null(),
                                        *pb,
                                        *tb,
                                        std::ptr::null(),
                                        &mut oa.0,
                                        &mut oa.1,
                                        use_radius,
                                        &mut ia,
                                        std::ptr::null_mut(),
                                    ),
                                    r(
                                        *pa,
                                        *ta,
                                        std::ptr::null(),
                                        *pb,
                                        *tb,
                                        std::ptr::null(),
                                        &mut ob.0,
                                        &mut ob.1,
                                        use_radius,
                                        &mut ib,
                                        std::ptr::null_mut(),
                                    ),
                                )
                            };
                            let ctx =
                                format!("c2GJK non-finite A={da} B={db} r={use_radius}");
                            assert_eq!(ia, ib, "{ctx} iterations");
                            if ia >= 20 {
                                // C would read an indeterminate `u` here; skip.
                                hit_cap += 1;
                                continue;
                            }
                            assert_f32_eq(cd, rd, &format!("{ctx} dist"));
                            assert_bytes_eq(&oa.0, &ob.0, &format!("{ctx} outA"));
                            assert_bytes_eq(&oa.1, &ob.1, &format!("{ctx} outB"));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("non-finite GJK cases compared: {n}, skipped at iteration cap: {hit_cap}");
    assert!(n > 5000, "non-finite sweep too small: {n}");
    assert_eq!(hit_cap, 0, "unexpectedly hit the indeterminate iteration-cap path");
}

/// Transforms that are not rotations (unnormalised, zero, non-finite).
#[test]
fn c2GJK_matches_on_hostile_transforms() {
    let l = libs();
    let (c, r) = l.pair::<GjkFn>("c2GJK");
    let rots = [
        c2r { c: 1.0, s: 0.0 },
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 0.0, s: 1.0 },
        c2r { c: -1.0, s: 0.0 },
        c2r { c: 3.0, s: 4.0 },
        c2r { c: 1e18, s: -1e18 },
        c2r { c: 1e-30, s: 1e-30 },
    ];
    let ps = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 10.0, y: -10.0 },
        c2v { x: 1e18, y: 1e18 },
        c2v { x: -0.0, y: -0.0 },
    ];

    let mut rng = Rng::new(0x33_0001);
    let mut n = 0usize;
    for &rc in &rots {
        for &p in &ps {
            let ax = c2x { p, r: rc };
            for _ in 0..scale(40) {
                let circ = rng.circle();
                let cap = rng.capsule();
                let bb = rng.aabb();
                let shapes: [(*const c_void, i32); 3] = [
                    (&circ as *const _ as *const c_void, C2_TYPE_CIRCLE),
                    (&cap as *const _ as *const c_void, C2_TYPE_CAPSULE),
                    (&bb as *const _ as *const c_void, C2_TYPE_AABB),
                ];
                for (pa, ta) in &shapes {
                    for (pb, tb) in &shapes {
                        let bx = c2x { p: rng.finite_vec(), r: rots[rng.below(7) as usize] };
                        let use_radius = rng.below(2) as i32;
                        let mut oa = (c2v::default(), c2v::default());
                        let mut ob = (c2v::default(), c2v::default());
                        let mut ia = -1i32;
                        let mut ib = -1i32;
                        let (cd, rd) = unsafe {
                            (
                                c(
                                    *pa, *ta, &ax, *pb, *tb, &bx, &mut oa.0, &mut oa.1,
                                    use_radius, &mut ia, std::ptr::null_mut(),
                                ),
                                r(
                                    *pa, *ta, &ax, *pb, *tb, &bx, &mut ob.0, &mut ob.1,
                                    use_radius, &mut ib, std::ptr::null_mut(),
                                ),
                            )
                        };
                        let ctx = format!("c2GJK xform ax={ax:?} bx={bx:?} ta={ta} tb={tb}");
                        assert_eq!(ia, ib, "{ctx} iterations");
                        assert!(ia < 20, "{ctx} reached the iteration cap");
                        assert_f32_eq(cd, rd, &format!("{ctx} dist"));
                        assert_bytes_eq(&oa.0, &ob.0, &format!("{ctx} outA"));
                        assert_bytes_eq(&oa.1, &ob.1, &format!("{ctx} outB"));
                        n += 1;
                    }
                }
            }
        }
    }
    assert!(n > 5000, "transform sweep too small: {n}");
}

// ---------------------------------------------------------------------------
// Simplex helpers under hostile floats
// ---------------------------------------------------------------------------

#[test]
fn simplex_helpers_match_on_hostile_floats() {
    let l = libs();
    let (c22c, c22r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c22");
    let (c23c, c23r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c23");
    let (cDc, cDr) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>("c2D");
    let (cLc, cLr) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>("c2L");
    let (cMc, cMr) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> f32>("c2GJKSimplexMetric");
    let (cWc, cWr) =
        l.pair::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)>("c2Witness");

    let mut n = 0usize;
    for &x in HOSTILE {
        for &y in HOSTILE {
            for &z in HOSTILE {
                let mut s = c2Simplex::default();
                for (k, v) in s.verts.iter_mut().enumerate() {
                    let f = [x, y, z][k % 3];
                    let g = [y, z, x][k % 3];
                    v.p = c2v { x: f, y: g };
                    v.sA = c2v { x: g, y: f };
                    v.sB = c2v { x: f, y: f };
                    v.u = g;
                    v.iA = k as i32;
                    v.iB = (3 - k) as i32;
                }
                s.div = z;

                for count in [0i32, 1, 2, 3, 4] {
                    s.count = count;

                    let (mut a, mut b) = (s, s);
                    unsafe {
                        c22c(&mut a);
                        c22r(&mut b);
                    }
                    assert_bytes_eq(&a, &b, &format!("c22 hostile {s:?}"));

                    let (mut a, mut b) = (s, s);
                    unsafe {
                        c23c(&mut a);
                        c23r(&mut b);
                    }
                    assert_bytes_eq(&a, &b, &format!("c23 hostile {s:?}"));

                    let (mut a, mut b) = (s, s);
                    unsafe {
                        assert_bytes_eq(&cDc(&mut a), &cDr(&mut b), &format!("c2D hostile {s:?}"));
                    }
                    let (mut a, mut b) = (s, s);
                    unsafe {
                        assert_bytes_eq(&cLc(&mut a), &cLr(&mut b), &format!("c2L hostile {s:?}"));
                    }
                    let (mut a, mut b) = (s, s);
                    unsafe {
                        assert_f32_eq(cMc(&mut a), cMr(&mut b), &format!("c2Metric hostile {s:?}"));
                    }

                    let (mut a, mut b) = (s, s);
                    let fill = c2v { x: 1234.5, y: -4321.5 };
                    let (mut wa, mut wb) = (fill, fill);
                    let (mut wc, mut wd) = (fill, fill);
                    unsafe {
                        cWc(&mut a, &mut wa, &mut wb);
                        cWr(&mut b, &mut wc, &mut wd);
                    }
                    assert_bytes_eq(&wa, &wc, &format!("c2Witness hostile a {s:?}"));
                    assert_bytes_eq(&wb, &wd, &format!("c2Witness hostile b {s:?}"));
                    n += 1;
                }
            }
        }
    }
    assert!(n > 10000, "hostile simplex sweep too small: {n}");
}

#[test]
fn c2Support_matches_on_hostile_floats() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32>("c2Support");
    for &x in HOSTILE {
        for &y in HOSTILE {
            let verts = [
                c2v { x, y },
                c2v { x: y, y: x },
                c2v { x: -x, y: -y },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: 1.0, y: 1.0 },
                c2v { x, y: x },
                c2v { x: y, y },
                c2v { x: -0.0, y: -0.0 },
            ];
            for &dx in HOSTILE {
                let d = c2v { x: dx, y: -dx };
                for count in [0i32, 1, 2, 4, 8] {
                    let (cv, rv) = unsafe {
                        (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d))
                    };
                    assert_eq!(cv, rv, "c2Support hostile count={count} d={d:?} {verts:?}");
                }
            }
        }
    }
}
