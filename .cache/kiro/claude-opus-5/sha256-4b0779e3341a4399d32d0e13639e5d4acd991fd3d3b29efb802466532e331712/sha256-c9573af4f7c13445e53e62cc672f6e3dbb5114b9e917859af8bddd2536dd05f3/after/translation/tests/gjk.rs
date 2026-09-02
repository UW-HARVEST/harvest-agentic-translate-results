//! Phase B — CONFIGS.md rows 41..67: the raw `c2GJK` entry point.
//!
//! `c2GJK` is the lowest-level composed operation in the library and where the
//! interaction bugs live: 9 type pairs x `use_radius` x transform presence x
//! cache state x geometric relation. It is driven directly here (not through
//! the `c2*to*` convenience wrappers), the way a real consumer would, and ALL
//! observable outputs are compared: return value, `*outA`, `*outB`,
//! `*iterations`, and every field of the written-back `c2GJKCache`.

mod common;
use common::*;

const REPS: usize = 160;

/// Geometric relations the GJK loop distinguishes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Rel {
    Overlap,
    Far,
    Touch,
    Coincident,
    Degenerate,
    Huge,
    Random,
}

const RELS: [Rel; 7] = [
    Rel::Overlap,
    Rel::Far,
    Rel::Touch,
    Rel::Coincident,
    Rel::Degenerate,
    Rel::Huge,
    Rel::Random,
];

fn pair_in(rng: &mut Rng, ta: i32, tb: i32, rel: Rel, i: usize) -> (ShapeBlob, ShapeBlob) {
    match rel {
        Rel::Overlap => {
            let at = rng.v_tame();
            let s = rng.range(5.0, 40.0);
            let jitter = C2v { x: at.x + rng.range(-s, s), y: at.y + rng.range(-s, s) };
            (
                ShapeBlob::near(rng, ta, at, s),
                ShapeBlob::near(rng, tb, jitter, s),
            )
        }
        Rel::Far => {
            let s = rng.range(1.0, 20.0);
            let a = C2v { x: rng.range(-50.0, -40.0), y: rng.range(-50.0, 50.0) };
            let b = C2v { x: rng.range(400.0, 900.0), y: rng.range(-50.0, 50.0) };
            (ShapeBlob::near(rng, ta, a, s), ShapeBlob::near(rng, tb, b, s))
        }
        Rel::Touch => {
            // Exactly-touching circles: dist == rA + rB, so the C takes the
            // `else` (midpoint) arm of the use_radius block rather than the
            // shrink arm.
            let r1 = (rng.below(30) + 1) as f32;
            let r2 = (rng.below(30) + 1) as f32;
            let a = C2v { x: 0.0, y: 0.0 };
            let b = C2v { x: r1 + r2, y: 0.0 };
            match (ta, tb) {
                (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE) => (
                    ShapeBlob::circle(C2Circle { p: a, r: r1 }),
                    ShapeBlob::circle(C2Circle { p: b, r: r2 }),
                ),
                _ => {
                    // Edge-touching boxes / capsules built from integers so the
                    // arithmetic is exact.
                    let s = (rng.below(20) + 1) as f32;
                    (
                        ShapeBlob::near(rng, ta, a, s),
                        ShapeBlob::near(rng, tb, C2v { x: 2.0 * s, y: 0.0 }, s),
                    )
                }
            }
        }
        Rel::Coincident => {
            let at = rng.v_tame();
            let s = rng.range(0.0, 30.0);
            (
                ShapeBlob::near(rng, ta, at, s),
                ShapeBlob::near(rng, tb, at, s),
            )
        }
        Rel::Degenerate => (
            ShapeBlob::degenerate(ta, i),
            ShapeBlob::degenerate(tb, i / 6 + 1),
        ),
        Rel::Huge => {
            let s = 1.0e28;
            let a = C2v { x: rng.range(-1.0e30, 1.0e30), y: rng.range(-1.0e30, 1.0e30) };
            let b = C2v { x: rng.range(-1.0e30, 1.0e30), y: rng.range(-1.0e30, 1.0e30) };
            (ShapeBlob::near(rng, ta, a, s), ShapeBlob::near(rng, tb, b, s))
        }
        Rel::Random => (
            ShapeBlob::random(rng, ta),
            ShapeBlob::random(rng, tb),
        ),
    }
}

