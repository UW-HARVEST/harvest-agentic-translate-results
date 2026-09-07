//! Phase C — error / rejection-path differential tests.
//!
//! One test (or one clearly-labelled block) per row of `ERRORS.md`, plus the
//! generic C-API boundaries: null pointers, zero and oversized lengths, values
//! one past a valid range, and raw out-of-range integer bit patterns crossing
//! the FFI boundary.

mod common;

use common::*;
use std::os::raw::c_int;

/// `s2 = 1.0f / expf(1.6f*1.6f*2.25f)` — recomputed here only to describe the
/// expected clamp behaviour in assertions, never used to fake a result.
fn s2_reference() -> f32 {
    let sigma = 1.6f32;
    let tetha = 2.25f32;
    1.0f32 / (sigma * sigma * tetha).exp()
}

fn all_zero_bits(v: &[u32], n: usize) -> bool {
    v[..n].iter().all(|&b| b == 0)
}

// ==========================================================================
// Row 1 — size <= -2: write loop guard false on entry, nothing touched.
// ==========================================================================

#[test]
fn err01_size_le_minus_two_touches_nothing() {
    let sizes: [c_int; 9] = [-2, -3, -4, -5, -17, -1000, -100_000, i32::MIN + 1, i32::MIN];
    for &size in &sizes {
        assert_eq!(written_len(size), 0, "size={size} should write nothing");
        for &radius in &[1.0f32, 0.0, -3.5, f32::NAN, f32::INFINITY] {
            let n = 16;
            let fill: Vec<f32> = (0..n)
                .map(|i| f32::from_bits(0xABCD_0000u32 | i as u32))
                .collect();
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err01");
            // Same rejection: BOTH must leave the buffer completely untouched.
            for i in 0..n {
                assert_eq!(
                    c[i], fill[i].to_bits(),
                    "C touched [{i}] with size={size}"
                );
                assert_eq!(
                    r[i], fill[i].to_bits(),
                    "Rust touched [{i}] with size={size}"
                );
            }
        }
    }
}

// ==========================================================================
// Row 2 — NULL dest with size <= -2 must return cleanly from both.
// Row 16 — size == INT_MIN is part of that safe set.
// ==========================================================================

#[test]
fn err02_null_dest_with_no_write_is_clean() {
    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    let sizes: [c_int; 8] = [-2, -3, -7, -12345, -1_000_000, i32::MIN + 2, i32::MIN + 1, i32::MIN];
    for &size in &sizes {
        for &radius in &[
            1.0f32,
            0.0,
            -0.0,
            -2.5,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MIN_POSITIVE,
        ] {
            // SAFETY: for size <= -2 the C provably performs no dereference
            // (row 1), so a null destination is a legitimate input. If either
            // side faults here, that IS the divergence being tested for.
            unsafe {
                cf(std::ptr::null_mut(), size, radius);
                rf(std::ptr::null_mut(), size, radius);
            }
        }
    }
}

// ==========================================================================
// Row 3 — size == -1: one write, no normalization.
// ==========================================================================

#[test]
fn err03_size_minus_one_writes_one_unnormalized() {
    let expected_unnormalized = 1.0f32 - s2_reference();
    for &radius in &[1.0f32, 2.0, 0.5, 100.0, 1e-30, -4.0, f32::INFINITY] {
        let n = 8;
        let fill = vec![poison(); n];
        let (c, r) = run_both(-1, radius, &fill);
        assert_same(-1, radius, &c, &r, "err03");
        assert_eq!(written_len(-1), 1);
        // dest[0] written, everything after untouched.
        assert_ne!(c[0], poison().to_bits(), "dest[0] should be written");
        for i in 1..n {
            assert_eq!(c[i], fill[i].to_bits());
            assert_eq!(r[i], fill[i].to_bits());
        }
        // Not normalized: for a finite radius the value is 1-s2 (or clamped 0),
        // never forced to 1.0.
        if radius.is_finite() && radius != 0.0 {
            let got = f32::from_bits(c[0]);
            assert!(
                got == expected_unnormalized || got == 0.0,
                "size=-1 radius={radius}: got {got}, expected {expected_unnormalized} or 0"
            );
        }
    }
}

// ==========================================================================
// Row 4 — size == 0 still writes exactly one element, un-normalized.
// ==========================================================================

