//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input,
//! calls BOTH the C `.so` and the Rust `.so` through `dlsym`, and asserts they
//! return the SAME rejection sentinel (`NULL`) — or, for the boundary rows that
//! the C *accepts*, the same non-NULL zero-length success.

mod common;

use common::{count_lines, diff_raw, diff_read, model, Outcome, Rng};
use std::ffi::c_char;
use std::ptr;

fn expect_null(buf: &mut [u8], num_lines: usize, buffer_size: usize, label: &str) {
    // Cross-check against the C algorithm model, then differential-test.
    assert_eq!(
        model(buf, num_lines, buffer_size),
        Outcome::Null,
        "{label}: test bug — the C algorithm does NOT reject this input"
    );
    let o = diff_read(buf, num_lines, buffer_size, 0, label);
    assert_eq!(
        o,
        Outcome::Null,
        "{label}: expected NULL rejection from both impls"
    );
}

/// Call both with an arbitrary (possibly absurd) `numLines`, reading back 0
/// slots, and require the same NULL/non-NULL verdict.
fn diff_verdict(
    buf: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
    label: &str,
) -> Outcome {
    unsafe { diff_raw(buf, num_lines, buffer_size, 0, label) }
}

// ============================================================ row 1
// numLines = SIZE_MAX -> SIZE_MAX*8 wraps to 0xFFFF_FFFF_FFFF_FFF8; malloc fails.
#[test]
fn err_01_num_lines_size_max_malloc_fails() {
    let mut buf = [0u8; 8];
    let p = buf.as_mut_ptr() as *mut c_char;
    for &size in &[0usize, 1, 8] {
        let o = diff_verdict(p, usize::MAX, size, "err01");
        assert_eq!(
            o,
            Outcome::Null,
            "err01: SIZE_MAX lines must be rejected (bufferSize={size})"
        );
    }
}

// ============================================================ row 2
// numLines = 2^60 -> malloc(2^63) = 8 EiB, must fail.
#[test]
fn err_02_num_lines_2pow60_malloc_fails() {
    let mut buf = [0u8; 8];
    let p = buf.as_mut_ptr() as *mut c_char;
    for shift in [60u32, 61, 62, 63] {
        // 2^61*8 wraps to 0, 2^62*8 wraps to 0, 2^63*8 wraps to 0 -> those are
        // row 8's territory; either way C and Rust must agree.
        let n = 1usize << shift;
        let o = diff_verdict(p, n, 0, &format!("err02-2^{shift}"));
        assert_eq!(o, Outcome::Null, "err02: 2^{shift} lines must be rejected");
    }
    // The genuine malloc-failure case (no wrap): 2^60 * 8 == 2^63.
    assert_eq!(
        (1usize << 60).wrapping_mul(8),
        1usize << 63,
        "sanity: 2^60*8 does not wrap"
    );
    let o = diff_verdict(p, 1usize << 60, 8, "err02-nonzero-size");
    assert_eq!(o, Outcome::Null);
}

// ============================================================ row 3
#[test]
fn err_03_zero_buffer_size_one_line() {
    let mut buf = [0u8; 4];
    expect_null(&mut buf, 1, 0, "err03");
}

// ============================================================ row 4
#[test]
fn err_04_zero_buffer_size_many_lines() {
    let mut buf = [0u8; 4];
    for n in [2usize, 3, 10, 1000, 65_536] {
        expect_null(&mut buf, n, 0, "err04");
    }
}

// ============================================================ row 5
#[test]
fn err_05_fewer_lines_than_requested() {
    let mut fixed = b"a\0b\0".to_vec();
    expect_null(&mut fixed, 3, 4, "err05-fixed");
    expect_null(&mut fixed, 4, 4, "err05-fixed4");

    // randomized: ask for strictly more lines than the buffer can yield
    let mut rng = Rng::new(0x2005);
    for case in 0..2000 {
        let cap = 1 + rng.below(64);
        let density = 1 + rng.below(16);
        let mut buf: Vec<u8> = (0..cap)
            .map(|_| {
                if rng.below(density) == 0 {
                    0
                } else {
                    let b = rng.byte();
                    if b == 0 {
                        1
                    } else {
                        b
                    }
                }
            })
            .collect();
        let size = 1 + rng.below(cap);
        let present = count_lines(&buf, size);
        let want = present + 1 + rng.below(8);
        expect_null(&mut buf, want, size, &format!("err05#{case}"));
    }
}

// ============================================================ row 6
#[test]
fn err_06_no_nul_at_all() {
    let mut fixed = b"abcd".to_vec();
    expect_null(&mut fixed, 2, 4, "err06-fixed");

    let mut rng = Rng::new(0x2006);
    for _ in 0..500 {
        let n = 1 + rng.below(40);
        let mut buf: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        assert_eq!(count_lines(&buf, n), 1);
        for want in [2usize, 3, 17] {
            expect_null(&mut buf, want, n, "err06");
        }
    }
}

