//! Phase B -- valid-path differential tests.
//!
//! One `#[test]` per row of CONFIGS.md, in the same order. Every row drives
//! BOTH shared objects through `libloading` with many randomized inputs
//! (fixed root seed) and asserts bit-identical results.

mod common;

use common::*;

// ===========================================================================
// spectral_contrast -- the lowest-level public entry point, driven directly.
// ===========================================================================

/// Helper: sweep `lengths` x `n` random draws of `shape` for both buffers.
fn sweep_sc(row: &str, shape: F32Shape, lengths: &[i32], n: usize) {
    for (li, &len) in lengths.iter().enumerate() {
        for k in 0..n {
            let mut rng = Rng::new((row.len() as u64) << 40 ^ (li as u64) << 20 ^ k as u64);
            let a = gen_f32_bits(shape, len as usize, &mut rng);
            let b = gen_f32_bits(shape, len as usize, &mut rng);
            diff_sc(&a, &b, len, &format!("{row} shape={shape:?} draw={k}"));
        }
    }
}

#[test]
fn cfg_row01_sc_len1_normal() {
    sweep_sc("row01", F32Shape::Normal, &[1], 512);
}

#[test]
fn cfg_row02_sc_len2_normal() {
    sweep_sc("row02", F32Shape::Normal, &[2], 512);
}

#[test]
fn cfg_row03_sc_len3_to_8_normal() {
    sweep_sc("row03", F32Shape::Normal, &[3, 4, 5, 6, 7, 8], 256);
}

#[test]
fn cfg_row04_sc_nsmooth_neighbourhood() {
    sweep_sc("row04", F32Shape::Normal, &[15, 16, 17], 256);
}

#[test]
fn cfg_row05_sc_len31_32_33_64() {
    sweep_sc("row05", F32Shape::Normal, &[31, 32, 33, 64], 128);
}

#[test]
fn cfg_row06_sc_long_vectors() {
    sweep_sc("row06", F32Shape::Normal, &[255, 256, 1024], 32);
}

#[test]
fn cfg_row07_sc_raw_bit_patterns() {
    sweep_sc("row07", F32Shape::RawBits, &[1, 2, 3, 7, 16, 17, 64], 512);
}

#[test]
fn cfg_row08_sc_one_side_all_zero() {
    for len in [1, 2, 3, 16, 17, 33] {
        for k in 0..64 {
            let mut rng = Rng::new(0x0800 ^ (len as u64) << 16 ^ k);
            let zeros = gen_f32_bits(F32Shape::PosZeros, len as usize, &mut rng);
            let other = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
            diff_sc(&zeros, &other, len, "row08 a=zeros");
            diff_sc(&other, &zeros, len, "row08 b=zeros");
        }
    }
}

#[test]
fn cfg_row09_sc_both_all_zero() {
    for len in [1, 2, 16, 17] {
        let mut rng = Rng::new(0x0900 ^ len as u64);
        let pz = gen_f32_bits(F32Shape::PosZeros, len as usize, &mut rng);
        let nz = gen_f32_bits(F32Shape::NegZeros, len as usize, &mut rng);
        diff_sc(&pz, &pz, len, "row09 +0/+0");
        diff_sc(&nz, &nz, len, "row09 -0/-0");
        diff_sc(&pz, &nz, len, "row09 +0/-0");
        diff_sc(&nz, &pz, len, "row09 -0/+0");
    }
}

#[test]
fn cfg_row10_sc_denormal_inputs() {
    sweep_sc("row10", F32Shape::Denormal, &[1, 2, 3, 16, 17, 64], 256);
}

#[test]
fn cfg_row11_sc_huge_inputs_overflow() {
    sweep_sc("row11", F32Shape::Huge, &[1, 2, 3, 16, 17, 64], 256);
}

#[test]
fn cfg_row12_sc_tiny_inputs_underflow() {
    sweep_sc("row12", F32Shape::Tiny, &[1, 2, 3, 16, 17, 64], 256);
}

#[test]
fn cfg_row13_sc_signed_zero_mixture() {
    sweep_sc("row13", F32Shape::SignedZeros, &[1, 2, 3, 16, 17], 256);
}

#[test]
fn cfg_row14_sc_aliased_same_pointer() {
    for shape in [F32Shape::Normal, F32Shape::RawBits, F32Shape::Denormal, F32Shape::Huge] {
        for len in [1, 2, 3, 16, 17, 64] {
            for k in 0..128 {
                let mut rng = Rng::new(0x1400 ^ (len as u64) << 16 ^ k);
                let a = gen_f32_bits(shape, len as usize, &mut rng);
                diff_sc_aliased(&a, len, &format!("row14 shape={shape:?}"));
            }
        }
    }
}

