//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! through their exported `driver` symbol and requires byte-identical stdout.
//! Rows use many randomized inputs from a fixed-seed PRNG, so runs are
//! reproducible but a single hand-picked value is never relied upon.
//!
//! `driver` is simultaneously the lowest-level and the only public entry point
//! (`nm -D` on the C `.so` exports nothing else), so "test the low-level entry
//! points too" is satisfied by construction.

mod common;

use common::*;

/// C1 — both arguments empty.
#[test]
fn c1_both_empty() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    // Repeated so we also confirm the result is stable across calls.
    for _ in 0..16 {
        assert_same_bytes(&pair, &mut cap, b"", b"");
    }
}

/// C2 — empty `s1`, randomized non-empty `s2`.
#[test]
fn c2_empty_s1_random_s2() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC2);
    let alpha = full_alphabet();
    for _ in 0..500 {
        let len = rng.range(1, 64);
        let s2 = rng.bytes(len, &alpha);
        assert_same_bytes(&pair, &mut cap, b"", &s2);
    }
}

/// C3 — randomized `s1`, empty `s2`: glibc's `strlen` degeneration.
#[test]
fn c3_random_s1_empty_s2() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC3);
    let alpha = full_alphabet();
    for _ in 0..500 {
        let len = rng.range(0, 300);
        let s1 = rng.bytes(len, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, b"");
    }
}

/// C4 — all 255x255 single-byte/single-byte combinations, exhaustively.
///
/// This is the row that pins down byte-value handling with no randomness at all:
/// every legal byte appears both as the `s1` byte and as the reject byte, so the
/// signed-`char` comparison hazard cannot hide anywhere in the domain.
#[test]
fn c4_single_byte_exhaustive() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            assert_same_bytes(&pair, &mut cap, &[a], &[b]);
        }
    }
}

/// C5 — single-byte reject set (`strchrnul` fast path) with randomized `s1`.
#[test]
fn c5_single_byte_reject() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC5);
    let alpha = ascii_alphabet();
    for _ in 0..1000 {
        let len = rng.range(0, 80);
        let s1 = rng.bytes(len, &alpha);
        let s2 = vec![rng.pick(&alpha)];
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C6 — two-byte reject set: the first size past both glibc fast paths.
#[test]
fn c6_two_byte_reject() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC6);
    let alpha = full_alphabet();
    for _ in 0..1000 {
        let len = rng.range(0, 80);
        let s1 = rng.bytes(len, &alpha);
        let s2 = rng.bytes(2, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C7 / C8 — reject sets one step either side of the 16- and 32-byte SIMD block
/// sizes.
#[test]
fn c7_c8_reject_sizes_around_simd_blocks() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC78);
    let alpha = full_alphabet();
    for &n in &[15usize, 16, 17, 31, 32, 33] {
        for _ in 0..200 {
            let len = rng.range(0, 120);
            let s1 = rng.bytes(len, &alpha);
            let s2 = rng.bytes(n, &alpha);
            assert_same_bytes(&pair, &mut cap, &s1, &s2);
        }
    }
}

