//! Phase B — CONFIGS.md rows 29..56: `c2GJK` driven end-to-end through both
//! `.so`s across the full cross-product of shape types, transform kinds,
//! `use_radius`, out-pointer nullability and cache state.

#![allow(non_snake_case)]

mod common;
use common::gjk::*;
use common::*;

/// Iterations per (row, type-pair) combination.
const N: u32 = 300;

fn pairs() -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for &a in TYPES.iter() {
        for &b in TYPES.iter() {
            v.push((a, b));
        }
    }
    v
}

/// Rows 29..37: one row per ordered shape-type pair, `use_radius = 1`,
/// NULL transforms, NULL cache; separation swept from far to nested.
fn typed_pair_row(row: u32, ta: u32, tb: u32) {
    let mut rng = Rng::new(0x4000 + row as u64);
    for i in 0..N {
        // separation regimes: far, touching-ish, overlapping, coincident
        let regime = i % 4;
        let scale = 1.0 + rng.unit() * 20.0;
        let ca = rng.v(30.0);
        let cb = match regime {
            0 => c2v { x: ca.x + 200.0 + rng.unit() * 500.0, y: ca.y + rng.sym(200.0) },
            1 => c2v { x: ca.x + scale * 2.0, y: ca.y },
            2 => c2v { x: ca.x + rng.sym(scale), y: ca.y + rng.sym(scale) },
            _ => ca,
        };
        let a = shape_at(&mut rng, ta, ca, scale);
        let b = shape_at(&mut rng, tb, cb, scale);
        let cfg = Cfg::new(a, b);
        check(
            &format!("row{row} {}x{} regime={regime} i={i}", ty_name(ta), ty_name(tb)),
            &cfg,
        );
        // same shapes, but exercised at several magnitudes
        for mag in [1e-4f32, 1e4] {
            let a = shape_at(&mut rng, ta, c2v { x: ca.x * mag, y: ca.y * mag }, scale * mag);
            let b = shape_at(&mut rng, tb, c2v { x: cb.x * mag, y: cb.y * mag }, scale * mag);
            check(
                &format!("row{row} {}x{} mag={mag} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b),
            );
        }
    }
}

macro_rules! typed_row {
    ($name:ident, $row:expr, $ta:expr, $tb:expr) => {
        #[test]
        fn $name() {
            typed_pair_row($row, $ta, $tb);
        }
    };
}

typed_row!(row29_circle_circle, 29, 0, 0);
typed_row!(row30_circle_aabb, 30, 0, 1);
typed_row!(row31_circle_capsule, 31, 0, 2);
typed_row!(row32_aabb_circle, 32, 1, 0);
typed_row!(row33_aabb_aabb, 33, 1, 1);
typed_row!(row34_aabb_capsule, 34, 1, 2);
typed_row!(row35_capsule_circle, 35, 2, 0);
typed_row!(row36_capsule_aabb, 36, 2, 1);
typed_row!(row37_capsule_capsule, 37, 2, 2);

// -------------------------------------------------------------------- row 38
#[test]
fn row38_use_radius_zero() {
    let mut rng = Rng::new(0x4038);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 20.0;
            let ca = rng.v(50.0);
            let cb = rng.v(50.0);
            let a = shape_at(&mut rng, ta, ca, scale);
            let b = shape_at(&mut rng, tb, cb, scale);
            check(
                &format!("row38 {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).radius(0),
            );
        }
    }
}

// --------------------------------------------------------------- rows 39, 40
#[test]
fn row39_40_use_radius_branches() {
    let mut rng = Rng::new(0x4039);
    let mut shrink = 0u32;
    let mut midpoint = 0u32;
    for (ta, tb) in pairs() {
        for i in 0..N {
            // row 39: well separated, both radii positive -> the shrink branch
            let scale = 1.0 + rng.unit() * 5.0;
            let ca = c2v { x: 0.0, y: 0.0 };
            let far = c2v { x: 500.0 + rng.unit() * 1000.0, y: rng.sym(300.0) };
            let a = shape_at(&mut rng, ta, ca, scale);
            let b = shape_at(&mut rng, tb, far, scale);
            let cfg = Cfg::new(a, b).radius(1);
            check(&format!("row39 {}x{} i={i}", ty_name(ta), ty_name(tb)), &cfg);
            let (c, _) = apis();
            if f32::from_bits(call(c.c2GJK, &cfg).dist) > 0.0 {
                shrink += 1;
            }

            // row 40: deliberately overlapping -> the midpoint branch
            let a = shape_at(&mut rng, ta, ca, scale);
            let off = c2v { x: rng.sym(scale * 0.2), y: rng.sym(scale * 0.2) };
            let b = shape_at(&mut rng, tb, off, scale);
            let cfg = Cfg::new(a, b).radius(1);
            check(&format!("row40 {}x{} i={i}", ty_name(ta), ty_name(tb)), &cfg);
            if f32::from_bits(call(c.c2GJK, &cfg).dist) == 0.0 {
                midpoint += 1;
            }
        }
    }
    assert!(shrink > 0 && midpoint > 0, "radius branch coverage: {shrink} / {midpoint}");
}

