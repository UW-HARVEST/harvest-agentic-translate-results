//! Phase B — CONFIGS.md rows 50-75: `c2GJK`, the low-level entry point.
//!
//! `c2GJK` is where the options actually interact: `use_radius`, the two
//! optional transforms, the optional warm-start cache, and three optional
//! out-parameters, crossed with 9 shape-type pairs and every shape relation.
//! The convenience wrappers (`c2AABBtoCapsule` etc.) only ever call it with one
//! fixed configuration, so these rows drive it directly.
//!
//! Each call compares the returned `f32`, `*outA`, `*outB`, `*iterations` and
//! the whole `c2GJKCache` after write-back.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

/// Which optional out-params / cache to supply.
#[derive(Copy, Clone, Debug)]
struct Opts {
    use_radius: c_int,
    want_outA: bool,
    want_outB: bool,
    want_iters: bool,
    /// `None` = pass a null cache pointer.
    cache: Option<c2GJKCache>,
}

impl Opts {
    fn plain(use_radius: c_int) -> Opts {
        Opts {
            use_radius,
            want_outA: true,
            want_outB: true,
            want_iters: true,
            cache: None,
        }
    }
}

#[derive(Debug)]
struct Out {
    dist: f32,
    outA: c2v,
    outB: c2v,
    iters: c_int,
    cache: Option<c2GJKCache>,
}

fn call(api: &Api, a: &ShapeBuf, ta: C2_TYPE, ax: Option<&c2x>, b: &ShapeBuf, tb: C2_TYPE, bx: Option<&c2x>, o: &Opts) -> Out {
    // Sentinel fills so a skipped write is observable.
    let mut outA = c2v { x: 111.25, y: -222.5 };
    let mut outB = c2v { x: -333.75, y: 444.0 };
    let mut iters: c_int = -12345;
    let mut cache = o.cache;

    let dist = unsafe {
        (api.c2GJK)(
            a.as_ptr(),
            ta,
            ax.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            b.as_ptr(),
            tb,
            bx.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            if o.want_outA { &mut outA } else { std::ptr::null_mut() },
            if o.want_outB { &mut outB } else { std::ptr::null_mut() },
            o.use_radius,
            if o.want_iters { &mut iters } else { std::ptr::null_mut() },
            cache.as_mut().map(|c| c as *mut c2GJKCache).unwrap_or(std::ptr::null_mut()),
        )
    };
    Out {
        dist,
        outA,
        outB,
        iters,
        cache,
    }
}

#[track_caller]
fn diff(ctx: &str, a: &ShapeBuf, ta: C2_TYPE, ax: Option<&c2x>, b: &ShapeBuf, tb: C2_TYPE, bx: Option<&c2x>, o: &Opts) {
    let (c, r) = apis();
    let co = call(c, a, ta, ax, b, tb, bx, o);
    let ro = call(r, a, ta, ax, b, tb, bx, o);
    let ctx = format!("{ctx} [{}/{} {o:?}]", type_name(ta), type_name(tb));
    eq_f32(&format!("{ctx} dist"), co.dist, ro.dist);
    eq_v(&format!("{ctx} outA"), co.outA, ro.outA);
    eq_v(&format!("{ctx} outB"), co.outB, ro.outB);
    eq_int(&format!("{ctx} iterations"), co.iters, ro.iters);
    match (co.cache, ro.cache) {
        (Some(cc), Some(rc)) => eq_cache(&format!("{ctx} cache"), &cc, &rc),
        (None, None) => {}
        _ => panic!("{ctx}: cache presence mismatch"),
    }
}

const PAIRS: usize = 400;

fn all_type_pairs() -> Vec<(C2_TYPE, C2_TYPE)> {
    let mut v = Vec::new();
    for &a in &VALID_TYPES {
        for &b in &VALID_TYPES {
            v.push((a, b));
        }
    }
    v
}

// ---------------------------------------------------------------------------
// Rows 50-51 — all 9 type pairs, use_radius 0 and 1, null transforms/cache
// ---------------------------------------------------------------------------

