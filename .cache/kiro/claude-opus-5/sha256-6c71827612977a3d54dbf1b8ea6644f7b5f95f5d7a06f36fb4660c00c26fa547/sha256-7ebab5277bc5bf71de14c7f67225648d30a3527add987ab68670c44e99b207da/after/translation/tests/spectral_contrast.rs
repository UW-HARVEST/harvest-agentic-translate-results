//! `spectral_contrast` -- the lowest-level exported symbol.
//!
//! `dot_product` and `normalize` are `static` in the C, so they are not
//! reachable through the `.so`; they are exercised transitively.
//!
//! `spectral_contrast` mutates both of its buffers in place, so every case
//! compares the return value *and* the two post-call buffers.

mod common;

use common::{Pair, Rng, assert_f32_slice_bits_eq, assert_f64_bits_eq, assert_f64_slice_bits_eq};

/// Runs one `float`-buffer case against C and every built Rust library.
#[track_caller]
fn check_f32(pair: &Pair, a: &[f32], b: &[f32], label: &str) {
    let c_fn = pair.c_spectral_contrast();
    let len = a.len() as core::ffi::c_int;

    let mut c_a = a.to_vec();
    let mut c_b = b.to_vec();
    let c_ret = unsafe { c_fn(c_a.as_mut_ptr(), c_b.as_mut_ptr(), len) };

    for (profile, rust_fn) in pair.rust_spectral_contrast() {
        let mut r_a = a.to_vec();
        let mut r_b = b.to_vec();
        let r_ret = unsafe { rust_fn(r_a.as_mut_ptr(), r_b.as_mut_ptr(), len) };

        let what = format!("spectral_contrast[{profile}] {label}: return");
        assert_f64_bits_eq(c_ret, r_ret, &what);
        assert_f32_slice_bits_eq(&c_a, &r_a, &format!("spectral_contrast[{profile}] {label}: a"));
        assert_f32_slice_bits_eq(&c_b, &r_b, &format!("spectral_contrast[{profile}] {label}: b"));
    }
}

/// Runs one case the way `match.h` misdeclares the function: `double` buffers
/// handed to a callee that reads and writes `float`s. Only the leading
/// `len * 4` bytes are touched, and the surrounding `double`s must be corrupted
/// identically by both implementations.
#[track_caller]
fn check_f64_buffers(pair: &Pair, a: &[f64], b: &[f64], len: core::ffi::c_int, label: &str) {
    let c_fn = pair.c_spectral_contrast();

    let mut c_a = a.to_vec();
    let mut c_b = b.to_vec();
    let c_ret = unsafe {
        c_fn(
            c_a.as_mut_ptr().cast(),
            c_b.as_mut_ptr().cast(),
            len,
        )
    };

    for (profile, rust_fn) in pair.rust_spectral_contrast() {
        let mut r_a = a.to_vec();
        let mut r_b = b.to_vec();
        let r_ret = unsafe {
            rust_fn(
                r_a.as_mut_ptr().cast(),
                r_b.as_mut_ptr().cast(),
                len,
            )
        };

        assert_f64_bits_eq(
            c_ret,
            r_ret,
            &format!("spectral_contrast[{profile}] {label} (double view): return"),
        );
        assert_f64_slice_bits_eq(
            &c_a,
            &r_a,
            &format!("spectral_contrast[{profile}] {label} (double view): a"),
        );
        assert_f64_slice_bits_eq(
            &c_b,
            &r_b,
            &format!("spectral_contrast[{profile}] {label} (double view): b"),
        );
    }
}

#[test]
fn zero_and_tiny_lengths() {
    let pair = Pair::load();
    check_f32(&pair, &[], &[], "len 0");
    check_f32(&pair, &[1.0], &[1.0], "len 1 ones");
    check_f32(&pair, &[3.0], &[-4.0], "len 1 signed");
    check_f32(&pair, &[1.0, 0.0], &[0.0, 1.0], "len 2 orthogonal");
    check_f32(&pair, &[1.0, 1.0], &[1.0, 1.0], "len 2 parallel");
    check_f32(&pair, &[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0], "len 3 reversed");
}

