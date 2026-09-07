//! Phase B — CONFIGS.md rows 57..66: degenerate shapes, extreme magnitudes and
//! fully unconstrained bit patterns through `c2GJK`.

#![allow(non_snake_case)]

mod common;
use common::gjk::*;
use common::*;

const N: u32 = 400;

fn pairs() -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for &a in TYPES.iter() {
        for &b in TYPES.iter() {
            v.push((a, b));
        }
    }
    v
}

/// `shape_r` with a randomly drawn centre (avoids double mutable borrows).
fn shape_r_rand(rng: &mut Rng, ty: u32, centre_mag: f32, scale: f32, radius: f32) -> Shape {
    let centre = rng.v(centre_mag);
    shape_r(rng, ty, centre, scale, radius)
}

/// A shape of the given type with an explicitly chosen radius.
fn shape_r(rng: &mut Rng, ty: u32, centre: c2v, scale: f32, radius: f32) -> Shape {
    match ty {
        0 => Shape::Circle(c2Circle { p: centre, r: radius }),
        1 => {
            // AABBs always have radius 0 in the C, so `radius` only shifts the extent.
            let hx = scale * (0.2 + rng.unit());
            let hy = scale * (0.2 + rng.unit());
            Shape::Aabb(c2AABB {
                min: c2v { x: centre.x - hx, y: centre.y - hy },
                max: c2v { x: centre.x + hx, y: centre.y + hy },
            })
        }
        _ => {
            let d = c2v { x: rng.sym(scale), y: rng.sym(scale) };
            Shape::Capsule(c2Capsule {
                a: c2v { x: centre.x - d.x, y: centre.y - d.y },
                b: c2v { x: centre.x + d.x, y: centre.y + d.y },
                r: radius,
            })
        }
    }
}

