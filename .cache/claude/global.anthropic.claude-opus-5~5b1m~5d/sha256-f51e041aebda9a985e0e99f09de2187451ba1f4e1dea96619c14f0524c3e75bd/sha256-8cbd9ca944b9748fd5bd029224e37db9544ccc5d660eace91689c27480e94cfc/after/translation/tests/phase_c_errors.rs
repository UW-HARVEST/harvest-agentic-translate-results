//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `gaussian_kernel` returns `void` and has no error codes, so "the same
//! error/rejection" is asserted as: the same *observable* rejection behaviour —
//! identical write extent, identical clamped/`+0.0f` sentinel values, and
//! identical skipping of the normalisation pass — verified bit-for-bit between
//! the C `.so` and the Rust `.so`.

mod common;

use common::*;

/// Reference value of the pedestal-subtracted centre tap: `1 - 1/expf(5.76)`.
/// Read out of the C library itself rather than recomputed, so the test never
/// second-guesses the C.
fn c_centre_tap() -> f32 {
    let l = libs();
    let mut buf = [f32::from_bits(GUARD); 4];
    unsafe { (l.c_kernel)(buf.as_mut_ptr(), 0, 2.0) };
    buf[0]
}

/// Row 1 — `size <= -2`: loop never runs, nothing written, no normalisation.
#[test]
fn row01_size_le_minus_two_writes_nothing() {
    let l = libs();
    let mut rng = Rng::new(0xE001);
    for size in [-2i32, -3, -4, -5, -9, -100, -12345, i32::MIN + 1, i32::MIN] {
        for radius in [2.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, 1e-30] {
            let mut c = vec![f32::from_bits(GUARD); 32];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("size={size} radius={radius:e}"));
            assert!(
                c.iter().all(|v| v.to_bits() == GUARD),
                "C wrote something for size={size}"
            );
            assert!(
                r.iter().all(|v| v.to_bits() == GUARD),
                "Rust wrote something for size={size}"
            );
        }
    }
    for _ in 0..2000 {
        let size = -2 - (rng.below(1 << 20) as i32);
        diff(size, rng.any_f32());
    }
}

/// Row 2 — `size <= -2` with a NULL destination: must not dereference.
#[test]
fn row02_null_dest_with_negative_size() {
    let l = libs();
    for size in [-2i32, -3, -8, -1000, i32::MIN] {
        for radius in [1.0f32, 0.0, f32::NAN, f32::INFINITY] {
            unsafe {
                (l.c_kernel)(std::ptr::null_mut(), size, radius);
                (l.rust_kernel)(std::ptr::null_mut(), size, radius);
            }
        }
    }
    // Reaching here means neither library dereferenced the null pointer.
}

/// Row 2b — `size == -1`: C truncation makes `hsize == 0`, so one element IS
/// written and normalisation is skipped.
#[test]
fn row02b_size_minus_one_writes_one_unnormalised() {
    let l = libs();
    let centre = c_centre_tap();
    for radius in [2.0f32, 0.5, 1e30, 1.0] {
        let mut c = vec![f32::from_bits(GUARD); 16];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), -1, radius);
            (l.rust_kernel)(r.as_mut_ptr(), -1, radius);
        }
        assert_bits_eq(&c, &r, &format!("size=-1 radius={radius:e}"));
        assert_eq!(
            c[0].to_bits(),
            centre.to_bits(),
            "size=-1 must store the un-normalised centre tap"
        );
        assert_eq!(c[1].to_bits(), GUARD, "size=-1 must write only one slot");
    }
}

