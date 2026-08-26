//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C17). Every call goes through
//! `dlopen`/`dlsym` on both the C `.so` and the Rust `.so`; results are
//! compared bit-for-bit (`to_bits`), never with float `==`, so NaN payloads,
//! NaN-vs-NaN and `+0.0`-vs-`-0.0` divergences cannot slip through.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

// ===========================================================================
// C1 — c2V
// ===========================================================================

#[test]
fn cfg_c1_c2v() {
    let (c, r) = apis();
    for &x in EDGE_FLOATS.iter() {
        for &y in EDGE_FLOATS.iter() {
            same("c2V", (x, y), (c.c2V)(x, y), (r.c2V)(x, y));
        }
    }
    let mut rng = Rng::new(0xC1);
    for _ in 0..N * 4 {
        let (x, y) = (rng.wild(), rng.wild());
        same("c2V", (x, y), (c.c2V)(x, y), (r.c2V)(x, y));
    }
}

// ===========================================================================
// C2 / C3 — c2Maxv / c2Minv
// ===========================================================================

/// Full cross product of the edge floats over all four components, which
/// covers `a>b`, `a<b`, `a==b`, `±0.0`, NaN in either/both operands and ±Inf
/// independently per component.
fn sweep_binary_v(name: &str, cf: extern "C" fn(C2v, C2v) -> C2v, rf: extern "C" fn(C2v, C2v) -> C2v) {
    for &ax in EDGE_FLOATS.iter() {
        for &bx in EDGE_FLOATS.iter() {
            for &ay in EDGE_FLOATS.iter() {
                for &by in EDGE_FLOATS.iter() {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    same(name, (a, b), cf(a, b), rf(a, b));
                }
            }
        }
    }
    let mut rng = Rng::new(0xB1);
    for _ in 0..N * 4 {
        let (a, b) = match rng.below(3) {
            0 => (rng.vec_grid(), rng.vec_grid()),
            1 => (rng.vec_wild(), rng.vec_wild()),
            _ => {
                // force exact ties in one component
                let v = rng.vec_wild();
                (v, C2v { x: v.x, y: rng.wild() })
            }
        };
        same(name, (a, b), cf(a, b), rf(a, b));
    }
}

#[test]
fn cfg_c2_c2maxv() {
    let (c, r) = apis();
    sweep_binary_v("c2Maxv", c.c2Maxv, r.c2Maxv);
}

#[test]
fn cfg_c3_c2minv() {
    let (c, r) = apis();
    sweep_binary_v("c2Minv", c.c2Minv, r.c2Minv);
}

// ===========================================================================
// C4 — c2Clampv (below / inside / above, lo==hi, inverted lo>hi, NaN, ±Inf)
// ===========================================================================

#[test]
fn cfg_c4_c2clampv() {
    let (c, r) = apis();

    // Exhaustive over the x components (y held at a fixed edge value), then
    // over the y components (x held fixed): 2 * 21^3 combinations.
    for &fixed in [0.0f32, f32::NAN, f32::INFINITY].iter() {
        for &a in EDGE_FLOATS.iter() {
            for &lo in EDGE_FLOATS.iter() {
                for &hi in EDGE_FLOATS.iter() {
                    let va = C2v { x: a, y: fixed };
                    let vlo = C2v { x: lo, y: fixed };
                    let vhi = C2v { x: hi, y: fixed };
                    same(
                        "c2Clampv/x",
                        (va, vlo, vhi),
                        (c.c2Clampv)(va, vlo, vhi),
                        (r.c2Clampv)(va, vlo, vhi),
                    );
                    let va = C2v { x: fixed, y: a };
                    let vlo = C2v { x: fixed, y: lo };
                    let vhi = C2v { x: fixed, y: hi };
                    same(
                        "c2Clampv/y",
                        (va, vlo, vhi),
                        (c.c2Clampv)(va, vlo, vhi),
                        (r.c2Clampv)(va, vlo, vhi),
                    );
                }
            }
        }
    }

    let mut rng = Rng::new(0xC4);
    for _ in 0..N * 4 {
        let (a, lo, hi) = match rng.below(4) {
            0 => (rng.vec_grid(), rng.vec_grid(), rng.vec_grid()), // often inverted
            1 => {
                let b = rng.aabb_grid();
                (rng.vec_grid(), b.min, b.max) // well-formed bounds
            }
            2 => {
                let v = rng.vec_grid();
                (v, v, v) // a == lo == hi
            }
            _ => (rng.vec_wild(), rng.vec_wild(), rng.vec_wild()),
        };
        same(
            "c2Clampv",
            (a, lo, hi),
            (c.c2Clampv)(a, lo, hi),
            (r.c2Clampv)(a, lo, hi),
        );
    }
}

