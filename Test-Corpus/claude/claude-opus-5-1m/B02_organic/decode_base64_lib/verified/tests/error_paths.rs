//! Phase C — error-path differential tests (one per row of `ERRORS.md`, plus
//! the generic FFI boundary cases).

mod common;

use common::{diff, diff_buf, diff_null, Rng, ALPHABET};
use std::ffi::c_char;

/// ERRORS.md row 1 — `src == NULL` must return NULL from both.
#[test]
fn e1_null_pointer() {
    diff_null(std::ptr::null(), "NULL src");
    // repeated, and interleaved with valid calls, to be sure the NULL path
    // leaves no state behind
    for _ in 0..100 {
        diff_null(std::ptr::null(), "NULL src (repeat)");
        diff(b"QUJD", "valid after NULL");
    }
}

/// ERRORS.md row 2 — empty string (`*src == '\0'`) must return NULL from both.
#[test]
fn e2_empty_string() {
    diff_buf(&[0u8], "empty string");
    diff_null(b"\0".as_ptr() as *const c_char, "empty string returns NULL");
    // a 1-byte heap buffer holding only NUL (no readable byte past it)
    let one = vec![0u8; 1];
    diff_null(one.as_ptr() as *const c_char, "1-byte NUL-only buffer");
    // a longer buffer whose first byte is NUL: everything after must be ignored
    let mut b = vec![0u8];
    b.extend_from_slice(b"QUJDRA");
    b.push(0);
    diff_buf(&b, "leading NUL then payload");
    diff_null(b.as_ptr() as *const c_char, "leading NUL returns NULL");
}

/// ERRORS.md generic boundary row — interior NUL: the length "lies", both must stop at
/// the first NUL and agree with the truncated string.
#[test]
fn g1_interior_nul_truncates() {
    let mut rng = Rng::new(105);
    for _ in 0..2_000 {
        let head = rng.range(1, 20);
        let mut buf: Vec<u8> = (0..head).map(|_| rng.nonzero_byte()).collect();
        let truncated = buf.clone();
        buf.push(0);
        let tail = rng.range(1, 20);
        buf.extend((0..tail).map(|_| rng.nonzero_byte()));
        buf.push(0);
        diff_buf(&buf, "interior NUL");
        diff(&truncated, "interior NUL (truncated reference)");
    }
}

/// ERRORS.md generic boundary row — every single byte value `0x01..=0xFF`: this is the
/// full "one step past every range boundary" surface for a `char` parameter
/// (`'A'-1`, `'Z'+1`, `'a'-1`, `'z'+1`, `'0'-1`, `'9'+1`, `'+'`, `'/'`, `'='`,
/// DEL, and all negative `char` values).
#[test]
fn g2_all_single_bytes() {
    for b in 1u16..=255 {
        diff(&[b as u8], &format!("byte 0x{b:02x}"));
        diff(&[b as u8, b as u8], &format!("byte 0x{b:02x} x2"));
        diff(&[b as u8, b as u8, b as u8], &format!("byte 0x{b:02x} x3"));
        diff(&[b as u8, b as u8, b as u8, b as u8], &format!("byte 0x{b:02x} x4"));
        diff(
            &[b as u8, b as u8, b as u8, b as u8, b as u8],
            &format!("byte 0x{b:02x} x5"),
        );
    }
}

/// ERRORS.md generic boundary row — the bytes exactly one step outside each accepted
/// range, in every group position, and mixed with valid data.
#[test]
fn g3_range_boundaries_in_all_positions() {
    // one step below / above every range `decode` and `is_base64` test for
    let boundaries: &[u8] = &[
        b'A' - 1, // '@'
        b'A',
        b'Z',
        b'Z' + 1, // '['
        b'a' - 1, // '`'
        b'a',
        b'z',
        b'z' + 1, // '{'
        b'0' - 1, // '/'  (still valid!)
        b'0',
        b'9',
        b'9' + 1, // ':'
        b'+' - 1, // '*'
        b'+',
        b'+' + 1, // ','
        b'/' - 1, // '.'
        b'/',
        b'/' + 1, // '0'
        b'=' - 1, // '<'
        b'=',
        b'=' + 1, // '>'
        0x7f,     // DEL
        0x80,     // first negative char
        0xff,     // last negative char
    ];
    let mut rng = Rng::new(107);
    for &b in boundaries {
        for slot in 0..4usize {
            for _ in 0..50 {
                let groups = rng.range(1, 3);
                let mut v: Vec<u8> = (0..groups * 4).map(|_| rng.pick(ALPHABET)).collect();
                let g = rng.below(groups);
                v[g * 4 + slot] = b;
                diff(&v, &format!("boundary 0x{b:02x} at slot {slot}"));
            }
            // and as the only content
            diff(&[b], &format!("boundary 0x{b:02x} alone"));
        }
        // every pair of boundary bytes
        for &c in boundaries {
            diff(&[b, c], "boundary pair");
            diff(&[b, c, b, c], "boundary quad");
        }
    }
}

/// ERRORS.md generic boundary row — "oversized" but still representable lengths, and
/// inputs whose *filtered* length is 0 while the raw length is large (the
/// destination buffer is then fully unwritten and must be all zeroes in both).
#[test]
fn g4_large_but_empty_after_filtering() {
    for &len in &[1usize, 2, 3, 4, 1023, 1024, 4096, 65_537] {
        diff(&vec![b'.'; len], &format!("{len} invalid bytes"));
        diff(&vec![0xffu8; len], &format!("{len} 0xff bytes"));
    }
}