#[test]
fn cfg_row15_sc_identical_content_distinct_buffers() {
    for shape in [F32Shape::Normal, F32Shape::RawBits, F32Shape::Denormal] {
        for len in [1, 2, 3, 16, 17, 64] {
            for k in 0..128 {
                let mut rng = Rng::new(0x1500 ^ (len as u64) << 16 ^ k);
                let a = gen_f32_bits(shape, len as usize, &mut rng);
                diff_sc(&a, &a, len, &format!("row15 shape={shape:?}"));
            }
        }
    }
}

#[test]
fn cfg_row16_sc_anti_parallel() {
    for len in [1, 2, 3, 16, 17, 64] {
        for k in 0..128 {
            let mut rng = Rng::new(0x1600 ^ (len as u64) << 16 ^ k);
            let a = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
            let b: Vec<u32> = a.iter().map(|&x| x ^ 0x8000_0000).collect();
            diff_sc(&a, &b, len, "row16 b=-a");
        }
    }
}

#[test]
fn cfg_row17_sc_infinities_mixed() {
    sweep_sc("row17", F32Shape::WithInf, &[1, 2, 3, 16, 17, 64], 256);
}

#[test]
fn cfg_row18_sc_nan_patterns_mixed() {
    sweep_sc("row18", F32Shape::WithNan, &[1, 2, 3, 16, 17, 64], 256);
}

// ===========================================================================
// match -- the composed pipeline.
// ===========================================================================

/// Helper: sweep `bins` x `thresholds` x `n` random draws of `shape`.
fn sweep_match(row: &str, shape: F64Shape, bins_list: &[i32], thresholds: &[f64], n: usize) {
    for (bi, &bins) in bins_list.iter().enumerate() {
        for k in 0..n {
            let mut rng = Rng::new((row.len() as u64) << 44 ^ (bi as u64) << 24 ^ k as u64);
            let t = gen_f64_bits(shape, bins as usize, &mut rng);
            let r = gen_f64_bits(shape, bins as usize, &mut rng);
            for &th in thresholds {
                diff_match(&t, &r, bins, th, &format!("{row} shape={shape:?} draw={k}"));
            }
        }
    }
}

#[test]
fn cfg_row19_match_bins1_threshold_sweep() {
    // bins==1: differentiate() forces v[0]=0, so preprocess yields an all-zero
    // vector, the magnitude is 0 and the contrast is NaN.
    sweep_match("row19", F64Shape::Positive, &[1], THRESHOLDS, 256);
    sweep_match("row19s", F64Shape::Signed, &[1], THRESHOLDS, 128);
}

#[test]
fn cfg_row20_match_bins2_positive() {
    sweep_match("row20", F64Shape::Positive, &[2], &[0.5], 512);
}

#[test]
fn cfg_row21_match_bins3_to_8() {
    sweep_match("row21", F64Shape::Positive, &[3, 4, 5, 6, 7, 8], &[0.5], 256);
}

#[test]
fn cfg_row22_match_nsmooth_boundary() {
    sweep_match("row22", F64Shape::Positive, &[15, 16, 17], &[0.25, 0.5, 1.0], 256);
    sweep_match("row22s", F64Shape::Signed, &[15, 16, 17], &[0.5], 128);
}

#[test]
fn cfg_row23_match_bins31_32_33() {
    sweep_match("row23", F64Shape::Positive, &[31, 32, 33], &[0.5, 1.0], 128);
}

#[test]
fn cfg_row24_match_long_vectors() {
    sweep_match("row24", F64Shape::Positive, &[63, 64, 128, 257, 1024], &[0.5], 32);
    // Larger still: two `bins`-element VLAs must stay within the thread stack
    // (16384 * 8 * 2 = 256 KiB), and the accumulation order over long vectors
    // must match.
    sweep_match("row24big", F64Shape::Peaked, &[4096, 16384], &[0.25, 0.5, 1.0], 4);
    for bins in [4096i32, 16384] {
        let mut rng = Rng::new(0x24B1 ^ bins as u64);
        let t = gen_f64_bits(F64Shape::Peaked, bins as usize, &mut rng);
        let r = gen_f64_bits(F64Shape::Peaked, bins as usize, &mut rng);
        diff_match_boundary(&t, &r, bins, "row24big boundary");
    }
}

