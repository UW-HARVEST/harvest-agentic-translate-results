//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!
//! ```c
//! typedef uint32_t tflac_u32;
//! tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
//! ```
//!
//! The header declares no function-renaming (namespace) macros, so the final
//! linker symbol is exactly `max_size_frame`, matching `nm -D` on the C `.so`.

#![allow(non_camel_case_types)]

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// Convert a C comparison result (`int` 0 or 1) into the unsigned type used by
/// the surrounding arithmetic.
///
/// In the original C the comparison operators yield `int`, which is then
/// subjected to the usual arithmetic conversions against `uint32_t` operands;
/// on the target platforms `int` and `uint32_t` share the same rank, so the
/// whole expression is evaluated in (wrapping) 32-bit unsigned arithmetic.
#[inline(always)]
fn b(cond: bool) -> tflac_u32 {
    if cond { 1 } else { 0 }
}

/// Translation of `max_size_frame` from `c_src/src/lib.c`.
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
/// Every operation is reproduced with explicit wrapping (modulo 2^32)
/// semantics, exactly as unsigned C arithmetic behaves, and the division is an
/// unsigned division by 8. No bugs or overflow quirks of the original are
/// "fixed": the result is bit-for-bit identical for all inputs.
#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(
    blocksize: tflac_u32,
    channels: tflac_u32,
    bitdepth: tflac_u32,
) -> tflac_u32 {
    // (blocksize * bitdepth * (channels * (channels != 2)))
    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(b(channels != 2)));

    // (blocksize * bitdepth * (channels == 2))
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(b(channels == 2));

    // (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2))
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(b(bitdepth != 32)))
        .wrapping_mul(b(channels == 2));

    // ... + +7) / 8
    let sum = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    // 18U + channels + (sum / 8)
    18u32.wrapping_add(channels).wrapping_add(sum / 8)
}
