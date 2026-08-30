//! Randomised differential tests with fixed seeds, so any failure is
//! reproducible. These exist to catch input classes the enumerated cases in
//! `differential.rs` missed.

mod harness;

use harness::{assert_same, Rng};

/// Tokens drawn from the classes `main` branches on, plus near-misses.
const TOKENS: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "7", "0", "8", "9", "-1", "+2", " 3", "\t4", "  ", "", "abc",
    "x", "007", "2147483648", "-2147483649", "4294967303", "99999999999999999999", "0x3", "3junk",
    "\r", "1e5", ".5", "--3", "+", "-", "\u{0}", "\u{b}", "\u{c}", "7 trailing",
];

/// Menu sessions built from realistic tokens and line separators.
#[test]
fn fuzz_token_sessions() {
    let mut rng = Rng::new(0x5eed_1234);
    for case in 0..200 {
        let lines = rng.below(7);
        let mut parts: Vec<&str> = Vec::new();
        for _ in 0..lines {
            parts.push(TOKENS[rng.below(TOKENS.len())]);
        }
        let sep = if rng.below(4) == 0 { "\r\n" } else { "\n" };
        let mut s = parts.join(sep);
        if rng.below(5) != 0 {
            s.push('\n');
        }
        let mut bytes = s.into_bytes();
        if rng.below(20) == 0 {
            // Occasionally overflow the 256-byte `fgets` buffer.
            bytes.extend(std::iter::repeat(b' ').take(250 + rng.below(60)));
            bytes.extend_from_slice(b"7\n");
        }
        assert_same(&format!("fuzz_tokens case {case}"), &bytes);
    }
}

/// Arbitrary byte soup, including NULs and invalid UTF-8, to make sure nothing
/// in the Rust I/O path chokes where C would not.
#[test]
fn fuzz_random_bytes() {
    let mut rng = Rng::new(0xabcd_ef01);
    for case in 0..150 {
        let len = rng.below(600);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u64() & 0xff) as u8).collect();
        assert_same(&format!("fuzz_bytes case {case}"), &bytes);
    }
}

/// Byte soup biased towards digits, whitespace and newlines, so more of it
/// actually reaches the demos.
#[test]
fn fuzz_digit_heavy_bytes() {
    const ALPHABET: &[u8] = b"01234567890123456789 \t\r\n\n\n+-\0abx";
    let mut rng = Rng::new(0x1357_9bdf);
    for case in 0..150 {
        let len = rng.below(400);
        let bytes: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect();
        assert_same(&format!("fuzz_digits case {case}"), &bytes);
    }
}

/// Long sessions: many demos in a row, checking output stays identical as the
/// byte count grows into the megabytes.
#[test]
fn fuzz_long_sessions() {
    let mut rng = Rng::new(0x2468_ace0);
    for case in 0..10 {
        let mut s = String::new();
        for _ in 0..60 {
            s.push_str(&format!("{}\n", 1 + rng.below(6)));
        }
        s.push_str("7\n");
        assert_same(&format!("fuzz_long case {case}"), s.as_bytes());
    }
}
