//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared library):
//!   * `hdr_compare`
//!
//! `hdr_valid` is `static` in the C source, so it is *not* an exported symbol
//! and is kept private here as well.

use std::ffi::c_int;

/// Translation of the C `static int hdr_valid(const uint8_t *h)`.
///
/// ```c
/// static int hdr_valid(const uint8_t *h) {
///     return h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) &&
///            ((((h[1]) >> 1) & 3) != 0) && (((h[2]) >> 4) != 15) &&
///            ((((h[2]) >> 2) & 3) != 3);
/// }
/// ```
///
/// The C `&&` / `||` operators short-circuit, so bytes are only read as far as
/// needed; the reads below are ordered and guarded identically.
///
/// # Safety
///
/// `h` must be valid for reads of the bytes the C code would have read
/// (up to 3 bytes), exactly as required by the original C function.
unsafe fn hdr_valid(h: *const u8) -> bool {
    // h[0] == 0xff
    if unsafe { *h } != 0xff {
        return false;
    }

    // ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
    let h1 = unsafe { *h.add(1) };
    if !((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2) {
        return false;
    }

    // ((((h[1]) >> 1) & 3) != 0)
    if ((h1 >> 1) & 3) == 0 {
        return false;
    }

    // (((h[2]) >> 4) != 15)
    let h2 = unsafe { *h.add(2) };
    if (h2 >> 4) == 15 {
        return false;
    }

    // ((((h[2]) >> 2) & 3) != 3)
    ((h2 >> 2) & 3) != 3
}

/// Translation of the C `int hdr_compare(const uint8_t *h1, const uint8_t *h2)`.
///
/// ```c
/// int hdr_compare(const uint8_t *h1, const uint8_t *h2) {
///     return hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 &&
///            ((h1[2] ^ h2[2]) & 0x0C) == 0 &&
///            !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0));
/// }
/// ```
///
/// Returns `1` or `0`, matching the C `int` result of the `&&` chain. Because
/// C short-circuits, `h1` is not dereferenced at all when `hdr_valid(h2)` is
/// false; that behaviour is reproduced here.
///
/// # Safety
///
/// `h1` and `h2` must be valid for the reads the C code performs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    // hdr_valid(h2)
    if !unsafe { hdr_valid(h2) } {
        return 0;
    }

    // ((h1[1] ^ h2[1]) & 0xFE) == 0
    let a1 = unsafe { *h1.add(1) };
    let b1 = unsafe { *h2.add(1) };
    if ((a1 ^ b1) & 0xFE) != 0 {
        return 0;
    }

    // ((h1[2] ^ h2[2]) & 0x0C) == 0
    let a2 = unsafe { *h1.add(2) };
    let b2 = unsafe { *h2.add(2) };
    if ((a2 ^ b2) & 0x0C) != 0 {
        return 0;
    }

    // !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0))
    let neither_or_both = ((a2 & 0xF0) == 0) ^ ((b2 & 0xF0) == 0);
    if neither_or_both { 0 } else { 1 }
}
