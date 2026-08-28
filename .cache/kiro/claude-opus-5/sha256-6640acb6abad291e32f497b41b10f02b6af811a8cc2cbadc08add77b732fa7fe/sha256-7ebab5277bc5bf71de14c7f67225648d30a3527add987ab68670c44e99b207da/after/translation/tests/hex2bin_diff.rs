//! Differential tests for `hex2bin`, comparing the C .so against the Rust .so
//! purely through their exported `hex2bin` symbols.
//!
//! Ordering follows the call hierarchy: the character-classification arithmetic
//! is the lowest-level behaviour, then the `ignore`/`strchr` path, then the
//! output-buffer and `hex_end_p` bookkeeping, then whole-string conversions and
//! randomized fuzzing.

mod common;

use common::{Impls, Rng, assert_same};

// ---------------------------------------------------------------------------
// Level 1: character classification (c_num0 / c_alpha0 / c_val)
// ---------------------------------------------------------------------------

/// Every possible byte as the *first* nibble. Exercises the branch-free
/// classification for the whole 0..=255 domain, including the "invalid, and no
/// ignore set" break path.
#[test]
fn single_byte_all_values_no_ignore() {
    let impls = Impls::load();
    for b in 0u16..=255 {
        let hex = [b as u8];
        for &want_end in &[true, false] {
            assert_same(&impls, "single/no-ignore", 16, &hex, 1, None, want_end);
        }
    }
}

/// Every possible byte as the first nibble, with a non-NULL `ignore`.
#[test]
fn single_byte_all_values_with_ignore() {
    let impls = Impls::load();
    let ignores: [&[u8]; 5] = [b"", b" ", b" \t\n:", b"0123456789", b"\xff\xfe\x80"];
    for b in 0u16..=255 {
        let hex = [b as u8];
        for ig in ignores {
            for &want_end in &[true, false] {
                assert_same(&impls, "single/ignore", 16, &hex, 1, Some(ig), want_end);
            }
        }
    }
}

/// Every ordered pair drawn from a representative alphabet, as two nibbles.
/// Covers valid/valid, valid/invalid, invalid/valid and the `state` toggle.
#[test]
fn byte_pairs_representative() {
    let impls = Impls::load();
    // digits, both alpha cases, boundary/adjacent chars, separators, high bytes.
    let alphabet: &[u8] = b"0123456789abcdefABCDEFghGH/:@[`{ \t\n\r\0\x7f\x80\xffZz+-";
    for &x in alphabet {
        for &y in alphabet {
            let hex = [x, y];
            assert_same(&impls, "pair/no-ignore", 16, &hex, 2, None, true);
            assert_same(&impls, "pair/ignore", 16, &hex, 2, Some(b" :\0"), true);
            assert_same(&impls, "pair/no-end", 16, &hex, 2, None, false);
        }
    }
}

/// Exhaustive 2-byte sweep over the full 0..=255 x 0..=255 space.
#[test]
fn byte_pairs_exhaustive() {
    let impls = Impls::load();
    for x in 0u16..=255 {
        for y in 0u16..=255 {
            let hex = [x as u8, y as u8];
            assert_same(&impls, "pair2/exhaustive", 8, &hex, 2, None, true);
        }
    }
}

