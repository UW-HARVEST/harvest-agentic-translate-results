//! Phase B — valid-path differential tests, rows 48..56 of `CONFIGS.md`
//! (`c2CastRay` in each of its three shape modes, and the `spec_ray`
//! entry point declared in `include/lib.h`).

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 3000;

fn perp(u: C2v) -> C2v {
    v(-u.y, u.x)
}

/* -------------------------------------------------------------- rows 48-50 - */

#[test]
fn row48_castray_circle_mode() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x48_0001);
    let mut d = Diff::new("48: c2CastRay(typeB = C2_TYPE_CIRCLE)");
    for i in 0..N {
        let center = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let rad = rng.range(0.05, 15.0);
        let u = rng.dir();
        let s = rng.range(-2.0, 2.0) * rad;
        let dist = rng.range(-20.0, 60.0);
        let circ = C2Circle {
            p: center,
            r: if i % 16 == 0 { rng.mixed() } else { rad },
        };
        let ray = C2Ray {
            p: vadd(vadd(center, vscale(perp(u), s)), vscale(u, -dist)),
            d: if i % 16 == 1 { rng.v_mixed() } else { u },
            t: if i % 16 == 2 {
                rng.mixed()
            } else {
                dist + 4.0 * rad
            },
        };
        let cc = call_castray_circle(c, ray, circ, C2_TYPE_CIRCLE);
        let cr = call_castray_circle(r, ray, circ, C2_TYPE_CIRCLE);
        d.check_call(
            || format!("{} {}", rayshow(&ray), circshow(&circ)),
            cc,
            cr,
        );
        // dispatch must be identical to calling the specific function
        let direct = call_raytocircle(c, ray, circ);
        d.check(direct.0 == cc.0 && cast_eq_bits(&direct.1, &cc.1), || {
            format!(
                "c2CastRay(CIRCLE) != c2RaytoCircle in the C library for {} {}",
                rayshow(&ray),
                circshow(&circ)
            )
        });
        let direct_r = call_raytocircle(r, ray, circ);
        d.check(direct_r.0 == cr.0 && cast_eq_bits(&direct_r.1, &cr.1), || {
            format!(
                "c2CastRay(CIRCLE) != c2RaytoCircle in the Rust library for {} {}",
                rayshow(&ray),
                circshow(&circ)
            )
        });
    }
    d.require_hits(200);
    d.require_misses(200);
    d.finish();
}

#[test]
fn row49_castray_aabb_mode() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x49_0001);
    let mut d = Diff::new("49: c2CastRay(typeB = C2_TYPE_AABB)");
    for i in 0..N {
        let x0 = rng.range(-40.0, 40.0);
        let y0 = rng.range(-40.0, 40.0);
        let b = C2AABB {
            min: v(x0, y0),
            max: v(x0 + rng.range(0.05, 30.0), y0 + rng.range(0.05, 30.0)),
        };
        let b = if i % 16 == 3 {
            C2AABB {
                min: rng.v_mixed(),
                max: rng.v_mixed(),
            }
        } else {
            b
        };
        let center = v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -rng.range(-10.0, 60.0))),
            d: if i % 16 == 1 { rng.v_mixed() } else { u },
            t: if i % 16 == 2 {
                rng.mixed()
            } else {
                rng.range(0.0, 120.0)
            },
        };
        let cc = call_castray_aabb(c, ray, b, C2_TYPE_AABB);
        let cr = call_castray_aabb(r, ray, b, C2_TYPE_AABB);
        d.check_call(|| format!("{} {}", rayshow(&ray), aabbshow(&b)), cc, cr);
        let direct = call_raytoaabb(c, ray, b);
        d.check(direct.0 == cc.0 && cast_eq_bits(&direct.1, &cc.1), || {
            format!(
                "c2CastRay(AABB) != c2RaytoAABB in the C library for {} {}",
                rayshow(&ray),
                aabbshow(&b)
            )
        });
        let direct_r = call_raytoaabb(r, ray, b);
        d.check(direct_r.0 == cr.0 && cast_eq_bits(&direct_r.1, &cr.1), || {
            format!(
                "c2CastRay(AABB) != c2RaytoAABB in the Rust library for {} {}",
                rayshow(&ray),
                aabbshow(&b)
            )
        });
    }
    d.require_hits(200);
    d.require_misses(200);
    d.finish();
}

