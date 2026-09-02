//! Phase B rows 28-50: `c2GJK`, the top-level entry point, across every
//! type pair / transform / `use_radius` / cache / out-pointer combination.

#![allow(non_snake_case)]

mod common;

use common::*;

/// Samples per (type-pair, option) cell.
const PER_CELL: usize = 220;

fn pair() -> Pair {
    load_pair()
}

fn cold() -> c2GJKCache {
    // A cold cache in the sense the C code means: count == 0. The other fields
    // are deliberately garbage so that "cache_was_good == 0" is really tested.
    c2GJKCache {
        metric: -7.5,
        count: 0,
        iA: [5, 6, 7],
        iB: [1, 2, 3],
        div: -99.0,
    }
}

/// Two shapes placed at a controlled relative distance so that the separated /
/// touching / overlapping / concentric regimes are all reached.
fn shape_pair(rng: &mut Rng, tyA: u32, tyB: u32, mag: f32) -> (Shape, Shape) {
    let a = rand_shape(rng, tyA, mag, 3);
    let mut b = rand_shape(rng, tyB, mag, 3);
    // Nudge B towards / away from A so overlap is common rather than rare.
    let regime = rng.below(5);
    let off = match regime {
        0 => c2v { x: 0.0, y: 0.0 },                       // concentric-ish
        1 => c2v { x: rng.sym(mag * 0.1), y: rng.sym(mag * 0.1) }, // overlapping
        2 => c2v { x: rng.sym(mag), y: rng.sym(mag) },      // mixed
        3 => c2v { x: mag * 3.0, y: rng.sym(mag) },         // separated
        _ => c2v { x: rng.sym(mag * 4.0), y: rng.sym(mag * 4.0) },
    };
    let shift = |v: c2v| c2v { x: v.x + off.x, y: v.y + off.y };
    b = match b {
        Shape::Circle(mut c) => {
            c.p = shift(c.p);
            Shape::Circle(c)
        }
        Shape::Aabb(mut c) => {
            c.min = shift(c.min);
            c.max = shift(c.max);
            Shape::Aabb(c)
        }
        Shape::Capsule(mut c) => {
            c.a = shift(c.a);
            c.b = shift(c.b);
            Shape::Capsule(c)
        }
    };
    (a, b)
}

/// Core sweep: one `CONFIGS.md` cell.
fn sweep(
    p: &Pair,
    tyA: u32,
    tyB: u32,
    use_radius: i32,
    with_ax: bool,
    with_bx: bool,
    mag: f32,
    seed: u64,
    row: &str,
) {
    let mut rng = Rng::new(seed);
    for i in 0..PER_CELL {
        let (a, b) = shape_pair(&mut rng, tyA, tyB, mag);
        let opts = GjkOpts {
            ax: if with_ax { Some(rng.xform(mag)) } else { None },
            bx: if with_bx { Some(rng.xform(mag)) } else { None },
            use_radius,
            want_out_a: true,
            want_out_b: true,
            want_iterations: true,
            cache: false,
        };
        gjk_diff(p, &a, tyA, &b, tyB, &opts, &cold(), &format!("{row} i={i} mag={mag}"));
    }
}

fn sweep_all_pairs(use_radius: i32, with_ax: bool, with_bx: bool, seed: u64, row: &str) {
    let p = pair();
    for (na, &tyA) in ALL_TYPES.iter().enumerate() {
        for (nb, &tyB) in ALL_TYPES.iter().enumerate() {
            for (nm, &mag) in [1.0f32, 50.0, 1.0e4].iter().enumerate() {
                sweep(
                    &p,
                    tyA,
                    tyB,
                    use_radius,
                    with_ax,
                    with_bx,
                    mag,
                    seed ^ ((na * 97 + nb * 13 + nm * 3) as u64),
                    &format!("{row} {}x{}", type_name(tyA), type_name(tyB)),
                );
            }
        }
    }
}

// --- rows 28-37: one row per type pair, identity transforms ----------------

fn pair_row(tyA: u32, tyB: u32, seed: u64, row: &str) {
    let p = pair();
    for use_radius in [0, 1] {
        for (nm, &mag) in [1.0f32, 50.0, 1.0e4].iter().enumerate() {
            sweep(
                &p,
                tyA,
                tyB,
                use_radius,
                false,
                false,
                mag,
                seed ^ (use_radius as u64 * 7919) ^ (nm as u64 * 31),
                row,
            );
        }
    }
}