// ============================================================ row 7
#[test]
fn err_07_num_lines_one_past_max_achievable() {
    // All-NUL buffer: the maximum achievable line count is exactly bufferSize.
    for size in 1..=64usize {
        let mut buf = vec![0u8; size];
        assert_eq!(count_lines(&buf, size), size);
        expect_null(&mut buf, size + 1, size, "err07");
    }
    // Same off-by-one on non-trivial content.
    let mut rng = Rng::new(0x2007);
    for _ in 0..500 {
        let n = 1 + rng.below(20);
        let lens: Vec<usize> = (0..n).map(|_| rng.below(6)).collect();
        let mut buf = Vec::new();
        for &l in &lens {
            for _ in 0..l {
                buf.push(b'z');
            }
            buf.push(0);
        }
        let size = buf.len();
        let present = count_lines(&buf, size);
        expect_null(&mut buf, present + 1, size, "err07-content");
    }
}

// ============================================================ row 8
// numLines = 2^61 -> numLines*8 wraps to 0 -> malloc(0) is NON-NULL, so the
// line-10 check does not fire; rejection must come from line 27 instead.
#[test]
fn err_08_alloc_size_wraps_to_zero() {
    let n = 1usize << 61;
    assert_eq!(n.wrapping_mul(8), 0, "sanity: 2^61*8 wraps to 0");
    let mut buf = [0u8; 8];
    let p = buf.as_mut_ptr() as *mut c_char;
    // bufferSize == 0 -> loop body never runs -> no writes into the 0-byte block.
    let o = diff_verdict(p, n, 0, "err08");
    assert_eq!(o, Outcome::Null);
    // Same for the other wrap-to-zero multiples.
    for n in [1usize << 62, 1usize << 63, (1usize << 61) * 3] {
        assert_eq!(n.wrapping_mul(8), 0);
        assert_eq!(diff_verdict(p, n, 0, "err08-more"), Outcome::Null);
    }
}

// ============================================================ row 9
#[test]
fn err_09_alloc_size_wraps_to_eight_buffer_zero() {
    let n = (1usize << 61) + 1;
    assert_eq!(n.wrapping_mul(8), 8, "sanity: (2^61+1)*8 wraps to 8");
    let mut buf = [0u8; 8];
    let p = buf.as_mut_ptr() as *mut c_char;
    assert_eq!(diff_verdict(p, n, 0, "err09"), Outcome::Null);
}

// ============================================================ row 10
// (2^61+1)*8 == 8 bytes == exactly one pointer slot. With bufferSize == 1 the
// loop writes exactly one slot, then stops, then rejects. No overflow.
#[test]
fn err_10_alloc_size_wraps_to_eight_one_slot_written() {
    let n = (1usize << 61) + 1;
    assert_eq!(n.wrapping_mul(8), 8);
    for content in [0u8, b'A', 0xFF] {
        let mut buf = [content; 1];
        let p = buf.as_mut_ptr() as *mut c_char;
        assert_eq!(
            diff_verdict(p, n, 1, "err10"),
            Outcome::Null,
            "content=0x{content:02x}"
        );
    }
}

// ============================================================ row 11
// (2^61+2)*8 == 16 bytes == exactly two slots; bufferSize == 2 on an all-NUL
// buffer writes exactly two slots.
#[test]
fn err_11_alloc_size_wraps_to_sixteen_two_slots_written() {
    let n = (1usize << 61) + 2;
    assert_eq!(n.wrapping_mul(8), 16, "sanity: (2^61+2)*8 wraps to 16");
    let mut buf = [0u8, 0u8];
    let p = buf.as_mut_ptr() as *mut c_char;
    assert_eq!(diff_verdict(p, n, 2, "err11"), Outcome::Null);

    // (2^61+4)*8 == 32 bytes == 4 slots, bufferSize == 4 all-NUL -> 4 writes.
    let n4 = (1usize << 61) + 4;
    assert_eq!(n4.wrapping_mul(8), 32);
    let mut buf4 = [0u8; 4];
    let p4 = buf4.as_mut_ptr() as *mut c_char;
    assert_eq!(diff_verdict(p4, n4, 4, "err11-four"), Outcome::Null);
}

// ============================================================ row 12
#[test]
fn err_12_null_buffer_zero_size_nonzero_lines() {
    // bufferSize == 0 keeps the loop from ever dereferencing `buffer`.
    for n in [1usize, 2, 7, 1024] {
        let o = diff_verdict(ptr::null_mut(), n, 0, "err12");
        assert_eq!(o, Outcome::Null, "numLines={n}");
    }
}