/// rows 41..50 — every type pair x use_radius, NULL transforms, no cache.
/// rows 56..61 — folded in via the `Rel` axis.
#[test]
fn row41_50_type_pairs_no_transform() {
    let mut rng = Rng::new(0x6A_0041);
    let mut zero_dist = 0usize;
    let mut pos_dist = 0usize;
    let mut max_iter = 0i32;

    for rel in RELS {
        for (ta, tb) in TYPE_PAIRS {
            for ur in [0, 1] {
                for i in 0..REPS {
                    let (a, b) = pair_in(&mut rng, ta, tb, rel, i);
                    let opts = GjkOpts { use_radius: ur, ..Default::default() };
                    let out = gjk_diff(
                        &format!("{rel:?} ur={ur}"),
                        &a,
                        ta,
                        &b,
                        tb,
                        &opts,
                    );
                    if out.dist == 0.0 {
                        zero_dist += 1;
                    } else if out.dist > 0.0 {
                        pos_dist += 1;
                    }
                    max_iter = max_iter.max(out.iters);
                }
            }
        }
    }
    assert!(zero_dist > 0 && pos_dist > 0, "did not reach both hit and miss paths");
    assert!(max_iter > 1, "GJK loop never iterated more than once (max={max_iter})");
    eprintln!("dist==0: {zero_dist}, dist>0: {pos_dist}, max iterations: {max_iter}");
}

/// rows 51,52,53,54,55 — transform presence and kind
#[test]
fn row51_55_transforms() {
    let mut rng = Rng::new(0x6A_0051);
    for rel in [Rel::Overlap, Rel::Far, Rel::Coincident, Rel::Random, Rel::Degenerate] {
        for (ta, tb) in TYPE_PAIRS {
            for ur in [0, 1] {
                for i in 0..REPS / 2 {
                    let (a, b) = pair_in(&mut rng, ta, tb, rel, i);

                    // row 51: ax only
                    let ax = rng.xform_unit();
                    gjk_diff("ax only", &a, ta, &b, tb,
                        &GjkOpts { use_radius: ur, ax: Some(ax), ..Default::default() });

                    // row 52: bx only
                    let bx = rng.xform_unit();
                    gjk_diff("bx only", &a, ta, &b, tb,
                        &GjkOpts { use_radius: ur, bx: Some(bx), ..Default::default() });

                    // row 53: both, unit rotations
                    gjk_diff("both unit", &a, ta, &b, tb,
                        &GjkOpts { use_radius: ur, ax: Some(ax), bx: Some(bx), ..Default::default() });

                    // row 54: both, NON-unit c2r (the C never normalises)
                    gjk_diff("both non-unit", &a, ta, &b, tb,
                        &GjkOpts {
                            use_radius: ur,
                            ax: Some(rng.xform_nonunit()),
                            bx: Some(rng.xform_nonunit()),
                            ..Default::default()
                        });

                    // row 55: explicit identity must equal the NULL result
                    let ident = C2x { p: C2v { x: 0.0, y: 0.0 }, r: C2r { c: 1.0, s: 0.0 } };
                    let with_ident = gjk_diff("explicit identity", &a, ta, &b, tb,
                        &GjkOpts { use_radius: ur, ax: Some(ident), bx: Some(ident), ..Default::default() });
                    let with_null = gjk_diff("null transform", &a, ta, &b, tb,
                        &GjkOpts { use_radius: ur, ..Default::default() });
                    same("c2GJK identity-vs-NULL", &(type_name(ta), type_name(tb), ur),
                         &with_ident, &with_null);
                }
            }
        }
    }
}