/// C reads `length` elements; a non-positive `length` runs no loop iterations,
/// leaves both buffers untouched, and `dot_product` returns `0.0`.
#[test]
fn non_positive_length() {
    let pair = Pair::load();
    let a: Vec<f32> = vec![1.5, -2.5, 3.5, 4.0];
    let b: Vec<f32> = vec![-1.0, 2.0, -3.0, 4.5];
    let c_fn = pair.c_spectral_contrast();

    for len in [0, -1, -7, i32::MIN] {
        let mut c_a = a.clone();
        let mut c_b = b.clone();
        let c_ret = unsafe { c_fn(c_a.as_mut_ptr(), c_b.as_mut_ptr(), len) };

        for (profile, rust_fn) in pair.rust_spectral_contrast() {
            let mut r_a = a.clone();
            let mut r_b = b.clone();
            let r_ret = unsafe { rust_fn(r_a.as_mut_ptr(), r_b.as_mut_ptr(), len) };
            assert_f64_bits_eq(
                c_ret,
                r_ret,
                &format!("spectral_contrast[{profile}] len {len}: return"),
            );
            assert_f32_slice_bits_eq(
                &c_a,
                &r_a,
                &format!("spectral_contrast[{profile}] len {len}: a"),
            );
            assert_f32_slice_bits_eq(
                &c_b,
                &r_b,
                &format!("spectral_contrast[{profile}] len {len}: b"),
            );
        }
    }
}

/// A zero vector gives `magnitude == 0`, so `normalize` divides by zero. The C
/// does not guard this, producing infinities and `NaN`s that must be reproduced
/// bit-for-bit.
#[test]
fn degenerate_magnitudes() {
    let pair = Pair::load();
    check_f32(&pair, &[0.0, 0.0, 0.0], &[1.0, 2.0, 3.0], "zero a");
    check_f32(&pair, &[1.0, 2.0, 3.0], &[0.0, 0.0, 0.0], "zero b");
    check_f32(&pair, &[0.0; 4], &[0.0; 4], "both zero");
    check_f32(&pair, &[-0.0, 0.0], &[0.0, -0.0], "signed zeros");
    check_f32(&pair, &[f32::NAN, 1.0], &[1.0, 1.0], "nan a");
    check_f32(&pair, &[f32::INFINITY, 1.0], &[1.0, 2.0], "inf a");
    check_f32(
        &pair,
        &[f32::NEG_INFINITY, f32::INFINITY],
        &[1.0, 1.0],
        "mixed inf",
    );
    // Overflows `float` in the squaring, so magnitude becomes +inf.
    check_f32(&pair, &[3.0e38, 3.0e38], &[1.0, 1.0], "overflow to inf");
    // Underflows: subnormal inputs, magnitude tiny.
    check_f32(
        &pair,
        &[f32::from_bits(1), f32::from_bits(2)],
        &[f32::from_bits(3), 1.0],
        "subnormals",
    );
    check_f32(
        &pair,
        &[f32::MIN_POSITIVE, f32::MIN_POSITIVE],
        &[f32::MIN_POSITIVE, 0.0],
        "min positive",
    );
}

/// The `float`-rounding of `a[i] * b[i]` before accumulation into the `double`
/// sum is observable; values chosen so a `double`-precision product would give a
/// different total.
#[test]
fn float_rounding_of_products() {
    let pair = Pair::load();
    let a: Vec<f32> = (0..64).map(|i| 1.0 + (i as f32) * 1.0e-7).collect();
    let b: Vec<f32> = (0..64).map(|i| 1.0 - (i as f32) * 3.0e-7).collect();
    check_f32(&pair, &a, &b, "near-one products");

    let a: Vec<f32> = (0..33)
        .map(|i| f32::from_bits(0x3f80_0001 + i as u32))
        .collect();
    let b: Vec<f32> = (0..33)
        .map(|i| f32::from_bits(0x3f80_0011 + i as u32))
        .collect();
    check_f32(&pair, &a, &b, "adjacent representables");

    // Mixed magnitudes: catastrophic cancellation in the accumulator.
    let a: Vec<f32> = vec![1.0e20, 1.0, -1.0e20, 1.0];
    let b: Vec<f32> = vec![1.0, 1.0, 1.0, 1.0];
    check_f32(&pair, &a, &b, "cancellation");
}

#[test]
fn randomized_float_buffers() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x5eed_1234);
    for len in [1usize, 2, 3, 5, 8, 15, 16, 17, 31, 64, 129, 512] {
        for trial in 0..12 {
            let a: Vec<f32> = (0..len).map(|_| rng.range(-1000.0, 1000.0) as f32).collect();
            let b: Vec<f32> = (0..len).map(|_| rng.range(-1000.0, 1000.0) as f32).collect();
            check_f32(&pair, &a, &b, &format!("random len {len} trial {trial}"));
        }
    }
}

