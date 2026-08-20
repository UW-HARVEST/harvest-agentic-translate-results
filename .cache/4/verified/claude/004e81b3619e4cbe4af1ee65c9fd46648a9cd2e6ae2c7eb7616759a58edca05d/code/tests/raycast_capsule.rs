//! Phase B — CONFIGS.md rows 49..66 (`c2RaytoCapsule`).
//!
//! `c2RaytoCapsule` writes `*out` *unconditionally* (L260-261) before any
//! branch, so the poison check here asserts that the Rust translation performs
//! the same unconditional write with the same value, including on the paths
//! that then `return 0`.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4096;

fn cmp(A: C2Ray, B: C2Capsule) {
    let (c, r) = (c(), rs());
    for seed in [0x0000_0000u32, 0xffff_ffff, 0x5555_5555] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe { (c.c2RaytoCapsule)(A, B, &mut oc) };
        let rr = unsafe { (r.c2RaytoCapsule)(A, B, &mut orr) };
        assert_eq!(
            rc,
            rr,
            "c2RaytoCapsule return: C={rc} RUST={rr}\n  ray p={} d={} t={}\n  capsule a={} b={} r={}",
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            vshow(B.a),
            vshow(B.b),
            fshow(B.r)
        );
        assert!(
            rceq(oc, orr),
            "c2RaytoCapsule out: C={} RUST={}\n  ray p={} d={} t={}\n  capsule a={} b={} r={}  (poison 0x{seed:08x})",
            rcshow(oc),
            rcshow(orr),
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            vshow(B.a),
            vshow(B.b),
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

fn cap(ax: f32, ay: f32, bx: f32, by: f32, r: f32) -> C2Capsule {
    C2Capsule {
        a: v(ax, ay),
        b: v(bx, by),
        r,
    }
}

/// Vertical capsule from (0,0) to (0,10) with radius 2.
const VCAP: C2Capsule = C2Capsule {
    a: C2v { x: 0.0, y: 0.0 },
    b: C2v { x: 0.0, y: 10.0 },
    r: 2.0,
};

/// Assert that the C library really takes the branch this row is about.
fn expect_c(A: C2Ray, B: C2Capsule, want: i32, what: &str) {
    let c = c();
    let mut o = poison(0);
    let got = unsafe { (c.c2RaytoCapsule)(A, B, &mut o) };
    assert_eq!(got, want, "{what}: C returned {got}, expected {want}");
}

// --- row 49: randomized shotgun -----------------------------------------
#[test]
fn row49_random_shotgun() {
    let mut rng = Rng::new(0x4949);
    for _ in 0..(2 * N) {
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            C2Capsule {
                a: rng.geom_v(),
                b: rng.geom_v(),
                r: rng.geom(),
            },
        );
    }
    for _ in 0..(2 * N) {
        // well-formed capsules with positive radius
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom().abs()),
            C2Capsule {
                a: rng.geom_v(),
                b: rng.geom_v(),
                r: 0.25 + rng.unit(4.0).abs(),
            },
        );
    }
    for _ in 0..N {
        cmp(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            C2Capsule {
                a: rng.wild_v(),
                b: rng.wild_v(),
                r: rng.wild(),
            },
        );
    }
}

// --- rows 50 & 51: shaft crossing from +x and from -x -------------------
#[test]
fn row50_row51_shaft_crossing() {
    // from -x: c == -r  =>  out->n = c2Skew(M.y)
    expect_c(ray(-5.0, 5.0, 1.0, 0.0, 10.0), VCAP, 1, "row50 side hit from -x");
    // from +x: c == +r  =>  out->n = M.x
    expect_c(ray(5.0, 5.0, -1.0, 0.0, 10.0), VCAP, 1, "row51 side hit from +x");
    let mut rng = Rng::new(0x5051);
    for _ in 0..N {
        let y = 0.5 + rng.unit(9.0).abs() % 9.0;
        let x = 3.0 + rng.unit(20.0).abs();
        cmp(ray(-x, y, 1.0, 0.0, x * 2.0), VCAP);
        cmp(ray(x, y, -1.0, 0.0, x * 2.0), VCAP);
        // slightly angled so yAe.y != yAp.y
        cmp(ray(-x, y, 1.0, 0.125, x * 2.0), VCAP);
        cmp(ray(x, y, -1.0, -0.125, x * 2.0), VCAP);
        // radius sweep
        for r in [0.5f32, 1.0, 2.0, 4.0] {
            cmp(ray(-x, y, 1.0, 0.0, x * 2.0), cap(0.0, 0.0, 0.0, 10.0, r));
            cmp(ray(x, y, -1.0, 0.0, x * 2.0), cap(0.0, 0.0, 0.0, 10.0, r));
        }
    }
}

