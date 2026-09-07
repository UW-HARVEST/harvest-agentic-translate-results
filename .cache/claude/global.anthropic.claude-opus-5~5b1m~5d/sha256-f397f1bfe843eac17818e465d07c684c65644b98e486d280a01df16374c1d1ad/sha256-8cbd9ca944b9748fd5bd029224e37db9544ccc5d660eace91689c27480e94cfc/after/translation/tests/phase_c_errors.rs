//! Phase C — error-path differential tests, one test per row of ERRORS.md.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so`, and asserts the SAME sentinel is returned (this
//! API's only error signal is a `NULL` return).

mod common;

use common::*;
use std::ffi::c_char;

/// ERRORS.md row 1 — `src == NULL` must yield NULL from both.
#[test]
fn err_row01_null_pointer() {
    unsafe {
        let c_null = call_c_ptr(std::ptr::null());
        let r_null = call_rust_ptr(std::ptr::null());
        assert!(c_null, "C did not return NULL for a NULL src");
        assert_eq!(
            c_null, r_null,
            "NULL-pointer divergence: C returned NULL={c_null}, Rust returned NULL={r_null}"
        );
    }
}

/// ERRORS.md row 2 — empty string (`*src == '\0'`) must yield NULL from both.
#[test]
fn err_row02_empty_string() {
    let empty: [c_char; 1] = [0];
    unsafe {
        let c_null = call_c_ptr(empty.as_ptr());
        let r_null = call_rust_ptr(empty.as_ptr());
        assert!(c_null, "C did not return NULL for an empty string");
        assert_eq!(
            c_null, r_null,
            "empty-string divergence: C NULL={c_null}, Rust NULL={r_null}"
        );
    }
    // and via the byte-slice harness (identical outcome, both NULL)
    assert_same("err_row02", b"");
    assert_eq!(c_call(b""), Outcome::Null);
    assert_eq!(rust_call(b""), Outcome::Null);
}

/// ERRORS.md rows 3 & 4 — allocation failure paths.
///
/// `calloc`/`malloc` failure cannot be induced from a normal differential test
/// on a 64-bit host, so this test pins the observable contract both
/// implementations share: for every input either BOTH return NULL or BOTH
/// return a buffer — the NULL/non-NULL verdict never diverges. The Rust code
/// performs the same `dest`-then-`buf` allocation order and the same
/// `free(dest)` cleanup on the second failure (see src/lib.rs lines 80-90).
#[test]
fn err_rows03_04_alloc_failure_paths_documented() {
    let mut rng = Rng::new(0xA110C);
    for _ in 0..4000 {
        let n = rng.below(200); // includes 0 => empty string => both NULL
        let s: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        let c = c_call(&s);
        let r = rust_call(&s);
        let c_is_null = c == Outcome::Null;
        let r_is_null = r == Outcome::Null;
        assert_eq!(
            c_is_null, r_is_null,
            "NULL-verdict divergence for input of len {n}"
        );
        assert_eq!(c, r, "buffer divergence for input of len {n}");
    }
    // Also confirm the very large allocation still succeeds identically
    // (l + 13 is well within reach, so both must return non-NULL).
    let big = vec![b'A'; 1 << 20];
    let c = c_call(&big);
    let r = rust_call(&big);
    assert_ne!(c, Outcome::Null);
    assert_eq!(c, r);
}

/// ERRORS.md row 5 — `decode()` fall-through sentinel 63 for every byte that is
/// not `A-Z a-z 0-9 '+'`. Only bytes that also survive `is_base64()` (`'/'` and
/// `'='`) can reach `decode()`, so those two are the observable cases; they must
/// both map to 63 in Rust as well.
#[test]
fn err_row05_decode_fallthrough_sentinel() {
    // '/' -> 63 and '=' -> 63 (aliasing is intentional in the C).
    // "////" decodes to 0xff 0xff 0xff; "====" suppresses bytes 2 and 3.
    for lit in [
        &b"////"[..],
        b"===",
        b"====",
        b"/===",
        b"//==",
        b"///=",
        b"=///",
        b"/",
        b"//",
        b"///",
        b"/A/A",
        b"=A=A",
        b"A/=/",
    ] {
        assert_same("err_row05", lit);
    }
    // '/' and '=' must be interchangeable in the c1/c2 slots (both decode to 63)
    // — assert that by comparing the C output of the two forms to each other,
    // then assert Rust matches C for both.
    let a = c_call(b"//AA");
    let b = c_call(b"==AA");
    assert_eq!(a, b, "C: '/' and '=' should both decode to 63 in c1/c2");
    assert_same("err_row05:slash", b"//AA");
    assert_same("err_row05:eq", b"==AA");

    // full sweep: every byte in a 4-group, in every slot, against 'A' filler
    for byte in 0u16..=255 {
        let byte = byte as u8;
        if byte == 0 {
            continue;
        }
        for slot in 0..4 {
            let mut s = [b'A'; 4];
            s[slot] = byte;
            assert_same("err_row05:sweep", &s);
        }
    }
}

