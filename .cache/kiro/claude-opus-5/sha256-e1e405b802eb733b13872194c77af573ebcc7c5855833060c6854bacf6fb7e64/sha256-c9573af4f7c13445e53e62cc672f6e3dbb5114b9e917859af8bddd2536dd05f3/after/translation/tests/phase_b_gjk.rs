//! Phase B, Tier 4: `CONFIGS.md` rows 44-71.
//!
//! `c2GJK` is the lowest-level *composed* entry point: it drives `c2MakeProxy`,
//! `c22`, `c23`, `c2D`, `c2L`, `c2Support`, `c2Witness` and
//! `c2GJKSimplexMetric` as a pipeline. Bugs in the composition are invisible to
//! the per-function tests in `phase_b_simplex.rs`, so every option axis is
//! driven here directly rather than through the `c2Collided` / `aabb` wrappers.
//!
//! Each call compares, bit for bit: the returned `dist`, both witness points,
//! the iteration count, and the 36-byte written-back `c2GJKCache`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Holds any of the three shapes at a stable address for `*const c_void`.
#[repr(C)]
#[derive(Copy, Clone)]
union ShapeU {
    circle: c2Circle,
    aabb: c2AABB,
    capsule: c2Capsule,
}

#[derive(Copy, Clone)]
struct Shape {
    ty: c_int,
    u: ShapeU,
    desc: [f32; 5],
}

impl Shape {
    fn circle(c: c2Circle) -> Shape {
        Shape { ty: C2_TYPE_CIRCLE, u: ShapeU { circle: c }, desc: [c.p.x, c.p.y, c.r, 0.0, 0.0] }
    }
    fn aabb(b: c2AABB) -> Shape {
        Shape {
            ty: C2_TYPE_AABB,
            u: ShapeU { aabb: b },
            desc: [b.min.x, b.min.y, b.max.x, b.max.y, 0.0],
        }
    }
    fn capsule(k: c2Capsule) -> Shape {
        Shape {
            ty: C2_TYPE_CAPSULE,
            u: ShapeU { capsule: k },
            desc: [k.a.x, k.a.y, k.b.x, k.b.y, k.r],
        }
    }
    fn ptr(&self) -> *const c_void {
        &raw const self.u as *const c_void
    }
    fn name(&self) -> &'static str {
        ["CIRCLE", "AABB", "CAPSULE"][self.ty as usize]
    }
}

fn show_shape(s: &Shape) -> String {
    format!("{}{:?}", s.name(), s.desc)
}

/// A full `c2GJK` invocation, with everything the C can write back compared.
#[derive(Clone, Copy)]
struct Call {
    ax: Option<c2x>,
    bx: Option<c2x>,
    use_radius: c_int,
    out_a: bool,
    out_b: bool,
    iters: bool,
    cache: Option<c2GJKCache>,
}

