//! Phase B — valid-path differential tests for the collision routines and the
//! `collided` dispatcher.
//!
//! Covers `CONFIGS.md` rows 29–66.

mod common;
use common::*;
use std::ffi::c_void;

const SEED: u64 = 0xFEED_FACE_CAFE_0001;

// ---------------------------------------------------------------------------
// per-function checkers
// ---------------------------------------------------------------------------

fn check_cc(l: &Pair, row: &str, a: c2Circle, b: c2Circle) {
    assert_int_eq!(
        row,
        l.c.c2CircletoCircle(a, b),
        l.rs.c2CircletoCircle(a, b),
        "c2CircletoCircle(p={} r={}, p={} r={})",
        showv(a.p),
        show(a.r),
        showv(b.p),
        show(b.r)
    );
}

fn check_ca(l: &Pair, row: &str, a: c2Circle, b: c2AABB) {
    assert_int_eq!(
        row,
        l.c.c2CircletoAABB(a, b),
        l.rs.c2CircletoAABB(a, b),
        "c2CircletoAABB(p={} r={}, min={} max={})",
        showv(a.p),
        show(a.r),
        showv(b.min),
        showv(b.max)
    );
}

fn check_aa(l: &Pair, row: &str, a: c2AABB, b: c2AABB) {
    assert_int_eq!(
        row,
        l.c.c2AABBtoAABB(a, b),
        l.rs.c2AABBtoAABB(a, b),
        "c2AABBtoAABB(min={} max={}, min={} max={})",
        showv(a.min),
        showv(a.max),
        showv(b.min),
        showv(b.max)
    );
}

// ===========================================================================
// c2CircletoCircle — rows 29–38
// ===========================================================================

#[test]
fn row29_cc_separated() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..30_000 {
        let a = circle(rng.signed(100.0), rng.signed(100.0), rng.signed(3.0).abs());
        // Place B far enough away that the circles cannot touch.
        let d = 200.0 + rng.signed(100.0).abs();
        let b = circle(a.p.x + d, a.p.y + d, rng.signed(3.0).abs());
        check_cc(&l, "row29", a, b);
    }
}

#[test]
fn row30_cc_overlapping() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..30_000 {
        let ra = 1.0 + rng.signed(10.0).abs();
        let rb = 1.0 + rng.signed(10.0).abs();
        let a = circle(rng.signed(100.0), rng.signed(100.0), ra);
        // Offset strictly less than ra+rb ⇒ overlap.
        let f = rng.signed(0.5).abs(); // [0, 0.5)
        let b = circle(a.p.x + (ra + rb) * f, a.p.y + (ra + rb) * f, rb);
        check_cc(&l, "row30", a, b);
    }
}

#[test]
fn row31_cc_exactly_tangent_boundary() {
    let l = libs();
    // Pythagorean triples make d2 == (rA+rB)^2 exactly representable, so the
    // strict `<` boundary is hit bit-exactly (expected result: 0).
    let triples: &[(f32, f32, f32)] = &[
        (3.0, 4.0, 5.0),
        (5.0, 12.0, 13.0),
        (8.0, 15.0, 17.0),
        (7.0, 24.0, 25.0),
        (20.0, 21.0, 29.0),
        (0.0, 1.0, 1.0),
        (1.0, 0.0, 1.0),
        (0.75, 1.0, 1.25),
    ];
    for &(dx, dy, h) in triples {
        for &split in &[0.0f32, 0.25, 0.5, 1.0] {
            let ra = h * split;
            let rb = h - ra;
            let a = circle(0.0, 0.0, ra);
            let b = circle(dx, dy, rb);
            check_cc(&l, "row31/exact", a, b);
            check_cc(&l, "row31/exact.swap", b, a);
            // One ULP inside and one ULP outside the boundary.
            for &nudge in &[-1i32, 1] {
                let rb2 = f32::from_bits((rb.to_bits() as i64 + nudge as i64) as u32);
                check_cc(&l, "row31/ulp", a, circle(dx, dy, rb2));
                let dx2 = f32::from_bits((dx.to_bits() as i64 + nudge as i64) as u32);
                check_cc(&l, "row31/ulp.d", a, circle(dx2, dy, rb));
            }
        }
    }
}

