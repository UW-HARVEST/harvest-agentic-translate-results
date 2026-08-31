//! Translation of `common/bits.h`
#![allow(dead_code)]

use crate::cmem::*;

#[inline(always)]
pub fn ZSTD_countTrailingZeros32(val: U32) -> u32 {
    val.trailing_zeros()
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
pub fn ZSTD_NbCommonBytes(val: usize) -> u32 {
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
