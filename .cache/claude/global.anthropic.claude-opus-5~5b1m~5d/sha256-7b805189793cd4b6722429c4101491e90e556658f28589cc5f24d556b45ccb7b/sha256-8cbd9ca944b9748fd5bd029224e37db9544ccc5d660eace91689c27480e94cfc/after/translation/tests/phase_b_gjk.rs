//! Phase B, Group 4 — `CONFIGS.md` rows 45-70.
//!
//! The full `c2GJK` pipeline driven as a real consumer would: shapes are set
//! up, options applied, and every observable (return value, `*outA`, `*outB`,
//! `*iterations`, and the whole written-back `c2GJKCache`) is compared
//! bit-for-bit between the two `.so`s. Each row runs over all 9 ordered
//! `(typeA, typeB)` pairs with many randomized instances at a fixed seed.

mod common;
use common::*;

/// Instances per (row, shape-pair). 9 pairs × this = samples per row.
const M: usize = 250;

/// Random pair of shapes at the given separation regime.
/// `mode`: 0 = far apart, 1 = overlapping, 2 = just touching, 3 = coincident.
fn pair_at(rng: &mut Rng, ka: i32, kb: i32, mode: u8) -> (Shape, Shape) {
    let ca = c2v::new(rng.uniform(-6.0, 6.0), rng.uniform(-6.0, 6.0));
    let sa = rng.uniform(0.3, 2.0);
    let a = rand_shape(rng, ka, ca, sa);
    let dir = {
        let t = rng.uniform(-3.15, 3.15);
        c2v::new(t.cos(), t.sin())
    };
    let gap = match mode {
        0 => rng.uniform(4.0, 25.0),
        1 => rng.uniform(-1.2, 0.05),
        2 => rng.uniform(-0.2, 0.4),
        _ => 0.0,
    };
    let cb = c2v::new(ca.x + dir.x * gap, ca.y + dir.y * gap);
    let sb = rng.uniform(0.3, 2.0);
    let b = rand_shape(rng, kb, cb, sb);
    (a, b)
}

fn for_each_pair<F: FnMut(&mut Rng, i32, i32)>(seed: u64, mut f: F) {
    for (ka, kb) in shape_pairs() {
        let mut rng = Rng::new(seed ^ ((ka as u64) << 8) ^ (kb as u64));
        f(&mut rng, ka, kb);
    }
}

// ---------------------------------------------------------------------------
// Rows 45-49 — the separation regimes × use_radius
// ---------------------------------------------------------------------------

#[test]
fn row45_separated_no_radius() {
    for_each_pair(0x45, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, 0);
            let o = GjkOpts {
                use_radius: 0,
                ..Default::default()
            };
            diff_gjk(
                &format!("r45 {}/{} i={i}", kind_name(ka), kind_name(kb)),
                &a,
                &b,
                &o,
            );
        }
    });
}

#[test]
fn row46_separated_with_radius() {
    for_each_pair(0x46, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, 0);
            diff_gjk(
                &format!("r46 {}/{} i={i}", kind_name(ka), kind_name(kb)),
                &a,
                &b,
                &GjkOpts::default(),
            );
        }
    });
}

#[test]
fn row47_overlapping_hit_path() {
    for_each_pair(0x47, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, 1);
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r47 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

#[test]
fn row48_touching_midpoint_path() {
    for_each_pair(0x48, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, 2);
            diff_gjk(
                &format!("r48 {}/{} i={i}", kind_name(ka), kind_name(kb)),
                &a,
                &b,
                &GjkOpts::default(),
            );
        }
    });
}

