//! `match` -- the top-level exported symbol, which calls `total`, `smoothen`,
//! `differentiate`, `preprocess` and then `spectral_contrast`.
//!
//! `match` must leave its two input buffers untouched, so each case compares the
//! `int` return value *and* both buffers afterwards.
//!
//! `bins` is used as a VLA length in the C (`float_t t[bins]`), so negative
//! values are not exercised: a negative VLA size is undefined behaviour and
//! aborts the process rather than returning a comparable answer.

mod common;

use common::{Pair, Rng, assert_f64_slice_bits_eq};

#[track_caller]
fn check(pair: &Pair, test: &[f64], reference: &[f64], bins: core::ffi::c_int, threshold: f64) {
    let label = format!("match(bins={bins}, threshold={threshold:?})");
    let c_fn = pair.c_match();

    let mut c_test = test.to_vec();
    let mut c_ref = reference.to_vec();
    let c_ret = unsafe { c_fn(c_test.as_mut_ptr(), c_ref.as_mut_ptr(), bins, threshold) };

    for (profile, rust_fn) in pair.rust_match() {
        let mut r_test = test.to_vec();
        let mut r_ref = reference.to_vec();
        let r_ret = unsafe { rust_fn(r_test.as_mut_ptr(), r_ref.as_mut_ptr(), bins, threshold) };

        assert_eq!(
            c_ret, r_ret,
            "{label} [{profile}]: C returned {c_ret}, Rust returned {r_ret}\n  test={test:?}\n  \
             reference={reference:?}"
        );
        assert_f64_slice_bits_eq(&c_test, &r_test, &format!("{label} [{profile}]: test buffer"));
        assert_f64_slice_bits_eq(&c_ref, &r_ref, &format!("{label} [{profile}]: reference buffer"));
        // Neither implementation may write through its input pointers.
        assert_f64_slice_bits_eq(
            test,
            &r_test,
            &format!("{label} [{profile}]: test buffer must be unmodified"),
        );
        assert_f64_slice_bits_eq(
            reference,
            &r_ref,
            &format!("{label} [{profile}]: reference buffer must be unmodified"),
        );
    }
}

const THRESHOLDS: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    0.9,
    0.99,
    0.999,
    1.0e-9,
    2.0,
    100.0,
    -100.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

#[test]
fn identical_vectors() {
    let pair = Pair::load();
    for bins in [1usize, 2, 3, 4, 8, 15, 16, 17, 33, 64, 100] {
        let v: Vec<f64> = (0..bins).map(|i| (i as f64).sin() + 1.5).collect();
        for &threshold in THRESHOLDS {
            check(&pair, &v, &v, bins as core::ffi::c_int, threshold);
        }
    }
}

/// `bins == 0` is *not* compared against C.
///
/// With `bins == 0` the C declares `float_t t[0]` -- a zero-length VLA that
/// `gcc` allocates at `rsp` without reserving any bytes -- and `differentiate`
/// then executes `v[length - 1] = 0`, i.e. `v[-1] = 0`, storing 8 bytes *below*
/// `rsp`. That overwrites the frame's saved registers and return address, and
/// the C library reliably crashes with `SIGSEGV` on return (verified by calling
/// the C `.so` directly through `dlopen`). There is no C behaviour to match, so
/// the case is excluded rather than asserted on.
///
/// The Rust translation returns instead of crashing, which is the only sane
/// option; this test just pins that down so the difference is deliberate and
/// documented.
#[test]
fn zero_bins_is_undefined_in_c_and_only_checked_on_the_rust_side() {
    let pair = Pair::load();
    for (profile, rust_fn) in pair.rust_match() {
        for &threshold in THRESHOLDS {
            let mut t: Vec<f64> = Vec::new();
            let mut r: Vec<f64> = Vec::new();
            let ret = unsafe { rust_fn(t.as_mut_ptr(), r.as_mut_ptr(), 0, threshold) };
            // `spectral_contrast` over zero elements returns `+0.0`, so the
            // result is `0.0 >= threshold`.
            let expected = i32::from(0.0f64 >= threshold);
            assert_eq!(
                ret, expected,
                "match[{profile}] bins=0 threshold={threshold:?}"
            );
        }
    }
}

#[test]
fn constant_and_zero_vectors() {
    let pair = Pair::load();
    for bins in [1usize, 2, 16, 17, 40] {
        let zeros = vec![0.0f64; bins];
        let ones = vec![1.0f64; bins];
        let neg = vec![-1.0f64; bins];
        for &threshold in THRESHOLDS {
            let n = bins as core::ffi::c_int;
            check(&pair, &zeros, &zeros, n, threshold);
            check(&pair, &ones, &ones, n, threshold);
            check(&pair, &zeros, &ones, n, threshold);
            check(&pair, &ones, &zeros, n, threshold);
            check(&pair, &neg, &ones, n, threshold);
            check(&pair, &ones, &neg, n, threshold);
        }
    }
}

