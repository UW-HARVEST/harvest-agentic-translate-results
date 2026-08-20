//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (as exported by the C shared library):
//!   * `max_size_frame`
//!
//! The C header declares:
//! ```c
//! #include <stdint.h>
//! typedef uint32_t tflac_u32;
//! tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
//! ```
//!
//! There are no namespace-renaming preprocessor macros in the public header, so
//! the final linker symbol name is identical to the source-level name.

#![allow(non_camel_case_types)]

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// C relational/equality operators yield `int` 1 or 0. Because every operand in
/// the expression below is a `uint32_t`, the usual arithmetic conversions turn
/// each of those `int` results back into `uint32_t`, so all of the arithmetic is
/// performed in unsigned 32-bit (wrapping) arithmetic.
#[inline(always)]
const fn c_bool(b: bool) -> u32 {
    if b { 1 } else { 0 }
}

/// Direct translation of:
///
/// ```c
/// tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth) {
///     return 18U + channels +
///            (((blocksize * bitdepth * (channels * (channels != 2))) +
///              (blocksize * bitdepth * (channels == 2)) +
///              (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2)) +
///              +7) /
///             8);
/// }
/// ```
///
/// All operations use wrapping (modulo 2^32) semantics to match unsigned C
/// arithmetic exactly, and the division is an unsigned division.
#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(
    blocksize: tflac_u32,
    channels: tflac_u32,
    bitdepth: tflac_u32,
) -> tflac_u32 {
    // (blocksize * bitdepth * (channels * (channels != 2)))
    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(c_bool(channels != 2)));

    // (blocksize * bitdepth * (channels == 2))
    let term2 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(c_bool(channels == 2));

    // (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2))
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(c_bool(bitdepth != 32)))
        .wrapping_mul(c_bool(channels == 2));

    // ((term1 + term2 + term3 + +7) / 8)
    let numerator = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    // 18U + channels + (numerator / 8)
    18u32
        .wrapping_add(channels)
        .wrapping_add(numerator / 8)
}