impl Default for Call {
    fn default() -> Call {
        Call {
            ax: None,
            bx: None,
            use_radius: 0,
            out_a: true,
            out_b: true,
            iters: true,
            cache: None,
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

const POISON_V: c2v = c2v { x: -1.0e-11, y: 7.5e13 };
const POISON_I: c_int = -123456789;

fn invoke(f: &libloading::Symbol<FnGJK>, A: &Shape, B: &Shape, call: &Call) -> Out {
    let mut a = POISON_V;
    let mut b = POISON_V;
    let mut it: c_int = POISON_I;
    let mut cache = call.cache.unwrap_or_default();
    let ax = call.ax;
    let bx = call.bx;
    let dist = unsafe {
        f(
            A.ptr(),
            A.ty,
            match &ax {
                Some(x) => x as *const c2x,
                None => std::ptr::null(),
            },
            B.ptr(),
            B.ty,
            match &bx {
                Some(x) => x as *const c2x,
                None => std::ptr::null(),
            },
            if call.out_a { &raw mut a } else { std::ptr::null_mut() },
            if call.out_b { &raw mut b } else { std::ptr::null_mut() },
            call.use_radius,
            if call.iters { &raw mut it } else { std::ptr::null_mut() },
            if call.cache.is_some() { &raw mut cache } else { std::ptr::null_mut() },
        )
    };
    Out { dist, a, b, iters: it, cache }
}

fn describe(A: &Shape, B: &Shape, call: &Call) -> String {
    let x = |o: &Option<c2x>| match o {
        None => "NULL".to_string(),
        Some(t) => format!("(p={} r=({},{}))", show_v(t.p), show_f32(t.r.c), show_f32(t.r.s)),
    };
    format!(
        "A={} B={} ax={} bx={} use_radius={} outA={} outB={} iters={} cache={}",
        show_shape(A),
        show_shape(B),
        x(&call.ax),
        x(&call.bx),
        call.use_radius,
        call.out_a,
        call.out_b,
        call.iters,
        match &call.cache {
            None => "NULL".to_string(),
            Some(c) => show_cache(c),
        }
    )
}

struct Gjk<'a> {
    c: libloading::Symbol<'a, FnGJK>,
    rs: libloading::Symbol<'a, FnGJK>,
}

impl<'a> Gjk<'a> {
    fn new(l: &'a Pair) -> Gjk<'a> {
        Gjk { c: l.c.sym("c2GJK"), rs: l.rs.sym("c2GJK") }
    }

    /// Run both libraries and compare every observable. Returns the C's cache so
    /// a caller can chain a warm-cache call.
    fn check(&self, rep: &mut Report, A: &Shape, B: &Shape, call: &Call) -> c2GJKCache {
        let oc = invoke(&self.c, A, B, call);
        let or = invoke(&self.rs, A, B, call);
        rep.check(same_f32(oc.dist, or.dist), || {
            format!(
                "c2GJK dist: C={} Rust={}\n  {}",
                show_f32(oc.dist),
                show_f32(or.dist),
                describe(A, B, call)
            )
        });
        rep.check(same_v(oc.a, or.a), || {
            format!(
                "c2GJK outA: C={} Rust={}\n  {}",
                show_v(oc.a),
                show_v(or.a),
                describe(A, B, call)
            )
        });
        rep.check(same_v(oc.b, or.b), || {
            format!(
                "c2GJK outB: C={} Rust={}\n  {}",
                show_v(oc.b),
                show_v(or.b),
                describe(A, B, call)
            )
        });
        rep.check(oc.iters == or.iters, || {
            format!(
                "c2GJK iterations: C={} Rust={}\n  {}",
                oc.iters,
                or.iters,
                describe(A, B, call)
            )
        });
        rep.check(same_cache(&oc.cache, &or.cache), || {
            format!(
                "c2GJK cache write-back:\n  C:    {}\n  Rust: {}\n  {}",
                show_cache(&oc.cache),
                show_cache(&or.cache),
                describe(A, B, call)
            )
        });
        // The iteration cap must be respected on both sides.
        if call.iters {
            rep.check(oc.iters >= 0 && oc.iters <= 20, || {
                format!("c2GJK iterations out of [0,20]: C={}\n  {}", oc.iters, describe(A, B, call))
            });
        }
        oc.cache
    }
}

// ---------------------------------------------------------------------------
// Shape generators
// ---------------------------------------------------------------------------

fn gen_shape(g: &mut Rng, ty: c_int, centre: c2v, scale: f32) -> Shape {
    let jitter = |g: &mut Rng| c2v { x: centre.x + g.sym(scale), y: centre.y + g.sym(scale) };
    match ty {
        C2_TYPE_CIRCLE => Shape::circle(c2Circle { p: jitter(g), r: g.radius() }),
        C2_TYPE_AABB => {
            let p = jitter(g);
            let q = jitter(g);
            match g.below(6) {
                0 => Shape::aabb(c2AABB { min: p, max: p }),  // degenerate point
                1 => Shape::aabb(c2AABB { min: p, max: c2v { x: q.x, y: p.y } }), // flat
                _ => Shape::aabb(c2AABB {
                    min: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                    max: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                }),
            }
        }
        _ => {
            let a = jitter(g);
            let b = if g.below(6) == 0 { a } else { jitter(g) };
            Shape::capsule(c2Capsule { a, b, r: g.radius() })
        }
    }
}

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Rows 44-53: one test body per `(typeA, typeB)` pair, both `use_radius`
/// settings, randomized shapes over separated / touching / overlapping regimes.
fn type_pair_sweep(name: &str, seed: u64, ta: c_int, tb: c_int, iters: usize) {
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(seed);
    let mut rep = Report::new();
    for i in 0..iters {
        // Sweep the separation regime: deeply overlapping through far apart.
        let sep = match i % 5 {
            0 => 0.0,   // coincident -> hit path
            1 => 5.0,   // overlapping
            2 => 25.0,  // touching-ish
            3 => 90.0,  // separated
            _ => g.unit() * 300.0,
        };
        let dir = g.unit() * std::f32::consts::TAU;
        let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 20.0);
        let B = gen_shape(
            &mut g,
            tb,
            c2v { x: sep * dir.cos(), y: sep * dir.sin() },
            20.0,
        );
        for use_radius in [0, 1] {
            gjk.check(&mut rep, &A, &B, &Call { use_radius, ..Default::default() });
        }
    }
    rep.finish(name);
}

#[test]
fn row44_row45_circle_circle() {
    type_pair_sweep("row44_row45_circle_circle", 0x2c01, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE, 1500);
}

#[test]
fn row46_circle_aabb() {
    type_pair_sweep("row46_circle_aabb", 0x2c02, C2_TYPE_CIRCLE, C2_TYPE_AABB, 1500);
}

#[test]
fn row47_circle_capsule() {
    type_pair_sweep("row47_circle_capsule", 0x2c03, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, 1500);
}

#[test]
fn row48_aabb_circle() {
    type_pair_sweep("row48_aabb_circle", 0x2c04, C2_TYPE_AABB, C2_TYPE_CIRCLE, 1500);
}

#[test]
fn row49_aabb_aabb() {
    type_pair_sweep("row49_aabb_aabb", 0x2c05, C2_TYPE_AABB, C2_TYPE_AABB, 2500);
}

#[test]
fn row50_aabb_capsule() {
    type_pair_sweep("row50_aabb_capsule", 0x2c06, C2_TYPE_AABB, C2_TYPE_CAPSULE, 1500);
}

#[test]
fn row51_capsule_circle() {
    type_pair_sweep("row51_capsule_circle", 0x2c07, C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, 1500);
}

#[test]
fn row52_capsule_aabb() {
    type_pair_sweep("row52_capsule_aabb", 0x2c08, C2_TYPE_CAPSULE, C2_TYPE_AABB, 1500);
}

#[test]
fn row53_capsule_capsule() {
    type_pair_sweep("row53_capsule_capsule", 0x2c09, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE, 2000);
}

// ---------------------------------------------------------------------------
// Rows 54-59 — the transform axes, across all 9 type pairs
// ---------------------------------------------------------------------------

/// `xmode`: 0 NULL, 1 identity, 2 translation only, 3 rotation only,
/// 4 rotation+translation, 5 arbitrary non-normalized `c2r`.
fn make_xform(g: &mut Rng, mode: u32) -> Option<c2x> {
    let ident = c2r { c: 1.0, s: 0.0 };
    match mode {
        0 => None,
        1 => Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: ident }),
        2 => Some(c2x { p: g.finite_v(), r: ident }),
        3 => {
            let a = g.unit() * std::f32::consts::TAU;
            Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: a.cos(), s: a.sin() } })
        }
        4 => {
            let a = g.unit() * std::f32::consts::TAU;
            Some(c2x { p: g.finite_v(), r: c2r { c: a.cos(), s: a.sin() } })
        }
        _ => Some(c2x { p: g.finite_v(), r: c2r { c: g.sym(3.0), s: g.sym(3.0) } }),
    }
}

