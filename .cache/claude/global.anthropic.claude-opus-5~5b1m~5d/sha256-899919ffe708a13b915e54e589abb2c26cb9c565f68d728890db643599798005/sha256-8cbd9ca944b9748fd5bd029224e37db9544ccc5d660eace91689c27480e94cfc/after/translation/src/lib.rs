//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared object, verified with `nm -D`):
//!   * `hdr_compare`
//!
//! `hdr_valid` is `static` in the C translation unit and therefore *not* part of
//! the exported ABI; it is reproduced here as a private helper with identical
//! semantics.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;

/// Translation of:
///
/// ```c
/// static int hdr_valid(const uint8_t *h) {
///     return h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) &&
///            ((((h[1]) >> 1) & 3) != 0) && (((h[2]) >> 4) != 15) &&
///            ((((h[2]) >> 2) & 3) != 3);
/// }
/// ```
///
/// The C operands are `uint8_t` values promoted to `int` before the bitwise
/// operations; every intermediate value stays inside `u8` range here, so the
/// promotion is not observable. All comparisons/short-circuit ordering are kept
/// exactly as written in the C source.
#[inline]
unsafe fn hdr_valid(h: *const u8) -> bool {
    // h[0] == 0xff
    if *h.add(0) != 0xff {
        return false;
    }

    let h1 = *h.add(1);

    // ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
    if !((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2) {
        return false;
    }

    // ((((h[1]) >> 1) & 3) != 0)
    if ((h1 >> 1) & 3) == 0 {
        return false;
    }

    let h2 = *h.add(2);

    // (((h[2]) >> 4) != 15)
    if (h2 >> 4) == 15 {
        return false;
    }

    // ((((h[2]) >> 2) & 3) != 3)
    if ((h2 >> 2) & 3) == 3 {
        return false;
    }

    true
}

/// Translation of:
///
/// ```c
/// int hdr_compare(const uint8_t *h1, const uint8_t *h2) {
///     return hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 &&
///            ((h1[2] ^ h2[2]) & 0x0C) == 0 &&
///            !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0));
/// }
/// ```
///
/// The C `&&` chain short-circuits, so `h1` is never dereferenced unless
/// `hdr_valid(h2)` is true; that behaviour is preserved verbatim. The result is
/// the value of a C logical expression, i.e. exactly `0` or `1`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    if !hdr_valid(h2) {
        return 0;
    }

    // ((h1[1] ^ h2[1]) & 0xFE) == 0
    if ((*h1.add(1) ^ *h2.add(1)) & 0xFE) != 0 {
        return 0;
    }

    let a2 = *h1.add(2);
    let b2 = *h2.add(2);

    // ((h1[2] ^ h2[2]) & 0x0C) == 0
    if ((a2 ^ b2) & 0x0C) != 0 {
        return 0;
    }

    // !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0))
    // The C operands of `^` are the ints 0/1 produced by `==`, so the bitwise
    // xor is equivalent to a boolean inequality test.
    if ((a2 & 0xF0) == 0) != ((b2 & 0xF0) == 0) {
        return 0;
    }

    1
}