#[test]
fn row32_33_34_cc_identical_centres_zero_and_negative_radii() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 32);
    let radii = [0.0f32, -0.0, 1.0, -1.0, 5.0, -5.0, 1e-40, -1e-40];
    for _ in 0..2_000 {
        let p = v(rng.signed(50.0), rng.signed(50.0));
        for &ra in &radii {
            for &rb in &radii {
                // identical centres (d2 == 0)
                check_cc(
                    &l,
                    "row32",
                    c2Circle { p, r: ra },
                    c2Circle { p, r: rb },
                );
                // and displaced ones with the same (possibly negative) radii
                let q = v(p.x + rng.signed(10.0), p.y + rng.signed(10.0));
                check_cc(
                    &l,
                    "row33+34",
                    c2Circle { p, r: ra },
                    c2Circle { p: q, r: rb },
                );
            }
        }
    }
}

#[test]
fn row35_36_cc_overflowing_and_infinite_radii() {
    let l = libs();
    let radii = [
        f32::MAX,
        -f32::MAX,
        3.0e38,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e20,
    ];
    let coords = [
        0.0f32,
        1.0,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e30,
    ];
    for &ra in &radii {
        for &rb in &radii {
            for &cx in &coords {
                for &cy in &coords {
                    check_cc(&l, "row35+36", circle(0.0, 0.0, ra), circle(cx, cy, rb));
                    check_cc(&l, "row35+36", circle(cx, cy, ra), circle(0.0, 0.0, rb));
                }
            }
        }
    }
}

#[test]
fn row37_cc_nan_in_centre_or_radius() {
    let l = libs();
    // Every non-empty subset of the six floats replaced by a NaN.
    let base = [0.0f32, 0.0, 1.0, 3.0, 4.0, 1.0]; // ax,ay,ar, bx,by,br
    for mask in 1u32..64 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..6 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            check_cc(
                &l,
                "row37",
                circle(c[0], c[1], c[2]),
                circle(c[3], c[4], c[5]),
            );
        }
    }
    // +inf and -inf radii summing to NaN.
    check_cc(
        &l,
        "row37/inf-sum",
        circle(0.0, 0.0, f32::INFINITY),
        circle(0.0, 0.0, f32::NEG_INFINITY),
    );
}

#[test]
fn row38_cc_fully_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 38);
    for _ in 0..100_000 {
        check_cc(&l, "row38/raw", rng.raw_circle(), rng.raw_circle());
    }
    for _ in 0..100_000 {
        check_cc(&l, "row38/nasty", rng.nasty_circle(), rng.nasty_circle());
    }
}

// ===========================================================================
// c2CircletoAABB — rows 39–48
// ===========================================================================

#[test]
fn row39_ca_centre_inside_box() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 39);
    for _ in 0..30_000 {
        let min = v(rng.signed(100.0), rng.signed(100.0));
        let max = v(min.x + 1.0 + rng.signed(20.0).abs(), min.y + 1.0 + rng.signed(20.0).abs());
        let p = v(
            min.x + (max.x - min.x) * rng.signed(0.5).abs(),
            min.y + (max.y - min.y) * rng.signed(0.5).abs(),
        );
        for &r in &[0.0f32, 1e-30, 0.5, 100.0, -0.5] {
            check_ca(&l, "row39", c2Circle { p, r }, c2AABB { min, max });
        }
    }
}

#[test]
fn row40_41_42_ca_clamp_on_each_face_and_corner() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 40);
    for _ in 0..5_000 {
        let min = v(rng.signed(50.0), rng.signed(50.0));
        let max = v(min.x + 4.0, min.y + 6.0);
        let midx = (min.x + max.x) * 0.5;
        let midy = (min.y + max.y) * 0.5;
        let out = 1.0 + rng.signed(5.0).abs();
        // 4 faces, 4 corners, exactly-on-face and exactly-on-corner, centre.
        let centres = [
            v(min.x - out, midy),   // left face
            v(max.x + out, midy),   // right face
            v(midx, min.y - out),   // bottom face
            v(midx, max.y + out),   // top face
            v(min.x - out, min.y - out), // corners
            v(max.x + out, min.y - out),
            v(min.x - out, max.y + out),
            v(max.x + out, max.y + out),
            v(min.x, midy),   // exactly on a face
            v(max.x, midy),
            v(midx, min.y),
            v(midx, max.y),
            v(min.x, min.y),  // exactly on a corner
            v(max.x, max.y),
            v(min.x, max.y),
            v(max.x, min.y),
            v(midx, midy),    // inside
        ];
        for &p in &centres {
            for &r in &[0.0f32, -1.0, 0.25, out, out * 2.0, 1e30] {
                check_ca(&l, "row40-42", c2Circle { p, r }, c2AABB { min, max });
            }
        }
    }
}