/// C9 — large randomized reject sets.
#[test]
fn c9_large_reject_sets() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC9);
    let alpha = full_alphabet();
    for _ in 0..400 {
        let n = rng.range(64, 255);
        let s2 = rng.bytes(n, &alpha);
        let len = rng.range(0, 200);
        let s1 = rng.bytes(len, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C10 — the match is forced at index 0 of `s1`.
#[test]
fn c10_match_at_index_zero() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC10);
    let alpha = full_alphabet();
    for _ in 0..500 {
        let len = rng.range(1, 100);
        let mut s1 = rng.bytes(len, &alpha);
        let n = rng.range(1, 20);
        let mut s2 = rng.bytes(n, &alpha);
        // Force membership of s1[0] in the reject set.
        s2[rng.below(n)] = s1[0];
        // ...and make sure s1[0] is what we think it is.
        s1[0] = s2.iter().copied().max().unwrap();
        s2.push(s1[0]);
        assert_eq!(expected_strcspn(&s1, &s2), 0);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C11 — the first match sits at a randomized interior index.
#[test]
fn c11_match_in_middle() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC11);
    // Split the alphabet so "not in the reject set" is constructible.
    let accept: Vec<u8> = (b'a'..=b'z').collect();
    let reject: Vec<u8> = (b'A'..=b'Z').collect();
    for _ in 0..1000 {
        let len = rng.range(2, 200);
        let mut s1 = rng.bytes(len, &accept);
        let at = rng.range(1, len - 1);
        let n = rng.range(1, 26);
        let mut s2 = rng.bytes(n, &reject);
        s1[at] = s2[rng.below(n)];
        // Guarantee the first hit really is at `at`.
        s2.retain(|b| reject.contains(b));
        assert_eq!(expected_strcspn(&s1, &s2), at);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C12 — the only match is `s1`'s final byte, so the result is `len - 1`.
#[test]
fn c12_match_at_last_byte() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC12);
    let accept: Vec<u8> = (b'a'..=b'z').collect();
    let reject: Vec<u8> = (b'A'..=b'Z').collect();
    for _ in 0..1000 {
        let len = rng.range(1, 300);
        let mut s1 = rng.bytes(len, &accept);
        let n = rng.range(1, 26);
        let s2 = rng.bytes(n, &reject);
        s1[len - 1] = s2[rng.below(n)];
        assert_eq!(expected_strcspn(&s1, &s2), len - 1);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C13 — disjoint byte sets, so the answer is always `strlen(s1)`.
#[test]
fn c13_no_match_disjoint_sets() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC13);
    for _ in 0..1000 {
        // Randomize the split point of the alphabet too.
        let split = rng.range(1, 254) as u8;
        let accept: Vec<u8> = (1u8..=255).filter(|b| *b <= split).collect();
        let reject: Vec<u8> = (1u8..=255).filter(|b| *b > split).collect();
        if accept.is_empty() || reject.is_empty() {
            continue;
        }
        let s1 = rng.bytes_range(0, 150, &accept);
        let s2 = rng.bytes_range(1, 60, &reject);
        assert_eq!(expected_strcspn(&s1, &s2), s1.len());
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C14 — bytes `0x80..=0xFF` only: the signed-`char` sign-extension hazard.
#[test]
fn c14_high_bit_bytes_only() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC14);
    let alpha = high_alphabet();
    for _ in 0..1500 {
        let s1 = rng.bytes_range(0, 100, &alpha);
        let s2 = rng.bytes_range(1, 40, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C15 — mixed ASCII and high-bit bytes.
#[test]
fn c15_mixed_ascii_and_high() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC15);
    let mut alpha = ascii_alphabet();
    alpha.extend(high_alphabet());
    for _ in 0..1500 {
        let s1 = rng.bytes_range(0, 120, &alpha);
        let s2 = rng.bytes_range(1, 50, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C16 — the full `0x01..=0xFF` alphabet on both sides.
#[test]
fn c16_full_alphabet() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC16);
    let alpha = full_alphabet();
    for _ in 0..2000 {
        let s1 = rng.bytes_range(0, 150, &alpha);
        let s2 = rng.bytes_range(1, 60, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C17 — `s2` contains every non-NUL byte, so any non-empty `s1` yields 0.
///
/// This also pins down that `s2`'s terminating NUL is *not* a reject-set member:
/// the reject set covers literally every other byte, yet a non-empty `s1` must
/// still report `0` rather than anything derived from the NUL.
#[test]
fn c17_reject_every_byte() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC17);
    let alpha = full_alphabet();
    let s2 = full_alphabet();
    // Empty s1 first: the answer must be 0 for a different reason (end of s1).
    assert_same_bytes(&pair, &mut cap, b"", &s2);
    for _ in 0..300 {
        let s1 = rng.bytes_range(1, 80, &alpha);
        assert_eq!(expected_strcspn(&s1, &s2), 0);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C18 — heavily duplicated reject bytes.
#[test]
fn c18_duplicate_heavy_reject() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC18);
    let alpha = full_alphabet();
    for _ in 0..600 {
        let two = [rng.pick(&alpha), rng.pick(&alpha)];
        let s2 = rng.bytes_range(1, 200, &two);
        let s1 = rng.bytes_range(0, 120, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C19 — `s2` is a superset of `s1`'s bytes.
#[test]
fn c19_reject_superset_of_s1() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC19);
    let alpha = full_alphabet();
    for _ in 0..600 {
        let s1 = rng.bytes_range(0, 120, &alpha);
        let mut s2: Vec<u8> = s1.clone();
        // Shuffle-ish and pad with extra bytes so it is a strict superset.
        for _ in 0..rng.range(1, 10) {
            s2.push(rng.pick(&alpha));
        }
        if s2.is_empty() {
            s2.push(rng.pick(&alpha));
        }
        for i in (1..s2.len()).rev() {
            let j = rng.below(i + 1);
            s2.swap(i, j);
        }
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C20 — `s1` lengths 0..=64 exhaustively, spanning the SIMD block boundaries.
#[test]
fn c20_s1_lengths_exhaustive_to_64() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC20);
    let alpha = full_alphabet();
    for len in 0..=64usize {
        for _ in 0..40 {
            let s1 = rng.bytes(len, &alpha);
            let s2 = rng.bytes_range(1, 20, &alpha);
            assert_same_bytes(&pair, &mut cap, &s1, &s2);
        }
    }
}

/// C21 — long `s1` with the match at a randomized index: multi-digit `%zu`.
#[test]
fn c21_long_s1_random_match_position() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC21);
    for _ in 0..120 {
        let len = rng.range(1024, 65536);
        let mut s1 = vec![b'a'; len];
        let s2 = b"Z".to_vec();
        let at = rng.below(len);
        s1[at] = b'Z';
        assert_eq!(expected_strcspn(&s1, &s2), at);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C22 — long `s1` with no match: the widest `%zu` output this API can produce
/// in reasonable time.
#[test]
fn c22_long_s1_no_match_wide_output() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC22);
    for &len in &[999usize, 1000, 9999, 10000, 65535, 65536, 100_000] {
        let s1 = vec![b'a'; len];
        let s2 = rng.bytes_range(1, 20, &(b'A'..=b'Z').collect::<Vec<u8>>());
        assert_eq!(expected_strcspn(&s1, &s2), len);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}

/// C23 — `s1` at every start offset 0..=16 inside its buffer.
#[test]
fn c23_s1_alignment_sweep() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC23);
    let alpha = full_alphabet();
    for off in 0..=16usize {
        for _ in 0..80 {
            let s1 = rng.bytes_range(0, 90, &alpha);
            let s2 = rng.bytes_range(1, 30, &alpha);
            let a = OffsetStrBuf::new(&s1, off);
            let b = CStrBuf::new(&s2);
            // SAFETY: both buffers are NUL-terminated and outlive the calls.
            unsafe {
                assert_same(&pair, &mut cap, a.ptr(), b.ptr(), || {
                    format!("offset {off}, s1={:?} s2={:?}", Escaped(&s1), Escaped(&s2))
                })
            }
        }
    }
}

/// C24 — `s2` at every start offset 0..=16 inside its buffer.
#[test]
fn c24_s2_alignment_sweep() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC24);
    let alpha = full_alphabet();
    for off in 0..=16usize {
        for _ in 0..80 {
            let s1 = rng.bytes_range(0, 90, &alpha);
            let s2 = rng.bytes_range(1, 30, &alpha);
            let a = CStrBuf::new(&s1);
            let b = OffsetStrBuf::new(&s2, off);
            // SAFETY: both buffers are NUL-terminated and outlive the calls.
            unsafe {
                assert_same(&pair, &mut cap, a.ptr(), b.ptr(), || {
                    format!("offset {off}, s1={:?} s2={:?}", Escaped(&s1), Escaped(&s2))
                })
            }
        }
    }
}

