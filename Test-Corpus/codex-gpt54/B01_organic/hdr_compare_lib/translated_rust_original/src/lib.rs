use std::ffi::c_int;

#[inline]
unsafe fn hdr_valid(h: *const u8) -> bool {
    // Match the C evaluation order and pointer-based reads exactly.
    unsafe {
        let h0 = *h.add(0);
        let h1 = *h.add(1);
        let h2 = *h.add(2);

        h0 == 0xff
            && ((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2)
            && (((h1 >> 1) & 3) != 0)
            && ((h2 >> 4) != 15)
            && (((h2 >> 2) & 3) != 3)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    unsafe {
        let valid_h2 = hdr_valid(h2);
        let same_h1 = ((*h1.add(1) ^ *h2.add(1)) & 0xFE) == 0;
        let same_h2 = ((*h1.add(2) ^ *h2.add(2)) & 0x0C) == 0;
        let same_mode = (((*h1.add(2)) & 0xF0) == 0) == (((*h2.add(2)) & 0xF0) == 0);

        (valid_h2 && same_h1 && same_h2 && same_mode) as c_int
    }
}
