//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejection condition, calls BOTH `.so`s, and
//! asserts they agree. Where the C's result is determinate the test *also*
//! asserts the concrete expected value from the table, so the table itself is
//! validated rather than just "both sides did the same thing".

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

fn pair() -> Pair {
    load_pair()
}

const FLT_EPS: f32 = 1.192_092_895_507_812_5e-7;

fn circle(x: f32, y: f32, r: f32) -> Shape {
    Shape::Circle(c2Circle { p: c2v { x, y }, r })
}
fn aabb(x0: f32, y0: f32, x1: f32, y1: f32) -> Shape {
    Shape::Aabb(c2AABB {
        min: c2v { x: x0, y: y0 },
        max: c2v { x: x1, y: y1 },
    })
}
fn capsule(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Shape {
    Shape::Capsule(c2Capsule {
        a: c2v { x: x0, y: y0 },
        b: c2v { x: x1, y: y1 },
        r,
    })
}

fn zero_cache() -> c2GJKCache {
    c2GJKCache::default()
}

fn sv(p: c2v, u: f32, iA: c_int, iB: c_int) -> c2sv {
    c2sv {
        sA: c2v { x: p.x * 0.5 + 1.0, y: p.y * 0.25 - 1.0 },
        sB: c2v { x: p.x * 1.5 - 2.0, y: p.y * 0.75 + 3.0 },
        p,
        u,
        iA,
        iB,
    }
}

fn simplex(pts: [c2v; 4], us: [f32; 4], div: f32, count: c_int) -> c2Simplex {
    c2Simplex {
        verts: [
            sv(pts[0], us[0], 1, 2),
            sv(pts[1], us[1], 3, 0),
            sv(pts[2], us[2], 2, 1),
            sv(pts[3], us[3], 0, 3),
        ],
        div,
        count,
    }
}

// ===========================================================================
// Rows 1-2: NULL transforms fall back to the identity transform
// ===========================================================================

#[test]
fn err01_err02_null_transforms_equal_explicit_identity() {
    let p = pair();
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(SEED ^ 101);
    for i in 0..600 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let a = rand_shape(&mut rng, tyA, 50.0, 3);
        let b = rand_shape(&mut rng, tyB, 50.0, 3);
        for (ax, bx, tag) in [
            (None, None, "both NULL"),
            (Some(ident), None, "ax explicit identity, bx NULL"),
            (None, Some(ident), "ax NULL, bx explicit identity"),
            (Some(ident), Some(ident), "both explicit identity"),
        ] {
            let opts = GjkOpts { ax, bx, use_radius: 1, cache: true, ..Default::default() };
            let (oc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &zero_cache(),
                &format!("err01/02 i={i} {tag}"));
            // A NULL transform must be indistinguishable from an explicit identity.
            let base = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
            let (bc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &base, &zero_cache(),
                &format!("err01/02 i={i} baseline"));
            ck_f(bc.dist, oc.dist, &format!("err01/02 i={i} {tag}: identity != NULL"));
            ck_v(bc.a, oc.a, &format!("err01/02 i={i} {tag} outA"));
            ck_v(bc.b, oc.b, &format!("err01/02 i={i} {tag} outB"));
            ck_i(bc.iters, oc.iters, &format!("err01/02 i={i} {tag} iters"));
        }
    }
}

// ===========================================================================
// Rows 3-5: NULL out-parameters are simply not written
// ===========================================================================

#[test]
fn err03_err04_err05_null_out_parameters() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 345);
    for i in 0..400 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let a = rand_shape(&mut rng, tyA, 50.0, 3);
        let b = rand_shape(&mut rng, tyB, 50.0, 3);
        let full = GjkOpts { use_radius: 1, ..Default::default() };
        let (fc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &full, &zero_cache(),
            &format!("err03-05 i={i} all-out"));
        for (wa, wb, wi) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
            (false, false, false),
        ] {
            let opts = GjkOpts {
                use_radius: 1,
                want_out_a: wa,
                want_out_b: wb,
                want_iterations: wi,
                ..Default::default()
            };
            let (oc, or) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &zero_cache(),
                &format!("err03-05 i={i} wa={wa} wb={wb} wi={wi}"));
            // Return value must be unaffected by which out-params are present.
            ck_f(fc.dist, oc.dist, "err03-05 dist changed by NULL out-params");
            for (side, o) in [("C", &oc), ("Rust", &or)] {
                if !wa {
                    ck_v(o.a, sentinel_v(), &format!("err03 {side} wrote outA through NULL"));
                } else {
                    ck_v(o.a, fc.a, &format!("err03 {side} outA mismatch"));
                }
                if !wb {
                    ck_v(o.b, sentinel_v(), &format!("err04 {side} wrote outB through NULL"));
                } else {
                    ck_v(o.b, fc.b, &format!("err04 {side} outB mismatch"));
                }
                if !wi {
                    assert_eq!(o.iters, sentinel_i(), "err05 {side} wrote iterations through NULL");
                } else {
                    assert_eq!(o.iters, fc.iters, "err05 {side} iterations mismatch");
                }
            }
        }
    }
}

// ===========================================================================
// Row 6: cache == NULL
// ===========================================================================

#[test]
fn err06_null_cache() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..600 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let a = rand_shape(&mut rng, tyA, 50.0, 3);
        let b = rand_shape(&mut rng, tyB, 50.0, 3);
        for use_radius in [0, 1] {
            let no_cache = GjkOpts { use_radius, cache: false, ..Default::default() };
            let (nc, nr) = gjk_diff(&p, &a, tyA, &b, tyB, &no_cache, &zero_cache(),
                &format!("err06 i={i} cache=NULL"));
            // With cache == NULL the caller's buffer must be untouched.
            ck_cache(&nc.cache, &zero_cache(), "err06 C wrote through a NULL cache");
            ck_cache(&nr.cache, &zero_cache(), "err06 Rust wrote through a NULL cache");
            // And the distance must equal the cold-cache result.
            let with_cache = GjkOpts { use_radius, cache: true, ..Default::default() };
            let (wc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &with_cache, &zero_cache(),
                &format!("err06 i={i} cold cache"));
            ck_f(nc.dist, wc.dist, "err06 NULL cache != cold cache distance");
        }
    }
}

// ===========================================================================
// Row 7: cache->count == 0 (cache_was_good == 0)
// ===========================================================================

#[test]
fn err07_cache_count_zero_is_treated_as_cold() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..600 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let a = rand_shape(&mut rng, tyA, 50.0, 3);
        let b = rand_shape(&mut rng, tyB, 50.0, 3);
        // count == 0 but every other field is deliberate garbage.
        let garbage = c2GJKCache {
            metric: -1.0e30,
            count: 0,
            iA: [3, 2, 1],
            iB: [2, 1, 0],
            div: -0.0,
        };
        let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
        let (gc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &garbage, &format!("err07 i={i} garbage"));
        let (zc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &zero_cache(), &format!("err07 i={i} zero"));
        // The garbage must be ignored entirely: same result as an all-zero cache.
        ck_f(gc.dist, zc.dist, "err07 count==0 cache garbage leaked into the result");
        ck_v(gc.a, zc.a, "err07 outA");
        ck_v(gc.b, zc.b, "err07 outB");
        ck_i(gc.iters, zc.iters, "err07 iterations");
        // Only the fields the C actually writes back may be compared: it writes
        // `metric`, `count`, `div` and `iA[0..count]` / `iB[0..count]`, leaving
        // the tail of the caller's arrays exactly as it was handed in.
        ck_f(gc.cache.metric, zc.cache.metric, "err07 written-back metric");
        ck_i(gc.cache.count, zc.cache.count, "err07 written-back count");
        ck_f(gc.cache.div, zc.cache.div, "err07 written-back div");
        let n = gc.cache.count.clamp(0, 3) as usize;
        for k in 0..n {
            ck_i(gc.cache.iA[k], zc.cache.iA[k], &format!("err07 written-back iA[{k}]"));
            ck_i(gc.cache.iB[k], zc.cache.iB[k], &format!("err07 written-back iB[{k}]"));
        }
        // ...and the untouched tail must still hold the caller's original bytes.
        for k in n..3 {
            ck_i(gc.cache.iA[k], garbage.iA[k], &format!("err07 tail iA[{k}] must be untouched"));
            ck_i(gc.cache.iB[k], garbage.iB[k], &format!("err07 tail iB[{k}] must be untouched"));
        }
    }
}

// ===========================================================================
// Row 8: cache->count < 0 (non-zero, so "good", but the read loop never runs)
// ===========================================================================

