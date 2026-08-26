//! Rust translation of c_src/src/lib.c
//!
//! Provides a C-compatible `hex2bin` function which decodes a hex string into
//! a binary buffer. The semantics match the original C implementation,
//! including its constant-time-style hex digit detection.

use core::ffi::{c_char, c_int};
use core::ptr;

/// Find the first occurrence of byte `c` in the NUL-terminated string `s`.
/// Returns a pointer to that location, or null if not found. Mirrors libc
/// `strchr` semantics (matching the trailing NUL when `c == 0`).
///
/// # Safety
///
/// `s` must be a valid pointer to a NUL-terminated C string.
unsafe fn strchr(s: *const c_char, c: u8) -> *const c_char {
    let mut p = s;
    loop {
        let ch = *p as u8;
        if ch == c {
            return p;
        }
        if ch == 0 {
            return ptr::null();
        }
        p = p.add(1);
    }
}

/// Decode a hex string into binary.
///
/// - `bin` / `bin_maxlen`: output buffer and its capacity.
/// - `hex` / `hex_len`: input hex characters and how many to scan.
/// - `ignore`: optional NUL-terminated string of characters to skip while
///   between (not within) hex byte pairs. May be null.
/// - `hex_end_p`: optional out-parameter receiving a pointer to the first
///   non-consumed character of `hex`. May be null; if null, leftover input
///   produces an error.
///
/// Returns the number of bytes written on success, or -1 on error.
///
/// # Safety
///
/// All pointers must be valid for the lengths supplied. `ignore`, if not null,
/// must point to a NUL-terminated C string. `hex_end_p`, if not null, must be
/// a writable pointer.
#[no_mangle]
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
        let c: u8 = *hex.add(hex_pos) as u8;
        let c_num: u8 = c ^ 48u8;
        // Use wrapping arithmetic to mirror C's unsigned overflow semantics.
        // The original C performs ((c_num - 10U) >> 8) where c_num is promoted
        // to unsigned int. We replicate that behaviour explicitly using u32.
        let c_num0: u8 = (((c_num as u32).wrapping_sub(10)) >> 8) as u8;
        let c_alpha: u8 = (c & !32u8).wrapping_sub(55u8);
        let c_alpha0: u8 = ((((c_alpha as u32).wrapping_sub(10))
            ^ ((c_alpha as u32).wrapping_sub(16)))
            >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null()
                && state == 0
                && !strchr(ignore, c).is_null()
            {
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
            *bin.add(bin_pos) = c_acc | c_val;
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
        *hex_end_p = hex.add(hex_pos);
    } else if hex_pos != hex_len {
        ret = -1;
    }

    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}
