use std::os::raw::c_int;

fn hdr_valid(h: &[u8; 4]) -> bool {
    h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) &&
    (((h[1] >> 1) & 3) != 0) && ((h[2] >> 4) != 15) &&
    (((h[2] >> 2) & 3) != 3)
}

#[unsafe(no_mangle)]
pub extern "C" fn hdr_compare(h1: *const u8, h2: *const u8) -> c_int {
    let h1_slice = unsafe { std::slice::from_raw_parts(h1, 4) };
    let h2_slice = unsafe { std::slice::from_raw_parts(h2, 4) };
    let h1_arr: &[u8; 4] = h1_slice.try_into().unwrap();
    let h2_arr: &[u8; 4] = h2_slice.try_into().unwrap();
    
    let result = hdr_valid(h2_arr) && ((h1_arr[1] ^ h2_arr[1]) & 0xFE) == 0 &&
                 ((h1_arr[2] ^ h2_arr[2]) & 0x0C) == 0 &&
                 !(((h1_arr[2] & 0xF0) == 0) ^ ((h2_arr[2] & 0xF0) == 0));
    
    result as c_int
}