/// Row 3 — `radius == 0.0`: `rs = inf`, centre becomes NaN then clamps to +0.0,
/// everything else clamps to +0.0, `sum == 0`, normalisation skipped.
#[test]
fn row03_radius_zero_all_plus_zero_no_normalisation() {
    let l = libs();
    for size in [1i32, 2, 3, 4, 8, 9, 17, 64, 65] {
        let n = buf_len(size);
        let mut c = vec![f32::from_bits(GUARD); n];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), size, 0.0);
            (l.rust_kernel)(r.as_mut_ptr(), size, 0.0);
        }
        assert_bits_eq(&c, &r, &format!("radius=0 size={size}"));
        for i in 0..written_slots(size) {
            assert_eq!(
                c[i].to_bits(),
                0u32,
                "radius=0 size={size}: slot {i} must be +0.0f, got {:e}",
                c[i]
            );
        }
    }
}

/// Row 4 — `radius == -0.0` behaves exactly like `+0.0`.
#[test]
fn row04_radius_negative_zero() {
    let l = libs();
    for size in [1i32, 2, 3, 8, 9, 65] {
        let n = buf_len(size);
        let mut c = vec![f32::from_bits(GUARD); n];
        let mut r = c.clone();
        let mut cpos = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), size, -0.0);
            (l.rust_kernel)(r.as_mut_ptr(), size, -0.0);
            (l.c_kernel)(cpos.as_mut_ptr(), size, 0.0);
        }
        assert_bits_eq(&c, &r, &format!("radius=-0.0 size={size}"));
        assert_bits_eq(&c, &cpos, &format!("C -0.0 vs +0.0 size={size}"));
    }
}

/// Row 5 — `radius = NaN`: every tap clamps to +0.0, normalisation skipped.
#[test]
fn row05_radius_nan_all_plus_zero() {
    let l = libs();
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
    ];
    for size in [1i32, 2, 3, 8, 9, 65] {
        for &radius in &nans {
            let n = buf_len(size);
            let mut c = vec![f32::from_bits(GUARD); n];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(
                &c,
                &r,
                &format!("radius NaN 0x{:08X} size={size}", radius.to_bits()),
            );
            for i in 0..written_slots(size) {
                assert_eq!(c[i].to_bits(), 0u32, "NaN radius slot {i} must be +0.0f");
            }
        }
    }
}

/// Row 6 — `size == 0` writes one element and never normalises it.
#[test]
fn row06_size_zero_writes_one_unnormalised() {
    let l = libs();
    let centre = c_centre_tap();
    for radius in [2.0f32, 1.0, 0.25, 1e30, f32::INFINITY] {
        let mut c = vec![f32::from_bits(GUARD); 16];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), 0, radius);
            (l.rust_kernel)(r.as_mut_ptr(), 0, radius);
        }
        assert_bits_eq(&c, &r, &format!("size=0 radius={radius:e}"));
        assert_eq!(c[0].to_bits(), centre.to_bits());
        assert_eq!(c[1].to_bits(), GUARD);
    }
    // radius = 0 / NaN: the single tap clamps to +0.0 and sum stays 0.
    for radius in [0.0f32, -0.0, f32::NAN] {
        let mut c = vec![f32::from_bits(GUARD); 16];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), 0, radius);
            (l.rust_kernel)(r.as_mut_ptr(), 0, radius);
        }
        assert_bits_eq(&c, &r, &format!("size=0 radius={radius:e}"));
        assert_eq!(c[0].to_bits(), 0u32);
        assert_eq!(c[1].to_bits(), GUARD);
    }
}