#[test]
fn rows28_29_circle_vs_circle() {
    pair_row(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE, SEED ^ 28, "row28/29 circle-circle");
    // Explicit regimes: separated, exactly touching, overlapping, concentric.
    let p = pair();
    let cases: [(c2Circle, c2Circle); 5] = [
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2Circle { p: c2v { x: 10.0, y: 0.0 }, r: 2.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: 2.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 }, c2Circle { p: c2v { x: 1.0, y: 0.0 }, r: 2.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }),
    ];
    for (k, (a, b)) in cases.iter().enumerate() {
        for use_radius in [0, 1] {
            let sa = Shape::Circle(*a);
            let sb = Shape::Circle(*b);
            let opts = GjkOpts { use_radius, ..Default::default() };
            gjk_diff(&p, &sa, C2_TYPE_CIRCLE, &sb, C2_TYPE_CIRCLE, &opts, &cold(),
                &format!("row29 explicit k={k} ur={use_radius}"));
        }
    }
}

#[test]
fn row30_circle_vs_aabb() {
    pair_row(C2_TYPE_CIRCLE, C2_TYPE_AABB, SEED ^ 30, "row30 circle-aabb");
}

#[test]
fn row31_circle_vs_capsule() {
    pair_row(C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, SEED ^ 31, "row31 circle-capsule");
}

#[test]
fn row32_aabb_vs_circle() {
    pair_row(C2_TYPE_AABB, C2_TYPE_CIRCLE, SEED ^ 32, "row32 aabb-circle");
}

#[test]
fn row33_aabb_vs_aabb() {
    pair_row(C2_TYPE_AABB, C2_TYPE_AABB, SEED ^ 33, "row33 aabb-aabb");
    let p = pair();
    let cases: [(c2AABB, c2AABB); 6] = [
        // disjoint
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: 6.0, y: 6.0 } }),
        // edge-touching
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 1.0, y: 0.0 }, max: c2v { x: 2.0, y: 1.0 } }),
        // corner-touching
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 2.0, y: 2.0 } }),
        // overlapping
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 2.0, y: 2.0 } },
         c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 3.0, y: 3.0 } }),
        // nested
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 10.0, y: 10.0 } },
         c2AABB { min: c2v { x: 4.0, y: 4.0 }, max: c2v { x: 5.0, y: 5.0 } }),
        // degenerate zero-area
        (c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
    ];
    for (k, (a, b)) in cases.iter().enumerate() {
        for use_radius in [0, 1] {
            let sa = Shape::Aabb(*a);
            let sb = Shape::Aabb(*b);
            let opts = GjkOpts { use_radius, ..Default::default() };
            gjk_diff(&p, &sa, C2_TYPE_AABB, &sb, C2_TYPE_AABB, &opts, &cold(),
                &format!("row33 explicit k={k} ur={use_radius}"));
        }
    }
}

#[test]
fn row34_aabb_vs_capsule() {
    pair_row(C2_TYPE_AABB, C2_TYPE_CAPSULE, SEED ^ 34, "row34 aabb-capsule");
}

#[test]
fn row35_capsule_vs_circle() {
    pair_row(C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, SEED ^ 35, "row35 capsule-circle");
}

#[test]
fn row36_capsule_vs_aabb() {
    pair_row(C2_TYPE_CAPSULE, C2_TYPE_AABB, SEED ^ 36, "row36 capsule-aabb");
}