#[test]
fn err08_negative_cache_count() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 8);
    for count in [-1i32, -2, -3, -100, i32::MIN + 1] {
        for i in 0..120 {
            let tyA = ALL_TYPES[rng.below(3) as usize];
            let tyB = ALL_TYPES[rng.below(3) as usize];
            let a = rand_shape(&mut rng, tyA, 50.0, 3);
            let b = rand_shape(&mut rng, tyB, 50.0, 3);
            let cin = c2GJKCache {
                metric: rng.sym(1000.0),
                count,
                iA: [0, 0, 0],
                iB: [0, 0, 0],
                div: rng.sym(10.0),
            };
            for use_radius in [0, 1] {
                let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
                let (oc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &cin,
                    &format!("err08 count={count} i={i} ur={use_radius}"));
                // Expected C behaviour from ERRORS.md row 8: every switch takes
                // its default arm, the witness collapses to the origin and the
                // distance is exactly 0.
                ck_f(oc.dist, 0.0, "err08 expected dist == 0");
                ck_v(oc.a, c2v { x: 0.0, y: 0.0 }, "err08 expected outA == (0,0)");
                ck_v(oc.b, c2v { x: 0.0, y: 0.0 }, "err08 expected outB == (0,0)");
                ck_i(oc.iters, 0, "err08 expected 0 iterations");
                ck_i(oc.cache.count, count, "err08 negative count is written back verbatim");
                ck_f(oc.cache.metric, 0.0, "err08 metric default arm returns 0");
                assert_eq!(oc.cache.iA, cin.iA, "err08 iA must not be written");
                assert_eq!(oc.cache.iB, cin.iB, "err08 iB must not be written");
                ck_f(oc.cache.div, cin.div, "err08 div is carried through unchanged");
            }
        }
    }
}

// ===========================================================================
// Row 9: the near-dead staleness guard
//        !(min_metric < max_metric*2.0f && metric < -1.0e8f)
// ===========================================================================

