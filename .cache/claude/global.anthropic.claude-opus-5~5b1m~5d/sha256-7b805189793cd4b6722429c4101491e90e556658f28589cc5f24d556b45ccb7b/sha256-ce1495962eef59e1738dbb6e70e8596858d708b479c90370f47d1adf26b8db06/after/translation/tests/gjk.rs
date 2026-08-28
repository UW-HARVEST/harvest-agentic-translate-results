//! Phase B — `CONFIGS.md` rows 42..82: the composed `c2GJK` pipeline and the
//! `gjk()` wrapper from `include/lib.h`.
//!
//! `c2GJK` is where the per-function tests stop being sufficient: a wrong
//! operand order or a wrong branch inside `c22`/`c23`/`c2Witness` only shows up
//! once the full iteration is run end to end with a real proxy, a real
//! transform and a real cache. Every call therefore compares the return value,
//! `*outA`, `*outB`, `*iterations` AND all 36 bytes of the written-back cache.

mod common;
use common::*;
use std::os::raw::c_int;

const N: usize = 1_500;

// ---------------------------------------------------------------------------
// Shape generation
// ---------------------------------------------------------------------------

fn mk(rng: &mut Rng, ty: c_int, center: c2v, scale: f32) -> Shape {
    let j = |rng: &mut Rng| rng.unit() * scale;
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: c2v {
                x: center.x + j(rng),
                y: center.y + j(rng),
            },
            r: rng.radius().min(scale * 2.0),
        }),
        C2_TYPE_AABB => {
            let lo = c2v {
                x: center.x + j(rng),
                y: center.y + j(rng),
            };
            match rng.below(8) {
                0 => Shape::Aabb(c2AABB { min: lo, max: lo }), // degenerate
                1 => Shape::Aabb(c2AABB {
                    // inverted
                    min: lo,
                    max: c2v {
                        x: lo.x - scale.abs(),
                        y: lo.y - scale.abs(),
                    },
                }),
                _ => Shape::Aabb(c2AABB {
                    min: lo,
                    max: c2v {
                        x: lo.x + rng.unit().abs() * scale,
                        y: lo.y + rng.unit().abs() * scale,
                    },
                }),
            }
        }
        _ => {
            let a = c2v {
                x: center.x + j(rng),
                y: center.y + j(rng),
            };
            let b = if rng.below(8) == 0 {
                a // zero-length capsule
            } else {
                c2v {
                    x: center.x + j(rng),
                    y: center.y + j(rng),
                }
            };
            Shape::Capsule(c2Capsule {
                a,
                b,
                r: rng.radius().min(scale * 2.0),
            })
        }
    }
}

