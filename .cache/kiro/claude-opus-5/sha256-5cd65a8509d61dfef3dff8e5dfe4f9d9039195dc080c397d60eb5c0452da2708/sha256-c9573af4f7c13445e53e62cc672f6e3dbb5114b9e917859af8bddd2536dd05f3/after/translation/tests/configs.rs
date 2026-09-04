//! Phase B -- valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both libraries are driven exclusively through their `.so` exports. Every row
//! runs many randomized inputs from a fixed seed; the assertion is bit-for-bit
//! equality of the returned value *and* of every buffer the callee may mutate.

mod common;

use common::*;

/// Iterations per (row, size, threshold) point.
const ITERS: usize = 24;

/// Sizes swept by the "n ∈ {...}" rows.
const SIZES: &[usize] = &[1, 2, 7, 16, 17, 64];

fn all_thresholds() -> Vec<f64> {
    let mut v = THRESHOLDS.to_vec();
    v.extend(nan_thresholds());
    v
}

// =========================================================================
// spectral_contrast -- the low-level entry point (float lanes)
// =========================================================================

/// Row 1 -- `n = 0` with non-null buffers.
#[test]
fn row01_sc_len_zero() {
    let mut rng = Rng::new(SEED ^ 1);
    for it in 0..ITERS {
        let a = gen_f32(&mut rng, 8, Data::Finite);
        let b = gen_f32(&mut rng, 8, Data::Finite);
        diff_sc(&format!("row01 it={it}"), &a, &b, 0);
    }
}

fn sc_row(row: &str, seed: u64, sizes: &[usize], d: Data) {
    let mut rng = Rng::new(SEED ^ seed);
    for &n in sizes {
        for it in 0..ITERS {
            let a = gen_f32(&mut rng, n, d);
            let b = gen_f32(&mut rng, n, d);
            diff_sc(&format!("{row} n={n} it={it} d={d:?}"), &a, &b, n as i32);
        }
    }
}

/// Row 2 -- `n = 1`.
#[test]
fn row02_sc_n1() {
    sc_row("row02", 2, &[1], Data::Finite);
}

/// Row 3 -- `n = 2`.
#[test]
fn row03_sc_n2() {
    sc_row("row03", 3, &[2], Data::Finite);
}

/// Row 4 -- `n = 3` (odd).
#[test]
fn row04_sc_n3() {
    sc_row("row04", 4, &[3], Data::Finite);
}

/// Row 5 -- `n = 15`, just under `N_SMOOTH`.
#[test]
fn row05_sc_n15() {
    sc_row("row05", 5, &[15], Data::Finite);
}

/// Row 6 -- `n = 16`.
#[test]
fn row06_sc_n16() {
    sc_row("row06", 6, &[16], Data::Finite);
}

/// Row 7 -- `n = 17`.
#[test]
fn row07_sc_n17() {
    sc_row("row07", 7, &[17], Data::Finite);
}

/// Row 8 -- `n = 33`.
#[test]
fn row08_sc_n33() {
    sc_row("row08", 8, &[33], Data::Finite);
}

/// Row 9 -- `n = 129`.
#[test]
fn row09_sc_n129() {
    sc_row("row09", 9, &[129], Data::Finite);
}

/// Row 10 -- `n = 1024`.
#[test]
fn row10_sc_n1024() {
    sc_row("row10", 10, &[1024], Data::Finite);
}

/// Row 11 -- all zeros: `magnitude == 0` -> `0.0/0.0` in every lane.
#[test]
fn row11_sc_all_zeros() {
    sc_row("row11", 11, SIZES, Data::AllZeros);
}

/// Row 12 -- exactly one nonzero lane.
#[test]
fn row12_sc_spike() {
    sc_row("row12", 12, SIZES, Data::Spike);
}

/// Row 13 -- constant vector.
#[test]
fn row13_sc_constant() {
    sc_row("row13", 13, SIZES, Data::Constant);
}

/// Row 14 -- huge magnitudes; `Σ x²` overflows the `float` product.
#[test]
fn row14_sc_huge() {
    sc_row("row14", 14, SIZES, Data::Huge);
}