#[test]
fn row37_capsule_vs_capsule() {
    pair_row(C2_TYPE_CAPSULE, C2_TYPE_CAPSULE, SEED ^ 37, "row37 capsule-capsule");
    let p = pair();
    let cases: [(c2Capsule, c2Capsule); 5] = [
        // parallel
        (c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 1.0 },
         c2Capsule { a: c2v { x: 0.0, y: 5.0 }, b: c2v { x: 10.0, y: 5.0 }, r: 1.0 }),
        // crossing
        (c2Capsule { a: c2v { x: -5.0, y: 0.0 }, b: c2v { x: 5.0, y: 0.0 }, r: 0.5 },
         c2Capsule { a: c2v { x: 0.0, y: -5.0 }, b: c2v { x: 0.0, y: 5.0 }, r: 0.5 }),
        // collinear
        (c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.25 },
         c2Capsule { a: c2v { x: 2.0, y: 0.0 }, b: c2v { x: 3.0, y: 0.0 }, r: 0.25 }),
        // degenerate: a == b (point capsules)
        (c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
         c2Capsule { a: c2v { x: 3.0, y: 0.0 }, b: c2v { x: 3.0, y: 0.0 }, r: 1.0 }),
        // identical
        (c2Capsule { a: c2v { x: 1.0, y: 2.0 }, b: c2v { x: 3.0, y: 4.0 }, r: 0.75 },
         c2Capsule { a: c2v { x: 1.0, y: 2.0 }, b: c2v { x: 3.0, y: 4.0 }, r: 0.75 }),
    ];
    for (k, (a, b)) in cases.iter().enumerate() {
        for use_radius in [0, 1] {
            let sa = Shape::Capsule(*a);
            let sb = Shape::Capsule(*b);
            let opts = GjkOpts { use_radius, ..Default::default() };
            gjk_diff(&p, &sa, C2_TYPE_CAPSULE, &sb, C2_TYPE_CAPSULE, &opts, &cold(),
                &format!("row37 explicit k={k} ur={use_radius}"));
        }
    }
}

// --- rows 38-41: transforms ------------------------------------------------

#[test]
fn row38_ax_only() {
    sweep_all_pairs(1, true, false, SEED ^ 38, "row38 ax-only");
}

#[test]
fn row39_bx_only() {
    sweep_all_pairs(1, false, true, SEED ^ 39, "row39 bx-only");
}

#[test]
fn row40_both_transforms_use_radius() {
    sweep_all_pairs(1, true, true, SEED ^ 40, "row40 both-xform ur=1");
}

#[test]
fn row41_both_transforms_no_radius() {
    sweep_all_pairs(0, true, true, SEED ^ 41, "row41 both-xform ur=0");
}

// --- rows 42-45: cache ------------------------------------------------------

#[test]
fn row42_cold_then_warm_cache() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 42);
    for (na, &tyA) in ALL_TYPES.iter().enumerate() {
        for (nb, &tyB) in ALL_TYPES.iter().enumerate() {
            for i in 0..PER_CELL {
                let (a, b) = shape_pair(&mut rng, tyA, tyB, 50.0);
                let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
                let ctx = format!(
                    "row42 {}x{} i={i} na={na} nb={nb}",
                    type_name(tyA),
                    type_name(tyB)
                );
                // Call 1: cold cache (count == 0) -> cache written back.
                let (oc1, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &cold(), &format!("{ctx} call1"));
                // Call 2: same shapes, warm cache from call 1.
                let (oc2, _) =
                    gjk_diff(&p, &a, tyA, &b, tyB, &opts, &oc1.cache, &format!("{ctx} call2"));
                // Call 3: idempotence of the warm path.
                gjk_diff(&p, &a, tyA, &b, tyB, &opts, &oc2.cache, &format!("{ctx} call3"));
            }
        }
    }
}

#[test]
fn row43_warm_cache_after_moving_shapes() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 43);
    for (na, &tyA) in ALL_TYPES.iter().enumerate() {
        for (nb, &tyB) in ALL_TYPES.iter().enumerate() {
            for i in 0..PER_CELL {
                let (a, b) = shape_pair(&mut rng, tyA, tyB, 50.0);
                let ctx = format!("row43 {}x{} i={i} {na}{nb}", type_name(tyA), type_name(tyB));
                let o1 = GjkOpts {
                    ax: Some(rng.xform(50.0)),
                    bx: Some(rng.xform(50.0)),
                    use_radius: 1,
                    cache: true,
                    ..Default::default()
                };
                let (oc1, _) = gjk_diff(&p, &a, tyA, &b, tyB, &o1, &cold(), &format!("{ctx} c1"));
                // Same cache, different transforms -> exercises the staleness guard.
                let o2 = GjkOpts {
                    ax: Some(rng.xform(50.0)),
                    bx: Some(rng.xform(50.0)),
                    use_radius: 1,
                    cache: true,
                    ..Default::default()
                };
                let (oc2, _) = gjk_diff(&p, &a, tyA, &b, tyB, &o2, &oc1.cache, &format!("{ctx} c2"));
                // And again with the transforms removed entirely.
                let o3 = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
                gjk_diff(&p, &a, tyA, &b, tyB, &o3, &oc2.cache, &format!("{ctx} c3"));
            }
        }
    }
}

