//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid
//! input/condition, calls BOTH `.so`s, and asserts the same rejection
//! (the same sentinel — `NULL` vs. a non-NULL block — not merely
//! "both failed somehow").

mod common;

use common::*;
use std::os::raw::c_char;

/// Assert both libraries return the `NULL` sentinel.
fn assert_both_null(bytes: Option<&[u8]>, num_lines: usize, buffer_size: usize, ctx: &str) {
    let p = pair();
    let mut owned: Vec<u8>;
    let base: *mut c_char = match bytes {
        None => std::ptr::null_mut(),
        Some(b) => {
            assert!(buffer_size <= b.len(), "test bug [{ctx}]: window past allocation");
            owned = b.to_vec();
            owned.as_mut_ptr() as *mut c_char
        }
    };
    let (oc, or) = unsafe {
        (
            observe(&p.c, base, num_lines, buffer_size),
            observe(&p.rust, base, num_lines, buffer_size),
        )
    };
    assert!(
        oc.null,
        "expected C to return NULL [{ctx}] (numLines={num_lines}, bufferSize={buffer_size}) \
         but it returned a block with offsets {:?}",
        oc.offsets
    );
    assert!(
        or.null,
        "C returned NULL but Rust returned a block [{ctx}] \
         (numLines={num_lines}, bufferSize={buffer_size}), offsets {:?}",
        or.offsets
    );
    assert_eq!(oc, or, "sentinel mismatch [{ctx}]");
}

