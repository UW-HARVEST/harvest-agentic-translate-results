//! Rust translation of `c_src/src/lib.c`.
//!
//! The C library exposes a single function, `max_size_frame`, which computes an
//! upper bound on the encoded size of a FLAC frame. The header declares no
//! namespace-renaming macros, so the final linker symbol is plain
//! `max_size_frame`.
//!
//! Every operand in the C expression has type `uint32_t` (`tflac_u32`), and the
//! comparison sub-expressions (`channels != 2`, `channels == 2`,
//! `bitdepth != 32`) yield an `int` of 0 or 1 that is then converted to
//! `unsigned int` by the usual arithmetic conversions. Unsigned overflow in C is
//! defined to wrap modulo 2^32, so all arithmetic below uses the explicit
//! `wrapping_*` operations to reproduce the original behaviour bit for bit,
//! including any overflow the original code may exhibit.

/// Mirrors `typedef uint32_t tflac_u32;` from `include/lib.h`.
#[allow(non_camel_case_types)]
pub type tflac_u32 = u32;

/// Converts a C comparison result (`0` or `1`) into `tflac_u32`, matching the
/// implicit `int` -> `unsigned int` conversion performed by C.
#[inline]
fn flag(cond: bool) -> tflac_u32 {
    cond as tflac_u32
}

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
/// The grouping and evaluation order of the C source is preserved exactly:
/// `*` is left-associative, so `blocksize * bitdepth * X` is
/// `(blocksize * bitdepth) * X`, and the three products are summed
/// left-to-right together with the trailing `+7` before the single division by
/// `8`.
#[unsafe(no_mangle)]
pub extern "C" fn max_size_frame(
    blocksize: tflac_u32,
    channels: tflac_u32,
    bitdepth: tflac_u32,
) -> tflac_u32 {
    // (blocksize * bitdepth * (channels * (channels != 2)))
    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(flag(channels != 2)));

    // (blocksize * bitdepth * (channels == 2))
    let term2 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(flag(channels == 2));

    // (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2))
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(flag(bitdepth != 32)))
        .wrapping_mul(flag(channels == 2));

    // (term1 + term2 + term3 + +7) / 8
    let bits = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);
    let bytes = bits / 8;

    // 18U + channels + bytes
    18u32.wrapping_add(channels).wrapping_add(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Independent re-derivation of the C expression using 64-bit math with an
    /// explicit truncation to 32 bits at every step, mimicking how a C compiler
    /// evaluates the unsigned expression.
    fn reference(blocksize: u64, channels: u64, bitdepth: u64) -> u32 {
        let m = |a: u64, b: u64| (a.wrapping_mul(b)) & 0xFFFF_FFFF;
        let a = |x: u64, y: u64| (x.wrapping_add(y)) & 0xFFFF_FFFF;

        let ne2 = u64::from(channels != 2);
        let eq2 = u64::from(channels == 2);
        let ne32 = u64::from(bitdepth != 32);

        let t1 = m(m(blocksize, bitdepth), m(channels, ne2));
        let t2 = m(m(blocksize, bitdepth), eq2);
        let t3 = m(m(blocksize, a(bitdepth, ne32)), eq2);

        let bits = a(a(a(t1, t2), t3), 7);
        a(a(18, channels), bits / 8) as u32
    }

    #[test]
    fn matches_reference_on_typical_inputs() {
        for &blocksize in &[0u32, 1, 16, 576, 1152, 4096, 65535] {
            for channels in 0u32..=8 {
                for &bitdepth in &[0u32, 1, 8, 12, 16, 20, 24, 32] {
                    assert_eq!(
                        max_size_frame(blocksize, channels, bitdepth),
                        reference(blocksize.into(), channels.into(), bitdepth.into()),
                        "blocksize={blocksize} channels={channels} bitdepth={bitdepth}"
                    );
                }
            }
        }
    }

    #[test]
    fn matches_reference_on_overflowing_inputs() {
        let extremes = [0u32, 1, 2, 3, 0x0000_FFFF, 0x8000_0000, u32::MAX];
        for &blocksize in &extremes {
            for &channels in &extremes {
                for &bitdepth in &extremes {
                    assert_eq!(
                        max_size_frame(blocksize, channels, bitdepth),
                        reference(blocksize.into(), channels.into(), bitdepth.into()),
                        "blocksize={blocksize} channels={channels} bitdepth={bitdepth}"
                    );
                }
            }
        }
    }

    /// Hand-computed spot checks.
    #[test]
    fn spot_checks() {
        // Stereo, 16-bit, 4096 samples: channels == 2 selects term2 and term3.
        // term1 = 0, term2 = 4096*16*1 = 65536, term3 = 4096*17*1 = 69632.
        // (65536 + 69632 + 7) / 8 = 135175 / 8 = 16896. 18 + 2 + 16896 = 16916.
        assert_eq!(max_size_frame(4096, 2, 16), 16916);

        // Mono, 16-bit, 4096 samples: only term1 contributes.
        // term1 = 4096*16*(1*1) = 65536. (65536 + 7)/8 = 8192. 18 + 1 + 8192 = 8211.
        assert_eq!(max_size_frame(4096, 1, 16), 8211);

        // Zero channels: every term vanishes. (0 + 0 + 0 + 7)/8 = 0. 18 + 0 + 0 = 18.
        assert_eq!(max_size_frame(4096, 0, 16), 18);

        // 32-bit depth keeps the (bitdepth != 32) bump at zero.
        // term2 = 1152*32 = 36864, term3 = 1152*32 = 36864.
        // (36864 + 36864 + 7)/8 = 73735/8 = 9216. 18 + 2 + 9216 = 9236.
        assert_eq!(max_size_frame(1152, 2, 32), 9236);
    }
}