fn transform_sweep(name: &str, seed: u64, amode: u32, bmode: u32, use_radius: c_int, iters: usize) {
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(seed);
    let mut rep = Report::new();
    for ta in TYPES {
        for tb in TYPES {
            for i in 0..iters {
                let sep = [0.0, 8.0, 40.0, 150.0][i % 4];
                let dir = g.unit() * std::f32::consts::TAU;
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 20.0);
                let B = gen_shape(
                    &mut g,
                    tb,
                    c2v { x: sep * dir.cos(), y: sep * dir.sin() },
                    20.0,
                );
                let call = Call {
                    ax: make_xform(&mut g, amode),
                    bx: make_xform(&mut g, bmode),
                    use_radius,
                    ..Default::default()
                };
                gjk.check(&mut rep, &A, &B, &call);
            }
        }
    }
    rep.finish(name);
}

#[test]
fn row54_ax_identity_bx_null() {
    transform_sweep("row54_ax_identity_bx_null", 0x3401, 1, 0, 0, 220);
}

#[test]
fn row55_ax_null_bx_identity() {
    transform_sweep("row55_ax_null_bx_identity", 0x3402, 0, 1, 1, 220);
}

#[test]
fn row56_both_translation() {
    transform_sweep("row56_both_translation", 0x3403, 2, 2, 0, 220);
    transform_sweep("row56_both_translation_radius", 0x3413, 2, 2, 1, 220);
}

