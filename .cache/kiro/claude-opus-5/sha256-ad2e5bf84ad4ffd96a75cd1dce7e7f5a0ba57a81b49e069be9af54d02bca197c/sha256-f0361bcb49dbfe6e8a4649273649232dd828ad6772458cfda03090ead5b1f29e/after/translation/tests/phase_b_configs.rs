//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads both `.so`s via `libloading` and calls the exported `tfm`
//! symbol; no Rust code from the crate is called directly.

mod common;

use common::*;

/// Force the then arm: `src[0] < src[1]`.
fn then_arm_triple(rng: &mut Rng) -> [f32; 3] {
    loop {
        let a = rng.finite_f32();
        let b = rng.finite_f32();
        if a < b {
            return [a, b, rng.finite_f32()];
        }
        if b < a {
            return [b, a, rng.finite_f32()];
        }
    }
}

/// Force the else arm: `src[0] >= src[1]` (or unordered).
fn else_arm_triple(rng: &mut Rng) -> [f32; 3] {
    loop {
        let a = rng.finite_f32();
        let b = rng.finite_f32();
        if a >= b {
            return [a, b, rng.finite_f32()];
        }
        if b >= a {
            return [b, a, rng.finite_f32()];
        }
    }
}

// ---------------------------------------------------------------------------
// Row 1 — count = 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_row01_count_zero() {
    let mut rng = Rng::new(SEED ^ 1);
    for it in 0..2000 {
        // src still holds garbage; nothing may be read or written.
        let src: Vec<f32> = (0..9).map(|_| rng.any_bits_f32()).collect();
        diff_disjoint(&src, 0, 6, &format!("row01 count=0 [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 2 — count = 1, then arm
// ---------------------------------------------------------------------------
#[test]
fn cfg_row02_count1_then_arm() {
    let mut rng = Rng::new(SEED ^ 2);
    for it in 0..20_000 {
        let t = then_arm_triple(&mut rng);
        assert!(t[0] < t[1]);
        diff_disjoint(&t, 1, 2, &format!("row02 then-arm [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 3 — count = 1, else arm
// ---------------------------------------------------------------------------
#[test]
fn cfg_row03_count1_else_arm() {
    let mut rng = Rng::new(SEED ^ 3);
    for it in 0..20_000 {
        let t = else_arm_triple(&mut rng);
        assert!(!(t[0] < t[1]));
        diff_disjoint(&t, 1, 2, &format!("row03 else-arm [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 4 — count = 1, src[0] == src[1] exactly (incl. +0.0 / -0.0)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row04_count1_equal_arm() {
    let mut rng = Rng::new(SEED ^ 4);
    // Hand-picked zero-sign pairs first: +0/-0 compare equal, so the else arm
    // must run for all four combinations.
    for (a, b) in [(0.0f32, 0.0f32), (0.0, -0.0), (-0.0, 0.0), (-0.0, -0.0)] {
        for dxy in [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
            diff_disjoint(&[a, b, dxy], 1, 2, "row04 signed-zero pair");
        }
    }
    for it in 0..20_000 {
        let v = rng.finite_f32();
        let dxy = if it % 3 == 0 { rng.special_f32() } else { rng.finite_f32() };
        diff_disjoint(&[v, v, dxy], 1, 2, &format!("row04 equal [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 5 — count = 2, both arms inside one call (validates +=3 / +=2 strides)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row05_count2_mixed_arms() {
    let mut rng = Rng::new(SEED ^ 5);
    for it in 0..20_000 {
        let a = then_arm_triple(&mut rng);
        let b = else_arm_triple(&mut rng);
        let mut src = Vec::with_capacity(6);
        if it % 2 == 0 {
            src.extend_from_slice(&a);
            src.extend_from_slice(&b);
        } else {
            src.extend_from_slice(&b);
            src.extend_from_slice(&a);
        }
        diff_disjoint(&src, 2, 4, &format!("row05 mixed arms [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 6 — many elements, fully random bit patterns
// ---------------------------------------------------------------------------
#[test]
fn cfg_row06_many_random_bitpatterns() {
    let mut rng = Rng::new(SEED ^ 6);
    diff_rows(&mut rng, 8000, "row06 random bits", |rng, _| {
        let count = (rng.below(62) + 3) as i32;
        let src: Vec<f32> = (0..count as usize * 3).map(|_| rng.any_bits_f32()).collect();
        (src, count)
    });
}

// ---------------------------------------------------------------------------
// Row 7 — many elements, curated special-value pool
// ---------------------------------------------------------------------------
#[test]
fn cfg_row07_many_special_values() {
    let mut rng = Rng::new(SEED ^ 7);
    diff_rows(&mut rng, 8000, "row07 specials", |rng, _| {
        let count = (rng.below(62) + 3) as i32;
        let src: Vec<f32> = (0..count as usize * 3).map(|_| rng.special_f32()).collect();
        (src, count)
    });
}

// ---------------------------------------------------------------------------
// Row 8 — sqd < 0, so the `0 > sqd` clamp fires
// ---------------------------------------------------------------------------
#[test]
fn cfg_row08_sqd_negative_clamped() {
    let mut rng = Rng::new(SEED ^ 8);
    // sqd = (dy2-dx2)^2 + 4*dxy^2 is mathematically >= 0, so it only goes
    // negative through rounding: make (dy2-dx2)^2 underflow/cancel while the
    // subtraction of 2*dx2*dy2 dominates. dx2 ~= dy2 large, dxy tiny.
    let mut hit = 0usize;
    for it in 0..30_000 {
        let base = rng.finite_f32();
        // Perturb by a few ULPs so dy2*dy2 - 2*dx2*dy2 + dx2*dx2 cancels badly.
        let ulps = rng.below(5) as i32 - 2;
        let other = f32::from_bits((base.to_bits() as i32).wrapping_add(ulps) as u32);
        let dxy = f32::from_bits(rng.below(0x0080_0000)); // subnormal-ish, tiny
        let (a, b) = if it % 2 == 0 { (base, other) } else { (other, base) };
        // Track whether the clamp actually engaged (computed the same way as C).
        let (dx2, dy2) = if a < b { (a, b) } else { (b, a) };
        let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * dxy * dxy);
        if sqd < 0.0 {
            hit += 1;
        }
        diff_disjoint(&[a, b, dxy], 1, 2, &format!("row08 clamp [iter {it}]"));
    }
    assert!(hit > 0, "row08 never produced a negative sqd; test is not exercising the clamp");
    eprintln!("row08: clamp engaged on {hit} / 30000 inputs");
}

// ---------------------------------------------------------------------------
// Row 9 — sqd huge / near overflow, clamp passes through
// ---------------------------------------------------------------------------
#[test]
fn cfg_row09_sqd_huge() {
    let mut rng = Rng::new(SEED ^ 9);
    for it in 0..20_000 {
        // Exponent near the top so squaring overflows to +inf.
        let big = |rng: &mut Rng| {
            let exp = rng.below(20) + 235; // biased exponent 235..254
            let mant = rng.next_u32() & 0x007F_FFFF;
            let sign = (rng.next_u32() & 1) << 31;
            f32::from_bits(sign | (exp << 23) | mant)
        };
        let a = big(&mut rng);
        let b = big(&mut rng);
        let dxy = if it % 3 == 0 { rng.finite_f32() } else { big(&mut rng) };
        diff_disjoint(&[a, b, dxy], 1, 2, &format!("row09 huge sqd [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 10 — sqd is NaN (unordered), so the clamp does NOT fire
// ---------------------------------------------------------------------------
#[test]
fn cfg_row10_sqd_nan() {
    let mut rng = Rng::new(SEED ^ 10);
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;
    // inf - inf inside the discriminant, from finite inputs whose products
    // overflow, and from literal infinities.
    for a in [inf, ninf, f32::MAX, f32::MIN] {
        for b in [inf, ninf, f32::MAX, f32::MIN] {
            for dxy in [inf, ninf, f32::MAX, f32::MIN, 0.0, -0.0, 1.0] {
                diff_disjoint(&[a, b, dxy], 1, 2, "row10 inf-cancellation");
            }
        }
    }
    for it in 0..20_000 {
        let pick = |rng: &mut Rng| match rng.below(4) {
            0 => f32::INFINITY,
            1 => f32::NEG_INFINITY,
            2 => f32::MAX,
            _ => f32::MIN,
        };
        let a = pick(&mut rng);
        let b = pick(&mut rng);
        let dxy = pick(&mut rng);
        diff_disjoint(&[a, b, dxy], 1, 2, &format!("row10 nan sqd [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 11 — exactly one NaN lane, random payload/sign
// ---------------------------------------------------------------------------
#[test]
fn cfg_row11_single_nan_lane() {
    let mut rng = Rng::new(SEED ^ 11);
    for it in 0..30_000 {
        let lane = (it % 3) as usize;
        let mut t = [rng.finite_f32(), rng.finite_f32(), rng.finite_f32()];
        t[lane] = rng.nan_f32();
        diff_disjoint(&t, 1, 2, &format!("row11 NaN in lane {lane} [iter {it}]"));
    }
    // Also both-arm coverage with a NaN dxy only (the arm predicate stays
    // ordered, so the then arm is reachable with a NaN discriminant term).
    for it in 0..10_000 {
        let mut t = then_arm_triple(&mut rng);
        t[2] = rng.nan_f32();
        diff_disjoint(&t, 1, 2, &format!("row11 then-arm NaN dxy [iter {it}]"));
        let mut t = else_arm_triple(&mut rng);
        t[2] = rng.nan_f32();
        diff_disjoint(&t, 1, 2, &format!("row11 else-arm NaN dxy [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 12 — subnormal lanes
// ---------------------------------------------------------------------------
#[test]
fn cfg_row12_subnormals() {
    let mut rng = Rng::new(SEED ^ 12);
    let subnormal = |rng: &mut Rng| {
        let sign = (rng.next_u32() & 1) << 31;
        let mant = rng.below(0x0080_0000); // 0..2^23-1, exponent field zero
        f32::from_bits(sign | mant)
    };
    for it in 0..30_000 {
        let t = [subnormal(&mut rng), subnormal(&mut rng), subnormal(&mut rng)];
        diff_disjoint(&t, 1, 2, &format!("row12 subnormals [iter {it}]"));
    }
    // Mixed: subnormal discriminant term with normal arm operands.
    for it in 0..10_000 {
        let t = [rng.finite_f32(), rng.finite_f32(), subnormal(&mut rng)];
        diff_disjoint(&t, 1, 2, &format!("row12 mixed subnormal dxy [iter {it}]"));
        let t = [subnormal(&mut rng), subnormal(&mut rng), rng.finite_f32()];
        diff_disjoint(&t, 1, 2, &format!("row12 mixed subnormal arm [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 13 — infinities in every position
// ---------------------------------------------------------------------------
#[test]
fn cfg_row13_infinities() {
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0f32,
        -0.0f32,
        1.0f32,
        -1.0f32,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ];
    // Exhaustive 10^3 cross product — this is the full A4 grid for the
    // interesting classes, so no randomization is needed here.
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                diff_disjoint(&[a, b, c], 1, 2, "row13 inf/zero grid");
            }
        }
    }
    // And as a multi-element run so the strides are exercised too.
    let mut src = Vec::new();
    for &a in &vals {
        for &b in &vals {
            src.extend_from_slice(&[a, b, f32::INFINITY]);
        }
    }
    let count = (src.len() / 3) as i32;
    diff_disjoint(&src, count, count as usize * 2, "row13 inf run");
}

// ---------------------------------------------------------------------------
// Row 14 — fully aliased dest == src
// ---------------------------------------------------------------------------
#[test]
fn cfg_row14_aliased_in_place() {
    let mut rng = Rng::new(SEED ^ 14);
    for it in 0..20_000 {
        let count = (rng.below(32) + 1) as i32;
        // Backing store must cover both the 3*count reads and 2*count writes.
        let n = count as usize * 3 + 8;
        let buf: Vec<f32> = (0..n)
            .map(|_| if it % 4 == 0 { rng.special_f32() } else { rng.finite_f32() })
            .collect();
        diff_aliased(&buf, 0, 0, count, &format!("row14 dest==src [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 15 — partial overlap
// ---------------------------------------------------------------------------
#[test]
fn cfg_row15_partial_overlap() {
    let mut rng = Rng::new(SEED ^ 15);
    for it in 0..20_000 {
        let count = (rng.below(32) + 1) as i32;
        let n = count as usize * 3 + 16;
        let buf: Vec<f32> = (0..n)
            .map(|_| if it % 4 == 0 { rng.special_f32() } else { rng.finite_f32() })
            .collect();
        // dest one slot after src, and one slot before src.
        diff_aliased(&buf, 1, 0, count, &format!("row15 dest=src+1 [iter {it}]"));
        diff_aliased(&buf, 0, 1, count, &format!("row15 dest=src-1 [iter {it}]"));
        // Larger, still-overlapping offsets.
        let off = (rng.below(5) + 2) as usize;
        diff_aliased(&buf, off, 0, count, &format!("row15 dest=src+{off} [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 16 — large count
// ---------------------------------------------------------------------------
#[test]
fn cfg_row16_large_count() {
    let mut rng = Rng::new(SEED ^ 16);
    for it in 0..40 {
        let count = (rng.below(3073) + 1024) as i32;
        let src: Vec<f32> = (0..count as usize * 3)
            .map(|_| match it % 3 {
                0 => rng.any_bits_f32(),
                1 => rng.special_f32(),
                _ => rng.finite_f32(),
            })
            .collect();
        diff_disjoint(&src, count, count as usize * 2, &format!("row16 large count [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 17 — bulk pruned cross product of A1 x A2 x A4
// ---------------------------------------------------------------------------
#[test]
fn cfg_row17_bulk_cross_product() {
    let mut rng = Rng::new(SEED ^ 17);
    for it in 0..20_000 {
        let count = rng.below(49) as i32; // includes 0
        let mut src = Vec::with_capacity(count.max(1) as usize * 3);
        for _ in 0..count {
            let mut lane = [0.0f32; 3];
            for k in 0..3 {
                lane[k] = match rng.below(5) {
                    0 => rng.any_bits_f32(),
                    1 => rng.special_f32(),
                    2 => rng.nan_f32(),
                    3 => rng.small_f32(),
                    _ => rng.finite_f32(),
                };
            }
            // Sometimes make lane 1 a near-duplicate of lane 0 to straddle the
            // `<` boundary and force badly-cancelling discriminants.
            if rng.below(3) == 0 {
                let ulps = rng.below(7) as i32 - 3;
                lane[1] = f32::from_bits((lane[0].to_bits() as i32).wrapping_add(ulps) as u32);
            }
            src.extend_from_slice(&lane);
        }
        diff_disjoint(&src, count, count.max(0) as usize * 2, &format!("row17 bulk [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 18 — 1-ULP branch boundary
// ---------------------------------------------------------------------------
#[test]
fn cfg_row18_one_ulp_branch_boundary() {
    let mut rng = Rng::new(SEED ^ 18);
    for it in 0..30_000 {
        let base = match it % 4 {
            0 => rng.finite_f32(),
            1 => f32::from_bits(rng.below(0x0080_0000)), // subnormal
            2 => f32::from_bits(0x7F00_0000 | (rng.next_u32() & 0x007F_FFFF)), // huge
            _ => rng.small_f32(),
        };
        let up = f32::from_bits(base.to_bits().wrapping_add(1));
        let down = f32::from_bits(base.to_bits().wrapping_sub(1));
        let dxy = if it % 2 == 0 { rng.finite_f32() } else { rng.special_f32() };
        diff_disjoint(&[base, up, dxy], 1, 2, &format!("row18 base<up [iter {it}]"));
        diff_disjoint(&[up, base, dxy], 1, 2, &format!("row18 up>base [iter {it}]"));
        diff_disjoint(&[base, down, dxy], 1, 2, &format!("row18 base>down [iter {it}]"));
        diff_disjoint(&[down, base, dxy], 1, 2, &format!("row18 down<base [iter {it}]"));
    }
}