#[test]
fn row50_51_all_pairs_use_radius_both() {
    let mut g = Rng::new(SEED ^ 50);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for i in 0..PAIRS {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row50/51 #{i}"), &a, ta, None, &b, tb, None, &o);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 52-57 — the transform axis
// ---------------------------------------------------------------------------

#[test]
fn row52_explicit_identity_transforms() {
    let (c, _) = apis();
    let ident = (c.c2xIdentity)();
    let mut g = Rng::new(SEED ^ 52);
    for (ta, tb) in all_type_pairs() {
        let o = Opts::plain(1);
        for i in 0..PAIRS {
            let a = rand_shape(&mut g, ta);
            let b = rand_shape(&mut g, tb);
            diff(
                &format!("row52 #{i}"),
                &a,
                ta,
                Some(&ident),
                &b,
                tb,
                Some(&ident),
                &o,
            );
        }
    }
}

#[test]
fn row53_pure_translation() {
    let mut g = Rng::new(SEED ^ 53);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for i in 0..PAIRS / 2 {
                let ax = c2x {
                    p: g.v(),
                    r: c2r { c: 1.0, s: 0.0 },
                };
                let bx = c2x {
                    p: g.v(),
                    r: c2r { c: 1.0, s: 0.0 },
                };
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row53 #{i}"), &a, ta, Some(&ax), &b, tb, Some(&bx), &o);
            }
        }
    }
}

#[test]
fn row54_pure_rotation() {
    let mut g = Rng::new(SEED ^ 54);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for i in 0..PAIRS / 2 {
                let angle_a = g.unit() * std::f32::consts::TAU;
                let angle_b = g.unit() * std::f32::consts::TAU;
                let ax = c2x {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: c2r {
                        c: angle_a.cos(),
                        s: angle_a.sin(),
                    },
                };
                let bx = c2x {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: c2r {
                        c: angle_b.cos(),
                        s: angle_b.sin(),
                    },
                };
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row54 #{i}"), &a, ta, Some(&ax), &b, tb, Some(&bx), &o);
            }
        }
    }
}

#[test]
fn row55_rotation_plus_translation() {
    let mut g = Rng::new(SEED ^ 55);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for i in 0..PAIRS / 2 {
                let angle_a = g.unit() * std::f32::consts::TAU;
                let angle_b = g.unit() * std::f32::consts::TAU;
                let ax = c2x {
                    p: g.v(),
                    r: c2r {
                        c: angle_a.cos(),
                        s: angle_a.sin(),
                    },
                };
                let bx = c2x {
                    p: g.v(),
                    r: c2r {
                        c: angle_b.cos(),
                        s: angle_b.sin(),
                    },
                };
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row55 #{i}"), &a, ta, Some(&ax), &b, tb, Some(&bx), &o);
            }
        }
    }
}

/// Row 56 — non-unit, zero and NaN `c2r`. The C never normalizes a `c2r`, so
/// these are all reachable inputs and they change `c2Mulxv` / `c2MulrvT`
/// (and hence which support vertex is chosen) drastically.
#[test]
fn row56_non_unit_and_nan_rotations() {
    let mut g = Rng::new(SEED ^ 56);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for i in 0..PAIRS / 2 {
                let ax = g.xform();
                let bx = g.xform();
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row56 #{i}"), &a, ta, Some(&ax), &b, tb, Some(&bx), &o);
            }
        }
    }
    // Explicit degenerate rotations.
    let degenerate = [
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 0.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: 1.0, y: 1.0 },
            r: c2r { c: 5.0, s: -3.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r {
                c: f32::NAN,
                s: 0.0,
            },
        },
        c2x {
            p: c2v {
                x: f32::INFINITY,
                y: 0.0,
            },
            r: c2r { c: 1.0, s: 0.0 },
        },
    ];
    for (ta, tb) in all_type_pairs() {
        for ax in &degenerate {
            for bx in &degenerate {
                for ur in [0, 1] {
                    let o = Opts::plain(ur);
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    diff("row56 degenerate", &a, ta, Some(ax), &b, tb, Some(bx), &o);
                }
            }
        }
    }
}