// --- row 52: origin inside the slab -> early return 1 -------------------
#[test]
fn row52_origin_in_slab() {
    expect_c(ray(0.0, 5.0, 1.0, 0.0, 10.0), VCAP, 1, "row52 slab");
    let mut rng = Rng::new(0x5252);
    for _ in 0..N {
        let x = rng.unit(2.0); // |x| <= r
        let y = (rng.unit(1.0).abs()) * 10.0; // 0 <= y <= yBb.y
        cmp(ray(x, y, rng.geom(), rng.geom(), rng.geom()), VCAP);
        // exact slab corners / edges
        cmp(ray(-2.0, 0.0, 1.0, 0.0, 5.0), VCAP);
        cmp(ray(2.0, 10.0, 1.0, 0.0, 5.0), VCAP);
        cmp(ray(2.0, 0.0, 1.0, 0.0, 5.0), VCAP);
        cmp(ray(-2.0, 10.0, 1.0, 0.0, 5.0), VCAP);
        cmp(ray(0.0, 0.0, 1.0, 0.0, 5.0), VCAP);
        cmp(ray(0.0, 10.0, 1.0, 0.0, 5.0), VCAP);
    }
}

// --- rows 53 & 54: origin inside cap A / cap B only ---------------------
#[test]
fn row53_row54_origin_in_caps() {
    expect_c(ray(0.0, -1.0, 1.0, 0.0, 10.0), VCAP, 1, "row53 cap A");
    expect_c(ray(0.0, 11.0, 1.0, 0.0, 10.0), VCAP, 1, "row54 cap B");
    let mut rng = Rng::new(0x5354);
    for _ in 0..N {
        // inside cap A: y < 0 but distance to a < r
        let ang = -rng.unit(1.0).abs() * std::f32::consts::PI; // lower half
        let d = rng.unit(1.0).abs() * 1.99;
        cmp(
            ray(ang.cos() * d, ang.sin() * d, rng.geom(), rng.geom(), rng.geom()),
            VCAP,
        );
        // inside cap B: y > 10 but distance to b < r
        let ang2 = rng.unit(1.0).abs() * std::f32::consts::PI; // upper half
        cmp(
            ray(
                ang2.cos() * d,
                10.0 + ang2.sin() * d,
                rng.geom(),
                rng.geom(),
                rng.geom(),
            ),
            VCAP,
        );
        // exactly on the cap rim (exclusive -> should NOT early-return)
        cmp(ray(0.0, -2.0, 1.0, 0.0, 5.0), VCAP);
        cmp(ray(0.0, 12.0, 1.0, 0.0, 5.0), VCAP);
    }
}

// --- rows 55 & 56: |yAp.x| < r, delegate to circle A / circle B ---------
#[test]
fn row55_row56_band_delegates_to_circles() {
    expect_c(ray(1.0, -5.0, 0.0, 1.0, 10.0), VCAP, 1, "row55 -> circle A");
    expect_c(ray(1.0, 15.0, 0.0, -1.0, 10.0), VCAP, 1, "row56 -> circle B");
    let mut rng = Rng::new(0x5556);
    for _ in 0..N {
        let x = rng.unit(1.99); // |x| < r, outside the caps
        let below = -(3.0 + rng.unit(20.0).abs());
        let above = 10.0 + 3.0 + rng.unit(20.0).abs();
        for t in [0.0f32, 0.5, 1.0, 5.0, 10.0, 100.0] {
            cmp(ray(x, below, 0.0, 1.0, t), VCAP); // yAp.y < 0 -> circle A
            cmp(ray(x, above, 0.0, -1.0, t), VCAP); // yAp.y >= 0 -> circle B
            cmp(ray(x, below, 0.0, -1.0, t), VCAP); // pointing away
            cmp(ray(x, above, 0.0, 1.0, t), VCAP);
            cmp(ray(x, below, 0.25, 1.0, t), VCAP);
        }
        // yAp.y exactly 0 (>= 0 -> circle B) with |x| < r but x outside slab
        cmp(ray(x, 0.0, 1.0, 0.0, 5.0), VCAP);
    }
}

