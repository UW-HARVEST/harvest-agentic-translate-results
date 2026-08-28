//! Differential tests for `encode_base64`, working bottom-up:
//!   1. the `encode()` alphabet helper (reachable only through `encode_base64`)
//!   2. single group / padding behaviour
//!   3. multi-group encoding
//!   4. the `size == 0` strlen path
//!   5. degenerate and negative `size` arguments

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Level 1: the static `encode()` helper.
//
// `encode` maps 0..=63 onto the base64 alphabet with five branches
// (A-Z, a-z, 0-9, '+', '/'). Every 6-bit value appears as `b4` for some single
// input byte, and every combination of `b5`/`b6` is covered by the exhaustive
// two-byte sweep below, so these tests cover all branches.
// ---------------------------------------------------------------------------

#[test]
fn all_single_bytes() {
    for b in 0u16..=255 {
        check_bytes(&[b as u8]);
    }
}

#[test]
fn all_byte_pairs() {
    // 65_536 cases: exhaustively covers b4/b5/b6 for the 2-byte (one '=' pad)
    // shape, i.e. every value the alphabet lookup can receive.
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            check_bytes(&[a as u8, b as u8]);
        }
    }
}

#[test]
fn all_third_bytes_full_group() {
    // With a full 3-byte group, b7 = b3 & 0x3f is used. Sweep the third byte
    // across all 256 values against a few first/second byte combinations.
    for &(a, b) in &[
        (0x00u8, 0x00u8),
        (0xff, 0xff),
        (0x01, 0x80),
        (0x7f, 0x80),
        (0xaa, 0x55),
        (0x3f, 0xc0),
    ] {
        for c in 0u16..=255 {
            check_bytes(&[a, b, c as u8]);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: padding shapes.
// ---------------------------------------------------------------------------

#[test]
fn padding_shapes() {
    // n % 3 == 1 -> "xx==", n % 3 == 2 -> "xxx=", n % 3 == 0 -> "xxxx"
    let data: Vec<u8> = (0u8..=250).collect();
    for len in 0..=64usize {
        check_bytes(&data[..len]);
    }
}

#[test]
fn known_vectors() {
    // Sanity-check against well-known base64 outputs, comparing C vs Rust as
    // well as the expected literal (the C result is the ground truth).
    let cases: &[(&str, &str)] = &[
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
        ("Hello, World!", "SGVsbG8sIFdvcmxkIQ=="),
    ];
    let l = libs();
    for (input, expected) in cases {
        check_bytes(input.as_bytes());

        // Also confirm the C side really produces `expected`, so the
        // differential comparison is anchored to real base64.
        let mut owned = input.as_bytes().to_vec();
        owned.push(0);
        let p = unsafe { (l.c_encode)(input.len() as c_int, owned.as_ptr() as *const c_char) };
        assert!(!p.is_null());
        let got = unsafe { std::ffi::CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(&got, expected, "C output for {input:?}");
    }
}

// ---------------------------------------------------------------------------
// Level 3: longer / structured inputs.
// ---------------------------------------------------------------------------

#[test]
fn long_inputs() {
    for &len in &[100usize, 255, 256, 257, 1000, 1023, 1024, 1025, 4096, 65535] {
        let data: Vec<u8> = (0..len).map(|i| (i * 31 + 7) as u8).collect();
        check_bytes(&data);
    }
}

#[test]
fn high_bit_bytes() {
    // `const char *` is signed on x86; the C code assigns through
    // `unsigned char`. Make sure sign handling matches.
    for &fill in &[0x80u8, 0xff, 0xfe, 0x81, 0xc0] {
        for len in 1..=12usize {
            check_bytes(&vec![fill; len]);
        }
    }
}

#[test]
fn random_fuzz() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    for _ in 0..3000 {
        let len = rng.below(300);
        let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        check_bytes(&data);
    }
}

// ---------------------------------------------------------------------------
// Level 4: the `size == 0` => strlen(src) path.
// ---------------------------------------------------------------------------

#[test]
fn strlen_path() {
    for s in [
        "",
        "a",
        "ab",
        "abc",
        "abcd",
        "hello world",
        "The quick brown fox jumps over the lazy dog",
    ] {
        check_bytes_size(s.as_bytes(), 0);
    }
}

#[test]
fn strlen_path_stops_at_nul() {
    // Buffer contains an embedded NUL: size == 0 must only encode up to it.
    let buf: &[u8] = b"abc\0defghij\0";
    check_raw(0, buf.as_ptr() as *const c_char, "embedded NUL, size=0");
}

#[test]
fn strlen_path_random() {
    let mut rng = Rng::new(0x5EED_5EED);
    for _ in 0..2000 {
        let len = rng.below(200);
        // Non-zero bytes only, so strlen() == len.
        let data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
        check_bytes_size(&data, 0);
    }
}

// ---------------------------------------------------------------------------
// Level 5: degenerate arguments.
// ---------------------------------------------------------------------------

#[test]
fn null_src() {
    for size in [0i32, 1, 5, -1, i32::MAX, i32::MIN] {
        check_raw(size, std::ptr::null(), "NULL src");
    }
}

#[test]
fn negative_size() {
    // C: the loop `for (i = 0; i < size; ...)` never runs, but calloc is still
    // called with the (wrapped, sign-extended) byte count.
    let data = b"whatever data here\0";
    for size in [-1i32, -2, -3, -4, -100, -1000] {
        check_raw(
            size,
            data.as_ptr() as *const c_char,
            "negative size (small)",
        );
    }
}

#[test]
fn negative_size_alloc_failure() {
    // For sufficiently negative sizes, `size * 4 / 3 + 4` stays negative and
    // sign-extends to a huge size_t, so calloc fails and both return NULL.
    let data = b"whatever\0";
    for size in [-4i32, -8, -1_000_000, i32::MIN / 2, i32::MIN + 4] {
        check_raw(size, data.as_ptr() as *const c_char, "negative size (large)");
    }
}

#[test]
fn size_smaller_than_input() {
    // Passing a size shorter than the buffer must only encode that prefix.
    let data = b"abcdefghijklmnopqrstuvwxyz\0";
    for size in 1..=26i32 {
        check_raw(
            size,
            data.as_ptr() as *const c_char,
            "size shorter than buffer",
        );
    }
}

#[test]
fn repeated_calls_are_stable() {
    // Guards against any accidental shared/static state in either build.
    for _ in 0..200 {
        check_bytes(b"stability check");
        check_bytes_size(b"stability check", 0);
    }
}
