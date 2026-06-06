use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    // Match the C check: bin_len >= 18446744073709551615UL / 2
    // On 64-bit platforms this is usize::MAX / 2.
    let limit: u64 = 18446744073709551615u64 / 2;
    if (bin_len as u64) >= limit || hex_maxlen <= bin_len.wrapping_mul(2) {
        std::process::abort();
    }

    let mut i: usize = 0;
    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        let c: u32 = (byte & 0x0f) as u32;
        let b: u32 = (byte >> 4) as u32;

        // (unsigned char)(87U + c + (((c - 10U) >> 8) & ~38U))
        let c_char_val: u8 = 87u32
            .wrapping_add(c)
            .wrapping_add((c.wrapping_sub(10) >> 8) & !38u32) as u8;
        let b_char_val: u8 = 87u32
            .wrapping_add(b)
            .wrapping_add((b.wrapping_sub(10) >> 8) & !38u32) as u8;

        // x = c_char_val << 8 | b_char_val
        let mut x: u32 = ((c_char_val as u32) << 8) | (b_char_val as u32);

        unsafe {
            *hex.add(i * 2) = (x & 0xff) as c_char;
        }
        x >>= 8;
        unsafe {
            *hex.add(i * 2 + 1) = (x & 0xff) as c_char;
        }
        i += 1;
    }
    unsafe {
        *hex.add(i * 2) = 0 as c_char;
    }
    hex
}
