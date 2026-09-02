//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each asserts that C and Rust reject with
//! the *same* sentinel (`NULL` vs non-`NULL`), not merely that "both failed".

mod common;

use common::{assert_same, assert_same_raw, cap_of, null_ness, Rng, SEED};
use std::ffi::{c_char, c_int};

/// Assert both `.so`s return exactly `NULL`.
fn assert_both_null(size: c_int, src: *const c_char, ctx: &str) {
    let (a_null, b_null) = null_ness(size, src);
    assert!(a_null, "{ctx}: C returned non-NULL for size={size}");
    assert!(b_null, "{ctx}: Rust returned non-NULL for size={size}");
}

/// Assert both `.so`s return non-`NULL` (without dereferencing).
fn assert_both_non_null(size: c_int, src: *const c_char, ctx: &str) {
    let (a_null, b_null) = null_ness(size, src);
    assert!(!a_null, "{ctx}: C returned NULL for size={size}");
    assert!(!b_null, "{ctx}: Rust returned NULL for size={size}");
}

// ------------------------------------------------- rows 1-6: NULL `src`
#[test]
fn error_01_null_src_positive_size() {
    assert_both_null(1, std::ptr::null(), "err1");
    assert_same_raw(1, std::ptr::null(), "err1");
}

#[test]
fn error_02_null_src_zero_size_no_strlen_crash() {
    // The null check on line 33 precedes the strlen on line 37.
    assert_both_null(0, std::ptr::null(), "err2");
    assert_same_raw(0, std::ptr::null(), "err2");
}

#[test]
fn error_03_null_src_negative_size() {
    for size in [-1i32, -2, -3, -4, -100] {
        assert_both_null(size, std::ptr::null(), "err3");
    }
}

#[test]
fn error_04_null_src_int_max() {
    assert_both_null(c_int::MAX, std::ptr::null(), "err4");
}

#[test]
fn error_05_null_src_int_min() {
    assert_both_null(c_int::MIN, std::ptr::null(), "err5");
}

#[test]
fn error_06_null_src_randomized_sizes() {
    let mut rng = Rng::new(SEED ^ 0x1111);
    for _ in 0..256 {
        let size = rng.next_u64() as u32 as c_int;
        assert_both_null(size, std::ptr::null(), "err6");
    }
    for size in [0, 1, -1, c_int::MAX, c_int::MIN, c_int::MAX - 1, c_int::MIN + 1] {
        assert_both_null(size, std::ptr::null(), "err6 extremes");
    }
}

// ------------------------- rows 7-11: calloc failure from a negative capacity
#[test]
fn error_07_size_minus_four_cap_minus_one() {
    assert_eq!(cap_of(-4), -1, "err7 precondition: cap must be -1");
    assert_both_null(-4, b"payload".as_ptr() as *const c_char, "err7");
}

#[test]
fn error_08_size_minus_five() {
    assert_eq!(cap_of(-5), -2);
    assert_both_null(-5, b"payload".as_ptr() as *const c_char, "err8");
}

#[test]
fn error_09_size_minus_six() {
    assert_eq!(cap_of(-6), -4);
    assert_both_null(-6, b"payload".as_ptr() as *const c_char, "err9");
}

#[test]
fn error_10_every_size_minus_4_to_minus_4096() {
    let src = b"payload".as_ptr() as *const c_char;
    for size in -4096i32..=-4 {
        assert!(cap_of(size) < 0, "err10 precondition for size={size}");
        assert_both_null(size, src, "err10");
    }
}

#[test]
fn error_11_very_negative_sizes_near_int_min() {
    // `size * 4` wraps here; assert equality without assuming the outcome.
    let src = b"payload".as_ptr() as *const c_char;
    for k in 4..=16 {
        let size = c_int::MIN + k;
        assert_same_raw(size, src, "err11");
    }
    for k in 0..=16 {
        let size = c_int::MIN / 2 + k;
        assert_same_raw(size, src, "err11 half");
    }
}

// ------------------- rows 12-13: positive signed overflow => negative capacity
#[test]
fn error_12_positive_overflow_2_pow_29() {
    // `calloc` fails before `src` is touched, so a tiny buffer is safe here.
    let size: c_int = 536_870_912; // 2^29
    assert!(cap_of(size) < 0, "err12 precondition: cap={}", cap_of(size));
    assert_both_null(size, b"tiny".as_ptr() as *const c_char, "err12");
}

#[test]
fn error_13_more_positive_overflow_sizes() {
    let src = b"tiny".as_ptr() as *const c_char;
    // NB: 2^30-3 ..= 2^30 must NOT appear here — for those `size * 4` wraps to
    // -12..=0, so `cap >= 0`, `calloc` succeeds and the C reads ~1 GiB (see the
    // "deliberately EXCLUDED" section of ERRORS.md).
    for size in [
        536_870_913i32,
        600_000_000,
        900_000_000,
        1_073_741_820,
        1_073_741_819,
        1_610_612_736,
        2_000_000_000,
    ] {
        assert!(cap_of(size) < 0, "err13 precondition for size={size}");
        assert_both_null(size, src, "err13");
    }
}

