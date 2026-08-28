//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the results bit-for-bit. Randomized rows use the fixed-seed
//! PRNG in `common` so failures are reproducible.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_void;

// ===========================================================================
// Row 1 — c2V
// ===========================================================================

#[test]
fn row01_c2V_all_float_classes() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1001);
    for i in 0..ITERS {
        let (x, y) = (rng.wild(), rng.wild());
        let ctx = format!("iter {i}: x=0x{:08x} y=0x{:08x}", fb(x), fb(y));
        let cv = unsafe { (c.c2V)(x, y) };
        let rv = unsafe { (r.c2V)(x, y) };
        eq_v("row01", &ctx, cv, rv);
        // c2V is a pure copy: the payload must survive untouched.
        assert_eq!(vb(cv), (fb(x), fb(y)), "[row01] C mutated the bits: {ctx}");
    }
}

// ===========================================================================
// Rows 2-3 — c2Maxv
// ===========================================================================

#[test]
fn row02_c2Maxv_finite() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2002);
    for i in 0..ITERS {
        // Small quantised grid ⇒ plenty of exact ties in both lanes.
        let (a, b) = (v(rng.coord(4.0), rng.coord(4.0)), v(rng.coord(4.0), rng.coord(4.0)));
        let ctx = format!("iter {i}: a={} b={}", show_v(a), show_v(b));
        eq_v("row02", &ctx, unsafe { (c.c2Maxv)(a, b) }, unsafe { (r.c2Maxv)(a, b) });
    }
}

#[test]
fn row03_c2Maxv_nan_and_signed_zero() {
    let (c, r) = both();
    let interesting = [
        0.0f32,
        -0.0f32,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7fc0_dead),
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_beef),
        f32::from_bits(0xff80_0001),
    ];
    // Full cross product over both lanes and both operands.
    for &ax in &interesting {
        for &bx in &interesting {
            for &ay in &interesting {
                for &by in &interesting {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let ctx = format!("a={} b={}", show_v(a), show_v(b));
                    eq_v("row03", &ctx, unsafe { (c.c2Maxv)(a, b) }, unsafe { (r.c2Maxv)(a, b) });
                }
            }
        }
    }
    // Plus randomized wild vectors.
    let mut rng = Rng::new(0x3003);
    for i in 0..ITERS {
        let (a, b) = (rng.v_wild(), rng.v_wild());
        let ctx = format!("rand {i}: a={} b={}", show_v(a), show_v(b));
        eq_v("row03", &ctx, unsafe { (c.c2Maxv)(a, b) }, unsafe { (r.c2Maxv)(a, b) });
    }
}

// ===========================================================================
// Rows 4-5 — c2Minv
// ===========================================================================

#[test]
fn row04_c2Minv_finite() {
    let (c, r) = both();
    let mut rng = Rng::new(0x4004);
    for i in 0..ITERS {
        let (a, b) = (v(rng.coord(4.0), rng.coord(4.0)), v(rng.coord(4.0), rng.coord(4.0)));
        let ctx = format!("iter {i}: a={} b={}", show_v(a), show_v(b));
        eq_v("row04", &ctx, unsafe { (c.c2Minv)(a, b) }, unsafe { (r.c2Minv)(a, b) });
    }
}

#[test]
fn row05_c2Minv_nan_and_signed_zero() {
    let (c, r) = both();
    let interesting = [
        0.0f32,
        -0.0f32,
        2.0,
        -2.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7fc1_2345),
        f32::from_bits(0x7f80_0002),
        f32::from_bits(0xffc0_0001),
    ];
    for &ax in &interesting {
        for &bx in &interesting {
            for &ay in &interesting {
                for &by in &interesting {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let ctx = format!("a={} b={}", show_v(a), show_v(b));
                    eq_v("row05", &ctx, unsafe { (c.c2Minv)(a, b) }, unsafe { (r.c2Minv)(a, b) });
                }
            }
        }
    }
    let mut rng = Rng::new(0x5005);
    for i in 0..ITERS {
        let (a, b) = (rng.v_wild(), rng.v_wild());
        let ctx = format!("rand {i}: a={} b={}", show_v(a), show_v(b));
        eq_v("row05", &ctx, unsafe { (c.c2Minv)(a, b) }, unsafe { (r.c2Minv)(a, b) });
    }
}

// ===========================================================================
// Rows 6-8 — c2Clampv
// ===========================================================================

#[test]
fn row06_c2Clampv_wellformed_all_nine_regions() {
    let (c, r) = both();
    let mut rng = Rng::new(0x6006);
    let mut regions = [0usize; 9];
    for i in 0..ITERS {
        let b = rng.b_small(); // lo <= hi
        let (lo, hi) = (b.min, b.max);
        // Point drawn from a wider range than the box ⇒ hits below/inside/above.
        let a = v(rng.coord(10.0), rng.coord(10.0));
        let rx = if a.x < lo.x { 0 } else if a.x > hi.x { 2 } else { 1 };
        let ry = if a.y < lo.y { 0 } else if a.y > hi.y { 2 } else { 1 };
        regions[ry * 3 + rx] += 1;
        let ctx = format!("iter {i}: a={} lo={} hi={}", show_v(a), show_v(lo), show_v(hi));
        eq_v("row06", &ctx, unsafe { (c.c2Clampv)(a, lo, hi) }, unsafe {
            (r.c2Clampv)(a, lo, hi)
        });
    }
    assert!(
        regions.iter().all(|&n| n > 0),
        "[row06] not all 9 clamp regions exercised: {regions:?}"
    );
}

#[test]
fn row07_c2Clampv_inverted_box() {
    let (c, r) = both();
    let mut rng = Rng::new(0x7007);
    for i in 0..ITERS {
        // Deliberately inverted on one or both axes.
        let (lo, hi) = match i % 3 {
            0 => (v(3.0, 3.0), v(-3.0, -3.0)),                 // both inverted
            1 => (v(rng.small().abs(), -4.0), v(-1.0, 4.0)),    // x inverted only
            _ => (v(-4.0, rng.small().abs()), v(4.0, -1.0)),    // y inverted only
        };
        let a = v(rng.coord(8.0), rng.coord(8.0));
        let ctx = format!("iter {i}: a={} lo={} hi={}", show_v(a), show_v(lo), show_v(hi));
        eq_v("row07", &ctx, unsafe { (c.c2Clampv)(a, lo, hi) }, unsafe {
            (r.c2Clampv)(a, lo, hi)
        });
    }
}

