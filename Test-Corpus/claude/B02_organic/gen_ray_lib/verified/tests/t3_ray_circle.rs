//! Phase B rows B19..B31 and Phase C rows E01..E05 for `c2RaytoCircle`,
//! the lowest-level raycast entry point.

mod common;
use common::*;

fn ray(px: f32, py: f32, dx: f32, dy: f32, t: f32) -> c2Ray {
    c2Ray {
        p: c2v { x: px, y: py },
        d: c2v { x: dx, y: dy },
        t,
    }
}

fn circle(px: f32, py: f32, r: f32) -> c2Circle {
    c2Circle {
        p: c2v { x: px, y: py },
        r,
    }
}

fn both(d: &mut Diff, label: &str, a: c2Ray, b: c2Circle) -> i32 {
    let (c, r) = apis();
    let rc = call_circle(c, a, b);
    let rr = call_circle(r, a, b);
    d.ray(label, || format!("{:?} {:?}", a, b), rc, rr);
    rc.0
}

/// Construct a ray that provably hits `circle(centre, rad)` at distance `len`.
fn hitting_ray(rng: &mut Rng, centre: c2v, rad: f32, len: f32, slack: f32) -> c2Ray {
    let n = rng.unit();
    let impact = c2v {
        x: centre.x + n.x * rad,
        y: centre.y + n.y * rad,
    };
    let mut dir = rng.unit();
    if dir.x * n.x + dir.y * n.y >= 0.0 {
        dir = c2v {
            x: -dir.x,
            y: -dir.y,
        };
    }
    c2Ray {
        p: c2v {
            x: impact.x - dir.x * len,
            y: impact.y - dir.y * len,
        },
        d: dir,
        t: len * slack,
    }
}

/// B19: origin outside, unit direction, generous `A.t` => hit.
#[test]
fn b19_hit_generic() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB19);
    let mut hits = 0;
    for _ in 0..20_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(30.0)).abs() + 1e-3;
        let len = (rng.uniform(50.0)).abs() + 1e-3;
        let a = hitting_ray(&mut rng, centre, rad, len, 1.5);
        hits += both(&mut d, "B19", a, c2Circle { p: centre, r: rad });
    }
    assert!(hits > 15_000, "expected mostly hits, got {hits}");
    d.finish("B19 c2RaytoCircle hit");
}

/// B20 + the `t >= 0` boundary: `t == 0` exactly (origin on the circle).
#[test]
fn b20_t_zero_boundary() {
    let mut d = Diff::new();
    // Exactly representable: t = -b - sqrt(b*b) with b = -1 => t = 0.
    for s in [1.0f32, 2.0, 4.0, 0.5, 1024.0] {
        let a = ray(s, 0.0, -1.0, 0.0, 0.0);
        let b = circle(0.0, 0.0, s);
        assert_eq!(both(&mut d, "B20/exact", a, b), 1, "expected a hit at t == 0");
        // and with A.t < 0 the same t == 0 must be rejected
        let a2 = ray(s, 0.0, -1.0, 0.0, -0.0);
        both(&mut d, "B20/-0.0", a2, b);
        let a3 = ray(s, 0.0, -1.0, 0.0, -1.0);
        assert_eq!(both(&mut d, "B20/negative-At", a3, b), 0);
    }
    // Randomized: put the origin on the circumference, direction inward.
    let mut rng = Rng::new(0xB20);
    for _ in 0..5_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let a = hitting_ray(&mut rng, centre, rad, 0.0, 1.0);
        both(&mut d, "B20/rand", a, c2Circle { p: centre, r: rad });
    }
    d.finish("B20 c2RaytoCircle t == 0");
}

/// B21 + E04: the `t <= A.t` boundary, probed with the exact `t` the C
/// library itself reports, plus one ULP either side.
#[test]
fn b21_e04_t_equals_ray_length() {
    let (c, _r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB21);
    let mut exact_hits = 0;
    for _ in 0..5_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let len = (rng.uniform(20.0)).abs() + 1e-3;
        let probe = hitting_ray(&mut rng, centre, rad, len, 4.0);
        let b = c2Circle { p: centre, r: rad };
        let (hit, out) = call_circle(c, probe, b);
        if hit == 0 || !out.t.is_finite() {
            continue;
        }
        let t = out.t;
        for at in [
            t,
            f32::from_bits(t.to_bits().wrapping_sub(1)), // one ULP below
            f32::from_bits(t.to_bits().wrapping_add(1)), // one ULP above
            t * 0.999_999,
            0.0,
            -0.0,
        ] {
            let a2 = c2Ray { t: at, ..probe };
            exact_hits += both(&mut d, "B21", a2, b);
        }
    }
    assert!(exact_hits > 0, "no boundary hits constructed");
    d.finish("B21/E04 c2RaytoCircle t == A.t boundary");
}