// --- rows 57 & 58: side crossing with y <= 0 / y >= yBb.y ---------------
#[test]
fn row57_row58_side_crossing_off_the_ends() {
    // (-5, 0.5) heading (1,-1): the x == -r crossing happens at y == -2.5 <= 0,
    // so the call is delegated to c2RaytoCircle on cap A, which *hits* at
    // t == 2.5.  The C library is the ground truth here: it returns 1.
    expect_c(ray(-5.0, 0.5, 1.0, -1.0, 10.0), VCAP, 1, "row57 -> circle A");
    // and above yBb.y: delegated to cap B.
    expect_c(ray(-5.0, 9.5, 1.0, 1.0, 10.0), VCAP, 1, "row58 -> circle B");
    let mut rng = Rng::new(0x5758);
    for _ in 0..N {
        let x = 3.0 + rng.unit(10.0).abs();
        // aim so that the x == -r crossing happens below y == 0
        cmp(ray(-x, 0.5, 1.0, -1.0, 10.0), VCAP);
        cmp(ray(-x, 1.0, 1.0, -2.0, 10.0), VCAP);
        cmp(ray(x, 0.5, -1.0, -1.0, 10.0), VCAP);
        // ... and above y == yBb.y
        cmp(ray(-x, 9.5, 1.0, 1.0, 10.0), VCAP);
        cmp(ray(-x, 9.0, 1.0, 2.0, 10.0), VCAP);
        cmp(ray(x, 9.5, -1.0, 1.0, 10.0), VCAP);
        // exactly y == 0 and y == yBb.y at the crossing
        cmp(ray(-4.0, 2.0, 1.0, -1.0, 10.0), VCAP);
        cmp(ray(-4.0, 8.0, 1.0, 1.0, 10.0), VCAP);
        // sweep the entry angle finely to walk across all three sub-branches
        let s = rng.unit(4.0);
        cmp(ray(-6.0, 5.0, 1.0, s, 12.0), VCAP);
        cmp(ray(6.0, 5.0, -1.0, s, 12.0), VCAP);
    }
}

// --- row 59: full miss (outer `if` false) — *out still overwritten -------
#[test]
fn row59_full_miss_out_still_written() {
    let (c, r) = (c(), rs());
    let a = ray(5.0, 5.0, 1.0, 0.0, 10.0);
    expect_c(a, VCAP, 0, "row59 full miss");
    for seed in [0u32, 1, 0xdead_beef, 0xffff_ffff] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe { (c.c2RaytoCapsule)(a, VCAP, &mut oc) };
        let rr = unsafe { (r.c2RaytoCapsule)(a, VCAP, &mut orr) };
        assert_eq!(rc, 0);
        assert_eq!(rc, rr);
        // The C library *does* overwrite *out even on this miss.
        assert!(
            !rceq(oc, poison(seed)),
            "expected the C library to have overwritten *out"
        );
        assert!(
            rceq(oc, orr),
            "out mismatch on a full miss: C={} RUST={}",
            rcshow(oc),
            rcshow(orr)
        );
    }
    let mut rng = Rng::new(0x5959);
    for _ in 0..N {
        let x = 3.0 + rng.unit(20.0).abs();
        cmp(ray(x, 5.0, 1.0, 0.0, 10.0), VCAP);
        cmp(ray(-x, 5.0, -1.0, 0.0, 10.0), VCAP);
        cmp(ray(x, -20.0, 0.0, 1.0, 10.0), VCAP);
    }
}

// --- rows 60 & 61: arbitrary / inverted capsule axes -------------------
#[test]
fn row60_row61_rotated_and_inverted_axes() {
    let mut rng = Rng::new(0x6061);
    for i in 0..64 {
        let ang = (i as f32) * std::f32::consts::TAU / 64.0;
        let (dx, dy) = (ang.cos(), ang.sin());
        let len = 4.0 + (i % 7) as f32;
        let capsule = cap(0.0, 0.0, dx * len, dy * len, 1.5);
        // capsule with a and b swapped (negative yBb.y direction)
        let flipped = cap(dx * len, dy * len, 0.0, 0.0, 1.5);
        for j in 0..16 {
            let a2 = (j as f32) * std::f32::consts::TAU / 16.0;
            let start = 8.0;
            let rr = ray(
                a2.cos() * start,
                a2.sin() * start,
                -a2.cos(),
                -a2.sin(),
                start * 2.0,
            );
            cmp(rr, capsule);
            cmp(rr, flipped);
        }
    }
    for _ in 0..N {
        let a = rng.geom_v();
        let b = rng.geom_v();
        let r0 = 0.25 + rng.unit(4.0).abs();
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom().abs()),
            C2Capsule { a, b, r: r0 },
        );
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom().abs()),
            C2Capsule { a: b, b: a, r: r0 },
        );
    }
    // strictly "downwards" capsule
    let down = cap(0.0, 10.0, 0.0, 0.0, 2.0);
    for y in [-2.0f32, 0.0, 2.0, 5.0, 8.0, 10.0, 12.0] {
        cmp(ray(-5.0, y, 1.0, 0.0, 10.0), down);
        cmp(ray(5.0, y, -1.0, 0.0, 10.0), down);
        cmp(ray(0.0, y, 1.0, 0.0, 10.0), down);
    }
}

