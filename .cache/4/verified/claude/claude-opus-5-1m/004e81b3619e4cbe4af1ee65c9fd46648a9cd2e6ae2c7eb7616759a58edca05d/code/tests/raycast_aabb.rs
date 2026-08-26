//! Phase B — CONFIGS.md rows 35..48 (`c2RaytoAABB`).
//!
//! Also exercises the two `static inline` helpers
//! (`c2SignedDistPointToPlane_OneDimensional`, `c2RayToPlane_OneDimensional`)
//! indirectly — they are not exported by the C `.so`, so this is the only way
//! to reach them.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4096;

fn cmp(A: C2Ray, B: C2AABB) {
    let (c, r) = (c(), rs());
    for seed in [0x0000_0000u32, 0xffff_ffff, 0x5555_5555] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe { (c.c2RaytoAABB)(A, B, &mut oc) };
        let rr = unsafe { (r.c2RaytoAABB)(A, B, &mut orr) };
        assert_eq!(
            rc,
            rr,
            "c2RaytoAABB return: C={rc} RUST={rr}\n  ray p={} d={} t={}\n  box min={} max={}",
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            vshow(B.min),
            vshow(B.max)
        );
        assert!(
            rceq(oc, orr),
            "c2RaytoAABB out: C={} RUST={}\n  ray p={} d={} t={}\n  box min={} max={}  (poison 0x{seed:08x})",
            rcshow(oc),
            rcshow(orr),
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            vshow(B.min),
            vshow(B.max)
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

fn bx(minx: f32, miny: f32, maxx: f32, maxy: f32) -> C2AABB {
    C2AABB {
        min: v(minx, miny),
        max: v(maxx, maxy),
    }
}

const UNIT: C2AABB = C2AABB {
    min: C2v { x: -1.0, y: -1.0 },
    max: C2v { x: 1.0, y: 1.0 },
};

// --- row 35: randomized shotgun ------------------------------------------
#[test]
fn row35_random_shotgun() {
    let mut rng = Rng::new(0x3535);
    for _ in 0..N {
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            C2AABB {
                min: rng.geom_v(),
                max: rng.geom_v(),
            },
        );
    }
    for _ in 0..N {
        // well-formed boxes
        let a = rng.geom_v();
        let b = rng.geom_v();
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom().abs()),
            bx(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)),
        );
    }
    for _ in 0..N {
        cmp(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            C2AABB {
                min: rng.wild_v(),
                max: rng.wild_v(),
            },
        );
    }
}

// --- rows 36 & 37: crossing hits through each of the 4 faces --------------
#[test]
fn row36_row37_each_face() {
    let mut rng = Rng::new(0x3637);
    for _ in 0..N {
        let hx = (rng.below(8) as f32 + 1.0) * 0.5;
        let hy = (rng.below(8) as f32 + 1.0) * 0.5;
        let cx = (rng.below(21) as f32 - 10.0) * 0.5;
        let cy = (rng.below(21) as f32 - 10.0) * 0.5;
        let b = bx(cx - hx, cy - hy, cx + hx, cy + hy);
        let far = 4.0 + rng.below(8) as f32;
        // offsets across the face so that the winning axis varies
        for k in 0..5 {
            let fy = cy + (k as f32 - 2.0) * hy * 0.5;
            let fx = cx + (k as f32 - 2.0) * hx * 0.5;
            // -x face  (t0 wins)
            cmp(ray(cx - hx - far, fy, 1.0, 0.0, far * 3.0), b);
            // +x face  (t1 wins)
            cmp(ray(cx + hx + far, fy, -1.0, 0.0, far * 3.0), b);
            // -y face  (t2 wins)
            cmp(ray(fx, cy - hy - far, 0.0, 1.0, far * 3.0), b);
            // +y face  (t3 wins)
            cmp(ray(fx, cy + hy + far, 0.0, -1.0, far * 3.0), b);
        }
    }
}