/// The early-out `total(test) < threshold * total(reference)` and the final
/// `>= threshold` are both driven here; `total` is also fed values that make the
/// products overflow or produce NaN.
#[test]
fn early_out_paths() {
    let pair = Pair::load();
    let big = vec![1.0e308f64; 8];
    let small = vec![1.0e-308f64; 8];
    let mixed: Vec<f64> = vec![1.0e308, -1.0e308, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let nan_vec: Vec<f64> = vec![f64::NAN, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let inf_vec: Vec<f64> = vec![f64::INFINITY, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let tail_nan: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, f64::NAN];

    let corpus = [&big, &small, &mixed, &nan_vec, &inf_vec, &tail_nan];
    for a in corpus {
        for b in corpus {
            for &threshold in THRESHOLDS {
                check(&pair, a, b, 8, threshold);
            }
        }
    }
}

/// Shifted / scaled variants of one shape, i.e. the intended use of the matcher.
#[test]
fn shifted_and_scaled_spectra() {
    let pair = Pair::load();
    let bins = 128usize;
    let base: Vec<f64> = (0..bins)
        .map(|i| {
            let x = i as f64 / bins as f64;
            (-((x - 0.3) * (x - 0.3)) / 0.005).exp() * 10.0
                + (-((x - 0.7) * (x - 0.7)) / 0.01).exp() * 4.0
                + 0.1
        })
        .collect();

    for shift in [0usize, 1, 2, 5, 12, 40] {
        let shifted: Vec<f64> = (0..bins).map(|i| base[(i + shift) % bins]).collect();
        for scale in [1.0f64, 0.5, 2.0, 10.0] {
            let scaled: Vec<f64> = shifted.iter().map(|v| v * scale).collect();
            for &threshold in &[0.0, 0.25, 0.5, 0.75, 0.9, 0.99, 1.0, 1.5] {
                check(&pair, &scaled, &base, bins as core::ffi::c_int, threshold);
                check(&pair, &base, &scaled, bins as core::ffi::c_int, threshold);
            }
        }
    }
}

#[test]
fn randomized_positive_spectra() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1357_9bdf);
    for bins in [1usize, 2, 3, 5, 8, 15, 16, 17, 31, 32, 63, 128, 257] {
        for _ in 0..8 {
            let a: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 100.0)).collect();
            let b: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 100.0)).collect();
            for &threshold in &[0.0, 0.1, 0.5, 0.9, 1.0, 1.0000001, 2.0] {
                check(&pair, &a, &b, bins as core::ffi::c_int, threshold);
            }
        }
    }
}

#[test]
fn randomized_signed_spectra() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x2468_ace0);
    for bins in [2usize, 7, 16, 33, 64, 200] {
        for _ in 0..8 {
            let a: Vec<f64> = (0..bins).map(|_| rng.range(-50.0, 50.0)).collect();
            let b: Vec<f64> = (0..bins).map(|_| rng.range(-50.0, 50.0)).collect();
            for &threshold in &[-1.0, 0.0, 0.3, 0.999, 1.0] {
                check(&pair, &a, &b, bins as core::ffi::c_int, threshold);
            }
        }
    }
}

/// Extreme exponents: `smoothen`/`differentiate` can overflow to infinity and
/// then produce NaN, and the `double`-viewed-as-`float` reinterpretation turns
/// arbitrary mantissa bits into arbitrary `float`s.
#[test]
fn randomized_wide_exponents() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0bad_c0de);
    for bins in [2usize, 8, 17, 64] {
        for _ in 0..10 {
            let a: Vec<f64> = (0..bins)
                .map(|_| rng.range(-1.0, 1.0) * 10f64.powf(rng.range(-300.0, 300.0)))
                .collect();
            let b: Vec<f64> = (0..bins)
                .map(|_| rng.range(-1.0, 1.0) * 10f64.powf(rng.range(-300.0, 300.0)))
                .collect();
            for &threshold in &[0.0, 0.5, 1.0, f64::INFINITY, f64::NEG_INFINITY] {
                check(&pair, &a, &b, bins as core::ffi::c_int, threshold);
            }
        }
    }
}

/// Random `double` bit patterns, so NaNs, infinities and subnormals reach every
/// stage of the pipeline.
#[test]
fn randomized_bit_patterns() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xcafe_f00d);
    for bins in [1usize, 4, 9, 32, 65] {
        for _ in 0..10 {
            let a: Vec<f64> = (0..bins).map(|_| f64::from_bits(rng.next_u64())).collect();
            let b: Vec<f64> = (0..bins).map(|_| f64::from_bits(rng.next_u64())).collect();
            for &threshold in &[0.0, 1.0, -1.0, f64::NAN] {
                check(&pair, &a, &b, bins as core::ffi::c_int, threshold);
            }
        }
    }
}

/// The same buffer for both arguments.
#[test]
fn aliased_inputs() {
    let pair = Pair::load();
    let c_fn = pair.c_match();
    let data: Vec<f64> = (0..24).map(|i| (i as f64) * 0.75 - 4.0).collect();

    for bins in [1usize, 2, 16, 24] {
        for &threshold in THRESHOLDS {
            let mut c_v = data.clone();
            let c_ret = unsafe {
                let p = c_v.as_mut_ptr();
                c_fn(p, p, bins as core::ffi::c_int, threshold)
            };
            for (profile, rust_fn) in pair.rust_match() {
                let mut r_v = data.clone();
                let r_ret = unsafe {
                    let p = r_v.as_mut_ptr();
                    rust_fn(p, p, bins as core::ffi::c_int, threshold)
                };
                assert_eq!(
                    c_ret, r_ret,
                    "match[{profile}] aliased bins={bins} threshold={threshold:?}"
                );
                assert_f64_slice_bits_eq(
                    &c_v,
                    &r_v,
                    &format!("match[{profile}] aliased bins={bins}: buffer"),
                );
            }
        }
    }
}

/// `bins` smaller than the buffers: only the leading `bins` elements may be read.
#[test]
fn partial_bins() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x9999_1111);
    let full_a: Vec<f64> = (0..64).map(|_| rng.range(0.0, 20.0)).collect();
    let full_b: Vec<f64> = (0..64).map(|_| rng.range(0.0, 20.0)).collect();
    for bins in [1usize, 2, 3, 16, 17, 33, 63, 64] {
        for &threshold in &[0.0, 0.5, 0.95, 1.0] {
            check(
                &pair,
                &full_a[..bins],
                &full_b[..bins],
                bins as core::ffi::c_int,
                threshold,
            );
        }
    }
}
