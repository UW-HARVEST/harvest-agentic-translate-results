use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        std::process::abort();
    }

    let mut i = 0;
    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        let c = u32::from(byte & 0x0f);
        let b = u32::from(byte >> 4);
        let mut x = ((87_u32
            .wrapping_add(c)
            .wrapping_add((c.wrapping_sub(10) >> 8) & !38)) as u8 as u32)
            << 8
            | ((87_u32
                .wrapping_add(b)
                .wrapping_add((b.wrapping_sub(10) >> 8) & !38)) as u8 as u32);

        unsafe {
            *hex.add(i * 2) = x as u8 as c_char;
            x >>= 8;
            *hex.add(i * 2 + 1) = x as u8 as c_char;
        }
        i += 1;
    }

    unsafe {
        *hex.add(i * 2) = 0;
    }
    hex
}
