//! Rust translation of `c_src/src/lib.c`.
//!
//! The C source declares a single public function, `rev16`, in
//! `c_src/include/lib.h`. There are no namespace-renaming preprocessor macros,
//! so the final linker symbol is plain `rev16`.

use std::ffi::c_uint;

/// Reverse the bit order of the low 16 bits of `a`.
///
/// This mirrors the C implementation exactly, including the fact that every
/// mask is a 16-bit constant. In the C code the very first statement
/// (`(a & 0xAAAA) >> 1 | (a & 0x5555) << 1`) discards bits 16..=31 of the
/// input, so the high half of the argument never contributes to the result.
/// That behavior is preserved here rather than "fixed".
#[unsafe(no_mangle)]
pub extern "C" fn rev16(a: c_uint) -> c_uint {
    // `c_uint` is `u32` on every platform this crate targets; operate on u32 so
    // the shifts have the same wrapping/truncation semantics as C's uint32_t.
    let mut a: u32 = a as u32;

    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);

    a as c_uint
}
