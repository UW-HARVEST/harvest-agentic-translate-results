//! Differential tests for `to_barycentric`, comparing the C `.so` against the
//! Rust `.so` through their exported C ABI symbols.
//!
//! `to_barycentric` is the only non-`static` function in `c_src/src/lib.c`, so
//! it is the whole public API. The helpers it builds on (`lm_v2`, `lm_sub2`,
//! `lm_dot2`) are `static` and therefore only reachable indirectly; the tests
//! below are ordered lowest-level-behavior first so that a failure points at
//! the smallest possible culprit:
//!
//! 1. struct pass/return marshalling (`lm_v2` identity paths)
//! 2. subtraction behavior (`lm_sub2`)
//! 3. dot-product behavior (`lm_dot2`), incl. rounding and overflow
//! 4. the full barycentric computation

mod common;

use common::{LmVec2 as V, Pair, Rng, EDGE_FLOATS};

/// Level 0: the ABI itself. A degenerate-free reference triangle whose result
/// is exactly representable, so any disagreement here is a calling-convention
/// or struct-layout problem rather than an arithmetic one.
#[test]
fn abi_struct_pass_and_return() {
    let pair = Pair::load();

    let p1 = V::new(0.0, 0.0);
    let p2 = V::new(0.0, 1.0);
    let p3 = V::new(1.0, 0.0);

    for (label, p) in [
        ("at p1", V::new(0.0, 0.0)),
        ("at p2", V::new(0.0, 1.0)),
        ("at p3", V::new(1.0, 0.0)),
        ("centroid", V::new(1.0 / 3.0, 1.0 / 3.0)),
    ] {
        pair.assert_same(label, p1, p2, p3, p);
    }

    // Sanity-check the reference values themselves so a mutually-wrong pair
    // cannot pass silently.
    let at_p3 = pair.rust(p1, p2, p3, p3);
    assert_eq!((at_p3.x, at_p3.y), (1.0, 0.0), "u,v at p3 should be (1,0)");
    let at_p2 = pair.rust(p1, p2, p3, p2);
    assert_eq!((at_p2.x, at_p2.y), (0.0, 1.0), "u,v at p2 should be (0,1)");
}

/// Level 1: `lm_sub2`. Values chosen so the subtractions hit signed zero,
/// cancellation, subnormal results and inf - inf.
#[test]
fn subtraction_paths() {
    let pair = Pair::load();

    let cases: &[(&str, [V; 4])] = &[
        (
            "signed zero",
            [V::new(-0.0, -0.0), V::new(0.0, 1.0), V::new(1.0, 0.0), V::new(0.0, 0.0)],
        ),
        (
            "cancellation",
            [
                V::new(16777216.0, 16777216.0),
                V::new(16777217.0, 16777216.0),
                V::new(16777216.0, 16777217.0),
                V::new(16777216.5, 16777216.5),
            ],
        ),
        (
            "subnormal deltas",
            [
                V::new(0.0, 0.0),
                V::new(1.0e-45, 0.0),
                V::new(0.0, 1.0e-45),
                V::new(1.0e-45, 1.0e-45),
            ],
        ),
        (
            "inf minus inf",
            [
                V::new(f32::INFINITY, f32::INFINITY),
                V::new(f32::INFINITY, 0.0),
                V::new(0.0, f32::INFINITY),
                V::new(1.0, 1.0),
            ],
        ),
        (
            "overflow on subtract",
            [
                V::new(f32::MIN, f32::MIN),
                V::new(f32::MAX, 0.0),
                V::new(0.0, f32::MAX),
                V::new(1.0, 1.0),
            ],
        ),
    ];

    for (label, pts) in cases {
        pair.assert_same(label, pts[0], pts[1], pts[2], pts[3]);
    }
}

