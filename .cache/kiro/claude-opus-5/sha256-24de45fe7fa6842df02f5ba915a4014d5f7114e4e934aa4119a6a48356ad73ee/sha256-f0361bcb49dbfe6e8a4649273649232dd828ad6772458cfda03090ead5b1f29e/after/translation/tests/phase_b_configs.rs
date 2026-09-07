//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `contrast_ratio` symbol and asserts the returned `float`s are equal
//! bit-for-bit (`to_bits()`), so NaN payloads and signed zeros are covered too.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Helpers shared by several rows.
// ---------------------------------------------------------------------------

/// A monotone proxy for a color's luminance, computed with the C library (the
/// ground truth). `contrast_ratio(X, WHITE)` is `LumWhite / LumX` for every
/// non-white `X`, so it is strictly decreasing in `LumX`. Used only to
/// *classify* inputs into the branch a row wants to exercise.
fn lum_proxy(p: &Pair, c: Rgb) -> f32 {
    p.c.call(c, WHITE)
}

/// `Ordering` of `LumA` vs `LumB` according to the C implementation.
fn lum_cmp(p: &Pair, a: Rgb, b: Rgb) -> std::cmp::Ordering {
    // proxy is decreasing in luminance, so reverse the comparison.
    let (pa, pb) = (lum_proxy(p, a), lum_proxy(p, b));
    pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
}

// ---------------------------------------------------------------------------
// Row 1 / Row 2 — exhaustive grayscale sweeps, both orientations.
// ---------------------------------------------------------------------------

#[test]
fn row01_exhaustive_grayscale_vs_white() {
    let p = load();
    for v in 0u16..=255 {
        let v = v as u8;
        p.assert_same(Rgb::new(v, v, v), WHITE, "row01 gray-vs-white");
    }
}

#[test]
fn row02_exhaustive_white_vs_grayscale() {
    let p = load();
    for v in 0u16..=255 {
        let v = v as u8;
        p.assert_same(WHITE, Rgb::new(v, v, v), "row02 white-vs-gray");
    }
}

// ---------------------------------------------------------------------------
// Rows 3-5 — exhaustive per-channel isolation (one weight at a time).
// ---------------------------------------------------------------------------

#[test]
fn row03_exhaustive_r_channel_only() {
    let p = load();
    for v in 0u16..=255 {
        let v = v as u8;
        p.assert_same(Rgb::new(v, 0, 0), WHITE, "row03 R-only");
        p.assert_same(Rgb::new(v, 0, 0), MIDGRAY, "row03 R-only vs gray");
    }
}

#[test]
fn row04_exhaustive_g_channel_only() {
    let p = load();
    for v in 0u16..=255 {
        let v = v as u8;
        p.assert_same(Rgb::new(0, v, 0), WHITE, "row04 G-only");
        p.assert_same(Rgb::new(0, v, 0), MIDGRAY, "row04 G-only vs gray");
    }
}

