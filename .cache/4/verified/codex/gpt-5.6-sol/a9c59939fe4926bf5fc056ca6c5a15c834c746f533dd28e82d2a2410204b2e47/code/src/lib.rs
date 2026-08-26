use std::ffi::c_char;

#[inline]
fn encode_nibble(nibble: u32) -> u8 {
    87_u32
        .wrapping_add(nibble)
        .wrapping_add((nibble.wrapping_sub(10) >> 8) & !38_u32) as u8
}

/// Convert `bin_len` bytes from `bin` to lowercase hexadecimal in `hex`.
///
/// # Safety
///
/// The pointers and buffer sizes must satisfy the contract of the C `bin2hex`
/// function.
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

    let mut i = 0_usize;
    while i < bin_len {
        // SAFETY: The C API requires `bin` to reference at least `bin_len`
        // readable bytes and `hex` to reference the validated output size.
        let byte = unsafe { *bin.add(i) };
        let low = u32::from(byte & 0x0f);
        let high = u32::from(byte >> 4);
        let mut encoded = (u32::from(encode_nibble(low)) << 8) | u32::from(encode_nibble(high));

        unsafe {
            *hex.add(i * 2) = encoded as u8 as c_char;
            encoded >>= 8;
            *hex.add(i * 2 + 1) = encoded as u8 as c_char;
        }
        i += 1;
    }

    unsafe {
        *hex.add(i * 2) = 0;
    }
    hex
}
