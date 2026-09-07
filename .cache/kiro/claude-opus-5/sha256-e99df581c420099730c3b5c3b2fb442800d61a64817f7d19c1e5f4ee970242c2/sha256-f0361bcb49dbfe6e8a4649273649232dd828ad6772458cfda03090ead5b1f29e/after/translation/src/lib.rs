//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   - `hdr_compare`
//!
//! The translation reproduces the C semantics exactly, including short-circuit
//! evaluation order of `&&` (so `h1` is never dereferenced unless `h2` passes
//! the validity check) and the C integer-promotion behaviour of the bit tests.

#![allow(non_snake_case)]

use core::ffi::c_int;

/// Translation of the file-local `static int hdr_valid(const uint8_t *h)`.
///
/// C source:
/// ```c
/// static int hdr_valid(const uint8_t *h) {
///     return h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) &&
///            ((((h[1]) >> 1) & 3) != 0) && (((h[2]) >> 4) != 15) &&
///            ((((h[2]) >> 2) & 3) != 3);
/// }
/// ```
///
/// `uint8_t` operands are promoted to `int` in C; because every operand here is
/// a non-negative byte and the masks/shifts are small, performing the arithmetic
/// on `u8` widened to `i32` is bit-for-bit equivalent.
///
/// # Safety
/// `h` must point to at least 3 readable bytes, exactly as the C code requires.
#[inline]
unsafe fn hdr_valid(h: *const u8) -> bool {
    // Byte 0 is read first, matching the C evaluation order.
    let h0 = i32::from(unsafe { *h });
    if h0 != 0xff {
        return false;
    }

    let h1 = i32::from(unsafe { *h.add(1) });
    if !((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2) {
        return false;
    }
    if ((h1 >> 1) & 3) == 0 {
        return false;
    }

    let h2 = i32::from(unsafe { *h.add(2) });
    if (h2 >> 4) == 15 {
        return false;
    }
    if ((h2 >> 2) & 3) == 3 {
        return false;
    }

    true
}

/// Translation of `int hdr_compare(const uint8_t *h1, const uint8_t *h2)`.
///
/// C source:
/// ```c
/// int hdr_compare(const uint8_t *h1, const uint8_t *h2) {
///     return hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 &&
///            ((h1[2] ^ h2[2]) & 0x0C) == 0 &&
///            !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0));
/// }
/// ```
///
/// The final term applies bitwise `^` to two `int` values that are each `0` or
/// `1`, then logically negates the result: that is exactly an equality test
/// between the two "high nibble is zero" predicates.
///
/// Returns the C `int` values `1` (true) or `0` (false).
///
/// # Safety
/// `h2` must point to at least 3 readable bytes. `h1` must point to at least 3
/// readable bytes whenever `hdr_valid(h2)` holds — the C code only dereferences
/// `h1` in that case, and this translation preserves that.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    // `&&` short-circuits in C: if h2 is not a valid header, h1 is never read.
    if !unsafe { hdr_valid(h2) } {
        return 0;
    }

    let a1 = i32::from(unsafe { *h1.add(1) });
    let b1 = i32::from(unsafe { *h2.add(1) });
    if ((a1 ^ b1) & 0xFE) != 0 {
        return 0;
    }

    let a2 = i32::from(unsafe { *h1.add(2) });
    let b2 = i32::from(unsafe { *h2.add(2) });
    if ((a2 ^ b2) & 0x0C) != 0 {
        return 0;
    }

    // !( ((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0) )
    if ((a2 & 0xF0) == 0) != ((b2 & 0xF0) == 0) {
        return 0;
    }

    1
}
