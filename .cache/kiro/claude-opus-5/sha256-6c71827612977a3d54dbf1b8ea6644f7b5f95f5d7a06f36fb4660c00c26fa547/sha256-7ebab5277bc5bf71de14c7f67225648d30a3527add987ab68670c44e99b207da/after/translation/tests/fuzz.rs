//! High-volume randomised differential fuzzing of both exported symbols.
//!
//! The targeted tests in `spectral_contrast.rs` and `match_fn.rs` pin down
//! specific edge cases; this file just throws a lot of structured randomness at
//! both libraries. The corpora deliberately over-represent NaNs, infinities and
//! subnormals, because those are where a translation is most likely to diverge:
//! IEEE 754 leaves NaN payloads unspecified, so C and Rust only agree if the
//! translation reproduces the hardware's operand-order-dependent choice.

mod common;

use common::{Pair, Rng, assert_f32_slice_bits_eq, assert_f64_bits_eq, assert_f64_slice_bits_eq};

/// Draws a `f64` from a mixture of "boring" and "hostile" distributions.
fn hostile_f64(rng: &mut Rng) -> f64 {
    const SPECIALS: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
    ];
    match rng.next_u64() % 8 {
        0 => SPECIALS[(rng.next_u64() as usize) % SPECIALS.len()],
        1 => f64::from_bits(rng.next_u64()),
        // A NaN with a random payload, so payload propagation is exercised.
        2 => f64::from_bits((rng.next_u64() & 0x800f_ffff_ffff_ffff) | 0x7ff0_0000_0000_0001),
        3 => f64::from_bits(rng.next_u64() & 0x000f_ffff_ffff_ffff), // subnormal
        4 => rng.range(-1.0, 1.0) * 10f64.powf(rng.range(-320.0, 320.0)),
        _ => rng.range(-1000.0, 1000.0),
    }
}

/// `f32` counterpart of [`hostile_f64`].
fn hostile_f32(rng: &mut Rng) -> f32 {
    const SPECIALS: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
    ];
    match rng.next_u64() % 8 {
        0 => SPECIALS[(rng.next_u64() as usize) % SPECIALS.len()],
        1 => f32::from_bits(rng.next_u64() as u32),
        2 => f32::from_bits(((rng.next_u64() as u32) & 0x807f_ffff) | 0x7f80_0001),
        3 => f32::from_bits((rng.next_u64() as u32) & 0x007f_ffff),
        4 => (rng.range(-1.0, 1.0) * 10f64.powf(rng.range(-42.0, 42.0))) as f32,
        _ => rng.range(-1000.0, 1000.0) as f32,
    }
}

#[test]
fn fuzz_spectral_contrast() {
    let pair = Pair::load();
    let c_fn = pair.c_spectral_contrast();
    let rust_fns = pair.rust_spectral_contrast();
    let mut rng = Rng::new(0x00c0_ffee_0bad_f00d);

    for iteration in 0..4000u32 {
        let len = 1 + (rng.next_u64() % 40) as usize;
        let a: Vec<f32> = (0..len).map(|_| hostile_f32(&mut rng)).collect();
        let b: Vec<f32> = (0..len).map(|_| hostile_f32(&mut rng)).collect();

        let mut c_a = a.clone();
        let mut c_b = b.clone();
        let c_ret =
            unsafe { c_fn(c_a.as_mut_ptr(), c_b.as_mut_ptr(), len as core::ffi::c_int) };

        for (profile, rust_fn) in &rust_fns {
            let mut r_a = a.clone();
            let mut r_b = b.clone();
            let r_ret =
                unsafe { rust_fn(r_a.as_mut_ptr(), r_b.as_mut_ptr(), len as core::ffi::c_int) };
            let what = format!("fuzz_spectral_contrast[{profile}] iter {iteration} len {len}");
            assert_f64_bits_eq(c_ret, r_ret, &format!("{what}: return\n  a={a:?}\n  b={b:?}"));
            assert_f32_slice_bits_eq(&c_a, &r_a, &format!("{what}: a"));
            assert_f32_slice_bits_eq(&c_b, &r_b, &format!("{what}: b"));
        }
    }
}

/// `spectral_contrast` reached through `match.h`'s misdeclared prototype:
/// `double` arrays reinterpreted as `float` arrays, which is how `match` calls it.
#[test]
fn fuzz_spectral_contrast_over_double_buffers() {
    let pair = Pair::load();
    let c_fn = pair.c_spectral_contrast();
    let rust_fns = pair.rust_spectral_contrast();
    let mut rng = Rng::new(0x1122_3344_5566_7788);

    for iteration in 0..3000u32 {
        let bins = 1 + (rng.next_u64() % 48) as usize;
        let a: Vec<f64> = (0..bins).map(|_| hostile_f64(&mut rng)).collect();
        let b: Vec<f64> = (0..bins).map(|_| hostile_f64(&mut rng)).collect();

        let mut c_a = a.clone();
        let mut c_b = b.clone();
        let c_ret = unsafe {
            c_fn(
                c_a.as_mut_ptr().cast(),
                c_b.as_mut_ptr().cast(),
                bins as core::ffi::c_int,
            )
        };

        for (profile, rust_fn) in &rust_fns {
            let mut r_a = a.clone();
            let mut r_b = b.clone();
            let r_ret = unsafe {
                rust_fn(
                    r_a.as_mut_ptr().cast(),
                    r_b.as_mut_ptr().cast(),
                    bins as core::ffi::c_int,
                )
            };
            let what = format!("fuzz_double_view[{profile}] iter {iteration} bins {bins}");
            assert_f64_bits_eq(c_ret, r_ret, &format!("{what}: return\n  a={a:?}\n  b={b:?}"));
            assert_f64_slice_bits_eq(&c_a, &r_a, &format!("{what}: a"));
            assert_f64_slice_bits_eq(&c_b, &r_b, &format!("{what}: b"));
        }
    }
}