/// Row 57 — asymmetric transform nullability (only one of the two supplied).
#[test]
fn row57_asymmetric_transform_nullability() {
    let mut g = Rng::new(SEED ^ 57);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for i in 0..PAIRS / 2 {
                let xf = g.xform();
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row57 a-only #{i}"), &a, ta, Some(&xf), &b, tb, None, &o);
                diff(&format!("row57 b-only #{i}"), &a, ta, None, &b, tb, Some(&xf), &o);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 58-60 — out-parameter nullability
// ---------------------------------------------------------------------------

#[test]
fn row58_60_out_param_nullability() {
    let mut g = Rng::new(SEED ^ 58);
    let combos = [
        (false, false, false), // row 58
        (true, false, false),  // row 59
        (false, true, false),  // row 59 mirror
        (false, false, true),  // row 60
        (true, true, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ];
    for (ta, tb) in all_type_pairs() {
        for &(oa, ob, oi) in &combos {
            for ur in [0, 1] {
                let o = Opts {
                    use_radius: ur,
                    want_outA: oa,
                    want_outB: ob,
                    want_iters: oi,
                    cache: None,
                };
                for i in 0..64 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    diff(&format!("row58-60 #{i}"), &a, ta, None, &b, tb, None, &o);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 61-64 — the cache axis
// ---------------------------------------------------------------------------

/// Row 61 — cold cache (`count == 0`): the warm-start block is skipped and the
/// cache is written back afterwards.
#[test]
fn row61_cold_cache() {
    let mut g = Rng::new(SEED ^ 61);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            for i in 0..PAIRS / 2 {
                let o = Opts {
                    use_radius: ur,
                    want_outA: true,
                    want_outB: true,
                    want_iters: true,
                    // count == 0 => cache_was_good is false, but the rest of the
                    // struct is still garbage-in / garbage-out, so randomize it.
                    cache: Some(c2GJKCache {
                        metric: g.coord(),
                        count: 0,
                        iA: [g.below(3) as c_int, g.below(3) as c_int, g.below(3) as c_int],
                        iB: [g.below(3) as c_int, g.below(3) as c_int, g.below(3) as c_int],
                        div: g.coord(),
                    }),
                };
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                diff(&format!("row61 #{i}"), &a, ta, None, &b, tb, None, &o);
            }
        }
    }
}

/// Row 62 — warm cache: call twice with the same cache and the same shapes,
/// comparing the outputs *and* the cache after each call.
#[test]
fn row62_warm_cache_repeat() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 62);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            for i in 0..PAIRS / 2 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                let mut cc = c2GJKCache::default();
                let mut rc = c2GJKCache::default();
                for pass in 0..4 {
                    let oc = Opts {
                        use_radius: ur,
                        want_outA: true,
                        want_outB: true,
                        want_iters: true,
                        cache: Some(cc),
                    };
                    let or = Opts {
                        use_radius: ur,
                        want_outA: true,
                        want_outB: true,
                        want_iters: true,
                        cache: Some(rc),
                    };
                    let co = call(c, &a, ta, None, &b, tb, None, &oc);
                    let ro = call(r, &a, ta, None, &b, tb, None, &or);
                    let ctx = format!(
                        "row62 #{i} pass{pass} {}/{} ur={ur}",
                        type_name(ta),
                        type_name(tb)
                    );
                    eq_f32(&format!("{ctx} dist"), co.dist, ro.dist);
                    eq_v(&format!("{ctx} outA"), co.outA, ro.outA);
                    eq_v(&format!("{ctx} outB"), co.outB, ro.outB);
                    eq_int(&format!("{ctx} iters"), co.iters, ro.iters);
                    cc = co.cache.unwrap();
                    rc = ro.cache.unwrap();
                    eq_cache(&format!("{ctx} cache"), &cc, &rc);
                }
            }
        }
    }
}

/// Row 63 — the real consumer pattern: one cache reused while the shapes move.
/// This is the composed pipeline that per-call tests cannot reach, because the
/// cached simplex indices from step N feed the warm start at step N+1.
#[test]
fn row63_warm_cache_sweep() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 63);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            for trial in 0..40 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                let mut cc = c2GJKCache::default();
                let mut rc = c2GJKCache::default();
                // Sweep B past A by translating it step by step.
                let dir = g.v();
                for step in 0..12 {
                    let bx = c2x {
                        p: c2v {
                            x: dir.x * step as f32,
                            y: dir.y * step as f32,
                        },
                        r: c2r { c: 1.0, s: 0.0 },
                    };
                    let oc = Opts {
                        use_radius: ur,
                        want_outA: true,
                        want_outB: true,
                        want_iters: true,
                        cache: Some(cc),
                    };
                    let or = Opts { cache: Some(rc), ..oc };
                    let co = call(c, &a, ta, None, &b, tb, Some(&bx), &oc);
                    let ro = call(r, &a, ta, None, &b, tb, Some(&bx), &or);
                    let ctx = format!(
                        "row63 trial{trial} step{step} {}/{} ur={ur}",
                        type_name(ta),
                        type_name(tb)
                    );
                    eq_f32(&format!("{ctx} dist"), co.dist, ro.dist);
                    eq_v(&format!("{ctx} outA"), co.outA, ro.outA);
                    eq_v(&format!("{ctx} outB"), co.outB, ro.outB);
                    eq_int(&format!("{ctx} iters"), co.iters, ro.iters);
                    cc = co.cache.unwrap();
                    rc = ro.cache.unwrap();
                    eq_cache(&format!("{ctx} cache"), &cc, &rc);
                }
            }
        }
    }
}