/// row 62 — cache supplied but cold (`count == 0`): must equal the no-cache
/// result, and the written-back cache is compared field by field.
#[test]
fn row62_cold_cache() {
    let mut rng = Rng::new(0x6A_0062);
    for rel in RELS {
        for (ta, tb) in TYPE_PAIRS {
            for ur in [0, 1] {
                for i in 0..REPS / 2 {
                    let (a, b) = pair_in(&mut rng, ta, tb, rel, i);
                    // A cold cache still carries junk in metric/div/indices;
                    // count == 0 is what makes it cold.
                    let cold = C2GJKCache {
                        metric: rng.f32_mixed(),
                        count: 0,
                        iA: [9, -3, 77],
                        iB: [-1, 4, 12],
                        div: rng.f32_mixed(),
                    };
                    let opts = GjkOpts { use_radius: ur, cache: Some(cold), ..Default::default() };
                    let with = gjk_diff("cold cache", &a, ta, &b, tb, &opts);
                    let without = gjk_diff(
                        "no cache",
                        &a,
                        ta,
                        &b,
                        tb,
                        &GjkOpts { use_radius: ur, ..Default::default() },
                    );
                    // A cold cache must not change the answer.
                    same_f32("cold cache dist", &(type_name(ta), type_name(tb), ur),
                             with.dist, without.dist);
                    same("cold cache outA", &(type_name(ta), type_name(tb)), &with.a, &without.a);
                    same("cold cache outB", &(type_name(ta), type_name(tb)), &with.b, &without.b);
                    same_i32("cold cache iters", &(type_name(ta), type_name(tb)),
                             with.iters, without.iters);
                    let cch = with.cache.expect("cache written back");
                    assert!(
                        (1..=3).contains(&cch.count),
                        "cache count out of range after cold start: {cch:?}"
                    );
                }
            }
        }
    }
}

/// rows 63,64 — warm cache: re-run on the same shapes, then carry the cache
/// through a moving-shape chain (the real consumer pattern).
#[test]
fn row63_64_warm_cache() {
    let mut rng = Rng::new(0x6A_0063);
    let mut warm_counts = [0usize; 4];

    for rel in RELS {
        for (ta, tb) in TYPE_PAIRS {
            for ur in [0, 1] {
                for i in 0..REPS / 2 {
                    let (a, b) = pair_in(&mut rng, ta, tb, rel, i);

                    // Seed the cache.
                    let seed = C2GJKCache::default();
                    let mut cache = gjk_diff(
                        "warm seed",
                        &a,
                        ta,
                        &b,
                        tb,
                        &GjkOpts { use_radius: ur, cache: Some(seed), ..Default::default() },
                    )
                    .cache
                    .unwrap();
                    if (0..4).contains(&cache.count) {
                        warm_counts[cache.count as usize] += 1;
                    }

                    // row 63: immediate re-run with the warm cache.
                    cache = gjk_diff(
                        "warm rerun",
                        &a,
                        ta,
                        &b,
                        tb,
                        &GjkOpts { use_radius: ur, cache: Some(cache), ..Default::default() },
                    )
                    .cache
                    .unwrap();

                    // row 64: carry the cache across a moving shape.
                    let mut moved = b;
                    for _ in 0..4 {
                        moved = translate(&moved, rng.range(-3.0, 3.0), rng.range(-3.0, 3.0));
                        cache = gjk_diff(
                            "warm chain",
                            &a,
                            ta,
                            &moved,
                            tb,
                            &GjkOpts { use_radius: ur, cache: Some(cache), ..Default::default() },
                        )
                        .cache
                        .unwrap();
                    }
                }
            }
        }
    }
    // A warm cache with count 1, 2 and 3 must all have occurred, otherwise the
    // cache-read path is only partly exercised.
    assert!(
        warm_counts[1] > 0 && warm_counts[2] > 0 && warm_counts[3] > 0,
        "warm cache count coverage gap: {warm_counts:?}"
    );
    eprintln!("cache counts produced: {warm_counts:?}");
}