#[test]
fn row57_both_rotation() {
    transform_sweep("row57_both_rotation", 0x3404, 3, 3, 0, 220);
    transform_sweep("row57_both_rotation_radius", 0x3414, 3, 3, 1, 220);
}

#[test]
fn row58_both_rotation_translation() {
    transform_sweep("row58_both_rotation_translation", 0x3405, 4, 4, 1, 260);
    transform_sweep("row58_both_rotation_translation_norad", 0x3415, 4, 4, 0, 260);
}

#[test]
fn row59_non_normalized_rotation() {
    transform_sweep("row59_non_normalized_rotation", 0x3406, 5, 5, 0, 220);
    transform_sweep("row59_non_normalized_rotation_radius", 0x3416, 5, 5, 1, 220);
}

// ---------------------------------------------------------------------------
// Rows 60-62 — the three terminal branches of the radius stage
// ---------------------------------------------------------------------------

#[test]
fn row60_hit_path() {
    // Deeply overlapping shapes: `c23` returns count == 3, so `hit = 1`,
    // `a = b` and `dist = 0` regardless of `use_radius`.
    //
    // `count` is only observable through the written-back cache, so each case is
    // run twice: once in the plain row-60 configuration (cache = NULL) and once
    // with a cold cache purely to read back `s.count` and prove the hit path was
    // really reached.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x3c);
    let mut rep = Report::new();
    let mut hits = 0usize;
    for ta in TYPES {
        for tb in TYPES {
            for _ in 0..400 {
                // Both shapes centred on the same point and large -> overlap.
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 3.0);
                let B = gen_shape(&mut g, tb, c2v { x: 0.0, y: 0.0 }, 3.0);
                for use_radius in [0, 1] {
                    let call = Call { use_radius, ..Default::default() };
                    gjk.check(&mut rep, &A, &B, &call);
                    let probe =
                        Call { cache: Some(c2GJKCache::default()), ..call };
                    let cache = gjk.check(&mut rep, &A, &B, &probe);
                    if cache.count == 3 {
                        hits += 1;
                        // The C's own postcondition on the hit path.
                        let oc = invoke(&gjk.c, &A, &B, &call);
                        let or = invoke(&gjk.rs, &A, &B, &call);
                        rep.check(oc.dist == 0.0 && or.dist == 0.0, || {
                            format!(
                                "hit path but dist != 0: C={} Rust={}\n  {}",
                                show_f32(oc.dist),
                                show_f32(or.dist),
                                describe(&A, &B, &call)
                            )
                        });
                        rep.check(same_v(oc.a, oc.b) && same_v(or.a, or.b), || {
                            format!("hit path but a != b\n  {}", describe(&A, &B, &call))
                        });
                    }
                }
            }
        }
    }
    assert!(hits > 200, "the hit (count==3) path was only reached {hits} times");
    eprintln!("row60: hit path reached {hits} times");
    rep.finish("row60_hit_path");
}

#[test]
fn row61_touching_midpoint_branch() {
    // Shapes whose cores are exactly `rA + rB` apart, or closer: the
    // `dist <= rA+rB` / `dist <= FLT_EPSILON` midpoint branch.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x3d);
    let mut rep = Report::new();
    for _ in 0..1200 {
        let rA = g.unit() * 20.0;
        let rB = g.unit() * 20.0;
        // Circle cores exactly rA+rB apart along a random axis.
        for scale in [1.0f32, 0.999999, 1.000001, 0.5, 0.0] {
            let d = (rA + rB) * scale;
            let ang = g.unit() * std::f32::consts::TAU;
            let A = Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
            let B = Shape::circle(c2Circle {
                p: c2v { x: d * ang.cos(), y: d * ang.sin() },
                r: rB,
            });
            gjk.check(&mut rep, &A, &B, &Call { use_radius: 1, ..Default::default() });
            gjk.check(&mut rep, &A, &B, &Call { use_radius: 0, ..Default::default() });
        }
        // Identical capsules -> dist == 0 exactly.
        let k = c2Capsule { a: g.finite_v(), b: g.finite_v(), r: rA };
        let A = Shape::capsule(k);
        gjk.check(&mut rep, &A, &A, &Call { use_radius: 1, ..Default::default() });
    }
    rep.finish("row61_touching_midpoint_branch");
}

