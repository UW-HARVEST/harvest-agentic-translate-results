//! Phase B rows 56–64: the composed pipeline — `c2CastRay` dispatch for all
//! three valid `C2_TYPE` variants, and the public `spec_ray` entry point.
//!
//! These are the rows that per-function tests cannot cover: a bug in how
//! `spec_ray` builds `ray.d` / `ray.t`, or in how `c2CastRay` reinterprets the
//! `const void *B` payload, is invisible when each leaf is tested alone.

mod common;
use common::*;

const N: usize = 20_000;

// ===========================================================================
// c2CastRay — rows 56–59
// ===========================================================================

#[test]
fn row56_cast_ray_circle() {
    let (c, r) = pair();
    let mut d = Diff::new("56: c2CastRay tag=C2_TYPE_CIRCLE");
    let mut g = Rng::new(0x5601);
    for _ in 0..N * 3 {
        let s = c2Circle {
            p: g.v(20.0),
            r: g.unit() * 15.0,
        };
        let ray = c2Ray {
            p: g.v(40.0),
            d: if g.below(2) == 0 { g.dir() } else { g.v(2.0) },
            t: g.unit() * 80.0,
        };
        cmp_cast_ray(&mut d, c, r, ray, as_bytes(&s), C2_TYPE_CIRCLE, "circle");
    }
    d.finish();
}

#[test]
fn row56b_cast_ray_circle_matches_direct_call() {
    // The dispatcher must be a pure forwarder: `c2CastRay(.., CIRCLE, ..)` and
    // `c2RaytoCircle(..)` must agree, in BOTH libraries.
    let (c, r) = pair();
    let mut d = Diff::new("56b: c2CastRay(CIRCLE) == c2RaytoCircle, both libs");
    let mut g = Rng::new(0x56B1);
    for _ in 0..N {
        let s = c2Circle {
            p: g.v(20.0),
            r: g.unit() * 15.0,
        };
        let ray = c2Ray {
            p: g.v(40.0),
            d: g.dir(),
            t: g.unit() * 80.0,
        };
        for lib in [c, r] {
            let mut a = OutBuf::filled();
            let mut b = OutBuf::filled();
            let ra = unsafe { (lib.c2RaytoCircle)(ray, s, a.as_ptr()) };
            let rb = unsafe {
                (lib.c2CastRay)(
                    ray,
                    &s as *const c2Circle as *const std::ffi::c_void,
                    C2_TYPE_CIRCLE,
                    b.as_ptr(),
                )
            };
            d.eq(
                || format!("{} direct-vs-dispatch {}", lib.name, fray(ray)),
                RayResult { ret: ra, out: a },
                RayResult { ret: rb, out: b },
            );
        }
    }
    d.finish();
}

#[test]
fn row57_cast_ray_aabb() {
    let (c, r) = pair();
    let mut d = Diff::new("57: c2CastRay tag=C2_TYPE_AABB");
    let mut g = Rng::new(0x5701);
    for _ in 0..N * 3 {
        let b = g.aabb(20.0);
        let ray = c2Ray {
            p: g.v(40.0),
            d: if g.below(2) == 0 { g.dir() } else { g.v(2.0) },
            t: g.unit() * 80.0,
        };
        cmp_cast_ray(&mut d, c, r, ray, as_bytes(&b), C2_TYPE_AABB, "aabb");
    }
    d.finish();
}

#[test]
fn row58_cast_ray_capsule() {
    let (c, r) = pair();
    let mut d = Diff::new("58: c2CastRay tag=C2_TYPE_CAPSULE");
    let mut g = Rng::new(0x5801);
    for _ in 0..N * 3 {
        let a = g.v(20.0);
        let len = 0.5 + g.unit() * 15.0;
        let u = g.dir();
        let cap = c2Capsule {
            a,
            b: c2v {
                x: a.x + u.x * len,
                y: a.y + u.y * len,
            },
            r: 0.05 + g.unit() * 5.0,
        };
        let ray = c2Ray {
            p: g.v(40.0),
            d: if g.below(2) == 0 { g.dir() } else { g.v(2.0) },
            t: g.unit() * 80.0,
        };
        cmp_cast_ray(&mut d, c, r, ray, as_bytes(&cap), C2_TYPE_CAPSULE, "capsule");
    }
    d.finish();
}

