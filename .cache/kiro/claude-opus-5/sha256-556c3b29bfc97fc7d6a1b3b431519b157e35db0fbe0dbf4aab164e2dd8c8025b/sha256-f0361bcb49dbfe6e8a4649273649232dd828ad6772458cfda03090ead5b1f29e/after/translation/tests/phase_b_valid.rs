//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared libraries
//! through their exported `driver` symbol (loaded with `libloading`) and compares
//! stdout byte-for-byte, over many randomized inputs from a fixed seed.

mod common;

use common::{assert_same, assert_same_and_value, assert_same_cstr, strcspn_ref, Rng, SEED};

/// Number of randomized cases per row (cheap rows).
const N: usize = 256;

// --- row 1 -----------------------------------------------------------------
#[test]
fn cfg_01_empty_s1() {
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..N {
        let s2 = rng.bytes_ascii_len(1, 32);
        assert_same_and_value(b"", &s2, 0, &format!("cfg_01 case {i}"));
    }
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn cfg_02_empty_s2() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..N {
        let len = rng.range(1, 64);
        let s1 = rng.bytes_any(len);
        // Empty reject set: the NUL of s2 is not a member, so nothing matches.
        assert_same_and_value(&s1, b"", len, &format!("cfg_02 case {i}"));
    }
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn cfg_03_both_empty() {
    assert_same_and_value(b"", b"", 0, "cfg_03");
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn cfg_04_single_single() {
    // Exhaustive over the interesting part of the space, not just random.
    for a in 1u8..=255 {
        for b in [1u8, a, a.wrapping_add(1).max(1), 0x7F, 0x80, 0xFF] {
            if b == 0 {
                continue;
            }
            let s1 = [a];
            let s2 = [b];
            let expect = if a == b { 0 } else { 1 };
            assert_same_and_value(&s1, &s2, expect, &format!("cfg_04 a={a:#02x} b={b:#02x}"));
        }
    }
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn cfg_05_small_disjoint() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N {
        let len = rng.range(2, 8);
        // s1 from the lower half of the alphabet, s2 from the upper half.
        let s1: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0x7F) as u8).collect();
        let s2: Vec<u8> = rng.bytes_in_len(0x80, 0xFF, 1, 8);
        assert_same_and_value(&s1, &s2, len, &format!("cfg_05 case {i}"));
    }
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn cfg_06_match_at_first() {
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let len = rng.range(1, 8);
        let s1 = rng.bytes_any(len);
        let mut s2 = rng.bytes_any_len(1, 6);
        s2.push(s1[0]); // guarantee the very first byte matches
        assert_same_and_value(&s1, &s2, 0, &format!("cfg_06 case {i}"));
    }
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn cfg_07_match_at_last() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N {
        let len = rng.range(2, 8);
        // Build s1 out of distinct bytes so "only the last matches" is exact.
        let mut s1: Vec<u8> = Vec::new();
        while s1.len() < len {
            let b = rng.byte_any();
            if !s1.contains(&b) {
                s1.push(b);
            }
        }
        let s2 = [*s1.last().unwrap()];
        assert_same_and_value(&s1, &s2, len - 1, &format!("cfg_07 case {i}"));
    }
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn cfg_08_match_interior() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..N {
        let len = rng.range(3, 16);
        let mut s1: Vec<u8> = Vec::new();
        while s1.len() < len {
            let b = rng.byte_any();
            if !s1.contains(&b) {
                s1.push(b);
            }
        }
        let idx = rng.range(1, len - 2);
        let s2 = [s1[idx]];
        assert_same_and_value(&s1, &s2, idx, &format!("cfg_08 case {i} idx={idx}"));
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn cfg_09_medium_random() {
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..N {
        let s1 = rng.bytes_any_len(9, 64);
        let s2 = rng.bytes_any_len(1, 8);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_09 case {i}"));
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn cfg_10_medium_medium() {
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let s1 = rng.bytes_any_len(9, 64);
        let s2 = rng.bytes_any_len(9, 64);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_10 case {i}"));
    }
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn cfg_11_large_s1_small_s2() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..64 {
        let s1 = rng.bytes_any_len(65, 4096);
        let s2 = rng.bytes_any_len(1, 4);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_11 case {i}"));
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn cfg_12_s2_all_bytes() {
    let mut rng = Rng::new(SEED ^ 12);
    let all: Vec<u8> = (1u8..=255).collect();
    for i in 0..64 {
        let s1 = rng.bytes_any_len(65, 4096);
        // Every non-NUL byte is rejected, so the answer is always 0.
        assert_same_and_value(&s1, &all, 0, &format!("cfg_12 case {i}"));
    }
    // ...and with an empty s1 it is still 0 for a different reason.
    assert_same_and_value(b"", &all, 0, "cfg_12 empty s1");
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn cfg_13_high_bit_alphabet() {
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..N {
        // Only bytes >= 0x80: negative when `char` is signed, as on x86-64.
        let s1: Vec<u8> = rng.bytes_in_len(0x80, 0xFF, 1, 64);
        let s2: Vec<u8> = rng.bytes_in_len(0x80, 0xFF, 1, 16);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_13 case {i}"));
    }
    // Explicit sign-extension traps: 0x80 vs 0x00 must NOT be confused, and
    // 0xFF must not be treated as -1 == some sentinel.
    assert_same_and_value(&[0x80], &[0xFF], 1, "cfg_13 0x80 vs 0xFF");
    assert_same_and_value(&[0xFF], &[0xFF], 0, "cfg_13 0xFF vs 0xFF");
    assert_same_and_value(&[0x80, 0xFF, 0x81], &[0x81], 2, "cfg_13 high-bit interior");
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn cfg_14_repeated_byte() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..N {
        let b = rng.byte_any();
        let len = rng.range(1, 128);
        let s1 = vec![b; len];
        // Equal case -> 0; different case -> len.
        assert_same_and_value(&s1, &[b], 0, &format!("cfg_14 eq case {i}"));
        let other = if b == 1 { 2 } else { b - 1 };
        assert_same_and_value(&s1, &[other], len, &format!("cfg_14 ne case {i}"));
    }
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn cfg_15_s2_duplicates() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..N {
        let b = rng.byte_any();
        let s2 = vec![b; rng.range(2, 64)]; // same byte over and over
        let s1 = rng.bytes_any_len(1, 64);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_15 case {i}"));
    }
}

// --- row 16 ----------------------------------------------------------------
#[test]
fn cfg_16_s2_superset() {
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..N {
        let s1 = rng.bytes_any_len(1, 64);
        let mut s2 = s1.clone();
        s2.extend(rng.bytes_any_len(1, 16));
        // Shuffle so the match is not trivially at s2[0].
        for k in (1..s2.len()).rev() {
            let j = rng.below(k + 1);
            s2.swap(k, j);
        }
        assert_same_and_value(&s1, &s2, 0, &format!("cfg_16 case {i}"));
    }
}

// --- row 17 ----------------------------------------------------------------
#[test]
fn cfg_17_aliased_same_ptr() {
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..N {
        let len = rng.range(1, 64);
        let mut buf = rng.bytes_any(len);
        buf.push(0);
        // Pass the SAME pointer as both arguments.
        let out = assert_same_cstr(&buf, &buf, &format!("cfg_17 case {i}"));
        assert_eq!(
            String::from_utf8_lossy(&out),
            "0\n",
            "cfg_17 case {i}: s1==s2 non-empty must yield 0"
        );
    }
    // Aliased empty string.
    assert_same_cstr(b"\0", b"\0", "cfg_17 aliased empty");
}

// --- row 18 ----------------------------------------------------------------
#[test]
fn cfg_18_misaligned_s1() {
    let mut rng = Rng::new(SEED ^ 18);
    for off in 0..=16usize {
        for i in 0..16 {
            let body = rng.bytes_any_len(1, 96);
            let s2 = rng.bytes_any_len(1, 4);
            // Over-allocate, then start the string at `off` bytes in.
            let mut buf = vec![0xAAu8; off];
            buf.extend_from_slice(&body);
            buf.push(0);
            let want = strcspn_ref(&body, &s2);

            let mut s2n = s2.clone();
            s2n.push(0);
            let out = assert_same_cstr(
                &buf[off..],
                &s2n,
                &format!("cfg_18 off={off} case {i}"),
            );
            assert_eq!(String::from_utf8_lossy(&out), format!("{want}\n"));
        }
    }
}

// --- row 19 ----------------------------------------------------------------
#[test]
fn cfg_19_misaligned_s2() {
    let mut rng = Rng::new(SEED ^ 19);
    for off in 0..=16usize {
        for i in 0..16 {
            let s1 = rng.bytes_any_len(1, 96);
            let body = rng.bytes_any_len(1, 32);
            let mut buf = vec![0xAAu8; off];
            buf.extend_from_slice(&body);
            buf.push(0);
            let want = strcspn_ref(&s1, &body);

            let mut s1n = s1.clone();
            s1n.push(0);
            let out = assert_same_cstr(
                &s1n,
                &buf[off..],
                &format!("cfg_19 off={off} case {i}"),
            );
            assert_eq!(String::from_utf8_lossy(&out), format!("{want}\n"));
        }
    }
}

// --- row 20 ----------------------------------------------------------------
#[test]
fn cfg_20_huge_s1_no_match() {
    const MIB: usize = 1024 * 1024;
    // s1 is a 1 MiB run of 'a'; the reject set has no 'a', so the full length is
    // printed and `%zu` must render a 7-digit value.
    let s1 = vec![b'a'; MIB];
    assert_same_and_value(&s1, b"XYZ", MIB, "cfg_20");
}

// --- row 21 ----------------------------------------------------------------
#[test]
fn cfg_21_huge_s1_far_match() {
    const MIB: usize = 1024 * 1024;
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..4 {
        let mut s1 = vec![b'a'; MIB];
        let idx = rng.range(MIB / 2, MIB - 1);
        s1[idx] = b'Z';
        assert_same_and_value(&s1, b"Z", idx, &format!("cfg_21 case {i} idx={idx}"));
    }
}

// --- row 22 ----------------------------------------------------------------
#[test]
fn cfg_22_huge_s2() {
    let mut rng = Rng::new(SEED ^ 22);
    // 64 KiB reject set of random bytes; s1 short. The reference tells us where
    // the first hit is, and both implementations must agree with it.
    for i in 0..8 {
        let s2 = rng.bytes_any(64 * 1024);
        let s1 = rng.bytes_any_len(1, 16);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_22 case {i}"));
    }
}

// --- row 23 ----------------------------------------------------------------
#[test]
fn cfg_23_boundary_lengths() {
    let mut rng = Rng::new(SEED ^ 23);
    for len in [
        1usize, 2, 3, 4, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
        511, 512, 513, 4095, 4096, 4097,
    ] {
        // No match: s1 all from the low half, s2 all from the high half.
        let s1: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0x7F) as u8).collect();
        let s2: Vec<u8> = (0..4).map(|_| rng.range(0x80, 0xFF) as u8).collect();
        assert_same_and_value(&s1, &s2, len, &format!("cfg_23 len={len} no-match"));

        // Match exactly at the final byte, i.e. result == len-1.
        let mut s1b = s1.clone();
        s1b[len - 1] = 0x80;
        assert_same_and_value(&s1b, &[0x80], len - 1, &format!("cfg_23 len={len} last"));

        // Match exactly at the first byte.
        let mut s1c = s1.clone();
        s1c[0] = 0x81;
        assert_same_and_value(&s1c, &[0x81], 0, &format!("cfg_23 len={len} first"));
    }
}

// --- row 24 ----------------------------------------------------------------
#[test]
fn cfg_24_unconstrained_fuzz() {
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..2048 {
        let s1 = rng.bytes_any_len(0, 512);
        let s2 = rng.bytes_any_len(0, 512);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_24 case {i}"));
    }
}

// --- row 25 ----------------------------------------------------------------
#[test]
fn cfg_25_repeated_calls_stateless() {
    // Hammer the same loaded pair with a long alternating sequence: if either
    // side kept hidden state, or stdio buffering differed across calls, the
    // per-call outputs would drift.
    let mut rng = Rng::new(SEED ^ 25);
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..512 {
        let s1 = rng.bytes_ascii_len(1, 32);
        let s2 = rng.bytes_ascii_len(1, 8);
        let want = strcspn_ref(&s1, &s2);
        assert_same_and_value(&s1, &s2, want, &format!("cfg_25 case {i}"));

        // Re-run one fixed input every iteration; it must never change.
        let fixed = assert_same(b"hello world", b"ow", &format!("cfg_25 fixed at {i}"));
        assert_eq!(String::from_utf8_lossy(&fixed), "4\n");
        if let Some(p) = &prev {
            assert_eq!(p, &fixed, "cfg_25: fixed input drifted at iteration {i}");
        }
        prev = Some(fixed);
    }
}
