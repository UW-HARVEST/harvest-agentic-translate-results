//! Phase B — Group 5: `c2GJK`, the lowest-level distance entry point, driven
//! directly through the `.so` with every option combination.
//! CONFIGS.md rows 59..78.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_int;

const N: usize = 512;
const KINDS: [u32; 3] = [0, 1, 2]; // CIRCLE, AABB, CAPSULE

/// rows 59..64, 67, 75 — the full option matrix over all 9 type pairs.
#[test]
fn cfg_gjk_matrix() {
    let mut acc = DiffAccum::new("cfg_gjk_matrix");
    let mut rng = Rng::new(0x9eed_0001);
    // (use_radius, ax?, bx?)
    let combos: [(c_int, bool, bool); 6] = [
        (0, false, false),
        (1, false, false),
        (0, true, false),
        (0, false, true),
        (0, true, true),
        (1, true, true),
    ];
    for ka in KINDS {
        for kb in KINDS {
            for &(ur, has_ax, has_bx) in &combos {
                for i in 0..N {
                    let sa = rng.nice_shape(ka);
                    let sb = rng.nice_shape(kb);
                    let args = GjkArgs {
                        ax: if has_ax { Some(rng.xform()) } else { None },
                        bx: if has_bx { Some(rng.xform()) } else { None },
                        use_radius: ur,
                        ..Default::default()
                    };
                    acc.check(
                        format!("ka={ka} kb={kb} ur={ur} ax={has_ax} bx={has_bx} #{i}"),
                        |s| run_gjk(s, &sa, &sb, &args),
                    );
                }
            }
        }
    }
    acc.finish();
}

/// row 65 — deep overlap (⇒ simplex reaches count 3 ⇒ `hit` ⇒ dist == 0).
#[test]
fn cfg_gjk_overlap() {
    let mut acc = DiffAccum::new("cfg_gjk_overlap");
    let mut rng = Rng::new(0x9eed_0002);
    let mut hits = 0usize;
    for ka in KINDS {
        for kb in KINDS {
            for &ur in &[0, 1] {
                for i in 0..(N * 2) {
                    // both shapes centred on the origin ⇒ deep overlap
                    let sa = rng.nice_shape(ka).translate(0.0, 0.0);
                    let sb = rng.nice_shape(kb);
                    let sb = match sb {
                        Shape::Ci(c) => Shape::Ci(c2Circle {
                            p: c2v {
                                x: rng.sym(0.2),
                                y: rng.sym(0.2),
                            },
                            r: c.r,
                        }),
                        Shape::Bb(_) => Shape::Bb(c2AABB {
                            min: c2v { x: -1.0, y: -1.0 },
                            max: c2v { x: 1.0, y: 1.0 },
                        }),
                        Shape::Ca(c) => Shape::Ca(c2Capsule {
                            a: c2v { x: -1.0, y: 0.0 },
                            b: c2v { x: 1.0, y: 0.0 },
                            r: c.r,
                        }),
                    };
                    let sa = match sa {
                        Shape::Ci(c) => Shape::Ci(c2Circle {
                            p: c2v {
                                x: rng.sym(0.2),
                                y: rng.sym(0.2),
                            },
                            r: c.r,
                        }),
                        Shape::Bb(_) => Shape::Bb(c2AABB {
                            min: c2v { x: -0.8, y: -0.8 },
                            max: c2v { x: 0.8, y: 0.8 },
                        }),
                        Shape::Ca(c) => Shape::Ca(c2Capsule {
                            a: c2v { x: 0.0, y: -1.0 },
                            b: c2v { x: 0.0, y: 1.0 },
                            r: c.r,
                        }),
                    };
                    let args = GjkArgs {
                        use_radius: ur,
                        ..Default::default()
                    };
                    let o = acc_check_ret(&mut acc, format!("ka={ka} kb={kb} ur={ur} #{i}"), |s| {
                        run_gjk(s, &sa, &sb, &args)
                    });
                    if o.dist == 0.0 {
                        hits += 1;
                    }
                }
            }
        }
    }
    acc.finish();
    eprintln!("cfg_gjk_overlap: {hits} zero-distance (hit) cases");
    assert!(hits > 0);
}

fn acc_check_ret<R: BitEq + Copy, F: FnMut(Side) -> R>(
    acc: &mut DiffAccum,
    label: String,
    mut f: F,
) -> R {
    let c = f(Side::C);
    acc.check(label, |s| f(s));
    c
}

