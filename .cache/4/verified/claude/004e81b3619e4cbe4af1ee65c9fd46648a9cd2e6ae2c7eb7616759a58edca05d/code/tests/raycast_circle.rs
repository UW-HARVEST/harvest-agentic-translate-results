//! Phase B — CONFIGS.md rows 25..34 (`c2RaytoCircle`).
//!
//! Every case runs the call twice with two *different* poison patterns in the
//! `c2Raycast` out-parameter, so "the C did not write to `*out`" is verified as
//! part of the differential comparison (row 34) for every single input.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4096;

fn cmp(A: C2Ray, B: C2Circle) {
    let (c, r) = (c(), rs());
    for seed in [0x0000_0000u32, 0xffff_ffff, 0x5555_5555] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe { (c.c2RaytoCircle)(A, B, &mut oc) };
        let rr = unsafe { (r.c2RaytoCircle)(A, B, &mut orr) };
        assert_eq!(
            rc,
            rr,
            "c2RaytoCircle return: C={rc} RUST={rr}\n  ray p={} d={} t={}\n  circle p={} r={}",
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            vshow(B.p),
            fshow(B.r)
        );
        assert!(
            rceq(oc, orr),
            "c2RaytoCircle out: C={} RUST={}\n  ray p={} d={} t={}\n  circle p={} r={}  (poison 0x{seed:08x})",
            rcshow(oc),
            rcshow(orr),
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            vshow(B.p),
            fshow(B.r)
        );
    }
}

fn ray(px: f32, py: f32, dx: f32, dy: f32, t: f32) -> C2Ray {
    C2Ray {
        p: v(px, py),
        d: v(dx, dy),
        t,
    }
}

fn circ(px: f32, py: f32, r: f32) -> C2Circle {
    C2Circle { p: v(px, py), r }
}

// --- row 25: fully randomized shotgun -------------------------------------
#[test]
fn row25_random_shotgun() {
    let mut rng = Rng::new(0x2525);
    for _ in 0..N {
        cmp(
            ray(
                rng.geom(),
                rng.geom(),
                rng.geom(),
                rng.geom(),
                rng.geom().abs(),
            ),
            circ(rng.geom(), rng.geom(), rng.geom()),
        );
    }
    for _ in 0..N {
        cmp(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            C2Circle {
                p: rng.wild_v(),
                r: rng.wild(),
            },
        );
    }
}

// --- row 26: hit (origin outside, aiming at the circle, t big enough) -----
#[test]
fn row26_clean_hit() {
    let mut rng = Rng::new(0x2626);
    for _ in 0..N {
        let cxy = (rng.unit(8.0), rng.unit(8.0));
        let rad = 0.25 + rng.unit(4.0).abs();
        let ang = rng.unit(std::f32::consts::PI);
        let dist = rad + 0.5 + rng.unit(6.0).abs();
        let origin = v(cxy.0 + ang.cos() * dist, cxy.1 + ang.sin() * dist);
        // unit direction pointing at the centre
        let dx = cxy.0 - origin.x;
        let dy = cxy.1 - origin.y;
        let l = (dx * dx + dy * dy).sqrt();
        cmp(
            C2Ray {
                p: origin,
                d: v(dx / l, dy / l),
                t: dist * 2.0,
            },
            circ(cxy.0, cxy.1, rad),
        );
        // axis-aligned variants
        cmp(ray(cxy.0 - dist, cxy.1, 1.0, 0.0, dist * 2.0), circ(cxy.0, cxy.1, rad));
        cmp(ray(cxy.0 + dist, cxy.1, -1.0, 0.0, dist * 2.0), circ(cxy.0, cxy.1, rad));
        cmp(ray(cxy.0, cxy.1 - dist, 0.0, 1.0, dist * 2.0), circ(cxy.0, cxy.1, rad));
        cmp(ray(cxy.0, cxy.1 + dist, 0.0, -1.0, dist * 2.0), circ(cxy.0, cxy.1, rad));
    }
}

