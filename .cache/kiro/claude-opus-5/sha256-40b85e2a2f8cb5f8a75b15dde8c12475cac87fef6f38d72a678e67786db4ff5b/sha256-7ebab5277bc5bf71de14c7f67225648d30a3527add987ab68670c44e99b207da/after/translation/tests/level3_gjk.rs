//! Level 3: `c2GJK`.
//!
//! Only `C2_TYPE_CIRCLE` / `C2_TYPE_AABB` / `C2_TYPE_CAPSULE` are driven here.
//! `c2MakeProxy` has no `C2_TYPE_POLY` case, so a polygon operand leaves the C's
//! `c2Proxy` uninitialized; that path is only reachable (and only meaningful)
//! through `c2CapsuletoPolyManifold`, which is covered in `level4_manifolds`.
//!
//! Cache indices are kept inside `[0, proxy.count)` for the same reason: the C
//! would otherwise read uninitialised `c2Proxy::verts` entries.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

type GjkFn = unsafe extern "C" fn(
    *const c_void,
    C2_TYPE,
    *const c2x,
    *const c_void,
    C2_TYPE,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;

/// Owns the shape storage so a `*const c_void` can be handed to both libraries.
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> C2_TYPE {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
    }
    /// Number of proxy vertices `c2MakeProxy` initialises for this shape.
    fn proxy_count(&self) -> u32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
    fn make(rng: &mut Rng, wild: bool) -> Shape {
        let f = |r: &mut Rng| if wild { r.wild() } else { r.tame() };
        match rng.below(3) {
            0 => Shape::Circle(c2Circle {
                p: c2v {
                    x: f(rng),
                    y: f(rng),
                },
                r: if wild { rng.wild() } else { rng.radius() },
            }),
            1 => {
                let (x1, y1, x2, y2) = (f(rng), f(rng), f(rng), f(rng));
                if wild {
                    Shape::Aabb(c2AABB {
                        min: c2v { x: x1, y: y1 },
                        max: c2v { x: x2, y: y2 },
                    })
                } else {
                    Shape::Aabb(c2AABB {
                        min: c2v {
                            x: x1.min(x2),
                            y: y1.min(y2),
                        },
                        max: c2v {
                            x: x1.max(x2),
                            y: y1.max(y2),
                        },
                    })
                }
            }
            _ => Shape::Capsule(c2Capsule {
                a: c2v {
                    x: f(rng),
                    y: f(rng),
                },
                b: c2v {
                    x: f(rng),
                    y: f(rng),
                },
                r: if wild { rng.wild() } else { rng.radius() },
            }),
        }
    }
}

fn gen_x(rng: &mut Rng, wild: bool) -> c2x {
    if wild {
        c2x {
            p: rng.vec_wild(),
            r: c2r {
                c: rng.wild(),
                s: rng.wild(),
            },
        }
    } else {
        let ang = (rng.next_u32() as f64 / u32::MAX as f64) as f32 * 6.283_185_5;
        c2x {
            p: rng.vec_tame(),
            r: c2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        }
    }
}

