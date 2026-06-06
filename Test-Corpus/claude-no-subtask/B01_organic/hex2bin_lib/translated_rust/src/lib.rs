use std::ffi::c_char;
use std::os::raw::c_int;

/// Translation of the C `hex2bin` function from c_src/src/lib.c.
///
/// # Safety
/// - `bin` must point to at least `bin_maxlen` writable bytes (or be null only if `bin_maxlen` is 0).
/// - `hex` must point to at least `hex_len` readable bytes.
/// - `ignore`, if non-null, must be a NUL-terminated C string.
/// - `hex_end_p`, if non-null, must point to a writable `*const c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hex2bin(
    bin: *mut u8,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    hex_end_p: *mut *const c_char,
) -> c_int {
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut c_acc: u8 = 0;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        // c = (unsigned char)hex[hex_pos];
        let c: u8 = unsafe { *hex.add(hex_pos) as u8 };

        // c_num = c ^ 48U;  (stored as unsigned char)
        let c_num: u8 = c ^ 48u8;

        // c_num0 = (c_num - 10U) >> 8;  (unsigned int arithmetic, truncated to u8)
        let c_num0: u8 = (((c_num as u32).wrapping_sub(10)) >> 8) as u8;

        // c_alpha = (c & ~32U) - 55U;  (unsigned int arithmetic, truncated to u8)
        let c_alpha: u8 = ((c as u32 & !32u32).wrapping_sub(55)) as u8;

        // c_alpha0 = ((c_alpha - 10U) ^ (c_alpha - 16U)) >> 8;
        let c_alpha_u = c_alpha as u32;
        let c_alpha0: u8 =
            ((c_alpha_u.wrapping_sub(10) ^ c_alpha_u.wrapping_sub(16)) >> 8) as u8;

        if (c_num0 | c_alpha0) == 0u8 {
            // Not a hex digit. Possibly an ignored character (only when state == 0).
            if !ignore.is_null() && state == 0u8 && unsafe { c_strchr(ignore, c) } {
                hex_pos += 1;
                continue;
            }
            break;
        }

        // c_val = (uint8_t)((c_num0 & c_num) | (c_alpha0 & c_alpha));
        let c_val: u8 = (c_num0 & c_num) | (c_alpha0 & c_alpha);

        if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }

        if state == 0u8 {
            // c_acc = c_val * 16U;  (unsigned char wraparound, but high nibble, so just shift)
            c_acc = c_val.wrapping_mul(16);
        } else {
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
            bin_pos += 1;
        }

        // state = ~state;  (toggle 0x00 <-> 0xFF)
        state = !state;
        hex_pos += 1;
    }

    if state != 0u8 {
        // Mirrors C: hex_pos--; (intentionally wraps if hex_pos was 0; not reachable since
        // state can only become non-zero after at least one increment of hex_pos).
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }

    if ret != 0 {
        bin_pos = 0;
    }

    if !hex_end_p.is_null() {
        unsafe { *hex_end_p = hex.add(hex_pos) };
    } else if hex_pos != hex_len {
        ret = -1;
    }

    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}

/// Replicates `strchr(s, c) != NULL` for a NUL-terminated C string `s` and a byte `c`.
///
/// Mirrors the C behavior where `strchr(s, '\0')` returns a pointer to the
/// terminating NUL (i.e. "found").
///
/// # Safety
/// `s` must be a valid pointer to a NUL-terminated C string.
unsafe fn c_strchr(s: *const c_char, c: u8) -> bool {
    let mut p = s;
    loop {
        let ch = unsafe { *p as u8 };
        if ch == c {
            return true;
        }
        if ch == 0 {
            return false;
        }
        p = unsafe { p.add(1) };
    }
}