#[test]
fn row59_cast_ray_payload_at_offsets() {
    // The C reads the payload with plain struct loads through a `void *`; the
    // pointer only needs 4-byte alignment for these types. Verify every valid
    // 4-byte-aligned offset inside a larger buffer behaves the same.
    let (c, r) = pair();
    let mut d = Diff::new("59: c2CastRay payload at assorted aligned offsets");
    let mut g = Rng::new(0x5901);

    #[repr(C, align(16))]
    struct Pad([u8; 64]);

    for i in 0..N {
        let s = c2Circle {
            p: g.v(20.0),
            r: g.unit() * 15.0,
        };
        let bx = g.aabb(20.0);
        let a = g.v(20.0);
        let u = g.dir();
        let len = 0.5 + g.unit() * 15.0;
        let cap = c2Capsule {
            a,
            b: c2v {
                x: a.x + u.x * len,
                y: a.y + u.y * len,
            },
            r: 0.05 + g.unit() * 5.0,
        };
        let ray = c2Ray {
            p: g.v(40.0),
            d: g.dir(),
            t: g.unit() * 80.0,
        };
        let (bytes, tag, label): (&[u8], _, _) = match i % 3 {
            0 => (as_bytes(&s), C2_TYPE_CIRCLE, "circle"),
            1 => (as_bytes(&bx), C2_TYPE_AABB, "aabb"),
            _ => (as_bytes(&cap), C2_TYPE_CAPSULE, "capsule"),
        };
        let off = 4 * (i % 6);
        let mut buf = Pad([0x5A; 64]);
        buf.0[off..off + bytes.len()].copy_from_slice(bytes);
        let p = unsafe { buf.0.as_ptr().add(off) } as *const std::ffi::c_void;

        let mut cb = OutBuf::filled();
        let mut rb = OutBuf::filled();
        let cres = RayResult {
            ret: unsafe { (c.c2CastRay)(ray, p, tag, cb.as_ptr()) },
            out: cb,
        };
        let rres = RayResult {
            ret: unsafe { (r.c2CastRay)(ray, p, tag, rb.as_ptr()) },
            out: rb,
        };
        d.eq(
            || format!("offset {off} {label} {}", fray(ray)),
            cres,
            rres,
        );
    }
    d.finish();
}

// ===========================================================================
// spec_ray — rows 60–64
// ===========================================================================

#[test]
fn row60_spec_ray_hits() {
    let (c, r) = pair();
    let mut d = Diff::new("60: spec_ray full pipeline, hitting configurations");
    let mut g = Rng::new(0x6001);
    for _ in 0..N * 4 {
        let centre = g.v(50.0);
        let rad = 0.05 + g.unit() * 20.0;
        // Ray origin somewhere outside; mouse point at or past the circle.
        let u = g.dir();
        let dist = rad * (1.5 + g.unit() * 20.0);
        let origin = c2v {
            x: centre.x - u.x * dist,
            y: centre.y - u.y * dist,
        };
        // mouse point along the same line, at a random fraction/multiple
        let k = dist * (0.2 + g.unit() * 2.5);
        // with a perpendicular jitter so some rays miss
        let n = c2v { x: -u.y, y: u.x };
        let j = g.sym(rad * 2.0);
        let mp = c2v {
            x: origin.x + u.x * k + n.x * j,
            y: origin.y + u.y * k + n.y * j,
        };
        cmp_spec_ray(&mut d, c, r, mp, centre, rad, origin);
    }
    d.finish();
}

#[test]
fn row61_spec_ray_degenerate_geometry() {
    let (c, r) = pair();
    let mut d = Diff::new("61: spec_ray mp on the surface / at the centre / origin inside");
    let mut g = Rng::new(0x6101);
    for i in 0..N * 2 {
        let centre = g.v(50.0);
        let rad = 0.05 + g.unit() * 20.0;
        let ang = g.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: ang.cos(),
            y: ang.sin(),
        };
        match i % 5 {
            // mp exactly on the surface, origin outside
            0 => {
                let mp = c2v {
                    x: centre.x + dir.x * rad,
                    y: centre.y + dir.y * rad,
                };
                let origin = c2v {
                    x: centre.x + dir.x * rad * (2.0 + g.unit() * 5.0),
                    y: centre.y + dir.y * rad * (2.0 + g.unit() * 5.0),
                };
                cmp_spec_ray(&mut d, c, r, mp, centre, rad, origin);
            }
            // mp exactly at the centre
            1 => {
                let origin = c2v {
                    x: centre.x + dir.x * rad * (2.0 + g.unit() * 5.0),
                    y: centre.y + dir.y * rad * (2.0 + g.unit() * 5.0),
                };
                cmp_spec_ray(&mut d, c, r, centre, centre, rad, origin);
            }
            // ray origin strictly inside the circle
            2 => {
                let origin = c2v {
                    x: centre.x + dir.x * rad * g.unit() * 0.9,
                    y: centre.y + dir.y * rad * g.unit() * 0.9,
                };
                cmp_spec_ray(&mut d, c, r, g.v(60.0), centre, rad, origin);
            }
            // mp == ray origin  =>  c2Norm(0,0) = NaN
            3 => {
                let p = g.v(50.0);
                cmp_spec_ray(&mut d, c, r, p, centre, rad, p);
            }
            // ray origin exactly on the surface
            _ => {
                let origin = c2v {
                    x: centre.x + dir.x * rad,
                    y: centre.y + dir.y * rad,
                };
                cmp_spec_ray(&mut d, c, r, g.v(60.0), centre, rad, origin);
            }
        }
    }
    d.finish();
}