/// C25 — `s1`'s NUL is the last readable byte before a `PROT_NONE` page, so any
/// over-read faults instead of silently succeeding.
#[test]
fn c25_s1_page_guarded() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC25);
    let alpha = full_alphabet();
    for _ in 0..300 {
        let s1 = rng.bytes_range(0, 100, &alpha);
        let s2 = rng.bytes_range(1, 30, &alpha);
        let a = GuardedBuf::terminated(&s1);
        let b = CStrBuf::new(&s2);
        // SAFETY: `a` is NUL-terminated within its readable page.
        unsafe {
            assert_same(&pair, &mut cap, a.ptr(), b.ptr(), || {
                format!("guarded s1={:?} s2={:?}", Escaped(&s1), Escaped(&s2))
            })
        }
    }
}

/// C26 — `s2`'s NUL is the last readable byte before a `PROT_NONE` page.
#[test]
fn c26_s2_page_guarded() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC26);
    let alpha = full_alphabet();
    for _ in 0..300 {
        let s1 = rng.bytes_range(0, 100, &alpha);
        let s2 = rng.bytes_range(0, 60, &alpha);
        let a = CStrBuf::new(&s1);
        let b = GuardedBuf::terminated(&s2);
        // SAFETY: `b` is NUL-terminated within its readable page.
        unsafe {
            assert_same(&pair, &mut cap, a.ptr(), b.ptr(), || {
                format!("s1={:?} guarded s2={:?}", Escaped(&s1), Escaped(&s2))
            })
        }
    }
}