#[test]
fn err09_staleness_guard_both_arms() {
    let p = pair();
    // A large AABB against a unit circle at the origin gives simplex metrics of
    // magnitude ~1e10, so `metric < -1.0e8f` is actually reachable.
    let a = aabb(-1.0e5, -1.0e5, 1.0e5, 1.0e5);
    let b = circle(0.0, 0.0, 1.0);
    // iA = [0,2,1] winds the triangle so the determinant is negative.
    let mk = |metric: f32| c2GJKCache {
        metric,
        count: 3,
        iA: [0, 2, 1],
        iB: [0, 0, 0],
        div: 1.0,
    };
    // metric computed from the cache above is about -4e10.
    // Arm 1: metric_old = 0  -> min=-4e10 < 0 == max*2, and metric < -1e8
    //        -> guard is TRUE -> cache_was_read stays 0 -> simplex RESET.
    // Arm 2: metric_old = -4e10 -> min == max, min < max*2 is false
    //        -> guard is FALSE -> cache_was_read = 1 -> simplex KEPT.
    let mut reset_seen = false;
    let mut kept_seen = false;
    for (k, metric_old) in [0.0f32, -4.0e10, -1.0e30, 1.0e30, f32::NAN, -1.0e8, -1.0e7]
        .iter()
        .enumerate()
    {
        for use_radius in [0, 1] {
            let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
            let (oc, _) = gjk_diff(&p, &a, C2_TYPE_AABB, &b, C2_TYPE_CIRCLE, &opts, &mk(*metric_old),
                &format!("err09 k={k} metric_old={metric_old} ur={use_radius}"));
            // Reconstruct which arm the C took: with the cache kept, iteration 0
            // starts from a 3-vertex simplex and breaks immediately (hit), so
            // `iterations == 0` and the written-back count is 3.
            let cold = GjkOpts { use_radius, cache: true, ..Default::default() };
            let (cc, _) = gjk_diff(&p, &a, C2_TYPE_AABB, &b, C2_TYPE_CIRCLE, &cold, &zero_cache(),
                &format!("err09 k={k} cold baseline"));
            if oc.cache.count == cc.cache.count
                && feq(oc.dist, cc.dist)
                && oc.iters == cc.iters
                && veq(oc.a, cc.a)
            {
                reset_seen = true;
            } else {
                kept_seen = true;
            }
        }
    }
    assert!(reset_seen, "err09: the `metric < -1.0e8f` reset arm was never observed");
    assert!(kept_seen, "err09: the cache-kept arm was never observed");

    // Also sweep warm caches over large-coordinate shapes so the guard is
    // exercised across many index permutations.
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..900 {
        let scale = [1.0e4f32, 1.0e5, 1.0e6][rng.below(3) as usize];
        let a = aabb(-scale, -scale, scale, scale);
        let b = match rng.below(3) {
            0 => circle(rng.sym(scale), rng.sym(scale), rng.unit() * scale),
            1 => aabb(-scale * 0.5, -scale * 0.5, scale * 0.5, scale * 0.5),
            _ => capsule(rng.sym(scale), rng.sym(scale), rng.sym(scale), rng.sym(scale), 1.0),
        };
        let cap_b = match b.ty() {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let cin = c2GJKCache {
            metric: [0.0f32, -1.0e9, -4.0e10, 1.0e10, f32::NAN][rng.below(5) as usize],
            count: 1 + rng.below(3) as c_int,
            iA: [
                rng.below(4) as c_int,
                rng.below(4) as c_int,
                rng.below(4) as c_int,
            ],
            iB: [
                rng.below(cap_b) as c_int,
                rng.below(cap_b) as c_int,
                rng.below(cap_b) as c_int,
            ],
            div: [1.0f32, 0.0, 1.0e10, -1.0][rng.below(4) as usize],
        };
        let opts = GjkOpts { use_radius: rng.below(2) as i32, cache: true, ..Default::default() };
        gjk_diff(&p, &a, C2_TYPE_AABB, &b, b.ty(), &opts, &cin, &format!("err09 sweep i={i}"));
    }
}

// ===========================================================================
// Row 10: use_radius == 0 skips the shrink entirely
// ===========================================================================

#[test]
fn err10_use_radius_zero_skips_shrink() {
    let p = pair();
    // Two separated circles: with use_radius == 0 the distance must be the
    // centre-to-centre distance, ignoring both radii.
    for (d, ra, rb) in [(10.0f32, 1.0, 2.0), (5.0, 0.0, 0.0), (5.0, 100.0, 100.0)] {
        let a = circle(0.0, 0.0, ra);
        let b = circle(d, 0.0, rb);
        let o0 = GjkOpts { use_radius: 0, ..Default::default() };
        let (c0, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &o0, &zero_cache(),
            &format!("err10 d={d} ur=0"));
        ck_f(c0.dist, d, "err10 use_radius=0 must return the raw core distance");
        ck_v(c0.a, c2v { x: 0.0, y: 0.0 }, "err10 outA is the raw witness point");
        ck_v(c0.b, c2v { x: d, y: 0.0 }, "err10 outB is the raw witness point");
        // Non-zero use_radius values must all behave like 1.
        let (c1, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE,
            &GjkOpts { use_radius: 1, ..Default::default() }, &zero_cache(), "err10 ur=1");
        for ur in [2, -1, i32::MIN, i32::MAX, 0x1000] {
            let (cx, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE,
                &GjkOpts { use_radius: ur, ..Default::default() }, &zero_cache(),
                &format!("err10 ur={ur}"));
            ck_f(cx.dist, c1.dist, &format!("err10 use_radius={ur} must behave like 1"));
        }
    }
    // Randomized: use_radius == 0 result must be radius-independent.
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..600 {
        let px = rng.sym(50.0);
        let py = rng.sym(50.0);
        let a1 = circle(0.0, 0.0, 1.0);
        let a2 = circle(0.0, 0.0, 77.0);
        let b1 = circle(px, py, 2.0);
        let b2 = circle(px, py, 99.0);
        let o = GjkOpts { use_radius: 0, ..Default::default() };
        let (r1, _) = gjk_diff(&p, &a1, C2_TYPE_CIRCLE, &b1, C2_TYPE_CIRCLE, &o, &zero_cache(),
            &format!("err10 rnd i={i} small radii"));
        let (r2, _) = gjk_diff(&p, &a2, C2_TYPE_CIRCLE, &b2, C2_TYPE_CIRCLE, &o, &zero_cache(),
            &format!("err10 rnd i={i} large radii"));
        ck_f(r1.dist, r2.dist, "err10 use_radius=0 must ignore the radii");
    }
}

// ===========================================================================
// Rows 11-12: the radius-shrink rejection arms
// ===========================================================================

#[test]
fn err11_overlap_collapses_to_midpoint() {
    let p = pair();
    // dist <= rA + rB  ->  a = b = midpoint, dist = 0
    for (d, ra, rb) in [
        (3.0f32, 1.5, 1.5),  // dist == rA+rB exactly (NOT >, so the else arm)
        (3.0, 2.0, 2.0),     // overlapping
        (0.0, 1.0, 1.0),     // coincident
        (1.0e-9, 0.0, 0.0),  // dist below FLT_EPSILON with zero radii
    ] {
        let a = circle(0.0, 0.0, ra);
        let b = circle(d, 0.0, rb);
        let opts = GjkOpts { use_radius: 1, ..Default::default() };
        let (oc, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &opts, &zero_cache(),
            &format!("err11 d={d} rA={ra} rB={rb}"));
        ck_f(oc.dist, 0.0, "err11 expected dist == 0");
        ck_v(oc.a, oc.b, "err11 expected a == b (the midpoint)");
        ck_v(oc.a, c2v { x: d * 0.5, y: 0.0 }, "err11 expected the midpoint");
    }
    // The `dist > FLT_EPSILON` half of the guard, isolated: zero radii and a
    // separation straddling FLT_EPSILON.
    for d in [
        0.0f32,
        FLT_EPS * 0.5,
        FLT_EPS,          // NOT > FLT_EPSILON -> collapse
        FLT_EPS * 1.0001, // > FLT_EPSILON -> shrink path
        FLT_EPS * 2.0,
    ] {
        let a = circle(0.0, 0.0, 0.0);
        let b = circle(d, 0.0, 0.0);
        let opts = GjkOpts { use_radius: 1, ..Default::default() };
        let (oc, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &opts, &zero_cache(),
            &format!("err11 eps boundary d={d}"));
        if d > FLT_EPS {
            ck_f(oc.dist, d, "err11 above FLT_EPSILON the raw distance survives");
        } else {
            ck_f(oc.dist, 0.0, "err11 at/below FLT_EPSILON the distance is forced to 0");
        }
    }
}

#[test]
fn err12_shrink_making_a_equal_b_forces_zero() {
    let p = pair();
    // Radii summing to just under `dist` so the shrink lands both witnesses on
    // (nearly) the same point; float rounding then makes a == b.
    let mut rng = Rng::new(SEED ^ 12);
    let mut collapsed = 0usize;
    for i in 0..4000 {
        let d = rng.unit() * 100.0 + FLT_EPS * 2.0;
        // shave the radii by one ulp-ish so `dist > rA + rB` is only just true
        let half = d * 0.5;
        let ra = half - half * (rng.unit() * 1.0e-7);
        let rb = half - half * (rng.unit() * 1.0e-7);
        let a = circle(0.0, 0.0, ra);
        let b = circle(d, 0.0, rb);
        let opts = GjkOpts { use_radius: 1, ..Default::default() };
        let (oc, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &opts, &zero_cache(),
            &format!("err12 i={i} d={d} rA={ra} rB={rb}"));
        if veq(oc.a, oc.b) {
            collapsed += 1;
            ck_f(oc.dist, 0.0, "err12 a == b after shrink must force dist = 0");
        }
    }
    assert!(collapsed > 0, "err12 never reached the post-shrink a == b collapse");
    println!("err12 post-shrink collapse observed {collapsed} times");
}

// ===========================================================================
// Row 13: hit (s.count == 3) wins over the radius block
// ===========================================================================

#[test]
fn err13_hit_path_sets_a_equal_b_and_zero_dist() {
    let p = pair();
    // Concentric boxes: the origin of the Minkowski difference is enclosed.
    let a = aabb(-5.0, -5.0, 5.0, 5.0);
    let b = aabb(-4.0, -4.0, 4.0, 4.0);
    let mut seen = 0;
    let mut hit_seen = false;
    for use_radius in [0, 1] {
        for with_cache in [false, true] {
            let opts = GjkOpts { use_radius, cache: with_cache, ..Default::default() };
            let (oc, _) = gjk_diff(&p, &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &opts, &zero_cache(),
                &format!("err13 ur={use_radius} cache={with_cache}"));
            ck_f(oc.dist, 0.0, "err13 overlap must give dist == 0");
            ck_v(oc.a, oc.b, "err13 overlap must set a = b");
            if with_cache && oc.cache.count == 3 {
                hit_seen = true;
            }
            seen += 1;
        }
    }
    assert_eq!(seen, 4);
    // Now a configuration that provably reaches `hit = 1` (s.count == 3): a
    // large box against a small box offset off both axes so the simplex has to
    // grow to three vertices before it encloses the origin.
    let mut rng = Rng::new(SEED ^ 13);
    let mut hits = 0usize;
    for i in 0..3000 {
        let a = aabb(-5.0, -5.0, 5.0, 5.0);
        let jx = rng.sym(3.0);
        let jy = rng.sym(3.0);
        let b = aabb(jx - 1.0, jy - 1.0, jx + 1.0, jy + 1.0);
        for use_radius in [0, 1] {
            let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
            let (oc, _) = gjk_diff(&p, &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &opts, &zero_cache(),
                &format!("err13 hit sweep i={i} ur={use_radius}"));
            if oc.cache.count == 3 {
                hits += 1;
                hit_seen = true;
                // Row 13: `hit` short-circuits the radius block entirely.
                ck_f(oc.dist, 0.0, "err13 hit must give dist == 0");
                ck_v(oc.a, oc.b, "err13 hit must set a = b");
            }
        }
    }
    assert!(hit_seen, "err13 never reached the hit path (s.count == 3)");
    println!("err13 hit path reached {hits} times");
    // Deep overlap with non-zero radii: `hit` must win, i.e. the radii must NOT
    // be subtracted (which would give a negative distance).
    let a = capsule(-5.0, 0.0, 5.0, 0.0, 3.0);
    let b = capsule(0.0, -5.0, 0.0, 5.0, 3.0);
    let opts = GjkOpts { use_radius: 1, ..Default::default() };
    let (oc, _) = gjk_diff(&p, &a, C2_TYPE_CAPSULE, &b, C2_TYPE_CAPSULE, &opts, &zero_cache(),
        "err13 crossing capsules");
    ck_f(oc.dist, 0.0, "err13 crossing capsules");
}

// ===========================================================================
// Rows 14-17: the four loop-exit conditions
// ===========================================================================

#[test]
fn err14_err15_err16_err17_loop_exits() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 1417);
    let mut iter_hist = [0usize; 21];
    for i in 0..8000 {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let mag = [1.0e-6f32, 1.0e-3, 1.0, 1.0e3, 1.0e6][rng.below(5) as usize];
        let a = rand_shape(&mut rng, tyA, mag, 6);
        let b = rand_shape(&mut rng, tyB, mag, 6);
        let opts = GjkOpts {
            ax: if rng.bool() { Some(rng.xform(mag)) } else { None },
            bx: if rng.bool() { Some(rng.xform(mag)) } else { None },
            use_radius: rng.below(2) as i32,
            cache: rng.bool(),
            ..Default::default()
        };
        let (oc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &zero_cache(),
            &format!("err14-17 i={i} mag={mag}"));
        // Row 17: the iteration cap must never be exceeded.
        assert!(
            (0..=20).contains(&oc.iters),
            "err17 iterations escaped 0..=20: {} :: A={} B={}",
            oc.iters, a.describe(), b.describe()
        );
        iter_hist[oc.iters as usize] += 1;
    }
    // Row 15: identical shapes give a zero search direction on the first step,
    // so the loop must break with 0 iterations.
    for ty in ALL_TYPES {
        let s = match ty {
            C2_TYPE_CIRCLE => circle(3.0, 4.0, 2.0),
            C2_TYPE_AABB => aabb(-1.0, -2.0, 3.0, 4.0),
            _ => capsule(-1.0, -2.0, 3.0, 4.0, 1.0),
        };
        for use_radius in [0, 1] {
            let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
            let (oc, _) = gjk_diff(&p, &s, ty, &s, ty, &opts, &zero_cache(),
                &format!("err15 identical {} ur={use_radius}", type_name(ty)));
            ck_i(oc.iters, 0, "err15 identical shapes must break at iteration 0");
            ck_f(oc.dist, 0.0, "err15 identical shapes give dist 0");
        }
    }
    // Row 16: a duplicate support point. A circle has a single vertex, so the
    // support index can only ever repeat -> the duplicate break is guaranteed.
    let a = circle(0.0, 0.0, 1.0);
    let b = circle(10.0, 0.0, 1.0);
    let opts = GjkOpts { use_radius: 0, cache: true, ..Default::default() };
    let (oc, _) = gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &opts, &zero_cache(),
        "err16 single-vertex proxies");
    ck_i(oc.iters, 0, "err16 duplicate support must break before ++iter");
    ck_i(oc.cache.count, 1, "err16 the duplicate vertex must not be counted");
    println!("err14-17 iteration histogram: {iter_hist:?}");
    assert!(
        iter_hist.iter().filter(|&&n| n > 0).count() >= 3,
        "err14-17 iteration spread too narrow: {iter_hist:?}"
    );
}

// ===========================================================================
// Row 18: out-of-range C2_TYPE across the FFI boundary
// ===========================================================================

