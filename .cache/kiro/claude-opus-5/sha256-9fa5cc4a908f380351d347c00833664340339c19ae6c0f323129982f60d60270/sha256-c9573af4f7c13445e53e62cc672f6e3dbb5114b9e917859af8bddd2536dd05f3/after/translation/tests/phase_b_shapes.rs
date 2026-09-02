//! Phase B — valid-path differential tests for the shape/collision entry points.
//!
//! Covers `CONFIGS.md` rows 16..31: `c2CircletoCircle`, `c2CircletoAABB`,
//! `c2CircletoCapsule`. Each row runs many randomized inputs from the fixed
//! project seed plus targeted boundary/degenerate values, and compares the raw
//! `int` returned by both `.so`s.

#![allow(non_snake_case)]

mod common;
use common::*;

fn specials() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    v.extend(SPECIAL_BITS.iter().map(|&b| f32::from_bits(b)));
    v
}

macro_rules! cmp_cc {
    ($c:expr, $r:expr, $A:expr, $B:expr, $($ctx:tt)*) => {{
        let (cv, rv) = unsafe { (($c.c2CircletoCircle)($A, $B), ($r.c2CircletoCircle)($A, $B)) };
        diff_assert!(cv == rv, "{} c2CircletoCircle(A={:?}, B={:?}): C={} RS={}",
                     format!($($ctx)*), $A, $B, cv, rv);
        cv
    }};
}
macro_rules! cmp_ca {
    ($c:expr, $r:expr, $A:expr, $B:expr, $($ctx:tt)*) => {{
        let (cv, rv) = unsafe { (($c.c2CircletoAABB)($A, $B), ($r.c2CircletoAABB)($A, $B)) };
        diff_assert!(cv == rv, "{} c2CircletoAABB(A={:?}, B={:?}): C={} RS={}",
                     format!($($ctx)*), $A, $B, cv, rv);
        cv
    }};
}
macro_rules! cmp_cp {
    ($c:expr, $r:expr, $A:expr, $B:expr, $($ctx:tt)*) => {{
        let (cv, rv) = unsafe { (($c.c2CircletoCapsule)($A, $B), ($r.c2CircletoCapsule)($A, $B)) };
        diff_assert!(cv == rv, "{} c2CircletoCapsule(A={:?}, B={:?}): C={} RS={}",
                     format!($($ctx)*), $A, $B, cv, rv);
        cv
    }};
}

// ===========================================================================
// Rows 16..19 — c2CircletoCircle
// ===========================================================================

#[test]
fn row16_circle_circle_random() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(16);
    let mut hits = 0usize;
    for i in 0..4096 {
        let (a, b) = (rng.circle(), rng.circle());
        hits += cmp_cc!(c, r, a, b, "row16 #{i}") as usize;
    }
    // Sanity: the row must exercise both outcomes, not just "always disjoint".
    assert!(hits > 0 && hits < 4096, "row16 degenerate coverage: {hits}/4096");
}

#[test]
fn row17_circle_circle_grazing_boundary() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(17);
    for i in 0..2048 {
        let ra = rng.radius();
        let rb = rng.radius();
        let sum = ra + rb;
        let ang = rng.unit() * std::f32::consts::TAU;
        // Sweep the separation through exactly `ra+rb` and its neighbours.
        let scale = match i % 5 {
            0 => 1.0f32,                                  // exactly touching
            1 => f32::from_bits(1.0f32.to_bits() - 1),    // 1 ULP inside
            2 => f32::from_bits(1.0f32.to_bits() + 1),    // 1 ULP outside
            3 => 1.0 - 1e-6,
            _ => 1.0 + 1e-6,
        };
        let d = sum * scale;
        let center = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        let A = c2Circle { p: center, r: ra };
        let B = c2Circle {
            p: c2v {
                x: center.x + d * ang.cos(),
                y: center.y + d * ang.sin(),
            },
            r: rb,
        };
        cmp_cc!(c, r, A, B, "row17 #{i} scale={scale}");
        // Exact coincidence: d2 == 0.
        let B0 = c2Circle { p: center, r: rb };
        cmp_cc!(c, r, A, B0, "row17 #{i} coincident");
    }
}