/// ERRORS.md row 6 — `is_base64()` FALSE means the byte is silently dropped.
/// Verified by sweeping every rejected byte, including 0x80..0xFF which are
/// NEGATIVE `char` values on x86-64 (the classic sign-extension trap).
#[test]
fn err_row06_is_base64_rejection_drops_byte() {
    let accepted: Vec<u8> = B64_ALPHABET.iter().copied().chain([b'=']).collect();
    let base = b"QUJD"; // "ABC"
    for byte in 1u16..=255 {
        let byte = byte as u8;
        // insert the byte at every position of a known-good input
        for pos in 0..=base.len() {
            let mut s = base[..pos].to_vec();
            s.push(byte);
            s.extend_from_slice(&base[pos..]);
            assert_same("err_row06:insert", &s);
        }
        // a rejected byte must not change the output at all; assert that
        // against the C itself, then assert Rust matches C.
        if !accepted.contains(&byte) {
            let with = c_call(&{
                let mut v = base.to_vec();
                v.insert(2, byte);
                v
            });
            let without = c_call(base);
            // payload region (first 3 bytes) must be identical; total alloc
            // length differs by 1 because it is derived from strlen(src).
            if let (Outcome::Buf(a), Outcome::Buf(b)) = (&with, &without) {
                assert_eq!(
                    a[..3],
                    b[..3],
                    "C did not drop rejected byte 0x{byte:02x}"
                );
            } else {
                panic!("unexpected NULL");
            }
        }
    }
    // high-bit bytes as the ONLY content: l == 0, all-zero buffer, non-NULL
    for byte in 0x80u8..=0xff {
        assert_same("err_row06:high_only", &[byte]);
        assert_same("err_row06:high_only2", &[byte, byte, byte]);
    }
}

/// Generic FFI boundaries: zero length, length 1, one-past-range values, and
/// out-of-range "enum" values. This API has no enum/int parameters, so the
/// out-of-range dimension is swept over every possible byte value of the one
/// `const char *` parameter, including the bytes immediately outside each
/// accepted range (`@ [ ` { / : ' ' * . 0x00-adjacent 0x7f 0x80 0xff`).
#[test]
fn err_generic_boundaries() {
    // one step past every accepted range in decode()/is_base64()
    let one_past: &[u8] = b"@[`{/:*.,-+ \t\n\x0b\x0c\r\x1f\x20\x7e\x7f\x80\x81\xfe\xff<>?";
    for &b in one_past {
        assert_same("bnd:solo", &[b]);
        assert_same("bnd:pair", &[b, b]);
        assert_same("bnd:group", &[b, b, b, b]);
        assert_same("bnd:in_group", &[b'A', b, b'B', b]);
        assert_same("bnd:prefix", &[b, b'A', b'A', b'A', b'A']);
        assert_same("bnd:suffix", &[b'A', b'A', b'A', b'A', b]);
    }
    // zero length
    assert_eq!(c_call(b""), Outcome::Null);
    assert_eq!(rust_call(b""), Outcome::Null);
    // length 1 for every byte
    for b in 1u8..=255 {
        assert_same("bnd:len1", &[b]);
    }
    // oversized-ish length
    let mut rng = Rng::new(0xB0057);
    for n in [1usize, 2, 3, 4, 255, 256, 257, 4095, 4096, 4097, 100_000] {
        let s: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        assert_same("bnd:size", &s);
    }
}

/// Repeated calls must not leave residual state (both are stateless).
#[test]
fn err_repeated_calls_stateless() {
    for _ in 0..200 {
        assert_same("stateless", b"SGVsbG8sIFdvcmxkIQ==");
        assert_same("stateless", b"");
        assert_same("stateless", b"!!!!");
        assert_same("stateless", b"A");
    }
}