/// Number of vertices `c2MakeProxy` actually initialises for a given type.
/// Cached indices `>= this` make the C read *uninitialised* `c2Proxy.verts[]`
/// stack slots (its `c2Proxy pA;` is not zero-initialised), which is genuinely
/// indeterminate — measured directly by `row64_out_of_count_cache_index_is_ub`.
fn proxy_vert_count(t: C2_TYPE) -> u32 {
    match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_CAPSULE => 2,
        C2_TYPE_AABB => 4,
        _ => 0,
    }
}

/// Row 64 (ERRORS.md rows 16, 25) — hand-forged caches, including ones no real
/// call would ever produce: `count` 1..3 in every combination, indices pointing
/// at any *initialised* proxy vertex (so a warm start can legitimately name a
/// vertex the previous call did not pick), and `metric` / `div` values that
/// drive both sides of the `metric < -1.0e8f` acceptance guard.
///
/// The C bounds-checks nothing here — not `cache->count` against `iA[3]`, and
/// not the indices against the proxy's vertex count — and the Rust must index
/// just as loosely. Indices are kept below `proxy_vert_count` so that both
/// builds read the same *defined* bytes; the out-of-count case is measured
/// separately below because the C reads indeterminate memory there.
#[test]
fn row64_hand_forged_cache() {
    let mut g = Rng::new(SEED ^ 64);
    for (ta, tb) in all_type_pairs() {
        let na = proxy_vert_count(ta);
        let nb = proxy_vert_count(tb);
        for count in 1..=3i32 {
            for ur in [0, 1] {
                for i in 0..80 {
                    let cache = c2GJKCache {
                        metric: match g.below(5) {
                            0 => 0.0,
                            1 => -1e9, // makes `metric < -1.0e8f` true
                            2 => f32::NAN,
                            3 => 1e9,
                            _ => g.coord(),
                        },
                        count,
                        iA: [
                            g.below(na) as c_int,
                            g.below(na) as c_int,
                            g.below(na) as c_int,
                        ],
                        iB: [
                            g.below(nb) as c_int,
                            g.below(nb) as c_int,
                            g.below(nb) as c_int,
                        ],
                        div: match g.below(4) {
                            0 => 0.0,
                            1 => 1.0,
                            _ => g.coord(),
                        },
                    };
                    let o = Opts {
                        use_radius: ur,
                        want_outA: true,
                        want_outB: true,
                        want_iters: true,
                        cache: Some(cache),
                    };
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    diff(&format!("row64 #{i} count={count}"), &a, ta, None, &b, tb, None, &o);
                }
            }
        }
    }
}

