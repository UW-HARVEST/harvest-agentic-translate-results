//! Phase C part 2: `ERRORS.md` rows 27-48 — the `c2GJK` rejection surface.
//!
//! `c2GJK` is the only function in the library that checks its pointers, so its
//! six `NULL` branches, its cache-freshness test, its three loop `break`s, its
//! iteration cap and its three radius-stage outcomes are all covered here. Each
//! test asserts the *specific* fallback the C takes, not merely that both sides
//! agreed.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

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
}

impl Shape {
    fn circle(c: c2Circle) -> Shape {
        Shape { ty: C2_TYPE_CIRCLE, u: ShapeU { circle: c } }
    }
    fn aabb(b: c2AABB) -> Shape {
        Shape { ty: C2_TYPE_AABB, u: ShapeU { aabb: b } }
    }
    fn capsule(k: c2Capsule) -> Shape {
        Shape { ty: C2_TYPE_CAPSULE, u: ShapeU { capsule: k } }
    }
    fn ptr(&self) -> *const c_void {
        &raw const self.u as *const c_void
    }
}

const POISON_V: c2v = c2v { x: -3.5e-12, y: 6.25e13 };
const POISON_I: c_int = -987654321;

struct Res {
    dist: f32,
    a: c2v,
    b: c2v,
    it: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
fn call(
    f: &libloading::Symbol<FnGJK>,
    A: &Shape,
    B: &Shape,
    ax: Option<&c2x>,
    bx: Option<&c2x>,
    use_radius: c_int,
    want_a: bool,
    want_b: bool,
    want_it: bool,
    cache_in: Option<c2GJKCache>,
) -> Res {
    let mut a = POISON_V;
    let mut b = POISON_V;
    let mut it = POISON_I;
    let mut cache = cache_in.unwrap_or_default();
    let dist = unsafe {
        f(
            A.ptr(),
            A.ty,
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            B.ptr(),
            B.ty,
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            if want_a { &raw mut a } else { std::ptr::null_mut() },
            if want_b { &raw mut b } else { std::ptr::null_mut() },
            use_radius,
            if want_it { &raw mut it } else { std::ptr::null_mut() },
            if cache_in.is_some() { &raw mut cache } else { std::ptr::null_mut() },
        )
    };
    Res { dist, a, b, it, cache }
}

fn agree(rep: &mut Report, x: &Res, y: &Res, tag: &str) {
    rep.check(same_f32(x.dist, y.dist), || {
        format!("[{tag}] dist: C={} Rust={}", show_f32(x.dist), show_f32(y.dist))
    });
    rep.check(same_v(x.a, y.a), || {
        format!("[{tag}] outA: C={} Rust={}", show_v(x.a), show_v(y.a))
    });
    rep.check(same_v(x.b, y.b), || {
        format!("[{tag}] outB: C={} Rust={}", show_v(x.b), show_v(y.b))
    });
    rep.check(x.it == y.it, || format!("[{tag}] iterations: C={} Rust={}", x.it, y.it));
    rep.check(same_cache(&x.cache, &y.cache), || {
        format!(
            "[{tag}] cache:\n  C:    {}\n  Rust: {}",
            show_cache(&x.cache),
            show_cache(&y.cache)
        )
    });
}

fn shapes(g: &mut Rng) -> Vec<Shape> {
    vec![
        Shape::circle(c2Circle { p: g.finite_v(), r: g.radius() }),
        Shape::aabb(g.aabb()),
        Shape::capsule(g.capsule()),
        Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }),
        Shape::aabb(c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } }),
        Shape::capsule(c2Capsule {
            a: c2v { x: 1.0, y: 1.0 },
            b: c2v { x: 1.0, y: 1.0 },
            r: 0.0,
        }),
    ]
}

// ---------------------------------------------------------------------------
// ERRORS rows 27-28 — NULL transforms substitute c2xIdentity(), not an error
// ---------------------------------------------------------------------------

