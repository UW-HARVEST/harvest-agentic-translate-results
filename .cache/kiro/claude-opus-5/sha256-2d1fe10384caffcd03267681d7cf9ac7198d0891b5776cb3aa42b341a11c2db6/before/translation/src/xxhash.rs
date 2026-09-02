//! Translation of the XXH32/XXH64 parts of `common/xxhash.h` (xxHash 0.8.2).
//!
//! The C build defines `XXH_NAMESPACE=ZSTD_`, so every public symbol is
//! prefixed: `XXH64()` links as `ZSTD_XXH64`. The exported names below match the
//! linker symbols of the C build, not the source-level names.
#![allow(dead_code)]

use core::ffi::{c_uint, c_void};

use crate::mem::*;

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 8;
pub const XXH_VERSION_RELEASE: u32 = 2;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

/// `XXH_errorcode`
pub type XXH_errorcode = c_uint;
pub const XXH_OK: XXH_errorcode = 0;
pub const XXH_ERROR: XXH_errorcode = 1;

pub type XXH32_hash_t = u32;
pub type XXH64_hash_t = u64;

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

/// `struct XXH32_state_s`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH32_state_t {
    pub total_len_32: XXH32_hash_t,
    pub large_len: XXH32_hash_t,
    pub v: [XXH32_hash_t; 4],
    pub mem32: [XXH32_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved: XXH32_hash_t,
}

/// `struct XXH64_state_s`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH64_state_t {
    pub total_len: XXH64_hash_t,
    pub v: [XXH64_hash_t; 4],
    pub mem64: [XXH64_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved32: XXH32_hash_t,
    pub reserved64: XXH64_hash_t,
}

/// `XXH32_canonical_t`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH32_canonical_t {
    pub digest: [u8; 4],
}

/// `XXH64_canonical_t`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH64_canonical_t {
    pub digest: [u8; 8],
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

/* ================== XXH32 ================== */

#[inline(always)]
fn xxh32_round(acc: u32, input: u32) -> u32 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME32_2));
    acc = acc.rotate_left(13);
    acc.wrapping_mul(XXH_PRIME32_1)
}

#[inline(always)]
fn xxh32_avalanche(mut hash: u32) -> u32 {
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(XXH_PRIME32_2);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(XXH_PRIME32_3);
    hash ^= hash >> 16;
    hash
}