#[test]
fn row18_circle_circle_zero_and_negative_radius() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(18);
    for i in 0..2048 {
        let p = rng.vec_coord();
        let q = rng.vec_coord();
        let radii = [
            0.0f32,
            -0.0,
            -rng.radius(),
            rng.radius(),
            -1.0e-30,
            f32::MIN_POSITIVE,
        ];
        for &ra in &radii {
            for &rb in &radii {
                cmp_cc!(
                    c,
                    r,
                    c2Circle { p, r: ra },
                    c2Circle { p: q, r: rb },
                    "row18 #{i}"
                );
                // Same position too (d2 == 0), where sign of r decides nothing
                // because r2 = (ra+rb)^2 >= 0 but may be exactly 0.
                cmp_cc!(
                    c,
                    r,
                    c2Circle { p, r: ra },
                    c2Circle { p, r: rb },
                    "row18 #{i} same-pos"
                );
            }
        }
    }
}

#[test]
fn row19_circle_circle_special_floats() {
    let (c, r) = libs();
    let sp = specials();
    for &u in &sp {
        for &v in &sp {
            let A = c2Circle {
                p: c2v { x: u, y: v },
                r: u,
            };
            let B = c2Circle {
                p: c2v { x: v, y: u },
                r: v,
            };
            cmp_cc!(c, r, A, B, "row19");
            // huge radii => (A.r+B.r)^2 overflows to inf
            let A2 = c2Circle {
                p: c2v { x: u, y: v },
                r: f32::MAX,
            };
            let B2 = c2Circle {
                p: c2v { x: v, y: u },
                r: f32::MAX,
            };
            cmp_cc!(c, r, A2, B2, "row19 overflow");
        }
    }
}

#[test]
fn row16b_circle_circle_random_raw_bits() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(160);
    for i in 0..300_000u64 {
        let A = c2Circle {
            p: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        let B = c2Circle {
            p: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        cmp_cc!(c, r, A, B, "row16b #{i}");
    }
}

// ===========================================================================
// Rows 20..24 — c2CircletoAABB
// ===========================================================================

#[test]
fn row20_circle_aabb_proper() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(20);
    let mut hits = 0usize;
    for i in 0..4096 {
        let bb = rng.aabb_proper();
        // Deliberately aim at inside / edge / corner / outside.
        let p = match i % 4 {
            0 => c2v {
                // inside
                x: bb.min.x + (bb.max.x - bb.min.x) * rng.unit(),
                y: bb.min.y + (bb.max.y - bb.min.y) * rng.unit(),
            },
            1 => c2v {
                // on an edge
                x: bb.min.x,
                y: bb.min.y + (bb.max.y - bb.min.y) * rng.unit(),
            },
            2 => bb.max, // exactly a corner
            _ => rng.vec_coord(),
        };
        let A = c2Circle { p, r: rng.radius() };
        hits += cmp_ca!(c, r, A, bb, "row20 #{i}") as usize;
    }
    assert!(hits > 0 && hits < 4096, "row20 degenerate coverage: {hits}/4096");
}

#[test]
fn row21_circle_aabb_degenerate_box() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(21);
    for i in 0..2048 {
        let p0 = rng.vec_coord();
        let point = c2AABB { min: p0, max: p0 };
        let flat_x = c2AABB {
            min: p0,
            max: c2v {
                x: p0.x,
                y: p0.y + rng.radius(),
            },
        };
        let flat_y = c2AABB {
            min: p0,
            max: c2v {
                x: p0.x + rng.radius(),
                y: p0.y,
            },
        };
        let A = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        cmp_ca!(c, r, A, point, "row21 #{i} point");
        cmp_ca!(c, r, A, flat_x, "row21 #{i} flat-x");
        cmp_ca!(c, r, A, flat_y, "row21 #{i} flat-y");
        // circle centre exactly at the degenerate box => d2 == 0
        let A0 = c2Circle { p: p0, r: rng.radius() };
        cmp_ca!(c, r, A0, point, "row21 #{i} coincident");
    }
}

