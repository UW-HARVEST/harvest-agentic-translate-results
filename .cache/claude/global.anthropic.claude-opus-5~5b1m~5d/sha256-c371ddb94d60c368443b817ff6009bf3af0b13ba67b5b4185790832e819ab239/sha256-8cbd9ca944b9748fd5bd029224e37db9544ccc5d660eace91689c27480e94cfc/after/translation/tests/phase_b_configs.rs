//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `wcscat` symbol and compares the return code and the entire destination
//! buffer byte-for-byte. Randomized rows use a fixed seed for reproducibility.

mod common;

use common::*;

/// C1 — minimal accepted call: empty dst, empty src, `numElem == 1`.
#[test]
fn c1_minimal_accepted_call() {
    assert_same("C1", &Case::new(vec![0], &[]));
}

/// C2 — empty dst, empty src, random `numElem` 1..64.
#[test]
fn c2_empty_dst_empty_src_random_num_elem() {
    let mut rng = Rng::new(0xC002);
    for iter in 0..400 {
        let n = rng.range(1, 64);
        for d in Domain::all() {
            // NUL at 0 with a zero-filled tail, then with a garbage tail.
            for garbage in [false, true] {
                let dst = make_dst(&mut rng, n, 0, d, garbage);
                assert_same(&format!("C2 iter={iter} n={n} {d:?} garbage={garbage}"),
                            &Case::new(dst, &[]));
            }
        }
    }
}

/// C3 — empty dst, src fits with slack.
#[test]
fn c3_empty_dst_src_fits_with_slack() {
    let mut rng = Rng::new(0xC003);
    for iter in 0..500 {
        let n = rng.range(2, 64);
        // room = n; a src of length L needs L+1 <= n, slack means L+1 < n.
        let max_len = n - 2;
        let len = rng.range(0, max_len);
        let dst = make_dst(&mut rng, n, 0, Domain::Ascii, iter % 2 == 0);
        let src = Domain::Ascii.vec(&mut rng, len);
        assert_same(&format!("C3 iter={iter} n={n} len={len}"), &Case::new(dst, &src));
    }
}

/// C4 — empty dst, `strlen(src) == numElem - 1`: exact fit.
#[test]
fn c4_empty_dst_exact_fit() {
    let mut rng = Rng::new(0xC004);
    for iter in 0..400 {
        let n = rng.range(1, 64);
        let len = n - 1; // terminator lands on the last window element
        for d in Domain::all() {
            let dst = make_dst(&mut rng, n, 0, d, true);
            let src = d.vec(&mut rng, len);
            assert_same(&format!("C4 iter={iter} n={n} len={len} {d:?}"),
                        &Case::new(dst, &src));
        }
    }
}

/// C5 — true append: `0 < k < numElem-1`, src fits with slack.
#[test]
fn c5_append_src_fits_with_slack() {
    let mut rng = Rng::new(0xC005);
    for iter in 0..600 {
        let n = rng.range(3, 64);
        let k = rng.range(1, n - 2);
        let room = n - k;
        let len = rng.range(0, room - 1);
        let dst = make_dst(&mut rng, n, k, Domain::Ascii, iter % 2 == 0);
        let src = Domain::Ascii.vec(&mut rng, len);
        assert_same(
            &format!("C5 iter={iter} n={n} k={k} len={len}"),
            &Case::new(dst, &src),
        );
    }
}

/// C6 — exact fit on append: `strlen(src) == numElem - k - 1`.
#[test]
fn c6_append_exact_fit() {
    let mut rng = Rng::new(0xC006);
    for iter in 0..500 {
        let n = rng.range(2, 64);
        let k = rng.range(1, n - 1);
        let len = n - k - 1;
        for d in Domain::all() {
            let dst = make_dst(&mut rng, n, k, d, true);
            let src = d.vec(&mut rng, len);
            assert_same(
                &format!("C6 iter={iter} n={n} k={k} len={len} {d:?}"),
                &Case::new(dst, &src),
            );
        }
    }
}

/// C7 — NUL on the last window element; only an empty src still fits.
#[test]
fn c7_nul_on_last_element_empty_src() {
    let mut rng = Rng::new(0xC007);
    for iter in 0..300 {
        let n = rng.range(1, 64);
        let k = n - 1;
        for d in Domain::all() {
            let dst = make_dst(&mut rng, n, k, d, true);
            assert_same(&format!("C7 iter={iter} n={n} k={k} {d:?}"), &Case::new(dst, &[]));
        }
    }
}

/// C8 — full 2x2 of the degenerate `numElem == 1` window.
#[test]
fn c8_num_elem_one_full_cross() {
    let mut rng = Rng::new(0xC008);
    for iter in 0..200 {
        for d in Domain::all() {
            let nonzero = d.gen(&mut rng);
            for dst0 in [0, nonzero] {
                for src_body in [vec![], vec![d.gen(&mut rng)], d.vec(&mut rng, 5)] {
                    assert_same(
                        &format!("C8 iter={iter} {d:?} dst0={dst0} srclen={}", src_body.len()),
                        &Case::new(vec![dst0], &src_body),
                    );
                }
            }
        }
    }
}

