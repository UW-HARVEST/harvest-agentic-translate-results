//! Phase B — differential tests for `c2GJK`, the full cross-product of
//! shape pair × transforms × use_radius × cache × separation regime.
//! CONFIGS.md rows 49–79.

mod common;
use common::*;
use std::ffi::c_int;

const N: usize = 900;

/// Builds a shape of `ty` whose coordinates live inside `[-span, span]^2` and
/// whose radius is at most `rmax`, so that the separation regime is
/// controllable.
fn shape_in(rng: &mut Rng, ty: c_int, centre: c2v, span: f32, rmax: f32) -> ShapeBuf {
    let pt = |rng: &mut Rng| c2v {
        x: centre.x + rng.sym(span),
        y: centre.y + rng.sym(span),
    };
    match ty {
        C2_TYPE_CIRCLE => ShapeBuf::circle(c2Circle {
            p: pt(rng),
            r: rng.unit() * rmax,
        }),
        C2_TYPE_AABB => {
            let a = pt(rng);
            let b = pt(rng);
            ShapeBuf::aabb(c2AABB {
                min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
            })
        }
        _ => ShapeBuf::capsule(c2Capsule {
            a: pt(rng),
            b: pt(rng),
            r: rng.unit() * rmax,
        }),
    }
}

fn pair_name(ta: c_int, tb: c_int) -> String {
    let n = |t: c_int| match t {
        C2_TYPE_CIRCLE => "circle",
        C2_TYPE_AABB => "aabb",
        _ => "capsule",
    };
    format!("{}x{}", n(ta), n(tb))
}

/// Runs one differential `c2GJK` comparison.
#[allow(clippy::too_many_arguments)]
fn diff(
    ctx: &str,
    a: &ShapeBuf,
    ax: Option<&c2x>,
    b: &ShapeBuf,
    bx: Option<&c2x>,
    use_radius: c_int,
    want_out: bool,
    want_iters: bool,
    cache_in: Option<c2GJKCache>,
) {
    let (c, r) = (&libs().c, &libs().r);
    let co = call_gjk(c, a, ax, b, bx, use_radius, want_out, want_iters, cache_in);
    let ro = call_gjk(r, a, ax, b, bx, use_radius, want_out, want_iters, cache_in);
    assert_gjk_eq(ctx, &co, &ro);
}

// ---------------------------------------------------------------------------
// rows 49-58: all 9 shape pairs, identity transforms, use_radius on and off
// ---------------------------------------------------------------------------

#[test]
fn row49_to_row58_all_pairs_identity_xforms() {
    let mut rng = Rng::new(SEED ^ 49);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N {
                // sweep the separation regime by varying the centre offset
                let span = [0.5f32, 2.0, 8.0][i % 3];
                let off = [0.0f32, 1.0, 3.0, 12.0, 500.0][i % 5];
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, span, 4.0);
                let b = shape_in(&mut rng, tb, c2v { x: off, y: off * 0.5 }, span, 4.0);
                for ur in [1i32, 0] {
                    diff(
                        &format!("row49-58 {name} ur={ur} #{i}"),
                        &a,
                        None,
                        &b,
                        None,
                        ur,
                        true,
                        true,
                        None,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 59-62: transforms
// ---------------------------------------------------------------------------

#[test]
fn row59_to_row61_transforms() {
    let mut rng = Rng::new(SEED ^ 59);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N {
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 3.0);
                let off = [0.0f32, 2.0, 9.0][i % 3];
                let b = shape_in(&mut rng, tb, c2v { x: off, y: -off }, 3.0, 3.0);
                // row 59: translate-only ax, bx == NULL
                let axt = c2x {
                    p: rng.coord_v(),
                    r: c2r { c: 1.0, s: 0.0 },
                };
                diff(
                    &format!("row59 {name} #{i}"),
                    &a,
                    Some(&axt),
                    &b,
                    None,
                    1,
                    true,
                    true,
                    None,
                );
                // row 60: ax == NULL, translate-only bx
                let bxt = c2x {
                    p: rng.coord_v(),
                    r: c2r { c: 1.0, s: 0.0 },
                };
                diff(
                    &format!("row60 {name} #{i}"),
                    &a,
                    None,
                    &b,
                    Some(&bxt),
                    1,
                    true,
                    true,
                    None,
                );
                // row 61: both non-NULL with real unit rotations
                let axr = rng.unit_x();
                let bxr = rng.unit_x();
                for ur in [1i32, 0] {
                    diff(
                        &format!("row61 {name} ur={ur} #{i}"),
                        &a,
                        Some(&axr),
                        &b,
                        Some(&bxr),
                        ur,
                        true,
                        true,
                        None,
                    );
                }
                // explicit identity structs (distinct from NULL in source, same
                // in behaviour) — verifies the !ax_ptr substitution
                let id = c2x {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: c2r { c: 1.0, s: 0.0 },
                };
                diff(
                    &format!("row61 {name} explicit-identity #{i}"),
                    &a,
                    Some(&id),
                    &b,
                    Some(&id),
                    1,
                    true,
                    true,
                    None,
                );
            }
        }
    }
}