#[test]
fn row43_ca_exactly_tangent_boundary() {
    let l = libs();
    // Clamp point is a corner at distance exactly `h` (Pythagorean triple), so
    // d2 == r2 exactly ⇒ strict `<` is false.
    let triples: &[(f32, f32, f32)] = &[
        (3.0, 4.0, 5.0),
        (5.0, 12.0, 13.0),
        (8.0, 15.0, 17.0),
        (0.75, 1.0, 1.25),
    ];
    let bx = aabb(0.0, 0.0, 2.0, 2.0);
    for &(dx, dy, h) in triples {
        let p = v(2.0 + dx, 2.0 + dy); // clamps to the (2,2) corner
        check_ca(&l, "row43/corner", c2Circle { p, r: h }, bx);
        check_ca(&l, "row43/corner", c2Circle { p, r: -h }, bx);
        for &nudge in &[-1i32, 1] {
            let h2 = f32::from_bits((h.to_bits() as i64 + nudge as i64) as u32);
            check_ca(&l, "row43/ulp", c2Circle { p, r: h2 }, bx);
        }
        // Axis-aligned tangency: clamp lands on a face, distance == r exactly.
        check_ca(
            &l,
            "row43/face",
            circle(2.0 + h, 1.0, h),
            bx,
        );
        check_ca(&l, "row43/face", circle(-h, 1.0, h), bx);
        check_ca(&l, "row43/face", circle(1.0, 2.0 + h, h), bx);
        check_ca(&l, "row43/face", circle(1.0, -h, h), bx);
    }
}

#[test]
fn row44_45_ca_degenerate_and_inverted_boxes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 44);
    for _ in 0..5_000 {
        let a = v(rng.signed(50.0), rng.signed(50.0));
        let b = v(a.x + rng.signed(20.0), a.y + rng.signed(20.0));
        let boxes = [
            c2AABB { min: a, max: a },   // zero extent (a point)
            c2AABB { min: a, max: b },   // possibly inverted, random ordering
            c2AABB { min: b, max: a },   // the other ordering
            aabb(a.x, a.y, a.x - 1.0, a.y + 1.0), // inverted on x only
            aabb(a.x, a.y, a.x + 1.0, a.y - 1.0), // inverted on y only
            aabb(a.x, a.y, a.x - 1.0, a.y - 1.0), // inverted on both
        ];
        for bx in boxes {
            for &r in &[0.0f32, -2.0, 0.5, 3.0, 1e-30] {
                let p = v(a.x + rng.signed(5.0), a.y + rng.signed(5.0));
                check_ca(&l, "row44+45", c2Circle { p, r }, bx);
            }
        }
    }
}

#[test]
fn row46_ca_radius_edge_cases() {
    let l = libs();
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        1e-45,
        nan(0),
        nan(3),
    ];
    let boxes = [
        aabb(0.0, 0.0, 1.0, 1.0),
        aabb(-1.0, -1.0, 1.0, 1.0),
        aabb(0.0, 0.0, 0.0, 0.0),
        aabb(1.0, 1.0, -1.0, -1.0),
        aabb(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY),
    ];
    let pts = [v(0.5, 0.5), v(5.0, 5.0), v(-5.0, 0.5), v(0.0, 0.0)];
    for &r in &radii {
        for &b in &boxes {
            for &p in &pts {
                check_ca(&l, "row46", c2Circle { p, r }, b);
            }
        }
    }
}