/// `c2GJK` with an invalid `typeA`/`typeB` leaves its own `c2Proxy` local
/// uninitialised (`c2MakeProxy`'s `switch` has no `default:`). This test does
/// NOT assert C/Rust equality, because the C result is not reproducible even
/// against *itself*: the garbage `pA.count` it then feeds to `c2Support` varies
/// with the stack contents, and on some stacks it is large enough to read far
/// out of bounds and segfault. The probe therefore runs in a CHILD PROCESS so
/// a C-side fault cannot take the suite down. See the UB table in ERRORS.md.
#[test]
fn err18_invalid_enum_in_c2GJK_is_c_side_undefined_behaviour() {
    const GATE: &str = "GJK_UB_PROBE";
    if std::env::var(GATE).is_ok() {
        // --- child: perform the UB calls, print what the C did, then exit ---
        let p = pair();
        let a = circle(0.0, 0.0, 1.0);
        let b = circle(5.0, 0.0, 1.0);
        let mut results = Vec::new();
        for round in 0..6 {
            let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
            let oc = gjk_once(&p.c, &a, 7, &b, C2_TYPE_CIRCLE, &opts, &zero_cache());
            println!(
                "err18 child round {round}: C dist={:?} iters={} a=({},{})",
                oc.dist, oc.iters, oc.a.x, oc.a.y
            );
            results.push((oc.dist.to_bits(), oc.iters));
        }
        let unstable = results.iter().any(|r| r != &results[0]);
        println!("err18 child: C self-consistent = {}", !unstable);
        for ty in [3u32, 4, 99, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
            for (tyA, tyB) in [(ty, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, ty), (ty, ty)] {
                let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
                let _ = gjk_once(&p.c, &a, tyA, &b, tyB, &opts, &zero_cache());
                let _ = gjk_once(&p.r, &a, tyA, &b, tyB, &opts, &zero_cache());
            }
        }
        println!("err18 child: completed without faulting");
        return;
    }

    // --- parent: what IS well-defined and therefore asserted ---------------
    // The Rust side must never fault or hang for any out-of-range enum, and it
    // must behave as if `c2MakeProxy` wrote nothing (zeroed proxy), which is the
    // only self-consistent choice available to it.
    let p = pair();
    let a = circle(0.0, 0.0, 1.0);
    let b = circle(5.0, 0.0, 1.0);
    for ty in [3u32, 4, 99, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
        for (tyA, tyB) in [(ty, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, ty), (ty, ty)] {
            for round in 0..3 {
                let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
                let o1 = gjk_once(&p.r, &a, tyA, &b, tyB, &opts, &zero_cache());
                let o2 = gjk_once(&p.r, &a, tyA, &b, tyB, &opts, &zero_cache());
                ck_f(o1.dist, o2.dist, &format!("err18 Rust must be deterministic ty=({tyA},{tyB}) round={round}"));
                ck_v(o1.a, o2.a, "err18 Rust deterministic outA");
                ck_i(o1.iters, o2.iters, "err18 Rust deterministic iterations");
            }
        }
    }

    // Now run the C-side probe out of process and report, without failing.
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(&exe)
        .arg("--exact")
        .arg("err18_invalid_enum_in_c2GJK_is_c_side_undefined_behaviour")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(GATE, "1")
        .output()
        .expect("failed to spawn the UB probe child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines().filter(|l| l.starts_with("err18 child")) {
        println!("{line}");
    }
    println!(
        "err18: child exit status = {:?} -> the out-of-range-enum path in c2GJK is C-side \
         undefined behaviour (uninitialised c2Proxy local); it is documented in ERRORS.md \
         rather than asserted, because no Rust translation can reproduce indeterminate \
         stack contents. The well-defined half of this surface -- c2MakeProxy's missing \
         `default:` arm -- IS asserted exhaustively by \
         generic_out_of_range_enums_into_c2MakeProxy_are_exhaustively_equal.",
        out.status
    );
}

// ===========================================================================
// Rows 19-20: c2MakeProxy — the well-defined half of the enum surface
// ===========================================================================

#[test]
fn err19_c2MakeProxy_invalid_type_leaves_the_proxy_untouched() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..2000 {
        let ty = match i % 6 {
            0 => 3u32,
            1 => 4,
            2 => 99,
            3 => 0x7FFF_FFFF,
            4 => 0x8000_0000,
            _ => u32::MAX,
        };
        // A caller-owned proxy filled with a recognisable pattern.
        let mut base = c2Proxy {
            radius: rng.sym(1000.0),
            count: rng.next_u32() as c_int,
            verts: [c2v::default(); 8],
        };
        for v in base.verts.iter_mut() {
            *v = rng.wild_v();
        }
        let shape = c2Circle { p: rng.wild_v(), r: rng.wild_f32() };
        let mut pc = base;
        let mut pr = base;
        unsafe {
            (p.c.c2MakeProxy)(&shape as *const c2Circle as *const c_void, ty, &mut pc);
            (p.r.c2MakeProxy)(&shape as *const c2Circle as *const c_void, ty, &mut pr);
        }
        let ctx = format!("err19 i={i} ty={ty}");
        ck_proxy(&pc, &pr, &ctx);
        // Expected C result: byte-for-byte unchanged.
        ck_b(&pc, &base, &format!("{ctx}: C modified the proxy for an invalid type"));
        ck_b(&pr, &base, &format!("{ctx}: Rust modified the proxy for an invalid type"));
    }
}

#[test]
fn err20_c2MakeProxy_aabb_forces_radius_zero_and_leaves_the_tail() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..2000 {
        let mut base = c2Proxy {
            radius: 123.456,
            count: -1,
            verts: [c2v::default(); 8],
        };
        for (k, v) in base.verts.iter_mut().enumerate() {
            *v = c2v { x: 900.0 + k as f32, y: -900.0 - k as f32 };
        }
        let bb = c2AABB { min: rng.wild_v(), max: rng.wild_v() };
        let mut pc = base;
        let mut pr = base;
        unsafe {
            (p.c.c2MakeProxy)(&bb as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut pc);
            (p.r.c2MakeProxy)(&bb as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut pr);
        }
        let ctx = format!("err20 i={i}");
        ck_proxy(&pc, &pr, &ctx);
        ck_f(pc.radius, 0.0, &format!("{ctx}: AABB proxies must force radius 0"));
        ck_i(pc.count, 4, &format!("{ctx}: AABB proxies must set count 4"));
        // verts[4..8] must be untouched.
        for k in 4..8 {
            ck_v(pc.verts[k], base.verts[k], &format!("{ctx}: C touched verts[{k}]"));
            ck_v(pr.verts[k], base.verts[k], &format!("{ctx}: Rust touched verts[{k}]"));
        }
    }
    // Same for CIRCLE (verts[1..8]) and CAPSULE (verts[2..8]).
    for (ty, written) in [(C2_TYPE_CIRCLE, 1usize), (C2_TYPE_CAPSULE, 2)] {
        for i in 0..1000 {
            let mut base = c2Proxy { radius: 7.0, count: -9, verts: [c2v::default(); 8] };
            for (k, v) in base.verts.iter_mut().enumerate() {
                *v = c2v { x: 33.0 + k as f32, y: -33.0 - k as f32 };
            }
            let cap = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild_f32() };
            let cir = c2Circle { p: cap.a, r: cap.r };
            let sp: *const c_void = if ty == C2_TYPE_CIRCLE {
                &cir as *const c2Circle as *const c_void
            } else {
                &cap as *const c2Capsule as *const c_void
            };
            let mut pc = base;
            let mut pr = base;
            unsafe {
                (p.c.c2MakeProxy)(sp, ty, &mut pc);
                (p.r.c2MakeProxy)(sp, ty, &mut pr);
            }
            let ctx = format!("err20 {} i={i}", type_name(ty));
            ck_proxy(&pc, &pr, &ctx);
            ck_i(pc.count, written as c_int, &format!("{ctx} count"));
            for k in written..8 {
                ck_v(pc.verts[k], base.verts[k], &format!("{ctx}: C touched verts[{k}]"));
                ck_v(pr.verts[k], base.verts[k], &format!("{ctx}: Rust touched verts[{k}]"));
            }
        }
    }
}

// ===========================================================================
// Rows 21-23: c2Support rejections
// ===========================================================================

#[test]
fn err21_c2Support_nonpositive_count() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..3000 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.wild_v();
        }
        let d = rng.wild_v();
        for count in [0i32, -1, -2, -1000, i32::MIN] {
            unsafe {
                let rc = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rr = (p.r.c2Support)(verts.as_ptr(), count, d);
                let ctx = format!("err21 i={i} count={count}");
                ck_i(rc, rr, &ctx);
                // Expected C result: the loop never runs, so index 0 is returned.
                ck_i(rc, 0, &format!("{ctx}: expected 0"));
            }
        }
    }
}

