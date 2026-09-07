//! Hashing: `stbds_rand_seed`, `stbds_hash_string`, `stbds_hash_bytes`
//! (and the internal `stbds_siphash_bytes`).

use core::ffi::{c_char, c_void};

use crate::STBDS_SIZE_T_BITS;

/// `static size_t stbds_hash_seed=0x31415926;`
pub(crate) static mut stbds_hash_seed: usize = 0x31415926;

/// `STBDS_ROTATE_LEFT(val, n)`
#[inline(always)]
pub(crate) fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ROTATE_RIGHT(val, n)`
#[inline(always)]
pub(crate) fn rotr(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

/// ```c
/// void stbds_rand_seed(size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

/// ```c
/// size_t stbds_hash_string(char *str, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str_ as *const u8;
    while *p != 0 {
        // hash = STBDS_ROTATE_LEFT(hash, 9) + (unsigned char) *str++;
        hash = rotl(hash, 9).wrapping_add(*p as usize);
        p = p.wrapping_add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotr(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotr(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotl($v1, 13);
        $v1 ^= $v0;
        $v0 = rotl($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotl($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotl($v1, 17);
        $v1 ^= $v2;
        $v2 = rotl($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotl($v3, 21);
        $v3 ^= $v0;
    }};
}

/// ```c
/// static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)
/// ```
///
/// The byte-gathering expressions in the C source are `int`-typed and
/// deliberately reproduced here, including the sign extension that happens
/// when `d[3] << 24` overflows `int` and the result is widened to `size_t`.
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let mut j: usize;
    let (mut v0, mut v1, mut v2, mut v3): (usize, usize, usize, usize);
    let mut data: usize;

    v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    i = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        let lo: i32 = (*d.wrapping_add(0) as i32)
            | ((*d.wrapping_add(1) as i32) << 8)
            | ((*d.wrapping_add(2) as i32) << 16)
            | ((*d.wrapping_add(3) as i32).wrapping_shl(24));
        data = lo as isize as usize;
        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: i32 = (*d.wrapping_add(4) as i32)
            | ((*d.wrapping_add(5) as i32) << 8)
            | ((*d.wrapping_add(6) as i32) << 16)
            | ((*d.wrapping_add(7) as i32).wrapping_shl(24));
        data |= ((hi as isize as usize) << 16) << 16;

        v3 ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            sipround!(v0, v1, v2, v3);
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.wrapping_add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) with fallthrough from `rem` down to 1
    let rem = len.wrapping_sub(i);
    if rem >= 7 {
        data |= ((*d.wrapping_add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.wrapping_add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.wrapping_add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        data |= ((*d.wrapping_add(3) as i32).wrapping_shl(24)) as isize as usize;
    }
    if rem >= 3 {
        data |= (*d.wrapping_add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*d.wrapping_add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *d.wrapping_add(0) as usize;
    }

    v3 ^= data;
    j = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        sipround!(v0, v1, v2, v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    j = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        sipround!(v0, v1, v2, v3);
        j += 1;
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// ```c
/// size_t stbds_hash_bytes(void *p, size_t len, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}
