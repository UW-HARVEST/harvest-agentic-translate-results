//! Phase B, CONFIGS.md rows 39-56: `c2GJK`, the lowest-level *composed* entry point.
//!
//! Driven the way a real consumer drives it: build the shapes, set the options
//! (transforms, `use_radius`, cache, which out-params are `NULL`), run the whole
//! solve, and compare the return value, both witness points, the iteration count and
//! the entire 36-byte cache byte-for-byte.
//!
//! `C2_TYPE_POLY` is deliberately **excluded** here: `c2MakeProxy` has no POLY case,
//! so the C library reads an uninitialised `c2Proxy` and is not a function of its
//! inputs (see `tests/probe_uninit.rs`).
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 1_500;

/// Owning storage for one shape, so we can hand `c2GJK` a `const void *`.
#[derive(Clone, Copy, Debug)]
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
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    /// Number of vertices this shape's proxy will have -- needed so hand-built
    /// caches only ever reference initialised proxy slots.
    fn proxy_count(&self) -> i32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

fn rvec(rng: &mut Rng, m: f32, special: bool) -> c2v {
    if special { rng.vec_special() } else { rng.vec_norm(m) }
}
fn rrad(rng: &mut Rng, m: f32, special: bool) -> f32 {
    if special { rng.f_special() } else { rng.f_pos(m) }
}

fn rand_shape(rng: &mut Rng, ty: C2_TYPE, mag: f32, special: bool) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => {
            let p = rvec(rng, mag, special);
            Shape::Circle(c2Circle { p, r: rrad(rng, mag * 0.25, special) })
        }
        C2_TYPE_AABB => {
            let min = rvec(rng, mag, special);
            if special {
                let max = rvec(rng, mag, special);
                Shape::Aabb(c2AABB { min, max })
            } else {
                let (dx, dy) = (rrad(rng, mag * 0.5, false), rrad(rng, mag * 0.5, false));
                Shape::Aabb(c2AABB { min, max: v(min.x + dx, min.y + dy) })
            }
        }
        _ => {
            let a = rvec(rng, mag, special);
            let b = rvec(rng, mag, special);
            Shape::Capsule(c2Capsule { a, b, r: rrad(rng, mag * 0.25, special) })
        }
    }
}

/// Result of one `c2GJK` call, everything the caller can observe.
#[derive(Clone, Copy, Debug)]
struct Out {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: i32,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
fn call(
    f: &libloading::Symbol<'_, FnGJK>,
    sa: &Shape,
    ax: Option<&c2x>,
    sb: &Shape,
    bx: Option<&c2x>,
    use_radius: i32,
    cache_in: Option<c2GJKCache>,
    want_a: bool,
    want_b: bool,
    want_iters: bool,
) -> Out {
    let mut a = poison_v(21);
    let mut b = poison_v(22);
    let mut iters = -999i32;
    let mut cache = cache_in.unwrap_or_default();
    let dist = unsafe {
        f(
            sa.ptr(),
            sa.ty(),
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            sb.ptr(),
            sb.ty(),
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            if want_a { &mut a } else { std::ptr::null_mut() },
            if want_b { &mut b } else { std::ptr::null_mut() },
            use_radius,
            if want_iters { &mut iters } else { std::ptr::null_mut() },
            if cache_in.is_some() { &mut cache } else { std::ptr::null_mut() },
        )
    };
    Out { dist, a, b, iters, cache }
}

#[track_caller]
fn cmp(ctx: &str, c: &Out, r: &Out) {
    eq_f32("c2GJK dist", ctx, c.dist, r.dist);
    eq("c2GJK outA", ctx, &c.a, &r.a);
    eq("c2GJK outB", ctx, &c.b, &r.b);
    eq_i32("c2GJK iterations", ctx, c.iters, r.iters);
    eq("c2GJK cache", ctx, &c.cache, &r.cache);
}

/// Generate a pair of shapes in one of the interesting geometric regimes.
fn regime_pair(rng: &mut Rng, ta: C2_TYPE, tb: C2_TYPE, regime: u32) -> (Shape, Shape) {
    match regime {
        // far apart
        0 => {
            let a = rand_shape(rng, ta, 5.0, false);
            let mut b = rand_shape(rng, tb, 5.0, false);
            shift(&mut b, v(200.0, 150.0));
            (a, b)
        }
        // near / touching, on a half-integer lattice so exact touches happen
        1 => {
            let a = lattice_shape(rng, ta, 4);
            let b = lattice_shape(rng, tb, 4);
            (a, b)
        }
        // overlapping
        2 => (rand_shape(rng, ta, 5.0, false), rand_shape(rng, tb, 5.0, false)),
        // coincident (deep overlap -> s.count == 3 -> `hit`)
        3 => {
            let a = rand_shape(rng, ta, 5.0, false);
            let mut b = rand_shape(rng, tb, 5.0, false);
            center_on(&mut b, centroid(&a));
            (a, b)
        }
        // degenerate shapes
        4 => (degenerate(rng, ta), degenerate(rng, tb)),
        // pathological floats
        _ => (rand_shape(rng, ta, 5.0, true), rand_shape(rng, tb, 5.0, true)),
    }
}

fn lattice_shape(rng: &mut Rng, ty: C2_TYPE, n: i32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: v(rng.f_half_lattice(n), rng.f_half_lattice(n)),
            r: (rng.below(4) as f32) * 0.5,
        }),
        C2_TYPE_AABB => {
            let min = v(rng.f_half_lattice(n), rng.f_half_lattice(n));
            Shape::Aabb(c2AABB {
                min,
                max: v(min.x + (rng.below(5) as f32) * 0.5, min.y + (rng.below(5) as f32) * 0.5),
            })
        }
        _ => Shape::Capsule(c2Capsule {
            a: v(rng.f_half_lattice(n), rng.f_half_lattice(n)),
            b: v(rng.f_half_lattice(n), rng.f_half_lattice(n)),
            r: (rng.below(4) as f32) * 0.5,
        }),
    }
}

