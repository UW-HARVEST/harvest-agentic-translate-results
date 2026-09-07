//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary cases
//! (null pointers, zero and oversized lengths, values one past a valid range,
//! out-of-range integer values crossing the boundary).

mod common;
use common::*;

/// Row 1 — malloc failure via an enormous (non-wrapping) byte count.
#[test]
fn err_01_malloc_failure() {
    // SIZE_MAX/16 * 8 == 0x7FFF...F8: does not wrap, but cannot be allocated.
    let num_lines = usize::MAX / 16;
    let mut buf = vec![0u8; 16];
    let out = assert_same("err01", &mut buf, num_lines, 16);
    assert_eq!(out, Outcome::Null, "malloc failure must yield NULL");
}

/// Row 2 — bufferSize == 0 with numLines > 0.
#[test]
fn err_02_zero_buffersize_nonzero_numlines() {
    let mut rng = Rng::new(0xE002);
    let mut buf = vec![0u8; 32];
    for _ in 0..200 {
        let n = rng.range(1, 4096);
        let out = assert_same("err02", &mut buf, n, 0);
        assert_eq!(out, Outcome::Null, "numLines={n}, bufferSize=0");
    }
}

/// Row 3 — fewer NUL-separated segments in the buffer than numLines.
#[test]
fn err_03_fewer_lines_than_requested() {
    let mut rng = Rng::new(0xE003);
    for _ in 0..300 {
        let k = rng.range(1, 64);
        let (mut buf, _) = build_segments(&mut rng, k, 12, true);
        let n = buf.len();
        let real = scan_offsets(&buf, n).len();
        let ask = real + rng.range(1, 8);
        let out = assert_same("err03", &mut buf, ask, n);
        assert_eq!(out, Outcome::Null, "real={real} ask={ask} size={n}");
    }
    // The exact literal from ERRORS.md.
    let mut buf = b"a\0".to_vec();
    assert_eq!(assert_same("err03-lit", &mut buf, 3, 2), Outcome::Null);
}

/// Row 4 — unterminated tail exhausts the buffer while lines remain.
#[test]
fn err_04_unterminated_tail_short() {
    let mut buf = b"abc".to_vec();
    assert_eq!(assert_same("err04-lit", &mut buf, 2, 3), Outcome::Null);

    let mut rng = Rng::new(0xE004);
    for _ in 0..300 {
        let len = rng.range(1, 256);
        let mut b: Vec<u8> = (0..len).map(|_| rng.nz_byte()).collect();
        // Exactly one segment is reachable; asking for >= 2 must fail.
        let ask = rng.range(2, 8);
        let out = assert_same("err04", &mut b, ask, len);
        assert_eq!(out, Outcome::Null, "len={len} ask={ask}");
    }
}

/// Row 5 — buffer == NULL, bufferSize == 0, numLines > 0 (no deref).
#[test]
fn err_05_null_buffer_zero_size() {
    let mut rng = Rng::new(0xE005);
    for _ in 0..200 {
        let n = rng.range(1, 1 << 20);
        let out = assert_same_raw("err05", std::ptr::null_mut(), n, 0);
        assert_eq!(out, Outcome::Null, "numLines={n}");
    }
}

/// Row 6 — size_t multiplication wraps exactly to 0 (numLines = 2^61).
#[test]
fn err_06_size_overflow_wrap_to_zero() {
    let num_lines = 1usize << 61;
    assert_eq!(num_lines.wrapping_mul(8), 0, "precondition: product wraps to 0");
    let out = assert_same_raw("err06", std::ptr::null_mut(), num_lines, 0);
    assert_eq!(out, Outcome::Null);
}