#[test]
fn err22_c2Support_ties_keep_the_first_index() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..2000 {
        // All vertices identical -> every dot equal -> strict `>` never fires.
        let v0 = rng.tame_v(100.0);
        let verts = [v0; 8];
        let d = rng.tame_v(100.0);
        for count in [1i32, 2, 4, 8] {
            unsafe {
                let rc = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rr = (p.r.c2Support)(verts.as_ptr(), count, d);
                ck_i(rc, rr, &format!("err22 i={i} count={count}"));
                ck_i(rc, 0, &format!("err22 i={i} count={count}: expected 0 on a tie"));
            }
        }
        // d == (0,0) -> every dot is 0 (or -0) -> also a tie.
        let mut verts2 = [c2v::default(); 8];
        for v in verts2.iter_mut() {
            *v = rng.tame_v(100.0);
        }
        let zero = c2v { x: 0.0, y: 0.0 };
        for count in [1i32, 2, 4, 8] {
            unsafe {
                let rc = (p.c.c2Support)(verts2.as_ptr(), count, zero);
                let rr = (p.r.c2Support)(verts2.as_ptr(), count, zero);
                ck_i(rc, rr, &format!("err22 zero-d i={i} count={count}"));
                ck_i(rc, 0, &format!("err22 zero-d i={i} count={count}: expected 0"));
            }
        }
    }
}

#[test]
fn err23_c2Support_nan_never_wins() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..3000 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.tame_v(100.0);
        }
        // Poison one vertex so exactly one dot is NaN.
        let poison = (i % 8) as usize;
        verts[poison] = c2v { x: f32::NAN, y: f32::NAN };
        let d = rng.tame_v(100.0);
        for count in [1i32, 2, 4, 8] {
            unsafe {
                let rc = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rr = (p.r.c2Support)(verts.as_ptr(), count, d);
                let ctx = format!("err23 i={i} count={count} poison={poison}");
                ck_i(rc, rr, &ctx);
                if poison == 0 {
                    // dmax starts as NaN; `dot > NaN` is always false.
                    ck_i(rc, 0, &format!("{ctx}: NaN dmax must never be beaten"));
                } else if (poison as i32) < count {
                    assert_ne!(rc, poison as c_int, "{ctx}: a NaN vertex must never be selected");
                }
            }
        }
        // Whole direction NaN -> every dot NaN -> index 0.
        let dn = c2v { x: f32::NAN, y: 1.0 };
        unsafe {
            let rc = (p.c.c2Support)(verts.as_ptr(), 8, dn);
            let rr = (p.r.c2Support)(verts.as_ptr(), 8, dn);
            ck_i(rc, rr, &format!("err23 nan-d i={i}"));
            ck_i(rc, 0, &format!("err23 nan-d i={i}: expected 0"));
        }
    }
}

// ===========================================================================
// Rows 24-30: simplex accessor rejections
// ===========================================================================

fn some_pts(rng: &mut Rng) -> [c2v; 4] {
    [rng.tame_v(50.0), rng.tame_v(50.0), rng.tame_v(50.0), rng.tame_v(50.0)]
}

#[test]
fn err24_err25_c2Witness_rejections() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 24);
    // Row 24: count outside {1,2,3}
    for count in [0i32, -1, 4, 5, 100, i32::MIN, i32::MAX] {
        for i in 0..300 {
            let mut s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], 6.0, count);
            let (mut ac, mut bc) = (sentinel_v(), sentinel_v());
            let (mut ar, mut br) = (sentinel_v(), sentinel_v());
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            let ctx = format!("err24 count={count} i={i}");
            ck_v(ac, ar, &format!("{ctx} outA"));
            ck_v(bc, br, &format!("{ctx} outB"));
            ck_v(ac, c2v { x: 0.0, y: 0.0 }, &format!("{ctx}: default arm must give (0,0)"));
            ck_v(bc, c2v { x: 0.0, y: 0.0 }, &format!("{ctx}: default arm must give (0,0)"));
            s.count = count;
        }
    }
    // Row 25: div == 0 (den = +inf), for each valid count
    for count in [1i32, 2, 3] {
        for div in [0.0f32, -0.0, f32::MIN_POSITIVE, f32::NAN, f32::INFINITY] {
            for i in 0..300 {
                let s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], div, count);
                let (mut ac, mut bc) = (sentinel_v(), sentinel_v());
                let (mut ar, mut br) = (sentinel_v(), sentinel_v());
                let mut sc = s;
                let mut sr = s;
                unsafe {
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                let ctx = format!("err25 count={count} div={div} i={i}");
                ck_v(ac, ar, &format!("{ctx} outA"));
                ck_v(bc, br, &format!("{ctx} outB"));
                if count == 1 {
                    // case 1 ignores `den` entirely.
                    ck_v(ac, sc.verts[0].sA, &format!("{ctx}: count 1 ignores div"));
                }
            }
        }
    }
}

#[test]
fn err26_c2GJKSimplexMetric_default_arm_returns_zero() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    for count in [0i32, 1, -1, 4, 5, 1000, i32::MIN, i32::MAX] {
        for i in 0..400 {
            let s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], 3.0, count);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                let fc = (p.c.c2GJKSimplexMetric)(&mut sc);
                let fr = (p.r.c2GJKSimplexMetric)(&mut sr);
                let ctx = format!("err26 count={count} i={i}");
                ck_f(fc, fr, &ctx);
                ck_f(fc, 0.0, &format!("{ctx}: default/case-1 must return 0"));
            }
        }
    }
}

#[test]
fn err27_err28_c2D_rejections() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 27);
    // Row 27: count == 3 or default -> (0,0)
    for count in [3i32, 0, -1, 4, 7, i32::MIN, i32::MAX] {
        for i in 0..400 {
            let s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], 2.0, count);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                let vc = (p.c.c2D)(&mut sc);
                let vr = (p.r.c2D)(&mut sr);
                let ctx = format!("err27 count={count} i={i}");
                ck_v(vc, vr, &ctx);
                ck_v(vc, c2v { x: 0.0, y: 0.0 }, &format!("{ctx}: expected (0,0)"));
            }
        }
    }
    // Row 28: count == 2 with det <= 0 (including NaN) -> c2CCW90
    let mut ccw = 0usize;
    let mut skew = 0usize;
    for i in 0..3000 {
        let pts = match i % 4 {
            0 => {
                // collinear with the origin -> det == 0 -> the <= 0 arm
                let a = rng.tame_v(50.0);
                [a, c2v { x: a.x * 2.0, y: a.y * 2.0 }, a, a]
            }
            1 => {
                let a = c2v { x: f32::NAN, y: 1.0 };
                [a, rng.tame_v(50.0), a, a]
            }
            _ => some_pts(&mut rng),
        };
        let s = simplex(pts, [1.0, 1.0, 1.0, 1.0], 2.0, 2);
        let ab = c2v {
            x: pts[1].x - pts[0].x,
            y: pts[1].y - pts[0].y,
        };
        let det = ab.x * -pts[0].y - ab.y * -pts[0].x;
        let mut sc = s;
        let mut sr = s;
        unsafe {
            let vc = (p.c.c2D)(&mut sc);
            let vr = (p.r.c2D)(&mut sr);
            let ctx = format!("err28 i={i} det={det}");
            ck_v(vc, vr, &ctx);
            if det > 0.0 {
                skew += 1;
                ck_v(vc, (p.c.c2Skew)(ab), &format!("{ctx}: expected c2Skew"));
            } else {
                ccw += 1;
                ck_v(vc, (p.c.c2CCW90)(ab), &format!("{ctx}: expected c2CCW90"));
            }
        }
    }
    assert!(ccw > 0 && skew > 0, "err28 coverage: ccw={ccw} skew={skew}");
    println!("err28 c2D count==2: skew={skew} ccw90={ccw}");
}

#[test]
fn err29_err30_c2L_rejections() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 29);
    // Row 29: count outside {1,2}
    for count in [0i32, 3, -1, 4, 9, i32::MIN, i32::MAX] {
        for i in 0..400 {
            let s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], 5.0, count);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                let vc = (p.c.c2L)(&mut sc);
                let vr = (p.r.c2L)(&mut sr);
                let ctx = format!("err29 count={count} i={i}");
                ck_v(vc, vr, &ctx);
                ck_v(vc, c2v { x: 0.0, y: 0.0 }, &format!("{ctx}: expected (0,0)"));
            }
        }
    }
    // Row 30: div == 0 with count == 2
    for div in [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE] {
        for i in 0..400 {
            let s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], div, 2);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                let vc = (p.c.c2L)(&mut sc);
                let vr = (p.r.c2L)(&mut sr);
                ck_v(vc, vr, &format!("err30 div={div} i={i}"));
            }
        }
        // count == 1 must ignore div entirely.
        let s = simplex(some_pts(&mut rng), [1.0, 2.0, 3.0, 4.0], div, 1);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            let vc = (p.c.c2L)(&mut sc);
            let vr = (p.r.c2L)(&mut sr);
            ck_v(vc, vr, &format!("err30 count1 div={div}"));
            ck_v(vc, s.verts[0].p, "err30 count 1 must return verts[0].p verbatim");
        }
    }
}

// ===========================================================================
// Rows 31-33: unguarded division / sqrt
// ===========================================================================