// --- row 62: degenerate a == b -----------------------------------------
#[test]
fn row62_degenerate_axis() {
    let mut rng = Rng::new(0x6262);
    for _ in 0..N {
        let p = rng.geom_v();
        let degen = C2Capsule {
            a: p,
            b: p,
            r: rng.geom(),
        };
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            degen,
        );
    }
    for &r0 in [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN].iter() {
        for p in [v(0.0, 0.0), v(-0.0, -0.0), v(1.0, 2.0), v(-3.0, 4.0)] {
            for d in [v(1.0, 0.0), v(0.0, 1.0), v(0.0, 0.0), v(-1.0, -1.0)] {
                for t in [0.0f32, 1.0, 10.0] {
                    cmp(C2Ray { p: v(0.0, 0.0), d, t }, C2Capsule { a: p, b: p, r: r0 });
                    cmp(C2Ray { p: v(-5.0, 0.0), d, t }, C2Capsule { a: p, b: p, r: r0 });
                }
            }
        }
    }
}

// --- row 63: r == 0 / r < 0 -------------------------------------------
#[test]
fn row63_degenerate_radius() {
    let mut rng = Rng::new(0x6363);
    for &r0 in [
        0.0f32,
        -0.0,
        -1.0,
        -5.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ]
    .iter()
    {
        let cc = cap(0.0, 0.0, 0.0, 10.0, r0);
        for y in [-5.0f32, -1.0, 0.0, 1.0, 5.0, 9.0, 10.0, 11.0, 15.0] {
            for x in [-8.0f32, -2.0, -0.5, 0.0, 0.5, 2.0, 8.0] {
                cmp(ray(x, y, 1.0, 0.0, 20.0), cc);
                cmp(ray(x, y, -1.0, 0.0, 20.0), cc);
                cmp(ray(x, y, 0.0, 1.0, 20.0), cc);
                cmp(ray(x, y, 0.0, 0.0, 20.0), cc);
            }
        }
    }
    for _ in 0..N {
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            cap(0.0, 0.0, 0.0, 10.0, -rng.geom().abs()),
        );
    }
}

// --- row 64: yAe.x - yAp.x == 0 (unguarded division) -----------------
#[test]
fn row64_zero_dx_division() {
    // yAd.x == 0 means the ray does not move in the capsule's local x, so
    // `d = yAe.x - yAp.x == 0` and `(c - yAp.x)/d` divides by zero.
    for x in [-8.0f32, -3.0, -2.0, 2.0, 3.0, 8.0] {
        for y in [-5.0f32, 0.0, 5.0, 10.0, 15.0] {
            for t in [0.0f32, 1.0, 10.0, 1.0e30, f32::INFINITY] {
                cmp(ray(x, y, 0.0, 1.0, t), VCAP);
                cmp(ray(x, y, 0.0, -1.0, t), VCAP);
                cmp(ray(x, y, -0.0, 1.0, t), VCAP);
            }
            // A.t == 0 also forces yAe == yAp
            cmp(ray(x, y, 1.0, 0.0, 0.0), VCAP);
            cmp(ray(x, y, 1.0, 0.0, -0.0), VCAP);
            // A.d == 0
            cmp(ray(x, y, 0.0, 0.0, 10.0), VCAP);
        }
    }
}

// --- row 65: A.t and A.d shapes -------------------------------------
#[test]
fn row65_t_and_d_shapes() {
    let ts = [
        0.0f32,
        -0.0,
        -1.0,
        -1.0e30,
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
        v(-0.0, 0.0),
        v(1.0, 0.0),
        v(-1.0, 0.0),
        v(0.0, 1.0),
        v(0.0, -1.0),
        v(1.0, 1.0),
        v(1.0e30, 1.0e30),
        v(f32::INFINITY, 1.0),
        v(f32::NAN, 0.0),
        v(f32::MIN_POSITIVE, 0.0),
    ];
    for &t in ts.iter() {
        for &d in ds.iter() {
            for p in [
                v(-6.0, 5.0),
                v(6.0, 5.0),
                v(0.0, 5.0),
                v(0.0, -6.0),
                v(1.0, -6.0),
                v(1.0, 16.0),
                v(0.0, 0.0),
            ] {
                cmp(C2Ray { p, d, t }, VCAP);
            }
        }
    }
}

// --- row 66: NaN / inf in every input slot -------------------------
#[test]
fn row66_nan_inf_each_slot() {
    let base = C2Ray {
        p: v(-6.0, 5.0),
        d: v(1.0, 0.0),
        t: 12.0,
    };
    for &s in SPECIALS.iter() {
        for slot in 0..10 {
            let mut a = base;
            let mut b = VCAP;
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.d.x = s,
                3 => a.d.y = s,
                4 => a.t = s,
                5 => b.a.x = s,
                6 => b.a.y = s,
                7 => b.b.x = s,
                8 => b.b.y = s,
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
            cap(s, 0.0, 0.0, s, s),
        );
    }
}