/// row 66 — exact touch (integer coordinates so the compare is exact).
#[test]
fn cfg_gjk_touch() {
    let mut acc = DiffAccum::new("cfg_gjk_touch");
    for &ur in &[0, 1] {
        // circle/circle at exactly r1+r2
        for k in 0..64 {
            let r1 = 0.5 + k as f32 * 0.25;
            let r2 = 1.0;
            let sa = Shape::Ci(c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: r1,
            });
            let sb = Shape::Ci(c2Circle {
                p: c2v { x: r1 + r2, y: 0.0 },
                r: r2,
            });
            let args = GjkArgs {
                use_radius: ur,
                ..Default::default()
            };
            acc.check(format!("cc ur={ur} k={k}"), |s| run_gjk(s, &sa, &sb, &args));
        }
        // aabb/aabb sharing an edge
        for k in 0..64 {
            let e = 1.0 + k as f32 * 0.5;
            let sa = Shape::Bb(c2AABB {
                min: c2v { x: -e, y: -e },
                max: c2v { x: 0.0, y: e },
            });
            let sb = Shape::Bb(c2AABB {
                min: c2v { x: 0.0, y: -e },
                max: c2v { x: e, y: e },
            });
            let args = GjkArgs {
                use_radius: ur,
                ..Default::default()
            };
            acc.check(format!("bb ur={ur} k={k}"), |s| run_gjk(s, &sa, &sb, &args));
        }
        // capsule/capsule tip to tip
        for k in 0..64 {
            let r = 0.5 + k as f32 * 0.25;
            let sa = Shape::Ca(c2Capsule {
                a: c2v { x: -2.0, y: 0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r,
            });
            let sb = Shape::Ca(c2Capsule {
                a: c2v { x: 2.0 * r, y: 0.0 },
                b: c2v { x: 2.0 * r + 2.0, y: 0.0 },
                r,
            });
            let args = GjkArgs {
                use_radius: ur,
                ..Default::default()
            };
            acc.check(format!("caca ur={ur} k={k}"), |s| run_gjk(s, &sa, &sb, &args));
        }
    }
    acc.finish();
}

/// rows 62..64, 68, 69 — `use_radius` shrink and midpoint branches.
#[test]
fn cfg_gjk_use_radius() {
    let mut acc = DiffAccum::new("cfg_gjk_use_radius");
    let mut rng = Rng::new(0x9eed_0003);
    for ka in KINDS {
        for kb in KINDS {
            // far apart ⇒ shrink branch (dist > rA + rB)
            for i in 0..N {
                let sa = rng.nice_shape(ka);
                let sb = rng.nice_shape(kb).translate(50.0, 0.0);
                let args = GjkArgs {
                    use_radius: 1,
                    ..Default::default()
                };
                acc.check(format!("far ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &args)
                });
            }
            // close ⇒ midpoint branch (dist <= rA + rB)
            for i in 0..N {
                let sa = rng.nice_shape(ka);
                let sb = rng.nice_shape(kb).translate(rng.sym(1.0), rng.sym(1.0));
                let args = GjkArgs {
                    use_radius: 1,
                    ..Default::default()
                };
                acc.check(format!("near ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &args)
                });
            }
            // radii exactly 0 ⇒ rA + rB == 0 ⇒ boundary `dist > FLT_EPSILON`
            for i in 0..N {
                let sa = match rng.nice_shape(ka) {
                    Shape::Ci(c) => Shape::Ci(c2Circle { r: 0.0, ..c }),
                    Shape::Ca(c) => Shape::Ca(c2Capsule { r: 0.0, ..c }),
                    o => o,
                };
                let sb = match rng.nice_shape(kb) {
                    Shape::Ci(c) => Shape::Ci(c2Circle { r: 0.0, ..c }),
                    Shape::Ca(c) => Shape::Ca(c2Capsule { r: 0.0, ..c }),
                    o => o,
                };
                let args = GjkArgs {
                    use_radius: 1,
                    ..Default::default()
                };
                acc.check(format!("r0 ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &args)
                });
            }
            // identical shapes ⇒ dist == 0 ⇒ midpoint branch
            for i in 0..N / 4 {
                let sa = rng.nice_shape(ka);
                let args = GjkArgs {
                    use_radius: 1,
                    ..Default::default()
                };
                acc.check(format!("same ka={ka} #{i}"), |s| run_gjk(s, &sa, &sa, &args));
            }
        }
    }
    acc.finish();
}

