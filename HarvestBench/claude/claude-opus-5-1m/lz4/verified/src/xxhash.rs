//! Translation of xxhash.c (xxHash 0.6.5), namespaced with `XXH_NAMESPACE=LZ4_`.
//!
//! On x86_64 `XXH_FORCE_ALIGN_CHECK` is 0 and the CPU is little endian, so all
//! reads go through the unaligned + little-endian path.

use crate::common::{calloc, free, malloc, memcpy, memset};
use core::ffi::c_void;
use core::ptr;

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 6;
pub const XXH_VERSION_RELEASE: u32 = 5;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

/* XXH_errorcode */
pub const XXH_OK: i32 = 0;
pub const XXH_ERROR: i32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH32_state_t {
    pub total_len_32: u32,
    pub large_len: u32,
    pub v1: u32,
    pub v2: u32,
    pub v3: u32,
    pub v4: u32,
    pub mem32: [u32; 4],
    pub memsize: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH64_state_t {
    pub total_len: u64,
    pub v1: u64,
    pub v2: u64,
    pub v3: u64,
    pub v4: u64,
    pub mem64: [u64; 4],
    pub memsize: u32,
    pub reserved: [u32; 2],
}

#[repr(C)]
pub struct XXH32_canonical_t {
    pub digest: [u8; 4],
}

#[repr(C)]
pub struct XXH64_canonical_t {
    pub digest: [u8; 8],
}

impl XXH32_state_t {
    pub const fn zeroed() -> Self {
        XXH32_state_t {
            total_len_32: 0,
            large_len: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            v4: 0,
            mem32: [0; 4],
            memsize: 0,
            reserved: 0,
        }
    }
}

impl XXH64_state_t {
    pub const fn zeroed() -> Self {
        XXH64_state_t {
            total_len: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            v4: 0,
            mem64: [0; 4],
            memsize: 0,
            reserved: [0; 2],
        }
    }
}

#[inline(always)]
unsafe fn XXH_read32(p: *const u8) -> u32 {
    ptr::read_unaligned(p as *const u32)
}
#[inline(always)]
unsafe fn XXH_read64(p: *const u8) -> u64 {
    ptr::read_unaligned(p as *const u64)
}
#[inline(always)]
unsafe fn XXH_readLE32(p: *const u8) -> u32 {
    XXH_read32(p)
}
#[inline(always)]
unsafe fn XXH_readLE64(p: *const u8) -> u64 {
    XXH_read64(p)
}
#[inline(always)]
unsafe fn XXH_readBE32(p: *const u8) -> u32 {
    XXH_read32(p).swap_bytes()
}
#[inline(always)]
unsafe fn XXH_readBE64(p: *const u8) -> u64 {
    XXH_read64(p).swap_bytes()
}

/* ============================================================== *
 *  32-bit hash functions
 * ============================================================== */
const PRIME32_1: u32 = 2654435761;
const PRIME32_2: u32 = 2246822519;
const PRIME32_3: u32 = 3266489917;
const PRIME32_4: u32 = 668265263;
const PRIME32_5: u32 = 374761393;

#[inline(always)]
fn XXH32_round(seed: u32, input: u32) -> u32 {
    let mut seed = seed.wrapping_add(input.wrapping_mul(PRIME32_2));
    seed = seed.rotate_left(13);
    seed = seed.wrapping_mul(PRIME32_1);
    seed
}

#[inline(always)]
fn XXH32_avalanche(mut h32: u32) -> u32 {
    h32 ^= h32 >> 15;
    h32 = h32.wrapping_mul(PRIME32_2);
    h32 ^= h32 >> 13;
    h32 = h32.wrapping_mul(PRIME32_3);
    h32 ^= h32 >> 16;
    h32
}

unsafe fn XXH32_finalize(mut h32: u32, ptr_in: *const u8, len: usize) -> u32 {
    let mut p = ptr_in;
    let n = len & 15;
    let n4 = n / 4;
    let n1 = n % 4;

    for _ in 0..n4 {
        /* PROCESS4 */
        h32 = h32.wrapping_add(XXH_readLE32(p).wrapping_mul(PRIME32_3));
        p = p.wrapping_add(4);
        h32 = h32.rotate_left(17).wrapping_mul(PRIME32_4);
    }
    for _ in 0..n1 {
        /* PROCESS1 */
        h32 = h32.wrapping_add((*p as u32).wrapping_mul(PRIME32_5));
        p = p.wrapping_add(1);
        h32 = h32.rotate_left(11).wrapping_mul(PRIME32_1);
    }
    XXH32_avalanche(h32)
}

unsafe fn XXH32_endian_align(input: *const u8, len: usize, seed: u32) -> u32 {
    let mut p = input;
    let bEnd = p.wrapping_add(len);
    let mut h32: u32;

    if len >= 16 {
        let limit = bEnd.wrapping_sub(15);
        let mut v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
        let mut v2 = seed.wrapping_add(PRIME32_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(PRIME32_1);

        loop {
            v1 = XXH32_round(v1, XXH_readLE32(p));
            p = p.wrapping_add(4);
            v2 = XXH32_round(v2, XXH_readLE32(p));
            p = p.wrapping_add(4);
            v3 = XXH32_round(v3, XXH_readLE32(p));
            p = p.wrapping_add(4);
            v4 = XXH32_round(v4, XXH_readLE32(p));
            p = p.wrapping_add(4);
            if !(p < limit) {
                break;
            }
        }

        h32 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
    } else {
        h32 = seed.wrapping_add(PRIME32_5);
    }

    h32 = h32.wrapping_add(len as u32);

    XXH32_finalize(h32, p, len & 15)
}

#[inline]
pub unsafe fn xxh32(input: *const u8, len: usize, seed: u32) -> u32 {
    XXH32_endian_align(input, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH_versionNumber() -> u32 {
    XXH_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32(input: *const c_void, len: usize, seed: u32) -> u32 {
    XXH32_endian_align(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_createState() -> *mut XXH32_state_t {
    malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_freeState(statePtr: *mut XXH32_state_t) -> i32 {
    free(statePtr as *mut u8);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_copyState(
    dstState: *mut XXH32_state_t,
    srcState: *const XXH32_state_t,
) {
    memcpy(
        dstState as *mut u8,
        srcState as *const u8,
        core::mem::size_of::<XXH32_state_t>(),
    );
}

pub unsafe fn xxh32_reset(statePtr: *mut XXH32_state_t, seed: u32) -> i32 {
    let mut state = XXH32_state_t::zeroed();
    memset(
        &mut state as *mut XXH32_state_t as *mut u8,
        0,
        core::mem::size_of::<XXH32_state_t>(),
    );
    state.v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
    state.v2 = seed.wrapping_add(PRIME32_2);
    state.v3 = seed.wrapping_add(0);
    state.v4 = seed.wrapping_sub(PRIME32_1);
    /* do not write into reserved */
    memcpy(
        statePtr as *mut u8,
        &state as *const XXH32_state_t as *const u8,
        core::mem::size_of::<XXH32_state_t>() - core::mem::size_of::<u32>(),
    );
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_reset(statePtr: *mut XXH32_state_t, seed: u32) -> i32 {
    xxh32_reset(statePtr, seed)
}

pub unsafe fn xxh32_update(state: *mut XXH32_state_t, input: *const u8, len: usize) -> i32 {
    if input.is_null() {
        return XXH_ERROR;
    }

    let mut p = input;
    let bEnd = p.wrapping_add(len);

    (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
    (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

    if ((*state).memsize as usize) + len < 16 {
        /* fill in tmp buffer */
        memcpy(
            ((*state).mem32.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
            input,
            len,
        );
        (*state).memsize = (*state).memsize.wrapping_add(len as u32);
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        /* some data left from previous update */
        memcpy(
            ((*state).mem32.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
            input,
            16 - (*state).memsize as usize,
        );
        {
            let mut p32 = (*state).mem32.as_ptr() as *const u8;
            (*state).v1 = XXH32_round((*state).v1, XXH_readLE32(p32));
            p32 = p32.wrapping_add(4);
            (*state).v2 = XXH32_round((*state).v2, XXH_readLE32(p32));
            p32 = p32.wrapping_add(4);
            (*state).v3 = XXH32_round((*state).v3, XXH_readLE32(p32));
            p32 = p32.wrapping_add(4);
            (*state).v4 = XXH32_round((*state).v4, XXH_readLE32(p32));
        }
        p = p.wrapping_add(16 - (*state).memsize as usize);
        (*state).memsize = 0;
    }

    if p <= bEnd.wrapping_sub(16) {
        let limit = bEnd.wrapping_sub(16);
        let mut v1 = (*state).v1;
        let mut v2 = (*state).v2;
        let mut v3 = (*state).v3;
        let mut v4 = (*state).v4;

        loop {
            v1 = XXH32_round(v1, XXH_readLE32(p));
            p = p.wrapping_add(4);
            v2 = XXH32_round(v2, XXH_readLE32(p));
            p = p.wrapping_add(4);
            v3 = XXH32_round(v3, XXH_readLE32(p));
            p = p.wrapping_add(4);
            v4 = XXH32_round(v4, XXH_readLE32(p));
            p = p.wrapping_add(4);
            if !(p <= limit) {
                break;
            }
        }

        (*state).v1 = v1;
        (*state).v2 = v2;
        (*state).v3 = v3;
        (*state).v4 = v4;
    }

    if p < bEnd {
        memcpy(
            (*state).mem32.as_mut_ptr() as *mut u8,
            p,
            (bEnd as usize).wrapping_sub(p as usize),
        );
        (*state).memsize = (bEnd as usize).wrapping_sub(p as usize) as u32;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_update(
    state_in: *mut XXH32_state_t,
    input: *const c_void,
    len: usize,
) -> i32 {
    xxh32_update(state_in, input as *const u8, len)
}

pub unsafe fn xxh32_digest(state: *const XXH32_state_t) -> u32 {
    let mut h32: u32;

    if (*state).large_len != 0 {
        h32 = (*state)
            .v1
            .rotate_left(1)
            .wrapping_add((*state).v2.rotate_left(7))
            .wrapping_add((*state).v3.rotate_left(12))
            .wrapping_add((*state).v4.rotate_left(18));
    } else {
        h32 = (*state).v3.wrapping_add(PRIME32_5);
    }

    h32 = h32.wrapping_add((*state).total_len_32);

    XXH32_finalize(
        h32,
        (*state).mem32.as_ptr() as *const u8,
        (*state).memsize as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_digest(state_in: *const XXH32_state_t) -> u32 {
    xxh32_digest(state_in)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_canonicalFromHash(dst: *mut XXH32_canonical_t, hash: u32) {
    let h = hash.swap_bytes();
    memcpy(dst as *mut u8, &h as *const u32 as *const u8, 4);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> u32 {
    XXH_readBE32(src as *const u8)
}

/* ============================================================== *
 *  64-bit hash functions
 * ============================================================== */
const PRIME64_1: u64 = 11400714785074694791;
const PRIME64_2: u64 = 14029467366897019727;
const PRIME64_3: u64 = 1609587929392839161;
const PRIME64_4: u64 = 9650029242287828579;
const PRIME64_5: u64 = 2870177450012600261;

#[inline(always)]
fn XXH64_round(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = acc.rotate_left(31);
    acc = acc.wrapping_mul(PRIME64_1);
    acc
}

#[inline(always)]
fn XXH64_mergeRound(acc: u64, val: u64) -> u64 {
    let val = XXH64_round(0, val);
    let mut acc = acc ^ val;
    acc = acc.wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4);
    acc
}

#[inline(always)]
fn XXH64_avalanche(mut h64: u64) -> u64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

unsafe fn XXH64_finalize(mut h64: u64, ptr_in: *const u8, len: usize) -> u64 {
    let mut p = ptr_in;
    let n = len & 31;
    let n8 = n / 8;
    let rem = n % 8;
    let n4 = rem / 4;
    let n1 = rem % 4;

    for _ in 0..n8 {
        /* PROCESS8_64 */
        let k1 = XXH64_round(0, XXH_readLE64(p));
        p = p.wrapping_add(8);
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(PRIME64_1)
            .wrapping_add(PRIME64_4);
    }
    for _ in 0..n4 {
        /* PROCESS4_64 */
        h64 ^= (XXH_readLE32(p) as u64).wrapping_mul(PRIME64_1);
        p = p.wrapping_add(4);
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(PRIME64_2)
            .wrapping_add(PRIME64_3);
    }
    for _ in 0..n1 {
        /* PROCESS1_64 */
        h64 ^= (*p as u64).wrapping_mul(PRIME64_5);
        p = p.wrapping_add(1);
        h64 = h64.rotate_left(11).wrapping_mul(PRIME64_1);
    }
    XXH64_avalanche(h64)
}

unsafe fn XXH64_endian_align(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let bEnd = p.wrapping_add(len);
    let mut h64: u64;

    if len >= 32 {
        let limit = bEnd.wrapping_sub(32);
        let mut v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
        let mut v2 = seed.wrapping_add(PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(PRIME64_1);

        loop {
            v1 = XXH64_round(v1, XXH_readLE64(p));
            p = p.wrapping_add(8);
            v2 = XXH64_round(v2, XXH_readLE64(p));
            p = p.wrapping_add(8);
            v3 = XXH64_round(v3, XXH_readLE64(p));
            p = p.wrapping_add(8);
            v4 = XXH64_round(v4, XXH_readLE64(p));
            p = p.wrapping_add(8);
            if !(p <= limit) {
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
        h64 = seed.wrapping_add(PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);

    XXH64_finalize(h64, p, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64(input: *const c_void, len: usize, seed: u64) -> u64 {
    XXH64_endian_align(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_createState() -> *mut XXH64_state_t {
    malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_freeState(statePtr: *mut XXH64_state_t) -> i32 {
    free(statePtr as *mut u8);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_copyState(
    dstState: *mut XXH64_state_t,
    srcState: *const XXH64_state_t,
) {
    memcpy(
        dstState as *mut u8,
        srcState as *const u8,
        core::mem::size_of::<XXH64_state_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_reset(statePtr: *mut XXH64_state_t, seed: u64) -> i32 {
    let mut state = XXH64_state_t::zeroed();
    memset(
        &mut state as *mut XXH64_state_t as *mut u8,
        0,
        core::mem::size_of::<XXH64_state_t>(),
    );
    state.v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
    state.v2 = seed.wrapping_add(PRIME64_2);
    state.v3 = seed.wrapping_add(0);
    state.v4 = seed.wrapping_sub(PRIME64_1);
    /* do not write into reserved */
    memcpy(
        statePtr as *mut u8,
        &state as *const XXH64_state_t as *const u8,
        core::mem::size_of::<XXH64_state_t>() - 2 * core::mem::size_of::<u32>(),
    );
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_update(
    state: *mut XXH64_state_t,
    input: *const c_void,
    len: usize,
) -> i32 {
    if input.is_null() {
        return XXH_ERROR;
    }
    let input = input as *const u8;

    let mut p = input;
    let bEnd = p.wrapping_add(len);

    (*state).total_len = (*state).total_len.wrapping_add(len as u64);

    if ((*state).memsize as usize) + len < 32 {
        /* fill in tmp buffer */
        memcpy(
            ((*state).mem64.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
            input,
            len,
        );
        (*state).memsize = (*state).memsize.wrapping_add(len as u32);
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        /* tmp buffer is full */
        memcpy(
            ((*state).mem64.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
            input,
            32 - (*state).memsize as usize,
        );
        let base = (*state).mem64.as_ptr() as *const u8;
        (*state).v1 = XXH64_round((*state).v1, XXH_readLE64(base));
        (*state).v2 = XXH64_round((*state).v2, XXH_readLE64(base.wrapping_add(8)));
        (*state).v3 = XXH64_round((*state).v3, XXH_readLE64(base.wrapping_add(16)));
        (*state).v4 = XXH64_round((*state).v4, XXH_readLE64(base.wrapping_add(24)));
        p = p.wrapping_add(32 - (*state).memsize as usize);
        (*state).memsize = 0;
    }

    if p.wrapping_add(32) <= bEnd {
        let limit = bEnd.wrapping_sub(32);
        let mut v1 = (*state).v1;
        let mut v2 = (*state).v2;
        let mut v3 = (*state).v3;
        let mut v4 = (*state).v4;

        loop {
            v1 = XXH64_round(v1, XXH_readLE64(p));
            p = p.wrapping_add(8);
            v2 = XXH64_round(v2, XXH_readLE64(p));
            p = p.wrapping_add(8);
            v3 = XXH64_round(v3, XXH_readLE64(p));
            p = p.wrapping_add(8);
            v4 = XXH64_round(v4, XXH_readLE64(p));
            p = p.wrapping_add(8);
            if !(p <= limit) {
                break;
            }
        }

        (*state).v1 = v1;
        (*state).v2 = v2;
        (*state).v3 = v3;
        (*state).v4 = v4;
    }

    if p < bEnd {
        memcpy(
            (*state).mem64.as_mut_ptr() as *mut u8,
            p,
            (bEnd as usize).wrapping_sub(p as usize),
        );
        (*state).memsize = (bEnd as usize).wrapping_sub(p as usize) as u32;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_digest(state: *const XXH64_state_t) -> u64 {
    let mut h64: u64;

    if (*state).total_len >= 32 {
        let v1 = (*state).v1;
        let v2 = (*state).v2;
        let v3 = (*state).v3;
        let v4 = (*state).v4;

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
        h64 = (*state).v3.wrapping_add(PRIME64_5);
    }

    h64 = h64.wrapping_add((*state).total_len);

    XXH64_finalize(
        h64,
        (*state).mem64.as_ptr() as *const u8,
        (*state).total_len as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, hash: u64) {
    let h = hash.swap_bytes();
    memcpy(dst as *mut u8, &h as *const u64 as *const u8, 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    XXH_readBE64(src as *const u8)
}

/* silence unused import warnings */
#[allow(dead_code)]
unsafe fn _unused(n: usize) -> *mut u8 {
    calloc(1, n)
}
