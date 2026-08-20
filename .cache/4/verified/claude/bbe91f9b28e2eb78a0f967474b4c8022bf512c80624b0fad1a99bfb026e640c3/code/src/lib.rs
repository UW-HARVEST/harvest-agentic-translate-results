//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (as exported by the C shared library):
//!   * `rev16`  -- from `c_src/src/lib.c` / `c_src/include/lib.h`
//!
//! The translation is intentionally literal: every masking/shifting step is
//! reproduced in the same order and with the same widths as the C original so
//! that the output is byte-identical for every possible input (including the
//! fact that the C code silently discards the upper 16 bits of the argument,
//! because all of the masks are 16 bits wide).

/// `uint32_t rev16(uint32_t a);`
///
/// C original (`c_src/src/lib.c`):
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
/// In C, `a` is `uint32_t` and the literal masks have type `int`; the usual
/// arithmetic conversions turn them into `unsigned int`, so all arithmetic is
/// performed modulo 2^32.  No step can overflow 32 bits (the widest
/// intermediate is `(a & 0x00FF) << 8 <= 0xFF00`), so plain `u32` operations
/// reproduce the behaviour exactly.  Note that the upper 16 bits of the input
/// are dropped by the very first statement -- this is preserved, not "fixed".
#[unsafe(no_mangle)]
pub extern "C" fn rev16(a: u32) -> u32 {
    let mut a: u32 = a;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}
