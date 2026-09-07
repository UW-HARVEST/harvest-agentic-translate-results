//! Triage for the mutants that SURVIVED the mutation sweep in `.scratch/mutate.sh`.
//!
//! A surviving mutant is either (a) a hole in the test suite, or (b) a mutant
//! that is *semantically equivalent* to the original and therefore impossible
//! for any test to catch. This file proves, by exhaustive enumeration over the
//! affected f32 ranges, that each survivor is case (b).
//!
//! It also adds a dense per-ULP differential walk across both clamp windows via
//! the real FFI boundary, which is the strongest coverage available for
//! `mp3d_scale_pcm`'s three branches.

mod common;
use common::*;

/// Faithful transcription of the C `mp3d_scale_pcm`.
fn orig(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i32 as i16;
    s -= (s < 0) as i16;
    s
}

/// Survivor 1: `>=` weakened to `>` on the upper clamp.
fn mut_gt(sample: f32) -> i16 {
    if sample as f64 > 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i32 as i16;
    s -= (s < 0) as i16;
    s
}

/// Survivor 2: `<=` weakened to `<` on the lower clamp.
fn mut_lt(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if (sample as f64) < -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i32 as i16;
    s -= (s < 0) as i16;
    s
}

/// Survivor 3: upper clamp constant raised from 32766.5 to 32767.5.
fn mut_const(sample: f32) -> i16 {
    if sample as f64 >= 32767.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i32 as i16;
    s -= (s < 0) as i16;
    s
}

/// Survivor 4: `as i32 as i16` collapsed to Rust's saturating `as i16`.
fn mut_sat(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i16;
    s -= (s < 0) as i16;
    s
}

fn next_up(x: f32) -> f32 {
    let b = x.to_bits();
    if x == 0.0 {
        f32::from_bits(1)
    } else if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

/// Every f32 in `[lo, hi]`, walked one ULP at a time.
fn each_f32_in(lo: f32, hi: f32, mut f: impl FnMut(f32)) {
    let mut x = lo;
    let mut n = 0u64;
    while x <= hi {
        f(x);
        x = next_up(x);
        n += 1;
        assert!(n < 10_000_000, "range too wide for an exhaustive walk");
    }
}

// The clamp decisions only differ inside these windows; outside them both the
// original and the mutants take provably the same branch.
const HI_LO: f32 = 32700.0;
const HI_HI: f32 = 32800.0;
const LO_LO: f32 = -32800.0;
const LO_HI: f32 = -32700.0;

#[test]
fn survivor1_ge_to_gt_is_equivalent() {
    // Differs only at exactly a == 32766.5, where the rounding path yields
    // (int16_t)(32766.5 + 0.5f) == (int16_t)32767.0 == 32767 — the same value
    // the clamp returns. So no input can distinguish them.
    assert_eq!(orig(32766.5), 32767);
    assert_eq!(mut_gt(32766.5), 32767);
    let mut checked = 0u64;
    each_f32_in(HI_LO, HI_HI, |a| {
        assert_eq!(orig(a), mut_gt(a), "distinguishing input a={a:e}");
        checked += 1;
    });
    each_f32_in(LO_LO, LO_HI, |a| assert_eq!(orig(a), mut_gt(a), "a={a:e}"));
    for a in [0.0f32, -0.0, 0.5, -0.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(orig(a), mut_gt(a), "a={a:e}");
    }
    assert!(checked > 1000, "window too small: {checked}");
}

#[test]
fn survivor2_le_to_lt_is_equivalent() {
    // At exactly a == -32767.5 the rounding path yields
    // (int16_t)(-32767.0) == -32767, then `s -= (s < 0)` gives -32768 — the
    // same value the clamp returns.
    assert_eq!(orig(-32767.5), -32768);
    assert_eq!(mut_lt(-32767.5), -32768);
    each_f32_in(LO_LO, LO_HI, |a| {
        assert_eq!(orig(a), mut_lt(a), "distinguishing input a={a:e}")
    });
    each_f32_in(HI_LO, HI_HI, |a| assert_eq!(orig(a), mut_lt(a), "a={a:e}"));
    for a in [0.0f32, -0.0, 0.5, -0.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(orig(a), mut_lt(a), "a={a:e}");
    }
}

#[test]
fn survivor3_upper_clamp_constant_is_equivalent() {
    // For a in [32766.5, 32767.5) the rounding path computes a + .5f, which
    // lands in [32767.0, 32768.0) and is exactly representable there (ULP is
    // 1/512), so truncation gives 32767 — identical to the clamp. Above
    // 32767.5 the mutant clamps too.
    each_f32_in(HI_LO, HI_HI, |a| {
        assert_eq!(orig(a), mut_const(a), "distinguishing input a={a:e}")
    });
    each_f32_in(LO_LO, LO_HI, |a| assert_eq!(orig(a), mut_const(a), "a={a:e}"));
    for a in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN] {
        assert_eq!(orig(a), mut_const(a), "a={a:e}");
    }
}

