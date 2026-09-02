//! Phase B — valid-path differential tests, one `#[test]` per row of
//! `CONFIGS.md`.
//!
//! Every test loads both shared objects with `libloading` and calls only their
//! exported symbols. Each row runs `DIFF_ITERS` (default 200) seeded-random
//! inputs and compares return values bit-for-bit and in-place-mutated buffers
//! byte-for-byte.

mod common;

use std::ffi::c_int;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1-14: `spectral_contrast`, the lowest-level public entry point.
// Note its element type is `float` (`<math.h>`'s `float_t`), not `double`.
// ---------------------------------------------------------------------------

/// Fixed `length`, values from `gen`.
fn spectral_fixed_len(seed: u64, len: usize, make: fn(&mut Rng, usize) -> Vec<f32>, row: &str) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for it in 0..iters() {
        let a = make(&mut rng, len);
        let b = make(&mut rng, len);
        diff_spectral(&p, &format!("{row} it={it} len={len}"), &a, &b, len as c_int);
    }
}

/// Random `length` in `1..=64`, values from `gen`.
fn spectral_random_len(seed: u64, make: fn(&mut Rng, usize) -> Vec<f32>, row: &str) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for it in 0..iters() {
        let len = rng.range_usize(1, 64);
        let a = make(&mut rng, len);
        let b = make(&mut rng, len);
        diff_spectral(&p, &format!("{row} it={it} len={len}"), &a, &b, len as c_int);
    }
}

#[test]
fn row01_spectral_len1_unit() {
    spectral_fixed_len(0x0101, 1, gen_unit_f32, "row01");
}

#[test]
fn row02_spectral_len2_signed() {
    spectral_fixed_len(0x0202, 2, gen_signed_f32, "row02");
}

#[test]
fn row03_spectral_len15_signed() {
    spectral_fixed_len(0x0303, 15, gen_signed_f32, "row03");
}

#[test]
fn row04_spectral_len16_signed() {
    spectral_fixed_len(0x0404, 16, gen_signed_f32, "row04");
}

#[test]
fn row05_spectral_len17_signed() {
    spectral_fixed_len(0x0505, 17, gen_signed_f32, "row05");
}

#[test]
fn row06_spectral_len64_signed() {
    spectral_fixed_len(0x0606, 64, gen_signed_f32, "row06");
}

#[test]
fn row07_spectral_len4096_signed() {
    spectral_fixed_len(0x0707, 4096, gen_signed_f32, "row07");
}

#[test]
fn row08_spectral_wide_exponents() {
    spectral_random_len(0x0808, gen_wide_f32, "row08");
}

#[test]
fn row09_spectral_raw_bit_patterns() {
    spectral_random_len(0x0909, gen_raw_f32, "row09");
}

#[test]
fn row10_spectral_nan_inf_sprinkled() {
    spectral_random_len(0x0A0A, gen_specials_f32, "row10");
}

#[test]
fn row11_spectral_all_zeros() {
    // magnitude == 0 -> `v[i] /= 0.0` -> the x86 indefinite QNaN.
    let p = pair();
    let mut rng = Rng::new(0x0B0B);
    for it in 0..iters() {
        let len = rng.range_usize(1, 64);
        let neg = rng.next_u64() & 1 == 1;
        let z = if neg { -0.0f32 } else { 0.0f32 };
        let a = vec![z; len];
        let b = vec![if rng.next_u64() & 1 == 1 { -0.0f32 } else { 0.0f32 }; len];
        diff_spectral(&p, &format!("row11 it={it} len={len} neg={neg}"), &a, &b, len as c_int);
    }
}

#[test]
fn row12_spectral_constant_dc() {
    let p = pair();
    let mut rng = Rng::new(0x0C0C);
    for it in 0..iters() {
        let len = rng.range_usize(1, 64);
        let a = vec![rng.signed() as f32; len];
        let b = vec![rng.signed() as f32; len];
        diff_spectral(&p, &format!("row12 it={it} len={len}"), &a, &b, len as c_int);
    }
}

#[test]
fn row13_spectral_subnormals() {
    spectral_random_len(0x0D0D, gen_subnormal_f32, "row13");
}

#[test]
fn row14_spectral_aliased() {
    let p = pair();
    let mut rng = Rng::new(0x0E0E);
    for it in 0..iters() {
        let len = rng.range_usize(1, 64);
        let v = gen_signed_f32(&mut rng, len);
        diff_spectral_aliased(&p, &format!("row14 it={it} len={len}"), &v, len as c_int);
    }
}