#[test]
fn row50_castray_capsule_mode() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x50_0001);
    let mut d = Diff::new("50: c2CastRay(typeB = C2_TYPE_CAPSULE)");
    for i in 0..N {
        let a = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let dir = rng.dir();
        let cap = C2Capsule {
            a,
            b: vadd(a, vscale(dir, rng.range(0.5, 40.0))),
            r: rng.range(0.05, 10.0),
        };
        let cap = if i % 16 == 3 {
            C2Capsule {
                a: rng.v_mixed(),
                b: rng.v_mixed(),
                r: rng.mixed(),
            }
        } else {
            cap
        };
        let center = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -rng.range(-10.0, 60.0))),
            d: if i % 16 == 1 { rng.v_mixed() } else { u },
            t: if i % 16 == 2 {
                rng.mixed()
            } else {
                rng.range(0.0, 120.0)
            },
        };
        let cc = call_castray_capsule(c, ray, cap, C2_TYPE_CAPSULE);
        let cr = call_castray_capsule(r, ray, cap, C2_TYPE_CAPSULE);
        d.check_call(|| format!("{} {}", rayshow(&ray), capshow(&cap)), cc, cr);
        let direct = call_raytocapsule(c, ray, cap);
        d.check(direct.0 == cc.0 && cast_eq_bits(&direct.1, &cc.1), || {
            format!(
                "c2CastRay(CAPSULE) != c2RaytoCapsule in the C library for {} {}",
                rayshow(&ray),
                capshow(&cap)
            )
        });
        let direct_r = call_raytocapsule(r, ray, cap);
        d.check(direct_r.0 == cr.0 && cast_eq_bits(&direct_r.1, &cr.1), || {
            format!(
                "c2CastRay(CAPSULE) != c2RaytoCapsule in the Rust library for {} {}",
                rayshow(&ray),
                capshow(&cap)
            )
        });
    }
    d.require_hits(200);
    d.require_misses(200);
    d.finish();
}

/* -------------------------------------------------------------- rows 51-56 - */

struct SpecArgs {
    mp: C2v,
    cp: C2v,
    cr: f32,
    rp: C2v,
}

fn spec_row(row: &str, seed: u64, min_hits: usize, min_misses: usize, mk: fn(&mut Rng) -> SpecArgs) {
    let (c, r) = apis();
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N {
        let a = mk(&mut rng);
        d.check_call(
            || {
                format!(
                    "spec_ray(mp={}, c_p={}, c_r={}, r_p={})",
                    vshow(a.mp),
                    vshow(a.cp),
                    fshow(a.cr),
                    vshow(a.rp)
                )
            },
            call_spec_ray(c, a.mp.x, a.mp.y, a.cp.x, a.cp.y, a.cr, a.rp.x, a.rp.y),
            call_spec_ray(r, a.mp.x, a.mp.y, a.cp.x, a.cp.y, a.cr, a.rp.x, a.rp.y),
        );
    }
    if min_hits > 0 {
        d.require_hits(min_hits);
    }
    if min_misses > 0 {
        d.require_misses(min_misses);
    }
    d.finish();
}

#[test]
fn row51_spec_ray_ordinary_hit() {
    // mouse point beyond the circle, ray origin outside: `ray.t` reaches the
    // circle, so this is the hit configuration.
    spec_row("51: spec_ray ordinary hit", 0x51_0001, N / 2, 0, |rng| {
        let cp = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let cr = rng.range(0.05, 15.0);
        let u = rng.dir();
        let s = rng.range(-0.9, 0.9) * cr;
        let rp = vadd(vadd(cp, vscale(perp(u), s)), vscale(u, -rng.range(cr + 0.1, 60.0)));
        // mouse point further along the same line, past the circle
        let mp = vadd(vadd(cp, vscale(perp(u), s)), vscale(u, rng.range(cr, cr + 40.0)));
        SpecArgs { mp, cp, cr, rp }
    });
}

