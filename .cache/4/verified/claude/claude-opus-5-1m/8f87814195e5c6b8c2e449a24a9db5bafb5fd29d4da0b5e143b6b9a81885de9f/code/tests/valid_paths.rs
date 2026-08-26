//! Phase B — valid-path differential tests.
//!
//! One test function per row of `CONFIGS.md`. Every row drives BOTH the C `.so`
//! and the Rust `.so` through `libloading` with many randomized inputs
//! (deterministic seed) and asserts the resulting memory is byte-identical.

mod common;

use common::*;

/// Iterations per randomized row. Override with `HARVEST_ITERS`.
fn iters(default: usize) -> usize {
    std::env::var("HARVEST_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

// ===========================================================================
// Rows 1-3 : axis A, the `if (s == 0)` early return
// ===========================================================================

#[test]
fn cfg_01_s_plus_zero_early_return() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let src = [rng.any_bits(), POS_ZERO, rng.any_bits()];
        assert_same(&c, &r, src, Alias::Separate, &format!("row1 #{n}"));
    }
}

#[test]
fn cfg_02_s_minus_zero_early_return() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let src = [rng.any_bits(), NEG_ZERO, rng.any_bits()];
        assert_same(&c, &r, src, Alias::Separate, &format!("row2 #{n}"));
    }
}

#[test]
fn cfg_03_s_zero_v_specials() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for s in [POS_ZERO, NEG_ZERO] {
        for v in SPECIALS {
            for h in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row3 specials");
            }
        }
    }
    for n in 0..iters(5_000) {
        let src = [rng.pick(&SPECIALS), rng.pick(&[POS_ZERO, NEG_ZERO]), rng.pick(&SPECIALS)];
        assert_same(&c, &r, src, Alias::Separate, &format!("row3 rnd #{n}"));
    }
}

// ===========================================================================
// Rows 4-11 : axis B, every `switch` arm
// ===========================================================================

/// Random `s` in (0, 1] (never exactly zero, so the main path is taken).
fn nonzero_unit_s(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.unit();
        if v != 0.0 {
            return v.to_bits();
        }
    }
}

fn arm_row(name: &str, lo: f32, hi: f32) {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let h = rng.range(lo, hi).to_bits();
        let s = nonzero_unit_s(&mut rng);
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("{name} #{n}"));
    }
}

#[test]
fn cfg_04_arm_i0_hue_0_60() {
    arm_row("row4 i=0", 0.0, 60.0);
}

#[test]
fn cfg_05_arm_i1_hue_60_120() {
    arm_row("row5 i=1", 60.0, 120.0);
}

#[test]
fn cfg_06_arm_i2_hue_120_180() {
    arm_row("row6 i=2", 120.0, 180.0);
}

#[test]
fn cfg_07_arm_i3_hue_180_240() {
    arm_row("row7 i=3", 180.0, 240.0);
}

#[test]
fn cfg_08_arm_i4_hue_240_300() {
    arm_row("row8 i=4", 240.0, 300.0);
}

#[test]
fn cfg_09_arm_default_hue_300_360() {
    arm_row("row9 i=5", 300.0, 360.0);
}

#[test]
fn cfg_10_arm_default_hue_above_360() {
    arm_row("row10 i>=6", 360.0, 100_000.0);
}

#[test]
fn cfg_11_arm_default_hue_negative() {
    arm_row("row11 i<0", -100_000.0, -0.000_001);
}

// ===========================================================================
// Rows 12-13 : sector seams
// ===========================================================================

#[test]
fn cfg_12_sector_boundaries_and_neighbours() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    let boundaries: [f32; 12] = [
        -0.0, 0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0, -60.0, -120.0, 30.0,
    ];
    for b in boundaries {
        let mut hs = vec![b.to_bits()];
        // a few ULPs either side of each seam
        let mut up = b;
        let mut dn = b;
        for _ in 0..4 {
            up = next_up(up);
            dn = next_down(dn);
            hs.push(up.to_bits());
            hs.push(dn.to_bits());
        }
        for h in hs {
            for _ in 0..iters(400) {
                let s = nonzero_unit_s(&mut rng);
                let v = rng.unit().to_bits();
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row12 seam");
            }
            // also with extreme s/v around the seam
            for s in SPECIALS {
                for v in [POS_ZERO, 0x3F80_0000, POS_INF, NEG_INF, NANS[0]] {
                    assert_same(&c, &r, [h, s, v], Alias::Separate, "row12 seam specials");
                }
            }
        }
    }
}

