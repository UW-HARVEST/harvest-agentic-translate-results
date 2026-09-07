//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so`, and asserts the SAME sentinel (`NULL` vs
//! non-`NULL`) is returned — `UTIL_createLinePointers` has no error codes, its
//! only rejection sentinel is `NULL`.

mod common;

use common::*;
use std::os::raw::c_char;

/// Direct helper: returns `true` when the implementation returned NULL.
fn returned_null(f: CreateLinePointersFn, buf: *mut c_char, n: usize, size: usize) -> bool {
    unsafe { f(buf, n, size).is_null() }
}

/// Assert both sides agree on NULL-ness (used for the huge-allocation rows
/// where we must NOT dereference the returned block).
fn assert_same_nullness(buf: *mut c_char, n: usize, size: usize, ctx: &str) -> bool {
    let l = libs();
    let c_null = returned_null(l.c_fn, buf, n, size);
    let r_null = returned_null(l.rust_fn, buf, n, size);
    assert_eq!(
        c_null, r_null,
        "divergence [{ctx}]: numLines={n} bufferSize={size} \
         (C returned {}, Rust returned {})",
        if c_null { "NULL" } else { "non-NULL" },
        if r_null { "NULL" } else { "non-NULL" },
    );
    c_null
}

/// ERRORS row 1: `malloc(numLines * 8)` fails — astronomically large request.
#[test]
fn err_01_malloc_failure_huge_numlines() {
    let mut buf = *b"a\0b\0";
    let p = buf.as_mut_ptr() as *mut c_char;
    // NOTE: values whose `n * 8` wraps to a *small* number (e.g. `1 << 62`,
    // where `n * 8 == 0 mod 2^64`) are deliberately excluded here: the C would
    // then succeed at `malloc(0)` and write past the block, which is undefined
    // behaviour in both implementations. Those wrap-to-zero values are covered
    // safely in `err_05_*` with `bufferSize == 0`.
    for &n in &[
        usize::MAX / 8,
        usize::MAX / 8 - 1,
        usize::MAX / 2,
        1usize << 60,
        1usize << 59,
    ] {
        let both_null = assert_same_nullness(p, n, buf.len(), &format!("err01 n={n}"));
        assert!(both_null, "expected NULL from malloc failure for numLines={n}");
    }
}

/// ERRORS row 2: wrapping multiplication still yields a huge value.
/// `numLines = SIZE_MAX` -> `SIZE_MAX * 8 mod 2^64 = 0xFFFF_FFFF_FFFF_FFF8`.
#[test]
fn err_02_malloc_failure_wrapping_numlines() {
    let mut buf = *b"a\0b\0";
    let p = buf.as_mut_ptr() as *mut c_char;
    for &n in &[usize::MAX, usize::MAX - 1, usize::MAX - 7] {
        let both_null = assert_same_nullness(p, n, buf.len(), &format!("err02 n={n}"));
        assert!(both_null, "expected NULL for wrapping numLines={n}");
    }
}

/// ERRORS row 3: `bufferSize == 0` with `numLines > 0`.
#[test]
fn err_03_zero_buffersize_nonzero_numlines() {
    let mut buf = vec![0u8; 16];
    for n in 1..=32usize {
        assert_same(&mut buf, n, 0, &format!("err03 n={n}"));
        let l = libs();
        assert!(
            returned_null(l.c_fn, buf.as_mut_ptr() as *mut c_char, n, 0),
            "C should return NULL for bufferSize=0, numLines={n}"
        );
        assert!(
            returned_null(l.rust_fn, buf.as_mut_ptr() as *mut c_char, n, 0),
            "Rust should return NULL for bufferSize=0, numLines={n}"
        );
    }
}

/// ERRORS row 4: buffer holds fewer lines than requested.
#[test]
fn err_04_fewer_lines_than_requested() {
    let mut rng = Rng::new(SEED ^ 104);
    for i in 0..ITERS {
        let count = rng.range(1, 10);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(1, 8);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let size = buf.len();
        let want = count + rng.range(1, 20);
        assert_same(&mut buf, want, size, &format!("err04 iter{i}"));
        let l = libs();
        assert!(
            returned_null(l.c_fn, buf.as_mut_ptr() as *mut c_char, want, size)
                && returned_null(l.rust_fn, buf.as_mut_ptr() as *mut c_char, want, size),
            "both should return NULL: lines={count} want={want}"
        );
    }
}