/// Level 2: `lm_dot2` — magnitudes that overflow, underflow, or round during
/// `a.x * b.x + a.y * b.y`, plus cases where the products cancel.
#[test]
fn dot_product_paths() {
    let pair = Pair::load();

    let scales = [
        1.0e-22f32, 1.0e-10, 1.0e-3, 1.0, 1.0e3, 1.0e10, 1.0e19, 1.0e22, f32::MAX,
    ];

    for (i, &s) in scales.iter().enumerate() {
        // Right triangle scaled so that dot00/dot11 reach the extremes of the
        // exponent range (dot products square the scale).
        pair.assert_same(
            &format!("scale #{i} axis-aligned"),
            V::new(0.0, 0.0),
            V::new(0.0, s),
            V::new(s, 0.0),
            V::new(s * 0.25, s * 0.25),
        );

        // Non-orthogonal edges so dot01 is non-zero and the products in
        // `dot00 * dot11 - dot01 * dot01` nearly cancel.
        pair.assert_same(
            &format!("scale #{i} skewed"),
            V::new(-s, s * 0.5),
            V::new(s * 0.5, s),
            V::new(s, -s * 0.25),
            V::new(s * 0.125, -s * 0.75),
        );

        // Mixed magnitudes: one component huge, the other tiny.
        pair.assert_same(
            &format!("scale #{i} mixed"),
            V::new(0.0, 0.0),
            V::new(s, 1.0e-20),
            V::new(1.0e-20, s),
            V::new(s, s),
        );
    }
}

/// Level 3: the documented-by-omission degenerate cases. The C code performs
/// no zero-denominator check, so these must reproduce its inf/NaN output
/// bit-for-bit rather than being "fixed".
#[test]
fn degenerate_triangles() {
    let pair = Pair::load();

    let cases: &[(&str, [V; 4])] = &[
        (
            "all points identical",
            [V::new(1.0, 1.0), V::new(1.0, 1.0), V::new(1.0, 1.0), V::new(1.0, 1.0)],
        ),
        (
            "p2 == p1",
            [V::new(0.0, 0.0), V::new(0.0, 0.0), V::new(1.0, 1.0), V::new(0.5, 0.5)],
        ),
        (
            "p3 == p1",
            [V::new(0.0, 0.0), V::new(1.0, 1.0), V::new(0.0, 0.0), V::new(0.5, 0.5)],
        ),
        (
            "collinear",
            [V::new(0.0, 0.0), V::new(1.0, 1.0), V::new(2.0, 2.0), V::new(0.5, 0.5)],
        ),
        (
            "collinear, p off-line",
            [V::new(0.0, 0.0), V::new(1.0, 0.0), V::new(2.0, 0.0), V::new(0.5, 7.0)],
        ),
        (
            "antiparallel edges",
            [V::new(0.0, 0.0), V::new(-1.0, 0.0), V::new(1.0, 0.0), V::new(0.25, 0.0)],
        ),
        (
            "near-degenerate (tiny denominator)",
            [
                V::new(0.0, 0.0),
                V::new(1.0, 0.0),
                V::new(1.0, 1.0e-20),
                V::new(0.5, 1.0e-21),
            ],
        ),
        (
            "infinite vertex",
            [
                V::new(0.0, 0.0),
                V::new(f32::INFINITY, 0.0),
                V::new(0.0, 1.0),
                V::new(1.0, 1.0),
            ],
        ),
        (
            "NaN vertex",
            [V::new(0.0, 0.0), V::new(f32::NAN, 0.0), V::new(0.0, 1.0), V::new(1.0, 1.0)],
        ),
        (
            "NaN query point",
            [
                V::new(0.0, 0.0),
                V::new(1.0, 0.0),
                V::new(0.0, 1.0),
                V::new(f32::NAN, f32::NAN),
            ],
        ),
        (
            "denominator overflows to inf",
            [
                V::new(0.0, 0.0),
                V::new(0.0, f32::MAX),
                V::new(f32::MAX, 0.0),
                V::new(1.0, 1.0),
            ],
        ),
        (
            "denominator underflows to zero",
            [
                V::new(0.0, 0.0),
                V::new(0.0, 1.0e-30),
                V::new(1.0e-30, 0.0),
                V::new(1.0e-31, 1.0e-31),
            ],
        ),
    ];

    for (label, pts) in cases {
        pair.assert_same(label, pts[0], pts[1], pts[2], pts[3]);
    }
}

