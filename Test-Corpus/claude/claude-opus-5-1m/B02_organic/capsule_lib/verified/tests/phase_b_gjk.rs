//! Phase B, levels 3-6: rows B31..B71 of CONFIGS.md — `c2GJK`, the six
//! boolean shape predicates, the `c2Collided` dispatcher and `capsule`.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Shape-pair generators covering the geometric relations the C distinguishes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rel {
    /// cores deeply overlapping -> origin enclosed -> `hit` path
    Overlap,
    /// distance exactly rA + rB (integer coordinates so it is exact)
    Touch,
    /// just barely separated
    Near,
    /// far apart
    Far,
    /// degenerate shapes (zero/negative radius, a == b, min == max, inverted)
    Degenerate,
    /// huge coordinates
    Huge,
    /// special float values
    Special,
    /// completely unconstrained
    Any,
}

pub const RELS: [Rel; 8] = [
    Rel::Overlap,
    Rel::Touch,
    Rel::Near,
    Rel::Far,
    Rel::Degenerate,
    Rel::Huge,
    Rel::Special,
    Rel::Any,
];

/// Build a shape of type `ty` centred at `c` with half-extent `e` and radius `r`.
fn build(rng: &mut Rng, ty: usize, c: C2v, e: f32, r: f32) -> Shape {
    match ty {
        0 => Shape::Circle(C2Circle { p: c, r }),
        1 => Shape::Aabb(C2Aabb {
            min: C2v { x: c.x - e, y: c.y - e },
            max: C2v { x: c.x + e, y: c.y + e },
        }),
        _ => {
            let d = match rng.below(3) {
                0 => C2v { x: e, y: 0.0 },
                1 => C2v { x: 0.0, y: e },
                _ => C2v { x: e, y: e },
            };
            Shape::Capsule(C2Capsule {
                a: C2v { x: c.x - d.x, y: c.y - d.y },
                b: C2v { x: c.x + d.x, y: c.y + d.y },
                r,
            })
        }
    }
}

/// A pair of shapes of the requested types in the requested geometric relation.
pub fn pair(rng: &mut Rng, tya: usize, tyb: usize, rel: Rel) -> (Shape, Shape) {
    match rel {
        Rel::Overlap => {
            let c = rng.v_small();
            let (ea, ra) = (rng.f32_in(4.0, 20.0), rng.f32_in(2.0, 10.0));
            let a = build(rng, tya, c, ea, ra);
            let (dx, dy) = (rng.f32_in(-3.0, 3.0), rng.f32_in(-3.0, 3.0));
            let (eb, rb) = (rng.f32_in(4.0, 20.0), rng.f32_in(2.0, 10.0));
            let b = build(rng, tyb, C2v { x: c.x + dx, y: c.y + dy }, eb, rb);
            (a, b)
        }
        Rel::Touch => {
            // Integer geometry: axis-aligned, so the core distance is exactly
            // representable and can land exactly on rA + rB.
            let ea = (rng.below(6) + 1) as f32;
            let eb = (rng.below(6) + 1) as f32;
            let ra = (rng.below(5)) as f32;
            let rb = (rng.below(5)) as f32;
            let gap = ea + eb + ra + rb; // exact touch of the inflated shapes
            let y = (rng.below(9) as f32) - 4.0;
            let a = build(rng, tya, C2v { x: 0.0, y: 0.0 }, ea, ra);
            let by = if rng.chance(2) { 0.0 } else { y };
            let b = build(rng, tyb, C2v { x: gap, y: by }, eb, rb);
            (a, b)
        }
        Rel::Near => {
            let ea = rng.f32_in(1.0, 10.0);
            let eb = rng.f32_in(1.0, 10.0);
            let ra = rng.f32_in(0.0, 5.0);
            let rb = rng.f32_in(0.0, 5.0);
            let gap = ea + eb + ra + rb + rng.f32_in(0.001, 1.5);
            let a = build(rng, tya, C2v { x: 0.0, y: 0.0 }, ea, ra);
            let by = rng.f32_in(-2.0, 2.0);
            let b = build(rng, tyb, C2v { x: gap, y: by }, eb, rb);
            (a, b)
        }
        Rel::Far => {
            let ca = rng.v_small();
            let (ea, ra) = (rng.f32_in(1.0, 8.0), rng.f32_in(0.0, 4.0));
            let a = build(rng, tya, ca, ea, ra);
            let cb = C2v { x: rng.f32_in(200.0, 4000.0), y: rng.f32_in(-4000.0, 4000.0) };
            let (eb, rb) = (rng.f32_in(1.0, 8.0), rng.f32_in(0.0, 4.0));
            let b = build(rng, tyb, cb, eb, rb);
            (a, b)
        }
        Rel::Degenerate => {
            let degen = |rng: &mut Rng, ty: usize| -> Shape {
                let c = rng.v_small();
                match ty {
                    0 => Shape::Circle(C2Circle {
                        p: c,
                        r: match rng.below(3) {
                            0 => 0.0,
                            1 => -rng.f32_in(0.0, 5.0),
                            _ => -0.0,
                        },
                    }),
                    1 => Shape::Aabb(match rng.below(3) {
                        0 => C2Aabb { min: c, max: c },
                        1 => C2Aabb {
                            min: C2v { x: c.x + 3.0, y: c.y + 3.0 },
                            max: c,
                        },
                        _ => C2Aabb {
                            min: c,
                            max: C2v { x: c.x, y: c.y + 4.0 },
                        },
                    }),
                    _ => Shape::Capsule(C2Capsule {
                        a: c,
                        b: if rng.chance(2) { c } else { rng.v_small() },
                        r: match rng.below(3) {
                            0 => 0.0,
                            1 => -rng.f32_in(0.0, 5.0),
                            _ => rng.f32_in(0.0, 4.0),
                        },
                    }),
                }
            };
            (degen(rng, tya), degen(rng, tyb))
        }
        Rel::Huge => {
            let mk = |rng: &mut Rng, ty: usize| -> Shape {
                let c = C2v { x: rng.huge(), y: rng.huge() };
                let e = if rng.chance(2) { rng.huge() } else { rng.f32_in(1.0, 100.0) };
                let rr = if rng.chance(2) { rng.huge() } else { 5.0 };
                build(rng, ty, c, e, rr)
            };
            (mk(rng, tya), mk(rng, tyb))
        }
        Rel::Special => {
            let mk = |rng: &mut Rng, ty: usize| -> Shape {
                match ty {
                    0 => Shape::Circle(C2Circle { p: rng.v_any(), r: rng.any() }),
                    1 => Shape::Aabb(C2Aabb { min: rng.v_any(), max: rng.v_any() }),
                    _ => Shape::Capsule(C2Capsule {
                        a: rng.v_any(),
                        b: rng.v_any(),
                        r: rng.any(),
                    }),
                }
            };
            (mk(rng, tya), mk(rng, tyb))
        }
        Rel::Any => (shape_of(rng, tya), shape_of(rng, tyb)),
    }
}