#[test]
fn row62_spec_ray_t_boundary() {
    let (c, r) = pair();
    let mut d = Diff::new("62: spec_ray with mp before / at / past the intersection");
    let mut g = Rng::new(0x6201);
    for _ in 0..N * 2 {
        let centre = g.v(40.0);
        let rad = 0.1 + g.unit() * 10.0;
        let u = g.dir();
        let dist = rad * (2.0 + g.unit() * 10.0);
        let origin = c2v {
            x: centre.x - u.x * dist,
            y: centre.y - u.y * dist,
        };
        // The near intersection is at distance (dist - rad) along u.
        let hit = dist - rad;
        for frac in [
            0.25f32, 0.5, 0.9,
            0.999_999, 1.0, 1.000_001,
            1.1, 2.0, 10.0,
        ] {
            let mp = c2v {
                x: origin.x + u.x * hit * frac,
                y: origin.y + u.y * hit * frac,
            };
            cmp_spec_ray(&mut d, c, r, mp, centre, rad, origin);
        }
    }
    d.finish();
}

#[test]
fn row63_spec_ray_quadrants_and_radius_classes() {
    let (c, r) = pair();
    let mut d = Diff::new("63: spec_ray quadrant / axis-aligned geometry, radius classes");
    let mut g = Rng::new(0x6301);
    const RADII: &[f32] = &[
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-6,
        1.0,
        -1.0,
        1e6,
        1e18,
        -1e18,
        f32::MAX,
    ];
    const SIGNS: &[(f32, f32)] = &[
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (-1.0, -1.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (-1.0, 0.0),
        (0.0, -1.0),
    ];
    for i in 0..N * 4 {
        let (sx, sy) = SIGNS[i % SIGNS.len()];
        let m = 1.0 + g.unit() * 100.0;
        let centre = c2v { x: sx * m, y: sy * m };
        let rad = RADII[(i / SIGNS.len()) % RADII.len()];
        let origin = c2v {
            x: g.sym(120.0),
            y: g.sym(120.0),
        };
        let mp = c2v {
            x: g.sym(120.0),
            y: g.sym(120.0),
        };
        cmp_spec_ray(&mut d, c, r, mp, centre, rad, origin);
    }
    d.finish();
}

#[test]
fn row64_spec_ray_hostile() {
    let (c, r) = pair();
    let mut d = Diff::new("64: spec_ray special classes and fully random bit patterns");
    let mut g = Rng::new(0x6401);
    for _ in 0..N * 6 {
        let pick = |g: &mut Rng| -> f32 {
            match g.below(4) {
                0 => g.special_f32(),
                1 => g.any_bits_f32(),
                _ => g.sym(1e3),
            }
        };
        let mp = c2v {
            x: pick(&mut g),
            y: pick(&mut g),
        };
        let cp = c2v {
            x: pick(&mut g),
            y: pick(&mut g),
        };
        let cr = pick(&mut g);
        let rp = c2v {
            x: pick(&mut g),
            y: pick(&mut g),
        };
        cmp_spec_ray(&mut d, c, r, mp, cp, cr, rp);
    }
    // and an exhaustive-ish grid over the special table for every argument slot
    let mut g2 = Rng::new(0x6402);
    for _ in 0..N * 2 {
        let mp = g2.v_special();
        let cp = g2.v_special();
        let cr = g2.special_f32();
        let rp = g2.v_special();
        cmp_spec_ray(&mut d, c, r, mp, cp, cr, rp);
    }
    d.finish();
}
