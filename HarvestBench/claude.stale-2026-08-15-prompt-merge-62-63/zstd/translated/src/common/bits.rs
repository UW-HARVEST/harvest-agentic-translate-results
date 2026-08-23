//! Translation of common/bits.h — bit counting helpers.
#![allow(dead_code)]
use super::mem::*;

#[inline]
pub fn count_trailing_zeros32(val: U32) -> u32 {
    debug_assert!(val != 0);
    val.trailing_zeros()
}
#[inline]
pub fn count_leading_zeros32(val: U32) -> u32 {
    debug_assert!(val != 0);
    val.leading_zeros()
}
#[inline]
pub fn count_trailing_zeros64(val: U64) -> u32 {
    debug_assert!(val != 0);
    val.trailing_zeros()
}
#[inline]
pub fn count_leading_zeros64(val: U64) -> u32 {
    debug_assert!(val != 0);
    val.leading_zeros()
}
#[inline]
pub fn nb_common_bytes(val: usize) -> u32 {
    if mem_is_little_endian() != 0 {
        if mem_64bits() != 0 {
            count_trailing_zeros64(val as U64) >> 3
        } else {
            count_trailing_zeros32(val as U32) >> 3
        }
    } else if mem_64bits() != 0 {
        count_leading_zeros64(val as U64) >> 3
    } else {
        count_leading_zeros32(val as U32) >> 3
    }
}
#[inline]
pub fn highbit32(val: U32) -> u32 {
    debug_assert!(val != 0);
    31 - count_leading_zeros32(val)
}
#[inline]
pub fn rotate_right_u64(value: U64, count: U32) -> U64 {
    let count = count & 0x3F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x3F))
}
#[inline]
pub fn rotate_right_u32(value: U32, count: U32) -> U32 {
    let count = count & 0x1F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x1F))
}
#[inline]
pub fn rotate_right_u16(value: U16, count: U32) -> U16 {
    let count = count & 0x0F;
    (value >> count) | (value << ((0u32.wrapping_sub(count)) & 0x0F))
}