/// ERRORS.md row 25, measured rather than assumed: a cached index at or beyond
/// the proxy's initialised vertex count makes the C read an **uninitialised**
/// `c2Proxy.verts[]` stack slot. That is indeterminate by the C standard and in
/// practice returns leftover stack bytes, whereas the Rust reads its
/// zero-initialised proxy. This test documents the boundary precisely: indices
/// **below** the count agree bit-for-bit (already covered exhaustively by
/// `row64_hand_forged_cache`), indices **at or above** it are not required to.
#[test]
fn row64_out_of_count_cache_index_is_ub() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0x64_ff);
    let mut in_count_agree = 0usize;
    let mut out_of_count_total = 0usize;
    let mut out_of_count_agree = 0usize;

    for (ta, tb) in all_type_pairs() {
        let na = proxy_vert_count(ta);
        let nb = proxy_vert_count(tb);
        for idx in 0..8u32 {
            let cache = c2GJKCache {
                metric: 0.0,
                count: 1,
                iA: [idx as c_int, 0, 0],
                iB: [idx as c_int, 0, 0],
                div: 1.0,
            };
            let o = Opts {
                use_radius: 1,
                want_outA: true,
                want_outB: true,
                want_iters: true,
                cache: Some(cache),
            };
            let a = rand_shape(&mut g, ta);
            let b = rand_shape(&mut g, tb);
            let co = call(c, &a, ta, None, &b, tb, None, &o);
            let ro = call(r, &a, ta, None, &b, tb, None, &o);
            let same = co.dist.to_bits() == ro.dist.to_bits()
                && co.outA.x.to_bits() == ro.outA.x.to_bits()
                && co.outA.y.to_bits() == ro.outA.y.to_bits()
                && co.outB.x.to_bits() == ro.outB.x.to_bits()
                && co.outB.y.to_bits() == ro.outB.y.to_bits()
                && co.iters == ro.iters;
            if idx < na.min(nb) {
                assert!(
                    same,
                    "in-count cache index {idx} for {}/{} MUST agree: C dist={:?} outA={:?}, \
                     Rust dist={:?} outA={:?}",
                    type_name(ta),
                    type_name(tb),
                    co.dist,
                    co.outA,
                    ro.dist,
                    ro.outA
                );
                in_count_agree += 1;
            } else {
                out_of_count_total += 1;
                if same {
                    out_of_count_agree += 1;
                }
            }
        }
    }
    println!(
        "cache-index boundary: in-count agreements={in_count_agree}; \
         out-of-count (C reads uninitialised stack) {out_of_count_agree}/{out_of_count_total} \
         happened to agree"
    );
    assert!(in_count_agree > 0 && out_of_count_total > 0);
}

// ---------------------------------------------------------------------------
// Rows 65-70 — shape relations, constructed rather than random
// ---------------------------------------------------------------------------

/// Build a shape of type `t` centred at `c` with "radius" `s`.
fn shape_at(t: C2_TYPE, cx: f32, cy: f32, s: f32) -> ShapeBuf {
    match t {
        C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
            p: c2v { x: cx, y: cy },
            r: s,
        }),
        C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
            min: c2v { x: cx - s, y: cy - s },
            max: c2v { x: cx + s, y: cy + s },
        }),
        _ => ShapeBuf::from_capsule(c2Capsule {
            a: c2v { x: cx - s, y: cy },
            b: c2v { x: cx + s, y: cy },
            r: s * 0.5,
        }),
    }
}

