//! Translation of `xxhash.c` (xxHash 0.6.5), compiled with `XXH_NAMESPACE=LZ4_`.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_uint, c_void};

pub const XXH_VERSION_MAJOR: c_uint = 0;
pub const XXH_VERSION_MINOR: c_uint = 6;
pub const XXH_VERSION_RELEASE: c_uint = 5;
pub const XXH_VERSION_NUMBER: c_uint =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

/// XXH_errorcode
pub const XXH_OK: c_int = 0;
pub const XXH_ERROR: c_int = 1;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

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

const _: () = {
    assert!(core::mem::size_of::<XXH32_state_t>() == 48);
    assert!(core::mem::size_of::<XXH64_state_t>() == 88);
};

/* ===== memory access (little endian host) ===== */
#[inline(always)]
unsafe fn XXH_read32(p: *const u8) -> u32 {
    unsafe { (p as *const u32).read_unaligned() }
}
#[inline(always)]
unsafe fn XXH_read64(p: *const u8) -> u64 {
    unsafe { (p as *const u64).read_unaligned() }
}
#[inline(always)]
unsafe fn XXH_readLE32(p: *const u8) -> u32 {
    unsafe { XXH_read32(p) }
}
#[inline(always)]
unsafe fn XXH_readLE64(p: *const u8) -> u64 {
    unsafe { XXH_read64(p) }
}
#[inline(always)]
unsafe fn XXH_readBE32(p: *const u8) -> u32 {
    unsafe { XXH_read32(p).swap_bytes() }
}
#[inline(always)]
unsafe fn XXH_readBE64(p: *const u8) -> u64 {
    unsafe { XXH_read64(p).swap_bytes() }
}