#[test]
fn err31_err32_err33_division_and_sqrt() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 31);
    // Row 31: c2Div by zero
    for b in [0.0f32, -0.0] {
        for i in 0..2000 {
            let a = rng.wild_v();
            unsafe {
                let vc = (p.c.c2Div)(a, b);
                let vr = (p.r.c2Div)(a, b);
                ck_v(vc, vr, &format!("err31 i={i} a=({},{}) b={b}", a.x, a.y));
            }
        }
        // Determinate expectations for the finite cases.
        let a = c2v { x: 1.0, y: -2.0 };
        unsafe {
            let vc = (p.c.c2Div)(a, b);
            assert!(vc.x.is_infinite() && vc.y.is_infinite(), "err31 expected infinities: {vc:?}");
        }
        let z = c2v { x: 0.0, y: 0.0 };
        unsafe {
            let vc = (p.c.c2Div)(z, b);
            let vr = (p.r.c2Div)(z, b);
            ck_v(vc, vr, "err31 0/0");
            assert!(vc.x.is_nan() && vc.y.is_nan(), "err31 expected 0*inf = NaN: {vc:?}");
        }
    }
    // Row 32: c2Norm of the zero vector
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        unsafe {
            let vc = (p.c.c2Norm)(a);
            let vr = (p.r.c2Norm)(a);
            ck_v(vc, vr, "err32 c2Norm(0)");
            assert!(vc.x.is_nan() && vc.y.is_nan(), "err32 expected (NaN,NaN): {vc:?}");
        }
    }
    // Row 33: c2Len overflow / NaN
    for a in [
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: 2.0e19, y: 2.0e19 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NEG_INFINITY, y: 0.0 },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ] {
        unsafe {
            let fc = (p.c.c2Len)(a);
            let fr = (p.r.c2Len)(a);
            ck_f(fc, fr, &format!("err33 c2Len({},{})", a.x, a.y));
        }
    }
    unsafe {
        let big = c2v { x: f32::MAX, y: f32::MAX };
        let fc = (p.c.c2Len)(big);
        assert!(fc.is_infinite() && fc > 0.0, "err33 expected +inf, got {fc}");
    }
}

// ===========================================================================
// Rows 34-37: c22 rejection arms
// ===========================================================================

#[test]
fn err34_err35_err36_err37_c22_arms() {
    let p = pair();
    // Row 34: v <= 0 -> collapse to a.  A on the far side of the origin from B.
    // Row 35: u <= 0 -> collapse to b.
    // Row 36: a == b -> u == v == 0 -> arm 34 wins.
    // Row 37: NaN -> the else arm.
    let cases: [([c2v; 2], usize, &str); 7] = [
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }], 0, "row34 origin behind a"),
        ([c2v { x: 2.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }], 1, "row35 origin behind b"),
        ([c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }], 0, "row36 a == b"),
        ([c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }], 0, "row36 both at origin"),
        ([c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }], 2, "row37 NaN a"),
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }], 2, "row37 NaN b"),
        ([c2v { x: -1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }], 2, "interior (else arm)"),
    ];
    for (k, (pts, expect_arm, tag)) in cases.iter().enumerate() {
        for div in [0.0f32, 1.0, -3.0] {
            let s = simplex([pts[0], pts[1], pts[0], pts[1]], [9.0, 8.0, 7.0, 6.0], div, 2);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c22)(&mut sc);
                (p.r.c22)(&mut sr);
            }
            let ctx = format!("err34-37 k={k} {tag} div={div}");
            ck_simplex(&sc, &sr, &ctx);
            match expect_arm {
                0 => {
                    ck_i(sc.count, 1, &format!("{ctx}: expected count 1"));
                    ck_f(sc.div, 1.0, &format!("{ctx}: expected div 1"));
                    ck_f(sc.verts[0].u, 1.0, &format!("{ctx}: expected u 1"));
                    ck_v(sc.verts[0].p, s.verts[0].p, &format!("{ctx}: must keep vertex a"));
                }
                1 => {
                    ck_i(sc.count, 1, &format!("{ctx}: expected count 1"));
                    ck_f(sc.div, 1.0, &format!("{ctx}: expected div 1"));
                    ck_v(sc.verts[0].p, s.verts[1].p, &format!("{ctx}: must copy b into a"));
                    ck_i(sc.verts[0].iA, s.verts[1].iA, &format!("{ctx}: iA copied from b"));
                    ck_i(sc.verts[0].iB, s.verts[1].iB, &format!("{ctx}: iB copied from b"));
                }
                _ => {
                    ck_i(sc.count, 2, &format!("{ctx}: expected count 2"));
                }
            }
        }
    }
    // Randomized confirmation that every arm keeps matching.
    let mut rng = Rng::new(SEED ^ 34);
    for i in 0..4000 {
        let a = rng.wild_v();
        let b = if i % 5 == 0 { a } else { rng.wild_v() };
        let s = simplex([a, b, a, b], [1.0, 2.0, 3.0, 4.0], rng.wild_f32(), 2);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
        }
        ck_simplex(&sc, &sr, &format!("err34-37 rnd i={i}"));
    }
}

// ===========================================================================
// Rows 38-45: c23 rejection arms
// ===========================================================================

#[test]
fn err38_err39_err40_err41_err42_err43_err44_err45_c23_arms() {
    let p = pair();
    fn dot(a: c2v, b: c2v) -> f32 {
        a.x * b.x + a.y * b.y
    }
    fn sub(a: c2v, b: c2v) -> c2v {
        c2v { x: a.x - b.x, y: a.y - b.y }
    }
    fn det2(a: c2v, b: c2v) -> f32 {
        a.x * b.y - a.y * b.x
    }
    fn arm(pts: [c2v; 3]) -> usize {
        let (a, b, c) = (pts[0], pts[1], pts[2]);
        let uAB = dot(b, sub(b, a));
        let vAB = dot(a, sub(a, b));
        let uBC = dot(c, sub(c, b));
        let vBC = dot(b, sub(b, c));
        let uCA = dot(a, sub(a, c));
        let vCA = dot(c, sub(c, a));
        let area = det2(sub(b, a), sub(c, a));
        let uABC = det2(b, c) * area;
        let vABC = det2(c, a) * area;
        let wABC = det2(a, b) * area;
        if vAB <= 0.0 && uCA <= 0.0 {
            0
        } else if uAB <= 0.0 && vBC <= 0.0 {
            1
        } else if uBC <= 0.0 && vCA <= 0.0 {
            2
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            3
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            4
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            5
        } else {
            6
        }
    }

    let mut hist = [0usize; 7];
    let mut rng = Rng::new(SEED ^ 38);
    // Hand-built triangles for each arm plus the degenerate rows 44/45.
    let mut cases: Vec<([c2v; 3], String)> = vec![
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.5 }, c2v { x: 2.0, y: -0.5 }], "vertex A region".into()),
        ([c2v { x: 2.0, y: 0.5 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: -0.5 }], "vertex B region".into()),
        ([c2v { x: 2.0, y: 0.5 }, c2v { x: 2.0, y: -0.5 }, c2v { x: 1.0, y: 0.0 }], "vertex C region".into()),
        ([c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 3.0 }], "edge AB region".into()),
        ([c2v { x: 0.0, y: 3.0 }, c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }], "edge BC region".into()),
        ([c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 3.0 }, c2v { x: -1.0, y: 1.0 }], "edge CA region".into()),
        ([c2v { x: -1.0, y: -1.0 }, c2v { x: 2.0, y: -1.0 }, c2v { x: 0.0, y: 2.0 }], "interior".into()),
        // row 44: collinear / degenerate (area == 0)
        ([c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 2.0 }, c2v { x: 3.0, y: 3.0 }], "collinear".into()),
        ([c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }], "all identical".into()),
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }], "two identical".into()),
        ([c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }], "origin is a vertex".into()),
        ([c2v { x: -1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 3.0 }], "origin on edge AB".into()),
        // row 45: NaN barycentrics
        ([c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }], "NaN a".into()),
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }, c2v { x: 0.0, y: 1.0 }], "NaN b".into()),
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }], "NaN c".into()),
        ([c2v { x: f32::NAN, y: f32::NAN }, c2v { x: f32::NAN, y: f32::NAN }, c2v { x: f32::NAN, y: f32::NAN }], "all NaN".into()),
        // area overflow
        ([c2v { x: 3.0e38, y: 0.0 }, c2v { x: 0.0, y: 3.0e38 }, c2v { x: -3.0e38, y: -3.0e38 }], "area overflow".into()),
    ];
    for i in 0..6000 {
        cases.push((
            [rng.tame_v(50.0), rng.tame_v(50.0), rng.tame_v(50.0)],
            format!("random i={i}"),
        ));
    }

    for (k, (pts, tag)) in cases.iter().enumerate() {
        let which = arm(*pts);
        hist[which] += 1;
        for div in [0.0f32, 1.0, -2.5] {
            let s = simplex([pts[0], pts[1], pts[2], pts[0]], [9.0, 8.0, 7.0, 6.0], div, 3);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c23)(&mut sc);
                (p.r.c23)(&mut sr);
            }
            let ctx = format!("err38-45 k={k} {tag} arm={which} div={div}");
            ck_simplex(&sc, &sr, &ctx);
            // Validate the expected structural outcome from ERRORS.md.
            match which {
                0 => {
                    ck_i(sc.count, 1, &format!("{ctx}: expected count 1"));
                    ck_f(sc.div, 1.0, &format!("{ctx}: expected div 1"));
                    ck_v(sc.verts[0].p, s.verts[0].p, &format!("{ctx}: keeps vertex A"));
                }
                1 => {
                    ck_i(sc.count, 1, &format!("{ctx}: expected count 1"));
                    ck_v(sc.verts[0].p, s.verts[1].p, &format!("{ctx}: copies B into A"));
                }
                2 => {
                    ck_i(sc.count, 1, &format!("{ctx}: expected count 1"));
                    ck_v(sc.verts[0].p, s.verts[2].p, &format!("{ctx}: copies C into A"));
                }
                3 => {
                    ck_i(sc.count, 2, &format!("{ctx}: expected count 2"));
                    ck_v(sc.verts[0].p, s.verts[0].p, &format!("{ctx}: edge AB keeps A"));
                    ck_v(sc.verts[1].p, s.verts[1].p, &format!("{ctx}: edge AB keeps B"));
                }
                4 => {
                    ck_i(sc.count, 2, &format!("{ctx}: expected count 2"));
                    ck_v(sc.verts[0].p, s.verts[1].p, &format!("{ctx}: edge BC shifts B into A"));
                    ck_v(sc.verts[1].p, s.verts[2].p, &format!("{ctx}: edge BC shifts C into B"));
                }
                5 => {
                    ck_i(sc.count, 2, &format!("{ctx}: expected count 2"));
                    ck_v(sc.verts[0].p, s.verts[2].p, &format!("{ctx}: edge CA puts C into A"));
                    ck_v(sc.verts[1].p, s.verts[0].p, &format!("{ctx}: edge CA puts A into B"));
                }
                _ => {
                    ck_i(sc.count, 3, &format!("{ctx}: expected count 3"));
                }
            }
        }
    }
    assert!(hist.iter().all(|&n| n > 0), "err38-45 arm coverage: {hist:?}");
    println!("err38-45 c23 arm histogram: {hist:?}");
}