#[test]
fn row65_70_shape_relations() {
    // Separations chosen to straddle every threshold: deep overlap, touching,
    // just apart (the radius-shrink branch), far apart, containment, coincident.
    let seps: [f32; 14] = [
        0.0, 1e-8, 1e-6, 0.001, 0.5, 1.0, 1.5, 1.9999, 2.0, 2.0001, 2.5, 5.0, 100.0, 1e6,
    ];
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for &sep in &seps {
                // Row 65-68: separation sweep along x, and along the diagonal.
                let a = shape_at(ta, 0.0, 0.0, 1.0);
                let b = shape_at(tb, sep, 0.0, 1.0);
                diff(&format!("row65-68 sep={sep} axis"), &a, ta, None, &b, tb, None, &o);
                let b2 = shape_at(tb, sep * 0.7071, sep * 0.7071, 1.0);
                diff(&format!("row65-68 sep={sep} diag"), &a, ta, None, &b2, tb, None, &o);

                // Row 69: containment — a tiny B deep inside a big A.
                let big = shape_at(ta, 0.0, 0.0, 50.0);
                let small = shape_at(tb, sep.min(10.0), 0.0, 0.25);
                diff(&format!("row69 sep={sep}"), &big, ta, None, &small, tb, None, &o);
                diff(&format!("row69 rev sep={sep}"), &small, tb, None, &big, ta, None, &o);
            }
            // Row 70: coincident / identical shapes.
            if ta == tb {
                let a = shape_at(ta, 0.0, 0.0, 1.0);
                let b = shape_at(tb, 0.0, 0.0, 1.0);
                diff("row70 coincident", &a, ta, None, &b, tb, None, &o);
            }
            // Row 66: exactly touching, radii accounted for.
            let a = shape_at(ta, 0.0, 0.0, 1.0);
            let b = shape_at(tb, 2.0, 0.0, 1.0);
            diff("row66 exact touch", &a, ta, None, &b, tb, None, &o);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 71-75 — degenerate, extreme and invalid inputs
// ---------------------------------------------------------------------------

/// Row 71 — degenerate shapes: zero radii, point capsules, empty and inverted
/// AABBs. These make the GJK search direction degenerate and drive the
/// `c2Dot(d,d) < eps*eps` bail-out and the `div == 0` witness path.
#[test]
fn row71_degenerate_shapes() {
    let degenerate: Vec<(C2_TYPE, ShapeBuf)> = vec![
        (
            C2_TYPE_CIRCLE,
            ShapeBuf::from_circle(c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 0.0,
            }),
        ),
        (
            C2_TYPE_CIRCLE,
            ShapeBuf::from_circle(c2Circle {
                p: c2v { x: 1.0, y: 1.0 },
                r: 0.0,
            }),
        ),
        (
            C2_TYPE_CAPSULE,
            ShapeBuf::from_capsule(c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r: 0.0,
            }),
        ),
        (
            C2_TYPE_CAPSULE,
            ShapeBuf::from_capsule(c2Capsule {
                a: c2v { x: 1.0, y: 2.0 },
                b: c2v { x: 1.0, y: 2.0 },
                r: 1.0,
            }),
        ),
        (
            C2_TYPE_AABB,
            ShapeBuf::from_aabb(c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 0.0, y: 0.0 },
            }),
        ),
        (
            C2_TYPE_AABB,
            ShapeBuf::from_aabb(c2AABB {
                min: c2v { x: 5.0, y: 5.0 },
                max: c2v { x: -5.0, y: -5.0 },
            }),
        ),
        (
            C2_TYPE_AABB,
            ShapeBuf::from_aabb(c2AABB {
                min: c2v { x: 0.0, y: -1.0 },
                max: c2v { x: 0.0, y: 1.0 },
            }),
        ),
    ];
    for (ta, a) in &degenerate {
        for (tb, b) in &degenerate {
            for ur in [0, 1] {
                for with_cache in [false, true] {
                    let o = Opts {
                        use_radius: ur,
                        want_outA: true,
                        want_outB: true,
                        want_iters: true,
                        cache: if with_cache {
                            Some(c2GJKCache::default())
                        } else {
                            None
                        },
                    };
                    diff("row71 degenerate", a, *ta, None, b, *tb, None, &o);
                }
            }
        }
    }
}

/// Row 72 — huge coordinates: `c2Dot` overflows to `inf`, `c2Len` returns
/// `inf`, and `c2Norm` divides by `inf`.
#[test]
fn row72_huge_coordinates() {
    let mags = [1e18f32, 1e30, 1e38, f32::MAX];
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            let o = Opts::plain(ur);
            for &m in &mags {
                let a = shape_at(ta, -m, 0.0, m * 0.001);
                let b = shape_at(tb, m, 0.0, m * 0.001);
                diff(&format!("row72 m={m:e}"), &a, ta, None, &b, tb, None, &o);
                let a2 = shape_at(ta, 0.0, 0.0, m);
                let b2 = shape_at(tb, m, m, m);
                diff(&format!("row72 big m={m:e}"), &a2, ta, None, &b2, tb, None, &o);
            }
        }
    }
}

/// Row 73 — `NaN` / `±inf` coordinates and radii.
#[test]
fn row73_nan_and_inf_shapes() {
    let mut g = Rng::new(SEED ^ 73);
    for (ta, tb) in all_type_pairs() {
        for ur in [0, 1] {
            for with_cache in [false, true] {
                let o = Opts {
                    use_radius: ur,
                    want_outA: true,
                    want_outB: true,
                    want_iters: true,
                    cache: if with_cache {
                        Some(c2GJKCache::default())
                    } else {
                        None
                    },
                };
                for i in 0..200 {
                    let a = match ta {
                        C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
                            p: g.wild_v(),
                            r: g.wild(),
                        }),
                        C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
                            min: g.wild_v(),
                            max: g.wild_v(),
                        }),
                        _ => ShapeBuf::from_capsule(c2Capsule {
                            a: g.wild_v(),
                            b: g.wild_v(),
                            r: g.wild(),
                        }),
                    };
                    let b = match tb {
                        C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
                            p: g.wild_v(),
                            r: g.wild(),
                        }),
                        C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
                            min: g.wild_v(),
                            max: g.wild_v(),
                        }),
                        _ => ShapeBuf::from_capsule(c2Capsule {
                            a: g.wild_v(),
                            b: g.wild_v(),
                            r: g.wild(),
                        }),
                    };
                    diff(&format!("row73 #{i}"), &a, ta, None, &b, tb, None, &o);
                }
            }
        }
    }
}

