use std::os::raw::c_int;

unsafe fn hdr_valid(h: *const u8) -> c_int {
    let h = unsafe { std::slice::from_raw_parts(h, 4) };
    ((h[0] == 0xff)
        && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
        && (((h[1]) >> 1) & 3) != 0
        && (((h[2]) >> 4) != 15)
        && ((((h[2]) >> 2) & 3) != 3)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    let s1 = unsafe { std::slice::from_raw_parts(h1, 4) };
    let s2 = unsafe { std::slice::from_raw_parts(h2, 4) };
    (unsafe { hdr_valid(h2) } != 0
        && ((s1[1] ^ s2[1]) & 0xFE) == 0
        && ((s1[2] ^ s2[2]) & 0x0C) == 0
        && !(((s1[2] & 0xF0) == 0) ^ ((s2[2] & 0xF0) == 0))) as c_int
}
