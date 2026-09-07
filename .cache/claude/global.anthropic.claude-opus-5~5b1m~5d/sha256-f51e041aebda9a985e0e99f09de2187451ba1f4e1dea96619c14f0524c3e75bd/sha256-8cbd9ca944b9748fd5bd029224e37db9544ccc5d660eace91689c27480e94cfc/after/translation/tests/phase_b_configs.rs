//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `gaussian_kernel` symbol and compares the entire destination buffer
//! bit-for-bit, including the slot the C writes one past the end for even
//! `size` and the untouched guard slack.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Row 1 — odd small `size`, moderate random `radius`.
#[test]
fn row01_odd_small_size_moderate_radius() {
    let mut rng = Rng::new(SEED);
    for &size in &[1i32, 3, 5, 7, 9, 11] {
        for _ in 0..400 {
            diff(size, rng.range(0.5, 8.0));
        }
    }
}

/// Row 2 — even small `size` (writes `size + 1` elements).
#[test]
fn row02_even_small_size_moderate_radius() {
    let mut rng = Rng::new(SEED ^ 2);
    for &size in &[2i32, 4, 6, 8, 10] {
        for _ in 0..400 {
            diff(size, rng.range(0.5, 8.0));
        }
    }
}

/// Row 3 — `size == 0`: one write, no normalisation.
#[test]
fn row03_size_zero() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        diff(0, rng.range(0.001, 1000.0));
    }
    for &r in &[1.0f32, 0.0, -0.0, f32::INFINITY, f32::NAN, 1e-30, 1e30] {
        diff(0, r);
    }
    // The single written slot must be the un-normalised 1 - s2.
    let l = libs();
    let mut c = vec![f32::from_bits(GUARD); 8];
    let mut r = c.clone();
    unsafe {
        (l.c_kernel)(c.as_mut_ptr(), 0, 2.0);
        (l.rust_kernel)(r.as_mut_ptr(), 0, 2.0);
    }
    assert_bits_eq(&c, &r, "size=0 explicit");
    assert_ne!(c[0].to_bits(), GUARD, "size=0 must still write dest[0]");
    assert_ne!(c[0], 1.0f32, "size=0 dest[0] must NOT be normalised");
    assert_eq!(
        c[1].to_bits(),
        GUARD,
        "size=0 must write exactly one element"
    );
}

/// Row 4 — `size == 1`: normalised to exactly 1.0.
#[test]
fn row04_size_one() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..2000 {
        diff(1, rng.range(0.001, 1000.0));
    }
    let l = libs();
    let mut c = vec![f32::from_bits(GUARD); 8];
    let mut r = c.clone();
    unsafe {
        (l.c_kernel)(c.as_mut_ptr(), 1, 3.0);
        (l.rust_kernel)(r.as_mut_ptr(), 1, 3.0);
    }
    assert_bits_eq(&c, &r, "size=1 explicit");
    assert_eq!(c[0], 1.0f32, "size=1 must normalise to 1.0");
}

/// Row 5 — odd large `size`.
#[test]
fn row05_odd_large_size() {
    let mut rng = Rng::new(SEED ^ 5);
    for &size in &[65i32, 127, 255, 511, 1023] {
        for _ in 0..60 {
            diff(size, rng.range(0.5, 64.0));
        }
    }
}

/// Row 6 — even large `size`.
#[test]
fn row06_even_large_size() {
    let mut rng = Rng::new(SEED ^ 6);
    for &size in &[64i32, 128, 256, 512, 1024] {
        for _ in 0..60 {
            diff(size, rng.range(0.5, 64.0));
        }
    }
}

/// Row 7 — huge `radius`, `rs` collapses to ~0.
#[test]
fn row07_huge_radius() {
    let mut rng = Rng::new(SEED ^ 7);
    for &size in &[1i32, 2, 3, 8, 9, 33, 64] {
        for _ in 0..200 {
            let mag = 1e6f32 * (1.0 + rng.unit() * 1e6);
            diff(size, mag);
        }
        for &r in &[1e6f32, 1e12, 1e20, 1e30, f32::MAX] {
            diff(size, r);
            diff(size, -r);
        }
    }
}

/// Row 8 — `radius == +inf` (`rs == +0.0`).
#[test]
fn row08_radius_pos_inf() {
    for &size in &[-4i32, -2, -1, 0, 1, 2, 3, 7, 8, 63, 64, 255] {
        diff(size, f32::INFINITY);
    }
}