// ============================================================ row 13
#[test]
fn err_13_null_buffer_zero_lines_zero_size() {
    // Degenerate SUCCESS: malloc(0) is non-NULL and 0 == 0.
    let o = unsafe { diff_raw(ptr::null_mut(), 0, 0, 0, "err13") };
    assert_eq!(
        o,
        Outcome::Ok(vec![]),
        "err13: must NOT be rejected — malloc(0) block is returned"
    );
}

// ============================================================ row 14
#[test]
fn err_14_null_buffer_zero_lines_nonzero_size() {
    for size in [1usize, 100, usize::MAX] {
        let o = unsafe { diff_raw(ptr::null_mut(), 0, size, 0, "err14") };
        assert_eq!(o, Outcome::Ok(vec![]), "bufferSize={size}");
    }
}

// ============================================================ row 15
#[test]
fn err_15_zero_lines_zero_size_real_buffer() {
    let mut buf = [b'x'; 16];
    let p = buf.as_mut_ptr() as *mut c_char;
    let o = unsafe { diff_raw(p, 0, 0, 0, "err15") };
    assert_eq!(o, Outcome::Ok(vec![]));
}

// ==================================================================
// Generic FFI-boundary coverage beyond the table.
//
// This API has no enum parameters (see ERRORS.md), so the "out-of-range enum
// value" class has no instance. The analogous class here is "any size_t bit
// pattern is a legal argument"; the sweep below covers extreme and
// one-past-boundary values for both size_t parameters.
// ==================================================================

#[test]
fn boundary_extreme_size_t_values_agree() {
    let mut buf = [0u8; 16];
    let p = buf.as_mut_ptr() as *mut c_char;

    let extremes: [usize; 14] = [
        0,
        1,
        2,
        7,
        8,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1usize << 32,
        1usize << 48,
        1usize << 60,
        1usize << 61,
        (1usize << 61) + 1,
        (1usize << 63) - 1,
    ];

    for &n in &extremes {
        // bufferSize == 0 is always safe: the loop never executes, so no matter
        // how the allocation size wrapped, nothing is written.
        assert_eq!(
            diff_verdict(p, n, 0, "boundary-n"),
            if n == 0 { Outcome::Ok(vec![]) } else { Outcome::Null },
            "numLines={n}, bufferSize=0"
        );
    }

    for &size in &extremes {
        // numLines == 0 is always safe: the loop never executes and `buffer` is
        // never touched, whatever bufferSize claims.
        assert_eq!(
            diff_verdict(p, 0, size, "boundary-size"),
            Outcome::Ok(vec![]),
            "numLines=0, bufferSize={size}"
        );
    }
}

#[test]
fn boundary_oversized_buffer_size_with_terminated_data() {
    // bufferSize far larger than the requested lines need: the scan stops as
    // soon as numLines lines are found, so the bytes past that are never read.
    // (16 NUL-terminated 1-byte lines live in the first 32 bytes.)
    let mut buf = vec![0u8; 32];
    for i in 0..16 {
        buf[i * 2] = b'q';
        buf[i * 2 + 1] = 0;
    }
    let p = buf.as_mut_ptr() as *mut c_char;
    for &size in &[32usize, 1 << 20, 1 << 40, usize::MAX] {
        // Requesting 16 lines consumes exactly 32 bytes, so no read goes past
        // the real allocation regardless of the bogus `bufferSize`.
        let o = unsafe { diff_raw(p, 16, size, 16, "boundary-oversize") };
        match &o {
            Outcome::Ok(v) => {
                let expect: Vec<isize> = (0..16).map(|i| i * 2).collect();
                assert_eq!(v, &expect, "bufferSize={size}");
            }
            Outcome::Null => panic!("boundary-oversize: unexpected NULL (bufferSize={size})"),
        }
    }
}

#[test]
fn boundary_one_step_past_valid_range_sweep() {
    // For every bufferSize, `count_lines` is the largest satisfiable numLines.
    // Assert accept at the boundary and reject one step past it, in both impls.
    let mut rng = Rng::new(0x2FFF);
    for _ in 0..800 {
        let cap = 1 + rng.below(48);
        let mut buf: Vec<u8> = (0..cap)
            .map(|_| if rng.below(3) == 0 { 0 } else { b'k' })
            .collect();
        let size = 1 + rng.below(cap);
        let present = count_lines(&buf, size);

        // exactly at the maximum -> accepted
        let ok = diff_read(&mut buf, present, size, present, "boundary-at-max");
        assert!(
            matches!(ok, Outcome::Ok(_)),
            "boundary-at-max: numLines={present} bufferSize={size} should succeed"
        );
        // one past -> rejected
        expect_null(&mut buf, present + 1, size, "boundary-past-max");
    }
}
