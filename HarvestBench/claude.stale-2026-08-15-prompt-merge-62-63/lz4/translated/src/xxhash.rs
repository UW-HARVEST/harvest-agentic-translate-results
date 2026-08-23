// Translation of xxhash.c (xxHash 0.6.5), namespaced as LZ4_XXH* via XXH_NAMESPACE=LZ4_.
// Target: x86_64 little-endian. XXH_CPU_LITTLE_ENDIAN=1, XXH_FORCE_ALIGN_CHECK=0.
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::ptr;

pub type XXH_errorcode = core::ffi::c_int;
pub const XXH_OK: XXH_errorcode = 0;
pub const XXH_ERROR: XXH_errorcode = 1;

pub const XXH_VERSION_NUMBER: u32 = 0 * 100 * 100 + 6 * 100 + 5;

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

#[inline]
unsafe fn XXH_read32(p: *const u8) -> u32 {
    (p as *const u32).read_unaligned().to_le()
}
#[inline]
unsafe fn XXH_read64(p: *const u8) -> u64 {
    (p as *const u64).read_unaligned().to_le()
}

#[inline]
fn XXH_swap32(x: u32) -> u32 {
    x.swap_bytes()
}
#[inline]
fn XXH_swap64(x: u64) -> u64 {
    x.swap_bytes()
}

#[inline]
fn XXH_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}
#[inline]
fn XXH_rotl64(x: u64, r: u32) -> u64 {
    (x << r) | (x >> (64 - r))
}

// little-endian target
#[inline]
unsafe fn XXH_readLE32(p: *const u8) -> u32 {
    XXH_read32(p)
}
#[inline]
unsafe fn XXH_readLE64(p: *const u8) -> u64 {
    XXH_read64(p)
}
#[inline]
unsafe fn XXH_readBE32(p: *const u8) -> u32 {
    XXH_swap32(XXH_read32(p))
}
#[inline]
unsafe fn XXH_readBE64(p: *const u8) -> u64 {
    XXH_swap64(XXH_read64(p))
}

const PRIME32_1: u32 = 2654435761;
const PRIME32_2: u32 = 2246822519;
const PRIME32_3: u32 = 3266489917;
const PRIME32_4: u32 = 668265263;
const PRIME32_5: u32 = 374761393;

#[inline]
fn XXH32_round(seed: u32, input: u32) -> u32 {
    let mut seed = seed.wrapping_add(input.wrapping_mul(PRIME32_2));
    seed = XXH_rotl32(seed, 13);
    seed = seed.wrapping_mul(PRIME32_1);
    seed
}