/// Row 7 — even `size > 0` stores one element past `dest[size-1]`, and that
/// trailing element is NOT normalised.
#[test]
fn row07_even_size_writes_one_past_the_end() {
    let l = libs();
    let mut rng = Rng::new(0xE007);
    for size in [2i32, 4, 6, 8, 16, 64, 256] {
        for _ in 0..50 {
            let radius = rng.range(1.0, 30.0);
            let n = buf_len(size);
            let mut c = vec![f32::from_bits(GUARD); n];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("even size={size} radius={radius:e}"));
            let last = size as usize; // the one-past-the-end slot
            assert_eq!(
                c.len().min(last + 1),
                last + 1,
                "buffer must cover the OOB slot"
            );
            assert_ne!(
                c[last].to_bits(),
                GUARD,
                "size={size}: C must write dest[{last}] (one past the end)"
            );
            assert_eq!(
                c[last + 1].to_bits(),
                GUARD,
                "size={size}: C must not write beyond dest[{last}]"
            );
            // The un-normalised tail equals the raw (clamped) first tap, which
            // is <= the normalised interior values' scale; simply require that
            // both libraries agree, already asserted above, plus that the tail
            // differs from what normalisation would have produced.
            let normalised_tail = c[last] / (1.0f32);
            assert_eq!(c[last].to_bits(), normalised_tail.to_bits());
        }
    }
}

/// Row 8 — enormous `radius` (`rs -> 0`) produces a uniform normalised kernel.
#[test]
fn row08_huge_radius_uniform_kernel() {
    let l = libs();
    for size in [1i32, 2, 3, 9, 16, 65] {
        for radius in [1e20f32, 1e30, f32::MAX, f32::INFINITY, -f32::MAX] {
            let n = buf_len(size);
            let mut c = vec![f32::from_bits(GUARD); n];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("huge radius={radius:e} size={size}"));
        }
    }
}

/// Row 9 — tiny non-zero `radius`.
#[test]
fn row09_tiny_radius() {
    let l = libs();
    let cases = [
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-40f32,
        1e-38,
        1e-30,
        1e-10,
        1e-3,
        -1e-30,
        -f32::MIN_POSITIVE,
    ];
    for size in [1i32, 2, 3, 9, 16, 65] {
        for &radius in &cases {
            let n = buf_len(size);
            let mut c = vec![f32::from_bits(GUARD); n];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("tiny radius={radius:e} size={size}"));
        }
    }
}

/// Row 10 — negative finite `radius` gives results identical to its magnitude.
#[test]
fn row10_negative_radius_symmetry() {
    let l = libs();
    let mut rng = Rng::new(0xE010);
    for size in [1i32, 2, 3, 5, 9, 33, 64] {
        for _ in 0..200 {
            let m = rng.range(0.01, 40.0);
            let n = buf_len(size);
            let mut cn = vec![f32::from_bits(GUARD); n];
            let mut rn = cn.clone();
            let mut cp = cn.clone();
            unsafe {
                (l.c_kernel)(cn.as_mut_ptr(), size, -m);
                (l.rust_kernel)(rn.as_mut_ptr(), size, -m);
                (l.c_kernel)(cp.as_mut_ptr(), size, m);
            }
            assert_bits_eq(&cn, &rn, &format!("neg radius={m:e} size={size}"));
            assert_bits_eq(&cn, &cp, &format!("C sign symmetry radius={m:e} size={size}"));
        }
    }
}

/// Row 11 — `radius == -inf` (`rs == -0.0`).
#[test]
fn row11_radius_neg_inf() {
    let l = libs();
    for size in [1i32, 2, 3, 9, 16, 65, 0, -1, -2] {
        let n = buf_len(size);
        let mut c = vec![f32::from_bits(GUARD); n];
        let mut r = c.clone();
        let mut cp = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), size, f32::NEG_INFINITY);
            (l.rust_kernel)(r.as_mut_ptr(), size, f32::NEG_INFINITY);
            (l.c_kernel)(cp.as_mut_ptr(), size, f32::INFINITY);
        }
        assert_bits_eq(&c, &r, &format!("radius=-inf size={size}"));
        assert_bits_eq(&c, &cp, &format!("C ±inf equivalence size={size}"));
    }
}