fn rnd_xform(rng: &mut Rng, translate: bool, rotate: bool, weird: bool) -> C2x {
    C2x {
        p: if translate {
            rng.v_coord()
        } else {
            C2v { x: 0.0, y: 0.0 }
        },
        r: if weird {
            rng.rot_weird()
        } else if rotate {
            rng.rot()
        } else {
            C2r { c: 1.0, s: 0.0 }
        },
    }
}

/// Runs one type-pair over every geometric relation, for both `use_radius`
/// settings, with the given transform factory.
fn gjk_type_pair(
    d: &mut Diff,
    c: &Api,
    r: &Api,
    tag: &str,
    seed: u64,
    tya: usize,
    tyb: usize,
    per_rel: usize,
    use_radius: &[c_int],
    mut xf: impl FnMut(&mut Rng) -> (Option<C2x>, Option<C2x>),
) {
    let mut rng = Rng::new(seed);
    for rel in RELS {
        for i in 0..per_rel {
            let (a, b) = pair(&mut rng, tya, tyb, rel);
            let (ax, bx) = xf(&mut rng);
            for &ur in use_radius {
                let o = GjkOpts {
                    ax,
                    bx,
                    use_radius: ur,
                    ..Default::default()
                };
                let ctx = format!(
                    "{tag}/{}x{} {rel:?}#{i} ur={ur} A={a:?} B={b:?} ax={ax:?} bx={bx:?}",
                    TYPE_NAMES[tya], TYPE_NAMES[tyb]
                );
                gjk_case(d, c, r, &ctx, &a, &b, &o);
            }
        }
    }
}

macro_rules! type_pair_test {
    ($name:ident, $row:expr, $tya:expr, $tyb:expr, $seed:expr, $urs:expr) => {
        #[test]
        fn $name() {
            let (c, r) = load_pair();
            let mut d = Diff::new($row);
            gjk_type_pair(
                &mut d, &c, &r, $row, SEED ^ $seed, $tya, $tyb, 300, $urs,
                |_rng| (None, None),
            );
            d.finish();
        }
    };
}

// B31..B40: every type pair, no transforms, both use_radius settings.
type_pair_test!(B31_gjk_circle_circle_ur1, "B31 gjk circle x circle ur=1", 0, 0, 100, &[1]);
type_pair_test!(B32_gjk_circle_circle_ur0, "B32 gjk circle x circle ur=0", 0, 0, 101, &[0]);
type_pair_test!(B33_gjk_circle_aabb, "B33 gjk circle x aabb", 0, 1, 102, &[0, 1]);
type_pair_test!(B34_gjk_circle_capsule, "B34 gjk circle x capsule", 0, 2, 103, &[0, 1]);
type_pair_test!(B35_gjk_aabb_circle, "B35 gjk aabb x circle", 1, 0, 104, &[0, 1]);
type_pair_test!(B36_gjk_aabb_aabb, "B36 gjk aabb x aabb", 1, 1, 105, &[0, 1]);
type_pair_test!(B37_gjk_aabb_capsule, "B37 gjk aabb x capsule", 1, 2, 106, &[0, 1]);
type_pair_test!(B38_gjk_capsule_circle, "B38 gjk capsule x circle", 2, 0, 107, &[0, 1]);
type_pair_test!(B39_gjk_capsule_aabb, "B39 gjk capsule x aabb", 2, 1, 108, &[0, 1]);
type_pair_test!(B40_gjk_capsule_capsule, "B40 gjk capsule x capsule", 2, 2, 109, &[0, 1]);

/// Helper: all 9 type pairs with a transform factory.
fn all_pairs_with_xform(
    row: &str,
    seed: u64,
    per_rel: usize,
    urs: &[c_int],
    mut xf: impl FnMut(&mut Rng) -> (Option<C2x>, Option<C2x>),
) {
    let (c, r) = load_pair();
    let mut d = Diff::new(row);
    for tya in 0..3 {
        for tyb in 0..3 {
            gjk_type_pair(
                &mut d,
                &c,
                &r,
                row,
                seed ^ ((tya * 3 + tyb) as u64) << 20,
                tya,
                tyb,
                per_rel,
                urs,
                &mut xf,
            );
        }
    }
    d.finish();
}

#[test]
fn B41_gjk_identity_transforms() {
    all_pairs_with_xform("B41 gjk explicit identity transforms", SEED ^ 110, 40, &[0, 1], |_| {
        let id = C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: C2r { c: 1.0, s: 0.0 },
        };
        (Some(id), Some(id))
    });
}

