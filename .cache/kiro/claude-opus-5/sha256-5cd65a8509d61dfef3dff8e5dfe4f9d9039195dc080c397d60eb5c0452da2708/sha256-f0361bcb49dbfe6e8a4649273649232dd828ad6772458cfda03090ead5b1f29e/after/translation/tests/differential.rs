//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached exclusively through `dlopen`/`dlsym` on the
//! two shared objects.

mod common;

use std::ffi::c_int;

use common::*;

fn diff_sc(ctx: &str, a: &[f32], b: &[f32], length: c_int) {
    let l = libs();
    let c = call_sc(&l.c, a, b, length);
    let r = call_sc(&l.r, a, b, length);
    assert_sc_eq(ctx, &c, &r);
}

fn diff_sc_aliased(ctx: &str, a: &[f32], length: c_int) {
    let l = libs();
    let c = call_sc_aliased(&l.c, a, length);
    let r = call_sc_aliased(&l.r, a, length);
    assert_sc_eq(ctx, &c, &r);
}

fn diff_match(ctx: &str, t: &[f64], r: &[f64], bins: c_int, thr: f64) {
    let l = libs();
    let cc = call_match(&l.c, t, r, bins, thr);
    let rr = call_match(&l.r, t, r, bins, thr);
    assert_match_eq(ctx, &cc, &rr);
}

fn diff_match_aliased(ctx: &str, d: &[f64], bins: c_int, thr: f64) {
    let l = libs();
    let cc = call_match_aliased(&l.c, d, bins, thr);
    let rr = call_match_aliased(&l.r, d, bins, thr);
    assert_match_eq(ctx, &cc, &rr);
}

// ===========================================================================
// A1 = spectral_contrast (lowest-level public entry point)
// ===========================================================================

/// Row 1 — `length <= 0`, buffers present and NULL.
#[test]
fn row01_sc_nonpositive_length() {
    let l = libs();
    let mut rng = Rng::new(0x1001);
    for &len in &[0i32, -1, -2, -16, i32::MIN, i32::MIN + 1] {
        let a = gen32(Shape32::Unit, 8, &mut rng);
        let b = gen32(Shape32::Unit, 8, &mut rng);
        diff_sc(&format!("row01 len={len}"), &a, &b, len);
        let cn = call_sc_null(&l.c, len);
        let rn = call_sc_null(&l.r, len);
        assert_eq!(cn, rn, "row01 NULL len={len}: C=0x{cn:016X} Rust=0x{rn:016X}");
    }
}

/// Row 2 — `length == 1`.
#[test]
fn row02_sc_length_one() {
    let mut rng = Rng::new(0x1002);
    for i in 0..400 {
        let a = gen32(Shape32::Signed, 1, &mut rng);
        let b = gen32(Shape32::Signed, 1, &mut rng);
        diff_sc(&format!("row02 #{i}"), &a, &b, 1);
    }
}

/// Row 3 — `length == 2`.
#[test]
fn row03_sc_length_two() {
    let mut rng = Rng::new(0x1003);
    for i in 0..400 {
        let a = gen32(Shape32::Signed, 2, &mut rng);
        let b = gen32(Shape32::Signed, 2, &mut rng);
        diff_sc(&format!("row03 #{i}"), &a, &b, 2);
    }
}

