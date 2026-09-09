/*
 * xxHash - Extremely Fast Hash algorithm
 * Copyright (c) Yann Collet - Meta Platforms, Inc
 *
 * Literal transliteration of the scalar reference implementation in
 * c_src/src/common/xxhash.h (compiled with XXH_NAMESPACE=ZSTD_).
 * Only XXH32 / XXH64 are exported here; XXH3 is NOT compiled into libzstd.
 *
 * Build configuration mirrored from the C:
 *   XXH_NAMESPACE=ZSTD_        -> public symbols are prefixed with ZSTD_
 *   XXH_FORCE_ALIGN_CHECK == 0 -> reads are always "unaligned"
 *   XXH32_ENDJMP == 0          -> XXH32_finalize uses the compact rerolled form
 *   XXH_CPU_LITTLE_ENDIAN == 1 -> little-endian target
 */

use crate::common::mem::{size_t, MEM_read32, MEM_readLE32, MEM_readLE64, MEM_swap32, MEM_swap64};
use crate::common::zstd_internal::{free, malloc, memcpy, memset};
use core::ffi::{c_uint, c_void};

/* ===== Version ===== */

pub const XXH_VERSION_MAJOR: c_uint = 0;
pub const XXH_VERSION_MINOR: c_uint = 8;
pub const XXH_VERSION_RELEASE: c_uint = 2;
pub const XXH_VERSION_NUMBER: c_uint =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

/* ===== Error code ===== */

pub type XXH_errorcode = c_uint;
pub const XXH_OK: XXH_errorcode = 0;
pub const XXH_ERROR: XXH_errorcode = 1;

/* ===== State & canonical types ===== */

pub type XXH32_hash_t = u32;
pub type XXH64_hash_t = u64;