#[test]
fn row08_c2Clampv_degenerate_and_nonfinite() {
    let (c, r) = both();
    // Degenerate: lo == hi, including ±0.0 pairs.
    let pts = [
        0.0f32,
        -0.0f32,
        1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7f80_0003),
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
    ];
    for &a in &pts {
        for &lo in &pts {
            for &hi in &pts {
                let (av, lov, hiv) = (v(a, lo), v(lo, hi), v(hi, a));
                let ctx = format!("a={} lo={} hi={}", show_v(av), show_v(lov), show_v(hiv));
                eq_v("row08", &ctx, unsafe { (c.c2Clampv)(av, lov, hiv) }, unsafe {
                    (r.c2Clampv)(av, lov, hiv)
                });
            }
        }
    }
    let mut rng = Rng::new(0x8008);
    for i in 0..ITERS {
        let (a, lo, hi) = (rng.v_wild(), rng.v_wild(), rng.v_wild());
        let ctx = format!("rand {i}: a={} lo={} hi={}", show_v(a), show_v(lo), show_v(hi));
        eq_v("row08", &ctx, unsafe { (c.c2Clampv)(a, lo, hi) }, unsafe {
            (r.c2Clampv)(a, lo, hi)
        });
    }
    // lo == hi exactly (degenerate box) with random finite values.
    let mut rng = Rng::new(0x8009);
    for i in 0..ITERS {
        let p = v(rng.small(), rng.small());
        let a = v(rng.coord(8.0), rng.coord(8.0));
        let ctx = format!("degen {i}: a={} lo=hi={}", show_v(a), show_v(p));
        eq_v("row08", &ctx, unsafe { (c.c2Clampv)(a, p, p) }, unsafe { (r.c2Clampv)(a, p, p) });
    }
}

// ===========================================================================
// Rows 9-10 — c2Sub
// ===========================================================================

#[test]
fn row09_c2Sub_finite_and_cancellation() {
    let (c, r) = both();
    let mut rng = Rng::new(0x9009);
    for i in 0..ITERS {
        let a = v(rng.coord(32.0), rng.coord(32.0));
        // Every 4th iteration subtracts a from itself ⇒ exact +0.0 cancellation.
        let b = if i % 4 == 0 { a } else { v(rng.coord(32.0), rng.coord(32.0)) };
        let ctx = format!("iter {i}: a={} b={}", show_v(a), show_v(b));
        eq_v("row09", &ctx, unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) });
    }
    // Signed-zero matrix: -0.0 - +0.0 == -0.0, +0.0 - +0.0 == +0.0, ...
    let zeros = [0.0f32, -0.0f32];
    for &ax in &zeros {
        for &bx in &zeros {
            for &ay in &zeros {
                for &by in &zeros {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let ctx = format!("zeros a={} b={}", show_v(a), show_v(b));
                    eq_v("row09", &ctx, unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) });
                }
            }
        }
    }
}

#[test]
fn row10_c2Sub_overflow_inf_snan_subnormal() {
    let (c, r) = both();
    let vals = [
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7fc0_0abc), // qNaN payload
        f32::from_bits(0x7f80_0001), // sNaN  -> must be quieted by subss
        f32::from_bits(0xff80_0001), // -sNaN
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        0.0,
        -0.0,
    ];
    for &ax in &vals {
        for &bx in &vals {
            for &ay in &vals {
                for &by in &vals {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let ctx = format!("a={} b={}", show_v(a), show_v(b));
                    eq_v("row10", &ctx, unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) });
                }
            }
        }
    }
    let mut rng = Rng::new(0xA00A);
    for i in 0..ITERS {
        let (a, b) = (rng.v_wild(), rng.v_wild());
        let ctx = format!("rand {i}: a={} b={}", show_v(a), show_v(b));
        eq_v("row10", &ctx, unsafe { (c.c2Sub)(a, b) }, unsafe { (r.c2Sub)(a, b) });
    }
}

// ===========================================================================
// Rows 11-12 — c2Dot
// ===========================================================================

#[test]
fn row11_c2Dot_finite_overflow_cancellation() {
    let (c, r) = both();
    let mut rng = Rng::new(0xB00B);
    for i in 0..ITERS {
        let (a, b) = (v(rng.coord(64.0), rng.coord(64.0)), v(rng.coord(64.0), rng.coord(64.0)));
        let ctx = format!("iter {i}: a={} b={}", show_v(a), show_v(b));
        eq_f32("row11", &ctx, unsafe { (c.c2Dot)(a, b) }, unsafe { (r.c2Dot)(a, b) });
    }
    // Products that overflow to inf, and sums that cancel exactly.
    let cases = [
        (v(f32::MAX, f32::MAX), v(f32::MAX, f32::MAX)),
        (v(f32::MAX, f32::MAX), v(f32::MAX, -f32::MAX)), // inf + -inf -> qNaN
        (v(1e20, 1e20), v(1e20, -1e20)),
        (v(3.0, 4.0), v(3.0, 4.0)), // 25 exactly
        (v(1.0, -1.0), v(1.0, 1.0)),
        (v(f32::MIN_POSITIVE, f32::MIN_POSITIVE), v(f32::MIN_POSITIVE, f32::MIN_POSITIVE)),
        (v(f32::from_bits(1), 1.0), v(1.0, f32::from_bits(1))),
        (v(0.0, f32::INFINITY), v(f32::INFINITY, 0.0)), // 0*inf twice -> qNaN
        (v(-0.0, 0.0), v(0.0, -0.0)),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        let ctx = format!("case {i}: a={} b={}", show_v(*a), show_v(*b));
        eq_f32("row11", &ctx, unsafe { (c.c2Dot)(*a, *b) }, unsafe { (r.c2Dot)(*a, *b) });
    }
}

