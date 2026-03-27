use std::ffi::c_char;

/// # Safety
/// Caller must ensure pointers and lengths are valid per the C contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    if bin_len >= 18446744073709551615_u64 as usize / 2
        || hex_maxlen <= bin_len.wrapping_mul(2)
    {
        std::process::abort();
    }
    let mut i: usize = 0;
    while i < bin_len {
        let byte = *bin.add(i);
        let c = (byte & 0xf) as u32;
        let b = (byte >> 4) as u32;
        let lo = (87_u32.wrapping_add(c).wrapping_add(
            (c.wrapping_sub(10) >> 8) & !38_u32,
        )) as u8;
        let hi = (87_u32.wrapping_add(b).wrapping_add(
            (b.wrapping_sub(10) >> 8) & !38_u32,
        )) as u8;
        let mut x: u32 = (lo as u32) << 8 | hi as u32;
        *hex.add(i * 2) = x as c_char;
        x >>= 8;
        *hex.add(i * 2 + 1) = x as c_char;
        i += 1;
    }
    *hex.add(i * 2) = 0;
    hex
}
