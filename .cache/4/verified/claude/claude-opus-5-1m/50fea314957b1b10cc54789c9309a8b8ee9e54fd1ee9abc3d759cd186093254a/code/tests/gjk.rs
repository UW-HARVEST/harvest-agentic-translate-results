//! Phase B — CONFIGS.md rows 28..50: `c2GJK`, the full composed pipeline.
//!
//! Driven the way a real consumer drives it: set up shapes + transforms, choose
//! the options, run the whole search, and (for the cache rows) run *sequences*
//! of calls where each library carries its OWN cache forward, so a divergence
//! compounds instead of being masked by re-seeding from the C side.

mod common;
use common::*;
use std::ffi::c_int;

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

fn type_name(t: c_int) -> &'static str {
    match t {
        C2_TYPE_CIRCLE => "circle",
        C2_TYPE_AABB => "aabb",
        C2_TYPE_CAPSULE => "capsule",
        _ => "invalid",
    }
}

/// Builds a shape of `ty` centred near `centre` with radius-ish extent `ext`.
fn shape_at(rng: &mut Rng, ty: c_int, centre: c2v, ext: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: c2v { x: centre.x + rng.scaled(ext * 0.2), y: centre.y + rng.scaled(ext * 0.2) },
            r: ext * (0.2 + rng.unit().abs() * 0.8),
        }),
        C2_TYPE_AABB => {
            let hx = ext * (0.2 + rng.unit().abs() * 0.8);
            let hy = ext * (0.2 + rng.unit().abs() * 0.8);
            Shape::Aabb(c2AABB {
                min: c2v { x: centre.x - hx, y: centre.y - hy },
                max: c2v { x: centre.x + hx, y: centre.y + hy },
            })
        }
        _ => Shape::Capsule(c2Capsule {
            a: c2v { x: centre.x + rng.scaled(ext), y: centre.y + rng.scaled(ext) },
            b: c2v { x: centre.x + rng.scaled(ext), y: centre.y + rng.scaled(ext) },
            r: ext * (0.1 + rng.unit().abs() * 0.5),
        }),
    }
}

/// Same as `shape_at` but draws the centre offset from `rng` first (writing
/// `shape_off(&mut rng, t, e, s)` inline would borrow `rng` twice).
fn shape_off(rng: &mut Rng, ty: c_int, off_scale: f32, ext: f32) -> Shape {
    let c = rng.vec_scaled(off_scale);
    shape_at(rng, ty, c, ext)
}

/// Which final branch `c2GJK` took, inferred from its observable outputs.
/// 0 = `hit` (simplex reached count 3), 1 = radius-shrink, 2 = midpoint.
fn classify_branch(out: &GjkOut, cache_valid: bool) -> usize {
    if cache_valid && out.cache.count == 3 {
        0
    } else if out.dist != 0.0 {
        1
    } else {
        2
    }
}

// ---------------------------------------------------------------------------
// Rows 28/29: the 3x3 shape-type cross product, use_radius = 1 and 0
// ---------------------------------------------------------------------------