#[test]
fn cfg_row25_match_odd_vs_even_bins() {
    // The float/double reinterpretation makes the *parity* of `bins` a real
    // branch: an odd `bins` reads a trailing low half-double only.
    for k in 0..256 {
        let mut rng = Rng::new(0x2500 ^ k);
        let t = gen_f64_bits(F64Shape::Positive, 18, &mut rng);
        let r = gen_f64_bits(F64Shape::Positive, 18, &mut rng);
        for bins in [5, 6, 17, 18] {
            for &th in &[0.0, 0.5, 1.0] {
                diff_match(&t[..bins as usize], &r[..bins as usize], bins, th, "row25 parity");
            }
        }
    }
}

#[test]
fn cfg_row26_match_full_threshold_sweep() {
    for shape in [F64Shape::Positive, F64Shape::Signed] {
        for bins in [1, 2, 16, 17] {
            for k in 0..64 {
                let mut rng = Rng::new(0x2600 ^ (bins as u64) << 20 ^ k);
                let t = gen_f64_bits(shape, bins as usize, &mut rng);
                let r = gen_f64_bits(shape, bins as usize, &mut rng);
                for &th in THRESHOLDS {
                    diff_match(&t, &r, bins, th, &format!("row26 shape={shape:?}"));
                }
            }
        }
    }
}

#[test]
fn cfg_row27_match_aliased_same_pointer() {
    for shape in [F64Shape::Positive, F64Shape::RawBits, F64Shape::Ramp] {
        for bins in [1, 2, 3, 16, 17] {
            for k in 0..128 {
                let mut rng = Rng::new(0x2700 ^ (bins as u64) << 20 ^ k);
                let v = gen_f64_bits(shape, bins as usize, &mut rng);
                for &th in &[0.0, 0.5, 1.0, 2.0] {
                    diff_match_aliased(&v, bins, th, &format!("row27 shape={shape:?}"));
                }
            }
        }
    }
}

#[test]
fn cfg_row28_match_identical_content() {
    for shape in [F64Shape::Positive, F64Shape::RawBits] {
        for bins in [2, 3, 16, 17] {
            for k in 0..128 {
                let mut rng = Rng::new(0x2800 ^ (bins as u64) << 20 ^ k);
                let v = gen_f64_bits(shape, bins as usize, &mut rng);
                for &th in &[0.5, 1.0] {
                    diff_match(&v, &v, bins, th, &format!("row28 shape={shape:?}"));
                }
            }
        }
    }
}

#[test]
fn cfg_row29_match_zero_test_vs_positive_reference() {
    for bins in [1, 2, 3, 16, 17, 33] {
        for k in 0..64 {
            let mut rng = Rng::new(0x2900 ^ (bins as u64) << 20 ^ k);
            let t = gen_f64_bits(F64Shape::Zeros, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            for &th in THRESHOLDS {
                diff_match(&t, &r, bins, th, "row29 gate-rejects");
            }
        }
    }
}

#[test]
fn cfg_row30_match_zero_reference_vs_positive_test() {
    for bins in [1, 2, 3, 16, 17, 33] {
        for k in 0..64 {
            let mut rng = Rng::new(0x3000 ^ (bins as u64) << 20 ^ k);
            let t = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Zeros, bins as usize, &mut rng);
            for &th in THRESHOLDS {
                diff_match(&t, &r, bins, th, "row30 zero-reference");
            }
        }
    }
}

#[test]
fn cfg_row31_match_ramps() {
    for bins in [1, 2, 3, 16, 17, 33, 64] {
        let mut rng = Rng::new(0x3100 ^ bins as u64);
        let up = gen_f64_bits(F64Shape::Ramp, bins as usize, &mut rng);
        let down = gen_f64_bits(F64Shape::RampDown, bins as usize, &mut rng);
        for &th in THRESHOLDS {
            diff_match(&up, &down, bins, th, "row31 ramp/rampdown");
            diff_match(&down, &up, bins, th, "row31 rampdown/ramp");
            diff_match(&up, &up, bins, th, "row31 ramp/ramp");
        }
    }
}

#[test]
fn cfg_row32_match_peaked_spectrum() {
    sweep_match("row32", F64Shape::Peaked, &[16, 17, 64, 257], &[0.25, 0.5, 0.9, 1.0], 64);
}

#[test]
fn cfg_row33_match_denormal_doubles() {
    sweep_match("row33", F64Shape::Denormal, &[2, 3, 16, 17], &[0.0, 0.5, 1.0], 256);
}

#[test]
fn cfg_row34_match_huge_doubles() {
    sweep_match("row34", F64Shape::Huge, &[2, 16, 17], &[0.0, 0.5, 1.0, 2.0], 256);
}

#[test]
fn cfg_row35_match_inf_nan_data() {
    sweep_match("row35", F64Shape::InfNan, &[2, 3, 16, 17], &[0.0, 0.5, 1.0], 256);
}