// --- row 38: exact corner hits (t0..t3 ties) ------------------------------
#[test]
fn row38_corner_ties() {
    // dyadic geometry so the arithmetic is exact and the ties really tie
    for d in [
        v(1.0, 1.0),
        v(-1.0, 1.0),
        v(1.0, -1.0),
        v(-1.0, -1.0),
        v(2.0, 2.0),
        v(0.5, 0.5),
    ] {
        for t in [0.0f32, 1.0, 2.0, 4.0, 8.0, 16.0] {
            for start in [-8.0f32, -4.0, -2.0, 0.0, 2.0, 4.0] {
                cmp(
                    C2Ray {
                        p: v(start, start),
                        d,
                        t,
                    },
                    UNIT,
                );
                cmp(
                    C2Ray {
                        p: v(start, -start),
                        d,
                        t,
                    },
                    UNIT,
                );
            }
        }
    }
    // ray tip exactly on a corner
    cmp(ray(-2.0, -2.0, 1.0, 1.0, 1.0), UNIT);
    cmp(ray(-2.0, -2.0, 1.0, 1.0, 3.0), UNIT);
    cmp(ray(2.0, 2.0, -1.0, -1.0, 1.0), UNIT);
    // symmetric ray through the centre (all four t equal)
    cmp(ray(0.0, 0.0, 1.0, 1.0, 4.0), UNIT);
    cmp(ray(0.0, 0.0, 0.0, 0.0, 4.0), UNIT);
}

// --- row 39: origin inside the box ---------------------------------------
#[test]
fn row39_origin_inside() {
    let mut rng = Rng::new(0x3939);
    for _ in 0..N {
        let px = rng.unit(0.99);
        let py = rng.unit(0.99);
        let ang = rng.unit(std::f32::consts::PI);
        cmp(
            C2Ray {
                p: v(px, py),
                d: v(ang.cos(), ang.sin()),
                t: 1.0 + rng.unit(8.0).abs(),
            },
            UNIT,
        );
        cmp(ray(px, py, 1.0, 0.0, 4.0), UNIT);
        cmp(ray(px, py, 0.0, 1.0, 4.0), UNIT);
        cmp(ray(px, py, -1.0, 0.0, 4.0), UNIT);
        cmp(ray(px, py, 0.0, -1.0, 4.0), UNIT);
        cmp(ray(px, py, 0.0, 0.0, 4.0), UNIT);
    }
}

// --- row 40: swept bb overlaps but SAT rejects (d > 0) -------------------
#[test]
fn row40_sat_reject() {
    // A diagonal ray whose sweep-box covers the unit box but whose supporting
    // line passes outside it.
    for k in 1..40 {
        let off = k as f32 * 0.25;
        cmp(ray(-4.0 - off, -4.0 + off, 1.0, 1.0, 20.0), UNIT);
        cmp(ray(-4.0 + off, -4.0 - off, 1.0, 1.0, 20.0), UNIT);
        cmp(ray(4.0 + off, -4.0 + off, -1.0, 1.0, 20.0), UNIT);
    }
    let mut rng = Rng::new(0x4040);
    for _ in 0..N {
        // random diagonal rays through the neighbourhood of the box
        let s = 3.0 + rng.unit(3.0).abs();
        let off = rng.unit(6.0);
        let ang = rng.unit(std::f32::consts::PI);
        let (dx, dy) = (ang.cos(), ang.sin());
        cmp(
            C2Ray {
                p: v(-dx * s + -dy * off, -dy * s + dx * off),
                d: v(dx, dy),
                t: s * 2.5,
            },
            UNIT,
        );
    }
}

