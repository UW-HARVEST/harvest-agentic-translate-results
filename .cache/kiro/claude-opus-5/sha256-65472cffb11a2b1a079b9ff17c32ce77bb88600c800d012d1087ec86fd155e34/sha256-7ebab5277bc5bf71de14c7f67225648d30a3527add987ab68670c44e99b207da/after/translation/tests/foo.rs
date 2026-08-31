//! Lowest level of the API: `int foo(const char *in, char c)`.
//!
//! `foo` is not declared in driver.h but has external linkage in driver.c, so
//! it is part of the `.so`'s ABI and is exercised directly here.
//!
//! Note on `c == '\0'`: the C loop does `s = strchr(s, c); s++`, so a NUL
//! needle matches the terminator and the next iteration scans past the end of
//! the buffer. That is out-of-bounds in the C original, and any comparison
//! would depend on unrelated heap contents, so it is deliberately not tested.

mod common;

use common::{Libs, Rng, cstr};
use std::ffi::{c_char, c_int};

/// Calls both `.so` exports with the same input and asserts the results match.
fn check(libs: &Libs, bytes: &[u8], c: u8) -> c_int {
    let (c_foo, rust_foo) = libs.foo();
    let buf = cstr(bytes);
    let needle = c as c_char;
    let got_c = unsafe { c_foo(buf.as_ptr(), needle) };
    let got_rust = unsafe { rust_foo(buf.as_ptr(), needle) };
    assert_eq!(
        got_c,
        got_rust,
        "foo(input={:?}, c={:#04x}): C returned {got_c}, Rust returned {got_rust}",
        String::from_utf8_lossy(bytes),
        c
    );
    got_c
}

#[test]
fn foo_empty_and_trivial() {
    let libs = Libs::load();
    assert_eq!(check(&libs, b"", b'A'), 0);
    assert_eq!(check(&libs, b"", b'x'), 0);
    assert_eq!(check(&libs, b"A", b'A'), 1);
    assert_eq!(check(&libs, b"A", b'x'), 0);
    assert_eq!(check(&libs, b"x", b'x'), 1);
}

#[test]
fn foo_positions_and_runs() {
    let libs = Libs::load();
    let cases: &[&[u8]] = &[
        b"Axxx",
        b"xxxA",
        b"xAx",
        b"AAAA",
        b"xxxxxxxx",
        b"aAaAaA",
        b"A A A ",
        b" A",
        b"A ",
        b"AxAxAxAx",
        b"the quick brown fox",
        b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        b"\t\n\r\x0b\x0c",
        b"case matters: a vs A, X vs x",
    ];
    for case in cases {
        for &needle in b"AxaX \t0\xff\x80\x7f".iter() {
            let expected = case.iter().filter(|&&b| b == needle).count() as c_int;
            let got = check(&libs, case, needle);
            assert_eq!(
                got,
                expected,
                "foo({:?}, {:#04x}) = {got}, expected {expected}",
                String::from_utf8_lossy(case),
                needle
            );
        }
    }
}

#[test]
fn foo_all_single_byte_needles() {
    let libs = Libs::load();
    // One input containing every non-NUL byte exactly once, plus a second copy
    // of a few of them, then every possible needle value.
    let mut bytes: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    bytes.extend_from_slice(&[b'A', b'A', b'x', 0xff, 0x80]);
    for needle in 1u16..=255 {
        check(&libs, &bytes, needle as u8);
    }
}

#[test]
fn foo_high_bit_and_signedness() {
    let libs = Libs::load();
    // `char` is signed on x86_64 Linux; make sure needles that are negative as
    // `c_char` (0x80..=0xff) are matched the same way by both sides.
    let bytes: Vec<u8> = vec![0x80, 0xff, 0x41, 0xc1, 0x7f, 0x80, 0xff, 0xff];
    for needle in [0x80u8, 0xffu8, 0x7fu8, 0xc1u8, 0x41u8, 0x01u8] {
        let expected = bytes.iter().filter(|&&b| b == needle).count() as c_int;
        assert_eq!(check(&libs, &bytes, needle), expected);
    }
}

#[test]
fn foo_long_inputs() {
    let libs = Libs::load();
    for len in [1usize, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 4096] {
        // All-'A' of the given length: exercises alignment/vectorisation paths
        // in libc's strchr against the byte-at-a-time Rust port.
        let all_a = vec![b'A'; len];
        assert_eq!(check(&libs, &all_a, b'A'), len as c_int);
        assert_eq!(check(&libs, &all_a, b'x'), 0);

        // A single 'A' at each interesting offset.
        let mut offsets = vec![0usize, 1, len / 2, len - 1];
        offsets.retain(|&o| o < len);
        offsets.sort_unstable();
        offsets.dedup();
        for off in offsets {
            let mut buf = vec![b'.'; len];
            buf[off] = b'A';
            assert_eq!(check(&libs, &buf, b'A'), 1);
        }
    }
}

#[test]
fn foo_randomised() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5eed_1234_abcd_ef01);
    for _ in 0..2000 {
        let len = rng.below(200);
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                // Bias towards the alphabet the driver cares about so counts
                // are frequently non-zero.
                match rng.below(4) {
                    0 => b'A',
                    1 => b'x',
                    2 => b'a' + rng.below(26) as u8,
                    _ => rng.nonzero_byte(),
                }
            })
            .collect();
        let needle = match rng.below(3) {
            0 => b'A',
            1 => b'x',
            _ => rng.nonzero_byte(),
        };
        let expected = bytes.iter().filter(|&&b| b == needle).count() as c_int;
        assert_eq!(check(&libs, &bytes, needle), expected);
    }
}