fn type_cross_product(use_radius: c_int, seed: u64, tag: &str) {
    let p = pair();
    let mut rng = Rng::new(seed);
    let mut branches = [0usize; 3];
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..512 {
                let scale = rng.scale_choice();
                // separation sweep: 0 (interpenetrating) .. 10x (far apart)
                let sep = [0.0f32, 0.3, 0.8, 1.0, 1.5, 3.0, 10.0][rng.below(7) as usize];
                let ang = rng.unit() * std::f32::consts::PI;
                let centre_b =
                    c2v { x: sep * scale * ang.cos(), y: sep * scale * ang.sin() };
                let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let b = shape_at(&mut rng, tb, centre_b, scale);
                let mut inp = GjkIn::new(&a, &b);
                inp.use_radius = use_radius;
                inp.cache = Some(c2GJKCache::default()); // count == 0 -> cold
                let ctx = format!(
                    "{tag} {}x{} sep={sep} scale={scale}[{i}]",
                    type_name(ta),
                    type_name(tb)
                );
                let out = diff_gjk(&ctx, p, &inp);
                branches[classify_branch(&out, true)] += 1;
                assert!(out.iters >= 0 && out.iters <= 20, "{ctx}: iterations {} out of range", out.iters);
            }
        }
    }
    eprintln!("{tag} branch coverage (hit, shrink, midpoint) = {branches:?}");
    assert!(branches[0] > 0, "{tag}: never reached the `hit` (count==3) path");
    if use_radius != 0 {
        // With use_radius != 0 both post-processing arms must be reachable.
        assert!(branches[1] > 0, "{tag}: never reached the radius-shrink path");
        assert!(branches[2] > 0, "{tag}: never reached the midpoint path");
    } else {
        // With use_radius == 0 the entire `else if (use_radius)` block is
        // skipped, so the midpoint arm is unreachable by construction.
        assert!(branches[1] > 0, "{tag}: never produced a non-zero distance");
    }
}

#[test]
fn row28_type_cross_product_use_radius() {
    type_cross_product(1, 0x2828, "row28");
}

#[test]
fn row29_type_cross_product_no_radius() {
    type_cross_product(0, 0x2929, "row29");
}

// ---------------------------------------------------------------------------
// Rows 30..33: transform combinations
// ---------------------------------------------------------------------------

fn transform_case(seed: u64, tag: &str, ax_on: bool, bx_on: bool, unnorm: bool) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..256 {
                let scale = rng.scale_choice();
                let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let b = shape_off(&mut rng, tb, scale * 2.0, scale);
                let mk = |rng: &mut Rng| {
                    if unnorm {
                        rand_transform_unnorm(rng, scale)
                    } else {
                        rand_transform(rng, scale)
                    }
                };
                let ax = if ax_on { Some(mk(&mut rng)) } else { None };
                let bx = if bx_on { Some(mk(&mut rng)) } else { None };
                let mut inp = GjkIn::new(&a, &b);
                inp.ax = ax;
                inp.bx = bx;
                inp.use_radius = if rng.below(2) == 0 { 1 } else { 0 };
                inp.cache = Some(c2GJKCache::default());
                diff_gjk(
                    &format!("{tag} {}x{}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &inp,
                );
            }
        }
    }
}

#[test]
fn row30_ax_null_bx_set() {
    transform_case(0x3030, "row30", false, true, false);
}

#[test]
fn row31_ax_set_bx_null() {
    transform_case(0x3131, "row31", true, false, false);
}

#[test]
fn row32_both_transforms_normalised() {
    transform_case(0x3232, "row32", true, true, false);
}

#[test]
fn row33_both_transforms_unnormalised() {
    transform_case(0x3333, "row33", true, true, true);
}

// ---------------------------------------------------------------------------
// Row 34: cache != NULL with count == 0 (cold start), full write-back checked
// ---------------------------------------------------------------------------