// --- row 41: swept bb misses entirely -----------------------------------
#[test]
fn row41_bb_miss() {
    let mut rng = Rng::new(0x4141);
    for _ in 0..N {
        let far = 3.0 + rng.unit(20.0).abs();
        cmp(ray(-far - 5.0, 0.0, -1.0, 0.0, 4.0), UNIT); // pointing away
        cmp(ray(0.0, far + 5.0, 0.0, 1.0, 4.0), UNIT);
        cmp(ray(far, far, 1.0, 1.0, 4.0), UNIT);
        cmp(ray(-far, 20.0, 1.0, 0.0, far * 4.0), UNIT); // parallel, high above
        cmp(ray(20.0, -far, 0.0, 1.0, far * 4.0), UNIT);
    }
}

// --- row 42 & 43: A.t == 0, A.d == 0 ------------------------------------
#[test]
fn row42_row43_degenerate_ray() {
    let mut rng = Rng::new(0x4243);
    for _ in 0..N {
        let p = rng.geom_v();
        let d = rng.geom_v();
        cmp(C2Ray { p, d, t: 0.0 }, UNIT);
        cmp(C2Ray { p, d, t: -0.0 }, UNIT);
        cmp(
            C2Ray {
                p,
                d: v(0.0, 0.0),
                t: rng.geom(),
            },
            UNIT,
        );
        cmp(
            C2Ray {
                p,
                d: v(-0.0, -0.0),
                t: rng.geom(),
            },
            UNIT,
        );
        cmp(
            C2Ray {
                p,
                d: v(0.0, 0.0),
                t: 0.0,
            },
            C2AABB {
                min: rng.geom_v(),
                max: rng.geom_v(),
            },
        );
    }
}

// --- row 44: degenerate boxes -------------------------------------------
#[test]
fn row44_degenerate_boxes() {
    let mut rng = Rng::new(0x4444);
    let boxes = [
        bx(0.0, 0.0, 0.0, 0.0),                 // point box at origin
        bx(1.0, 2.0, 1.0, 2.0),                 // point box elsewhere
        bx(-1.0, 0.0, 1.0, 0.0),                // horizontal line
        bx(0.0, -1.0, 0.0, 1.0),                // vertical line
        bx(-0.0, -0.0, 0.0, 0.0),               // signed-zero point box
        bx(-1.0e30, -1.0e30, 1.0e30, 1.0e30),   // huge
        bx(f32::MIN, f32::MIN, f32::MAX, f32::MAX),
    ];
    for &b in boxes.iter() {
        for d in [
            v(1.0, 0.0),
            v(0.0, 1.0),
            v(-1.0, 0.0),
            v(0.0, -1.0),
            v(1.0, 1.0),
            v(0.0, 0.0),
        ] {
            for p in [
                v(-4.0, 0.0),
                v(0.0, 0.0),
                v(4.0, 4.0),
                v(1.0, 2.0),
                v(-1.0e20, 0.0),
            ] {
                for t in [0.0f32, 1.0, 10.0, 1.0e30] {
                    cmp(C2Ray { p, d, t }, b);
                }
            }
        }
    }
    for _ in 0..N {
        let p = rng.geom_v();
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            C2AABB { min: p, max: p },
        );
    }
}

// --- row 45: inverted boxes (min > max) --------------------------------
#[test]
fn row45_inverted_boxes() {
    let mut rng = Rng::new(0x4545);
    for _ in 0..N {
        let a = rng.geom_v();
        let b = rng.geom_v();
        let inverted = bx(a.x.max(b.x), a.y.max(b.y), a.x.min(b.x), a.y.min(b.y));
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom().abs()),
            inverted,
        );
        cmp(ray(-10.0, 0.0, 1.0, 0.0, 40.0), inverted);
        cmp(ray(0.0, -10.0, 0.0, 1.0, 40.0), inverted);
    }
    cmp(ray(-5.0, 0.0, 1.0, 0.0, 20.0), bx(1.0, 1.0, -1.0, -1.0));
    cmp(ray(0.0, 0.0, 1.0, 0.0, 20.0), bx(1.0, 1.0, -1.0, -1.0));
}