#[test]
fn row44_warm_cache_across_type_change() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 44);
    // A cache produced for one type pair is reused for another. Indices are
    // clamped into range so the read stays inside the initialised proxy verts.
    for i in 0..(PER_CELL * 6) {
        let tyA1 = ALL_TYPES[rng.below(3) as usize];
        let tyB1 = ALL_TYPES[rng.below(3) as usize];
        let tyA2 = ALL_TYPES[rng.below(3) as usize];
        let tyB2 = ALL_TYPES[rng.below(3) as usize];
        let (a1, b1) = shape_pair(&mut rng, tyA1, tyB1, 50.0);
        let (a2, b2) = shape_pair(&mut rng, tyA2, tyB2, 50.0);
        let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
        let ctx = format!(
            "row44 i={i} {}x{} -> {}x{}",
            type_name(tyA1),
            type_name(tyB1),
            type_name(tyA2),
            type_name(tyB2)
        );
        let (oc1, _) = gjk_diff(&p, &a1, tyA1, &b1, tyB1, &opts, &cold(), &format!("{ctx} c1"));
        // Clamp the cached indices to the second pair's proxy vertex counts so
        // the reuse stays well-defined (see ERRORS.md UB notes).
        let cap = |t: u32| match t {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let mut cache = oc1.cache;
        cache.count = cache.count.clamp(0, 3);
        for k in 0..3 {
            cache.iA[k] = cache.iA[k].rem_euclid(cap(tyA2));
            cache.iB[k] = cache.iB[k].rem_euclid(cap(tyB2));
        }
        gjk_diff(&p, &a2, tyA2, &b2, tyB2, &opts, &cache, &format!("{ctx} c2"));
    }
}

#[test]
fn row45_long_cache_chain() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 45);
    for (na, &tyA) in ALL_TYPES.iter().enumerate() {
        for (nb, &tyB) in ALL_TYPES.iter().enumerate() {
            for i in 0..60 {
                let (mut a, mut b) = shape_pair(&mut rng, tyA, tyB, 50.0);
                let opts = GjkOpts { use_radius: 1, cache: true, ..Default::default() };
                let mut cache = cold();
                for step in 0..8 {
                    let ctx = format!(
                        "row45 {}x{} i={i} step={step} {na}{nb}",
                        type_name(tyA),
                        type_name(tyB)
                    );
                    let (oc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &cache, &ctx);
                    cache = oc.cache;
                    // Drift the shapes a little between steps.
                    let d = c2v { x: rng.sym(3.0), y: rng.sym(3.0) };
                    let sh = |v: c2v| c2v { x: v.x + d.x, y: v.y + d.y };
                    a = match a {
                        Shape::Circle(mut c) => { c.p = sh(c.p); Shape::Circle(c) }
                        Shape::Aabb(mut c) => { c.min = sh(c.min); c.max = sh(c.max); Shape::Aabb(c) }
                        Shape::Capsule(mut c) => { c.a = sh(c.a); c.b = sh(c.b); Shape::Capsule(c) }
                    };
                    b = match b {
                        Shape::Circle(mut c) => { c.p = sh(c.p); Shape::Circle(c) }
                        Shape::Aabb(mut c) => { c.min = sh(c.min); c.max = sh(c.max); Shape::Aabb(c) }
                        Shape::Capsule(mut c) => { c.a = sh(c.a); c.b = sh(c.b); Shape::Capsule(c) }
                    };
                }
            }
        }
    }
}

// --- row 46: optional out-parameters ---------------------------------------