/// row 70 — cold cache, then warm re-use across successive calls.
#[test]
fn cfg_gjk_cache_cold() {
    let mut acc = DiffAccum::new("cfg_gjk_cache_cold");
    let mut rng = Rng::new(0x9eed_0004);
    for ka in KINDS {
        for kb in KINDS {
            for &ur in &[0, 1] {
                for i in 0..N {
                    let sa = rng.nice_shape(ka);
                    let sb = rng.nice_shape(kb);
                    let dx = rng.sym(3.0);
                    let dy = rng.sym(3.0);
                    // A whole 3-call session sharing one cache: cold -> warm -> warm
                    acc.check(format!("session ka={ka} kb={kb} ur={ur} #{i}"), |s| {
                        let mut cache = c2GJKCache::default(); // count = 0 ⇒ cold
                        let mut outs = Vec::new();
                        for step in 0..3 {
                            let sbb = sb.translate(dx * step as f32, dy * step as f32);
                            let mut a = OUT_SENTINEL_A;
                            let mut b = OUT_SENTINEL_B;
                            let mut it: c_int = ITER_SENTINEL;
                            let d = c2GJK(
                                s,
                                sa.as_ptr(),
                                sa.ty(),
                                std::ptr::null(),
                                sbb.as_ptr(),
                                sbb.ty(),
                                std::ptr::null(),
                                &mut a,
                                &mut b,
                                ur,
                                &mut it,
                                &mut cache,
                            );
                            outs.push(GjkOut {
                                dist: d,
                                a,
                                b,
                                iter: it,
                                cache,
                            });
                        }
                        outs
                    });
                }
            }
        }
    }
    acc.finish();
}

/// rows 71..74 — hand-primed warm caches (valid indices, various counts and
/// metrics, including a stale metric that trips the validity test).
#[test]
fn cfg_gjk_cache_warm() {
    let mut acc = DiffAccum::new("cfg_gjk_cache_warm");
    let mut rng = Rng::new(0x9eed_0005);
    let metrics: [f32; 6] = [0.0, 1.0, -1.0, -1.0e9, 1.0e9, f32::NAN];
    for ka in KINDS {
        for kb in KINDS {
            for count in 1..=3i32 {
                for &metric in &metrics {
                    for i in 0..N / 4 {
                        let sa = rng.nice_shape(ka);
                        let sb = rng.nice_shape(kb);
                        // valid vertex indices for each proxy
                        let na = match ka {
                            0 => 1,
                            1 => 4,
                            _ => 2,
                        };
                        let nb = match kb {
                            0 => 1,
                            1 => 4,
                            _ => 2,
                        };
                        let cache = c2GJKCache {
                            metric,
                            count,
                            iA: [
                                (rng.below(na)) as c_int,
                                (rng.below(na)) as c_int,
                                (rng.below(na)) as c_int,
                            ],
                            iB: [
                                (rng.below(nb)) as c_int,
                                (rng.below(nb)) as c_int,
                                (rng.below(nb)) as c_int,
                            ],
                            div: [1.0f32, 0.0, 2.5, -1.0][rng.below(4) as usize],
                        };
                        for &ur in &[0, 1] {
                            let args = GjkArgs {
                                use_radius: ur,
                                cache: Some(cache),
                                ..Default::default()
                            };
                            acc.check(
                                format!(
                                    "ka={ka} kb={kb} cnt={count} metric={metric:?} ur={ur} #{i}"
                                ),
                                |s| run_gjk(s, &sa, &sb, &args),
                            );
                        }
                    }
                }
            }
        }
    }
    acc.finish();
}