#[test]
fn row62_degenerate_transforms() {
    let mut rng = Rng::new(SEED ^ 62);
    let rots = [
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 2.0, s: 3.0 },      // non-unit
        c2r { c: -1.0, s: 0.0 },
        c2r { c: f32::MAX, s: f32::MAX },
        c2r { c: f32::NAN, s: 0.0 },
        c2r { c: 0.0, s: f32::NAN },
        c2r { c: f32::INFINITY, s: 1.0 },
        c2r { c: f32::MIN_POSITIVE, s: f32::MIN_POSITIVE },
    ];
    let ps = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 1e6, y: -1e6 },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ];
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for (ri, &rot) in rots.iter().enumerate() {
                for (pi, &p) in ps.iter().enumerate() {
                    for i in 0..8 {
                        let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 2.0);
                        let b = shape_in(&mut rng, tb, c2v { x: 4.0, y: 0.0 }, 3.0, 2.0);
                        let x = c2x { p, r: rot };
                        for ur in [1i32, 0] {
                            diff(
                                &format!("row62 {name} rot#{ri} p#{pi} ur={ur} A #{i}"),
                                &a,
                                Some(&x),
                                &b,
                                None,
                                ur,
                                true,
                                true,
                                None,
                            );
                            diff(
                                &format!("row62 {name} rot#{ri} p#{pi} ur={ur} B #{i}"),
                                &a,
                                None,
                                &b,
                                Some(&x),
                                ur,
                                true,
                                true,
                                None,
                            );
                            diff(
                                &format!("row62 {name} rot#{ri} p#{pi} ur={ur} AB #{i}"),
                                &a,
                                Some(&x),
                                &b,
                                Some(&x),
                                ur,
                                true,
                                true,
                                None,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 63-66: the cache
// ---------------------------------------------------------------------------

#[test]
fn row63_cold_cache() {
    let mut rng = Rng::new(SEED ^ 63);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N {
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 3.0);
                let off = [0.0f32, 1.5, 7.0][i % 3];
                let b = shape_in(&mut rng, tb, c2v { x: off, y: off }, 3.0, 3.0);
                // A truly cold cache: count == 0 (cache_was_good false).
                let cold = c2GJKCache {
                    metric: 0.0,
                    count: 0,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 0.0,
                };
                for ur in [1i32, 0] {
                    diff(
                        &format!("row63 {name} ur={ur} #{i}"),
                        &a,
                        None,
                        &b,
                        None,
                        ur,
                        true,
                        true,
                        Some(cold),
                    );
                }
                // cold but with garbage in the *unused* fields, to prove the
                // count==0 short-circuit really does ignore them
                let dirty = c2GJKCache {
                    metric: rng.wild(),
                    count: 0,
                    iA: [3, 1, 2],
                    iB: [2, 0, 1],
                    div: rng.wild(),
                };
                diff(
                    &format!("row63 {name} dirty-cold #{i}"),
                    &a,
                    None,
                    &b,
                    None,
                    1,
                    true,
                    true,
                    Some(dirty),
                );
            }
        }
    }
}

/// Two calls sharing one cache object: the second call reads back what the
/// first wrote, so the whole `cache_was_read` path (including the C's inverted
/// `metric < -1.0e8f` test) is exercised.
#[test]
fn row64_row65_warm_cache() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 64);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N {
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 3.0);
                let off = [0.0f32, 1.5, 7.0][i % 3];
                let b = shape_in(&mut rng, tb, c2v { x: off, y: off }, 3.0, 3.0);
                let ax = if i % 2 == 0 { Some(rng.unit_x()) } else { None };
                let bx = if i % 3 == 0 { Some(rng.unit_x()) } else { None };
                let ur = (i % 2) as c_int;

                let mut ccache = c2GJKCache::default();
                let mut rcache = c2GJKCache::default();

                // first (cold) call
                let c1 = call_gjk(c, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, Some(ccache));
                let r1 = call_gjk(r, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, Some(rcache));
                assert_gjk_eq(&format!("row64 {name} call1 #{i}"), &c1, &r1);
                ccache = c1.cache;
                rcache = r1.cache;

                // row 64: second call, shapes unchanged (warm cache hit)
                let c2_ = call_gjk(c, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, Some(ccache));
                let r2_ = call_gjk(r, &a, ax.as_ref(), &b, bx.as_ref(), ur, true, true, Some(rcache));
                assert_gjk_eq(&format!("row64 {name} call2-same #{i}"), &c2_, &r2_);
                ccache = c2_.cache;
                rcache = r2_.cache;

                // row 65: third call with shape B moved (stale cache)
                let b2 = shift_shape(&b, c2v { x: rng.sym(6.0), y: rng.sym(6.0) });
                let c3 = call_gjk(c, &a, ax.as_ref(), &b2, bx.as_ref(), ur, true, true, Some(ccache));
                let r3 = call_gjk(r, &a, ax.as_ref(), &b2, bx.as_ref(), ur, true, true, Some(rcache));
                assert_gjk_eq(&format!("row65 {name} call3-moved #{i}"), &c3, &r3);
            }
        }
    }
}

