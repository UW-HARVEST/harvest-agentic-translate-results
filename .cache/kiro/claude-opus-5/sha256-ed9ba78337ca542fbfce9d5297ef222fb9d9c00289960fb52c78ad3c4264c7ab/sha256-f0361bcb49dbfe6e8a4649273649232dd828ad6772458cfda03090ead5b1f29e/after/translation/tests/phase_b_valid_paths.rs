//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH `.so`s through their
//! exported `wcscat` symbol with many randomized inputs (fixed seed) and asserts
//! the return value and the whole post-call buffer image match byte-for-byte.

mod common;

use common::*;

const SEED: u64 = 0xC0FF_EE12_3456_789A;
/// Randomized iterations per row.
const ITERS: usize = 400;

#[test]
fn platform_assumptions() {
    assert_platform_assumptions();
    // Both symbols must resolve through the FFI boundary.
    let _ = c_wcscat();
    let _ = rust_wcscat();
}

// ---------------------------------------------------------------- rows 1..=4
// `dst` empty (dst[0] == 0).

#[test]
fn row01_dst_empty_src_empty_slack() {
    let mut rng = Rng::new(SEED + 1);
    for _ in 0..ITERS {
        let n = rng.range(2, 32);
        let dst = make_dst(&mut rng, n, 0, false);
        let src = make_src(&mut rng, 0, false);
        assert_same("row01", &dst, n, Some(&src), false);
    }
}

#[test]
fn row02_dst_empty_src_len1_slack() {
    let mut rng = Rng::new(SEED + 2);
    for _ in 0..ITERS {
        let n = rng.range(3, 32);
        let dst = make_dst(&mut rng, n, 0, false);
        let src = make_src(&mut rng, 1, false);
        assert_same("row02", &dst, n, Some(&src), false);
    }
}

#[test]
fn row03_dst_empty_src_long_slack() {
    let mut rng = Rng::new(SEED + 3);
    for _ in 0..ITERS {
        let n = rng.range(6, 48);
        let src_len = rng.range(1, n - 2); // leaves at least one element of slack
        let dst = make_dst(&mut rng, n, 0, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row03", &dst, n, Some(&src), false);
    }
}

#[test]
fn row04_dst_empty_src_exact_fit() {
    let mut rng = Rng::new(SEED + 4);
    for _ in 0..ITERS {
        let n = rng.range(1, 40);
        // wcslen(dst)=0, so exact fit means src_len + 1 == n.
        let src_len = n - 1;
        let dst = make_dst(&mut rng, n, 0, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row04", &dst, n, Some(&src), false);
    }
}

// ---------------------------------------------------------------- rows 5..=8
// `dst` non-empty, terminator strictly inside the buffer.

#[test]
fn row05_dst_nonempty_src_empty_slack() {
    let mut rng = Rng::new(SEED + 5);
    for _ in 0..ITERS {
        let n = rng.range(3, 32);
        let plen = rng.range(1, n - 1);
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, 0, false);
        assert_same("row05", &dst, n, Some(&src), false);
    }
}

#[test]
fn row06_dst_nonempty_src_len1_slack() {
    let mut rng = Rng::new(SEED + 6);
    for _ in 0..ITERS {
        let n = rng.range(4, 32);
        let plen = rng.range(1, n - 2);
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, 1, false);
        assert_same("row06", &dst, n, Some(&src), false);
    }
}

#[test]
fn row07_dst_nonempty_src_long_slack() {
    let mut rng = Rng::new(SEED + 7);
    for _ in 0..ITERS {
        let n = rng.range(6, 48);
        let plen = rng.range(1, n - 3);
        let max_src = n - plen - 2; // >= 1, keeps one element of slack
        let src_len = rng.range(1, max_src.max(1));
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row07", &dst, n, Some(&src), false);
    }
}