#[test]
fn cfg_row36_match_raw_bit_patterns() {
    sweep_match("row36", F64Shape::RawBits, &[1, 2, 3, 16, 17, 33], &[0.5], 512);
    sweep_match("row36t", F64Shape::RawBits, &[2, 17], THRESHOLDS, 64);
}

#[test]
fn cfg_row37_match_negative_data() {
    sweep_match("row37", F64Shape::Negative, &[2, 3, 16, 17], THRESHOLDS, 64);
}

#[test]
fn cfg_row38_match_scaled_copy() {
    for bins in [2, 3, 16, 17, 64] {
        for k in 0..128 {
            let mut rng = Rng::new(0x3800 ^ (bins as u64) << 20 ^ k);
            let r_bits = gen_f64_bits(F64Shape::Peaked, bins as usize, &mut rng);
            for scale in [0.25f64, 0.5, 1.0, 2.0, 4.0] {
                let t_bits: Vec<u64> = r_bits
                    .iter()
                    .map(|&x| (f64::from_bits(x) * scale).to_bits())
                    .collect();
                for &th in &[0.25f64, 0.5, 0.9, 1.0] {
                    diff_match(&t_bits, &r_bits, bins, th, &format!("row38 scale={scale}"));
                }
            }
        }
    }
}

// Row 39 (input buffers must be unmodified) is asserted inside `diff_match`
// for every single `match` call above, and additionally pinned here.
#[test]
fn cfg_row39_match_does_not_modify_inputs() {
    for bins in [1, 2, 3, 16, 17, 64] {
        for k in 0..64 {
            let mut rng = Rng::new(0x3900 ^ (bins as u64) << 20 ^ k);
            let t = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            // diff_match asserts the C output buffers still equal the inputs,
            // and that C and Rust agree on the (unmodified) buffers.
            for &th in &[0.0, 0.5, 1.0] {
                diff_match(&t, &r, bins, th, "row39 immutability");
            }
        }
    }
}

// Row 40 lives in phase_c.rs (size/pointer boundary values).

// ===========================================================================
// Row 45: the sharp oracle.
//
// `match` only returns one bit, so a wrong internal `spectral_contrast` result
// is invisible unless `threshold` happens to fall between the C's and the
// Rust's value.  `diff_match_boundary` bisects on `threshold` to recover the
// exact decision boundary -- i.e. the full 53-bit
// `min(total(test)/total(reference), contrast)` -- and compares that instead.
// ===========================================================================

#[test]
fn cfg_row45_match_decision_boundary_is_bit_exact() {
    for shape in [
        F64Shape::Positive,
        F64Shape::Peaked,
        F64Shape::Ramp,
        F64Shape::Signed,
        F64Shape::Negative,
        F64Shape::Denormal,
        F64Shape::Huge,
        F64Shape::RawBits,
        F64Shape::InfNan,
        F64Shape::FiniteWithDistinctNans,
    ] {
        for bins in [1, 2, 3, 5, 8, 15, 16, 17, 18, 33, 64] {
            for k in 0..24u64 {
                let mut rng = Rng::new(0x4500 ^ (bins as u64) << 24 ^ k ^ (shape as u64) << 48);
                let t = gen_f64_bits(shape, bins as usize, &mut rng);
                let r = gen_f64_bits(shape, bins as usize, &mut rng);
                diff_match_boundary(&t, &r, bins, &format!("row45 shape={shape:?}"));
                // Also the aliased / identical-content variants.
                diff_match_boundary(&t, &t, bins, &format!("row45 identical shape={shape:?}"));
            }
        }
    }
}

#[test]
fn cfg_row45b_match_boundary_scaled_copies() {
    // The realistic case: same spectrum at a different gain. The boundary is
    // then the contrast itself (the gate ratio is far away), so this compares
    // the internal `spectral_contrast` result to the last bit.
    for bins in [2, 3, 16, 17, 33, 64, 257] {
        for k in 0..32u64 {
            let mut rng = Rng::new(0x45B0 ^ (bins as u64) << 24 ^ k);
            let r = gen_f64_bits(F64Shape::Peaked, bins as usize, &mut rng);
            for scale in [1.0f64, 0.5, 2.0, 8.0, 1.0e6] {
                let t: Vec<u64> =
                    r.iter().map(|&x| (f64::from_bits(x) * scale).to_bits()).collect();
                diff_match_boundary(&t, &r, bins, &format!("row45b scale={scale}"));
            }
        }
    }
}