fn degenerate(rng: &mut Rng, ty: C2_TYPE) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.vec_norm(5.0), r: 0.0 }),
        C2_TYPE_AABB => {
            let p = rng.vec_norm(5.0);
            Shape::Aabb(c2AABB { min: p, max: p })
        }
        _ => {
            let p = rng.vec_norm(5.0);
            Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 })
        }
    }
}

fn centroid(s: &Shape) -> c2v {
    match s {
        Shape::Circle(c) => c.p,
        Shape::Aabb(b) => v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5),
        Shape::Capsule(c) => v((c.a.x + c.b.x) * 0.5, (c.a.y + c.b.y) * 0.5),
    }
}

fn shift(s: &mut Shape, d: c2v) {
    match s {
        Shape::Circle(c) => c.p = v(c.p.x + d.x, c.p.y + d.y),
        Shape::Aabb(b) => {
            b.min = v(b.min.x + d.x, b.min.y + d.y);
            b.max = v(b.max.x + d.x, b.max.y + d.y);
        }
        Shape::Capsule(c) => {
            c.a = v(c.a.x + d.x, c.a.y + d.y);
            c.b = v(c.b.x + d.x, c.b.y + d.y);
        }
    }
}

fn center_on(s: &mut Shape, target: c2v) {
    let cur = centroid(s);
    shift(s, v(target.x - cur.x, target.y - cur.y));
}

// ---------------------------------------------------------------------------
// Rows 39-45: each type pair, no transforms, both use_radius values
// ---------------------------------------------------------------------------

fn pair_no_transform(seed: u64, ta: C2_TYPE, tb: C2_TYPE, label: &str) {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(seed);
    for regime in 0..6u32 {
        for &use_radius in [0i32, 1].iter() {
            for i in 0..N {
                let (sa, sb) = regime_pair(&mut rng, ta, tb, regime);
                let c = call(&cf, &sa, None, &sb, None, use_radius, None, true, true, true);
                let r = call(&rf, &sa, None, &sb, None, use_radius, None, true, true, true);
                cmp(
                    &format!("{label} regime={regime} use_radius={use_radius} i={i} A={sa:?} B={sb:?}"),
                    &c,
                    &r,
                );
            }
        }
    }
}

