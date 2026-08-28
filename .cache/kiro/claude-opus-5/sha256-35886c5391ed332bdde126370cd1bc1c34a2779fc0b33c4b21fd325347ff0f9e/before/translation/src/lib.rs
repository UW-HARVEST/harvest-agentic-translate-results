//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is a byte-for-byte reproduction of the C implementation,
//! including its use of `abort()` on invalid arguments.

use std::ffi::{c_char, c_uchar};

/// `SIZE_MAX` as spelled literally in the C source (`18446744073709551615UL`).
const C_SIZE_MAX: usize = usize::MAX;

/// char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);
///
/// Converts `bin_len` bytes into a NUL-terminated lowercase hex string written
/// to `hex`. Aborts (like the C code) when `bin_len` is too large or the output
/// buffer cannot hold `bin_len * 2` characters plus the terminator.
///
/// # Safety
/// `hex` must be writable for at least `bin_len * 2 + 1` bytes and `bin` must be
/// readable for `bin_len` bytes, exactly as required by the C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const c_uchar,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;

    // Same order and semantics as the C validation, including abort().
    if bin_len >= C_SIZE_MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2) {
        std::process::abort();
    }

    while i < bin_len {
        let byte = unsafe { *bin.add(i) };

        // In C `c` and `b` are `int`s holding values in 0..=15; the mixed
        // arithmetic with `10U`/`87U`/`~38U` is performed in `unsigned int`.
        let c: u32 = (byte & 0x0f) as u32;
        let b: u32 = (byte >> 4) as u32;

        let lo = (87u32
            .wrapping_add(c)
            .wrapping_add((c.wrapping_sub(10) >> 8) & !38u32)) as u8;
        let hi = (87u32
            .wrapping_add(b)
            .wrapping_add((b.wrapping_sub(10) >> 8) & !38u32)) as u8;

        let mut x: u32 = ((lo as u32) << 8) | (hi as u32);

        unsafe {
            *hex.add(i * 2) = x as u8 as c_char;
            x >>= 8;
            *hex.add(i * 2 + 1) = x as u8 as c_char;
        }

        i += 1;
    }

    unsafe {
        *hex.add(i * 2) = 0;
    }

    hex
}