/// Row 15 -- subnormal `float`s; `cvtsd2ss` under/overflow on writeback.
#[test]
fn row15_sc_tiny() {
    sc_row("row15", 15, SIZES, Data::Tiny);
}

/// Row 16 -- mixed `±0.0`.
#[test]
fn row16_sc_signed_zeros() {
    sc_row("row16", 16, SIZES, Data::SignedZeros);
}

/// Row 17 -- `±inf` lanes.
#[test]
fn row17_sc_with_inf() {
    sc_row("row17", 17, SIZES, Data::WithInf);
}

/// Row 18 -- quiet NaNs with random payloads (`addsd`/`mulss` destination rule).
#[test]
fn row18_sc_with_qnan() {
    sc_row("row18", 18, SIZES, Data::WithQNaN);
}

/// Row 19 -- signalling NaNs with random payloads (quieting).
#[test]
fn row19_sc_with_snan() {
    sc_row("row19", 19, SIZES, Data::WithSNaN);
}

/// Row 20 -- fully random 32-bit patterns (every IEEE class).
#[test]
fn row20_sc_random_bits() {
    let mut rng = Rng::new(SEED ^ 20);
    for &n in &[1usize, 2, 3, 7, 16, 17, 31, 64, 129] {
        for it in 0..(ITERS * 4) {
            let a = gen_f32(&mut rng, n, Data::RandomBits);
            let b = gen_f32(&mut rng, n, Data::RandomBits);
            diff_sc(&format!("row20 n={n} it={it}"), &a, &b, n as i32);
        }
    }
}

/// Row 21 -- aliased `a == b`, finite data (buffer normalised twice).
#[test]
fn row21_sc_aliased_finite() {
    let mut rng = Rng::new(SEED ^ 21);
    for &n in SIZES {
        for it in 0..ITERS {
            let a = gen_f32(&mut rng, n, Data::Finite);
            diff_sc_aliased(&format!("row21 n={n} it={it}"), &a, n as i32);
        }
    }
}

/// Row 22 -- aliased `a == b`, random bit patterns.
#[test]
fn row22_sc_aliased_random_bits() {
    let mut rng = Rng::new(SEED ^ 22);
    for &n in &[1usize, 2, 3, 7, 16, 17, 64] {
        for it in 0..(ITERS * 2) {
            let a = gen_f32(&mut rng, n, Data::RandomBits);
            diff_sc_aliased(&format!("row22 n={n} it={it}"), &a, n as i32);
        }
    }
}

/// Row 23 -- `b = -a`: contrast should be ≈ `-1`.
#[test]
fn row23_sc_negated() {
    let mut rng = Rng::new(SEED ^ 23);
    for &n in &[1usize, 7, 16, 17] {
        for it in 0..ITERS {
            let a = gen_f32(&mut rng, n, Data::Finite);
            let b: Vec<f32> = a.iter().map(|x| -x).collect();
            diff_sc(&format!("row23 n={n} it={it}"), &a, &b, n as i32);
        }
    }
}

/// Row 24 -- `b = k·a` for random `k`.
#[test]
fn row24_sc_scaled() {
    let mut rng = Rng::new(SEED ^ 24);
    for &n in SIZES {
        for it in 0..ITERS {
            let a = gen_f32(&mut rng, n, Data::Finite);
            let k = [1e-30f32, 1e-6, 0.5, 1.0, 2.0, 1e6, 1e30][it % 7];
            let b: Vec<f32> = a.iter().map(|x| x * k).collect();
            diff_sc(&format!("row24 n={n} it={it} k={k:e}"), &a, &b, n as i32);
        }
    }
}

/// Row 25 -- identical contents in distinct buffers: contrast ≈ `+1`.
#[test]
fn row25_sc_identical() {
    let mut rng = Rng::new(SEED ^ 25);
    for &n in SIZES {
        for it in 0..ITERS {
            let a = gen_f32(&mut rng, n, Data::Finite);
            diff_sc(&format!("row25 n={n} it={it}"), &a, &a.clone(), n as i32);
        }
    }
}