#[test]
fn row22_circle_aabb_inverted_box() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(22);
    for i in 0..2048 {
        let bb = rng.aabb_proper();
        let both = c2AABB {
            min: bb.max,
            max: bb.min,
        };
        let only_x = c2AABB {
            min: c2v {
                x: bb.max.x,
                y: bb.min.y,
            },
            max: c2v {
                x: bb.min.x,
                y: bb.max.y,
            },
        };
        let only_y = c2AABB {
            min: c2v {
                x: bb.min.x,
                y: bb.max.y,
            },
            max: c2v {
                x: bb.max.x,
                y: bb.min.y,
            },
        };
        let A = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        cmp_ca!(c, r, A, both, "row22 #{i} both-axes");
        cmp_ca!(c, r, A, only_x, "row22 #{i} x-only");
        cmp_ca!(c, r, A, only_y, "row22 #{i} y-only");
    }
}

#[test]
fn row23_circle_aabb_unbounded_and_special() {
    let (c, r) = libs();
    let sp = specials();
    let inf = f32::INFINITY;
    for &u in &sp {
        for &v in &sp {
            let A = c2Circle {
                p: c2v { x: u, y: v },
                r: v,
            };
            let boxes = [
                // unbounded
                c2AABB {
                    min: c2v { x: -inf, y: -inf },
                    max: c2v { x: inf, y: inf },
                },
                // reversed unbounded
                c2AABB {
                    min: c2v { x: inf, y: inf },
                    max: c2v { x: -inf, y: -inf },
                },
                // NaN bounds
                c2AABB {
                    min: c2v { x: f32::NAN, y: v },
                    max: c2v { x: u, y: f32::NAN },
                },
                // fully special bounds
                c2AABB {
                    min: c2v { x: u, y: v },
                    max: c2v { x: v, y: u },
                },
            ];
            for (k, bb) in boxes.iter().enumerate() {
                cmp_ca!(c, r, A, *bb, "row23 box{k}");
                cmp_ca!(
                    c,
                    r,
                    c2Circle {
                        p: c2v { x: u, y: v },
                        r: 0.0
                    },
                    *bb,
                    "row23 box{k} r=0"
                );
                cmp_ca!(
                    c,
                    r,
                    c2Circle {
                        p: c2v { x: u, y: v },
                        r: -1.0
                    },
                    *bb,
                    "row23 box{k} r<0"
                );
            }
        }
    }
}

#[test]
fn row24_circle_aabb_grazing_boundary() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(24);
    for i in 0..2048 {
        let bb = rng.aabb_proper();
        let rad = rng.radius();
        // Place the centre exactly `rad * scale` to the left of min.x, on a row
        // inside the box, so the clamp point is (min.x, p.y) and d2 == (dx)^2.
        let scale = match i % 5 {
            0 => 1.0f32,
            1 => f32::from_bits(1.0f32.to_bits() - 1),
            2 => f32::from_bits(1.0f32.to_bits() + 1),
            3 => 1.0 - 1e-6,
            _ => 1.0 + 1e-6,
        };
        let p = c2v {
            x: bb.min.x - rad * scale,
            y: bb.min.y + (bb.max.y - bb.min.y) * rng.unit(),
        };
        cmp_ca!(c, r, c2Circle { p, r: rad }, bb, "row24 #{i} scale={scale}");
        // And exactly on the corner diagonal.
        let pc = c2v {
            x: bb.min.x - rad * scale * std::f32::consts::FRAC_1_SQRT_2,
            y: bb.min.y - rad * scale * std::f32::consts::FRAC_1_SQRT_2,
        };
        cmp_ca!(c, r, c2Circle { p: pc, r: rad }, bb, "row24 #{i} corner");
    }
}

#[test]
fn row20b_circle_aabb_random_raw_bits() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(200);
    for i in 0..300_000u64 {
        let A = c2Circle {
            p: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        let bb = c2AABB {
            min: rng.vec_raw(),
            max: rng.vec_raw(),
        };
        cmp_ca!(c, r, A, bb, "row20b #{i}");
    }
}

// ===========================================================================
// Rows 25..31 — c2CircletoCapsule
// ===========================================================================