// ---------------------------------------------------------------------------
// Rows 15-35: `match`, the composed pipeline.
// ---------------------------------------------------------------------------

/// Fixed `bins`, unit-uniform data, `threshold` uniform in `[-1, 2]`.
fn match_fixed_bins(seed: u64, bins: usize, row: &str) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for it in 0..iters() {
        let t = gen_unit_f64(&mut rng, bins);
        let r = gen_unit_f64(&mut rng, bins);
        let thr = rng.range_f64(-1.0, 2.0);
        diff_match(
            &p,
            &format!("{row} it={it} bins={bins} thr={thr}"),
            &t,
            &r,
            bins as c_int,
            thr,
        );
    }
}

/// Random `bins` in `1..=40`, values from `gen`, `threshold` uniform in `[-1, 2]`.
fn match_random_bins(seed: u64, make: fn(&mut Rng, usize) -> Vec<f64>, row: &str) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        let t = make(&mut rng, bins);
        let r = make(&mut rng, bins);
        let thr = rng.range_f64(-1.0, 2.0);
        diff_match(
            &p,
            &format!("{row} it={it} bins={bins} thr={thr}"),
            &t,
            &r,
            bins as c_int,
            thr,
        );
    }
}

#[test]
fn row15_match_bins1() {
    match_fixed_bins(0x1515, 1, "row15");
}

#[test]
fn row16_match_bins2() {
    match_fixed_bins(0x1616, 2, "row16");
}

#[test]
fn row17_match_bins15() {
    match_fixed_bins(0x1717, 15, "row17");
}

#[test]
fn row18_match_bins16() {
    match_fixed_bins(0x1818, 16, "row18");
}

#[test]
fn row19_match_bins17() {
    match_fixed_bins(0x1919, 17, "row19");
}

#[test]
fn row20_match_bins32_and_33() {
    match_fixed_bins(0x2020, 32, "row20a");
    match_fixed_bins(0x2021, 33, "row20b");
}

#[test]
fn row21_match_bins4096() {
    match_fixed_bins(0x2121, 4096, "row21");
}

#[test]
fn row22_match_bins_100000() {
    // Two 100000-element `double` VLAs = 1.6 MiB of stack inside the C `match`,
    // more than a default libtest thread allows, so run on a roomy stack.
    let handle = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let p = pair();
            let mut rng = Rng::new(0x2222);
            let bins = 100_000usize;
            // Fewer iterations: each one is 200k elements through the pipeline.
            let n = (iters() / 40).clamp(3, 20);
            for it in 0..n {
                let t = gen_unit_f64(&mut rng, bins);
                let r = gen_unit_f64(&mut rng, bins);
                let thr = rng.range_f64(-1.0, 2.0);
                diff_match(
                    &p,
                    &format!("row22 it={it} bins={bins} thr={thr}"),
                    &t,
                    &r,
                    bins as c_int,
                    thr,
                );
            }
        })
        .expect("spawn big-stack thread");
    handle.join().expect("row22 worker panicked");
}

#[test]
fn row23_match_signed_values() {
    match_random_bins(0x2323, gen_signed_f64, "row23");
}

#[test]
fn row24_match_raw_bit_patterns() {
    match_random_bins(0x2424, gen_raw_f64, "row24");
}

#[test]
fn row25_match_nan_inf_sprinkled() {
    match_random_bins(0x2525, gen_specials_f64, "row25");
}

#[test]
fn row26_match_wide_exponents() {
    match_random_bins(0x2626, gen_wide_f64, "row26");
}

#[test]
fn row27_match_constant_dc() {
    match_random_bins(0x2727, gen_dc_f64, "row27");
}

#[test]
fn row28_match_all_zeros() {
    let p = pair();
    let mut rng = Rng::new(0x2828);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        let t = vec![if rng.next_u64() & 1 == 1 { -0.0 } else { 0.0 }; bins];
        let r = vec![if rng.next_u64() & 1 == 1 { -0.0 } else { 0.0 }; bins];
        for &thr in SPECIAL_THRESHOLDS {
            diff_match(
                &p,
                &format!("row28 it={it} bins={bins} thr={thr:?}"),
                &t,
                &r,
                bins as c_int,
                thr,
            );
        }
    }
}

#[test]
fn row29_match_linear_ramp() {
    match_random_bins(0x2929, gen_ramp_f64, "row29");
}

#[test]
fn row30_match_subnormals() {
    match_random_bins(0x3030, gen_subnormal_f64, "row30");
}