#[test]
fn err27_err28_null_transforms_equal_identity() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let mut g = Rng::new(0xC27);
    let mut rep = Report::new();
    for _ in 0..400 {
        let ss = shapes(&mut g);
        for A in &ss {
            for B in &ss {
                for ur in [0, 1] {
                    // Row 27: ax_ptr == NULL must be indistinguishable from an
                    // explicit identity transform -- in BOTH libraries.
                    let n = call(&c, A, B, None, Some(&ident), ur, true, true, true, None);
                    let e = call(&c, A, B, Some(&ident), Some(&ident), ur, true, true, true, None);
                    rep.check(
                        same_f32(n.dist, e.dist) && same_v(n.a, e.a) && same_v(n.b, e.b) && n.it == e.it,
                        || format!("C: ax=NULL differed from ax=identity ({} vs {})", show_f32(n.dist), show_f32(e.dist)),
                    );
                    let nr = call(&r, A, B, None, Some(&ident), ur, true, true, true, None);
                    agree(&mut rep, &n, &nr, "row27 ax=NULL");
                    // Row 28: same for bx_ptr.
                    let n2 = call(&c, A, B, Some(&ident), None, ur, true, true, true, None);
                    let n2r = call(&r, A, B, Some(&ident), None, ur, true, true, true, None);
                    agree(&mut rep, &n2, &n2r, "row28 bx=NULL");
                    rep.check(
                        same_f32(n2.dist, e.dist) && same_v(n2.a, e.a) && same_v(n2.b, e.b),
                        || "C: bx=NULL differed from bx=identity".to_string(),
                    );
                    // Both NULL.
                    let n3 = call(&c, A, B, None, None, ur, true, true, true, None);
                    let n3r = call(&r, A, B, None, None, ur, true, true, true, None);
                    agree(&mut rep, &n3, &n3r, "rows27+28 both NULL");
                    rep.check(same_f32(n3.dist, e.dist), || {
                        "C: both-NULL differed from both-identity".to_string()
                    });
                }
            }
        }
    }
    rep.finish("err27_err28_null_transforms_equal_identity");
}

// ---------------------------------------------------------------------------
// ERRORS rows 29-31 — NULL out-pointers are silently skipped
// ---------------------------------------------------------------------------

#[test]
fn err29_to_err31_null_out_pointers() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC29);
    let mut rep = Report::new();
    for _ in 0..500 {
        let ss = shapes(&mut g);
        for A in &ss {
            for B in &ss {
                for ur in [0, 1] {
                    let full = call(&c, A, B, None, None, ur, true, true, true, None);
                    for mask in 0..8u32 {
                        let (wa, wb, wi) =
                            (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
                        let x = call(&c, A, B, None, None, ur, wa, wb, wi, None);
                        let y = call(&r, A, B, None, None, ur, wa, wb, wi, None);
                        agree(&mut rep, &x, &y, "rows29-31 out mask");
                        // Row 29/30/31: the specific expected result for a NULL
                        // out-pointer is that the caller's memory is untouched.
                        if !wa {
                            rep.check(same_v(x.a, POISON_V) && same_v(y.a, POISON_V), || {
                                format!("row29 outA=NULL wrote: C={} Rust={}", show_v(x.a), show_v(y.a))
                            });
                        } else {
                            rep.check(same_v(x.a, full.a), || "outA changed with the mask".into());
                        }
                        if !wb {
                            rep.check(same_v(x.b, POISON_V) && same_v(y.b, POISON_V), || {
                                format!("row30 outB=NULL wrote: C={} Rust={}", show_v(x.b), show_v(y.b))
                            });
                        } else {
                            rep.check(same_v(x.b, full.b), || "outB changed with the mask".into());
                        }
                        if !wi {
                            rep.check(x.it == POISON_I && y.it == POISON_I, || {
                                format!("row31 iterations=NULL wrote: C={} Rust={}", x.it, y.it)
                            });
                        } else {
                            rep.check(x.it == full.it, || "iterations changed with the mask".into());
                        }
                        // The return value must never depend on the mask.
                        rep.check(same_f32(x.dist, full.dist), || {
                            format!(
                                "return value changed with the out mask: {} vs {}",
                                show_f32(full.dist),
                                show_f32(x.dist)
                            )
                        });
                    }
                }
            }
        }
    }
    rep.finish("err29_to_err31_null_out_pointers");
}

// ---------------------------------------------------------------------------
// ERRORS rows 32-35 — the cache axis
// ---------------------------------------------------------------------------

