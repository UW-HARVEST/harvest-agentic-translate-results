//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single public symbol, `bin2hex`, declared in
//! `include/lib.h` and defined in `src/lib.c`. There are no namespace/renaming
//! preprocessor macros in the header, so the final linker symbol is plainly
//! `bin2hex`.
//!
//! The original implementation uses a branch-free nibble-to-ASCII conversion
//! that relies on C's integer promotion rules (mixing `int` and `unsigned int`
//! operands makes the whole expression `unsigned int`, with wrapping
//! semantics). All of that arithmetic is reproduced bit-for-bit below using
//! explicit `u32` wrapping operations.

use core::ffi::{c_char, c_uchar};

/// `18446744073709551615UL / 2` as evaluated by the C compiler.
///
/// The C source spells `SIZE_MAX` out as a literal and divides it by two using
/// integer division, yielding `9223372036854775807`.
const C_BIN_LEN_LIMIT: usize = (18446744073709551615u64 / 2) as usize;

/// Branch-free nibble -> ASCII hex digit conversion, exactly as written in C:
///
/// ```c
/// (unsigned char)(87U + n + (((n - 10U) >> 8) & ~38U))
/// ```
///
/// `n` is an `int` in the C source, but because `10U` and `87U` are `unsigned
/// int`, every operand is converted to `unsigned int` before the arithmetic
/// happens. Therefore `n - 10U` wraps around for `n < 10`, and `>>` is a
/// logical (not arithmetic) shift.
#[inline]
fn c_nibble_to_hex(n: i32) -> u8 {
    // `int` -> `unsigned int` conversion (modulo 2^32).
    let n = n as u32;
    // ((n - 10U) >> 8) & ~38U
    let adjust = n.wrapping_sub(10u32) >> 8 & !38u32;
    // (unsigned char)(87U + n + adjust)
    87u32.wrapping_add(n).wrapping_add(adjust) as u8
}

/// ```c
/// char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);
/// ```
///
/// Writes `bin_len * 2` lowercase hex characters plus a NUL terminator into
/// `hex`, returning `hex`. Calls `abort()` when `bin_len` is absurdly large or
/// when `hex_maxlen` cannot hold the result together with its terminator.
///
/// # Safety
///
/// Same contract as the C function: `hex` must point to at least `hex_maxlen`
/// writable bytes and `bin` must point to at least `bin_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const c_uchar,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;

    // The C code performs both checks in this order, with `bin_len * 2U`
    // computed in wrapping `size_t` arithmetic.
    if bin_len >= C_BIN_LEN_LIMIT || hex_maxlen <= bin_len.wrapping_mul(2) {
        // Matches C's `abort()`: raises SIGABRT without unwinding.
        std::process::abort();
    }

    // Past the guard above we know `hex_maxlen > bin_len * 2`, so writing
    // `bin_len * 2 + 1` bytes stays inside the caller's buffer. Borrowing the
    // buffers as slices lets the body of the loop stay safe Rust. Slices are
    // only formed when non-empty, since `from_raw_parts` rejects null pointers
    // even at length zero.
    let out_len = bin_len * 2 + 1;
    let out: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(hex.cast::<u8>(), out_len) };
    let src: &[u8] = if bin_len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(bin, bin_len) }
    };

    while i < bin_len {
        // c = bin[i] & 0xf;  b = bin[i] >> 4;  (both `int` after promotion)
        let c = (src[i] & 0xf) as i32;
        let b = (src[i] >> 4) as i32;

        // x = (unsigned char)(...c...) << 8 | (unsigned char)(...b...);
        let mut x: u32 = (c_nibble_to_hex(c) as u32) << 8 | c_nibble_to_hex(b) as u32;

        // hex[i * 2U] = (char)x;  -> low byte, i.e. the high nibble's digit
        out[i * 2] = x as u8;
        x >>= 8;
        // hex[i * 2U + 1U] = (char)x;  -> the low nibble's digit
        out[i * 2 + 1] = x as u8;

        i += 1;
    }

    // hex[i * 2U] = 0U;
    out[i * 2] = 0;

    hex
}
