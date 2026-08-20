//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (must match `nm -D` of the C shared object exactly):
//!   * `bin2hex`
//!
//! The translation is intentionally literal: the C integer promotions,
//! wrapping unsigned arithmetic, truncating casts and the exact order of the
//! validation checks are all reproduced so that the observable behaviour
//! (including the `abort()` paths) is byte-for-byte identical.

use std::ffi::c_char;

/// `char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);`
///
/// C source:
/// ```c
/// char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin,
///                     size_t bin_len) {
///     size_t i = (size_t)0U;
///     unsigned int x;
///     int b;
///     int c;
///     if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
///         abort();
///     }
///     while (i < bin_len) {
///         c = bin[i] & 0xf;
///         b = bin[i] >> 4;
///         x = (unsigned char)(87U + c + (((c - 10U) >> 8) & ~38U)) << 8 |
///             (unsigned char)(87U + b + (((b - 10U) >> 8) & ~38U));
///         hex[i * 2U] = (char)x;
///         x >>= 8;
///         hex[i * 2U + 1U] = (char)x;
///         i++;
///     }
///     hex[i * 2U] = 0U;
///     return hex;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;

    // The C literal 18446744073709551615UL is UINT64_MAX == SIZE_MAX on the
    // 64-bit LP64 targets this library is built for.  `bin_len * 2U` is a
    // size_t multiplication in C, hence the wrapping multiply here.
    //
    // Note the short-circuit `||`: the length check happens before the
    // buffer-capacity check, and both are evaluated before any store.
    if bin_len >= (18446744073709551615u64 as usize) / 2
        || hex_maxlen <= bin_len.wrapping_mul(2)
    {
        std::process::abort();
    }

    while i < bin_len {
        let byte = unsafe { *bin.add(i) };

        // `c` and `b` are `int` in C; the operands below are promoted to
        // `unsigned int` because of the `10U` / `87U` / `~38U` literals, so all
        // of this arithmetic is modulo 2^32.
        let c: u32 = (byte & 0xf) as u32;
        let b: u32 = (byte >> 4) as u32;

        // (unsigned char)(87U + c + (((c - 10U) >> 8) & ~38U))
        let lo_ch: u32 = (87u32
            .wrapping_add(c)
            .wrapping_add((c.wrapping_sub(10) >> 8) & !38u32)) as u8 as u32;
        // (unsigned char)(87U + b + (((b - 10U) >> 8) & ~38U))
        let hi_ch: u32 = (87u32
            .wrapping_add(b)
            .wrapping_add((b.wrapping_sub(10) >> 8) & !38u32)) as u8 as u32;

        // The low-nibble character is placed in the *high* byte of x and the
        // high-nibble character in the low byte, matching the C expression.
        let mut x: u32 = (lo_ch << 8) | hi_ch;

        unsafe {
            *hex.add(i.wrapping_mul(2)) = x as u8 as c_char;
            x >>= 8;
            *hex.add(i.wrapping_mul(2).wrapping_add(1)) = x as u8 as c_char;
        }

        i = i.wrapping_add(1);
    }

    // i == bin_len here; the NUL terminator goes right after the last digit.
    unsafe {
        *hex.add(i.wrapping_mul(2)) = 0;
    }

    hex
}
