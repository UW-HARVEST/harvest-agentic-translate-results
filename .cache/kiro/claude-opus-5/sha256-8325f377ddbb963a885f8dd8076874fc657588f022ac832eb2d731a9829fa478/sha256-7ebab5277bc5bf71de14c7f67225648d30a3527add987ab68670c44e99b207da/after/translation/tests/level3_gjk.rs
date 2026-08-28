//! Level 3: `c2GJK`, the full distance query.

#![allow(non_snake_case)]

mod harness;
use harness::*;
use std::ffi::c_void;

type FnGJK = unsafe extern "C" fn(
    *const c_void,
    i32,
    *const X,
    *const c_void,
    i32,
    *const X,
    *mut V,
    *mut V,
    i32,
    *mut i32,
    *mut GJKCache,
) -> f32;

/// One of the three shapes, kept alive by the caller.
#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Circle),
    Aabb(AABB),
    Capsule(Capsule),
}

impl Shape {
    fn ty(&self) -> i32 {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const Circle as *const _,
            Shape::Aabb(b) => b as *const AABB as *const _,
            Shape::Capsule(c) => c as *const Capsule as *const _,
        }
    }

    /// Number of proxy vertices, i.e. the legal range for cached indices.
    fn vert_count(&self) -> u32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

/// `scale` controls how far apart the shapes are placed, which selects between
/// the separated, touching and overlapping regimes.
fn rand_shape(rng: &mut Rng, scale: f32) -> Shape {
    let coord = |rng: &mut Rng| -> f32 {
        match rng.below(6) {
            0 => 0.0,
            1 => (rng.below(9) as f32 - 4.0) * scale,
            2 => (rng.below(9) as f32 - 4.0) * 0.5 * scale,
            _ => rng.unit() * 4.0 * scale,
        }
    };
    let radius = |rng: &mut Rng| -> f32 {
        match rng.below(5) {
            0 => 0.0,
            1 => 1.0 * scale,
            2 => (rng.below(5) as f32) * scale,
            _ => rng.unit().abs() * 2.0 * scale,
        }
    };
    match rng.below(3) {
        0 => Shape::Circle(Circle {
            p: V {
                x: coord(rng),
                y: coord(rng),
            },
            r: radius(rng),
        }),
        1 => {
            // Both well-formed and inverted boxes: the C code never validates.
            let (a, b) = (coord(rng), coord(rng));
            let (c, d) = (coord(rng), coord(rng));
            Shape::Aabb(AABB {
                min: V { x: a, y: c },
                max: V { x: b, y: d },
            })
        }
        _ => Shape::Capsule(Capsule {
            a: V {
                x: coord(rng),
                y: coord(rng),
            },
            b: V {
                x: coord(rng),
                y: coord(rng),
            },
            r: radius(rng),
        }),
    }
}

fn rand_transform(rng: &mut Rng, scale: f32) -> X {
    let angle = match rng.below(4) {
        0 => 0.0,
        1 => std::f32::consts::FRAC_PI_2,
        2 => std::f32::consts::PI,
        _ => rng.unit() * std::f32::consts::PI,
    };
    X {
        p: V {
            x: rng.unit() * 4.0 * scale,
            y: rng.unit() * 4.0 * scale,
        },
        r: R {
            c: angle.cos(),
            s: angle.sin(),
        },
    }
}

struct Case {
    a: Shape,
    b: Shape,
    ax: Option<X>,
    bx: Option<X>,
    use_radius: i32,
    cache: Option<GJKCache>,
}

impl std::fmt::Debug for Case {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Case {{ a: {:?}, b: {:?}, ax: {:?}, bx: {:?}, use_radius: {}, cache: {:?} }}",
            self.a, self.b, self.ax, self.bx, self.use_radius, self.cache
        )
    }
}

fn rand_case(rng: &mut Rng, scale: f32, with_transforms: bool, with_cache: bool) -> Case {
    let a = rand_shape(rng, scale);
    let b = rand_shape(rng, scale);
    let cache = if with_cache && rng.below(2) == 0 {
        // Cached indices must stay inside each proxy's vertex range; going
        // outside is an out-of-bounds read in the C original (it would read
        // uninitialised stack), so it is not a behaviour that can be compared.
        // Counts above 3 are likewise out of bounds for the C `int saveA[3]`,
        // but zero and negative counts are perfectly well defined there (every
        // `for (int i = 0; i < count; ++i)` simply does not run).
        let count = match rng.below(8) {
            0 => 0,
            1 => -1,
            2 => -1234,
            n => (n as i32) % 3 + 1,
        };
        let mut c = GJKCache {
            metric: match rng.below(4) {
                0 => 0.0,
                1 => rng.unit() * 10.0,
                2 => -1.0e9,
                _ => rng.unit().abs() * 1.0e9,
            },
            count,
            iA: [0; 3],
            iB: [0; 3],            div: match rng.below(3) {
                0 => 1.0,
                1 => 0.0,
                _ => rng.unit() * 10.0,
            },
        };
        for i in 0..3 {
            c.iA[i] = rng.below(a.vert_count()) as i32;
            c.iB[i] = rng.below(b.vert_count()) as i32;
        }
        Some(c)
    } else {
        None
    };
    Case {
        a,
        b,
        ax: if with_transforms && rng.below(2) == 0 {
            Some(rand_transform(rng, scale))
        } else {
            None
        },
        bx: if with_transforms && rng.below(2) == 0 {
            Some(rand_transform(rng, scale))
        } else {
            None
        },
        use_radius: (rng.below(2) as i32),
        cache,
    }
}

struct Outcome {
    dist: f32,
    a: V,
    b: V,
    iters: i32,
    cache: GJKCache,
}