/// C9 — dense exhaustive boundary sweep for small `numElem`.
/// Every `k` in `0..=numElem` (including "no NUL in window") crossed with every
/// `strlen(src)` in `0..=numElem+2`, over every value domain. No random gaps.
#[test]
fn c9_dense_exhaustive_small_window() {
    let mut rng = Rng::new(0xC009);
    for n in 1..=8usize {
        for k in 0..=n {
            // k == n means: no NUL anywhere in the window.
            for len in 0..=(n + 2) {
                for d in Domain::all() {
                    for garbage in [false, true] {
                        let dst = make_dst(&mut rng, n, k, d, garbage);
                        let src = d.vec(&mut rng, len);
                        assert_same(
                            &format!("C9 n={n} k={k} len={len} {d:?} garbage={garbage}"),
                            &Case::new(dst, &src),
                        );
                    }
                }
            }
        }
    }
}

/// C10 — short window inside a larger buffer with a non-zero garbage tail past
/// `numElem`; the tail must be left untouched by both implementations.
#[test]
fn c10_short_window_tail_untouched() {
    let mut rng = Rng::new(0xC010);
    for iter in 0..600 {
        let capacity = rng.range(4, 80);
        let n = rng.range(1, capacity);
        let k = rng.range(0, n.saturating_sub(1));
        for d in Domain::all() {
            // Build the window, then append a deliberately non-zero tail.
            let mut dst = make_dst(&mut rng, n, k, d, true);
            for _ in n..capacity {
                dst.push(d.gen(&mut rng)); // never 0
            }
            let len = rng.range(0, n + 2);
            let src = d.vec(&mut rng, len);
            let case = Case::new(dst, &src).num_elem(n);
            assert_same(
                &format!("C10 iter={iter} cap={capacity} n={n} k={k} len={len} {d:?}"),
                &case,
            );
        }
    }
}

/// C11 — a NUL exists only BEYOND the window, so the first loop must saturate
/// and the call must take the `return 34` path.
#[test]
fn c11_nul_only_beyond_window() {
    let mut rng = Rng::new(0xC011);
    for iter in 0..500 {
        let n = rng.range(1, 40);
        let capacity = n + rng.range(1, 20);
        for d in Domain::all() {
            // Window entirely non-zero...
            let mut dst: Vec<WcharT> = (0..n).map(|_| d.gen(&mut rng)).collect();
            // ...NUL placed just past the window, then more garbage.
            dst.push(0);
            while dst.len() < capacity {
                dst.push(d.gen(&mut rng));
            }
            let len = rng.range(0, 6);
            let src = d.vec(&mut rng, len);
            let case = Case::new(dst, &src).num_elem(n);
            assert_same(
                &format!("C11 iter={iter} n={n} cap={capacity} len={len} {d:?}"),
                &case,
            );
        }
    }
}

/// C12 — src containing values above the BMP (16-bit `wchar_t` regression guard).
#[test]
fn c12_above_bmp_values() {
    let mut rng = Rng::new(0xC012);
    // Hand-picked values that a 16-bit truncation would map onto 0 or onto a
    // different codepoint.
    let interesting: Vec<WcharT> = vec![
        0x1_0000, 0x10_FFFF, 0xFFFF, 0x1_0001, 0x2_0000, 0x7FFF_FFFF, 0x0001_0000,
        // 0x...0000 low half: truncating to u16 would yield 0 and falsely terminate.
        0x0002_0000, 0x0003_0000, 0x00FF_0000,
    ];
    for iter in 0..300 {
        let n = rng.range(2, 40);
        let k = rng.range(0, n - 1);
        let room = n - k;
        let len = rng.range(0, room + 2);
        let mut dst = make_dst(&mut rng, n, k, Domain::AboveBmp, true);
        if dst.len() != n {
            dst.resize(n, 1);
        }
        let src: Vec<WcharT> = (0..len)
            .map(|i| interesting[(rng.range(0, interesting.len() - 1) + i) % interesting.len()])
            .collect();
        assert_same(
            &format!("C12 iter={iter} n={n} k={k} len={len}"),
            &Case::new(dst, &src),
        );
    }
}

/// C13 — src containing negative / high-bit-set `wchar_t` (signedness guard).
#[test]
fn c13_negative_src_values() {
    let mut rng = Rng::new(0xC013);
    let interesting: Vec<WcharT> = vec![-1, i32::MIN, i32::MAX, -2, -0x8000, 0x8000_0000u32 as i32];
    for iter in 0..400 {
        let n = rng.range(2, 40);
        let k = rng.range(0, n - 1);
        let room = n - k;
        let len = rng.range(0, room + 2);
        let dst = make_dst(&mut rng, n, k, Domain::Ascii, true);
        let src: Vec<WcharT> = (0..len)
            .map(|_| interesting[rng.range(0, interesting.len() - 1)])
            .collect();
        assert_same(
            &format!("C13 iter={iter} n={n} k={k} len={len}"),
            &Case::new(dst, &src),
        );
    }
}

