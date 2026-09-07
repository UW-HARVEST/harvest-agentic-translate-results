//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary cases (null
//! pointers, zero / oversized lengths, one-past-the-range values, and arbitrary
//! out-of-range scalar values crossing the boundary).
//!
//! Every assertion compares the *exact* return code (22 / 34 / 0) and the whole
//! post-call memory image, not merely "both failed somehow".

mod common;

use common::*;

const SEED: u64 = 0xBADC_0DE0_0FF1_CE55;
const ITERS: usize = 300;

/// Sanity: the C library really returns the codes `ERRORS.md` claims, so the
/// differential assertions below are anchored to absolute values and not just to
/// "C agrees with Rust".
#[test]
fn c_return_codes_are_22_and_34_as_documented() {
    let c = c_wcscat();
    let r = rust_wcscat();
    let src = [b'x' as Wchar, 0];

    // dst == NULL -> 22
    assert_eq!(unsafe { c(std::ptr::null_mut(), 8, src.as_ptr()) }, 22);
    assert_eq!(unsafe { r(std::ptr::null_mut(), 8, src.as_ptr()) }, 22);

    // numElem == 0 -> 22
    let mut b = [0 as Wchar; 4];
    assert_eq!(unsafe { c(b.as_mut_ptr(), 0, src.as_ptr()) }, 22);
    assert_eq!(unsafe { r(b.as_mut_ptr(), 0, src.as_ptr()) }, 22);

    // src == NULL -> 22
    assert_eq!(unsafe { c(b.as_mut_ptr(), 4, std::ptr::null()) }, 22);
    assert_eq!(unsafe { r(b.as_mut_ptr(), 4, std::ptr::null()) }, 22);

    // no room -> 34
    let mut full = [b'a' as Wchar, b'b' as Wchar, 0, 0];
    assert_eq!(unsafe { c(full.as_mut_ptr(), 3, src.as_ptr()) }, 34);
    let mut full = [b'a' as Wchar, b'b' as Wchar, 0, 0];
    assert_eq!(unsafe { r(full.as_mut_ptr(), 3, src.as_ptr()) }, 34);

    // success -> 0
    let mut ok = [0 as Wchar; 4];
    assert_eq!(unsafe { c(ok.as_mut_ptr(), 4, src.as_ptr()) }, 0);
    let mut ok = [0 as Wchar; 4];
    assert_eq!(unsafe { r(ok.as_mut_ptr(), 4, src.as_ptr()) }, 0);
}

// ------------------------------------------------------------------- row 1
#[test]
fn err_row01_null_dst_nonzero_len() {
    let mut rng = Rng::new(SEED + 1);
    for _ in 0..ITERS {
        let src = { let __l = rng.range(0, 12); make_src(&mut rng, __l, false) };
        let n = rng.range(1, 64);
        assert_same("err01", &[], n, Some(&src), true);
    }
    // Plus a pathologically large length: the `!dst` check must short-circuit
    // before anything touches memory.
    let src = make_src(&mut rng, 3, false);
    assert_same("err01/huge", &[], usize::MAX, Some(&src), true);
    assert_same("err01/huge2", &[], usize::MAX / 2, Some(&src), true);
}

// ------------------------------------------------------------------- row 2
#[test]
fn err_row02_null_dst_zero_len() {
    let mut rng = Rng::new(SEED + 2);
    for _ in 0..ITERS {
        let src = { let __l = rng.range(0, 12); make_src(&mut rng, __l, false) };
        assert_same("err02", &[], 0, Some(&src), true);
    }
}

// ------------------------------------------------------------------- row 3
#[test]
fn err_row03_null_dst_and_null_src() {
    for &n in &[0usize, 1, 2, 7, 1024, usize::MAX / 4, usize::MAX] {
        assert_same("err03", &[], n, None, true);
    }
}

