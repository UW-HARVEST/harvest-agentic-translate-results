//! Phase B — CONFIGS.md rows 36-71: `c2GJK`, the full option cross-product.
//!
//! `c2GJK` is the real workhorse: 11 parameters, 3x3 shape-type pairs, optional
//! transforms, an optional warm-start cache and a radius mode. Every call
//! compares the returned distance, both witness points, the iteration count AND
//! the whole cache struct, bit-for-bit.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// Shape plumbing
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Shape {
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
            Shape::Circle(c) => c as *const Circle as *const c_void,
            Shape::Aabb(a) => a as *const AABB as *const c_void,
            Shape::Capsule(p) => p as *const Capsule as *const c_void,
        }
    }
    /// Number of proxy verts the C code will produce for this shape.
    fn vert_count(&self) -> i32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

/// Build shape `kind` centred at `center`.
fn shape_at(kind: u32, center: V, size: f32, radius: f32, g: &mut Rng) -> Shape {
    match kind % 3 {
        0 => Shape::Circle(Circle { p: center, r: radius }),
        1 => Shape::Aabb(AABB {
            min: V::new(center.x - size, center.y - size),
            max: V::new(center.x + size, center.y + size),
        }),
        _ => {
            let ang = g.range(-3.2, 3.2);
            let (dx, dy) = (size * ang.cos(), size * ang.sin());
            Shape::Capsule(Capsule {
                a: V::new(center.x - dx, center.y - dy),
                b: V::new(center.x + dx, center.y + dy),
                r: radius,
            })
        }
    }
}

/// One `c2GJK` invocation's full option set.
#[derive(Copy, Clone, Debug)]
struct Opts {
    ax: Option<X>,
    bx: Option<X>,
    use_radius: i32,
    cache: Option<GJKCache>,
    want_out: bool,
    want_iter: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            ax: None,
            bx: None,
            use_radius: 1,
            cache: None,
            want_out: true,
            want_iter: true,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Outcome {
    dist_bits: u32,
    a: (u32, u32),
    b: (u32, u32),
    iters: i32,
    cache: Option<GJKCache>,
}

/// Invoke one library and capture every observable output.
unsafe fn invoke(f: &FnGJK, a: &Shape, b: &Shape, o: &Opts) -> Outcome {
    let poison = f32::from_bits(0xA5A5_A5A5);
    let mut outa = V::new(poison, poison);
    let mut outb = V::new(poison, poison);
    let mut iters: i32 = -12345;
    let mut cache = o.cache;

    let ax_ptr = o.ax.as_ref().map_or(std::ptr::null(), |x| x as *const X);
    let bx_ptr = o.bx.as_ref().map_or(std::ptr::null(), |x| x as *const X);
    let outa_ptr = if o.want_out { &mut outa as *mut V } else { std::ptr::null_mut() };
    let outb_ptr = if o.want_out { &mut outb as *mut V } else { std::ptr::null_mut() };
    let it_ptr = if o.want_iter { &mut iters as *mut i32 } else { std::ptr::null_mut() };
    let cache_ptr = cache.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut GJKCache);

    let dist = unsafe {
        f(
            a.ptr(),
            a.ty(),
            ax_ptr,
            b.ptr(),
            b.ty(),
            bx_ptr,
            outa_ptr,
            outb_ptr,
            o.use_radius,
            it_ptr,
            cache_ptr,
        )
    };

    Outcome {
        dist_bits: dist.to_bits(),
        a: outa.bits(),
        b: outb.bits(),
        iters,
        cache,
    }
}

impl Outcome {
    /// Bit-exact equality. Deliberately NOT `PartialEq`: the cache holds floats
    /// that can be NaN, and `NaN != NaN` would both hide real divergences and
    /// invent fake ones. Raw bytes are the only correct comparison here.
    fn identical(&self, o: &Outcome) -> bool {
        self.dist_bits == o.dist_bits
            && self.a == o.a
            && self.b == o.b
            && self.iters == o.iters
            && match (&self.cache, &o.cache) {
                (None, None) => true,
                (Some(x), Some(y)) => raw(x) == raw(y),
                _ => false,
            }
    }
}