#[test]
fn cfg_13_exact_multiples_of_60() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for k in -40i32..=40 {
        let h = (60.0f32 * k as f32).to_bits();
        for _ in 0..iters(500) {
            let s = nonzero_unit_s(&mut rng);
            let v = rng.unit().to_bits();
            assert_same(&c, &r, [h, s, v], Alias::Separate, "row13 60*k");
        }
    }
}

// ===========================================================================
// Rows 14-16 : `(int)` cast out of range / inf / NaN hue
// ===========================================================================

#[test]
fn cfg_14_hue_out_of_int_range() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    let two31 = 2147483648.0f32;
    let mut hs: Vec<u32> = vec![
        1.3e11f32.to_bits(),
        (-1.3e11f32).to_bits(),
        f32::MAX.to_bits(),
        f32::MIN.to_bits(),
        (60.0f32 * two31).to_bits(),
        (-60.0f32 * two31).to_bits(),
        1e30f32.to_bits(),
        (-1e30f32).to_bits(),
    ];
    // hues that make h/60 land exactly on / just around +-2^31
    for base in [two31, -two31] {
        let mut x = base * 60.0;
        for _ in 0..3 {
            hs.push(x.to_bits());
            hs.push(next_up(x).to_bits());
            hs.push(next_down(x).to_bits());
            x = next_up(x);
        }
    }
    for h in hs {
        for _ in 0..iters(600) {
            let s = nonzero_unit_s(&mut rng);
            let v = rng.unit().to_bits();
            assert_same(&c, &r, [h, s, v], Alias::Separate, "row14 int-range");
        }
        for s in SPECIALS {
            for v in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row14 specials");
            }
        }
    }
}

#[test]
fn cfg_15_hue_infinite() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for h in [POS_INF, NEG_INF] {
        for _ in 0..iters(3_000) {
            let s = nonzero_unit_s(&mut rng);
            let v = rng.unit().to_bits();
            assert_same(&c, &r, [h, s, v], Alias::Separate, "row15 inf hue");
        }
        for s in SPECIALS {
            for v in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row15 specials");
            }
        }
    }
}

#[test]
fn cfg_16_hue_nan_all_payloads() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for h in NANS {
        for _ in 0..iters(3_000) {
            let s = nonzero_unit_s(&mut rng);
            let v = rng.unit().to_bits();
            assert_same(&c, &r, [h, s, v], Alias::Separate, "row16 NaN hue");
        }
        for s in SPECIALS {
            for v in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row16 specials");
            }
        }
        // NaN hue x NaN saturation: exercises NaN-payload precedence in the
        // `s * (1 - f)` / `1 - s * f` products.
        for s in NANS {
            for v in NANS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row16 NaN x NaN");
            }
        }
    }
}

// ===========================================================================
// Rows 17-22 : out-of-unit-range and non-finite s / v
// ===========================================================================

#[test]
fn cfg_17_s_out_of_unit_range() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let h = rng.range(0.0, 360.0).to_bits();
        let s = if n % 2 == 0 {
            rng.range(1.0, 16.0)
        } else {
            rng.range(-16.0, -1e-6)
        }
        .to_bits();
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row17 #{n}"));
    }
}

#[test]
fn cfg_18_v_out_of_unit_range() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let h = rng.range(-720.0, 1080.0).to_bits();
        let s = nonzero_unit_s(&mut rng);
        let v = rng.range(-1e6, 1e6).to_bits();
        assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row18 #{n}"));
    }
}

#[test]
fn cfg_19_s_infinite() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for s in [POS_INF, NEG_INF] {
        for n in 0..iters(4_000) {
            let h = rng.range(-720.0, 1080.0).to_bits();
            let v = rng.range(-100.0, 100.0).to_bits();
            assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row19 #{n}"));
        }
        for h in SPECIALS {
            for v in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row19 specials");
            }
        }
    }
}

