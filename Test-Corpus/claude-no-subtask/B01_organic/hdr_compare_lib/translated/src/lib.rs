use std::ffi::c_int;

fn hdr_valid(h: &[u8; 3]) -> bool {
    h[0] == 0xff
        && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2)
        && (((h[1] >> 1) & 3) != 0)
        && ((h[2] >> 4) != 15)
        && (((h[2] >> 2) & 3) != 3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    let h1_arr = unsafe { [*h1, *h1.add(1), *h1.add(2)] };
    let h2_arr = unsafe { [*h2, *h2.add(1), *h2.add(2)] };

    let result = hdr_valid(&h2_arr)
        && ((h1_arr[1] ^ h2_arr[1]) & 0xFE) == 0
        && ((h1_arr[2] ^ h2_arr[2]) & 0x0C) == 0
        && !(((h1_arr[2] & 0xF0) == 0) ^ ((h2_arr[2] & 0xF0) == 0));

    result as c_int
}