#[test]
fn row12_c2Dot_nan_payload_matrix() {
    let (c, r) = both();
    // Distinct payloads in every lane: pins down which mulss/addss operand
    // order the C compiler chose (the Rust emulates it explicitly).
    let nans = [
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0x7fc0_ff00),
        f32::from_bits(0x7f80_0001), // sNaN
        f32::from_bits(0x7f80_00ff), // sNaN
        f32::from_bits(0xffc0_0002), // -qNaN
        f32::from_bits(0xff80_0007), // -sNaN
        0.0,
        -0.0,
        1.0,
        -2.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &ax in &nans {
        for &bx in &nans {
            for &ay in &nans {
                for &by in &nans {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let ctx = format!("a={} b={}", show_v(a), show_v(b));
                    eq_f32("row12", &ctx, unsafe { (c.c2Dot)(a, b) }, unsafe { (r.c2Dot)(a, b) });
                }
            }
        }
    }
    let mut rng = Rng::new(0xC00C);
    for i in 0..ITERS {
        let (a, b) = (rng.v_wild(), rng.v_wild());
        let ctx = format!("rand {i}: a={} b={}", show_v(a), show_v(b));
        eq_f32("row12", &ctx, unsafe { (c.c2Dot)(a, b) }, unsafe { (r.c2Dot)(a, b) });
    }
}

// ===========================================================================
// Rows 13-16 — c2CircletoCircle
// ===========================================================================

#[test]
fn row13_CircletoCircle_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0xD00D);
    let (mut hits, mut misses) = (0usize, 0usize);
    for i in 0..ITERS {
        let (A, B) = (rng.c_small(), rng.c_small());
        let ctx = format!("iter {i}: A={} B={}", show_c(A), show_c(B));
        let cv = unsafe { (c.c2CircletoCircle)(A, B) };
        let rv = unsafe { (r.c2CircletoCircle)(A, B) };
        eq_int("row13", &ctx, cv, rv);
        assert_bool_like("row13", &ctx, cv);
        if cv == 1 { hits += 1 } else { misses += 1 }
    }
    assert!(hits > 100 && misses > 100, "[row13] one branch barely taken: {hits} hits / {misses} misses");
}

#[test]
fn row14_CircletoCircle_exact_touch_and_ulp() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE00E);
    for i in 0..ITERS {
        // Quantised radii/positions ⇒ the sum and the difference are exact,
        // so d2 == r2 holds bit-for-bit at the touching distance.
        let ra = (rng.below(64) as f32) / 4.0;
        let rb = (rng.below(64) as f32) / 4.0;
        let sum = ra + rb;
        let ax = rng.coord(4.0);
        let ay = rng.coord(4.0);
        let A = circle(ax, ay, ra);
        for (label, dx) in [
            ("touch", sum),
            ("inside_ulp", f32::from_bits(sum.to_bits().wrapping_sub(1))),
            ("outside_ulp", f32::from_bits(sum.to_bits().wrapping_add(1))),
            ("zero_dist", 0.0),
        ] {
            let B = circle(ax + dx, ay, rb);
            let ctx = format!("iter {i} {label}: A={} B={}", show_c(A), show_c(B));
            eq_int("row14", &ctx, unsafe { (c.c2CircletoCircle)(A, B) }, unsafe {
                (r.c2CircletoCircle)(A, B)
            });
        }
        // Identical circles, and zero-radius circles.
        let same = circle(ax, ay, ra);
        let ctx = format!("iter {i} identical: {}", show_c(same));
        eq_int("row14", &ctx, unsafe { (c.c2CircletoCircle)(same, same) }, unsafe {
            (r.c2CircletoCircle)(same, same)
        });
        let z0 = circle(ax, ay, 0.0);
        let z1 = circle(ax, ay, 0.0);
        let ctx = format!("iter {i} zero radius: {}", show_c(z0));
        eq_int("row14", &ctx, unsafe { (c.c2CircletoCircle)(z0, z1) }, unsafe {
            (r.c2CircletoCircle)(z0, z1)
        });
    }
}

#[test]
fn row15_CircletoCircle_negative_radius() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF00F);
    for i in 0..ITERS {
        let ra = -(rng.below(64) as f32) / 4.0;
        let rb = match i % 3 {
            0 => -ra,                          // sum == 0 ⇒ r2 == 0 ⇒ never a hit
            1 => -(rng.below(64) as f32) / 4.0, // both negative ⇒ sum² > 0
            _ => (rng.below(64) as f32) / 4.0,
        };
        let A = circle(rng.coord(4.0), rng.coord(4.0), ra);
        let B = circle(rng.coord(4.0), rng.coord(4.0), rb);
        let ctx = format!("iter {i}: A={} B={}", show_c(A), show_c(B));
        eq_int("row15", &ctx, unsafe { (c.c2CircletoCircle)(A, B) }, unsafe {
            (r.c2CircletoCircle)(A, B)
        });
    }
}