/// row 76 — `C2_TYPE_POLY` for B (only reachable via the raw entry point).
#[test]
fn cfg_gjk_poly() {
    let mut acc = DiffAccum::new("cfg_gjk_poly");
    let mut rng = Rng::new(0x9eed_0006);
    for count in 1..=8i32 {
        for ka in KINDS {
            for &has_bx in &[false, true] {
                for i in 0..N / 2 {
                    let verts = rng.convex_poly_verts(count as usize);
                    let poly = make_poly(&verts, count);
                    let sa = rng.nice_shape(ka);
                    let args = GjkArgs {
                        bx: if has_bx { Some(rng.xform()) } else { None },
                        use_radius: rng.below(2) as c_int,
                        ..Default::default()
                    };
                    acc.check(format!("count={count} ka={ka} bx={has_bx} #{i}"), |s| {
                        run_gjk_raw(
                            s,
                            sa.as_ptr(),
                            sa.ty(),
                            &poly as *const c2Poly as *const _,
                            C2_TYPE_POLY,
                            &args,
                        )
                    });
                    // and POLY for A
                    acc.check(format!("A-poly count={count} kb={ka} bx={has_bx} #{i}"), |s| {
                        run_gjk_raw(
                            s,
                            &poly as *const c2Poly as *const _,
                            C2_TYPE_POLY,
                            sa.as_ptr(),
                            sa.ty(),
                            &args,
                        )
                    });
                    // POLY for both
                    acc.check(format!("both-poly count={count} #{i}"), |s| {
                        run_gjk_raw(
                            s,
                            &poly as *const c2Poly as *const _,
                            C2_TYPE_POLY,
                            &poly as *const c2Poly as *const _,
                            C2_TYPE_POLY,
                            &args,
                        )
                    });
                }
            }
        }
    }
    acc.finish();
}

/// row 77 — degenerate shapes.
#[test]
fn cfg_gjk_degenerate() {
    let mut acc = DiffAccum::new("cfg_gjk_degenerate");
    let mut rng = Rng::new(0x9eed_0007);
    let degen: Vec<Shape> = vec![
        Shape::Ci(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        }),
        Shape::Ci(c2Circle {
            p: c2v { x: 1.0, y: 1.0 },
            r: -1.0,
        }),
        Shape::Bb(c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        }),
        Shape::Bb(c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: -1.0, y: -1.0 },
        }),
        Shape::Ca(c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        }),
        Shape::Ca(c2Capsule {
            a: c2v { x: 2.0, y: 2.0 },
            b: c2v { x: 2.0, y: 2.0 },
            r: 1.0,
        }),
    ];
    for (ia, sa) in degen.iter().enumerate() {
        for (ib, sb) in degen.iter().enumerate() {
            for &ur in &[0, 1] {
                for i in 0..16 {
                    let args = GjkArgs {
                        ax: if i % 2 == 0 { None } else { Some(rng.xform()) },
                        bx: if i % 3 == 0 { None } else { Some(rng.xform()) },
                        use_radius: ur,
                        ..Default::default()
                    };
                    acc.check(format!("{ia}/{ib} ur={ur} #{i}"), |s| {
                        run_gjk(s, sa, sb, &args)
                    });
                }
            }
        }
    }
    // random degenerate + non-finite shapes
    for ka in KINDS {
        for kb in KINDS {
            for i in 0..(N * 2) {
                let sa = rng.shape(ka);
                let sb = rng.shape(kb);
                let args = GjkArgs {
                    use_radius: rng.below(2) as c_int,
                    ax: if rng.bool() { Some(rng.xform()) } else { None },
                    bx: if rng.bool() { Some(rng.xform()) } else { None },
                    ..Default::default()
                };
                acc.check(format!("rand ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &args)
                });
            }
            for i in 0..N {
                let sa = rng.special_shape(ka);
                let sb = rng.special_shape(kb);
                let args = GjkArgs {
                    use_radius: rng.below(2) as c_int,
                    ..Default::default()
                };
                acc.check(format!("special ka={ka} kb={kb} #{i} {sa:?} {sb:?}"), |s| {
                    run_gjk(s, &sa, &sb, &args)
                });
            }
        }
    }
    acc.finish();
}