/// B22 + E03: origin strictly inside the circle => `disc > 0` but `t < 0`.
#[test]
fn b22_e03_origin_inside() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB22);
    for _ in 0..20_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(30.0)).abs() + 1e-3;
        let dir = rng.unit();
        let off = rng.unit();
        let f = ((rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32) * 0.99;
        let a = c2Ray {
            p: c2v {
                x: centre.x + off.x * rad * f,
                y: centre.y + off.y * rad * f,
            },
            d: dir,
            t: rad * 10.0,
        };
        let ret = both(&mut d, "B22", a, c2Circle { p: centre, r: rad });
        d.check(ret == 0, || {
            format!("origin inside should miss: {:?} {:?} {}", a, centre, rad)
        });
    }
    d.finish("B22/E03 c2RaytoCircle origin inside");
}

/// B23 + E01: direction pointing away / line missing the circle (`disc < 0`).
#[test]
fn b23_e01_miss() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB23);
    for i in 0..20_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let len = (rng.uniform(20.0)).abs() + rad + 1e-3;
        let n = rng.unit();
        let a = if i % 2 == 0 {
            // pointing away: origin outside, direction = +n (outward)
            c2Ray {
                p: c2v {
                    x: centre.x + n.x * (rad + len),
                    y: centre.y + n.y * (rad + len),
                },
                d: n,
                t: len * 10.0,
            }
        } else {
            // E01: parallel line offset by more than the radius => disc < 0
            let perp = c2v { x: -n.y, y: n.x };
            c2Ray {
                p: c2v {
                    x: centre.x + n.x * (rad * 2.0 + 1.0) - perp.x * len,
                    y: centre.y + n.y * (rad * 2.0 + 1.0) - perp.y * len,
                },
                d: perp,
                t: len * 4.0,
            }
        };
        let ret = both(&mut d, "B23", a, c2Circle { p: centre, r: rad });
        d.check(ret == 0, || format!("should miss: {:?} {:?} {}", a, centre, rad));
    }
    d.finish("B23/E01 c2RaytoCircle miss");
}

/// B24: tangent rays (`disc == 0` exactly, built from powers of two).
#[test]
fn b24_tangent() {
    let mut d = Diff::new();
    for rad in [1.0f32, 2.0, 4.0, 0.25, 64.0] {
        for l in [rad, rad * 2.0, rad * 0.5] {
            // m = (rad, -l): c = rad^2 + l^2 - rad^2 = l^2, b = -l, disc = 0
            let a = ray(rad, -l, 0.0, 1.0, l * 4.0);
            let b = circle(0.0, 0.0, rad);
            both(&mut d, "B24/exact", a, b);
            // near-tangent, both sides
            for eps in [1e-4f32, -1e-4] {
                let a2 = ray(rad + eps, -l, 0.0, 1.0, l * 4.0);
                both(&mut d, "B24/near", a2, b);
            }
        }
    }
    let mut rng = Rng::new(0xB24);
    for _ in 0..5_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let n = rng.unit();
        let perp = c2v { x: -n.y, y: n.x };
        let len = (rng.uniform(10.0)).abs() + 1e-3;
        // origin offset by exactly `rad` from the centre line => tangent
        let a = c2Ray {
            p: c2v {
                x: centre.x + n.x * rad - perp.x * len,
                y: centre.y + n.y * rad - perp.y * len,
            },
            d: perp,
            t: len * 4.0,
        };
        both(&mut d, "B24/rand", a, c2Circle { p: centre, r: rad });
    }
    d.finish("B24 c2RaytoCircle tangent");
}

/// B25 + B26 + E04: `A.t == 0`, `A.t == -0.0`, `A.t < 0`.
#[test]
fn b25_b26_ray_length_zero_negative() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB25);
    for _ in 0..20_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let len = (rng.uniform(20.0)).abs() + 1e-3;
        let probe = hitting_ray(&mut rng, centre, rad, len, 1.0);
        let b = c2Circle { p: centre, r: rad };
        for at in [0.0f32, -0.0, -1.0, -len, f32::NEG_INFINITY] {
            both(&mut d, "B25/B26", c2Ray { t: at, ..probe }, b);
        }
    }
    d.finish("B25/B26 c2RaytoCircle A.t <= 0");
}

/// B27 + E33: radius `0`, `-0.0`, negative, huge, infinite, NaN.
#[test]
fn b27_e33_radius_variants() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB27);
    let radii = [
        0.0f32,
        -0.0,
        1e-30,
        -1.0,
        -1e30,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    for _ in 0..2_000 {
        let centre = rng.vec_nice();
        let base = hitting_ray(&mut rng, centre, 1.0, 5.0, 2.0);
        for &rad in &radii {
            both(&mut d, "B27", base, c2Circle { p: centre, r: rad });
            // ray origin exactly at the centre, too
            let a = c2Ray { p: centre, ..base };
            both(&mut d, "B27/at-centre", a, c2Circle { p: centre, r: rad });
        }
    }
    d.finish("B27/E33 c2RaytoCircle radius variants");
}