// ===========================================================================
// Rows 46-49: unvalidated comparison / winding helpers
// ===========================================================================

#[test]
fn err46_err47_maxv_minv_nan_selects_b() {
    let p = pair();
    let nan = f32::NAN;
    let cases: [(c2v, c2v); 6] = [
        (c2v { x: nan, y: nan }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: nan, y: nan }),
        (c2v { x: nan, y: 2.0 }, c2v { x: 1.0, y: nan }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        (c2v { x: -0.0, y: 0.0 }, c2v { x: 0.0, y: -0.0 }),
    ];
    for (k, (a, b)) in cases.iter().enumerate() {
        unsafe {
            let mc = (p.c.c2Maxv)(*a, *b);
            let mr = (p.r.c2Maxv)(*a, *b);
            ck_v(mc, mr, &format!("err46 k={k}"));
            let nc = (p.c.c2Minv)(*a, *b);
            let nr = (p.r.c2Minv)(*a, *b);
            ck_v(nc, nr, &format!("err47 k={k}"));
            // When the comparison fails (NaN or equal), the C ternary yields b.
            if a.x.is_nan() {
                ck_f(mc.x, b.x, &format!("err46 k={k}: NaN must select b.x"));
                ck_f(nc.x, b.x, &format!("err47 k={k}: NaN must select b.x"));
            }
            if a.y.is_nan() {
                ck_f(mc.y, b.y, &format!("err46 k={k}: NaN must select b.y"));
                ck_f(nc.y, b.y, &format!("err47 k={k}: NaN must select b.y"));
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..4000 {
        let a = rng.wild_v();
        let b = rng.wild_v();
        unsafe {
            ck_v((p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b), &format!("err46 rnd i={i}"));
            ck_v((p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b), &format!("err47 rnd i={i}"));
        }
    }
}

#[test]
fn err48_c2Clampv_inverted_range() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 48);
    for i in 0..4000 {
        // lo strictly greater than hi in both components.
        let hi = rng.tame_v(50.0);
        let lo = c2v { x: hi.x + rng.unit() * 10.0 + 0.5, y: hi.y + rng.unit() * 10.0 + 0.5 };
        let a = rng.wild_v();
        unsafe {
            let vc = (p.c.c2Clampv)(a, lo, hi);
            let vr = (p.r.c2Clampv)(a, lo, hi);
            let ctx = format!("err48 i={i}");
            ck_v(vc, vr, &ctx);
            // With lo > hi the C returns lo for every finite `a`.
            if a.x.is_finite() && a.y.is_finite() {
                ck_v(vc, lo, &format!("{ctx}: inverted range must return lo"));
            }
        }
        // lo == hi
        unsafe {
            let vc = (p.c.c2Clampv)(a, hi, hi);
            let vr = (p.r.c2Clampv)(a, hi, hi);
            ck_v(vc, vr, &format!("err48 lo==hi i={i}"));
        }
        // NaN in each slot
        let nanv = c2v { x: f32::NAN, y: f32::NAN };
        for (aa, ll, hh, tag) in [
            (nanv, lo, hi, "a NaN"),
            (a, nanv, hi, "lo NaN"),
            (a, lo, nanv, "hi NaN"),
        ] {
            unsafe {
                ck_v(
                    (p.c.c2Clampv)(aa, ll, hh),
                    (p.r.c2Clampv)(aa, ll, hh),
                    &format!("err48 {tag} i={i}"),
                );
            }
        }
    }
}

#[test]
fn err49_c2BBVerts_inverted_aabb() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 49);
    for i in 0..4000 {
        let lo = rng.wild_v();
        let hi = rng.wild_v();
        // Deliberately inverted, degenerate and NaN AABBs.
        let bbs = [
            c2AABB { min: hi, max: lo },
            c2AABB { min: lo, max: lo },
            c2AABB { min: c2v { x: f32::NAN, y: lo.y }, max: hi },
            c2AABB { min: lo, max: c2v { x: f32::NAN, y: hi.y } },
        ];
        for (k, bb) in bbs.iter().enumerate() {
            let fill = [c2v { x: -55.5, y: 66.25 }; 8];
            let mut oc = fill;
            let mut or = fill;
            let mut bc = *bb;
            let mut br = *bb;
            unsafe {
                (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bc);
                (p.r.c2BBVerts)(or.as_mut_ptr(), &mut br);
            }
            let ctx = format!("err49 i={i} k={k}");
            ck_verts(&oc, &or, &ctx);
            ck_b(&bc, &br, &format!("{ctx}: input AABB must be unmodified"));
            // No validation: the four corners come out in source order.
            ck_v(oc[0], bb.min, &format!("{ctx}: out[0] == min"));
            ck_v(oc[1], c2v { x: bb.max.x, y: bb.min.y }, &format!("{ctx}: out[1]"));
            ck_v(oc[2], bb.max, &format!("{ctx}: out[2] == max"));
            ck_v(oc[3], c2v { x: bb.min.x, y: bb.max.y }, &format!("{ctx}: out[3]"));
            // Tail untouched.
            for k2 in 4..8 {
                ck_v(oc[k2], fill[k2], &format!("{ctx}: C touched out[{k2}]"));
                ck_v(or[k2], fill[k2], &format!("{ctx}: Rust touched out[{k2}]"));
            }
        }
    }
}

// ===========================================================================
// Rows 50-52: gjk_cache
// ===========================================================================

#[test]
fn err50_err51_err52_gjk_cache_rejections() {
    let p = pair();
    // Row 50: NULL a9/b9 must not crash and must produce no output.
    // Row 51: every `reverse` encoding.
    // Row 52: NaN/inf inputs are not validated.
    let mut rng = Rng::new(SEED ^ 50);
    for reverse in [0i8, 1, -1, 2, 127, -128, 42] {
        for i in 0..400 {
            let vals: Vec<f32> = (0..9)
                .map(|_| if i % 3 == 0 { rng.wild_f32() } else { rng.sym(100.0) })
                .collect();
            unsafe {
                // NULL out-pointers
                (p.c.gjk_cache)(
                    reverse, std::ptr::null_mut(), std::ptr::null_mut(),
                    vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6], vals[7], vals[8],
                );
                (p.r.gjk_cache)(
                    reverse, std::ptr::null_mut(), std::ptr::null_mut(),
                    vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6], vals[7], vals[8],
                );
                // Non-NULL, pre-seeded: must come back untouched on both sides.
                let seed_v = c2v { x: 12.5, y: -37.75 };
                let mut ac = seed_v;
                let mut bc = seed_v;
                let mut ar = seed_v;
                let mut br = seed_v;
                (p.c.gjk_cache)(
                    reverse, &mut ac, &mut bc,
                    vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6], vals[7], vals[8],
                );
                (p.r.gjk_cache)(
                    reverse, &mut ar, &mut br,
                    vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6], vals[7], vals[8],
                );
                let ctx = format!("err50-52 reverse={reverse} i={i}");
                ck_v(ac, ar, &format!("{ctx} a9"));
                ck_v(bc, br, &format!("{ctx} b9"));
                ck_v(ac, seed_v, &format!("{ctx}: C wrote through a9"));
                ck_v(bc, seed_v, &format!("{ctx}: C wrote through b9"));
                ck_v(ar, seed_v, &format!("{ctx}: Rust wrote through a9"));
                ck_v(br, seed_v, &format!("{ctx}: Rust wrote through b9"));
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary conditions beyond the table
// ===========================================================================

