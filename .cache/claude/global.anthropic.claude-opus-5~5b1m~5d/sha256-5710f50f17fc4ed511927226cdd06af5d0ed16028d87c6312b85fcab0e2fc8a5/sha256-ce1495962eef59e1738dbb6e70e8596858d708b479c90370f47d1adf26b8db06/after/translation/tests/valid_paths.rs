//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects through
//! their exported `driver` symbol and compares stdout byte-for-byte. Rows marked
//! "randomized" in CONFIGS.md use many seeded inputs (SplitMix64, fixed seed).

mod common;

use common::*;

// --- row 1 -----------------------------------------------------------------
#[test]
fn cfg_01_both_empty() {
    assert_same_and_eq(b"", b"", 0);
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn cfg_02_s1_empty_s2_nonempty() {
    let mut rng = Rng::new(0x0002);
    for _ in 0..500 {
        let s2 = rng.bytes_range(1, 64);
        assert_same_and_eq(b"", &s2, 0);
    }
    // Hand-picked shapes too.
    for s2 in [&b"a"[..], b"abc", b"\xff", &all_nonzero_bytes()] {
        assert_same_and_eq(b"", s2, 0);
    }
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn cfg_03_s2_empty_reject_set() {
    let mut rng = Rng::new(0x0003);
    for _ in 0..2000 {
        let len = rng.range(1, 256);
        let s1 = rng.bytes(len);
        // Empty reject set => result is strlen(s1).
        assert_same_and_eq(&s1, b"", len);
    }
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn cfg_04_single_byte_each() {
    // Exhaustive over a sampled grid plus randomized pairs.
    let mut rng = Rng::new(0x0004);
    for _ in 0..3000 {
        let a = rng.nonzero_byte();
        let b = rng.nonzero_byte();
        let expected = if a == b { 0 } else { 1 };
        assert_same_and_eq(&[a], &[b], expected);
    }
    // Exhaustive equal case for every byte value.
    for b in 1u8..=255 {
        assert_same_and_eq(&[b], &[b], 0);
    }
    // Boundary byte values against each other.
    for &a in &[1u8, 0x1f, 0x20, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        for &b in &[1u8, 0x1f, 0x20, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
            assert_same_and_eq(&[a], &[b], if a == b { 0 } else { 1 });
        }
    }
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn cfg_05_match_at_index_zero() {
    let mut rng = Rng::new(0x0005);
    for _ in 0..2000 {
        let len = rng.range(1, 128);
        let mut s1 = rng.bytes(len);
        let hit = rng.nonzero_byte();
        s1[0] = hit;
        // Reject set contains s1[0] (plus noise) => result 0.
        let mut s2 = rng.bytes_range(0, 8);
        s2.push(hit);
        assert_same_and_eq(&s1, &s2, 0);
    }
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn cfg_06_match_in_middle() {
    let mut rng = Rng::new(0x0006);
    for _ in 0..3000 {
        let len = rng.range(3, 200);
        let idx = rng.range(1, len - 2);
        // Build s1 from an alphabet, then reserve a distinct byte as the marker.
        let marker = rng.nonzero_byte();
        let mut s1: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != marker {
                    break b;
                }
            })
            .collect();
        s1[idx] = marker;
        let s2 = [marker];
        assert_same_and_eq(&s1, &s2, idx);
    }
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn cfg_07_match_at_last_byte() {
    let mut rng = Rng::new(0x0007);
    for _ in 0..2000 {
        let len = rng.range(2, 200);
        let marker = rng.nonzero_byte();
        let mut s1: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != marker {
                    break b;
                }
            })
            .collect();
        s1[len - 1] = marker;
        assert_same_and_eq(&s1, &[marker], len - 1);
    }
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn cfg_08_no_match_disjoint_alphabets() {
    let mut rng = Rng::new(0x0008);
    for _ in 0..2000 {
        // Split the byte domain into two disjoint halves.
        let pivot = rng.range(2, 253) as u8;
        let low: Vec<u8> = (1u8..pivot).collect();
        let high: Vec<u8> = (pivot..=255).collect();
        let len = rng.range(1, 200);
        let s1 = rng.bytes_from(len, &low);
        let s2 = rng.bytes_from_range(1, 40, &high);
        assert_same_and_eq(&s1, &s2, len);
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn cfg_09_every_byte_matches() {
    let mut rng = Rng::new(0x0009);
    for _ in 0..1500 {
        let len = rng.range(1, 128);
        let s1 = rng.bytes(len);
        // s2 = exactly the set of bytes in s1 (in a shuffled-ish order).
        let mut s2: Vec<u8> = s1.clone();
        s2.reverse();
        assert_same_and_eq(&s1, &s2, 0);
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn cfg_10_simd_block_boundaries() {
    let mut rng = Rng::new(0x0010);
    for &len in &[
        1usize, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 47, 48, 49, 63, 64, 65, 95, 96, 97, 127, 128,
        129, 255, 256, 257,
    ] {
        for _ in 0..40 {
            let marker = rng.nonzero_byte();
            let body: Vec<u8> = (0..len)
                .map(|_| loop {
                    let b = rng.nonzero_byte();
                    if b != marker {
                        break b;
                    }
                })
                .collect();

            // (a) no match
            assert_same_and_eq(&body, &[marker], len);

            // (b) match at the very last byte
            let mut last = body.clone();
            last[len - 1] = marker;
            assert_same_and_eq(&last, &[marker], len - 1);

            // (c) match at index 0
            let mut first = body.clone();
            first[0] = marker;
            assert_same_and_eq(&first, &[marker], 0);

            // (d) match at a random interior index
            let idx = rng.below(len);
            let mut mid = body.clone();
            mid[idx] = marker;
            assert_same_and_eq(&mid, &[marker], idx);
        }
    }
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn cfg_11_maximal_reject_set() {
    let all = all_nonzero_bytes();
    assert_eq!(all.len(), 255);
    let mut rng = Rng::new(0x0011);
    // Any non-empty s1 must yield 0 because every possible byte is rejected.
    for _ in 0..1000 {
        let s1 = rng.bytes_range(1, 128);
        assert_same_and_eq(&s1, &all, 0);
    }
    // And the empty s1 still yields 0.
    assert_same_and_eq(b"", &all, 0);
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn cfg_12_ascii_printable_domain() {
    let mut rng = Rng::new(0x0012);
    for _ in 0..4000 {
        let s1 = rng.bytes_from_range(0, 96, ASCII_PRINTABLE);
        let s2 = rng.bytes_from_range(0, 20, ASCII_PRINTABLE);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn cfg_13_high_byte_domain_signedness() {
    // 0x80..=0xFF are negative when `char` is signed — the one place a Rust i8
    // comparison could diverge from C.
    let high = high_bytes();
    let mut rng = Rng::new(0x0013);
    for _ in 0..4000 {
        let s1 = rng.bytes_from_range(0, 96, &high);
        let s2 = rng.bytes_from_range(0, 20, &high);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
    // Mixed ASCII + high, so sign-extension bugs cannot hide behind a uniform set.
    let mixed: Vec<u8> = ASCII_PRINTABLE.iter().copied().chain(high).collect();
    for _ in 0..4000 {
        let s1 = rng.bytes_from_range(0, 96, &mixed);
        let s2 = rng.bytes_from_range(0, 20, &mixed);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
    // Exhaustive: 0x7F vs 0x80 vs 0xFF crossings.
    for &a in &[0x7fu8, 0x80, 0x81, 0xfe, 0xff] {
        for &b in &[0x7fu8, 0x80, 0x81, 0xfe, 0xff] {
            assert_same_and_eq(&[a, b], &[b], if a == b { 0 } else { 1 });
            assert_same_and_eq(&[a, b], &[a], 0);
        }
    }
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn cfg_14_full_byte_domain() {
    let all = all_nonzero_bytes();
    let mut rng = Rng::new(0x0014);
    for _ in 0..5000 {
        let s1 = rng.bytes_from_range(0, 80, &all);
        let s2 = rng.bytes_from_range(0, 12, &all);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
    // Control bytes only.
    let ctrl = control_bytes();
    for _ in 0..1500 {
        let s1 = rng.bytes_from_range(0, 64, &ctrl);
        let s2 = rng.bytes_from_range(0, 10, &ctrl);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn cfg_15_duplicate_and_long_reject_set() {
    let mut rng = Rng::new(0x0015);
    for _ in 0..2000 {
        // s2 full of duplicates.
        let b = rng.nonzero_byte();
        let s2: Vec<u8> = std::iter::repeat_n(b, rng.range(1, 64)).collect();
        let s1 = rng.bytes_range(0, 40);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
    for _ in 0..2000 {
        // s2 strictly longer than s1 (worst-case O(n*m), no early exit).
        let n = rng.range(1, 20);
        let m = rng.range(n + 1, n + 200);
        let s1 = rng.bytes(n);
        let s2 = rng.bytes(m);
        assert_same_and_eq(&s1, &s2, strcspn_ref(&s1, &s2));
    }
}

// --- row 16 ----------------------------------------------------------------
#[test]
fn cfg_16_long_s1_random_match_index() {
    let mut rng = Rng::new(0x0016);
    for _ in 0..300 {
        let len = rng.range(256, 4096);
        let marker = rng.nonzero_byte();
        let mut s1: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != marker {
                    break b;
                }
            })
            .collect();
        let idx = rng.below(len);
        s1[idx] = marker;
        assert_same_and_eq(&s1, &[marker], idx);
    }
}

// --- row 17 ----------------------------------------------------------------
#[test]
fn cfg_17_very_long_s1_large_result() {
    // >= 1 MiB with no match: exercises %zu on a large size_t.
    for &len in &[1usize << 20, (1 << 20) + 1, 3_000_000] {
        let s1: Vec<u8> = std::iter::repeat_n(b'a', len).collect();
        assert_same_and_eq(&s1, b"XYZ", len);
    }
    // Same size but a match near the very end.
    let len = 1usize << 20;
    let mut s1: Vec<u8> = std::iter::repeat_n(b'a', len).collect();
    s1[len - 1] = b'Z';
    assert_same_and_eq(&s1, b"Z", len - 1);
}

// --- row 18 ----------------------------------------------------------------
#[test]
fn cfg_18_misaligned_buffers() {
    let mut rng = Rng::new(0x0018);
    for off1 in 0..16usize {
        for off2 in 0..16usize {
            for _ in 0..8 {
                let len1 = rng.range(1, 80);
                let len2 = rng.range(1, 16);
                let b1 = rng.bytes(len1);
                let b2 = rng.bytes(len2);

                // Over-allocate and start the strings at the requested offsets.
                let mut pad1 = vec![0xEEu8; off1];
                pad1.extend_from_slice(&b1);
                pad1.push(0);
                let mut pad2 = vec![0xEEu8; off2];
                pad2.extend_from_slice(&b2);
                pad2.push(0);

                let p1 = unsafe { pad1.as_ptr().add(off1) } as *const std::ffi::c_char;
                let p2 = unsafe { pad2.as_ptr().add(off2) } as *const std::ffi::c_char;

                let c = c_out(p1, p2);
                let r = rust_out(p1, p2);
                assert_eq!(
                    c,
                    r,
                    "misaligned divergence off1={off1} off2={off2}\n s1={:?} s2={:?}",
                    Escaped(&b1),
                    Escaped(&b2)
                );
                assert_eq!(
                    c,
                    format!("{}\n", strcspn_ref(&b1, &b2)).into_bytes(),
                    "wrong value for off1={off1} off2={off2}"
                );
            }
        }
    }
}

// --- row 19 ----------------------------------------------------------------
#[test]
fn cfg_19_s1_flush_against_guard_page() {
    let mut rng = Rng::new(0x0019);
    for _ in 0..200 {
        let len = rng.range(1, 200);
        let marker = rng.nonzero_byte();
        let body: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != marker {
                    break b;
                }
            })
            .collect();

        // (a) no match: the scan must stop exactly at the NUL, not read the guard page.
        {
            let g = GuardedString::new(&body);
            let s2 = CBuf::new(&[marker]);
            let res = assert_same_fork(g.ptr(), s2.ptr(), "row19 no-match guarded s1");
            assert_eq!(res.outcome, Outcome::Exited(0), "guarded s1 should not fault");
            assert_eq!(res.stdout, format!("{len}\n").into_bytes());
        }
        // (b) match at the last byte.
        {
            let mut m = body.clone();
            m[len - 1] = marker;
            let g = GuardedString::new(&m);
            let s2 = CBuf::new(&[marker]);
            let res = assert_same_fork(g.ptr(), s2.ptr(), "row19 last-byte guarded s1");
            assert_eq!(res.outcome, Outcome::Exited(0));
            assert_eq!(res.stdout, format!("{}\n", len - 1).into_bytes());
        }
    }
    // Lengths straddling exact page multiples.
    let page = 4096usize;
    for &len in &[page - 2, page - 1, page, page + 1, 2 * page - 1, 2 * page] {
        let body: Vec<u8> = std::iter::repeat_n(b'a', len).collect();
        let g = GuardedString::new(&body);
        let s2 = CBuf::new(b"Z");
        let res = assert_same_fork(g.ptr(), s2.ptr(), "row19 page-multiple guarded s1");
        assert_eq!(res.outcome, Outcome::Exited(0));
        assert_eq!(res.stdout, format!("{len}\n").into_bytes());
    }
}

// --- row 20 ----------------------------------------------------------------
#[test]
fn cfg_20_s2_flush_against_guard_page() {
    let mut rng = Rng::new(0x0020);
    for _ in 0..200 {
        let n = rng.range(1, 64);
        let m = rng.range(1, 64);
        // Disjoint alphabets so the whole reject set is scanned for every byte of s1
        // (worst case for an over-reading implementation).
        let s1 = rng.bytes_from(n, &(1u8..=127).collect::<Vec<u8>>());
        let s2 = rng.bytes_from(m, &(128u8..=255).collect::<Vec<u8>>());
        let g = GuardedString::new(&s2);
        let a = CBuf::new(&s1);
        let res = assert_same_fork(a.ptr(), g.ptr(), "row20 guarded s2");
        assert_eq!(res.outcome, Outcome::Exited(0), "guarded s2 should not fault");
        assert_eq!(res.stdout, format!("{n}\n").into_bytes());
    }
    // Both strings guarded simultaneously.
    for _ in 0..100 {
        let s1 = rng.bytes_from_range(1, 64, &(1u8..=127).collect::<Vec<u8>>());
        let s2 = rng.bytes_from_range(1, 64, &(128u8..=255).collect::<Vec<u8>>());
        let g1 = GuardedString::new(&s1);
        let g2 = GuardedString::new(&s2);
        let res = assert_same_fork(g1.ptr(), g2.ptr(), "row20 both guarded");
        assert_eq!(res.outcome, Outcome::Exited(0));
        assert_eq!(res.stdout, format!("{}\n", s1.len()).into_bytes());
    }
}

// --- row 21 ----------------------------------------------------------------
#[test]
fn cfg_21_result_digit_count_sweep() {
    // 1..7 digit results, plus each 10^k boundary and 10^k - 1.
    for k in 0..7u32 {
        for len in [
            10usize.pow(k),
            10usize.pow(k) + 1,
            10usize.pow(k + 1) - 1,
        ] {
            let s1: Vec<u8> = std::iter::repeat_n(b'q', len).collect();
            assert_same_and_eq(&s1, b"\x01", len);
        }
    }
    // Result 0 prints exactly "0\n".
    let a = CBuf::new(b"abc");
    let b = CBuf::new(b"a");
    assert_eq!(c_out(a.ptr(), b.ptr()), b"0\n".to_vec());
    assert_eq!(rust_out(a.ptr(), b.ptr()), b"0\n".to_vec());
}

// --- row 22 ----------------------------------------------------------------
#[test]
fn cfg_22_interleaved_stateless_invocation() {
    // C, Rust, C, Rust ... in one process: the function must be stateless and
    // must not disturb stdio state for the next caller.
    let mut rng = Rng::new(0x0022);
    let (cf, rf) = (libs().c, libs().rust);
    for _ in 0..1000 {
        let s1 = rng.bytes_range(0, 40);
        let s2 = rng.bytes_range(0, 10);
        let a = CBuf::new(&s1);
        let b = CBuf::new(&s2);
        let expect = format!("{}\n", strcspn_ref(&s1, &s2)).into_bytes();

        // Four calls, alternating implementations, all inside ONE capture:
        // the concatenated output must be the same line four times.
        let joined = capture(|| unsafe {
            cf(a.ptr(), b.ptr());
            rf(a.ptr(), b.ptr());
            cf(a.ptr(), b.ptr());
            rf(a.ptr(), b.ptr());
        });
        let mut want = Vec::new();
        for _ in 0..4 {
            want.extend_from_slice(&expect);
        }
        assert_eq!(
            joined,
            want,
            "interleaved output mismatch for s1={:?} s2={:?} got {:?}",
            Escaped(&s1),
            Escaped(&s2),
            String::from_utf8_lossy(&joined)
        );
    }
    // Randomized order, many repetitions of the same arguments.
    for _ in 0..200 {
        let s1 = rng.bytes_range(1, 30);
        let s2 = rng.bytes_range(1, 6);
        let a = CBuf::new(&s1);
        let b = CBuf::new(&s2);
        let expect = format!("{}\n", strcspn_ref(&s1, &s2)).into_bytes();
        for _ in 0..5 {
            let use_c = rng.below(2) == 0;
            let out = if use_c {
                c_out(a.ptr(), b.ptr())
            } else {
                rust_out(a.ptr(), b.ptr())
            };
            assert_eq!(out, expect, "stateless violation (use_c={use_c})");
        }
    }
}

// --- row 23 ----------------------------------------------------------------
#[test]
fn cfg_23_uniform_fuzz() {
    let mut rng = Rng::new(0xDEADBEEF);
    for i in 0..20_000 {
        let s1 = rng.bytes_below(65);
        let s2 = rng.bytes_below(65);
        let a = CBuf::new(&s1);
        let b = CBuf::new(&s2);
        let c = c_out(a.ptr(), b.ptr());
        let r = rust_out(a.ptr(), b.ptr());
        assert_eq!(
            c,
            r,
            "fuzz divergence at iteration {i}\n s1={:?}\n s2={:?}\n C={:?} Rust={:?}",
            Escaped(&s1),
            Escaped(&s2),
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        assert_eq!(
            c,
            format!("{}\n", strcspn_ref(&s1, &s2)).into_bytes(),
            "both wrong vs reference at iteration {i}"
        );
    }
}