/// ERRORS row 5: `numLines * 8` wraps to exactly 0 so `malloc(0)` SUCCEEDS,
/// but the line count can never be reached. With `bufferSize == 0` the loop
/// body never runs, so the `lineIndex != numLines` path is taken (free + NULL)
/// — this must NOT be confused with a malloc failure.
#[test]
fn err_05_wrap_to_zero_malloc_succeeds_then_line_mismatch() {
    // 2^61 * 8 == 2^64 == 0 (mod 2^64)
    let wrap_to_zero: usize = 1usize << 61;
    assert_eq!(wrap_to_zero.wrapping_mul(8), 0, "precondition");
    let p = std::ptr::null_mut::<c_char>();
    for &n in &[wrap_to_zero, wrap_to_zero * 2, wrap_to_zero * 3] {
        // bufferSize == 0 keeps the loop from ever writing into the 0-byte block
        let both_null = assert_same_nullness(p, n, 0, &format!("err05 n={n}"));
        assert!(both_null, "expected NULL (line-count mismatch) for numLines={n}");
    }
}

/// ERRORS row 6: `buffer == NULL` with `numLines == 0` is NOT an error.
#[test]
fn err_06_null_buffer_zero_lines() {
    let p = std::ptr::null_mut::<c_char>();
    for &size in &[0usize, 1, 16, 4096, usize::MAX] {
        let both_null = assert_same_nullness(p, 0, size, &format!("err06 size={size}"));
        assert!(
            !both_null,
            "malloc(0) must succeed, so numLines=0 is a success (size={size})"
        );
        // and the (empty) offset vectors must match too
        assert_same_raw(p, 0, size, &format!("err06-offsets size={size}"));
    }
}

/// ERRORS row 7: `buffer == NULL`, `numLines > 0`, `bufferSize == 0`.
#[test]
fn err_07_null_buffer_zero_buffersize() {
    let p = std::ptr::null_mut::<c_char>();
    for n in 1..=32usize {
        let both_null = assert_same_nullness(p, n, 0, &format!("err07 n={n}"));
        assert!(both_null, "expected NULL for NULL buffer, numLines={n}");
        assert_same_raw(p, n, 0, &format!("err07-offsets n={n}"));
    }
}

/// ERRORS row 8: `numLines` exactly one past the number of lines present.
#[test]
fn err_08_off_by_one_past_line_count() {
    let mut rng = Rng::new(SEED ^ 108);
    for i in 0..ITERS {
        let count = rng.range(1, 12);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(0, 8);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let size = buf.len();
        // exactly `count` succeeds, `count + 1` must fail — check both sides agree
        assert_same(&mut buf, count, size, &format!("err08-ok iter{i}"));
        assert_same(&mut buf, count + 1, size, &format!("err08-past iter{i}"));
        let l = libs();
        let p = buf.as_mut_ptr() as *mut c_char;
        assert!(!returned_null(l.c_fn, p, count, size));
        assert!(returned_null(l.c_fn, p, count + 1, size));
        assert!(returned_null(l.rust_fn, p, count + 1, size));
    }
}

/// ERRORS row 9: buffer with no terminator at all and `numLines >= 2`.
#[test]
fn err_09_no_terminator_multiple_lines() {
    let mut rng = Rng::new(SEED ^ 109);
    for i in 0..ITERS {
        let n = rng.range(1, 40);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        let want = rng.range(2, 12);
        assert_same(&mut buf, want, n, &format!("err09 iter{i}"));
        let l = libs();
        let p = buf.as_mut_ptr() as *mut c_char;
        assert!(
            returned_null(l.c_fn, p, want, n) && returned_null(l.rust_fn, p, want, n),
            "both should return NULL: unterminated buffer, want={want}"
        );
    }
}

/// Generic boundary sweep required by Phase C: zero and oversized lengths,
/// one step past every interesting range, and extreme `size_t` values in both
/// parameters (the API takes no enums, so out-of-range "enum" values degenerate
/// into these out-of-range `size_t`s).
#[test]
fn err_10_generic_boundary_sweep() {
    let mut buf = *b"one\0two\0three\0";
    let p = buf.as_mut_ptr() as *mut c_char;
    let interesting_sizes = [0usize, 1, 2, 3, 4, 7, 8, 13, 14];
    let interesting_counts = [0usize, 1, 2, 3, 4, 5, 6, 100];
    for &size in &interesting_sizes {
        for &n in &interesting_counts {
            assert_same_raw(p, n, size, &format!("err10 n={n} size={size}"));
        }
    }
    // Oversized bufferSize values must never be reached because numLines caps
    // the loop; pair them with small numLines so no OOB read occurs.
    for &size in &[usize::MAX, usize::MAX - 1, 1usize << 40] {
        // numLines=0 -> loop never entered, no read at all
        assert_same_raw(p, 0, size, &format!("err10-oversize size={size}"));
    }
    // numLines huge + bufferSize huge -> allocation fails first
    for &n in &[usize::MAX, usize::MAX / 8, 1usize << 60] {
        let both_null = assert_same_nullness(p, n, usize::MAX, &format!("err10-both-max n={n}"));
        assert!(both_null);
    }
}