/// Row 4 — `3 <= length <= 15`.
#[test]
fn row04_sc_small_lengths() {
    let mut rng = Rng::new(0x1004);
    for n in 3..=15usize {
        for i in 0..120 {
            let a = gen32(Shape32::Unit, n, &mut rng);
            let b = gen32(Shape32::Unit, n, &mut rng);
            diff_sc(&format!("row04 n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 5 — `length` around `N_SMOOTH`.
#[test]
fn row05_sc_around_16() {
    let mut rng = Rng::new(0x1005);
    for n in [15usize, 16, 17] {
        for i in 0..200 {
            let a = gen32(Shape32::Unit, n, &mut rng);
            let b = gen32(Shape32::Signed, n, &mut rng);
            diff_sc(&format!("row05 n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 6 — larger lengths.
#[test]
fn row06_sc_large_lengths() {
    let mut rng = Rng::new(0x1006);
    for n in [31usize, 32, 33, 64, 257, 1024] {
        for i in 0..40 {
            let a = gen32(Shape32::Unit, n, &mut rng);
            let b = gen32(Shape32::Signed, n, &mut rng);
            diff_sc(&format!("row06 n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 7 — fully bit-random `f32` lanes (all exponent classes at once).
#[test]
fn row07_sc_bit_random() {
    let mut rng = Rng::new(0x1007);
    for i in 0..2000 {
        let n = 1 + rng.range(40);
        let a = gen32(Shape32::BitRandom, n, &mut rng);
        let b = gen32(Shape32::BitRandom, n, &mut rng);
        diff_sc(&format!("row07 #{i} n={n}"), &a, &b, n as c_int);
    }
}

/// Row 8 — zero magnitude (`0/0`, `x/0`).
#[test]
fn row08_sc_zero_magnitude() {
    let mut rng = Rng::new(0x1008);
    for n in [1usize, 2, 5, 16, 17, 33] {
        for i in 0..150 {
            let a = gen32(Shape32::Zeros, n, &mut rng);
            let b = if rng.bool() {
                gen32(Shape32::Zeros, n, &mut rng)
            } else {
                gen32(Shape32::Unit, n, &mut rng)
            };
            diff_sc(&format!("row08 n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 9 — denormals (magnitude underflows) and huge lanes (`dot_product` overflows).
#[test]
fn row09_sc_denormal_and_huge() {
    let mut rng = Rng::new(0x1009);
    for n in [1usize, 3, 16, 17, 40] {
        for i in 0..150 {
            let a = gen32(Shape32::Denormals, n, &mut rng);
            let b = gen32(Shape32::Denormals, n, &mut rng);
            diff_sc(&format!("row09d n={n} #{i}"), &a, &b, n as c_int);
            let a = gen32(Shape32::Huge, n, &mut rng);
            let b = gen32(Shape32::Huge, n, &mut rng);
            diff_sc(&format!("row09h n={n} #{i}"), &a, &b, n as c_int);
            let a = gen32(Shape32::Huge, n, &mut rng);
            let b = gen32(Shape32::Denormals, n, &mut rng);
            diff_sc(&format!("row09m n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 10 — infinities (`inf*inf`, `inf*0`).
#[test]
fn row10_sc_infinities() {
    let mut rng = Rng::new(0x100A);
    for n in [1usize, 2, 16, 17, 40] {
        for i in 0..200 {
            let a = gen32(Shape32::Infinities, n, &mut rng);
            let b = match rng.range(3) {
                0 => gen32(Shape32::Infinities, n, &mut rng),
                1 => gen32(Shape32::Zeros, n, &mut rng),
                _ => gen32(Shape32::Unit, n, &mut rng),
            };
            diff_sc(&format!("row10 n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 11 — NaN lanes with distinct payloads in both operands. This pins the
/// `MULSS` / `ADDSD` destination-operand roles.
#[test]
fn row11_sc_nan_payloads() {
    let mut rng = Rng::new(0x100B);
    // exhaustive payload pairs at length 1 and 2
    for &pa in F32_NANS.iter() {
        for &pb in F32_NANS.iter() {
            let a = vec![f32::from_bits(pa)];
            let b = vec![f32::from_bits(pb)];
            diff_sc(&format!("row11 pair 0x{pa:08X}/0x{pb:08X} n=1"), &a, &b, 1);
            let a2 = vec![f32::from_bits(pa), f32::from_bits(pb)];
            let b2 = vec![f32::from_bits(pb), f32::from_bits(pa)];
            diff_sc(&format!("row11 pair 0x{pa:08X}/0x{pb:08X} n=2"), &a2, &b2, 2);
            // NaN mixed with a finite value, both operand positions
            let a3 = vec![f32::from_bits(pa), 2.0];
            let b3 = vec![3.0, f32::from_bits(pb)];
            diff_sc(&format!("row11 mixed 0x{pa:08X}/0x{pb:08X}"), &a3, &b3, 2);
        }
    }
    for n in [1usize, 2, 3, 16, 17, 40] {
        for i in 0..200 {
            let a = gen32(Shape32::Nans, n, &mut rng);
            let b = gen32(Shape32::Nans, n, &mut rng);
            diff_sc(&format!("row11 rand n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

/// Row 12 — `a == b` aliased.
#[test]
fn row12_sc_aliased() {
    let mut rng = Rng::new(0x100C);
    for shape in ALL_SHAPE32 {
        for n in [0usize, 1, 2, 16, 17, 40] {
            for i in 0..40 {
                let a = gen32(shape, n, &mut rng);
                diff_sc_aliased(&format!("row12 {shape:?} n={n} #{i}"), &a, n as c_int);
            }
        }
    }
}

/// Row 13 — mixed-sign lanes, negative dot products.
#[test]
fn row13_sc_mixed_sign() {
    let mut rng = Rng::new(0x100D);
    for n in [64usize, 128, 512] {
        for i in 0..60 {
            let a = gen32(Shape32::Signed, n, &mut rng);
            let b = gen32(Shape32::Signed, n, &mut rng);
            diff_sc(&format!("row13 n={n} #{i}"), &a, &b, n as c_int);
            let ramp = gen32(Shape32::Ramp, n, &mut rng);
            let neg: Vec<f32> = ramp.iter().map(|x| -x).collect();
            diff_sc(&format!("row13neg n={n} #{i}"), &ramp, &neg, n as c_int);
        }
    }
}

// ===========================================================================
// A2 = match (composed pipeline)
// ===========================================================================

/// Row 14 — `bins == 0`.
///
/// The C **cannot** be called in-process here: with `bins == 0` GCC allocates a
/// zero-length VLA (so `rsp` is unchanged), and `differentiate` then executes
/// `v[length - 1] = 0`, i.e. `v[-1] = 0`, which lands exactly on `preprocess`'s
/// saved return address and makes the C `.so` jump to address 0. This is
/// verified out-of-process in `error_paths.rs::err04_bins_zero_out_of_process`
/// (`ERRORS.md` rows 4 and 9); the smallest in-process size is therefore
/// `bins == 1` (row 15).
#[test]
fn row14_match_bins_zero() {
    // Assert the *pre-condition* of this row rather than a value: the C really
    // does die, so there is no defined result to compare against.
    let (c_sig, _) = common::probe::spawn_probe("bins_zero");
    assert_eq!(c_sig, Some(11), "expected C SIGSEGV for bins == 0, got {c_sig:?}");
}

/// Row 15 — `bins == 1` (`differentiate` loop skipped) across every `threshold`.
#[test]
fn row15_match_bins_one() {
    let mut rng = Rng::new(0x2002);
    let mut ths: Vec<f64> = THRESHOLDS.to_vec();
    ths.push(nan_threshold());
    for &thr in &ths {
        for shape in ALL_SHAPE64 {
            for i in 0..40 {
                let t = gen64(shape, 1, &mut rng);
                let r = gen64(shape, 1, &mut rng);
                diff_match(&format!("row15 thr={thr:e} {shape:?} #{i}"), &t, &r, 1, thr);
            }
        }
    }
}

/// Row 16 — `bins == 2`.
#[test]
fn row16_match_bins_two() {
    let mut rng = Rng::new(0x2003);
    for &thr in &[-0.0f64, 0.0, 0.25, 1.0, 1.5] {
        for shape in ALL_SHAPE64 {
            for i in 0..60 {
                let t = gen64(shape, 2, &mut rng);
                let r = gen64(shape, 2, &mut rng);
                diff_match(&format!("row16 thr={thr} {shape:?} #{i}"), &t, &r, 2, thr);
            }
        }
    }
}

/// Row 17 — `3 <= bins <= 15`: every `smoothen` window is truncated.
#[test]
fn row17_match_bins_3_to_15() {
    let mut rng = Rng::new(0x2004);
    for n in 3..=15usize {
        for i in 0..80 {
            let t = gen64(Shape64::Unit, n, &mut rng);
            let r = gen64(Shape64::Unit, n, &mut rng);
            diff_match(&format!("row17 n={n} #{i}"), &t, &r, n as c_int, 0.25);
            diff_match(&format!("row17b n={n} #{i}"), &t, &r, n as c_int, 0.9);
        }
    }
}

/// Row 18 — `bins == 16`: exactly one full window.
#[test]
fn row18_match_bins_16() {
    let mut rng = Rng::new(0x2005);
    for i in 0..400 {
        let t = gen64(Shape64::Unit, 16, &mut rng);
        let r = gen64(Shape64::Unit, 16, &mut rng);
        diff_match(&format!("row18 #{i}"), &t, &r, 16, 0.25);
    }
}

/// Row 19 — `bins == 17`.
#[test]
fn row19_match_bins_17() {
    let mut rng = Rng::new(0x2006);
    for i in 0..400 {
        let t = gen64(Shape64::Unit, 17, &mut rng);
        let r = gen64(Shape64::Unit, 17, &mut rng);
        diff_match(&format!("row19 #{i}"), &t, &r, 17, 0.25);
    }
}

/// Row 20 — `bins` around `2 * N_SMOOTH`.
#[test]
fn row20_match_bins_31_33() {
    let mut rng = Rng::new(0x2007);
    for n in [31usize, 32, 33] {
        for i in 0..200 {
            let t = gen64(Shape64::Unit, n, &mut rng);
            let r = gen64(Shape64::Unit, n, &mut rng);
            diff_match(&format!("row20 n={n} #{i}"), &t, &r, n as c_int, 0.25);
        }
    }
}

/// Row 21 — large `bins`.
#[test]
fn row21_match_bins_large() {
    let mut rng = Rng::new(0x2008);
    for n in [64usize, 128, 1000, 4096] {
        for i in 0..30 {
            let t = gen64(Shape64::Unit, n, &mut rng);
            let r = gen64(Shape64::Unit, n, &mut rng);
            diff_match(&format!("row21 n={n} #{i}"), &t, &r, n as c_int, 0.25);
        }
    }
}

/// Row 22 — mixed-sign doubles; the gate can reject on negative totals.
#[test]
fn row22_match_mixed_sign() {
    let mut rng = Rng::new(0x2009);
    for n in [64usize, 128] {
        for &thr in &[-1.5f64, -0.0, 0.0, 0.25, 1.5] {
            for i in 0..40 {
                let t = gen64(Shape64::Signed, n, &mut rng);
                let r = gen64(Shape64::Signed, n, &mut rng);
                diff_match(&format!("row22 n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 23 — exact integers / powers of two: the `float` reinterpretation of the
/// low half of each `double` is `±0`, so `magnitude == 0`.
#[test]
fn row23_match_zero_float_lanes() {
    let mut rng = Rng::new(0x200A);
    for shape in [Shape64::SmallInts, Shape64::PowersOfTwo] {
        for n in [1usize, 2, 5, 16, 17, 33, 64] {
            for i in 0..60 {
                let t = gen64(shape, n, &mut rng);
                let r = gen64(shape, n, &mut rng);
                for &thr in &[0.0f64, 0.25, 1.0] {
                    diff_match(
                        &format!("row23 {shape:?} n={n} thr={thr} #{i}"),
                        &t,
                        &r,
                        n as c_int,
                        thr,
                    );
                }
            }
        }
    }
}

/// Row 24 — constant vectors: `differentiate` yields all zeros.
#[test]
fn row24_match_constant() {
    let mut rng = Rng::new(0x200B);
    for n in [1usize, 2, 16, 17, 40] {
        for i in 0..80 {
            let t = gen64(Shape64::Constant, n, &mut rng);
            let r = gen64(Shape64::Constant, n, &mut rng);
            for &thr in &[0.0f64, 0.5, 1.0] {
                diff_match(&format!("row24 n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 25 — `±0.0` mixtures at the `0 < 0` gate boundary.
#[test]
fn row25_match_zeros() {
    let mut rng = Rng::new(0x200C);
    for n in [1usize, 2, 16, 40] {
        for i in 0..100 {
            let t = gen64(Shape64::Zeros, n, &mut rng);
            let r = gen64(Shape64::Zeros, n, &mut rng);
            for &thr in &[-0.0f64, 0.0, 1e-12, 1.0] {
                diff_match(&format!("row25 n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 26 — denormal and `1e±300` magnitudes.
#[test]
fn row26_match_extremes() {
    let mut rng = Rng::new(0x200D);
    for n in [1usize, 2, 16, 17, 40] {
        for i in 0..100 {
            let t = gen64(Shape64::Extremes, n, &mut rng);
            let r = gen64(Shape64::Extremes, n, &mut rng);
            for &thr in &[0.0f64, 0.25, 1.0, 1e12] {
                diff_match(&format!("row26 n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 27 — infinities: `inf - inf` in `differentiate`, `inf * 0` in the gate.
#[test]
fn row27_match_infinities() {
    let mut rng = Rng::new(0x200E);
    for n in [1usize, 2, 3, 16, 17, 40] {
        for i in 0..100 {
            let t = gen64(Shape64::Infinities, n, &mut rng);
            let r = gen64(Shape64::Infinities, n, &mut rng);
            for &thr in &[0.0f64, 0.25, 1.0, f64::INFINITY, f64::NEG_INFINITY] {
                diff_match(&format!("row27 n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 28 — NaN payloads in `test` and/or `reference`.
#[test]
fn row28_match_nan_payloads() {
    let mut rng = Rng::new(0x200F);
    // exhaustive single-bin payload pairs
    for &pa in F64_NANS.iter() {
        for &pb in F64_NANS.iter() {
            let t = vec![f64::from_bits(pa)];
            let r = vec![f64::from_bits(pb)];
            for &thr in &[0.0f64, 0.25, 1.0] {
                diff_match(&format!("row28 pair 0x{pa:016X}/0x{pb:016X} thr={thr}"), &t, &r, 1, thr);
            }
            let t2 = vec![f64::from_bits(pa), 1.5, f64::from_bits(pb)];
            let r2 = vec![2.5, f64::from_bits(pb), f64::from_bits(pa)];
            diff_match(&format!("row28 tri 0x{pa:016X}/0x{pb:016X}"), &t2, &r2, 3, 0.25);
        }
    }
    for n in [1usize, 2, 3, 16, 17, 40] {
        for i in 0..120 {
            let t = gen64(Shape64::Nans, n, &mut rng);
            let r = gen64(Shape64::Nans, n, &mut rng);
            for &thr in &[0.0f64, 0.25, 1.0] {
                diff_match(&format!("row28 rand n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 29 — monotone ramp (the intended spectral use).
#[test]
fn row29_match_ramp() {
    let mut rng = Rng::new(0x2010);
    for n in [1usize, 2, 16, 17, 64, 256] {
        for i in 0..60 {
            let t = gen64(Shape64::Ramp, n, &mut rng);
            let r = gen64(Shape64::Ramp, n, &mut rng);
            for &thr in &[0.25f64, 0.9, 1.0] {
                diff_match(&format!("row29 n={n} thr={thr} #{i}"), &t, &r, n as c_int, thr);
            }
        }
    }
}

/// Row 30 — impulse (single non-zero bin).
#[test]
fn row30_match_impulse() {
    let mut rng = Rng::new(0x2011);
    for n in [1usize, 2, 16, 17, 64] {
        for i in 0..80 {
            let t = gen64(Shape64::Impulse, n, &mut rng);
            let r = gen64(Shape64::Impulse, n, &mut rng);
            diff_match(&format!("row30 n={n} #{i}"), &t, &r, n as c_int, 0.25);
        }
    }
}

/// Row 31 — `test == reference` (aliased) across every threshold.
#[test]
fn row31_match_aliased() {
    let mut rng = Rng::new(0x2012);
    let mut ths: Vec<f64> = THRESHOLDS.to_vec();
    ths.push(nan_threshold());
    for shape in ALL_SHAPE64 {
        for n in [1usize, 2, 16, 17, 40] {
            for &thr in &ths {
                let d = gen64(shape, n, &mut rng);
                diff_match_aliased(
                    &format!("row31 {shape:?} n={n} thr={thr:e}"),
                    &d,
                    n as c_int,
                    thr,
                );
            }
        }
    }
}

/// Row 32 — `threshold = ±inf`.
#[test]
fn row32_match_infinite_threshold() {
    let mut rng = Rng::new(0x2013);
    for &thr in &[f64::INFINITY, f64::NEG_INFINITY] {
        for shape in ALL_SHAPE64 {
            for n in [1usize, 2, 16, 17, 40] {
                for i in 0..20 {
                    let t = gen64(shape, n, &mut rng);
                    let r = gen64(shape, n, &mut rng);
                    diff_match(
                        &format!("row32 thr={thr} {shape:?} n={n} #{i}"),
                        &t,
                        &r,
                        n as c_int,
                        thr,
                    );
                }
            }
        }
    }
}

/// Row 33 — `threshold = NaN` (both compares unordered).
#[test]
fn row33_match_nan_threshold() {
    let mut rng = Rng::new(0x2014);
    let nans: Vec<f64> = F64_NANS.iter().map(|&b| f64::from_bits(b)).collect();
    for &thr in &nans {
        for shape in ALL_SHAPE64 {
            for n in [1usize, 2, 16, 17, 40] {
                for i in 0..10 {
                    let t = gen64(shape, n, &mut rng);
                    let r = gen64(shape, n, &mut rng);
                    diff_match(
                        &format!("row33 thr=0x{:016X} {shape:?} n={n} #{i}", thr.to_bits()),
                        &t,
                        &r,
                        n as c_int,
                        thr,
                    );
                }
            }
        }
    }
}

/// Row 34 — the un-pruned fuzz row: every axis randomised simultaneously.
#[test]
fn row34_match_fuzz_all_axes() {
    let mut rng = Rng::new(0x2015);
    let mut ths: Vec<f64> = THRESHOLDS.to_vec();
    ths.push(nan_threshold());
    for &b in F64_NANS.iter() {
        ths.push(f64::from_bits(b));
    }
    for i in 0..3000 {
        let n = 1 + rng.range(40);
        let sa = ALL_SHAPE64[rng.range(ALL_SHAPE64.len())];
        let sb = ALL_SHAPE64[rng.range(ALL_SHAPE64.len())];
        let t = gen64(sa, n, &mut rng);
        let r = gen64(sb, n, &mut rng);
        let thr = if rng.range(8) == 0 {
            f64::from_bits(rng.next_u64())
        } else {
            ths[rng.range(ths.len())]
        };
        diff_match(
            &format!("row34 #{i} n={n} {sa:?}/{sb:?} thr=0x{:016X}", thr.to_bits()),
            &t,
            &r,
            n as c_int,
            thr,
        );
    }
}

/// Row 34b — the same fuzz for the low-level entry point.
#[test]
fn row34b_sc_fuzz_all_axes() {
    let mut rng = Rng::new(0x2016);
    for i in 0..3000 {
        let n = rng.range(41);
        let sa = ALL_SHAPE32[rng.range(ALL_SHAPE32.len())];
        let sb = ALL_SHAPE32[rng.range(ALL_SHAPE32.len())];
        let a = gen32(sa, n, &mut rng);
        let b = gen32(sb, n, &mut rng);
        diff_sc(&format!("row34b #{i} n={n} {sa:?}/{sb:?}"), &a, &b, n as c_int);
    }
}

/// Row 35 — pipeline consistency: run `match` on random data, then feed the
/// *same* `float`-reinterpreted view that `match` internally hands to
/// `spectral_contrast` into the low-level entry point directly, and require the
/// two libraries to agree on both. This exercises the composed path and the
/// standalone path over identical bytes.
#[test]
fn row35_pipeline_consistency() {
    let mut rng = Rng::new(0x2017);
    for i in 0..500 {
        let n = 1 + rng.range(40);
        let shape = ALL_SHAPE64[rng.range(ALL_SHAPE64.len())];
        let t = gen64(shape, n, &mut rng);
        let r = gen64(shape, n, &mut rng);
        diff_match(&format!("row35 match #{i} n={n} {shape:?}"), &t, &r, n as c_int, 0.25);

        // The `float` view `match` hands to `spectral_contrast` is the raw byte
        // reinterpretation of the `double` buffer: lane `2k` is the *low* half
        // of `double` `k` and lane `2k+1` is its *high* half. `bins` lanes
        // therefore cover only the first `ceil(bins/2)` doubles. Feed exactly
        // that lane sequence to the standalone entry point.
        let lanes = |v: &[f64]| -> Vec<f32> {
            let mut out = Vec::with_capacity(v.len());
            for x in v {
                let bits = x.to_bits();
                out.push(f32::from_bits(bits as u32));
                out.push(f32::from_bits((bits >> 32) as u32));
            }
            out.truncate(v.len());
            out
        };
        let lanes_t = lanes(&t);
        let lanes_r = lanes(&r);
        diff_sc(&format!("row35 sc #{i} n={n} {shape:?}"), &lanes_t, &lanes_r, n as c_int);
    }
}

/// Row 36 — optional deep fuzz. Runs `FUZZ_ITERS` iterations (default 0, i.e.
/// skipped) with a seed from `FUZZ_SEED`, randomising every axis of both entry
/// points at once. Used to confirm the enumerated rows are not hiding a gap:
///
/// ```sh
/// FUZZ_ITERS=400000 cargo test --release --test differential row36 -- --nocapture
/// ```
#[test]
fn row36_deep_fuzz() {
    let iters: usize =
        std::env::var("FUZZ_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    if iters == 0 {
        eprintln!("row36: skipped (set FUZZ_ITERS to enable)");
        return;
    }
    let seed: u64 =
        std::env::var("FUZZ_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0xDEAD_BEEF);
    let mut rng = Rng::new(seed);
    let mut ths: Vec<f64> = THRESHOLDS.to_vec();
    ths.push(nan_threshold());
    for &b in F64_NANS.iter() {
        ths.push(f64::from_bits(b));
    }
    for i in 0..iters {
        // low-level entry point
        let n = rng.range(24);
        let sa = ALL_SHAPE32[rng.range(ALL_SHAPE32.len())];
        let sb = ALL_SHAPE32[rng.range(ALL_SHAPE32.len())];
        let a = gen32(sa, n, &mut rng);
        let b = gen32(sb, n, &mut rng);
        if rng.bool() {
            diff_sc(&format!("row36 sc #{i} n={n} {sa:?}/{sb:?}"), &a, &b, n as c_int);
        } else {
            diff_sc_aliased(&format!("row36 sca #{i} n={n} {sa:?}"), &a, n as c_int);
        }

        // composed pipeline
        let m = 1 + rng.range(24);
        let ta = ALL_SHAPE64[rng.range(ALL_SHAPE64.len())];
        let tb = ALL_SHAPE64[rng.range(ALL_SHAPE64.len())];
        let t = gen64(ta, m, &mut rng);
        let r = gen64(tb, m, &mut rng);
        let thr = if rng.range(4) == 0 {
            f64::from_bits(rng.next_u64())
        } else {
            ths[rng.range(ths.len())]
        };
        let ctx = format!("row36 match #{i} m={m} {ta:?}/{tb:?} thr=0x{:016X}", thr.to_bits());
        if rng.range(8) == 0 {
            diff_match_aliased(&ctx, &t, m as c_int, thr);
        } else {
            diff_match(&ctx, &t, &r, m as c_int, thr);
        }
    }
    eprintln!("row36: {iters} iterations, seed 0x{seed:X}, no divergence");
}
