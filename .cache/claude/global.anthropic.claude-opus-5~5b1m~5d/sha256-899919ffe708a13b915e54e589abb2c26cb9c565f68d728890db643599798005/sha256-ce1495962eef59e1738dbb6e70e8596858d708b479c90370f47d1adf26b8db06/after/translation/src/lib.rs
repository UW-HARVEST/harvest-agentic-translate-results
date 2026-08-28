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

/// Loads one byte, exactly like the C's `h[i]` subscript.
///
/// This deliberately goes through [`core::ptr::read`] instead of writing `*p` directly.
/// A plain raw-pointer dereference makes `rustc` emit a null-pointer *precondition* check
/// whenever `-C debug-assertions=on` (the default for the `dev`/`test` profiles); that check
/// panics with "null pointer dereference occurred", and a panic escaping an `extern "C"`
/// function aborts the process with `SIGABRT`. The C has no such check: `h[i]` on an invalid
/// pointer simply performs the load and the hardware raises `SIGSEGV`.
///
/// Going through `ptr::read` — which lowers to the same single `movb`, and whose safety
/// contract for a 1-byte type carries no alignment requirement — keeps the Rust's observable
/// behaviour on invalid pointers identical to the C's in *every* build profile.
#[inline(always)]
unsafe fn byte(p: *const u8, i: usize) -> u8 {
    core::ptr::read(p.add(i))
}

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
    if byte(h, 0) != 0xff {
        return false;
    }

    let h1 = byte(h, 1);

    // ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
    if !((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2) {
        return false;
    }

    // ((((h[1]) >> 1) & 3) != 0)
    if ((h1 >> 1) & 3) == 0 {
        return false;
    }

    let h2 = byte(h, 2);

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
    if ((byte(h1, 1) ^ byte(h2, 1)) & 0xFE) != 0 {
        return 0;
    }

    let a2 = byte(h1, 2);
    let b2 = byte(h2, 2);

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