#[test]
fn err04_size_zero_writes_one_element() {
    assert_eq!(written_len(0), 1);
    let expected_unnormalized = 1.0f32 - s2_reference();
    for &radius in &[1.0f32, 3.0, 0.25, 1e6, -1.0, 1e-40, f32::NAN, 0.0] {
        let n = 8;
        let fill = vec![poison(); n];
        let (c, r) = run_both(0, radius, &fill);
        assert_same(0, radius, &c, &r, "err04");
        assert_ne!(c[0], poison().to_bits(), "size=0 must still write dest[0]");
        assert_ne!(r[0], poison().to_bits(), "size=0 must still write dest[0]");
        for i in 1..n {
            assert_eq!(c[i], fill[i].to_bits());
            assert_eq!(r[i], fill[i].to_bits());
        }
        if radius.is_finite() && radius != 0.0 && !radius.is_nan() {
            let got = f32::from_bits(c[0]);
            assert!(
                got == expected_unnormalized || got == 0.0,
                "size=0 radius={radius}: got {got} (must be un-normalized)"
            );
        }
    }
}

// ==========================================================================
// Row 5 — even size: exactly one element written past dest[size-1], and that
// element is NOT normalized.
// ==========================================================================

#[test]
fn err05_even_size_writes_one_past_end() {
    let mut rng = Rng::new(0x2005);
    for &size in &[2i32, 4, 6, 8, 10, 32, 64, 100, 128, 512, 1024] {
        assert_eq!(written_len(size), size as usize + 1);
        for _ in 0..40 {
            let radius = rng.logunif_f32(1e-2, 1e3);
            let n = buffer_len(size);
            let fill: Vec<f32> = (0..n)
                .map(|i| f32::from_bits(0xBEEF_0000u32 | (i as u32 & 0xFFFF)))
                .collect();
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err05 OOB-by-one");
            // dest[size] IS written by both...
            let oob = size as usize;
            assert_ne!(
                c[oob], fill[oob].to_bits(),
                "C must write the out-of-bounds element dest[{oob}]"
            );
            assert_ne!(
                r[oob], fill[oob].to_bits(),
                "Rust must write the out-of-bounds element dest[{oob}]"
            );
            // ...and dest[size+1..] is NOT.
            for i in (oob + 1)..n {
                assert_eq!(c[i], fill[i].to_bits(), "C over-wrote [{i}]");
                assert_eq!(r[i], fill[i].to_bits(), "Rust over-wrote [{i}]");
            }
        }
    }
}

// ==========================================================================
// Rows 6 & 7 — radius == +0.0 / -0.0: rs = ±inf, centre gives 0*inf = NaN,
// everything clamps to 0, sum == 0, normalization skipped.
// ==========================================================================

#[test]
fn err06_07_radius_zero_and_negative_zero() {
    for &radius in &[0.0f32, -0.0f32] {
        for size in [-1i32, 0, 1, 2, 3, 4, 5, 7, 16, 33, 64, 129] {
            let n = buffer_len(size);
            let fill = vec![poison(); n];
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err06/07 radius=±0");
            let w = written_len(size);
            assert!(
                all_zero_bits(&c, w),
                "C: radius={radius} size={size} expected all +0.0, got {:?}",
                &c[..w]
            );
            assert!(
                all_zero_bits(&r, w),
                "Rust: radius={radius} size={size} expected all +0.0"
            );
        }
    }
}

// ==========================================================================
// Row 8 — radius = NaN (several payloads, quiet and signaling, both signs).
// ==========================================================================

#[test]
fn err08_radius_nan_payloads() {
    let nan_bits: [u32; 10] = [
        0x7FC0_0000, // canonical quiet NaN
        0xFFC0_0000, // negative quiet NaN
        0x7F80_0001, // signaling NaN, minimal payload
        0xFF80_0001,
        0x7FFF_FFFF, // max payload
        0xFFFF_FFFF,
        0x7FC0_1234,
        0xFFA5_5A5A,
        0x7F81_0000,
        0xFFBF_FFFF,
    ];
    for &bits in &nan_bits {
        let radius = f32::from_bits(bits);
        assert!(radius.is_nan());
        for size in [-2i32, -1, 0, 1, 2, 3, 8, 17, 64] {
            let n = buffer_len(size);
            let fill = vec![poison(); n];
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err08 radius=NaN");
            let w = written_len(size);
            assert!(
                all_zero_bits(&c, w),
                "C: NaN radius {bits:#010x} size={size} expected all +0.0, got {:?}",
                &c[..w]
            );
        }
    }
}

// ==========================================================================
// Rows 9 & 10 — radius = ±inf: rs = ±0 -> every element equals 1-s2,
// sum > 0 -> normalized to 1/(2*hsize+1).
// ==========================================================================

