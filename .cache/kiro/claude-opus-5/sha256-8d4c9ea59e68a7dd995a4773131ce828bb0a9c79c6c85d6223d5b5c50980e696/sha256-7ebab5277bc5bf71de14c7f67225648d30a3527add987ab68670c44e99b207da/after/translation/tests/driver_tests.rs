//! Tests for the top-level exported function, `driver`.
//!
//! `driver` exercises the static `parse_val` (i.e. `strtol` + range checks)
//! and then calls `run` twice on the same struct.

mod common;

use common::assert_driver_matches;

fn driver_simple_values() {
    for s in ["0", "1", "2", "5", "10", "-1", "-5", "+5", "+0", "-0", "007", "0000000000000005"] {
        assert_driver_matches(s.as_bytes());
    }
}

fn driver_int_boundaries() {
    for s in [
        "2147483647",  // INT_MAX
        "2147483646",
        "2147483648",  // INT_MAX + 1
        "-2147483648", // INT_MIN
        "-2147483647",
        "-2147483649", // INT_MIN - 1
        "4294967295",
        "4294967296",
    ] {
        assert_driver_matches(s.as_bytes());
    }
}

fn driver_long_boundaries() {
    for s in [
        "9223372036854775807",  // LONG_MAX
        "9223372036854775806",
        "9223372036854775808",  // LONG_MAX + 1 -> ERANGE
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775807",
        "-9223372036854775809", // LONG_MIN - 1 -> ERANGE
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999999999999999",
        "-99999999999999999999999999999999",
    ] {
        assert_driver_matches(s.as_bytes());
    }
}

fn driver_no_conversion() {
    for s in [
        "", " ", "  ", "abc", "+", "-", "++", "--", "+-5", "-+5", ".", ".5", "-.5", "e5", "x",
        "\t", "\n", "\r", "\x0b", "\x0c", " \t\n\r\x0b\x0c", "  +  5", "- 5", "null", "NaN",
        "inf", "0x", "/", ":", "@", "\u{7f}",
    ] {
        assert_driver_matches(s.as_bytes());
    }
}

fn driver_partial_conversion() {
    for s in [
        "12abc", "5 ", "5\n", " 42", "\t42", "\n\r 42", "  -7  ", "0x10", "0X10", "010", "1e5",
        "3.9", "-3.9", "7,8", "9-", "1 2 3", "+12xyz", "-12xyz", "  0009zzz",
    ] {
        assert_driver_matches(s.as_bytes());
    }
}

fn driver_leading_zero_padding_of_huge_values() {
    // Long strings of leading zeros must not be mistaken for overflow.
    let zeros = "0".repeat(400);
    for tail in ["0", "1", "5", "2147483647", "2147483648", "9223372036854775807"] {
        assert_driver_matches(format!("{zeros}{tail}").as_bytes());
        assert_driver_matches(format!("-{zeros}{tail}").as_bytes());
        assert_driver_matches(format!("+{zeros}{tail}").as_bytes());
    }
}

fn driver_very_long_digit_runs() {
    for n in [19usize, 20, 21, 40, 100, 1000] {
        assert_driver_matches("9".repeat(n).as_bytes());
        assert_driver_matches(format!("-{}", "9".repeat(n)).as_bytes());
        assert_driver_matches("1".repeat(n).as_bytes());
    }
}

fn driver_high_bytes() {
    // Non-ASCII / high bytes must be treated as non-digits, not as whitespace.
    for b in [0x80u8, 0xa0, 0xc3, 0xff] {
        assert_driver_matches(&[b]);
        assert_driver_matches(&[b, b'5']);
        assert_driver_matches(&[b'5', b]);
        assert_driver_matches(&[b' ', b, b'5']);
    }
}

fn driver_exhaustive_small_range() {
    for v in -300i64..=300 {
        assert_driver_matches(v.to_string().as_bytes());
    }
}

fn driver_pseudo_random_numeric_sweep() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..400 {
        let v = next() as i64;
        assert_driver_matches(v.to_string().as_bytes());
        assert_driver_matches((v as i32).to_string().as_bytes());
        assert_driver_matches((v as u64).to_string().as_bytes());
    }
}

fn driver_pseudo_random_fuzz_strings() {
    const ALPHABET: &[u8] = b"0123456789+- \t\nabxX.,\x0b\x0c\x0d";
    let mut state: u64 = 0xDEAD_BEEF_1234_5678;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..600 {
        let len = (next() % 12) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_driver_matches(&s);
    }
}

/// Every byte value used as a leading character, to pin down `strtol`'s
/// whitespace classification exactly (the Rust side hand-rolls `isspace`).
fn driver_every_leading_byte() {
    for b in 1u8..=255 {
        assert_driver_matches(&[b]);
        assert_driver_matches(&[b, b'5']);
        assert_driver_matches(&[b, b'-', b'5']);
        assert_driver_matches(&[b, b, b'7']);
        assert_driver_matches(&[b' ', b, b'7']);
        assert_driver_matches(&[b'-', b, b'7']);
        assert_driver_matches(&[b'4', b, b'7']);
    }
}

/// Exhaustive sweep of all short strings over a hostile alphabet.
fn driver_exhaustive_short_strings() {
    const ALPHABET: &[u8] = b"09+- \ta.\x0b";
    let n = ALPHABET.len();
    let mut word = Vec::new();
    for len in 0..=3usize {
        let total = n.pow(len as u32);
        for mut idx in 0..total {
            word.clear();
            for _ in 0..len {
                word.push(ALPHABET[idx % n]);
                idx /= n;
            }
            assert_driver_matches(&word);
        }
    }
}

/// Decorated forms of every interesting numeric boundary: sign variants,
/// leading whitespace, zero padding and trailing junk.
fn driver_decorated_boundaries() {
    let magnitudes: [&str; 12] = [
        "0",
        "1",
        "2147483647",
        "2147483648",
        "2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551616",
    ];
    let prefixes = ["", " ", "\t", "\n", "  \t ", "\x0b\x0c\r "];
    let signs = ["", "+", "-"];
    let pads = ["", "0", "000000000000000000000"];
    let suffixes = ["", " ", "x", "0", "-", "\n"];
    for m in magnitudes {
        for p in prefixes {
            for s in signs {
                for z in pads {
                    for suf in suffixes {
                        assert_driver_matches(format!("{p}{s}{z}{m}{suf}").as_bytes());
                    }
                }
            }
        }
    }
}

// Single entry point: fd 1 redirection during capture is process-global, so
// this binary must run exactly one libtest test.
#[test]
fn all_cases() {
    driver_simple_values();
    driver_int_boundaries();
    driver_long_boundaries();
    driver_no_conversion();
    driver_partial_conversion();
    driver_leading_zero_padding_of_huge_values();
    driver_very_long_digit_runs();
    driver_high_bytes();
    driver_every_leading_byte();
    driver_exhaustive_small_range();
    driver_exhaustive_short_strings();
    driver_decorated_boundaries();
    driver_pseudo_random_numeric_sweep();
    driver_pseudo_random_fuzz_strings();
}
