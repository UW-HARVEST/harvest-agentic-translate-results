//! Phase B — valid-path differential tests for the SHAPE predicates.
//!
//! Covers `CONFIGS.md` rows 11-24: `c2CircletoCircle`, `c2CircletoAABB` and
//! `c2CircletoCapsule` driven through both `.so`s, every branch arm targeted
//! deliberately AND fuzzed with a fixed-seed PRNG.

mod common;

use common::*;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// c2CircletoCircle — rows 11-13
// ---------------------------------------------------------------------------

/// Rows 11 + 12 — random circles; the generator is arranged so that both the
/// overlapping (returns 1) and separated (returns 0) outcomes occur, and both
/// counts are asserted non-zero so the row genuinely covers both.
#[test]
fn row11_12_circle_circle_random() {
    let p = load();
    let mut rng = Rng::default_seeded();
    let mut hits = 0usize;
    let mut misses = 0usize;

    for _ in 0..N {
        let a = rng.circle();
        let b = rng.circle();
        let (c, rs) = unsafe { ((p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b)) };
        assert_int_eq(&format!("c2CircletoCircle({a:?},{b:?})"), c, rs);
        if c != 0 {
            hits += 1
        } else {
            misses += 1
        }

        // Deliberately overlapping: place B within A's radius.
        let near = C2Circle {
            p: v(a.p.x + rng.range(-1.0, 1.0) * a.r, a.p.y),
            r: a.r.abs() * 0.5,
        };
        let (c, rs) = unsafe {
            (
                (p.c.c2CircletoCircle)(a, near),
                (p.rs.c2CircletoCircle)(a, near),
            )
        };
        assert_int_eq("c2CircletoCircle overlapping", c, rs);
        if c != 0 {
            hits += 1
        } else {
            misses += 1
        }

        // Deliberately far apart.
        let far = C2Circle {
            p: v(a.p.x + 1.0e5, a.p.y - 1.0e5),
            r: 1.0,
        };
        let (c, rs) =
            unsafe { ((p.c.c2CircletoCircle)(a, far), (p.rs.c2CircletoCircle)(a, far)) };
        assert_int_eq("c2CircletoCircle separated", c, rs);
        if c != 0 {
            hits += 1
        } else {
            misses += 1
        }
    }

    assert!(hits > 0 && misses > 0, "row not covering both outcomes: hits={hits} misses={misses}");
}

/// Row 13 — exact tangency (`d2 == r2`, strict `<` must reject), concentric
/// circles, zero radius, negative radius, and radius-sum overflow to `inf`.
#[test]
fn row13_circle_circle_boundaries() {
    let p = load();
    let mut rng = Rng::default_seeded();

    let check = |a: C2Circle, b: C2Circle, what: &str| {
        let (c, rs) = unsafe { ((p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b)) };
        assert_int_eq(what, c, rs);
    };

    // Exact tangency along x: distance == r1 + r2 exactly (powers of two keep
    // the arithmetic exact).
    for &(r1, r2) in &[
        (1.0f32, 1.0f32),
        (2.0, 4.0),
        (0.25, 0.75),
        (16.0, 16.0),
        (0.0, 0.0),
        (0.0, 4.0),
    ] {
        let d = r1 + r2;
        let a = C2Circle { p: v(0.0, 0.0), r: r1 };
        for (label, dx) in [
            ("exact", d),
            ("just inside", d - d * f32::EPSILON * 4.0),
            ("just outside", d + d * f32::EPSILON * 4.0),
        ] {
            let b = C2Circle { p: v(dx, 0.0), r: r2 };
            check(a, b, &format!("tangency {label} r=({r1},{r2})"));
        }
    }

    // Concentric / identical.
    for _ in 0..2000 {
        let a = rng.circle();
        check(a, a, "concentric self");
        check(
            a,
            C2Circle { p: a.p, r: -a.r },
            "concentric negated radius",
        );
    }

    // Zero, negative, huge, infinite and NaN radii against random positions.
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -1.0e30,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
    ];
    for &r1 in &radii {
        for &r2 in &radii {
            for _ in 0..40 {
                let a = C2Circle { p: rng.vec_coord(), r: r1 };
                let b = C2Circle { p: rng.vec_coord(), r: r2 };
                check(a, b, &format!("radii({r1:?},{r2:?})"));
            }
            // Extreme positions too (inf - inf = NaN inside c2Sub).
            for &px in EDGE_F32 {
                let a = C2Circle { p: v(px, px), r: r1 };
                let b = C2Circle { p: v(-px, px), r: r2 };
                check(a, b, &format!("edge pos {px:?} radii({r1:?},{r2:?})"));
            }
        }
    }

    // Fully random bit patterns.
    for _ in 0..N {
        let a = C2Circle { p: rng.vec_any(), r: rng.any_bits_f32() };
        let b = C2Circle { p: rng.vec_any(), r: rng.any_bits_f32() };
        check(a, b, "random-bits");
    }
}