#[test]
fn row16_CircletoCircle_nonfinite() {
    let (c, r) = both();
    let vals = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
    ];
    // Vary one field at a time over the whole class list (full 6-field cross
    // product would be 3M cases; this covers each field in each class).
    let base = circle(1.0, 2.0, 3.0);
    for &val in &vals {
        for field in 0..6 {
            let (mut A, mut B) = (base, circle(-1.0, 5.0, 2.0));
            match field {
                0 => A.p.x = val,
                1 => A.p.y = val,
                2 => A.r = val,
                3 => B.p.x = val,
                4 => B.p.y = val,
                _ => B.r = val,
            }
            let ctx = format!("field {field} val 0x{:08x}: A={} B={}", fb(val), show_c(A), show_c(B));
            eq_int("row16", &ctx, unsafe { (c.c2CircletoCircle)(A, B) }, unsafe {
                (r.c2CircletoCircle)(A, B)
            });
        }
    }
    // Radii whose sum overflows to inf, huge coordinates, and fully wild input.
    let cases = [
        (circle(0.0, 0.0, f32::MAX), circle(0.0, 0.0, f32::MAX)),
        (circle(f32::MAX, 0.0, 1.0), circle(-f32::MAX, 0.0, 1.0)),
        (circle(f32::INFINITY, 0.0, 1.0), circle(f32::INFINITY, 0.0, 1.0)), // inf-inf ⇒ NaN
        (circle(1e30, 1e30, 1e30), circle(-1e30, -1e30, 1e30)),
    ];
    for (i, (A, B)) in cases.iter().enumerate() {
        let ctx = format!("case {i}: A={} B={}", show_c(*A), show_c(*B));
        eq_int("row16", &ctx, unsafe { (c.c2CircletoCircle)(*A, *B) }, unsafe {
            (r.c2CircletoCircle)(*A, *B)
        });
    }
    let mut rng = Rng::new(0x1111);
    for i in 0..ITERS {
        let (A, B) = (rng.c_wild(), rng.c_wild());
        let ctx = format!("rand {i}: A={} B={}", show_c(A), show_c(B));
        eq_int("row16", &ctx, unsafe { (c.c2CircletoCircle)(A, B) }, unsafe {
            (r.c2CircletoCircle)(A, B)
        });
    }
}

// ===========================================================================
// Rows 17-23 — c2CircletoAABB
// ===========================================================================

#[test]
fn row17_CircletoAABB_centre_inside() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1212);
    for i in 0..ITERS {
        let b = rng.b_small();
        // Pick a centre guaranteed to be inside (or on the border of) the box.
        let tx = (rng.below(17) as f32) / 16.0;
        let ty = (rng.below(17) as f32) / 16.0;
        let p = v(b.min.x + (b.max.x - b.min.x) * tx, b.min.y + (b.max.y - b.min.y) * ty);
        for rad in [0.0f32, -0.0, rng.coord(4.0), f32::from_bits(1)] {
            let A = C2Circle { p, r: rad };
            let ctx = format!("iter {i}: A={} B={}", show_c(A), show_b(b));
            eq_int("row17", &ctx, unsafe { (c.c2CircletoAABB)(A, b) }, unsafe {
                (r.c2CircletoAABB)(A, b)
            });
        }
    }
}

#[test]
fn row18_CircletoAABB_edge_regions() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1313);
    for i in 0..ITERS {
        let b = rng.b_small();
        let d = (rng.below(64) as f32) / 8.0 + 1.0 / 8.0; // > 0 ⇒ strictly outside
        let inx = b.min.x + (b.max.x - b.min.x) * ((rng.below(17) as f32) / 16.0);
        let iny = b.min.y + (b.max.y - b.min.y) * ((rng.below(17) as f32) / 16.0);
        // The four edge regions: clamped on exactly one axis.
        let centres = [
            v(inx, b.min.y - d), // below
            v(inx, b.max.y + d), // above
            v(b.min.x - d, iny), // left
            v(b.max.x + d, iny), // right
        ];
        for (k, p) in centres.iter().enumerate() {
            let A = C2Circle { p: *p, r: (rng.below(64) as f32) / 8.0 };
            let ctx = format!("iter {i} edge {k}: A={} B={}", show_c(A), show_b(b));
            eq_int("row18", &ctx, unsafe { (c.c2CircletoAABB)(A, b) }, unsafe {
                (r.c2CircletoAABB)(A, b)
            });
        }
    }
}

#[test]
fn row19_CircletoAABB_corner_regions() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1414);
    for i in 0..ITERS {
        let b = rng.b_small();
        let dx = (rng.below(48) as f32) / 8.0 + 1.0 / 8.0;
        let dy = (rng.below(48) as f32) / 8.0 + 1.0 / 8.0;
        let centres = [
            v(b.min.x - dx, b.min.y - dy),
            v(b.max.x + dx, b.min.y - dy),
            v(b.min.x - dx, b.max.y + dy),
            v(b.max.x + dx, b.max.y + dy),
        ];
        for (k, p) in centres.iter().enumerate() {
            let A = C2Circle { p: *p, r: (rng.below(80) as f32) / 8.0 };
            let ctx = format!("iter {i} corner {k}: A={} B={}", show_c(A), show_b(b));
            eq_int("row19", &ctx, unsafe { (c.c2CircletoAABB)(A, b) }, unsafe {
                (r.c2CircletoAABB)(A, b)
            });
        }
    }
}

#[test]
fn row20_CircletoAABB_exact_touch() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1515);
    for i in 0..ITERS {
        // --- edge touch: distance is a single exact coordinate delta ---
        let b = rng.b_small();
        let d = (rng.below(64) as f32) / 8.0 + 1.0 / 8.0;
        let inx = b.min.x + (b.max.x - b.min.x) * ((rng.below(17) as f32) / 16.0);
        for (label, rad) in [
            ("touch", d),
            ("inside_ulp", f32::from_bits(d.to_bits().wrapping_add(1))),
            ("outside_ulp", f32::from_bits(d.to_bits().wrapping_sub(1))),
        ] {
            let A = circle(inx, b.min.y - d, rad);
            let ctx = format!("iter {i} edge {label}: A={} B={}", show_c(A), show_b(b));
            eq_int("row20", &ctx, unsafe { (c.c2CircletoAABB)(A, b) }, unsafe {
                (r.c2CircletoAABB)(A, b)
            });
        }
        // --- corner touch: 3-4-5 triple scaled by a power of two (exact) ---
        let s = 1.0f32 / (1u32 << rng.below(4)) as f32;
        let ox = rng.coord(4.0);
        let oy = rng.coord(4.0);
        let bx = aabb(ox - 8.0 * s, oy - 8.0 * s, ox, oy);
        for (label, rad) in [
            ("touch", 5.0 * s),
            ("inside_ulp", f32::from_bits((5.0f32 * s).to_bits().wrapping_add(1))),
            ("outside_ulp", f32::from_bits((5.0f32 * s).to_bits().wrapping_sub(1))),
        ] {
            let A = circle(ox + 3.0 * s, oy + 4.0 * s, rad);
            let ctx = format!("iter {i} corner {label}: A={} B={}", show_c(A), show_b(bx));
            eq_int("row20", &ctx, unsafe { (c.c2CircletoAABB)(A, bx) }, unsafe {
                (r.c2CircletoAABB)(A, bx)
            });
        }
    }
}