// ===========================================================================
// Rows 41-44: NaN *payload* propagation order.
//
// When both operands of a commutative SSE op are NaN, x86 keeps SRC1's payload.
// gcc compiles `sum += X` to `addsd %sum, %X` (X is SRC1) and `a[i] * b[i]` to
// `mulss %a[i], %b[i]` (b[i] is SRC1), whereas idiomatic Rust would pick the
// accumulator / the left operand.  These rows use *distinct* payloads per
// element so the surviving payload identifies the operand order.
// ===========================================================================

#[test]
fn cfg_row41_sc_distinct_nan_payloads_pick_correct_operand() {
    // Discriminates `mulss` SRC1 (b[i] vs a[i]) and `addsd` SRC1
    // (product vs sum) inside dot_product.
    for len in [1, 2, 3, 4, 8, 16, 17, 33] {
        for k in 0..256u64 {
            let a: Vec<u32> = (0..len).map(|i| distinct_nan_f32(k as u32 * 97 + i as u32)).collect();
            let b: Vec<u32> =
                (0..len).map(|i| distinct_nan_f32(k as u32 * 31 + 5000 + i as u32)).collect();
            diff_sc(&a, &b, len as i32, "row41 both NaN, distinct payloads");
            // One side finite: `mulss` must fall through to SRC2's payload.
            let mut rng = Rng::new(0x4100 ^ k);
            let fin = gen_f32_bits(F32Shape::Normal, len, &mut rng);
            diff_sc(&a, &fin, len as i32, "row41 a=NaN b=finite");
            diff_sc(&fin, &b, len as i32, "row41 a=finite b=NaN");
        }
    }
    sweep_sc("row41mix", F32Shape::DistinctNans, &[1, 2, 3, 16, 17, 64], 256);
    sweep_sc("row41fin", F32Shape::FiniteWithDistinctNans, &[1, 2, 3, 16, 17, 64], 512);
}

#[test]
fn cfg_row42_match_distinct_nan_payloads_in_smoothen() {
    // `smoothen` stores `sum / 16` back into the array, so the payload that
    // survives its `addsd` chain is observable all the way out through
    // `spectral_contrast`.
    for bins in [1, 2, 3, 4, 8, 15, 16, 17, 18, 33, 64] {
        for k in 0..64u64 {
            let t: Vec<u64> = (0..bins).map(|i| distinct_nan_f64(k * 101 + i as u64)).collect();
            let r: Vec<u64> =
                (0..bins).map(|i| distinct_nan_f64(k * 47 + 900_000 + i as u64)).collect();
            for &th in &[0.0, 0.5, 1.0, f64::NAN] {
                diff_match(&t, &r, bins as i32, th, "row42 all-NaN distinct payloads");
            }
        }
    }
}

#[test]
fn cfg_row43_match_finite_with_distinct_nans() {
    sweep_match(
        "row43",
        F64Shape::FiniteWithDistinctNans,
        &[1, 2, 3, 8, 15, 16, 17, 18, 33],
        &[0.0, 0.5, 1.0],
        256,
    );
    sweep_match("row43all", F64Shape::DistinctNans, &[2, 3, 16, 17], &[0.0, 0.5, 1.0], 256);
}

#[test]
fn cfg_row44_match_single_nan_at_every_position() {
    // A single NaN walking through every index: catches window-dependent
    // payload selection in `smoothen`'s 16-tap sum and in `differentiate`.
    for bins in [1, 2, 3, 16, 17, 18, 33] {
        for pos in 0..bins {
            for k in 0..8u64 {
                let mut rng = Rng::new(0x4400 ^ (bins as u64) << 20 ^ (pos as u64) << 8 ^ k);
                let mut t = gen_f64_bits(F64Shape::Positive, bins, &mut rng);
                let mut r = gen_f64_bits(F64Shape::Positive, bins, &mut rng);
                t[pos] = distinct_nan_f64(pos as u64 + 1);
                r[pos] = distinct_nan_f64(pos as u64 + 12_345);
                for &th in &[0.0, 0.5, 1.0] {
                    diff_match(&t, &r, bins as i32, th, "row44 single NaN position sweep");
                    // also with only one side carrying the NaN
                    let clean_t = gen_f64_bits(F64Shape::Positive, bins, &mut Rng::new(k));
                    diff_match(&clean_t, &r, bins as i32, th, "row44 NaN in reference only");
                    diff_match(&t, &clean_t, bins as i32, th, "row44 NaN in test only");
                }
                t[pos] = f64::INFINITY.to_bits();
                r[pos] = f64::NEG_INFINITY.to_bits();
                diff_match(&t, &r, bins as i32, 0.5, "row44 inf position sweep");
            }
        }
    }
}