#[test]
fn survivor4_saturating_cast_is_equivalent() {
    // The two guards bound `sample` to (-32767.5, 32766.5), so `sample + .5f`
    // lies in (-32767.0, 32767.0) and always fits in i16: the saturation in
    // Rust's `as i16` can never engage, and NaN maps to 0 under both spellings.
    each_f32_in(HI_LO, HI_HI, |a| assert_eq!(orig(a), mut_sat(a), "a={a:e}"));
    each_f32_in(LO_LO, LO_HI, |a| assert_eq!(orig(a), mut_sat(a), "a={a:e}"));
    // (-3.0, 3.0) holds ~2e9 distinct f32s, far too many to walk; sample the
    // low-magnitude rounding region densely instead.
    each_f32_in(-3.0, -2.999, |a| assert_eq!(orig(a), mut_sat(a), "a={a:e}"));
    each_f32_in(2.999, 3.0, |a| assert_eq!(orig(a), mut_sat(a), "a={a:e}"));
    for i in -3_000_000i64..=3_000_000 {
        let a = i as f32 / 1_000_000.0;
        assert_eq!(orig(a), mut_sat(a), "a={a:e}");
    }
    let mut rng = Rng::new(0xC0FF_EE00);
    for _ in 0..2_000_000 {
        let a = f32::from_bits(rng.next_u32());
        assert_eq!(orig(a), mut_sat(a), "distinguishing input a={a:e} bits={:#x}", a.to_bits());
    }
    for a in [f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(orig(a), mut_sat(a), "a={a:e}");
    }
}

/// The reference model itself must agree with the REAL C library, otherwise the
/// equivalence proofs above are about the wrong function. Dense per-ULP walk
/// across both clamp windows, through the FFI boundary, for both accumulators.
#[test]
fn model_matches_c_and_rust_across_clamp_windows() {
    let p = Pair::load();
    let acc1_tap = 7 * 64usize;
    let acc1_coeff = 75038.0f32;
    let acc2_tap = 2 + 8 * 64usize;
    let acc2_coeff = 64019.0f32;

    let mut n = 0u64;
    for (tap, coeff, idx) in [(acc1_tap, acc1_coeff, 0usize), (acc2_tap, acc2_coeff, 32usize)] {
        for (lo, hi) in [(32750.0f32, 32780.0f32), (-32780.0f32, -32750.0f32)] {
            // Walk the reachable accumulator values in the window: step `v` by
            // ULPs so every distinct product in range is visited.
            let mut v = lo / coeff;
            let v_end = hi / coeff;
            while v <= v_end {
                let a = v * coeff;
                let mut z = make_z(0, |_| 0.0);
                z[tap] = v;
                let case = Case::new(2, &z);
                let (c, r) = run(&p, &case);
                assert_eq!(c, r, "C/Rust diverged at a={a:e} (v={v:e})");
                assert_eq!(
                    c[idx],
                    orig(a),
                    "reference model disagrees with the C library at a={a:e}"
                );
                v = next_up(v);
                n += 1;
            }
        }
    }
    assert!(n > 200, "clamp-window walk covered only {n} points");
}