// -------------------------------------- rows 14-17: non-NULL boundary results
#[test]
fn error_14_size_minus_three_zero_capacity() {
    assert_eq!(cap_of(-3), 0, "err14 precondition: calloc(1, 0)");
    // Do not dereference a zero-length allocation; only compare NULL-ness.
    assert_both_non_null(-3, b"payload".as_ptr() as *const c_char, "err14");
}

#[test]
fn error_15_size_minus_one_and_two_return_empty() {
    for (size, cap) in [(-1i32, 3i32), (-2, 2)] {
        assert_eq!(cap_of(size), cap);
        let out = assert_same(size, b"payload", "err15").expect("non-NULL");
        assert!(out.is_empty());
    }
}

#[test]
fn error_16_int_min_wraps_to_capacity_four() {
    assert_eq!(cap_of(c_int::MIN), 4);
    let out = assert_same(c_int::MIN, b"payload", "err16").expect("non-NULL");
    assert!(out.is_empty());
}

#[test]
fn error_17_int_min_plus_one_capacity_five() {
    assert_eq!(cap_of(c_int::MIN + 1), 5);
    let out = assert_same(c_int::MIN + 1, b"payload", "err17").expect("non-NULL");
    assert!(out.is_empty());
}

// ---------------------------------------------- rows 18-20: small valid edges
#[test]
fn error_18_zero_size_empty_string() {
    assert_eq!(cap_of(0), 4);
    let out = assert_same(0, b"\0", "err18").expect("non-NULL");
    assert!(out.is_empty());
}

#[test]
fn error_19_zero_size_leading_nul_with_garbage() {
    let out = assert_same(0, b"\0\xde\xad\xbe\xef", "err19").expect("non-NULL");
    assert!(out.is_empty(), "err19: strlen stops at the first NUL");
}

#[test]
fn error_20_smallest_non_empty_explicit_size() {
    assert_eq!(cap_of(1), 5);
    for v in 0u16..=255 {
        let out = assert_same(1, &[v as u8], "err20").expect("non-NULL");
        assert_eq!(out.len(), 4);
        assert_eq!(&out[2..], b"==");
    }
}

// ------------------------------------- generic boundaries beyond the table
#[test]
fn error_generic_out_of_range_scalar_sweep() {
    // The API takes no enums, so the analogous "value with no valid variant"
    // input is an out-of-range `int size`. Sweep the whole boundary
    // neighbourhood of every documented range, skipping only the values that
    // make the C itself read out of bounds (see ERRORS.md).
    const BUF: &[u8; 16] = b"0123456789abcdef";
    let src = BUF.as_ptr() as *const c_char;

    // Safe to call iff the C either (a) reads only in-bounds bytes,
    // (b) fails in calloc first, or (c) skips the loop entirely.
    let safe = |size: c_int| size <= BUF.len() as c_int || cap_of(size) < 0;

    let mut candidates: Vec<c_int> = Vec::new();
    candidates.extend(-64i32..=16);
    candidates.extend([
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 2,
        c_int::MIN + 3,
        c_int::MIN + 4,
        c_int::MIN / 2,
        -1_000_000,
        -100_000,
        -10_000,
        536_870_911,
        536_870_912,
        536_870_913,
        1_073_741_819,
        1_073_741_820,
        1_073_741_821, // cap >= 0 -> skipped by `safe`
        1_073_741_823, // cap == 3 -> skipped by `safe`
        1_073_741_824, // cap == 4 -> skipped by `safe`
        1_610_612_735, // cap >= 0 -> skipped by `safe`
        1_610_612_736,
        c_int::MAX - 1,
        c_int::MAX, // cap == 3 -> skipped by `safe`
    ]);
    let mut called = 0usize;
    for size in candidates {
        if !safe(size) {
            continue;
        }
        assert_same_raw(size, src, "generic sweep");
        called += 1;
    }
    assert!(called > 80, "generic sweep exercised only {called} values");
}

#[test]
fn error_generic_zero_length_and_pointer_identity() {
    // A zero-length (but non-NULL) buffer in strlen mode.
    let empty: [u8; 1] = [0];
    assert_same(0, &empty, "generic empty").expect("non-NULL");

    // Repeated calls must stay consistent (no hidden state in either .so).
    for _ in 0..64 {
        let out = assert_same(3, b"abc", "generic repeat").expect("non-NULL");
        assert_eq!(out, b"YWJj".to_vec());
    }
}