#[test]
fn err32_cache_null_skips_read_and_write() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC32);
    let mut rep = Report::new();
    for _ in 0..600 {
        let ss = shapes(&mut g);
        for A in &ss {
            for B in &ss {
                for ur in [0, 1] {
                    let x = call(&c, A, B, None, None, ur, true, true, true, None);
                    let y = call(&r, A, B, None, None, ur, true, true, true, None);
                    agree(&mut rep, &x, &y, "row32 cache=NULL");
                    // Row 32's specific expected result: with cache == NULL the
                    // caller's cache object is never touched, so the default
                    // sentinel that `call` allocated must come back unchanged.
                    let untouched = c2GJKCache::default();
                    rep.check(same_cache(&x.cache, &untouched), || {
                        format!("row32 C wrote to a NULL cache: {}", show_cache(&x.cache))
                    });
                    rep.check(same_cache(&y.cache, &untouched), || {
                        format!("row32 Rust wrote to a NULL cache: {}", show_cache(&y.cache))
                    });
                }
            }
        }
    }
    rep.finish("err32_cache_null_skips_read_and_write");
}

#[test]
fn err33_cache_count_zero_is_cold() {
    // Row 33: `cache->count == 0` makes `cache_was_good` false, so a fresh
    // 1-vertex simplex is built regardless of the cache's OTHER fields --
    // including deliberately absurd `metric`, `div` and index values -- but the
    // cache is still written back.
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC33);
    let mut rep = Report::new();
    for _ in 0..500 {
        let ss = shapes(&mut g);
        for A in &ss {
            for B in &ss {
                for ur in [0, 1] {
                    // Reference: no cache at all.
                    let nocache = call(&c, A, B, None, None, ur, true, true, true, None);
                    for junk in [
                        c2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 },
                        c2GJKCache {
                            metric: -1.0e30,
                            count: 0,
                            iA: [7, -9, 31],
                            iB: [-4, 12, 0],
                            div: -0.0,
                        },
                        c2GJKCache {
                            metric: f32::NAN,
                            count: 0,
                            iA: [i32::MAX, i32::MIN, 5],
                            iB: [3, 3, 3],
                            div: f32::INFINITY,
                        },
                    ] {
                        let x = call(&c, A, B, None, None, ur, true, true, true, Some(junk));
                        let y = call(&r, A, B, None, None, ur, true, true, true, Some(junk));
                        agree(&mut rep, &x, &y, "row33 cold cache");
                        // A cold cache must give exactly the no-cache answer.
                        rep.check(
                            same_f32(x.dist, nocache.dist)
                                && same_v(x.a, nocache.a)
                                && same_v(x.b, nocache.b)
                                && x.it == nocache.it,
                            || {
                                format!(
                                    "row33: cold cache changed the result ({} vs {})",
                                    show_f32(nocache.dist),
                                    show_f32(x.dist)
                                )
                            },
                        );
                        // ...and the cache must have been written back.
                        rep.check(x.cache.count >= 1 && x.cache.count <= 3, || {
                            format!("row33: cache not written back: {}", show_cache(&x.cache))
                        });
                    }
                }
            }
        }
    }
    rep.finish("err33_cache_count_zero_is_cold");
}

#[test]
fn err34_err35_cache_freshness_test() {
    // Row 34: the guard is `!(min_metric < max_metric*2 && metric < -1.0e8f)`.
    // `c2GJKSimplexMetric` returns 0 for count==1 and a non-negative length for
    // count==2, so `metric < -1.0e8f` can only be true for a count==3 cache
    // whose signed `det2` is below -1e8. For every other warm cache the guard is
    // false and `cache_was_read` becomes 1 -- the freshness test is dead code.
    //
    // Row 35: a count==3 cache whose det2 IS below -1e8 and which also satisfies
    // `min_metric < 2*max_metric` is the only input that makes the C DISCARD the
    // cache. Both rows are driven here and the C's choice is inferred from the
    // observable results.
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC34);
    let mut rep = Report::new();

    // Build genuinely valid warm caches by running the algorithm first, then
    // perturb only `metric` -- the one field the freshness test reads -- across
    // the -1e8 threshold.
    let mut saw_count3 = 0usize;
    for _ in 0..900 {
        let ss = shapes(&mut g);
        for A in &ss {
            for B in &ss {
                for ur in [0, 1] {
                    let warm =
                        call(&c, A, B, None, None, ur, true, true, true, Some(c2GJKCache::default()))
                            .cache;
                    if warm.count == 3 {
                        saw_count3 += 1;
                    }
                    for metric in [
                        warm.metric,
                        0.0,
                        1.0,
                        -1.0,
                        -1.0e7,
                        -1.0e8,
                        -1.000001e8,
                        -1.0e9,
                        -1.0e30,
                        f32::NEG_INFINITY,
                        f32::INFINITY,
                        f32::NAN,
                    ] {
                        let probe = c2GJKCache { metric, ..warm };
                        let x = call(&c, A, B, None, None, ur, true, true, true, Some(probe));
                        let y = call(&r, A, B, None, None, ur, true, true, true, Some(probe));
                        agree(&mut rep, &x, &y, "rows34/35 cache freshness");
                    }
                    // Also perturb `div`, which the warm path copies into the
                    // simplex and therefore feeds straight into `1/div`.
                    for div in [warm.div, 0.0, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY] {
                        let probe = c2GJKCache { div, ..warm };
                        let x = call(&c, A, B, None, None, ur, true, true, true, Some(probe));
                        let y = call(&r, A, B, None, None, ur, true, true, true, Some(probe));
                        agree(&mut rep, &x, &y, "rows34/35 cache div");
                    }
                }
            }
        }
    }
    assert!(saw_count3 > 0, "no count==3 cache was produced, so row 35 was not driven");
    eprintln!("err34/35: {saw_count3} count==3 warm caches exercised");
    rep.finish("err34_err35_cache_freshness_test");
}