/// The un-normalized centre weight `1 - s2`, obtained FROM THE C LIBRARY
/// itself (`size == 0` writes exactly one element and skips normalization) so
/// that no assertion depends on Rust's own `f32::exp`.
fn c_unnormalized_weight() -> f32 {
    let cf = c_gaussian_kernel();
    let mut b = [poison(); 4];
    // SAFETY: size == 0 writes exactly dest[0].
    unsafe { cf(b.as_mut_ptr(), 0, f32::INFINITY) };
    b[0]
}

#[test]
fn err09_10_radius_infinite() {
    let unnorm = c_unnormalized_weight();
    for &radius in &[f32::INFINITY, f32::NEG_INFINITY] {
        for size in [-2i32, -1, 0, 1, 2, 3, 4, 5, 9, 32, 65, 128, 1024] {
            let n = buffer_len(size);
            let fill = vec![poison(); n];
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err09/10 radius=±inf");
            let w = written_len(size);
            // rs = 1.6/±inf = ±0 -> every x = 0 -> every raw v == 1 - s2.
            // The normalize loop covers only [0, size), so for even `size` the
            // extra element at index `size` stays un-normalized.
            let norm_len = if size > 0 { w.min(size as usize) } else { 0 };
            for i in 1..norm_len {
                assert_eq!(
                    c[i], c[0],
                    "radius={radius} size={size}: normalized element {i} differs from 0"
                );
            }
            for i in norm_len..w {
                assert_eq!(
                    c[i], unnorm.to_bits(),
                    "radius={radius} size={size}: element {i} must be the \
                     un-normalized weight {unnorm:.9}"
                );
            }
            if norm_len > 0 {
                let v = f32::from_bits(c[0]);
                assert!(v > 0.0, "expected positive normalized weight, got {v}");
            }
        }
    }
}

// ==========================================================================
// Row 11 — tiny positive radius: only the centre survives the clamp; after
// normalization the centre is exactly 1.0 and the rest +0.0.
// Row 18 — smallest subnormal radius makes rs = +inf (row 6 behaviour).
// ==========================================================================

#[test]
fn err11_tiny_radius_only_centre_survives() {
    for &radius in &[1e-6f32, 1e-10, 1e-20, 1e-30, 1e-37, f32::MIN_POSITIVE] {
        for size in [1i32, 3, 5, 9, 33, 65] {
            let n = buffer_len(size);
            let fill = vec![poison(); n];
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err11 tiny radius");
            let hsize = (size / 2) as usize;
            for i in 0..written_len(size) {
                let got = f32::from_bits(c[i]);
                if i == hsize {
                    assert_eq!(got, 1.0f32, "centre should normalize to 1.0 (size={size}, radius={radius:e})");
                } else {
                    assert_eq!(c[i], 0u32, "tail [{i}] should be +0.0");
                }
            }
        }
    }
}

#[test]
fn err18_subnormal_radius_makes_rs_infinite() {
    let subnormals: [u32; 6] = [1, 2, 7, 0x0000_FFFF, 0x0040_0000 - 1, 0x0000_0100];
    let mut saw_infinite_rs = false;
    for &bits in &subnormals {
        for &sign in &[0u32, 0x8000_0000] {
            let radius = f32::from_bits(bits | sign);
            assert!(radius.is_subnormal(), "{bits:#x} should be subnormal");
            // Only the *small* subnormals overflow `rs = 1.6f / radius` to
            // infinity; the larger ones (e.g. 0x003FFFFF -> rs ~= 2.7e38) stay
            // finite, so the all-zero expectation applies to the former only.
            let rs_infinite = (1.6f32 / radius).is_infinite();
            saw_infinite_rs |= rs_infinite;
            for size in [-1i32, 0, 1, 2, 3, 8, 33] {
                let n = buffer_len(size);
                let fill = vec![poison(); n];
                let (c, r) = run_both(size, radius, &fill);
                assert_same(size, radius, &c, &r, "err18 subnormal radius");
                if rs_infinite {
                    let w = written_len(size);
                    assert!(
                        all_zero_bits(&c, w),
                        "C: subnormal radius {radius:e} size={size} expected all +0.0, got {:?}",
                        &c[..w]
                    );
                }
            }
        }
    }
    assert!(saw_infinite_rs, "test data must include an rs-overflowing subnormal");
}

// ==========================================================================
// Row 12 — very large finite radius.
// ==========================================================================