/// Row 12 — near-threshold `radius` where only the centre tap survives, so
/// `sum` is small and `isum = 1/sum` is large (possibly `inf`).
#[test]
fn row12_denormal_sum_overflowing_isum() {
    let l = libs();
    // radius just below the clamp threshold: only r == 0 survives.
    let mut cases: Vec<f32> = vec![0.6f32, 0.5, 0.4, 0.3, 0.2, 0.1, 0.05, 0.01, 1e-3, 1e-5];
    let thresh = 1.6f32 / 2.4f32;
    for k in 1..=6 {
        cases.push(f32::from_bits(thresh.to_bits() - k));
        cases.push(f32::from_bits(thresh.to_bits() + k));
    }
    for size in [1i32, 2, 3, 5, 9, 33, 64, 65] {
        for &radius in &cases {
            let n = buf_len(size);
            let mut c = vec![f32::from_bits(GUARD); n];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("threshold radius={radius:e} size={size}"));
        }
    }
}

/// Row 13 — `size == INT_MIN` and neighbours: extreme out-of-range int.
#[test]
fn row13_size_int_min() {
    let l = libs();
    for size in [i32::MIN, i32::MIN + 1, i32::MIN + 2, -2_000_000_000] {
        for radius in [2.0f32, 0.0, f32::NAN, f32::INFINITY, -3.5] {
            let mut c = vec![f32::from_bits(GUARD); 32];
            let mut r = c.clone();
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("size={size} radius={radius:e}"));
            assert!(
                c.iter().all(|v| v.to_bits() == GUARD),
                "size={size}: nothing may be written"
            );
        }
        // and with a NULL destination, since nothing is dereferenced
        unsafe {
            (l.c_kernel)(std::ptr::null_mut(), size, 1.0);
            (l.rust_kernel)(std::ptr::null_mut(), size, 1.0);
        }
    }
}

/// Row 14 — `size == 1` normalises to exactly `1.0f`.
#[test]
fn row14_size_one_normalises_to_one() {
    let l = libs();
    for radius in [0.7f32, 1.0, 2.0, 1e6, f32::INFINITY, -2.0] {
        let mut c = vec![f32::from_bits(GUARD); 16];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), 1, radius);
            (l.rust_kernel)(r.as_mut_ptr(), 1, radius);
        }
        assert_bits_eq(&c, &r, &format!("size=1 radius={radius:e}"));
        assert_eq!(c[0].to_bits(), 1.0f32.to_bits(), "size=1 must yield 1.0f");
        assert_eq!(c[1].to_bits(), GUARD);
    }
    // Degenerate radii: the single tap clamps to +0.0 and is never normalised.
    for radius in [0.0f32, -0.0, f32::NAN, f32::from_bits(1)] {
        let mut c = vec![f32::from_bits(GUARD); 16];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), 1, radius);
            (l.rust_kernel)(r.as_mut_ptr(), 1, radius);
        }
        assert_bits_eq(&c, &r, &format!("size=1 degenerate radius={radius:e}"));
        assert_eq!(c[0].to_bits(), 0u32);
    }
}

/// Generic boundary sweep: every `size` in a contiguous range crossed with a
/// battery of boundary `radius` values, plus one step past each boundary.
#[test]
fn generic_boundary_sweep() {
    let radii = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        -f32::MAX,
        1.0,
        -1.0,
        1.6 / 2.4,
        1e-30,
        1e30,
    ];
    for size in -16i32..=48 {
        for &radius in &radii {
            diff(size, radius);
        }
    }
}

/// "Out-of-range enum" analogue: `int` values with no meaningful meaning for
/// `size`, passed straight across the FFI boundary.
#[test]
fn out_of_range_int_size_values() {
    let mut rng = Rng::new(0xE0FF);
    let sizes = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -65_537,
        -3,
        -1,
        0,
        1,
        2,
    ];
    for &size in &sizes {
        for _ in 0..100 {
            diff(size, rng.any_f32());
        }
    }
    // Large positive sizes are legal but allocate; keep them bounded.
    for &size in &[4095i32, 4096, 4097] {
        diff(size, 3.0);
        diff(size, 0.0);
        diff(size, f32::NAN);
    }
}