struct Out {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
unsafe fn call(
    f: &GjkFn,
    A: &Shape,
    ax: Option<&c2x>,
    B: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    cache_in: Option<c2GJKCache>,
) -> Out {
    let mut a = c2v { x: 11.0, y: -11.0 };
    let mut b = c2v { x: -12.0, y: 12.0 };
    let mut iters: c_int = -777;
    let mut cache = cache_in.unwrap_or_default();
    let cache_ptr = if cache_in.is_some() {
        &mut cache as *mut c2GJKCache
    } else {
        std::ptr::null_mut()
    };
    let dist = unsafe {
        f(
            A.ptr(),
            A.ty(),
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            B.ptr(),
            B.ty(),
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            &mut a,
            &mut b,
            use_radius,
            &mut iters,
            cache_ptr,
        )
    };
    Out {
        dist,
        a,
        b,
        iters,
        cache,
    }
}

/// `what` is only invoked when something differs: allocating in the comparison
/// loop perturbs glibc `malloc`'s stack usage, which the C's uninitialised
/// `c2Proxy` is sensitive to (see `common::assert_same_lazy`).
fn compare<F: Fn() -> String>(what: F, c: &Out, r: &Out) {
    assert_f32_lazy(c.dist, r.dist, || format!("{} return", what()));
    assert_same_lazy(&c.a, &r.a, || format!("{} outA", what()));
    assert_same_lazy(&c.b, &r.b, || format!("{} outB", what()));
    if c.iters != r.iters {
        panic!("{} iterations: C={} Rust={}", what(), c.iters, r.iters);
    }
    assert_same_lazy(&c.cache, &r.cache, || format!("{} cache", what()));
}

fn run(seed: u64, iterations: usize, wild: bool, with_cache: bool) {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(seed);
    for it in 0..iterations {
        let A = Shape::make(&mut rng, wild);
        let B = Shape::make(&mut rng, wild);
        let ax = if rng.below(2) == 0 {
            Some(gen_x(&mut rng, wild))
        } else {
            None
        };
        let bx = if rng.below(2) == 0 {
            Some(gen_x(&mut rng, wild))
        } else {
            None
        };
        let use_radius = (rng.below(2)) as c_int;
        let cache = if with_cache {
            let count = rng.below(4) as c_int; // 0 => "cache not good"
            let mut ca = c2GJKCache {
                metric: if wild { rng.wild() } else { rng.tame() },
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: if wild { rng.wild() } else { rng.tame() },
            };
            for k in 0..3 {
                ca.iA[k] = rng.below(A.proxy_count()) as c_int;
                ca.iB[k] = rng.below(B.proxy_count()) as c_int;
            }
            Some(ca)
        } else {
            None
        };
        unsafe {
            let co = call(&cf, &A, ax.as_ref(), &B, bx.as_ref(), use_radius, cache);
            let ro = call(&rf, &A, ax.as_ref(), &B, bx.as_ref(), use_radius, cache);
            compare(
                || {
                    format!(
                        "c2GJK #{it} seed={seed} wild={wild} tA={} tB={} ax={} bx={} ur={use_radius} cache={:?}",
                        A.ty(),
                        B.ty(),
                        ax.is_some(),
                        bx.is_some(),
                        cache
                    )
                },
                &co,
                &ro,
            );
        }
    }
}

#[test]
fn gjk_tame_no_cache() {
    run(201, 60_000, false, false);
}

#[test]
fn gjk_tame_with_cache() {
    run(202, 60_000, false, true);
}

#[test]
fn gjk_wild_no_cache() {
    run(203, 60_000, true, false);
}

#[test]
fn gjk_wild_with_cache() {
    run(204, 60_000, true, true);
}

/// Null `outA`/`outB`/`iterations` must be tolerated identically.
#[test]
fn gjk_null_outputs() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(205);
    for _ in 0..20_000 {
        let A = Shape::make(&mut rng, false);
        let B = Shape::make(&mut rng, false);
        unsafe {
            let dc = cf(
                A.ptr(),
                A.ty(),
                std::ptr::null(),
                B.ptr(),
                B.ty(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            let dr = rf(
                A.ptr(),
                A.ty(),
                std::ptr::null(),
                B.ptr(),
                B.ty(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            assert_f32("c2GJK null outputs", dc, dr);
        }
    }
}

/// Overlapping and exactly-touching configurations, which drive the `hit` and
/// `use_radius` branches much harder than uniform random input.
#[test]
fn gjk_overlapping_grid() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<GjkFn>("c2GJK");
    let coords: [f32; 7] = [-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0];
    let radii: [f32; 4] = [0.0, 0.5, 1.0, 2.0];
    let mut n = 0usize;
    for &dx in &coords {
        for &dy in &coords {
            for &r in &radii {
                let shapes_a = [
                    Shape::Circle(c2Circle {
                        p: c2v { x: 0.0, y: 0.0 },
                        r,
                    }),
                    Shape::Aabb(c2AABB {
                        min: c2v { x: -1.0, y: -1.0 },
                        max: c2v { x: 1.0, y: 1.0 },
                    }),
                    Shape::Capsule(c2Capsule {
                        a: c2v { x: -1.0, y: 0.0 },
                        b: c2v { x: 1.0, y: 0.0 },
                        r,
                    }),
                ];
                let shapes_b = [
                    Shape::Circle(c2Circle {
                        p: c2v { x: dx, y: dy },
                        r,
                    }),
                    Shape::Aabb(c2AABB {
                        min: c2v { x: dx - 1.0, y: dy - 1.0 },
                        max: c2v { x: dx + 1.0, y: dy + 1.0 },
                    }),
                    Shape::Capsule(c2Capsule {
                        a: c2v { x: dx - 1.0, y: dy },
                        b: c2v { x: dx + 1.0, y: dy },
                        r,
                    }),
                ];
                for A in &shapes_a {
                    for B in &shapes_b {
                        for ur in [0, 1] {
                            unsafe {
                                let co = call(&cf, A, None, B, None, ur, None);
                                let ro = call(&rf, A, None, B, None, ur, None);
                                compare(|| format!("grid dx={dx} dy={dy} r={r} ur={ur}"), &co, &ro);
                            }
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(n > 1000, "grid should be non-trivial, got {n}");
}