/// Assert both libraries return the *same non-NULL* block (same contents).
fn assert_both_non_null(bytes: Option<&[u8]>, num_lines: usize, buffer_size: usize, ctx: &str) {
    let p = pair();
    let mut owned: Vec<u8>;
    let base: *mut c_char = match bytes {
        None => std::ptr::null_mut(),
        Some(b) => {
            assert!(buffer_size <= b.len(), "test bug [{ctx}]: window past allocation");
            owned = b.to_vec();
            owned.as_mut_ptr() as *mut c_char
        }
    };
    let (oc, or) = unsafe {
        (
            observe(&p.c, base, num_lines, buffer_size),
            observe(&p.rust, base, num_lines, buffer_size),
        )
    };
    assert!(!oc.null, "expected C to return non-NULL [{ctx}]");
    assert!(
        !or.null,
        "C returned non-NULL but Rust returned NULL [{ctx}] \
         (numLines={num_lines}, bufferSize={buffer_size})"
    );
    assert_eq!(oc, or, "block contents mismatch [{ctx}]");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — malloc failure via huge numLines (1<<60 => 2^63 bytes)
// ---------------------------------------------------------------------------
#[test]
fn err_01_malloc_failure_huge_numlines() {
    for &k in &[1usize << 60, 1usize << 59, (1usize << 61) - 1] {
        assert_both_null(None, k, 0, "err01-nullbuf");
        assert_both_null(Some(&[b'a', 0, b'b', 0]), k, 4, "err01-realbuf");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — numLines = SIZE_MAX (and SIZE_MAX-1): malloc still fails
// ---------------------------------------------------------------------------
#[test]
fn err_02_malloc_failure_size_max() {
    for &k in &[usize::MAX, usize::MAX - 1, usize::MAX - 7, usize::MAX / 8] {
        assert_both_null(None, k, 0, "err02-nullbuf");
        assert_both_null(Some(&[0u8; 4]), k, 4, "err02-realbuf");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — numLines*8 wraps to 0 => malloc(0) succeeds, then the
//                   lineIndex != numLines path frees and returns NULL.
// ---------------------------------------------------------------------------
#[test]
fn err_03_size_wrap_to_zero_bufsize_zero() {
    for &k in &[1usize << 61, 1usize << 62, 1usize << 63] {
        // numLines * 8 == 0 (mod 2^64) -> malloc(0) is non-NULL on glibc,
        // so line 10 is NOT taken; the rejection must come from line 27-30.
        assert_both_null(None, k, 0, "err03-nullbuf");
        assert_both_null(Some(&[1u8, 2, 3, 4]), k, 0, "err03-realbuf-zero-window");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — numLines*8 wraps to 8 => malloc(8) succeeds, still NULL
// ---------------------------------------------------------------------------
#[test]
fn err_04_size_wrap_to_eight_bufsize_zero() {
    for &k in &[
        (1usize << 61) + 1,
        (1usize << 61) + 2,
        (1usize << 61) + 7,
        (1usize << 62) + 3,
        usize::MAX / 8 + 2,
    ] {
        assert_both_null(None, k, 0, "err04-nullbuf");
        assert_both_null(Some(&[9u8; 16]), k, 0, "err04-realbuf-zero-window");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — bufferSize == 0 with numLines > 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_zero_buffersize_nonzero_numlines() {
    for k in 1..=64usize {
        assert_both_null(None, k, 0, "err05-nullbuf");
        assert_both_null(Some(&[b'x'; 32]), k, 0, "err05-realbuf");
    }
    // and a couple of large-but-allocatable numLines
    for &k in &[1usize << 10, 1usize << 20, 1usize << 24] {
        assert_both_null(Some(&[b'x'; 32]), k, 0, "err05-large");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — fewer segments in the buffer than numLines
// ---------------------------------------------------------------------------
#[test]
fn err_06_fewer_segments_than_numlines() {
    // "a\0b\0" has exactly 2 segments; ask for 3..10
    let bytes = b"a\0b\0".to_vec();
    for k in 3..=10usize {
        assert_both_null(Some(&bytes), k, bytes.len(), "err06-fixed");
    }
    // randomized: k segments present, ask for k+delta
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..4000 {
        let present = rng.range(0, 12);
        let terminate = rng.next_u32() % 2 == 0;
        let b = segments(&mut rng, present, 5, terminate);
        let n = b.len();
        // the real number of lines the C would find
        let found = match model(&b, usize::MAX / 2, n) {
            None => {
                // model returns None because numLines is unreachable; count manually
                let mut pos = 0usize;
                let mut c = 0usize;
                while pos < n {
                    c += 1;
                    let mut len = 0;
                    while pos + len < n && b[pos + len] != 0 {
                        len += 1;
                    }
                    pos += len;
                    if pos < n {
                        pos += 1;
                    }
                }
                c
            }
            Some(v) => v.len(),
        };
        let delta = rng.range(1, 6);
        assert_both_null(Some(&b), found + delta, n, "err06-random");
        // exactly `found` must succeed (unless found == 0 and n == 0 handled below)
        assert_both_non_null(Some(&b), found, n, "err06-boundary-ok");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — off-by-one: exactly k segments, ask for k+1
// ---------------------------------------------------------------------------
#[test]
fn err_07_one_past_valid_numlines() {
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..4000 {
        let k = rng.range(1, 20);
        let terminate = rng.next_u32() % 2 == 0;
        let b = segments(&mut rng, k, 6, terminate);
        if b.is_empty() {
            continue;
        }
        let n = b.len();
        // k is the segment count only when every segment is separated properly;
        // recompute to be exact.
        let found = {
            let mut pos = 0usize;
            let mut c = 0usize;
            while pos < n {
                c += 1;
                let mut len = 0;
                while pos + len < n && b[pos + len] != 0 {
                    len += 1;
                }
                pos += len;
                if pos < n {
                    pos += 1;
                }
            }
            c
        };
        assert_both_non_null(Some(&b), found, n, "err07-at-limit");
        assert_both_null(Some(&b), found + 1, n, "err07-one-past");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — NULL buffer, bufferSize 0, numLines > 0 (no deref)
// ---------------------------------------------------------------------------
#[test]
fn err_08_null_buffer_zero_size_nonzero_lines() {
    for k in 1..=32usize {
        assert_both_null(None, k, 0, "err08");
    }
    assert_both_null(None, 1, 0, "err08-one");
    assert_both_null(None, usize::MAX, 0, "err08-max");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 — NULL buffer, bufferSize 0, numLines 0 => non-NULL
// ---------------------------------------------------------------------------
#[test]
fn err_09_null_buffer_zero_size_zero_lines() {
    assert_both_non_null(None, 0, 0, "err09-nullbuf");
    assert_both_non_null(Some(&[]), 0, 0, "err09-emptyvec");
    assert_both_non_null(Some(&[1u8, 2, 3]), 0, 0, "err09-zero-window");
    assert_both_non_null(Some(&[1u8, 2, 3]), 0, 3, "err09-full-window");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 — extreme scalar matrix across the FFI boundary.
//
// `size_t` has no invalid variants, so the analogue of "out-of-range enum
// value" is every distinguished bit pattern. Pairs are chosen so that none of
// the two documented UB rows (10/11) is hit: whenever `numLines * 8` wraps to
// something smaller than the number of elements that would be written, the
// `bufferSize` is 0 so the loop body never runs.
// ---------------------------------------------------------------------------
#[test]
fn err_12_extreme_scalar_matrix() {
    let extremes: [usize; 16] = [
        0,
        1,
        2,
        7,
        8,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX - 7,
        1usize << 63,
        1usize << 62,
        1usize << 61,
        (1usize << 61) - 1,
        (1usize << 61) + 1,
        1usize << 60,
        usize::MAX / 8,
        usize::MAX / 8 + 1,
    ];

    let backing: Vec<u8> = b"ab\0cd\0\0ef".to_vec();

    for &k in &extremes {
        // bufferSize == 0: always safe, exercises every allocation outcome.
        assert_same_null_buffer(k, 0, "err12-zero-window-nullbuf");

        let p = pair();
        let mut owned = backing.clone();
        let base = owned.as_mut_ptr() as *mut c_char;
        let (oc, or) = unsafe {
            (
                observe(&p.c, base, k, 0),
                observe(&p.rust, base, k, 0),
            )
        };
        assert_eq!(oc, or, "err12 mismatch at numLines={k}, bufferSize=0");

        // Non-zero window only for numLines values whose `*8` does NOT wrap
        // (k <= SIZE_MAX/8) -- then either malloc fails (returns early) or the
        // allocation is genuinely large enough.
        if k <= usize::MAX / 8 {
            for &bs in &[1usize, 2, 3, backing.len()] {
                if bs > backing.len() {
                    continue;
                }
                let mut owned = backing.clone();
                let base = owned.as_mut_ptr() as *mut c_char;
                let (oc, or) = unsafe {
                    (
                        observe(&p.c, base, k, bs),
                        observe(&p.rust, base, k, bs),
                    )
                };
                assert_eq!(
                    oc, or,
                    "err12 mismatch at numLines={k}, bufferSize={bs}"
                );
            }
        }
    }

    // bufferSize extremes with small, safe numLines (bufferSize must stay
    // within the real allocation; SIZE_MAX bufferSize is ERRORS.md row 10 UB
    // and is deliberately not executed).
    for &bs in &[0usize, 1, 2, 8, 9] {
        if bs > backing.len() {
            continue;
        }
        for k in 0..=10usize {
            assert_same_and_model(&backing, k, bs, "err12-bufsize-sweep");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic boundary coverage required by Phase C beyond the table:
// zero and oversized lengths, one step past every valid range.
// ---------------------------------------------------------------------------
#[test]
fn err_generic_boundaries() {
    // zero lengths in every combination
    assert_same_null_buffer(0, 0, "gen-0-0");
    assert_same_and_model(&[], 0, 0, "gen-empty-0-0");

    // one step past the valid numLines for a family of exact buffers
    for k in 0..=24usize {
        let bytes = vec![0u8; k]; // exactly k empty lines
        assert_same_and_model(&bytes, k, k, "gen-exact");
        assert_same_and_model(&bytes, k + 1, k, "gen-exact-plus-1");
        if k > 0 {
            assert_same_and_model(&bytes, k - 1, k, "gen-exact-minus-1");
        }
    }

    // one step past the valid bufferSize window inside a larger allocation
    let backing = b"one\0two\0three\0".to_vec();
    for bs in 0..=backing.len() {
        for k in 0..=6usize {
            assert_same_and_model(&backing, k, bs, "gen-window-sweep");
        }
    }

    // oversized numLines that still allocates fine (1 MiB of pointers)
    assert_same_and_model(&backing, 1 << 17, backing.len(), "gen-oversized-numlines");
}
