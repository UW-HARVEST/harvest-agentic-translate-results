use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2) {
        std::process::abort();
    }
    let mut i: usize = 0;
    while i < bin_len {
        let byte = unsafe { *bin.add(i) };
        let c = (byte & 0xf) as i32;
        let b = (byte >> 4) as i32;
        let mut x: u32 = (((87u32.wrapping_add(c as u32)
            .wrapping_add(((c.wrapping_sub(10)) as u32 >> 8) & !38u32))
            as u8 as u32)
            << 8)
            | ((87u32.wrapping_add(b as u32)
                .wrapping_add(((b.wrapping_sub(10)) as u32 >> 8) & !38u32))
                as u8 as u32);
        unsafe {
            *hex.add(i * 2) = x as c_char;
        }
        x >>= 8;
        unsafe {
            *hex.add(i * 2 + 1) = x as c_char;
        }
        i += 1;
    }
    unsafe {
        *hex.add(i * 2) = 0;
    }
    hex
}
