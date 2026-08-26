//! Rust port of the C library `lib.c`. The functions reproduce the
//! original behavior byte-for-byte.

/// Equivalent of the C `hdr_valid` static helper. Returns `true` when the
/// 3-byte header in `h` looks valid according to the original validation
/// rules.
fn hdr_valid(h: &[u8]) -> bool {
    h[0] == 0xff
        && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
        && (((h[1] >> 1) & 3) != 0)
        && ((h[2] >> 4) != 15)
        && (((h[2] >> 2) & 3) != 3)
}

/// Equivalent of the C `hdr_compare`. Returns a non-zero value (1) when the
/// two headers `h1` and `h2` are considered compatible, otherwise 0.
///
/// The function expects each input slice to contain at least three bytes.
#[no_mangle]
pub extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> i32 {
    // Match the C semantics, which read three bytes from each pointer.
    let h1_slice = unsafe { core::slice::from_raw_parts(h1, 3) };
    let h2_slice = unsafe { core::slice::from_raw_parts(h2, 3) };
    hdr_compare_safe(h1_slice, h2_slice) as i32
}

/// Safe Rust variant of `hdr_compare` operating on slices.
pub fn hdr_compare_safe(h1: &[u8], h2: &[u8]) -> bool {
    hdr_valid(h2)
        && ((h1[1] ^ h2[1]) & 0xFE) == 0
        && ((h1[2] ^ h2[2]) & 0x0C) == 0
        && !(((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0))
}