struct Gjk<'a> {
    c: libloading::Symbol<'a, FnGJK>,
    r: libloading::Symbol<'a, FnGJK>,
}

impl<'a> Gjk<'a> {
    fn load(l: &'a Pair) -> Self {
        let (c, r) = l.get::<FnGJK>("c2GJK");
        Gjk { c, r }
    }

    /// Run both libraries and assert byte-identical observable output.
    #[track_caller]
    fn check(&self, ctx: &str, a: &Shape, b: &Shape, o: &Opts) -> Outcome {
        let co = unsafe { invoke(&self.c, a, b, o) };
        let ro = unsafe { invoke(&self.r, a, b, o) };
        if !co.identical(&ro) {
            panic!(
                "c2GJK MISMATCH\n  ctx : {ctx}\n  A   : {a:?}\n  B   : {b:?}\n  opts: {o:?}\n\
                 \n  C   : dist={:?} bits={:#010x} a={:#010x?} b={:#010x?} it={} cache={:?}\
                 \n  Rust: dist={:?} bits={:#010x} a={:#010x?} b={:#010x?} it={} cache={:?}",
                f32::from_bits(co.dist_bits), co.dist_bits, co.a, co.b, co.iters, co.cache,
                f32::from_bits(ro.dist_bits), ro.dist_bits, ro.a, ro.b, ro.iters, ro.cache,
            );
        }
        co
    }
}

/// A warm cache whose indices are guaranteed in range for both proxies (so the
/// C stays inside initialised proxy verts and the comparison is meaningful).
fn warm_cache(a: &Shape, b: &Shape, count: i32, metric: f32, div: f32, g: &mut Rng) -> GJKCache {
    let (na, nb) = (a.vert_count(), b.vert_count());
    let mut c = GJKCache {
        metric,
        count,
        iA: [0; 3],
        iB: [0; 3],
        div,
    };
    for k in 0..3 {
        c.iA[k] = (g.next_u32() as i32).rem_euclid(na);
        c.iB[k] = (g.next_u32() as i32).rem_euclid(nb);
    }
    c
}

/// Geometry relation classes (CONFIGS rows 61-67).
fn relation_offset(rel: u32, size: f32, radius: f32, g: &mut Rng) -> V {
    let span = 2.0 * size + 2.0 * radius;
    let ang = g.range(-3.2, 3.2);
    let d = match rel {
        0 => g.range(50.0, 200.0),   // far apart
        1 => span * g.range(0.95, 1.05), // nearly touching (straddles rA+rB)
        2 => span,                   // exactly touching
        3 => span * g.range(0.2, 0.8), // overlapping
        4 => 0.0,                    // coincident centres / one inside other
        _ => g.range(0.0, span * 2.0),
    };
    V::new(d * ang.cos(), d * ang.sin())
}

// ---------------------------------------------------------------------------
// Rows 36-44: the nine shape-type pairs, each under the full option matrix
// ---------------------------------------------------------------------------

fn type_pair_sweep(name: &str, ka: u32, kb: u32, seed: u64) {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(seed);

    for i in 0..4000 {
        let size_a = g.range(0.5, 20.0);
        let size_b = g.range(0.5, 20.0);
        let rad_a = g.radius();
        let rad_b = g.radius();
        let rel = (i % 6) as u32;
        let ca = V::new(0.0, 0.0);
        let off = relation_offset(rel, size_a.max(size_b), rad_a.max(rad_b), &mut g);
        let a = shape_at(ka, ca, size_a, rad_a, &mut g);
        let b = shape_at(kb, off, size_b, rad_b, &mut g);

        // option matrix: transforms x use_radius x cache
        let xmode_a = (i / 6) % 4;
        let xmode_b = (i / 24) % 4;
        let ax = if xmode_a == 0 { None } else { Some(g.xform(xmode_a as u32)) };
        let bx = if xmode_b == 0 { None } else { Some(g.xform(xmode_b as u32)) };

        for &use_radius in &[0i32, 1] {
            for cache_mode in 0..3u32 {
                let cache = match cache_mode {
                    0 => None,
                    1 => Some(GJKCache::default()), // cold: count == 0
                    _ => {
                        let cnt = 1 + (i % 3) as i32;
                        Some(warm_cache(&a, &b, cnt, g.range(-5.0, 50.0), g.range(0.5, 5.0), &mut g))
                    }
                };
                let o = Opts {
                    ax,
                    bx,
                    use_radius,
                    cache,
                    want_out: true,
                    want_iter: true,
                };
                gjk.check(
                    &format!("{name} i={i} rel={rel} ur={use_radius} cm={cache_mode}"),
                    &a,
                    &b,
                    &o,
                );
            }
        }
    }
}