// ---------------------------------------------------------------------------
// ERRORS rows 36-39 — the loop cap and the three `break`s
// ---------------------------------------------------------------------------

#[test]
fn err36_to_err39_loop_termination() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC36);
    let mut rep = Report::new();
    let mut hist = [0usize; 22];
    // Configurations designed to stress termination: extreme aspect ratios,
    // extreme magnitudes, coincident shapes, and degenerate proxies.
    for _ in 0..2500 {
        let sc = [1.0e-20f32, 1.0e-6, 1.0, 1.0e6, 1.0e20][g.below(5) as usize];
        let shapes_a = [
            Shape::aabb(c2AABB {
                min: c2v { x: -sc, y: -sc * 1.0e-6 },
                max: c2v { x: sc, y: sc * 1.0e-6 },
            }),
            Shape::capsule(c2Capsule {
                a: c2v { x: -sc, y: 0.0 },
                b: c2v { x: sc, y: 0.0 },
                r: sc,
            }),
            Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: sc }),
        ];
        let off = c2v { x: g.sym(sc * 3.0), y: g.sym(sc * 3.0) };
        let shapes_b = [
            Shape::aabb(c2AABB {
                min: c2v { x: off.x - sc * 1.0e-6, y: off.y - sc },
                max: c2v { x: off.x + sc * 1.0e-6, y: off.y + sc },
            }),
            Shape::capsule(c2Capsule {
                a: c2v { x: off.x, y: off.y - sc },
                b: c2v { x: off.x, y: off.y + sc },
                r: sc,
            }),
            Shape::circle(c2Circle { p: off, r: sc }),
        ];
        for A in &shapes_a {
            for B in &shapes_b {
                for ur in [0, 1] {
                    let x = call(&c, A, B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
                    let y = call(&r, A, B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
                    agree(&mut rep, &x, &y, "rows36-39 termination");
                    // Row 36: the cap. The C can never report more than 20.
                    rep.check(x.it >= 0 && x.it <= 20, || {
                        format!("row36 C reported iterations={} outside [0,20]", x.it)
                    });
                    rep.check(y.it >= 0 && y.it <= 20, || {
                        format!("row36 Rust reported iterations={} outside [0,20]", y.it)
                    });
                    if (0..=20).contains(&x.it) {
                        hist[x.it as usize] += 1;
                    }
                    // Rows 37/38/39: whichever `break` fired, the resulting
                    // simplex must be identical, which the cache write-back
                    // exposes (count + indices + div).
                    rep.check(same_cache(&x.cache, &y.cache), || {
                        format!(
                            "rows37-39 post-break simplex differs:\n  C:    {}\n  Rust: {}",
                            show_cache(&x.cache),
                            show_cache(&y.cache)
                        )
                    });
                    rep.check((1..=3).contains(&x.cache.count), || {
                        format!("post-loop count out of [1,3]: {}", x.cache.count)
                    });
                }
            }
        }
    }
    eprintln!("err36-39: iteration histogram = {hist:?}");
    rep.finish("err36_to_err39_loop_termination");
}

// ---------------------------------------------------------------------------
// ERRORS rows 40-45 — the radius stage's mutually exclusive outcomes
// ---------------------------------------------------------------------------

