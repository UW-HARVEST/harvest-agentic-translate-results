//! Differential tests for `decode_base64`, comparing the C shared library and
//! the Rust cdylib through their exported `decode_base64` symbol only.
//!
//! Ordered from the lowest-level behaviour reachable through the public ABI
//! (single characters, which exercise the static `is_base64` / `decode`
//! helpers) up to long, mixed, real-world inputs.

mod common;

use common::{Harness, Rng};

/// Every character `is_base64` accepts, plus the padding character.
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";

/// One representative of every branch in `is_base64` / `decode`, including
/// boundary characters just outside each accepted range.
const REPRESENTATIVES: &[u8] = b"AZaz09+/=@[`{:.\x2f\x7f\x80\xff \t\n-_*,";

// ---------------------------------------------------------------------------
// Level 0: NULL / empty handling
// ---------------------------------------------------------------------------

#[test]
fn null_pointer_input() {
    Harness::load().assert_same_null_input();
}

#[test]
fn empty_string_input() {
    Harness::load().assert_same(b"");
}

// ---------------------------------------------------------------------------
// Level 1: single bytes — exhaustively covers is_base64() and decode()
// ---------------------------------------------------------------------------

#[test]
fn all_single_bytes() {
    let h = Harness::load();
    for b in 1u16..=255 {
        h.assert_same(&[b as u8]);
    }
}

// ---------------------------------------------------------------------------
// Level 2: all byte pairs — covers the k+1 partial-group path for every
// combination of accepted/rejected characters.
// ---------------------------------------------------------------------------

#[test]
fn all_byte_pairs() {
    let h = Harness::load();
    for a in 1u16..=255 {
        for b in 1u16..=255 {
            h.assert_same(&[a as u8, b as u8]);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 3: exhaustive triples over the base64 alphabet (65^3 = 274_625).
// Covers every (c1, c2, c3) decode/padding combination for a 3-char group.
// ---------------------------------------------------------------------------

#[test]
fn all_alphabet_triples() {
    let h = Harness::load();
    for &a in ALPHABET {
        for &b in ALPHABET {
            for &c in ALPHABET {
                h.assert_same(&[a, b, c]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 4: exhaustive quadruples over branch representatives, so every
// combination of "=" placement and character class is hit in a full group.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_representative_quads() {
    let h = Harness::load();
    for &a in REPRESENTATIVES {
        for &b in REPRESENTATIVES {
            for &c in REPRESENTATIVES {
                for &d in REPRESENTATIVES {
                    h.assert_same(&[a, b, c, d]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 5: padding and length-not-multiple-of-4 edge cases
// ---------------------------------------------------------------------------

#[test]
fn padding_and_ragged_lengths() {
    let h = Harness::load();
    let cases: &[&[u8]] = &[
        b"=",
        b"==",
        b"===",
        b"====",
        b"=====",
        b"A=",
        b"A==",
        b"A===",
        b"AA",
        b"AA=",
        b"AA==",
        b"AA===",
        b"AAA",
        b"AAA=",
        b"AAAA",
        b"AAAA=",
        b"AAAAA",
        b"AAAAAA",
        b"AAAAAAA",
        b"AAAAAAAA",
        b"QQ==",
        b"QUI=",
        b"QUJD",
        b"QUJDRA==",
        b"QUJDRUY=",
        b"////",
        b"++++",
        b"/+/+",
        b"+/=/",
        b"=AAA",
        b"A=AA",
        b"AA=A",
        b"AAA=",
        b"=A=A",
        b"==AA",
        b"AAAA====",
        b"====AAAA",
        b"/w==",
        b"//8=",
        b"////AA==",
        b"zzzz",
        b"ZZZZ",
        b"9999",
        b"0000",
    ];
    for c in cases {
        h.assert_same(c);
    }
}

// ---------------------------------------------------------------------------
// Level 6: non-base64 characters are skipped ("as per the POSIX standard")
// ---------------------------------------------------------------------------

#[test]
fn ignored_characters() {
    let h = Harness::load();
    let cases: &[&[u8]] = &[
        b"!",
        b"!!!!",
        b"!@#$%^&*()",
        b" \t\r\n",
        b"Q U J D",
        b"QU\nJD",
        b"Q\tU\rJ\nD",
        b"-QUJD-",
        b"Q-U-J-D",
        b"...QUJD...",
        b"\x80\x81\xfe\xff",
        b"\xffQ\xffU\xffJ\xffD\xff",
        b"Q\x80U\x81J\x82D\x83",
        b"[]{}<>?,;:'\"\\|`~",
        b"QUJD\x7fRUY=",
        b"\x01\x02\x03\x04QUJD",
    ];
    for c in cases {
        h.assert_same(c);
    }
}

// ---------------------------------------------------------------------------
// Level 7: realistic / long inputs
// ---------------------------------------------------------------------------

#[test]
fn realistic_and_long_inputs() {
    let h = Harness::load();
    let cases: &[&[u8]] = &[
        b"SGVsbG8sIFdvcmxkIQ==",
        b"VGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZw==",
        b"dXNlcm5hbWU6cGFzc3dvcmQ=",
        b"YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXpBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWjAxMjM0NTY3ODkrLw==",
        b"SGVsbG8sIFdvcmxkIQ==\n",
        b"SGVsbG8s\nIFdvcmxk\nIQ==\n",
    ];
    for c in cases {
        h.assert_same(c);
    }

    // Long uniform and long alphabet-cycling inputs of many lengths, so the
    // destination sizing (strlen + 14) and the k += 4 stride are exercised
    // across every residue class.
    for len in 1..=300usize {
        let uniform: Vec<u8> = vec![b'A'; len];
        h.assert_same(&uniform);

        let cycling: Vec<u8> = (0..len).map(|i| ALPHABET[i % ALPHABET.len()]).collect();
        h.assert_same(&cycling);
    }

    // Very long input.
    let long: Vec<u8> = (0..8192).map(|i| ALPHABET[(i * 7) % ALPHABET.len()]).collect();
    h.assert_same(&long);
}

// ---------------------------------------------------------------------------
// Level 8: randomised differential fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_alphabet_only() {
    let h = Harness::load();
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..20_000 {
        let len = rng.below(64);
        let s: Vec<u8> = (0..len).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect();
        h.assert_same(&s);
    }
}

#[test]
fn fuzz_arbitrary_bytes() {
    let h = Harness::load();
    let mut rng = Rng::new(0xDEADBEEF);
    for _ in 0..20_000 {
        let len = rng.below(96);
        let s: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        h.assert_same(&s);
    }
}

#[test]
fn fuzz_mixed_valid_and_noise() {
    let h = Harness::load();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..20_000 {
        let len = rng.below(128);
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(3) == 0 {
                    rng.nonzero_byte()
                } else {
                    ALPHABET[rng.below(ALPHABET.len())]
                }
            })
            .collect();
        h.assert_same(&s);
    }
}

#[test]
fn fuzz_padding_heavy() {
    let h = Harness::load();
    let mut rng = Rng::new(0x0BADF00D);
    let pool = b"AZaz09+/====";
    for _ in 0..20_000 {
        let len = rng.below(40);
        let s: Vec<u8> = (0..len).map(|_| pool[rng.below(pool.len())]).collect();
        h.assert_same(&s);
    }
}
