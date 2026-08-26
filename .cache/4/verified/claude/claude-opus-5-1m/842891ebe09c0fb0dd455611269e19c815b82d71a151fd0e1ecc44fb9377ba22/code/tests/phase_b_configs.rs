//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md` (`c01_*` .. `c18_*`). Every test drives
//! BOTH shared objects through their exported `contrast_ratio` symbol and
//! asserts the returned `f32` matches bit-for-bit.

mod common;

use common::*;

/// Scale factor for the randomized sweeps, so CI can dial the run time.
fn n(default: usize) -> usize {
    match std::env::var("DIFF_N") {
        Ok(v) => v.parse().unwrap_or(default),
        Err(_) => default,
    }
}

// ---------------------------------------------------------------------------
// C1 — both colours entirely on the linear (`x / 12.92`) branch
// ---------------------------------------------------------------------------
#[test]
fn c01_both_all_linear_branch() {
    let p = pair();
    let mut rng = Rng::new(0xC001_0001);
    for _ in 0..n(20_000) {
        let a = Rgb::new(
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
        );
        let b = Rgb::new(
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
        );
        assert_same(p, a, b, "C1 all-linear");
    }
    // Exhaustive over the whole linear sub-cube would be 11^6; instead cover
    // every (A,B) pair of the 11^2 greys on that branch exhaustively.
    for na in 0..=LAST_LINEAR {
        for nb in 0..=LAST_LINEAR {
            assert_same(
                p,
                Rgb::new(na, na, na),
                Rgb::new(nb, nb, nb),
                "C1 all-linear greys",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C2 — both colours entirely on the `pow` branch
// ---------------------------------------------------------------------------
#[test]
fn c02_both_all_pow_branch() {
    let p = pair();
    let mut rng = Rng::new(0xC002_0002);
    for _ in 0..n(20_000) {
        let a = Rgb::new(
            rng.range_u8(FIRST_POW, 255),
            rng.range_u8(FIRST_POW, 255),
            rng.range_u8(FIRST_POW, 255),
        );
        let b = Rgb::new(
            rng.range_u8(FIRST_POW, 255),
            rng.range_u8(FIRST_POW, 255),
            rng.range_u8(FIRST_POW, 255),
        );
        assert_same(p, a, b, "C2 all-pow");
    }
}

// ---------------------------------------------------------------------------
// C3 — A all-linear (dark), B all-pow (bright) -> the `High < Low` swap fires
// ---------------------------------------------------------------------------
#[test]
fn c03_dark_a_bright_b_forces_swap() {
    let p = pair();
    let mut rng = Rng::new(0xC003_0003);
    let mut swaps = 0usize;
    for _ in 0..n(20_000) {
        let a = Rgb::new(
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
        );
        let b = Rgb::new(
            rng.range_u8(200, 255),
            rng.range_u8(200, 255),
            rng.range_u8(200, 255),
        );
        if approx_luminance(a) < approx_luminance(b) {
            swaps += 1;
        }
        assert_same(p, a, b, "C3 dark-A bright-B");
    }
    assert!(swaps > 0, "C3 never exercised the swap branch");
}

// ---------------------------------------------------------------------------
// C4 — A all-pow (bright), B all-linear (dark) -> the swap does not fire
// ---------------------------------------------------------------------------
#[test]
fn c04_bright_a_dark_b_no_swap() {
    let p = pair();
    let mut rng = Rng::new(0xC004_0004);
    let mut no_swaps = 0usize;
    for _ in 0..n(20_000) {
        let a = Rgb::new(
            rng.range_u8(200, 255),
            rng.range_u8(200, 255),
            rng.range_u8(200, 255),
        );
        let b = Rgb::new(
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
            rng.range_u8(0, LAST_LINEAR),
        );
        if approx_luminance(a) >= approx_luminance(b) {
            no_swaps += 1;
        }
        assert_same(p, a, b, "C4 bright-A dark-B");
    }
    assert!(no_swaps > 0, "C4 never exercised the no-swap branch");
}

// ---------------------------------------------------------------------------
// C5 — full 8x8 cross product of the per-channel branch masks
// ---------------------------------------------------------------------------
#[test]
fn c05_branch_mask_cross_product() {
    let p = pair();
    let per_combo = n(2_000);
    for mask_a in 0u8..8 {
        for mask_b in 0u8..8 {
            let mut rng = Rng::new(0xC005_0000 ^ ((mask_a as u64) << 8) ^ mask_b as u64);
            for _ in 0..per_combo {
                let a = color_for_mask(&mut rng, mask_a);
                let b = color_for_mask(&mut rng, mask_b);
                // Sanity: the drawn bytes really do select the intended branches.
                for (byte, want_pow) in [
                    (a.r, mask_a & 1 != 0),
                    (a.g, mask_a & 2 != 0),
                    (a.b, mask_a & 4 != 0),
                    (b.r, mask_b & 1 != 0),
                    (b.g, mask_b & 2 != 0),
                    (b.b, mask_b & 4 != 0),
                ] {
                    assert_eq!(byte >= FIRST_POW, want_pow, "branch-mask setup broken");
                }
                assert_same(p, a, b, &format!("C5 mask_a={mask_a:03b} mask_b={mask_b:03b}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C6 — strict no-swap (LumA > LumB), normal denominator
// ---------------------------------------------------------------------------
#[test]
fn c06_strict_no_swap_normal_denominator() {
    let p = pair();
    let mut rng = Rng::new(0xC006_0006);
    let mut hits = 0usize;
    let target = n(20_000);
    let mut tries = 0usize;
    while hits < target && tries < target * 40 {
        tries += 1;
        let a = rng.color();
        let b = rng.color();
        let (la, lb) = (approx_luminance(a), approx_luminance(b));
        if la > lb && lb > 0.0 {
            assert_same(p, a, b, "C6 no-swap normal");
            hits += 1;
        }
    }
    assert!(hits >= target / 2, "C6 produced too few samples: {hits}");
}

// ---------------------------------------------------------------------------
// C7 — strict swap (LumA < LumB), normal denominator
// ---------------------------------------------------------------------------
#[test]
fn c07_strict_swap_normal_denominator() {
    let p = pair();
    let mut rng = Rng::new(0xC007_0007);
    let mut hits = 0usize;
    let target = n(20_000);
    let mut tries = 0usize;
    while hits < target && tries < target * 40 {
        tries += 1;
        let a = rng.color();
        let b = rng.color();
        let (la, lb) = (approx_luminance(a), approx_luminance(b));
        if la < lb && la > 0.0 {
            assert_same(p, a, b, "C7 swap normal");
            hits += 1;
        }
    }
    assert!(hits >= target / 2, "C7 produced too few samples: {hits}");
}

// ---------------------------------------------------------------------------
// C8 — equality edge: A == B  (`High < Low` is false)
// ---------------------------------------------------------------------------
#[test]
fn c08_equal_colors_equality_edge() {
    let p = pair();
    // Exhaustive over all 256 greys.
    for v in 0u16..=255 {
        let c = Rgb::new(v as u8, v as u8, v as u8);
        assert_same(p, c, c, "C8 A==B grey");
    }
    // Exhaustive over all 256 values in each single channel position.
    for v in 0u16..=255 {
        let v = v as u8;
        for c in [Rgb::new(v, 0, 0), Rgb::new(0, v, 0), Rgb::new(0, 0, v)] {
            assert_same(p, c, c, "C8 A==B single-channel");
        }
    }
    let mut rng = Rng::new(0xC008_0008);
    for _ in 0..n(20_000) {
        let c = rng.color();
        assert_same(p, c, c, "C8 A==B random");
    }
}

// ---------------------------------------------------------------------------
// C9 — isolate the three luminance coefficients
// ---------------------------------------------------------------------------
#[test]
fn c09_single_channel_weight_isolation() {
    let p = pair();
    let partners = [
        BLACK,
        WHITE,
        MID,
        Rgb::new(255, 0, 0),
        Rgb::new(0, 255, 0),
        Rgb::new(0, 0, 255),
        Rgb::new(10, 11, 12),
        Rgb::new(11, 10, 11),
        Rgb::new(1, 0, 0),
        Rgb::new(0, 1, 0),
        Rgb::new(0, 0, 1),
    ];
    for v in 0u16..=255 {
        let v = v as u8;
        let singles = [Rgb::new(v, 0, 0), Rgb::new(0, v, 0), Rgb::new(0, 0, v)];
        for a in singles {
            for b in partners {
                assert_same(p, a, b, "C9 single-channel A");
                assert_same(p, b, a, "C9 single-channel B");
            }
            for b in singles {
                assert_same(p, a, b, "C9 single x single");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — exhaustive 8x8 corner cross product
// ---------------------------------------------------------------------------
#[test]
fn c10_corner_cross_product_exhaustive() {
    let p = pair();
    let cs = corners();
    let mut saw_nan = false;
    let mut saw_inf = false;
    let mut saw_one = false;
    for &a in &cs {
        for &b in &cs {
            let v = assert_same(p, a, b, "C10 corners");
            if v.is_nan() {
                saw_nan = true;
            }
            if v.is_infinite() {
                saw_inf = true;
            }
            if v == 1.0 {
                saw_one = true;
            }
        }
    }
    assert!(saw_nan, "C10 should have produced the 0/0 NaN (black vs black)");
    assert!(saw_inf, "C10 should have produced +inf (black vs non-black)");
    assert!(saw_one, "C10 should have produced ratio == 1 (A == B)");
}

// ---------------------------------------------------------------------------
// C11 — exhaustive single-position sweep over all 6 channel positions
// ---------------------------------------------------------------------------
#[test]
fn c11_exhaustive_per_position_sweep() {
    let p = pair();
    let mut rng = Rng::new(0xC011_0011);
    let random_bg = (rng.color(), rng.color());
    let backgrounds: [(Rgb, Rgb); 6] = [
        (BLACK, BLACK),
        (Rgb::new(11, 11, 11), Rgb::new(11, 11, 11)),
        (MID, MID),
        (WHITE, WHITE),
        (Rgb::new(10, 11, 10), Rgb::new(11, 10, 11)),
        random_bg,
    ];
    for (bi, (ba, bb)) in backgrounds.into_iter().enumerate() {
        for pos in 0..6usize {
            for v in 0u16..=255 {
                let v = v as u8;
                let mut a = ba;
                let mut b = bb;
                match pos {
                    0 => a.r = v,
                    1 => a.g = v,
                    2 => a.b = v,
                    3 => b.r = v,
                    4 => b.g = v,
                    _ => b.b = v,
                }
                assert_same(p, a, b, &format!("C11 bg={bi} pos={pos}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — boundary-byte cross product (randomized over BOUNDARY_BYTES)
// ---------------------------------------------------------------------------
#[test]
fn c12_boundary_byte_combinations() {
    let p = pair();
    let mut rng = Rng::new(0xC012_0012);
    for _ in 0..n(60_000) {
        let a = Rgb::new(
            rng.pick(&BOUNDARY_BYTES),
            rng.pick(&BOUNDARY_BYTES),
            rng.pick(&BOUNDARY_BYTES),
        );
        let b = Rgb::new(
            rng.pick(&BOUNDARY_BYTES),
            rng.pick(&BOUNDARY_BYTES),
            rng.pick(&BOUNDARY_BYTES),
        );
        assert_same(p, a, b, "C12 boundary bytes");
    }
    // Deterministic: every (10|11) pattern in all 6 positions -> 2^6 = 64 cases,
    // straddling the `> 0.04045` test in every combination.
    for m in 0u32..64 {
        let pick = |i: u32| if m >> i & 1 == 0 { LAST_LINEAR } else { FIRST_POW };
        let a = Rgb::new(pick(0), pick(1), pick(2));
        let b = Rgb::new(pick(3), pick(4), pick(5));
        assert_same(p, a, b, "C12 10-vs-11 exhaustive");
    }
}

// ---------------------------------------------------------------------------
// C13 — exhaustive greyscale x greyscale (65 536 pairs)
// ---------------------------------------------------------------------------
#[test]
fn c13_exhaustive_grey_x_grey() {
    let p = pair();
    for na in 0u16..=255 {
        for nb in 0u16..=255 {
            let a = Rgb::new(na as u8, na as u8, na as u8);
            let b = Rgb::new(nb as u8, nb as u8, nb as u8);
            assert_same(p, a, b, "C13 grey x grey");
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — exhaustive 2-D sweeps (256 x 256 each)
// ---------------------------------------------------------------------------
#[test]
fn c14_exhaustive_two_dimensional_sweeps() {
    let p = pair();

    // A.R x A.G, with A.B and B fixed.
    for (ab, b) in [(0u8, WHITE), (11u8, MID), (255u8, Rgb::new(0, 0, 1))] {
        for r in 0u16..=255 {
            for g in 0u16..=255 {
                assert_same(
                    p,
                    Rgb::new(r as u8, g as u8, ab),
                    b,
                    "C14 A.R x A.G",
                );
            }
        }
    }

    // A.R x B.R (cross-argument interaction).
    for r in 0u16..=255 {
        for br in 0u16..=255 {
            assert_same(
                p,
                Rgb::new(r as u8, 11, 200),
                Rgb::new(br as u8, 200, 11),
                "C14 A.R x B.R",
            );
        }
    }

    // A.B x B.G (the smallest and the largest luminance weights against
    // each other).
    for x in 0u16..=255 {
        for y in 0u16..=255 {
            assert_same(
                p,
                Rgb::new(0, 0, x as u8),
                Rgb::new(0, y as u8, 0),
                "C14 A.B x B.G",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — large uniform randomized sweep over all 6 bytes
// ---------------------------------------------------------------------------
#[test]
fn c15_uniform_random_sweep() {
    let p = pair();
    let mut rng = Rng::new(0xC015_0015);
    for _ in 0..n(300_000) {
        let a = rng.color();
        let b = rng.color();
        assert_same(p, a, b, "C15 uniform random");
    }
}

// ---------------------------------------------------------------------------
// C16 — zero denominator crossed with the branch masks, in both positions
// ---------------------------------------------------------------------------
#[test]
fn c16_zero_denominator_x_branch_masks() {
    let p = pair();
    for mask in 0u8..8 {
        let mut rng = Rng::new(0xC016_0000 ^ mask as u64);
        for _ in 0..n(3_000) {
            let other = color_for_mask(&mut rng, mask);
            // B black -> no-swap route into `x / +0.0`
            let v1 = assert_same(p, other, BLACK, "C16 B black");
            // A black -> swap route into `x / +0.0`
            let v2 = assert_same(p, BLACK, other, "C16 A black");
            if other != BLACK {
                assert!(v1.is_infinite(), "C16 expected +inf, got {v1:?}");
                assert!(v2.is_infinite(), "C16 expected +inf, got {v2:?}");
            }
        }
    }
    // And exhaustively for every single-channel non-black colour.
    for v in 1u16..=255 {
        let v = v as u8;
        for c in [Rgb::new(v, 0, 0), Rgb::new(0, v, 0), Rgb::new(0, 0, v)] {
            assert_same(p, c, BLACK, "C16 single-channel / black");
            assert_same(p, BLACK, c, "C16 black / single-channel");
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — tiny (unguarded) denominator
// ---------------------------------------------------------------------------
#[test]
fn c17_tiny_denominator() {
    let p = pair();
    let darkest = [
        Rgb::new(1, 0, 0),
        Rgb::new(0, 1, 0),
        Rgb::new(0, 0, 1),
        Rgb::new(1, 1, 1),
        Rgb::new(2, 0, 0),
        Rgb::new(0, 0, 2),
    ];
    let mut rng = Rng::new(0xC017_0017);
    for &low in &darkest {
        for _ in 0..n(3_000) {
            let high = Rgb::new(
                rng.range_u8(128, 255),
                rng.range_u8(128, 255),
                rng.range_u8(128, 255),
            );
            let v = assert_same(p, high, low, "C17 tiny denominator");
            // Sanity check on the test setup itself: the denominator really is
            // tiny, so the (unguarded) ratio must be huge but still finite.
            assert!(
                v.is_finite() && v > 100.0,
                "C17 expected a large finite ratio, got {v:?}"
            );
            assert_same(p, low, high, "C17 tiny denominator swapped");
        }
        // Also against every grey.
        for g in 0u16..=255 {
            let grey = Rgb::new(g as u8, g as u8, g as u8);
            assert_same(p, grey, low, "C17 grey / tiny");
            assert_same(p, low, grey, "C17 tiny / grey");
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — valid inputs invoked with junk in the struct padding bits
// ---------------------------------------------------------------------------
#[test]
fn c18_valid_inputs_with_junk_padding() {
    let p = pair();
    let mut rng = Rng::new(0xC018_0018);
    for _ in 0..n(60_000) {
        let a = rng.color();
        let b = rng.color();
        // Reference result with clean padding, via the struct signature.
        let clean = assert_same(p, a, b, "C18 clean");

        let junk_a = rng.next_u64() & !0x00FF_FFFFu64;
        let junk_b = rng.next_u64() & !0x00FF_FFFFu64;
        let v = assert_same_raw(
            p,
            a.as_reg_bits() | junk_a,
            b.as_reg_bits() | junk_b,
            "C18 junk padding",
        );
        assert_eq!(
            v.to_bits(),
            clean.to_bits(),
            "C18: padding bits changed the result (A={a:?} B={b:?} \
             junk_a=0x{junk_a:016X} junk_b=0x{junk_b:016X})"
        );
    }
}
