//! Focused stress tests for NaN/infinity propagation.
//!
//! On x86 the surviving NaN payload depends on which operand of each scalar FP
//! instruction is the destination register, so any input where several NaNs
//! compete is where a translation is most likely to diverge. These tests feed
//! the computation coordinates drawn from a pool of distinct NaN payloads
//! (quiet and signaling), infinities, zeros and overflow-inducing magnitudes.

mod common;

use common::{LmVec2 as V, Pair, Rng};

/// Distinct NaN payloads plus the values most likely to *create* a fresh NaN
/// via an invalid operation (inf - inf, inf * 0, 0 / 0).
fn nan_pool() -> Vec<f32> {
    let mut pool = vec![
        f32::from_bits(0x7FC0_0000), // default quiet NaN, positive
        f32::from_bits(0xFFC0_0000), // x86 "indefinite" quiet NaN
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFE7_57C9),
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0x7FA0_0000), // signaling NaN
        f32::from_bits(0xFFA0_0000), // signaling NaN, negative
        f32::from_bits(0x7F80_0001), // smallest signaling payload
        f32::from_bits(0xFF80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        1.0e-30,
        1.0e20,
        -1.0e20,
    ];
    pool.dedup_by_key(|v| v.to_bits());
    pool
}

/// Sweep every pool value through every one of the eight input coordinates,
/// on top of several different backgrounds.
#[test]
fn nan_single_coordinate_sweep() {
    let pair = Pair::load();
    let pool = nan_pool();

    let backgrounds: &[[V; 4]] = &[
        // Well-conditioned triangle.
        [
            V::new(0.0, 0.0),
            V::new(0.0, 1.0),
            V::new(1.0, 0.0),
            V::new(0.25, 0.25),
        ],
        // Degenerate: zero denominator, so invDenom is already inf/NaN.
        [
            V::new(1.0, 1.0),
            V::new(2.0, 2.0),
            V::new(3.0, 3.0),
            V::new(1.5, 1.5),
        ],
        // Huge magnitudes: dot products overflow to inf before any NaN input.
        [
            V::new(1.0e20, -1.0e20),
            V::new(1.0e20, 1.0e20),
            V::new(-1.0e20, 1.0e20),
            V::new(1.0e20, 0.0),
        ],
        // Tiny magnitudes: dot products underflow to zero.
        [
            V::new(0.0, 0.0),
            V::new(1.0e-30, 0.0),
            V::new(0.0, 1.0e-30),
            V::new(1.0e-30, 1.0e-30),
        ],
    ];

    for (bg_idx, bg) in backgrounds.iter().enumerate() {
        for slot in 0..4 {
            for axis in 0..2 {
                for &val in &pool {
                    let mut pts = *bg;
                    if axis == 0 {
                        pts[slot].x = val;
                    } else {
                        pts[slot].y = val;
                    }
                    pair.assert_same(
                        &format!(
                            "bg {bg_idx} slot {slot} axis {axis} val {:#010x}",
                            val.to_bits()
                        ),
                        pts[0],
                        pts[1],
                        pts[2],
                        pts[3],
                    );
                }
            }
        }
    }
}

/// All ordered pairs of pool values placed in all ordered pairs of coordinate
/// positions, so two competing NaN payloads meet inside the same operation.
#[test]
fn nan_pairwise_sweep() {
    let pair = Pair::load();
    let pool = nan_pool();

    let base = [
        V::new(0.0, 0.0),
        V::new(0.0, 1.0),
        V::new(1.0, 0.0),
        V::new(0.25, 0.25),
    ];

    // 8 coordinate positions, addressed as (slot, axis).
    let positions: Vec<(usize, usize)> =
        (0..4).flat_map(|s| (0..2).map(move |a| (s, a))).collect();

    let set = |pts: &mut [V; 4], (slot, axis): (usize, usize), val: f32| {
        if axis == 0 {
            pts[slot].x = val;
        } else {
            pts[slot].y = val;
        }
    };

    for (i, &pos_a) in positions.iter().enumerate() {
        for &pos_b in positions.iter().skip(i + 1) {
            for &va in &pool {
                for &vb in &pool {
                    let mut pts = base;
                    set(&mut pts, pos_a, va);
                    set(&mut pts, pos_b, vb);
                    pair.assert_same(
                        &format!(
                            "{pos_a:?}={:#010x} {pos_b:?}={:#010x}",
                            va.to_bits(),
                            vb.to_bits()
                        ),
                        pts[0],
                        pts[1],
                        pts[2],
                        pts[3],
                    );
                }
            }
        }
    }
}

/// Every coordinate drawn from the pool at random: heavy simultaneous NaN /
/// infinity traffic through all eight inputs at once.
#[test]
fn nan_saturated_random() {
    let pair = Pair::load();
    let pool = nan_pool();
    let mut rng = Rng::new(0xFEED_FACE_CAFE_B0BA);

    let pick = |rng: &mut Rng, pool: &[f32]| pool[(rng.next_u32() as usize) % pool.len()];

    for i in 0..200_000 {
        let p1 = V::new(pick(&mut rng, &pool), pick(&mut rng, &pool));
        let p2 = V::new(pick(&mut rng, &pool), pick(&mut rng, &pool));
        let p3 = V::new(pick(&mut rng, &pool), pick(&mut rng, &pool));
        let p = V::new(pick(&mut rng, &pool), pick(&mut rng, &pool));
        pair.assert_same(&format!("saturated iter {i}"), p1, p2, p3, p);
    }
}