#[test]
fn row21_CircletoAABB_degenerate_box_and_zero_radius() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1616);
    for i in 0..ITERS {
        let p = v(rng.small(), rng.small());
        let boxes = [
            aabb(p.x, p.y, p.x, p.y),                     // point box
            aabb(p.x, p.y, p.x + rng.small().abs(), p.y),  // zero height
            aabb(p.x, p.y, p.x, p.y + rng.small().abs()),  // zero width
            aabb(-0.0, -0.0, 0.0, 0.0),                    // signed-zero box
        ];
        for (k, b) in boxes.iter().enumerate() {
            for rad in [0.0f32, -0.0, 1.0 / 16.0, rng.coord(4.0)] {
                let A = C2Circle { p: v(rng.coord(8.0), rng.coord(8.0)), r: rad };
                let ctx = format!("iter {i} box {k}: A={} B={}", show_c(A), show_b(*b));
                eq_int("row21", &ctx, unsafe { (c.c2CircletoAABB)(A, *b) }, unsafe {
                    (r.c2CircletoAABB)(A, *b)
                });
            }
        }
    }
}

#[test]
fn row22_CircletoAABB_inverted_box() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1717);
    for i in 0..ITERS {
        let b = rng.b_small();
        let inverted = [
            aabb(b.max.x, b.max.y, b.min.x, b.min.y), // both axes swapped
            aabb(b.max.x, b.min.y, b.min.x, b.max.y), // x swapped
            aabb(b.min.x, b.max.y, b.max.x, b.min.y), // y swapped
        ];
        for (k, bb) in inverted.iter().enumerate() {
            let A = C2Circle {
                p: v(rng.coord(8.0), rng.coord(8.0)),
                r: (rng.below(64) as f32) / 8.0,
            };
            let ctx = format!("iter {i} inv {k}: A={} B={}", show_c(A), show_b(*bb));
            eq_int("row22", &ctx, unsafe { (c.c2CircletoAABB)(A, *bb) }, unsafe {
                (r.c2CircletoAABB)(A, *bb)
            });
        }
    }
}

#[test]
fn row23_CircletoAABB_nonfinite() {
    let (c, r) = both();
    let vals = [
        0.0f32,
        -0.0,
        1.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
    ];
    let baseA = circle(1.0, 2.0, 3.0);
    let baseB = aabb(-2.0, -2.0, 2.0, 2.0);
    for &val in &vals {
        for field in 0..7 {
            let (mut A, mut B) = (baseA, baseB);
            match field {
                0 => A.p.x = val,
                1 => A.p.y = val,
                2 => A.r = val,
                3 => B.min.x = val,
                4 => B.min.y = val,
                5 => B.max.x = val,
                _ => B.max.y = val,
            }
            let ctx = format!("field {field} val 0x{:08x}: A={} B={}", fb(val), show_c(A), show_b(B));
            eq_int("row23", &ctx, unsafe { (c.c2CircletoAABB)(A, B) }, unsafe {
                (r.c2CircletoAABB)(A, B)
            });
        }
    }
    // d2 overflowing to inf, plus fully wild input.
    let cases = [
        (circle(f32::MAX, f32::MAX, f32::MAX), aabb(-f32::MAX, -f32::MAX, 0.0, 0.0)),
        (circle(1e30, 1e30, 1e30), aabb(-1e30, -1e30, -1e29, -1e29)),
        (circle(f32::INFINITY, f32::INFINITY, f32::INFINITY), aabb(0.0, 0.0, f32::INFINITY, f32::INFINITY)),
    ];
    for (i, (A, B)) in cases.iter().enumerate() {
        let ctx = format!("case {i}: A={} B={}", show_c(*A), show_b(*B));
        eq_int("row23", &ctx, unsafe { (c.c2CircletoAABB)(*A, *B) }, unsafe {
            (r.c2CircletoAABB)(*A, *B)
        });
    }
    let mut rng = Rng::new(0x1818);
    for i in 0..ITERS {
        let (A, B) = (rng.c_wild(), rng.b_wild());
        let ctx = format!("rand {i}: A={} B={}", show_c(A), show_b(B));
        eq_int("row23", &ctx, unsafe { (c.c2CircletoAABB)(A, B) }, unsafe {
            (r.c2CircletoAABB)(A, B)
        });
    }
}

// ===========================================================================
// Rows 24-26 — c2AABBtoAABB
// ===========================================================================

/// Bookkeeping only (never used to decide the expected answer): which of the
/// four separating-axis flags the C code would set.
fn sep_mask(A: C2AABB, B: C2AABB) -> u8 {
    ((B.max.x < A.min.x) as u8)
        | (((A.max.x < B.min.x) as u8) << 1)
        | (((B.max.y < A.min.y) as u8) << 2)
        | (((A.max.y < B.min.y) as u8) << 3)
}

#[test]
fn row24_AABBtoAABB_all_sixteen_flag_combos() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1919);
    let mut seen = [0usize; 16];
    for i in 0..ITERS * 4 {
        // Independent min/max draws on a tiny integer grid ⇒ inverted boxes and
        // ties are common, so all 16 flag combinations occur.
        let mk = |rng: &mut Rng| {
            let g = |rng: &mut Rng| (rng.below(5) as f32) - 2.0;
            C2AABB { min: v(g(rng), g(rng)), max: v(g(rng), g(rng)) }
        };
        let (A, B) = (mk(&mut rng), mk(&mut rng));
        seen[sep_mask(A, B) as usize] += 1;
        let ctx = format!("iter {i}: A={} B={}", show_b(A), show_b(B));
        let cv = unsafe { (c.c2AABBtoAABB)(A, B) };
        eq_int("row24", &ctx, cv, unsafe { (r.c2AABBtoAABB)(A, B) });
        assert_bool_like("row24", &ctx, cv);
    }
    assert!(
        seen.iter().all(|&n| n > 0),
        "[row24] not all 16 separating-flag combinations exercised: {seen:?}"
    );
}