// --- row 27: t just too small (reject via t > A.t), and exactly equal -----
#[test]
fn row27_t_boundary() {
    let mut rng = Rng::new(0x2727);
    for _ in 0..N {
        let rad = (rng.below(8) as f32 + 1.0) * 0.5;
        let dist = rad + (rng.below(16) as f32 + 1.0) * 0.5;
        let hit_t = dist - rad; // exact for these dyadic values
        for scale in [
            0.0f32, 0.5, 0.999_999_94, 1.0, 1.000_000_1, 1.5, 2.0,
        ] {
            cmp(ray(-dist, 0.0, 1.0, 0.0, hit_t * scale), circ(0.0, 0.0, rad));
        }
        // t exactly at the far intersection as well
        cmp(ray(-dist, 0.0, 1.0, 0.0, dist + rad), circ(0.0, 0.0, rad));
    }
}

// --- row 28: origin inside the circle (c < 0 -> t < 0) --------------------
#[test]
fn row28_origin_inside() {
    let mut rng = Rng::new(0x2828);
    for _ in 0..N {
        let rad = 1.0 + rng.unit(5.0).abs();
        let inner = rng.unit(1.0).abs() * rad * 0.99;
        let ang = rng.unit(std::f32::consts::PI);
        let p = v(ang.cos() * inner, ang.sin() * inner);
        cmp(
            C2Ray {
                p,
                d: v(ang.cos(), ang.sin()),
                t: 100.0,
            },
            circ(0.0, 0.0, rad),
        );
        // exactly at the centre
        cmp(ray(0.0, 0.0, 1.0, 0.0, 10.0), circ(0.0, 0.0, rad));
        cmp(ray(0.0, 0.0, 0.0, 1.0, 10.0), circ(0.0, 0.0, rad));
    }
}

// --- row 29: circle behind the ray ---------------------------------------
#[test]
fn row29_circle_behind() {
    let mut rng = Rng::new(0x2929);
    for _ in 0..N {
        let rad = 0.5 + rng.unit(3.0).abs();
        let dist = rad + 0.5 + rng.unit(8.0).abs();
        cmp(ray(-dist, 0.0, -1.0, 0.0, 1000.0), circ(0.0, 0.0, rad));
        cmp(ray(dist, 0.0, 1.0, 0.0, 1000.0), circ(0.0, 0.0, rad));
        cmp(ray(0.0, dist, 0.0, 1.0, 1000.0), circ(0.0, 0.0, rad));
        cmp(ray(0.0, -dist, 0.0, -1.0, 1000.0), circ(0.0, 0.0, rad));
    }
}

// --- row 30: tangent rays (disc ~ 0) -------------------------------------
#[test]
fn row30_tangent() {
    let mut rng = Rng::new(0x3030);
    for _ in 0..N {
        let rad = (rng.below(16) as f32 + 1.0) * 0.5;
        // Exactly tangent: offset == r.
        for off in [
            rad,
            rad * 0.999_999_94,
            rad * 1.000_000_1,
            rad - f32::EPSILON,
            rad + f32::EPSILON,
        ] {
            cmp(ray(-10.0, off, 1.0, 0.0, 100.0), circ(0.0, 0.0, rad));
            cmp(ray(-10.0, -off, 1.0, 0.0, 100.0), circ(0.0, 0.0, rad));
            cmp(ray(off, -10.0, 0.0, 1.0, 100.0), circ(0.0, 0.0, rad));
        }
    }
}