// -------------------------------------------------------------------- row 41
#[test]
fn row41_use_radius_noncanonical() {
    let mut rng = Rng::new(0x4041);
    for &ur in [2i32, -1, i32::MIN, i32::MAX, 0x100, 7].iter() {
        for (ta, tb) in pairs() {
            for i in 0..40 {
                let scale = 1.0 + rng.unit() * 10.0;
                let a = shape_rand(&mut rng, ta, 50.0, scale);
                let b = shape_rand(&mut rng, tb, 50.0, scale);
                check(
                    &format!("row41 ur={ur} {}x{} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).radius(ur),
                );
            }
        }
    }
}

// --------------------------------------------------------------- rows 42..47
#[test]
fn row42_47_transforms() {
    let mut rng = Rng::new(0x4042);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, ta, 30.0, scale);
            let b = shape_rand(&mut rng, tb, 30.0, scale);
            let xs = xforms(&mut rng);
            for (na, xa) in xs.iter() {
                for (nb, xb) in xs.iter() {
                    for ur in [0i32, 1] {
                        check(
                            &format!(
                                "row42-47 {}x{} ax={na} bx={nb} ur={ur} i={i}",
                                ty_name(ta),
                                ty_name(tb)
                            ),
                            &Cfg::new(a, b).xf(*xa, *xb).radius(ur),
                        );
                    }
                }
            }
        }
    }
}

/// Row 43 specifically: an explicit identity transform must be bit-identical
/// to passing NULL (the C substitutes `c2xIdentity()`).
#[test]
fn row43_null_equals_identity() {
    let mut rng = Rng::new(0x4043);
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let (c, _) = apis();
    for (ta, tb) in pairs() {
        for _ in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, ta, 30.0, scale);
            let b = shape_rand(&mut rng, tb, 30.0, scale);
            for (xa, xb) in [
                (None, None),
                (Some(ident), None),
                (None, Some(ident)),
                (Some(ident), Some(ident)),
            ] {
                let cfg = Cfg::new(a, b).xf(xa, xb);
                check("row43", &cfg);
                // and all four must agree with each other in the C itself
                let base = call(c.c2GJK, &Cfg::new(a, b));
                let got = call(c.c2GJK, &cfg);
                assert_eq!(base, got, "NULL vs explicit identity differ in the C");
            }
        }
    }
}

// --------------------------------------------------------------- rows 48, 49
#[test]
fn row48_49_null_outputs() {
    let mut rng = Rng::new(0x4048);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, ta, 30.0, scale);
            let b = shape_rand(&mut rng, tb, 30.0, scale);
            for (oa, ob, it) in [
                (true, true, true),
                (false, true, true),
                (true, false, true),
                (false, false, true),
                (true, true, false),
                (false, false, false),
            ] {
                check(
                    &format!("row48-49 {}x{} outs={oa}{ob}{it} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).outs(oa, ob, it),
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 50
#[test]
fn row50_cache_null() {
    let mut rng = Rng::new(0x4050);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, ta, 50.0, scale);
            let b = shape_rand(&mut rng, tb, 50.0, scale);
            check(
                &format!("row50 {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(None),
            );
        }
    }
}

// -------------------------------------------------------------------- row 51
#[test]
fn row51_cache_cold() {
    let mut rng = Rng::new(0x4051);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, ta, 50.0, scale);
            let b = shape_rand(&mut rng, tb, 50.0, scale);
            // count == 0 means "cold"; the other fields are junk the C must ignore.
            let cold = c2GJKCache {
                metric: rng.spicy(100.0),
                count: 0,
                iA: [rng.next_u32() as i32, 5, -7],
                iB: [3, rng.next_u32() as i32, 11],
                div: rng.spicy(100.0),
            };
            check(
                &format!("row51 {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(cold)),
            );
        }
    }
}

