use std::os::raw::c_int;

fn hdr_valid(h: *const u8) -> c_int {
    unsafe {
        let h0 = *h;
        let h1 = *h.add(1);
        let h2 = *h.add(2);
        (h0 == 0xff
            && ((h1 & 0xF0) == 0xf0 || (h1 & 0xFE) == 0xe2)
            && (((h1) >> 1) & 3) != 0
            && (((h2) >> 4) != 15)
            && ((((h2) >> 2) & 3) != 3)) as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    unsafe {
        let h1_2 = *h1.add(1);
        let h2_2 = *h2.add(1);
        let h1_3 = *h1.add(2);
        let h2_3 = *h2.add(2);
        (hdr_valid(h2) != 0
            && ((h1_2 ^ h2_2) & 0xFE) == 0
            && ((h1_3 ^ h2_3) & 0x0C) == 0
            && !(((h1_3 & 0xF0) == 0) ^ ((h2_3 & 0xF0) == 0))) as c_int
    }
}