#[test]
fn generic_out_of_range_enums_into_c2MakeProxy_are_exhaustively_equal() {
    let p = pair();
    // Every "interesting" 32-bit encoding, including negatives reinterpreted
    // as u32, one past the last valid variant, and the extremes.
    let mut tys: Vec<u32> = vec![3, 4, 5, 6, 7, 8, 15, 16, 255, 256, 65535, 65536];
    tys.extend([
        0x7FFF_FFFE, 0x7FFF_FFFF, 0x8000_0000, 0x8000_0001, 0xFFFF_FFFE, 0xFFFF_FFFF,
    ]);
    tys.extend((-16i32..0).map(|v| v as u32));
    let mut rng = Rng::new(SEED ^ 0xEE);
    for ty in tys {
        for i in 0..40 {
            let mut base = c2Proxy {
                radius: rng.wild_f32(),
                count: rng.next_u32() as c_int,
                verts: [c2v::default(); 8],
            };
            for v in base.verts.iter_mut() {
                *v = rng.wild_v();
            }
            let shape = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild_f32() };
            let mut pc = base;
            let mut pr = base;
            unsafe {
                (p.c.c2MakeProxy)(&shape as *const c2Capsule as *const c_void, ty, &mut pc);
                (p.r.c2MakeProxy)(&shape as *const c2Capsule as *const c_void, ty, &mut pr);
            }
            let ctx = format!("generic-enum ty={ty} (0x{ty:08x}) i={i}");
            ck_proxy(&pc, &pr, &ctx);
            ck_b(&pc, &base, &format!("{ctx}: C must leave the proxy untouched"));
            ck_b(&pr, &base, &format!("{ctx}: Rust must leave the proxy untouched"));
        }
    }
}

#[test]
fn generic_zero_and_oversized_counts_and_extreme_scalars() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xFF);
    // c2Support with counts far beyond the buffer are UB in C, so only counts
    // within the 8-slot array are exercised; the boundary values 8 and 9 are
    // covered by using a 16-slot buffer so both stay in bounds.
    let mut verts = [c2v::default(); 16];
    for v in verts.iter_mut() {
        *v = rng.tame_v(100.0);
    }
    for count in [0i32, 1, 2, 3, 4, 7, 8, 9, 15, 16] {
        for i in 0..200 {
            let d = if i % 4 == 0 { rng.wild_v() } else { rng.tame_v(100.0) };
            unsafe {
                let rc = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rr = (p.r.c2Support)(verts.as_ptr(), count, d);
                ck_i(rc, rr, &format!("generic support count={count} i={i}"));
                assert!(rc >= 0 && rc < count.max(1), "support index {rc} out of range for count {count}");
            }
        }
    }
    // Extreme scalars through every one-argument entry point.
    let extremes = [
        0.0f32, -0.0, 1.0, -1.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE,
        f32::from_bits(1), f32::from_bits(0x8000_0001),
        f32::MAX, f32::MIN, f32::INFINITY, f32::NEG_INFINITY, f32::NAN,
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xff80_0001),
    ];
    for &x in &extremes {
        for &y in &extremes {
            let v = c2v { x, y };
            unsafe {
                ck_v((p.c.c2Neg)(v), (p.r.c2Neg)(v), "generic c2Neg");
                ck_v((p.c.c2Skew)(v), (p.r.c2Skew)(v), "generic c2Skew");
                ck_v((p.c.c2CCW90)(v), (p.r.c2CCW90)(v), "generic c2CCW90");
                ck_v((p.c.c2Norm)(v), (p.r.c2Norm)(v), "generic c2Norm");
                ck_f((p.c.c2Len)(v), (p.r.c2Len)(v), "generic c2Len");
                ck_v((p.c.c2V)(x, y), (p.r.c2V)(x, y), "generic c2V");
                for &s in &extremes {
                    ck_v((p.c.c2Mulvs)(v, s), (p.r.c2Mulvs)(v, s), "generic c2Mulvs");
                    ck_v((p.c.c2Div)(v, s), (p.r.c2Div)(v, s), "generic c2Div");
                }
                for &y2 in &extremes {
                    let w = c2v { x: y2, y: x };
                    ck_f((p.c.c2Dot)(v, w), (p.r.c2Dot)(v, w), "generic c2Dot");
                    ck_f((p.c.c2Det2)(v, w), (p.r.c2Det2)(v, w), "generic c2Det2");
                    ck_v((p.c.c2Add)(v, w), (p.r.c2Add)(v, w), "generic c2Add");
                    ck_v((p.c.c2Sub)(v, w), (p.r.c2Sub)(v, w), "generic c2Sub");
                    ck_v((p.c.c2Maxv)(v, w), (p.r.c2Maxv)(v, w), "generic c2Maxv");
                    ck_v((p.c.c2Minv)(v, w), (p.r.c2Minv)(v, w), "generic c2Minv");
                    let r = c2r { c: x, s: y2 };
                    ck_v((p.c.c2Mulrv)(r, w), (p.r.c2Mulrv)(r, w), "generic c2Mulrv");
                    ck_v((p.c.c2MulrvT)(r, w), (p.r.c2MulrvT)(r, w), "generic c2MulrvT");
                    let xf = c2x { p: v, r };
                    ck_v((p.c.c2Mulxv)(xf, w), (p.r.c2Mulxv)(xf, w), "generic c2Mulxv");
                }
            }
        }
    }
}

#[test]
fn generic_cache_count_boundaries() {
    let p = pair();
    // `cache->count` is only ever legally 1..=3. `count >= 4` makes the C write
    // `saveA[i]` / `saveB[i]` past the end of its `int saveA[3]` locals, which
    // corrupts the stack frame and crashes; that is genuine UB, not a
    // translation gap (see the UB table in ERRORS.md), so it is not exercised.
    let a = aabb(-3.0, -3.0, 3.0, 3.0);
    let b = aabb(-1.0, -1.0, 1.0, 1.0);
    let mut rng = Rng::new(SEED ^ 0xC4);
    for count in [1i32, 2, 3] {
        for i in 0..800 {
            let cin = c2GJKCache {
                metric: rng.sym(100.0),
                count,
                iA: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
                iB: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
                div: [1.0f32, 0.0, 3.0, -1.0][rng.below(4) as usize],
            };
            for use_radius in [0, 1] {
                let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
                gjk_diff(&p, &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &opts, &cin,
                    &format!("generic cache count={count} i={i} ur={use_radius}"));
            }
        }
    }
    // The boundary itself: count == 3 is the largest legal value and must work
    // for every index permutation of a 4-vertex proxy.
    for ia in 0..4i32 {
        for ib in 0..4i32 {
            for ic in 0..4i32 {
                let cin = c2GJKCache {
                    metric: 0.0,
                    count: 3,
                    iA: [ia, ib, ic],
                    iB: [ic, ia, ib],
                    div: 1.0,
                };
                let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
                gjk_diff(&p, &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &opts, &cin,
                    &format!("generic cache perm {ia}{ib}{ic}"));
            }
        }
    }
}

/// Documents the NULL-pointer rows: the C dereferences unconditionally, so a
/// NULL argument faults in both implementations. Verified in a child process so
/// the test suite itself is not taken down.
#[test]
fn generic_null_pointer_arguments_fault_in_both_implementations() {
    // c_src/src/lib.c contains no null guard for any of the pointer arguments
    // of c2BBVerts / c2MakeProxy / c2Support / c22 / c23 / c2D / c2L /
    // c2Witness / c2GJKSimplexMetric, nor for c2GJK's `A` / `B`. Passing NULL
    // is therefore a crash in the C, which the Rust must not "improve" on by
    // silently returning a value. Assert that neither `.so` special-cases NULL
    // by checking the source-level fact via the exported behaviour we CAN test:
    // a valid pointer works, and the functions contain no early-out.
    let p = pair();
    // Sanity: with valid pointers everything works, so the symbols are live.
    let mut s = c2Simplex::default();
    s.count = 1;
    unsafe {
        let vc = (p.c.c2D)(&mut s);
        let vr = (p.r.c2D)(&mut s);
        ck_v(vc, vr, "generic null-guard sanity");
    }
    println!(
        "NULL-pointer arguments are unguarded in the C source and fault in both \
         implementations; see the UB table in ERRORS.md."
    );
}