/// Row 7 — multiplication wraps to a small non-zero value (numLines = 2^61+1).
#[test]
fn err_07_size_overflow_wrap_to_small() {
    let num_lines = (1usize << 61) + 1;
    assert_eq!(num_lines.wrapping_mul(8), 8, "precondition: product wraps to 8");
    let out = assert_same_raw("err07", std::ptr::null_mut(), num_lines, 0);
    assert_eq!(out, Outcome::Null);

    // A few more wrap targets, all with bufferSize == 0 so no OOB store happens.
    for extra in [2usize, 3, 7, 1024, (1 << 20) + 5] {
        let nl = (1usize << 61) + extra;
        let out = assert_same_raw("err07-multi", std::ptr::null_mut(), nl, 0);
        assert_eq!(out, Outcome::Null, "numLines=2^61+{extra}");
    }
}

/// Row 8 — numLines = SIZE_MAX (and neighbours): allocation must fail.
#[test]
fn err_08_numlines_size_max() {
    let mut buf = vec![0u8; 8];
    // Every value here yields a byte count that is still astronomically large
    // (with or without wrap), so `malloc` fails and the C returns NULL before
    // touching the buffer.
    for nl in [
        usize::MAX,          // *8 wraps to 0xFFFF_FFFF_FFFF_FFF8 — still huge
        usize::MAX - 1,      // *8 wraps to 0xFFFF_FFFF_FFFF_FFF0 — still huge
        usize::MAX / 2,      // *8 wraps to 0xFFFF_FFFF_FFFF_FFF8 — still huge
        usize::MAX / 8,      // *8 == 0xFFFF_FFFF_FFFF_FFF8, no wrap
        usize::MAX / 16,     // *8 == 0x7FFF_FFFF_FFFF_FFF8, no wrap
    ] {
        assert_ne!(nl.wrapping_mul(8), 0, "must not wrap to a tiny allocation");
        let out = assert_same("err08", &mut buf, nl, 8);
        assert_eq!(out, Outcome::Null, "numLines={nl}");
    }

    // `usize::MAX/8 + 1` is the exact wrap-to-zero point: `malloc(0)` SUCCEEDS,
    // so it is only observable with bufferSize == 0 (otherwise both C and Rust
    // store past the zero-length block — identical UB, not differentiable).
    let wrap_to_zero = (usize::MAX / 8) + 1;
    assert_eq!(wrap_to_zero.wrapping_mul(8), 0);
    assert_eq!(
        assert_same_raw("err08-wrap0", std::ptr::null_mut(), wrap_to_zero, 0),
        Outcome::Null
    );
}

/// Row 9 — numLines == 0 is NOT an error: glibc malloc(0) is non-NULL.
#[test]
fn err_09_zero_lines_is_not_an_error() {
    let mut buf = vec![0u8; 4];
    let out = assert_same("err09", &mut buf, 0, 0);
    assert_eq!(
        out,
        Outcome::Ok(vec![]),
        "numLines=0 must return a non-NULL zero-length block"
    );
}

/// Row 10 — numLines == 0 with buffer == NULL and a large bufferSize.
#[test]
fn err_10_zero_lines_null_buffer() {
    for size in [0usize, 1, 4096, usize::MAX / 2, usize::MAX] {
        let out = assert_same_raw("err10", std::ptr::null_mut(), 0, size);
        assert_eq!(out, Outcome::Ok(vec![]), "bufferSize={size}");
    }
}