#[test]
fn B42_gjk_ax_only() {
    all_pairs_with_xform("B42 gjk ax rotated+translated, bx NULL", SEED ^ 111, 40, &[0, 1], |g| {
        (Some(rnd_xform(g, true, true, false)), None)
    });
}

#[test]
fn B43_gjk_bx_only() {
    all_pairs_with_xform("B43 gjk ax NULL, bx rotated+translated", SEED ^ 112, 40, &[0, 1], |g| {
        (None, Some(rnd_xform(g, true, true, false)))
    });
}

#[test]
fn B44_gjk_both_transforms() {
    all_pairs_with_xform("B44 gjk both rotated+translated", SEED ^ 113, 40, &[0, 1], |g| {
        (
            Some(rnd_xform(g, true, true, false)),
            Some(rnd_xform(g, true, true, false)),
        )
    });
}

#[test]
fn B45_gjk_translation_only_and_rotation_only() {
    all_pairs_with_xform("B45 gjk translation-only / rotation-only", SEED ^ 114, 25, &[0, 1], |g| {
        if g.chance(2) {
            (
                Some(rnd_xform(g, true, false, false)),
                Some(rnd_xform(g, true, false, false)),
            )
        } else {
            (
                Some(rnd_xform(g, false, true, false)),
                Some(rnd_xform(g, false, true, false)),
            )
        }
    });
}

#[test]
fn B46_gjk_non_normalised_rotations() {
    all_pairs_with_xform("B46 gjk non-normalised c2r in transforms", SEED ^ 115, 40, &[0, 1], |g| {
        (
            Some(rnd_xform(g, true, true, true)),
            Some(rnd_xform(g, true, true, true)),
        )
    });
}

// ---------------------------------------------------------------------------
// Cache rows
// ---------------------------------------------------------------------------