/// Random bit patterns, including `NaN`s, infinities and subnormals.
#[test]
fn randomized_bit_patterns() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xdead_beef);
    for len in [1usize, 4, 9, 32, 100] {
        for trial in 0..10 {
            let a: Vec<f32> = (0..len)
                .map(|_| f32::from_bits(rng.next_u64() as u32))
                .collect();
            let b: Vec<f32> = (0..len)
                .map(|_| f32::from_bits(rng.next_u64() as u32))
                .collect();
            check_f32(&pair, &a, &b, &format!("bits len {len} trial {trial}"));
        }
    }
}

/// The call shape `match` uses: `double` arrays viewed as `float` arrays.
#[test]
fn double_buffers_viewed_as_float() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xfeed_face);
    for bins in [1usize, 2, 3, 4, 16, 17, 63, 256] {
        for trial in 0..8 {
            let a: Vec<f64> = (0..bins).map(|_| rng.range(-10.0, 10.0)).collect();
            let b: Vec<f64> = (0..bins).map(|_| rng.range(-10.0, 10.0)).collect();
            check_f64_buffers(
                &pair,
                &a,
                &b,
                bins as core::ffi::c_int,
                &format!("bins {bins} trial {trial}"),
            );
        }
    }
    // Small integral doubles: their low 4 bytes are all-zero, so the even-index
    // `float` reads are `+0.0` and the odd-index reads carry the exponent bits.
    let a: Vec<f64> = (1..=8).map(|i| i as f64).collect();
    let b: Vec<f64> = (1..=8).map(|i| (9 - i) as f64).collect();
    check_f64_buffers(&pair, &a, &b, 8, "integral doubles");
}

/// Same pointer for both arguments: C normalizes the buffer twice and then dots
/// it with itself. No `restrict` qualifier stops this, so the Rust must agree.
#[test]
fn aliased_arguments() {
    let pair = Pair::load();
    let c_fn = pair.c_spectral_contrast();

    for data in [
        vec![1.0f32, 2.0, 3.0, 4.0],
        vec![0.0f32; 4],
        vec![-1.5f32, 0.25, 1.0e-30, 7.0],
        (0..40).map(|i| (i as f32) - 20.0).collect::<Vec<f32>>(),
    ] {
        let len = data.len() as core::ffi::c_int;
        let mut c_v = data.clone();
        let c_ret = unsafe {
            let p = c_v.as_mut_ptr();
            c_fn(p, p, len)
        };

        for (profile, rust_fn) in pair.rust_spectral_contrast() {
            let mut r_v = data.clone();
            let r_ret = unsafe {
                let p = r_v.as_mut_ptr();
                rust_fn(p, p, len)
            };
            assert_f64_bits_eq(
                c_ret,
                r_ret,
                &format!("spectral_contrast[{profile}] aliased len {len}: return"),
            );
            assert_f32_slice_bits_eq(
                &c_v,
                &r_v,
                &format!("spectral_contrast[{profile}] aliased len {len}: buffer"),
            );
        }
    }
}

/// Partially overlapping buffers (`b` starts one element into `a`).
#[test]
fn overlapping_arguments() {
    let pair = Pair::load();
    let c_fn = pair.c_spectral_contrast();
    let data: Vec<f32> = (0..17).map(|i| (i as f32) * 0.5 - 3.0).collect();

    for len in [1usize, 2, 8, 16] {
        let mut c_v = data.clone();
        let c_ret = unsafe {
            let p = c_v.as_mut_ptr();
            c_fn(p, p.add(1), len as core::ffi::c_int)
        };
        for (profile, rust_fn) in pair.rust_spectral_contrast() {
            let mut r_v = data.clone();
            let r_ret = unsafe {
                let p = r_v.as_mut_ptr();
                rust_fn(p, p.add(1), len as core::ffi::c_int)
            };
            assert_f64_bits_eq(
                c_ret,
                r_ret,
                &format!("spectral_contrast[{profile}] overlap len {len}: return"),
            );
            assert_f32_slice_bits_eq(
                &c_v,
                &r_v,
                &format!("spectral_contrast[{profile}] overlap len {len}: buffer"),
            );
        }
    }
}