// -------------------------------------------------------------------- row 57
#[test]
fn row57_zero_radius() {
    let mut rng = Rng::new(0x5057);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let ca = rng.v(50.0);
            let cb = rng.v(50.0);
            let a = shape_r(&mut rng, ta, ca, scale, 0.0);
            let b = shape_r(&mut rng, tb, cb, scale, 0.0);
            for ur in [0i32, 1] {
                check(
                    &format!("row57 {}x{} ur={ur} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).radius(ur),
                );
            }
            // also -0.0, which is `<= 0` but has a sign bit
            let a = shape_r(&mut rng, ta, ca, scale, -0.0);
            let b = shape_r(&mut rng, tb, cb, scale, -0.0);
            for ur in [0i32, 1] {
                check(
                    &format!("row57 negzero {}x{} ur={ur} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).radius(ur),
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 58
#[test]
fn row58_negative_radius() {
    let mut rng = Rng::new(0x5058);
    for (ta, tb) in pairs() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let ra = -(0.5 + rng.unit() * 20.0);
            let rb = -(0.5 + rng.unit() * 20.0);
            let a = shape_r_rand(&mut rng, ta, 50.0, scale, ra);
            let b = shape_r_rand(&mut rng, tb, 50.0, scale, rb);
            for ur in [0i32, 1] {
                check(
                    &format!("row58 {}x{} ur={ur} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).radius(ur),
                );
            }
            // huge radii -> rA + rB overflows
            let a = shape_r_rand(&mut rng, ta, 50.0, scale, 1e38);
            let b = shape_r_rand(&mut rng, tb, 50.0, scale, 1e38);
            check(
                &format!("row58 huge {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).radius(1),
            );
            // NaN radius
            let a = shape_r_rand(&mut rng, ta, 50.0, scale, f32::NAN);
            let b = shape_r_rand(&mut rng, tb, 50.0, scale, f32::from_bits(0xFFC0_0055));
            check(
                &format!("row58 nan {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).radius(1),
            );
        }
    }
}

// --------------------------------------------------------------- rows 59, 60
#[test]
fn row59_60_degenerate_aabbs() {
    let mut rng = Rng::new(0x5059);
    for &tb in TYPES.iter() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let p = rng.v(50.0);
            // row 59: min == max (a point box)
            let point = Shape::Aabb(c2AABB { min: p, max: p });
            // row 60: min > max (inverted / reversed winding)
            let hx = 0.5 + rng.unit() * 10.0;
            let hy = 0.5 + rng.unit() * 10.0;
            let inverted = Shape::Aabb(c2AABB {
                min: c2v { x: p.x + hx, y: p.y + hy },
                max: c2v { x: p.x - hx, y: p.y - hy },
            });
            // half-inverted: only one axis reversed
            let half = Shape::Aabb(c2AABB {
                min: c2v { x: p.x + hx, y: p.y - hy },
                max: c2v { x: p.x - hx, y: p.y + hy },
            });
            let other = shape_rand(&mut rng, tb, 50.0, scale);
            for (name, deg) in [("point", point), ("inverted", inverted), ("half", half)] {
                for ur in [0i32, 1] {
                    check(
                        &format!("row59-60 {name} x {} ur={ur} i={i}", ty_name(tb)),
                        &Cfg::new(deg, other).radius(ur),
                    );
                    check(
                        &format!("row59-60 {} x {name} ur={ur} i={i}", ty_name(tb)),
                        &Cfg::new(other, deg).radius(ur),
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------------------- row 61
#[test]
fn row61_point_capsule() {
    let mut rng = Rng::new(0x5061);
    for &tb in TYPES.iter() {
        for i in 0..N {
            let scale = 1.0 + rng.unit() * 10.0;
            let p = rng.v(50.0);
            for r in [0.0f32, 1.0, -1.0] {
                let cap = Shape::Capsule(c2Capsule { a: p, b: p, r });
                let other = shape_rand(&mut rng, tb, 50.0, scale);
                for ur in [0i32, 1] {
                    check(
                        &format!("row61 cap(a==b,r={r}) x {} ur={ur} i={i}", ty_name(tb)),
                        &Cfg::new(cap, other).radius(ur),
                    );
                    check(
                        &format!("row61 {} x cap(a==b,r={r}) ur={ur} i={i}", ty_name(tb)),
                        &Cfg::new(other, cap).radius(ur),
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------------------- row 62
#[test]
fn row62_identical_shapes() {
    let mut rng = Rng::new(0x5062);
    let mut hits = 0u32;
    for &t in TYPES.iter() {
        for i in 0..N * 2 {
            let scale = 1.0 + rng.unit() * 10.0;
            let s = shape_rand(&mut rng, t, 50.0, scale);
            for ur in [0i32, 1] {
                let cfg = Cfg::new(s, s).radius(ur);
                check(&format!("row62 {} ur={ur} i={i}", ty_name(t)), &cfg);
                let (c, _) = apis();
                if f32::from_bits(call(c.c2GJK, &cfg).dist) == 0.0 {
                    hits += 1;
                }
            }
            // and with a warm cache
            let warm = check(
                &format!("row62 cached {} i={i}", ty_name(t)),
                &Cfg::new(s, s).cache(Some(c2GJKCache::default())),
            )
            .unwrap();
            check(
                &format!("row62 cached2 {} i={i}", ty_name(t)),
                &Cfg::new(s, s).cache(Some(warm)),
            );
        }
    }
    assert!(hits > 0, "identical shapes never produced dist == 0");
}

// ---------------------------------------------------------- rows 63, 64, 65
#[test]
fn row63_64_65_magnitudes() {
    let mut rng = Rng::new(0x5063);
    // row 63 tiny, row 64 huge, row 65 mixed
    let regimes: [(&str, f32, f32); 7] = [
        ("tiny", 1e-6, 1e-6),
        ("tinier", 1e-20, 1e-20),
        ("denormal", 1e-40, 1e-40),
        ("huge", 1e18, 1e18),
        ("huger", 1e30, 1e30),
        ("mixed-a", 1e30, 1e-20),
        ("mixed-b", 1e-20, 1e30),
    ];
    for (name, ma, mb) in regimes.iter() {
        for (ta, tb) in pairs() {
            for i in 0..N / 2 {
                let a = shape_rand(&mut rng, ta, *ma, *ma);
                let b = shape_rand(&mut rng, tb, *mb, *mb);
                for ur in [0i32, 1] {
                    check(
                        &format!("row63-65 {name} {}x{} ur={ur} i={i}", ty_name(ta), ty_name(tb)),
                        &Cfg::new(a, b).radius(ur),
                    );
                }
                // with a cache too (same types, so indices stay in range)
                let warm = check(
                    &format!("row63-65 {name} cached {}x{} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).cache(Some(c2GJKCache::default())),
                )
                .unwrap();
                check(
                    &format!("row63-65 {name} cached2 {}x{} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).cache(Some(warm)),
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 66
#[test]
fn row66_unconstrained_bits() {
    let mut rng = Rng::new(0x5066);
    for (ta, tb) in pairs() {
        for i in 0..N * 4 {
            // fully arbitrary bit patterns: NaN / inf / denormal all reachable
            let a = shape_bits(&mut rng, ta);
            let b = shape_bits(&mut rng, tb);
            for ur in [0i32, 1] {
                check(
                    &format!("row66 bits {}x{} ur={ur} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).radius(ur),
                );
            }
            // and the "spicy" mixture, which hits the special values much more
            // often than uniform random bits do
            let a = shape_spicy(&mut rng, ta, 1e3);
            let b = shape_spicy(&mut rng, tb, 1e3);
            let xs = xforms(&mut rng);
            for (nx, xa) in xs.iter() {
                check(
                    &format!("row66 spicy {}x{} ax={nx} i={i}", ty_name(ta), ty_name(tb)),
                    &Cfg::new(a, b).xf(*xa, None),
                );
            }
            // spicy shapes with a warm cache (same types -> indices in range)
            let warm = check(
                &format!("row66 spicy cached {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(c2GJKCache::default())),
            )
            .unwrap();
            let warm = clamp_cache(warm, a.ty(), b.ty());
            check(
                &format!("row66 spicy cached2 {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &Cfg::new(a, b).cache(Some(warm)),
            );
        }
    }
}

/// Row 51 / 64 interaction: force the 20-iteration cap and the `d1 > d0` exit
/// by using shapes whose support function keeps producing fresh vertices.
#[test]
fn row64b_iteration_cap_and_d1_gt_d0() {
    let mut rng = Rng::new(0x5064);
    let (c, _) = apis();
    let mut max_iters = 0;
    for (ta, tb) in pairs() {
        for i in 0..N * 2 {
            let a = shape_spicy(&mut rng, ta, 1e30);
            let b = shape_spicy(&mut rng, tb, 1e30);
            let cfg = Cfg::new(a, b);
            check(
                &format!("row64b {}x{} i={i}", ty_name(ta), ty_name(tb)),
                &cfg,
            );
            if let Some(it) = call(c.c2GJK, &cfg).iters {
                max_iters = max_iters.max(it);
            }
        }
    }
    // The loop must genuinely iterate in at least some configurations.
    assert!(max_iters >= 1, "c2GJK never iterated (max_iters = {max_iters})");
}
