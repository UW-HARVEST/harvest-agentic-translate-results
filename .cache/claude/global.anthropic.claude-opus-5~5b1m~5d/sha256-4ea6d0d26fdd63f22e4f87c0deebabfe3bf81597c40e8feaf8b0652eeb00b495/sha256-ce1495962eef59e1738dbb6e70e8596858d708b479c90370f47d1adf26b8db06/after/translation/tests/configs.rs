//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row drives the single public entry point `hsl_to_rgb` through both
//! shared objects with many fixed-seed randomized inputs and compares the whole
//! post-call memory image bit-for-bit (see `common::assert_same_layout`).

mod common;

use common::*;

/// How many randomized samples each ordinary row uses.
const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Hue-sector rows: A1 (branch taken) x randomized saturation/lightness
// ---------------------------------------------------------------------------

/// `s` uniform in `(0, 1]` — never exactly 0, so the early return is not taken.
fn sat_unit(rng: &mut Rng) -> f32 {
    let v = rng.range(0.0, 1.0);
    if v == 0.0 { 1.0 } else { v }
}

fn sector_row(ctx: &str, seed: u64, lo: f32, hi: f32) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let h = rng.range(lo, hi);
        let s = sat_unit(&mut rng);
        let l = rng.range(0.0, 1.0);
        assert_same(ctx, h, s, l);
    }
    // Plus the two endpoints of the interval, which the uniform draw reaches
    // only with probability ~0.
    assert_same(ctx, lo, 0.7, 0.3);
    assert_same(ctx, next_down(hi), 0.7, 0.3);
}

#[test]
fn c1_sector_b1_hue_0_to_60() {
    sector_row("C1/B1 h in [0,60)", 0xC001, 0.0, 60.0);
}

#[test]
fn c2_sector_b2_hue_60_to_120() {
    sector_row("C2/B2 h in [60,120)", 0xC002, 60.0, 120.0);
}

/// The 60 degrees that `lib.c:27`'s `h < 120.0f && h < 180.0f` typo makes
/// unreachable: this sector must come out grey, not cyan.
#[test]
fn c3_sector_hole_hue_120_to_180_is_grey() {
    sector_row("C3/B7-hole h in [120,180)", 0xC003, 120.0, 180.0);
    // Additionally pin down *what* the C produces, so the row cannot pass by
    // both sides being wrong in the same way.
    let (h, s, l) = (150.0f32, 1.0f32, 0.5f32);
    let out = run(c_lib(), h, s, l);
    assert_eq!(out[0], out[1], "expected grey from the [120,180) hole");
    assert_eq!(out[1], out[2], "expected grey from the [120,180) hole");
    assert_same("C3 pinned", h, s, l);
}

#[test]
fn c4_sector_b4_hue_180_to_240() {
    sector_row("C4/B4 h in [180,240)", 0xC004, 180.0, 240.0);
}

#[test]
fn c5_sector_b5_hue_240_to_300() {
    sector_row("C5/B5 h in [240,300)", 0xC005, 240.0, 300.0);
}

#[test]
fn c6_sector_b6_hue_300_to_360() {
    sector_row("C6/B6 h in [300,360)", 0xC006, 300.0, 360.0);
}

#[test]
fn c7_hue_at_or_above_360_is_grey() {
    sector_row("C7/B7 h in [360,1e6)", 0xC007, 360.0, 1.0e6);
}

/// Negative hues are the *only* way to reach the third branch, because of the
/// `h < 120 && h < 180` typo.
#[test]
fn c8_negative_hue_reaches_branch_three() {
    sector_row("C8/B3 h in (-1e6,0)", 0xC008, -1.0e6, 0.0);
    let out = run(c_lib(), -30.0, 1.0, 0.5);
    assert_ne!(
        out[0], out[1],
        "negative hue should reach branch 3 ({{m, c+m, x+m}}), not the grey else"
    );
    assert_same("C8 pinned", -30.0, 1.0, 0.5);
}

// ---------------------------------------------------------------------------
// Exact-boundary rows
// ---------------------------------------------------------------------------

#[test]
fn c9_exact_hue_boundaries() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..2_000 {
        for &h in HUE_BOUNDARIES {
            let s = sat_unit(&mut rng);
            let l = rng.range(0.0, 1.0);
            assert_same("C9 exact boundary", h, s, l);
        }
    }
    // ... and against the special-value pools too.
    for &h in HUE_BOUNDARIES {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("C9 boundary x specials", h, s, l);
            }
        }
    }
}