#[test]
fn row36_circle_circle() {
    type_pair_sweep("circle/circle", 0, 0, 0x3601);
}
#[test]
fn row37_circle_aabb() {
    type_pair_sweep("circle/aabb", 0, 1, 0x3701);
}
#[test]
fn row38_circle_capsule() {
    type_pair_sweep("circle/capsule", 0, 2, 0x3801);
}
#[test]
fn row39_aabb_circle() {
    type_pair_sweep("aabb/circle", 1, 0, 0x3901);
}
#[test]
fn row40_aabb_aabb() {
    type_pair_sweep("aabb/aabb", 1, 1, 0x4001);
}
#[test]
fn row41_aabb_capsule() {
    type_pair_sweep("aabb/capsule", 1, 2, 0x4101);
}
#[test]
fn row42_capsule_circle() {
    type_pair_sweep("capsule/circle", 2, 0, 0x4201);
}
#[test]
fn row43_capsule_aabb() {
    type_pair_sweep("capsule/aabb", 2, 1, 0x4301);
}
#[test]
fn row44_capsule_capsule() {
    type_pair_sweep("capsule/capsule", 2, 2, 0x4401);
}

// ---------------------------------------------------------------------------
// Rows 45-50: transform options
// ---------------------------------------------------------------------------

/// Rows 45/46 — NULL transforms vs explicit identity must agree, and both libs
/// must agree with each other.
#[test]
fn row45_46_null_vs_explicit_identity() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x4501);
    for i in 0..3000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), g.radius(), &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 3.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), g.radius(), &mut g);

                let o_null = Opts::default();
                let o_id = Opts {
                    ax: Some(X::IDENTITY),
                    bx: Some(X::IDENTITY),
                    ..Opts::default()
                };
                let r_null = gjk.check(&format!("null-x i={i} {ka}/{kb}"), &a, &b, &o_null);
                let r_id = gjk.check(&format!("id-x i={i} {ka}/{kb}"), &a, &b, &o_id);
                // The C substitutes c2xIdentity() for a NULL transform, so the
                // two must be indistinguishable.
                assert!(
                    r_null.identical(&r_id),
                    "NULL transform must equal explicit identity: i={i} {ka}/{kb} a={a:?} b={b:?}\n  null={r_null:?}\n  id  ={r_id:?}"
                );
            }
        }
    }
}

/// Rows 47/48/49/50 — translation only, rotation only, both, and non-unit `c2r`.
#[test]
fn row47_50_transform_modes() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x4701);
    for i in 0..4000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), g.radius(), &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 3.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), g.radius(), &mut g);

                // row 47: pure translation on A only
                let o = Opts { ax: Some(g.xform(1)), bx: None, ..Opts::default() };
                gjk.check(&format!("xlate-A i={i} {ka}/{kb}"), &a, &b, &o);

                // row 48: pure rotation on both
                let o = Opts { ax: Some(g.xform(2)), bx: Some(g.xform(2)), ..Opts::default() };
                gjk.check(&format!("rot-both i={i} {ka}/{kb}"), &a, &b, &o);

                // row 49: rotation + translation on both
                let o = Opts { ax: Some(g.xform(3)), bx: Some(g.xform(3)), ..Opts::default() };
                gjk.check(&format!("rot+xlate i={i} {ka}/{kb}"), &a, &b, &o);

                // row 50: deliberately non-unit / degenerate c2r
                let nonunit = X {
                    p: g.v_coord(),
                    r: R { c: g.range(-2.0, 2.0), s: g.range(-2.0, 2.0) },
                };
                let o = Opts { ax: Some(nonunit), bx: Some(nonunit), ..Opts::default() };
                gjk.check(&format!("nonunit-rot i={i} {ka}/{kb}"), &a, &b, &o);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 51-60: use_radius, cache and iterations options