#[test]
fn row25_AABBtoAABB_touching_containment_identical() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1A1A);
    for i in 0..ITERS {
        let A = rng.b_small();
        let w = (rng.below(32) as f32) / 8.0;
        let h = (rng.below(32) as f32) / 8.0;
        let variants = [
            // touching on each of the four sides (strict `<` ⇒ still a hit)
            aabb(A.max.x, A.min.y, A.max.x + w, A.max.y + h),
            aabb(A.min.x - w, A.min.y, A.min.x, A.max.y),
            aabb(A.min.x, A.max.y, A.max.x + w, A.max.y + h),
            aabb(A.min.x, A.min.y - h, A.max.x, A.min.y),
            // one ULP past touching on the right ⇒ separated
            aabb(
                f32::from_bits(A.max.x.to_bits().wrapping_add(1)),
                A.min.y,
                A.max.x + w + 1.0,
                A.max.y,
            ),
            // containment and identity
            aabb(
                A.min.x + (A.max.x - A.min.x) / 4.0,
                A.min.y + (A.max.y - A.min.y) / 4.0,
                A.max.x - (A.max.x - A.min.x) / 4.0,
                A.max.y - (A.max.y - A.min.y) / 4.0,
            ),
            A,
            // corner-touching diagonal neighbour
            aabb(A.max.x, A.max.y, A.max.x + w, A.max.y + h),
        ];
        for (k, B) in variants.iter().enumerate() {
            let ctx = format!("iter {i} variant {k}: A={} B={}", show_b(A), show_b(*B));
            eq_int("row25", &ctx, unsafe { (c.c2AABBtoAABB)(A, *B) }, unsafe {
                (r.c2AABBtoAABB)(A, *B)
            });
        }
    }
}

#[test]
fn row26_AABBtoAABB_inverted_degenerate_nonfinite() {
    let (c, r) = both();
    let vals = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::MIN_POSITIVE,
    ];
    let baseA = aabb(-1.0, -1.0, 1.0, 1.0);
    let baseB = aabb(0.5, 0.5, 2.0, 2.0);
    for &val in &vals {
        for field in 0..8 {
            let (mut A, mut B) = (baseA, baseB);
            let f = |b: &mut C2AABB, k: usize, val: f32| match k {
                0 => b.min.x = val,
                1 => b.min.y = val,
                2 => b.max.x = val,
                _ => b.max.y = val,
            };
            if field < 4 {
                f(&mut A, field, val)
            } else {
                f(&mut B, field - 4, val)
            }
            let ctx = format!("field {field} val 0x{:08x}: A={} B={}", fb(val), show_b(A), show_b(B));
            eq_int("row26", &ctx, unsafe { (c.c2AABBtoAABB)(A, B) }, unsafe {
                (r.c2AABBtoAABB)(A, B)
            });
        }
    }
    // All-NaN boxes: every flag is 0 ⇒ the C returns 1.
    let nan_box = aabb(f32::NAN, f32::NAN, f32::NAN, f32::NAN);
    let ctx = "both boxes all-NaN".to_string();
    let cv = unsafe { (c.c2AABBtoAABB)(nan_box, nan_box) };
    eq_int("row26", &ctx, cv, unsafe { (r.c2AABBtoAABB)(nan_box, nan_box) });
    assert_eq!(cv, 1, "[row26] C's !(d0|d1|d2|d3) with NaN edges should be 1");
    // Signed-zero and infinite edges, inverted/degenerate, and wild boxes.
    let mut rng = Rng::new(0x1B1B);
    for i in 0..ITERS {
        let (A, B) = (rng.b_wild(), rng.b_wild());
        let ctx = format!("rand {i}: A={} B={}", show_b(A), show_b(B));
        eq_int("row26", &ctx, unsafe { (c.c2AABBtoAABB)(A, B) }, unsafe {
            (r.c2AABBtoAABB)(A, B)
        });
    }
    for i in 0..ITERS {
        let A = rng.b_small();
        let inv = aabb(A.max.x, A.max.y, A.min.x, A.min.y);
        let B = rng.b_small();
        for (k, (x, y)) in [(inv, B), (B, inv), (inv, inv)].iter().enumerate() {
            let ctx = format!("inv {i}/{k}: A={} B={}", show_b(*x), show_b(*y));
            eq_int("row26", &ctx, unsafe { (c.c2AABBtoAABB)(*x, *y) }, unsafe {
                (r.c2AABBtoAABB)(*x, *y)
            });
        }
    }
}

// ===========================================================================
// Rows 27-33 — collided (the dispatcher), driven exactly as a consumer would
// ===========================================================================

fn cp(x: &C2Circle) -> *const c_void {
    x as *const C2Circle as *const c_void
}
fn bp(x: &C2AABB) -> *const c_void {
    x as *const C2AABB as *const c_void
}

#[test]
fn row27_collided_circle_circle() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2727);
    let (mut hits, mut misses) = (0usize, 0usize);
    for i in 0..ITERS {
        let (A, B) = (rng.c_small(), rng.c_small());
        let ctx = format!("iter {i}: A={} B={}", show_c(A), show_c(B));
        let cv = unsafe { (c.collided)(cp(&A), C2_TYPE_CIRCLE, cp(&B), C2_TYPE_CIRCLE) };
        let rv = unsafe { (r.collided)(cp(&A), C2_TYPE_CIRCLE, cp(&B), C2_TYPE_CIRCLE) };
        eq_int("row27", &ctx, cv, rv);
        // The dispatcher must agree with the direct predicate in both libraries.
        eq_int("row27/direct-C", &ctx, cv, unsafe { (c.c2CircletoCircle)(A, B) });
        eq_int("row27/direct-Rust", &ctx, rv, unsafe { (r.c2CircletoCircle)(A, B) });
        if cv == 1 { hits += 1 } else { misses += 1 }
    }
    assert!(hits > 50 && misses > 50, "[row27] {hits} hits / {misses} misses");
}