// --- row 31: un-normalised / zero / huge directions, odd A.t --------------
#[test]
fn row31_direction_and_t_shapes() {
    let mut rng = Rng::new(0x3131);
    let ts = [
        0.0f32,
        -0.0,
        -1.0,
        -1000.0,
        1.0e-30,
        1.0,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let ds = [
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(1.0, 0.0),
        v(-1.0, 0.0),
        v(0.0, 1.0),
        v(0.0, -1.0),
        v(2.0, 0.0),
        v(0.5, 0.0),
        v(1.0e20, 1.0e20),
        v(1.0e-20, 1.0e-20),
        v(f32::INFINITY, 0.0),
        v(f32::NAN, 0.0),
        v(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
    ];
    for &t in ts.iter() {
        for &d in ds.iter() {
            for rad in [0.5f32, 1.0, 3.0] {
                cmp(C2Ray { p: v(-4.0, 0.0), d, t }, circ(0.0, 0.0, rad));
                cmp(C2Ray { p: v(0.0, 0.0), d, t }, circ(0.0, 0.0, rad));
                cmp(C2Ray { p: v(4.0, 2.0), d, t }, circ(0.0, 0.0, rad));
            }
        }
    }
    for _ in 0..N {
        let d = ds[rng.below(ds.len() as u32) as usize];
        let t = ts[rng.below(ts.len() as u32) as usize];
        cmp(
            C2Ray { p: rng.geom_v(), d, t },
            circ(rng.geom(), rng.geom(), rng.geom()),
        );
    }
}

// --- row 32: degenerate radii --------------------------------------------
#[test]
fn row32_radius_shapes() {
    let rads = [
        0.0f32,
        -0.0,
        -1.0,
        -5.0,
        1.0e-30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &rad in rads.iter() {
        for px in [-4.0f32, -1.0, 0.0, 1.0, 4.0] {
            for py in [-1.0f32, 0.0, 1.0] {
                cmp(ray(px, py, 1.0, 0.0, 10.0), circ(0.0, 0.0, rad));
                cmp(ray(px, py, 0.0, 1.0, 10.0), circ(0.0, 0.0, rad));
                cmp(ray(px, py, 0.0, 0.0, 10.0), circ(0.0, 0.0, rad));
                cmp(ray(px, py, -1.0, -1.0, 10.0), circ(0.0, 0.0, rad));
            }
        }
    }
    // r == 0 and the ray passes exactly through the centre -> c2Norm(0)
    cmp(ray(-3.0, 0.0, 1.0, 0.0, 10.0), circ(0.0, 0.0, 0.0));
    cmp(ray(0.0, -3.0, 0.0, 1.0, 10.0), circ(0.0, 0.0, 0.0));
    cmp(ray(-3.0, -3.0, 1.0, 1.0, 10.0), circ(0.0, 0.0, 0.0));
    cmp(ray(-3.0, 0.0, 1.0, 0.0, 10.0), circ(0.0, 0.0, -0.0));
}

// --- row 33: NaN in each input slot -------------------------------------
#[test]
fn row33_nan_each_slot() {
    let base = C2Ray {
        p: v(-4.0, 0.5),
        d: v(1.0, 0.0),
        t: 10.0,
    };
    let bcirc = circ(0.0, 0.0, 2.0);
    for &s in SPECIALS.iter() {
        for slot in 0..8 {
            let mut a = base;
            let mut b = bcirc;
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.d.x = s,
                3 => a.d.y = s,
                4 => a.t = s,
                5 => b.p.x = s,
                6 => b.p.y = s,
                _ => b.r = s,
            }
            cmp(a, b);
        }
    }
    for &sb in SPECIAL_BITS.iter() {
        let s = f32::from_bits(sb);
        cmp(
            C2Ray {
                p: v(s, s),
                d: v(s, 1.0),
                t: s,
            },
            circ(s, 0.0, s),
        );
    }
}

// --- row 34: explicit "out untouched on miss" (covered for every case) ----
#[test]
fn row34_out_untouched_on_miss() {
    let (c, r) = (c(), rs());
    // disc < 0  => early return, no write
    let a = ray(-10.0, 100.0, 1.0, 0.0, 5.0);
    let b = circ(0.0, 0.0, 1.0);
    for seed in [0u32, 1, 0xdead_beef, 0xffff_ffff] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe { (c.c2RaytoCircle)(a, b, &mut oc) };
        let rr = unsafe { (r.c2RaytoCircle)(a, b, &mut orr) };
        assert_eq!(rc, 0, "expected a miss from the C library");
        assert_eq!(rc, rr);
        assert!(rceq(oc, poison(seed)), "C wrote to *out on a miss?!");
        assert!(
            rceq(orr, poison(seed)),
            "RUST wrote to *out on a miss (C did not): {}",
            rcshow(orr)
        );
    }
}
