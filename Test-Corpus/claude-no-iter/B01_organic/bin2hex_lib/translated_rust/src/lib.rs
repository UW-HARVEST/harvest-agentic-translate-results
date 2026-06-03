use std::ffi::c_char;
use std::process::abort;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    // Match the original C error-check ordering.
    if bin_len >= (u64::MAX as usize) / 2 || hex_maxlen <= bin_len * 2usize {
        abort();
    }

    let mut i: usize = 0;
    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        let c: i32 = (byte & 0xf) as i32;
        let b: i32 = (byte >> 4) as i32;

        // Replicate: (unsigned char)(87U + n + (((n - 10U) >> 8) & ~38U))
        // All intermediate arithmetic is unsigned int (u32) in C.
        let c_diff: u32 = (c as u32).wrapping_sub(10u32);
        let b_diff: u32 = (b as u32).wrapping_sub(10u32);

        let lo_char: u8 =
            (87u32.wrapping_add(c as u32).wrapping_add((c_diff >> 8) & !38u32)) as u8;
        let hi_char: u8 =
            (87u32.wrapping_add(b as u32).wrapping_add((b_diff >> 8) & !38u32)) as u8;

        // x = lo_char << 8 | hi_char  (as unsigned int)
        let mut x: u32 = ((lo_char as u32) << 8) | (hi_char as u32);

        unsafe {
            *hex.add(i * 2) = x as u8 as c_char;
        }
        x >>= 8;
        unsafe {
            *hex.add(i * 2 + 1) = x as u8 as c_char;
        }
        i += 1;
    }
    unsafe {
        *hex.add(i * 2) = 0 as c_char;
    }
    hex
}
