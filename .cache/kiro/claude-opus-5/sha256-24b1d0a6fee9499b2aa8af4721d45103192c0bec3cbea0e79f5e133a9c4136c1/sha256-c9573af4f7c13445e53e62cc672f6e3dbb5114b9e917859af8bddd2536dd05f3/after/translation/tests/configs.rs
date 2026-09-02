//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH `.so`s through
//! their exported `encode_base64` symbol and compares the whole allocated
//! buffer byte for byte (plus `malloc_usable_size`).

mod common;

use common::{assert_same, Rng, SEED};
use std::ffi::c_int;

/// Pack four 6-bit sextets into the three input bytes that produce them.
/// (The C splits 24 bits of input into `b4,b5,b6,b7`.)
fn from_sextets(s: [u8; 4]) -> [u8; 3] {
    let bits: u32 = ((s[0] as u32 & 0x3f) << 18)
        | ((s[1] as u32 & 0x3f) << 12)
        | ((s[2] as u32 & 0x3f) << 6)
        | (s[3] as u32 & 0x3f);
    [(bits >> 16) as u8, (bits >> 8) as u8, bits as u8]
}

// ---------------------------------------------------------------- row 1
#[test]
fn config_01_exhaustive_one_byte() {
    for v in 0u16..=255 {
        let payload = [v as u8];
        let out = assert_same(1, &payload, "row1 size=1").expect("non-NULL");
        assert_eq!(out.len(), 4, "row1: 1 byte must encode to 4 chars");
        assert_eq!(&out[2..], b"==", "row1: 1 byte must be double padded");
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn config_02_exhaustive_two_bytes() {
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            let payload = [a as u8, b as u8];
            let out = assert_same(2, &payload, "row2 size=2").expect("non-NULL");
            assert_eq!(out.len(), 4);
            assert_eq!(out[3], b'=', "row2: 2 bytes must be single padded");
            assert_ne!(out[2], b'=');
        }
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn config_03_three_bytes_random_and_systematic() {
    // systematic: every sextet value in every position, and all-equal sextets
    for v in 0u8..64 {
        for pos in 0..4 {
            let mut s = [0u8; 4];
            s[pos] = v;
            assert_same(3, &from_sextets(s), "row3 systematic");
        }
        assert_same(3, &from_sextets([v; 4]), "row3 all-equal");
    }
    // randomized
    let mut rng = Rng::new(SEED);
    for _ in 0..200_000 {
        let payload = [rng.byte(), rng.byte(), rng.byte()];
        let out = assert_same(3, &payload, "row3 random").expect("non-NULL");
        assert_eq!(out.len(), 4);
        assert!(!out.contains(&b'='), "row3: exact multiple of 3 => no padding");
    }
}

// ------------------------------------------------------------- rows 4,5,6
#[test]
fn config_04_05_06_two_groups() {
    let mut rng = Rng::new(SEED ^ 0x456);
    for size in [4i32, 5, 6] {
        for _ in 0..20_000 {
            let payload = rng.bytes(size as usize);
            let out = assert_same(size, &payload, "rows4-6").expect("non-NULL");
            assert_eq!(out.len(), 8);
            match size % 3 {
                0 => assert!(!out.contains(&b'=')),
                1 => assert_eq!(&out[6..], b"=="),
                _ => {
                    assert_eq!(out[7], b'=');
                    assert_ne!(out[6], b'=');
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn config_07_every_size_1_to_256() {
    let mut rng = Rng::new(SEED ^ 0x777);
    for size in 1i32..=256 {
        for _ in 0..64 {
            let payload = rng.bytes(size as usize);
            let out = assert_same(size, &payload, "row7").expect("non-NULL");
            let groups = ((size as usize) + 2) / 3;
            assert_eq!(out.len(), groups * 4);
        }
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn config_08_large_random_sizes() {
    let mut rng = Rng::new(SEED ^ 0x888);
    for _ in 0..400 {
        let size = 257 + rng.below(4096 - 257) as c_int;
        let payload = rng.bytes(size as usize);
        assert_same(size, &payload, "row8").expect("non-NULL");
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn config_09_all_zero_payload() {
    for size in 1i32..=64 {
        let payload = vec![0u8; size as usize];
        let out = assert_same(size, &payload, "row9 zeros").expect("non-NULL");
        assert!(
            out.iter().all(|&c| c == b'A' || c == b'='),
            "row9: zero bytes must encode to 'A' (encode() branch 1)"
        );
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn config_10_all_ff_payload() {
    for size in 1i32..=64 {
        let payload = vec![0xFFu8; size as usize];
        let out = assert_same(size, &payload, "row10 0xFF").expect("non-NULL");
        assert!(
            out.contains(&b'/'),
            "row10: 0xFF must reach encode()'s fallthrough '/'"
        );
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn config_11_sextet_62_plus() {
    // every position forced to 62 -> '+'
    for pos in 0..4 {
        let mut s = [0u8; 4];
        s[pos] = 62;
        let out = assert_same(3, &from_sextets(s), "row11 '+'").expect("non-NULL");
        assert_eq!(out[pos], b'+', "row11: sextet 62 must encode to '+'");
    }
    let out = assert_same(3, &from_sextets([62; 4]), "row11 all '+'").expect("non-NULL");
    assert_eq!(&out[..], b"++++");
}

// ---------------------------------------------------------------- row 12
#[test]
fn config_12_sextet_letter_and_digit_branches() {
    // branch 2: 26..=51 -> 'a'..'z'   branch 3: 52..=61 -> '0'..'9'
    for v in 26u8..=61 {
        for pos in 0..4 {
            let mut s = [0u8; 4];
            s[pos] = v;
            let out = assert_same(3, &from_sextets(s), "row12").expect("non-NULL");
            let expect = if v < 52 {
                b'a' + (v - 26)
            } else {
                b'0' + (v - 52)
            };
            assert_eq!(out[pos], expect);
        }
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn config_13_full_sextet_domain_every_position() {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for v in 0u8..64 {
        for pos in 0..4 {
            let mut s = [0u8; 4];
            s[pos] = v;
            let out = assert_same(3, &from_sextets(s), "row13").expect("non-NULL");
            assert_eq!(out.len(), 4);
            assert_eq!(out[pos], TABLE[v as usize], "row13 sextet {v} at pos {pos}");
        }
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn config_14_high_bit_bytes_only() {
    let mut rng = Rng::new(SEED ^ 0xA1A1);
    for size in 1i32..=48 {
        for _ in 0..200 {
            let payload: Vec<u8> = (0..size).map(|_| rng.byte() | 0x80).collect();
            assert_same(size, &payload, "row14 high-bit").expect("non-NULL");
        }
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn config_15_embedded_nul_bytes() {
    let mut rng = Rng::new(SEED ^ 0xB2B2);
    for size in 1i32..=48 {
        for _ in 0..200 {
            let mut payload = rng.bytes(size as usize);
            let holes = 1 + rng.below(4);
            for _ in 0..holes {
                let idx = rng.below(size as usize);
                payload[idx] = 0;
            }
            let out = assert_same(size, &payload, "row15 embedded NUL").expect("non-NULL");
            let groups = ((size as usize) + 2) / 3;
            assert_eq!(
                out.len(),
                groups * 4,
                "row15: embedded NULs must be encoded, not terminate"
            );
        }
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn config_16_size_smaller_than_buffer() {
    let mut rng = Rng::new(SEED ^ 0xC3C3);
    for _ in 0..5_000 {
        let size = 1 + rng.below(64) as c_int;
        let slack = rng.below(32);
        let mut buf = vec![0xAAu8; size as usize + slack];
        for i in 0..size as usize {
            buf[i] = rng.byte();
        }
        assert_same(size, &buf, "row16 slack").expect("non-NULL");
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn config_17_large_exact_multiple_of_three() {
    let mut rng = Rng::new(SEED ^ 0xD4D4);
    for _ in 0..20 {
        let payload = rng.bytes(3072);
        let out = assert_same(3072, &payload, "row17").expect("non-NULL");
        assert_eq!(out.len(), 4096);
        assert!(!out.contains(&b'='));
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn config_18_capacity_tight_sizes() {
    let mut rng = Rng::new(SEED ^ 0xE5E5);
    // size % 3 == 1 is the tightest case: written = 4*ceil(size/3), cap = size*4/3+4
    let mut sizes: Vec<c_int> = (0..80).map(|k| 1 + 3 * k).collect();
    sizes.extend([2, 3, 5, 6, 8, 9, 3071, 3073]);
    for size in sizes {
        for _ in 0..50 {
            let payload = rng.bytes(size as usize);
            let out = assert_same(size, &payload, "row18 tight cap").expect("non-NULL");
            let groups = ((size as usize) + 2) / 3;
            assert_eq!(out.len(), groups * 4);
            assert!(
                groups * 4 <= common::cap_of(size) as usize,
                "row18: written bytes must fit the C capacity"
            );
        }
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn config_19_strlen_mode_all_lengths() {
    let mut rng = Rng::new(SEED ^ 0xF6F6);
    for len in 0usize..=128 {
        for _ in 0..40 {
            // NUL-terminated, no interior NULs
            let mut s: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 255)).collect();
            s.push(0);
            let out = assert_same(0, &s, "row19 strlen mode");
            let out = out.expect("non-NULL");
            let groups = (len + 2) / 3;
            assert_eq!(out.len(), groups * 4, "row19 len={len}");
        }
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn config_20_strlen_mode_high_bit_bytes() {
    let mut rng = Rng::new(SEED ^ 0x0707);
    for len in 1usize..=64 {
        for _ in 0..40 {
            let mut s: Vec<u8> = (0..len).map(|_| rng.byte() | 0x80).collect();
            s.push(0);
            assert_same(0, &s, "row20 strlen high-bit").expect("non-NULL");
        }
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn config_21_strlen_mode_empty_string() {
    let out = assert_same(0, b"\0", "row21 empty").expect("non-NULL");
    assert!(out.is_empty(), "row21: empty input => empty output");
}

// ---------------------------------------------------------------- row 22
#[test]
fn config_22_strlen_mode_truncates_at_nul() {
    for content in [
        &b"a\0GARBAGE"[..],
        &b"ab\0\xff\xff\xff"[..],
        &b"abc\0zzzz"[..],
        &b"abcd\0\x01\x02"[..],
        &b"hello world\0trailing junk here"[..],
    ] {
        let n = content.iter().position(|&c| c == 0).unwrap();
        let out = assert_same(0, content, "row22 truncation").expect("non-NULL");
        let groups = (n + 2) / 3;
        assert_eq!(out.len(), groups * 4);
        // cross-check: same bytes with an explicit size must give the same thing
        let explicit = assert_same(n as c_int, &content[..n], "row22 explicit").expect("non-NULL");
        assert_eq!(out, explicit);
    }
}

// ---------------------------------------------------------------- row 23
#[test]
fn config_23_negative_size_allocation_succeeds() {
    let payload = b"some bytes that must never be read";
    for size in [-1i32, -2] {
        let out = assert_same(size, payload, "row23 negative size").expect("non-NULL");
        assert!(out.is_empty(), "row23: loop is skipped for size<0");
    }
}

// ---------------------------------------------------------------- row 24
#[test]
fn config_24_int_min_wraparound() {
    let payload = b"never read";
    for size in [c_int::MIN, c_int::MIN + 1, c_int::MIN + 2, c_int::MIN + 3] {
        let out = assert_same(size, payload, "row24 INT_MIN wrap");
        if let Some(o) = out {
            assert!(o.is_empty(), "row24: loop skipped, buffer stays zeroed");
        }
    }
}

// ---------------------------------------------------------------- row 25
#[test]
fn config_25_randomized_fuzz_sweep() {
    let mut rng = Rng::new(SEED ^ 0x2525);
    for _ in 0..20_000 {
        let size = 1 + rng.below(512) as c_int;
        let slack = rng.below(8);
        let payload = rng.bytes(size as usize + slack);
        assert_same(size, &payload, "row25 fuzz").expect("non-NULL");
    }
}