#[test]
fn cfg_20_v_infinite() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for v in [POS_INF, NEG_INF] {
        for n in 0..iters(4_000) {
            let h = rng.range(-720.0, 1080.0).to_bits();
            let s = nonzero_unit_s(&mut rng);
            assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row20 #{n}"));
        }
        for h in SPECIALS {
            for s in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row20 specials");
            }
        }
    }
}

#[test]
fn cfg_21_s_nan_all_payloads() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for s in NANS {
        for n in 0..iters(4_000) {
            let h = rng.range(-720.0, 1080.0).to_bits();
            let v = rng.range(-100.0, 100.0).to_bits();
            assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row21 #{n}"));
        }
        for h in SPECIALS {
            for v in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row21 specials");
            }
        }
    }
}

#[test]
fn cfg_22_v_nan_all_payloads() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for v in NANS {
        for n in 0..iters(4_000) {
            let h = rng.range(-720.0, 1080.0).to_bits();
            let s = nonzero_unit_s(&mut rng);
            assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row22 #{n}"));
        }
        for h in SPECIALS {
            for s in SPECIALS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row22 specials");
            }
        }
    }
}

// ===========================================================================
// Row 23 : subnormals
// ===========================================================================

#[test]
fn cfg_23_subnormal_inputs() {
    let (c, r) = load_pair();
    for h in TINY {
        for s in TINY {
            for v in TINY {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row23 tiny^3");
            }
        }
    }
    // tiny s/v combined with ordinary hues (all arms)
    let mut rng = Rng::seeded();
    for n in 0..iters(6_000) {
        let h = rng.range(-400.0, 400.0).to_bits();
        let s = rng.pick(&TINY);
        let v = rng.pick(&TINY);
        assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row23 #{n}"));
    }
    // tiny hue with ordinary s/v
    for n in 0..iters(6_000) {
        let h = rng.pick(&TINY);
        let s = nonzero_unit_s(&mut rng);
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row23b #{n}"));
    }
}

// ===========================================================================
// Row 24 : unconstrained bit-pattern fuzz
// ===========================================================================

#[test]
fn cfg_24_unconstrained_bit_fuzz() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(200_000) {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        assert_same(&c, &r, src, Alias::Separate, &format!("row24 #{n}"));
    }
}

// ===========================================================================
// Rows 25-29 : aliasing
// ===========================================================================

#[test]
fn cfg_25_alias_same_main_path() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let h = rng.range(-400.0, 400.0).to_bits();
        let s = nonzero_unit_s(&mut rng);
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::Same, &format!("row25 #{n}"));
    }
}

#[test]
fn cfg_26_alias_same_early_return() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(20_000) {
        let src = [rng.any_bits(), rng.pick(&[POS_ZERO, NEG_ZERO]), rng.any_bits()];
        assert_same(&c, &r, src, Alias::Same, &format!("row26 #{n}"));
    }
}

#[test]
fn cfg_27_alias_dest_plus_one() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(40_000) {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        assert_same(&c, &r, src, Alias::DestPlus1, &format!("row27 #{n}"));
    }
    // plus a well-formed sweep over all arms
    for n in 0..iters(10_000) {
        let h = rng.range(-400.0, 400.0).to_bits();
        let s = nonzero_unit_s(&mut rng);
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::DestPlus1, &format!("row27b #{n}"));
    }
}

#[test]
fn cfg_28_alias_dest_minus_one() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(40_000) {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        assert_same(&c, &r, src, Alias::DestMinus1, &format!("row28 #{n}"));
    }
    for n in 0..iters(10_000) {
        let h = rng.range(-400.0, 400.0).to_bits();
        let s = nonzero_unit_s(&mut rng);
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::DestMinus1, &format!("row28b #{n}"));
    }
}

#[test]
fn cfg_29_alias_same_bit_fuzz() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(40_000) {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        assert_same(&c, &r, src, Alias::Same, &format!("row29 #{n}"));
    }
}

// ===========================================================================
// Row 30 : write extent (canaries) — dedicated row
// ===========================================================================

