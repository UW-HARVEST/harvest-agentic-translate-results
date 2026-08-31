//! Translation of `common/xxhash.h` / `common/xxhash.c` (XXH32 + XXH64 only;
//! XXH_NO_XXH3 is defined by zstd's xxhash.h).
//!
//! Exported symbols are namespaced with `ZSTD_` (XXH_NAMESPACE=ZSTD_).
#![allow(dead_code)]

use core::ffi::{c_uint, c_void};

use crate::cmem::*;

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 8;
pub const XXH_VERSION_RELEASE: u32 = 2;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

pub type XXH_errorcode = c_uint;
pub const XXH_OK: XXH_errorcode = 0;
pub const XXH_ERROR: XXH_errorcode = 1;

const XXH_PRIME32_1: u32 = 0x9E3779B1;
const XXH_PRIME32_2: u32 = 0x85EBCA77;
const XXH_PRIME32_3: u32 = 0xC2B2AE3D;
const XXH_PRIME32_4: u32 = 0x27D4EB2F;
const XXH_PRIME32_5: u32 = 0x165667B1;

const XXH_PRIME64_1: u64 = 0x9E3779B185EBCA87;
const XXH_PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
const XXH_PRIME64_3: u64 = 0x165667B19E3779F9;
const XXH_PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
const XXH_PRIME64_5: u64 = 0x27D4EB2F165667C5;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct XXH32_state_t {
    pub total_len_32: u32,
    pub large_len: u32,
    pub v: [u32; 4],
    pub mem32: [u32; 4],
    pub memsize: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct XXH64_state_t {
    pub total_len: u64,
    pub v: [u64; 4],
    pub mem64: [u64; 4],
    pub memsize: u32,
    pub reserved32: u32,
    pub reserved64: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct XXH32_canonical_t {
    pub digest: [u8; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct XXH64_canonical_t {
    pub digest: [u8; 8],
}

/* ==================== XXH32 ==================== */

#[inline(always)]
fn XXH32_round(mut acc: u32, input: u32) -> u32 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME32_2));
    acc = acc.rotate_left(13);
    acc = acc.wrapping_mul(XXH_PRIME32_1);
    acc
}

#[inline(always)]
fn XXH32_avalanche(mut hash: u32) -> u32 {
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(XXH_PRIME32_2);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(XXH_PRIME32_3);
    hash ^= hash >> 16;
    hash
}

#[inline(always)]
unsafe fn XXH32_finalize(mut hash: u32, mut ptr: *const u8, mut len: usize) -> u32 {
    len &= 15;
    while len >= 4 {
        hash = hash.wrapping_add(
            u32::from_le((ptr as *const u32).read_unaligned()).wrapping_mul(XXH_PRIME32_3),
        );
        ptr = ptr.add(4);
        hash = hash.rotate_left(17).wrapping_mul(XXH_PRIME32_4);
        len -= 4;
    }
    while len > 0 {
        hash = hash.wrapping_add((*ptr as u32).wrapping_mul(XXH_PRIME32_5));
        ptr = ptr.add(1);
        hash = hash.rotate_left(11).wrapping_mul(XXH_PRIME32_1);
        len -= 1;
    }
    XXH32_avalanche(hash)
}

unsafe fn XXH32_endian_align(mut input: *const u8, len: usize, seed: u32) -> u32 {
    let mut h32: u32;

    if len >= 16 {
        let bEnd = input.add(len);
        let limit = bEnd.sub(15);
        let mut v1 = seed
            .wrapping_add(XXH_PRIME32_1)
            .wrapping_add(XXH_PRIME32_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME32_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME32_1);

        loop {
            v1 = XXH32_round(v1, u32::from_le((input as *const u32).read_unaligned()));
            input = input.add(4);
            v2 = XXH32_round(v2, u32::from_le((input as *const u32).read_unaligned()));
            input = input.add(4);
            v3 = XXH32_round(v3, u32::from_le((input as *const u32).read_unaligned()));
            input = input.add(4);
            v4 = XXH32_round(v4, u32::from_le((input as *const u32).read_unaligned()));
            input = input.add(4);
            if input >= limit {
                break;
            }
        }

        h32 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
    } else {
        h32 = seed.wrapping_add(XXH_PRIME32_5);
    }

    h32 = h32.wrapping_add(len as u32);

    XXH32_finalize(h32, input, len & 15)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32(input: *const c_void, len: usize, seed: u32) -> u32 {
    XXH32_endian_align(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_createState() -> *mut XXH32_state_t {
    malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_freeState(statePtr: *mut XXH32_state_t) -> XXH_errorcode {
    free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_copyState(
    dstState: *mut XXH32_state_t,
    srcState: *const XXH32_state_t,
) {
    ZSTD_memcpy(
        dstState as *mut c_void,
        srcState as *const c_void,
        core::mem::size_of::<XXH32_state_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_reset(
    statePtr: *mut XXH32_state_t,
    seed: u32,
) -> XXH_errorcode {
    ZSTD_memset(
        statePtr as *mut c_void,
        0,
        core::mem::size_of::<XXH32_state_t>(),
    );
    (*statePtr).v[0] = seed
        .wrapping_add(XXH_PRIME32_1)
        .wrapping_add(XXH_PRIME32_2);
    (*statePtr).v[1] = seed.wrapping_add(XXH_PRIME32_2);
    (*statePtr).v[2] = seed;
    (*statePtr).v[3] = seed.wrapping_sub(XXH_PRIME32_1);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_update(
    state: *mut XXH32_state_t,
    input: *const c_void,
    len: usize,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_OK;
    }
    let mut p = input as *const u8;
    let bEnd = p.add(len);

    (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
    (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

    if ((*state).memsize as usize) + len < 16 {
        ZSTD_memcpy(
            ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            len,
        );
        (*state).memsize += len as u32;
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        ZSTD_memcpy(
            ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            16 - (*state).memsize as usize,
        );
        {
            let mut p32 = (*state).mem32.as_ptr();
            (*state).v[0] = XXH32_round((*state).v[0], u32::from_le(p32.read_unaligned()));
            p32 = p32.add(1);
            (*state).v[1] = XXH32_round((*state).v[1], u32::from_le(p32.read_unaligned()));
            p32 = p32.add(1);
            (*state).v[2] = XXH32_round((*state).v[2], u32::from_le(p32.read_unaligned()));
            p32 = p32.add(1);
            (*state).v[3] = XXH32_round((*state).v[3], u32::from_le(p32.read_unaligned()));
        }
        p = p.add(16 - (*state).memsize as usize);
        (*state).memsize = 0;
    }

    if p as usize <= (bEnd as usize) - 16 {
        let limit = bEnd.sub(16);
        loop {
            (*state).v[0] = XXH32_round((*state).v[0], u32::from_le((p as *const u32).read_unaligned()));
            p = p.add(4);
            (*state).v[1] = XXH32_round((*state).v[1], u32::from_le((p as *const u32).read_unaligned()));
            p = p.add(4);
            (*state).v[2] = XXH32_round((*state).v[2], u32::from_le((p as *const u32).read_unaligned()));
            p = p.add(4);
            (*state).v[3] = XXH32_round((*state).v[3], u32::from_le((p as *const u32).read_unaligned()));
            p = p.add(4);
            if p > limit {
                break;
            }
        }
    }

    if p < bEnd {
        ZSTD_memcpy(
            (*state).mem32.as_mut_ptr() as *mut c_void,
            p as *const c_void,
            bEnd as usize - p as usize,
        );
        (*state).memsize = (bEnd as usize - p as usize) as u32;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_digest(state: *const XXH32_state_t) -> u32 {
    let mut h32: u32;
    if (*state).large_len != 0 {
        h32 = (*state).v[0]
            .rotate_left(1)
            .wrapping_add((*state).v[1].rotate_left(7))
            .wrapping_add((*state).v[2].rotate_left(12))
            .wrapping_add((*state).v[3].rotate_left(18));
    } else {
        h32 = (*state).v[2].wrapping_add(XXH_PRIME32_5);
    }
    h32 = h32.wrapping_add((*state).total_len_32);
    XXH32_finalize(
        h32,
        (*state).mem32.as_ptr() as *const u8,
        (*state).memsize as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(dst: *mut XXH32_canonical_t, hash: u32) {
    let h = hash.to_be();
    ZSTD_memcpy(dst as *mut c_void, &h as *const u32 as *const c_void, 4);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> u32 {
    MEM_readBE32(src as *const c_void)
}

/* ==================== XXH64 ==================== */

#[inline(always)]
fn XXH64_round(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc = acc.wrapping_mul(XXH_PRIME64_1);
    acc
}

#[inline(always)]
fn XXH64_mergeRound(mut acc: u64, val: u64) -> u64 {
    let val = XXH64_round(0, val);
    acc ^= val;
    acc = acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
    acc
}

#[inline(always)]
fn XXH64_avalanche(mut hash: u64) -> u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(XXH_PRIME64_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(XXH_PRIME64_3);
    hash ^= hash >> 32;
    hash
}

#[inline(always)]
unsafe fn XXH64_finalize(mut hash: u64, mut ptr: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = XXH64_round(0, u64::from_le((ptr as *const u64).read_unaligned()));
        ptr = ptr.add(8);
        hash ^= k1;
        hash = hash
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        hash ^= (u32::from_le((ptr as *const u32).read_unaligned()) as u64)
            .wrapping_mul(XXH_PRIME64_1);
        ptr = ptr.add(4);
        hash = hash
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        hash ^= (*ptr as u64).wrapping_mul(XXH_PRIME64_5);
        ptr = ptr.add(1);
        hash = hash.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    XXH64_avalanche(hash)
}

unsafe fn XXH64_endian_align(mut input: *const u8, len: usize, seed: u64) -> u64 {
    let mut h64: u64;

    if len >= 32 {
        let bEnd = input.add(len);
        let limit = bEnd.sub(31);
        let mut v1 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = XXH64_round(v1, u64::from_le((input as *const u64).read_unaligned()));
            input = input.add(8);
            v2 = XXH64_round(v2, u64::from_le((input as *const u64).read_unaligned()));
            input = input.add(8);
            v3 = XXH64_round(v3, u64::from_le((input as *const u64).read_unaligned()));
            input = input.add(8);
            v4 = XXH64_round(v4, u64::from_le((input as *const u64).read_unaligned()));
            input = input.add(8);
            if input >= limit {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = XXH64_mergeRound(h64, v1);
        h64 = XXH64_mergeRound(h64, v2);
        h64 = XXH64_mergeRound(h64, v3);
        h64 = XXH64_mergeRound(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);

    XXH64_finalize(h64, input, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64(input: *const c_void, len: usize, seed: u64) -> u64 {
    XXH64_endian_align(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_createState() -> *mut XXH64_state_t {
    malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_freeState(statePtr: *mut XXH64_state_t) -> XXH_errorcode {
    free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_copyState(
    dstState: *mut XXH64_state_t,
    srcState: *const XXH64_state_t,
) {
    ZSTD_memcpy(
        dstState as *mut c_void,
        srcState as *const c_void,
        core::mem::size_of::<XXH64_state_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_reset(
    statePtr: *mut XXH64_state_t,
    seed: u64,
) -> XXH_errorcode {
    ZSTD_memset(
        statePtr as *mut c_void,
        0,
        core::mem::size_of::<XXH64_state_t>(),
    );
    (*statePtr).v[0] = seed
        .wrapping_add(XXH_PRIME64_1)
        .wrapping_add(XXH_PRIME64_2);
    (*statePtr).v[1] = seed.wrapping_add(XXH_PRIME64_2);
    (*statePtr).v[2] = seed;
    (*statePtr).v[3] = seed.wrapping_sub(XXH_PRIME64_1);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_update(
    state: *mut XXH64_state_t,
    input: *const c_void,
    len: usize,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_OK;
    }
    let mut p = input as *const u8;
    let bEnd = p.add(len);

    (*state).total_len = (*state).total_len.wrapping_add(len as u64);

    if ((*state).memsize as usize) + len < 32 {
        ZSTD_memcpy(
            ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            len,
        );
        (*state).memsize += len as u32;
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        ZSTD_memcpy(
            ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            32 - (*state).memsize as usize,
        );
        (*state).v[0] = XXH64_round(
            (*state).v[0],
            u64::from_le((*state).mem64.as_ptr().add(0).read_unaligned()),
        );
        (*state).v[1] = XXH64_round(
            (*state).v[1],
            u64::from_le((*state).mem64.as_ptr().add(1).read_unaligned()),
        );
        (*state).v[2] = XXH64_round(
            (*state).v[2],
            u64::from_le((*state).mem64.as_ptr().add(2).read_unaligned()),
        );
        (*state).v[3] = XXH64_round(
            (*state).v[3],
            u64::from_le((*state).mem64.as_ptr().add(3).read_unaligned()),
        );
        p = p.add(32 - (*state).memsize as usize);
        (*state).memsize = 0;
    }

    if (p as usize) + 32 <= bEnd as usize {
        let limit = bEnd.sub(32);
        loop {
            (*state).v[0] = XXH64_round((*state).v[0], u64::from_le((p as *const u64).read_unaligned()));
            p = p.add(8);
            (*state).v[1] = XXH64_round((*state).v[1], u64::from_le((p as *const u64).read_unaligned()));
            p = p.add(8);
            (*state).v[2] = XXH64_round((*state).v[2], u64::from_le((p as *const u64).read_unaligned()));
            p = p.add(8);
            (*state).v[3] = XXH64_round((*state).v[3], u64::from_le((p as *const u64).read_unaligned()));
            p = p.add(8);
            if p > limit {
                break;
            }
        }
    }

    if p < bEnd {
        ZSTD_memcpy(
            (*state).mem64.as_mut_ptr() as *mut c_void,
            p as *const c_void,
            bEnd as usize - p as usize,
        );
        (*state).memsize = (bEnd as usize - p as usize) as u32;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(state: *const XXH64_state_t) -> u64 {
    let mut h64: u64;
    if (*state).total_len >= 32 {
        h64 = (*state).v[0]
            .rotate_left(1)
            .wrapping_add((*state).v[1].rotate_left(7))
            .wrapping_add((*state).v[2].rotate_left(12))
            .wrapping_add((*state).v[3].rotate_left(18));
        h64 = XXH64_mergeRound(h64, (*state).v[0]);
        h64 = XXH64_mergeRound(h64, (*state).v[1]);
        h64 = XXH64_mergeRound(h64, (*state).v[2]);
        h64 = XXH64_mergeRound(h64, (*state).v[3]);
    } else {
        h64 = (*state).v[2].wrapping_add(XXH_PRIME64_5);
    }
    h64 = h64.wrapping_add((*state).total_len);
    XXH64_finalize(
        h64,
        (*state).mem64.as_ptr() as *const u8,
        (*state).total_len as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, hash: u64) {
    let h = hash.to_be();
    ZSTD_memcpy(dst as *mut c_void, &h as *const u64 as *const c_void, 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    MEM_readBE64(src as *const c_void)
}

/* Convenience aliases used internally by the rest of the crate. */
#[inline(always)]
pub unsafe fn XXH32(input: *const c_void, len: usize, seed: u32) -> u32 {
    ZSTD_XXH32(input, len, seed)
}
#[inline(always)]
pub unsafe fn XXH64(input: *const c_void, len: usize, seed: u64) -> u64 {
    ZSTD_XXH64(input, len, seed)
}
#[inline(always)]
pub unsafe fn XXH64_reset(s: *mut XXH64_state_t, seed: u64) -> XXH_errorcode {
    ZSTD_XXH64_reset(s, seed)
}
#[inline(always)]
pub unsafe fn XXH64_update(s: *mut XXH64_state_t, input: *const c_void, len: usize) -> XXH_errorcode {
    ZSTD_XXH64_update(s, input, len)
}
#[inline(always)]
pub unsafe fn XXH64_digest(s: *const XXH64_state_t) -> u64 {
    ZSTD_XXH64_digest(s)
}