// ---------------------------------------------------------------------------
// c2CircletoAABB — rows 14-18
// ---------------------------------------------------------------------------

/// Rows 14-16 — the clamp regions: centre inside the box, on each of the 4
/// edges (one axis clamped) and in each of the 4 corners (both axes clamped).
#[test]
fn row14_16_circle_aabb_regions() {
    let p = load();
    let mut rng = Rng::default_seeded();

    let check = |a: C2Circle, b: C2Aabb, what: &str| {
        let (c, rs) = unsafe { ((p.c.c2CircletoAABB)(a, b), (p.rs.c2CircletoAABB)(a, b)) };
        assert_int_eq(what, c, rs);
    };

    for _ in 0..N {
        let b = rng.aabb_sorted();
        let (w, h) = (b.max.x - b.min.x, b.max.y - b.min.y);
        let r = rng.radius();
        let mid = v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);

        // Row 14: inside (clamp is the identity, d2 == 0).
        check(C2Circle { p: mid, r }, b, "aabb inside");

        // Row 15: the 4 edge regions — outside on exactly one axis.
        for (label, q) in [
            ("left", v(b.min.x - w - 1.0, mid.y)),
            ("right", v(b.max.x + w + 1.0, mid.y)),
            ("below", v(mid.x, b.min.y - h - 1.0)),
            ("above", v(mid.x, b.max.y + h + 1.0)),
        ] {
            check(C2Circle { p: q, r }, b, &format!("aabb edge {label}"));
            // and just barely outside, where the radius decides
            let q2 = v(
                mid.x + (q.x - mid.x).signum() * (w * 0.5 + r.abs() * 0.5),
                mid.y + (q.y - mid.y).signum() * (h * 0.5 + r.abs() * 0.5),
            );
            check(C2Circle { p: q2, r }, b, &format!("aabb near-edge {label}"));
        }

        // Row 16: the 4 corner regions — outside on both axes.
        for (label, q) in [
            ("bl", v(b.min.x - 1.0, b.min.y - 1.0)),
            ("br", v(b.max.x + 1.0, b.min.y - 1.0)),
            ("tl", v(b.min.x - 1.0, b.max.y + 1.0)),
            ("tr", v(b.max.x + 1.0, b.max.y + 1.0)),
        ] {
            check(C2Circle { p: q, r }, b, &format!("aabb corner {label}"));
        }

        // Fully random placement.
        check(rng.circle(), b, "aabb random");
    }
}