// ---------------------------------------------------------------------------

/// Rows 51/52 — `use_radius` 1 vs 0.
#[test]
fn row51_52_use_radius() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x5101);
    for i in 0..6000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), g.radius(), &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 3.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), g.radius(), &mut g);
                for &ur in &[1i32, 0] {
                    let o = Opts { use_radius: ur, ..Opts::default() };
                    gjk.check(&format!("ur={ur} i={i} {ka}/{kb}"), &a, &b, &o);
                }
            }
        }
    }
}

/// Rows 53/54/60 — cache NULL, cache cold (count==0, contents compared), and
/// the `iterations` out-parameter.
#[test]
fn row53_54_60_cache_none_cold_iters() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x5301);
    for i in 0..6000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), g.radius(), &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 3.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), g.radius(), &mut g);

                // row 53: no cache at all
                let o = Opts { cache: None, ..Opts::default() };
                let no_cache = gjk.check(&format!("nocache i={i}"), &a, &b, &o);

                // row 54: cold cache — the written-back cache is compared too
                let o = Opts { cache: Some(GJKCache::default()), ..Opts::default() };
                let cold = gjk.check(&format!("coldcache i={i}"), &a, &b, &o);
                // count==0 means cache_was_good==0, so the result must equal the
                // no-cache result exactly.
                assert_eq!(
                    no_cache.dist_bits, cold.dist_bits,
                    "cold cache must not change the result: i={i} a={a:?} b={b:?}"
                );
                let wc = cold.cache.expect("cache written back");
                assert!(
                    wc.count >= 1 && wc.count <= 3,
                    "cache count out of range after cold start: {wc:?}"
                );

                // row 60: iterations must be reported and bounded by the C's cap
                assert!(
                    cold.iters >= 0 && cold.iters <= 20,
                    "iterations out of range: {} (i={i})",
                    cold.iters
                );
            }
        }
    }
}

/// Rows 55/56 — real warm-start usage: call twice with the SAME cache object,
/// with and without moving the shapes in between.
#[test]
fn row55_56_cache_warm_reuse() {
    let l = libs();
    let gjkc = Gjk::load(l);
    let mut g = Rng::new(0x5501);

    for i in 0..4000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), g.radius(), &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 3.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), g.radius(), &mut g);

                // --- first call, cold cache; keep BOTH libs' caches separately
                let mut cc = GJKCache::default();
                let o1 = Opts { cache: Some(cc), ..Opts::default() };
                let r1 = gjkc.check(&format!("warm1 i={i} {ka}/{kb}"), &a, &b, &o1);
                cc = r1.cache.unwrap(); // C and Rust caches proved identical by `check`

                // --- row 55: second call, same shapes, cache carried over
                let o2 = Opts { cache: Some(cc), ..Opts::default() };
                let r2 = gjkc.check(&format!("warm2-same i={i} {ka}/{kb}"), &a, &b, &o2);
                cc = r2.cache.unwrap();

                // --- row 56: third call with the shapes MOVED, cache carried
                let off2 = relation_offset(((i + 3) % 6) as u32, 8.0, 3.0, &mut g);
                let b2 = shape_at(kb, off2, g.range(0.5, 10.0), g.radius(), &mut g);
                let o3 = Opts { cache: Some(cc), ..Opts::default() };
                let r3 = gjkc.check(&format!("warm3-moved i={i} {ka}/{kb}"), &a, &b2, &o3);

                // and a fourth, to exercise a cache written by a warm read
                let o4 = Opts { cache: r3.cache, ..Opts::default() };
                gjkc.check(&format!("warm4 i={i} {ka}/{kb}"), &a, &b2, &o4);
            }
        }
    }
}

