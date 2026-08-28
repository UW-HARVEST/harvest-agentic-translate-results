//! Level 0: pure / self-contained functions.
//!   stbds_hash_string, stbds_hash_bytes, strkey

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn cbuf(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

const SEEDS: [usize; 10] = [
    0,
    1,
    2,
    0x3141_5926,
    0xffff_ffff,
    0x8000_0000_0000_0000,
    0xffff_ffff_ffff_ffff,
    0x0123_4567_89ab_cdef,
    0xdead_beef_dead_beef,
    0x5555_5555_5555_5555,
];

#[test]
fn hash_string_matches() {
    let (c, r) = both();
    let mut cases: Vec<String> = vec![
        String::new(),
        "a".into(),
        "ab".into(),
        "abc".into(),
        "abcd".into(),
        "abcde".into(),
        "abcdef".into(),
        "abcdefg".into(),
        "abcdefgh".into(),
        "abcdefghi".into(),
        "test_0".into(),
        "test_-1".into(),
        "The quick brown fox jumps over the lazy dog".into(),
        "\u{7f}\u{80}\u{ff}".into(),
    ];
    // high-bit bytes (signed char sign-extension trap in the C source)
    cases.push(String::from_utf8_lossy(&[0xC3, 0xBF, 0xC3, 0xBE]).into_owned());
    for n in 0..64 {
        cases.push("x".repeat(n));
    }
    for n in 0..64 {
        cases.push((0..n).map(|i| ((i * 37 + 1) % 255 + 1) as u8 as char).collect());
    }

    for s in &cases {
        let mut buf = cbuf(s);
        for seed in SEEDS {
            let a = unsafe { (c.hash_string)(buf.as_mut_ptr(), seed) };
            let b = unsafe { (r.hash_string)(buf.as_mut_ptr(), seed) };
            assert_eq!(a, b, "hash_string({s:?}, {seed:#x})");
        }
    }
}

#[test]
fn hash_bytes_matches() {
    let (c, r) = both();

    // every length 0..=80, with a pattern that exercises high-bit bytes so the
    // `d[3] << 24` sign-extension path in the C source is hit.
    for len in 0..=80usize {
        for pat in 0..4u32 {
            let mut data: Vec<u8> = (0..len)
                .map(|i| match pat {
                    0 => i as u8,
                    1 => 0xff,
                    2 => (i as u8).wrapping_mul(0x9d).wrapping_add(0x80),
                    _ => if i % 2 == 0 { 0x00 } else { 0x80 },
                })
                .collect();
            data.push(0); // guarantee a valid pointer for len == 0
            for seed in SEEDS {
                let p = data.as_mut_ptr() as *mut c_void;
                let a = unsafe { (c.hash_bytes)(p, len, seed) };
                let b = unsafe { (r.hash_bytes)(p, len, seed) };
                assert_eq!(a, b, "hash_bytes(len={len}, pat={pat}, seed={seed:#x})");
            }
        }
    }
}

#[test]
fn strkey_matches() {
    let (c, r) = both();
    for n in [
        0, 1, 9, 10, 99, 100, 12345, -1, -42, i32::MAX, i32::MIN, 7, -7,
    ] {
        let a = unsafe { c_string((c.strkey)(n)) };
        let b = unsafe { c_string((r.strkey)(n)) };
        assert_eq!(a, b, "strkey({n})");
    }
    // repeated calls reuse the same static buffer in both implementations
    let mut seq_c = Vec::new();
    let mut seq_r = Vec::new();
    for n in [-123456, 5, -5, 1000000, 0] {
        seq_c.push(unsafe { c_string((c.strkey)(n)) });
        seq_r.push(unsafe { c_string((r.strkey)(n)) });
    }
    assert_eq!(seq_c, seq_r);
}