#[test]
fn row08_dst_nonempty_src_exact_fit() {
    let mut rng = Rng::new(SEED + 8);
    for _ in 0..ITERS {
        let n = rng.range(3, 40);
        let plen = rng.range(1, n - 2);
        let src_len = n - plen - 1; // plen + src_len + 1 == n
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row08", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------- rows 9..=10
// `dst`'s terminator occupies the very last element.

#[test]
fn row09_terminator_in_last_element_src_empty() {
    let mut rng = Rng::new(SEED + 9);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let dst = make_dst(&mut rng, n, n - 1, false); // terminator at n-1
        let src = make_src(&mut rng, 0, false);
        assert_same("row09", &dst, n, Some(&src), false);
    }
}

#[test]
fn row10_terminator_in_last_element_src_nonempty() {
    let mut rng = Rng::new(SEED + 10);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let dst = make_dst(&mut rng, n, n - 1, false);
        let src = { let __l = rng.range(1, 8); make_src(&mut rng, __l, false) };
        assert_same("row10", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------- rows 11..=12

#[test]
fn row11_num_elem_one_empty_dst_empty_src() {
    let mut rng = Rng::new(SEED + 11);
    for _ in 0..ITERS {
        let src = make_src(&mut rng, 0, false);
        assert_same("row11", &[0], 1, Some(&src), false);
    }
}

#[test]
fn row12_residue_after_terminator_preserved() {
    let mut rng = Rng::new(SEED + 12);
    for _ in 0..ITERS {
        let n = rng.range(8, 40);
        let plen = rng.range(0, 3);
        // Short append so a long non-zero residue survives past the new NUL.
        let src_len = rng.range(0, 2);
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row12", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------- rows 13..=14
// `dst` unterminated inside `numElem`.

#[test]
fn row13_dst_unterminated_src_nonempty() {
    let mut rng = Rng::new(SEED + 13);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let dst = make_dst(&mut rng, n, n, false); // no terminator at all
        let src = { let __l = rng.range(1, 8); make_src(&mut rng, __l, false) };
        assert_same("row13", &dst, n, Some(&src), false);
    }
}

#[test]
fn row14_dst_unterminated_src_empty() {
    let mut rng = Rng::new(SEED + 14);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let dst = make_dst(&mut rng, n, n, false);
        let src = make_src(&mut rng, 0, false);
        assert_same("row14", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------- rows 15..=16
// Overflow shapes reached from otherwise valid inputs.

#[test]
fn row15_overflow_by_exactly_one() {
    let mut rng = Rng::new(SEED + 15);
    for _ in 0..ITERS {
        let n = rng.range(2, 40);
        let plen = rng.range(0, n - 1);
        let src_len = n - plen; // plen + src_len + 1 == n + 1
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row15", &dst, n, Some(&src), false);
    }
}

#[test]
fn row16_overflow_by_many() {
    let mut rng = Rng::new(SEED + 16);
    for _ in 0..ITERS {
        let n = rng.range(1, 24);
        let plen = rng.range(0, n - 1);
        let src_len = n + rng.range(1, 32);
        let dst = make_dst(&mut rng, n, plen, false);
        let src = make_src(&mut rng, src_len, false);
        assert_same("row16", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------- rows 17..=18
// `numElem` vs. the real allocation.

#[test]
fn row17_num_elem_smaller_than_allocation() {
    let mut rng = Rng::new(SEED + 17);
    for _ in 0..ITERS {
        let alloc = rng.range(8, 48);
        let n = rng.range(1, alloc);
        // Terminator position chosen inside the *reported* length or beyond it.
        let plen = rng.range(0, alloc);
        let dst = make_dst(&mut rng, alloc, plen.min(alloc), false);
        let src = { let __l = rng.range(0, 6); make_src(&mut rng, __l, false) };
        assert_same("row17", &dst, n, Some(&src), false);
    }
}

#[test]
fn row18_num_elem_oversized_but_within_allocation() {
    let mut rng = Rng::new(SEED + 18);
    for _ in 0..ITERS {
        // Huge reported length; the data terminates long before the real end so
        // the bogus length is never noticed by the C code.
        let alloc = 4096;
        let n = rng.range(2048, 4096);
        let plen = rng.range(0, 16);
        let dst = make_dst(&mut rng, alloc, plen, false);
        let src = { let __l = rng.range(0, 16); make_src(&mut rng, __l, false) };
        assert_same("row18", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------- rows 19..=20
// Element value domains.

#[test]
fn row19_full_range_random_element_values() {
    let mut rng = Rng::new(SEED + 19);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 40);
        let plen = rng.range(0, n);
        let dst = make_dst(&mut rng, n, plen, false);
        let src = { let __l = rng.range(0, 40); make_src(&mut rng, __l, false) };
        assert_same("row19", &dst, n, Some(&src), false);
    }
}

#[test]
fn row20_extreme_element_values() {
    let mut rng = Rng::new(SEED + 20);
    // Exhaustive over the extremes for dst residue / src content.
    for &dv in EXTREMES {
        for &sv in EXTREMES {
            for n in 1..=6usize {
                for plen in 0..=n {
                    let mut dst = vec![dv; n];
                    if plen < n {
                        dst[plen] = 0;
                        for e in dst.iter_mut().skip(plen + 1) {
                            *e = dv;
                        }
                    }
                    for src_len in 0..=4usize {
                        let mut src = vec![sv; src_len];
                        src.push(0);
                        assert_same("row20", &dst, n, Some(&src), false);
                    }
                }
            }
        }
    }
    let _ = rng.next_u64();
}

// -------------------------------------------------------------------- row 21
// The cross-product fuzz sweep.

#[test]
fn row21_cross_product_fuzz() {
    let mut rng = Rng::new(SEED + 21);
    for _ in 0..20_000 {
        let n = rng.range(1, 24);
        // Terminator anywhere in 0..=n, where n means "unterminated".
        let plen = rng.range(0, n);
        let ascii = rng.below(2) == 0;
        let dst = make_dst(&mut rng, n, plen, ascii);
        let src_len = rng.range(0, 30);
        let src = make_src(&mut rng, src_len, ascii);
        assert_same("row21", &dst, n, Some(&src), false);
    }
}

// -------------------------------------------------------------------- row 22
// Exhaustive small shapes.

#[test]
fn row22_exhaustive_small_shapes() {
    let mut rng = Rng::new(SEED + 22);
    for n in 1..=4usize {
        for plen in 0..=n {
            for src_len in 0..=4usize {
                for _ in 0..8 {
                    let dst = make_dst(&mut rng, n, plen, true);
                    let src = make_src(&mut rng, src_len, true);
                    assert_same("row22", &dst, n, Some(&src), false);
                }
            }
        }
    }
}