#[test]
fn B47_gjk_cold_cache() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B47 gjk cold cache (count == 0), write-back compared");
    let mut rng = Rng::new(SEED ^ 120);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in RELS {
                for i in 0..25 {
                    let (a, b) = pair(&mut rng, tya, tyb, rel);
                    // Two flavours of "cold": all zeros, and count==0 with
                    // garbage everywhere else (the C must leave the slots it
                    // does not write untouched).
                    let start = if i % 2 == 0 {
                        C2GJKCache::default()
                    } else {
                        let mut g: C2GJKCache = poison(i as u8);
                        g.count = 0;
                        g
                    };
                    let o = GjkOpts {
                        use_radius: (i % 3 != 0) as c_int,
                        ..Default::default()
                    };
                    let ctx = format!(
                        "B47/{}x{} {rel:?}#{i} A={a:?} B={b:?} start={start:?}",
                        TYPE_NAMES[tya], TYPE_NAMES[tyb]
                    );
                    gjk_case_cached(&mut d, &c, &r, &ctx, &a, &b, &o, &start);
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B48_gjk_warm_cache_same_call_twice() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B48 gjk warm cache, identical call issued twice");
    let mut rng = Rng::new(SEED ^ 121);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in RELS {
                for i in 0..25 {
                    let (a, b) = pair(&mut rng, tya, tyb, rel);
                    let o = GjkOpts {
                        use_radius: (i % 2) as c_int,
                        ..Default::default()
                    };
                    let ctx = format!(
                        "B48/{}x{} {rel:?}#{i} A={a:?} B={b:?}",
                        TYPE_NAMES[tya], TYPE_NAMES[tyb]
                    );
                    let (cc, rc) = gjk_case_cached(
                        &mut d,
                        &c,
                        &r,
                        &format!("{ctx}/pass1"),
                        &a,
                        &b,
                        &o,
                        &C2GJKCache::default(),
                    );
                    // Second pass: each library re-reads the cache it wrote.
                    let mut cc2 = cc;
                    let mut rc2 = rc;
                    let co = call_gjk(&c, &a, &b, &o, Some(&mut cc2));
                    let ro = call_gjk(&r, &a, &b, &o, Some(&mut rc2));
                    cmp_gjk(&mut d, &format!("{ctx}/pass2"), &co, &ro);
                    d.cache(&format!("{ctx}/pass2 cache"), &cc2, &rc2);
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B49_gjk_warm_cache_moving_shapes() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B49 gjk warm cache across 4 frames with moving shapes");
    let mut rng = Rng::new(SEED ^ 122);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in RELS {
                for i in 0..12 {
                    let (a0, b0) = pair(&mut rng, tya, tyb, rel);
                    let o = GjkOpts {
                        use_radius: (i % 2) as c_int,
                        ..Default::default()
                    };
                    let mut cc = C2GJKCache::default();
                    let mut rc = C2GJKCache::default();
                    let mut a = a0;
                    let mut b = b0;
                    for frame in 0..4 {
                        let ctx = format!(
                            "B49/{}x{} {rel:?}#{i}/frame{frame} A={a:?} B={b:?}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        );
                        let co = call_gjk(&c, &a, &b, &o, Some(&mut cc));
                        let ro = call_gjk(&r, &a, &b, &o, Some(&mut rc));
                        cmp_gjk(&mut d, &ctx, &co, &ro);
                        d.cache(&format!("{ctx}/cache"), &cc, &rc);
                        let step = C2v {
                            x: rng.f32_in(-6.0, 6.0),
                            y: rng.f32_in(-6.0, 6.0),
                        };
                        a = a.translated(C2v { x: -step.x, y: -step.y });
                        b = b.translated(step);
                    }
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B50_gjk_cache_and_transforms_and_no_radius() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B50 gjk warm cache + transforms + use_radius=0");
    let mut rng = Rng::new(SEED ^ 123);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in RELS {
                for i in 0..12 {
                    let (a0, b0) = pair(&mut rng, tya, tyb, rel);
                    let ax = rnd_xform(&mut rng, true, true, i % 4 == 0);
                    let bx = rnd_xform(&mut rng, true, true, i % 5 == 0);
                    let o = GjkOpts {
                        ax: Some(ax),
                        bx: Some(bx),
                        use_radius: 0,
                        ..Default::default()
                    };
                    let mut cc = C2GJKCache::default();
                    let mut rc = C2GJKCache::default();
                    let mut a = a0;
                    let mut b = b0;
                    for frame in 0..3 {
                        let ctx = format!(
                            "B50/{}x{} {rel:?}#{i}/frame{frame} A={a:?} B={b:?} ax={ax:?} bx={bx:?}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        );
                        let co = call_gjk(&c, &a, &b, &o, Some(&mut cc));
                        let ro = call_gjk(&r, &a, &b, &o, Some(&mut rc));
                        cmp_gjk(&mut d, &ctx, &co, &ro);
                        d.cache(&format!("{ctx}/cache"), &cc, &rc);
                        let step = C2v {
                            x: rng.f32_in(-4.0, 4.0),
                            y: rng.f32_in(-4.0, 4.0),
                        };
                        a = a.translated(step);
                        b = b.translated(C2v { x: -step.y, y: step.x });
                    }
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B51_gjk_out_param_subsets() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B51 gjk out-parameter subsets (NULL combinations)");
    let mut rng = Rng::new(SEED ^ 124);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in RELS {
                for mask in 0..8u32 {
                    for i in 0..3 {
                        let (a, b) = pair(&mut rng, tya, tyb, rel);
                        let o = GjkOpts {
                            use_radius: (i % 2) as c_int,
                            want_outa: mask & 1 != 0,
                            want_outb: mask & 2 != 0,
                            want_iters: mask & 4 != 0,
                            ..Default::default()
                        };
                        let ctx = format!(
                            "B51/{}x{} {rel:?} mask={mask}#{i} A={a:?} B={b:?}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        );
                        gjk_case(&mut d, &c, &r, &ctx, &a, &b, &o);
                        // and the same with a cache but no out-params at all
                        if mask == 0 {
                            gjk_case_cached(
                                &mut d,
                                &c,
                                &r,
                                &format!("{ctx}/cache-only"),
                                &a,
                                &b,
                                &o,
                                &C2GJKCache::default(),
                            );
                        }
                    }
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B52_gjk_deep_overlap_hit_path() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B52 gjk deep overlap (hit path, simplex count == 3)");
    let mut rng = Rng::new(SEED ^ 125);
    let mut hits = 0u32;
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..200 {
                // Concentric, large, guaranteed to enclose the origin.
                let ctr = rng.v_small();
                let (ea, ra) = (rng.f32_in(10.0, 40.0), rng.f32_in(1.0, 8.0));
                let a = build(&mut rng, tya, ctr, ea, ra);
                let (dx, dy) = (rng.f32_in(-2.0, 2.0), rng.f32_in(-2.0, 2.0));
                let (eb, rb) = (rng.f32_in(10.0, 40.0), rng.f32_in(1.0, 8.0));
                let b = build(&mut rng, tyb, C2v { x: ctr.x + dx, y: ctr.y + dy }, eb, rb);
                for ur in [0, 1] {
                    let o = GjkOpts { use_radius: ur, ..Default::default() };
                    let ctx = format!(
                        "B52/{}x{}#{i} ur={ur} A={a:?} B={b:?}",
                        TYPE_NAMES[tya], TYPE_NAMES[tyb]
                    );
                    let co = call_gjk(&c, &a, &b, &o, None);
                    let ro = call_gjk(&r, &a, &b, &o, None);
                    cmp_gjk(&mut d, &ctx, &co, &ro);
                    if co.dist == 0.0 {
                        hits += 1;
                    }
                }
            }
        }
    }
    d.finish();
    assert!(hits > 500, "expected many zero-distance (hit) results, got {hits}");
    eprintln!("B52 zero-distance results: {hits}");
}

#[test]
fn B53_gjk_exact_touch() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B53 gjk exactly touching (integer geometry)");
    let mut rng = Rng::new(SEED ^ 126);
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..250 {
                let (a, b) = pair(&mut rng, tya, tyb, Rel::Touch);
                for ur in [0, 1] {
                    let o = GjkOpts { use_radius: ur, ..Default::default() };
                    let ctx = format!(
                        "B53/{}x{}#{i} ur={ur} A={a:?} B={b:?}",
                        TYPE_NAMES[tya], TYPE_NAMES[tyb]
                    );
                    gjk_case(&mut d, &c, &r, &ctx, &a, &b, &o);
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B54_gjk_far_and_huge() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B54 gjk far apart / huge magnitudes");
    let mut rng = Rng::new(SEED ^ 127);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in [Rel::Far, Rel::Huge] {
                for i in 0..150 {
                    let (a, b) = pair(&mut rng, tya, tyb, rel);
                    for ur in [0, 1] {
                        let o = GjkOpts { use_radius: ur, ..Default::default() };
                        let ctx = format!(
                            "B54/{}x{} {rel:?}#{i} ur={ur} A={a:?} B={b:?}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        );
                        gjk_case(&mut d, &c, &r, &ctx, &a, &b, &o);
                        gjk_case_cached(
                            &mut d,
                            &c,
                            &r,
                            &format!("{ctx}/cached"),
                            &a,
                            &b,
                            &o,
                            &C2GJKCache::default(),
                        );
                    }
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B55_gjk_degenerate_shapes() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B55 gjk degenerate shapes (r<=0, a==b, min==max, inverted)");
    let mut rng = Rng::new(SEED ^ 128);
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..300 {
                let (a, b) = pair(&mut rng, tya, tyb, Rel::Degenerate);
                for ur in [0, 1] {
                    let o = GjkOpts { use_radius: ur, ..Default::default() };
                    let ctx = format!(
                        "B55/{}x{}#{i} ur={ur} A={a:?} B={b:?}",
                        TYPE_NAMES[tya], TYPE_NAMES[tyb]
                    );
                    gjk_case(&mut d, &c, &r, &ctx, &a, &b, &o);
                    gjk_case_cached(
                        &mut d,
                        &c,
                        &r,
                        &format!("{ctx}/cached"),
                        &a,
                        &b,
                        &o,
                        &C2GJKCache::default(),
                    );
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B56_gjk_identical_shapes() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B56 gjk byte-identical shapes at the same position");
    let mut rng = Rng::new(SEED ^ 129);
    for ty in 0..3 {
        for i in 0..400 {
            let s = shape_of(&mut rng, ty);
            for ur in [0, 1] {
                let o = GjkOpts { use_radius: ur, ..Default::default() };
                // distinct copies -> distinct pointers, identical bytes
                let s2 = s;
                let ctx = format!("B56/{}#{i} ur={ur} S={s:?}", TYPE_NAMES[ty]);
                gjk_case(&mut d, &c, &r, &ctx, &s, &s2, &o);
                gjk_case_cached(
                    &mut d,
                    &c,
                    &r,
                    &format!("{ctx}/cached"),
                    &s,
                    &s2,
                    &o,
                    &C2GJKCache::default(),
                );
            }
        }
    }
    d.finish();
}

#[test]
fn B57_gjk_aliased_arguments() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B57 gjk aliased arguments (same pointer for A and B)");
    let mut rng = Rng::new(SEED ^ 130);
    for ty in 0..3 {
        for i in 0..400 {
            let s = shape_of(&mut rng, ty);
            for ur in [0, 1] {
                let o = GjkOpts { use_radius: ur, ..Default::default() };
                let ctx = format!("B57/{}#{i} ur={ur} S={s:?}", TYPE_NAMES[ty]);
                gjk_case(&mut d, &c, &r, &ctx, &s, &s, &o);
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Level 4 — the six boolean predicates
// ---------------------------------------------------------------------------

#[test]
fn B58_aabb_to_aabb() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B58 c2AABBtoAABB");
    let mut rng = Rng::new(SEED ^ 140);
    let mut collided = 0u32;
    for i in 0..20000 {
        let (a, b) = match rng.below(6) {
            // exact edge / corner touching (integer coordinates)
            0 => {
                let x = (rng.below(21) as f32) - 10.0;
                let y = (rng.below(21) as f32) - 10.0;
                let w = (rng.below(6) + 1) as f32;
                (
                    C2Aabb { min: C2v { x, y }, max: C2v { x: x + w, y: y + w } },
                    C2Aabb {
                        min: C2v { x: x + w, y: y + w },
                        max: C2v { x: x + 2.0 * w, y: y + 2.0 * w },
                    },
                )
            }
            1 => {
                // nested
                let m = rng.v_coord();
                (
                    C2Aabb { min: m, max: C2v { x: m.x + 40.0, y: m.y + 40.0 } },
                    C2Aabb {
                        min: C2v { x: m.x + 10.0, y: m.y + 10.0 },
                        max: C2v { x: m.x + 20.0, y: m.y + 20.0 },
                    },
                )
            }
            2 => (
                C2Aabb { min: rng.v_special(), max: rng.v_special() },
                C2Aabb { min: rng.v_special(), max: rng.v_special() },
            ),
            _ => (rng.aabb(), rng.aabb()),
        };
        let cv = (c.c2AABBtoAABB)(a, b);
        let rv = (r.c2AABBtoAABB)(a, b);
        d.int(&format!("B58#{i} A={a:?} B={b:?}"), cv, rv);
        collided += (cv != 0) as u32;
        // reversed argument order too
        d.int(
            &format!("B58#{i} rev A={b:?} B={a:?}"),
            (c.c2AABBtoAABB)(b, a),
            (r.c2AABBtoAABB)(b, a),
        );
    }
    d.finish();
    assert!(collided > 1000 && collided < 19000, "B58 collided={collided} (unbalanced)");
    eprintln!("B58 collided: {collided}/20000");
}

#[test]
fn B59_circle_to_circle() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B59 c2CircletoCircle");
    let mut rng = Rng::new(SEED ^ 141);
    let mut collided = 0u32;
    for i in 0..20000 {
        let (a, b) = match rng.below(6) {
            0 => {
                // exact touch: |d| == rA + rB with a Pythagorean triple
                let ra = (rng.below(5) + 1) as f32;
                let rb = (rng.below(5) + 1) as f32;
                let s = (ra + rb) / 5.0;
                let p = rng.v_coord();
                (
                    C2Circle { p, r: ra },
                    C2Circle { p: C2v { x: p.x + 3.0 * s, y: p.y + 4.0 * s }, r: rb },
                )
            }
            1 => {
                let p = rng.v_coord(); // concentric
                (C2Circle { p, r: rng.radius() }, C2Circle { p, r: rng.radius() })
            }
            2 => (
                C2Circle { p: rng.v_special(), r: rng.any() },
                C2Circle { p: rng.v_special(), r: rng.any() },
            ),
            _ => (rng.circle(), rng.circle()),
        };
        let cv = (c.c2CircletoCircle)(a, b);
        let rv = (r.c2CircletoCircle)(a, b);
        d.int(&format!("B59#{i} A={a:?} B={b:?}"), cv, rv);
        collided += (cv != 0) as u32;
        d.int(
            &format!("B59#{i} rev"),
            (c.c2CircletoCircle)(b, a),
            (r.c2CircletoCircle)(b, a),
        );
    }
    d.finish();
    eprintln!("B59 collided: {collided}/20000");
}

#[test]
fn B60_circle_to_aabb() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B60 c2CircletoAABB");
    let mut rng = Rng::new(SEED ^ 142);
    let mut collided = 0u32;
    for i in 0..20000 {
        let (a, b) = match rng.below(6) {
            0 => {
                // centre exactly on an edge / corner of the box
                let m = C2v { x: (rng.below(21) as f32) - 10.0, y: (rng.below(21) as f32) - 10.0 };
                let w = (rng.below(8) + 1) as f32;
                let bb = C2Aabb { min: m, max: C2v { x: m.x + w, y: m.y + w } };
                let p = match rng.below(4) {
                    0 => m,
                    1 => bb.max,
                    2 => C2v { x: m.x, y: m.y + w / 2.0 },
                    _ => C2v { x: m.x + w / 2.0, y: m.y + w },
                };
                (C2Circle { p, r: (rng.below(4)) as f32 }, bb)
            }
            1 => {
                // centre strictly inside
                let bb = rng.aabb();
                (
                    C2Circle {
                        p: C2v {
                            x: (bb.min.x + bb.max.x) * 0.5,
                            y: (bb.min.y + bb.max.y) * 0.5,
                        },
                        r: rng.radius(),
                    },
                    bb,
                )
            }
            2 => (
                C2Circle { p: rng.v_special(), r: rng.any() },
                C2Aabb { min: rng.v_special(), max: rng.v_special() },
            ),
            _ => (rng.circle(), rng.aabb()),
        };
        let cv = (c.c2CircletoAABB)(a, b);
        let rv = (r.c2CircletoAABB)(a, b);
        d.int(&format!("B60#{i} A={a:?} B={b:?}"), cv, rv);
        collided += (cv != 0) as u32;
    }
    d.finish();
    eprintln!("B60 collided: {collided}/20000");
}

/// Which distance branch `c2CircletoCapsule` takes (mirrors the C predicates).
fn circle_capsule_branch(a: &C2Circle, b: &C2Capsule) -> usize {
    let n = C2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
    let ap = C2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
    let da = ap.x * n.x + ap.y * n.y;
    if da < 0.0 {
        return 0;
    }
    let pb = C2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
    let db = pb.x * n.x + pb.y * n.y;
    if db < 0.0 { 1 } else { 2 }
}

#[test]
fn B61_circle_to_capsule() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B61 c2CircletoCapsule, all three distance branches");
    let mut rng = Rng::new(SEED ^ 143);
    let mut cover = [0u32; 3];
    let mut collided = 0u32;
    for i in 0..20000 {
        let (a, b) = match rng.below(6) {
            0 => {
                // axis-aligned capsule, circle placed before / inside / after
                let y = (rng.below(11) as f32) - 5.0;
                let cap = C2Capsule {
                    a: C2v { x: 0.0, y: 0.0 },
                    b: C2v { x: (rng.below(20) + 1) as f32, y: 0.0 },
                    r: (rng.below(6)) as f32,
                };
                let x = (rng.below(31) as f32) - 10.0;
                (C2Circle { p: C2v { x, y }, r: (rng.below(6)) as f32 }, cap)
            }
            1 => {
                // degenerate capsule (a == b)
                let p = rng.v_coord();
                (
                    rng.circle(),
                    C2Capsule { a: p, b: p, r: rng.radius() },
                )
            }
            2 => (
                C2Circle { p: rng.v_special(), r: rng.any() },
                C2Capsule { a: rng.v_special(), b: rng.v_special(), r: rng.any() },
            ),
            _ => (rng.circle(), rng.capsule()),
        };
        cover[circle_capsule_branch(&a, &b)] += 1;
        let cv = (c.c2CircletoCapsule)(a, b);
        let rv = (r.c2CircletoCapsule)(a, b);
        d.int(&format!("B61#{i} A={a:?} B={b:?}"), cv, rv);
        collided += (cv != 0) as u32;
    }
    d.finish();
    assert!(cover.iter().all(|&x| x >= 100), "B61 branch coverage {cover:?}");
    eprintln!("B61 branch coverage {cover:?}, collided {collided}/20000");
}

#[test]
fn B62_aabb_to_capsule() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B62 c2AABBtoCapsule");
    let mut rng = Rng::new(SEED ^ 144);
    let mut collided = 0u32;
    for i in 0..8000 {
        let (a, b) = match rng.below(5) {
            0 => {
                // capsule crossing a corner of the box
                let m = C2v { x: (rng.below(11) as f32) - 5.0, y: (rng.below(11) as f32) - 5.0 };
                let w = (rng.below(8) + 1) as f32;
                (
                    C2Aabb { min: m, max: C2v { x: m.x + w, y: m.y + w } },
                    C2Capsule {
                        a: C2v { x: m.x - 4.0, y: m.y - 4.0 },
                        b: C2v { x: m.x + w + 4.0, y: m.y + w + 4.0 },
                        r: (rng.below(5)) as f32,
                    },
                )
            }
            1 => {
                let p = rng.v_coord();
                (rng.aabb(), C2Capsule { a: p, b: p, r: rng.radius() })
            }
            2 => (
                C2Aabb { min: rng.v_special(), max: rng.v_special() },
                C2Capsule { a: rng.v_special(), b: rng.v_special(), r: rng.any() },
            ),
            _ => {
                let (x, y) = (rng.aabb(), rng.capsule());
                (x, y)
            }
        };
        let cv = (c.c2AABBtoCapsule)(a, b);
        let rv = (r.c2AABBtoCapsule)(a, b);
        d.int(&format!("B62#{i} A={a:?} B={b:?}"), cv, rv);
        collided += (cv != 0) as u32;
    }
    d.finish();
    eprintln!("B62 collided: {collided}/8000");
}

#[test]
fn B63_capsule_to_capsule() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B63 c2CapsuletoCapsule");
    let mut rng = Rng::new(SEED ^ 145);
    let mut collided = 0u32;
    for i in 0..8000 {
        let (a, b) = match rng.below(6) {
            0 => {
                // crossing at right angles, integer geometry
                let s = (rng.below(10) + 1) as f32;
                (
                    C2Capsule {
                        a: C2v { x: -s, y: 0.0 },
                        b: C2v { x: s, y: 0.0 },
                        r: (rng.below(4)) as f32,
                    },
                    C2Capsule {
                        a: C2v { x: 0.0, y: -s },
                        b: C2v { x: 0.0, y: s },
                        r: (rng.below(4)) as f32,
                    },
                )
            }
            1 => {
                // parallel, offset
                let cap = rng.capsule();
                let off = C2v { x: rng.f32_in(-20.0, 20.0), y: rng.f32_in(-20.0, 20.0) };
                (
                    cap,
                    C2Capsule {
                        a: C2v { x: cap.a.x + off.x, y: cap.a.y + off.y },
                        b: C2v { x: cap.b.x + off.x, y: cap.b.y + off.y },
                        r: rng.radius(),
                    },
                )
            }
            2 => {
                let cap = rng.capsule(); // coincident
                (cap, cap)
            }
            3 => (
                C2Capsule { a: rng.v_special(), b: rng.v_special(), r: rng.any() },
                C2Capsule { a: rng.v_special(), b: rng.v_special(), r: rng.any() },
            ),
            _ => (rng.capsule(), rng.capsule()),
        };
        let cv = (c.c2CapsuletoCapsule)(a, b);
        let rv = (r.c2CapsuletoCapsule)(a, b);
        d.int(&format!("B63#{i} A={a:?} B={b:?}"), cv, rv);
        collided += (cv != 0) as u32;
        d.int(
            &format!("B63#{i} rev"),
            (c.c2CapsuletoCapsule)(b, a),
            (r.c2CapsuletoCapsule)(b, a),
        );
    }
    d.finish();
    eprintln!("B63 collided: {collided}/8000");
}

#[test]
fn B64_predicates_specials_and_huge() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B64 all six predicates with huge / special values");
    let mut rng = Rng::new(SEED ^ 146);
    for i in 0..6000 {
        let pick = |rng: &mut Rng| -> f32 {
            match rng.below(3) {
                0 => rng.special(),
                1 => rng.huge(),
                _ => rng.tiny(),
            }
        };
        let v = |rng: &mut Rng| C2v { x: pick(rng), y: pick(rng) };
        let ci = C2Circle { p: v(&mut rng), r: pick(&mut rng) };
        let cj = C2Circle { p: v(&mut rng), r: pick(&mut rng) };
        let bi = C2Aabb { min: v(&mut rng), max: v(&mut rng) };
        let bj = C2Aabb { min: v(&mut rng), max: v(&mut rng) };
        let ki = C2Capsule { a: v(&mut rng), b: v(&mut rng), r: pick(&mut rng) };
        let kj = C2Capsule { a: v(&mut rng), b: v(&mut rng), r: pick(&mut rng) };
        let t = format!("B64#{i}");
        d.int(&format!("{t}/AABBtoAABB {bi:?} {bj:?}"), (c.c2AABBtoAABB)(bi, bj), (r.c2AABBtoAABB)(bi, bj));
        d.int(&format!("{t}/CircletoCircle {ci:?} {cj:?}"), (c.c2CircletoCircle)(ci, cj), (r.c2CircletoCircle)(ci, cj));
        d.int(&format!("{t}/CircletoAABB {ci:?} {bi:?}"), (c.c2CircletoAABB)(ci, bi), (r.c2CircletoAABB)(ci, bi));
        d.int(&format!("{t}/CircletoCapsule {ci:?} {ki:?}"), (c.c2CircletoCapsule)(ci, ki), (r.c2CircletoCapsule)(ci, ki));
        d.int(&format!("{t}/AABBtoCapsule {bi:?} {ki:?}"), (c.c2AABBtoCapsule)(bi, ki), (r.c2AABBtoCapsule)(bi, ki));
        d.int(&format!("{t}/CapsuletoCapsule {ki:?} {kj:?}"), (c.c2CapsuletoCapsule)(ki, kj), (r.c2CapsuletoCapsule)(ki, kj));
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Level 5 — c2Collided dispatcher
// ---------------------------------------------------------------------------

#[test]
fn B65_collided_all_type_pairs() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B65 c2Collided, all 9 type pairs");
    let mut rng = Rng::new(SEED ^ 150);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in RELS {
                for i in 0..120 {
                    let (a, b) = pair(&mut rng, tya, tyb, rel);
                    let cv = unsafe { (c.c2Collided)(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()) };
                    let rv = unsafe { (r.c2Collided)(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()) };
                    d.int(
                        &format!(
                            "B65/{}x{} {rel:?}#{i} A={a:?} B={b:?}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        ),
                        cv,
                        rv,
                    );
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B66_collided_degenerate_and_special() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B66 c2Collided, degenerate / special shapes");
    let mut rng = Rng::new(SEED ^ 151);
    for tya in 0..3 {
        for tyb in 0..3 {
            for rel in [Rel::Degenerate, Rel::Special, Rel::Huge] {
                for i in 0..200 {
                    let (a, b) = pair(&mut rng, tya, tyb, rel);
                    let cv = unsafe { (c.c2Collided)(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()) };
                    let rv = unsafe { (r.c2Collided)(a.as_ptr(), a.ty(), b.as_ptr(), b.ty()) };
                    d.int(
                        &format!(
                            "B66/{}x{} {rel:?}#{i} A={a:?} B={b:?}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        ),
                        cv,
                        rv,
                    );
                }
            }
        }
    }
    d.finish();
}

#[test]
fn B67_collided_aliased() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B67 c2Collided with aliased A/B pointer");
    let mut rng = Rng::new(SEED ^ 152);
    for ty in 0..3 {
        for i in 0..800 {
            let s = shape_of(&mut rng, ty);
            let p: *const c_void = s.as_ptr();
            let cv = unsafe { (c.c2Collided)(p, s.ty(), p, s.ty()) };
            let rv = unsafe { (r.c2Collided)(p, s.ty(), p, s.ty()) };
            d.int(&format!("B67/{}#{i} S={s:?}", TYPE_NAMES[ty]), cv, rv);
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Level 6 — the `capsule` entry point from include/lib.h
// ---------------------------------------------------------------------------

#[test]
fn B68_capsule_random() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B68 capsule(), random finite arguments");
    let mut rng = Rng::new(SEED ^ 160);
    for i in 0..20000 {
        let (a, b, e, f) = (rng.coord(), rng.coord(), rng.coord(), rng.coord());
        let g = rng.f32_in(0.0, 60.0);
        d.int(
            &format!("B68#{i} capsule({a},{b},{e},{f},{g})"),
            (c.capsule)(a, b, e, f, g),
            (r.capsule)(a, b, e, f, g),
        );
    }
    d.finish();
}

#[test]
fn B69_capsule_all_result_bit_patterns() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B69 capsule(), every reachable result value");
    let mut rng = Rng::new(SEED ^ 161);
    let mut seen = [0u32; 8];
    // The three reference shapes live in x in [-80,-5], y in [-45,105].
    for i in 0..60000 {
        let (a, b) = (rng.f32_in(-90.0, 10.0), rng.f32_in(-60.0, 120.0));
        let (e, f) = (rng.f32_in(-90.0, 10.0), rng.f32_in(-60.0, 120.0));
        let g = rng.f32_in(0.0, 45.0);
        let cv = (c.capsule)(a, b, e, f, g);
        let rv = (r.capsule)(a, b, e, f, g);
        d.int(&format!("B69#{i} capsule({a},{b},{e},{f},{g})"), cv, rv);
        if (0..8).contains(&cv) {
            seen[cv as usize] += 1;
        }
    }
    d.finish();
    let missing: Vec<usize> = (0..8).filter(|&k| seen[k] == 0).collect();
    assert!(
        missing.is_empty(),
        "capsule() result values never produced: {missing:?} (seen {seen:?})"
    );
    eprintln!("B69 result histogram: {seen:?}");
}

#[test]
fn B70_capsule_specials() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B70 capsule(), huge / denormal / negative-r / specials");
    let mut rng = Rng::new(SEED ^ 162);
    // exhaustive over the special list for the first two parameters
    for &a in SPECIALS {
        for &b in SPECIALS {
            for k in 0..6 {
                let (e, f, g) = match k {
                    0 => (0.0, 0.0, 0.0),
                    1 => (-30.0, -30.0, 10.0),
                    2 => (f32::NAN, 1.0, 5.0),
                    3 => (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY),
                    4 => (1e-40, -1e-40, -5.0),
                    _ => (f32::MAX, f32::MIN, f32::MAX),
                };
                d.int(
                    &format!("B70 capsule({a:?},{b:?},{e:?},{f:?},{g:?})"),
                    (c.capsule)(a, b, e, f, g),
                    (r.capsule)(a, b, e, f, g),
                );
            }
        }
    }
    for i in 0..8000 {
        let (a, b, e, f, g) = (rng.any(), rng.any(), rng.any(), rng.any(), rng.any());
        d.int(
            &format!("B70r#{i} capsule({a:?},{b:?},{e:?},{f:?},{g:?})"),
            (c.capsule)(a, b, e, f, g),
            (r.capsule)(a, b, e, f, g),
        );
    }
    d.finish();
}

#[test]
fn B71_capsule_grid_sweep() {
    let (c, r) = load_pair();
    let mut d = Diff::new("B71 capsule(), dense grid sweep over the reference region");
    // 21 x 21 x 21 x 21 grid over x/y in [-90, 10] x [-60, 120] would be huge;
    // sweep the two endpoints on a coarse grid with several radii instead.
    let xs: Vec<f32> = (0..15).map(|k| -90.0 + 100.0 * k as f32 / 14.0).collect();
    let ys: Vec<f32> = (0..14).map(|k| -60.0 + 180.0 * k as f32 / 13.0).collect();
    let rs = [0.0f32, 1.0, 10.0, 25.0];
    let mut n = 0u64;
    for &ax in &xs {
        for &ay in &ys {
            for &bx in &xs {
                for &by in &ys {
                    for &rr in &rs {
                        // keep the run bounded: sample a deterministic subset
                        n += 1;
                        if n % 7 != 0 {
                            continue;
                        }
                        d.int(
                            &format!("B71 capsule({ax},{ay},{bx},{by},{rr})"),
                            (c.capsule)(ax, ay, bx, by, rr),
                            (r.capsule)(ax, ay, bx, by, rr),
                        );
                    }
                }
            }
        }
    }
    d.finish();
}
