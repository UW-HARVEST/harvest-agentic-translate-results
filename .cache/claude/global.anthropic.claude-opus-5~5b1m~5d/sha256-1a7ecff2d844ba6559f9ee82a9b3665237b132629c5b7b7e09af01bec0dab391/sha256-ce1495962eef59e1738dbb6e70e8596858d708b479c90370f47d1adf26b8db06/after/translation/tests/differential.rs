//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH `libdriver.so`
//! (C) and `libdriver.so` (Rust) via `libloading` and calls the exported
//! `UTIL_createLinePointers` through the FFI boundary.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// CONFIGS.md row 1 — numLines = 0, bufferSize = 0, buffer = NULL
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_zero_lines_zero_size_null_buffer() {
    assert_same_null_buffer(0, 0, "cfg01");
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 2 — numLines = 0, bufferSize = 0, valid non-null buffer
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_zero_lines_zero_size_valid_buffer() {
    for n in 0..=8usize {
        let bytes = vec![b'x'; n];
        assert_same_and_model(&bytes, 0, 0, "cfg02");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 3 — numLines = 0, bufferSize > 0, random bytes
// ---------------------------------------------------------------------------
#[test]
fn cfg_03_zero_lines_nonzero_size_random() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        let n = rng.range(1, 64);
        let bytes = rng.bytes(n, 25);
        assert_same_and_model(&bytes, 0, n, "cfg03");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 4 — numLines = 1, bufferSize = 1, buffer = "\0"
// ---------------------------------------------------------------------------
#[test]
fn cfg_04_one_line_one_byte_nul() {
    assert_same_and_model(&[0u8], 1, 1, "cfg04");
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 5 — numLines = 1, bufferSize = 1, buffer = "A" (no NUL)
// ---------------------------------------------------------------------------
#[test]
fn cfg_05_one_line_one_byte_no_nul() {
    for b in 1u16..=255 {
        assert_same_and_model(&[b as u8], 1, 1, "cfg05");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 6 — numLines = 1, no NUL anywhere
// ---------------------------------------------------------------------------
#[test]
fn cfg_06_one_line_no_nul_anywhere() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..2000 {
        let n = rng.range(2, 64);
        let bytes: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        assert_same_and_model(&bytes, 1, n, "cfg06");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 7 — numLines = 1, single NUL strictly inside
// ---------------------------------------------------------------------------
#[test]
fn cfg_07_one_line_nul_strictly_inside() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..2000 {
        let n = rng.range(2, 64);
        let mut bytes: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        let nul_at = rng.below(n - 1); // strictly inside => not the last byte
        bytes[nul_at] = 0;
        assert_same_and_model(&bytes, 1, n, "cfg07");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 8 — exactly k terminated segments, exact fit
// ---------------------------------------------------------------------------
#[test]
fn cfg_08_exact_k_terminated_segments() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..3000 {
        let k = rng.range(1, 16);
        let bytes = segments(&mut rng, k, 6, true);
        let n = bytes.len();
        assert_same_and_model(&bytes, k, n, "cfg08");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 9 — k segments, last one unterminated
// ---------------------------------------------------------------------------
#[test]
fn cfg_09_last_segment_unterminated() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..3000 {
        let k = rng.range(1, 16);
        let bytes = segments(&mut rng, k, 6, false);
        if bytes.is_empty() {
            continue;
        }
        let n = bytes.len();
        assert_same_and_model(&bytes, k, n, "cfg09");
        // one fewer / one more requested line, same buffer
        assert_same_and_model(&bytes, k.saturating_sub(1), n, "cfg09-minus1");
        assert_same_and_model(&bytes, k + 1, n, "cfg09-plus1");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 10 — all-NUL buffer, bufferSize == numLines
// ---------------------------------------------------------------------------
#[test]
fn cfg_10_all_nul_exact() {
    for k in 1..=64usize {
        let bytes = vec![0u8; k];
        assert_same_and_model(&bytes, k, k, "cfg10");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 11 — all-NUL buffer, bufferSize > numLines (early stop)
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_all_nul_early_stop() {
    for k in 0..=32usize {
        for extra in 1..=8usize {
            let bytes = vec![0u8; k + extra];
            assert_same_and_model(&bytes, k, k + extra, "cfg11");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 12 — numLines strictly less than the segment count
// ---------------------------------------------------------------------------
#[test]
fn cfg_12_numlines_less_than_segments() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..3000 {
        let present = rng.range(2, 20);
        let bytes = segments(&mut rng, present, 5, true);
        let n = bytes.len();
        let want = rng.below(present); // 0 ..= present-1
        assert_same_and_model(&bytes, want, n, "cfg12");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 13 — mixed empty / non-empty segments (consecutive NULs)
// ---------------------------------------------------------------------------
#[test]
fn cfg_13_mixed_empty_and_nonempty() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..3000 {
        let k = rng.range(1, 20);
        let mut bytes = Vec::new();
        for _ in 0..k {
            // ~40% chance of an empty segment
            let len = if rng.next_u32() % 100 < 40 {
                0
            } else {
                rng.range(1, 5)
            };
            for _ in 0..len {
                let b = rng.byte();
                bytes.push(if b == 0 { 1 } else { b });
            }
            bytes.push(0);
        }
        let n = bytes.len();
        assert_same_and_model(&bytes, k, n, "cfg13");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 14 — leading NUL (first line empty)
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_leading_nul() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..2000 {
        let k = rng.range(2, 12);
        let mut bytes = vec![0u8];
        let rest = segments(&mut rng, k - 1, 6, true);
        bytes.extend_from_slice(&rest);
        let n = bytes.len();
        assert_same_and_model(&bytes, k, n, "cfg14");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 15 — trailing run of NULs longer than needed
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_trailing_nul_run() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..2000 {
        let k = rng.range(1, 12);
        let mut bytes = segments(&mut rng, k, 6, true);
        let pad = rng.range(1, 10);
        bytes.extend(std::iter::repeat(0u8).take(pad));
        let n = bytes.len();
        assert_same_and_model(&bytes, k, n, "cfg15");
        // asking for the padded lines too must still succeed
        assert_same_and_model(&bytes, k + pad, n, "cfg15-with-pad");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 16 — high-bit bytes (signed `char` on x86-64)
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_high_bit_bytes() {
    // every single high-bit byte on its own
    for b in 0x80u16..=0xFF {
        assert_same_and_model(&[b as u8], 1, 1, "cfg16-single");
        assert_same_and_model(&[b as u8, 0, b as u8], 2, 3, "cfg16-triple");
    }
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..2000 {
        let n = rng.range(1, 48);
        let bytes: Vec<u8> = (0..n)
            .map(|_| {
                if rng.next_u32() % 100 < 20 {
                    0
                } else {
                    0x80 | (rng.byte() & 0x7F)
                }
            })
            .collect();
        for k in 0..=6usize {
            assert_same_and_model(&bytes, k, n, "cfg16-fuzz");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 17 — '\n', '\r', 0x7F are NOT separators
// ---------------------------------------------------------------------------
#[test]
fn cfg_17_newlines_are_not_separators() {
    let bytes = b"ab\ncd\r\nef\x7fgh".to_vec();
    let n = bytes.len();
    for k in 0..=4usize {
        assert_same_and_model(&bytes, k, n, "cfg17-nonul");
    }
    let bytes2 = b"ab\ncd\0e\rf\0\n".to_vec();
    let n2 = bytes2.len();
    for k in 0..=5usize {
        assert_same_and_model(&bytes2, k, n2, "cfg17-mixed");
    }
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..1500 {
        let n = rng.range(1, 40);
        let alphabet = [b'\n', b'\r', 0x7F, b'a', 0u8];
        let bytes: Vec<u8> = (0..n).map(|_| alphabet[rng.below(5)]).collect();
        for k in 0..=8usize {
            assert_same_and_model(&bytes, k, n, "cfg17-fuzz");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 18 — large scale (1000 lines, ~16 KiB)
// ---------------------------------------------------------------------------
#[test]
fn cfg_18_large_scale() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..20 {
        let k = 1000usize;
        let bytes = segments(&mut rng, k, 30, true);
        let n = bytes.len();
        assert_same_and_model(&bytes, k, n, "cfg18-exact");
        assert_same_and_model(&bytes, k / 2, n, "cfg18-half");
        assert_same_and_model(&bytes, k + 1, n, "cfg18-plus1");
        assert_same_and_model(&bytes, n, n, "cfg18-bytes-as-lines");
    }
    // one unterminated 64 KiB blob, a single "line"
    let big = rng.bytes(64 * 1024, 0);
    let n = big.len();
    assert_same_and_model(&big, 1, n, "cfg18-blob");
    assert_same_and_model(&big, 2, n, "cfg18-blob-2");
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 19 — numLines = 1, fully random buffer
// ---------------------------------------------------------------------------
#[test]
fn cfg_19_one_line_random_buffer() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..5000 {
        let n = rng.range(1, 64);
        let density = [0u32, 5, 25, 50, 100][rng.below(5)];
        let bytes = rng.bytes(n, density);
        assert_same_and_model(&bytes, 1, n, "cfg19");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 20 — full fuzz, ~25% NUL density
// ---------------------------------------------------------------------------
#[test]
fn cfg_20_fuzz_mixed_density() {
    let mut rng = Rng::new(SEED ^ 20);
    let mut null_seen = 0usize;
    let mut ok_seen = 0usize;
    for _ in 0..20_000 {
        let n = rng.below(49); // 0..=48
        let k = rng.below(25); // 0..=24
        let bytes = rng.bytes(n, 25);
        assert_same_and_model(&bytes, k, n, "cfg20");
        match model(&bytes, k, n) {
            None => null_seen += 1,
            Some(_) => ok_seen += 1,
        }
    }
    assert!(
        null_seen > 100 && ok_seen > 100,
        "fuzz did not cover both outcomes: NULL={null_seen} OK={ok_seen}"
    );
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 21 — full fuzz, NUL-dense (~75%)
// ---------------------------------------------------------------------------
#[test]
fn cfg_21_fuzz_nul_dense() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..20_000 {
        let n = rng.below(49);
        let k = rng.below(25);
        let bytes = rng.bytes(n, 75);
        assert_same_and_model(&bytes, k, n, "cfg21");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 22 — full fuzz, NUL-free
// ---------------------------------------------------------------------------
#[test]
fn cfg_22_fuzz_nul_free() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..20_000 {
        let n = rng.below(49);
        let k = rng.below(9);
        let bytes = rng.bytes(n, 0);
        assert_same_and_model(&bytes, k, n, "cfg22");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 23 — exhaustive small-input sweep
// ---------------------------------------------------------------------------
#[test]
fn cfg_23_exhaustive_small_sweep() {
    // Exhaustive over every NUL/non-NUL mask for bufferSize 0..=12,
    // crossed with numLines 0..=14.
    for n in 0..=12usize {
        for mask in 0u32..(1u32 << n) {
            let bytes: Vec<u8> = (0..n)
                .map(|i| if mask >> i & 1 == 1 { 0u8 } else { b'a' })
                .collect();
            for k in 0..=14usize {
                assert_same_and_model(&bytes, k, n, "cfg23");
            }
        }
    }
    // bufferSize 13..=17 with randomized masks (2^17 * 19 would be slow)
    let mut rng = Rng::new(SEED ^ 23);
    for n in 13..=17usize {
        for _ in 0..1000 {
            let bytes: Vec<u8> = (0..n)
                .map(|_| if rng.next_u32() % 2 == 0 { 0u8 } else { b'a' })
                .collect();
            for k in 0..=18usize {
                assert_same_and_model(&bytes, k, n, "cfg23-wide");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 24 — size multiplication wraps, defined outcome
// ---------------------------------------------------------------------------
#[test]
fn cfg_24_size_multiplication_wrap_defined() {
    // numLines * 8 wraps; bufferSize == 0 so the loop body never runs and the
    // (undersized) allocation is never written -> both must return NULL.
    let wrapping = [
        1usize << 61,           // *8 == 0
        (1usize << 61) + 1,     // *8 == 8
        (1usize << 61) + 2,     // *8 == 16
        (1usize << 61) | 0xFF,  // *8 == 0x7F8
        1usize << 62,           // *8 == 0
        1usize << 63,           // *8 == 0
        usize::MAX / 8 + 1,     // == 1<<61
    ];
    for &k in &wrapping {
        assert_same_null_buffer(k, 0, "cfg24");
    }
    // Same values with a real (but zero-length window) buffer.
    for &k in &wrapping {
        assert_same_and_model(&[1u8, 2, 3, 4], k, 0, "cfg24-realbuf");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 25 — allocation-failure configuration
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_allocation_failure_matrix() {
    let huge = [
        1usize << 60,
        1usize << 59,
        (1usize << 60) + 12345,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 8,
    ];
    for &k in &huge {
        assert_same_null_buffer(k, 0, "cfg25-nullbuf");
        assert_same_and_model(&[0u8; 8], k, 0, "cfg25-zero-window");
        // With a real window the C would write linePointers[0] -- but the
        // allocation failed, so it returns before the loop. Safe to run.
        assert_same_and_model(&[b'a', 0, b'b', 0, 0, 0, 0, 0], k, 8, "cfg25-real-window");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 26 — statelessness / repeated interleaved invocations
// ---------------------------------------------------------------------------
#[test]
fn cfg_26_stateless_interleaved() {
    use std::os::raw::c_char;

    let mut rng = Rng::new(SEED ^ 26);
    let p = pair();
    for _ in 0..3000 {
        let n = rng.range(1, 40);
        let k = rng.below(12);
        let mut buf = rng.bytes(n, 30);
        let base = buf.as_mut_ptr() as *mut c_char;

        // C, Rust, C, Rust, Rust, C over the very same memory.
        let order = [0u8, 1, 0, 1, 1, 0];
        let mut results = Vec::new();
        for &which in &order {
            let imp = if which == 0 { &p.c } else { &p.rust };
            results.push(unsafe { observe(imp, base, k, n) });
        }
        for (i, r) in results.iter().enumerate() {
            assert_eq!(
                *r, results[0],
                "invocation {i} diverged from the first (numLines={k}, bufferSize={n}) \
                 -- hidden state?"
            );
        }
        // and against the model
        match model(&buf, k, n) {
            None => assert!(results[0].null, "model mismatch"),
            Some(exp) => {
                assert!(!results[0].null, "model mismatch");
                assert_eq!(results[0].offsets, exp, "model mismatch");
            }
        }
    }
}
