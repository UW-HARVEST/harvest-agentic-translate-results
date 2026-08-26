use std::ffi::{c_char, c_uchar};
use std::os::raw::c_size_t;
use std::process::abort;

#[unsafe(no_mangle)]
pub extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: c_size_t,
    bin: *const c_uchar,
    bin_len: c_size_t,
) -> *mut c_char {
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        abort();
    }
    let bin_slice = unsafe { std::slice::from_raw_parts(bin, bin_len) };
    let hex_slice = unsafe { std::slice::from_raw_parts_mut(hex as *mut u8, bin_len * 2) };
    for i in 0..bin_len {
        let byte = bin_slice[i];
        let c = (byte & 0xf) as u32;
        let b = (byte >> 4) as u32;
        let x = (87u32.wrapping_add(c).wrapping_add(((c.wrapping_sub(10)) >> 8) & !38)) << 8
            | (87u32.wrapping_add(b).wrapping_add(((b.wrapping_sub(10)) >> 8) & !38));
        hex_slice[i * 2] = x as u8;
        hex_slice[i * 2 + 1] = (x >> 8) as u8;
    }
    unsafe {
        *hex.add(bin_len * 2) = 0;
    }
    hex
}