// ------------------------------------------------------------------- row 4
#[test]
fn err_row04_zero_len_valid_dst_valid_src() {
    let mut rng = Rng::new(SEED + 4);
    for _ in 0..ITERS {
        let alloc = rng.range(1, 32);
        let plen = rng.range(0, alloc);
        let dst = make_dst(&mut rng, alloc, plen, false);
        let src = { let __l = rng.range(0, 12); make_src(&mut rng, __l, false) };
        // numElem == 0 must leave dst completely untouched (dst[0] is NOT zeroed
        // on this path) — the full-image comparison proves it.
        assert_same("err04", &dst, 0, Some(&src), false);
    }
}

// ------------------------------------------------------------------- row 5
#[test]
fn err_row05_zero_len_valid_dst_null_src() {
    let mut rng = Rng::new(SEED + 5);
    for _ in 0..ITERS {
        let alloc = rng.range(1, 32);
        let plen = rng.range(0, alloc);
        let dst = make_dst(&mut rng, alloc, plen, false);
        assert_same("err05", &dst, 0, None, false);
    }
}

// ------------------------------------------------------------------- row 6
#[test]
fn err_row06_null_src_zeroes_dst0() {
    let mut rng = Rng::new(SEED + 6);
    for _ in 0..ITERS {
        let n = rng.range(1, 40);
        let plen = rng.range(0, n);
        let dst = make_dst(&mut rng, n, plen, false);
        assert_same("err06", &dst, n, None, false);
    }
    // Also with an oversized reported length (still inside the allocation).
    let dst = make_dst(&mut rng, 512, 400, false);
    assert_same("err06/oversized", &dst, 512, None, false);
}

// ------------------------------------------------------------------- row 7
#[test]
fn err_row07_null_src_num_elem_one() {
    let mut rng = Rng::new(SEED + 7);
    for _ in 0..64 {
        let v = rng.nonzero_wchar();
        assert_same("err07", &[v], 1, None, false);
        assert_same("err07/zero", &[0], 1, None, false);
    }
    for &v in EXTREMES {
        assert_same("err07/extreme", &[v], 1, None, false);
    }
}

// ------------------------------------------------------------------- row 8
#[test]
fn err_row08_no_room_partial_fill_then_zero() {
    let mut rng = Rng::new(SEED + 8);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(2, 40);
        let plen = rng.range(0, n - 1); // terminated inside the buffer
        // Make the concatenation too long by a random amount.
        let over = rng.range(1, 20);
        let src_len = n - plen - 1 + over;
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("err08", &dst, n, Some(&src), false);
    }
}

// ------------------------------------------------------------------- row 9
#[test]
fn err_row09_one_element_too_long() {
    let mut rng = Rng::new(SEED + 9);
    for n in 1..=48usize {
        for plen in 0..n {
            let src_len = n - plen; // plen + src_len + 1 == n + 1
            let dst = make_dst(&mut rng, n, plen, false);
            let src = make_src(&mut rng, src_len, false);
            assert_same("err09", &dst, n, Some(&src), false);
            // And the exact-fit neighbour, to pin the boundary from both sides.
            if src_len >= 1 {
                let dst2 = make_dst(&mut rng, n, plen, false);
                let src2 = make_src(&mut rng, src_len - 1, false);
                assert_same("err09/fits", &dst2, n, Some(&src2), false);
            }
        }
    }
}

// ------------------------------------------------------------------ row 10
#[test]
fn err_row10_unterminated_dst_src_never_read() {
    let mut rng = Rng::new(SEED + 10);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 40);
        let dst = make_dst(&mut rng, n, n, false); // unterminated
        let src = { let __l = rng.range(0, 20); make_src(&mut rng, __l, false) };
        assert_same("err10", &dst, n, Some(&src), false);
    }
}

// ------------------------------------------------------------------ row 11
#[test]
fn err_row11_unterminated_num_elem_one() {
    let mut rng = Rng::new(SEED + 11);
    for _ in 0..64 {
        let v = rng.nonzero_wchar();
        let src = { let __l = rng.range(0, 4); make_src(&mut rng, __l, false) };
        assert_same("err11", &[v], 1, Some(&src), false);
    }
    for &v in EXTREMES {
        let src = make_src(&mut rng, 2, false);
        assert_same("err11/extreme", &[v], 1, Some(&src), false);
    }
}