/// Rows 57/58/59 — hand-built warm caches with count 1, 2 and 3.
#[test]
fn row57_59_cache_handbuilt_counts() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x5701);
    for i in 0..4000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), g.radius(), &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 3.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), g.radius(), &mut g);
                for count in 1..=3i32 {
                    let metric = match i % 5 {
                        0 => 0.0,
                        1 => g.range(-1e9, -1e7),
                        2 => g.range(0.0, 1e9),
                        3 => f32::NAN,
                        _ => g.range(-10.0, 10.0),
                    };
                    let div = match i % 4 {
                        0 => 1.0,
                        1 => 0.0,
                        2 => g.range(0.01, 10.0),
                        _ => g.range(-10.0, 10.0),
                    };
                    let cache = warm_cache(&a, &b, count, metric, div, &mut g);
                    let o = Opts { cache: Some(cache), ..Opts::default() };
                    gjk.check(
                        &format!("handcache count={count} i={i} {ka}/{kb} metric={metric:?} div={div:?}"),
                        &a,
                        &b,
                        &o,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 61-70: input shapes / geometric relations
// ---------------------------------------------------------------------------

/// Rows 61-67 — each geometry relation class, all nine type pairs.
#[test]
fn row61_67_geometry_relations() {
    let l = libs();
    let gjk = Gjk::load(l);
    let names = [
        "far apart",
        "nearly touching",
        "exactly touching",
        "overlapping",
        "coincident",
        "random span",
    ];
    for rel in 0..6u32 {
        let mut g = Rng::new(0x6100 + rel as u64);
        for i in 0..3000 {
            for ka in 0..3u32 {
                for kb in 0..3u32 {
                    let size = g.range(0.5, 15.0);
                    let rad_a = g.range(0.0, 5.0);
                    let rad_b = g.range(0.0, 5.0);
                    let a = shape_at(ka, V::new(0.0, 0.0), size, rad_a, &mut g);
                    let off = relation_offset(rel, size, rad_a.max(rad_b), &mut g);
                    let b = shape_at(kb, off, size, rad_b, &mut g);
                    for &ur in &[0i32, 1] {
                        let o = Opts { use_radius: ur, ..Opts::default() };
                        gjk.check(
                            &format!("rel={} ({}) i={i} {ka}/{kb} ur={ur}", rel, names[rel as usize]),
                            &a,
                            &b,
                            &o,
                        );
                    }
                }
            }
        }
    }
}

/// Row 65 — one shape fully inside the other.
#[test]
fn row65_containment() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x6501);
    for i in 0..4000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let big = g.range(20.0, 60.0);
                let small = g.range(0.1, 2.0);
                let a = shape_at(ka, V::new(0.0, 0.0), big, g.range(0.0, 3.0), &mut g);
                let inner = V::new(g.range(-2.0, 2.0), g.range(-2.0, 2.0));
                let b = shape_at(kb, inner, small, g.range(0.0, 0.5), &mut g);
                for &ur in &[0i32, 1] {
                    let o = Opts { use_radius: ur, ..Opts::default() };
                    gjk.check(&format!("contained i={i} {ka}/{kb} ur={ur}"), &a, &b, &o);
                }
            }
        }
    }
}

/// Row 67 — grid-snapped coordinates: maximal support-function ties and exact
/// touching, the configurations most likely to expose tie-breaking differences.
#[test]
fn row67_grid_snapped() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x6701);
    for i in 0..20_000 {
        let a = match (i % 3) as u32 {
            0 => Shape::Circle(Circle { p: g.v_grid(), r: g.grid().abs() }),
            1 => Shape::Aabb(AABB { min: g.v_grid(), max: g.v_grid() }),
            _ => Shape::Capsule(Capsule { a: g.v_grid(), b: g.v_grid(), r: g.grid().abs() }),
        };
        let b = match ((i / 3) % 3) as u32 {
            0 => Shape::Circle(Circle { p: g.v_grid(), r: g.grid().abs() }),
            1 => Shape::Aabb(AABB { min: g.v_grid(), max: g.v_grid() }),
            _ => Shape::Capsule(Capsule { a: g.v_grid(), b: g.v_grid(), r: g.grid().abs() }),
        };
        for &ur in &[0i32, 1] {
            for cache in [None, Some(GJKCache::default())] {
                let o = Opts { use_radius: ur, cache, ..Opts::default() };
                gjk.check(&format!("grid i={i} ur={ur}"), &a, &b, &o);
            }
        }
    }
}