/// Row 9 — `radius == -inf` (`rs == -0.0`).
#[test]
fn row09_radius_neg_inf() {
    for &size in &[-4i32, -2, -1, 0, 1, 2, 3, 7, 8, 63, 64, 255] {
        diff(size, f32::NEG_INFINITY);
    }
}

/// Row 10 — negative finite `radius` mirrors the positive one exactly.
#[test]
fn row10_negative_radius_mirrors_positive() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    for &size in &[1i32, 2, 3, 4, 9, 16, 33] {
        for _ in 0..300 {
            let mag = rng.range(0.3, 12.0);
            diff(size, -mag);
            // and the mirror property itself, measured on the C library
            let n = buf_len(size);
            let (mut a, mut b) = (
                vec![f32::from_bits(GUARD); n],
                vec![f32::from_bits(GUARD); n],
            );
            unsafe {
                (l.c_kernel)(a.as_mut_ptr(), size, mag);
                (l.c_kernel)(b.as_mut_ptr(), size, -mag);
            }
            assert_bits_eq(&a, &b, &format!("C mirror property size={size} r={mag}"));
        }
    }
}

/// Row 11 — small `radius`: most taps clamp to +0.0, centre survives.
#[test]
fn row11_small_radius_sparse_kernel() {
    let mut rng = Rng::new(SEED ^ 11);
    for &size in &[3i32, 4, 9, 16, 65] {
        for _ in 0..400 {
            diff(size, rng.range(0.01, 0.5));
        }
    }
}

/// Row 12 — boundary sweep around the clamp threshold `radius == 1.6/2.4`.
#[test]
fn row12_clamp_threshold_boundary() {
    let thresh = 1.6f32 / 2.4f32; // ~0.6666667
    let mut cases: Vec<f32> = vec![thresh, 0.6666, 0.66666, 0.666666, 0.6667, 0.66667, 0.667];
    // walk a few ULPs either side of the exact threshold
    let mut up = thresh;
    let mut down = thresh;
    for _ in 0..8 {
        up = f32::from_bits(up.to_bits() + 1);
        down = f32::from_bits(down.to_bits() - 1);
        cases.push(up);
        cases.push(down);
    }
    for &size in &[3i32, 4, 5, 8, 9, 33, 64] {
        for &r in &cases {
            diff(size, r);
            diff(size, -r);
        }
    }
}

/// Row 13 — tiny / denormal `radius`: `rs` overflows, `sum == 0`.
#[test]
fn row13_tiny_denormal_radius() {
    let mut cases: Vec<f32> = vec![
        f32::MIN_POSITIVE,
        f32::from_bits(1), // smallest denormal
        f32::from_bits(2),
        1e-45,
        1e-40,
        1e-38,
        1e-30,
        1e-20,
        1e-10,
        1e-6,
    ];
    let neg: Vec<f32> = cases.iter().map(|v| -v).collect();
    cases.extend(neg);
    for &size in &[-1i32, 0, 1, 2, 3, 8, 9, 65] {
        for &r in &cases {
            diff(size, r);
        }
    }
}

/// Row 14 — `radius == ±0.0`.
#[test]
fn row14_radius_zero() {
    for &size in &[-4i32, -2, -1, 0, 1, 2, 3, 7, 8, 63, 64, 255] {
        diff(size, 0.0);
        diff(size, -0.0);
    }
}

/// Row 15 — `radius` NaN, several bit patterns.
#[test]
fn row15_radius_nan() {
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0xFFC0_0000),
    ];
    for &size in &[-2i32, -1, 0, 1, 2, 3, 8, 9, 65] {
        for &r in &nans {
            diff(size, r);
        }
    }
}

/// Row 16 — pre-poisoned destination buffer.
#[test]
fn row16_prepoisoned_buffer() {
    let mut rng = Rng::new(SEED ^ 16);
    let poisons = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        1e30,
        f32::MIN_POSITIVE,
    ];
    for &size in &[-3i32, -2, -1, 0, 1, 2, 3, 4, 8, 9, 33, 64] {
        let n = buf_len(size);
        for &p in &poisons {
            let prefill = vec![p; n];
            diff_case(size, 2.5, &prefill);
            diff_case(size, 0.0, &prefill);
            diff_case(size, f32::NAN, &prefill);
        }
        for _ in 0..200 {
            let prefill: Vec<f32> = (0..n).map(|_| rng.any_f32()).collect();
            diff_case(size, rng.range(0.2, 6.0), &prefill);
        }
    }
}