// ------------------------------------------------------------------ row 12
#[test]
fn err_row12_num_elem_one_empty_dst_nonempty_src() {
    let mut rng = Rng::new(SEED + 12);
    for _ in 0..128 {
        let src = { let __l = rng.range(1, 8); make_src(&mut rng, __l, false) };
        assert_same("err12", &[0], 1, Some(&src), false);
    }
}

// ------------------------------------------------------------------ row 13
#[test]
fn err_row13_oversized_num_elem_is_not_validated() {
    let mut rng = Rng::new(SEED + 13);
    // A big real allocation with a big reported length; the data is short, so
    // the C code returns 0 and never notices that `numElem` is absurd relative
    // to the useful content.
    for _ in 0..64 {
        let alloc = 8192;
        let n = rng.range(4096, 8192);
        let plen = rng.range(0, 8);
        let dst = make_dst(&mut rng, alloc, plen, false);
        let src = { let __l = rng.range(0, 8); make_src(&mut rng, __l, false) };
        assert_same("err13", &dst, n, Some(&src), false);
    }
}

// ------------------------------------------------------------------ row 14
#[test]
fn err_row14_out_of_range_scalar_values_across_ffi() {
    // The API has no enum parameter; the equivalent "any int is accepted"
    // surface is the `wchar_t` element type. Values with no valid Unicode
    // meaning must be copied verbatim, and only `== 0` may terminate.
    let mut rng = Rng::new(SEED + 14);
    for &v in EXTREMES {
        for n in 1..=5usize {
            for plen in 0..=n {
                let mut dst = vec![v; n];
                if plen < n {
                    dst[plen] = 0;
                }
                for src_len in 0..=3usize {
                    let mut src = vec![v; src_len];
                    src.push(0);
                    assert_same("err14", &dst, n, Some(&src), false);
                }
            }
        }
    }
    // Mixed random garbage, including negatives.
    for _ in 0..2000 {
        let n = rng.range(1, 16);
        let plen = rng.range(0, n);
        let mut dst = make_dst(&mut rng, n, plen, false);
        for e in dst.iter_mut() {
            if *e != 0 && rng.below(3) == 0 {
                *e = EXTREMES[rng.below(EXTREMES.len())];
            }
        }
        let mut src = { let __l = rng.range(0, 20); make_src(&mut rng, __l, false) };
        let last = src.len() - 1;
        for e in src[..last].iter_mut() {
            if rng.below(3) == 0 {
                *e = EXTREMES[rng.below(EXTREMES.len())];
            }
        }
        assert_same("err14/mixed", &dst, n, Some(&src), false);
    }
}

// ------------------------------------------------- generic boundary sweep
/// Every combination of (dst null?, src null?, numElem in a boundary set).
#[test]
fn generic_null_and_length_boundary_matrix() {
    let mut rng = Rng::new(SEED + 99);
    let lens: [usize; 9] = [
        0,
        1,
        2,
        3,
        8,
        16,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ];
    for &n in &lens {
        for &null_dst in &[true, false] {
            for &null_src in &[true, false] {
                // A non-null dst may only be given a length it can really back,
                // otherwise both implementations would run out of bounds.
                if !null_dst && n > 16 {
                    continue;
                }
                let alloc = if null_dst { 0 } else { 16 };
                let dst = if null_dst {
                    Vec::new()
                } else {
                    { let __l = rng.range(0, alloc); make_dst(&mut rng, alloc, __l, false) }
                };
                let src = { let __l = rng.range(0, 6); make_src(&mut rng, __l, false) };
                let s = if null_src { None } else { Some(&src[..]) };
                assert_same("generic-matrix", &dst, n, s, null_dst);
            }
        }
    }
}