#[test]
fn row46_optional_out_parameters() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..(PER_CELL * 8) {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let (a, b) = shape_pair(&mut rng, tyA, tyB, 50.0);
        for (wa, wb, wi) in [
            (false, true, true),
            (true, false, true),
            (false, false, true),
            (true, true, false),
            (false, false, false),
        ] {
            for use_radius in [0, 1] {
                for with_cache in [false, true] {
                    let opts = GjkOpts {
                        ax: if rng.bool() { Some(rng.xform(50.0)) } else { None },
                        bx: if rng.bool() { Some(rng.xform(50.0)) } else { None },
                        use_radius,
                        want_out_a: wa,
                        want_out_b: wb,
                        want_iterations: wi,
                        cache: with_cache,
                    };
                    let (oc, or) = gjk_diff(
                        &p, &a, tyA, &b, tyB, &opts, &cold(),
                        &format!("row46 i={i} wa={wa} wb={wb} wi={wi} ur={use_radius} cache={with_cache}"),
                    );
                    // Unwritten out-params must still hold the sentinel on BOTH sides.
                    if !wa {
                        ck_v(oc.a, sentinel_v(), "row46 C wrote outA despite NULL");
                        ck_v(or.a, sentinel_v(), "row46 Rust wrote outA despite NULL");
                    }
                    if !wb {
                        ck_v(oc.b, sentinel_v(), "row46 C wrote outB despite NULL");
                        ck_v(or.b, sentinel_v(), "row46 Rust wrote outB despite NULL");
                    }
                    if !wi {
                        assert_eq!(oc.iters, sentinel_i(), "row46 C wrote iterations despite NULL");
                        assert_eq!(or.iters, sentinel_i(), "row46 Rust wrote iterations despite NULL");
                    }
                }
            }
        }
    }
}

// --- row 47: radius boundary values ----------------------------------------

#[test]
fn row47_radius_edges() {
    let p = pair();
    // dist == rA + rB exactly (the `dist > rA + rB` boundary), plus r == 0 and
    // absurdly large radii.
    let mut cases: Vec<(Shape, Shape, String)> = Vec::new();
    for (i, &(d, ra, rb)) in [
        (3.0f32, 1.0f32, 2.0f32),  // exactly touching after shrink
        (3.0, 1.5, 1.5),
        (3.0, 0.0, 3.0),
        (3.0, 3.0, 0.0),
        (3.0, 0.0, 0.0),
        (3.0, 100.0, 100.0),       // radii swamp the distance
        (1.0e-6, 0.0, 0.0),        // dist just above FLT_EPSILON
        (1.0e-8, 0.0, 0.0),        // dist below FLT_EPSILON
        (0.0, 0.0, 0.0),           // coincident
        (3.0e38, 1.0, 1.0),        // huge separation
    ]
    .iter()
    .enumerate()
    {
        cases.push((
            Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra }),
            Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: rb }),
            format!("row47 circle k={i} d={d} rA={ra} rB={rb}"),
        ));
        cases.push((
            Shape::Capsule(c2Capsule { a: c2v { x: 0.0, y: -1.0 }, b: c2v { x: 0.0, y: 1.0 }, r: ra }),
            Shape::Capsule(c2Capsule { a: c2v { x: d, y: -1.0 }, b: c2v { x: d, y: 1.0 }, r: rb }),
            format!("row47 capsule k={i} d={d} rA={ra} rB={rb}"),
        ));
    }
    for (a, b, ctx) in &cases {
        for use_radius in [0, 1] {
            for with_cache in [false, true] {
                let opts = GjkOpts { use_radius, cache: with_cache, ..Default::default() };
                gjk_diff(&p, a, a.ty(), b, b.ty(), &opts, &cold(),
                    &format!("{ctx} ur={use_radius} cache={with_cache}"));
            }
        }
    }
    // Randomized: radius chosen relative to the actual separation.
    let mut rng = Rng::new(SEED ^ 47);
    for i in 0..(PER_CELL * 6) {
        let d = rng.unit() * 10.0;
        let frac = rng.unit() * 2.0;
        let ra = d * frac * 0.5;
        let rb = d * frac * 0.5;
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
        let b = Shape::Capsule(c2Capsule {
            a: c2v { x: d, y: -1.0 },
            b: c2v { x: d, y: 1.0 },
            r: rb,
        });
        for use_radius in [0, 1] {
            let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
            gjk_diff(&p, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CAPSULE, &opts, &cold(),
                &format!("row47 rand i={i} d={d} ur={use_radius}"));
        }
    }
}

// --- row 48: extreme scales -------------------------------------------------