#[test]
fn cfg_30_write_extent_canaries() {
    // `assert_same` verifies the canaries on every single call; this row makes
    // the check explicit for both paths and all aliasing modes, and also checks
    // each library independently (not just C-vs-Rust equality).
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    let base = canaries();
    for alias in Alias::ALL {
        for n in 0..iters(4_000) {
            // alternate early-return and main path
            let s = if n % 3 == 0 {
                rng.pick(&[POS_ZERO, NEG_ZERO])
            } else {
                nonzero_unit_s(&mut rng)
            };
            let src = [rng.any_bits(), s, rng.any_bits()];
            assert_same(&c, &r, src, alias, &format!("row30 {alias:?} #{n}"));

            for imp in [&c, &r] {
                let out = call(imp, src, alias);
                let allowed: Vec<usize> = match alias {
                    Alias::Separate => vec![],
                    Alias::Same => vec![WINDOW, WINDOW + 1, WINDOW + 2],
                    Alias::DestPlus1 => vec![WINDOW + 1, WINDOW + 2, WINDOW + 3],
                    Alias::DestMinus1 => vec![WINDOW - 1, WINDOW, WINDOW + 1],
                };
                for i in 0..BUF_WORDS {
                    if (WINDOW..WINDOW + 3).contains(&i) || allowed.contains(&i) {
                        continue;
                    }
                    assert_eq!(
                        out.sbuf.0[i], base.0[i],
                        "{} wrote outside the 3-float window (src buf word {i}, {alias:?})",
                        imp.name
                    );
                }
                if alias == Alias::Separate {
                    for i in 0..BUF_WORDS {
                        if (WINDOW..WINDOW + 3).contains(&i) {
                            continue;
                        }
                        assert_eq!(
                            out.dbuf.0[i], base.0[i],
                            "{} wrote outside dest[0..3] (dest buf word {i})",
                            imp.name
                        );
                    }
                } else {
                    assert_eq!(out.dbuf, base, "{} touched the unused buffer", imp.name);
                }
            }
        }
    }
}

// ===========================================================================
// Row 31 : full pruned A x B x D cross product
// ===========================================================================

/// A hue representative for every distinguishable `i`.
fn hue_for_arm(kind: usize, rng: &mut Rng) -> u32 {
    match kind {
        0 => rng.range(0.0, 60.0).to_bits(),      // i = 0
        1 => rng.range(60.0, 120.0).to_bits(),    // i = 1
        2 => rng.range(120.0, 180.0).to_bits(),   // i = 2
        3 => rng.range(180.0, 240.0).to_bits(),   // i = 3
        4 => rng.range(240.0, 300.0).to_bits(),   // i = 4
        5 => rng.range(300.0, 1e5).to_bits(),     // i >= 5
        6 => rng.range(-1e5, -1e-3).to_bits(),    // i < 0
        _ => rng.pick(&[POS_INF, NEG_INF, NANS[0], NANS[3], 1.3e11f32.to_bits()]), // INT_MIN
    }
}