/// Same exhaustive sweep but with an `ignore` set that includes a NUL, a space
/// and a hex digit -- the `ignore` list is deliberately allowed to overlap the
/// hex alphabet, and the C code checks classification *first*, so overlapping
/// entries must be ignored.
#[test]
fn byte_pairs_exhaustive_with_ignore() {
    let impls = Impls::load();
    for x in 0u16..=255 {
        for y in 0u16..=255 {
            let hex = [x as u8, y as u8];
            assert_same(&impls, "pair2/ignore", 8, &hex, 2, Some(b" a\x80"), true);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: the `ignore` / strchr path
// ---------------------------------------------------------------------------

/// `strchr(ignore, 0)` finds the terminator, so an embedded NUL in `hex` is
/// skipped whenever `ignore` is non-NULL and `state == 0`.
#[test]
fn embedded_nul_is_ignored_via_terminator() {
    let impls = Impls::load();
    let cases: &[&[u8]] = &[
        b"\0",
        b"\0\0",
        b"a\0b",
        b"\0ab",
        b"ab\0",
        b"a\0",
        b"\0a",
        b"de\0ad\0be\0ef",
        b"\0\0\0\0",
    ];
    for hex in cases {
        for ig in [None, Some(&b""[..]), Some(&b" "[..]), Some(&b"\0"[..])] {
            for &want_end in &[true, false] {
                assert_same(&impls, "nul-ignore", 32, hex, hex.len(), ig, want_end);
            }
        }
    }
}

/// `ignore` is only consulted while `state == 0`, i.e. never mid-byte.
#[test]
fn ignore_only_honoured_between_bytes() {
    let impls = Impls::load();
    let cases: &[&[u8]] = &[
        b"a b",       // ignorable char while state==1 -> break, odd nibble
        b" ab",       // leading ignorable
        b"ab ",       // trailing ignorable
        b"a bc",      //
        b"ab cd",     // ignorable exactly between bytes
        b"  ab  cd  ",
        b"a:b:c:d",
        b"de ad be ef",
        b"de:ad:be:ef",
        b"::::",
        b" ",
        b"   ",
    ];
    for hex in cases {
        for ig in [None, Some(&b" "[..]), Some(&b" :"[..]), Some(&b":"[..])] {
            for &want_end in &[true, false] {
                assert_same(&impls, "ignore-state", 32, hex, hex.len(), ig, want_end);
            }
        }
    }
}

/// High-bit bytes in `ignore`: C passes `c` as `int` to `strchr`, which compares
/// against `(char)c`, so 0x80..0xFF must still match.
#[test]
fn ignore_high_bit_bytes() {
    let impls = Impls::load();
    for b in 0x80u16..=0xFF {
        let hex = [b as u8, b'a', b'b'];
        let ig = [b as u8, 0];
        assert_same(&impls, "ignore-high", 8, &hex, 3, Some(&ig), true);
        assert_same(&impls, "ignore-high-no-end", 8, &hex, 3, Some(&ig), false);
    }
}

// ---------------------------------------------------------------------------
// Level 3: bin_maxlen and hex_end_p bookkeeping
// ---------------------------------------------------------------------------

#[test]
fn bin_maxlen_boundaries() {
    let impls = Impls::load();
    let hex: &[u8] = b"deadbeef";
    for bin_maxlen in 0..=8usize {
        for hex_len in 0..=hex.len() {
            for &want_end in &[true, false] {
                assert_same(
                    &impls,
                    "maxlen",
                    bin_maxlen,
                    hex,
                    hex_len,
                    None,
                    want_end,
                );
                assert_same(
                    &impls,
                    "maxlen/ignore",
                    bin_maxlen,
                    hex,
                    hex_len,
                    Some(b" "),
                    want_end,
                );
            }
        }
    }
}

/// Truncation triggers `ret = -1` mid-byte and mid-stream; check `hex_end_p`
/// and the (zeroed) return in every combination.
#[test]
fn truncation_and_odd_nibbles() {
    let impls = Impls::load();
    let cases: &[&[u8]] = &[
        b"", b"a", b"ab", b"abc", b"abcd", b"abcde", b"abcdef", b"0", b"00", b"000",
        b"f", b"ff", b"fff", b"ffff", b"FFFFF",
    ];
    for hex in cases {
        for bin_maxlen in 0..=4usize {
            for &want_end in &[true, false] {
                assert_same(&impls, "odd", bin_maxlen, hex, hex.len(), None, want_end);
                assert_same(
                    &impls,
                    "odd/ignore",
                    bin_maxlen,
                    hex,
                    hex.len(),
                    Some(b" -"),
                    want_end,
                );
            }
        }
    }
}

/// `hex_len` shorter than the buffer: the callee must stop at `hex_len` and
/// never look past it (checked implicitly by identical `hex_end` offsets).
#[test]
fn partial_hex_len() {
    let impls = Impls::load();
    let hex: &[u8] = b"0123456789abcdefABCDEF !@\0zz";
    for hex_len in 0..=hex.len() {
        for &bin_maxlen in &[0usize, 1, 3, 8, 64] {
            for ig in [None, Some(&b" !@"[..])] {
                for &want_end in &[true, false] {
                    assert_same(
                        &impls, "partial", bin_maxlen, hex, hex_len, ig, want_end,
                    );
                }
            }
        }
    }
}

/// Zero-length input, including a NULL-ish (zero-capacity) output buffer.
#[test]
fn empty_input() {
    let impls = Impls::load();
    for &want_end in &[true, false] {
        assert_same(&impls, "empty", 0, b"", 0, None, want_end);
        assert_same(&impls, "empty", 0, b"", 0, Some(b" "), want_end);
        assert_same(&impls, "empty", 16, b"", 0, None, want_end);
        assert_same(&impls, "empty", 16, b"", 0, Some(b""), want_end);
    }
}

// ---------------------------------------------------------------------------
// Level 4: full-string conversions
// ---------------------------------------------------------------------------

#[test]
fn full_round_trip_all_byte_values() {
    let impls = Impls::load();
    // Every byte 0x00..=0xFF encoded lower-case, then upper-case, then mixed.
    let mut lower = Vec::new();
    let mut upper = Vec::new();
    let mut mixed = Vec::new();
    for b in 0u16..=255 {
        let s = format!("{:02x}", b);
        lower.extend_from_slice(s.as_bytes());
        upper.extend_from_slice(s.to_uppercase().as_bytes());
        let bytes = s.as_bytes();
        mixed.push(bytes[0].to_ascii_uppercase());
        mixed.push(bytes[1]);
    }
    for hex in [&lower, &upper, &mixed] {
        for &want_end in &[true, false] {
            assert_same(&impls, "roundtrip", 256, hex, hex.len(), None, want_end);
            assert_same(&impls, "roundtrip", 255, hex, hex.len(), None, want_end);
            assert_same(
                &impls,
                "roundtrip/ignore",
                256,
                hex,
                hex.len(),
                Some(b" :-"),
                want_end,
            );
        }
    }
}

#[test]
fn separated_full_strings() {
    let impls = Impls::load();
    let mut colon = Vec::new();
    for b in 0u16..=255 {
        if !colon.is_empty() {
            colon.push(b':');
        }
        colon.extend_from_slice(format!("{:02X}", b).as_bytes());
    }
    for ig in [None, Some(&b":"[..]), Some(&b" "[..])] {
        for &want_end in &[true, false] {
            assert_same(&impls, "colon", 256, &colon, colon.len(), ig, want_end);
            assert_same(&impls, "colon-tight", 4, &colon, colon.len(), ig, want_end);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 5: randomized fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_arbitrary_bytes() {
    let impls = Impls::load();
    let mut rng = Rng(0x1234_5678_9abc_def0);
    let ignore_sets: [&[u8]; 6] = [b"", b" ", b" \t\r\n", b":-", b"0aF\0", b"\xff\x80 "];

    for _ in 0..4000 {
        let len = rng.below(24);
        let hex: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let hex_len = rng.below(len + 1);
        let bin_maxlen = rng.below(16);
        let ig = match rng.below(ignore_sets.len() + 1) {
            0 => None,
            k => Some(ignore_sets[k - 1]),
        };
        let want_end = rng.below(2) == 0;
        assert_same(&impls, "fuzz-bytes", bin_maxlen, &hex, hex_len, ig, want_end);
    }
}

#[test]
fn fuzz_mostly_valid_hex() {
    let impls = Impls::load();
    let mut rng = Rng(0x0bad_c0de_dead_beef);
    // Alphabet skewed towards things that actually parse, plus separators and
    // the occasional junk byte, so long successful runs are exercised too.
    let alphabet: &[u8] = b"0123456789abcdefABCDEF0123456789abcdef :\0-Zz\xff";
    let ignore_sets: [&[u8]; 4] = [b"", b" ", b" :-", b" :\0"];

    for _ in 0..4000 {
        let len = rng.below(40);
        let hex: Vec<u8> = (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let bin_maxlen = rng.below(24);
        let ig = match rng.below(ignore_sets.len() + 1) {
            0 => None,
            k => Some(ignore_sets[k - 1]),
        };
        let want_end = rng.below(2) == 0;
        assert_same(&impls, "fuzz-hex", bin_maxlen, &hex, len, ig, want_end);
    }
}

#[test]
fn fuzz_long_inputs() {
    let impls = Impls::load();
    let mut rng = Rng(0xfeed_face_cafe_babe);
    let alphabet: &[u8] = b"0123456789abcdefABCDEF :";
    for _ in 0..200 {
        let len = 256 + rng.below(1024);
        let hex: Vec<u8> = (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let bin_maxlen = rng.below(len + 1);
        let ig = if rng.below(2) == 0 { None } else { Some(&b" :"[..]) };
        let want_end = rng.below(2) == 0;
        assert_same(&impls, "fuzz-long", bin_maxlen, &hex, len, ig, want_end);
    }
}
