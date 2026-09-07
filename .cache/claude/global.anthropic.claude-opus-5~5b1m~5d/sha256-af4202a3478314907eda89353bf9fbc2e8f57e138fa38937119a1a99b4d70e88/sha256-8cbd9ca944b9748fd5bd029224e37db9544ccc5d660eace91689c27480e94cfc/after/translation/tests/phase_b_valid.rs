//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! through `dlopen`/`dlsym` and compares outputs bit-for-bit over many
//! randomized inputs (fixed PRNG seeds, so failures are reproducible).

mod common;

use common::*;

/// Mirror of the C's `sqd` expression, in C's exact operand order, used only to
/// *classify* generated inputs into the clamp-axis buckets. It is never used as
/// an oracle — the oracle is always the C `.so`.
fn sqd_of(dx2: f32, dy2: f32, dxy: f32) -> f32 {
    (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * dxy * dxy)
}

/// Apply the C's branch selection to a raw triple, returning `(dx2, dy2, dxy)`
/// as the C would bind them.
fn bind(s: &[f32]) -> (f32, f32, f32) {
    if s[0] < s[1] {
        (s[0], s[1], s[2])
    } else {
        (s[1], s[0], s[2])
    }
}

fn took_if_branch(s: &[f32]) -> bool {
    s[0] < s[1]
}

// ===========================================================================
// Row 1 — count == 1, `if` branch forced, random finite normals
// ===========================================================================
#[test]
fn row01_count1_if_branch_random_finite() {
    let mut rng = Rng::new(0xA1);
    let mut n = 0;
    for _ in 0..20_000 {
        let a = rng.finite();
        let b = rng.finite();
        // Force src[0] < src[1].
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        if !(lo < hi) {
            continue;
        }
        let src = [lo, hi, rng.finite()];
        assert!(took_if_branch(&src));
        diff_call(&src, 1, "row01 if-branch");
        n += 1;
    }
    assert!(n > 10_000, "row01: only {n} cases generated");
}

// ===========================================================================
// Row 2 — count == 1, `else` branch forced (src[0] > src[1])
// ===========================================================================
#[test]
fn row02_count1_else_branch_random_finite() {
    let mut rng = Rng::new(0xA2);
    let mut n = 0;
    for _ in 0..20_000 {
        let a = rng.finite();
        let b = rng.finite();
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        if !(lo < hi) {
            continue;
        }
        let src = [hi, lo, rng.finite()];
        assert!(!took_if_branch(&src));
        diff_call(&src, 1, "row02 else-branch");
        n += 1;
    }
    assert!(n > 10_000, "row02: only {n} cases generated");
}

