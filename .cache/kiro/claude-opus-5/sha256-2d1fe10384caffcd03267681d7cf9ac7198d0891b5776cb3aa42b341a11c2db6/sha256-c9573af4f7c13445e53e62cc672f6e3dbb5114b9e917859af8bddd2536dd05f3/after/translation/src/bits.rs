//! Translation of `common/bits.h`.
#![allow(dead_code)]

use crate::mem::*;

/// `ZSTD_countTrailingZeros32()` — GCC path uses `__builtin_ctz`.
#[inline(always)]
pub fn zstd_count_trailing_zeros32(val: U32) -> u32 {
    debug_assert!(val != 0);
    val.trailing_zeros()
}

/// `ZSTD_countLeadingZeros32()`
#[inline(always)]
pub fn zstd_count_leading_zeros32(val: U32) -> u32 {
    debug_assert!(val != 0);
    val.leading_zeros()
}

/// `ZSTD_countTrailingZeros64()`
#[inline(always)]
pub fn zstd_count_trailing_zeros64(val: U64) -> u32 {
    debug_assert!(val != 0);
    val.trailing_zeros()
}

/// `ZSTD_countLeadingZeros64()`
#[inline(always)]
pub fn zstd_count_leading_zeros64(val: U64) -> u32 {
    debug_assert!(val != 0);
    val.leading_zeros()
}

/// `ZSTD_NbCommonBytes()`
#[inline(always)]
pub fn zstd_nb_common_bytes(val: usize) -> u32 {
    if mem_is_little_endian() {
        if mem_64bits() {
            zstd_count_trailing_zeros64(val as U64) >> 3
        } else {
            zstd_count_trailing_zeros32(val as U32) >> 3
        }
    } else if mem_64bits() {
        zstd_count_leading_zeros64(val as U64) >> 3
    } else {
        zstd_count_leading_zeros32(val as U32) >> 3
    }
}

/// `ZSTD_highbit32()`
#[inline(always)]
pub fn zstd_highbit32(val: U32) -> u32 {
    debug_assert!(val != 0);
    31 - zstd_count_leading_zeros32(val)
}

/// `ZSTD_rotateRight_U64()`
#[inline(always)]
pub fn zstd_rotate_right_u64(value: U64, count: u32) -> U64 {
    debug_assert!(count < 64);
    let count = count & 0x3F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x3F))
}

/// `ZSTD_rotateRight_U32()`
#[inline(always)]
pub fn zstd_rotate_right_u32(value: U32, count: u32) -> U32 {
    debug_assert!(count < 32);
    let count = count & 0x1F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x1F))
}

/// `ZSTD_rotateRight_U16()`
#[inline(always)]
pub fn zstd_rotate_right_u16(value: U16, count: u32) -> U16 {
    debug_assert!(count < 16);
    let count = count & 0x0F;
    (value >> count) | ((value as u32) << ((0u32.wrapping_sub(count)) & 0x0F)) as U16
}