#[test]
fn row47_ca_nan_and_inf_all_subsets() {
    let l = libs();
    // 7 floats: A.p.x, A.p.y, A.r, B.min.x, B.min.y, B.max.x, B.max.y
    let base = [0.5f32, 0.5, 1.0, 0.0, 0.0, 2.0, 2.0];
    let injects = [nan(0), nan(2), nan(3), f32::INFINITY, f32::NEG_INFINITY];
    for mask in 1u32..128 {
        for (ii, &inj) in injects.iter().enumerate() {
            let mut c = base;
            for k in 0..7 {
                if mask & (1 << k) != 0 {
                    c[k] = if inj.is_nan() { nan(ii + k) } else { inj };
                }
            }
            check_ca(
                &l,
                "row47",
                circle(c[0], c[1], c[2]),
                aabb(c[3], c[4], c[5], c[6]),
            );
        }
    }
}

#[test]
fn row48_ca_fully_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 48);
    for _ in 0..100_000 {
        check_ca(&l, "row48/raw", rng.raw_circle(), rng.raw_aabb());
    }
    for _ in 0..100_000 {
        check_ca(&l, "row48/nasty", rng.nasty_circle(), rng.nasty_aabb());
    }
}

// ===========================================================================
// c2AABBtoAABB — rows 49–57
// ===========================================================================

#[test]
fn row49_aa_random_overlapping() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 49);
    for _ in 0..30_000 {
        let a = aabb(
            rng.signed(100.0),
            rng.signed(100.0),
            0.0,
            0.0,
        );
        let a = aabb(a.min.x, a.min.y, a.min.x + 5.0, a.min.y + 5.0);
        // Offset by less than the extent ⇒ guaranteed overlap.
        let ox = rng.signed(4.9);
        let oy = rng.signed(4.9);
        let b = aabb(a.min.x + ox, a.min.y + oy, a.max.x + ox, a.max.y + oy);
        check_aa(&l, "row49", a, b);
    }
}

#[test]
fn row50_aa_separated_on_each_axis() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 50);
    for _ in 0..10_000 {
        let a = {
            let p = v(rng.signed(100.0), rng.signed(100.0));
            aabb(p.x, p.y, p.x + 3.0, p.y + 3.0)
        };
        let gap = 1.0 + rng.signed(10.0).abs();
        // d1: A.max.x < B.min.x
        check_aa(
            &l,
            "row50/d1",
            a,
            aabb(a.max.x + gap, a.min.y, a.max.x + gap + 3.0, a.max.y),
        );
        // d0: B.max.x < A.min.x
        check_aa(
            &l,
            "row50/d0",
            a,
            aabb(a.min.x - gap - 3.0, a.min.y, a.min.x - gap, a.max.y),
        );
        // d3: A.max.y < B.min.y
        check_aa(
            &l,
            "row50/d3",
            a,
            aabb(a.min.x, a.max.y + gap, a.max.x, a.max.y + gap + 3.0),
        );
        // d2: B.max.y < A.min.y
        check_aa(
            &l,
            "row50/d2",
            a,
            aabb(a.min.x, a.min.y - gap - 3.0, a.max.x, a.min.y - gap),
        );
        // separated on both axes at once
        check_aa(
            &l,
            "row50/both",
            a,
            aabb(
                a.max.x + gap,
                a.max.y + gap,
                a.max.x + gap + 3.0,
                a.max.y + gap + 3.0,
            ),
        );
    }
}

#[test]
fn row51_aa_exactly_touching_each_edge() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 51);
    for _ in 0..10_000 {
        let p = v(rng.signed(100.0), rng.signed(100.0));
        let a = aabb(p.x, p.y, p.x + 3.0, p.y + 3.0);
        // Exactly touching: strict `<` is false, so C returns 1.
        let touching = [
            aabb(a.max.x, a.min.y, a.max.x + 3.0, a.max.y), // right edge
            aabb(a.min.x - 3.0, a.min.y, a.min.x, a.max.y), // left edge
            aabb(a.min.x, a.max.y, a.max.x, a.max.y + 3.0), // top edge
            aabb(a.min.x, a.min.y - 3.0, a.max.x, a.min.y), // bottom edge
            aabb(a.max.x, a.max.y, a.max.x + 1.0, a.max.y + 1.0), // corner touch
        ];
        for b in touching {
            check_aa(&l, "row51/touch", a, b);
            check_aa(&l, "row51/touch.swap", b, a);
        }
        // One ULP separated on each side — must flip to 0.
        let up = |f: f32| f32::from_bits(f.to_bits() + 1);
        check_aa(
            &l,
            "row51/ulp",
            a,
            aabb(up(a.max.x.abs()), a.min.y, a.max.x + 3.0, a.max.y),
        );
    }
}