/// `XXH32_finalize()` — `XXH32_ENDJMP` is 0 by default, so this is the
/// "compact rerolled" branch.
#[inline(always)]
unsafe fn xxh32_finalize(mut hash: u32, mut ptr: *const u8, mut len: usize) -> u32 {
    len &= 15;
    while len >= 4 {
        hash = hash.wrapping_add(mem_read_le32(ptr).wrapping_mul(XXH_PRIME32_3));
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
    xxh32_avalanche(hash)
}

/// `XXH32_endian_align()`
unsafe fn xxh32_endian_align(mut input: *const u8, len: usize, seed: u32) -> u32 {
    let mut h32: u32;

    if len >= 16 {
        let b_end = input.add(len);
        let limit = b_end.sub(15);
        let mut v1 = seed.wrapping_add(XXH_PRIME32_1).wrapping_add(XXH_PRIME32_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME32_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME32_1);

        loop {
            v1 = xxh32_round(v1, mem_read_le32(input));
            input = input.add(4);
            v2 = xxh32_round(v2, mem_read_le32(input));
            input = input.add(4);
            v3 = xxh32_round(v3, mem_read_le32(input));
            input = input.add(4);
            v4 = xxh32_round(v4, mem_read_le32(input));
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

    xxh32_finalize(h32, input, len & 15)
}

/// `XXH32()` → linker symbol `ZSTD_XXH32`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32(
    input: *const c_void,
    len: usize,
    seed: XXH32_hash_t,
) -> XXH32_hash_t {
    xxh32_endian_align(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_createState() -> *mut XXH32_state_t {
    malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_freeState(state_ptr: *mut XXH32_state_t) -> XXH_errorcode {
    free(state_ptr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_copyState(
    dst_state: *mut XXH32_state_t,
    src_state: *const XXH32_state_t,
) {
    core::ptr::copy_nonoverlapping(src_state, dst_state, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_reset(
    state_ptr: *mut XXH32_state_t,
    seed: XXH32_hash_t,
) -> XXH_errorcode {
    let mut state = XXH32_state_t::default();
    state.v[0] = seed.wrapping_add(XXH_PRIME32_1).wrapping_add(XXH_PRIME32_2);
    state.v[1] = seed.wrapping_add(XXH_PRIME32_2);
    state.v[2] = seed;
    state.v[3] = seed.wrapping_sub(XXH_PRIME32_1);
    *state_ptr = state;
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
    let st = &mut *state;
    let mut p = input as *const u8;
    let b_end = p.add(len);

    st.total_len_32 = st.total_len_32.wrapping_add(len as XXH32_hash_t);
    st.large_len |= ((len >= 16) as XXH32_hash_t) | ((st.total_len_32 >= 16) as XXH32_hash_t);

    if (st.memsize as usize) + len < 16 {
        core::ptr::copy_nonoverlapping(
            p,
            (st.mem32.as_mut_ptr() as *mut u8).add(st.memsize as usize),
            len,
        );
        st.memsize += len as XXH32_hash_t;
        return XXH_OK;
    }

    if st.memsize != 0 {
        core::ptr::copy_nonoverlapping(
            p,
            (st.mem32.as_mut_ptr() as *mut u8).add(st.memsize as usize),
            16 - st.memsize as usize,
        );
        {
            let mut p32 = st.mem32.as_ptr() as *const u8;
            st.v[0] = xxh32_round(st.v[0], mem_read_le32(p32));
            p32 = p32.add(4);
            st.v[1] = xxh32_round(st.v[1], mem_read_le32(p32));
            p32 = p32.add(4);
            st.v[2] = xxh32_round(st.v[2], mem_read_le32(p32));
            p32 = p32.add(4);
            st.v[3] = xxh32_round(st.v[3], mem_read_le32(p32));
        }
        p = p.add(16 - st.memsize as usize);
        st.memsize = 0;
    }

    if (p as usize) <= (b_end as usize).wrapping_sub(16) {
        let limit = b_end.sub(16);
        loop {
            st.v[0] = xxh32_round(st.v[0], mem_read_le32(p));
            p = p.add(4);
            st.v[1] = xxh32_round(st.v[1], mem_read_le32(p));
            p = p.add(4);
            st.v[2] = xxh32_round(st.v[2], mem_read_le32(p));
            p = p.add(4);
            st.v[3] = xxh32_round(st.v[3], mem_read_le32(p));
            p = p.add(4);
            if p > limit {
                break;
            }
        }
    }

    if p < b_end {
        let rem = b_end as usize - p as usize;
        core::ptr::copy_nonoverlapping(p, st.mem32.as_mut_ptr() as *mut u8, rem);
        st.memsize = rem as XXH32_hash_t;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_digest(state: *const XXH32_state_t) -> XXH32_hash_t {
    let st = &*state;
    let mut h32: u32 = if st.large_len != 0 {
        st.v[0]
            .rotate_left(1)
            .wrapping_add(st.v[1].rotate_left(7))
            .wrapping_add(st.v[2].rotate_left(12))
            .wrapping_add(st.v[3].rotate_left(18))
    } else {
        st.v[2].wrapping_add(XXH_PRIME32_5)
    };

    h32 = h32.wrapping_add(st.total_len_32);

    xxh32_finalize(h32, st.mem32.as_ptr() as *const u8, st.memsize as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(
    dst: *mut XXH32_canonical_t,
    hash: XXH32_hash_t,
) {
    let hash = if cfg!(target_endian = "little") {
        hash.swap_bytes()
    } else {
        hash
    };
    core::ptr::copy_nonoverlapping(&hash as *const u32 as *const u8, dst as *mut u8, 4);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(
    src: *const XXH32_canonical_t,
) -> XXH32_hash_t {
    mem_read_be32(src as *const u8)
}

/* ================== XXH64 ================== */

#[inline(always)]
fn xxh64_round(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

#[inline(always)]
fn xxh64_merge_round(acc: u64, val: u64) -> u64 {
    let val = xxh64_round(0, val);
    let acc = acc ^ val;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

#[inline(always)]
fn xxh64_avalanche(mut hash: u64) -> u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(XXH_PRIME64_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(XXH_PRIME64_3);
    hash ^= hash >> 32;
    hash
}

/// `XXH64_finalize()`
#[inline(always)]
unsafe fn xxh64_finalize(mut hash: u64, mut ptr: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, mem_read_le64(ptr));
        ptr = ptr.add(8);
        hash ^= k1;
        hash = hash
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        hash ^= (mem_read_le32(ptr) as u64).wrapping_mul(XXH_PRIME64_1);
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
    xxh64_avalanche(hash)
}

/// `XXH64_endian_align()`
unsafe fn xxh64_endian_align(mut input: *const u8, len: usize, seed: u64) -> u64 {
    let mut h64: u64;

    if len >= 32 {
        let b_end = input.add(len);
        let limit = b_end.sub(31);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, mem_read_le64(input));
            input = input.add(8);
            v2 = xxh64_round(v2, mem_read_le64(input));
            input = input.add(8);
            v3 = xxh64_round(v3, mem_read_le64(input));
            input = input.add(8);
            v4 = xxh64_round(v4, mem_read_le64(input));
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
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);

    xxh64_finalize(h64, input, len)
}

/// `XXH64()` → linker symbol `ZSTD_XXH64`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64(
    input: *const c_void,
    len: usize,
    seed: XXH64_hash_t,
) -> XXH64_hash_t {
    xxh64_endian_align(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_createState() -> *mut XXH64_state_t {
    malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_freeState(state_ptr: *mut XXH64_state_t) -> XXH_errorcode {
    free(state_ptr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_copyState(
    dst_state: *mut XXH64_state_t,
    src_state: *const XXH64_state_t,
) {
    core::ptr::copy_nonoverlapping(src_state, dst_state, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_reset(
    state_ptr: *mut XXH64_state_t,
    seed: XXH64_hash_t,
) -> XXH_errorcode {
    let mut state = XXH64_state_t::default();
    state.v[0] = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
    state.v[1] = seed.wrapping_add(XXH_PRIME64_2);
    state.v[2] = seed;
    state.v[3] = seed.wrapping_sub(XXH_PRIME64_1);
    *state_ptr = state;
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
    let st = &mut *state;
    let mut p = input as *const u8;
    let b_end = p.add(len);

    st.total_len = st.total_len.wrapping_add(len as u64);

    if (st.memsize as usize) + len < 32 {
        core::ptr::copy_nonoverlapping(
            p,
            (st.mem64.as_mut_ptr() as *mut u8).add(st.memsize as usize),
            len,
        );
        st.memsize += len as XXH32_hash_t;
        return XXH_OK;
    }

    if st.memsize != 0 {
        core::ptr::copy_nonoverlapping(
            p,
            (st.mem64.as_mut_ptr() as *mut u8).add(st.memsize as usize),
            32 - st.memsize as usize,
        );
        let base = st.mem64.as_ptr() as *const u8;
        st.v[0] = xxh64_round(st.v[0], mem_read_le64(base));
        st.v[1] = xxh64_round(st.v[1], mem_read_le64(base.add(8)));
        st.v[2] = xxh64_round(st.v[2], mem_read_le64(base.add(16)));
        st.v[3] = xxh64_round(st.v[3], mem_read_le64(base.add(24)));
        p = p.add(32 - st.memsize as usize);
        st.memsize = 0;
    }

    if (p as usize) + 32 <= b_end as usize {
        let limit = b_end.sub(32);
        loop {
            st.v[0] = xxh64_round(st.v[0], mem_read_le64(p));
            p = p.add(8);
            st.v[1] = xxh64_round(st.v[1], mem_read_le64(p));
            p = p.add(8);
            st.v[2] = xxh64_round(st.v[2], mem_read_le64(p));
            p = p.add(8);
            st.v[3] = xxh64_round(st.v[3], mem_read_le64(p));
            p = p.add(8);
            if p > limit {
                break;
            }
        }
    }

    if p < b_end {
        let rem = b_end as usize - p as usize;
        core::ptr::copy_nonoverlapping(p, st.mem64.as_mut_ptr() as *mut u8, rem);
        st.memsize = rem as XXH32_hash_t;
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(state: *const XXH64_state_t) -> XXH64_hash_t {
    let st = &*state;
    let mut h64: u64;
    if st.total_len >= 32 {
        h64 = st.v[0]
            .rotate_left(1)
            .wrapping_add(st.v[1].rotate_left(7))
            .wrapping_add(st.v[2].rotate_left(12))
            .wrapping_add(st.v[3].rotate_left(18));
        h64 = xxh64_merge_round(h64, st.v[0]);
        h64 = xxh64_merge_round(h64, st.v[1]);
        h64 = xxh64_merge_round(h64, st.v[2]);
        h64 = xxh64_merge_round(h64, st.v[3]);
    } else {
        h64 = st.v[2].wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(st.total_len);

    xxh64_finalize(h64, st.mem64.as_ptr() as *const u8, st.total_len as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(
    dst: *mut XXH64_canonical_t,
    hash: XXH64_hash_t,
) {
    let hash = if cfg!(target_endian = "little") {
        hash.swap_bytes()
    } else {
        hash
    };
    core::ptr::copy_nonoverlapping(&hash as *const u64 as *const u8, dst as *mut u8, 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(
    src: *const XXH64_canonical_t,
) -> XXH64_hash_t {
    mem_read_be64(src as *const u8)
}

/* Internal Rust-side helpers, used by the zstd frame checksum code. */

#[inline(always)]
pub unsafe fn xxh64(input: *const u8, len: usize, seed: u64) -> u64 {
    xxh64_endian_align(input, len, seed)
}