#[inline(always)]
fn XXH_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}
#[inline(always)]
fn XXH_rotl64(x: u64, r: u32) -> u64 {
    (x << r) | (x >> (64 - r))
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

/* ===== 32-bit ===== */
const PRIME32_1: u32 = 2654435761;
const PRIME32_2: u32 = 2246822519;
const PRIME32_3: u32 = 3266489917;
const PRIME32_4: u32 = 668265263;
const PRIME32_5: u32 = 374761393;

#[inline(always)]
fn XXH32_round(mut seed: u32, input: u32) -> u32 {
    seed = seed.wrapping_add(input.wrapping_mul(PRIME32_2));
    seed = XXH_rotl32(seed, 13);
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

unsafe fn XXH32_finalize(mut h32: u32, ptr: *const c_void, len: usize) -> u32 {
    unsafe {
        let mut p = ptr as *const u8;

        macro_rules! PROCESS1 {
            () => {{
                h32 = h32.wrapping_add((*p as u32).wrapping_mul(PRIME32_5));
                p = p.wrapping_add(1);
                h32 = XXH_rotl32(h32, 11).wrapping_mul(PRIME32_1);
            }};
        }
        macro_rules! PROCESS4 {
            () => {{
                h32 = h32.wrapping_add(XXH_readLE32(p).wrapping_mul(PRIME32_3));
                p = p.wrapping_add(4);
                h32 = XXH_rotl32(h32, 17).wrapping_mul(PRIME32_4);
            }};
        }

        match len & 15 {
            12 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS4!();
                XXH32_avalanche(h32)
            }
            8 => {
                PROCESS4!();
                PROCESS4!();
                XXH32_avalanche(h32)
            }
            4 => {
                PROCESS4!();
                XXH32_avalanche(h32)
            }
            13 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS4!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            9 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            5 => {
                PROCESS4!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            14 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS4!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            10 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            6 => {
                PROCESS4!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            15 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS4!();
                PROCESS1!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            11 => {
                PROCESS4!();
                PROCESS4!();
                PROCESS1!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            7 => {
                PROCESS4!();
                PROCESS1!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            3 => {
                PROCESS1!();
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            2 => {
                PROCESS1!();
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            1 => {
                PROCESS1!();
                XXH32_avalanche(h32)
            }
            _ => XXH32_avalanche(h32),
        }
    }
}

unsafe fn XXH32_endian_align(input: *const c_void, len: usize, seed: u32) -> u32 {
    unsafe {
        let mut p = input as *const u8;
        let bEnd = p.wrapping_add(len);
        let mut h32: u32;

        if len >= 16 {
            let limit = bEnd.wrapping_sub(15);
            let mut v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
            let mut v2 = seed.wrapping_add(PRIME32_2);
            let mut v3 = seed;
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

            h32 = XXH_rotl32(v1, 1)
                .wrapping_add(XXH_rotl32(v2, 7))
                .wrapping_add(XXH_rotl32(v3, 12))
                .wrapping_add(XXH_rotl32(v4, 18));
        } else {
            h32 = seed.wrapping_add(PRIME32_5);
        }

        h32 = h32.wrapping_add(len as u32);

        XXH32_finalize(h32, p as *const c_void, len & 15)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32(input: *const c_void, len: usize, seed: c_uint) -> c_uint {
    unsafe { XXH32_endian_align(input, len, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_createState() -> *mut XXH32_state_t {
    unsafe { malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_freeState(statePtr: *mut XXH32_state_t) -> c_int {
    unsafe {
        free(statePtr as *mut c_void);
        XXH_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_copyState(
    dstState: *mut XXH32_state_t,
    srcState: *const XXH32_state_t,
) {
    unsafe {
        core::ptr::copy_nonoverlapping(
            srcState as *const u8,
            dstState as *mut u8,
            core::mem::size_of::<XXH32_state_t>(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_reset(statePtr: *mut XXH32_state_t, seed: c_uint) -> c_int {
    unsafe {
        let mut state: XXH32_state_t = core::mem::zeroed();
        state.v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
        state.v2 = seed.wrapping_add(PRIME32_2);
        state.v3 = seed;
        state.v4 = seed.wrapping_sub(PRIME32_1);
        core::ptr::copy_nonoverlapping(
            &state as *const XXH32_state_t as *const u8,
            statePtr as *mut u8,
            core::mem::size_of::<XXH32_state_t>() - core::mem::size_of::<u32>(),
        );
        XXH_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_update(
    state: *mut XXH32_state_t,
    input: *const c_void,
    len: usize,
) -> c_int {
    unsafe {
        if input.is_null() {
            return XXH_ERROR;
        }

        let mut p = input as *const u8;
        let bEnd = p.wrapping_add(len);

        (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
        (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

        if ((*state).memsize as usize) + len < 16 {
            core::ptr::copy_nonoverlapping(
                input as *const u8,
                ((*state).mem32.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                len,
            );
            (*state).memsize += len as u32;
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            core::ptr::copy_nonoverlapping(
                input as *const u8,
                ((*state).mem32.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                16 - (*state).memsize as usize,
            );
            {
                let mut p32 = (*state).mem32.as_ptr();
                (*state).v1 = XXH32_round((*state).v1, XXH_readLE32(p32 as *const u8));
                p32 = p32.wrapping_add(1);
                (*state).v2 = XXH32_round((*state).v2, XXH_readLE32(p32 as *const u8));
                p32 = p32.wrapping_add(1);
                (*state).v3 = XXH32_round((*state).v3, XXH_readLE32(p32 as *const u8));
                p32 = p32.wrapping_add(1);
                (*state).v4 = XXH32_round((*state).v4, XXH_readLE32(p32 as *const u8));
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
            let n = (bEnd as usize) - (p as usize);
            core::ptr::copy_nonoverlapping(p, (*state).mem32.as_mut_ptr() as *mut u8, n);
            (*state).memsize = n as u32;
        }

        XXH_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_digest(state: *const XXH32_state_t) -> c_uint {
    unsafe {
        let mut h32: u32;

        if (*state).large_len != 0 {
            h32 = XXH_rotl32((*state).v1, 1)
                .wrapping_add(XXH_rotl32((*state).v2, 7))
                .wrapping_add(XXH_rotl32((*state).v3, 12))
                .wrapping_add(XXH_rotl32((*state).v4, 18));
        } else {
            h32 = (*state).v3.wrapping_add(PRIME32_5);
        }

        h32 = h32.wrapping_add((*state).total_len_32);

        XXH32_finalize(
            h32,
            (*state).mem32.as_ptr() as *const c_void,
            (*state).memsize as usize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_canonicalFromHash(dst: *mut XXH32_canonical_t, hash: u32) {
    unsafe {
        let h = hash.swap_bytes();
        core::ptr::copy_nonoverlapping(&h as *const u32 as *const u8, dst as *mut u8, 4);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> u32 {
    unsafe { XXH_readBE32(src as *const u8) }
}

/* ===== 64-bit ===== */
const PRIME64_1: u64 = 11400714785074694791;
const PRIME64_2: u64 = 14029467366897019727;
const PRIME64_3: u64 = 1609587929392839161;
const PRIME64_4: u64 = 9650029242287828579;
const PRIME64_5: u64 = 2870177450012600261;

#[inline(always)]
fn XXH64_round(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = XXH_rotl64(acc, 31);
    acc = acc.wrapping_mul(PRIME64_1);
    acc
}

#[inline(always)]
fn XXH64_mergeRound(mut acc: u64, mut val: u64) -> u64 {
    val = XXH64_round(0, val);
    acc ^= val;
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

unsafe fn XXH64_finalize(mut h64: u64, ptr: *const c_void, len: usize) -> u64 {
    unsafe {
        let mut p = ptr as *const u8;

        macro_rules! PROCESS1_64 {
            () => {{
                h64 ^= (*p as u64).wrapping_mul(PRIME64_5);
                p = p.wrapping_add(1);
                h64 = XXH_rotl64(h64, 11).wrapping_mul(PRIME64_1);
            }};
        }
        macro_rules! PROCESS4_64 {
            () => {{
                h64 ^= (XXH_readLE32(p) as u64).wrapping_mul(PRIME64_1);
                p = p.wrapping_add(4);
                h64 = XXH_rotl64(h64, 23)
                    .wrapping_mul(PRIME64_2)
                    .wrapping_add(PRIME64_3);
            }};
        }
        macro_rules! PROCESS8_64 {
            () => {{
                let k1 = XXH64_round(0, XXH_readLE64(p));
                p = p.wrapping_add(8);
                h64 ^= k1;
                h64 = XXH_rotl64(h64, 27)
                    .wrapping_mul(PRIME64_1)
                    .wrapping_add(PRIME64_4);
            }};
        }

        match len & 31 {
            24 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                XXH64_avalanche(h64)
            }
            16 => {
                PROCESS8_64!();
                PROCESS8_64!();
                XXH64_avalanche(h64)
            }
            8 => {
                PROCESS8_64!();
                XXH64_avalanche(h64)
            }
            28 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                XXH64_avalanche(h64)
            }
            20 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                XXH64_avalanche(h64)
            }
            12 => {
                PROCESS8_64!();
                PROCESS4_64!();
                XXH64_avalanche(h64)
            }
            4 => {
                PROCESS4_64!();
                XXH64_avalanche(h64)
            }
            25 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            17 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            9 => {
                PROCESS8_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            29 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            21 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            13 => {
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            5 => {
                PROCESS4_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            26 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            18 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            10 => {
                PROCESS8_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            30 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            22 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            14 => {
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            6 => {
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            27 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            19 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            11 => {
                PROCESS8_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            31 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            23 => {
                PROCESS8_64!();
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            15 => {
                PROCESS8_64!();
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            7 => {
                PROCESS4_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            3 => {
                PROCESS1_64!();
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            2 => {
                PROCESS1_64!();
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            1 => {
                PROCESS1_64!();
                XXH64_avalanche(h64)
            }
            _ => XXH64_avalanche(h64),
        }
    }
}

unsafe fn XXH64_endian_align(input: *const c_void, len: usize, seed: u64) -> u64 {
    unsafe {
        let mut p = input as *const u8;
        let bEnd = p.wrapping_add(len);
        let mut h64: u64;

        if len >= 32 {
            let limit = bEnd.wrapping_sub(32);
            let mut v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
            let mut v2 = seed.wrapping_add(PRIME64_2);
            let mut v3 = seed;
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

            h64 = XXH_rotl64(v1, 1)
                .wrapping_add(XXH_rotl64(v2, 7))
                .wrapping_add(XXH_rotl64(v3, 12))
                .wrapping_add(XXH_rotl64(v4, 18));
            h64 = XXH64_mergeRound(h64, v1);
            h64 = XXH64_mergeRound(h64, v2);
            h64 = XXH64_mergeRound(h64, v3);
            h64 = XXH64_mergeRound(h64, v4);
        } else {
            h64 = seed.wrapping_add(PRIME64_5);
        }

        h64 = h64.wrapping_add(len as u64);

        XXH64_finalize(h64, p as *const c_void, len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64(input: *const c_void, len: usize, seed: u64) -> u64 {
    unsafe { XXH64_endian_align(input, len, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_createState() -> *mut XXH64_state_t {
    unsafe { malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_freeState(statePtr: *mut XXH64_state_t) -> c_int {
    unsafe {
        free(statePtr as *mut c_void);
        XXH_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_copyState(
    dstState: *mut XXH64_state_t,
    srcState: *const XXH64_state_t,
) {
    unsafe {
        core::ptr::copy_nonoverlapping(
            srcState as *const u8,
            dstState as *mut u8,
            core::mem::size_of::<XXH64_state_t>(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_reset(statePtr: *mut XXH64_state_t, seed: u64) -> c_int {
    unsafe {
        let mut state: XXH64_state_t = core::mem::zeroed();
        state.v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
        state.v2 = seed.wrapping_add(PRIME64_2);
        state.v3 = seed;
        state.v4 = seed.wrapping_sub(PRIME64_1);
        core::ptr::copy_nonoverlapping(
            &state as *const XXH64_state_t as *const u8,
            statePtr as *mut u8,
            core::mem::size_of::<XXH64_state_t>() - core::mem::size_of::<[u32; 2]>(),
        );
        XXH_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_update(
    state: *mut XXH64_state_t,
    input: *const c_void,
    len: usize,
) -> c_int {
    unsafe {
        if input.is_null() {
            return XXH_ERROR;
        }

        let mut p = input as *const u8;
        let bEnd = p.wrapping_add(len);

        (*state).total_len = (*state).total_len.wrapping_add(len as u64);

        if ((*state).memsize as usize) + len < 32 {
            core::ptr::copy_nonoverlapping(
                input as *const u8,
                ((*state).mem64.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                len,
            );
            (*state).memsize += len as u32;
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            core::ptr::copy_nonoverlapping(
                input as *const u8,
                ((*state).mem64.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                32 - (*state).memsize as usize,
            );
            let m = (*state).mem64.as_ptr();
            (*state).v1 = XXH64_round((*state).v1, XXH_readLE64(m.wrapping_add(0) as *const u8));
            (*state).v2 = XXH64_round((*state).v2, XXH_readLE64(m.wrapping_add(1) as *const u8));
            (*state).v3 = XXH64_round((*state).v3, XXH_readLE64(m.wrapping_add(2) as *const u8));
            (*state).v4 = XXH64_round((*state).v4, XXH_readLE64(m.wrapping_add(3) as *const u8));
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
            let n = (bEnd as usize) - (p as usize);
            core::ptr::copy_nonoverlapping(p, (*state).mem64.as_mut_ptr() as *mut u8, n);
            (*state).memsize = n as u32;
        }

        XXH_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_digest(state: *const XXH64_state_t) -> u64 {
    unsafe {
        let mut h64: u64;

        if (*state).total_len >= 32 {
            let v1 = (*state).v1;
            let v2 = (*state).v2;
            let v3 = (*state).v3;
            let v4 = (*state).v4;

            h64 = XXH_rotl64(v1, 1)
                .wrapping_add(XXH_rotl64(v2, 7))
                .wrapping_add(XXH_rotl64(v3, 12))
                .wrapping_add(XXH_rotl64(v4, 18));
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
            (*state).mem64.as_ptr() as *const c_void,
            (*state).total_len as usize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, hash: u64) {
    unsafe {
        let h = hash.swap_bytes();
        core::ptr::copy_nonoverlapping(&h as *const u64 as *const u8, dst as *mut u8, 8);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    unsafe { XXH_readBE64(src as *const u8) }
}