#[test]
fn row52_spec_ray_mouse_short_of_or_inside_circle() {
    // `ray.t` is exactly the distance from the ray origin to the mouse point, so
    // a mouse point that stops short of the circle makes the hit fall beyond
    // `A.t` (miss), while a mouse point inside the circle still hits (the entry
    // point comes first).  Both sub-configurations are generated here.
    spec_row(
        "52: spec_ray mouse point short of / inside the circle",
        0x52_0001,
        N / 4,
        N / 4,
        |rng| {
            let cp = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
            let cr = rng.range(0.5, 15.0);
            let u = rng.dir();
            let dist = rng.range(cr + 5.0, 60.0);
            let rp = vadd(cp, vscale(u, -dist));
            let mp = if rng.chance(2) {
                // short of the circle => t > A.t => miss
                vadd(rp, vscale(u, dist - cr - rng.range(0.01, 4.0)))
            } else {
                // inside the circle => still a hit
                vadd(cp, vscale(u, -rng.range(0.05, 0.95) * cr))
            };
            SpecArgs { mp, cp, cr, rp }
        },
    );
}

#[test]
fn row53_spec_ray_origin_inside_circle() {
    spec_row("53: spec_ray ray origin inside the circle", 0x53_0001, 0, N / 2, |rng| {
        let cp = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let cr = rng.range(0.5, 15.0);
        let rp = vadd(cp, vscale(rng.dir(), rng.range(0.0, 0.95) * cr));
        let mp = vadd(cp, vscale(rng.dir(), rng.range(cr, cr + 60.0)));
        SpecArgs { mp, cp, cr, rp }
    });
}

#[test]
fn row54_spec_ray_degenerate_direction() {
    spec_row("54: spec_ray mp == r_p (c2Norm of the zero vector)", 0x54_0001, 0, N / 2, |rng| {
        let cp = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let cr = rng.range(0.05, 15.0);
        let p = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        // exactly equal, and the -0.0 variants
        let mp = match rng.below(3) {
            0 => p,
            1 => v(-p.x * 0.0 + p.x, p.y),
            _ => v(p.x, p.y),
        };
        SpecArgs { mp, cp, cr, rp: p }
    });
}

#[test]
fn row55_spec_ray_radius_shapes() {
    spec_row("55: spec_ray c_r = 0 / negative / huge", 0x55_0001, 0, 0, |rng| {
        let cp = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
        let cr = match rng.below(6) {
            0 => 0.0,
            1 => -0.0,
            2 => -rng.range(0.05, 15.0),
            3 => rng.range(1e30, 3e38),
            4 => f32::from_bits(rng.below(0x0080_0000)),
            _ => rng.range(0.05, 15.0),
        };
        let u = rng.dir();
        let rp = vadd(cp, vscale(u, -rng.range(0.1, 60.0)));
        // mouse point exactly at the centre => the ray line passes through it
        let mp = if rng.chance(2) {
            cp
        } else {
            vadd(cp, vscale(u, rng.range(0.0, 40.0)))
        };
        SpecArgs { mp, cp, cr, rp }
    });
}

#[test]
fn row56_spec_ray_full_noise_fuzz() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x56_0001);
    let mut d = Diff::new("56: spec_ray arbitrary bit patterns");
    for i in 0..N * 4 {
        let f: [f32; 7] = match i % 3 {
            0 => [
                rng.mixed(),
                rng.mixed(),
                rng.mixed(),
                rng.mixed(),
                rng.mixed(),
                rng.mixed(),
                rng.mixed(),
            ],
            1 => [
                rng.special(),
                rng.special(),
                rng.special(),
                rng.special(),
                rng.special(),
                rng.special(),
                rng.special(),
            ],
            _ => [
                rng.any_bits(),
                rng.any_bits(),
                rng.any_bits(),
                rng.any_bits(),
                rng.any_bits(),
                rng.any_bits(),
                rng.any_bits(),
            ],
        };
        d.check_call(
            || {
                format!(
                    "spec_ray({}, {}, {}, {}, {}, {}, {})",
                    fshow(f[0]),
                    fshow(f[1]),
                    fshow(f[2]),
                    fshow(f[3]),
                    fshow(f[4]),
                    fshow(f[5]),
                    fshow(f[6])
                )
            },
            call_spec_ray(c, f[0], f[1], f[2], f[3], f[4], f[5], f[6]),
            call_spec_ray(r, f[0], f[1], f[2], f[3], f[4], f[5], f[6]),
        );
    }
    d.finish();
}