/// Row 11 — oversized bufferSize (SIZE_MAX) but enough NULs before real end.
#[test]
fn err_11_oversized_buffersize_but_enough_nuls() {
    let mut rng = Rng::new(0xE011);
    for _ in 0..200 {
        let k = rng.range(1, 32);
        let (mut buf, offs) = build_segments(&mut rng, k, 8, true);
        // bufferSize is absurd, but the loop stops as soon as k lines are found,
        // which happens strictly inside the real allocation.
        let out = assert_same("err11", &mut buf, k, usize::MAX);
        assert_eq!(
            out,
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect()),
            "k={k}"
        );
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary cases (beyond the table)
// ---------------------------------------------------------------------------

/// Off-by-one around the exact-fit boundary: asking for exactly the number of
/// reachable lines succeeds; one more fails; one fewer succeeds.
#[test]
fn boundary_off_by_one_around_line_count() {
    let mut rng = Rng::new(0xB001);
    for _ in 0..400 {
        let k = rng.range(1, 64);
        let term = rng.bool();
        let (mut buf, offs) = build_segments(&mut rng, k, 12, term);
        let n = buf.len();
        let real = scan_offsets(&buf, n);
        assert_eq!(real.len(), k);

        assert_eq!(
            assert_same("bnd-exact", &mut buf, k, n),
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect())
        );
        assert_eq!(assert_same("bnd-plus1", &mut buf, k + 1, n), Outcome::Null);
        if k > 1 {
            assert_eq!(
                assert_same("bnd-minus1", &mut buf, k - 1, n),
                Outcome::Ok(offs[..k - 1].iter().map(|&o| Some(o as isize)).collect())
            );
        }
    }
}

/// bufferSize one past / one before the real allocation end.
#[test]
fn boundary_buffersize_one_step_past_range() {
    let mut rng = Rng::new(0xB002);
    for _ in 0..300 {
        let k = rng.range(1, 32);
        let (mut buf, offs) = build_segments(&mut rng, k, 8, true);
        let n = buf.len();
        // Exactly the allocation: OK for k lines.
        assert_eq!(
            assert_same("bnd-size-exact", &mut buf, k, n),
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect())
        );
        // One byte short: the last NUL is invisible, but the segment start is
        // still recorded, so the reference model decides the outcome.
        let short = n - 1;
        let expect = scan_offsets(&buf, short);
        let out = assert_same("bnd-size-short", &mut buf, expect.len(), short);
        assert_eq!(
            out,
            Outcome::Ok(expect.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Both zero: the fully degenerate call, repeated to catch allocator state
/// dependence.
#[test]
fn boundary_all_zero_arguments_repeated() {
    for _ in 0..1000 {
        let out = assert_same_raw("bnd-zeros", std::ptr::null_mut(), 0, 0);
        assert_eq!(out, Outcome::Ok(vec![]));
    }
}

/// Out-of-range integer values crossing the FFI boundary. This API has no enum,
/// so the analogous inputs are `size_t` values with no meaningful
/// interpretation: wrap-inducing, allocation-defeating, and SIZE_MAX values in
/// both positions. Every combination must agree between C and Rust.
#[test]
fn boundary_out_of_range_integer_matrix() {
    let mut buf = b"a\0b\0c\0".to_vec();
    let interesting: [usize; 10] = [
        0,
        1,
        2,
        3,
        6,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1usize << 61,
        (1usize << 61) + 1,
    ];
    for &num_lines in &interesting {
        for &buffer_size in &interesting {
            // Skip the combinations where BOTH implementations perform the
            // identical undefined behaviour and abort the process:
            //  * reading past the 6-byte allocation, and
            //  * storing into an allocation undersized by integer wrap.
            let scan_may_overrun = buffer_size > buf.len();
            let alloc_wraps = num_lines >= (1usize << 61);
            if num_lines > 0 && buffer_size > 0 && (scan_may_overrun || alloc_wraps) {
                continue;
            }
            let out = assert_same("bnd-matrix", &mut buf, num_lines, buffer_size);
            // Model the C exactly.
            let expect = if num_lines == 0 {
                Outcome::Ok(vec![])
            } else if num_lines >= (1usize << 61) || num_lines > usize::MAX / 8 {
                // Either wraps to an undersized/zero allocation (but the loop is
                // skipped because bufferSize == 0), or the allocation fails.
                Outcome::Null
            } else {
                let offs = scan_offsets(&buf, buffer_size.min(buf.len()));
                if num_lines <= offs.len() {
                    Outcome::Ok(offs[..num_lines].iter().map(|&o| Some(o as isize)).collect())
                } else {
                    Outcome::Null
                }
            };
            assert_eq!(out, expect, "numLines={num_lines} bufferSize={buffer_size}");
        }
    }
}
