use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;
    // (18446744073709551615UL) / 2 is ULONG_MAX / 2. On 64-bit Linux platforms
    // (where size_t == unsigned long == 64 bits), this equals usize::MAX / 2.
    if bin_len >= 18446744073709551615u64 as usize / 2
        || hex_maxlen <= bin_len.wrapping_mul(2)
    {
        std::process::abort();
    }
    while i < bin_len {
        let byte: u8 = unsafe { *bin.add(i) };
        // In the original C, `c` and `b` are `int`. The computations involve
        // `(c - 10U)`, which is unsigned-int arithmetic (10U promotes c to
        // unsigned int). We mirror that by using u32 here.
        let c: u32 = (byte & 0xf) as u32;
        let b: u32 = (byte >> 4) as u32;
        let mask_c: u32 = (c.wrapping_sub(10) >> 8) & !38u32;
        let mask_b: u32 = (b.wrapping_sub(10) >> 8) & !38u32;
        let lo_byte: u8 = 87u32.wrapping_add(c).wrapping_add(mask_c) as u8;
        let hi_byte: u8 = 87u32.wrapping_add(b).wrapping_add(mask_b) as u8;
        // x = (unsigned char)(...) << 8 | (unsigned char)(...);
        let mut x: u32 = ((lo_byte as u32) << 8) | (hi_byte as u32);
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
        *hex.add(i * 2) = 0;
    }
    hex
}