/// Rows 68/69/70 — huge coordinates, subnormal coordinates, zero radii.
#[test]
fn row68_70_extreme_magnitudes() {
    let l = libs();
    let gjk = Gjk::load(l);

    // row 68 — huge
    let mut g = Rng::new(0x6801);
    for i in 0..4000 {
        let s = 1e30f32;
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(g.range(-s, s), g.range(-s, s)), s * 0.1, s * 0.01, &mut g);
                let b = shape_at(kb, V::new(g.range(-s, s), g.range(-s, s)), s * 0.1, s * 0.01, &mut g);
                for &ur in &[0i32, 1] {
                    let o = Opts { use_radius: ur, ..Opts::default() };
                    gjk.check(&format!("huge i={i} {ka}/{kb} ur={ur}"), &a, &b, &o);
                }
            }
        }
    }

    // row 69 — subnormal / tiny
    let mut g = Rng::new(0x6901);
    for i in 0..4000 {
        let s = 1e-40f32;
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(g.range(-s, s), g.range(-s, s)), s, s, &mut g);
                let b = shape_at(kb, V::new(g.range(-s, s), g.range(-s, s)), s, s, &mut g);
                for &ur in &[0i32, 1] {
                    let o = Opts { use_radius: ur, ..Opts::default() };
                    gjk.check(&format!("tiny i={i} {ka}/{kb} ur={ur}"), &a, &b, &o);
                }
            }
        }
    }

    // row 70 — both radii exactly zero (rA+rB == 0 -> L480 falls to midpoint)
    let mut g = Rng::new(0x7001);
    for i in 0..4000 {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let a = shape_at(ka, V::new(0.0, 0.0), g.range(0.5, 10.0), 0.0, &mut g);
                let off = relation_offset((i % 6) as u32, 8.0, 0.0, &mut g);
                let b = shape_at(kb, off, g.range(0.5, 10.0), 0.0, &mut g);
                for &ur in &[0i32, 1] {
                    let o = Opts { use_radius: ur, ..Opts::default() };
                    gjk.check(&format!("zerorad i={i} {ka}/{kb} ur={ur}"), &a, &b, &o);
                }
            }
        }
    }
}

/// Row 71 — the full cross-product sweep:
/// 9 type pairs x {use_radius 0,1} x {cache NULL, cold, warm} x
/// {4 transform modes} x {out/iter present or NULL} x random geometry.
#[test]
fn row71_full_cross_product() {
    let l = libs();
    let gjk = Gjk::load(l);
    let mut g = Rng::new(0x7101);

    let mut combos = 0usize;
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for &ur in &[0i32, 1] {
                for cache_mode in 0..3u32 {
                    for xmode_a in 0..4u32 {
                        for xmode_b in 0..4u32 {
                            combos += 1;
                            for i in 0..80 {
                                let size = g.range(0.2, 20.0);
                                let rad_a = g.radius();
                                let rad_b = g.radius();
                                let a = shape_at(ka, V::new(0.0, 0.0), size, rad_a, &mut g);
                                let off = relation_offset((i % 6) as u32, size, rad_a.max(rad_b), &mut g);
                                let b = shape_at(kb, off, size, rad_b, &mut g);

                                let cache = match cache_mode {
                                    0 => None,
                                    1 => Some(GJKCache::default()),
                                    _ => Some(warm_cache(&a, &b, 1 + (i % 3) as i32,
                                        g.range(-50.0, 50.0),
                                        g.range(0.1, 5.0), &mut g)),
                                };
                                let o = Opts {
                                    ax: if xmode_a == 0 { None } else { Some(g.xform(xmode_a)) },
                                    bx: if xmode_b == 0 { None } else { Some(g.xform(xmode_b)) },
                                    use_radius: ur,
                                    cache,
                                    want_out: i % 7 != 0,
                                    want_iter: i % 5 != 0,
                                };
                                gjk.check(
                                    &format!(
                                        "sweep {ka}/{kb} ur={ur} cm={cache_mode} xa={xmode_a} xb={xmode_b} i={i}"
                                    ),
                                    &a,
                                    &b,
                                    &o,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(combos, 3 * 3 * 2 * 3 * 4 * 4, "unexpected combo count");
    eprintln!("row71: {combos} distinct option combinations x 80 random geometries");
}