/// C27 — both arguments page-guarded at once, including the empty-string cases.
#[test]
fn c27_both_page_guarded() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC27);
    let alpha = full_alphabet();
    for len1 in 0..=40usize {
        for len2 in 0..=40usize {
            if (len1 + len2) % 7 != 0 {
                continue; // prune the cross-product, keep the boundaries
            }
            let s1 = rng.bytes(len1, &alpha);
            let s2 = rng.bytes(len2, &alpha);
            let a = GuardedBuf::terminated(&s1);
            let b = GuardedBuf::terminated(&s2);
            // SAFETY: both are NUL-terminated within their readable pages.
            unsafe {
                assert_same(&pair, &mut cap, a.ptr(), b.ptr(), || {
                    format!("guarded both: {:?} / {:?}", Escaped(&s1), Escaped(&s2))
                })
            }
        }
    }
}

/// C28 — interleaved repeated calls: no residual state, identical buffering.
#[test]
fn c28_interleaved_repeated_calls() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC28);
    let alpha = full_alphabet();
    let cases: Vec<(Vec<u8>, Vec<u8>)> = (0..200)
        .map(|_| {
            (
                rng.bytes_range(0, 40, &alpha),
                rng.bytes_range(0, 20, &alpha),
            )
        })
        .collect();
    // Three passes over the same inputs, so any state carried between calls
    // would show up as a changed answer.
    for _ in 0..3 {
        for (s1, s2) in &cases {
            assert_same_bytes(&pair, &mut cap, s1, s2);
        }
    }
}

/// C29 — broad randomized sweep over lengths, alphabet size and contents.
#[test]
fn c29_broad_fuzz_sweep() {
    let pair = load_pair();
    let mut cap = Capture::begin();
    let mut rng = Rng::new(0xC29);
    for _ in 0..20_000 {
        let asize = rng.range(1, 255);
        let alpha: Vec<u8> = (0..asize).map(|_| rng.range(1, 255) as u8).collect();
        let s1 = rng.bytes_range(0, 512, &alpha);
        let s2 = rng.bytes_range(0, 64, &alpha);
        assert_same_bytes(&pair, &mut cap, &s1, &s2);
    }
}
