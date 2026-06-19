use core::ffi::c_char;

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

    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2) {
        std::process::abort();
    }

    while i < bin_len {
        let byte = *bin.add(i);

        c = i32::from(byte & 0x0f);
        b = i32::from(byte >> 4);

        x = ((87u32)
            .wrapping_add(c as u32)
            .wrapping_add((((c as u32).wrapping_sub(10)) >> 8) & !38u32)
            as u8 as u32)
            << 8
            | ((87u32)
                .wrapping_add(b as u32)
                .wrapping_add((((b as u32).wrapping_sub(10)) >> 8) & !38u32)
                as u8 as u32);

        *hex.add(i.wrapping_mul(2)) = x as u8 as c_char;
        x >>= 8;
        *hex.add(i.wrapping_mul(2).wrapping_add(1)) = x as u8 as c_char;

        i = i.wrapping_add(1);
    }

    *hex.add(i.wrapping_mul(2)) = 0 as c_char;
    hex
}