/// Cartesian sweep over interesting float values, applied one coordinate at a
/// time on top of a well-conditioned base triangle.
#[test]
fn edge_value_sweep() {
    let pair = Pair::load();

    let base = [
        V::new(0.25, -0.5),
        V::new(2.0, 0.75),
        V::new(-1.5, 3.0),
        V::new(0.125, 0.625),
    ];

    for slot in 0..4 {
        for axis in 0..2 {
            for &a in EDGE_FLOATS {
                let mut pts = base;
                match axis {
                    0 => pts[slot].x = a,
                    _ => pts[slot].y = a,
                }
                pair.assert_same(
                    &format!("slot {slot} axis {axis} value {a:e}"),
                    pts[0],
                    pts[1],
                    pts[2],
                    pts[3],
                );
            }
        }
    }

    // Pairwise: perturb two coordinates at once across all slot/axis pairs.
    for &a in EDGE_FLOATS {
        for &b in EDGE_FLOATS {
            let mut pts = base;
            pts[0].x = a;
            pts[1].y = b;
            pair.assert_same(&format!("pair p1.x={a:e} p2.y={b:e}"), pts[0], pts[1], pts[2], pts[3]);

            let mut pts = base;
            pts[2].x = a;
            pts[3].y = b;
            pair.assert_same(&format!("pair p3.x={a:e} p.y={b:e}"), pts[0], pts[1], pts[2], pts[3]);
        }
    }
}

/// Randomized sweep over well-behaved coordinates at several magnitudes.
#[test]
fn random_well_conditioned() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);

    for range in [1.0e-20f32, 1.0e-6, 1.0, 100.0, 1.0e6, 1.0e18] {
        for i in 0..20_000 {
            let p1 = V::new(rng.coord(range), rng.coord(range));
            let p2 = V::new(rng.coord(range), rng.coord(range));
            let p3 = V::new(rng.coord(range), rng.coord(range));
            let p = V::new(rng.coord(range), rng.coord(range));
            pair.assert_same(&format!("range {range:e} iter {i}"), p1, p2, p3, p);
        }
    }
}

/// Randomized sweep over completely arbitrary bit patterns, so NaN payloads,
/// infinities and subnormals all flow through the computation.
#[test]
fn random_arbitrary_bit_patterns() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);

    for i in 0..100_000 {
        let p1 = V::new(rng.any_f32(), rng.any_f32());
        let p2 = V::new(rng.any_f32(), rng.any_f32());
        let p3 = V::new(rng.any_f32(), rng.any_f32());
        let p = V::new(rng.any_f32(), rng.any_f32());
        pair.assert_same(&format!("bitpattern iter {i}"), p1, p2, p3, p);
    }
}

/// Randomized sweep biased toward degenerate/near-degenerate configurations by
/// reusing coordinates between vertices.
#[test]
fn random_degenerate_bias() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0BAD_F00D_DEAD_BEEF);

    for i in 0..50_000 {
        let ax = rng.coord(10.0);
        let ay = rng.coord(10.0);
        let bx = rng.coord(10.0);
        let by = rng.coord(10.0);
        let t = rng.coord(4.0);

        let p1 = V::new(ax, ay);
        // p2/p3 both on the line through p1 in direction (bx, by): denominator
        // cancels to (near) zero.
        let p2 = V::new(ax + bx, ay + by);
        let p3 = V::new(ax + bx * t, ay + by * t);
        let p = V::new(ax + bx * rng.coord(2.0), ay + by * rng.coord(2.0));

        pair.assert_same(&format!("degenerate iter {i}"), p1, p2, p3, p);
    }
}