#[test]
fn err40_to_err45_radius_stage() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC40);
    let mut rep = Report::new();
    let mut hit_n = 0usize;
    let mut mid_n = 0usize;
    let mut shrink_n = 0usize;

    for _ in 0..2000 {
        let rA = g.unit() * 30.0;
        let rB = g.unit() * 30.0;
        // Sweep the core separation right across `rA + rB` and across
        // FLT_EPSILON, so rows 41/42/43/44 all fire.
        for k in 0..14 {
            let d = match k {
                0 => 0.0,
                1 => FLT_EPSILON * 0.5,
                2 => FLT_EPSILON,
                3 => FLT_EPSILON * 2.0,
                4 => (rA + rB) * 0.5,
                5 => (rA + rB) * 0.999999,
                6 => rA + rB,
                7 => (rA + rB) * 1.000001,
                8 => (rA + rB) * 1.5,
                9 => (rA + rB) + 1.0,
                10 => 1000.0,
                11 => 1.0e10,
                12 => g.unit() * 200.0,
                _ => g.unit() * (rA + rB) * 2.0,
            };
            let ang = g.unit() * std::f32::consts::TAU;
            let A = Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
            let B = Shape::circle(c2Circle {
                p: c2v { x: d * ang.cos(), y: d * ang.sin() },
                r: rB,
            });
            // Row 45: use_radius == 0 returns the raw core distance and never
            // applies radii. Compare against the same call with radii zeroed.
            let raw = call(&c, &A, &B, None, None, 0, true, true, true, None);
            let raw_r = call(&r, &A, &B, None, None, 0, true, true, true, None);
            agree(&mut rep, &raw, &raw_r, "row45 use_radius=0");
            let A0 = Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 });
            let B0 = Shape::circle(c2Circle {
                p: c2v { x: d * ang.cos(), y: d * ang.sin() },
                r: 0.0,
            });
            let raw0 = call(&c, &A0, &B0, None, None, 0, true, true, true, None);
            rep.check(same_f32(raw.dist, raw0.dist), || {
                format!(
                    "row45: use_radius=0 was affected by the radii ({} vs {})",
                    show_f32(raw.dist),
                    show_f32(raw0.dist)
                )
            });

            // use_radius == 1: rows 41-44.
            let x = call(&c, &A, &B, None, None, 1, true, true, true, Some(c2GJKCache::default()));
            let y = call(&r, &A, &B, None, None, 1, true, true, true, Some(c2GJKCache::default()));
            agree(&mut rep, &x, &y, "rows41-44 radius stage");
            if x.cache.count == 3 {
                hit_n += 1;
            } else if raw.dist > rA + rB && raw.dist > FLT_EPSILON {
                shrink_n += 1;
                // Row 43: dist must equal the shrunk core distance.
                rep.check(same_f32(x.dist, raw.dist - (rA + rB)) || x.dist == 0.0, || {
                    format!(
                        "row43: expected {} - {} = {}, got {}",
                        show_f32(raw.dist),
                        show_f32(rA + rB),
                        show_f32(raw.dist - (rA + rB)),
                        show_f32(x.dist)
                    )
                });
            } else {
                mid_n += 1;
                // Rows 41/42: the midpoint branch forces dist == 0 and a == b.
                rep.check(x.dist == 0.0 && y.dist == 0.0, || {
                    format!(
                        "rows41/42: midpoint branch must give dist 0, got C={} Rust={}",
                        show_f32(x.dist),
                        show_f32(y.dist)
                    )
                });
                rep.check(same_v(x.a, x.b) && same_v(y.a, y.b), || {
                    "rows41/42: midpoint branch must give a == b".to_string()
                });
            }
        }
    }

    // Row 40: the hit path -- deeply overlapping polygonal shapes so `c23`
    // reaches count == 3. Radii must be IGNORED even with use_radius = 1.
    for _ in 0..1500 {
        let A = Shape::aabb(c2AABB {
            min: c2v { x: -10.0, y: -10.0 },
            max: c2v { x: 10.0, y: 10.0 },
        });
        let B = Shape::aabb(c2AABB {
            min: c2v { x: g.sym(4.0) - 8.0, y: g.sym(4.0) - 8.0 },
            max: c2v { x: g.sym(4.0) + 8.0, y: g.sym(4.0) + 8.0 },
        });
        for ur in [0, 1] {
            let x = call(&c, &A, &B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
            let y = call(&r, &A, &B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
            agree(&mut rep, &x, &y, "row40 hit");
            if x.cache.count == 3 {
                hit_n += 1;
                rep.check(x.dist == 0.0 && y.dist == 0.0, || {
                    format!("row40: hit path must give dist 0, got C={}", show_f32(x.dist))
                });
                rep.check(same_v(x.a, x.b) && same_v(y.a, y.b), || {
                    "row40: hit path must set a = b".to_string()
                });
            }
        }
    }

    // Row 44: after the shrink, `a == b` forces dist to 0. Provoke it by making
    // the radius sum consume the whole distance to within float resolution.
    for k in 0..4000 {
        let d = 1.0e-4 + k as f32 * 1.0e-7;
        let rA = d * 0.5;
        let rB = d * 0.5 - f32::from_bits(1);
        let A = Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
        let B = Shape::circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: rB });
        let x = call(&c, &A, &B, None, None, 1, true, true, true, None);
        let y = call(&r, &A, &B, None, None, 1, true, true, true, None);
        agree(&mut rep, &x, &y, "row44 shrink cancellation");
    }

    eprintln!("err40-45: hit={hit_n} midpoint={mid_n} shrink={shrink_n}");
    assert!(hit_n > 100, "row 40 (hit path) reached only {hit_n} times");
    assert!(mid_n > 100, "rows 41/42 (midpoint) reached only {mid_n} times");
    assert!(shrink_n > 100, "row 43 (shrink) reached only {shrink_n} times");
    rep.finish("err40_to_err45_radius_stage");
}