#[test]
fn row62_radius_shrink_branch() {
    // Far-separated shapes with radii: `dist > rA+rB` and `dist > FLT_EPSILON`,
    // so `dist -= rA+rB` and the witnesses are pushed out along `c2Norm(b-a)`.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x3e);
    let mut rep = Report::new();
    for ta in TYPES {
        for tb in TYPES {
            for _ in 0..350 {
                let A = gen_shape(&mut g, ta, c2v { x: -400.0, y: 0.0 }, 10.0);
                let off = c2v { x: 400.0, y: g.sym(200.0) };
                let B = gen_shape(&mut g, tb, off, 10.0);
                gjk.check(&mut rep, &A, &B, &Call { use_radius: 1, ..Default::default() });
                gjk.check(&mut rep, &A, &B, &Call { use_radius: 0, ..Default::default() });
            }
        }
    }
    rep.finish("row62_radius_shrink_branch");
}

// ---------------------------------------------------------------------------
// Rows 63-66 — the cache axis
// ---------------------------------------------------------------------------

#[test]
fn row63_cache_cold() {
    // Non-null cache with count == 0: `cache_was_good` is false, so a fresh
    // 1-vertex simplex is built, but the cache is still written back.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x3f);
    let mut rep = Report::new();
    for ta in TYPES {
        for tb in TYPES {
            for i in 0..250 {
                let sep = [0.0, 10.0, 60.0, 200.0][i % 4];
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 20.0);
                let off = c2v { x: sep, y: g.sym(sep) };
                let B = gen_shape(&mut g, tb, off, 20.0);
                // count == 0 but the *other* fields deliberately non-zero, to
                // prove the C really only looks at `count` for the cold test.
                let cold = c2GJKCache {
                    metric: g.sym(100.0),
                    count: 0,
                    iA: [9, -3, 77],
                    iB: [-1, 4, 2],
                    div: g.sym(10.0),
                };
                for use_radius in [0, 1] {
                    gjk.check(
                        &mut rep,
                        &A,
                        &B,
                        &Call { use_radius, cache: Some(cold), ..Default::default() },
                    );
                }
            }
        }
    }
    rep.finish("row63_cache_cold");
}

#[test]
fn row64_row66_cache_warm_repeat() {
    // Call twice with the same cache and the same shapes. The second call takes
    // the `cache_was_read` path. Chained through 4 generations so a triangle
    // cache (count == 3, row 66) is also fed back in.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x40);
    let mut rep = Report::new();
    let mut counts = [0usize; 5];
    for ta in TYPES {
        for tb in TYPES {
            for i in 0..300 {
                let sep = [0.0, 3.0, 12.0, 70.0, 250.0][i % 5];
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 15.0);
                let off = c2v { x: sep, y: g.sym(sep) };
                let B = gen_shape(&mut g, tb, off, 15.0);
                let use_radius = (i % 2) as c_int;
                let mut cache = c2GJKCache::default();
                for _gen in 0..4 {
                    let call =
                        Call { use_radius, cache: Some(cache), ..Default::default() };
                    cache = gjk.check(&mut rep, &A, &B, &call);
                    if (0..=4).contains(&cache.count) {
                        counts[cache.count as usize] += 1;
                    }
                }
            }
        }
    }
    // Row 66: prove count == 3 caches really were fed back in.
    assert!(counts[3] > 100, "no triangle (count==3) cache was exercised: {counts:?}");
    assert!(counts[1] > 100 && counts[2] > 100, "cache count coverage too narrow: {counts:?}");
    eprintln!("row64/66: cache count histogram (0..4) = {counts:?}");
    rep.finish("row64_row66_cache_warm_repeat");
}

