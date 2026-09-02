//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared library):
//!   * `max_size_frame`
//!
//! The C header declares:
//! ```c
//! typedef uint32_t tflac_u32;
//! tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
//! ```
//! There are no namespace-renaming macros in the header, so the linker symbol
//! is plain `max_size_frame`.

#![allow(non_snake_case)]

/// `typedef uint32_t tflac_u32;`
#[allow(non_camel_case_types)]
pub type tflac_u32 = u32;

/// Translation of:
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
/// Every operand in the C expression is `uint32_t` (i.e. `unsigned int` on the
/// target ABI), so no integer promotion to `int` happens and all of the
/// arithmetic is performed modulo 2^32. The boolean sub-expressions
/// (`channels != 2`, `channels == 2`, `bitdepth != 32`) are C `int` values 0 or
/// 1 which convert to `unsigned int` in the surrounding multiplications.
/// Wrapping arithmetic is therefore used throughout to reproduce the C
/// behaviour bit-for-bit (including any overflow). The final division is an
/// unsigned division by 8, which can never trap.
#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(
    blocksize: tflac_u32,
    channels: tflac_u32,
    bitdepth: tflac_u32,
) -> tflac_u32 {
    // (channels != 2) as unsigned
    let chan_ne_2: tflac_u32 = (channels != 2) as tflac_u32;
    // (channels == 2) as unsigned
    let chan_eq_2: tflac_u32 = (channels == 2) as tflac_u32;
    // (bitdepth != 32) as unsigned
    let depth_ne_32: tflac_u32 = (bitdepth != 32) as tflac_u32;

    // blocksize * bitdepth * (channels * (channels != 2))
    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(chan_ne_2));

    // blocksize * bitdepth * (channels == 2)
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(chan_eq_2);

    // blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2)
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(depth_ne_32))
        .wrapping_mul(chan_eq_2);

    // (term1 + term2 + term3 + +7) / 8
    let bits = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);
    let bytes = bits / 8;

    // 18U + channels + bytes
    18u32.wrapping_add(channels).wrapping_add(bytes)
}