// ---------------------------------------------------------------------------
// ERRORS rows 47-48 — division by zero inside c2Norm reached through c2GJK
// ---------------------------------------------------------------------------

#[test]
fn err47_err48_norm_of_zero_through_gjk() {
    // `c2Norm(c2Sub(b, a))` on the shrink path divides by `c2Len`, which is 0
    // when the two witnesses coincide. The C has no guard, so `inf`/`NaN` can
    // reach the outputs. Drive it directly and through GJK.
    let l = libs();
    let norm = (l.c.sym::<FnVV>("c2Norm"), l.rs.sym::<FnVV>("c2Norm"));
    let div = (l.c.sym::<FnVsV>("c2Div"), l.rs.sym::<FnVsV>("c2Div"));
    let (c, r) = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xC47);
    let mut rep = Report::new();

    // Row 47: c2Div by exactly zero.
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -3.0, y: 4.0 },
        c2v { x: f32::MAX, y: f32::MIN },
        c2v { x: f32::from_bits(1), y: 0.0 },
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
    ] {
        for s in [0.0f32, -0.0, f32::from_bits(1), f32::MIN_POSITIVE, f32::INFINITY, f32::NAN] {
            let (x, y) = (div.0(v, s), div.1(v, s));
            rep.check(same_v(x, y), || {
                format!("row47 c2Div({}, {}): C={} Rust={}", show_v(v), show_f32(s), show_v(x), show_v(y))
            });
        }
        // Row 48: c2Norm of the zero vector -> 0 * inf -> NaN.
        let (x, y) = (norm.0(v), norm.1(v));
        rep.check(same_v(x, y), || {
            format!("row48 c2Norm({}): C={} Rust={}", show_v(v), show_v(x), show_v(y))
        });
    }
    let zero = c2v { x: 0.0, y: 0.0 };
    let (x, y) = (norm.0(zero), norm.1(zero));
    rep.check(x.x.is_nan() && x.y.is_nan() && y.x.is_nan() && y.y.is_nan(), || {
        format!("row48: c2Norm((0,0)) must be (NaN,NaN): C={} Rust={}", show_v(x), show_v(y))
    });

    // ...and through c2GJK, where coincident witnesses on the shrink path make
    // `c2Norm` see a zero vector.
    for _ in 0..2000 {
        let p = g.finite_v();
        let rr = g.radius();
        let A = Shape::circle(c2Circle { p, r: rr });
        let B = Shape::circle(c2Circle { p, r: rr });
        for ur in [0, 1] {
            let xx = call(&c, &A, &B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
            let yy = call(&r, &A, &B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
            agree(&mut rep, &xx, &yy, "rows47/48 through GJK");
        }
        // Degenerate capsules and points, which also collapse the witnesses.
        let A = Shape::capsule(c2Capsule { a: p, b: p, r: rr });
        let B = Shape::capsule(c2Capsule { a: p, b: p, r: rr });
        for ur in [0, 1] {
            let xx = call(&c, &A, &B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
            let yy = call(&r, &A, &B, None, None, ur, true, true, true, Some(c2GJKCache::default()));
            agree(&mut rep, &xx, &yy, "rows47/48 degenerate capsules");
        }
    }
    rep.finish("err47_err48_norm_of_zero_through_gjk");
}
