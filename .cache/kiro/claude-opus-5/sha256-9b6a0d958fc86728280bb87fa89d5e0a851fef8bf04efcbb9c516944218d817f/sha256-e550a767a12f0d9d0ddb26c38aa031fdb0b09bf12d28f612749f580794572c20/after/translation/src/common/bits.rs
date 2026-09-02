//! Translation of `common/bits.h`.
#![allow(dead_code)]
#![allow(non_snake_case)]

use super::mem::*;

#[inline(always)]
pub fn ZSTD_countTrailingZeros32_fallback(val: U32) -> u32 {
    const DE_BRUIJN_BYTE_POS: [U32; 32] = [
        0, 1, 28, 2, 29, 14, 24, 3, 30, 22, 20, 15, 25, 17, 4, 8, 31, 27, 13, 23, 21, 19, 16, 7,
        26, 12, 18, 6, 11, 5, 10, 9,
    ];
    let v = (val & (val as S32).wrapping_neg() as U32).wrapping_mul(0x077CB531u32);
    DE_BRUIJN_BYTE_POS[(v >> 27) as usize]
}

#[inline(always)]
pub fn ZSTD_countTrailingZeros32(val: U32) -> u32 {
    val.trailing_zeros()
}

#[inline(always)]
pub fn ZSTD_countLeadingZeros32_fallback(mut val: U32) -> u32 {
    const DE_BRUIJN_CLZ: [U32; 32] = [
        0, 9, 1, 10, 13, 21, 2, 29, 11, 14, 16, 18, 22, 25, 3, 30, 8, 12, 20, 28, 15, 17, 24, 7,
        19, 27, 23, 6, 26, 5, 4, 31,
    ];
    val |= val >> 1;
    val |= val >> 2;
    val |= val >> 4;
    val |= val >> 8;
    val |= val >> 16;
    31 - DE_BRUIJN_CLZ[(val.wrapping_mul(0x07C4ACDDu32) >> 27) as usize]
}

#[inline(always)]
pub fn ZSTD_countLeadingZeros32(val: U32) -> u32 {
    val.leading_zeros()
}

#[inline(always)]
pub fn ZSTD_countTrailingZeros64(val: U64) -> u32 {
    val.trailing_zeros()
}

#[inline(always)]
pub fn ZSTD_countLeadingZeros64(val: U64) -> u32 {
    val.leading_zeros()
}

#[inline(always)]
pub fn ZSTD_NbCommonBytes(val: size_t) -> u32 {
    if MEM_isLittleEndian() != 0 {
        if MEM_64bits() != 0 {
            ZSTD_countTrailingZeros64(val as U64) >> 3
        } else {
            ZSTD_countTrailingZeros32(val as U32) >> 3
        }
    } else if MEM_64bits() != 0 {
        ZSTD_countLeadingZeros64(val as U64) >> 3
    } else {
        ZSTD_countLeadingZeros32(val as U32) >> 3
    }
}

#[inline(always)]
pub fn ZSTD_highbit32(val: U32) -> u32 {
    31 - ZSTD_countLeadingZeros32(val)
}

#[inline(always)]
pub fn ZSTD_rotateRight_U64(value: U64, count: U32) -> U64 {
    let count = count & 0x3F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x3F))
}

#[inline(always)]
pub fn ZSTD_rotateRight_U32(value: U32, count: U32) -> U32 {
    let count = count & 0x1F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x1F))
}

#[inline(always)]
pub fn ZSTD_rotateRight_U16(value: U16, count: U32) -> U16 {
    let count = count & 0x0F;
    (value >> count) | ((value as u32) << ((0u32.wrapping_sub(count)) & 0x0F)) as U16
}