// -------------------------------------------------------------------- row 52
#[test]
fn row52_cache_warm_same_shapes() {
    let mut rng = Rng::new(0x4052);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, ta, 50.0, scale);
            let b = shape_rand(&mut rng, tb, 50.0, scale);
            let cold = c2GJKCache::default();
            // first call (cold) -> cache written
            let warm = check(
                &format!("row52a {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(cold)),
            )
            .unwrap();
            // second call with the warm cache: this is the `gjk_cache` d0/d1 pattern
            let warm2 = check(
                &format!("row52b {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(warm)),
            )
            .unwrap();
            // and a third, to make sure it has converged identically
            check(
                &format!("row52c {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(warm2)),
            );
        }
    }
}

// -------------------------------------------------------------------- row 53
#[test]
fn row53_cache_after_motion() {
    let mut rng = Rng::new(0x4053);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let ca = rng.v(50.0);
            let a = shape_at(&mut rng, ta, ca, scale);
            let b = shape_rand(&mut rng, tb, 50.0, scale);
            let warm = check(
                &format!("row53a {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(c2GJKCache::default())),
            )
            .unwrap();
            // move the shapes, then reuse the (now stale) cache
            let a2 = shape_rand(&mut rng, ta, 200.0, scale);
            let b2 = shape_rand(&mut rng, tb, 200.0, scale);
            check(
                &format!("row53b {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a2, b2).cache(Some(warm)),
            );
        }
    }
}

// -------------------------------------------------------------------- row 54
#[test]
fn row54_cache_after_type_change() {
    let mut rng = Rng::new(0x4054);
    for (ta, tb) in pairs() {
        for (ua, ub) in pairs() {
            for i in 0..30 {
                let scale = 1.0 + rng.unit() * 10.0;
                let a = shape_rand(&mut rng, ta, 50.0, scale);
                let b = shape_rand(&mut rng, tb, 50.0, scale);
                let warm = check(
                    &format!("row54a {}x{} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).cache(Some(c2GJKCache::default())),
                )
                .unwrap();
                // reuse the cache with DIFFERENT shape types: the cached vertex
                // indices may now exceed the new proxy `count`.
                let a2 = shape_rand(&mut rng, ua, 50.0, scale);
                let b2 = shape_rand(&mut rng, ub, 50.0, scale);
                // The cached vertex indices came from the OLD proxies.  Fold
                // them into the new proxies' initialised range: an index >=
                // the new `count` makes the C read uninitialised stack
                // (ERRORS.md rows 14/15), which has no reproducible behaviour
                // to match.  That case is covered separately in Phase C.
                let warm = clamp_cache(warm, a2.ty(), b2.ty());
                check(
                    &format!(
                        "row54b {}x{} -> {}x{} i={i}",
                        ty_name(ta),
                        ty_name(tb),
                        ty_name(ua),
                        ty_name(ub)
                    ),
                    &Cfg::new(a2, b2).cache(Some(warm)),
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 55
#[test]
fn row55_cache_long_motion_chain() {
    let mut rng = Rng::new(0x4055);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let mut ca = rng.v(20.0);
            let mut cb = c2v { x: ca.x + 100.0, y: ca.y + 40.0 };
            let step = c2v { x: rng.sym(15.0), y: rng.sym(15.0) };
            let mut cache = Some(c2GJKCache::default());
            for k in 0..8 {
                let a = shape_at(&mut rng, ta, ca, scale);
                let b = shape_at(&mut rng, tb, cb, scale);
                cache = check(
                    &format!("row55 {}x{} i={i} step={k}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).cache(cache),
                );
                cb = c2v { x: cb.x - step.x, y: cb.y - step.y };
                ca = c2v { x: ca.x + step.x * 0.25, y: ca.y + step.y * 0.25 };
            }
        }
    }
}

// -------------------------------------------------------------------- row 56
#[test]
fn row56_cache_across_type_pairs_sequence() {
    let mut rng = Rng::new(0x4056);
    let seq = pairs();
    for i in 0..N * 2 {
        let mut cache = Some(c2GJKCache::default());
        for (k, (ta, tb)) in seq.iter().enumerate() {
            let scale = 1.0 + rng.unit() * 10.0;
            let a = shape_rand(&mut rng, *ta, 40.0, scale);
            let b = shape_rand(&mut rng, *tb, 40.0, scale);
            // see row 54: keep the carried-over indices inside the new proxies
            let cache_in = cache.map(|c| clamp_cache(c, a.ty(), b.ty()));
            cache = check(
                &format!("row56 i={i} k={k} {}x{}", ty_name(*ta), ty_name(*tb)),
                &Cfg::new(a, b).cache(cache_in),
            );
        }
    }
}