#[test]
fn row65_cache_warm_moved_shapes() {
    // Warm the cache on one placement, then reuse it for a *moved* placement of
    // the same shape types. Indices stay in range because the proxy vertex count
    // depends only on the type, so this is the well-defined `cache_was_read`
    // path the C is designed for.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x41);
    let mut rep = Report::new();
    for ta in TYPES {
        for tb in TYPES {
            for _ in 0..300 {
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 15.0);
                let B0 = gen_shape(&mut g, tb, c2v { x: 30.0, y: 0.0 }, 15.0);
                let use_radius = g.below(2) as c_int;
                let mut cache = c2GJKCache::default();
                cache = gjk.check(
                    &mut rep,
                    &A,
                    &B0,
                    &Call { use_radius, cache: Some(cache), ..Default::default() },
                );
                // Now nudge B along a path, reusing the warm cache each step.
                for step in 1..8 {
                    let B = gen_shape(
                        &mut g,
                        tb,
                        c2v { x: 30.0 - step as f32 * 9.0, y: step as f32 * 4.0 },
                        15.0,
                    );
                    cache = gjk.check(
                        &mut rep,
                        &A,
                        &B,
                        &Call { use_radius, cache: Some(cache), ..Default::default() },
                    );
                }
                // Also reuse the warm cache with the transforms turned on.
                let call = Call {
                    ax: make_xform(&mut g, 4),
                    bx: make_xform(&mut g, 4),
                    use_radius,
                    cache: Some(cache),
                    ..Default::default()
                };
                gjk.check(&mut rep, &A, &B0, &call);
            }
        }
    }
    rep.finish("row65_cache_warm_moved_shapes");
}

// ---------------------------------------------------------------------------
// Rows 67-68 — the iteration counter and the optional out-pointers
// ---------------------------------------------------------------------------

#[test]
fn row67_iterations() {
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x43);
    let mut rep = Report::new();
    let mut hist = [0usize; 21];
    for ta in TYPES {
        for tb in TYPES {
            for i in 0..500 {
                // Include extreme aspect ratios and magnitudes, which is what
                // makes GJK take many iterations.
                let scale = [0.001f32, 1.0, 1000.0, 1.0e6][i % 4];
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let off = c2v { x: g.sym(scale * 4.0), y: g.sym(scale * 4.0) };
                let B = gen_shape(&mut g, tb, off, scale);
                for use_radius in [0, 1] {
                    let call = Call { use_radius, ..Default::default() };
                    gjk.check(&mut rep, &A, &B, &call);
                    let it = invoke(&gjk.c, &A, &B, &call).iters;
                    if (0..=20).contains(&it) {
                        hist[it as usize] += 1;
                    }
                }
            }
        }
    }
    eprintln!("row67: iteration-count histogram = {hist:?}");
    assert!(hist.iter().filter(|&&n| n > 0).count() >= 3, "iteration counts too uniform: {hist:?}");
    rep.finish("row67_iterations");
}

#[test]
fn row68_optional_out_pointers() {
    // Every combination of NULL / non-NULL for outA, outB and iterations. The
    // return value must be unaffected, and a NULL out-param must leave the
    // caller's poison value untouched on both sides.
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x44);
    let mut rep = Report::new();
    for ta in TYPES {
        for tb in TYPES {
            for i in 0..120 {
                let sep = [0.0, 15.0, 120.0][i % 3];
                let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, 18.0);
                let B = gen_shape(&mut g, tb, c2v { x: sep, y: 0.0 }, 18.0);
                let mut baseline: Option<f32> = None;
                for mask in 0..8u32 {
                    for use_radius in [0, 1] {
                        let call = Call {
                            out_a: mask & 1 != 0,
                            out_b: mask & 2 != 0,
                            iters: mask & 4 != 0,
                            use_radius,
                            ..Default::default()
                        };
                        gjk.check(&mut rep, &A, &B, &call);
                        // Poison must survive where the pointer was NULL.
                        let oc = invoke(&gjk.c, &A, &B, &call);
                        let or = invoke(&gjk.rs, &A, &B, &call);
                        if !call.out_a {
                            rep.check(same_v(oc.a, POISON_V) && same_v(or.a, POISON_V), || {
                                "outA=NULL but a value was written".to_string()
                            });
                        }
                        if !call.out_b {
                            rep.check(same_v(oc.b, POISON_V) && same_v(or.b, POISON_V), || {
                                "outB=NULL but a value was written".to_string()
                            });
                        }
                        if !call.iters {
                            rep.check(oc.iters == POISON_I && or.iters == POISON_I, || {
                                "iterations=NULL but a value was written".to_string()
                            });
                        }
                        if use_radius == 0 {
                            match baseline {
                                None => baseline = Some(oc.dist),
                                Some(d) => rep.check(same_f32(d, oc.dist), || {
                                    format!(
                                        "return value changed with the out-pointer mask: {} vs {}",
                                        show_f32(d),
                                        show_f32(oc.dist)
                                    )
                                }),
                            }
                        }
                    }
                }
            }
        }
    }
    rep.finish("row68_optional_out_pointers");
}