/// Row 49: separation in `(0, FLT_EPSILON]`, so `dist > rA+rB` can hold while
/// `dist > FLT_EPSILON` fails ⇒ the midpoint branch despite separation.
#[test]
fn row49_epsilon_separation() {
    for_each_pair(0x49, |rng, ka, kb| {
        for i in 0..M {
            // Zero-radius shapes separated by an epsilon-scale gap.
            let ca = c2v::new(rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0));
            let mk = |k: i32, at: c2v| -> Shape {
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: at, r: 0.0 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: at, max: at }),
                    _ => Shape::Capsule(c2Capsule {
                        a: at,
                        b: at,
                        r: 0.0,
                    }),
                }
            };
            let eps_mult = [0.0f32, 0.25, 0.5, 1.0, 1.0000001, 2.0, 10.0][i % 7];
            let d = FLT_EPS * eps_mult;
            let cb = c2v::new(ca.x + d, ca.y);
            diff_gjk(
                &format!("r49 {}/{} i={i} m={eps_mult}", kind_name(ka), kind_name(kb)),
                &mk(ka, ca),
                &mk(kb, cb),
                &GjkOpts::default(),
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Rows 50-53 — transforms
// ---------------------------------------------------------------------------

#[test]
fn row50_ax_only() {
    for_each_pair(0x50, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 3) as u8);
            let ax = rng.xform();
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r50 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        ax: Some(ax),
                        bx: None,
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

#[test]
fn row51_bx_only() {
    for_each_pair(0x51, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 3) as u8);
            let bx = rng.xform();
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r51 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        ax: None,
                        bx: Some(bx),
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

#[test]
fn row52_both_transforms() {
    for_each_pair(0x52, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            let ax = rng.xform();
            let bx = rng.xform();
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r52 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        ax: Some(ax),
                        bx: Some(bx),
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

/// Row 53: unnormalised / degenerate `c2r` — perfectly legal input, the C never
/// checks `c² + s² == 1`.
#[test]
fn row53_unnormalised_transforms() {
    for_each_pair(0x53, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 3) as u8);
            let ax = rng.xform_wild();
            let bx = rng.xform_wild();
            diff_gjk(
                &format!("r53 {}/{} i={i}", kind_name(ka), kind_name(kb)),
                &a,
                &b,
                &GjkOpts {
                    ax: Some(ax),
                    bx: Some(bx),
                    ..Default::default()
                },
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Rows 54-57 — optional out-params
// ---------------------------------------------------------------------------

#[test]
fn row54_57_optional_outparams() {
    for_each_pair(0x54, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            // Row 54: iterations NULL vs non-NULL.
            // Rows 55-57: outA / outB / both NULL.
            for (wa, wb, wi) in [
                (true, true, true),
                (true, true, false),
                (false, true, true),
                (true, false, true),
                (false, false, true),
                (false, false, false),
            ] {
                diff_gjk(
                    &format!(
                        "r54-57 {}/{} i={i} out=({wa},{wb},{wi})",
                        kind_name(ka),
                        kind_name(kb)
                    ),
                    &a,
                    &b,
                    &GjkOpts {
                        want_a: wa,
                        want_b: wb,
                        want_iters: wi,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Rows 58-62 — the cache
// ---------------------------------------------------------------------------

/// Row 58: cold cache (all-zero) — the written-back cache is compared too.
#[test]
fn row58_cold_cache() {
    for_each_pair(0x58, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r58 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        cache: Some(c2GJKCache::default()),
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

/// Row 59: warm cache — call twice with the same cache and unchanged shapes.
#[test]
fn row59_warm_cache_same_shapes() {
    let p = api();
    for_each_pair(0x59, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            let ctx = format!("r59 {}/{} i={i}", kind_name(ka), kind_name(kb));
            let mut o = GjkOpts {
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            // First call seeds the cache identically in both libraries.
            let c1 = call_gjk(&p.c, &a, &b, &o);
            let r1 = call_gjk(&p.r, &a, &b, &o);
            eq_f32("r59 pass1 dist", &ctx, c1.dist, r1.dist);
            eq_bits("r59 pass1 cache", &ctx, &c1.cache, &r1.cache);
            eq_v("r59 pass1 outA", &ctx, c1.a, r1.a);
            eq_v("r59 pass1 outB", &ctx, c1.b, r1.b);
            eq_i32("r59 pass1 iters", &ctx, c1.iters, r1.iters);
            // Second call feeds each library its OWN cache back (they are
            // equal, so this is still a differential test, and it verifies the
            // cache-read path against a realistic cache).
            o.cache = Some(c1.cache);
            let c2_ = call_gjk(&p.c, &a, &b, &o);
            let mut o2 = o;
            o2.cache = Some(r1.cache);
            let r2 = call_gjk(&p.r, &a, &b, &o2);
            eq_f32("r59 pass2 dist", &ctx, c2_.dist, r2.dist);
            eq_bits("r59 pass2 cache", &ctx, &c2_.cache, &r2.cache);
            eq_v("r59 pass2 outA", &ctx, c2_.a, r2.a);
            eq_v("r59 pass2 outB", &ctx, c2_.b, r2.b);
            eq_i32("r59 pass2 iters", &ctx, c2_.iters, r2.iters);
        }
    });
}

/// Row 60: warm cache + moved shapes (exercises the metric-staleness test).
#[test]
fn row60_warm_cache_moved_shapes() {
    let p = api();
    for_each_pair(0x60, |rng, ka, kb| {
        for i in 0..M {
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            let ctx = format!("r60 {}/{} i={i}", kind_name(ka), kind_name(kb));
            let o0 = GjkOpts {
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let c1 = call_gjk(&p.c, &a, &b, &o0);
            let r1 = call_gjk(&p.r, &a, &b, &o0);
            eq_bits("r60 seed cache", &ctx, &c1.cache, &r1.cache);
            // Move B by a random amount and re-run with the warm cache.
            let d = c2v::new(rng.uniform(-8.0, 8.0), rng.uniform(-8.0, 8.0));
            let b2 = b.shifted(d);
            let oc = GjkOpts {
                cache: Some(c1.cache),
                ..Default::default()
            };
            let orr = GjkOpts {
                cache: Some(r1.cache),
                ..Default::default()
            };
            let c2_ = call_gjk(&p.c, &a, &b2, &oc);
            let r2 = call_gjk(&p.r, &a, &b2, &orr);
            eq_f32("r60 dist", &ctx, c2_.dist, r2.dist);
            eq_v("r60 outA", &ctx, c2_.a, r2.a);
            eq_v("r60 outB", &ctx, c2_.b, r2.b);
            eq_i32("r60 iters", &ctx, c2_.iters, r2.iters);
            eq_bits("r60 cache", &ctx, &c2_.cache, &r2.cache);
        }
    });
}

/// Row 61: a chain of 8 sequential calls carrying the cache forward while the
/// shape drifts — the real consumer pattern, where state accumulates.
#[test]
fn row61_cache_chain() {
    let p = api();
    for_each_pair(0x61, |rng, ka, kb| {
        for i in 0..M / 4 {
            let (a, mut b) = pair_at(rng, ka, kb, 0);
            let mut ccache = c2GJKCache::default();
            let mut rcache = c2GJKCache::default();
            // Drift B towards, through, and past A.
            let step = c2v::new(rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0));
            for k in 0..8 {
                let ctx = format!("r61 {}/{} i={i} step={k}", kind_name(ka), kind_name(kb));
                let co = GjkOpts {
                    cache: Some(ccache),
                    ..Default::default()
                };
                let ro = GjkOpts {
                    cache: Some(rcache),
                    ..Default::default()
                };
                let cr = call_gjk(&p.c, &a, &b, &co);
                let rr = call_gjk(&p.r, &a, &b, &ro);
                eq_f32("r61 dist", &ctx, cr.dist, rr.dist);
                eq_v("r61 outA", &ctx, cr.a, rr.a);
                eq_v("r61 outB", &ctx, cr.b, rr.b);
                eq_i32("r61 iters", &ctx, cr.iters, rr.iters);
                eq_bits("r61 cache", &ctx, &cr.cache, &rr.cache);
                ccache = cr.cache;
                rcache = rr.cache;
                b = b.shifted(step);
            }
        }
    });
}

/// Row 62: hand-seeded caches with `count` 1/2/3 and **in-range** `iA`/`iB`
/// (strictly `< pX.count`, so no uninitialised proxy slot is read), plus
/// random `metric` / `div`.
#[test]
fn row62_hand_seeded_cache() {
    let vert_count = |k: i32| match k {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    };
    for_each_pair(0x62, |rng, ka, kb| {
        let (na, nb) = (vert_count(ka), vert_count(kb));
        for i in 0..M * 2 {
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            let count = 1 + (i % 3) as i32;
            let mut cache = c2GJKCache {
                metric: match i % 8 {
                    0 => 0.0,
                    1 => -1.0e9,
                    2 => -1.0e7,
                    3 => f32::NAN,
                    4 => f32::INFINITY,
                    5 => FLT_MAX,
                    _ => rng.uniform(-20.0, 20.0),
                },
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: match i % 6 {
                    0 => 1.0,
                    1 => 0.0,
                    2 => f32::NAN,
                    _ => rng.uniform(0.1, 8.0),
                },
            };
            for k in 0..3 {
                cache.iA[k] = rng.below(na) as i32;
                cache.iB[k] = rng.below(nb) as i32;
            }
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!(
                        "r62 {}/{} i={i} count={count} ur={ur}",
                        kind_name(ka),
                        kind_name(kb)
                    ),
                    &a,
                    &b,
                    &GjkOpts {
                        cache: Some(cache),
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Rows 63-69 — degenerate and extreme input shapes
// ---------------------------------------------------------------------------

#[test]
fn row63_degenerate_shapes() {
    for_each_pair(0x63, |rng, ka, kb| {
        for i in 0..M * 2 {
            let ca = c2v::new(rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0));
            let cb = c2v::new(rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0));
            // Mix fully-degenerate with normal shapes so both orders are hit.
            let a = if i % 2 == 0 {
                degenerate_shape(ka, ca)
            } else {
                rand_shape(rng, ka, ca, 1.0)
            };
            let b = if i % 3 == 0 {
                degenerate_shape(kb, cb)
            } else {
                rand_shape(rng, kb, cb, 1.0)
            };
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r63 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    },
                );
            }
        }
    });
}

#[test]
fn row64_inverted_aabb() {
    for_each_pair(0x64, |rng, ka, kb| {
        for i in 0..M {
            let flip = |s: Shape| match s {
                Shape::Aabb(b) => Shape::Aabb(c2AABB {
                    min: b.max,
                    max: b.min,
                }),
                other => other,
            };
            let (a, b) = pair_at(rng, ka, kb, (i % 3) as u8);
            for (fa, fb) in [(true, false), (false, true), (true, true)] {
                let a2 = if fa { flip(a) } else { a };
                let b2 = if fb { flip(b) } else { b };
                diff_gjk(
                    &format!("r64 {}/{} i={i} f=({fa},{fb})", kind_name(ka), kind_name(kb)),
                    &a2,
                    &b2,
                    &GjkOpts::default(),
                );
            }
        }
    });
}

#[test]
fn row65_negative_radius() {
    for_each_pair(0x65, |rng, ka, kb| {
        for i in 0..M * 2 {
            let neg = |s: Shape, r: f32| match s {
                Shape::Circle(c) => Shape::Circle(c2Circle { p: c.p, r }),
                Shape::Capsule(c) => Shape::Capsule(c2Capsule { a: c.a, b: c.b, r }),
                other => other,
            };
            let (a, b) = pair_at(rng, ka, kb, (i % 3) as u8);
            let ra = -rng.uniform(0.0, 6.0);
            let rb = -rng.uniform(0.0, 6.0);
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r65 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &neg(a, ra),
                    &neg(b, rb),
                    &GjkOpts {
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    });
}

#[test]
fn row66_coincident_shapes() {
    for_each_pair(0x66, |rng, ka, kb| {
        for i in 0..M {
            let at = c2v::new(rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0));
            let sc = rng.uniform(0.2, 3.0);
            let a = rand_shape(rng, ka, at, sc);
            // Same centre, same scale — maximal overlap.
            let b = rand_shape(rng, kb, at, sc);
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r66 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    },
                );
            }
            // Literally identical shapes when the kinds match.
            if ka == kb {
                diff_gjk(
                    &format!("r66-ident {} i={i}", kind_name(ka)),
                    &a,
                    &a,
                    &GjkOpts::default(),
                );
            }
        }
    });
}

/// Row 67: coordinates so large that `c2Dot` overflows to `Inf`.
#[test]
fn row67_huge_coordinates() {
    for_each_pair(0x67, |rng, ka, kb| {
        for i in 0..M {
            let sc = [1.0e12f32, 1.0e18, 1.0e30, 1.0e38, FLT_MAX][i % 5];
            let mk = |rng: &mut Rng, k: i32| -> Shape {
                let at = c2v::new(rng.uniform(-1.0, 1.0) * sc, rng.uniform(-1.0, 1.0) * sc);
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                        p: at,
                        r: rng.uniform(0.0, 1.0) * sc,
                    }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: at,
                        max: c2v::new(at.x + rng.uniform(0.0, 1.0) * sc, at.y + sc * 0.1),
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: at,
                        b: c2v::new(at.x + sc * 0.3, at.y - sc * 0.2),
                        r: rng.uniform(0.0, 1.0) * sc,
                    }),
                }
            };
            let (a, b) = (mk(rng, ka), mk(rng, kb));
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r67 {}/{} i={i} sc={sc:e} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    },
                );
            }
        }
    });
}

/// Row 68: subnormal / tiny coordinates and radii.
#[test]
fn row68_tiny_coordinates() {
    for_each_pair(0x68, |rng, ka, kb| {
        for i in 0..M {
            let sc = [
                1.0e-12f32,
                1.0e-20,
                1.0e-30,
                1.0e-38,
                f32::MIN_POSITIVE,
                f32::from_bits(4),
            ][i % 6];
            let mk = |rng: &mut Rng, k: i32| -> Shape {
                let at = c2v::new(rng.uniform(-1.0, 1.0) * sc, rng.uniform(-1.0, 1.0) * sc);
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                        p: at,
                        r: rng.uniform(0.0, 1.0) * sc,
                    }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: at,
                        max: c2v::new(at.x + sc, at.y + sc),
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: at,
                        b: c2v::new(at.x + sc, at.y - sc),
                        r: rng.uniform(0.0, 1.0) * sc,
                    }),
                }
            };
            let (a, b) = (mk(rng, ka), mk(rng, kb));
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r68 {}/{} i={i} sc={sc:e} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    },
                );
            }
        }
    });
}

