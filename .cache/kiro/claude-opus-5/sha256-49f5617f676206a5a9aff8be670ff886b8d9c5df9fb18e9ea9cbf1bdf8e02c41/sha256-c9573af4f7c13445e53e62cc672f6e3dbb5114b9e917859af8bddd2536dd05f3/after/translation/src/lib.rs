//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface: `hex2bin` (see `c_src/include/lib.h`).
//!
//! The translation preserves the original C semantics exactly, including the
//! integer promotions that the constant-time-ish bit tricks in the C code rely
//! on:
//!
//! * `c_num0`, `c_alpha0` are computed with `unsigned int` arithmetic (the
//!   literals are `10U`/`16U`/`55U`), so the subtractions wrap modulo 2^32 and
//!   `>> 8` is a *logical* shift; the result is then truncated to
//!   `unsigned char`.
//! * `state = ~state` toggles between `0x00` and `0xFF` (not `0`/`1`).
//! * `strchr(ignore, c)` matches the NUL terminator when `c == 0`, so a NUL
//!   byte in the input is treated as "ignorable" whenever `ignore` is non-NULL
//!   and we are on an even nibble. This quirk is reproduced, not fixed.
//! * `hex_pos--` on a trailing odd nibble is reproduced with wrapping
//!   arithmetic.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int};

/// Equivalent of `strchr(s, c)`: returns true when the byte `c` occurs in the
/// NUL-terminated string `s`. As in C, the terminating NUL is part of the
/// searched string, so `c == 0` always matches.
///
/// # Safety
/// `s` must point to a NUL-terminated string.
unsafe fn c_strchr_found(s: *const c_char, c: u8) -> bool {
    let mut p = s;
    loop {
        let b = unsafe { *p } as u8;
        if b == c {
            return true;
        }
        if b == 0 {
            return false;
        }
        p = unsafe { p.add(1) };
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
        let c: u8 = unsafe { *hex.add(hex_pos) } as u8;

        // c_num = c ^ 48U;
        let c_num: u8 = c ^ 48u8;
        // c_num0 = (c_num - 10U) >> 8;   /* unsigned int arithmetic */
        let c_num0: u8 = ((c_num as u32).wrapping_sub(10) >> 8) as u8;

        // c_alpha = (c & ~32U) - 55U;
        let c_alpha: u8 = ((c as u32 & !32u32).wrapping_sub(55)) as u8;
        // c_alpha0 = ((c_alpha - 10U) ^ (c_alpha - 16U)) >> 8;
        let c_alpha0: u8 = (((c_alpha as u32).wrapping_sub(10)
            ^ (c_alpha as u32).wrapping_sub(16))
            >> 8) as u8;

        if (c_num0 | c_alpha0) == 0u8 {
            if !ignore.is_null() && state == 0u8 && unsafe { c_strchr_found(ignore, c) } {
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
            // c_acc = c_val * 16U;
            c_acc = c_val.wrapping_mul(16);
        } else {
            // bin[bin_pos++] = c_acc | c_val;
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
            bin_pos += 1;
        }

        // state = ~state;
        state = !state;
        hex_pos += 1;
    }

    if state != 0u8 {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end_p.is_null() {
        // *hex_end_p = &hex[hex_pos];
        unsafe { *hex_end_p = hex.wrapping_add(hex_pos) };
    } else if hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}
