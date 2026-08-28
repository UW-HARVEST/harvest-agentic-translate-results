//! Rust translation of `c_src/src/lib.c` (`hex2bin`).
//!
//! The original C relies heavily on integer promotion and truncation tricks to
//! stay branch-free while classifying hex characters. Every one of those
//! conversions is reproduced explicitly here so the observable behaviour --
//! return value, bytes written, and `*hex_end_p` -- is identical, including the
//! quirky cases (e.g. `strchr(ignore, 0)` matching the NUL terminator).

use std::ffi::{c_char, c_int};

/// Equivalent of `strchr(ignore, c) != NULL`.
///
/// Mirrors C `strchr` semantics: the terminating NUL is part of the searched
/// string, so looking for `0` always succeeds.
unsafe fn strchr_found(ignore: *const c_char, c: u8) -> bool {
    let mut p = ignore;
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
    // `unsigned char state`: toggled with `~state`, so it alternates 0 / 0xFF.
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        let c: u8 = unsafe { *hex.add(hex_pos) } as u8;

        // c_num = c ^ 48U;                      (truncated to unsigned char)
        let c_num: u8 = c ^ 48u8;
        // c_num0 = (c_num - 10U) >> 8;          (int arithmetic, then truncated)
        let c_num0: u8 = ((c_num as i32 - 10) >> 8) as u8;
        // c_alpha = (c & ~32U) - 55U;           (unsigned int, then truncated)
        let c_alpha: u8 = ((c as u32 & !32u32).wrapping_sub(55)) as u8;
        // c_alpha0 = ((c_alpha - 10U) ^ (c_alpha - 16U)) >> 8;
        let c_alpha0: u8 = (((c_alpha as i32 - 10) ^ (c_alpha as i32 - 16)) >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 && unsafe { strchr_found(ignore, c) } {
                hex_pos += 1;
                continue;
            }
            break;
        }

        let c_val: u8 = (c_num0 & c_num) | (c_alpha0 & c_alpha);

        if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }
        if state == 0 {
            c_acc = c_val.wrapping_mul(16);
        } else {
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
            bin_pos += 1;
        }
        state = !state;
        hex_pos += 1;
    }

    if state != 0 {
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