#[test]
fn c10_one_ulp_either_side_of_every_boundary() {
    let hues = hue_boundary_neighbours();
    let mut rng = Rng::new(0xC010);
    for _ in 0..2_000 {
        for &h in &hues {
            let s = sat_unit(&mut rng);
            let l = rng.range(0.0, 1.0);
            assert_same("C10 boundary +/- 1ulp", h, s, l);
        }
    }
    for &h in &hues {
        for &s in &specials_and_nans() {
            for &l in &specials_and_nans() {
                assert_same("C10 boundary+/-1ulp x specials", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Lightness-regime rows
// ---------------------------------------------------------------------------

fn lightness_row(ctx: &str, seed: u64, gen_l: impl Fn(&mut Rng) -> f32) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let h = random_hue_any_sector(&mut rng);
        let s = sat_unit(&mut rng);
        let l = gen_l(&mut rng);
        assert_same(ctx, h, s, l);
    }
    // Every exact sector boundary as well, so the regime is checked on the
    // branch edges and not only in sector interiors.
    for &h in HUE_BOUNDARIES {
        for _ in 0..200 {
            let s = sat_unit(&mut rng);
            let l = gen_l(&mut rng);
            assert_same(ctx, h, s, l);
        }
    }
}

#[test]
fn c11_negative_lightness() {
    lightness_row("C11 l<0", 0xC011, |r| {
        let v = r.range(-1.0e3, 0.0);
        if v == 0.0 { -1.0 } else { v }
    });
}

#[test]
fn c12_lightness_signed_zero() {
    let mut rng = Rng::new(0xC012);
    for &l in &[0.0f32, -0.0f32] {
        for _ in 0..N {
            let h = random_hue_any_sector(&mut rng);
            let s = sat_unit(&mut rng);
            assert_same("C12 l = +/-0", h, s, l);
        }
        for &h in HUE_BOUNDARIES {
            for &s in &specials_and_nans() {
                assert_same("C12 l = +/-0 x specials", h, s, l);
            }
        }
    }
}

#[test]
fn c13_lightness_below_half() {
    lightness_row("C13 0<l<0.5", 0xC013, |r| {
        let v = r.range(0.0, 0.5);
        if v == 0.0 { 0.25 } else { v }
    });
}

#[test]
fn c14_lightness_exactly_half() {
    lightness_row("C14 l=0.5", 0xC014, |_| 0.5);
}

#[test]
fn c15_lightness_above_half() {
    lightness_row("C15 0.5<l<1", 0xC015, |r| {
        let v = r.range(0.5, 1.0);
        if v == 0.5 { 0.75 } else { v }
    });
}

#[test]
fn c16_lightness_exactly_one() {
    lightness_row("C16 l=1", 0xC016, |_| 1.0);
}

#[test]
fn c17_lightness_above_one() {
    lightness_row("C17 l>1", 0xC017, |r| {
        let v = r.range(1.0, 1.0e3);
        if v == 1.0 { 2.0 } else { v }
    });
}

#[test]
fn c18_lightness_special_pool() {
    let pool = specials_and_nans();
    let mut rng = Rng::new(0xC018);
    for _ in 0..N {
        let h = random_hue_any_sector(&mut rng);
        let s = sat_unit(&mut rng);
        let l = rng.pick(&pool);
        assert_same("C18 l special", h, s, l);
    }
    // Exhaustive over (boundary hue) x (special l) x (special s).
    for &h in HUE_BOUNDARIES {
        for &l in &pool {
            for &s in &pool {
                assert_same("C18 exhaustive specials", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Saturation-regime rows
// ---------------------------------------------------------------------------

/// `s == +0.0` is the library's only early return; it must copy `l` verbatim for
/// *every* hue, including NaN and out-of-range hues.
#[test]
fn c19_saturation_positive_zero_early_return() {
    let pool = specials_and_nans();
    let mut rng = Rng::new(0xC019);
    for _ in 0..N {
        let h = random_hue_any_sector(&mut rng);
        let l = if rng.below(2) == 0 { rng.pick(&pool) } else { rng.bits_f32() };
        assert_same("C19 s=+0", h, 0.0, l);
    }
    for &h in HUE_BOUNDARIES {
        for &l in &pool {
            assert_same("C19 s=+0 boundary x special l", h, 0.0, l);
        }
    }
    for &h in &pool {
        for &l in &pool {
            assert_same("C19 s=+0 special h x special l", h, 0.0, l);
        }
    }
}

#[test]
fn c20_saturation_negative_zero_early_return() {
    let pool = specials_and_nans();
    let mut rng = Rng::new(0xC020);
    let neg_zero = -0.0f32;
    assert_eq!(neg_zero.to_bits(), 0x8000_0000);
    for _ in 0..N {
        let h = random_hue_any_sector(&mut rng);
        let l = if rng.below(2) == 0 { rng.pick(&pool) } else { rng.bits_f32() };
        assert_same("C20 s=-0", h, neg_zero, l);
    }
    for &h in &pool {
        for &l in &pool {
            assert_same("C20 s=-0 special x special", h, neg_zero, l);
        }
    }
}

fn saturation_row(ctx: &str, seed: u64, gen_s: impl Fn(&mut Rng) -> f32) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let h = random_hue_any_sector(&mut rng);
        let s = gen_s(&mut rng);
        let l = rng.range(0.0, 1.0);
        assert_same(ctx, h, s, l);
    }
    for &h in HUE_BOUNDARIES {
        for _ in 0..200 {
            let s = gen_s(&mut rng);
            let l = rng.range(0.0, 1.0);
            assert_same(ctx, h, s, l);
        }
    }
}

#[test]
fn c21_saturation_in_unit_interval() {
    saturation_row("C21 0<s<1", 0xC021, |r| {
        let v = r.range(0.0, 1.0);
        if v == 0.0 { 0.5 } else { v }
    });
}

#[test]
fn c22_saturation_exactly_one() {
    saturation_row("C22 s=1", 0xC022, |_| 1.0);
}

#[test]
fn c23_saturation_above_one() {
    saturation_row("C23 s>1", 0xC023, |r| {
        let v = r.range(1.0, 1.0e6);
        if v == 1.0 { 2.0 } else { v }
    });
}

#[test]
fn c24_saturation_negative() {
    saturation_row("C24 s<0", 0xC024, |r| {
        let v = r.range(-1.0e6, 0.0);
        if v == 0.0 { -1.0 } else { v }
    });
}

#[test]
fn c25_saturation_special_pool() {
    // Drop the two zeros: they are rows C19/C20 and take a different path.
    let pool: Vec<f32> = specials_and_nans().into_iter().filter(|v| *v != 0.0).collect();
    let mut rng = Rng::new(0xC025);
    for _ in 0..N {
        let h = random_hue_any_sector(&mut rng);
        let s = rng.pick(&pool);
        let l = rng.range(0.0, 1.0);
        assert_same("C25 s special", h, s, l);
    }
    for &h in HUE_BOUNDARIES {
        for &s in &pool {
            for &l in &specials_and_nans() {
                assert_same("C25 exhaustive specials", h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// fmodf-regime rows (the axis where the two libraries run *different* code:
// glibc fmodf vs the statically linked compiler_builtins fmodf)
// ---------------------------------------------------------------------------

fn fmod_row(ctx: &str, seed: u64, gen_h: impl Fn(&mut Rng) -> f32) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let h = gen_h(&mut rng);
        let s = sat_unit(&mut rng);
        let l = rng.range(0.0, 1.0);
        assert_same(ctx, h, s, l);
    }
    // Repeat with hostile s/l so the NaN produced by fmodf has to interact with
    // a NaN/Inf produced by the chroma computation.
    let pool = specials_and_nans();
    for _ in 0..N {
        let h = gen_h(&mut rng);
        let s = rng.pick(&pool);
        let l = rng.pick(&pool);
        assert_same(ctx, h, s, l);
    }
}

#[test]
fn c26_fmod_identity_range() {
    // |h/60| < 2  =>  fmodf returns its first argument unchanged.
    fmod_row("C26 |h|<120", 0xC026, |r| r.range(-120.0, 120.0));
}

#[test]
fn c27_fmod_reduction_range() {
    fmod_row("C27 |h|>=120, moderate exponents", 0xC027, |r| {
        let mag = (2.0f32).powi(r.below(61) as i32 - 20) * (1.0 + r.unit());
        let mag = if mag < 120.0 { mag + 120.0 } else { mag };
        if r.below(2) == 0 { mag } else { -mag }
    });
}

#[test]
fn c28_fmod_full_exponent_range() {
    fmod_row("C28 h log-uniform over all exponents", 0xC028, |r| r.log_uniform());
}

#[test]
fn c29_fmod_subnormal_hue() {
    fmod_row("C29 h subnormal", 0xC029, |r| {
        let sign = (r.next_u32() & 1) << 31;
        // Exponent 0 (subnormal) or 1 (smallest normals).
        let exp = r.below(2);
        f32::from_bits(sign | (exp << 23) | (r.next_u32() & 0x007f_ffff))
    });
}

#[test]
fn c30_fmod_domain_errors() {
    let hues: Vec<f32> = {
        let mut v = vec![f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN];
        v.extend(nan_floats());
        v
    };
    let pool = specials_and_nans();
    for &h in &hues {
        for &s in &pool {
            for &l in &pool {
                assert_same("C30 fmodf domain error", h, s, l);
            }
        }
    }
    let mut rng = Rng::new(0xC030);
    for _ in 0..N {
        let h = rng.pick(&hues);
        let s = sat_unit(&mut rng);
        let l = rng.range(0.0, 1.0);
        assert_same("C30 fmodf domain error x random s/l", h, s, l);
    }
}

// ---------------------------------------------------------------------------
// Buffer-shape / aliasing rows
// ---------------------------------------------------------------------------

fn random_triple(rng: &mut Rng) -> (f32, f32, f32) {
    // Mixed generator so aliasing rows also see NaNs, infinities and the
    // early-return path.
    let pool = specials_and_nans();
    let mk = |r: &mut Rng| match r.below(4) {
        0 => r.bits_f32(),
        1 => r.log_uniform(),
        2 => pool[r.below(pool.len() as u32) as usize],
        _ => r.range(-400.0, 400.0),
    };
    let h = if rng.below(3) == 0 { random_hue_any_sector(rng) } else { mk(rng) };
    let s = mk(rng);
    let l = mk(rng);
    (h, s, l)
}

fn layout_row(ctx: &str, seed: u64, lay: Layout) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let (h, s, l) = random_triple(&mut rng);
        assert_same_layout(ctx, lay, h, s, l);
    }
}

#[test]
fn c31_disjoint_buffers_with_canaries() {
    layout_row("C31 disjoint", 0xC031, DISJOINT);
    // A few more disjoint placements, both orders.
    layout_row("C31 disjoint dest-first", 0xC131, Layout::new(11, 8, 0));
    layout_row("C31 disjoint tight", 0xC231, Layout::new(6, 0, 3));
}

#[test]
fn c32_full_aliasing_dest_equals_src() {
    layout_row("C32 dest==src", 0xC032, Layout::new(11, 4, 4));
}

#[test]
fn c33_partial_overlap_dest_is_src_plus_one() {
    layout_row("C33 dest==src+1", 0xC033, Layout::new(11, 4, 5));
}

#[test]
fn c34_partial_overlap_dest_is_src_minus_one() {
    layout_row("C34 dest==src-1", 0xC034, Layout::new(11, 4, 3));
}

#[test]
fn c35_partial_overlap_plus_minus_two() {
    layout_row("C35 dest==src+2", 0xC035, Layout::new(11, 4, 6));
    layout_row("C35 dest==src-2", 0xC135, Layout::new(11, 4, 2));
}

#[test]
fn c36_every_offset_in_a_16_float_allocation() {
    let mut rng = Rng::new(0xC036);
    for dst_off in 0..=8usize {
        for src_off in 0..=8usize {
            let lay = Layout::new(16, src_off, dst_off);
            for _ in 0..400 {
                let (h, s, l) = random_triple(&mut rng);
                assert_same_layout("C36 offsets", lay, h, s, l);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sequencing / statelessness rows
// ---------------------------------------------------------------------------

#[test]
fn c37_long_sequences_and_interleaving_agree() {
    let mut rng = Rng::new(0xC037);
    let cases: Vec<(f32, f32, f32)> = (0..4096).map(|_| random_triple(&mut rng)).collect();

    // (a) Each library run to completion on its own, results collected.
    let c_batch: Vec<[u32; 3]> = cases.iter().map(|&(h, s, l)| run(c_lib(), h, s, l)).collect();
    let mut rust_batches = Vec::new();
    for r in rust_libs() {
        rust_batches.push(cases.iter().map(|&(h, s, l)| run(r, h, s, l)).collect::<Vec<_>>());
    }
    for (r, batch) in rust_libs().iter().zip(&rust_batches) {
        assert_eq!(&c_batch, batch, "{} diverged in batch mode", r.name);
    }

    // (b) The same calls interleaved C/Rust/C/Rust, to catch any hidden state
    //     or cross-library interference (e.g. a modified MXCSR).
    for (i, &(h, s, l)) in cases.iter().enumerate() {
        assert_same("C37 interleaved", h, s, l);
        // and confirm the answer still matches the batch-mode answer
        assert_eq!(run(c_lib(), h, s, l), c_batch[i], "C not deterministic at {i}");
        for (r, batch) in rust_libs().iter().zip(&rust_batches) {
            assert_eq!(run(r, h, s, l), batch[i], "{} not deterministic at {i}", r.name);
        }
    }
}

#[test]
fn c38_repeated_identical_calls_are_stable() {
    let pool = specials_and_nans();
    for &h in &pool {
        for &s in &pool {
            for &l in &pool {
                let first_c = run(c_lib(), h, s, l);
                for _ in 0..8 {
                    assert_eq!(run(c_lib(), h, s, l), first_c);
                }
                for r in rust_libs() {
                    let first_r = run(r, h, s, l);
                    assert_eq!(first_r, first_c, "{} diverged on repeat", r.name);
                    for _ in 0..8 {
                        assert_eq!(run(r, h, s, l), first_r);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Whole-space fuzz rows
// ---------------------------------------------------------------------------

#[test]
fn c39_uniform_random_bit_patterns() {
    let mut rng = Rng::new(0xC039);
    for _ in 0..200_000 {
        let h = rng.bits_f32();
        let s = rng.bits_f32();
        let l = rng.bits_f32();
        assert_same("C39 random bits", h, s, l);
    }
}

#[test]
fn c40_structured_generator_cross_product() {
    let pool = specials_and_nans();
    // Four independent generators per component => 64 combinations.
    let gens: [fn(&mut Rng, &[f32]) -> f32; 4] = [
        |r, _| r.bits_f32(),
        |r, _| r.log_uniform(),
        |r, p| p[r.below(p.len() as u32) as usize],
        |r, _| r.range(-400.0, 400.0),
    ];
    let mut rng = Rng::new(0xC040);
    for (gi, gh) in gens.iter().enumerate() {
        for (gj, gs) in gens.iter().enumerate() {
            for (gk, gl) in gens.iter().enumerate() {
                let ctx = format!("C40 gen({gi},{gj},{gk})");
                for _ in 0..3_000 {
                    let h = gh(&mut rng, &pool);
                    let s = gs(&mut rng, &pool);
                    let l = gl(&mut rng, &pool);
                    assert_same(&ctx, h, s, l);
                }
            }
        }
    }
}

#[test]
fn c41_dense_hue_grid() {
    let mut rng = Rng::new(0xC041);
    let mut h = -720.0f32;
    let mut n = 0usize;
    while h <= 1080.0 {
        let s = sat_unit(&mut rng);
        let l = rng.range(0.0, 1.0);
        assert_same("C41 dense hue grid", h, s, l);
        // and a hostile s/l at the same hue
        let pool = specials_and_nans();
        let s2 = rng.pick(&pool);
        let l2 = rng.pick(&pool);
        assert_same("C41 dense hue grid, hostile s/l", h, s2, l2);
        h += 0.25;
        n += 1;
    }
    assert_eq!(n, 7201, "hue grid did not have the expected length");
}

#[test]
fn c42_sweep_all_high_16_bits_of_hue() {
    // Every sign/exponent/high-mantissa combination of the hue, with a fixed
    // non-trivial low half, so all 2^8 exponents (incl. subnormal and NaN/Inf)
    // are visited.
    let mut rng = Rng::new(0xC042);
    for hi in 0u32..=0xFFFF {
        let h = f32::from_bits((hi << 16) | 0x5A3C);
        let s = sat_unit(&mut rng);
        let l = rng.range(0.0, 1.0);
        assert_same("C42 hue high-bit sweep", h, s, l);
    }
    // Second pass with saturation/lightness taken from the special pool.
    let pool = specials_and_nans();
    for hi in 0u32..=0xFFFF {
        let h = f32::from_bits((hi << 16) | 0x0001);
        let s = rng.pick(&pool);
        let l = rng.pick(&pool);
        assert_same("C42 hue high-bit sweep, specials", h, s, l);
    }
}