#[repr(C)]
pub struct XXH32_state_s {
    pub total_len_32: XXH32_hash_t,
    pub large_len: XXH32_hash_t,
    pub v: [XXH32_hash_t; 4],
    pub mem32: [XXH32_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved: XXH32_hash_t,
}
pub type XXH32_state_t = XXH32_state_s;

#[repr(C)]
pub struct XXH64_state_s {
    pub total_len: XXH64_hash_t,
    pub v: [XXH64_hash_t; 4],
    pub mem64: [XXH64_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved32: XXH32_hash_t,
    pub reserved64: XXH64_hash_t,
}
pub type XXH64_state_t = XXH64_state_s;

#[repr(C)]
pub struct XXH32_canonical_t {
    pub digest: [u8; 4],
}

#[repr(C)]
pub struct XXH64_canonical_t {
    pub digest: [u8; 8],
}

/* ===== Memory access helpers (little-endian target) ===== */

#[inline(always)]
unsafe fn XXH_readLE32(ptr: *const c_void) -> u32 {
    MEM_readLE32(ptr as *const u8)
}

#[inline(always)]
unsafe fn XXH_readLE64(ptr: *const c_void) -> u64 {
    MEM_readLE64(ptr as *const u8)
}

/* XXH_readBE32 on a little-endian CPU == swap32(read32) */
#[inline(always)]
unsafe fn XXH_readBE32(ptr: *const c_void) -> u32 {
    MEM_swap32(MEM_read32(ptr as *const u8))
}

/* XXH_readBE64 on a little-endian CPU == swap64(read64) */
#[inline(always)]
unsafe fn XXH_readBE64(ptr: *const c_void) -> u64 {
    // MEM_read64 reads native; swap to obtain big-endian interpretation.
    MEM_swap64(crate::common::mem::MEM_read64(ptr as *const u8))
}

/*
 * XXH_readLE32_align / XXH_readLE64_align:
 * XXH_FORCE_ALIGN_CHECK == 0, so `align` is always XXH_unaligned in practice and
 * these reduce to the plain unaligned reads.
 */
#[inline(always)]
unsafe fn XXH_get32bits(p: *const c_void) -> u32 {
    XXH_readLE32(p)
}

#[inline(always)]
unsafe fn XXH_get64bits(p: *const c_void) -> u64 {
    XXH_readLE64(p)
}

#[inline(always)]
fn XXH_rotl32(x: u32, r: u32) -> u32 {
    x.rotate_left(r)
}

#[inline(always)]
fn XXH_rotl64(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

/* ===== Misc ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

/* ===============================================================
 * 32-bit hash functions
 * =============================================================== */

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

/*
 * XXH32_finalize: XXH32_ENDJMP == 0 -> compact rerolled version.
 */
unsafe fn XXH32_finalize(mut hash: u32, mut ptr: *const u8, mut len: size_t) -> u32 {
    // XXH_PROCESS1
    macro_rules! process1 {
        () => {{
            hash = hash.wrapping_add((*ptr as u32).wrapping_mul(XXH_PRIME32_5));
            ptr = ptr.add(1);
            hash = XXH_rotl32(hash, 11).wrapping_mul(XXH_PRIME32_1);
        }};
    }
    // XXH_PROCESS4
    macro_rules! process4 {
        () => {{
            hash = hash
                .wrapping_add(XXH_get32bits(ptr as *const c_void).wrapping_mul(XXH_PRIME32_3));
            ptr = ptr.add(4);
            hash = XXH_rotl32(hash, 17).wrapping_mul(XXH_PRIME32_4);
        }};
    }

    len &= 15;
    while len >= 4 {
        process4!();
        len -= 4;
    }
    while len > 0 {
        process1!();
        len -= 1;
    }
    XXH32_avalanche(hash)
}

#[inline(always)]
unsafe fn XXH32_endian_align(mut input: *const u8, len: size_t, seed: u32) -> u32 {
    let mut h32: u32;

    if len >= 16 {
        let bEnd = input.add(len);
        let limit = bEnd.sub(15);
        let mut v1 = seed.wrapping_add(XXH_PRIME32_1).wrapping_add(XXH_PRIME32_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME32_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME32_1);

        loop {
            v1 = XXH32_round(v1, XXH_get32bits(input as *const c_void));
            input = input.add(4);
            v2 = XXH32_round(v2, XXH_get32bits(input as *const c_void));
            input = input.add(4);
            v3 = XXH32_round(v3, XXH_get32bits(input as *const c_void));
            input = input.add(4);
            v4 = XXH32_round(v4, XXH_get32bits(input as *const c_void));
            input = input.add(4);
            if !(input < limit) {
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
pub unsafe extern "C" fn ZSTD_XXH32(input: *const c_void, len: size_t, seed: u32) -> u32 {
    // XXH_FORCE_ALIGN_CHECK == 0 -> always unaligned path.
    XXH32_endian_align(input as *const u8, len, seed)
}

/*******   Hash streaming   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_createState() -> *mut XXH32_state_t {
    malloc(core::mem::size_of::<XXH32_state_t>() as size_t) as *mut XXH32_state_t
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
    memcpy(
        dstState as *mut c_void,
        srcState as *const c_void,
        core::mem::size_of::<XXH32_state_t>() as size_t,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_reset(statePtr: *mut XXH32_state_t, seed: u32) -> XXH_errorcode {
    memset(
        statePtr as *mut c_void,
        0,
        core::mem::size_of::<XXH32_state_t>() as size_t,
    );
    (*statePtr).v[0] = seed.wrapping_add(XXH_PRIME32_1).wrapping_add(XXH_PRIME32_2);
    (*statePtr).v[1] = seed.wrapping_add(XXH_PRIME32_2);
    (*statePtr).v[2] = seed.wrapping_add(0);
    (*statePtr).v[3] = seed.wrapping_sub(XXH_PRIME32_1);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_update(
    state: *mut XXH32_state_t,
    input: *const c_void,
    len: size_t,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_OK;
    }

    let mut p = input as *const u8;
    let bEnd = p.add(len);

    (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
    (*state).large_len |=
        ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

    if ((*state).memsize as size_t) + len < 16 {
        // fill in tmp buffer
        memcpy(
            ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            len,
        );
        (*state).memsize = (*state).memsize.wrapping_add(len as u32);
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        // some data left from previous update
        memcpy(
            ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            (16 - (*state).memsize) as size_t,
        );
        {
            let mut p32 = (*state).mem32.as_ptr();
            (*state).v[0] = XXH32_round((*state).v[0], XXH_readLE32(p32 as *const c_void));
            p32 = p32.add(1);
            (*state).v[1] = XXH32_round((*state).v[1], XXH_readLE32(p32 as *const c_void));
            p32 = p32.add(1);
            (*state).v[2] = XXH32_round((*state).v[2], XXH_readLE32(p32 as *const c_void));
            p32 = p32.add(1);
            (*state).v[3] = XXH32_round((*state).v[3], XXH_readLE32(p32 as *const c_void));
        }
        p = p.add((16 - (*state).memsize) as usize);
        (*state).memsize = 0;
    }

    if p <= bEnd.sub(16) {
        let limit = bEnd.sub(16);
        loop {
            (*state).v[0] = XXH32_round((*state).v[0], XXH_readLE32(p as *const c_void));
            p = p.add(4);
            (*state).v[1] = XXH32_round((*state).v[1], XXH_readLE32(p as *const c_void));
            p = p.add(4);
            (*state).v[2] = XXH32_round((*state).v[2], XXH_readLE32(p as *const c_void));
            p = p.add(4);
            (*state).v[3] = XXH32_round((*state).v[3], XXH_readLE32(p as *const c_void));
            p = p.add(4);
            if !(p <= limit) {
                break;
            }
        }
    }

    if p < bEnd {
        memcpy(
            (*state).mem32.as_mut_ptr() as *mut c_void,
            p as *const c_void,
            bEnd.offset_from(p) as size_t,
        );
        (*state).memsize = bEnd.offset_from(p) as u32;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_digest(state: *const XXH32_state_t) -> u32 {
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
        (*state).memsize as size_t,
    )
}

/*******   Canonical representation   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(dst: *mut XXH32_canonical_t, mut hash: u32) {
    // XXH_CPU_LITTLE_ENDIAN == 1
    hash = MEM_swap32(hash);
    memcpy(
        dst as *mut c_void,
        &hash as *const u32 as *const c_void,
        core::mem::size_of::<XXH32_canonical_t>() as size_t,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> u32 {
    XXH_readBE32(src as *const c_void)
}

/* ===============================================================
 * 64-bit hash functions
 * =============================================================== */

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

unsafe fn XXH64_finalize(mut hash: u64, mut ptr: *const u8, mut len: size_t) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = XXH64_round(0, XXH_get64bits(ptr as *const c_void));
        ptr = ptr.add(8);
        hash ^= k1;
        hash = XXH_rotl64(hash, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        hash ^= (XXH_get32bits(ptr as *const c_void) as u64).wrapping_mul(XXH_PRIME64_1);
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

#[inline(always)]
unsafe fn XXH64_endian_align(mut input: *const u8, len: size_t, seed: u64) -> u64 {
    let mut h64: u64;

    if len >= 32 {
        let bEnd = input.add(len);
        let limit = bEnd.sub(31);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = XXH64_round(v1, XXH_get64bits(input as *const c_void));
            input = input.add(8);
            v2 = XXH64_round(v2, XXH_get64bits(input as *const c_void));
            input = input.add(8);
            v3 = XXH64_round(v3, XXH_get64bits(input as *const c_void));
            input = input.add(8);
            v4 = XXH64_round(v4, XXH_get64bits(input as *const c_void));
            input = input.add(8);
            if !(input < limit) {
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
pub unsafe extern "C" fn ZSTD_XXH64(input: *const c_void, len: size_t, seed: u64) -> u64 {
    XXH64_endian_align(input as *const u8, len, seed)
}

/*******   Hash Streaming   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_createState() -> *mut XXH64_state_t {
    malloc(core::mem::size_of::<XXH64_state_t>() as size_t) as *mut XXH64_state_t
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
    memcpy(
        dstState as *mut c_void,
        srcState as *const c_void,
        core::mem::size_of::<XXH64_state_t>() as size_t,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_reset(statePtr: *mut XXH64_state_t, seed: u64) -> XXH_errorcode {
    memset(
        statePtr as *mut c_void,
        0,
        core::mem::size_of::<XXH64_state_t>() as size_t,
    );
    (*statePtr).v[0] = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
    (*statePtr).v[1] = seed.wrapping_add(XXH_PRIME64_2);
    (*statePtr).v[2] = seed.wrapping_add(0);
    (*statePtr).v[3] = seed.wrapping_sub(XXH_PRIME64_1);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_update(
    state: *mut XXH64_state_t,
    input: *const c_void,
    len: size_t,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_OK;
    }

    let mut p = input as *const u8;
    let bEnd = p.add(len);

    (*state).total_len = (*state).total_len.wrapping_add(len as u64);

    if ((*state).memsize as size_t) + len < 32 {
        // fill in tmp buffer
        memcpy(
            ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            len,
        );
        (*state).memsize = (*state).memsize.wrapping_add(len as u32);
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        // tmp buffer is full
        memcpy(
            ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize) as *mut c_void,
            input,
            (32 - (*state).memsize) as size_t,
        );
        (*state).v[0] = XXH64_round(
            (*state).v[0],
            XXH_readLE64((*state).mem64.as_ptr().add(0) as *const c_void),
        );
        (*state).v[1] = XXH64_round(
            (*state).v[1],
            XXH_readLE64((*state).mem64.as_ptr().add(1) as *const c_void),
        );
        (*state).v[2] = XXH64_round(
            (*state).v[2],
            XXH_readLE64((*state).mem64.as_ptr().add(2) as *const c_void),
        );
        (*state).v[3] = XXH64_round(
            (*state).v[3],
            XXH_readLE64((*state).mem64.as_ptr().add(3) as *const c_void),
        );
        p = p.add((32 - (*state).memsize) as usize);
        (*state).memsize = 0;
    }

    if p.add(32) <= bEnd {
        let limit = bEnd.sub(32);
        loop {
            (*state).v[0] = XXH64_round((*state).v[0], XXH_readLE64(p as *const c_void));
            p = p.add(8);
            (*state).v[1] = XXH64_round((*state).v[1], XXH_readLE64(p as *const c_void));
            p = p.add(8);
            (*state).v[2] = XXH64_round((*state).v[2], XXH_readLE64(p as *const c_void));
            p = p.add(8);
            (*state).v[3] = XXH64_round((*state).v[3], XXH_readLE64(p as *const c_void));
            p = p.add(8);
            if !(p <= limit) {
                break;
            }
        }
    }

    if p < bEnd {
        memcpy(
            (*state).mem64.as_mut_ptr() as *mut c_void,
            p as *const c_void,
            bEnd.offset_from(p) as size_t,
        );
        (*state).memsize = bEnd.offset_from(p) as u32;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(state: *const XXH64_state_t) -> u64 {
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
        (*state).total_len as size_t,
    )
}

/*******   Canonical representation   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, mut hash: u64) {
    // XXH_CPU_LITTLE_ENDIAN == 1
    hash = MEM_swap64(hash);
    memcpy(
        dst as *mut c_void,
        &hash as *const u64 as *const c_void,
        core::mem::size_of::<XXH64_canonical_t>() as size_t,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    XXH_readBE64(src as *const c_void)
}

/* ===============================================================
 * Un-prefixed aliases for use by other Rust modules
 * (the C code refers to XXH32/XXH64/... which, after XXH_NAMESPACE
 * substitution, link as ZSTD_XXH*).
 * =============================================================== */

pub use self::ZSTD_XXH32 as XXH32;
pub use self::ZSTD_XXH32_canonicalFromHash as XXH32_canonicalFromHash;
pub use self::ZSTD_XXH32_copyState as XXH32_copyState;
pub use self::ZSTD_XXH32_createState as XXH32_createState;
pub use self::ZSTD_XXH32_digest as XXH32_digest;
pub use self::ZSTD_XXH32_freeState as XXH32_freeState;
pub use self::ZSTD_XXH32_hashFromCanonical as XXH32_hashFromCanonical;
pub use self::ZSTD_XXH32_reset as XXH32_reset;
pub use self::ZSTD_XXH32_update as XXH32_update;

pub use self::ZSTD_XXH64 as XXH64;
pub use self::ZSTD_XXH64_canonicalFromHash as XXH64_canonicalFromHash;
pub use self::ZSTD_XXH64_copyState as XXH64_copyState;
pub use self::ZSTD_XXH64_createState as XXH64_createState;
pub use self::ZSTD_XXH64_digest as XXH64_digest;
pub use self::ZSTD_XXH64_freeState as XXH64_freeState;
pub use self::ZSTD_XXH64_hashFromCanonical as XXH64_hashFromCanonical;
pub use self::ZSTD_XXH64_reset as XXH64_reset;
pub use self::ZSTD_XXH64_update as XXH64_update;

pub use self::ZSTD_XXH_versionNumber as XXH_versionNumber;