/// Which of the three arms of `c2CircletoCapsule` a given input takes,
/// recomputed independently from the C source's algebra.
fn capsule_arm(A: &c2Circle, B: &c2Capsule) -> u8 {
    let n = c2v {
        x: B.b.x - B.a.x,
        y: B.b.y - B.a.y,
    };
    let ap = c2v {
        x: A.p.x - B.a.x,
        y: A.p.y - B.a.y,
    };
    let da = ap.x * n.x + ap.y * n.y;
    if da < 0.0 {
        return 1;
    }
    let bp = c2v {
        x: A.p.x - B.b.x,
        y: A.p.y - B.b.y,
    };
    let db = bp.x * n.x + bp.y * n.y;
    if db < 0.0 {
        2
    } else {
        3
    }
}

/// Build a horizontal capsule from (0,0) to (len,0), translated by `off`.
fn horiz_capsule(off: c2v, len: f32, rad: f32) -> c2Capsule {
    c2Capsule {
        a: off,
        b: c2v {
            x: off.x + len,
            y: off.y,
        },
        r: rad,
    }
}

#[test]
fn row25_capsule_arm1_before_a() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(25);
    for i in 0..2048 {
        let off = rng.vec_coord();
        let len = 1.0 + rng.unit() * 100.0;
        let cap = horiz_capsule(off, len, rng.radius());
        // t < 0 along the segment => da < 0
        let t = -(0.001 + rng.unit() * 10.0);
        let A = c2Circle {
            p: c2v {
                x: off.x + t * len,
                y: off.y + rng.sym(20.0),
            },
            r: rng.radius(),
        };
        assert_eq!(capsule_arm(&A, &cap), 1, "row25 #{i} wrong arm");
        cmp_cp!(c, r, A, cap, "row25 #{i}");
    }
}

#[test]
fn row26_capsule_arm2_projection() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(26);
    for i in 0..2048 {
        let off = rng.vec_coord();
        let len = 1.0 + rng.unit() * 100.0;
        let cap = horiz_capsule(off, len, rng.radius());
        // 0 <= t < 1  => da >= 0 && db < 0  (the `da / dot(n,n)` arm)
        let t = rng.unit() * 0.999;
        let A = c2Circle {
            p: c2v {
                x: off.x + t * len,
                y: off.y + rng.sym(20.0),
            },
            r: rng.radius(),
        };
        assert_eq!(capsule_arm(&A, &cap), 2, "row26 #{i} wrong arm");
        cmp_cp!(c, r, A, cap, "row26 #{i}");
    }
}

#[test]
fn row27_capsule_arm3_past_b() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(27);
    for i in 0..2048 {
        let off = rng.vec_coord();
        let len = 1.0 + rng.unit() * 100.0;
        let cap = horiz_capsule(off, len, rng.radius());
        // t >= 1 => da >= 0 && db >= 0
        let t = 1.0 + rng.unit() * 10.0;
        let A = c2Circle {
            p: c2v {
                x: off.x + t * len,
                y: off.y + rng.sym(20.0),
            },
            r: rng.radius(),
        };
        assert_eq!(capsule_arm(&A, &cap), 3, "row27 #{i} wrong arm");
        cmp_cp!(c, r, A, cap, "row27 #{i}");
    }
}

#[test]
fn row28_capsule_random_all_arms() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(28);
    let mut arms = [0usize; 4];
    let mut hits = 0usize;
    for i in 0..4096 {
        let A = rng.circle();
        let B = rng.capsule();
        arms[capsule_arm(&A, &B) as usize] += 1;
        hits += cmp_cp!(c, r, A, B, "row28 #{i}") as usize;
    }
    // Exact-boundary cases: da == 0 and db == 0.
    for i in 0..1024 {
        let off = rng.vec_coord();
        let len = 1.0 + rng.unit() * 100.0;
        let cap = horiz_capsule(off, len, rng.radius());
        // exactly at a  => da == 0 (not < 0) => arm 2
        let Aa = c2Circle {
            p: c2v {
                x: off.x,
                y: off.y + rng.sym(5.0),
            },
            r: rng.radius(),
        };
        cmp_cp!(c, r, Aa, cap, "row28 da==0 #{i}");
        // exactly at b  => db == 0 (not < 0) => arm 3
        let Ab = c2Circle {
            p: c2v {
                x: off.x + len,
                y: off.y + rng.sym(5.0),
            },
            r: rng.radius(),
        };
        cmp_cp!(c, r, Ab, cap, "row28 db==0 #{i}");
    }
    assert!(
        arms[1] > 100 && arms[2] > 100 && arms[3] > 100,
        "row28 arm coverage too thin: {arms:?}"
    );
    assert!(hits > 0, "row28 never reported a collision");
}

