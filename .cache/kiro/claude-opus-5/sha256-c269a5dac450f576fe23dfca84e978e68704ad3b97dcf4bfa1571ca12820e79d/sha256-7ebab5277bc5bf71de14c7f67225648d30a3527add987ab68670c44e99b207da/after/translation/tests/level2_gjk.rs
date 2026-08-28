//! Level 2: `c2GJK`, the boolean collision predicates, `c2Collided`, and the
//! public `aabb` entry point.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

type GjkFn = unsafe extern "C" fn(
    *const u8,   // A
    c_int,       // typeA
    *const c2x,  // ax
    *const u8,   // B
    c_int,       // typeB
    *const c2x,  // bx
    *mut c2v,    // outA
    *mut c2v,    // outB
    c_int,       // use_radius
    *mut c_int,  // iterations
    *mut c2GJKCache,
) -> f32;

/// One of the three shape kinds, kept alive so we can hand out a raw pointer.
#[derive(Debug, Clone, Copy)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const u8 {
        match self {
            Shape::Circle(c) => (c as *const c2Circle).cast(),
            Shape::Aabb(a) => (a as *const c2AABB).cast(),
            Shape::Capsule(c) => (c as *const c2Capsule).cast(),
        }
    }
    /// Number of proxy verts this shape produces, so cache indices can be
    /// kept inside the initialised region (the C code leaves `c2Proxy` on an
    /// uninitialised stack slot, so reading past `count` is not well defined).
    fn vert_count(&self) -> usize {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

fn rand_shape(rng: &mut Rng, kind: usize, tame: bool) -> Shape {
    let co = |rng: &mut Rng| if tame { rng.coord() } else { rng.spicy() };
    match kind % 3 {
        0 => Shape::Circle(c2Circle {
            p: c2v {
                x: co(rng),
                y: co(rng),
            },
            r: if tame {
                rng.unit().abs() * 40.0
            } else {
                rng.spicy()
            },
        }),
        1 => {
            let (x, y) = (co(rng), co(rng));
            if tame {
                Shape::Aabb(c2AABB {
                    min: c2v { x, y },
                    max: c2v {
                        x: x + rng.unit().abs() * 60.0,
                        y: y + rng.unit().abs() * 60.0,
                    },
                })
            } else {
                Shape::Aabb(c2AABB {
                    min: c2v { x, y },
                    max: c2v {
                        x: co(rng),
                        y: co(rng),
                    },
                })
            }
        }
        _ => Shape::Capsule(c2Capsule {
            a: c2v {
                x: co(rng),
                y: co(rng),
            },
            b: c2v {
                x: co(rng),
                y: co(rng),
            },
            r: if tame {
                rng.unit().abs() * 25.0
            } else {
                rng.spicy()
            },
        }),
    }
}

fn rand_x(rng: &mut Rng, tame: bool) -> c2x {
    if tame {
        let ang = rng.unit() * std::f32::consts::PI;
        c2x {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: c2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        }
    } else {
        c2x {
            p: rng.spicy_vec(),
            r: c2r {
                c: rng.spicy(),
                s: rng.spicy(),
            },
        }
    }
}

struct GjkOut {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: Option<c2GJKCache>,
}

#[allow(clippy::too_many_arguments)]
unsafe fn call_gjk(
    f: &GjkFn,
    A: &Shape,
    ax: Option<&c2x>,
    B: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    // Distinctive fill so an unwritten out-param is detectable.
    let fill = c2v {
        x: f32::from_bits(0x7357_0001),
        y: f32::from_bits(0x7357_0002),
    };
    let mut a = fill;
    let mut b = fill;
    let mut iters: c_int = -12345;
    let mut cache = cache_in;
    let cache_ptr = match cache.as_mut() {
        Some(c) => c as *mut c2GJKCache,
        None => std::ptr::null_mut(),
    };
    let dist = unsafe {
        f(
            A.ptr(),
            A.ty(),
            ax.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            B.ptr(),
            B.ty(),
            bx.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            &mut a,
            &mut b,
            use_radius,
            &mut iters,
            cache_ptr,
        )
    };
    GjkOut {
        dist,
        a,
        b,
        iters,
        cache,
    }
}

fn assert_gjk_eq(c: &GjkOut, r: &GjkOut, ctx: &str) {
    assert!(
        f32_eq_nan_ok(c.dist, r.dist),
        "dist mismatch: C={:?} (0x{:08x}) Rust={:?} (0x{:08x}) | {ctx}",
        c.dist,
        c.dist.to_bits(),
        r.dist,
        r.dist.to_bits()
    );
    assert!(
        c2v_eq_nan_ok(c.a, r.a),
        "outA mismatch: C={:?} Rust={:?} | {ctx}",
        c.a,
        r.a
    );
    assert!(
        c2v_eq_nan_ok(c.b, r.b),
        "outB mismatch: C={:?} Rust={:?} | {ctx}",
        c.b,
        r.b
    );
    assert_eq!(c.iters, r.iters, "iterations mismatch | {ctx}");
    match (&c.cache, &r.cache) {
        (Some(cc), Some(rc)) => {
            assert!(
                f32_eq_nan_ok(cc.metric, rc.metric),
                "cache.metric mismatch: C={:?} Rust={:?} | {ctx}",
                cc.metric,
                rc.metric
            );
            assert_eq!(cc.count, rc.count, "cache.count mismatch | {ctx}");
            assert_eq!(cc.iA, rc.iA, "cache.iA mismatch | {ctx}");
            assert_eq!(cc.iB, rc.iB, "cache.iB mismatch | {ctx}");
            assert!(
                f32_eq_nan_ok(cc.div, rc.div),
                "cache.div mismatch: C={:?} Rust={:?} | {ctx}",
                cc.div,
                rc.div
            );
        }
        (None, None) => {}
        _ => panic!("cache presence mismatch | {ctx}"),
    }
}

// ---------------------------------------------------------------------------
// c2GJK
// ---------------------------------------------------------------------------

/// All 9 type pairs, both `use_radius` values, null and non-null transforms.
#[test]
fn t_gjk_all_type_pairs() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let mut rng = Rng::new(40);

    for tame in [true, false] {
        for ka in 0..3usize {
            for kb in 0..3usize {
                for use_radius in [0i32, 1] {
                    for xform in 0..4u32 {
                        let iters = if tame { 400 } else { 200 };
                        for i in 0..iters {
                            let A = rand_shape(&mut rng, ka, tame);
                            let B = rand_shape(&mut rng, kb, tame);
                            let ax = rand_x(&mut rng, tame);
                            let bx = rand_x(&mut rng, tame);
                            let (axp, bxp) = match xform {
                                0 => (None, None),
                                1 => (Some(&ax), None),
                                2 => (None, Some(&bx)),
                                _ => (Some(&ax), Some(&bx)),
                            };
                            let co = unsafe {
                                call_gjk(&cf, &A, axp, &B, bxp, use_radius, None)
                            };
                            let ro = unsafe {
                                call_gjk(&rf, &A, axp, &B, bxp, use_radius, None)
                            };
                            assert_gjk_eq(
                                &co,
                                &ro,
                                &format!(
                                    "tame={tame} ka={ka} kb={kb} ur={use_radius} \
                                     xform={xform} i={i} A={A:?} B={B:?} ax={ax:?} bx={bx:?}"
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Overlapping / touching / tangent configurations, where the branch taken
/// inside GJK is most sensitive to rounding.
#[test]
fn t_gjk_near_contact() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let mut rng = Rng::new(41);

    for ka in 0..3usize {
        for kb in 0..3usize {
            for use_radius in [0i32, 1] {
                for i in 0..1500 {
                    let A = rand_shape(&mut rng, ka, true);
                    // Place B very close to A so the pair straddles contact.
                    let jitter = [0.0f32, 1e-7, -1e-7, 1e-3, -1e-3, 0.5, -0.5][i % 7];
                    let B = match rand_shape(&mut rng, kb, true) {
                        Shape::Circle(mut c) => {
                            c.p = c2v {
                                x: anchor(&A).x + jitter,
                                y: anchor(&A).y + jitter,
                            };
                            Shape::Circle(c)
                        }
                        Shape::Aabb(mut b) => {
                            let w = b.max.x - b.min.x;
                            let h = b.max.y - b.min.y;
                            b.min = c2v {
                                x: anchor(&A).x + jitter,
                                y: anchor(&A).y + jitter,
                            };
                            b.max = c2v {
                                x: b.min.x + w,
                                y: b.min.y + h,
                            };
                            Shape::Aabb(b)
                        }
                        Shape::Capsule(mut c) => {
                            let d = c2v {
                                x: c.b.x - c.a.x,
                                y: c.b.y - c.a.y,
                            };
                            c.a = c2v {
                                x: anchor(&A).x + jitter,
                                y: anchor(&A).y + jitter,
                            };
                            c.b = c2v {
                                x: c.a.x + d.x,
                                y: c.a.y + d.y,
                            };
                            Shape::Capsule(c)
                        }
                    };
                    let co = unsafe { call_gjk(&cf, &A, None, &B, None, use_radius, None) };
                    let ro = unsafe { call_gjk(&rf, &A, None, &B, None, use_radius, None) };
                    assert_gjk_eq(
                        &co,
                        &ro,
                        &format!(
                            "ka={ka} kb={kb} ur={use_radius} i={i} jitter={jitter} \
                             A={A:?} B={B:?}"
                        ),
                    );
                }
            }
        }
    }
}

fn anchor(s: &Shape) -> c2v {
    match s {
        Shape::Circle(c) => c.p,
        Shape::Aabb(a) => a.min,
        Shape::Capsule(c) => c.a,
    }
}

fn translate(s: &Shape, dx: f32, dy: f32) -> Shape {
    match *s {
        Shape::Circle(mut c) => {
            c.p.x += dx;
            c.p.y += dy;
            Shape::Circle(c)
        }
        Shape::Aabb(mut a) => {
            a.min.x += dx;
            a.min.y += dy;
            a.max.x += dx;
            a.max.y += dy;
            Shape::Aabb(a)
        }
        Shape::Capsule(mut c) => {
            c.a.x += dx;
            c.a.y += dy;
            c.b.x += dx;
            c.b.y += dy;
            Shape::Capsule(c)
        }
    }
}

/// Identical shapes (fully coincident) — degenerate simplices and zero
/// distances.
#[test]
fn t_gjk_coincident() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let mut rng = Rng::new(42);
    for k in 0..3usize {
        for use_radius in [0i32, 1] {
            for i in 0..1000 {
                let A = rand_shape(&mut rng, k, true);
                let B = A;
                let co = unsafe { call_gjk(&cf, &A, None, &B, None, use_radius, None) };
                let ro = unsafe { call_gjk(&rf, &A, None, &B, None, use_radius, None) };
                assert_gjk_eq(
                    &co,
                    &ro,
                    &format!("coincident k={k} ur={use_radius} i={i} A={A:?}"),
                );
            }
        }
    }
    // Degenerate shapes: zero-radius circles, zero-extent AABBs, point capsules.
    let degenerates: [Shape; 6] = [
        Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        }),
        Shape::Circle(c2Circle {
            p: c2v { x: 5.0, y: -5.0 },
            r: -3.0,
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: 4.0, y: 4.0 },
            max: c2v { x: -4.0, y: -4.0 },
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: 2.0, y: 2.0 },
            b: c2v { x: 2.0, y: 2.0 },
            r: 0.0,
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: -1.0,
        }),
    ];
    for (i, A) in degenerates.iter().enumerate() {
        for (j, B) in degenerates.iter().enumerate() {
            for use_radius in [0i32, 1] {
                let co = unsafe { call_gjk(&cf, A, None, B, None, use_radius, None) };
                let ro = unsafe { call_gjk(&rf, A, None, B, None, use_radius, None) };
                assert_gjk_eq(
                    &co,
                    &ro,
                    &format!("degenerate i={i} j={j} ur={use_radius} A={A:?} B={B:?}"),
                );
            }
        }
    }
}

/// Null out-params must be tolerated identically (and not written).
#[test]
fn t_gjk_null_outputs() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let mut rng = Rng::new(43);
    for i in 0..2000 {
        let A = rand_shape(&mut rng, i, true);
        let B = rand_shape(&mut rng, i / 3, true);
        let use_radius = (i % 2) as c_int;
        // Every combination of null / non-null out-pointers.
        for mask in 0..8u32 {
            let mut ca = c2v::default();
            let mut cb = c2v::default();
            let mut ci: c_int = 0;
            let mut ra = c2v::default();
            let mut rb = c2v::default();
            let mut ri: c_int = 0;
            let pa_c = if mask & 1 != 0 { &mut ca as *mut c2v } else { std::ptr::null_mut() };
            let pb_c = if mask & 2 != 0 { &mut cb as *mut c2v } else { std::ptr::null_mut() };
            let pi_c = if mask & 4 != 0 { &mut ci as *mut c_int } else { std::ptr::null_mut() };
            let pa_r = if mask & 1 != 0 { &mut ra as *mut c2v } else { std::ptr::null_mut() };
            let pb_r = if mask & 2 != 0 { &mut rb as *mut c2v } else { std::ptr::null_mut() };
            let pi_r = if mask & 4 != 0 { &mut ri as *mut c_int } else { std::ptr::null_mut() };
            let cd = unsafe {
                cf(
                    A.ptr(), A.ty(), std::ptr::null(),
                    B.ptr(), B.ty(), std::ptr::null(),
                    pa_c, pb_c, use_radius, pi_c, std::ptr::null_mut(),
                )
            };
            let rd = unsafe {
                rf(
                    A.ptr(), A.ty(), std::ptr::null(),
                    B.ptr(), B.ty(), std::ptr::null(),
                    pa_r, pb_r, use_radius, pi_r, std::ptr::null_mut(),
                )
            };
            assert!(
                f32_eq_nan_ok(cd, rd),
                "mask={mask} i={i}: dist C={cd:?} Rust={rd:?}"
            );
            assert!(c2v_eq_nan_ok(ca, ra), "mask={mask} i={i} outA");
            assert!(c2v_eq_nan_ok(cb, rb), "mask={mask} i={i} outB");
            assert_eq!(ci, ri, "mask={mask} i={i} iterations");
        }
    }
}

/// Cache write-back on a fresh (zeroed) cache, then cache reuse.
#[test]
fn t_gjk_cache() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let mut rng = Rng::new(44);

    for ka in 0..3usize {
        for kb in 0..3usize {
            for use_radius in [0i32, 1] {
                for i in 0..300 {
                    let A = rand_shape(&mut rng, ka, true);
                    let B = rand_shape(&mut rng, kb, true);

                    // Pass 1: zeroed cache -> cache_was_good == 0.
                    let mut c_cache = c2GJKCache::default();
                    let mut r_cache = c2GJKCache::default();
                    let co =
                        unsafe { call_gjk(&cf, &A, None, &B, None, use_radius, Some(c_cache)) };
                    let ro =
                        unsafe { call_gjk(&rf, &A, None, &B, None, use_radius, Some(r_cache)) };
                    assert_gjk_eq(
                        &co,
                        &ro,
                        &format!("cache pass1 ka={ka} kb={kb} ur={use_radius} i={i} A={A:?} B={B:?}"),
                    );

                    // Pass 2: feed the written-back cache straight back in.
                    c_cache = co.cache.unwrap();
                    r_cache = ro.cache.unwrap();
                    let co2 =
                        unsafe { call_gjk(&cf, &A, None, &B, None, use_radius, Some(c_cache)) };
                    let ro2 =
                        unsafe { call_gjk(&rf, &A, None, &B, None, use_radius, Some(r_cache)) };
                    assert_gjk_eq(
                        &co2,
                        &ro2,
                        &format!("cache pass2 ka={ka} kb={kb} ur={use_radius} i={i} A={A:?} B={B:?}"),
                    );

                    // Pass 3: same cache, slightly moved shapes (the usual
                    // temporal-coherence usage).
                    let A2 = translate(&A, 0.25, -0.125);
                    let co3 = unsafe {
                        call_gjk(&cf, &A2, None, &B, None, use_radius, co2.cache)
                    };
                    let ro3 = unsafe {
                        call_gjk(&rf, &A2, None, &B, None, use_radius, ro2.cache)
                    };
                    assert_gjk_eq(
                        &co3,
                        &ro3,
                        &format!("cache pass3 ka={ka} kb={kb} ur={use_radius} i={i}"),
                    );
                }
            }
        }
    }
}

/// Hand-built caches, including ones whose `metric`/`div` force the
/// `cache_was_read` decision either way. Indices stay inside each proxy's
/// initialised vert range.
#[test]
fn t_gjk_synthetic_caches() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let mut rng = Rng::new(45);

    let metrics = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        1e-9,
        -1e9,
        -1.0e8,
        -1.0000001e8,
        -2.0e8,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];

    for ka in 0..3usize {
        for kb in 0..3usize {
            for use_radius in [0i32, 1] {
                for count in 0..4i32 {
                    for &metric in metrics.iter() {
                        for i in 0..12 {
                            let A = rand_shape(&mut rng, ka, true);
                            let B = rand_shape(&mut rng, kb, true);
                            let na = A.vert_count();
                            let nb = B.vert_count();
                            let mut cache = c2GJKCache {
                                metric,
                                count,
                                iA: [0; 3],
                                iB: [0; 3],
                                div: [1.0f32, 0.0, -3.5, 1e-8][i % 4],
                            };
                            for k in 0..3 {
                                cache.iA[k] = (rng.next_u32() as usize % na) as c_int;
                                cache.iB[k] = (rng.next_u32() as usize % nb) as c_int;
                            }
                            let co = unsafe {
                                call_gjk(&cf, &A, None, &B, None, use_radius, Some(cache))
                            };
                            let ro = unsafe {
                                call_gjk(&rf, &A, None, &B, None, use_radius, Some(cache))
                            };
                            assert_gjk_eq(
                                &co,
                                &ro,
                                &format!(
                                    "synthcache ka={ka} kb={kb} ur={use_radius} count={count} \
                                     metric={metric:?} i={i} cache={cache:?} A={A:?} B={B:?}"
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Out-of-range shape types: `c2MakeProxy` leaves the proxy alone, so GJK
/// then reads whatever was on the stack. That is genuinely
/// implementation-defined, so only check that neither side crashes and that
/// the well-defined types still agree — covered above. Here we just confirm
/// the symbol tolerates the call without differing in `iterations` sign
/// conventions for the *valid* range.
#[test]
fn t_gjk_type_range_valid_only() {
    let l = libs();
    let (cf, rf) = l.sym::<GjkFn>(b"c2GJK");
    let shapes = [
        Shape::Circle(c2Circle {
            p: c2v { x: 1.0, y: 2.0 },
            r: 3.0,
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 2.0, y: 0.0 },
            r: 1.0,
        }),
    ];
    for A in shapes.iter() {
        for B in shapes.iter() {
            for ur in [0i32, 1] {
                let co = unsafe { call_gjk(&cf, A, None, B, None, ur, None) };
                let ro = unsafe { call_gjk(&rf, A, None, B, None, ur, None) };
                assert_gjk_eq(&co, &ro, &format!("fixed A={A:?} B={B:?} ur={ur}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Boolean predicates
// ---------------------------------------------------------------------------

#[test]
fn t_c2AABBtoAABB() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2AABB, c2AABB) -> c_int>(b"c2AABBtoAABB");
    let mut rng = Rng::new(50);
    for tame in [true, false] {
        for i in 0..8000 {
            let (A, B) = match (rand_shape(&mut rng, 1, tame), rand_shape(&mut rng, 1, tame)) {
                (Shape::Aabb(a), Shape::Aabb(b)) => (a, b),
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { c(A, B) },
                unsafe { r(A, B) },
                "tame={tame} i={i} A={A:?} B={B:?}"
            );
        }
    }
    // Exactly-touching edges.
    for d in [-1e-7f32, 0.0, 1e-7, 1.0, -1.0] {
        let A = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let B = c2AABB {
            min: c2v { x: 1.0 + d, y: 0.0 },
            max: c2v { x: 2.0 + d, y: 1.0 },
        };
        assert_eq!(unsafe { c(A, B) }, unsafe { r(A, B) }, "touch d={d}");
    }
}

#[test]
fn t_c2CircletoCircle() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2Circle, c2Circle) -> c_int>(b"c2CircletoCircle");
    let mut rng = Rng::new(51);
    for tame in [true, false] {
        for i in 0..8000 {
            let (A, B) = match (rand_shape(&mut rng, 0, tame), rand_shape(&mut rng, 0, tame)) {
                (Shape::Circle(a), Shape::Circle(b)) => (a, b),
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { c(A, B) },
                unsafe { r(A, B) },
                "tame={tame} i={i} A={A:?} B={B:?}"
            );
        }
    }
}

#[test]
fn t_c2CircletoAABB() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2Circle, c2AABB) -> c_int>(b"c2CircletoAABB");
    let mut rng = Rng::new(52);
    for tame in [true, false] {
        for i in 0..8000 {
            let A = match rand_shape(&mut rng, 0, tame) {
                Shape::Circle(a) => a,
                _ => unreachable!(),
            };
            let B = match rand_shape(&mut rng, 1, tame) {
                Shape::Aabb(b) => b,
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { c(A, B) },
                unsafe { r(A, B) },
                "tame={tame} i={i} A={A:?} B={B:?}"
            );
        }
    }
}

#[test]
fn t_c2CircletoCapsule() {
    let l = libs();
    let (c, r) =
        l.sym::<unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int>(b"c2CircletoCapsule");
    let mut rng = Rng::new(53);
    for tame in [true, false] {
        for i in 0..8000 {
            let A = match rand_shape(&mut rng, 0, tame) {
                Shape::Circle(a) => a,
                _ => unreachable!(),
            };
            let B = match rand_shape(&mut rng, 2, tame) {
                Shape::Capsule(b) => b,
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { c(A, B) },
                unsafe { r(A, B) },
                "tame={tame} i={i} A={A:?} B={B:?}"
            );
        }
    }
    // Degenerate capsule (a == b) makes `c2Dot(n, n)` zero -> division by zero.
    let cap = c2Capsule {
        a: c2v { x: 3.0, y: 3.0 },
        b: c2v { x: 3.0, y: 3.0 },
        r: 2.0,
    };
    let mut rng = Rng::new(54);
    for i in 0..2000 {
        let A = c2Circle {
            p: rng.vec(),
            r: rng.unit() * 10.0,
        };
        assert_eq!(
            unsafe { c(A, cap) },
            unsafe { r(A, cap) },
            "degenerate capsule i={i} A={A:?}"
        );
    }
}

#[test]
fn t_c2AABBtoCapsule() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int>(b"c2AABBtoCapsule");
    let mut rng = Rng::new(55);
    for tame in [true, false] {
        for i in 0..4000 {
            let A = match rand_shape(&mut rng, 1, tame) {
                Shape::Aabb(a) => a,
                _ => unreachable!(),
            };
            let B = match rand_shape(&mut rng, 2, tame) {
                Shape::Capsule(b) => b,
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { c(A, B) },
                unsafe { r(A, B) },
                "tame={tame} i={i} A={A:?} B={B:?}"
            );
        }
    }
}

#[test]
fn t_c2CapsuletoCapsule() {
    let l = libs();
    let (c, r) =
        l.sym::<unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int>(b"c2CapsuletoCapsule");
    let mut rng = Rng::new(56);
    for tame in [true, false] {
        for i in 0..4000 {
            let A = match rand_shape(&mut rng, 2, tame) {
                Shape::Capsule(a) => a,
                _ => unreachable!(),
            };
            let B = match rand_shape(&mut rng, 2, tame) {
                Shape::Capsule(b) => b,
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { c(A, B) },
                unsafe { r(A, B) },
                "tame={tame} i={i} A={A:?} B={B:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// c2Collided
// ---------------------------------------------------------------------------

#[test]
fn t_c2Collided() {
    let l = libs();
    let (c, r) = l
        .sym::<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int>(b"c2Collided");
    let mut rng = Rng::new(57);
    for tame in [true, false] {
        for ka in 0..3usize {
            for kb in 0..3usize {
                for i in 0..2500 {
                    let A = rand_shape(&mut rng, ka, tame);
                    let B = rand_shape(&mut rng, kb, tame);
                    assert_eq!(
                        unsafe { c(A.ptr(), A.ty(), B.ptr(), B.ty()) },
                        unsafe { r(A.ptr(), A.ty(), B.ptr(), B.ty()) },
                        "tame={tame} ka={ka} kb={kb} i={i} A={A:?} B={B:?}"
                    );
                }
            }
        }
    }
    // Out-of-range type tags hit the `default:` arms, which return 0 without
    // dereferencing the shape pointers.
    let s = Shape::Circle(c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 1.0,
    });
    for ta in [-1i32, 3, 7, 1000] {
        for tb in [-1i32, 0, 1, 2, 3, 7] {
            assert_eq!(
                unsafe { c(s.ptr(), ta, s.ptr(), tb) },
                unsafe { r(s.ptr(), ta, s.ptr(), tb) },
                "bad typeA={ta} typeB={tb}"
            );
        }
    }
    for ta in [0i32, 1, 2] {
        for tb in [-1i32, 3, 7, 1000] {
            assert_eq!(
                unsafe { c(s.ptr(), ta, s.ptr(), tb) },
                unsafe { r(s.ptr(), ta, s.ptr(), tb) },
                "typeA={ta} bad typeB={tb}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// aabb — the public entry point
// ---------------------------------------------------------------------------

#[test]
fn t_aabb_public_entry() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(f32, f32, f32, f32) -> c_int>(b"aabb");

    // Dense sweep over the region occupied by the three hard-coded shapes.
    let mut checked = 0usize;
    let mut results = [0usize; 8];
    let mut n = -120i32;
    while n <= 120 {
        let mut m = -120i32;
        while m <= 120 {
            let min_x = n as f32;
            let min_y = m as f32;
            for (dx, dy) in [(5.0f32, 5.0f32), (25.0, 25.0), (0.0, 0.0), (60.0, 15.0)] {
                let cv = unsafe { c(min_x, min_y, min_x + dx, min_y + dy) };
                let rv = unsafe { r(min_x, min_y, min_x + dx, min_y + dy) };
                assert_eq!(
                    cv, rv,
                    "aabb({min_x}, {min_y}, {}, {})",
                    min_x + dx,
                    min_y + dy
                );
                if (0..8).contains(&cv) {
                    results[cv as usize] += 1;
                }
                checked += 1;
            }
            m += 5;
        }
        n += 5;
    }
    assert!(checked > 4000, "sweep too small: {checked}");
    // The sweep must actually exercise several distinct return values.
    let distinct = results.iter().filter(|&&v| v > 0).count();
    assert!(distinct >= 4, "only {distinct} distinct results seen: {results:?}");

    // Random + adversarial inputs.
    let mut rng = Rng::new(58);
    for i in 0..40_000 {
        let (a, b, cc, d) = if i % 3 == 0 {
            (rng.spicy(), rng.spicy(), rng.spicy(), rng.spicy())
        } else {
            (rng.coord(), rng.coord(), rng.coord(), rng.coord())
        };
        assert_eq!(
            unsafe { c(a, b, cc, d) },
            unsafe { r(a, b, cc, d) },
            "i={i} aabb({a:?}, {b:?}, {cc:?}, {d:?})"
        );
    }
    // NaN inputs: the return value is an int derived from comparisons, so it
    // must match exactly even though NaN payloads are unspecified.
    let mut rng = Rng::new(59);
    for i in 0..20_000 {
        let (a, b, cc, d) = (rng.nanny(), rng.nanny(), rng.nanny(), rng.nanny());
        assert_eq!(
            unsafe { c(a, b, cc, d) },
            unsafe { r(a, b, cc, d) },
            "nan i={i} aabb({a:?}, {b:?}, {cc:?}, {d:?})"
        );
    }
}
