use std::ffi::c_int;

fn hdr_valid(h: &[u8]) -> c_int {
    let cond = h[0] == 0xff
        && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
        && (((h[1] >> 1) & 3) != 0)
        && ((h[2] >> 4) != 15)
        && (((h[2] >> 2) & 3) != 3);
    cond as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    // Reproduce C semantics: build slices for safe access.
    let h1s = unsafe { std::slice::from_raw_parts(h1, 3) };
    let h2s = unsafe { std::slice::from_raw_parts(h2, 3) };

    // C: hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 &&
    //    ((h1[2] ^ h2[2]) & 0x0C) == 0 &&
    //    !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0))
    let valid = hdr_valid(h2s);
    if valid == 0 {
        return 0;
    }
    let c1 = ((h1s[1] ^ h2s[1]) & 0xFE) == 0;
    let c2 = ((h1s[2] ^ h2s[2]) & 0x0C) == 0;
    let a = (h1s[2] & 0xF0) == 0;
    let b = (h2s[2] & 0xF0) == 0;
    // !(a ^ b) -> a == b
    let c3 = a == b;

    (c1 && c2 && c3) as c_int
}
