//! Differential tests for `driver` (the only symbol the C library exports).
//!
//! Every call goes through `dlopen`/`dlsym` on both `libdriver.so` builds, so
//! the Rust `#[no_mangle]` export wrapper is exercised the same way an external
//! C caller would exercise it.

mod common;

use common::{Libs, compare};

/// The C library's exported surface, so a missing Rust export fails loudly.
#[test]
fn exports_driver_symbol() {
    let libs = Libs::load();
    // Resolving the symbol in both libraries is the assertion.
    let _ = libs.c_driver();
    let _ = libs.rust_driver();
}

#[test]
fn empty_inputs() {
    let libs = Libs::load();
    compare(&libs, b"", b"", "both empty");
    compare(&libs, b"", b"abc", "empty s1");
    compare(&libs, b"abc", b"", "empty s2 -> strlen(s1)");
}

#[test]
fn basic_matches() {
    let libs = Libs::load();
    let cases: &[(&[u8], &[u8])] = &[
        (b"fcba73", b"1234567890"),
        (b"hello world", b"o"),
        (b"hello world", b"h"),
        (b"hello world", b"d"),
        (b"hello world", b"xyz"),
        (b"abcdef", b"f"),
        (b"abcdef", b"a"),
        (b"abcdef", b"fa"),
        (b"aaaaaa", b"a"),
        (b"aaaaaa", b"b"),
        (b"The quick brown fox", b" "),
        (b"The quick brown fox", b"aeiou"),
    ];
    for (s1, s2) in cases {
        compare(&libs, s1, s2, "basic");
    }
}

#[test]
fn reject_set_duplicates_and_order() {
    let libs = Libs::load();
    compare(&libs, b"abcdefg", b"gggggg", "duplicate reject bytes");
    compare(&libs, b"abcdefg", b"gfedcba", "reversed reject set");
    compare(&libs, b"abcdefg", b"zzzzzzzzzzzzzzzzzzzzzzzzc", "long reject set, late hit");
}

#[test]
fn match_at_every_position() {
    let libs = Libs::load();
    let s1 = b"0123456789abcdef";
    for i in 0..s1.len() {
        compare(&libs, s1, &[s1[i]], "single-byte reject at each index");
    }
}

#[test]
fn high_bytes_and_signedness() {
    let libs = Libs::load();
    // Bytes above 0x7F are where a signed-char index bug would show up.
    compare(&libs, b"ab\x80cd", b"\x80", "0x80 in reject set");
    compare(&libs, b"ab\xffcd", b"\xff", "0xff in reject set");
    compare(&libs, b"\x80\x81\x82", b"\x82", "all-high s1");
    compare(&libs, b"abc\xfe", b"\x7f", "no match, high byte in s1");
    compare(&libs, b"\xc3\xa9abc", b"a", "utf-8 lead bytes");

    // Every non-NUL byte value, as both haystack and reject set.
    let all: Vec<u8> = (1u8..=255).collect();
    compare(&libs, &all, b"\x01", "all bytes, reject first");
    compare(&libs, &all, b"\xff", "all bytes, reject last");
    for b in 1u8..=255 {
        compare(&libs, &all, &[b], "all bytes, single reject");
        compare(&libs, &[b], &all, "single byte vs all-byte reject set");
    }
}

#[test]
fn whole_byte_range_as_reject_set() {
    let libs = Libs::load();
    let all: Vec<u8> = (1u8..=255).collect();
    compare(&libs, b"anything", &all, "reject everything");
    compare(&libs, &all, &all, "reject everything, long s1");
}

#[test]
fn long_strings_and_wide_values() {
    let libs = Libs::load();
    // Exercises multi-digit %zu formatting.
    for len in [9usize, 10, 99, 100, 999, 1000, 4095, 65536] {
        let s1 = vec![b'a'; len];
        compare(&libs, &s1, b"z", "long run, no match");
        let mut s3 = vec![b'a'; len];
        *s3.last_mut().unwrap() = b'z';
        compare(&libs, &s3, b"z", "long run, match at end");
    }
}

#[test]
fn pseudorandom_fuzz() {
    let libs = Libs::load();
    // Deterministic xorshift so failures are reproducible.
    let mut state: u64 = 0x2545F4914F6CDD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..400 {
        let n1 = (next() % 64) as usize;
        let n2 = (next() % 8) as usize;
        // Draw from a small alphabet so hits are common, plus raw bytes.
        let s1: Vec<u8> = (0..n1).map(|_| 1 + (next() % 255) as u8).collect();
        let s2: Vec<u8> = (0..n2).map(|_| 1 + (next() % 255) as u8).collect();
        compare(&libs, &s1, &s2, "fuzz wide alphabet");

        let s1: Vec<u8> = (0..n1).map(|_| b'a' + (next() % 4) as u8).collect();
        let s2: Vec<u8> = (0..n2).map(|_| b'a' + (next() % 4) as u8).collect();
        compare(&libs, &s1, &s2, "fuzz narrow alphabet");
    }
}

#[test]
fn repeated_calls_share_stream_state() {
    let libs = Libs::load();
    // Several calls in a row, to catch buffering/flush differences.
    for _ in 0..3 {
        compare(&libs, b"abcdef", b"c", "repeat");
        compare(&libs, b"", b"", "repeat empty");
        compare(&libs, b"zzzz", b"q", "repeat no match");
    }
}