#[test]
fn row28_collided_circle_aabb() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2828);
    let (mut hits, mut misses) = (0usize, 0usize);
    for i in 0..ITERS {
        let A = rng.c_small();
        let B = rng.b_small();
        let ctx = format!("iter {i}: A={} B={}", show_c(A), show_b(B));
        let cv = unsafe { (c.collided)(cp(&A), C2_TYPE_CIRCLE, bp(&B), C2_TYPE_AABB) };
        let rv = unsafe { (r.collided)(cp(&A), C2_TYPE_CIRCLE, bp(&B), C2_TYPE_AABB) };
        eq_int("row28", &ctx, cv, rv);
        eq_int("row28/direct-C", &ctx, cv, unsafe { (c.c2CircletoAABB)(A, B) });
        eq_int("row28/direct-Rust", &ctx, rv, unsafe { (r.c2CircletoAABB)(A, B) });
        if cv == 1 { hits += 1 } else { misses += 1 }
    }
    assert!(hits > 50 && misses > 50, "[row28] {hits} hits / {misses} misses");
}

#[test]
fn row29_collided_aabb_circle_argument_swap() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2929);
    let (mut hits, mut misses) = (0usize, 0usize);
    for i in 0..ITERS {
        // A is the BOX here and B is the CIRCLE: the C swaps them internally.
        let A = rng.b_small();
        let B = rng.c_small();
        let ctx = format!("iter {i}: A(box)={} B(circle)={}", show_b(A), show_c(B));
        let cv = unsafe { (c.collided)(bp(&A), C2_TYPE_AABB, cp(&B), C2_TYPE_CIRCLE) };
        let rv = unsafe { (r.collided)(bp(&A), C2_TYPE_AABB, cp(&B), C2_TYPE_CIRCLE) };
        eq_int("row29", &ctx, cv, rv);
        // Pin the swap down: it must equal c2CircletoAABB(circle=B, box=A).
        eq_int("row29/swap-C", &ctx, cv, unsafe { (c.c2CircletoAABB)(B, A) });
        eq_int("row29/swap-Rust", &ctx, rv, unsafe { (r.c2CircletoAABB)(B, A) });
        if cv == 1 { hits += 1 } else { misses += 1 }
    }
    assert!(hits > 50 && misses > 50, "[row29] {hits} hits / {misses} misses");
}

#[test]
fn row30_collided_aabb_aabb() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3030);
    let (mut hits, mut misses) = (0usize, 0usize);
    for i in 0..ITERS {
        let (A, B) = (rng.b_small(), rng.b_small());
        let ctx = format!("iter {i}: A={} B={}", show_b(A), show_b(B));
        let cv = unsafe { (c.collided)(bp(&A), C2_TYPE_AABB, bp(&B), C2_TYPE_AABB) };
        let rv = unsafe { (r.collided)(bp(&A), C2_TYPE_AABB, bp(&B), C2_TYPE_AABB) };
        eq_int("row30", &ctx, cv, rv);
        eq_int("row30/direct-C", &ctx, cv, unsafe { (c.c2AABBtoAABB)(A, B) });
        eq_int("row30/direct-Rust", &ctx, rv, unsafe { (r.c2AABBtoAABB)(A, B) });
        if cv == 1 { hits += 1 } else { misses += 1 }
    }
    assert!(hits > 50 && misses > 50, "[row30] {hits} hits / {misses} misses");
}

#[test]
fn row31_collided_all_tag_pairs_with_edge_values() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3131);
    for i in 0..ITERS {
        // Wild (non-finite / inverted / degenerate) operands through every
        // valid tag pair.
        let (ca, cb) = (rng.c_wild(), rng.c_wild());
        let (ba, bb) = (rng.b_wild(), rng.b_wild());
        let ctx = format!(
            "iter {i}: ca={} cb={} ba={} bb={}",
            show_c(ca),
            show_c(cb),
            show_b(ba),
            show_b(bb)
        );
        unsafe {
            eq_int(
                "row31/CC",
                &ctx,
                (c.collided)(cp(&ca), C2_TYPE_CIRCLE, cp(&cb), C2_TYPE_CIRCLE),
                (r.collided)(cp(&ca), C2_TYPE_CIRCLE, cp(&cb), C2_TYPE_CIRCLE),
            );
            eq_int(
                "row31/CA",
                &ctx,
                (c.collided)(cp(&ca), C2_TYPE_CIRCLE, bp(&bb), C2_TYPE_AABB),
                (r.collided)(cp(&ca), C2_TYPE_CIRCLE, bp(&bb), C2_TYPE_AABB),
            );
            eq_int(
                "row31/AC",
                &ctx,
                (c.collided)(bp(&ba), C2_TYPE_AABB, cp(&cb), C2_TYPE_CIRCLE),
                (r.collided)(bp(&ba), C2_TYPE_AABB, cp(&cb), C2_TYPE_CIRCLE),
            );
            eq_int(
                "row31/AA",
                &ctx,
                (c.collided)(bp(&ba), C2_TYPE_AABB, bp(&bb), C2_TYPE_AABB),
                (r.collided)(bp(&ba), C2_TYPE_AABB, bp(&bb), C2_TYPE_AABB),
            );
        }
    }
}