// ===========================================================================
// C5 — c2Sub  /  C6 — c2Dot
// ===========================================================================

#[test]
fn cfg_c5_c2sub() {
    let (c, r) = apis();
    for &ax in EDGE_FLOATS.iter() {
        for &bx in EDGE_FLOATS.iter() {
            for &ay in EDGE_FLOATS.iter() {
                for &by in EDGE_FLOATS.iter() {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    same("c2Sub", (a, b), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                }
            }
        }
    }
    let mut rng = Rng::new(0xC5);
    for _ in 0..N * 4 {
        let a = if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_wild() };
        let b = match rng.below(3) {
            0 => a,                                     // x - x
            1 => C2v { x: -a.x, y: -a.y },              // x - (-x): overflow / Inf
            _ => rng.vec_wild(),
        };
        same("c2Sub", (a, b), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
    }
}

#[test]
fn cfg_c6_c2dot() {
    let (c, r) = apis();
    for &ax in EDGE_FLOATS.iter() {
        for &bx in EDGE_FLOATS.iter() {
            for &ay in EDGE_FLOATS.iter() {
                for &by in EDGE_FLOATS.iter() {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    same("c2Dot", (a, b), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                }
            }
        }
    }
    let mut rng = Rng::new(0xC6);
    for _ in 0..N * 4 {
        let a = if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_wild() };
        let b = match rng.below(4) {
            0 => C2v { x: a.y, y: -a.x },  // exact cancellation -> ±0
            1 => C2v { x: a.x, y: a.y },   // squares, may overflow to +Inf
            2 => C2v { x: 1e30 * a.x, y: 1e30 * a.y },
            _ => rng.vec_wild(),
        };
        same("c2Dot", (a, b), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
    }
}

// ===========================================================================
// C7 — c2CircletoCircle
// ===========================================================================

/// Hand-picked circle pairs: exact tangency (`d2 == r2`, C returns 0),
/// concentric, containment, zero radius, negative radius, radius overflow,
/// NaN / Inf coordinates.
fn circle_circle_edge_cases() -> Vec<(C2Circle, C2Circle)> {
    let cir = |x: f32, y: f32, r: f32| C2Circle { p: C2v { x, y }, r };
    let mut v = vec![
        // exact tangency: d = 2, rA+rB = 2  ->  d2 == r2  ->  0
        (cir(0.0, 0.0, 1.0), cir(2.0, 0.0, 1.0)),
        (cir(0.0, 0.0, 1.0), cir(0.0, 2.0, 1.0)),
        (cir(-1.0, 0.0, 0.5), cir(0.0, 0.0, 0.5)),
        // just overlapping / just separated around the tie
        (cir(0.0, 0.0, 1.0), cir(1.999_999_8, 0.0, 1.0)),
        (cir(0.0, 0.0, 1.0), cir(2.000_000_5, 0.0, 1.0)),
        // concentric
        (cir(1.0, 1.0, 2.0), cir(1.0, 1.0, 3.0)),
        (cir(1.0, 1.0, 0.0), cir(1.0, 1.0, 0.0)),
        // containment
        (cir(0.0, 0.0, 10.0), cir(1.0, 1.0, 0.25)),
        // zero radius
        (cir(0.0, 0.0, 0.0), cir(0.0, 0.0, 1.0)),
        (cir(0.0, 0.0, 0.0), cir(1.0, 0.0, 1.0)),
        // negative radii (C does not validate; (rA+rB)^2 hides the sign)
        (cir(0.0, 0.0, -1.0), cir(1.0, 0.0, -1.0)),
        (cir(0.0, 0.0, -3.0), cir(1.0, 0.0, 1.0)),
        (cir(0.0, 0.0, -1.0), cir(0.5, 0.0, 1.0)),
        // radius sum overflows f32 on squaring
        (cir(0.0, 0.0, f32::MAX), cir(1.0, 0.0, f32::MAX)),
        (cir(0.0, 0.0, 1e30), cil_far()),
        // Inf / NaN
        (cir(f32::INFINITY, 0.0, 1.0), cir(0.0, 0.0, 1.0)),
        (cir(f32::NAN, 0.0, 1.0), cir(0.0, 0.0, 1.0)),
        (cir(0.0, 0.0, f32::NAN), cir(0.0, 0.0, 1.0)),
        (cir(0.0, 0.0, f32::INFINITY), cir(f32::INFINITY, 0.0, 1.0)),
        (cir(-0.0, -0.0, -0.0), cir(0.0, 0.0, 0.0)),
        // subnormal separation
        (cir(0.0, 0.0, 1e-45), cir(1e-45, 0.0, 1e-45)),
    ];
    fn cil_far() -> C2Circle {
        C2Circle {
            p: C2v { x: 1e30, y: 0.0 },
            r: 1e30,
        }
    }
    // add the mirrored order of every pair too
    let mirrored: Vec<_> = v.iter().map(|&(a, b)| (b, a)).collect();
    v.extend(mirrored);
    v
}

#[test]
fn cfg_c7_circle_circle() {
    let (c, r) = apis();
    for (a, b) in circle_circle_edge_cases() {
        same(
            "c2CircletoCircle",
            (a, b),
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
        );
    }
    let mut rng = Rng::new(0xC7);
    let mut hits = 0usize;
    for _ in 0..N * 4 {
        let (a, b) = if rng.below(4) == 0 {
            (rng.circle_wild(), rng.circle_wild())
        } else {
            (rng.circle_grid(), rng.circle_grid())
        };
        let rc = (c.c2CircletoCircle)(a, b);
        same("c2CircletoCircle", (a, b), rc, (r.c2CircletoCircle)(a, b));
        hits += (rc != 0) as usize;
    }
    // sanity: the random generator must produce both outcomes, otherwise the
    // row would only be testing one branch of the comparison
    assert!(hits > 100 && hits < N * 4 - 100, "poor hit/miss balance: {hits}");
}

// ===========================================================================
// C8 — c2CircletoAABB
// ===========================================================================

fn circle_aabb_edge_cases() -> Vec<(C2Circle, C2Aabb)> {
    let cir = |x: f32, y: f32, r: f32| C2Circle { p: C2v { x, y }, r };
    let box_ = |x0: f32, y0: f32, x1: f32, y1: f32| C2Aabb {
        min: C2v { x: x0, y: y0 },
        max: C2v { x: x1, y: y1 },
    };
    let unit = box_(-1.0, -1.0, 1.0, 1.0);
    vec![
        // centre inside
        (cir(0.0, 0.0, 0.5), unit),
        (cir(0.0, 0.0, 0.0), unit),   // r == 0 with centre inside -> d2 == r2 == 0 -> 0
        (cir(0.999, 0.999, 0.001), unit),
        // exactly on each edge (d2 == 0)
        (cir(1.0, 0.0, 0.5), unit),
        (cir(-1.0, 0.0, 0.5), unit),
        (cir(0.0, 1.0, 0.5), unit),
        (cir(0.0, -1.0, 0.5), unit),
        // just outside each edge, exact tie d == r
        (cir(2.0, 0.0, 1.0), unit),
        (cir(-2.0, 0.0, 1.0), unit),
        (cir(0.0, 2.0, 1.0), unit),
        (cir(0.0, -2.0, 1.0), unit),
        // just inside / just outside the tie
        (cir(2.0, 0.0, 1.000_001), unit),
        (cir(2.0, 0.0, 0.999_999), unit),
        // past each corner, exact tie d^2 = 2
        (cir(2.0, 2.0, 1.414_213_6), unit),
        (cir(-2.0, 2.0, 1.414_213_6), unit),
        (cir(2.0, -2.0, 1.414_213_6), unit),
        (cir(-2.0, -2.0, 1.414_213_6), unit),
        (cir(2.0, 2.0, 1.5), unit),
        (cir(2.0, 2.0, 1.4), unit),
        // exactly on a corner
        (cir(1.0, 1.0, 0.5), unit),
        (cir(1.0, 1.0, 0.0), unit),
        // negative radius (never validated: r*r > 0 so it behaves like |r|)
        (cir(2.0, 0.0, -1.5), unit),
        (cir(0.0, 0.0, -0.5), unit),
        // degenerate box (min == max) -> clamp collapses to the point
        (cir(0.0, 0.0, 1.0), box_(0.0, 0.0, 0.0, 0.0)),
        (cir(0.5, 0.5, 1.0), box_(0.0, 0.0, 0.0, 0.0)),
        // INVERTED box (min > max): c2Clampv = max(lo, min(a,hi)) == lo
        (cir(0.0, 0.0, 1.0), box_(1.0, 1.0, -1.0, -1.0)),
        (cir(5.0, 5.0, 1.0), box_(1.0, 1.0, -1.0, -1.0)),
        (cir(1.0, 1.0, 0.5), box_(1.0, 1.0, -1.0, -1.0)),
        // Inf / NaN in the box or the circle
        (cir(0.0, 0.0, 1.0), box_(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY)),
        (cir(0.0, 0.0, 1.0), box_(f32::NAN, 0.0, 1.0, 1.0)),
        (cir(0.0, 0.0, 1.0), box_(0.0, 0.0, f32::NAN, 1.0)),
        (cir(f32::NAN, 0.0, 1.0), unit),
        (cir(0.0, 0.0, f32::NAN), unit),
        (cir(f32::INFINITY, 0.0, f32::INFINITY), unit),
        (cir(0.0, 0.0, f32::INFINITY), box_(f32::INFINITY, 0.0, f32::INFINITY, 0.0)),
        // ±0 boundaries
        (cir(-0.0, -0.0, 0.0), box_(-0.0, -0.0, 0.0, 0.0)),
        // huge magnitudes -> d2 overflows to +Inf
        (cir(f32::MAX, f32::MAX, f32::MAX), box_(f32::MIN, f32::MIN, 0.0, 0.0)),
        // subnormal
        (cir(0.0, 0.0, 1e-45), box_(1e-45, 1e-45, 2e-45, 2e-45)),
    ]
}

#[test]
fn cfg_c8_circle_aabb() {
    let (c, r) = apis();
    for (a, b) in circle_aabb_edge_cases() {
        same(
            "c2CircletoAABB",
            (a, b),
            (c.c2CircletoAABB)(a, b),
            (r.c2CircletoAABB)(a, b),
        );
    }
    let mut rng = Rng::new(0xC8);
    let mut hits = 0usize;
    for _ in 0..N * 4 {
        let (a, b) = if rng.below(4) == 0 {
            (rng.circle_wild(), rng.aabb_wild())
        } else {
            (rng.circle_grid(), rng.aabb_grid())
        };
        let rc = (c.c2CircletoAABB)(a, b);
        same("c2CircletoAABB", (a, b), rc, (r.c2CircletoAABB)(a, b));
        hits += (rc != 0) as usize;
    }
    assert!(hits > 100 && hits < N * 4 - 100, "poor hit/miss balance: {hits}");
}

// ===========================================================================
// C9 — c2AABBtoAABB
// ===========================================================================

fn aabb_aabb_edge_cases() -> Vec<(C2Aabb, C2Aabb)> {
    let box_ = |x0: f32, y0: f32, x1: f32, y1: f32| C2Aabb {
        min: C2v { x: x0, y: y0 },
        max: C2v { x: x1, y: y1 },
    };
    let unit = box_(-1.0, -1.0, 1.0, 1.0);
    let mut v = vec![
        // overlapping
        (unit, box_(0.0, 0.0, 2.0, 2.0)),
        // edge-touching: A.max.x == B.min.x -> `<` false -> returns 1
        (unit, box_(1.0, -1.0, 3.0, 1.0)),
        (unit, box_(-1.0, 1.0, 1.0, 3.0)),
        (box_(1.0, -1.0, 3.0, 1.0), unit),
        // corner-touching only
        (unit, box_(1.0, 1.0, 3.0, 3.0)),
        // separated in x only / y only / both
        (unit, box_(1.000_001, -1.0, 3.0, 1.0)),
        (unit, box_(-1.0, 1.000_001, 1.0, 3.0)),
        (unit, box_(2.0, 2.0, 3.0, 3.0)),
        // containment
        (unit, box_(-0.5, -0.5, 0.5, 0.5)),
        (box_(-0.5, -0.5, 0.5, 0.5), unit),
        // identical
        (unit, unit),
        // degenerate (min == max)
        (box_(0.0, 0.0, 0.0, 0.0), unit),
        (box_(0.0, 0.0, 0.0, 0.0), box_(0.0, 0.0, 0.0, 0.0)),
        (box_(2.0, 2.0, 2.0, 2.0), unit),
        // INVERTED (min > max), never validated by C
        (box_(1.0, 1.0, -1.0, -1.0), unit),
        (box_(1.0, 1.0, -1.0, -1.0), box_(1.0, 1.0, -1.0, -1.0)),
        (box_(5.0, 5.0, -5.0, -5.0), box_(0.0, 0.0, 1.0, 1.0)),
        // ±0
        (box_(-0.0, -0.0, 0.0, 0.0), box_(0.0, 0.0, -0.0, -0.0)),
        // Inf
        (box_(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY), unit),
        (box_(f32::INFINITY, f32::INFINITY, f32::INFINITY, f32::INFINITY), unit),
        (box_(f32::NEG_INFINITY, 0.0, f32::NEG_INFINITY, 0.0), unit),
        // NaN in each of the eight slots
        (box_(f32::NAN, -1.0, 1.0, 1.0), unit),
        (box_(-1.0, f32::NAN, 1.0, 1.0), unit),
        (box_(-1.0, -1.0, f32::NAN, 1.0), unit),
        (box_(-1.0, -1.0, 1.0, f32::NAN), unit),
        (unit, box_(f32::NAN, -1.0, 1.0, 1.0)),
        (unit, box_(-1.0, f32::NAN, 1.0, 1.0)),
        (unit, box_(-1.0, -1.0, f32::NAN, 1.0)),
        (unit, box_(-1.0, -1.0, 1.0, f32::NAN)),
        // extremes
        (box_(f32::MIN, f32::MIN, f32::MAX, f32::MAX), unit),
        (box_(1e-45, 1e-45, 2e-45, 2e-45), box_(0.0, 0.0, 1e-45, 1e-45)),
    ];
    let mirrored: Vec<_> = v.iter().map(|&(a, b)| (b, a)).collect();
    v.extend(mirrored);
    v
}

#[test]
fn cfg_c9_aabb_aabb() {
    let (c, r) = apis();
    for (a, b) in aabb_aabb_edge_cases() {
        same(
            "c2AABBtoAABB",
            (a, b),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
    let mut rng = Rng::new(0xC9);
    let mut hits = 0usize;
    for _ in 0..N * 4 {
        let (a, b) = if rng.below(4) == 0 {
            (rng.aabb_wild(), rng.aabb_wild())
        } else {
            (rng.aabb_grid(), rng.aabb_grid())
        };
        let rc = (c.c2AABBtoAABB)(a, b);
        same("c2AABBtoAABB", (a, b), rc, (r.c2AABBtoAABB)(a, b));
        hits += (rc != 0) as usize;
    }
    assert!(hits > 100 && hits < N * 4 - 100, "poor hit/miss balance: {hits}");
}

// ===========================================================================
// C10..C13 — `collided` dispatch, all four valid tag pairs
// ===========================================================================

#[test]
fn cfg_c10_collided_circle_circle() {
    for (a, b) in circle_circle_edge_cases() {
        collided_bytes_both(
            &circle_bytes(&a),
            C2_TYPE_CIRCLE,
            &circle_bytes(&b),
            C2_TYPE_CIRCLE,
            0,
            0,
            (a, b),
        );
    }
    let mut rng = Rng::new(0xC10);
    for _ in 0..N {
        let (a, b) = if rng.below(4) == 0 {
            (rng.circle_wild(), rng.circle_wild())
        } else {
            (rng.circle_grid(), rng.circle_grid())
        };
        collided_bytes_both(
            &circle_bytes(&a),
            C2_TYPE_CIRCLE,
            &circle_bytes(&b),
            C2_TYPE_CIRCLE,
            0,
            0,
            (a, b),
        );
    }
}

#[test]
fn cfg_c11_collided_circle_aabb() {
    for (a, b) in circle_aabb_edge_cases() {
        collided_bytes_both(
            &circle_bytes(&a),
            C2_TYPE_CIRCLE,
            &aabb_bytes(&b),
            C2_TYPE_AABB,
            0,
            0,
            (a, b),
        );
    }
    let mut rng = Rng::new(0xC11);
    for _ in 0..N {
        let (a, b) = if rng.below(4) == 0 {
            (rng.circle_wild(), rng.aabb_wild())
        } else {
            (rng.circle_grid(), rng.aabb_grid())
        };
        collided_bytes_both(
            &circle_bytes(&a),
            C2_TYPE_CIRCLE,
            &aabb_bytes(&b),
            C2_TYPE_AABB,
            0,
            0,
            (a, b),
        );
    }
}

/// The argument-swapping arm: C computes `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)`,
/// i.e. `A` is the AABB and `B` is the circle. A translation that forgot the
/// swap would still pass C11, so this row is checked separately and also
/// cross-validated against `c2CircletoAABB` called directly.
#[test]
fn cfg_c12_collided_aabb_circle() {
    let (c, r) = apis();
    for (circle, aabb) in circle_aabb_edge_cases() {
        collided_bytes_both(
            &aabb_bytes(&aabb),
            C2_TYPE_AABB,
            &circle_bytes(&circle),
            C2_TYPE_CIRCLE,
            0,
            0,
            (aabb, circle),
        );
        // the dispatcher must agree with the direct mid-level call, in BOTH libs
        let ab = aabb_bytes(&aabb);
        let cb = circle_bytes(&circle);
        let via_c = unsafe {
            (c.collided)(
                ab.as_ptr() as *const c_void,
                C2_TYPE_AABB,
                cb.as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
            )
        };
        let via_r = unsafe {
            (r.collided)(
                ab.as_ptr() as *const c_void,
                C2_TYPE_AABB,
                cb.as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
            )
        };
        same("collided(AABB,CIRCLE) vs c2CircletoAABB [C]", (aabb, circle), (c.c2CircletoAABB)(circle, aabb), via_c);
        same("collided(AABB,CIRCLE) vs c2CircletoAABB [Rust]", (aabb, circle), (r.c2CircletoAABB)(circle, aabb), via_r);
    }
    let mut rng = Rng::new(0xC12);
    for _ in 0..N {
        let (circle, aabb) = if rng.below(4) == 0 {
            (rng.circle_wild(), rng.aabb_wild())
        } else {
            (rng.circle_grid(), rng.aabb_grid())
        };
        collided_bytes_both(
            &aabb_bytes(&aabb),
            C2_TYPE_AABB,
            &circle_bytes(&circle),
            C2_TYPE_CIRCLE,
            0,
            0,
            (aabb, circle),
        );
    }
}

#[test]
fn cfg_c13_collided_aabb_aabb() {
    for (a, b) in aabb_aabb_edge_cases() {
        collided_bytes_both(
            &aabb_bytes(&a),
            C2_TYPE_AABB,
            &aabb_bytes(&b),
            C2_TYPE_AABB,
            0,
            0,
            (a, b),
        );
    }
    let mut rng = Rng::new(0xC13);
    for _ in 0..N {
        let (a, b) = if rng.below(4) == 0 {
            (rng.aabb_wild(), rng.aabb_wild())
        } else {
            (rng.aabb_grid(), rng.aabb_grid())
        };
        collided_bytes_both(
            &aabb_bytes(&a),
            C2_TYPE_AABB,
            &aabb_bytes(&b),
            C2_TYPE_AABB,
            0,
            0,
            (a, b),
        );
    }
}

// ===========================================================================
// C14 — misaligned pointers through `collided` (all four valid tag pairs)
// ===========================================================================

#[test]
fn cfg_c14_collided_unaligned_pointers() {
    let mut rng = Rng::new(0xC14);
    for off_a in 0..8usize {
        for off_b in 0..8usize {
            for _ in 0..64 {
                let ci = rng.circle_grid();
                let cj = rng.circle_grid();
                let bi = rng.aabb_grid();
                let bj = rng.aabb_grid();
                collided_bytes_both(&circle_bytes(&ci), C2_TYPE_CIRCLE, &circle_bytes(&cj), C2_TYPE_CIRCLE, off_a, off_b, (ci, cj));
                collided_bytes_both(&circle_bytes(&ci), C2_TYPE_CIRCLE, &aabb_bytes(&bj), C2_TYPE_AABB, off_a, off_b, (ci, bj));
                collided_bytes_both(&aabb_bytes(&bi), C2_TYPE_AABB, &circle_bytes(&cj), C2_TYPE_CIRCLE, off_a, off_b, (bi, cj));
                collided_bytes_both(&aabb_bytes(&bi), C2_TYPE_AABB, &aabb_bytes(&bj), C2_TYPE_AABB, off_a, off_b, (bi, bj));
            }
        }
    }
}

// ===========================================================================
// C15 — aliased pointers (A == B) through `collided`
// ===========================================================================

#[test]
fn cfg_c15_collided_aliased_pointers() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC15);
    for _ in 0..N {
        // 16 bytes of payload readable as either a c2Circle (first 12) or a c2AABB
        let b = rng.aabb_grid();
        let bytes = aabb_bytes(&b);
        let p = bytes.as_ptr() as *const c_void;
        for &(ta, tb) in [
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ]
        .iter()
        {
            let rc = unsafe { (c.collided)(p, ta, p, tb) };
            let rr = unsafe { (r.collided)(p, ta, p, tb) };
            same("collided aliased", (b, ta, tb), rc, rr);
        }
    }
}

// ===========================================================================
// C16 / C17 — composed low-level pipelines
// ===========================================================================

/// Reproduce the internals of `c2CircletoCircle` by driving the low-level
/// exports (`c2V`, `c2Sub`, `c2Dot`) across the FFI boundary, and require that
/// the composition matches in both libraries AND agrees with the mid-level
/// entry point.
#[test]
fn cfg_c16_composed_pipeline() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC16);
    for i in 0..N * 2 {
        let (a, b) = if i % 4 == 0 {
            (rng.circle_wild(), rng.circle_wild())
        } else {
            (rng.circle_grid(), rng.circle_grid())
        };

        // low-level chain, executed independently in each library
        let av_c = (c.c2V)(a.p.x, a.p.y);
        let bv_c = (c.c2V)(b.p.x, b.p.y);
        let d_c = (c.c2Sub)(bv_c, av_c);
        let d2_c = (c.c2Dot)(d_c, d_c);

        let av_r = (r.c2V)(a.p.x, a.p.y);
        let bv_r = (r.c2V)(b.p.x, b.p.y);
        let d_r = (r.c2Sub)(bv_r, av_r);
        let d2_r = (r.c2Dot)(d_r, d_r);

        same("composed c2V", (a, b), av_c, av_r);
        same("composed c2Sub", (a, b), d_c, d_r);
        same("composed c2Dot", (a, b), d2_c, d2_r);

        // and the composition must reproduce the mid-level result in each lib
        let rsum = a.r + b.r;
        let expect = ((d2_c < rsum * rsum) as i32) as std::ffi::c_int;
        same("pipeline vs c2CircletoCircle [C]", (a, b), (c.c2CircletoCircle)(a, b), expect);
        let expect_r = ((d2_r < rsum * rsum) as i32) as std::ffi::c_int;
        same("pipeline vs c2CircletoCircle [Rust]", (a, b), (r.c2CircletoCircle)(a, b), expect_r);
    }
}