#[test]
fn row34_cache_cold_writeback() {
    let p = pair();
    let mut rng = Rng::new(0x3434);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..512 {
                let scale = rng.scale_choice();
                let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let b = shape_off(&mut rng, tb, scale * 2.0, scale);
                // A cache whose `count` is 0 but whose other fields are junk:
                // the C must ignore the junk and overwrite all of it.
                let mut cache = c2GJKCache {
                    metric: rng.scaled(1e6),
                    count: 0,
                    iA: [rng.below(1000) as c_int; 3],
                    iB: [rng.below(1000) as c_int; 3],
                    div: rng.scaled(1e6),
                };
                cache.count = 0;
                let mut inp = GjkIn::new(&a, &b);
                inp.cache = Some(cache);
                let out = diff_gjk(
                    &format!("row34 {}x{}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &inp,
                );
                // write-back must have happened
                assert!(
                    out.cache.count >= 1 && out.cache.count <= 3,
                    "row34: cache.count = {} after write-back",
                    out.cache.count
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 35..38: warm-cache sequences, each side carrying its OWN cache
// ---------------------------------------------------------------------------

/// Runs `steps` successive `c2GJK` calls, keeping a separate cache per library
/// and comparing every output plus both caches after each step.
fn cached_chain(
    ctx: &str,
    p: &Pair,
    shapes: &[(Shape, Shape)],
    ax: Option<c2x>,
    bx: Option<c2x>,
    use_radius: c_int,
) {
    let mut cc = c2GJKCache::default(); // count == 0
    let mut cr = c2GJKCache::default();
    for (step, (a, b)) in shapes.iter().enumerate() {
        let (mut ac, mut bc) = (c2v::default(), c2v::default());
        let (mut ar, mut br) = (c2v::default(), c2v::default());
        let mut ic: c_int = -1;
        let mut ir: c_int = -1;
        let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let dc = unsafe {
            (p.c.c2GJK)(a.as_ptr(), a.ty(), axp, b.as_ptr(), b.ty(), bxp, &mut ac, &mut bc,
                        use_radius, &mut ic, &mut cc)
        };
        let dr = unsafe {
            (p.r.c2GJK)(a.as_ptr(), a.ty(), axp, b.as_ptr(), b.ty(), bxp, &mut ar, &mut br,
                        use_radius, &mut ir, &mut cr)
        };
        let c = format!("{ctx} step={step}");
        eq_f32(&format!("{c}: dist"), dc, dr);
        eq_v(&format!("{c}: outA"), ac, ar);
        eq_v(&format!("{c}: outB"), bc, br);
        eq_i(&format!("{c}: iterations"), ic, ir);
        eq_cache(&format!("{c}: cache"), &cc, &cr);
    }
}

#[test]
fn row35_warm_cache_same_shapes() {
    // The `gjk_cache` scenario: two identical calls sharing one cache.
    let p = pair();
    let mut rng = Rng::new(0x3535);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..256 {
                let scale = rng.scale_choice();
                let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let b = shape_off(&mut rng, tb, scale * 2.0, scale);
                let seq = vec![(a, b), (a, b), (a, b)];
                cached_chain(
                    &format!("row35 {}x{}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &seq,
                    None,
                    None,
                    1,
                );
            }
        }
    }
}

#[test]
fn row36_warm_cache_moved_shapes() {
    // Stale cache: the shapes move between the two calls, but per ERRORS.md
    // row 32 the C accepts the stale cache anyway.
    let p = pair();
    let mut rng = Rng::new(0x3636);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..256 {
                let scale = rng.scale_choice();
                let a1 = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let b1 = shape_off(&mut rng, tb, scale * 2.0, scale);
                let a2 = shape_off(&mut rng, ta, scale, scale);
                let b2 = shape_off(&mut rng, tb, scale * 3.0, scale);
                let seq = vec![(a1, b1), (a2, b2), (a1, b2), (a2, b1)];
                cached_chain(
                    &format!("row36 {}x{}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &seq,
                    None,
                    None,
                    1,
                );
            }
        }
    }
}

#[test]
fn row37_warm_cache_across_type_change() {
    // A cache is reused across a shape-TYPE change. To keep this a valid-path
    // row, the vertex count must never DECREASE along the sequence: a cache
    // written by an AABB holds indices up to 3, which are out of range for a
    // 2-vertex capsule and would make the C read an uninitialised `c2Proxy`
    // slot (ERRORS.md row 37 - undefined, covered in the error suite instead).
    // circle(1 vert) -> capsule(2) -> aabb(4) keeps every cached index valid.
    let p = pair();
    let mut rng = Rng::new(0x3737);
    for i in 0..2048 {
        let scale = rng.scale_choice();
        let circle = shape_at(&mut rng, C2_TYPE_CIRCLE, c2v { x: 0.0, y: 0.0 }, scale);
        let caps = shape_at(&mut rng, C2_TYPE_CAPSULE, c2v { x: scale, y: 0.0 }, scale);
        let bb = shape_at(&mut rng, C2_TYPE_AABB, c2v { x: scale * 2.0, y: 0.0 }, scale);
        let a = shape_at(&mut rng, C2_TYPE_CIRCLE, c2v { x: -scale, y: 0.0 }, scale);

        // B grows in vertex count; A is a circle throughout (index always 0).
        let seq = vec![(a, circle), (a, caps), (a, bb)];
        cached_chain(&format!("row37 growB[{i}]"), p, &seq, None, None, 1);

        // And the mirror image: A grows in vertex count, B is a fixed circle.
        let seq2 = vec![(circle, a), (caps, a), (bb, a)];
        cached_chain(&format!("row37 growA[{i}]"), p, &seq2, None, None, 1);

        // Same type family repeated is always index-safe too.
        let seq3 = vec![(bb, bb), (bb, bb), (bb, bb)];
        cached_chain(&format!("row37 sameType[{i}]"), p, &seq3, None, None, 1);
    }
}

#[test]
fn row38_long_cached_chain_drifting_shapes() {
    let p = pair();
    let mut rng = Rng::new(0x3838);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..128 {
                let scale = rng.scale_choice();
                // 8 steps, B drifting from deep overlap out to well separated:
                // exercises every count transition 1<->2<->3 with a live cache.
                let mut seq = Vec::new();
                for step in 0..8 {
                    let sep = step as f32 * 0.5;
                    let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                    let jitter = rng.scaled(scale * 0.25);
                    let b = shape_at(
                        &mut rng,
                        tb,
                        c2v { x: sep * scale, y: jitter },
                        scale,
                    );
                    seq.push((a, b));
                }
                let ur = if rng.below(2) == 0 { 1 } else { 0 };
                cached_chain(
                    &format!("row38 {}x{} ur={ur}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &seq,
                    None,
                    None,
                    ur,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 39: all 8 out-parameter present/NULL combinations
// ---------------------------------------------------------------------------

#[test]
fn row39_out_param_matrix() {
    let p = pair();
    let mut rng = Rng::new(0x3939);
    for mask in 0..8u32 {
        for i in 0..512 {
            let scale = rng.scale_choice();
            let ta = TYPES[rng.below(3) as usize];
            let tb = TYPES[rng.below(3) as usize];
            let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
            let b = shape_off(&mut rng, tb, scale * 2.0, scale);
            let mut inp = GjkIn::new(&a, &b);
            inp.want_out_a = mask & 1 != 0;
            inp.want_out_b = mask & 2 != 0;
            inp.want_iters = mask & 4 != 0;
            // cover cache = NULL as well (ERRORS.md row 26)
            inp.cache = if rng.below(2) == 0 { Some(c2GJKCache::default()) } else { None };
            diff_gjk(&format!("row39 mask={mask}[{i}]"), p, &inp);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 40..44: explicit geometry relations
// ---------------------------------------------------------------------------

#[test]
fn row40_separated() {
    let p = pair();
    let mut rng = Rng::new(0x4040);
    let mut saw_shrink = 0;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..256 {
                let scale = rng.scale_choice();
                let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                // push B far enough that dist > rA + rB is guaranteed
                let b = shape_at(&mut rng, tb, c2v { x: scale * 50.0, y: 0.0 }, scale);
                let mut inp = GjkIn::new(&a, &b);
                inp.cache = Some(c2GJKCache::default());
                let out = diff_gjk(
                    &format!("row40 {}x{}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &inp,
                );
                if out.dist > 0.0 {
                    saw_shrink += 1;
                }
            }
        }
    }
    assert!(saw_shrink > 0, "row40 never produced a positive distance");
}

#[test]
fn row41_touching_boundary() {
    // dist == rA + rB exactly: the boundary of `dist > rA + rB`.
    let p = pair();
    let mut rng = Rng::new(0x4141);
    for i in 0..4096 {
        let ra = [0.5f32, 1.0, 2.0, 10.0, 0.0][rng.below(5) as usize];
        let rb = [0.5f32, 1.0, 3.0, 7.0, 0.0][rng.below(5) as usize];
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
        // centre distance exactly ra+rb -> core distance == ra+rb
        let b = Shape::Circle(c2Circle { p: c2v { x: ra + rb, y: 0.0 }, r: rb });
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = Some(c2GJKCache::default());
        diff_gjk(&format!("row41 exact ra={ra} rb={rb}[{i}]"), p, &inp);

        // and one ULP either side of the boundary
        for delta in [-2i32, -1, 0, 1, 2] {
            let d = ra + rb;
            let bits = d.to_bits() as i64 + delta as i64;
            let d2 = f32::from_bits(bits as u32);
            let b2 = Shape::Circle(c2Circle { p: c2v { x: d2, y: 0.0 }, r: rb });
            let mut inp2 = GjkIn::new(&a, &b2);
            inp2.cache = Some(c2GJKCache::default());
            diff_gjk(&format!("row41 ulp{delta} ra={ra} rb={rb}[{i}]"), p, &inp2);
        }
    }
}

#[test]
fn row42_overlapping_radii_cores_disjoint() {
    let p = pair();
    let mut rng = Rng::new(0x4242);
    let mut saw_midpoint = 0;
    for i in 0..4096 {
        let scale = rng.scale_choice();
        // two capsules whose segments miss each other but whose radii overlap
        let r = scale * 2.0;
        let a = Shape::Capsule(c2Capsule {
            a: c2v { x: -scale, y: 0.0 },
            b: c2v { x: scale, y: 0.0 },
            r,
        });
        let b = Shape::Capsule(c2Capsule {
            a: c2v { x: -scale, y: scale * 0.5 },
            b: c2v { x: scale, y: scale * 0.5 },
            r,
        });
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = Some(c2GJKCache::default());
        let out = diff_gjk(&format!("row42[{i}]"), p, &inp);
        if out.dist == 0.0 {
            saw_midpoint += 1;
        }
        // also a randomized variant
        let a2 = shape_at(&mut rng, C2_TYPE_CIRCLE, c2v { x: 0.0, y: 0.0 }, scale);
        let b2 = shape_at(&mut rng, C2_TYPE_CIRCLE, c2v { x: scale * 0.8, y: 0.0 }, scale);
        let mut inp2 = GjkIn::new(&a2, &b2);
        inp2.cache = Some(c2GJKCache::default());
        diff_gjk(&format!("row42 rand[{i}]"), p, &inp2);
    }
    assert!(saw_midpoint > 0, "row42 never hit the midpoint branch");
}

#[test]
fn row43_cores_intersect_hit_path() {
    let p = pair();
    let mut rng = Rng::new(0x4343);
    let mut saw_hit = 0;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..256 {
                let scale = rng.scale_choice();
                // both centred on the origin -> cores overlap -> simplex hits 3
                let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                let b = shape_at(&mut rng, tb, c2v { x: 0.0, y: 0.0 }, scale);
                let mut inp = GjkIn::new(&a, &b);
                inp.cache = Some(c2GJKCache::default());
                inp.use_radius = if rng.below(2) == 0 { 1 } else { 0 };
                let out = diff_gjk(
                    &format!("row43 {}x{}[{i}]", type_name(ta), type_name(tb)),
                    p,
                    &inp,
                );
                if out.cache.count == 3 {
                    saw_hit += 1;
                }
            }
        }
    }
    assert!(saw_hit > 0, "row43 never reached the `hit` path");
    eprintln!("row43 hit-path count = {saw_hit}");
}

#[test]
fn row44_identical_shapes() {
    let p = pair();
    let mut rng = Rng::new(0x4444);
    for &ta in TYPES.iter() {
        for i in 0..512 {
            let scale = rng.scale_choice();
            let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
            let b = a; // byte-identical shape
            for ur in [0, 1] {
                let mut inp = GjkIn::new(&a, &b);
                inp.use_radius = ur;
                inp.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("row44 {} ur={ur}[{i}]", type_name(ta)), p, &inp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 45..47: degenerate and "invalid but unvalidated" shapes
// ---------------------------------------------------------------------------

#[test]
fn row45_degenerate_shapes() {
    let p = pair();
    let mut rng = Rng::new(0x4545);
    for i in 0..2048 {
        let scale = rng.scale_choice();
        let pt = rng.vec_scaled(scale);
        let degens = [
            Shape::Circle(c2Circle { p: pt, r: 0.0 }),                    // zero-radius circle
            Shape::Aabb(c2AABB { min: pt, max: pt }),                     // zero-area AABB
            Shape::Capsule(c2Capsule { a: pt, b: pt, r: 0.0 }),           // point capsule
            Shape::Capsule(c2Capsule { a: pt, b: pt, r: scale * 0.5 }),    // zero-length capsule
            Shape::Aabb(c2AABB { min: pt, max: c2v { x: pt.x, y: pt.y + scale } }), // zero width
        ];
        for (ja, a) in degens.iter().enumerate() {
            for (jb, b) in degens.iter().enumerate() {
                let mut inp = GjkIn::new(a, b);
                inp.cache = Some(c2GJKCache::default());
                inp.use_radius = (i % 2) as c_int;
                diff_gjk(&format!("row45 [{ja}]x[{jb}][{i}]"), p, &inp);
            }
        }
    }
}

#[test]
fn row46_inverted_aabb() {
    let p = pair();
    let mut rng = Rng::new(0x4646);
    for i in 0..2048 {
        let scale = rng.scale_choice();
        let lo = rng.vec_scaled(scale);
        let hi = c2v { x: lo.x + scale, y: lo.y + scale };
        // min > max: no validation anywhere in the C
        let inverted = Shape::Aabb(c2AABB { min: hi, max: lo });
        let partial = Shape::Aabb(c2AABB { min: c2v { x: hi.x, y: lo.y }, max: c2v { x: lo.x, y: hi.y } });
        let ot = TYPES[rng.below(3) as usize];
        let other = shape_at(&mut rng, ot, c2v { x: 0.0, y: 0.0 }, scale);
        for (j, bad) in [inverted, partial].iter().enumerate() {
            for ur in [0, 1] {
                let mut inp = GjkIn::new(bad, &other);
                inp.use_radius = ur;
                inp.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("row46 fwd[{j}] ur={ur}[{i}]"), p, &inp);
                let mut inp2 = GjkIn::new(&other, bad);
                inp2.use_radius = ur;
                inp2.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("row46 rev[{j}] ur={ur}[{i}]"), p, &inp2);
            }
        }
    }
}

#[test]
fn row47_negative_radii() {
    let p = pair();
    let mut rng = Rng::new(0x4747);
    for i in 0..2048 {
        let scale = rng.scale_choice();
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -scale });
        let b = Shape::Capsule(c2Capsule {
            a: c2v { x: scale * 2.0, y: 0.0 },
            b: c2v { x: scale * 2.0, y: scale },
            r: -scale * 0.5,
        });
        let c = Shape::Circle(c2Circle { p: c2v { x: scale * 3.0, y: 0.0 }, r: scale });
        for (j, (x, y)) in [(&a, &b), (&a, &c), (&b, &c), (&b, &a), (&c, &a)].iter().enumerate() {
            for ur in [0, 1] {
                let mut inp = GjkIn::new(x, y);
                inp.use_radius = ur;
                inp.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("row47 [{j}] ur={ur}[{i}]"), p, &inp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 48: coordinate-scale sweep
// ---------------------------------------------------------------------------

#[test]
fn row48_scale_sweep() {
    let p = pair();
    let mut rng = Rng::new(0x4848);
    let mut max_iter = 0;
    for scale in [1e-6f32, 1e-3, 1.0, 1e3, 1e5, 1e7, 1e15, 1e30] {
        for &ta in TYPES.iter() {
            for &tb in TYPES.iter() {
                for i in 0..128 {
                    let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
                    let b = shape_off(&mut rng, tb, scale * 2.0, scale);
                    let mut inp = GjkIn::new(&a, &b);
                    inp.cache = Some(c2GJKCache::default());
                    inp.use_radius = (i % 2) as c_int;
                    let out = diff_gjk(
                        &format!("row48 scale={scale} {}x{}[{i}]", type_name(ta), type_name(tb)),
                        p,
                        &inp,
                    );
                    max_iter = max_iter.max(out.iters);
                }
            }
        }
    }
    eprintln!("row48 max iterations observed = {max_iter}");
}

// ---------------------------------------------------------------------------
// Row 49: use_radius truthiness
// ---------------------------------------------------------------------------

#[test]
fn row49_use_radius_truthiness() {
    let p = pair();
    let mut rng = Rng::new(0x4949);
    for ur in [0i32, 1, 2, -1, i32::MIN, i32::MAX, 0x100] {
        for i in 0..512 {
            let scale = rng.scale_choice();
            let ta = TYPES[rng.below(3) as usize];
            let tb = TYPES[rng.below(3) as usize];
            let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
            let b = shape_off(&mut rng, tb, scale * 3.0, scale);
            let mut inp = GjkIn::new(&a, &b);
            inp.use_radius = ur as c_int;
            inp.cache = Some(c2GJKCache::default());
            diff_gjk(&format!("row49 ur={ur}[{i}]"), p, &inp);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 50: large joint random sweep over every axis at once
// ---------------------------------------------------------------------------

#[test]
fn row50_joint_random_sweep() {
    let p = pair();
    let mut rng = Rng::new(0x5050);
    let mut branches = [0usize; 3];
    let mut iters_hist = [0usize; 21];
    for i in 0..16384 {
        let scale = rng.scale_choice();
        let ta = TYPES[rng.below(3) as usize];
        let tb = TYPES[rng.below(3) as usize];
        let sep = [0.0f32, 0.25, 0.5, 1.0, 2.0, 5.0, 20.0, 100.0][rng.below(8) as usize];
        let ang = rng.unit() * std::f32::consts::PI;
        let a = shape_at(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, scale);
        let b = shape_at(
            &mut rng,
            tb,
            c2v { x: sep * scale * ang.cos(), y: sep * scale * ang.sin() },
            scale,
        );
        let mut inp = GjkIn::new(&a, &b);
        inp.ax = match rng.below(3) {
            0 => None,
            1 => Some(rand_transform(&mut rng, scale)),
            _ => Some(rand_transform_unnorm(&mut rng, scale)),
        };
        inp.bx = match rng.below(3) {
            0 => None,
            1 => Some(rand_transform(&mut rng, scale)),
            _ => Some(rand_transform_unnorm(&mut rng, scale)),
        };
        inp.use_radius = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => -1,
        };
        let has_cache = rng.below(3) != 0;
        inp.cache = if has_cache { Some(c2GJKCache::default()) } else { None };
        inp.want_out_a = rng.below(4) != 0;
        inp.want_out_b = rng.below(4) != 0;
        inp.want_iters = rng.below(4) != 0;
        let out = diff_gjk(&format!("row50[{i}]"), p, &inp);
        branches[classify_branch(&out, has_cache)] += 1;
        if inp.want_iters && out.iters >= 0 && out.iters <= 20 {
            iters_hist[out.iters as usize] += 1;
        }
    }
    eprintln!("row50 branch coverage (hit, shrink, midpoint) = {branches:?}");
    eprintln!("row50 iteration histogram = {iters_hist:?}");
    assert!(branches.iter().all(|&n| n > 0), "row50 missed a final branch: {branches:?}");
}