#[test]
fn row29_capsule_degenerate_zero_length() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(29);
    for i in 0..2048 {
        let p0 = rng.vec_coord();
        // a == b  =>  n == (0,0)  =>  dot(n,n) == 0  =>  da/0 is unguarded.
        let cap = c2Capsule {
            a: p0,
            b: p0,
            r: rng.radius(),
        };
        let A = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        cmp_cp!(c, r, A, cap, "row29 #{i} a==b");
        // circle centre exactly on the degenerate point: ap == (0,0), da == 0
        let A0 = c2Circle { p: p0, r: rng.radius() };
        cmp_cp!(c, r, A0, cap, "row29 #{i} coincident");
        // a == b but with -0.0 components, so n = -0.0 - -0.0 etc.
        let capz = c2Capsule {
            a: c2v { x: -0.0, y: -0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: rng.radius(),
        };
        cmp_cp!(c, r, A, capz, "row29 #{i} signed-zero segment");
    }
}

#[test]
fn row30_capsule_axis_aligned_and_zero_radius() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(30);
    for i in 0..2048 {
        let off = rng.vec_coord();
        let len = 1.0 + rng.unit() * 100.0;
        let caps = [
            // horizontal
            horiz_capsule(off, len, rng.radius()),
            // vertical
            c2Capsule {
                a: off,
                b: c2v {
                    x: off.x,
                    y: off.y + len,
                },
                r: rng.radius(),
            },
            // zero radius
            horiz_capsule(off, len, 0.0),
            // negative radius
            horiz_capsule(off, len, -rng.radius()),
            // reversed direction
            c2Capsule {
                a: c2v {
                    x: off.x + len,
                    y: off.y,
                },
                b: off,
                r: rng.radius(),
            },
        ];
        for (k, cap) in caps.iter().enumerate() {
            for &ar in &[0.0f32, -0.0, rng.radius(), -rng.radius()] {
                let A = c2Circle {
                    p: rng.vec_coord(),
                    r: ar,
                };
                cmp_cp!(c, r, A, *cap, "row30 #{i} cap{k} ar={ar}");
            }
        }
    }
}

#[test]
fn row31_capsule_special_floats() {
    let (c, r) = libs();
    let sp = specials();
    for &u in &sp {
        for &v in &sp {
            let A = c2Circle {
                p: c2v { x: u, y: v },
                r: u,
            };
            let caps = [
                c2Capsule {
                    a: c2v { x: v, y: u },
                    b: c2v { x: u, y: v },
                    r: v,
                },
                // degenerate + special
                c2Capsule {
                    a: c2v { x: u, y: v },
                    b: c2v { x: u, y: v },
                    r: v,
                },
                // infinite segment
                c2Capsule {
                    a: c2v {
                        x: f32::NEG_INFINITY,
                        y: 0.0,
                    },
                    b: c2v {
                        x: f32::INFINITY,
                        y: 0.0,
                    },
                    r: v,
                },
                // NaN segment
                c2Capsule {
                    a: c2v { x: f32::NAN, y: v },
                    b: c2v { x: u, y: f32::NAN },
                    r: u,
                },
            ];
            for (k, cap) in caps.iter().enumerate() {
                cmp_cp!(c, r, A, *cap, "row31 cap{k}");
            }
        }
    }
}

#[test]
fn row28b_capsule_random_raw_bits() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(280);
    for i in 0..300_000u64 {
        let A = c2Circle {
            p: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        let B = c2Capsule {
            a: rng.vec_raw(),
            b: rng.vec_raw(),
            r: rng.raw_f32(),
        };
        cmp_cp!(c, r, A, B, "row28b #{i}");
    }
}
