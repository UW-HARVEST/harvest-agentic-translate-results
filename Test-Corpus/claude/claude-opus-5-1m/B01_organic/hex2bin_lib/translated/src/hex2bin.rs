//! Translation of `c_src/src/lib.c`.

use core::ffi::{c_char, c_int};

/// Faithful re-implementation of `strchr(ignore, c)` returning whether a match
/// was found.
///
/// Note the C behaviour that is relied upon by the original code: when `c` is
/// `0`, `strchr` matches the string's terminating NUL byte and therefore
/// returns a non-NULL pointer.
#[inline]
unsafe fn strchr_matches(s: *const c_char, c: u8) -> bool {
    let mut p = s;
    loop {
        // Read one byte; `char` may be signed, so go through `u8`.
        let b = unsafe { *p } as u8;
        if b == c {
            return true;
        }
        if b == 0 {
            return false;
        }
        p = p.wrapping_add(1);
    }
}

/// ```c
/// int hex2bin(uint8_t *bin, size_t bin_maxlen, const char *hex,
///             size_t hex_len, const char *ignore, const char **hex_end_p);
/// ```
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
        let c: u8 = unsafe { *hex.wrapping_add(hex_pos) } as u8;

        // All of the arithmetic below is performed in C on `unsigned int`
        // (integer promotion of `unsigned char` to `int`, then conversion to
        // `unsigned int` because of the unsigned literals) and the result is
        // then truncated back into an `unsigned char` object.

        // c_num = c ^ 48U;
        let c_num: u8 = ((c as u32) ^ 48u32) as u8;
        // c_num0 = (c_num - 10U) >> 8;
        let c_num0: u8 = ((c_num as u32).wrapping_sub(10u32) >> 8) as u8;
        // c_alpha = (c & ~32U) - 55U;
        let c_alpha: u8 = (((c as u32) & !32u32).wrapping_sub(55u32)) as u8;
        // c_alpha0 = ((c_alpha - 10U) ^ (c_alpha - 16U)) >> 8;
        let c_alpha0: u8 = ((((c_alpha as u32).wrapping_sub(10u32))
            ^ ((c_alpha as u32).wrapping_sub(16u32)))
            >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 && unsafe { strchr_matches(ignore, c) } {
                hex_pos = hex_pos.wrapping_add(1);
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
        if state == 0 {
            // c_acc = c_val * 16U;
            c_acc = c_val.wrapping_mul(16);
        } else {
            // bin[bin_pos++] = c_acc | c_val;
            unsafe {
                *bin.wrapping_add(bin_pos) = c_acc | c_val;
            }
            bin_pos = bin_pos.wrapping_add(1);
        }
        // state = ~state;  (0U <-> 0xFFU for an unsigned char object)
        state = !state;
        hex_pos = hex_pos.wrapping_add(1);
    }

    if state != 0 {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end_p.is_null() {
        unsafe {
            *hex_end_p = hex.wrapping_add(hex_pos);
        }
    } else if hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}