#[test]
fn row52_53_aa_containment_identity_and_points() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 52);
    for _ in 0..10_000 {
        let p = v(rng.signed(100.0), rng.signed(100.0));
        let outer = aabb(p.x - 10.0, p.y - 10.0, p.x + 10.0, p.y + 10.0);
        let inner = aabb(p.x - 1.0, p.y - 1.0, p.x + 1.0, p.y + 1.0);
        let point = aabb(p.x, p.y, p.x, p.y);
        let far_point = aabb(p.x + 50.0, p.y, p.x + 50.0, p.y);
        check_aa(&l, "row52/contain", outer, inner);
        check_aa(&l, "row52/contain.swap", inner, outer);
        check_aa(&l, "row52/identical", outer, outer);
        check_aa(&l, "row53/point-in-box", point, outer);
        check_aa(&l, "row53/box-in-point", outer, point);
        check_aa(&l, "row53/point-point", point, point);
        check_aa(&l, "row53/point-far", point, far_point);
        // Point exactly on the outer boundary / corner.
        for &q in &[
            v(outer.min.x, outer.min.y),
            v(outer.max.x, outer.max.y),
            v(outer.min.x, outer.max.y),
            v(outer.max.x, p.y),
        ] {
            check_aa(&l, "row53/point-on-edge", aabb(q.x, q.y, q.x, q.y), outer);
        }
    }
}

#[test]
fn row54_aa_inverted_boxes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 54);
    for _ in 0..20_000 {
        let p = v(rng.signed(50.0), rng.signed(50.0));
        let q = v(rng.signed(50.0), rng.signed(50.0));
        // Random min/max with no ordering guarantee ⇒ mixture of valid and
        // inverted boxes on either operand.
        let a = c2AABB { min: p, max: q };
        let b = c2AABB { min: q, max: p };
        check_aa(&l, "row54", a, b);
        check_aa(&l, "row54", a, a);
        check_aa(&l, "row54", b, b);
        // Explicitly inverted on one axis only.
        check_aa(
            &l,
            "row54/onaxis",
            aabb(p.x, p.y, p.x - 1.0, p.y + 1.0),
            aabb(q.x, q.y, q.x + 1.0, q.y - 1.0),
        );
    }
}

#[test]
fn row55_aa_nan_all_255_subsets() {
    let l = libs();
    let base = [0.0f32, 0.0, 2.0, 2.0, 1.0, 1.0, 3.0, 3.0];
    for mask in 1u32..256 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..8 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            check_aa(
                &l,
                "row55",
                aabb(c[0], c[1], c[2], c[3]),
                aabb(c[4], c[5], c[6], c[7]),
            );
        }
    }
}

#[test]
fn row56_aa_infinities() {
    let l = libs();
    let inf_box = aabb(
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::INFINITY,
    );
    let empty_inf = aabb(
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    let others = [
        aabb(0.0, 0.0, 1.0, 1.0),
        aabb(-1e30, -1e30, 1e30, 1e30),
        inf_box,
        empty_inf,
        aabb(f32::NEG_INFINITY, 0.0, 0.0, 1.0),
        aabb(0.0, f32::NEG_INFINITY, 1.0, f32::INFINITY),
        aabb(f32::MAX, f32::MIN, f32::MIN, f32::MAX),
    ];
    for &a in &others {
        for &b in &others {
            check_aa(&l, "row56", a, b);
        }
    }
    // Exhaustive over ±inf / 0 in all 8 slots would be 6561 cases — sample the
    // full 3^8 grid of {-inf, 0, +inf}.
    let vals = [f32::NEG_INFINITY, 0.0f32, f32::INFINITY];
    for i in 0..3usize.pow(8) {
        let mut n = i;
        let mut c = [0.0f32; 8];
        for k in 0..8 {
            c[k] = vals[n % 3];
            n /= 3;
        }
        check_aa(
            &l,
            "row56/grid",
            aabb(c[0], c[1], c[2], c[3]),
            aabb(c[4], c[5], c[6], c[7]),
        );
    }
}

#[test]
fn row57_aa_fully_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 57);
    for _ in 0..100_000 {
        check_aa(&l, "row57/raw", rng.raw_aabb(), rng.raw_aabb());
    }
    for _ in 0..100_000 {
        check_aa(&l, "row57/nasty", rng.nasty_aabb(), rng.nasty_aabb());
    }
}