#[test]
fn cfg_31_cross_product_a_b_d() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for alias in Alias::ALL {
        for arm in 0..8usize {
            for skind in 0..5usize {
                for n in 0..iters(300) {
                    let h = hue_for_arm(arm, &mut rng);
                    let s = match skind {
                        0 => POS_ZERO,
                        1 => NEG_ZERO,
                        2 => nonzero_unit_s(&mut rng),
                        3 => rng.pick(&NANS),
                        _ => rng.pick(&[POS_INF, NEG_INF]),
                    };
                    let v = if n % 4 == 0 {
                        rng.pick(&SPECIALS)
                    } else {
                        rng.range(-10.0, 10.0).to_bits()
                    };
                    assert_same(
                        &c,
                        &r,
                        [h, s, v],
                        alias,
                        &format!("row31 alias={alias:?} arm={arm} s={skind} #{n}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 32 : repeat-call stability (no hidden global state)
// ===========================================================================

#[test]
fn cfg_32_repeat_call_stability() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    for n in 0..iters(5_000) {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        for alias in Alias::ALL {
            for imp in [&c, &r] {
                let a = call(imp, src, alias);
                let b = call(imp, src, alias);
                let d = call(imp, src, alias);
                assert!(
                    a == b && b == d,
                    "{} is not deterministic across repeated calls (#{n}, {alias:?})",
                    imp.name
                );
            }
            assert_same(&c, &r, src, alias, &format!("row32 #{n}"));
        }
    }
}

// ---------------------------------------------------------------------------
// next_up / next_down without depending on unstable `f32::next_up`
// ---------------------------------------------------------------------------

fn next_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    let b = x.to_bits();
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let nb = if (b >> 31) == 0 { b + 1 } else { b - 1 };
    f32::from_bits(nb)
}

fn next_down(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    let b = x.to_bits();
    if x == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    let nb = if (b >> 31) == 0 { b - 1 } else { b + 1 };
    f32::from_bits(nb)
}

// ===========================================================================
// Row 33 : large-but-in-range `i`, where `(float)i` (CVTSI2SS) must round
// ===========================================================================

/// Random `h` whose `h/60` magnitude lands in `[2^(e-127), 2^(e-126))` for a
/// random biased exponent `e` in `[lo, hi)`, with a random sign and mantissa.
fn log_uniform_hue(rng: &mut Rng, lo: u32, hi: u32) -> u32 {
    let e = lo + (rng.next_u32() % (hi - lo));
    let mant = rng.next_u32() & 0x007F_FFFF;
    let sign = (rng.next_u32() & 1) << 31;
    sign | (e << 23) | mant
}

#[test]
fn cfg_33_large_in_range_i() {
    let (c, r) = load_pair();
    let mut rng = Rng::seeded();
    // biased exponents 132..=163  ->  |h| in [60, ~1.5e11)  ->  |h/60| in
    // [1, 2^31): `i` spans the whole int range, so `cvtsi2ss` must round.
    for n in 0..iters(200_000) {
        let h = log_uniform_hue(&mut rng, 132, 164);
        let s = nonzero_unit_s(&mut rng);
        let v = rng.unit().to_bits();
        assert_same(&c, &r, [h, s, v], Alias::Separate, &format!("row33 #{n}"));
    }
    // and the extreme in-range end, one ULP at a time around |h/60| = 2^31
    let two31_60 = (2147483648.0f32 * 60.0).to_bits();
    for d in 0..2048u32 {
        for sign in [0u32, 0x8000_0000] {
            let h = sign | (two31_60 - d);
            for _ in 0..iters(4) {
                let s = nonzero_unit_s(&mut rng);
                let v = rng.unit().to_bits();
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row33 near 2^31");
            }
        }
    }
}

// ===========================================================================
// Rows 34-36 : near-exhaustive sweeps of one slot at a time
// ===========================================================================

const LOW16: [u32; 6] = [0x0000, 0x0001, 0x7FFF, 0x8000, 0xFFFF, 0xACE1];

/// `(s, v)` pairs covering: normal, early-return, NaN and infinite saturation,
/// and negative / out-of-range value.
const SV_PAIRS: [(u32, u32); 6] = [
    (0x3F00_0000, 0x3F80_0000), // s=0.5, v=1.0
    (0x3F80_0000, 0x3E80_0000), // s=1.0, v=0.25
    (0x0000_0000, 0x4048_0000), // s=+0.0 (early return), v=3.125
    (0x7FC0_1234, 0xBF80_0000), // s=qNaN payload, v=-1.0
    (0x7F80_0000, 0x0000_0000), // s=+inf, v=+0.0  -> invalid op
    (0xC000_0000, 0x7F7F_FFFF), // s=-2.0, v=FLT_MAX
];

#[test]
fn cfg_34_exhaustive_hue_high16() {
    let (c, r) = load_pair();
    for hi in 0u32..=0xFFFF {
        for lo in LOW16 {
            let h = (hi << 16) | lo;
            for (s, v) in SV_PAIRS {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row34 hue sweep");
            }
        }
    }
}

#[test]
fn cfg_35_exhaustive_sat_high16() {
    let (c, r) = load_pair();
    let hues: [u32; 8] = [
        0x0000_0000,             // +0.0      -> i = 0
        30.0f32.to_bits(),       // arm 0
        90.0f32.to_bits(),       // arm 1
        200.0f32.to_bits(),      // arm 3
        330.0f32.to_bits(),      // default (i = 5)
        (-45.0f32).to_bits(),    // default (i < 0)
        POS_INF,                 // default (i = INT_MIN)
        0x7FC0_5555,             // NaN hue -> default
    ];
    for hi in 0u32..=0xFFFF {
        for lo in [0x0000u32, 0x0001, 0xFFFF] {
            let s = (hi << 16) | lo;
            for h in hues {
                assert_same(&c, &r, [h, s, 0x3F80_0000], Alias::Separate, "row35 sat sweep");
            }
        }
    }
}

#[test]
fn cfg_36_exhaustive_val_high16() {
    let (c, r) = load_pair();
    let cases: [(u32, u32); 6] = [
        (30.0f32.to_bits(), 0x3F00_0000),
        (150.0f32.to_bits(), 0x3F7F_FFFF),
        (270.0f32.to_bits(), 0x3800_0000),
        (350.0f32.to_bits(), 0x4000_0000),
        (POS_INF, 0x3F00_0000),
        (0x0000_0000, 0x0000_0000), // early return
    ];
    for hi in 0u32..=0xFFFF {
        for lo in [0x0000u32, 0x0001, 0xFFFF] {
            let v = (hi << 16) | lo;
            for (h, s) in cases {
                assert_same(&c, &r, [h, s, v], Alias::Separate, "row36 val sweep");
            }
        }
    }
}

// ===========================================================================
// Row 37 : high-volume unconstrained fuzz across all aliasing modes
// ===========================================================================

#[test]
fn cfg_37_massive_random_fuzz() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xDEAD_BEEF_1234_5678);
    for n in 0..iters(1_000_000) {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        let alias = Alias::ALL[(n as usize) & 3];
        assert_same(&c, &r, src, alias, &format!("row37 #{n}"));
    }
}

// ===========================================================================
// Row 38 : truly exhaustive hue sweep (all 2^32 bit patterns), sharded.
// Ignored by default because it takes minutes; run with
//   cargo test --release -- --ignored cfg_38  (optionally SHARDS/SHARD env)
// ===========================================================================

/// Fixed values for the two slots that are *not* being swept, chosen to reach
/// every distinguishable code path: normal, the `s == 0` early return, NaN with
/// a distinctive payload, and an infinity that triggers `0 * inf`.
const SWEEP_PRESETS: [(u32, u32); 5] = [
    (0x3F00_0000, 0x3F80_0000), // 0.5 , 1.0
    (0x3F80_0000, 0xBE80_0000), // 1.0 , -0.25
    (0x0000_0000, 0x0080_0001), // +0.0 (early return) , just above FLT_MIN
    (0x7FC0_1234, 0xFF80_0001), // qNaN payload , negative sNaN
    (0x7F80_0000, 0x8000_0000), // +inf , -0.0  -> invalid operation
];

/// Exhaustive sweep of all 2^32 bit patterns in one argument slot.
///
/// Configured by environment variables so it can be sharded across processes:
///   `SLOT`   0 = hue, 1 = saturation, 2 = value (default 0)
///   `PRESET` index into [`SWEEP_PRESETS`] for the other two slots (default 0)
///   `SHARDS` / `SHARD` stride and offset (defaults 64 / 0)
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly (see run_exhaustive.sh)"]
fn cfg_38_exhaustive_one_slot_all_bits() {
    let (c, r) = load_pair();
    let env_u64 = |k: &str, d: u64| -> u64 {
        std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
    };
    let slot = env_u64("SLOT", 0) as usize;
    let preset = env_u64("PRESET", 0) as usize % SWEEP_PRESETS.len();
    let shards = env_u64("SHARDS", 64).max(1);
    let shard = env_u64("SHARD", 0) % shards;
    let (a, b) = SWEEP_PRESETS[preset];
    let ctx = format!("row38 exhaustive slot={slot} preset={preset}");

    let mut x: u64 = shard;
    while x <= 0xFFFF_FFFF {
        let w = x as u32;
        let src = match slot {
            0 => [w, a, b],
            1 => [a, w, b],
            _ => [a, b, w],
        };
        assert_same(&c, &r, src, Alias::Separate, &ctx);
        x += shards;
    }
}