/// B28: non-normalized directions (‖d‖ != 1) change the meaning of `t`.
#[test]
fn b28_non_normalized_direction() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB28);
    for _ in 0..20_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let len = (rng.uniform(20.0)).abs() + 1e-3;
        let base = hitting_ray(&mut rng, centre, rad, len, 2.0);
        for scale in [0.1f32, 0.5, 2.0, 100.0, 1e-6, 1e6] {
            let a = c2Ray {
                d: c2v {
                    x: base.d.x * scale,
                    y: base.d.y * scale,
                },
                ..base
            };
            both(&mut d, "B28", a, c2Circle { p: centre, r: rad });
        }
    }
    d.finish("B28 c2RaytoCircle non-normalized direction");
}

/// B29 + E30: zero / NaN direction vectors.
#[test]
fn b29_e30_degenerate_direction() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB29);
    // c2Norm of the zero vector is the C's own way of producing NaN.
    let nan_dir_c = (c.c2Norm)(c2v { x: 0.0, y: 0.0 });
    let nan_dir_r = (r.c2Norm)(c2v { x: 0.0, y: 0.0 });
    d.check(v_eq(nan_dir_c, nan_dir_r), || {
        format!(
            "c2Norm((0,0)): C={} RUST={}",
            fmt_v(nan_dir_c),
            fmt_v(nan_dir_r)
        )
    });
    for _ in 0..5_000 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let p = rng.vec_nice();
        for dir in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            nan_dir_c,
            c2v {
                x: f32::INFINITY,
                y: 0.0,
            },
        ] {
            for t in [0.0f32, 1.0, f32::INFINITY, f32::NAN] {
                let a = c2Ray { p, d: dir, t };
                both(&mut d, "B29", a, c2Circle { p: centre, r: rad });
            }
        }
    }
    d.finish("B29/E30 c2RaytoCircle degenerate direction");
}

/// B30 + E02 + E05: a special value in every individual field position.
#[test]
fn b30_e02_e05_special_in_each_field() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB30);
    for _ in 0..300 {
        let centre = rng.vec_nice();
        let rad = (rng.uniform(10.0)).abs() + 1e-3;
        let base = hitting_ray(&mut rng, centre, rad, 5.0, 2.0);
        let bcirc = c2Circle { p: centre, r: rad };
        for &s in &SPECIALS {
            // each of the 5 ray fields
            both(&mut d, "B30/A.p.x", c2Ray { p: c2v { x: s, ..base.p }, ..base }, bcirc);
            both(&mut d, "B30/A.p.y", c2Ray { p: c2v { y: s, ..base.p }, ..base }, bcirc);
            both(&mut d, "B30/A.d.x", c2Ray { d: c2v { x: s, ..base.d }, ..base }, bcirc);
            both(&mut d, "B30/A.d.y", c2Ray { d: c2v { y: s, ..base.d }, ..base }, bcirc);
            // E05: A.t == NaN is included in SPECIALS
            both(&mut d, "B30/A.t", c2Ray { t: s, ..base }, bcirc);
            // each of the 3 circle fields
            both(&mut d, "B30/B.p.x", base, c2Circle { p: c2v { x: s, ..bcirc.p }, ..bcirc });
            both(&mut d, "B30/B.p.y", base, c2Circle { p: c2v { y: s, ..bcirc.p }, ..bcirc });
            both(&mut d, "B30/B.r", base, c2Circle { r: s, ..bcirc });
        }
    }
    // E02: force `disc` to be NaN via inf - inf while `b` stays finite.
    for a in [
        ray(f32::INFINITY, 0.0, 1.0, 0.0, 10.0),
        ray(0.0, 0.0, 1.0, 0.0, 10.0),
    ] {
        for b in [
            circle(0.0, 0.0, f32::INFINITY),
            circle(f32::INFINITY, 0.0, f32::INFINITY),
            circle(f32::INFINITY, 0.0, 1.0),
        ] {
            both(&mut d, "E02/disc-NaN", a, b);
        }
    }
    d.finish("B30/E02/E05 c2RaytoCircle specials per field");
}

/// B31: unconstrained fuzz, "nice" and "hostile".
#[test]
fn b31_fuzz() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB31);
    for _ in 0..20_000 {
        let a = rng.ray_nice();
        let b = rng.circle_nice();
        both(&mut d, "B31/nice", a, b);
    }
    for _ in 0..20_000 {
        let a = rng.ray_hostile();
        let b = rng.circle_hostile();
        both(&mut d, "B31/hostile", a, b);
    }
    // mixed
    for _ in 0..20_000 {
        let a = rng.ray_nice();
        let b = rng.circle_hostile();
        both(&mut d, "B31/mix1", a, b);
        let a2 = rng.ray_hostile();
        let b2 = rng.circle_nice();
        both(&mut d, "B31/mix2", a2, b2);
    }
    d.finish("B31 c2RaytoCircle fuzz");
}