#[test]
fn row32_collided_aliasing_and_unaligned_buffers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3232);

    // --- aliasing: the same pointer for A and B ---
    for i in 0..ITERS {
        let circ = rng.c_wild();
        let bx = rng.b_wild();
        let ctx = format!("alias {i}: circ={} box={}", show_c(circ), show_b(bx));
        unsafe {
            eq_int(
                "row32/alias-CC",
                &ctx,
                (c.collided)(cp(&circ), C2_TYPE_CIRCLE, cp(&circ), C2_TYPE_CIRCLE),
                (r.collided)(cp(&circ), C2_TYPE_CIRCLE, cp(&circ), C2_TYPE_CIRCLE),
            );
            eq_int(
                "row32/alias-AA",
                &ctx,
                (c.collided)(bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_AABB),
                (r.collided)(bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_AABB),
            );
            // Same address read as a circle by one tag and a box by the other:
            // the AABB read consumes 16 bytes, so back it with a 16-byte object.
            eq_int(
                "row32/alias-AC",
                &ctx,
                (c.collided)(bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_CIRCLE),
                (r.collided)(bp(&bx), C2_TYPE_AABB, bp(&bx), C2_TYPE_CIRCLE),
            );
        }
    }

    // --- unaligned: place the operands at every odd byte offset ---
    let mut buf = [0u8; 64];
    for i in 0..ITERS {
        let circ = rng.c_small();
        let bx = rng.b_small();
        let off_c = 1 + (rng.below(7) as usize); // 1..=7, never 4-aligned for some
        let off_b = 17 + (rng.below(7) as usize);
        buf.fill(0);
        let cbytes: [u8; 12] = unsafe { std::mem::transmute(circ) };
        let bbytes: [u8; 16] = unsafe { std::mem::transmute(bx) };
        buf[off_c..off_c + 12].copy_from_slice(&cbytes);
        buf[off_b..off_b + 16].copy_from_slice(&bbytes);
        let pc = unsafe { buf.as_ptr().add(off_c) } as *const c_void;
        let pb = unsafe { buf.as_ptr().add(off_b) } as *const c_void;
        let ctx = format!(
            "unaligned {i}: off_c={off_c} off_b={off_b} circ={} box={}",
            show_c(circ),
            show_b(bx)
        );
        unsafe {
            eq_int(
                "row32/unaligned-CC",
                &ctx,
                (c.collided)(pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE),
                (r.collided)(pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE),
            );
            eq_int(
                "row32/unaligned-CA",
                &ctx,
                (c.collided)(pc, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB),
                (r.collided)(pc, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB),
            );
            eq_int(
                "row32/unaligned-AC",
                &ctx,
                (c.collided)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE),
                (r.collided)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE),
            );
            eq_int(
                "row32/unaligned-AA",
                &ctx,
                (c.collided)(pb, C2_TYPE_AABB, pb, C2_TYPE_AABB),
                (r.collided)(pb, C2_TYPE_AABB, pb, C2_TYPE_AABB),
            );
        }
    }
}

#[test]
fn row33_end_to_end_scene_pipeline() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3333);
    let circles: Vec<C2Circle> = (0..48).map(|_| rng.c_small()).collect();
    let boxes: Vec<C2AABB> = (0..48).map(|_| rng.b_small()).collect();

    // 1) Broad phase through the dispatcher, all pairs, all tag pairs.
    let mut c_results = Vec::new();
    let mut r_results = Vec::new();
    for a in 0..circles.len() {
        for b in 0..boxes.len() {
            let (ca, cb) = (circles[a], circles[b]);
            let (ba, bb) = (boxes[a], boxes[b]);
            unsafe {
                c_results.push((c.collided)(cp(&ca), C2_TYPE_CIRCLE, cp(&cb), C2_TYPE_CIRCLE));
                c_results.push((c.collided)(cp(&ca), C2_TYPE_CIRCLE, bp(&bb), C2_TYPE_AABB));
                c_results.push((c.collided)(bp(&ba), C2_TYPE_AABB, cp(&cb), C2_TYPE_CIRCLE));
                c_results.push((c.collided)(bp(&ba), C2_TYPE_AABB, bp(&bb), C2_TYPE_AABB));
                r_results.push((r.collided)(cp(&ca), C2_TYPE_CIRCLE, cp(&cb), C2_TYPE_CIRCLE));
                r_results.push((r.collided)(cp(&ca), C2_TYPE_CIRCLE, bp(&bb), C2_TYPE_AABB));
                r_results.push((r.collided)(bp(&ba), C2_TYPE_AABB, cp(&cb), C2_TYPE_CIRCLE));
                r_results.push((r.collided)(bp(&ba), C2_TYPE_AABB, bp(&bb), C2_TYPE_AABB));
            }
        }
    }
    assert_eq!(c_results, r_results, "[row33] scene result vectors differ");
    let hits = c_results.iter().filter(|&&x| x == 1).count();
    assert!(
        hits > 0 && hits < c_results.len(),
        "[row33] degenerate scene: {hits}/{} hits",
        c_results.len()
    );

    // 2) The same geometry recomputed by composing the LOW-LEVEL entry points
    //    (each library composing its own helpers), so intermediate values are
    //    compared as well as the final verdict.
    let mut c_pipe: Vec<(u32, u32, u32, i32)> = Vec::new();
    let mut r_pipe: Vec<(u32, u32, u32, i32)> = Vec::new();
    for circ in &circles {
        for bx in &boxes {
            unsafe {
                // C pipeline: c2V -> c2Clampv -> c2Sub -> c2Dot -> predicate
                let p = (c.c2V)(circ.p.x, circ.p.y);
                let lo = (c.c2Minv)(bx.min, bx.max);
                let hi = (c.c2Maxv)(bx.min, bx.max);
                let l = (c.c2Clampv)(p, lo, hi);
                let ab = (c.c2Sub)(p, l);
                let d2 = (c.c2Dot)(ab, ab);
                let verdict = (c.c2CircletoAABB)(*circ, aabb(lo.x, lo.y, hi.x, hi.y));
                c_pipe.push((fb(l.x), fb(l.y), fb(d2), verdict));

                let p = (r.c2V)(circ.p.x, circ.p.y);
                let lo = (r.c2Minv)(bx.min, bx.max);
                let hi = (r.c2Maxv)(bx.min, bx.max);
                let l = (r.c2Clampv)(p, lo, hi);
                let ab = (r.c2Sub)(p, l);
                let d2 = (r.c2Dot)(ab, ab);
                let verdict = (r.c2CircletoAABB)(*circ, aabb(lo.x, lo.y, hi.x, hi.y));
                r_pipe.push((fb(l.x), fb(l.y), fb(d2), verdict));
            }
        }
    }
    assert_eq!(c_pipe, r_pipe, "[row33] composed low-level pipeline differs");
}
