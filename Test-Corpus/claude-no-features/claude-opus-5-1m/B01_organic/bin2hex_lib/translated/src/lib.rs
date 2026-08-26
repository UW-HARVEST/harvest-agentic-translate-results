use std::ffi::c_char;
use std::process::abort;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    // Match: bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        abort();
    }

    let mut i: usize = 0;
    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        // c = bin[i] & 0xf;  (int)
        // b = bin[i] >> 4;   (int)
        let c: u32 = (byte & 0xf) as u32;
        let b: u32 = (byte >> 4) as u32;

        // (unsigned char)(87U + c + (((c - 10U) >> 8) & ~38U))
        let lo: u8 = 87u32
            .wrapping_add(c)
            .wrapping_add((c.wrapping_sub(10) >> 8) & !38u32) as u8;
        // (unsigned char)(87U + b + (((b - 10U) >> 8) & ~38U))
        let hi: u8 = 87u32
            .wrapping_add(b)
            .wrapping_add((b.wrapping_sub(10) >> 8) & !38u32) as u8;

        // x = lo << 8 | hi
        let mut x: u32 = ((lo as u32) << 8) | (hi as u32);

        unsafe {
            // hex[i * 2U] = (char)x;
            *hex.add(i * 2) = x as c_char;
            x >>= 8;
            // hex[i * 2U + 1U] = (char)x;
            *hex.add(i * 2 + 1) = x as c_char;
        }
        i += 1;
    }
    // hex[i * 2U] = 0U;
    unsafe {
        *hex.add(i * 2) = 0;
    }
    hex
}
