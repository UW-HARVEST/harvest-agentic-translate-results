//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) whose only
//! public (dynamically exported) symbol is `bin2hex`, declared in
//! `include/lib.h` as:
//!
//! ```c
//! char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin,
//!               size_t bin_len);
//! ```
//!
//! There are no namespace/renaming macros in the public header, so the final
//! linker symbol is plain `bin2hex`.
//!
//! The translation reproduces the original C semantics exactly, including the
//! constant-time-ish (branch free) nibble-to-ASCII conversion performed with
//! wrapping `unsigned int` arithmetic, the exact order of the validation
//! checks, and the `abort()` call on invalid arguments.
//!
//! ## Why the memory accesses go through `ptr::read` / `ptr::write`
//!
//! `bin2hex` performs no null checks, so C reaches the hardware with a null
//! `hex` or `bin` and the process dies from `SIGSEGV`. A Rust place expression
//! (`*p` / `*p = v`) is compiled with an inserted null-pointer assertion
//! whenever `-C debug-assertions` is on (the default `dev` profile), which turns
//! that same input into a `SIGABRT` ("null pointer dereference occurred") and so
//! *diverges from C in debug builds*. `core::ptr::read` / `core::ptr::write`
//! lower to a plain load/store with no inserted check, reproducing the C
//! failure mode (`SIGSEGV`) under every profile.
//!
//! For the same reason the address arithmetic uses `<*T>::wrapping_add` rather
//! than `add`: it computes the identical address without asserting that the
//! offset stays in bounds, so out-of-range `bin_len` values fault exactly where
//! the C code faults instead of tripping a Rust-level check.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_uchar, c_uint};

unsafe extern "C" {
    /// libc's `abort()`, as used by `src/lib.c` via `<stdlib.h>`.
    ///
    /// Using the real libc `abort` (rather than a Rust-level panic) keeps the
    /// observable behaviour identical: the process dies from `SIGABRT`.
    fn abort() -> !;
}

/// `SIZE_MAX` as spelled literally in the C source
/// (`18446744073709551615UL`). The C code hard-codes the 64-bit value, and the
/// build targets a 64-bit platform, so this is `usize::MAX` there.
const C_SIZE_MAX: usize = 18446744073709551615u64 as usize;

/// Translation of the branch-free nibble -> ASCII hex digit expression:
///
/// ```c
/// (unsigned char)(87U + n + (((n - 10U) >> 8) & ~38U))
/// ```
///
/// `n` is an `int` in C, but because it is combined with the `unsigned int`
/// constants `87U`, `10U` and `~38U`, every operation is carried out modulo
/// 2^32 in `unsigned int`. For `n < 10` the subtraction wraps around, the
/// right shift leaves `0x00ffffff`, and masking with `~38U` (`0xffffffd9`)
/// yields `0x00ffffd9`, so that `87 + n + 0x00ffffd9` truncates (via the
/// `(unsigned char)` cast) to `'0' + n`. For `10 <= n <= 15` the correction
/// term is zero and the result is `87 + n`, i.e. `'a' + n - 10`.
#[inline]
fn nibble_to_hex(n: c_int) -> c_uchar {
    let n = n as c_uint;
    let corrected = (87u32)
        .wrapping_add(n)
        .wrapping_add((n.wrapping_sub(10u32) >> 8) & !38u32);
    // The `(unsigned char)` cast keeps only the low 8 bits.
    corrected as c_uchar
}

/// Byte-for-byte translation of `bin2hex` from `c_src/src/lib.c`.
///
/// # Safety
///
/// Same contract as the C function: `bin` must point to at least `bin_len`
/// readable bytes and `hex` must point to at least `bin_len * 2 + 1` writable
/// bytes (which the `hex_maxlen` check partially enforces, aborting otherwise).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    // C: size_t i = (size_t)0U; unsigned int x; int b; int c;
    let mut i: usize = 0usize;

    // if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
    //     abort();
    // }
    //
    // Note the order of the checks and the short-circuiting `||`: the first
    // check guarantees that `bin_len * 2` cannot overflow. `bin_len * 2U`
    // promotes `2U` to `size_t`, so the product is computed in `size_t`.
    if bin_len >= C_SIZE_MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2usize) {
        unsafe { abort() };
    }

    while i < bin_len {
        // c = bin[i] & 0xf;  b = bin[i] >> 4;   (uint8_t promoted to int)
        let byte = unsafe { core::ptr::read(bin.wrapping_add(i)) } as c_int;
        let c: c_int = byte & 0xf;
        let b: c_int = byte >> 4;
        // x = lo_nibble_char << 8 | hi_nibble_char;
        let mut x: c_uint = ((nibble_to_hex(c) as c_uint) << 8) | (nibble_to_hex(b) as c_uint);
        // hex[i * 2U] = (char)x;  -> the high nibble's character
        unsafe { core::ptr::write(hex.wrapping_add(i.wrapping_mul(2usize)), x as u8 as c_char) };
        x >>= 8;
        // hex[i * 2U + 1U] = (char)x;  -> the low nibble's character
        unsafe {
            core::ptr::write(
                hex.wrapping_add(i.wrapping_mul(2usize).wrapping_add(1usize)),
                x as u8 as c_char,
            )
        };
        i = i.wrapping_add(1usize);
    }
    // hex[i * 2U] = 0U;
    unsafe { core::ptr::write(hex.wrapping_add(i.wrapping_mul(2usize)), 0) };

    hex
}