/// Rows 17 + 18 — degenerate / thin / inverted boxes, and zero / negative /
/// huge / infinite / NaN radii, plus fully random bit patterns.
#[test]
fn row17_18_circle_aabb_degenerate() {
    let p = load();
    let mut rng = Rng::default_seeded();

    let check = |a: C2Circle, b: C2Aabb, what: &str| {
        let (c, rs) = unsafe { ((p.c.c2CircletoAABB)(a, b), (p.rs.c2CircletoAABB)(a, b)) };
        assert_int_eq(what, c, rs);
    };

    for _ in 0..N {
        let s = rng.aabb_sorted();
        let a = rng.circle();

        // Degenerate: point box.
        check(a, C2Aabb { min: s.min, max: s.min }, "aabb point");
        // Thin: zero width, non-zero height (and vice versa).
        check(
            a,
            C2Aabb { min: s.min, max: v(s.min.x, s.max.y) },
            "aabb zero-width",
        );
        check(
            a,
            C2Aabb { min: s.min, max: v(s.max.x, s.min.y) },
            "aabb zero-height",
        );
        // Inverted: min > max on both axes.
        check(a, C2Aabb { min: s.max, max: s.min }, "aabb inverted");
        // Inverted on one axis only.
        check(
            a,
            C2Aabb { min: v(s.max.x, s.min.y), max: v(s.min.x, s.max.y) },
            "aabb inverted-x",
        );
        // Unsorted random corners.
        check(a, rng.aabb_raw(), "aabb raw");
    }

    // Row 18: radius axis. r == 0 can never collide (d2 < 0 is impossible).
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -1.0e20,
        1.0e20,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &r in &radii {
        for _ in 0..500 {
            let b = rng.aabb_sorted();
            let mid = v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);
            check(C2Circle { p: mid, r }, b, &format!("aabb r={r:?} inside"));
            check(C2Circle { p: rng.vec_coord(), r }, b, &format!("aabb r={r:?} random"));
        }
        for &e in EDGE_F32 {
            let b = C2Aabb { min: v(-e, -e), max: v(e, e) };
            check(C2Circle { p: v(e, -e), r }, b, &format!("aabb edge {e:?} r={r:?}"));
        }
    }

    for _ in 0..N {
        let a = C2Circle { p: rng.vec_any(), r: rng.any_bits_f32() };
        let b = C2Aabb { min: rng.vec_any(), max: rng.vec_any() };
        check(a, b, "aabb random-bits");
    }
}

// ---------------------------------------------------------------------------
// c2CircletoCapsule — rows 19-24
// ---------------------------------------------------------------------------