#[allow(clippy::too_many_arguments)]
fn run(f: &FnGJK, case: &Case, out_null: bool) -> Outcome {
    let mut oa = V { x: -7.5, y: 13.25 };
    let mut ob = V { x: 91.0, y: -0.5 };
    let mut iters: i32 = -12345;
    let mut cache = case.cache.unwrap_or_default();

    let ax = case.ax;
    let bx = case.bx;
    let ax_ptr = ax.as_ref().map_or(std::ptr::null(), |v| v as *const X);
    let bx_ptr = bx.as_ref().map_or(std::ptr::null(), |v| v as *const X);
    let cache_ptr = if case.cache.is_some() {
        &mut cache as *mut GJKCache
    } else {
        std::ptr::null_mut()
    };

    let dist = unsafe {
        f(
            case.a.ptr(),
            case.a.ty(),
            ax_ptr,
            case.b.ptr(),
            case.b.ty(),
            bx_ptr,
            if out_null {
                std::ptr::null_mut()
            } else {
                &mut oa
            },
            if out_null {
                std::ptr::null_mut()
            } else {
                &mut ob
            },
            case.use_radius,
            if out_null {
                std::ptr::null_mut()
            } else {
                &mut iters
            },
            cache_ptr,
        )
    };

    Outcome {
        dist,
        a: oa,
        b: ob,
        iters,
        cache,
    }
}

#[track_caller]
fn compare(case: &Case, c: &Outcome, r: &Outcome) {
    assert_f("c2GJK dist", case, c.dist, r.dist);
    assert_v("c2GJK outA", case, c.a, r.a);
    assert_v("c2GJK outB", case, c.b, r.b);
    assert_eq!(c.iters, r.iters, "c2GJK iterations\n  input: {case:?}");
    assert!(
        cache_eq(&c.cache, &r.cache),
        "c2GJK cache:\n  C   ={:?}\n  Rust={:?}\n  input: {case:?}",
        c.cache,
        r.cache
    );
}

fn sweep(seed: u64, n0: u32, scale: f32, with_transforms: bool, with_cache: bool) {
    let (c, r) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(seed);
    let n = volume(n0);
    for i in 0..n {
        let case = rand_case(&mut rng, scale, with_transforms, with_cache);
        let out_null = i % 32 == 0;
        let oc = run(&c, &case, out_null);
        let or = run(&r, &case, out_null);
        compare(&case, &oc, &or);
    }
}

#[test]
fn c2GJK_matches_unit_scale() {
    sweep(101, 40_000, 1.0, false, false);
}

#[test]
fn c2GJK_matches_with_transforms() {
    sweep(102, 40_000, 1.0, true, false);
}

#[test]
fn c2GJK_matches_with_cache() {
    sweep(103, 40_000, 1.0, true, true);
}

#[test]
fn c2GJK_matches_tiny_scale() {
    // Around FLT_EPSILON the early-exit tests and the radius shrink both sit
    // right on their thresholds.
    sweep(104, 20_000, 1.0e-7, true, true);
}

#[test]
fn c2GJK_matches_large_scale() {
    sweep(105, 20_000, 1.0e6, true, true);
}

#[test]
fn c2GJK_matches_huge_scale() {
    // Large enough that squared distances overflow to +inf.
    sweep(106, 20_000, 1.0e30, true, true);
}

#[test]
fn c2GJK_matches_degenerate_shapes() {
    // Integer coordinates in a small range produce lots of exactly-coincident
    // points, zero-area boxes and zero-length capsules.
    let (c, r) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(107);
    for _ in 0..volume(40_000) {
        let coord = |rng: &mut Rng| (rng.below(4) as f32) - 1.5;
        let a = match rng.below(3) {
            0 => Shape::Circle(Circle {
                p: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                r: rng.below(3) as f32,
            }),
            1 => Shape::Aabb(AABB {
                min: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                max: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
            }),
            _ => Shape::Capsule(Capsule {
                a: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                b: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                r: rng.below(3) as f32,
            }),
        };
        let b = match rng.below(3) {
            0 => Shape::Circle(Circle {
                p: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                r: rng.below(3) as f32,
            }),
            1 => Shape::Aabb(AABB {
                min: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                max: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
            }),
            _ => Shape::Capsule(Capsule {
                a: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                b: V {
                    x: coord(&mut rng),
                    y: coord(&mut rng),
                },
                r: rng.below(3) as f32,
            }),
        };
        let case = Case {
            a,
            b,
            ax: None,
            bx: None,
            use_radius: rng.below(2) as i32,
            cache: None,
        };
        compare(&case, &run(&c, &case, false), &run(&r, &case, false));
    }
}

/// Same query fed back through its own cache repeatedly, which is the way a
/// caller is meant to use `c2GJKCache` and exercises `cache_was_read == 1`.
#[test]
fn c2GJK_matches_across_cache_reuse_chains() {
    let (c, r) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(108);
    for _ in 0..volume(4_000) {
        let a = rand_shape(&mut rng, 1.0);
        let b = rand_shape(&mut rng, 1.0);
        let use_radius = rng.below(2) as i32;
        let mut cc = GJKCache::default();
        let mut rc = GJKCache::default();
        for _ in 0..6 {
            let case = Case {
                a,
                b,
                ax: None,
                bx: None,
                use_radius,
                cache: Some(cc),
            };
            let rcase = Case {
                a,
                b,
                ax: None,
                bx: None,
                use_radius,
                cache: Some(rc),
            };
            let oc = run(&c, &case, false);
            let or = run(&r, &rcase, false);
            compare(&case, &oc, &or);
            cc = oc.cache;
            rc = or.cache;
        }
    }
}
