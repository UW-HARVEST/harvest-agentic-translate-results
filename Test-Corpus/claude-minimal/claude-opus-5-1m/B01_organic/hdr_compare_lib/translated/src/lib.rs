//! Rust translation of c_src/src/lib.c

/// Safety: `h` must point to a valid array of at least 3 bytes.
unsafe fn hdr_valid(h: *const u8) -> bool {
    let h0 = *h.add(0);
    let h1 = *h.add(1);
    let h2 = *h.add(2);
    h0 == 0xff
        && ((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2)
        && (((h1 >> 1) & 3) != 0)
        && ((h2 >> 4) != 15)
        && (((h2 >> 2) & 3) != 3)
}

/// Compare two MPEG audio frame headers.
///
/// Returns a non-zero `c_int` value if the headers are compatible, otherwise 0,
/// matching the C semantics of `int` returns.
///
/// # Safety
/// Both `h1` and `h2` must point to valid arrays of at least 3 bytes.
#[no_mangle]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> core::ffi::c_int {
    let valid = hdr_valid(h2);
    let h1_1 = *h1.add(1);
    let h2_1 = *h2.add(1);
    let h1_2 = *h1.add(2);
    let h2_2 = *h2.add(2);

    let result = valid
        && ((h1_1 ^ h2_1) & 0xFE) == 0
        && ((h1_2 ^ h2_2) & 0x0C) == 0
        && !(((h1_2 & 0xF0) == 0) ^ ((h2_2 & 0xF0) == 0));

    result as core::ffi::c_int
}
