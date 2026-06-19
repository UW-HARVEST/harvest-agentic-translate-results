use std::ffi::c_int;

unsafe fn hdr_valid(h: *const u8) -> bool {
    unsafe {
        *h.add(0) == 0xff
            && ((*h.add(1) & 0xF0) == 0xf0 || (*h.add(1) & 0xFE) == 0xe2)
            && (((*h.add(1) >> 1) & 3) != 0)
            && ((*h.add(2) >> 4) != 15)
            && (((*h.add(2) >> 2) & 3) != 3)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    unsafe {
        (hdr_valid(h2)
            && ((*h1.add(1) ^ *h2.add(1)) & 0xFE) == 0
            && ((*h1.add(2) ^ *h2.add(2)) & 0x0C) == 0
            && !(((*h1.add(2) & 0xF0) == 0) ^ ((*h2.add(2) & 0xF0) == 0))) as c_int
    }
}