#[test]
fn fuzz_match() {
    let pair = Pair::load();
    let c_fn = pair.c_match();
    let rust_fns = pair.rust_match();
    let mut rng = Rng::new(0x9876_5432_10fe_dcba);

    let thresholds: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.95,
        0.999_999,
        1.000_001,
        1.0e-12,
        1.0e12,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];

    for iteration in 0..3000u32 {
        // `bins >= 1`: `bins == 0` is undefined behaviour in the C (see
        // `match_fn.rs`).
        let bins = 1 + (rng.next_u64() % 48) as usize;
        let test: Vec<f64> = (0..bins).map(|_| hostile_f64(&mut rng)).collect();
        let reference: Vec<f64> = (0..bins).map(|_| hostile_f64(&mut rng)).collect();
        let threshold = if rng.next_u64() % 4 == 0 {
            thresholds[(rng.next_u64() as usize) % thresholds.len()]
        } else {
            rng.range(-2.0, 2.0)
        };

        let mut c_t = test.clone();
        let mut c_r = reference.clone();
        let c_ret = unsafe {
            c_fn(
                c_t.as_mut_ptr(),
                c_r.as_mut_ptr(),
                bins as core::ffi::c_int,
                threshold,
            )
        };

        for (profile, rust_fn) in &rust_fns {
            let mut r_t = test.clone();
            let mut r_r = reference.clone();
            let r_ret = unsafe {
                rust_fn(
                    r_t.as_mut_ptr(),
                    r_r.as_mut_ptr(),
                    bins as core::ffi::c_int,
                    threshold,
                )
            };
            let what =
                format!("fuzz_match[{profile}] iter {iteration} bins {bins} threshold {threshold:?}");
            assert_eq!(
                c_ret, r_ret,
                "{what}: C {c_ret} vs Rust {r_ret}\n  test={test:?}\n  reference={reference:?}"
            );
            assert_f64_slice_bits_eq(&test, &c_t, &format!("{what}: C mutated test buffer"));
            assert_f64_slice_bits_eq(&test, &r_t, &format!("{what}: Rust mutated test buffer"));
            assert_f64_slice_bits_eq(&reference, &c_r, &format!("{what}: C mutated reference"));
            assert_f64_slice_bits_eq(&reference, &r_r, &format!("{what}: Rust mutated reference"));
        }
    }
}

/// Well-behaved positive spectra with fine-grained thresholds -- the regime the
/// matcher is actually meant for, where the answer flips between 0 and 1 and any
/// last-bit difference in the contrast would show up.
#[test]
fn fuzz_match_threshold_boundary() {
    let pair = Pair::load();
    let c_fn = pair.c_match();
    let rust_fns = pair.rust_match();
    let mut rng = Rng::new(0x0f0f_0f0f_f0f0_f0f0);

    for iteration in 0..1500u32 {
        let bins = 4 + (rng.next_u64() % 200) as usize;
        let base: Vec<f64> = (0..bins).map(|_| rng.range(0.0, 50.0)).collect();
        // A perturbed copy, so the contrast lands near 1.
        let noise = rng.range(0.0, 0.4);
        let other: Vec<f64> = base
            .iter()
            .map(|v| v * (1.0 + rng.range(-noise, noise)))
            .collect();

        for &threshold in &[0.0, 0.5, 0.8, 0.9, 0.95, 0.99, 0.999, 1.0] {
            let mut c_t = other.clone();
            let mut c_r = base.clone();
            let c_ret = unsafe {
                c_fn(
                    c_t.as_mut_ptr(),
                    c_r.as_mut_ptr(),
                    bins as core::ffi::c_int,
                    threshold,
                )
            };
            for (profile, rust_fn) in &rust_fns {
                let mut r_t = other.clone();
                let mut r_r = base.clone();
                let r_ret = unsafe {
                    rust_fn(
                        r_t.as_mut_ptr(),
                        r_r.as_mut_ptr(),
                        bins as core::ffi::c_int,
                        threshold,
                    )
                };
                assert_eq!(
                    c_ret, r_ret,
                    "fuzz_match_threshold_boundary[{profile}] iter {iteration} bins {bins} \
                     threshold {threshold}"
                );
            }
        }
    }
}
