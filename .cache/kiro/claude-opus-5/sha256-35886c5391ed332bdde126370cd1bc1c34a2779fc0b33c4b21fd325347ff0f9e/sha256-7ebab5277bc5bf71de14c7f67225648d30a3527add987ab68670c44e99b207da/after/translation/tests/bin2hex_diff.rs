//! Differential tests: the C `.so` and the Rust `.so` are both loaded with
//! `libloading` and their `bin2hex` exports are compared byte-for-byte.

mod common;

use common::{run, Impls};

/// Every possible single input byte — covers both nibble positions and the
/// 0..9 / a..f branch of the branch-free digit computation.
#[test]
fn single_bytes_all_256() {
    let im = Impls::load();
    for v in 0u16..=255 {
        let bin = [v as u8];
        for maxlen in 3..=8usize {
            let c = run(im.c_bin2hex, maxlen, &bin, 0xAA);
            let r = run(im.rust_bin2hex, maxlen, &bin, 0xAA);
            assert_eq!(c, r, "byte {v:#04x}, hex_maxlen {maxlen}");
        }
    }
}

#[test]
fn empty_input() {
    let im = Impls::load();
    for maxlen in 1..=4usize {
        let c = run(im.c_bin2hex, maxlen, &[], 0x5A);
        let r = run(im.rust_bin2hex, maxlen, &[], 0x5A);
        assert_eq!(c, r, "empty input, hex_maxlen {maxlen}");
    }
}

/// All two-byte combinations: 65536 cases, checks ordering of the two output
/// characters within a byte and across bytes.
#[test]
fn all_two_byte_pairs() {
    let im = Impls::load();
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            let bin = [a as u8, b as u8];
            let c = run(im.c_bin2hex, 5, &bin, 0xFF);
            let r = run(im.rust_bin2hex, 5, &bin, 0xFF);
            assert_eq!(c, r, "bytes {a:#04x} {b:#04x}");
        }
    }
}

/// Deterministic pseudo-random buffers of many lengths, with the output buffer
/// sized both at the minimum (`bin_len * 2 + 1`) and generously oversized so
/// that the untouched tail bytes are compared too.
#[test]
fn random_buffers_various_lengths() {
    let im = Impls::load();
    let mut state: u64 = 0x0123_4567_89ab_cdef;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 24) as u8
    };

    for len in 0..=257usize {
        let bin: Vec<u8> = (0..len).map(|_| next()).collect();
        for extra in [1usize, 2, 9] {
            let maxlen = len * 2 + extra;
            let c = run(im.c_bin2hex, maxlen, &bin, 0x37);
            let r = run(im.rust_bin2hex, maxlen, &bin, 0x37);
            assert_eq!(c, r, "len {len}, hex_maxlen {maxlen}");
        }
    }
}

/// Buffers made of repeated single values and of ascending byte patterns.
#[test]
fn patterned_buffers() {
    let im = Impls::load();
    let patterns: Vec<Vec<u8>> = vec![
        vec![0x00; 64],
        vec![0xFF; 64],
        vec![0x0F; 33],
        vec![0xF0; 33],
        vec![0x99; 17],
        vec![0xA0; 17],
        (0..=255u16).map(|v| v as u8).collect(),
        (0..=255u16).rev().map(|v| v as u8).collect(),
        vec![0x09, 0x0A, 0x90, 0xA0, 0x99, 0xAA, 0x0f, 0xf0],
    ];
    for (idx, bin) in patterns.iter().enumerate() {
        let maxlen = bin.len() * 2 + 1;
        let c = run(im.c_bin2hex, maxlen, bin, 0x11);
        let r = run(im.rust_bin2hex, maxlen, bin, 0x11);
        assert_eq!(c, r, "pattern {idx}");
    }
}

/// Sanity check against the expected lowercase-hex encoding, so a mutually
/// consistent but wrong pair of implementations cannot pass silently.
#[test]
fn matches_expected_lowercase_hex() {
    let im = Impls::load();
    let bin: Vec<u8> = (0..=255u16).map(|v| v as u8).collect();
    let expected: String = bin.iter().map(|b| format!("{b:02x}")).collect();
    let out = run(im.rust_bin2hex, bin.len() * 2 + 1, &bin, 0x00);
    let got = &out.buf[..bin.len() * 2];
    assert_eq!(got, expected.as_bytes());
    assert!(out.returned_input_ptr);
    let cout = run(im.c_bin2hex, bin.len() * 2 + 1, &bin, 0x00);
    assert_eq!(cout, out);
}
