//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (the complete set of symbols exported by the C shared library,
//! verified with `nm -D` on the CMake-built `.so`):
//!
//!   * `uint32_t rev16(uint32_t a)`
//!
//! The public header `c_src/include/lib.h` declares only `rev16` and contains
//! no namespace/renaming macros, so the final linker symbol name is identical
//! to the source-level name.

use std::ffi::c_uint;

/// Reverses the order of the low 16 bits of `a`.
///
/// Faithful translation of `rev16` from `c_src/src/lib.c`:
///
/// ```c
/// uint32_t rev16(uint32_t a) {
///     a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
///     a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
///     a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
///     a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
///     return a;
/// }
/// ```
///
/// The statements are kept in the original order, using the original masks and
/// shift amounts.
///
/// Note that every mask is only 16 bits wide, so the very first statement
/// discards bits 16..=31 of the input and the result therefore always lies in
/// the low 16 bits. That is the behaviour of the original C code and it is
/// reproduced here verbatim rather than "fixed".
///
/// All intermediate values stay well inside the range of `u32` (the largest
/// shifted value is `0xFF00`), so no overflow or wrapping can occur.
#[unsafe(no_mangle)]
pub extern "C" fn rev16(a: c_uint) -> c_uint {
    // `uint32_t` and `unsigned int` (`c_uint`) are both 32-bit unsigned types
    // on every platform this library targets; work on a `u32` for clarity.
    let mut a: u32 = a as u32;

    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);

    a as c_uint
}