// ===========================================================================
// collided — rows 58–66
// ===========================================================================

/// Call `collided` on both libraries with the given payload bytes, honouring an
/// arbitrary byte offset so unaligned pointers can be tested.
fn collided_bytes(
    l: &Pair,
    row: &str,
    abytes: &[u8],
    ta: C2_TYPE,
    bbytes: &[u8],
    tb: C2_TYPE,
    offset: usize,
) {
    let mut buf_a = vec![0u8; abytes.len() + offset + 8];
    let mut buf_b = vec![0u8; bbytes.len() + offset + 8];
    buf_a[offset..offset + abytes.len()].copy_from_slice(abytes);
    buf_b[offset..offset + bbytes.len()].copy_from_slice(bbytes);
    let pa = unsafe { buf_a.as_ptr().add(offset) } as *const c_void;
    let pb = unsafe { buf_b.as_ptr().add(offset) } as *const c_void;
    let cv = l.c.collided_raw(pa, ta, pb, tb);
    let rv = l.rs.collided_raw(pa, ta, pb, tb);
    assert_int_eq!(
        row,
        cv,
        rv,
        "collided(ta={ta}, tb={tb}, offset={offset}, A={abytes:02x?}, B={bbytes:02x?})"
    );
}

fn circle_bytes(c: c2Circle) -> [u8; 12] {
    let mut o = [0u8; 12];
    o[0..4].copy_from_slice(&c.p.x.to_bits().to_ne_bytes());
    o[4..8].copy_from_slice(&c.p.y.to_bits().to_ne_bytes());
    o[8..12].copy_from_slice(&c.r.to_bits().to_ne_bytes());
    o
}

fn aabb_bytes(b: c2AABB) -> [u8; 16] {
    let mut o = [0u8; 16];
    o[0..4].copy_from_slice(&b.min.x.to_bits().to_ne_bytes());
    o[4..8].copy_from_slice(&b.min.y.to_bits().to_ne_bytes());
    o[8..12].copy_from_slice(&b.max.x.to_bits().to_ne_bytes());
    o[12..16].copy_from_slice(&b.max.y.to_bits().to_ne_bytes());
    o
}

#[test]
fn row58_collided_circle_circle() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 58);
    for _ in 0..40_000 {
        let (a, b) = (rng.nasty_circle(), rng.nasty_circle());
        collided_bytes(
            &l,
            "row58",
            &circle_bytes(a),
            C2_TYPE_CIRCLE,
            &circle_bytes(b),
            C2_TYPE_CIRCLE,
            0,
        );
        // Cross-check the dispatcher against the direct entry point (row 66).
        let direct = l.c.c2CircletoCircle(a, b);
        let via = l.rs.collided_raw(
            &a as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            &b as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
        );
        assert_int_eq!("row66/cc", direct, via, "dispatch composition");
    }
}

#[test]
fn row59_collided_circle_aabb() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 59);
    for _ in 0..40_000 {
        let (a, b) = (rng.nasty_circle(), rng.nasty_aabb());
        collided_bytes(
            &l,
            "row59",
            &circle_bytes(a),
            C2_TYPE_CIRCLE,
            &aabb_bytes(b),
            C2_TYPE_AABB,
            0,
        );
        let direct = l.c.c2CircletoAABB(a, b);
        let via = l.rs.collided_raw(
            &a as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            &b as *const _ as *const c_void,
            C2_TYPE_AABB,
        );
        assert_int_eq!("row66/ca", direct, via, "dispatch composition");
    }
}

#[test]
fn row60_collided_aabb_circle_argument_swap() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 60);
    for _ in 0..40_000 {
        // typeA = AABB, typeB = CIRCLE: the C calls
        // c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A) — arguments SWAPPED.
        let bx = rng.nasty_aabb();
        let ci = rng.nasty_circle();
        collided_bytes(
            &l,
            "row60",
            &aabb_bytes(bx),
            C2_TYPE_AABB,
            &circle_bytes(ci),
            C2_TYPE_CIRCLE,
            0,
        );
        // Must equal c2CircletoAABB(circle, box), NOT the other order.
        let expect = l.c.c2CircletoAABB(ci, bx);
        let via = l.rs.collided_raw(
            &bx as *const _ as *const c_void,
            C2_TYPE_AABB,
            &ci as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
        );
        assert_int_eq!("row60/swap", expect, via, "AABB-vs-circle argument swap");
    }
}