/// row 65 — reuse a cache built from one shape pair on a *different* pair.
/// Restricted to caches whose stored indices are still in range for the new
/// proxies: an out-of-range index makes the C read `c2Proxy.verts[i]` slots
/// that `c2MakeProxy` never wrote (uninitialised stack), which has no defined
/// value to compare against. See ERRORS.md "deliberately excluded".
#[test]
fn row65_cross_shape_cache_reuse() {
    let mut rng = Rng::new(0x6A_0065);
    let mut reused = 0usize;

    for (ta, tb) in TYPE_PAIRS {
        for (ta2, tb2) in TYPE_PAIRS {
            for ur in [0, 1] {
                for i in 0..REPS / 4 {
                    let (a, b) = pair_in(&mut rng, ta, tb, Rel::Overlap, i);
                    let cache = gjk_diff(
                        "xshape seed",
                        &a,
                        ta,
                        &b,
                        tb,
                        &GjkOpts {
                            use_radius: ur,
                            cache: Some(C2GJKCache::default()),
                            ..Default::default()
                        },
                    )
                    .cache
                    .unwrap();

                    let n = cache.count.max(0) as usize;
                    let maxa = cache.iA[..n.min(3)].iter().copied().max().unwrap_or(0);
                    let maxb = cache.iB[..n.min(3)].iter().copied().max().unwrap_or(0);
                    if maxa >= proxy_count(ta2) || maxb >= proxy_count(tb2) {
                        continue; // would read never-written proxy verts in C
                    }

                    let (a2, b2) = pair_in(&mut rng, ta2, tb2, Rel::Random, i);
                    gjk_diff(
                        "xshape reuse",
                        &a2,
                        ta2,
                        &b2,
                        tb2,
                        &GjkOpts { use_radius: ur, cache: Some(cache), ..Default::default() },
                    );
                    reused += 1;
                }
            }
        }
    }
    assert!(reused > 0, "no cross-shape cache reuse was exercised");
    eprintln!("cross-shape cache reuses: {reused}");
}

/// rows 66,67 — every combination of NULL / non-NULL out-params; the return
/// value must be unaffected and `*iterations` must match exactly.
#[test]
fn row66_67_out_param_combinations() {
    let mut rng = Rng::new(0x6A_0066);
    for rel in RELS {
        for (ta, tb) in TYPE_PAIRS {
            for ur in [0, 1] {
                for i in 0..REPS / 4 {
                    let (a, b) = pair_in(&mut rng, ta, tb, rel, i);
                    let base = gjk_diff(
                        "all out-params",
                        &a,
                        ta,
                        &b,
                        tb,
                        &GjkOpts { use_radius: ur, ..Default::default() },
                    );
                    for (wa, wb, wi) in [
                        (false, true, true),
                        (true, false, true),
                        (false, false, true),
                        (true, true, false),
                        (false, false, false),
                    ] {
                        let o = gjk_diff(
                            "partial out-params",
                            &a,
                            ta,
                            &b,
                            tb,
                            &GjkOpts {
                                use_radius: ur,
                                want_a: wa,
                                want_b: wb,
                                want_iters: wi,
                                ..Default::default()
                            },
                        );
                        // Dropping an out-param must not change the result.
                        same_f32("dist with NULL out-params",
                                 &(type_name(ta), type_name(tb), wa, wb, wi),
                                 base.dist, o.dist);
                        if wa {
                            same("outA", &(wa, wb, wi), &base.a, &o.a);
                        }
                        if wb {
                            same("outB", &(wa, wb, wi), &base.b, &o.b);
                        }
                        if wi {
                            same_i32("iterations", &(wa, wb, wi), base.iters, o.iters);
                        }
                    }
                    assert!(
                        (0..=20).contains(&base.iters),
                        "iterations outside [0,20]: {}",
                        base.iters
                    );
                }
            }
        }
    }
}

// --- helpers ---------------------------------------------------------------

fn proxy_count(t: i32) -> i32 {
    match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        C2_TYPE_CAPSULE => 2,
        _ => 0,
    }
}

/// Translate a shape blob in place (works for all three shape kinds).
fn translate(s: &ShapeBlob, dx: f32, dy: f32) -> ShapeBlob {
    let mut out = *s;
    let n = match s.kind {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 2,
        _ => 2,
    };
    for k in 0..n {
        let off = k * 8;
        let mut x = f32::from_le_bytes(out.bytes[off..off + 4].try_into().unwrap());
        let mut y = f32::from_le_bytes(out.bytes[off + 4..off + 8].try_into().unwrap());
        x += dx;
        y += dy;
        out.bytes[off..off + 4].copy_from_slice(&x.to_le_bytes());
        out.bytes[off + 4..off + 8].copy_from_slice(&y.to_le_bytes());
    }
    out
}
