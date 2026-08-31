//! Translation of `xxhash.c` (xxHash 0.6.5, as bundled with lz4 1.10.0).
//!
//! Built with `XXH_NAMESPACE=LZ4_`, so all public linker symbols are prefixed
//! with `LZ4_`.

use core::ffi::{c_uint, c_void};

pub type XXH32Hash = u32;
pub type XXH64Hash = u64;

pub const XXH_OK: c_uint = 0;
pub const XXH_ERROR: c_uint = 1;

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 6;
pub const XXH_VERSION_RELEASE: u32 = 5;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH32State {
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

impl XXH32State {
    pub const fn new() -> Self {
        XXH32State {
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH64State {
    pub total_len: u64,
    pub v1: u64,
    pub v2: u64,
    pub v3: u64,
    pub v4: u64,
    pub mem64: [u64; 4],
    pub memsize: u32,
    pub reserved: [u32; 2],
}

impl XXH64State {
    pub const fn new() -> Self {
        XXH64State {
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH32Canonical {
    pub digest: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH64Canonical {
    pub digest: [u8; 8],
}

/* ===== memory access helpers ===== */

#[inline(always)]
unsafe fn xxh_read32(p: *const u8) -> u32 {
    unsafe { core::ptr::read_unaligned(p as *const u32) }
}

#[inline(always)]
unsafe fn xxh_read64(p: *const u8) -> u64 {
    unsafe { core::ptr::read_unaligned(p as *const u64) }
}

/* The reference build targets little-endian hosts (x86_64), where
 * XXH_readLE32 == XXH_read32. Use to_le() so the byte-level behaviour is the
 * same on either endianness of host. */
#[inline(always)]
unsafe fn xxh_read_le32(p: *const u8) -> u32 {
    unsafe { u32::from_le(xxh_read32(p)) }
}

#[inline(always)]
unsafe fn xxh_read_le64(p: *const u8) -> u64 {
    unsafe { u64::from_le(xxh_read64(p)) }
}

/* ===== 32-bit hash ===== */

const PRIME32_1: u32 = 2654435761;
const PRIME32_2: u32 = 2246822519;
const PRIME32_3: u32 = 3266489917;
const PRIME32_4: u32 = 668265263;
const PRIME32_5: u32 = 374761393;

#[inline(always)]
fn xxh32_round(seed: u32, input: u32) -> u32 {
    let mut seed = seed.wrapping_add(input.wrapping_mul(PRIME32_2));
    seed = seed.rotate_left(13);
    seed = seed.wrapping_mul(PRIME32_1);
    seed
}

#[inline(always)]
fn xxh32_avalanche(mut h32: u32) -> u32 {
    h32 ^= h32 >> 15;
    h32 = h32.wrapping_mul(PRIME32_2);
    h32 ^= h32 >> 13;
    h32 = h32.wrapping_mul(PRIME32_3);
    h32 ^= h32 >> 16;
    h32
}

unsafe fn xxh32_finalize(mut h32: u32, ptr: *const u8, len: usize) -> u32 {
    let mut p = ptr;

    macro_rules! process1 {
        () => {{
            h32 = h32.wrapping_add((unsafe { *p } as u32).wrapping_mul(PRIME32_5));
            p = unsafe { p.add(1) };
            h32 = h32.rotate_left(11).wrapping_mul(PRIME32_1);
        }};
    }
    macro_rules! process4 {
        () => {{
            h32 = h32.wrapping_add(unsafe { xxh_read_le32(p) }.wrapping_mul(PRIME32_3));
            p = unsafe { p.add(4) };
            h32 = h32.rotate_left(17).wrapping_mul(PRIME32_4);
        }};
    }

    match len & 15 {
        12 => {
            process4!();
            process4!();
            process4!();
            xxh32_avalanche(h32)
        }
        8 => {
            process4!();
            process4!();
            xxh32_avalanche(h32)
        }
        4 => {
            process4!();
            xxh32_avalanche(h32)
        }
        13 => {
            process4!();
            process4!();
            process4!();
            process1!();
            xxh32_avalanche(h32)
        }
        9 => {
            process4!();
            process4!();
            process1!();
            xxh32_avalanche(h32)
        }
        5 => {
            process4!();
            process1!();
            xxh32_avalanche(h32)
        }
        14 => {
            process4!();
            process4!();
            process4!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        10 => {
            process4!();
            process4!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        6 => {
            process4!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        15 => {
            process4!();
            process4!();
            process4!();
            process1!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        11 => {
            process4!();
            process4!();
            process1!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        7 => {
            process4!();
            process1!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        3 => {
            process1!();
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        2 => {
            process1!();
            process1!();
            xxh32_avalanche(h32)
        }
        1 => {
            process1!();
            xxh32_avalanche(h32)
        }
        _ => xxh32_avalanche(h32),
    }
}

pub unsafe fn xxh32(input: *const u8, len: usize, seed: u32) -> u32 {
    unsafe {
        let mut p = input;
        let b_end = p.add(len);
        let mut h32: u32;

        if len >= 16 {
            let limit = b_end.sub(15);
            let mut v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
            let mut v2 = seed.wrapping_add(PRIME32_2);
            let mut v3 = seed;
            let mut v4 = seed.wrapping_sub(PRIME32_1);

            loop {
                v1 = xxh32_round(v1, xxh_read_le32(p));
                p = p.add(4);
                v2 = xxh32_round(v2, xxh_read_le32(p));
                p = p.add(4);
                v3 = xxh32_round(v3, xxh_read_le32(p));
                p = p.add(4);
                v4 = xxh32_round(v4, xxh_read_le32(p));
                p = p.add(4);
                if p >= limit {
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

        xxh32_finalize(h32, p, len & 15)
    }
}

pub fn xxh32_reset(state: &mut XXH32State, seed: u32) {
    let mut s = XXH32State::new();
    s.v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
    s.v2 = seed.wrapping_add(PRIME32_2);
    s.v3 = seed;
    s.v4 = seed.wrapping_sub(PRIME32_1);
    /* do not write into reserved */
    let reserved = state.reserved;
    *state = s;
    state.reserved = reserved;
}

pub unsafe fn xxh32_update(state: &mut XXH32State, input: *const u8, len: usize) -> c_uint {
    unsafe {
        if input.is_null() {
            return XXH_ERROR;
        }

        let mut p = input;
        let b_end = p.add(len);

        state.total_len_32 = state.total_len_32.wrapping_add(len as u32);
        state.large_len |= ((len >= 16) as u32) | ((state.total_len_32 >= 16) as u32);

        if (state.memsize as usize) + len < 16 {
            let dst = (state.mem32.as_mut_ptr() as *mut u8).add(state.memsize as usize);
            core::ptr::copy_nonoverlapping(input, dst, len);
            state.memsize += len as u32;
            return XXH_OK;
        }

        if state.memsize != 0 {
            let dst = (state.mem32.as_mut_ptr() as *mut u8).add(state.memsize as usize);
            core::ptr::copy_nonoverlapping(input, dst, 16 - state.memsize as usize);
            {
                let mut p32 = state.mem32.as_ptr() as *const u8;
                state.v1 = xxh32_round(state.v1, xxh_read_le32(p32));
                p32 = p32.add(4);
                state.v2 = xxh32_round(state.v2, xxh_read_le32(p32));
                p32 = p32.add(4);
                state.v3 = xxh32_round(state.v3, xxh_read_le32(p32));
                p32 = p32.add(4);
                state.v4 = xxh32_round(state.v4, xxh_read_le32(p32));
            }
            p = p.add(16 - state.memsize as usize);
            state.memsize = 0;
        }

        if p as usize <= (b_end as usize) - 16 {
            let limit = b_end.sub(16);
            let mut v1 = state.v1;
            let mut v2 = state.v2;
            let mut v3 = state.v3;
            let mut v4 = state.v4;

            loop {
                v1 = xxh32_round(v1, xxh_read_le32(p));
                p = p.add(4);
                v2 = xxh32_round(v2, xxh_read_le32(p));
                p = p.add(4);
                v3 = xxh32_round(v3, xxh_read_le32(p));
                p = p.add(4);
                v4 = xxh32_round(v4, xxh_read_le32(p));
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
            let n = b_end as usize - p as usize;
            core::ptr::copy_nonoverlapping(p, state.mem32.as_mut_ptr() as *mut u8, n);
            state.memsize = n as u32;
        }

        XXH_OK
    }
}

pub fn xxh32_digest(state: &XXH32State) -> u32 {
    let mut h32: u32;

    if state.large_len != 0 {
        h32 = state
            .v1
            .rotate_left(1)
            .wrapping_add(state.v2.rotate_left(7))
            .wrapping_add(state.v3.rotate_left(12))
            .wrapping_add(state.v4.rotate_left(18));
    } else {
        h32 = state.v3.wrapping_add(PRIME32_5);
    }

    h32 = h32.wrapping_add(state.total_len_32);

    unsafe {
        xxh32_finalize(
            h32,
            state.mem32.as_ptr() as *const u8,
            state.memsize as usize,
        )
    }
}

/* ===== 64-bit hash ===== */

const PRIME64_1: u64 = 11400714785074694791;
const PRIME64_2: u64 = 14029467366897019727;
const PRIME64_3: u64 = 1609587929392839161;
const PRIME64_4: u64 = 9650029242287828579;
const PRIME64_5: u64 = 2870177450012600261;

#[inline(always)]
fn xxh64_round(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = acc.rotate_left(31);
    acc = acc.wrapping_mul(PRIME64_1);
    acc
}

#[inline(always)]
fn xxh64_merge_round(acc: u64, val: u64) -> u64 {
    let val = xxh64_round(0, val);
    let mut acc = acc ^ val;
    acc = acc.wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4);
    acc
}

#[inline(always)]
fn xxh64_avalanche(mut h64: u64) -> u64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

unsafe fn xxh64_finalize(mut h64: u64, ptr: *const u8, len: usize) -> u64 {
    let mut p = ptr;

    macro_rules! process1_64 {
        () => {{
            h64 ^= (unsafe { *p } as u64).wrapping_mul(PRIME64_5);
            p = unsafe { p.add(1) };
            h64 = h64.rotate_left(11).wrapping_mul(PRIME64_1);
        }};
    }
    macro_rules! process4_64 {
        () => {{
            h64 ^= (unsafe { xxh_read_le32(p) } as u64).wrapping_mul(PRIME64_1);
            p = unsafe { p.add(4) };
            h64 = h64
                .rotate_left(23)
                .wrapping_mul(PRIME64_2)
                .wrapping_add(PRIME64_3);
        }};
    }
    macro_rules! process8_64 {
        () => {{
            let k1 = xxh64_round(0, unsafe { xxh_read_le64(p) });
            p = unsafe { p.add(8) };
            h64 ^= k1;
            h64 = h64
                .rotate_left(27)
                .wrapping_mul(PRIME64_1)
                .wrapping_add(PRIME64_4);
        }};
    }

    macro_rules! p8 {
        ($n:expr) => {{
            for _ in 0..$n {
                process8_64!();
            }
        }};
    }
    macro_rules! p1 {
        ($n:expr) => {{
            for _ in 0..$n {
                process1_64!();
            }
        }};
    }

    match len & 31 {
        24 => {
            p8!(3);
            xxh64_avalanche(h64)
        }
        16 => {
            p8!(2);
            xxh64_avalanche(h64)
        }
        8 => {
            p8!(1);
            xxh64_avalanche(h64)
        }
        28 => {
            p8!(3);
            process4_64!();
            xxh64_avalanche(h64)
        }
        20 => {
            p8!(2);
            process4_64!();
            xxh64_avalanche(h64)
        }
        12 => {
            p8!(1);
            process4_64!();
            xxh64_avalanche(h64)
        }
        4 => {
            process4_64!();
            xxh64_avalanche(h64)
        }
        25 => {
            p8!(3);
            p1!(1);
            xxh64_avalanche(h64)
        }
        17 => {
            p8!(2);
            p1!(1);
            xxh64_avalanche(h64)
        }
        9 => {
            p8!(1);
            p1!(1);
            xxh64_avalanche(h64)
        }
        29 => {
            p8!(3);
            process4_64!();
            p1!(1);
            xxh64_avalanche(h64)
        }
        21 => {
            p8!(2);
            process4_64!();
            p1!(1);
            xxh64_avalanche(h64)
        }
        13 => {
            p8!(1);
            process4_64!();
            p1!(1);
            xxh64_avalanche(h64)
        }
        5 => {
            process4_64!();
            p1!(1);
            xxh64_avalanche(h64)
        }
        26 => {
            p8!(3);
            p1!(2);
            xxh64_avalanche(h64)
        }
        18 => {
            p8!(2);
            p1!(2);
            xxh64_avalanche(h64)
        }
        10 => {
            p8!(1);
            p1!(2);
            xxh64_avalanche(h64)
        }
        30 => {
            p8!(3);
            process4_64!();
            p1!(2);
            xxh64_avalanche(h64)
        }
        22 => {
            p8!(2);
            process4_64!();
            p1!(2);
            xxh64_avalanche(h64)
        }
        14 => {
            p8!(1);
            process4_64!();
            p1!(2);
            xxh64_avalanche(h64)
        }
        6 => {
            process4_64!();
            p1!(2);
            xxh64_avalanche(h64)
        }
        27 => {
            p8!(3);
            p1!(3);
            xxh64_avalanche(h64)
        }
        19 => {
            p8!(2);
            p1!(3);
            xxh64_avalanche(h64)
        }
        11 => {
            p8!(1);
            p1!(3);
            xxh64_avalanche(h64)
        }
        31 => {
            p8!(3);
            process4_64!();
            p1!(3);
            xxh64_avalanche(h64)
        }
        23 => {
            p8!(2);
            process4_64!();
            p1!(3);
            xxh64_avalanche(h64)
        }
        15 => {
            p8!(1);
            process4_64!();
            p1!(3);
            xxh64_avalanche(h64)
        }
        7 => {
            process4_64!();
            p1!(3);
            xxh64_avalanche(h64)
        }
        3 => {
            p1!(3);
            xxh64_avalanche(h64)
        }
        2 => {
            p1!(2);
            xxh64_avalanche(h64)
        }
        1 => {
            p1!(1);
            xxh64_avalanche(h64)
        }
        _ => xxh64_avalanche(h64),
    }
}

pub unsafe fn xxh64(input: *const u8, len: usize, seed: u64) -> u64 {
    unsafe {
        let mut p = input;
        let b_end = p.add(len);
        let mut h64: u64;

        if len >= 32 {
            let limit = b_end.sub(32);
            let mut v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
            let mut v2 = seed.wrapping_add(PRIME64_2);
            let mut v3 = seed;
            let mut v4 = seed.wrapping_sub(PRIME64_1);

            loop {
                v1 = xxh64_round(v1, xxh_read_le64(p));
                p = p.add(8);
                v2 = xxh64_round(v2, xxh_read_le64(p));
                p = p.add(8);
                v3 = xxh64_round(v3, xxh_read_le64(p));
                p = p.add(8);
                v4 = xxh64_round(v4, xxh_read_le64(p));
                p = p.add(8);
                if p > limit {
                    break;
                }
            }

            h64 = v1
                .rotate_left(1)
                .wrapping_add(v2.rotate_left(7))
                .wrapping_add(v3.rotate_left(12))
                .wrapping_add(v4.rotate_left(18));
            h64 = xxh64_merge_round(h64, v1);
            h64 = xxh64_merge_round(h64, v2);
            h64 = xxh64_merge_round(h64, v3);
            h64 = xxh64_merge_round(h64, v4);
        } else {
            h64 = seed.wrapping_add(PRIME64_5);
        }

        h64 = h64.wrapping_add(len as u64);

        xxh64_finalize(h64, p, len)
    }
}

pub fn xxh64_reset(state: &mut XXH64State, seed: u64) {
    let mut s = XXH64State::new();
    s.v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
    s.v2 = seed.wrapping_add(PRIME64_2);
    s.v3 = seed;
    s.v4 = seed.wrapping_sub(PRIME64_1);
    let reserved = state.reserved;
    *state = s;
    state.reserved = reserved;
}

pub unsafe fn xxh64_update(state: &mut XXH64State, input: *const u8, len: usize) -> c_uint {
    unsafe {
        if input.is_null() {
            return XXH_ERROR;
        }

        let mut p = input;
        let b_end = p.add(len);

        state.total_len = state.total_len.wrapping_add(len as u64);

        if (state.memsize as usize) + len < 32 {
            let dst = (state.mem64.as_mut_ptr() as *mut u8).add(state.memsize as usize);
            core::ptr::copy_nonoverlapping(input, dst, len);
            state.memsize += len as u32;
            return XXH_OK;
        }

        if state.memsize != 0 {
            let dst = (state.mem64.as_mut_ptr() as *mut u8).add(state.memsize as usize);
            core::ptr::copy_nonoverlapping(input, dst, 32 - state.memsize as usize);
            let m = state.mem64.as_ptr() as *const u8;
            state.v1 = xxh64_round(state.v1, xxh_read_le64(m));
            state.v2 = xxh64_round(state.v2, xxh_read_le64(m.add(8)));
            state.v3 = xxh64_round(state.v3, xxh_read_le64(m.add(16)));
            state.v4 = xxh64_round(state.v4, xxh_read_le64(m.add(24)));
            p = p.add(32 - state.memsize as usize);
            state.memsize = 0;
        }

        if (p as usize) + 32 <= b_end as usize {
            let limit = b_end.sub(32);
            let mut v1 = state.v1;
            let mut v2 = state.v2;
            let mut v3 = state.v3;
            let mut v4 = state.v4;

            loop {
                v1 = xxh64_round(v1, xxh_read_le64(p));
                p = p.add(8);
                v2 = xxh64_round(v2, xxh_read_le64(p));
                p = p.add(8);
                v3 = xxh64_round(v3, xxh_read_le64(p));
                p = p.add(8);
                v4 = xxh64_round(v4, xxh_read_le64(p));
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
            let n = b_end as usize - p as usize;
            core::ptr::copy_nonoverlapping(p, state.mem64.as_mut_ptr() as *mut u8, n);
            state.memsize = n as u32;
        }

        XXH_OK
    }
}

pub fn xxh64_digest(state: &XXH64State) -> u64 {
    let mut h64: u64;

    if state.total_len >= 32 {
        let v1 = state.v1;
        let v2 = state.v2;
        let v3 = state.v3;
        let v4 = state.v4;

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = state.v3.wrapping_add(PRIME64_5);
    }

    h64 = h64.wrapping_add(state.total_len);

    unsafe {
        xxh64_finalize(
            h64,
            state.mem64.as_ptr() as *const u8,
            state.total_len as usize,
        )
    }
}

/* ===== public C API (namespaced with LZ4_) ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER as c_uint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32(input: *const c_void, len: usize, seed: c_uint) -> u32 {
    unsafe { xxh32(input as *const u8, len, seed) }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_XXH32_createState() -> *mut XXH32State {
    unsafe { crate::util::malloc(core::mem::size_of::<XXH32State>()) as *mut XXH32State }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_freeState(state_ptr: *mut XXH32State) -> c_uint {
    unsafe { crate::util::free(state_ptr as *mut c_void) };
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_copyState(dst: *mut XXH32State, src: *const XXH32State) {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, 1) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_reset(state_ptr: *mut XXH32State, seed: c_uint) -> c_uint {
    unsafe { xxh32_reset(&mut *state_ptr, seed) };
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_update(
    state_ptr: *mut XXH32State,
    input: *const c_void,
    len: usize,
) -> c_uint {
    unsafe { xxh32_update(&mut *state_ptr, input as *const u8, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_digest(state_ptr: *const XXH32State) -> u32 {
    unsafe { xxh32_digest(&*state_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_canonicalFromHash(dst: *mut XXH32Canonical, hash: u32) {
    unsafe { (*dst).digest = hash.to_be_bytes() };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_hashFromCanonical(src: *const XXH32Canonical) -> u32 {
    unsafe { u32::from_be_bytes((*src).digest) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64(
    input: *const c_void,
    len: usize,
    seed: core::ffi::c_ulonglong,
) -> u64 {
    unsafe { xxh64(input as *const u8, len, seed) }
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4_XXH64_createState() -> *mut XXH64State {
    unsafe { crate::util::malloc(core::mem::size_of::<XXH64State>()) as *mut XXH64State }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_freeState(state_ptr: *mut XXH64State) -> c_uint {
    unsafe { crate::util::free(state_ptr as *mut c_void) };
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_copyState(dst: *mut XXH64State, src: *const XXH64State) {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, 1) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_reset(
    state_ptr: *mut XXH64State,
    seed: core::ffi::c_ulonglong,
) -> c_uint {
    unsafe { xxh64_reset(&mut *state_ptr, seed) };
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_update(
    state_ptr: *mut XXH64State,
    input: *const c_void,
    len: usize,
) -> c_uint {
    unsafe { xxh64_update(&mut *state_ptr, input as *const u8, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_digest(state_ptr: *const XXH64State) -> u64 {
    unsafe { xxh64_digest(&*state_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_canonicalFromHash(dst: *mut XXH64Canonical, hash: u64) {
    unsafe { (*dst).digest = hash.to_be_bytes() };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_hashFromCanonical(src: *const XXH64Canonical) -> u64 {
    unsafe { u64::from_be_bytes((*src).digest) }
}