/// Row 26 -- the exact buffer shape `match` produces: `n` `float` lanes viewed
/// out of an `f64`-aligned allocation, `n` odd so the last lane is the low half
/// of an `f64` slot.
#[test]
fn row26_sc_f64_backed_view() {
    let (c, rs) = libs();
    let mut rng = Rng::new(SEED ^ 26);
    for &n in &[1usize, 3, 5, 7, 15, 17, 33, 65] {
        for it in 0..ITERS {
            let src = gen_f64(&mut rng, n, Data::Finite);
            let mut ac = src.clone();
            let mut bc = src.clone();
            let mut ar = src.clone();
            let mut br = src.clone();

            let vc = unsafe {
                (c.spectral_contrast)(
                    ac.as_mut_ptr() as *mut f32,
                    bc.as_mut_ptr() as *mut f32,
                    n as i32,
                )
            };
            let vr = unsafe {
                (rs.spectral_contrast)(
                    ar.as_mut_ptr() as *mut f32,
                    br.as_mut_ptr() as *mut f32,
                    n as i32,
                )
            };
            assert_eq!(
                vc.to_bits(),
                vr.to_bits(),
                "row26 n={n} it={it}: return diverged (C={vc:?} Rust={vr:?})"
            );
            assert_eq!(bits64(&ac), bits64(&ar), "row26 n={n} it={it}: `a` diverged");
            assert_eq!(bits64(&bc), bits64(&br), "row26 n={n} it={it}: `b` diverged");
        }
    }
}

// =========================================================================
// match -- the composed pipeline (double lanes, forwards to the float callee)
// =========================================================================
//
// NOTE: every `match` row uses `bins >= 1`. `bins <= 0` is not a valid
// configuration -- the C faults on it (ERRORS.md rows 3-8, verified in
// tests/errors.rs), so it belongs to the error surface, not here.

fn match_row(row: &str, seed: u64, bins_set: &[usize], d: Data) {
    let mut rng = Rng::new(SEED ^ seed);
    let thresholds = all_thresholds();
    for &bins in bins_set {
        for it in 0..ITERS {
            let test = gen_f64(&mut rng, bins, d);
            let reference = gen_f64(&mut rng, bins, d);
            for &t in &thresholds {
                diff_match(
                    &format!("{row} bins={bins} it={it} d={d:?} thr={t:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    t,
                );
            }
        }
    }
}

/// Row 27 -- `bins = 0` is an *error* configuration; asserted in
/// `tests/errors.rs` (`err03_match_bins_zero_faults_in_c`). Kept here as an
/// explicit pointer so the row is not silently dropped.
#[test]
fn row27_match_bins_zero_is_an_error_row() {
    // The C segfaults; see ERRORS.md rows 3/4 and tests/errors.rs.
    // Nothing to compare on the valid path.
}

/// Row 28 -- `bins = 1`.
#[test]
fn row28_match_bins1() {
    match_row("row28", 28, &[1], Data::Positive);
}

/// Row 29 -- `bins = 2`.
#[test]
fn row29_match_bins2() {
    match_row("row29", 29, &[2], Data::Positive);
}

/// Row 30 -- `bins = 3` (odd, below `N_SMOOTH`).
#[test]
fn row30_match_bins3() {
    match_row("row30", 30, &[3], Data::Positive);
}

/// Row 31 -- `bins = 15`.
#[test]
fn row31_match_bins15() {
    match_row("row31", 31, &[15], Data::Positive);
}

/// Row 32 -- `bins = 16` (== `N_SMOOTH`).
#[test]
fn row32_match_bins16() {
    match_row("row32", 32, &[16], Data::Positive);
}

/// Row 33 -- `bins = 17`.
#[test]
fn row33_match_bins17() {
    match_row("row33", 33, &[17], Data::Positive);
}

/// Row 34 -- `bins ∈ {31, 32, 33}`.
#[test]
fn row34_match_bins31_33() {
    match_row("row34", 34, &[31, 32, 33], Data::Positive);
}

/// Row 35 -- `bins = 64`.
#[test]
fn row35_match_bins64() {
    match_row("row35", 35, &[64], Data::Positive);
}

/// Row 36 -- `bins = 257`.
#[test]
fn row36_match_bins257() {
    match_row("row36", 36, &[257], Data::Positive);
}

/// Row 37 -- `bins = 1000`.
#[test]
fn row37_match_bins1000() {
    match_row("row37", 37, &[1000], Data::Positive);
}

/// Row 38 -- signed data (totals can go negative, flipping the gate).
#[test]
fn row38_match_signed() {
    match_row("row38", 38, &[1, 2, 15, 16, 17, 64], Data::Finite);
}

/// Row 39 -- all zeros in both inputs.
#[test]
fn row39_match_all_zeros() {
    match_row("row39", 39, &[1, 2, 15, 16, 17, 64], Data::AllZeros);
}

/// Row 40 -- constant input: `differentiate` zeroes everything -> `magnitude = 0`.
#[test]
fn row40_match_constant() {
    match_row("row40", 40, &[1, 2, 15, 16, 17, 64], Data::Constant);
}

/// Row 41 -- monotone ramp.
#[test]
fn row41_match_ramp() {
    match_row("row41", 41, &[1, 2, 15, 16, 17, 64], Data::Ramp);
}

/// Row 42 -- single spike, index swept across the buffer so both the interior
/// `smoothen` rows and the truncated last-15 rows are hit.
#[test]
fn row42_match_spike_swept() {
    let (c, rs) = libs();
    let thresholds = all_thresholds();
    for &bins in &[1usize, 2, 16, 17, 20, 33] {
        for spike in 0..bins {
            let mut test = vec![0.0f64; bins];
            let mut reference = vec![0.0f64; bins];
            test[spike] = 3.5;
            reference[(spike + 1) % bins] = 3.5;
            for &t in &thresholds {
                let mut tc = test.clone();
                let mut rc = reference.clone();
                let mut tr = test.clone();
                let mut rr = reference.clone();
                let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rc.as_mut_ptr(), bins as i32, t) };
                let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rr.as_mut_ptr(), bins as i32, t) };
                assert_eq!(
                    vc, vr,
                    "row42 bins={bins} spike={spike} thr={t:?}: C={vc} Rust={vr}"
                );
            }
        }
    }
}