/// Mix pool values with ordinary random coordinates, which is where a NaN meets
/// a finite value produced by cancellation or overflow.
#[test]
fn nan_mixed_with_random() {
    let pair = Pair::load();
    let pool = nan_pool();
    let mut rng = Rng::new(0x5EED_0000_1234_ABCD);

    for i in 0..200_000 {
        let mut coords = [0.0f32; 8];
        for c in coords.iter_mut() {
            // Roughly one coordinate in three comes from the special pool.
            *c = if rng.next_u32() % 3 == 0 {
                pool[(rng.next_u32() as usize) % pool.len()]
            } else {
                rng.coord(1.0e18)
            };
        }
        pair.assert_same(
            &format!("mixed iter {i}"),
            V::new(coords[0], coords[1]),
            V::new(coords[2], coords[3]),
            V::new(coords[4], coords[5]),
            V::new(coords[6], coords[7]),
        );
    }
}

/// Signed-zero and subnormal handling, where the sign of the result and the
/// rounding direction are easy to get wrong.
#[test]
fn signed_zero_and_subnormal() {
    let pair = Pair::load();

    let vals = [
        0.0f32,
        -0.0,
        f32::from_bits(1),        // smallest positive subnormal
        f32::from_bits(0x8000_0001), // smallest negative subnormal
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x007F_FFFF), // largest subnormal
    ];

    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                pair.assert_same(
                    &format!(
                        "zeros {:#010x}/{:#010x}/{:#010x}",
                        a.to_bits(),
                        b.to_bits(),
                        c.to_bits()
                    ),
                    V::new(a, b),
                    V::new(b, c),
                    V::new(c, a),
                    V::new(a, c),
                );
            }
        }
    }
}

/// Directed coverage for the operand ordering *inside* `lm_dot2`.
///
/// `lm_dot2(a, b)` computes the `y` product with `b.y` as the destination and
/// then adds the `x` product into the `y` product, so the operand order is
/// observable whenever competing NaN payloads reach the same instruction. These
/// cases plant distinct NaN payloads in the vertex coordinates so those
/// comparisons happen; the outer expression frequently masks the difference by
/// substituting another payload, so the broader randomized sweeps in this file
/// are what actually discriminate operand order. This test pins the directed
/// cases down as regression coverage.
#[test]
fn dot2_operand_order_via_distinct_y_nans() {
    let pair = Pair::load();

    let nan_a = f32::from_bits(0x7FC0_0001);
    let nan_b = f32::from_bits(0xFFD5_5555);
    let nan_c = f32::from_bits(0x7FEA_AAAA);
    let nan_d = f32::from_bits(0xFFC0_0003);
    let nans = [nan_a, nan_b, nan_c, nan_d];

    // Distinct NaNs in the y slots of p2/p3 (so v0.y and v1.y differ), with
    // p1/p all finite.
    for &a in &nans {
        for &b in &nans {
            pair.assert_same(
                &format!("v0.y/v1.y {:#010x}/{:#010x}", a.to_bits(), b.to_bits()),
                V::new(0.0, 0.0),
                V::new(2.0, b),
                V::new(3.0, a),
                V::new(0.5, 0.25),
            );
        }
    }

    // Now every vertex y is a distinct NaN, so v0.y, v1.y and v2.y all differ
    // and each of the five dot products has competing payloads.
    for &a in &nans {
        for &b in &nans {
            for &c in &nans {
                pair.assert_same(
                    &format!(
                        "all-y {:#010x}/{:#010x}/{:#010x}",
                        a.to_bits(),
                        b.to_bits(),
                        c.to_bits()
                    ),
                    V::new(1.0, 0.0),
                    V::new(2.0, a),
                    V::new(3.0, b),
                    V::new(4.0, c),
                );
            }
        }
    }

    // Mirror image: distinct NaNs in the x slots, finite y, which exercises the
    // `x` product's destination choice and the addition's source operand.
    for &a in &nans {
        for &b in &nans {
            for &c in &nans {
                pair.assert_same(
                    &format!(
                        "all-x {:#010x}/{:#010x}/{:#010x}",
                        a.to_bits(),
                        b.to_bits(),
                        c.to_bits()
                    ),
                    V::new(0.0, 1.0),
                    V::new(a, 2.0),
                    V::new(b, 3.0),
                    V::new(c, 4.0),
                );
            }
        }
    }

    // Both components NaN, with the x and y payloads deliberately different so
    // the addition's dest/src preference is observable.
    for &a in &nans {
        for &b in &nans {
            pair.assert_same(
                &format!("both-comp {:#010x}/{:#010x}", a.to_bits(), b.to_bits()),
                V::new(0.0, 0.0),
                V::new(a, b),
                V::new(b, a),
                V::new(a, a),
            );
        }
    }
}