#[test]
fn row39_40_circle_circle() {
    pair_no_transform(3940, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE, "CIRCLE/CIRCLE");
}

#[test]
fn row41_circle_aabb() {
    pair_no_transform(41, C2_TYPE_CIRCLE, C2_TYPE_AABB, "CIRCLE/AABB");
    pair_no_transform(4101, C2_TYPE_AABB, C2_TYPE_CIRCLE, "AABB/CIRCLE");
}

#[test]
fn row42_circle_capsule() {
    pair_no_transform(42, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, "CIRCLE/CAPSULE");
    pair_no_transform(4201, C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, "CAPSULE/CIRCLE");
}

#[test]
fn row43_aabb_aabb() {
    pair_no_transform(43, C2_TYPE_AABB, C2_TYPE_AABB, "AABB/AABB");
}

#[test]
fn row44_aabb_capsule() {
    pair_no_transform(44, C2_TYPE_AABB, C2_TYPE_CAPSULE, "AABB/CAPSULE");
    pair_no_transform(4401, C2_TYPE_CAPSULE, C2_TYPE_AABB, "CAPSULE/AABB");
}

#[test]
fn row45_capsule_capsule() {
    pair_no_transform(45, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE, "CAPSULE/CAPSULE");
    // extra: parallel, crossing and collinear capsule configurations
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(4501);
    for i in 0..N * 4 {
        let o = rng.vec_norm(5.0);
        let d = rng.vec_norm(5.0);
        let (a, b) = match i % 4 {
            0 => (
                // parallel
                c2Capsule { a: o, b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                c2Capsule {
                    a: v(o.x - d.y, o.y + d.x),
                    b: v(o.x + d.x - d.y, o.y + d.y + d.x),
                    r: rng.f_pos(1.0),
                },
            ),
            1 => (
                // crossing
                c2Capsule { a: v(o.x - d.x, o.y - d.y), b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                c2Capsule { a: v(o.x - d.y, o.y + d.x), b: v(o.x + d.y, o.y - d.x), r: rng.f_pos(1.0) },
            ),
            2 => (
                // collinear
                c2Capsule { a: o, b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                c2Capsule {
                    a: v(o.x + 2.0 * d.x, o.y + 2.0 * d.y),
                    b: v(o.x + 3.0 * d.x, o.y + 3.0 * d.y),
                    r: rng.f_pos(1.0),
                },
            ),
            _ => (
                // identical
                c2Capsule { a: o, b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                c2Capsule { a: o, b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
            ),
        };
        let (sa, sb) = (Shape::Capsule(a), Shape::Capsule(b));
        for &use_radius in [0i32, 1].iter() {
            let c = call(&cf, &sa, None, &sb, None, use_radius, None, true, true, true);
            let r = call(&rf, &sa, None, &sb, None, use_radius, None, true, true, true);
            cmp(&format!("capsule geom i={i} ur={use_radius} A={a:?} B={b:?}"), &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 46-49: transforms
// ---------------------------------------------------------------------------

#[test]
fn row46_47_48_49_transforms() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(4649);
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for mode in 0..4u32 {
                for i in 0..N {
                    let (sa, sb) = regime_pair(&mut rng, ta, tb, i as u32 % 6);
                    let xa = rng.xform(20.0);
                    let xb = rng.xform(20.0);
                    let (oa, ob) = match mode {
                        0 => (Some(&xa), None),           // row 46
                        1 => (None, Some(&xb)),           // row 47
                        2 => (Some(&xa), Some(&xb)),      // row 48
                        _ => (Some(&xa), Some(&xb)),      // row 49 (rng.xform emits
                                                          // zero and non-unit rots)
                    };
                    for &use_radius in [0i32, 1].iter() {
                        let c = call(&cf, &sa, oa, &sb, ob, use_radius, None, true, true, true);
                        let r = call(&rf, &sa, oa, &sb, ob, use_radius, None, true, true, true);
                        cmp(
                            &format!(
                                "ta={ta} tb={tb} mode={mode} ur={use_radius} i={i} \
                                 A={sa:?} B={sb:?} xa={xa:?} xb={xb:?}"
                            ),
                            &c,
                            &r,
                        );
                    }
                }
            }
        }
    }
}

/// Row 49, explicitly: rotations that are not unit-length, zero, infinite or NaN.
#[test]
fn row49_pathological_rotations() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(49);
    let rots = [
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 2.0, s: 3.0 },
        c2r { c: -1.0, s: 0.0 },
        c2r { c: f32::INFINITY, s: 0.0 },
        c2r { c: f32::NAN, s: 1.0 },
        c2r { c: 1.0, s: f32::NAN },
        c2r { c: f32::from_bits(1), s: f32::from_bits(1) },
        c2r { c: FLT_MAX, s: FLT_MAX },
    ];
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for ra in rots.iter() {
                for rb in rots.iter() {
                    for i in 0..24 {
                        let (sa, sb) = regime_pair(&mut rng, ta, tb, i as u32 % 6);
                        let xa = c2x { p: rng.vec_norm(10.0), r: *ra };
                        let xb = c2x { p: rng.vec_norm(10.0), r: *rb };
                        for &use_radius in [0i32, 1].iter() {
                            let c = call(&cf, &sa, Some(&xa), &sb, Some(&xb), use_radius, None, true, true, true);
                            let r = call(&rf, &sa, Some(&xa), &sb, Some(&xb), use_radius, None, true, true, true);
                            cmp(
                                &format!("ta={ta} tb={tb} ra={ra:?} rb={rb:?} ur={use_radius} A={sa:?} B={sb:?}"),
                                &c,
                                &r,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 50-53: the GJK cache
// ---------------------------------------------------------------------------

#[test]
fn row50_cold_cache() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(50);
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..N {
                let (sa, sb) = regime_pair(&mut rng, ta, tb, i as u32 % 6);
                // count == 0 -> cache_was_good false, but the write-back still happens
                let cold = c2GJKCache { metric: rng.f_norm(10.0), count: 0, iA: [7, 7, 7], iB: [7, 7, 7], div: rng.f_norm(10.0) };
                for &use_radius in [0i32, 1].iter() {
                    let c = call(&cf, &sa, None, &sb, None, use_radius, Some(cold), true, true, true);
                    let r = call(&rf, &sa, None, &sb, None, use_radius, Some(cold), true, true, true);
                    cmp(&format!("cold ta={ta} tb={tb} i={i} ur={use_radius} A={sa:?} B={sb:?}"), &c, &r);
                }
            }
        }
    }
}

#[test]
fn row51_52_warm_cache_roundtrip() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(51);
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..N {
                let (sa, sb) = regime_pair(&mut rng, ta, tb, i as u32 % 6);
                let cold = c2GJKCache::default();
                // first call warms the cache independently on each side
                let c1 = call(&cf, &sa, None, &sb, None, 0, Some(cold), true, true, true);
                let r1 = call(&rf, &sa, None, &sb, None, 0, Some(cold), true, true, true);
                cmp(&format!("warm#1 ta={ta} tb={tb} i={i}"), &c1, &r1);

                // second call reuses each side's own cache
                let c2 = call(&cf, &sa, None, &sb, None, 0, Some(c1.cache), true, true, true);
                let r2 = call(&rf, &sa, None, &sb, None, 0, Some(r1.cache), true, true, true);
                cmp(&format!("warm#2 ta={ta} tb={tb} i={i} A={sa:?} B={sb:?}"), &c2, &r2);

                // row 52: move the shapes between the two calls
                let xa = rng.xform(20.0);
                let c3 = call(&cf, &sa, Some(&xa), &sb, None, 1, Some(c2.cache), true, true, true);
                let r3 = call(&rf, &sa, Some(&xa), &sb, None, 1, Some(r2.cache), true, true, true);
                cmp(&format!("warm#3-moved ta={ta} tb={tb} i={i} xa={xa:?}"), &c3, &r3);
            }
        }
    }
}

#[test]
fn row53_handbuilt_cache() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(53);
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for count in 1i32..=3 {
                for i in 0..N {
                    let (sa, sb) = regime_pair(&mut rng, ta, tb, i as u32 % 6);
                    // iA/iB must index *initialised* proxy vertices, otherwise the C
                    // library reads uninitialised stack (see probe_uninit.rs).
                    let (na, nb) = (sa.proxy_count() as u32, sb.proxy_count() as u32);
                    let cache = c2GJKCache {
                        metric: match i % 4 {
                            0 => 0.0,
                            1 => rng.f_norm(100.0),
                            2 => -1.0e9,
                            _ => f32::NAN,
                        },
                        count,
                        iA: [
                            rng.below(na) as i32,
                            rng.below(na) as i32,
                            rng.below(na) as i32,
                        ],
                        iB: [
                            rng.below(nb) as i32,
                            rng.below(nb) as i32,
                            rng.below(nb) as i32,
                        ],
                        div: match i % 5 {
                            0 => 1.0,
                            1 => 0.0,
                            2 => rng.f_norm(10.0),
                            3 => f32::NAN,
                            _ => 3.0,
                        },
                    };
                    for &use_radius in [0i32, 1].iter() {
                        let c = call(&cf, &sa, None, &sb, None, use_radius, Some(cache), true, true, true);
                        let r = call(&rf, &sa, None, &sb, None, use_radius, Some(cache), true, true, true);
                        cmp(
                            &format!("handbuilt ta={ta} tb={tb} count={count} ur={use_radius} cache={cache:?} A={sa:?} B={sb:?}"),
                            &c,
                            &r,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 54: all eight NULL out-param combinations
// ---------------------------------------------------------------------------

#[test]
fn row54_null_out_params() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(54);
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for mask in 0..8u32 {
                let (wa, wb, wi) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
                for i in 0..N / 2 {
                    let (sa, sb) = regime_pair(&mut rng, ta, tb, i as u32 % 6);
                    for &use_radius in [0i32, 1].iter() {
                        let c = call(&cf, &sa, None, &sb, None, use_radius, None, wa, wb, wi);
                        let r = call(&rf, &sa, None, &sb, None, use_radius, None, wa, wb, wi);
                        cmp(
                            &format!("nullmask={mask} ta={ta} tb={tb} ur={use_radius} A={sa:?} B={sb:?}"),
                            &c,
                            &r,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 55-56: pathological inputs, and deep overlap (`hit`)
// ---------------------------------------------------------------------------

#[test]
fn row55_pathological_inputs() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(55);
    let mut max_iters = 0;
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..N * 4 {
                let sa = rand_shape(&mut rng, ta, 5.0, true);
                let sb = rand_shape(&mut rng, tb, 5.0, true);
                for &use_radius in [0i32, 1].iter() {
                    let c = call(&cf, &sa, None, &sb, None, use_radius, None, true, true, true);
                    let r = call(&rf, &sa, None, &sb, None, use_radius, None, true, true, true);
                    cmp(&format!("path ta={ta} tb={tb} i={i} ur={use_radius} A={sa:?} B={sb:?}"), &c, &r);
                    max_iters = max_iters.max(c.iters);
                }
            }
        }
    }
    println!("row55 max iterations observed: {max_iters}");
}

#[test]
fn row56_deep_overlap_hit() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(56);
    let mut hits = 0u32;
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..N * 2 {
                // Both shapes centred on the same point and generously sized, so the
                // simplex reaches count == 3 and `hit` fires.
                let mut sa = rand_shape(&mut rng, ta, 8.0, false);
                let mut sb = rand_shape(&mut rng, tb, 8.0, false);
                let ctr = rng.vec_norm(3.0);
                center_on(&mut sa, ctr);
                center_on(&mut sb, ctr);
                for &use_radius in [0i32, 1].iter() {
                    let c = call(&cf, &sa, None, &sb, None, use_radius, None, true, true, true);
                    let r = call(&rf, &sa, None, &sb, None, use_radius, None, true, true, true);
                    cmp(&format!("deep ta={ta} tb={tb} i={i} ur={use_radius} A={sa:?} B={sb:?}"), &c, &r);
                    if c.dist == 0.0 && c.a == c.b {
                        hits += 1;
                    }
                }
            }
        }
    }
    println!("row56 zero-distance results: {hits}");
    assert!(hits > 0, "row 56 never produced a `hit` (dist == 0, a == b)");
}
