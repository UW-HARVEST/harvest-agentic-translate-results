//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared library):
//!   * `hex2bin`
//!
//! The translation is bit-for-bit faithful to the C implementation, including
//! its integer-promotion / truncation behaviour and its quirks (for example the
//! fact that a NUL byte inside the `hex` buffer is treated as an "ignore"
//! character whenever `ignore` is non-NULL, because `strchr()` also matches the
//! terminating NUL of the ignore set).

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int};

/// Faithful re-implementation of `strchr(s, c) != NULL`.
///
/// As mandated by the C standard, the terminating NUL byte of `s` is considered
/// to be part of the string, therefore searching for `c == 0` always succeeds.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string.
unsafe fn strchr_found(s: *const c_char, c: u8) -> bool {
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

/// Result of decoding one candidate character.
struct Nibble {
    /// `c_num` (`unsigned char`)
    c_num: u8,
    /// `c_num0` (`unsigned char`)
    c_num0: u8,
    /// `c_alpha` (`unsigned char`)
    c_alpha: u8,
    /// `c_alpha0` (`unsigned char`)
    c_alpha0: u8,
}

/// Reproduces the branch-free hex-digit classification of the C code, keeping
/// every intermediate value in the exact same width the C compiler would use:
/// the arithmetic is performed in `unsigned int` (32 bit, wrapping) and the
/// result is then truncated back into an `unsigned char`.
#[inline]
fn classify(c: u8) -> Nibble {
    let cu = c as u32;

    // c_num = c ^ 48U;
    let c_num = (cu ^ 48u32) as u8;
    // c_num0 = (c_num - 10U) >> 8;
    let c_num0 = ((c_num as u32).wrapping_sub(10u32) >> 8) as u8;
    // c_alpha = (c & ~32U) - 55U;
    let c_alpha = (cu & !32u32).wrapping_sub(55u32) as u8;
    // c_alpha0 = ((c_alpha - 10U) ^ (c_alpha - 16U)) >> 8;
    let ca = c_alpha as u32;
    let c_alpha0 = ((ca.wrapping_sub(10u32) ^ ca.wrapping_sub(16u32)) >> 8) as u8;

    Nibble {
        c_num,
        c_num0,
        c_alpha,
        c_alpha0,
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
        let c = unsafe { *hex.add(hex_pos) } as u8;

        let n = classify(c);

        if (n.c_num0 | n.c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 && unsafe { strchr_found(ignore, c) } {
                hex_pos += 1;
                continue;
            }
            break;
        }

        // c_val = (uint8_t)((c_num0 & c_num) | (c_alpha0 & c_alpha));
        let c_val: u8 = (n.c_num0 & n.c_num) | (n.c_alpha0 & n.c_alpha);

        if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }

        if state == 0 {
            // c_acc = c_val * 16U;  (computed in int, truncated to uint8_t)
            c_acc = c_val.wrapping_mul(16);
        } else {
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
            bin_pos += 1;
        }

        // state = ~state;  (0 -> 0xFF -> 0 ...)
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
        unsafe { *hex_end_p = hex.wrapping_add(hex_pos) };
    } else if hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}