#[test]
fn err12_huge_finite_radius() {
    let radii: [f32; 8] = [
        3.4e38,
        f32::MAX,
        -f32::MAX,
        1e30,
        1e20,
        f32::MAX / 2.0,
        f32::from_bits(0x7F7F_FFFE),
        1.6e38,
    ];
    for &radius in &radii {
        for size in [-2i32, -1, 0, 1, 2, 3, 7, 64, 129] {
            check(size, radius, "err12 huge finite radius");
        }
    }
}

// ==========================================================================
// Row 13 — negative ordinary radius produces the identical result to +radius.
// ==========================================================================

#[test]
fn err13_negative_radius_matches_positive() {
    let cf = c_gaussian_kernel();
    let mut rng = Rng::new(0x2013);
    for _ in 0..300 {
        let size = rng.int_in(0, 64) as c_int;
        let mag = rng.logunif_f32(1e-3, 1e3);
        check(size, -mag, "err13 negative radius (differential)");

        // and confirm the C itself is sign-agnostic, which the Rust must mirror
        let n = buffer_len(size);
        let mut a = vec![poison(); n];
        let mut b = vec![poison(); n];
        unsafe {
            cf(a.as_mut_ptr(), size, mag);
            cf(b.as_mut_ptr(), size, -mag);
        }
        let ab: Vec<u32> = a.iter().map(|f| f.to_bits()).collect();
        let bb: Vec<u32> = b.iter().map(|f| f.to_bits()).collect();
        assert_eq!(ab, bb, "C is not sign-agnostic for radius={mag:e} size={size}");
    }
}

// ==========================================================================
// Row 14 — the per-element clamp branch, including exact v == 0.0f.
// ==========================================================================

#[test]
fn err14_clamp_branch_mixed() {
    // radius chosen so the window straddles |x| == 2.4 -> some elements clamp.
    let mut rng = Rng::new(0x2014);
    for _ in 0..400 {
        let size = rng.int_in(9, 81) as c_int | 1;
        // put the flip point in the middle of the window
        let hsize = size / 2;
        let radius = (hsize as f32) * 1.6f32 / 2.4f32 * (0.4 + 1.2 * rng.unit() as f32);
        let (c, r) = run_both_poisoned(size, radius);
        assert_same(size, radius, &c, &r, "err14 mixed clamp");
        let w = written_len(size);
        let zeros = c[..w].iter().filter(|&&b| b == 0).count();
        let nonzeros = w - zeros;
        assert!(nonzeros > 0, "expected some surviving elements");
        // every clamped slot must be exactly +0.0 in BOTH
        for i in 0..w {
            if c[i] == 0 {
                assert_eq!(r[i], 0, "clamped slot [{i}] differs");
            }
        }
    }
}

// ==========================================================================
// Row 15 — sum > 0.0f false: normalization entirely skipped.
// ==========================================================================

#[test]
fn err15_sum_zero_skips_normalization() {
    // radius == 0 / NaN / subnormal all clamp everything to zero -> sum == 0.
    for &radius in &[0.0f32, -0.0f32, f32::NAN, f32::from_bits(1), 1e-45] {
        for size in [1i32, 2, 3, 4, 17, 64] {
            let (c, r) = run_both_poisoned(size, radius);
            assert_same(size, radius, &c, &r, "err15 sum==0");
            let w = written_len(size);
            assert!(
                all_zero_bits(&c, w),
                "radius={radius} size={size}: normalization must be skipped, leaving +0.0"
            );
        }
    }
}

// ==========================================================================
// Row 16 — size == INT_MIN and neighbours (guard evaluation, no writes).
// ==========================================================================