#[test]
fn row48_extreme_scales() {
    let p = pair();
    for (mi, &mag) in [1.0e18f32, 1.0e-30, 1.0e30, 1.0e-6, 3.0e38].iter().enumerate() {
        let mut rng = Rng::new(SEED ^ 48 ^ mi as u64);
        for (na, &tyA) in ALL_TYPES.iter().enumerate() {
            for (nb, &tyB) in ALL_TYPES.iter().enumerate() {
                for i in 0..60 {
                    let (a, b) = shape_pair(&mut rng, tyA, tyB, mag);
                    for use_radius in [0, 1] {
                        let opts = GjkOpts {
                            ax: if rng.bool() { Some(rng.xform(mag)) } else { None },
                            bx: if rng.bool() { Some(rng.xform(mag)) } else { None },
                            use_radius,
                            cache: rng.bool(),
                            ..Default::default()
                        };
                        gjk_diff(&p, &a, tyA, &b, tyB, &opts, &cold(),
                            &format!("row48 mag={mag} {}x{} i={i} {na}{nb}", type_name(tyA), type_name(tyB)));
                    }
                }
            }
        }
    }
}

// --- row 49: deep overlap forcing hit = 1 ----------------------------------

#[test]
fn row49_deep_overlap_hit_path() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 49);
    let mut hits = 0usize;
    let mut total = 0usize;
    for (na, &tyA) in ALL_TYPES.iter().enumerate() {
        for (nb, &tyB) in ALL_TYPES.iter().enumerate() {
            for i in 0..PER_CELL {
                // Concentric, similar size -> origin is enclosed -> s.count == 3.
                let a = match tyA {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: c2v { x: -5.0, y: -5.0 },
                        max: c2v { x: 5.0, y: 5.0 },
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: c2v { x: -5.0, y: 0.0 },
                        b: c2v { x: 5.0, y: 0.0 },
                        r: 2.0,
                    }),
                };
                let jitter = c2v { x: rng.sym(1.0), y: rng.sym(1.0) };
                let b = match tyB {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: jitter, r: 4.0 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: c2v { x: jitter.x - 4.0, y: jitter.y - 4.0 },
                        max: c2v { x: jitter.x + 4.0, y: jitter.y + 4.0 },
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: c2v { x: jitter.x - 4.0, y: jitter.y },
                        b: c2v { x: jitter.x + 4.0, y: jitter.y },
                        r: 1.5,
                    }),
                };
                for use_radius in [0, 1] {
                    let opts = GjkOpts { use_radius, cache: true, ..Default::default() };
                    let (oc, _) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &cold(),
                        &format!("row49 {}x{} i={i} ur={use_radius} {na}{nb}", type_name(tyA), type_name(tyB)));
                    total += 1;
                    if oc.cache.count == 3 {
                        hits += 1;
                    }
                }
            }
        }
    }
    assert!(hits > 0, "row49 never reached the hit path (count==3)");
    println!("row49 hit path reached {hits}/{total} times");
}

// --- row 50: iteration behaviour -------------------------------------------

#[test]
fn row50_iteration_counts_and_break_paths() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 50);
    let mut hist = [0usize; 21];
    for i in 0..(PER_CELL * 20) {
        let tyA = ALL_TYPES[rng.below(3) as usize];
        let tyB = ALL_TYPES[rng.below(3) as usize];
        let mag = [1.0f32, 1.0e-4, 1.0e4, 1.0][rng.below(4) as usize];
        let (a, b) = shape_pair(&mut rng, tyA, tyB, mag);
        let opts = GjkOpts {
            ax: if rng.bool() { Some(rng.xform(mag)) } else { None },
            bx: if rng.bool() { Some(rng.xform(mag)) } else { None },
            use_radius: rng.below(2) as i32,
            cache: rng.bool(),
            ..Default::default()
        };
        let (oc, or) = gjk_diff(&p, &a, tyA, &b, tyB, &opts, &cold(), &format!("row50 i={i}"));
        assert_eq!(oc.iters, or.iters);
        assert!(
            (0..=20).contains(&oc.iters),
            "iterations out of the documented 0..=20 range: {}",
            oc.iters
        );
        hist[oc.iters as usize] += 1;
    }
    let distinct = hist.iter().filter(|&&n| n > 0).count();
    assert!(distinct >= 3, "row50 iteration spread too narrow: {hist:?}");
    println!("row50 iteration histogram: {hist:?}");
}
