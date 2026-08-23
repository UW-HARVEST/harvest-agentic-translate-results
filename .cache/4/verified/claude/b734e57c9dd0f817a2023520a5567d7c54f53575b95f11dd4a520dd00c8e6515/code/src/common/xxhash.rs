//! Translation of `common/xxhash.c` / `common/xxhash.h` (XXH32 + XXH64).
//! Built with `XXH_NAMESPACE=ZSTD_`, so every public symbol is prefixed with
//! `ZSTD_`.
#![allow(dead_code)]

use crate::libc::{free, malloc, ZSTD_memcpy, ZSTD_memset};
use core::ffi::c_void;

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 8;
pub const XXH_VERSION_RELEASE: u32 = 2;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

pub type XXH_errorcode = i32;
pub const XXH_OK: XXH_errorcode = 0;
pub const XXH_ERROR: XXH_errorcode = 1;

pub type XXH32_hash_t = u32;
pub type XXH64_hash_t = u64;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct XXH32_state_t {
    pub total_len_32: XXH32_hash_t,
    pub large_len: XXH32_hash_t,
    pub v: [XXH32_hash_t; 4],
    pub mem32: [XXH32_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved: XXH32_hash_t,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct XXH64_state_t {
    pub total_len: XXH64_hash_t,
    pub v: [XXH64_hash_t; 4],
    pub mem64: [XXH64_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved32: XXH32_hash_t,
    pub reserved64: XXH64_hash_t,
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

/* ---- primitives ---- */

#[inline(always)]
fn XXH_rotl32(x: u32, r: u32) -> u32 {
    x.rotate_left(r)
}

#[inline(always)]
fn XXH_rotl64(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

#[inline(always)]
unsafe fn XXH_readLE32(ptr: *const u8) -> u32 {
    let v = (ptr as *const u32).read_unaligned();
    if cfg!(target_endian = "little") {
        v
    } else {
        v.swap_bytes()
    }
}

#[inline(always)]
unsafe fn XXH_readBE32(ptr: *const u8) -> u32 {
    let v = (ptr as *const u32).read_unaligned();
    if cfg!(target_endian = "little") {
        v.swap_bytes()
    } else {
        v
    }
}

#[inline(always)]
unsafe fn XXH_readLE64(ptr: *const u8) -> u64 {
    let v = (ptr as *const u64).read_unaligned();
    if cfg!(target_endian = "little") {
        v
    } else {
        v.swap_bytes()
    }
}

#[inline(always)]
unsafe fn XXH_readBE64(ptr: *const u8) -> u64 {
    let v = (ptr as *const u64).read_unaligned();
    if cfg!(target_endian = "little") {
        v.swap_bytes()
    } else {
        v
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_XXH_versionNumber() -> u32 {
    XXH_VERSION_NUMBER
}

/* ---- XXH32 ---- */

const XXH_PRIME32_1: u32 = 0x9E3779B1;
const XXH_PRIME32_2: u32 = 0x85EBCA77;
const XXH_PRIME32_3: u32 = 0xC2B2AE3D;
const XXH_PRIME32_4: u32 = 0x27D4EB2F;
const XXH_PRIME32_5: u32 = 0x165667B1;

#[inline(always)]
fn XXH32_round(mut acc: u32, input: u32) -> u32 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME32_2));
    acc = XXH_rotl32(acc, 13);
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

unsafe fn XXH32_finalize(mut hash: u32, mut ptr: *const u8, mut len: usize) -> u32 {
    len &= 15;
    while len >= 4 {
        hash = hash.wrapping_add(XXH_readLE32(ptr).wrapping_mul(XXH_PRIME32_3));
        ptr = ptr.add(4);
        hash = XXH_rotl32(hash, 17).wrapping_mul(XXH_PRIME32_4);
        len -= 4;
    }
    while len > 0 {
        hash = hash.wrapping_add((*ptr as u32).wrapping_mul(XXH_PRIME32_5));
        ptr = ptr.add(1);
        hash = XXH_rotl32(hash, 11).wrapping_mul(XXH_PRIME32_1);
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
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME32_1);
        loop {
            v1 = XXH32_round(v1, XXH_readLE32(input));
            input = input.add(4);
            v2 = XXH32_round(v2, XXH_readLE32(input));
            input = input.add(4);
            v3 = XXH32_round(v3, XXH_readLE32(input));
            input = input.add(4);
            v4 = XXH32_round(v4, XXH_readLE32(input));
            input = input.add(4);
            if input >= limit {
                break;
            }
        }
        h32 = XXH_rotl32(v1, 1)
            .wrapping_add(XXH_rotl32(v2, 7))
            .wrapping_add(XXH_rotl32(v3, 12))
            .wrapping_add(XXH_rotl32(v4, 18));
    } else {
        h32 = seed.wrapping_add(XXH_PRIME32_5);
    }
    h32 = h32.wrapping_add(len as u32);
    XXH32_finalize(h32, input, len & 15)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32(
    input: *const c_void,
    len: usize,
    seed: XXH32_hash_t,
) -> XXH32_hash_t {
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
    seed: XXH32_hash_t,
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
    (*statePtr).v[2] = seed.wrapping_add(0);
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
    {
        let mut p = input as *const u8;
        let bEnd = p.add(len);
        (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as XXH32_hash_t);
        (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);
        if (*state).memsize as usize + len < 16 {
            ZSTD_memcpy(
                ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                len,
            );
            (*state).memsize += len as XXH32_hash_t;
            return XXH_OK;
        }
        if (*state).memsize != 0 {
            ZSTD_memcpy(
                ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                16 - (*state).memsize as usize,
            );
            {
                let mut p32 = (*state).mem32.as_ptr() as *const u8;
                (*state).v[0] = XXH32_round((*state).v[0], XXH_readLE32(p32));
                p32 = p32.add(4);
                (*state).v[1] = XXH32_round((*state).v[1], XXH_readLE32(p32));
                p32 = p32.add(4);
                (*state).v[2] = XXH32_round((*state).v[2], XXH_readLE32(p32));
                p32 = p32.add(4);
                (*state).v[3] = XXH32_round((*state).v[3], XXH_readLE32(p32));
            }
            p = p.add(16 - (*state).memsize as usize);
            (*state).memsize = 0;
        }
        if p <= bEnd.wrapping_sub(16) {
            let limit = bEnd.wrapping_sub(16);
            loop {
                (*state).v[0] = XXH32_round((*state).v[0], XXH_readLE32(p));
                p = p.add(4);
                (*state).v[1] = XXH32_round((*state).v[1], XXH_readLE32(p));
                p = p.add(4);
                (*state).v[2] = XXH32_round((*state).v[2], XXH_readLE32(p));
                p = p.add(4);
                (*state).v[3] = XXH32_round((*state).v[3], XXH_readLE32(p));
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
                bEnd.offset_from(p) as usize,
            );
            (*state).memsize = bEnd.offset_from(p) as u32;
        }
    }
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_digest(state: *const XXH32_state_t) -> XXH32_hash_t {
    let mut h32: u32;
    if (*state).large_len != 0 {
        h32 = XXH_rotl32((*state).v[0], 1)
            .wrapping_add(XXH_rotl32((*state).v[1], 7))
            .wrapping_add(XXH_rotl32((*state).v[2], 12))
            .wrapping_add(XXH_rotl32((*state).v[3], 18));
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
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(
    dst: *mut XXH32_canonical_t,
    hash: XXH32_hash_t,
) {
    let h = if cfg!(target_endian = "little") {
        hash.swap_bytes()
    } else {
        hash
    };
    ZSTD_memcpy(
        dst as *mut c_void,
        core::ptr::addr_of!(h) as *const c_void,
        core::mem::size_of::<XXH32_canonical_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(
    src: *const XXH32_canonical_t,
) -> XXH32_hash_t {
    XXH_readBE32(src as *const u8)
}

/* ---- XXH64 ---- */

const XXH_PRIME64_1: u64 = 0x9E3779B185EBCA87;
const XXH_PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
const XXH_PRIME64_3: u64 = 0x165667B19E3779F9;
const XXH_PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
const XXH_PRIME64_5: u64 = 0x27D4EB2F165667C5;

#[inline(always)]
fn XXH64_round(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = XXH_rotl64(acc, 31);
    acc = acc.wrapping_mul(XXH_PRIME64_1);
    acc
}

#[inline(always)]
fn XXH64_mergeRound(mut acc: u64, mut val: u64) -> u64 {
    val = XXH64_round(0, val);
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

unsafe fn XXH64_finalize(mut hash: u64, mut ptr: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = XXH64_round(0, XXH_readLE64(ptr));
        ptr = ptr.add(8);
        hash ^= k1;
        hash = XXH_rotl64(hash, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        hash ^= (XXH_readLE32(ptr) as u64).wrapping_mul(XXH_PRIME64_1);
        ptr = ptr.add(4);
        hash = XXH_rotl64(hash, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        hash ^= (*ptr as u64).wrapping_mul(XXH_PRIME64_5);
        ptr = ptr.add(1);
        hash = XXH_rotl64(hash, 11).wrapping_mul(XXH_PRIME64_1);
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
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        loop {
            v1 = XXH64_round(v1, XXH_readLE64(input));
            input = input.add(8);
            v2 = XXH64_round(v2, XXH_readLE64(input));
            input = input.add(8);
            v3 = XXH64_round(v3, XXH_readLE64(input));
            input = input.add(8);
            v4 = XXH64_round(v4, XXH_readLE64(input));
            input = input.add(8);
            if input >= limit {
                break;
            }
        }
        h64 = XXH_rotl64(v1, 1)
            .wrapping_add(XXH_rotl64(v2, 7))
            .wrapping_add(XXH_rotl64(v3, 12))
            .wrapping_add(XXH_rotl64(v4, 18));
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
pub unsafe extern "C" fn ZSTD_XXH64(
    input: *const c_void,
    len: usize,
    seed: XXH64_hash_t,
) -> XXH64_hash_t {
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
    seed: XXH64_hash_t,
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
    (*statePtr).v[2] = seed.wrapping_add(0);
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
    {
        let mut p = input as *const u8;
        let bEnd = p.add(len);
        (*state).total_len = (*state).total_len.wrapping_add(len as u64);
        if (*state).memsize as usize + len < 32 {
            ZSTD_memcpy(
                ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                len,
            );
            (*state).memsize += len as u32;
            return XXH_OK;
        }
        if (*state).memsize != 0 {
            ZSTD_memcpy(
                ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                32 - (*state).memsize as usize,
            );
            let m = (*state).mem64.as_ptr() as *const u8;
            (*state).v[0] = XXH64_round((*state).v[0], XXH_readLE64(m));
            (*state).v[1] = XXH64_round((*state).v[1], XXH_readLE64(m.add(8)));
            (*state).v[2] = XXH64_round((*state).v[2], XXH_readLE64(m.add(16)));
            (*state).v[3] = XXH64_round((*state).v[3], XXH_readLE64(m.add(24)));
            p = p.add(32 - (*state).memsize as usize);
            (*state).memsize = 0;
        }
        if p.add(32) <= bEnd {
            let limit = bEnd.sub(32);
            loop {
                (*state).v[0] = XXH64_round((*state).v[0], XXH_readLE64(p));
                p = p.add(8);
                (*state).v[1] = XXH64_round((*state).v[1], XXH_readLE64(p));
                p = p.add(8);
                (*state).v[2] = XXH64_round((*state).v[2], XXH_readLE64(p));
                p = p.add(8);
                (*state).v[3] = XXH64_round((*state).v[3], XXH_readLE64(p));
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
                bEnd.offset_from(p) as usize,
            );
            (*state).memsize = bEnd.offset_from(p) as u32;
        }
    }
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(state: *const XXH64_state_t) -> XXH64_hash_t {
    let mut h64: u64;
    if (*state).total_len >= 32 {
        h64 = XXH_rotl64((*state).v[0], 1)
            .wrapping_add(XXH_rotl64((*state).v[1], 7))
            .wrapping_add(XXH_rotl64((*state).v[2], 12))
            .wrapping_add(XXH_rotl64((*state).v[3], 18));
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
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(
    dst: *mut XXH64_canonical_t,
    hash: XXH64_hash_t,
) {
    let h = if cfg!(target_endian = "little") {
        hash.swap_bytes()
    } else {
        hash
    };
    ZSTD_memcpy(
        dst as *mut c_void,
        core::ptr::addr_of!(h) as *const c_void,
        core::mem::size_of::<XXH64_canonical_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(
    src: *const XXH64_canonical_t,
) -> XXH64_hash_t {
    XXH_readBE64(src as *const u8)
}