#[test]
fn row05_exhaustive_b_channel_only() {
    let p = load();
    for v in 0u16..=255 {
        let v = v as u8;
        p.assert_same(Rgb::new(0, 0, v), WHITE, "row05 B-only");
        p.assert_same(Rgb::new(0, 0, v), MIDGRAY, "row05 B-only vs gray");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — exhaustive two-channel plane.
// ---------------------------------------------------------------------------

#[test]
fn row06_exhaustive_rg_plane_vs_white() {
    let p = load();
    for i in 0u16..=255 {
        for j in 0u16..=255 {
            p.assert_same(Rgb::new(i as u8, j as u8, 0), WHITE, "row06 RG-plane");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7-8 — exhaustive over the ENTIRE 2^24 color domain, both operand
// positions. Because the result depends on each operand only through its
// luminance, these two rows exhaust every reachable luminance value.
// ---------------------------------------------------------------------------

#[test]
fn row07_exhaustive_all_colors_as_a() {
    let p = load();
    let stride = stride();
    let mut n = 0usize;
    let mut i = 0usize;
    while i < (1 << 24) {
        let a = Rgb::new((i & 0xFF) as u8, ((i >> 8) & 0xFF) as u8, ((i >> 16) & 0xFF) as u8);
        p.assert_same(a, MIDGRAY, "row07 all-colors-as-A");
        n += 1;
        i += stride;
    }
    assert!(n > 0, "row07 executed no cases");
    eprintln!("row07: {n} exhaustive cases (stride {stride})");
}

#[test]
fn row08_exhaustive_all_colors_as_b() {
    let p = load();
    let stride = stride();
    let mut n = 0usize;
    let mut i = 0usize;
    while i < (1 << 24) {
        let b = Rgb::new((i & 0xFF) as u8, ((i >> 8) & 0xFF) as u8, ((i >> 16) & 0xFF) as u8);
        p.assert_same(MIDGRAY, b, "row08 all-colors-as-B");
        n += 1;
        i += stride;
    }
    assert!(n > 0, "row08 executed no cases");
    eprintln!("row08: {n} exhaustive cases (stride {stride})");
}

// ---------------------------------------------------------------------------
// Row 9 — all 64 transfer-branch pattern pairs, randomized within each branch.
// ---------------------------------------------------------------------------

#[test]
fn row09_all_64_transfer_branch_pattern_pairs() {
    let p = load();
    let per_pair = samples(2000);
    for pa in 0u8..8 {
        for pb in 0u8..8 {
            let mut rng = Rng::for_row(900 + (pa as u64) * 8 + pb as u64);
            for _ in 0..per_pair {
                let a = rgb_on_pattern(&mut rng, pa);
                let b = rgb_on_pattern(&mut rng, pb);
                p.assert_same(a, b, &format!("row09 pattern A={pa:03b} B={pb:03b}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-11 — the swap branch in `cbContrastRatio`, both directions.
// ---------------------------------------------------------------------------

#[test]
fn row10_swap_taken_lum_a_less_than_lum_b() {
    let p = load();
    let want = samples(20_000);
    let mut rng = Rng::for_row(10);
    let mut hits = 0usize;
    let mut tries = 0usize;
    while hits < want && tries < want * 20 {
        tries += 1;
        let (a, b) = (rng.rgb(), rng.rgb());
        if a == BLACK || b == BLACK {
            continue;
        }
        if lum_cmp(&p, a, b) == std::cmp::Ordering::Less {
            p.assert_same(a, b, "row10 swap-taken");
            hits += 1;
        }
    }
    assert!(hits >= want / 2, "row10 only exercised {hits} swap cases");
}

#[test]
fn row11_swap_not_taken_lum_a_greater_than_lum_b() {
    let p = load();
    let want = samples(20_000);
    let mut rng = Rng::for_row(11);
    let mut hits = 0usize;
    let mut tries = 0usize;
    while hits < want && tries < want * 20 {
        tries += 1;
        let (a, b) = (rng.rgb(), rng.rgb());
        if a == BLACK || b == BLACK {
            continue;
        }
        if lum_cmp(&p, a, b) == std::cmp::Ordering::Greater {
            p.assert_same(a, b, "row11 no-swap");
            hits += 1;
        }
    }
    assert!(hits >= want / 2, "row11 only exercised {hits} no-swap cases");
}

// ---------------------------------------------------------------------------
// Row 12 — distinct colors with *equal* float luminance (the `High == Low`
// edge of the `<` comparison reached without `A == B`).
// ---------------------------------------------------------------------------

/// A local re-derivation of the C `cbLuminance`, used ONLY to *select* candidate
/// inputs for row 12 (it is not part of the translation under test). It is
/// validated against the C library by `row12_luminance_model_matches_c` below
/// before being trusted, so a wrong model cannot silently weaken the row.
fn luminance_model(c: Rgb) -> f32 {
    let lin = |byte: u8| -> f32 {
        let ch = (byte as f32) / 255.0f32;
        let d = ch as f64;
        let v = if d > 0.04045 {
            ((d + 0.055) / 1.055).powf(2.4)
        } else {
            d / 12.92
        };
        v as f32
    };
    0.2126f32 * lin(c.r) + 0.7152f32 * lin(c.g) + 0.0722f32 * lin(c.b)
}

/// Validates `luminance_model` against the C `.so`: for every pair, the C's
/// return value must be exactly `max(LumA,LumB) / min(LumA,LumB)` as computed by
/// the model. This is itself a differential check on the composed pipeline.
#[test]
fn row12_luminance_model_matches_c() {
    let p = load();
    let mut rng = Rng::for_row(1201);
    let mut check = |a: Rgb, b: Rgb| {
        let (la, lb) = (luminance_model(a), luminance_model(b));
        // Mirror the C's `if (High < Low)` exactly, including its NaN behaviour.
        let (mut high, mut low) = (la, lb);
        if high < low {
            high = lb;
            low = la;
        }
        // `black_box` keeps LLVM from constant-folding the division for constant
        // inputs. A folded `0.0 / 0.0` yields the positive quiet NaN
        // 0x7FC00000, whereas the hardware `divss` both libraries execute yields
        // 0xFFC00000; without this the model would differ from the C purely
        // because of when the division happened, not how.
        let expect = std::hint::black_box(high) / std::hint::black_box(low);
        let got = p.c.call(a, b);
        assert_eq!(
            expect.to_bits(),
            got.to_bits(),
            "luminance model disagrees with C for A=({},{},{}) B=({},{},{}): \
             model={expect:?} C={got:?}",
            a.r,
            a.g,
            a.b,
            b.r,
            b.g,
            b.b
        );
    };
    for _ in 0..samples(200_000) {
        check(rng.rgb(), rng.rgb());
    }
    for &a in &CORNERS {
        for &b in &CORNERS {
            check(a, b);
        }
    }
}

/// Search for pairs of distinct colors whose f32 luminance is bit-identical.
/// Equal luminance means `High == Low`, so `if (High < Low)` is false and the C
/// divides a value by itself.
fn find_equal_luminance_pairs(want: usize) -> Vec<(Rgb, Rgb)> {
    let mut out = Vec::new();
    for &hi in &[32u16, 128, 256] {
        let mut seen: std::collections::HashMap<u32, Rgb> = std::collections::HashMap::new();
        for r in 0..hi {
            for g in 0..hi {
                for b in 0..hi {
                    let c = Rgb::new(r as u8, g as u8, b as u8);
                    if c == BLACK {
                        continue;
                    }
                    let key = luminance_model(c).to_bits();
                    match seen.get(&key) {
                        Some(&prev) if prev != c => {
                            out.push((prev, c));
                            if out.len() >= want {
                                return out;
                            }
                        }
                        Some(_) => {}
                        None => {
                            seen.insert(key, c);
                        }
                    }
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    out
}

#[test]
fn row12_equal_luminance_distinct_colors() {
    let p = load();
    let pairs = find_equal_luminance_pairs(samples(500));
    assert!(
        !pairs.is_empty(),
        "row12 found no distinct colors with equal f32 luminance"
    );
    for (a, b) in &pairs {
        assert_ne!(a, b, "row12 pair must be distinct colors");
        assert_eq!(
            luminance_model(*a).to_bits(),
            luminance_model(*b).to_bits(),
            "row12 candidate pair does not actually have equal luminance"
        );
        p.assert_same(*a, *b, "row12 equal-lum distinct colors");
        p.assert_same(*b, *a, "row12 equal-lum distinct colors (swapped)");
        // Equal luminance => the no-swap path divides a value by itself.
        let v = p.c.call(*a, *b);
        assert_eq!(
            v.to_bits(),
            1.0f32.to_bits(),
            "row12 expected exactly 1.0 for equal-luminance pair \
             ({},{},{}) / ({},{},{}), got {v:?}",
            a.r,
            a.g,
            a.b,
            b.r,
            b.g,
            b.b
        );
    }
    eprintln!("row12: {} equal-luminance distinct pairs", pairs.len());
}

// ---------------------------------------------------------------------------
// Row 13 — identical non-black colors.
// ---------------------------------------------------------------------------

#[test]
fn row13_identical_non_black_colors() {
    let p = load();
    let mut rng = Rng::for_row(13);
    for _ in 0..samples(20_000) {
        let mut a = rng.rgb();
        if a == BLACK {
            a = Rgb::new(1, 1, 1);
        }
        p.assert_same(a, a, "row13 A==B non-black");
        assert_eq!(
            p.c.call(a, a).to_bits(),
            1.0f32.to_bits(),
            "row13 expected exactly 1.0 for A==B"
        );
    }
    // Plus every corner except black, and the whole grayscale diagonal.
    for &c in CORNERS.iter().filter(|&&c| c != BLACK) {
        p.assert_same(c, c, "row13 corner A==B");
    }
    for v in 1u16..=255 {
        let c = Rgb::new(v as u8, v as u8, v as u8);
        p.assert_same(c, c, "row13 gray A==B");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-16 — the unguarded division: zero denominator and 0/0.
// (Also rows 1-3 of ERRORS.md; asserted here as valid-path shapes too.)
// ---------------------------------------------------------------------------

#[test]
fn row14_black_as_b_yields_infinity() {
    let p = load();
    let mut rng = Rng::for_row(14);
    for _ in 0..samples(20_000) {
        let mut a = rng.rgb();
        if a == BLACK {
            a = Rgb::new(0, 0, 1);
        }
        p.assert_same(a, BLACK, "row14 A vs black");
        assert_eq!(
            p.c.call(a, BLACK).to_bits(),
            f32::INFINITY.to_bits(),
            "row14 expected +inf"
        );
    }
}

#[test]
fn row15_black_as_a_yields_infinity() {
    let p = load();
    let mut rng = Rng::for_row(15);
    for _ in 0..samples(20_000) {
        let mut b = rng.rgb();
        if b == BLACK {
            b = Rgb::new(1, 0, 0);
        }
        p.assert_same(BLACK, b, "row15 black vs B");
        assert_eq!(
            p.c.call(BLACK, b).to_bits(),
            f32::INFINITY.to_bits(),
            "row15 expected +inf"
        );
    }
}

#[test]
fn row16_black_vs_black_is_nan() {
    let p = load();
    p.assert_same(BLACK, BLACK, "row16 black vs black");
    let cv = p.c.call(BLACK, BLACK);
    let rv = p.rust.call(BLACK, BLACK);
    assert!(cv.is_nan(), "row16 C should produce NaN, got {cv:?}");
    assert!(rv.is_nan(), "row16 Rust should produce NaN, got {rv:?}");
    assert_eq!(cv.to_bits(), rv.to_bits(), "row16 NaN bit patterns differ");
}

// ---------------------------------------------------------------------------
// Row 17 — the full boundary-byte corner grid (6^3 x 6^3 = 46 656 pairs).
// ---------------------------------------------------------------------------

#[test]
fn row17_boundary_byte_corner_grid() {
    let p = load();
    let mut colors = Vec::new();
    for &r in &BOUNDARY_BYTES {
        for &g in &BOUNDARY_BYTES {
            for &b in &BOUNDARY_BYTES {
                colors.push(Rgb::new(r, g, b));
            }
        }
    }
    assert_eq!(colors.len(), 216);
    for &a in &colors {
        for &b in &colors {
            p.assert_same(a, b, "row17 boundary grid");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — threshold straddle, per channel position.
// ---------------------------------------------------------------------------

#[test]
fn row18_threshold_straddle_per_channel() {
    let p = load();
    for pos in 0..3 {
        let mk = |v: u8| match pos {
            0 => Rgb::new(v, 128, 128),
            1 => Rgb::new(128, v, 128),
            _ => Rgb::new(128, 128, v),
        };
        let a = mk(LINEAR_MAX); // 10 -> linear branch
        let b = mk(POW_MIN); // 11 -> pow branch
        p.assert_same(a, b, "row18 straddle");
        p.assert_same(b, a, "row18 straddle swapped");
        // and each against a neutral partner
        p.assert_same(a, WHITE, "row18 linear-side vs white");
        p.assert_same(b, WHITE, "row18 pow-side vs white");
        // one step further out on each side
        p.assert_same(mk(LINEAR_MAX - 1), mk(POW_MIN + 1), "row18 straddle +-1");
    }
}

// ---------------------------------------------------------------------------
// Row 19 — unconstrained randomized pairs (the default consumer usage).
// ---------------------------------------------------------------------------

#[test]
fn row19_randomized_unconstrained_pairs() {
    let p = load();
    let mut rng = Rng::for_row(19);
    for _ in 0..samples(300_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        p.assert_same(a, b, "row19 random pair");
    }
}

// ---------------------------------------------------------------------------
// Rows 20-21 — dark and bright subdomains.
// ---------------------------------------------------------------------------

#[test]
fn row20_dark_subdomain_all_linear_branch() {
    let p = load();
    let mut rng = Rng::for_row(20);
    for _ in 0..samples(100_000) {
        let a = rng.rgb_in(0, LINEAR_MAX);
        let b = rng.rgb_in(0, LINEAR_MAX);
        p.assert_same(a, b, "row20 dark subdomain");
    }
    // Exhaustive over the dark cube (11^3 = 1331 colors) against a few partners.
    for r in 0..=LINEAR_MAX {
        for g in 0..=LINEAR_MAX {
            for b in 0..=LINEAR_MAX {
                let c = Rgb::new(r, g, b);
                p.assert_same(c, Rgb::new(1, 1, 1), "row20 dark cube");
                p.assert_same(c, WHITE, "row20 dark cube vs white");
            }
        }
    }
}

#[test]
fn row21_bright_subdomain_all_pow_branch() {
    let p = load();
    let mut rng = Rng::for_row(21);
    for _ in 0..samples(100_000) {
        let a = rng.rgb_in(245, 255);
        let b = rng.rgb_in(245, 255);
        p.assert_same(a, b, "row21 bright subdomain");
    }
    for r in 245..=255u8 {
        for g in 245..=255u8 {
            for b in 245..=255u8 {
                p.assert_same(Rgb::new(r, g, b), WHITE, "row21 bright cube");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — extreme-contrast corners, all 64 ordered pairs.
// ---------------------------------------------------------------------------

#[test]
fn row22_corner_cross_product() {
    let p = load();
    for &a in &CORNERS {
        for &b in &CORNERS {
            p.assert_same(a, b, "row22 corners");
        }
    }
    // Sanity on the shape of the results (C is the oracle; this only documents
    // that the row really does reach inf / NaN / 1.0).
    assert!(p.c.call(BLACK, BLACK).is_nan());
    assert!(p.c.call(WHITE, BLACK).is_infinite());
    assert!(p.c.call(BLACK, WHITE).is_infinite());
    assert_eq!(p.c.call(WHITE, WHITE).to_bits(), 1.0f32.to_bits());
}

// ---------------------------------------------------------------------------
// Row 23 — argument-order symmetry must match between the two libraries.
// ---------------------------------------------------------------------------

#[test]
fn row23_argument_order_symmetry() {
    let p = load();
    let mut rng = Rng::for_row(23);
    for _ in 0..samples(100_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        p.assert_same(a, b, "row23 forward");
        p.assert_same(b, a, "row23 reversed");
        // The C's `if (High < Low)` makes the function symmetric; the Rust must
        // be symmetric in exactly the same way.
        let (cf, cr) = (p.c.call(a, b), p.c.call(b, a));
        let (rf, rr) = (p.rust.call(a, b), p.rust.call(b, a));
        assert_eq!(
            cf.to_bits() == cr.to_bits(),
            rf.to_bits() == rr.to_bits(),
            "row23 symmetry disagreement for A=({},{},{}) B=({},{},{}): \
             C {cf:?}/{cr:?} vs Rust {rf:?}/{rr:?}",
            a.r,
            a.g,
            a.b,
            b.r,
            b.g,
            b.b
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26 — ABI: garbage in the unused high bytes of the argument
// registers (the 3-byte struct occupies only the low 24 bits).
// ---------------------------------------------------------------------------

#[test]
fn row24_dirty_padding_first_argument() {
    let p = load();
    let mut rng = Rng::for_row(24);
    for _ in 0..samples(50_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        p.assert_same_reg(a, b, u64::MAX, 0, "row24 dirty arg1");
        p.assert_same_reg(a, b, rng.next_u64(), 0, "row24 dirty arg1 random");
    }
}

#[test]
fn row25_dirty_padding_second_argument() {
    let p = load();
    let mut rng = Rng::for_row(25);
    for _ in 0..samples(50_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        p.assert_same_reg(a, b, 0, u64::MAX, "row25 dirty arg2");
        p.assert_same_reg(a, b, 0, rng.next_u64(), "row25 dirty arg2 random");
    }
}

#[test]
fn row26_dirty_padding_both_arguments() {
    let p = load();
    let mut rng = Rng::for_row(26);
    for _ in 0..samples(50_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        let (ga, gb) = (rng.next_u64(), rng.next_u64());
        p.assert_same_reg(a, b, ga, gb, "row26 dirty both");
    }
    // Degenerate inputs with dirty padding, including the NaN and inf cases.
    for &(a, b) in &[
        (BLACK, BLACK),
        (BLACK, WHITE),
        (WHITE, BLACK),
        (WHITE, WHITE),
    ] {
        p.assert_same_reg(a, b, u64::MAX, u64::MAX, "row26 degenerate dirty");
        p.assert_same_reg(a, b, 0xDEAD_BEEF_0000_0000, 0xFEED_FACE_0000_0000, "row26 degenerate dirty 2");
    }
}

// ---------------------------------------------------------------------------
// Row 27 — statelessness: the C has no globals or `static` storage; repeated
// and interleaved calls must be bit-identical on both sides.
// ---------------------------------------------------------------------------

#[test]
fn row27_stateless_repeated_calls() {
    let p = load();
    let mut rng = Rng::for_row(27);
    let probes: Vec<(Rgb, Rgb)> = (0..64).map(|_| (rng.rgb(), rng.rgb())).collect();
    let baseline_c: Vec<u32> = probes.iter().map(|&(a, b)| p.c.call(a, b).to_bits()).collect();
    let baseline_r: Vec<u32> = probes
        .iter()
        .map(|&(a, b)| p.rust.call(a, b).to_bits())
        .collect();
    assert_eq!(baseline_c, baseline_r, "row27 baseline already diverges");

    for _round in 0..200 {
        // Interleave unrelated noise calls between the probes.
        for _ in 0..16 {
            let (na, nb) = (rng.rgb(), rng.rgb());
            let _ = p.c.call(na, nb);
            let _ = p.rust.call(na, nb);
        }
        for (i, &(a, b)) in probes.iter().enumerate() {
            assert_eq!(
                p.c.call(a, b).to_bits(),
                baseline_c[i],
                "row27 C is not stateless at probe {i}"
            );
            assert_eq!(
                p.rust.call(a, b).to_bits(),
                baseline_r[i],
                "row27 Rust is not stateless at probe {i}"
            );
        }
    }
}
