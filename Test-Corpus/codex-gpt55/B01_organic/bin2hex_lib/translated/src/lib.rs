use std::ffi::c_char;

#[inline]
fn hex_digit(value: i32) -> u8 {
    let value = value as u32;
    (87u32
        .wrapping_add(value)
        .wrapping_add((value.wrapping_sub(10) >> 8) & !38u32)) as u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i = 0usize;

    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        std::process::abort();
    }

    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        let c = (byte & 0x0f) as i32;
        let b = (byte >> 4) as i32;
        let mut x = ((hex_digit(c) as u32) << 8) | hex_digit(b) as u32;

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