// ===========================================================================
// Row 3 — count == 1, exact equality src[0] == src[1] (boundary of `<`)
// ===========================================================================
#[test]
fn row03_count1_equal_operands_boundary() {
    let mut rng = Rng::new(0xA3);
    for _ in 0..20_000 {
        let v = rng.finite();
        let src = [v, v, rng.finite()];
        assert!(!took_if_branch(&src), "equality must take the else branch");
        diff_call(&src, 1, "row03 equal");
    }
    // Also the signed-zero equality boundary: -0.0 < 0.0 is false.
    for &(a, b) in &[(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
        for &c in SPECIALS {
            assert!(!took_if_branch(&[a, b, c]));
            diff_call(&[a, b, c], 1, "row03 signed-zero equality");
        }
    }
    // And one-ulp-apart neighbours on both sides of the boundary.
    let mut rng = Rng::new(0xA33);
    for _ in 0..20_000 {
        let v = rng.finite();
        let up = f32::from_bits(v.to_bits().wrapping_add(1));
        let dn = f32::from_bits(v.to_bits().wrapping_sub(1));
        for pair in [(v, up), (up, v), (v, dn), (dn, v)] {
            if !pair.0.is_finite() || !pair.1.is_finite() {
                continue;
            }
            diff_call(&[pair.0, pair.1, rng.finite()], 1, "row03 one-ulp");
        }
    }
}

// ===========================================================================
// Row 4 — count == 1, unconstrained random finite normals (data-chosen branch)
// ===========================================================================
#[test]
fn row04_count1_unconstrained_random_finite() {
    let mut rng = Rng::new(0xA4);
    let mut ifs = 0;
    let mut elses = 0;
    for _ in 0..50_000 {
        let src = [rng.finite(), rng.finite(), rng.finite()];
        if took_if_branch(&src) {
            ifs += 1;
        } else {
            elses += 1;
        }
        diff_call(&src, 1, "row04 unconstrained");
    }
    assert!(ifs > 1_000 && elses > 1_000, "row04 branch coverage: if={ifs} else={elses}");
}

// ===========================================================================
// Rows 5, 6, 7 — the clamp axis: sqd > 0, sqd == +0.0, sqd < 0
// ===========================================================================

#[test]
fn row05_clamp_positive_discriminant() {
    let mut rng = Rng::new(0xA5);
    let mut n = 0;
    for _ in 0..200_000 {
        let src = [rng.finite(), rng.finite(), rng.finite()];
        let (dx2, dy2, dxy) = bind(&src);
        let s = sqd_of(dx2, dy2, dxy);
        if !(s > 0.0) || !s.is_finite() {
            continue;
        }
        diff_call(&src, 1, "row05 sqd>0");
        n += 1;
        if n >= 30_000 {
            break;
        }
    }
    assert!(n > 10_000, "row05: only {n} positive-discriminant cases");
}

#[test]
fn row06_clamp_exactly_plus_zero_discriminant() {
    // dx2 == dy2 and dxy == 0 makes the discriminant exactly +0.0:
    //   (v*v) - (2*v*v) + (v*v) == 0 exactly (the doubling is exact), and
    //   4*0*0 == +0.0.
    let mut rng = Rng::new(0xA6);
    let mut n = 0;
    for _ in 0..40_000 {
        let v = rng.finite();
        for &z in &[0.0f32, -0.0f32] {
            let src = [v, v, z];
            let (dx2, dy2, dxy) = bind(&src);
            let s = sqd_of(dx2, dy2, dxy);
            if s.to_bits() != 0.0f32.to_bits() {
                continue;
            }
            diff_call(&src, 1, "row06 sqd==+0");
            n += 1;
        }
    }
    assert!(n > 10_000, "row06: only {n} exactly-zero-discriminant cases");

    // Explicit small hand-checked set too.
    for &v in &[1.0f32, -1.0, 2.0, 0.5, 1e-30, 1e30, f32::MIN_POSITIVE] {
        for &z in &[0.0f32, -0.0] {
            diff_call(&[v, v, z], 1, "row06 explicit");
        }
    }
}

#[test]
fn row07_clamp_negative_discriminant() {
    // Catastrophic cancellation in the *expanded* discriminant makes it
    // genuinely negative even though the factored form is >= 0. Generate
    // dx2 ~ dy2 (a few ulps apart) with a tiny dxy.
    let mut rng = Rng::new(0xA7);
    let mut n = 0;
    for _ in 0..400_000 {
        let base = f32::from_bits(0x3f80_0000u32.wrapping_add(rng.next_u32() & 0x0fff_ffff));
        if !base.is_finite() || base == 0.0 {
            continue;
        }
        let k = (rng.next_u32() & 0x3ff) as i32 - 512;
        let other = f32::from_bits((base.to_bits() as i32).wrapping_add(k) as u32);
        let dxy = f32::from_bits(
            base.to_bits()
                .wrapping_sub((rng.next_u32() & 0x0f00_0000) + 0x0800_0000),
        );
        if !other.is_finite() || !dxy.is_finite() {
            continue;
        }
        // Try both orderings so both C branches are exercised.
        for src in [[base, other, dxy], [other, base, dxy]] {
            let (dx2, dy2, d) = bind(&src);
            let s = sqd_of(dx2, dy2, d);
            if !(s < 0.0) {
                continue;
            }
            diff_call(&src, 1, "row07 sqd<0 (clamped)");
            n += 1;
        }
        if n >= 30_000 {
            break;
        }
    }
    assert!(n > 5_000, "row07: only {n} negative-discriminant cases");

    // The known-good literal cases found by exhaustive search.
    for &(a, b, c) in &[
        (0x4dfc_bb3cu32, 0x4dfc_bb09u32, 0x45fc_bb3cu32),
        (0x430d_d2a4, 0x430d_d24e, 0x370d_d2a4),
        (0x418a_179a, 0x418a_15ed, 0x328a_179a),
        (0x4a5e_90b4, 0x4a5e_9053, 0x365e_90b4),
        (0x4bc1_fe94, 0x4bc1_fcd8, 0x3ac1_fe94),
    ] {
        let (x, y, z) = (f32::from_bits(a), f32::from_bits(b), f32::from_bits(c));
        for src in [[x, y, z], [y, x, z]] {
            diff_call(&src, 1, "row07 literal sqd<0");
        }
    }
}

// ===========================================================================
// Row 8 — dxy == 0 (the 4*dxy*dxy term vanishes), both zero signs
// ===========================================================================
#[test]
fn row08_dxy_zero_both_signs() {
    let mut rng = Rng::new(0xA8);
    for _ in 0..20_000 {
        let a = rng.finite();
        let b = rng.finite();
        for &z in &[0.0f32, -0.0f32] {
            diff_call(&[a, b, z], 1, "row08 dxy=0");
            diff_call(&[b, a, z], 1, "row08 dxy=0 swapped");
        }
    }
}

// ===========================================================================
// Row 9 — perfect-square discriminants (exact sqrtf results)
// ===========================================================================
#[test]
fn row09_exact_square_roots() {
    // dxy == 0 and dy2 - dx2 a power of two gives an exactly-representable
    // discriminant with an exact square root.
    let mut rng = Rng::new(0xA9);
    for _ in 0..5_000 {
        let e = (rng.below(60) as i32) - 30;
        let d = (2.0f64).powi(e) as f32;
        let base = rng.normal_in(1.0, 1e3);
        for src in [[base, base + d, 0.0f32], [base + d, base, 0.0f32]] {
            diff_call(&src, 1, "row09 exact sqrt");
        }
    }
    // Integer discriminants that are perfect squares: dx2=0, dy2=n, dxy=0
    // gives sqd = n*n and sqrtf(n*n) = n exactly.
    for n in 1..=512u32 {
        let v = n as f32;
        diff_call(&[0.0, v, 0.0], 1, "row09 integer square");
        diff_call(&[v, 0.0, 0.0], 1, "row09 integer square swapped");
        diff_call(&[-v, 0.0, 0.0], 1, "row09 integer square neg");
    }
}

// ===========================================================================
// Row 10 — exhaustive 3-way cross product of the special-value set
// ===========================================================================
#[test]
fn row10_specials_cross_product_exhaustive() {
    let mut n = 0;
    for &a in SPECIALS {
        for &b in SPECIALS {
            for &c in SPECIALS {
                diff_call(&[a, b, c], 1, "row10 specials");
                n += 1;
            }
        }
    }
    assert_eq!(n, SPECIALS.len().pow(3));

    // The same cross product with every NaN pattern substituted in each slot.
    let nans = nan_patterns();
    for &nv in &nans {
        for &a in SPECIALS {
            for &b in SPECIALS {
                diff_call(&[nv, a, b], 1, "row10 nan@0");
                diff_call(&[a, nv, b], 1, "row10 nan@1");
                diff_call(&[a, b, nv], 1, "row10 nan@2");
            }
        }
    }
}

// ===========================================================================
// Row 11 — huge magnitudes: overflow to inf and inf - inf invalid ops
// ===========================================================================
#[test]
fn row11_overflow_to_infinity() {
    let mut rng = Rng::new(0xAB);
    for _ in 0..40_000 {
        // Magnitudes above sqrt(FLT_MAX) ~ 1.8e19 make x*x overflow.
        let a = rng.normal_in(1.0e19, f32::MAX as f32 * 0.99);
        let b = rng.normal_in(1.0e19, f32::MAX as f32 * 0.99);
        let c = rng.normal_in(1.0e19, f32::MAX as f32 * 0.99);
        diff_call(&[a, b, c], 1, "row11 overflow");
        diff_call(&[b, a, c], 1, "row11 overflow swapped");
    }
    // FLT_MAX / inf corners in every slot combination.
    let big = [f32::MAX, f32::MIN, f32::INFINITY, f32::NEG_INFINITY, 1.0e38, -1.0e38];
    for &a in &big {
        for &b in &big {
            for &c in &big {
                diff_call(&[a, b, c], 1, "row11 big corners");
            }
        }
    }
    // Mixed: huge with 0 (0 * inf invalid op) — the documented indefinite-NaN path.
    for &a in &big {
        for &b in &big {
            for &z in &[0.0f32, -0.0f32] {
                diff_call(&[a, b, z], 1, "row11 huge+zero");
                diff_call(&[a, z, b], 1, "row11 huge+zero");
                diff_call(&[z, a, b], 1, "row11 huge+zero");
            }
        }
    }
}

// ===========================================================================
// Row 12 — tiny magnitudes: underflow to subnormal / zero
// ===========================================================================
#[test]
fn row12_underflow_subnormal() {
    let mut rng = Rng::new(0xAC);
    for _ in 0..40_000 {
        let a = rng.normal_in(1.0e-44, 1.0e-20);
        let b = rng.normal_in(1.0e-44, 1.0e-20);
        let c = rng.normal_in(1.0e-44, 1.0e-20);
        diff_call(&[a, b, c], 1, "row12 underflow");
        diff_call(&[b, a, c], 1, "row12 underflow swapped");
    }
    // Random raw subnormal bit patterns (exponent field == 0, non-zero mantissa).
    let mut rng = Rng::new(0xACC);
    let sub = |r: &mut Rng| -> f32 {
        let m = (r.next_u32() & 0x007F_FFFF) | 1;
        let s = (r.next_u32() & 1) << 31;
        f32::from_bits(s | m)
    };
    for _ in 0..40_000 {
        let (a, b, c) = (sub(&mut rng), sub(&mut rng), sub(&mut rng));
        diff_call(&[a, b, c], 1, "row12 raw subnormal");
    }
    // Explicit boundary values.
    let tiny = [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
        0.0,
        -0.0,
    ];
    for &a in &tiny {
        for &b in &tiny {
            for &c in &tiny {
                diff_call(&[a, b, c], 1, "row12 tiny corners");
            }
        }
    }
}

// ===========================================================================
// Row 13 — fully random 32-bit bit patterns (every class at once)
// ===========================================================================
#[test]
fn row13_random_bit_patterns() {
    let mut rng = Rng::new(0xAD);
    for _ in 0..300_000 {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        diff_call(&src, 1, "row13 random bits");
    }
}

// ===========================================================================
// Row 14 — mixed magnitudes (all 6 permutations) → catastrophic cancellation
// ===========================================================================
#[test]
fn row14_mixed_magnitudes_permutations() {
    let mut rng = Rng::new(0xAE);
    for _ in 0..20_000 {
        let huge = rng.normal_in(1.0e20, 1.0e38);
        let tiny = rng.normal_in(1.0e-38, 1.0e-20);
        let mid = rng.normal_in(0.1, 10.0);
        let perms = [
            [huge, tiny, mid],
            [huge, mid, tiny],
            [tiny, huge, mid],
            [tiny, mid, huge],
            [mid, huge, tiny],
            [mid, tiny, huge],
        ];
        for src in perms {
            diff_call(&src, 1, "row14 mixed magnitude");
        }
    }
    // Near-cancellation: dy2 and dx2 within a few ulps, dxy spanning scales.
    let mut rng = Rng::new(0xAEE);
    for _ in 0..40_000 {
        let v = rng.finite();
        let k = (rng.next_u32() & 0xff) as i32 - 128;
        let w = f32::from_bits((v.to_bits() as i32).wrapping_add(k) as u32);
        if !w.is_finite() {
            continue;
        }
        let c = rng.normal_in(1.0e-38, 1.0e38);
        diff_call(&[v, w, c], 1, "row14 near-cancel");
        diff_call(&[w, v, c], 1, "row14 near-cancel swapped");
    }
}

// ===========================================================================
// Row 15 — count == 2 (pointer advance src+=3 / dest+=2)
// ===========================================================================
#[test]
fn row15_count2() {
    let mut rng = Rng::new(0xB0);
    for _ in 0..50_000 {
        let src: Vec<f32> = (0..6).map(|_| rng.finite()).collect();
        diff_call(&src, 2, "row15 count=2");
    }
    // count == 2 with specials in every slot of the second triple.
    let mut rng = Rng::new(0xB00);
    for &s in SPECIALS {
        for i in 0..6 {
            let mut src: Vec<f32> = (0..6).map(|_| rng.finite()).collect();
            src[i] = s;
            diff_call(&src, 2, "row15 count=2 special");
        }
    }
}

// ===========================================================================
// Row 16 — vector-width boundaries and odd tails
// ===========================================================================
#[test]
fn row16_count_boundaries() {
    let counts = [3usize, 4, 5, 7, 8, 15, 16, 17, 31, 33];
    let mut rng = Rng::new(0xB1);
    for &n in &counts {
        for _ in 0..2_000 {
            let src: Vec<f32> = (0..3 * n).map(|_| rng.finite()).collect();
            diff_call(&src, n as i32, &format!("row16 count={n}"));
        }
        // Same counts with random bit patterns.
        for _ in 0..2_000 {
            let src: Vec<f32> = (0..3 * n).map(|_| rng.any_bits()).collect();
            diff_call(&src, n as i32, &format!("row16 count={n} bits"));
        }
    }
}

// ===========================================================================
// Row 17 — count == 1000 (large), random finite normals
// ===========================================================================
#[test]
fn row17_count_1000_finite() {
    let mut rng = Rng::new(0xB2);
    for _ in 0..200 {
        let src: Vec<f32> = (0..3_000).map(|_| rng.finite()).collect();
        diff_call(&src, 1_000, "row17 count=1000");
    }
    // A really long one, too.
    let src: Vec<f32> = (0..3 * 100_000).map(|_| rng.finite()).collect();
    diff_call(&src, 100_000, "row17 count=100000");
}

// ===========================================================================
// Row 18 — count == 1000 with random bit patterns
// ===========================================================================
#[test]
fn row18_count_1000_random_bits() {
    let mut rng = Rng::new(0xB3);
    for _ in 0..200 {
        let src: Vec<f32> = (0..3_000).map(|_| rng.any_bits()).collect();
        diff_call(&src, 1_000, "row18 count=1000 bits");
    }
    let src: Vec<f32> = (0..3 * 50_000).map(|_| rng.any_bits()).collect();
    diff_call(&src, 50_000, "row18 count=50000 bits");
}

// ===========================================================================
// Row 19 — large count drawn from the special-value set (alternating branches)
// ===========================================================================
#[test]
fn row19_large_count_specials() {
    let mut rng = Rng::new(0xB4);
    let mut pool: Vec<f32> = SPECIALS.to_vec();
    pool.extend(nan_patterns());
    for _ in 0..300 {
        let n = 1 + rng.below(500);
        let src: Vec<f32> = (0..3 * n).map(|_| pool[rng.below(pool.len())]).collect();
        diff_call(&src, n as i32, "row19 large specials");
    }
    // Deterministic sweep: every triple of the specials set, laid end to end in
    // one long call, so consecutive iterations alternate branch and clamp.
    let mut src: Vec<f32> = Vec::new();
    for &a in SPECIALS {
        for &b in SPECIALS {
            for &c in SPECIALS {
                src.extend_from_slice(&[a, b, c]);
            }
        }
    }
    let n = (src.len() / 3) as i32;
    diff_call(&src, n, "row19 full specials sweep in one call");
}

// ===========================================================================
// Rows 20-23 — aliasing (no `restrict`, mismatched strides)
// ===========================================================================

/// Aliasing helper: both implementations get their own copy of the same buffer,
/// `dest` is `buf + dest_off`, `src` is `buf + src_off`.
#[track_caller]
fn diff_alias(buf: &[f32], dest_off: usize, src_off: usize, count: i32, ctx: &str) {
    let mut c_buf = buf.to_vec();
    let mut r_buf = buf.to_vec();
    let l = libs();
    unsafe {
        (l.c_tfm)(
            c_buf.as_mut_ptr().add(dest_off),
            c_buf.as_ptr().add(src_off),
            count,
        );
        (l.rust_tfm)(
            r_buf.as_mut_ptr().add(dest_off),
            r_buf.as_ptr().add(src_off),
            count,
        );
    }
    assert!(
        bits_eq(&c_buf, &r_buf),
        "ALIAS MISMATCH [{ctx}]\n  count={count} dest_off={dest_off} src_off={src_off}\n  \
         in   = {}\n  C    = {}\n  Rust = {}",
        show(buf),
        show(&c_buf),
        show(&r_buf),
    );
}

#[test]
fn row20_exact_aliasing_dest_eq_src() {
    let mut rng = Rng::new(0xC0);
    for count in 1..=64usize {
        for _ in 0..200 {
            // Buffer must be big enough for both 3*count reads and 2*count writes.
            let buf: Vec<f32> = (0..3 * count + 8).map(|_| rng.finite()).collect();
            diff_alias(&buf, 0, 0, count as i32, "row20 dest==src");
        }
    }
}

#[test]
fn row21_forward_partial_overlap() {
    let mut rng = Rng::new(0xC1);
    for k in 1..=6usize {
        for count in 1..=32usize {
            for _ in 0..50 {
                let len = 3 * count + k + 8;
                let buf: Vec<f32> = (0..len).map(|_| rng.finite()).collect();
                // dest = base, src = base + k  (src ahead of dest)
                diff_alias(&buf, 0, k, count as i32, "row21 src=dest+k");
            }
        }
    }
}

#[test]
fn row22_backward_partial_overlap() {
    let mut rng = Rng::new(0xC2);
    for k in 1..=6usize {
        for count in 1..=32usize {
            for _ in 0..50 {
                let len = 3 * count + k + 8;
                let buf: Vec<f32> = (0..len).map(|_| rng.finite()).collect();
                // dest = base + k, src = base  (dest ahead of src)
                diff_alias(&buf, k, 0, count as i32, "row22 dest=src+k");
            }
        }
    }
}

#[test]
fn row23_aliasing_random_bit_patterns() {
    let mut rng = Rng::new(0xC3);
    for count in 1..=32usize {
        for _ in 0..200 {
            let buf: Vec<f32> = (0..3 * count + 8).map(|_| rng.any_bits()).collect();
            diff_alias(&buf, 0, 0, count as i32, "row23 dest==src bits");
        }
    }
    // Plus overlapping offsets with bit patterns.
    for k in 0..=6usize {
        for count in 1..=16usize {
            for _ in 0..100 {
                let buf: Vec<f32> = (0..3 * count + k + 8).map(|_| rng.any_bits()).collect();
                diff_alias(&buf, 0, k, count as i32, "row23 overlap bits");
                diff_alias(&buf, k, 0, count as i32, "row23 overlap bits rev");
            }
        }
    }
}

// ===========================================================================
// Row 24 — exact read/write extents (guards on both sides of both buffers)
// ===========================================================================
#[test]
fn row24_exact_extents_with_guards() {
    const G: usize = 8;
    let mut rng = Rng::new(0xC4);
    for count in 1..=8usize {
        for _ in 0..2_000 {
            let src_live: Vec<f32> = (0..3 * count).map(|_| rng.finite()).collect();

            // src with poison guards fore and aft. If the implementation read
            // outside [G, G+3*count) the results would differ from a call on a
            // tightly-sized buffer.
            let mut padded: Vec<f32> = vec![f32::NAN; G];
            padded.extend_from_slice(&src_live);
            padded.extend(std::iter::repeat(f32::NAN).take(G));

            let mut c_dest = vec![FILL; 2 * count + G];
            let mut r_dest = vec![FILL; 2 * count + G];
            let l = libs();
            unsafe {
                (l.c_tfm)(
                    c_dest.as_mut_ptr(),
                    padded.as_ptr().add(G),
                    count as i32,
                );
                (l.rust_tfm)(
                    r_dest.as_mut_ptr(),
                    padded.as_ptr().add(G),
                    count as i32,
                );
            }
            assert!(bits_eq(&c_dest, &r_dest), "row24 mismatch count={count}");

            // Exactly 2*count writes: guard slots untouched in BOTH.
            for i in 2 * count..c_dest.len() {
                assert_eq!(c_dest[i].to_bits(), FILL.to_bits(), "row24 C overwrote {i}");
                assert_eq!(r_dest[i].to_bits(), FILL.to_bits(), "row24 Rust overwrote {i}");
            }

            // Same input in a tight buffer must give the same answer (proves no
            // read past 3*count influenced the result).
            let tight = diff_call(&src_live, count as i32, "row24 tight");
            assert!(bits_eq(&tight, &c_dest[..2 * count]), "row24 padded vs tight differ");
        }
    }
}

// ===========================================================================
// Row 25 — relatively-unaligned src/dest inside larger allocations
// ===========================================================================
#[test]
fn row25_unaligned_relative_offsets() {
    let mut rng = Rng::new(0xC5);
    for so in 0..4usize {
        for dof in 0..4usize {
            for count in 1..=17usize {
                for _ in 0..40 {
                    let src_buf: Vec<f32> =
                        (0..so + 3 * count + 4).map(|_| rng.any_bits()).collect();
                    let mut c_dest = vec![FILL; dof + 2 * count + 4];
                    let mut r_dest = c_dest.clone();
                    let l = libs();
                    unsafe {
                        (l.c_tfm)(
                            c_dest.as_mut_ptr().add(dof),
                            src_buf.as_ptr().add(so),
                            count as i32,
                        );
                        (l.rust_tfm)(
                            r_dest.as_mut_ptr().add(dof),
                            src_buf.as_ptr().add(so),
                            count as i32,
                        );
                    }
                    assert!(
                        bits_eq(&c_dest, &r_dest),
                        "row25 mismatch so={so} dof={dof} count={count}\n C={}\n R={}",
                        show(&c_dest),
                        show(&r_dest)
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 26 — statelessness across repeated back-to-back calls
// ===========================================================================
#[test]
fn row26_stateless_repeated_calls() {
    let mut rng = Rng::new(0xC6);
    let l = libs();
    for _ in 0..2_000 {
        let count = 1 + rng.below(16);
        let src: Vec<f32> = (0..3 * count).map(|_| rng.any_bits()).collect();

        let mut c_dest = vec![FILL; 2 * count];
        let mut r_dest = vec![FILL; 2 * count];
        let mut first_c: Option<Vec<f32>> = None;

        // Ten repetitions, interleaving unrelated calls between them.
        for _ in 0..10 {
            let junk: Vec<f32> = (0..30).map(|_| rng.any_bits()).collect();
            let mut junk_out = vec![0.0f32; 20];
            unsafe {
                (l.c_tfm)(junk_out.as_mut_ptr(), junk.as_ptr(), 10);
                (l.rust_tfm)(junk_out.as_mut_ptr(), junk.as_ptr(), 10);
                (l.c_tfm)(c_dest.as_mut_ptr(), src.as_ptr(), count as i32);
                (l.rust_tfm)(r_dest.as_mut_ptr(), src.as_ptr(), count as i32);
            }
            assert!(bits_eq(&c_dest, &r_dest), "row26 mismatch");
            match &first_c {
                None => first_c = Some(c_dest.clone()),
                Some(f) => assert!(bits_eq(f, &c_dest), "row26 C not stateless"),
            }
        }
    }
}