#[test]
fn row61_collided_aabb_aabb() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 61);
    for _ in 0..40_000 {
        let (a, b) = (rng.nasty_aabb(), rng.nasty_aabb());
        collided_bytes(
            &l,
            "row61",
            &aabb_bytes(a),
            C2_TYPE_AABB,
            &aabb_bytes(b),
            C2_TYPE_AABB,
            0,
        );
        let direct = l.c.c2AABBtoAABB(a, b);
        let via = l.rs.collided_raw(
            &a as *const _ as *const c_void,
            C2_TYPE_AABB,
            &b as *const _ as *const c_void,
            C2_TYPE_AABB,
        );
        assert_int_eq!("row66/aa", direct, via, "dispatch composition");
    }
}

#[test]
fn row62_collided_all_tag_pairs_raw_bytes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 62);
    for _ in 0..100_000 {
        // 16 fully random bytes are a valid payload for either struct type.
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        for k in 0..4 {
            a[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
            b[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
        }
        for &(ta, tb) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            collided_bytes(&l, "row62", &a, ta, &b, tb, 0);
        }
    }
}

#[test]
fn row63_collided_unaligned_buffers() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 63);
    for _ in 0..5_000 {
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        for k in 0..4 {
            a[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
            b[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
        }
        for offset in [1usize, 2, 3, 5, 7] {
            for &(ta, tb) in &[
                (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
                (C2_TYPE_CIRCLE, C2_TYPE_AABB),
                (C2_TYPE_AABB, C2_TYPE_CIRCLE),
                (C2_TYPE_AABB, C2_TYPE_AABB),
            ] {
                collided_bytes(&l, "row63", &a, ta, &b, tb, offset);
            }
        }
    }
}

#[test]
fn row64_collided_aliasing_same_buffer() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 64);
    for _ in 0..30_000 {
        let mut buf = [0u8; 16];
        for k in 0..4 {
            buf[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
        }
        let p = buf.as_ptr() as *const c_void;
        for &(ta, tb) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            assert_int_eq!(
                "row64",
                l.c.collided_raw(p, ta, p, tb),
                l.rs.collided_raw(p, ta, p, tb),
                "self-collision ta={ta} tb={tb} buf={buf:02x?}"
            );
        }
    }
}

#[test]
fn row65_collided_tag_pointee_size_mismatch() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 65);
    for _ in 0..30_000 {
        // A 12-byte circle buffer read as a 16-byte AABB: the 4 bytes past the
        // end are whatever the surrounding allocation holds, so pin them with a
        // 16-byte buffer whose tail is known.
        let c = rng.nasty_circle();
        let mut buf = [0u8; 16];
        buf[0..12].copy_from_slice(&circle_bytes(c));
        buf[12..16].copy_from_slice(&rng.next_u32().to_ne_bytes());
        // AABB tag over circle-shaped data.
        collided_bytes(&l, "row65/aabb-over-circle", &buf, C2_TYPE_AABB, &buf, C2_TYPE_AABB, 0);
        collided_bytes(&l, "row65/aabb-over-circle", &buf, C2_TYPE_AABB, &buf, C2_TYPE_CIRCLE, 0);
        // Circle tag over AABB-shaped data (last 4 bytes ignored).
        let bx = rng.nasty_aabb();
        let ab = aabb_bytes(bx);
        collided_bytes(&l, "row65/circle-over-aabb", &ab, C2_TYPE_CIRCLE, &ab, C2_TYPE_CIRCLE, 0);
        collided_bytes(&l, "row65/circle-over-aabb", &ab, C2_TYPE_CIRCLE, &ab, C2_TYPE_AABB, 0);
        collided_bytes(&l, "row65/circle-over-aabb", &ab, C2_TYPE_AABB, &ab, C2_TYPE_CIRCLE, 0);
    }
}