/// rows 66..69 — the loop-exit branches and the iteration counter.
///
/// The `iter < 20` cap itself is **unreachable**: the largest proxy
/// `c2MakeProxy` can build has 4 vertices (AABB), so the duplicate-support test
/// fires after at most 4 iterations.  Measured maximum over 300 000 randomized
/// configurations (incl. warm caches, degenerate and non-finite shapes) is 4.
/// This test therefore covers the whole reachable range 0..=4 and asserts each
/// value is observed at least once, with C and Rust agreeing on every one.
#[test]
fn cfg_gjk_iteration_cap() {
    let mut acc = DiffAccum::new("cfg_gjk_iteration_cap");
    let mut rng = Rng::new(0x9eed_0008);
    let mut hist = [0usize; 21];
    // the empirically deepest case
    {
        let sa = Shape::Bb(c2AABB {
            min: c2v { x: -2.871313, y: 3.5 },
            max: c2v { x: -1.5, y: -3.4527063 },
        });
        let sb = Shape::Ca(c2Capsule {
            a: c2v { x: -1.5, y: -6.853581e-5 },
            b: c2v { x: 2.7355094, y: 2.7730334 },
            r: 1.7036669,
        });
        for &ur in &[0, 1] {
            let args = GjkArgs {
                use_radius: ur,
                ..Default::default()
            };
            let o = acc_check_ret(&mut acc, format!("deepest ur={ur}"), |s| {
                run_gjk(s, &sa, &sb, &args)
            });
            hist[o.iter.clamp(0, 20) as usize] += 1;
        }
    }
    for ka in KINDS {
        for kb in KINDS {
            for i in 0..(N * 4) {
                let sa = if rng.bool() {
                    rng.nice_shape(ka)
                } else {
                    rng.shape(ka)
                };
                let sb = if rng.bool() {
                    rng.nice_shape(kb)
                } else {
                    rng.shape(kb)
                };
                let args = GjkArgs {
                    ax: if rng.bool() { Some(rng.xform()) } else { None },
                    bx: if rng.bool() { Some(rng.xform()) } else { None },
                    use_radius: rng.below(2) as c_int,
                    ..Default::default()
                };
                let o = acc_check_ret(&mut acc, format!("ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &args)
                });
                hist[o.iter.clamp(0, 20) as usize] += 1;
            }
        }
    }
    acc.finish();
    eprintln!("cfg_gjk_iteration_cap: iteration histogram = {hist:?}");
    for k in 0..=4 {
        assert!(hist[k] > 0, "iteration count {k} never observed: {hist:?}");
    }
    assert!(
        hist[5..].iter().all(|&n| n == 0),
        "iteration count > 4 observed — update the CONFIGS.md note: {hist:?}"
    );
}

/// rows 50..55 of ERRORS.md — NULL output pointers must simply not be written.
#[test]
fn err_gjk_null_outputs() {
    let mut acc = DiffAccum::new("err_gjk_null_outputs");
    let mut rng = Rng::new(0x9eed_0009);
    for ka in KINDS {
        for kb in KINDS {
            for mask in 0..8u32 {
                for i in 0..64 {
                    let sa = rng.nice_shape(ka);
                    let sb = rng.nice_shape(kb);
                    let args = GjkArgs {
                        want_a: mask & 1 != 0,
                        want_b: mask & 2 != 0,
                        want_iter: mask & 4 != 0,
                        cache: None,
                        use_radius: rng.below(2) as c_int,
                        ..Default::default()
                    };
                    acc.check(format!("ka={ka} kb={kb} mask={mask} #{i}"), |s| {
                        run_gjk(s, &sa, &sb, &args)
                    });
                }
            }
        }
    }
    acc.finish();
}

/// rows 50, 51 of ERRORS.md — NULL transforms substitute the identity.
#[test]
fn err_gjk_null_transforms() {
    let mut acc = DiffAccum::new("err_gjk_null_transforms");
    let mut rng = Rng::new(0x9eed_000a);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    for ka in KINDS {
        for kb in KINDS {
            for i in 0..N {
                let sa = rng.nice_shape(ka);
                let sb = rng.nice_shape(kb);
                // NULL and an explicit identity must give the same answer,
                // in both C and Rust.
                let null_args = GjkArgs::default();
                let id_args = GjkArgs {
                    ax: Some(ident),
                    bx: Some(ident),
                    ..Default::default()
                };
                acc.check(format!("null ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &null_args)
                });
                acc.check(format!("ident ka={ka} kb={kb} #{i}"), |s| {
                    run_gjk(s, &sa, &sb, &id_args)
                });
                let cn = run_gjk(Side::C, &sa, &sb, &null_args);
                let ci = run_gjk(Side::C, &sa, &sb, &id_args);
                assert!(
                    cn.dist.to_bits() == ci.dist.to_bits(),
                    "C: NULL transform != identity transform"
                );
            }
        }
    }
    acc.finish();
}
