use std::ffi::c_char;
use std::os::raw::c_uchar;

#[unsafe(no_mangle)]
pub extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        std::process::abort();
    }

    let hex_slice = unsafe { std::slice::from_raw_parts_mut(hex as *mut u8, hex_maxlen) };
    let bin_slice = unsafe { std::slice::from_raw_parts(bin as *const c_uchar, bin_len) };

    for (i, &byte) in bin_slice.iter().enumerate() {
        let b = (byte >> 4) as u8;
        let c = (byte & 0x0f) as u8;

        let high = if b < 10 { b'0' + b } else { b'a' + (b - 10) };
        let low = if c < 10 { b'0' + c } else { b'a' + (c - 10) };

        hex_slice[i * 2] = high;
        hex_slice[i * 2 + 1] = low;
    }

    hex_slice[bin_len * 2] = 0;
    hex
}