#[test]
fn err16_int_min_size() {
    let sizes: [c_int; 6] = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN / 2,
        -1_073_741_824,
        -2_147_483_647,
    ];
    for &size in &sizes {
        assert_eq!(written_len(size), 0);
        for &radius in &[1.0f32, 0.0, f32::NAN, -1e30] {
            let n = 16;
            let fill: Vec<f32> = (0..n)
                .map(|i| f32::from_bits(0x1234_0000u32 | i as u32))
                .collect();
            let (c, r) = run_both(size, radius, &fill);
            assert_same(size, radius, &c, &r, "err16 INT_MIN size");
            assert_eq!(c, fill.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
            assert_eq!(r, fill.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        }
    }
}

// ==========================================================================
// Row 17 — smallest positive size normalizes to exactly 1.0.
// ==========================================================================

#[test]
fn err17_size_one_normalizes_to_exactly_one() {
    let mut rng = Rng::new(0x2017);
    for _ in 0..300 {
        let radius = rng.logunif_f32(1e-3, 1e6);
        let (c, r) = run_both_poisoned(1, radius);
        assert_same(1, radius, &c, &r, "err17 size=1");
        assert_eq!(
            f32::from_bits(c[0]), 1.0f32,
            "size=1 must normalize to exactly 1.0 (radius={radius:e})"
        );
    }
    // and the degenerate radii where sum == 0 so it stays 0.0
    for &radius in &[0.0f32, -0.0, f32::NAN] {
        let (c, r) = run_both_poisoned(1, radius);
        assert_same(1, radius, &c, &r, "err17 size=1 degenerate");
        assert_eq!(c[0], 0u32);
    }
}

// ==========================================================================
// Row 19 — raw out-of-range integer bit patterns for `size` across the FFI
// boundary (the C `int` parameter accepts any bit pattern; there is no enum in
// this API, so this is the equivalent "no valid variant" input class).
// ==========================================================================

#[test]
fn err19_arbitrary_int_bit_patterns_for_size() {
    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    let mut rng = Rng::new(0x2019);

    // (a) Arbitrary negative bit patterns <= -2: provably no writes, so a
    //     null destination is safe and both must simply return.
    for _ in 0..2000 {
        let size = -(rng.int_in(2, i32::MAX as i64) as i64) as c_int;
        assert!(size <= -2);
        let radius = rng.finite_f32();
        unsafe {
            cf(std::ptr::null_mut(), size, radius);
            rf(std::ptr::null_mut(), size, radius);
        }
    }

    // (b) Arbitrary bit patterns in a range we can actually allocate for,
    //     including the degenerate -1 / 0 values.
    for _ in 0..2000 {
        let size = rng.int_in(-8, 200) as c_int;
        let radius = rng.finite_f32();
        check(size, radius, "err19 arbitrary size");
    }

    // (c) Explicit "one step past" boundaries around every interesting value.
    for base in [-2i32, -1, 0, 1, 2, 3, 4] {
        for d in -1i32..=1 {
            let size = base + d;
            if size <= -2 {
                unsafe {
                    cf(std::ptr::null_mut(), size, 1.0);
                    rf(std::ptr::null_mut(), size, 1.0);
                }
            }
            if size >= -1 {
                for &radius in &[1.0f32, 0.0, f32::NAN, f32::INFINITY, -2.0] {
                    check(size, radius, "err19 one-step-past");
                }
            }
        }
    }
}

// ==========================================================================
// Generic boundary sweep: every f32 exponent field, both signs, sampled
// mantissas — covers inf, NaN, subnormals, and every normal magnitude class.
// ==========================================================================

#[test]
fn generic_full_exponent_sweep_for_radius() {
    let mantissas: [u32; 6] = [0, 1, 0x0000_0F0F, 0x0040_0000 - 1, 0x0020_0000, 0x007F_FFFF];
    let sizes: [c_int; 6] = [-2, 0, 1, 2, 3, 17];
    for exp in 0u32..=255 {
        for &m in &mantissas {
            for &sign in &[0u32, 0x8000_0000u32] {
                let bits = sign | (exp << 23) | (m & 0x007F_FFFF);
                let radius = f32::from_bits(bits);
                for &size in &sizes {
                    check(size, radius, "generic exponent sweep");
                }
            }
        }
    }
}

// ==========================================================================
// Generic boundary: zero / oversized length behaviour already covered above;
// here the oversized-but-allocatable end plus size == INT_MAX guard eval.
// ==========================================================================

#[test]
fn generic_oversized_size_guard_only() {
    // We cannot allocate 2*(INT_MAX/2)+1 floats, but the guard arithmetic
    // (`hsize = size/2`, `-hsize`, `r <= hsize`) must not panic or wrap
    // differently in Rust. Exercise it on the negative mirror where the loop
    // body never runs, so it is safe with a null destination.
    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();
    for &size in &[i32::MIN, i32::MIN + 1, -(i32::MAX / 2), -(i32::MAX - 1), -i32::MAX] {
        unsafe {
            cf(std::ptr::null_mut(), size, 1.6);
            rf(std::ptr::null_mut(), size, 1.6);
        }
    }

    // A genuinely large positive size we CAN allocate, to catch index math
    // divergence at scale.
    for &size in &[65_535i32, 65_536, 100_001] {
        check(size, 12.5, "generic large allocatable size");
        check(size, 1e-8, "generic large allocatable size tiny radius");
    }
}