/// Row 17 — broad randomized fuzz over `size` and arbitrary `radius` bits.
#[test]
fn row17_fuzz_all_axes() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..20_000 {
        let size = (rng.below(73) as i32) - 8; // -8 ..= 64
        let radius = if rng.below(4) == 0 {
            rng.any_f32()
        } else {
            let m = rng.range(-40.0, 40.0);
            if rng.below(8) == 0 {
                m * 1e-20
            } else {
                m
            }
        };
        diff(size, radius);
    }
}

/// Row 18 — repeated calls on the same buffer (no hidden state).
#[test]
fn row18_repeated_calls_same_buffer() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 18);
    for &size in &[1i32, 2, 3, 8, 9, 32, 33] {
        let n = buf_len(size);
        let mut c = vec![f32::from_bits(GUARD); n];
        let mut r = c.clone();
        for i in 0..50 {
            let radius = rng.range(0.05, 20.0);
            unsafe {
                (l.c_kernel)(c.as_mut_ptr(), size, radius);
                (l.rust_kernel)(r.as_mut_ptr(), size, radius);
            }
            assert_bits_eq(&c, &r, &format!("repeat {i} size={size} radius={radius}"));
        }
    }
}

/// Row 19 — non-zero base offset into the buffer.
#[test]
fn row19_offset_destination() {
    let mut rng = Rng::new(SEED ^ 19);
    for &size in &[-2i32, -1, 0, 1, 2, 3, 8, 9, 33] {
        for &off in &[1usize, 2, 3, 5, 7] {
            for _ in 0..40 {
                diff_at_offset(size, rng.range(0.05, 20.0), off);
            }
            diff_at_offset(size, 0.0, off);
            diff_at_offset(size, f32::NAN, off);
        }
    }
}

/// Row 20 — negative `size` truncation boundary, including the surprising
/// `size == -1` case where the write loop still executes once.
#[test]
fn row20_negative_size_truncation_boundary() {
    let mut rng = Rng::new(SEED ^ 20);
    for &size in &[-1i32, -2, -3, -4, -5, -6, -7, -8] {
        for _ in 0..200 {
            diff(size, rng.range(0.05, 20.0));
        }
        for &r in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, 1e-30, 1e30, 2.0] {
            diff(size, r);
        }
    }

    // size == -1 really does write exactly one (un-normalised) element.
    let l = libs();
    let mut c = vec![f32::from_bits(GUARD); 8];
    let mut r = c.clone();
    unsafe {
        (l.c_kernel)(c.as_mut_ptr(), -1, 2.0);
        (l.rust_kernel)(r.as_mut_ptr(), -1, 2.0);
    }
    assert_bits_eq(&c, &r, "size=-1 explicit");
    assert_ne!(c[0].to_bits(), GUARD, "size=-1 must write dest[0]");
    assert_eq!(c[1].to_bits(), GUARD, "size=-1 must write only dest[0]");

    // size <= -2 writes nothing at all.
    for size in [-2i32, -3, -4, -100] {
        let mut c = vec![f32::from_bits(GUARD); 8];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), size, 2.0);
            (l.rust_kernel)(r.as_mut_ptr(), size, 2.0);
        }
        assert_bits_eq(&c, &r, &format!("size={size} explicit"));
        assert!(
            c.iter().all(|v| v.to_bits() == GUARD),
            "size={size} must not write anything"
        );
    }
}

/// Cross-check that the number of slots actually written matches
/// `2*(size/2)+1`, for both libraries, over the whole small-size range.
#[test]
fn write_extent_matches_c_for_both_libraries() {
    let l = libs();
    for size in -8i32..=40 {
        let n = buf_len(size);
        let mut c = vec![f32::from_bits(GUARD); n];
        let mut r = c.clone();
        unsafe {
            (l.c_kernel)(c.as_mut_ptr(), size, 4.0);
            (l.rust_kernel)(r.as_mut_ptr(), size, 4.0);
        }
        assert_bits_eq(&c, &r, &format!("extent size={size}"));
        let c_written = c.iter().take_while(|v| v.to_bits() != GUARD).count();
        let expected = written_slots(size);
        // radius 4.0 keeps every tap non-zero for these sizes, so untouched
        // slots are exactly the ones still holding GUARD.
        assert_eq!(
            c_written, expected,
            "size={size}: C wrote {c_written} slots, expected {expected}"
        );
        let r_written = r.iter().take_while(|v| v.to_bits() != GUARD).count();
        assert_eq!(r_written, expected, "size={size}: Rust write extent differs");
    }
}