// ---------------------------------------------------------------------------
// Rows 69-71 — degenerate shapes, extreme magnitudes, coincident shapes
// ---------------------------------------------------------------------------

#[test]
fn row69_degenerate_shapes() {
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x45);
    let mut rep = Report::new();
    let degenerates: Vec<Shape> = vec![
        Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }),
        Shape::circle(c2Circle { p: c2v { x: 3.0, y: -2.0 }, r: 0.0 }),
        Shape::circle(c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: -5.0 }),
        Shape::aabb(c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } }),
        Shape::aabb(c2AABB { min: c2v { x: -2.0, y: 1.0 }, max: c2v { x: 2.0, y: 1.0 } }), // flat
        Shape::aabb(c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } }), // inverted
        Shape::capsule(c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 }),
        Shape::capsule(c2Capsule { a: c2v { x: 2.0, y: 2.0 }, b: c2v { x: 2.0, y: 2.0 }, r: 4.0 }),
        Shape::capsule(c2Capsule { a: c2v { x: -3.0, y: 0.0 }, b: c2v { x: 3.0, y: 0.0 }, r: 0.0 }),
        Shape::capsule(c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: -2.0 }),
    ];
    for A in &degenerates {
        for B in &degenerates {
            for use_radius in [0, 1] {
                gjk.check(&mut rep, A, B, &Call { use_radius, ..Default::default() });
                // Same, with transforms and a warm cache.
                let call = Call {
                    ax: make_xform(&mut g, 4),
                    bx: make_xform(&mut g, 2),
                    use_radius,
                    cache: Some(c2GJKCache::default()),
                    ..Default::default()
                };
                let cache = gjk.check(&mut rep, A, B, &call);
                gjk.check(&mut rep, A, B, &Call { cache: Some(cache), ..call });
            }
        }
        // Degenerate against randomized ordinary shapes.
        for tb in TYPES {
            for _ in 0..150 {
                let ctr = g.finite_v();
                let B = gen_shape(&mut g, tb, ctr, 25.0);
                for use_radius in [0, 1] {
                    gjk.check(&mut rep, A, &B, &Call { use_radius, ..Default::default() });
                    gjk.check(&mut rep, &B, A, &Call { use_radius, ..Default::default() });
                }
            }
        }
    }
    rep.finish("row69_degenerate_shapes");
}

#[test]
fn row70_extreme_magnitudes() {
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x46);
    let mut rep = Report::new();
    for mag in [1.0e-30f32, 1.0e-8, 1.0, 1.0e8, 1.0e18, 1.0e30] {
        for ta in TYPES {
            for tb in TYPES {
                for _ in 0..120 {
                    let A = gen_shape(&mut g, ta, c2v { x: 0.0, y: 0.0 }, mag);
                    let off = c2v { x: g.sym(mag), y: g.sym(mag) };
                    let B = gen_shape(&mut g, tb, off, mag);
                    for use_radius in [0, 1] {
                        gjk.check(&mut rep, &A, &B, &Call { use_radius, ..Default::default() });
                    }
                }
            }
        }
    }
    rep.finish("row70_extreme_magnitudes");
}

#[test]
fn row71_coincident_shapes() {
    // The same shape passed as both A and B: the Minkowski difference is
    // centred exactly on the origin, which is the worst case for the simplex
    // solvers (zero-length edges, zero-area triangles, div == 0).
    let l = libs();
    let gjk = Gjk::new(&l);
    let mut g = Rng::new(0x47);
    let mut rep = Report::new();
    for ty in TYPES {
        for _ in 0..800 {
            let ctr = g.finite_v();
            let A = gen_shape(&mut g, ty, ctr, 30.0);
            for use_radius in [0, 1] {
                gjk.check(&mut rep, &A, &A, &Call { use_radius, ..Default::default() });
                // With identical transforms on both sides.
                let x = make_xform(&mut g, 4);
                gjk.check(
                    &mut rep,
                    &A,
                    &A,
                    &Call { ax: x, bx: x, use_radius, ..Default::default() },
                );
                // And with a warm cache chained twice.
                let mut cache = c2GJKCache::default();
                for _ in 0..3 {
                    cache = gjk.check(
                        &mut rep,
                        &A,
                        &A,
                        &Call { use_radius, cache: Some(cache), ..Default::default() },
                    );
                }
            }
        }
    }
    rep.finish("row71_coincident_shapes");
}