#[test]
fn row31_match_aliased() {
    let p = pair();
    let mut rng = Rng::new(0x3131);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        let v = gen_signed_f64(&mut rng, bins);
        let thr = rng.range_f64(-1.0, 2.0);
        diff_match_aliased(&p, &format!("row31 it={it} bins={bins} thr={thr}"), &v, bins as c_int, thr);
    }
}

#[test]
fn row32_match_special_thresholds() {
    let p = pair();
    let mut rng = Rng::new(0x3232);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        // Rotate through the value shapes so the special thresholds meet
        // ordinary, signed, special and raw data alike.
        let make: fn(&mut Rng, usize) -> Vec<f64> = match it % 4 {
            0 => gen_unit_f64,
            1 => gen_signed_f64,
            2 => gen_specials_f64,
            _ => gen_raw_f64,
        };
        let t = make(&mut rng, bins);
        let r = make(&mut rng, bins);
        for &thr in SPECIAL_THRESHOLDS {
            diff_match(
                &p,
                &format!("row32 it={it} bins={bins} thr={thr:?}"),
                &t,
                &r,
                bins as c_int,
                thr,
            );
        }
    }
}

#[test]
fn row33_match_threshold_on_energy_gate_boundary() {
    // Drive `total(test) < threshold * total(reference)` to its exact tipping
    // point, then step one ULP either side, so strict `<` is pinned down.
    let p = pair();
    let mut rng = Rng::new(0x3333);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        let t = gen_unit_f64(&mut rng, bins);
        let r = gen_unit_f64(&mut rng, bins);
        let st: f64 = t.iter().sum();
        let sr: f64 = r.iter().sum();
        let exact = st / sr;
        let mut candidates = vec![exact];
        for k in 1..=3i64 {
            candidates.push(f64::from_bits(exact.to_bits().wrapping_add(k as u64)));
            candidates.push(f64::from_bits(exact.to_bits().wrapping_sub(k as u64)));
        }
        for thr in candidates {
            diff_match(
                &p,
                &format!("row33 it={it} bins={bins} thr={thr:?}"),
                &t,
                &r,
                bins as c_int,
                thr,
            );
        }
    }
}

#[test]
fn row34_match_one_side_zero() {
    let p = pair();
    let mut rng = Rng::new(0x3434);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        let data = gen_signed_f64(&mut rng, bins);
        let zeros = vec![0.0f64; bins];
        for &thr in SPECIAL_THRESHOLDS {
            diff_match(
                &p,
                &format!("row34a it={it} bins={bins} thr={thr:?}"),
                &data,
                &zeros,
                bins as c_int,
                thr,
            );
            diff_match(
                &p,
                &format!("row34b it={it} bins={bins} thr={thr:?}"),
                &zeros,
                &data,
                bins as c_int,
                thr,
            );
        }
    }
}

#[test]
fn row35_match_seam_matches_low_level_spectral_contrast() {
    // Reproduce `match`'s pipeline by hand and cross the seam explicitly:
    // preprocess in `double`, then hand the buffer to the *low-level*
    // `spectral_contrast` export reinterpreted as `float`, exactly as the C
    // `match` does through its PLT. Both `.so`s must agree at that seam, and
    // the seam result must be consistent with each library's own `match`.
    let p = pair();
    let mut rng = Rng::new(0x3535);
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        let t = gen_unit_f64(&mut rng, bins);
        let r = gen_unit_f64(&mut rng, bins);

        let mut tp = preprocess_ref(&t);
        let mut rp = preprocess_ref(&r);
        // `bins` f32 lanes live in the low 4*bins bytes of the f64 buffers.
        let ctx = format!("row35 it={it} bins={bins}");
        let (ta, tb) = (as_f32_lanes(&mut tp, bins), as_f32_lanes(&mut rp, bins));
        diff_spectral(&p, &ctx, &ta, &tb, bins as c_int);

        // The contrast value the seam produces must decide `match`'s result.
        let mut ca = ta.clone();
        let mut cb = tb.clone();
        let contrast =
            unsafe { (p.c.spectral_contrast)(ca.as_mut_ptr(), cb.as_mut_ptr(), bins as c_int) };
        let thr = if rng.next_u64() & 1 == 1 {
            contrast
        } else {
            f64::from_bits(contrast.to_bits().wrapping_sub(1))
        };
        diff_match(&p, &format!("{ctx} thr={thr:?}"), &t, &r, bins as c_int, thr);
    }
}
