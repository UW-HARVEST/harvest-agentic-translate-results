use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;
    let mut x: u32;
    let mut b: i32;
    let mut c: i32;

    if bin_len >= (18446744073709551615u64 as usize) / 2 || hex_maxlen <= bin_len * 2 {
        std::process::abort();
    }

    while i < bin_len {
        let bin_byte: u8 = unsafe { *bin.add(i) };
        c = (bin_byte & 0xf) as i32;
        b = (bin_byte >> 4) as i32;

        // Replicates C's unsigned int arithmetic with wrapping semantics:
        //   x = (unsigned char)(87U + c + (((c - 10U) >> 8) & ~38U)) << 8 |
        //       (unsigned char)(87U + b + (((b - 10U) >> 8) & ~38U));
        let c_adj: u32 = ((c as u32).wrapping_sub(10) >> 8) & !38u32;
        let b_adj: u32 = ((b as u32).wrapping_sub(10) >> 8) & !38u32;

        let lower_u8: u8 = 87u32
            .wrapping_add(c as u32)
            .wrapping_add(c_adj) as u8;
        let upper_u8: u8 = 87u32
            .wrapping_add(b as u32)
            .wrapping_add(b_adj) as u8;

        x = ((lower_u8 as u32) << 8) | (upper_u8 as u32);

        unsafe {
            *hex.add(i * 2) = x as u8 as c_char;
        }
        let x_shifted = x >> 8;
        unsafe {
            *hex.add(i * 2 + 1) = x_shifted as u8 as c_char;
        }
        i += 1;
    }
    unsafe {
        *hex.add(i * 2) = 0 as c_char;
    }
    hex
}