/// C14 — dst prefix contains negative / high-bit-set values; the first loop must
/// skip them as non-NUL.
#[test]
fn c14_negative_dst_prefix() {
    let mut rng = Rng::new(0xC014);
    for iter in 0..400 {
        let n = rng.range(2, 40);
        let k = rng.range(1, n - 1);
        let dst = make_dst(&mut rng, n, k, Domain::Negative, true);
        let len = rng.range(0, n + 2);
        let src = Domain::Ascii.vec(&mut rng, len);
        assert_same(
            &format!("C14 iter={iter} n={n} k={k} len={len}"),
            &Case::new(dst, &src),
        );
    }
}

/// C15 — non-zero garbage tail after the NUL in dst, src fits with slack: proves
/// no zero-padding is written past the copied terminator.
#[test]
fn c15_no_zero_padding_past_terminator() {
    let mut rng = Rng::new(0xC015);
    for iter in 0..500 {
        let n = rng.range(4, 64);
        let k = rng.range(0, n - 3);
        let room = n - k;
        let len = rng.range(0, room - 2); // strictly fits, leaves slack
        for d in Domain::all() {
            let dst = make_dst(&mut rng, n, k, d, /* garbage_tail */ true);
            let src = d.vec(&mut rng, len);
            assert_same(
                &format!("C15 iter={iter} n={n} k={k} len={len} {d:?}"),
                &Case::new(dst, &src),
            );
        }
    }
}

/// C16 — oversized `numElem` (`1 << 40`) with an early NUL in a small real
/// buffer. The nominal window vastly exceeds the allocation, but the copied NUL
/// terminates the loop long before it, so no out-of-bounds access occurs.
#[test]
fn c16_oversized_num_elem_terminated_early() {
    let mut rng = Rng::new(0xC016);
    for iter in 0..200 {
        let capacity = rng.range(8, 64);
        let len = rng.range(0, 5);
        for d in Domain::all() {
            // NUL at index 0, plenty of real room behind it.
            let dst = make_dst(&mut rng, capacity, 0, d, true);
            let src = d.vec(&mut rng, len);
            let case = Case::new(dst, &src).num_elem(1usize << 40);
            assert_same(
                &format!("C16 iter={iter} cap={capacity} len={len} {d:?}"),
                &case,
            );
        }
    }
}

/// C17 — fully randomized fuzz across every axis simultaneously.
#[test]
fn c17_full_axis_fuzz() {
    let mut rng = Rng::new(0xC017);
    for iter in 0..8000 {
        let capacity = rng.range(1, 96);
        let n = rng.range(1, capacity);
        // nul_at == capacity (or beyond) means "no NUL at all".
        let nul_at = rng.range(0, capacity + 1);
        let d = Domain::all()[rng.range(0, 3)];
        let garbage = rng.range(0, 1) == 1;

        let mut dst = make_dst(&mut rng, capacity, nul_at, d, garbage);
        dst.resize(capacity, d.gen(&mut rng));

        // src length spans fitting and overflowing cases.
        let len = rng.range(0, capacity + 4);
        let src = d.vec(&mut rng, len);

        let case = Case::new(dst, &src).num_elem(n);
        assert_same(
            &format!("C17 iter={iter} cap={capacity} n={n} nul_at={nul_at} len={len} {d:?} garbage={garbage}"),
            &case,
        );
    }
}

/// C17b — fuzz with `src` NOT NUL-terminated within its own allocation is
/// deliberately excluded (it would be an out-of-bounds read in C). Instead this
/// fuzzes `src` buffers that contain *interior* NULs, which is well-defined and
/// must stop the copy at the first one.
#[test]
fn c17b_interior_nul_in_src() {
    let mut rng = Rng::new(0xC17B);
    for iter in 0..2000 {
        let capacity = rng.range(1, 64);
        let n = rng.range(1, capacity);
        let nul_at = rng.range(0, capacity);
        let d = Domain::all()[rng.range(0, 3)];

        let dst = make_dst(&mut rng, capacity, nul_at, d, true);

        // src with a NUL somewhere in the middle, plus trailing junk, plus a
        // final NUL so it is always a valid C string.
        let head = rng.range(0, 10);
        let mut src = d.vec(&mut rng, head);
        src.push(0);
        let tail = rng.range(0, 10);
        src.extend(d.vec(&mut rng, tail));
        src.push(0);

        let case = Case::new(dst, &[]).num_elem(n).raw_src(src);
        assert_same(
            &format!("C17b iter={iter} cap={capacity} n={n} nul_at={nul_at} head={head} tail={tail} {d:?}"),
            &case,
        );
    }
}
