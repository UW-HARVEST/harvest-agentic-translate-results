use std::ffi::c_int;

/// Safety: `h` must point to at least 3 valid bytes.
unsafe fn hdr_valid(h: *const u8) -> c_int {
    let b0 = unsafe { *h.add(0) };
    let b1 = unsafe { *h.add(1) };
    let b2 = unsafe { *h.add(2) };
    let cond = b0 == 0xff
        && ((b1 & 0xF0) == 0xf0 || (b1 & 0xFE) == 0xe2)
        && (((b1 >> 1) & 3) != 0)
        && ((b2 >> 4) != 15)
        && (((b2 >> 2) & 3) != 3);
    cond as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    let valid = unsafe { hdr_valid(h2) };
    if valid == 0 {
        return 0;
    }
    let h1_1 = unsafe { *h1.add(1) };
    let h1_2 = unsafe { *h1.add(2) };
    let h2_1 = unsafe { *h2.add(1) };
    let h2_2 = unsafe { *h2.add(2) };

    let cond = ((h1_1 ^ h2_1) & 0xFE) == 0
        && ((h1_2 ^ h2_2) & 0x0C) == 0
        && !(((h1_2 & 0xF0) == 0) ^ ((h2_2 & 0xF0) == 0));
    cond as c_int
}