/// Row 74 — out-of-range type tags passed to `c2GJK` **directly**.
///
/// This path is undefined behaviour in the C and cannot be exercised in-process:
/// `c2MakeProxy` writes nothing for an unknown type, so `c2GJK` proceeds on an
/// uninitialised `c2Proxy` whose `count` field is whatever was on the stack.
/// `c2Support(pA.verts, pA.count, d)` then loops `count` times over an 8-element
/// array — with a garbage `count` that reads arbitrary memory and segfaults
/// nondeterministically (observed: the C crashes on some runs and not others,
/// depending on the caller's stack contents).
///
/// It is therefore characterised in `probe_ub_isolated.rs`, which runs each
/// library in a forked child so a crash is data rather than a dead test binary.
/// The **defined** public behaviour for an unknown type — `c2Collided` and
/// `omni_collide` returning a deterministic `0` without dereferencing the shape
/// pointer — is asserted in `level5_dispatch.rs` and `errors_phase_c.rs`.
#[test]
fn row74_invalid_type_tags_see_fork_isolated_probe() {
    // Nothing to assert in-process; this test documents the deliberate
    // relocation so the CONFIGS.md row is not silently dropped.
    let (c, r) = apis();
    // What IS safe and defined here: the dispatchers reject the tag first.
    for &bad in &BAD_TYPES {
        for &good in &VALID_TYPES {
            let z = ShapeBuf::zeroed();
            let cv = unsafe { (c.c2Collided)(z.as_ptr(), bad, z.as_ptr(), good) };
            let rv = unsafe { (r.c2Collided)(z.as_ptr(), bad, z.as_ptr(), good) };
            eq_int(&format!("row74 dispatcher {bad}/{}", type_name(good)), cv, rv);
            eq_int(&format!("row74 dispatcher {bad} is 0"), cv, 0);
        }
    }
}

/// Row 75 — negative radii flip the `dist > rA + rB` comparison, taking the
/// radius-shrink branch for shapes that are *not* actually close.
#[test]
fn row75_negative_radii() {
    let radii = [-0.0f32, -1e-7, -0.5, -1.0, -100.0, -1e30];
    for (ta, tb) in all_type_pairs() {
        // Only circle/capsule carry a radius; AABB proxies force radius 0.
        for &ra in &radii {
            for &rb in &radii {
                for ur in [0, 1] {
                    let o = Opts::plain(ur);
                    for &sep in &[0.0f32, 0.5, 1.0, 3.0, 100.0] {
                        let a = match ta {
                            C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
                                p: c2v { x: 0.0, y: 0.0 },
                                r: ra,
                            }),
                            C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
                                min: c2v { x: -1.0, y: -1.0 },
                                max: c2v { x: 1.0, y: 1.0 },
                            }),
                            _ => ShapeBuf::from_capsule(c2Capsule {
                                a: c2v { x: -1.0, y: 0.0 },
                                b: c2v { x: 1.0, y: 0.0 },
                                r: ra,
                            }),
                        };
                        let b = match tb {
                            C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
                                p: c2v { x: sep, y: 0.0 },
                                r: rb,
                            }),
                            C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
                                min: c2v { x: sep - 1.0, y: -1.0 },
                                max: c2v { x: sep + 1.0, y: 1.0 },
                            }),
                            _ => ShapeBuf::from_capsule(c2Capsule {
                                a: c2v { x: sep - 1.0, y: 0.0 },
                                b: c2v { x: sep + 1.0, y: 0.0 },
                                r: rb,
                            }),
                        };
                        diff(
                            &format!("row75 ra={ra} rb={rb} sep={sep}"),
                            &a,
                            ta,
                            None,
                            &b,
                            tb,
                            None,
                            &o,
                        );
                    }
                }
            }
        }
    }
}
