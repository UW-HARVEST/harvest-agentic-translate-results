//! Differential tests for `custom_strdup`, the sole public entry point of
//! `c_src/include/lib.h`.
//!
//! Every call is dispatched through `dlsym` on the C `.so` and the Rust `.so`
//! and the results are compared byte-for-byte.

mod common;

use std::ffi::c_char;
use std::ffi::c_void;

use common::Both;
use common::Rng;
use common::assert_same;
use common::free;
use common::snapshot;

/// `!str` short-circuit: both implementations must return `NULL` for `NULL`.
#[test]
fn null_input_returns_null() {
    let both = Both::load();

    // SAFETY: passing NULL is explicitly handled by the documented contract.
    let c_out = unsafe { both.c_strdup()(std::ptr::null()) };
    // SAFETY: as above.
    let rust_out = unsafe { both.rust_strdup()(std::ptr::null()) };

    assert!(c_out.is_null(), "C must return NULL for a NULL argument");
    assert!(
        rust_out.is_null(),
        "Rust must return NULL for a NULL argument"
    );
}

/// `strlen("") + 1 == 1`: a one-byte buffer containing just the terminator.
#[test]
fn empty_string() {
    let both = Both::load();
    assert_same(&both, b"", "empty");
}

#[test]
fn short_ascii_strings() {
    let both = Both::load();
    for input in [
        &b"a"[..],
        b"ab",
        b"abc",
        b"hello",
        b"hello, world",
        b"The quick brown fox jumps over the lazy dog",
        b" leading and trailing ",
        b"\t\n\r\x0b\x0c",
        b"//?*[]{}()<>|&;$`\"'\\",
    ] {
        assert_same(&both, input, &format!("ascii/{}", input.len()));
    }
}

/// Every possible non-NUL byte value, as a one-character payload.
#[test]
fn all_single_byte_values() {
    let both = Both::load();
    for byte in 1u8..=255 {
        assert_same(&both, &[byte], &format!("byte/{byte:#04x}"));
    }
}

/// High/non-UTF-8 byte sequences: the API is byte-oriented, not text-oriented,
/// so invalid UTF-8 must round-trip unchanged.
#[test]
fn non_utf8_payloads() {
    let both = Both::load();
    for input in [
        &b"\xff\xfe\xfd\xfc"[..],
        b"\x80\x80\x80",
        b"\xc3",             // truncated 2-byte sequence
        b"\xe2\x82",         // truncated 3-byte sequence
        b"\xf0\x9f\x92",     // truncated 4-byte sequence
        b"caf\xe9",          // latin-1 "café"
        b"\xed\xa0\x80",     // encoded surrogate half
    ] {
        assert_same(&both, input, &format!("non-utf8/{}", input.len()));
    }
}

/// Length sweep across every small size, including the word/SIMD boundaries
/// that `strlen` and `memcpy` implementations tend to special-case.
#[test]
fn length_sweep() {
    let both = Both::load();
    for len in 0..=512usize {
        let input: Vec<u8> = (0..len).map(|i| ((i % 255) + 1) as u8).collect();
        assert_same(&both, &input, &format!("sweep/{len}"));
    }
}

/// Sizes straddling powers of two and page boundaries.
#[test]
fn boundary_lengths() {
    let both = Both::load();
    let mut rng = Rng::new(0x5EED_1234);
    for base in [
        1usize, 2, 4, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511,
        512, 513, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537,
    ] {
        let input: Vec<u8> = (0..base).map(|_| rng.next_nonzero_byte()).collect();
        assert_same(&both, &input, &format!("boundary/{base}"));
    }
}

/// Randomised payloads with a fixed seed, so any failure is reproducible.
#[test]
fn randomised_payloads() {
    let both = Both::load();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    for case in 0..2000 {
        let len = rng.next_below(1024);
        let input: Vec<u8> = (0..len).map(|_| rng.next_nonzero_byte()).collect();
        assert_same(&both, &input, &format!("random/{case}/len{len}"));
    }
}

/// A multi-megabyte payload, well past any inlined copy fast path.
#[test]
fn large_payload() {
    let both = Both::load();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let input: Vec<u8> = (0..4 * 1024 * 1024).map(|_| rng.next_nonzero_byte()).collect();
    assert_same(&both, &input, "large/4MiB");
}

/// Bytes after the terminator must be ignored: `custom_strdup` copies exactly
/// `strlen + 1` bytes, so trailing garbage in the source must not appear in the
/// result and must not change its length.
#[test]
fn stops_at_first_nul() {
    let both = Both::load();

    // "visible" + NUL + trailing bytes that must never be copied.
    let mut buf = Vec::new();
    buf.extend_from_slice(b"visible");
    buf.push(0);
    buf.extend_from_slice(b"HIDDEN-MUST-NOT-BE-COPIED");
    buf.push(0);

    let arg = buf.as_ptr() as *const c_char;

    // SAFETY: `arg` is NUL-terminated at index 7 and outlives both calls.
    let c_out = unsafe { both.c_strdup()(arg) };
    // SAFETY: as above.
    let rust_out = unsafe { both.rust_strdup()(arg) };

    assert!(!c_out.is_null());
    assert!(!rust_out.is_null());

    // SAFETY: both buffers hold 8 bytes ("visible" + NUL).
    let c_bytes = unsafe { snapshot(c_out, 7) };
    // SAFETY: as above.
    let rust_bytes = unsafe { snapshot(rust_out, 7) };

    assert_eq!(c_bytes, rust_bytes, "outputs differ after an interior NUL");
    assert_eq!(c_bytes, b"visible\0", "copy did not stop at the first NUL");

    // SAFETY: both pointers came from `malloc`.
    unsafe {
        free(c_out as *mut c_void);
        free(rust_out as *mut c_void);
    }
}

/// Repeated calls must be independent: each result is a fresh allocation whose
/// contents are unaffected by later calls.
#[test]
fn results_are_independent_allocations() {
    let both = Both::load();

    let a = b"first-string\0";
    let b = b"second-string-that-is-longer\0";

    // SAFETY: all four arguments are NUL-terminated literals.
    let (c1, r1, c2, r2) = unsafe {
        (
            both.c_strdup()(a.as_ptr() as *const c_char),
            both.rust_strdup()(a.as_ptr() as *const c_char),
            both.c_strdup()(b.as_ptr() as *const c_char),
            both.rust_strdup()(b.as_ptr() as *const c_char),
        )
    };

    for p in [c1, r1, c2, r2] {
        assert!(!p.is_null());
    }

    // SAFETY: lengths are known from the literals above.
    unsafe {
        assert_eq!(snapshot(c1, a.len() - 1), snapshot(r1, a.len() - 1));
        assert_eq!(snapshot(c2, b.len() - 1), snapshot(r2, b.len() - 1));
        assert_eq!(snapshot(c1, a.len() - 1), a.to_vec());
        assert_eq!(snapshot(c2, b.len() - 1), b.to_vec());
    }

    // SAFETY: all pointers came from `malloc`.
    unsafe {
        free(c1 as *mut c_void);
        free(r1 as *mut c_void);
        free(c2 as *mut c_void);
        free(r2 as *mut c_void);
    }
}