// --- row 46: box faces exactly on the ray endpoints (da == db guard) ------
#[test]
fn row46_da_equals_db() {
    // da == db happens when p0.x == p1.x (i.e. the ray does not move in x),
    // making `d = da - db == 0` and forcing the `return 0` guard in
    // c2RayToPlane_OneDimensional.
    for y0 in [-4.0f32, -1.0, 0.0, 1.0, 4.0] {
        for t in [0.0f32, 1.0, 2.0, 4.0, 10.0] {
            for x in [-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
                cmp(ray(x, y0, 0.0, 1.0, t), UNIT); // no x movement
                cmp(ray(x, y0, 0.0, -1.0, t), UNIT);
                cmp(ray(y0, x, 1.0, 0.0, t), UNIT); // no y movement
                cmp(ray(y0, x, -1.0, 0.0, t), UNIT);
            }
        }
    }
    // ray endpoints exactly on the faces
    cmp(ray(-1.0, 0.0, 1.0, 0.0, 2.0), UNIT);
    cmp(ray(1.0, 0.0, -1.0, 0.0, 2.0), UNIT);
    cmp(ray(-1.0, -1.0, 1.0, 1.0, 2.0), UNIT);
    cmp(ray(-3.0, 0.0, 1.0, 0.0, 2.0), UNIT); // tip exactly on -x face
    cmp(ray(-3.0, 0.0, 1.0, 0.0, 4.0), UNIT); // tip exactly on +x face
}

// --- row 47: inf / NaN / huge -------------------------------------------
#[test]
fn row47_nan_inf_huge() {
    let base = C2Ray {
        p: v(-4.0, 0.25),
        d: v(1.0, 0.0),
        t: 10.0,
    };
    for &s in SPECIALS.iter() {
        for slot in 0..9 {
            let mut a = base;
            let mut b = UNIT;
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.d.x = s,
                3 => a.d.y = s,
                4 => a.t = s,
                5 => b.min.x = s,
                6 => b.min.y = s,
                7 => b.max.x = s,
                _ => b.max.y = s,
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
            bx(s, -1.0, 1.0, s),
        );
    }
    // overflow in c2Dot
    cmp(ray(-1.0e30, 0.0, 1.0e30, 1.0e30, 1.0e30), UNIT);
    cmp(
        ray(-f32::MAX, -f32::MAX, f32::MAX, f32::MAX, f32::MAX),
        UNIT,
    );
}

// --- row 48: out sentinel on each of the 3 miss paths -------------------
#[test]
fn row48_out_untouched_on_each_miss_path() {
    let (c, r) = (c(), rs());
    let cases: [(&str, C2Ray); 3] = [
        // 1. swept bb misses B
        ("bb miss", ray(-100.0, -100.0, -1.0, -1.0, 1.0)),
        // 2. SAT reject (d > 0)
        ("sat reject", ray(-4.0, 2.5, 1.0, 1.0, 20.0)),
        // 3. hit flags all zero
        ("no hit flags", ray(-4.0, 8.0, 1.0, 0.0, 1.0)),
    ];
    for (name, a) in cases {
        for seed in [0u32, 1, 0xdead_beef, 0xffff_ffff] {
            let mut oc = poison(seed);
            let mut orr = poison(seed);
            let rc = unsafe { (c.c2RaytoAABB)(a, UNIT, &mut oc) };
            let rr = unsafe { (r.c2RaytoAABB)(a, UNIT, &mut orr) };
            assert_eq!(rc, rr, "{name}: C={rc} RUST={rr}");
            assert_eq!(rc, 0, "{name}: expected the C library to miss");
            assert!(rceq(oc, poison(seed)), "{name}: C wrote to *out on a miss");
            assert!(
                rceq(orr, poison(seed)),
                "{name}: RUST wrote to *out on a miss: {}",
                rcshow(orr)
            );
        }
    }
}