/// Rows 19-22 — each of the three branch arms hit deliberately, plus the
/// `da == 0` / `db == 0` boundaries where both `<` tests are false.
#[test]
fn row19_22_circle_capsule_arms() {
    let p = load();
    let mut rng = Rng::default_seeded();

    let check = |a: C2Circle, b: C2Capsule, what: &str| {
        let (c, rs) = unsafe {
            (
                (p.c.c2CircletoCapsule)(a, b),
                (p.rs.c2CircletoCapsule)(a, b),
            )
        };
        assert_int_eq(what, c, rs);
    };

    let mut arm_a = 0usize;
    let mut arm_side = 0usize;
    let mut arm_b = 0usize;

    for _ in 0..N {
        // Non-degenerate segment.
        let start = rng.vec_coord();
        let dir = {
            let d = v(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
            let l = (d.x * d.x + d.y * d.y).sqrt();
            if l == 0.0 || !l.is_finite() {
                v(1.0, 0.0)
            } else {
                v(d.x / l, d.y / l)
            }
        };
        let len = rng.range(0.001, 200.0);
        let end = v(start.x + dir.x * len, start.y + dir.y * len);
        let cap = C2Capsule { a: start, b: end, r: rng.radius() };
        let n = v(end.x - start.x, end.y - start.y);
        let perp = v(-dir.y, dir.x);

        // Arm 1: da < 0 — before endpoint `a`.
        let t = -rng.range(0.01, 5.0);
        let q = v(start.x + n.x * t, start.y + n.y * t);
        check(C2Circle { p: q, r: rng.radius() }, cap, "capsule arm da<0");
        arm_a += 1;

        // Arm 2: da >= 0 && db < 0 — beside the segment (the division arm).
        let t = rng.range(0.0, 1.0);
        let off = rng.range(-100.0, 100.0);
        let q = v(
            start.x + n.x * t + perp.x * off,
            start.y + n.y * t + perp.y * off,
        );
        check(C2Circle { p: q, r: rng.radius() }, cap, "capsule arm side");
        arm_side += 1;

        // Arm 3: da >= 0 && db >= 0 — past endpoint `b`.
        let t = 1.0 + rng.range(0.01, 5.0);
        let q = v(start.x + n.x * t, start.y + n.y * t);
        check(C2Circle { p: q, r: rng.radius() }, cap, "capsule arm db>=0");
        arm_b += 1;

        // Row 22: da == 0 exactly (circle centre on the plane through `a`)
        // and db == 0 exactly (plane through `b`).
        let q = v(start.x + perp.x * off, start.y + perp.y * off);
        check(C2Circle { p: q, r: rng.radius() }, cap, "capsule da==0");
        let q = v(end.x + perp.x * off, end.y + perp.y * off);
        check(C2Circle { p: q, r: rng.radius() }, cap, "capsule db==0");

        // Exactly at the endpoints.
        check(C2Circle { p: start, r: rng.radius() }, cap, "capsule at a");
        check(C2Circle { p: end, r: rng.radius() }, cap, "capsule at b");

        // Fully random circle vs this capsule.
        check(rng.circle(), cap, "capsule random circle");
    }

    assert!(arm_a > 0 && arm_side > 0 && arm_b > 0, "not all capsule arms exercised");
}

/// Rows 23 + 24 — degenerate (`a == b`) / axis-aligned / oblique segments,
/// zero / negative / huge radii, denormal-length segments that make
/// `da / dot(n,n)` overflow, and fully random bit patterns.
#[test]
fn row23_24_circle_capsule_degenerate() {
    let p = load();
    let mut rng = Rng::default_seeded();

    let check = |a: C2Circle, b: C2Capsule, what: &str| {
        let (c, rs) = unsafe {
            (
                (p.c.c2CircletoCapsule)(a, b),
                (p.rs.c2CircletoCapsule)(a, b),
            )
        };
        assert_int_eq(what, c, rs);
    };

    for _ in 0..N {
        let a0 = rng.vec_coord();
        let a1 = rng.vec_coord();
        let r = rng.radius();
        let circ = rng.circle();

        // Degenerate segment: dot(n, n) == 0 -> division by zero if reached.
        check(circ, C2Capsule { a: a0, b: a0, r }, "capsule degenerate");
        check(
            C2Circle { p: a0, r: circ.r },
            C2Capsule { a: a0, b: a0, r },
            "capsule degenerate coincident",
        );
        // Axis-aligned.
        check(
            circ,
            C2Capsule { a: a0, b: v(a1.x, a0.y), r },
            "capsule horizontal",
        );
        check(
            circ,
            C2Capsule { a: a0, b: v(a0.x, a1.y), r },
            "capsule vertical",
        );
        // Oblique.
        check(circ, C2Capsule { a: a0, b: a1, r }, "capsule oblique");
        // Reversed orientation.
        check(circ, C2Capsule { a: a1, b: a0, r }, "capsule reversed");
    }

    // Denormal / tiny segments: dot(n,n) underflows so da/dot(n,n) overflows.
    let tinies = [1.0e-45f32, 1.0e-40, f32::MIN_POSITIVE, 1.0e-30, 1.0e-20];
    for &t in &tinies {
        for _ in 0..500 {
            let a0 = rng.vec_coord();
            let cap = C2Capsule { a: a0, b: v(a0.x + t, a0.y + t), r: rng.radius() };
            check(rng.circle(), cap, &format!("capsule tiny {t:?}"));
            // Circle placed so that da >= 0 and db < 0 (forces the division).
            let q = v(a0.x + t * 0.5 - 1.0, a0.y + t * 0.5 + 1.0);
            check(C2Circle { p: q, r: rng.radius() }, cap, &format!("capsule tiny side {t:?}"));
        }
    }

    // Extreme radii and extreme coordinates.
    let radii = [
        0.0f32,
        -0.0,
        -5.0,
        1.0e30,
        -1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &r in &radii {
        for &e in EDGE_F32 {
            let cap = C2Capsule { a: v(e, -e), b: v(-e, e), r };
            check(C2Circle { p: v(e, e), r }, cap, &format!("capsule edge {e:?} r={r:?}"));
            check(C2Circle { p: v(0.0, 0.0), r: e }, cap, &format!("capsule edge2 {e:?} r={r:?}"));
        }
        for _ in 0..500 {
            check(rng.circle(), C2Capsule { a: rng.vec_coord(), b: rng.vec_coord(), r }, "capsule r sweep");
        }
    }

    for _ in 0..N {
        let a = C2Circle { p: rng.vec_any(), r: rng.any_bits_f32() };
        let b = C2Capsule { a: rng.vec_any(), b: rng.vec_any(), r: rng.any_bits_f32() };
        check(a, b, "capsule random-bits");
    }
}
