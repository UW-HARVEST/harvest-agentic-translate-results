//! Translation of `common/bits.h`
#![allow(dead_code)]

use super::mem::*;

#[inline(always)]
pub fn ZSTD_countTrailingZeros32_fallback(val: U32) -> u32 {
    static DeBruijnBytePos: [U32; 32] = [
        0, 1, 28, 2, 29, 14, 24, 3, 30, 22, 20, 15, 25, 17, 4, 8, 31, 27, 13, 23, 21, 19, 16, 7,
        26, 12, 18, 6, 11, 5, 10, 9,
    ];
    DeBruijnBytePos[((val & (0i32.wrapping_sub(val as S32)) as U32).wrapping_mul(0x077CB531u32)
        >> 27) as usize]
}

#[inline(always)]
pub fn ZSTD_countTrailingZeros32(val: U32) -> u32 {
    val.trailing_zeros()
}

#[inline(always)]
pub fn ZSTD_countLeadingZeros32_fallback(val: U32) -> u32 {
    /* Faithful transliteration of the C DeBruijn table, including its value at
     * `val == 0`: the smearing leaves `val == 0`, the index is 0, and the C
     * returns `31 - DeBruijnClz[0]` == 31. */
    static DeBruijnClz: [U32; 32] = [
        0, 9, 1, 10, 13, 21, 2, 29, 11, 14, 16, 18, 22, 25, 3, 30, 8, 12, 20, 28, 15, 17, 24, 7,
        19, 27, 23, 6, 26, 5, 4, 31,
    ];
    let mut val = val;
    val |= val >> 1;
    val |= val >> 2;
    val |= val >> 4;
    val |= val >> 8;
    val |= val >> 16;
    31u32.wrapping_sub(DeBruijnClz[(val.wrapping_mul(0x07C4ACDDu32) >> 27) as usize])
}

#[inline(always)]
pub fn ZSTD_countLeadingZeros32(val: U32) -> u32 {
    /* C: `assert(val != 0); return __builtin_clz(val);`
     *
     * `val == 0` IS reachable (see `ZSTD_highbit32` below) and is undefined
     * behaviour in the C, so the reference build's *observed* result is the
     * contract we must match. gcc at the -O0 this library is built with emits
     *
     *     mov %edi,-0x4(%rbp) ; bsr -0x4(%rbp),%eax ; xor $0x1f,%eax
     *
     * and `bsr` with a zero source leaves its destination register untouched.
     * The only caller in the whole library is `ZSTD_highbit32`, which at -O0
     * always does `mov -0x4(%rbp),%eax; mov %eax,%edi; call`, so `%eax` holds
     * `val` (== 0) when `bsr` runs: the result is `0 ^ 31` == 31.
     *
     * 31 is also exactly what `ZSTD_countLeadingZeros32_fallback(0)` computes,
     * so the two C implementations agree and 31 is the value to reproduce. */
    if val == 0 {
        31
    } else {
        val.leading_zeros()
    }
}

#[inline(always)]
pub fn ZSTD_countTrailingZeros64(val: U64) -> u32 {
    val.trailing_zeros()
}

#[inline(always)]
pub fn ZSTD_countLeadingZeros64(val: U64) -> u32 {
    /* Same reasoning as `ZSTD_countLeadingZeros32`: gcc emits
     * `bsr -0x8(%rbp),%rax ; xor $0x3f,%rax`, and with a zero source `%rax`
     * still holds `val` (== 0), giving `0 ^ 63` == 63. Only reachable from
     * `ZSTD_NbCommonBytes`'s big-endian branch, which this build never takes. */
    if val == 0 {
        63
    } else {
        val.leading_zeros()
    }
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
    /* C: `31 - ZSTD_countLeadingZeros32(val)` with `assert(val != 0)`.
     * Several callers (e.g. `FSE_optimalTableLog_internal` with srcSize == 1)
     * do reach it with val == 0, where the C computes `31 - 32` in `unsigned`
     * and wraps to 0xFFFFFFFF. Spell the wrap out so this can never panic. */
    31u32.wrapping_sub(ZSTD_countLeadingZeros32(val))
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
