use std::os::raw::{c_char, c_uchar};
use std::process::abort;

#[unsafe(no_mangle)]
pub extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const c_uchar,
    bin_len: usize,
) -> *mut c_char {
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        abort();
    }

    let mut i = 0;
    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        let c = (byte & 0xf) as u32;
        let b = (byte >> 4) as u32;

        let char_c = (87 + c + ((c.wrapping_sub(10) >> 8) & !38)) as u8;
        let char_b = (87 + b + ((b.wrapping_sub(10) >> 8) & !38)) as u8;

        unsafe {
            *hex.add(i * 2) = char_b as c_char;
            *hex.add(i * 2 + 1) = char_c as c_char;
        }
        i += 1;
    }
    unsafe {
        *hex.add(i * 2) = 0;
    }

    hex
}