/// Row 43 -- huge magnitudes: `total` overflows, `differentiate` does `inf-inf`.
#[test]
fn row43_match_huge() {
    match_row("row43", 43, &[1, 2, 15, 16, 17, 64], Data::Huge);
}

/// Row 44 -- subnormal / tiny doubles.
#[test]
fn row44_match_tiny() {
    match_row("row44", 44, &[1, 2, 15, 16, 17, 64], Data::Tiny);
}

/// Row 45 -- `±inf` lanes.
#[test]
fn row45_match_with_inf() {
    match_row("row45", 45, &[1, 2, 15, 16, 17, 64], Data::WithInf);
}

/// Row 46 -- QNaN and SNaN lanes with random payloads.
#[test]
fn row46_match_with_nan() {
    match_row("row46a", 46, &[1, 2, 15, 16, 17, 64], Data::WithQNaN);
    match_row("row46b", 461, &[1, 2, 15, 16, 17, 64], Data::WithSNaN);
}

/// Row 47 -- fully random 64-bit patterns (every IEEE class).
#[test]
fn row47_match_random_bits() {
    let mut rng = Rng::new(SEED ^ 47);
    let thresholds = all_thresholds();
    for &bins in &[1usize, 2, 3, 7, 16, 17, 31, 64] {
        for it in 0..(ITERS * 2) {
            let test = gen_f64(&mut rng, bins, Data::RandomBits);
            let reference = gen_f64(&mut rng, bins, Data::RandomBits);
            for &t in &thresholds {
                diff_match(
                    &format!("row47 bins={bins} it={it} thr={t:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    t,
                );
            }
        }
    }
}

/// Row 48 -- inputs whose **low 32 bits** form a chosen `float` class, which is
/// the only part `match` ever forwards to `spectral_contrast`.
#[test]
fn row48_match_low_word_classes() {
    for (name, d) in [
        ("inf", Data::LowWordInf),
        ("nan", Data::LowWordNaN),
        ("subnormal", Data::LowWordSubnormal),
    ] {
        match_row(
            &format!("row48/{name}"),
            48 + name.len() as u64,
            &[1, 2, 15, 16, 17, 64],
            d,
        );
    }
}

/// Row 49 -- aliased `test == reference`.
#[test]
fn row49_match_aliased() {
    let mut rng = Rng::new(SEED ^ 49);
    let thresholds = all_thresholds();
    for &bins in &[1usize, 2, 15, 16, 17, 64] {
        for it in 0..ITERS {
            let buf = gen_f64(&mut rng, bins, Data::Finite);
            for &t in &thresholds {
                diff_match_aliased(
                    &format!("row49 bins={bins} it={it} thr={t:?}"),
                    &buf,
                    bins as i32,
                    t,
                );
            }
        }
    }
}

/// Row 50 -- identical contents in distinct buffers.
#[test]
fn row50_match_identical() {
    let mut rng = Rng::new(SEED ^ 50);
    let thresholds = all_thresholds();
    for &bins in &[1usize, 2, 15, 16, 17, 64] {
        for it in 0..ITERS {
            let buf = gen_f64(&mut rng, bins, Data::Positive);
            for &t in &thresholds {
                diff_match(
                    &format!("row50 bins={bins} it={it} thr={t:?}"),
                    &buf,
                    &buf.clone(),
                    bins as i32,
                    t,
                );
            }
        }
    }
}

/// Row 51 -- `reference = -test`.
#[test]
fn row51_match_negated() {
    let mut rng = Rng::new(SEED ^ 51);
    let thresholds = all_thresholds();
    for &bins in &[1usize, 2, 15, 16, 17, 64] {
        for it in 0..ITERS {
            let test = gen_f64(&mut rng, bins, Data::Positive);
            let reference: Vec<f64> = test.iter().map(|x| -x).collect();
            for &t in &thresholds {
                diff_match(
                    &format!("row51 bins={bins} it={it} thr={t:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    t,
                );
            }
        }
    }
}

/// Row 52 -- `reference = k·test` sweeping the gate's decision boundary.
#[test]
fn row52_match_scaled() {
    let mut rng = Rng::new(SEED ^ 52);
    let thresholds = all_thresholds();
    for &bins in &[1usize, 2, 16, 17, 64] {
        for &k in &[1e-6f64, 0.25, 0.5, 1.0, 2.0, 4.0, 1e6] {
            for it in 0..(ITERS / 4).max(2) {
                let test = gen_f64(&mut rng, bins, Data::Positive);
                let reference: Vec<f64> = test.iter().map(|x| x * k).collect();
                for &t in &thresholds {
                    diff_match(
                        &format!("row52 bins={bins} k={k:e} it={it} thr={t:?}"),
                        &test,
                        &reference,
                        bins as i32,
                        t,
                    );
                }
            }
        }
    }
}

/// Row 53 -- thresholds swept at `±1 ulp` around the *gate* boundary
/// `total(test) / total(reference)`, forcing the `comisd`/`jbe` branch to flip.
#[test]
fn row53_match_gate_boundary() {
    let mut rng = Rng::new(SEED ^ 53);
    for &bins in &[1usize, 2, 16, 17, 64] {
        for it in 0..(ITERS * 2) {
            let test = gen_f64(&mut rng, bins, Data::Positive);
            let reference = gen_f64(&mut rng, bins, Data::Positive);
            let st: f64 = test.iter().sum();
            let sr: f64 = reference.iter().sum();
            let boundary = st / sr;
            for d in -3i64..=3 {
                let t = f64::from_bits((boundary.to_bits() as i64 + d) as u64);
                diff_match(
                    &format!("row53 bins={bins} it={it} d={d} thr={t:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    t,
                );
            }
        }
    }
}

/// Row 54 -- thresholds swept at `±1 ulp` around the *contrast* boundary: the
/// value `spectral_contrast` actually returned for this input, obtained by
/// replaying `match`'s pipeline and calling the exported `spectral_contrast`.
#[test]
fn row54_match_contrast_boundary() {
    let (c, _rs) = libs();
    let mut rng = Rng::new(SEED ^ 54);
    for &bins in &[1usize, 2, 3, 16, 17, 33, 64] {
        for it in 0..ITERS {
            let test = gen_f64(&mut rng, bins, Data::Positive);
            let reference = gen_f64(&mut rng, bins, Data::Positive);

            // Replay match's preprocessing with the C library itself so the
            // boundary is exact, then probe thresholds around it.
            let mut t_buf = preprocess_ref(&test);
            let mut r_buf = preprocess_ref(&reference);
            let contrast = unsafe {
                (c.spectral_contrast)(
                    t_buf.as_mut_ptr() as *mut f32,
                    r_buf.as_mut_ptr() as *mut f32,
                    bins as i32,
                )
            };
            if !contrast.is_finite() {
                continue;
            }
            for d in -3i64..=3 {
                let thr = f64::from_bits((contrast.to_bits() as i64 + d) as u64);
                diff_match(
                    &format!("row54 bins={bins} it={it} d={d} thr={thr:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    thr,
                );
            }
        }
    }
}

/// Row 55 -- composed end-to-end check: drive `match`'s pipeline by hand and
/// feed the resulting `double` buffers to **both** libraries' exported
/// `spectral_contrast`, verifying the `float`-reinterpretation path directly.
#[test]
fn row55_composed_pipeline_through_both_sos() {
    let (c, rs) = libs();
    let mut rng = Rng::new(SEED ^ 55);
    for &bins in &[1usize, 2, 3, 5, 15, 16, 17, 31, 64, 129] {
        for d in [
            Data::Positive,
            Data::Finite,
            Data::Constant,
            Data::Ramp,
            Data::Spike,
            Data::Huge,
            Data::Tiny,
            Data::WithInf,
            Data::WithQNaN,
            Data::WithSNaN,
            Data::RandomBits,
        ] {
            for it in 0..8 {
                let test = gen_f64(&mut rng, bins, d);
                let reference = gen_f64(&mut rng, bins, d);
                let mut tc = preprocess_ref(&test);
                let mut rc = preprocess_ref(&reference);
                let mut tr = tc.clone();
                let mut rr = rc.clone();

                let vc = unsafe {
                    (c.spectral_contrast)(
                        tc.as_mut_ptr() as *mut f32,
                        rc.as_mut_ptr() as *mut f32,
                        bins as i32,
                    )
                };
                let vr = unsafe {
                    (rs.spectral_contrast)(
                        tr.as_mut_ptr() as *mut f32,
                        rr.as_mut_ptr() as *mut f32,
                        bins as i32,
                    )
                };
                let ctx = format!("row55 bins={bins} d={d:?} it={it}");
                assert_eq!(
                    vc.to_bits(),
                    vr.to_bits(),
                    "{ctx}: composed contrast diverged (C={vc:?} Rust={vr:?})"
                );
                assert_eq!(bits64(&tc), bits64(&tr), "{ctx}: `t` buffer diverged");
                assert_eq!(bits64(&rc), bits64(&rr), "{ctx}: `r` buffer diverged");
            }
        }
    }
}

/// A local replica of the C's `static preprocess` (smoothen / differentiate /
/// smoothen with `N_SMOOTH = 16`), used only to *build test fixtures* for rows
/// 54 and 55 -- never as an oracle for the assertions themselves.
fn preprocess_ref(source: &[f64]) -> Vec<f64> {
    const N_SMOOTH: usize = 16;
    let mut v = source.to_vec();
    let n = v.len();
    let smoothen = |v: &mut Vec<f64>| {
        for i in 0..n {
            let mut sum = 0.0f64;
            let mut j = 0;
            while j < N_SMOOTH && i + j < n {
                sum += v[i + j];
                j += 1;
            }
            v[i] = sum / N_SMOOTH as f64;
        }
    };
    smoothen(&mut v);
    if n > 0 {
        for i in 0..n - 1 {
            v[i] = v[i + 1] - v[i];
        }
        v[n - 1] = 0.0;
    }
    smoothen(&mut v);
    v
}
