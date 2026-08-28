//! Level 3: the full `c2GJK` solver and the `gjk_cache` entry point.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_int, c_void};

/// A shape kept alive together with the `C2_TYPE` tag and vertex count that
/// `c2MakeProxy` would derive from it.
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
    fn vert_count(&self) -> u32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    fn describe(&self) -> String {
        match self {
            Shape::Circle(c) => format!("Circle{c:?}"),
            Shape::Aabb(c) => format!("AABB{c:?}"),
            Shape::Capsule(c) => format!("Capsule{c:?}"),
        }
    }
}

fn rand_shape(g: &mut Rng, range: f32) -> Shape {
    match g.below(3) {
        0 => Shape::Circle(c2Circle {
            p: g.v(range),
            r: g.f(range).abs(),
        }),
        1 => {
            // Both well-formed and inverted boxes.
            let a = g.v(range);
            let b = g.v(range);
            if g.below(4) == 0 {
                Shape::Aabb(c2AABB { min: a, max: b })
            } else {
                Shape::Aabb(c2AABB {
                    min: c2v {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    max: c2v {
                        x: a.x.max(b.x),
                        y: a.y.max(b.y),
                    },
                })
            }
        }
        _ => Shape::Capsule(c2Capsule {
            a: g.v(range),
            b: if g.below(6) == 0 {
                // degenerate capsule (zero-length segment)
                c2v { x: 0.0, y: 0.0 }
            } else {
                g.v(range)
            },
            r: g.f(range).abs(),
        }),
    }
}

fn rand_transform(g: &mut Rng, range: f32) -> c2x {
    let ang = g.f(std::f32::consts::PI);
    c2x {
        p: g.v(range),
        r: match g.below(4) {
            0 => c2r { c: 1.0, s: 0.0 },
            _ => c2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        },
    }
}

struct GjkOutcome {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
unsafe fn run(
    f: FnGJK,
    A: &Shape,
    ax: Option<&c2x>,
    B: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    want_iters: bool,
    cache_in: Option<c2GJKCache>,
    sentinel: (c2v, c2v),
) -> GjkOutcome {
    let (mut a, mut b) = sentinel;
    let mut iters: c_int = -12345;
    let mut cache = cache_in.unwrap_or_default();
    let dist = f(
        A.ptr(),
        A.ty(),
        ax.map_or(std::ptr::null(), |x| x as *const c2x),
        B.ptr(),
        B.ty(),
        bx.map_or(std::ptr::null(), |x| x as *const c2x),
        &mut a,
        &mut b,
        use_radius,
        if want_iters {
            &mut iters
        } else {
            std::ptr::null_mut()
        },
        if cache_in.is_some() {
            &mut cache
        } else {
            std::ptr::null_mut()
        },
    );
    GjkOutcome {
        dist,
        a,
        b,
        iters,
        cache,
    }
}

fn compare(ctx: &str, c: &GjkOutcome, r: &GjkOutcome) {
    assert_f32(&format!("{ctx} / dist"), c.dist, r.dist);
    assert_v(&format!("{ctx} / outA"), c.a, r.a);
    assert_v(&format!("{ctx} / outB"), c.b, r.b);
    assert_eq!(c.iters, r.iters, "{ctx} / iterations");
    assert_bytes(&format!("{ctx} / cache"), &c.cache, &r.cache);
}

/// Exhaustive sweep over shape-type pairs, transforms and flags.
#[test]
fn t_c2GJK_random() {
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let mut g = Rng::new(30);
    for i in 0..40_000u32 {
        // Mixed scales: overlapping shapes at small range, separated at large.
        let range = match i % 4 {
            0 => 1.0,
            1 => 10.0,
            2 => 100.0,
            _ => 1000.0,
        };
        let A = rand_shape(&mut g, range);
        let B = rand_shape(&mut g, range);
        let ax = if g.below(3) == 0 {
            None
        } else {
            Some(rand_transform(&mut g, range))
        };
        let bx = if g.below(3) == 0 {
            None
        } else {
            Some(rand_transform(&mut g, range))
        };
        let use_radius = (g.below(2)) as c_int;
        let want_iters = g.below(2) == 0;
        let with_cache = g.below(2) == 0;
        let sentinel = (g.v(RANGE_SENTINEL), g.v(RANGE_SENTINEL));

        let cache_in = if with_cache {
            Some(c2GJKCache::default())
        } else {
            None
        };
        let ctx = format!(
            "c2GJK[{i}] A={} ax={:?} B={} bx={:?} use_radius={use_radius} iters={want_iters} cache={with_cache}",
            A.describe(),
            ax,
            B.describe(),
            bx
        );
        unsafe {
            let co = run(
                cf,
                &A,
                ax.as_ref(),
                &B,
                bx.as_ref(),
                use_radius,
                want_iters,
                cache_in,
                sentinel,
            );
            let ro = run(
                rf,
                &A,
                ax.as_ref(),
                &B,
                bx.as_ref(),
                use_radius,
                want_iters,
                cache_in,
                sentinel,
            );
            compare(&ctx, &co, &ro);
        }
    }
}

const RANGE_SENTINEL: f32 = 77.0;

/// Every ordered pair of shape types, with and without transforms/radius.
#[test]
fn t_c2GJK_type_matrix() {
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let mut g = Rng::new(31);
    let mk = |g: &mut Rng, ty: u32, range: f32| -> Shape {
        match ty {
            0 => Shape::Circle(c2Circle {
                p: g.v(range),
                r: g.f(range).abs(),
            }),
            1 => {
                let a = g.v(range);
                let b = g.v(range);
                Shape::Aabb(c2AABB {
                    min: c2v {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    max: c2v {
                        x: a.x.max(b.x),
                        y: a.y.max(b.y),
                    },
                })
            }
            _ => Shape::Capsule(c2Capsule {
                a: g.v(range),
                b: g.v(range),
                r: g.f(range).abs(),
            }),
        }
    };
    for tA in 0..3u32 {
        for tB in 0..3u32 {
            for use_radius in [0, 1] {
                for with_tf in [false, true] {
                    for _ in 0..800 {
                        let range = 20.0;
                        let A = mk(&mut g, tA, range);
                        let B = mk(&mut g, tB, range);
                        let (ax, bx) = if with_tf {
                            (
                                Some(rand_transform(&mut g, range)),
                                Some(rand_transform(&mut g, range)),
                            )
                        } else {
                            (None, None)
                        };
                        let sentinel = (g.v(RANGE_SENTINEL), g.v(RANGE_SENTINEL));
                        let ctx = format!(
                            "c2GJK matrix tA={tA} tB={tB} radius={use_radius} tf={with_tf} A={} B={}",
                            A.describe(),
                            B.describe()
                        );
                        unsafe {
                            let co = run(
                                cf,
                                &A,
                                ax.as_ref(),
                                &B,
                                bx.as_ref(),
                                use_radius,
                                true,
                                Some(c2GJKCache::default()),
                                sentinel,
                            );
                            let ro = run(
                                rf,
                                &A,
                                ax.as_ref(),
                                &B,
                                bx.as_ref(),
                                use_radius,
                                true,
                                Some(c2GJKCache::default()),
                                sentinel,
                            );
                            compare(&ctx, &co, &ro);
                        }
                    }
                }
            }
        }
    }
}

/// Feeds a pre-populated cache back in, which is the branch `gjk_cache` relies
/// on. Indices are kept inside each proxy's vertex count so that both
/// implementations read initialised vertices.
#[test]
fn t_c2GJK_warm_cache() {
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let mut g = Rng::new(32);
    for i in 0..30_000u32 {
        let range = if i % 2 == 0 { 20.0 } else { 200.0 };
        let A = rand_shape(&mut g, range);
        let B = rand_shape(&mut g, range);
        let nA = A.vert_count();
        let nB = B.vert_count();

        let count = g.below(4) as i32; // 0..3
        let mut cache = c2GJKCache {
            metric: match g.below(5) {
                0 => 0.0,
                1 => -1.0e9,
                2 => f32::INFINITY,
                _ => g.f(range),
            },
            count,
            iA: [0; 3],
            iB: [0; 3],
            div: match g.below(4) {
                0 => 1.0,
                1 => 0.0,
                _ => g.f(range),
            },
        };
        for k in 0..3 {
            cache.iA[k] = g.below(nA) as i32;
            cache.iB[k] = g.below(nB) as i32;
        }
        let use_radius = g.below(2) as c_int;
        let sentinel = (g.v(RANGE_SENTINEL), g.v(RANGE_SENTINEL));
        let ctx = format!(
            "c2GJK warm[{i}] A={} B={} cache={:?} radius={use_radius}",
            A.describe(),
            B.describe(),
            cache
        );
        unsafe {
            let co = run(
                cf,
                &A,
                None,
                &B,
                None,
                use_radius,
                true,
                Some(cache),
                sentinel,
            );
            let ro = run(
                rf,
                &A,
                None,
                &B,
                None,
                use_radius,
                true,
                Some(cache),
                sentinel,
            );
            compare(&ctx, &co, &ro);
        }
    }
}

/// Repeatedly re-uses each implementation's own cache output, mirroring how a
/// caller drives the solver frame after frame. Divergence compounds here, so it
/// is a strong end-to-end check.
#[test]
fn t_c2GJK_cache_chain() {
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let mut g = Rng::new(33);
    for _ in 0..4_000 {
        let range = 30.0;
        let A = rand_shape(&mut g, range);
        let B = rand_shape(&mut g, range);
        let mut c_cache = c2GJKCache::default();
        let mut r_cache = c2GJKCache::default();
        for step in 0..6 {
            let ax = rand_transform(&mut g, range);
            let bx = rand_transform(&mut g, range);
            let use_radius = (step % 2) as c_int;
            let sentinel = (g.v(RANGE_SENTINEL), g.v(RANGE_SENTINEL));
            let ctx = format!(
                "c2GJK chain step={step} A={} B={}",
                A.describe(),
                B.describe()
            );
            unsafe {
                let co = run(
                    cf,
                    &A,
                    Some(&ax),
                    &B,
                    Some(&bx),
                    use_radius,
                    true,
                    Some(c_cache),
                    sentinel,
                );
                let ro = run(
                    rf,
                    &A,
                    Some(&ax),
                    &B,
                    Some(&bx),
                    use_radius,
                    true,
                    Some(r_cache),
                    sentinel,
                );
                compare(&ctx, &co, &ro);
                c_cache = co.cache;
                r_cache = ro.cache;
            }
        }
    }
}

/// Hand-picked degenerate configurations: coincident shapes, zero radii, zero
/// extents, exact containment and touching contacts.
#[test]
fn t_c2GJK_degenerate() {
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let zero = c2v { x: 0.0, y: 0.0 };
    let shapes: Vec<Shape> = vec![
        Shape::Circle(c2Circle { p: zero, r: 0.0 }),
        Shape::Circle(c2Circle { p: zero, r: 1.0 }),
        Shape::Circle(c2Circle {
            p: c2v { x: 2.0, y: 0.0 },
            r: 1.0,
        }),
        Shape::Circle(c2Circle {
            p: c2v { x: 1.0e-8, y: 0.0 },
            r: 1.1920929e-7,
        }),
        Shape::Aabb(c2AABB {
            min: zero,
            max: zero,
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: 1.0, y: -1.0 },
            max: c2v { x: 3.0, y: 1.0 },
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: -1.0, y: -1.0 },
        }),
        Shape::Capsule(c2Capsule {
            a: zero,
            b: zero,
            r: 0.0,
        }),
        Shape::Capsule(c2Capsule {
            a: zero,
            b: zero,
            r: 2.0,
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: -5.0, y: 0.0 },
            b: c2v { x: 5.0, y: 0.0 },
            r: 1.0,
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: 0.0, y: 2.0 },
            b: c2v { x: 0.0, y: 3.0 },
            r: 1.0,
        }),
    ];
    let transforms = [
        None,
        Some(c2x {
            p: zero,
            r: c2r { c: 1.0, s: 0.0 },
        }),
        Some(c2x {
            p: c2v { x: 10.0, y: -10.0 },
            r: c2r {
                c: 0.70710677,
                s: 0.70710677,
            },
        }),
        Some(c2x {
            p: zero,
            r: c2r { c: 0.0, s: 0.0 },
        }),
    ];
    let sentinel = (c2v { x: 9.0, y: -9.0 }, c2v { x: -8.0, y: 8.0 });
    for A in &shapes {
        for B in &shapes {
            for ax in &transforms {
                for bx in &transforms {
                    for use_radius in [0, 1] {
                        for with_cache in [false, true] {
                            let ctx = format!(
                                "c2GJK degenerate A={} B={} ax={ax:?} bx={bx:?} radius={use_radius} cache={with_cache}",
                                A.describe(),
                                B.describe()
                            );
                            let cache_in = if with_cache {
                                Some(c2GJKCache::default())
                            } else {
                                None
                            };
                            unsafe {
                                let co = run(
                                    cf,
                                    A,
                                    ax.as_ref(),
                                    B,
                                    bx.as_ref(),
                                    use_radius,
                                    true,
                                    cache_in,
                                    sentinel,
                                );
                                let ro = run(
                                    rf,
                                    A,
                                    ax.as_ref(),
                                    B,
                                    bx.as_ref(),
                                    use_radius,
                                    true,
                                    cache_in,
                                    sentinel,
                                );
                                compare(&ctx, &co, &ro);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Null `outA` / `outB` / `iterations` / `cache` in every combination.
#[test]
fn t_c2GJK_null_outputs() {
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let A = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let B = c2Capsule {
        a: c2v { x: 3.0, y: 0.0 },
        b: c2v { x: 5.0, y: 2.0 },
        r: 0.5,
    };
    for mask in 0..16u32 {
        let want_a = mask & 1 != 0;
        let want_b = mask & 2 != 0;
        let want_it = mask & 4 != 0;
        let want_cache = mask & 8 != 0;
        let mut ca = c2v { x: 1.0, y: 2.0 };
        let mut cb = c2v { x: 3.0, y: 4.0 };
        let mut ra = ca;
        let mut rb = cb;
        let mut cit: c_int = -999;
        let mut rit: c_int = -999;
        let mut cc = c2GJKCache::default();
        let mut rc = c2GJKCache::default();
        let call = |f: FnGJK,
                    oa: *mut c2v,
                    ob: *mut c2v,
                    it: *mut c_int,
                    ch: *mut c2GJKCache|
         -> f32 {
            unsafe {
                f(
                    &A as *const c2AABB as *const c_void,
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    &B as *const c2Capsule as *const c_void,
                    C2_TYPE_CAPSULE,
                    std::ptr::null(),
                    oa,
                    ob,
                    1,
                    it,
                    ch,
                )
            }
        };
        let np = std::ptr::null_mut();
        let cd = call(
            cf,
            if want_a { &mut ca } else { np },
            if want_b { &mut cb } else { np },
            if want_it {
                &mut cit
            } else {
                std::ptr::null_mut()
            },
            if want_cache {
                &mut cc
            } else {
                std::ptr::null_mut()
            },
        );
        let rd = call(
            rf,
            if want_a { &mut ra } else { np },
            if want_b { &mut rb } else { np },
            if want_it {
                &mut rit
            } else {
                std::ptr::null_mut()
            },
            if want_cache {
                &mut rc
            } else {
                std::ptr::null_mut()
            },
        );
        let ctx = format!("c2GJK nulls mask={mask:04b}");
        assert_f32(&format!("{ctx} / dist"), cd, rd);
        assert_v(&format!("{ctx} / outA"), ca, ra);
        assert_v(&format!("{ctx} / outB"), cb, rb);
        assert_eq!(cit, rit, "{ctx} / iterations");
        assert_bytes(&format!("{ctx} / cache"), &cc, &rc);
    }
}

/// The exported entry point from `include/lib.h`. The C body writes nothing
/// through `a9`/`b9` and returns void, so the observable contract is that the
/// caller's buffers stay untouched and the call is side-effect free.
#[test]
fn t_gjk_cache() {
    let (cf, rf) = both::<FnGjkCache>("gjk_cache");
    let mut g = Rng::new(34);

    let check = |reverse: i8, p: [f32; 9], label: String| {
        let seed_a = c2v { x: 11.5, y: -22.25 };
        let seed_b = c2v { x: -33.75, y: 44.0 };
        let (mut ca, mut cb) = (seed_a, seed_b);
        let (mut ra, mut rb) = (seed_a, seed_b);
        unsafe {
            cf(
                reverse, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
            );
            rf(
                reverse, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
            );
        }
        assert_v(&format!("{label} / a9"), ca, ra);
        assert_v(&format!("{label} / b9"), cb, rb);
        // Both must leave the buffers exactly as the C original does.
        assert_v(&format!("{label} / a9 untouched"), seed_a, ca);
        assert_v(&format!("{label} / b9 untouched"), seed_b, cb);
    };

    // Deterministic sweep.
    for i in 0..20_000u32 {
        let range = match i % 4 {
            0 => 1.0,
            1 => 25.0,
            2 => 150.0,
            _ => 2000.0,
        };
        let mut p = [0f32; 9];
        let a = g.v(range);
        let b = g.v(range);
        p[0] = a.x.min(b.x);
        p[1] = a.y.min(b.y);
        p[2] = a.x.max(b.x);
        p[3] = a.y.max(b.y);
        let ca = g.v(range);
        let cb = g.v(range);
        p[4] = ca.x;
        p[5] = ca.y;
        p[6] = cb.x;
        p[7] = cb.y;
        p[8] = g.f(range).abs();
        let reverse = if i % 2 == 0 { 0 } else { 1 };
        check(reverse, p, format!("gjk_cache[{i}] reverse={reverse}"));
    }

    // Inverted AABBs, zero-size shapes and a few extreme values.
    let specials: [[f32; 9]; 8] = [
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        [-1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [-10.0, -10.0, 10.0, 10.0, 0.0, 0.0, 0.0, 0.0, 5.0],
        [
            -1.0e30, -1.0e30, 1.0e30, 1.0e30, -1.0e30, 1.0e30, 1.0e30, -1.0e30, 1.0e30,
        ],
        [
            1.1920929e-7,
            1.1920929e-7,
            1.1920929e-7,
            1.1920929e-7,
            0.0,
            0.0,
            0.0,
            0.0,
            1.1920929e-7,
        ],
        [-1.0, -1.0, 1.0, 1.0, 100.0, -25.0, 75.0, 100.0, 10.0],
        [
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::MIN_POSITIVE,
            f32::MIN_POSITIVE,
            1.0,
            1.0,
            1.0,
            1.0,
            0.0,
        ],
    ];
    for (i, p) in specials.iter().enumerate() {
        for reverse in [0i8, 1, -1, 127, -128] {
            check(reverse, *p, format!("gjk_cache special[{i}] reverse={reverse}"));
        }
    }
}

/// Coverage probe (diagnostic): reports how often the generated inputs reach the
/// interesting branches of `c2GJK` -- deep iteration counts, the `hit` (count==3)
/// early exit, and each terminal simplex size. Run with
/// `cargo test --test level3_gjk -- --ignored --nocapture`.
#[test]
#[ignore = "diagnostic: reports which c2GJK branches the generators reach"]
fn coverage_probe() {
    let (cf, _) = both::<FnGJK>("c2GJK");
    let mut g = Rng::new(30); // same seed as t_c2GJK_random
    let mut iter_hist = [0u32; 21];
    let mut count_hist = [0u32; 4];
    let mut zero_dist = 0u32;
    let mut radius_shrink = 0u32;
    let total = 40_000u32;
    for i in 0..total {
        let range = match i % 4 {
            0 => 1.0,
            1 => 10.0,
            2 => 100.0,
            _ => 1000.0,
        };
        let A = rand_shape(&mut g, range);
        let B = rand_shape(&mut g, range);
        let ax = if g.below(3) == 0 {
            None
        } else {
            Some(rand_transform(&mut g, range))
        };
        let bx = if g.below(3) == 0 {
            None
        } else {
            Some(rand_transform(&mut g, range))
        };
        let use_radius = (g.below(2)) as c_int;
        let _ = g.below(2);
        let _ = g.below(2);
        let sentinel = (g.v(RANGE_SENTINEL), g.v(RANGE_SENTINEL));
        unsafe {
            let o = run(
                cf,
                &A,
                ax.as_ref(),
                &B,
                bx.as_ref(),
                use_radius,
                true,
                Some(c2GJKCache::default()),
                sentinel,
            );
            if o.iters >= 0 && o.iters <= 20 {
                iter_hist[o.iters as usize] += 1;
            }
            if o.cache.count >= 1 && o.cache.count <= 3 {
                count_hist[o.cache.count as usize] += 1;
            }
            if o.dist == 0.0 {
                zero_dist += 1;
            }
            if use_radius == 1 && o.dist > 0.0 {
                radius_shrink += 1;
            }
        }
    }
    println!("\nc2GJK coverage over {total} random cases");
    println!("  iterations histogram:");
    for (k, n) in iter_hist.iter().enumerate() {
        if *n > 0 {
            println!("    iter={k:<3} {n}");
        }
    }
    println!("  terminal simplex count:");
    for (k, n) in count_hist.iter().enumerate().skip(1) {
        println!("    count={k} {n}");
    }
    println!("  dist == 0 (hit or radius-collapsed): {zero_dist}");
    println!("  use_radius with separation kept:      {radius_shrink}");
    assert!(
        count_hist[3] > 0 && count_hist[2] > 0 && count_hist[1] > 0,
        "generators must reach every terminal simplex size"
    );
    assert!(
        iter_hist[2] > 0 || iter_hist[3] > 0,
        "generators must reach multi-iteration solves"
    );
}
