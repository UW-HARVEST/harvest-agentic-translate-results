//! Rust translation of `c_src/src/lib.c`.
//!
//! Provides an MPEG audio frame-header comparison routine. Behaviour (including
//! the original's quirks, such as never validating `h1`) is preserved exactly.

use std::ffi::c_int;

/// Translation of the C `static int hdr_valid(const uint8_t *h)`.
///
/// The C version short-circuits, so byte `h[1]` is only touched once `h[0]`
/// matches and `h[2]` only once the `h[1]` tests pass. That laziness is kept
/// here so this function reads no more memory than the original.
///
/// # Safety
/// `h` must point to enough readable bytes for the checks that actually run
/// (up to 3 bytes), exactly as required by the C code.
unsafe fn hdr_valid(h: *const u8) -> bool {
    if unsafe { *h.add(0) } != 0xff {
        return false;
    }

    let h1 = unsafe { *h.add(1) };
    if !((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2) {
        return false;
    }
    if ((h1 >> 1) & 3) == 0 {
        return false;
    }

    let h2 = unsafe { *h.add(2) };
    if (h2 >> 4) == 15 {
        return false;
    }
    ((h2 >> 2) & 3) != 3
}

/// Translation of the C `int hdr_compare(const uint8_t *h1, const uint8_t *h2)`.
///
/// Returns 1 when the two headers are considered compatible, 0 otherwise. Note
/// that, as in the C original, only `h2` is validated.
///
/// # Safety
/// Both pointers must reference at least 3 readable bytes when the
/// short-circuiting evaluation reaches them, matching the C contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    if !unsafe { hdr_valid(h2) } {
        return 0;
    }

    let a1 = unsafe { *h1.add(1) };
    let b1 = unsafe { *h2.add(1) };
    if ((a1 ^ b1) & 0xFE) != 0 {
        return 0;
    }

    let a2 = unsafe { *h1.add(2) };
    let b2 = unsafe { *h2.add(2) };
    if ((a2 ^ b2) & 0x0C) != 0 {
        return 0;
    }

    // C: !(((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0))
    let result = ((a2 & 0xF0) == 0) == ((b2 & 0xF0) == 0);
    result as c_int
}