/// Row 69: NaN / ±Inf coordinates and radii — the C has no guards, so the
/// 20-iteration loop runs with every comparison unordered.
#[test]
fn row69_non_finite_shapes() {
    for_each_pair(0x69, |rng, ka, kb| {
        for i in 0..M * 2 {
            let mk = |rng: &mut Rng, k: i32| -> Shape {
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                        p: rng.wild_vec(),
                        r: rng.wild_f32(),
                    }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: rng.wild_vec(),
                        max: rng.wild_vec(),
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: rng.wild_vec(),
                        b: rng.wild_vec(),
                        r: rng.wild_f32(),
                    }),
                }
            };
            let (a, b) = (mk(rng, ka), mk(rng, kb));
            for ur in [0i32, 1] {
                diff_gjk(
                    &format!("r69 {}/{} i={i} ur={ur}", kind_name(ka), kind_name(kb)),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        cache: Some(c2GJKCache::default()),
                        ax: if i % 3 == 0 { Some(rng.xform_wild()) } else { None },
                        bx: if i % 4 == 0 { Some(rng.xform_wild()) } else { None },
                        ..Default::default()
                    },
                );
            }
        }
    });
}

/// Row 70: drive the loop to its early exits (`d1 > d0`, `dup`) and towards
/// the 20-iteration cap; the compared `*iterations` is what proves which exit
/// was taken.
#[test]
fn row70_loop_exits_and_iteration_count() {
    let p = api();
    let mut hist = [0usize; 21];
    for_each_pair(0x70, |rng, ka, kb| {
        for i in 0..M * 4 {
            // Mix every regime and every transform shape so that all three
            // loop exits (count==3 hit, d1>d0, dup, tiny-d) occur.
            let (a, b) = pair_at(rng, ka, kb, (i % 4) as u8);
            let o = GjkOpts {
                use_radius: (i % 2) as i32,
                ax: if i % 5 == 0 { Some(rng.xform()) } else { None },
                bx: if i % 7 == 0 { Some(rng.xform()) } else { None },
                cache: if i % 3 == 0 {
                    Some(c2GJKCache::default())
                } else {
                    None
                },
                ..Default::default()
            };
            let out = diff_gjk(
                &format!("r70 {}/{} i={i}", kind_name(ka), kind_name(kb)),
                &a,
                &b,
                &o,
            );
            if (0..=20).contains(&out.iters) {
                hist[out.iters as usize] += 1;
            }
            // Also confirm the two libraries agree when called back-to-back
            // many times (no hidden global state).
            if i % 97 == 0 {
                for _ in 0..3 {
                    let c1 = call_gjk(&p.c, &a, &b, &o);
                    let r1 = call_gjk(&p.r, &a, &b, &o);
                    eq_f32("r70 repeat", "", c1.dist, r1.dist);
                    eq_bits("r70 repeat cache", "", &c1.cache, &r1.cache);
                }
            }
        }
    });
    let distinct = hist.iter().filter(|&&n| n > 0).count();
    assert!(
        distinct >= 3,
        "iteration-count coverage too narrow: {hist:?}"
    );
}