/// The internal chain of `c2CircletoAABB`: `c2Clampv` → `c2Sub` → `c2Dot`.
#[test]
fn cfg_c17_composed_clamp_pipeline() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC17);
    for i in 0..N * 2 {
        let (a, bx) = if i % 4 == 0 {
            (rng.circle_wild(), rng.aabb_wild())
        } else {
            (rng.circle_grid(), rng.aabb_grid())
        };

        let l_c = (c.c2Clampv)(a.p, bx.min, bx.max);
        let ab_c = (c.c2Sub)(a.p, l_c);
        let d2_c = (c.c2Dot)(ab_c, ab_c);

        let l_r = (r.c2Clampv)(a.p, bx.min, bx.max);
        let ab_r = (r.c2Sub)(a.p, l_r);
        let d2_r = (r.c2Dot)(ab_r, ab_r);

        same("composed c2Clampv", (a, bx), l_c, l_r);
        same("composed c2Sub", (a, bx), ab_c, ab_r);
        same("composed c2Dot", (a, bx), d2_c, d2_r);

        let expect_c = ((d2_c < a.r * a.r) as i32) as std::ffi::c_int;
        same("pipeline vs c2CircletoAABB [C]", (a, bx), (c.c2CircletoAABB)(a, bx), expect_c);
        let expect_r = ((d2_r < a.r * a.r) as i32) as std::ffi::c_int;
        same("pipeline vs c2CircletoAABB [Rust]", (a, bx), (r.c2CircletoAABB)(a, bx), expect_r);

        // …and the dispatcher must agree with the mid-level call in both libs
        collided_bytes_both(
            &circle_bytes(&a),
            C2_TYPE_CIRCLE,
            &aabb_bytes(&bx),
            C2_TYPE_AABB,
            0,
            0,
            (a, bx),
        );
    }
}