/// A long-lived cache re-used across a sweep, so the cached simplex indices go
/// stale gradually rather than all at once.
#[test]
fn row66_long_lived_cache_sweep() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 66);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N / 3 {
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 2.5, 2.0);
                let b0 = shape_in(&mut rng, tb, c2v { x: 0.0, y: 0.0 }, 2.5, 2.0);
                let step = c2v { x: rng.sym(2.0), y: rng.sym(2.0) };
                let mut ccache = c2GJKCache::default();
                let mut rcache = c2GJKCache::default();
                let mut d = c2v { x: -8.0, y: -8.0 };
                for k in 0..8 {
                    let b = shift_shape(&b0, d);
                    let co = call_gjk(c, &a, None, &b, None, 1, true, true, Some(ccache));
                    let ro = call_gjk(r, &a, None, &b, None, 1, true, true, Some(rcache));
                    assert_gjk_eq(&format!("row66 {name} #{i} step {k}"), &co, &ro);
                    ccache = co.cache;
                    rcache = ro.cache;
                    d = c2v { x: d.x + step.x, y: d.y + step.y };
                }
            }
        }
    }
}

/// Hand-forged caches with in-range indices (out-of-range indices would read
/// uninitialised proxy slots in the C — see ERRORS.md rows 11/12).
#[test]
fn row63_forged_cache_in_range() {
    let mut rng = Rng::new(SEED ^ 67);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            let na = match ta {
                C2_TYPE_CIRCLE => 1,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            let nb = match tb {
                C2_TYPE_CIRCLE => 1,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            for count in [1i32, 2, 3] {
                for i in 0..200 {
                    let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 2.0);
                    let b = shape_in(&mut rng, tb, c2v { x: 3.0, y: 1.0 }, 3.0, 2.0);
                    let mut iA = [0i32; 3];
                    let mut iB = [0i32; 3];
                    for k in 0..3 {
                        iA[k] = rng.below(na) as c_int;
                        iB[k] = rng.below(nb) as c_int;
                    }
                    let metric = match i % 6 {
                        0 => 0.0,
                        1 => -1.0e9,  // the only value that can satisfy metric < -1e8
                        2 => 1.0e9,
                        3 => rng.coord(),
                        4 => f32::NAN,
                        _ => rng.sym(50.0),
                    };
                    let div = match i % 5 {
                        0 => 1.0,
                        1 => 0.0,
                        2 => rng.coord(),
                        3 => f32::NAN,
                        _ => rng.sym(10.0),
                    };
                    let forged = c2GJKCache {
                        metric,
                        count,
                        iA,
                        iB,
                        div,
                    };
                    for ur in [1i32, 0] {
                        diff(
                            &format!("row63 {name} forged count={count} ur={ur} #{i}"),
                            &a,
                            None,
                            &b,
                            None,
                            ur,
                            true,
                            true,
                            Some(forged),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 67-70: separation regimes
// ---------------------------------------------------------------------------

#[test]
fn row67_to_row70_separation_regimes() {
    let mut rng = Rng::new(SEED ^ 70);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N {
                // row 67: clearly disjoint
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 1.0, 0.5);
                let b = shape_in(&mut rng, tb, c2v { x: 100.0, y: 60.0 }, 1.0, 0.5);
                diff(&format!("row67 {name} #{i}"), &a, None, &b, None, 1, true, true, None);
                diff(&format!("row67 {name} ur0 #{i}"), &a, None, &b, None, 0, true, true, None);

                // row 68: deeply penetrating (both centred on the origin, big)
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 8.0, 6.0);
                let b = shape_in(&mut rng, tb, c2v { x: 0.0, y: 0.0 }, 8.0, 6.0);
                diff(&format!("row68 {name} #{i}"), &a, None, &b, None, 1, true, true, None);
                diff(&format!("row68 {name} ur0 #{i}"), &a, None, &b, None, 0, true, true, None);

                // row 69: grazing — tiny offsets around the contact distance
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 1.0, 1.0);
                let eps = [0.0f32, 1e-7, -1e-7, 1e-4, -1e-4, 1.0, -1.0][i % 7];
                let b = shape_in(&mut rng, tb, c2v { x: 2.0 + eps, y: 0.0 }, 1.0, 1.0);
                diff(&format!("row69 {name} eps={eps:?} #{i}"), &a, None, &b, None, 1, true, true, None);
                diff(&format!("row69 {name} eps={eps:?} ur0 #{i}"), &a, None, &b, None, 0, true, true, None);
            }
            // row 70: coincident shapes — literally the same bytes for A and B
            for i in 0..N {
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 2.0);
                let mut b = shape_in(&mut rng, tb, c2v { x: 0.0, y: 0.0 }, 3.0, 2.0);
                if ta == tb {
                    b = a;
                }
                for ur in [1i32, 0] {
                    diff(
                        &format!("row70 {name} coincident ur={ur} #{i}"),
                        &a,
                        None,
                        &b,
                        None,
                        ur,
                        true,
                        true,
                        None,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 71-76: degenerate and extreme shapes
// ---------------------------------------------------------------------------

fn degenerate_shapes(rng: &mut Rng, ty: c_int, which: usize) -> ShapeBuf {
    let p = rng.coord_v();
    match ty {
        C2_TYPE_CIRCLE => ShapeBuf::circle(match which {
            0 => c2Circle { p, r: 0.0 },
            1 => c2Circle { p, r: -rng.unit() * 4.0 },
            2 => c2Circle { p, r: f32::MAX },
            3 => c2Circle { p, r: f32::MIN_POSITIVE },
            4 => c2Circle { p, r: f32::NAN },
            5 => c2Circle { p, r: f32::INFINITY },
            6 => c2Circle { p: c2v { x: f32::NAN, y: p.y }, r: 1.0 },
            7 => c2Circle { p: c2v { x: f32::INFINITY, y: p.y }, r: 1.0 },
            8 => c2Circle { p: c2v { x: f32::MAX, y: f32::MAX }, r: 1.0 },
            _ => c2Circle { p: c2v { x: f32::from_bits(1), y: 0.0 }, r: f32::from_bits(1) },
        }),
        C2_TYPE_AABB => ShapeBuf::aabb(match which {
            0 => c2AABB { min: p, max: p },                                       // zero area
            1 => c2AABB { min: p, max: c2v { x: p.x - 1.0, y: p.y - 1.0 } },      // inverted
            2 => c2AABB { min: p, max: c2v { x: p.x + 1.0, y: p.y } },            // zero height
            3 => c2AABB { min: p, max: c2v { x: p.x, y: p.y + 1.0 } },            // zero width
            4 => c2AABB { min: c2v { x: f32::MIN, y: f32::MIN }, max: c2v { x: f32::MAX, y: f32::MAX } },
            5 => c2AABB { min: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY }, max: c2v { x: f32::INFINITY, y: f32::INFINITY } },
            6 => c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
            7 => c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: f32::NAN, y: 1.0 } },
            8 => c2AABB { min: c2v { x: -0.0, y: -0.0 }, max: c2v { x: 0.0, y: 0.0 } },
            _ => c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: f32::from_bits(1), y: f32::from_bits(1) } },
        }),
        _ => ShapeBuf::capsule(match which {
            0 => c2Capsule { a: p, b: p, r: 1.0 },                                // zero length
            1 => c2Capsule { a: p, b: p, r: 0.0 },                                // point
            2 => c2Capsule { a: p, b: c2v { x: p.x + 1.0, y: p.y }, r: 0.0 },     // zero radius
            3 => c2Capsule { a: p, b: c2v { x: p.x + 1.0, y: p.y }, r: -2.0 },    // negative radius
            4 => c2Capsule { a: p, b: c2v { x: p.x + 1.0, y: p.y }, r: f32::MAX },
            5 => c2Capsule { a: p, b: c2v { x: p.x + 1.0, y: p.y }, r: f32::NAN },
            6 => c2Capsule { a: p, b: c2v { x: p.x + 1.0, y: p.y }, r: f32::INFINITY },
            7 => c2Capsule { a: c2v { x: f32::NAN, y: 0.0 }, b: p, r: 1.0 },
            8 => c2Capsule { a: c2v { x: f32::NEG_INFINITY, y: 0.0 }, b: c2v { x: f32::INFINITY, y: 0.0 }, r: 1.0 },
            _ => c2Capsule { a: c2v { x: f32::MIN_POSITIVE, y: 0.0 }, b: c2v { x: f32::from_bits(1), y: 0.0 }, r: f32::from_bits(3) },
        }),
    }
}

#[test]
fn row71_to_row76_degenerate_and_extreme_shapes() {
    let mut rng = Rng::new(SEED ^ 71);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for wa in 0..10usize {
                for wb in 0..10usize {
                    for i in 0..4 {
                        let a = degenerate_shapes(&mut rng, ta, wa);
                        let b = degenerate_shapes(&mut rng, tb, wb);
                        for ur in [1i32, 0] {
                            diff(
                                &format!("row71-76 {name} A{wa} B{wb} ur={ur} #{i}"),
                                &a,
                                None,
                                &b,
                                None,
                                ur,
                                true,
                                true,
                                None,
                            );
                        }
                        // and once more with a cache and rotations in play
                        let ax = rng.unit_x();
                        let cold = c2GJKCache::default();
                        diff(
                            &format!("row71-76 {name} A{wa} B{wb} xform+cache #{i}"),
                            &a,
                            Some(&ax),
                            &b,
                            None,
                            1,
                            true,
                            true,
                            Some(cold),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 77-79: out-pointer options and the iteration cap
// ---------------------------------------------------------------------------

#[test]
fn row77_row78_out_pointer_options() {
    let mut rng = Rng::new(SEED ^ 77);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let name = pair_name(ta, tb);
            for i in 0..N / 2 {
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, 3.0, 2.0);
                let off = [0.0f32, 2.0, 9.0][i % 3];
                let b = shape_in(&mut rng, tb, c2v { x: off, y: off }, 3.0, 2.0);
                for want_out in [true, false] {
                    for want_iters in [true, false] {
                        for cache in [None, Some(c2GJKCache::default())] {
                            for ur in [1i32, 0] {
                                diff(
                                    &format!(
                                        "row77-78 {name} out={want_out} it={want_iters} cache={} ur={ur} #{i}",
                                        cache.is_some()
                                    ),
                                    &a,
                                    None,
                                    &b,
                                    None,
                                    ur,
                                    want_out,
                                    want_iters,
                                    cache,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Shapes engineered so GJK needs many iterations: two long thin boxes /
/// capsules at awkward angles, plus huge coordinate magnitudes where the
/// `d1 > d0` progress test fires late.
#[test]
fn row79_iteration_pressure() {
    let mut rng = Rng::new(SEED ^ 79);
    let mut max_c = 0i32;
    for i in 0..6000 {
        let scale = [1.0f32, 1e-3, 1e3, 1e6][i % 4];
        let thin = ShapeBuf::aabb(c2AABB {
            min: c2v { x: -1e3 * scale, y: -1e-3 * scale },
            max: c2v { x: 1e3 * scale, y: 1e-3 * scale },
        });
        let cap = ShapeBuf::capsule(c2Capsule {
            a: c2v { x: rng.sym(1e3 * scale), y: rng.sym(1e3 * scale) },
            b: c2v { x: rng.sym(1e3 * scale), y: rng.sym(1e3 * scale) },
            r: rng.unit() * 1e-4 * scale,
        });
        let ax = rng.unit_x();
        let bx = rng.unit_x();
        let (c, r) = (&libs().c, &libs().r);
        for (aa, bb) in [(&thin, &cap), (&cap, &thin)] {
            for (xa, xb) in [
                (None, None),
                (Some(&ax), None),
                (None, Some(&bx)),
                (Some(&ax), Some(&bx)),
            ] {
                let co = call_gjk(c, aa, xa, bb, xb, 1, true, true, Some(c2GJKCache::default()));
                let ro = call_gjk(r, aa, xa, bb, xb, 1, true, true, Some(c2GJKCache::default()));
                assert_gjk_eq(&format!("row79 iteration pressure #{i}"), &co, &ro);
                max_c = max_c.max(co.iters);
            }
        }
    }
    eprintln!("row79 max iterations observed: {max_c}");
    assert!(max_c >= 2, "iteration pressure test never iterated ({max_c})");
}

/// Evidence that the Phase B sweep really reaches each terminal regime of
/// `c2GJK`, classified from observable outputs only (by running the same
/// configuration with `use_radius` on and off).
#[test]
fn coverage_gjk_regimes() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0xC0FFEE);
    let mut hit = 0usize; // s.count == 3  => ur=0 gives dist 0 and a == b
    let mut shrink = 0usize; // ur=1 shrink branch taken (dist changed, > 0)
    let mut midpoint = 0usize; // ur=1 else-branch: dist collapsed to 0
    let mut plain = 0usize; // ur=0, positive distance
    let mut iters = [0usize; 21];

    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for i in 0..3000 {
                let span = [0.5f32, 2.0, 8.0][i % 3];
                let off = [0.0f32, 0.5, 1.0, 2.0, 4.0, 30.0][i % 6];
                let a = shape_in(&mut rng, ta, c2v { x: 0.0, y: 0.0 }, span, 3.0);
                let b = shape_in(&mut rng, tb, c2v { x: off, y: off * 0.3 }, span, 3.0);
                let c0 = call_gjk(c, &a, None, &b, None, 0, true, true, None);
                let r0 = call_gjk(r, &a, None, &b, None, 0, true, true, None);
                assert_gjk_eq("coverage ur=0", &c0, &r0);
                let c1 = call_gjk(c, &a, None, &b, None, 1, true, true, None);
                let r1 = call_gjk(r, &a, None, &b, None, 1, true, true, None);
                assert_gjk_eq("coverage ur=1", &c1, &r1);

                if (0..=20).contains(&c0.iters) {
                    iters[c0.iters as usize] += 1;
                }
                if c0.dist == 0.0 && veq(c0.a, c0.b) {
                    hit += 1;
                } else if c0.dist > 0.0 {
                    plain += 1;
                    if c1.dist == 0.0 {
                        midpoint += 1;
                    } else if c1.dist > 0.0 && c1.dist != c0.dist {
                        shrink += 1;
                    }
                }
            }
        }
    }
    eprintln!("gjk regime coverage: hit={hit} plain={plain} shrink={shrink} midpoint={midpoint}");
    eprintln!("iteration histogram (index = iter count): {iters:?}");
    assert!(hit > 0, "the s.count==3 (hit) path was never reached");
    assert!(shrink > 0, "the use_radius shrink path was never reached");
    assert!(
        midpoint > 0,
        "the use_radius midpoint-collapse path was never reached"
    );
    assert!(plain > 0, "no positive-distance case was produced");
    assert!(
        iters[0] > 0 && iters[1] > 0 && iters[2] > 0,
        "iteration counts 0..2 not all seen: {iters:?}"
    );
}
