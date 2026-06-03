use std::os::raw::c_char;

/// Convert a binary buffer to a hex-encoded null-terminated string.
///
/// # Safety
///
/// - `hex` must point to a writable buffer of at least `hex_maxlen` bytes.
/// - `bin` must point to a readable buffer of at least `bin_len` bytes.
/// - `hex_maxlen` must be strictly greater than `bin_len * 2` to leave room
///   for the trailing NUL byte. If this is not the case, the process aborts.
/// - `bin_len` must be less than `usize::MAX / 2`. Otherwise, the process aborts.
#[no_mangle]
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
        let byte = *bin.add(i);
        let c: u32 = (byte & 0xf) as u32;
        let b: u32 = (byte >> 4) as u32;

        // Replicates the original C bit trick:
        //   (87 + c + (((c - 10) >> 8) & ~38))
        // For c in 0..=9, ((c - 10) >> 8) & ~38 == 0xD8 (effectively -39 as u8),
        //   producing '0'..='9' (48..=57).
        // For c in 10..=15, ((c - 10) >> 8) & ~38 == 0,
        //   producing 'a'..='f' (97..=102).
        // The original C uses `unsigned int` arithmetic where the subtraction
        // for c >= 10 yields a small non-negative value (so the >>8 is 0),
        // and for c < 10 yields a large value with high bits set.
        let c_low = (c.wrapping_sub(10) >> 8) & !38u32;
        let b_low = (b.wrapping_sub(10) >> 8) & !38u32;

        let lo = (87u32.wrapping_add(c).wrapping_add(c_low)) as u8;
        let hi = (87u32.wrapping_add(b).wrapping_add(b_low)) as u8;

        let x: u32 = ((lo as u32) << 8) | (hi as u32);

        *hex.add(i * 2) = (x & 0xff) as c_char;
        let x_shifted = x >> 8;
        *hex.add(i * 2 + 1) = (x_shifted & 0xff) as c_char;

        i += 1;
    }
    *hex.add(i * 2) = 0;
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin2hex_basic() {
        let bin: [u8; 4] = [0x00, 0x0f, 0xab, 0xff];
        let mut hex_buf = [0i8; 16];
        unsafe {
            let ret = bin2hex(hex_buf.as_mut_ptr(), hex_buf.len(), bin.as_ptr(), bin.len());
            assert_eq!(ret, hex_buf.as_mut_ptr());
        }
        let s: Vec<u8> = hex_buf.iter().take_while(|&&b| b != 0).map(|&b| b as u8).collect();
        assert_eq!(std::str::from_utf8(&s).unwrap(), "000fabff");
    }

    #[test]
    fn test_bin2hex_empty() {
        let bin: [u8; 0] = [];
        let mut hex_buf = [0i8; 4];
        unsafe {
            bin2hex(hex_buf.as_mut_ptr(), hex_buf.len(), bin.as_ptr(), bin.len());
        }
        assert_eq!(hex_buf[0], 0);
    }
}