/// Two shapes in the same neighbourhood, so both the overlapping and the
/// separated regime occur often.
fn near_pair(rng: &mut Rng, ta: c_int, tb: c_int) -> (Shape, Shape) {
    let scale = *pick(&[0.25f32, 1.0, 2.0, 10.0, 100.0], rng);
    let sep = *pick(&[0.0f32, 0.5, 1.0, 2.0, 4.0], rng);
    let a = mk(rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
    let bcx = rng.unit() * scale * sep;
    let bcy = rng.unit() * scale * sep;
    let b = mk(rng, tb, c2v { x: bcx, y: bcy }, scale);
    (a, b)
}

/// Run one configuration against both libraries and compare everything.
#[track_caller]
fn diff(ctx: &str, a: &Shape, b: &Shape, o: &GjkOpts) -> GjkOut {
    let (ca, ra) = both();
    let cout = call_gjk(ca, a, b, o);
    let rout = call_gjk(ra, a, b, o);
    assert_gjk_eq(ctx, &cout, &rout);
    assert!(
        cout.iters <= 20,
        "iteration cap violated: {} > 20 [{ctx}]",
        cout.iters
    );
    cout
}

// ---------------------------------------------------------------------------
// Rows 42..50, 55, 60 — the full 3x3 type matrix
// ---------------------------------------------------------------------------

#[test]
fn cfg_gjk_type_matrix() {
    let mut zero = 0usize;
    let mut positive = 0usize;
    for (ai, &ta) in ALL_TYPES.iter().enumerate() {
        for (bi, &tb) in ALL_TYPES.iter().enumerate() {
            let mut rng = Rng::new(42 + (ai * 3 + bi) as u64);
            for i in 0..N {
                let (a, b) = near_pair(&mut rng, ta, tb);
                let o = GjkOpts::default(); // identity, use_radius=1, no cache
                let out = diff(
                    &format!("c2GJK types({ta},{tb}) #{i} A={a:?} B={b:?}"),
                    &a,
                    &b,
                    &o,
                );
                assert!(out.wrote_a && out.wrote_b && out.wrote_iters);
                if out.dist == 0.0 {
                    zero += 1
                } else {
                    positive += 1
                }
            }
        }
    }
    assert!(
        zero > 100 && positive > 100,
        "both regimes must be sampled: dist==0 -> {zero}, dist>0 -> {positive}"
    );
    println!("type matrix: dist==0 in {zero} runs, dist>0 in {positive} runs");
}

// ---------------------------------------------------------------------------
// Rows 51..53 — the transform axis crossed with the type matrix
// ---------------------------------------------------------------------------

#[test]
fn cfg_gjk_transform_matrix() {
    for (ai, &ta) in ALL_TYPES.iter().enumerate() {
        for (bi, &tb) in ALL_TYPES.iter().enumerate() {
            let mut rng = Rng::new(51 + (ai * 3 + bi) as u64);
            for i in 0..N {
                let (a, b) = near_pair(&mut rng, ta, tb);
                let xa = rng.xform();
                let xb = rng.xform();
                // Row 51: only bx set. Row 52: only ax. Row 53: both.
                for (k, o) in [
                    GjkOpts {
                        ax: None,
                        bx: Some(xb),
                        ..Default::default()
                    },
                    GjkOpts {
                        ax: Some(xa),
                        bx: None,
                        ..Default::default()
                    },
                    GjkOpts {
                        ax: Some(xa),
                        bx: Some(xb),
                        ..Default::default()
                    },
                    // An explicit identity must equal passing NULL.
                    GjkOpts {
                        ax: Some(c2x {
                            p: c2v { x: 0.0, y: 0.0 },
                            r: c2r { c: 1.0, s: 0.0 },
                        }),
                        bx: Some(c2x {
                            p: c2v { x: 0.0, y: 0.0 },
                            r: c2r { c: 1.0, s: 0.0 },
                        }),
                        ..Default::default()
                    },
                ]
                .iter()
                .enumerate()
                {
                    diff(
                        &format!("c2GJK xform({ta},{tb}) case{k} #{i}"),
                        &a,
                        &b,
                        o,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 54, 56 — use_radius
// ---------------------------------------------------------------------------

#[test]
fn cfg_gjk_use_radius_off() {
    for (ai, &ta) in ALL_TYPES.iter().enumerate() {
        for (bi, &tb) in ALL_TYPES.iter().enumerate() {
            let mut rng = Rng::new(54 + (ai * 3 + bi) as u64);
            for i in 0..N {
                let (a, b) = near_pair(&mut rng, ta, tb);
                diff(
                    &format!("c2GJK use_radius=0 ({ta},{tb}) #{i}"),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: 0,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

/// `use_radius` is tested with `!= 0`, so every non-zero int must behave like 1.
#[test]
fn cfg_gjk_use_radius_other() {
    let mut rng = Rng::new(56);
    for i in 0..N {
        let ta = *pick(&ALL_TYPES, &mut rng);
        let tb = *pick(&ALL_TYPES, &mut rng);
        let (a, b) = near_pair(&mut rng, ta, tb);
        let mut ref_out: Option<f32> = None;
        for ur in [1, 2, -1, i32::MIN, i32::MAX, 0x1_0000] {
            let out = diff(
                &format!("c2GJK use_radius={ur} #{i}"),
                &a,
                &b,
                &GjkOpts {
                    use_radius: ur,
                    ..Default::default()
                },
            );
            match ref_out {
                None => ref_out = Some(out.dist),
                Some(d) => assert_eq!(
                    d.to_bits(),
                    out.dist.to_bits(),
                    "use_radius={ur} must behave identically to 1 [#{i}]"
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 57..59 — the three terminal paths (hit / radius-shrink / midpoint)
// ---------------------------------------------------------------------------

/// Deeply overlapping shapes drive `s.count` to 3, which sets `hit` and skips
/// the `use_radius` block entirely.
#[test]
fn cfg_gjk_overlap_hit() {
    let mut rng = Rng::new(57);
    let mut hits = 0usize;
    for i in 0..N * 4 {
        // Two boxes/capsules centred on nearly the same point -> deep overlap.
        let ta = *pick(&ALL_TYPES, &mut rng);
        let tb = *pick(&ALL_TYPES, &mut rng);
        let c0 = c2v { x: 0.0, y: 0.0 };
        let a = mk(&mut rng, ta, c0, 4.0);
        let b = mk(&mut rng, tb, c0, 4.0);
        for ur in [0, 1] {
            let out = diff(
                &format!("c2GJK overlap ur={ur} ({ta},{tb}) #{i}"),
                &a,
                &b,
                &GjkOpts {
                    use_radius: ur,
                    ..Default::default()
                },
            );
            // In the `hit` path a == b exactly and dist is exactly +0.0.
            if out.dist.to_bits() == 0 && bytes_of(&out.a) == bytes_of(&out.b) {
                hits += 1;
            }
        }
    }
    assert!(hits > 200, "the hit / dist==0 path was barely reached ({hits})");
    println!("overlap: {hits} runs ended with dist=+0.0 and a==b");
}

/// Separated shapes with radii: `dist > rA + rB` shrinks by the radii, and
/// `dist <= rA + rB` collapses to the midpoint with `dist = 0`.
#[test]
fn cfg_gjk_radius_shrink() {
    let mut rng = Rng::new(58);
    let mut shrunk = 0usize;
    let mut midpoint = 0usize;
    for i in 0..N * 4 {
        // Circle/capsule pairs with a controlled gap and controlled radii, so
        // both sides of `dist > rA + rB` are sampled.
        let ra = rng.radius().min(3.0);
        let rb = rng.radius().min(3.0);
        let gap = *pick(&[0.0f32, 0.25, 1.0, 3.0, 8.0, 40.0], &mut rng);
        let a = Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: ra,
        });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: gap, y: 0.0 },
            r: rb,
        });
        let out = diff(&format!("c2GJK radius circles #{i}"), &a, &b, &GjkOpts::default());
        if out.dist > 0.0 {
            shrunk += 1
        } else {
            midpoint += 1
        }

        // Capsule vs AABB at a controlled separation.
        let cap = Shape::Capsule(c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: ra,
        });
        let bb = Shape::Aabb(c2AABB {
            min: c2v { x: gap + 1.0, y: -1.0 },
            max: c2v { x: gap + 3.0, y: 1.0 },
        });
        let out2 = diff(&format!("c2GJK radius cap/bb #{i}"), &cap, &bb, &GjkOpts::default());
        if out2.dist > 0.0 {
            shrunk += 1
        } else {
            midpoint += 1
        }
        let _ = rb;
    }
    assert!(
        shrunk > 100 && midpoint > 100,
        "both radius arms needed: shrunk={shrunk} midpoint={midpoint}"
    );
    println!("radius arms: shrink={shrunk} midpoint={midpoint}");
}

// ---------------------------------------------------------------------------
// Rows 61..67 — the cache axis
// ---------------------------------------------------------------------------

/// Row 61/62/46 — cold cache, then feed the produced cache back in three
/// generations while the shapes move, which is exactly how a real consumer
/// warm-starts the solver.
#[test]
fn cfg_gjk_cache_roundtrip() {
    let (ca, ra) = both();
    for (ai, &ta) in ALL_TYPES.iter().enumerate() {
        for (bi, &tb) in ALL_TYPES.iter().enumerate() {
            let mut rng = Rng::new(61 + (ai * 3 + bi) as u64);
            for i in 0..N {
                // Generation 0: a cold cache (count == 0).
                let mut c_cache = c2GJKCache::default();
                let mut r_cache = c2GJKCache::default();
                let (mut a, mut b) = near_pair(&mut rng, ta, tb);

                for generation in 0..4 {
                    let co = GjkOpts {
                        cache: Some(c_cache),
                        ..Default::default()
                    };
                    let ro = GjkOpts {
                        cache: Some(r_cache),
                        ..Default::default()
                    };
                    let cout = call_gjk(ca, &a, &b, &co);
                    let rout = call_gjk(ra, &a, &b, &ro);
                    assert_gjk_eq(
                        &format!("c2GJK cache gen{generation} ({ta},{tb}) #{i}"),
                        &cout,
                        &rout,
                    );
                    // The C only ever writes counts 1..3 back.
                    assert!(
                        (1..=3).contains(&cout.cache.count),
                        "written-back cache count {} out of range",
                        cout.cache.count
                    );
                    c_cache = cout.cache;
                    r_cache = rout.cache;

                    // Nudge the shapes so the next generation re-uses the cache
                    // against slightly different geometry.
                    let d = c2v {
                        x: rng.unit() * 0.5,
                        y: rng.unit() * 0.5,
                    };
                    a = translate(&a, d);
                    b = translate(&b, d);
                }
            }
        }
    }
}

fn translate(s: &Shape, d: c2v) -> Shape {
    let t = |p: c2v| c2v {
        x: p.x + d.x,
        y: p.y + d.y,
    };
    match *s {
        Shape::Circle(c) => Shape::Circle(c2Circle { p: t(c.p), r: c.r }),
        Shape::Aabb(c) => Shape::Aabb(c2AABB {
            min: t(c.min),
            max: t(c.max),
        }),
        Shape::Capsule(c) => Shape::Capsule(c2Capsule {
            a: t(c.a),
            b: t(c.b),
            r: c.r,
        }),
    }
}

/// Rows 63..65 — hand-built warm caches with `count` 1, 2 and 3 and only
/// **in-range** vertex indices (`< c2MakeProxy`'s written vertex count for that
/// shape, so no uninitialised proxy slot is ever read — see `ERRORS.md` U2).
fn warm_cache_test(stream: u64, count: c_int) {
    let mut rng = Rng::new(stream);
    for (ai, &ta) in ALL_TYPES.iter().enumerate() {
        for (bi, &tb) in ALL_TYPES.iter().enumerate() {
            let _ = (ai, bi);
            for i in 0..N {
                let (a, b) = near_pair(&mut rng, ta, tb);
                let na = a.vert_count();
                let nb = b.vert_count();
                let mut cache = c2GJKCache {
                    metric: *pick(
                        &[0.0f32, 1.0, -1.0, -1.0e8, -1.0e9, 5.0, FLT_MAX],
                        &mut rng,
                    ),
                    count,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: *pick(&[1.0f32, 2.0, 0.5, 3.0], &mut rng),
                };
                for k in 0..count as usize {
                    cache.iA[k] = (rng.below(na as u32)) as c_int;
                    cache.iB[k] = (rng.below(nb as u32)) as c_int;
                }
                diff(
                    &format!("c2GJK warm cache count={count} ({ta},{tb}) #{i} {cache:?}"),
                    &a,
                    &b,
                    &GjkOpts {
                        cache: Some(cache),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

#[test]
fn cfg_gjk_cache_warm_1() {
    warm_cache_test(63, 1);
}

#[test]
fn cfg_gjk_cache_warm_2() {
    warm_cache_test(64, 2);
}

#[test]
fn cfg_gjk_cache_warm_3() {
    warm_cache_test(65, 3);
}

/// Row 66 — sweep `cache->metric` across the `-1.0e8f` threshold in lib.c:400,
/// including both infinities and NaN, which decides `cache_was_read`.
#[test]
fn cfg_gjk_cache_metric_sweep() {
    let mut rng = Rng::new(66);
    let metrics = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        -9.999_999e7,
        -1.0e8,
        -1.000_000_1e8,
        -1.0e9,
        -FLT_MAX,
        FLT_MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_1234),
        FLT_EPSILON,
    ];
    for i in 0..N {
        let ta = *pick(&ALL_TYPES, &mut rng);
        let tb = *pick(&ALL_TYPES, &mut rng);
        let (a, b) = near_pair(&mut rng, ta, tb);
        let na = a.vert_count();
        let nb = b.vert_count();
        for count in [1i32, 2, 3] {
            for &metric in &metrics {
                let mut cache = c2GJKCache {
                    metric,
                    count,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 1.0,
                };
                for k in 0..count as usize {
                    cache.iA[k] = (rng.below(na as u32)) as c_int;
                    cache.iB[k] = (rng.below(nb as u32)) as c_int;
                }
                diff(
                    &format!("c2GJK metric={metric:?} count={count} #{i}"),
                    &a,
                    &b,
                    &GjkOpts {
                        cache: Some(cache),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

/// Row 67 — `cache->div` feeds straight into `s.div`, hence into
/// `1.0f / s->div` in `c2L`/`c2Witness`.
#[test]
fn cfg_gjk_cache_div_sweep() {
    let mut rng = Rng::new(67);
    let divs = [
        1.0f32,
        0.0,
        -0.0,
        2.0,
        -3.5,
        f32::from_bits(1),
        FLT_MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xff80_0001),
    ];
    for i in 0..N {
        let ta = *pick(&ALL_TYPES, &mut rng);
        let tb = *pick(&ALL_TYPES, &mut rng);
        let (a, b) = near_pair(&mut rng, ta, tb);
        let na = a.vert_count();
        let nb = b.vert_count();
        for count in [1i32, 2, 3] {
            for &div in &divs {
                let mut cache = c2GJKCache {
                    metric: -1.0e9,
                    count,
                    iA: [0; 3],
                    iB: [0; 3],
                    div,
                };
                for k in 0..count as usize {
                    cache.iA[k] = (rng.below(na as u32)) as c_int;
                    cache.iB[k] = (rng.below(nb as u32)) as c_int;
                }
                diff(
                    &format!("c2GJK div={div:?} count={count} #{i}"),
                    &a,
                    &b,
                    &GjkOpts {
                        cache: Some(cache),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 68..72 — degenerate and extreme geometry
// ---------------------------------------------------------------------------

#[test]
fn cfg_gjk_degenerate_shapes() {
    let mut rng = Rng::new(68);
    for i in 0..N {
        let p = rng.v();
        let q = rng.v();
        let degen: Vec<Shape> = vec![
            Shape::Aabb(c2AABB { min: p, max: p }),                    // row 68
            Shape::Aabb(c2AABB { min: q, max: p }),                    // inverted
            Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 }),          // row 69
            Shape::Capsule(c2Capsule { a: p, b: p, r: FLT_MAX }),      // huge r
            Shape::Capsule(c2Capsule { a: p, b: q, r: 0.0 }),
            Shape::Circle(c2Circle { p, r: 0.0 }),                     // row 70
            Shape::Circle(c2Circle { p, r: FLT_MAX }),
            Shape::Circle(c2Circle { p, r: f32::from_bits(1) }),
        ];
        for (j, a) in degen.iter().enumerate() {
            for (k, b) in degen.iter().enumerate() {
                for ur in [0, 1] {
                    diff(
                        &format!("c2GJK degenerate #{i} {j}x{k} ur={ur}"),
                        a,
                        b,
                        &GjkOpts {
                            use_radius: ur,
                            ..Default::default()
                        },
                    );
                }
            }
        }
        // Row 71 — coincident: A and B are literally the same geometry.
        for (j, a) in degen.iter().enumerate() {
            diff(
                &format!("c2GJK coincident #{i} {j}"),
                a,
                a,
                &GjkOpts::default(),
            );
        }
    }
}

/// Row 72 — magnitudes where `c2Dot` overflows to infinity and where every
/// coordinate is subnormal.
#[test]
fn cfg_gjk_extreme_scale() {
    let mut rng = Rng::new(72);
    let scales = [
        f32::from_bits(1),
        FLT_EPSILON,
        1.0,
        1.0e18,
        FLT_MAX,
        FLT_MAX * 0.5,
    ];
    for i in 0..N {
        for &s in &scales {
            let ta = *pick(&ALL_TYPES, &mut rng);
            let tb = *pick(&ALL_TYPES, &mut rng);
            let a = mk(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, s);
            let bx = s * rng.unit();
            let by = s * rng.unit();
            let b = mk(&mut rng, tb, c2v { x: bx, y: by }, s);
            for ur in [0, 1] {
                diff(
                    &format!("c2GJK scale={s:e} #{i} ur={ur}"),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 74..82 — the `gjk()` wrapper declared in include/lib.h
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn diff_wrapper(ctx: &str, rev: i8, f: [f32; 9]) {
    let (c, r) = both();
    let sentinel = c2v {
        x: -1.234_567_8e-11,
        y: 9.876_543e-13,
    };
    let mut ca = sentinel;
    let mut cb = sentinel;
    let mut ra = sentinel;
    let mut rb = sentinel;
    unsafe {
        (c.gjk)(
            rev, &mut ca, &mut cb, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8],
        );
        (r.gjk)(
            rev, &mut ra, &mut rb, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8],
        );
    }
    assert_bits_eq(&format!("{ctx} / a"), &ca, &ra);
    assert_bits_eq(&format!("{ctx} / b"), &cb, &rb);
}

/// Rows 74..77 — `reverse` swaps which shape is A, and only the low byte of the
/// `char` is examined (`cmpb $0x0,-0x34(%rbp)`).
#[test]
fn cfg_gjk_wrapper_reverse() {
    let mut rng = Rng::new(75);
    for i in 0..N * 4 {
        let a1 = rng.coord();
        let a2 = rng.coord();
        let f = [
            a1,
            a2,
            a1 + rng.radius(),
            a2 + rng.radius(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        for rev in [0i8, 1, -1, 2, 0x7f, -128, 42] {
            diff_wrapper(&format!("gjk reverse={rev} #{i} {f:?}"), rev, f);
        }
    }
}

/// Rows 78..80 — the wrapper's own geometric regimes.
#[test]
fn cfg_gjk_wrapper_regions() {
    let mut rng = Rng::new(78);
    for i in 0..N * 2 {
        let cases: [[f32; 9]; 6] = [
            // overlapping: capsule through the middle of the box
            [-2.0, -2.0, 2.0, 2.0, -1.0, 0.0, 1.0, 0.0, 0.5],
            // touching exactly at the box edge
            [0.0, 0.0, 1.0, 1.0, 2.0, 0.5, 3.0, 0.5, 1.0],
            // disjoint, increasing separation
            [
                0.0,
                0.0,
                1.0,
                1.0,
                4.0 + rng.unit().abs() * 20.0,
                0.5,
                5.0,
                0.5,
                0.25,
            ],
            // zero-area AABB
            [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 2.0, 2.0, 0.5],
            // zero-length capsule, zero radius
            [-1.0, -1.0, 1.0, 1.0, 3.0, 3.0, 3.0, 3.0, 0.0],
            // inverted AABB (min > max)
            [2.0, 2.0, -2.0, -2.0, 0.0, 0.0, 1.0, 1.0, 0.5],
        ];
        for (k, f) in cases.iter().enumerate() {
            for rev in [0i8, 1] {
                diff_wrapper(&format!("gjk region{k} rev={rev} #{i}"), rev, *f);
            }
        }
    }
}

/// Row 81 — `b5` is the 9th float, so it is passed on the **stack** rather than
/// in an XMM register. A miscounted argument slot would show up only here.
#[test]
fn cfg_gjk_wrapper_stack_arg() {
    let mut rng = Rng::new(81);
    let b5s = [
        0.0f32,
        -0.0,
        1.0,
        -1.0, // a negative radius: the C never validates it
        FLT_MAX,
        f32::from_bits(1),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        FLT_EPSILON,
        1.0e8,
    ];
    for i in 0..N * 2 {
        let base = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
        ];
        for &b5 in &b5s {
            let f = [
                base[0], base[1], base[2], base[3], base[4], base[5], base[6], base[7], b5,
            ];
            for rev in [0i8, 1] {
                diff_wrapper(&format!("gjk b5={b5:?} rev={rev} #{i}"), rev, f);
            }
        }
    }
}

/// Row 82 — completely unconstrained float bit patterns for all nine floats.
#[test]
fn cfg_gjk_wrapper_fuzz() {
    let mut rng = Rng::new(82);
    for i in 0..N * 8 {
        let f = [
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
            rng.any_f32(),
        ];
        let rev = (rng.next_u32() & 0xff) as i8;
        diff_wrapper(&format!("gjk fuzz #{i} rev={rev} {f:?}"), rev, f);
    }
}
