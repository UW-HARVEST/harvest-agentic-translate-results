//! Byte-swapping helpers, translated from the `static` helpers in
//! `c_src/src/lib.c`.
//!
//! The C code unconditionally byte swaps in `ima_btoh*` (there is no
//! endianness check at all), so the "big endian to host" conversions are plain
//! byte swaps on every target. That behaviour is preserved verbatim.

use crate::{ima_u16_t, ima_u32_t, ima_u64_t};

/// ```c
/// static ima_u16_t ima_bswap16(ima_u16_t v) {
///     return (v << 0x08 & 0xff00u) | (v >> 0x08 & 0x00ffu);
/// }
/// ```
#[inline]
pub fn ima_bswap16(v: ima_u16_t) -> ima_u16_t {
    // The C expression promotes `v` to `int` before shifting, so no information
    // is lost by the left shift; masking then discards the high bits. This is
    // exactly a 16-bit byte swap.
    (((v as u32) << 0x08) & 0xff00) as ima_u16_t | (((v as u32) >> 0x08) & 0x00ff) as ima_u16_t
}

/// ```c
/// static ima_u32_t ima_bswap32(ima_u32_t v) {
///     return (v << 0x18 & 0xff000000ul) | (v << 0x08 & 0x00ff0000ul) |
///            (v >> 0x08 & 0x0000ff00ul) | (v >> 0x18 & 0x000000fful);
/// }
/// ```
#[inline]
pub fn ima_bswap32(v: ima_u32_t) -> ima_u32_t {
    (v.wrapping_shl(0x18) & 0xff00_0000)
        | (v.wrapping_shl(0x08) & 0x00ff_0000)
        | (v.wrapping_shr(0x08) & 0x0000_ff00)
        | (v.wrapping_shr(0x18) & 0x0000_00ff)
}

/// ```c
/// static ima_u64_t ima_bswap64(ima_u64_t v) { ... }
/// ```
#[inline]
pub fn ima_bswap64(v: ima_u64_t) -> ima_u64_t {
    (v.wrapping_shl(0x38) & 0xff00_0000_0000_0000)
        | (v.wrapping_shl(0x28) & 0x00ff_0000_0000_0000)
        | (v.wrapping_shl(0x18) & 0x0000_ff00_0000_0000)
        | (v.wrapping_shl(0x08) & 0x0000_00ff_0000_0000)
        | (v.wrapping_shr(0x08) & 0x0000_0000_ff00_0000)
        | (v.wrapping_shr(0x18) & 0x0000_0000_00ff_0000)
        | (v.wrapping_shr(0x28) & 0x0000_0000_0000_ff00)
        | (v.wrapping_shr(0x38) & 0x0000_0000_0000_00ff)
}

/// `static ima_u16_t ima_btoh16(ima_u16_t v) { return ima_bswap16(v); }`
#[inline]
pub fn ima_btoh16(v: ima_u16_t) -> ima_u16_t {
    ima_bswap16(v)
}

/// `static ima_u32_t ima_btoh32(ima_u32_t v) { return ima_bswap32(v); }`
#[inline]
pub fn ima_btoh32(v: ima_u32_t) -> ima_u32_t {
    ima_bswap32(v)
}

/// `static ima_u64_t ima_btoh64(ima_u64_t v) { return ima_bswap64(v); }`
#[inline]
pub fn ima_btoh64(v: ima_u64_t) -> ima_u64_t {
    ima_bswap64(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_match_builtin() {
        for v in [0u16, 1, 0x0100, 0x1234, 0xffff, 0xff00, 0x00ff] {
            assert_eq!(ima_bswap16(v), v.swap_bytes());
        }
        for v in [0u32, 1, 0x0100_0000, 0x1234_5678, 0xffff_ffff, 0x6666_6163] {
            assert_eq!(ima_bswap32(v), v.swap_bytes());
        }
        for v in [
            0u64,
            1,
            0x0123_4567_89ab_cdef,
            u64::MAX,
            0x8000_0000_0000_0000,
        ] {
            assert_eq!(ima_bswap64(v), v.swap_bytes());
        }
    }
}