#[inline]
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
    macro_rules! process1 {
        () => {{
            h32 = h32.wrapping_add((*p as u32).wrapping_mul(PRIME32_5));
            p = p.add(1);
            h32 = XXH_rotl32(h32, 11).wrapping_mul(PRIME32_1);
        }};
    }
    macro_rules! process4 {
        () => {{
            h32 = h32.wrapping_add(XXH_readLE32(p).wrapping_mul(PRIME32_3));
            p = p.add(4);
            h32 = XXH_rotl32(h32, 17).wrapping_mul(PRIME32_4);
        }};
    }
    match len & 15 {
        12 => {
            process4!();
            process4!();
            process4!();
            XXH32_avalanche(h32)
        }
        8 => {
            process4!();
            process4!();
            XXH32_avalanche(h32)
        }
        4 => {
            process4!();
            XXH32_avalanche(h32)
        }
        13 => {
            process4!();
            process4!();
            process4!();
            process1!();
            XXH32_avalanche(h32)
        }
        9 => {
            process4!();
            process4!();
            process1!();
            XXH32_avalanche(h32)
        }
        5 => {
            process4!();
            process1!();
            XXH32_avalanche(h32)
        }
        14 => {
            process4!();
            process4!();
            process4!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        10 => {
            process4!();
            process4!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        6 => {
            process4!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        15 => {
            process4!();
            process4!();
            process4!();
            process1!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        11 => {
            process4!();
            process4!();
            process1!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        7 => {
            process4!();
            process1!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        3 => {
            process1!();
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        2 => {
            process1!();
            process1!();
            XXH32_avalanche(h32)
        }
        1 => {
            process1!();
            XXH32_avalanche(h32)
        }
        0 => XXH32_avalanche(h32),
        _ => h32,
    }
}

unsafe fn XXH32_endian_align(input: *const u8, len: usize, seed: u32) -> u32 {
    let mut p = input;
    let b_end = p.add(len);
    let mut h32: u32;

    if len >= 16 {
        let limit = b_end.sub(15);
        let mut v1 = seed
            .wrapping_add(PRIME32_1)
            .wrapping_add(PRIME32_2);
        let mut v2 = seed.wrapping_add(PRIME32_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(PRIME32_1);

        loop {
            v1 = XXH32_round(v1, XXH_readLE32(p));
            p = p.add(4);
            v2 = XXH32_round(v2, XXH_readLE32(p));
            p = p.add(4);
            v3 = XXH32_round(v3, XXH_readLE32(p));
            p = p.add(4);
            v4 = XXH32_round(v4, XXH_readLE32(p));
            p = p.add(4);
            if p >= limit {
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

    XXH32_finalize(h32, p, len & 15)
}

pub unsafe fn xxh32(input: *const u8, len: usize, seed: u32) -> u32 {
    XXH32_endian_align(input, len, seed)
}

pub unsafe fn xxh32_reset(state_ptr: *mut XXH32_state_t, seed: u32) -> XXH_errorcode {
    let mut state: XXH32_state_t = core::mem::zeroed();
    state.v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
    state.v2 = seed.wrapping_add(PRIME32_2);
    state.v3 = seed.wrapping_add(0);
    state.v4 = seed.wrapping_sub(PRIME32_1);
    // do not write into reserved
    ptr::copy_nonoverlapping(
        &state as *const XXH32_state_t as *const u8,
        state_ptr as *mut u8,
        core::mem::size_of::<XXH32_state_t>() - core::mem::size_of::<u32>(),
    );
    XXH_OK
}

pub unsafe fn xxh32_update(
    state: *mut XXH32_state_t,
    input: *const u8,
    len: usize,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_ERROR;
    }
    let state = &mut *state;
    let mut p = input;
    let b_end = p.add(len);

    state.total_len_32 = state.total_len_32.wrapping_add(len as u32);
    state.large_len |= ((len >= 16) as u32) | ((state.total_len_32 >= 16) as u32);

    if (state.memsize as usize) + len < 16 {
        // fill in tmp buffer
        let dst = (state.mem32.as_mut_ptr() as *mut u8).add(state.memsize as usize);
        ptr::copy_nonoverlapping(input, dst, len);
        state.memsize += len as u32;
        return XXH_OK;
    }

    if state.memsize != 0 {
        let dst = (state.mem32.as_mut_ptr() as *mut u8).add(state.memsize as usize);
        ptr::copy_nonoverlapping(input, dst, 16 - state.memsize as usize);
        let mut p32 = state.mem32.as_ptr();
        state.v1 = XXH32_round(state.v1, XXH_readLE32(p32 as *const u8));
        p32 = p32.add(1);
        state.v2 = XXH32_round(state.v2, XXH_readLE32(p32 as *const u8));
        p32 = p32.add(1);
        state.v3 = XXH32_round(state.v3, XXH_readLE32(p32 as *const u8));
        p32 = p32.add(1);
        state.v4 = XXH32_round(state.v4, XXH_readLE32(p32 as *const u8));
        p = p.add(16 - state.memsize as usize);
        state.memsize = 0;
    }

    if p <= b_end.sub(16) {
        let limit = b_end.sub(16);
        let mut v1 = state.v1;
        let mut v2 = state.v2;
        let mut v3 = state.v3;
        let mut v4 = state.v4;

        loop {
            v1 = XXH32_round(v1, XXH_readLE32(p));
            p = p.add(4);
            v2 = XXH32_round(v2, XXH_readLE32(p));
            p = p.add(4);
            v3 = XXH32_round(v3, XXH_readLE32(p));
            p = p.add(4);
            v4 = XXH32_round(v4, XXH_readLE32(p));
            p = p.add(4);
            if p > limit {
                break;
            }
        }

        state.v1 = v1;
        state.v2 = v2;
        state.v3 = v3;
        state.v4 = v4;
    }

    if p < b_end {
        ptr::copy_nonoverlapping(
            p,
            state.mem32.as_mut_ptr() as *mut u8,
            b_end as usize - p as usize,
        );
        state.memsize = (b_end as usize - p as usize) as u32;
    }

    XXH_OK
}

pub unsafe fn xxh32_digest(state: *const XXH32_state_t) -> u32 {
    let state = &*state;
    let mut h32: u32;
    if state.large_len != 0 {
        h32 = XXH_rotl32(state.v1, 1)
            .wrapping_add(XXH_rotl32(state.v2, 7))
            .wrapping_add(XXH_rotl32(state.v3, 12))
            .wrapping_add(XXH_rotl32(state.v4, 18));
    } else {
        h32 = state.v3.wrapping_add(PRIME32_5);
    }
    h32 = h32.wrapping_add(state.total_len_32);
    XXH32_finalize(
        h32,
        state.mem32.as_ptr() as *const u8,
        state.memsize as usize,
    )
}

const PRIME64_1: u64 = 11400714785074694791;
const PRIME64_2: u64 = 14029467366897019727;
const PRIME64_3: u64 = 1609587929392839161;
const PRIME64_4: u64 = 9650029242287828579;
const PRIME64_5: u64 = 2870177450012600261;

#[inline]
fn XXH64_round(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = XXH_rotl64(acc, 31);
    acc = acc.wrapping_mul(PRIME64_1);
    acc
}

#[inline]
fn XXH64_mergeRound(acc: u64, val: u64) -> u64 {
    let val = XXH64_round(0, val);
    let mut acc = acc ^ val;
    acc = acc.wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4);
    acc
}

#[inline]
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
    macro_rules! process1_64 {
        () => {{
            h64 ^= (*p as u64).wrapping_mul(PRIME64_5);
            p = p.add(1);
            h64 = XXH_rotl64(h64, 11).wrapping_mul(PRIME64_1);
        }};
    }
    macro_rules! process4_64 {
        () => {{
            h64 ^= (XXH_readLE32(p) as u64).wrapping_mul(PRIME64_1);
            p = p.add(4);
            h64 = XXH_rotl64(h64, 23)
                .wrapping_mul(PRIME64_2)
                .wrapping_add(PRIME64_3);
        }};
    }
    macro_rules! process8_64 {
        () => {{
            let k1 = XXH64_round(0, XXH_readLE64(p));
            p = p.add(8);
            h64 ^= k1;
            h64 = XXH_rotl64(h64, 27)
                .wrapping_mul(PRIME64_1)
                .wrapping_add(PRIME64_4);
        }};
    }
    match len & 31 {
        24 => {
            process8_64!();
            process8_64!();
            process8_64!();
            XXH64_avalanche(h64)
        }
        16 => {
            process8_64!();
            process8_64!();
            XXH64_avalanche(h64)
        }
        8 => {
            process8_64!();
            XXH64_avalanche(h64)
        }
        28 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            XXH64_avalanche(h64)
        }
        20 => {
            process8_64!();
            process8_64!();
            process4_64!();
            XXH64_avalanche(h64)
        }
        12 => {
            process8_64!();
            process4_64!();
            XXH64_avalanche(h64)
        }
        4 => {
            process4_64!();
            XXH64_avalanche(h64)
        }
        25 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        17 => {
            process8_64!();
            process8_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        9 => {
            process8_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        29 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        21 => {
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        13 => {
            process8_64!();
            process4_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        5 => {
            process4_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        26 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        18 => {
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        10 => {
            process8_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        30 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        22 => {
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        14 => {
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        6 => {
            process4_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        27 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        19 => {
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        11 => {
            process8_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        31 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        23 => {
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        15 => {
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        7 => {
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        3 => {
            process1_64!();
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        2 => {
            process1_64!();
            process1_64!();
            XXH64_avalanche(h64)
        }
        1 => {
            process1_64!();
            XXH64_avalanche(h64)
        }
        0 => XXH64_avalanche(h64),
        _ => 0,
    }
}

unsafe fn XXH64_endian_align(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end = p.add(len);
    let mut h64: u64;

    if len >= 32 {
        let limit = b_end.sub(32);
        let mut v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
        let mut v2 = seed.wrapping_add(PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(PRIME64_1);

        loop {
            v1 = XXH64_round(v1, XXH_readLE64(p));
            p = p.add(8);
            v2 = XXH64_round(v2, XXH_readLE64(p));
            p = p.add(8);
            v3 = XXH64_round(v3, XXH_readLE64(p));
            p = p.add(8);
            v4 = XXH64_round(v4, XXH_readLE64(p));
            p = p.add(8);
            if p > limit {
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

    XXH64_finalize(h64, p, len)
}

pub unsafe fn xxh64(input: *const u8, len: usize, seed: u64) -> u64 {
    XXH64_endian_align(input, len, seed)
}

pub unsafe fn xxh64_reset(state_ptr: *mut XXH64_state_t, seed: u64) -> XXH_errorcode {
    let mut state: XXH64_state_t = core::mem::zeroed();
    state.v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
    state.v2 = seed.wrapping_add(PRIME64_2);
    state.v3 = seed.wrapping_add(0);
    state.v4 = seed.wrapping_sub(PRIME64_1);
    ptr::copy_nonoverlapping(
        &state as *const XXH64_state_t as *const u8,
        state_ptr as *mut u8,
        core::mem::size_of::<XXH64_state_t>() - core::mem::size_of::<[u32; 2]>(),
    );
    XXH_OK
}

pub unsafe fn xxh64_update(
    state: *mut XXH64_state_t,
    input: *const u8,
    len: usize,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_ERROR;
    }
    let state = &mut *state;
    let mut p = input;
    let b_end = p.add(len);

    state.total_len = state.total_len.wrapping_add(len as u64);

    if (state.memsize as usize) + len < 32 {
        let dst = (state.mem64.as_mut_ptr() as *mut u8).add(state.memsize as usize);
        ptr::copy_nonoverlapping(input, dst, len);
        state.memsize += len as u32;
        return XXH_OK;
    }

    if state.memsize != 0 {
        let dst = (state.mem64.as_mut_ptr() as *mut u8).add(state.memsize as usize);
        ptr::copy_nonoverlapping(input, dst, 32 - state.memsize as usize);
        state.v1 = XXH64_round(state.v1, XXH_readLE64(state.mem64.as_ptr().add(0) as *const u8));
        state.v2 = XXH64_round(state.v2, XXH_readLE64(state.mem64.as_ptr().add(1) as *const u8));
        state.v3 = XXH64_round(state.v3, XXH_readLE64(state.mem64.as_ptr().add(2) as *const u8));
        state.v4 = XXH64_round(state.v4, XXH_readLE64(state.mem64.as_ptr().add(3) as *const u8));
        p = p.add(32 - state.memsize as usize);
        state.memsize = 0;
    }

    if p.add(32) <= b_end {
        let limit = b_end.sub(32);
        let mut v1 = state.v1;
        let mut v2 = state.v2;
        let mut v3 = state.v3;
        let mut v4 = state.v4;

        loop {
            v1 = XXH64_round(v1, XXH_readLE64(p));
            p = p.add(8);
            v2 = XXH64_round(v2, XXH_readLE64(p));
            p = p.add(8);
            v3 = XXH64_round(v3, XXH_readLE64(p));
            p = p.add(8);
            v4 = XXH64_round(v4, XXH_readLE64(p));
            p = p.add(8);
            if p > limit {
                break;
            }
        }

        state.v1 = v1;
        state.v2 = v2;
        state.v3 = v3;
        state.v4 = v4;
    }

    if p < b_end {
        ptr::copy_nonoverlapping(
            p,
            state.mem64.as_mut_ptr() as *mut u8,
            b_end as usize - p as usize,
        );
        state.memsize = (b_end as usize - p as usize) as u32;
    }

    XXH_OK
}

pub unsafe fn xxh64_digest(state: *const XXH64_state_t) -> u64 {
    let state = &*state;
    let mut h64: u64;
    if state.total_len >= 32 {
        let v1 = state.v1;
        let v2 = state.v2;
        let v3 = state.v3;
        let v4 = state.v4;
        h64 = XXH_rotl64(v1, 1)
            .wrapping_add(XXH_rotl64(v2, 7))
            .wrapping_add(XXH_rotl64(v3, 12))
            .wrapping_add(XXH_rotl64(v4, 18));
        h64 = XXH64_mergeRound(h64, v1);
        h64 = XXH64_mergeRound(h64, v2);
        h64 = XXH64_mergeRound(h64, v3);
        h64 = XXH64_mergeRound(h64, v4);
    } else {
        h64 = state.v3.wrapping_add(PRIME64_5);
    }
    h64 = h64.wrapping_add(state.total_len);
    XXH64_finalize(
        h64,
        state.mem64.as_ptr() as *const u8,
        state.total_len as usize,
    )
}

// ============ allocation helpers (malloc/free semantics via crate::alloc) ============
pub unsafe fn xxh32_create_state() -> *mut XXH32_state_t {
    crate::c_malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}
pub unsafe fn xxh32_free_state(p: *mut XXH32_state_t) -> XXH_errorcode {
    crate::c_free(p as *mut u8);
    XXH_OK
}
pub unsafe fn xxh64_create_state() -> *mut XXH64_state_t {
    crate::c_malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}
pub unsafe fn xxh64_free_state(p: *mut XXH64_state_t) -> XXH_errorcode {
    crate::c_free(p as *mut u8);
    XXH_OK
}

// ============ Public C ABI (namespaced LZ4_XXH*) ============

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_XXH_versionNumber() -> core::ffi::c_uint {
    XXH_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32(
    input: *const core::ffi::c_void,
    len: usize,
    seed: core::ffi::c_uint,
) -> u32 {
    xxh32(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_createState() -> *mut XXH32_state_t {
    xxh32_create_state()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_freeState(p: *mut XXH32_state_t) -> XXH_errorcode {
    xxh32_free_state(p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_copyState(
    dst: *mut XXH32_state_t,
    src: *const XXH32_state_t,
) {
    ptr::copy_nonoverlapping(src, dst, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_reset(
    state_ptr: *mut XXH32_state_t,
    seed: core::ffi::c_uint,
) -> XXH_errorcode {
    xxh32_reset(state_ptr, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_update(
    state: *mut XXH32_state_t,
    input: *const core::ffi::c_void,
    len: usize,
) -> XXH_errorcode {
    xxh32_update(state, input as *const u8, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_digest(state: *const XXH32_state_t) -> u32 {
    xxh32_digest(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_canonicalFromHash(
    dst: *mut XXH32_canonical_t,
    hash: u32,
) {
    // sizeof canonical == sizeof hash
    let h = XXH_swap32(hash); // CPU little-endian -> swap
    ptr::copy_nonoverlapping(
        &h as *const u32 as *const u8,
        dst as *mut u8,
        core::mem::size_of::<XXH32_canonical_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> u32 {
    XXH_readBE32(src as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64(
    input: *const core::ffi::c_void,
    len: usize,
    seed: core::ffi::c_ulonglong,
) -> u64 {
    xxh64(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_createState() -> *mut XXH64_state_t {
    xxh64_create_state()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_freeState(p: *mut XXH64_state_t) -> XXH_errorcode {
    xxh64_free_state(p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_copyState(
    dst: *mut XXH64_state_t,
    src: *const XXH64_state_t,
) {
    ptr::copy_nonoverlapping(src, dst, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_reset(
    state_ptr: *mut XXH64_state_t,
    seed: core::ffi::c_ulonglong,
) -> XXH_errorcode {
    xxh64_reset(state_ptr, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_update(
    state: *mut XXH64_state_t,
    input: *const core::ffi::c_void,
    len: usize,
) -> XXH_errorcode {
    xxh64_update(state, input as *const u8, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_digest(state: *const XXH64_state_t) -> u64 {
    xxh64_digest(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_canonicalFromHash(
    dst: *mut XXH64_canonical_t,
    hash: u64,
) {
    let h = XXH_swap64(hash);
    ptr::copy_nonoverlapping(
        &h as *const u64 as *const u8,
        dst as *mut u8,
        core::mem::size_of::<XXH64_canonical_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    XXH_readBE64(src as *const u8)
}
