//! Translation of common/xxhash.{h,c} with XXH_NAMESPACE=ZSTD_.
//! Implements XXH32 and XXH64 (standard xxHash algorithm).
#![allow(dead_code)]
use core::ffi::c_void;

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 8;
pub const XXH_VERSION_RELEASE: u32 = 2;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

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

pub const XXH_OK: i32 = 0;
pub const XXH_ERROR: i32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH32_state_t {
    pub total_len_32: u32,
    pub large_len: u32,
    pub v: [u32; 4],
    pub mem32: [u32; 4],
    pub memsize: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XXH64_state_t {
    pub total_len: u64,
    pub v: [u64; 4],
    pub mem64: [u64; 4],
    pub memsize: u32,
    pub reserved32: u32,
    pub reserved64: u64,
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
unsafe fn read32(p: *const u8) -> u32 {
    let mut v = 0u32;
    core::ptr::copy_nonoverlapping(p, &mut v as *mut u32 as *mut u8, 4);
    v.to_le()
}
#[inline]
unsafe fn read64(p: *const u8) -> u64 {
    let mut v = 0u64;
    core::ptr::copy_nonoverlapping(p, &mut v as *mut u64 as *mut u8, 8);
    v.to_le()
}

// ===== XXH32 =====
#[inline]
fn xxh32_round(mut acc: u32, input: u32) -> u32 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME32_2));
    acc = acc.rotate_left(13);
    acc.wrapping_mul(XXH_PRIME32_1)
}

#[inline]
fn xxh32_avalanche(mut h: u32) -> u32 {
    h ^= h >> 15;
    h = h.wrapping_mul(XXH_PRIME32_2);
    h ^= h >> 13;
    h = h.wrapping_mul(XXH_PRIME32_3);
    h ^= h >> 16;
    h
}

unsafe fn xxh32_finalize(mut h: u32, mut p: *const u8, mut len: usize) -> u32 {
    macro_rules! process1 {
        () => {{
            h = h.wrapping_add((*p as u32).wrapping_mul(XXH_PRIME32_5));
            p = p.add(1);
            h = h.rotate_left(11).wrapping_mul(XXH_PRIME32_1);
        }};
    }
    macro_rules! process4 {
        () => {{
            h = h.wrapping_add(read32(p).wrapping_mul(XXH_PRIME32_3));
            p = p.add(4);
            h = h.rotate_left(17).wrapping_mul(XXH_PRIME32_4);
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
    xxh32_avalanche(h)
}

unsafe fn xxh32_endian(input: *const u8, len: usize, seed: u32) -> u32 {
    let mut p = input;
    let b_end = p.add(len);
    let mut h32: u32;
    if len >= 16 {
        let limit = b_end.sub(16);
        let mut v1 = seed.wrapping_add(XXH_PRIME32_1).wrapping_add(XXH_PRIME32_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME32_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME32_1);
        loop {
            v1 = xxh32_round(v1, read32(p));
            p = p.add(4);
            v2 = xxh32_round(v2, read32(p));
            p = p.add(4);
            v3 = xxh32_round(v3, read32(p));
            p = p.add(4);
            v4 = xxh32_round(v4, read32(p));
            p = p.add(4);
            if p > limit {
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
    xxh32_finalize(h32, p, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32(input: *const c_void, len: usize, seed: u32) -> u32 {
    xxh32_endian(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_createState() -> *mut XXH32_state_t {
    super::allocations::malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_freeState(statePtr: *mut XXH32_state_t) -> i32 {
    super::allocations::free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_copyState(dst: *mut XXH32_state_t, src: *const XXH32_state_t) {
    core::ptr::copy_nonoverlapping(src, dst, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_reset(statePtr: *mut XXH32_state_t, seed: u32) -> i32 {
    let mut state: XXH32_state_t = core::mem::zeroed();
    state.v[0] = seed.wrapping_add(XXH_PRIME32_1).wrapping_add(XXH_PRIME32_2);
    state.v[1] = seed.wrapping_add(XXH_PRIME32_2);
    state.v[2] = seed;
    state.v[3] = seed.wrapping_sub(XXH_PRIME32_1);
    core::ptr::copy_nonoverlapping(
        &state as *const XXH32_state_t as *const u8,
        statePtr as *mut u8,
        core::mem::size_of::<XXH32_state_t>() - core::mem::size_of::<u32>(),
    );
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_update(
    state: *mut XXH32_state_t,
    input: *const c_void,
    len: usize,
) -> i32 {
    if input.is_null() {
        return XXH_OK;
    }
    let mut p = input as *const u8;
    let b_end = p.add(len);
    (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
    (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

    if ((*state).memsize as usize + len) < 16 {
        core::ptr::copy_nonoverlapping(
            p,
            ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize),
            len,
        );
        (*state).memsize += len as u32;
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        core::ptr::copy_nonoverlapping(
            p,
            ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize),
            (16 - (*state).memsize) as usize,
        );
        let m = (*state).mem32.as_ptr() as *const u8;
        (*state).v[0] = xxh32_round((*state).v[0], read32(m));
        (*state).v[1] = xxh32_round((*state).v[1], read32(m.add(4)));
        (*state).v[2] = xxh32_round((*state).v[2], read32(m.add(8)));
        (*state).v[3] = xxh32_round((*state).v[3], read32(m.add(12)));
        p = p.add((16 - (*state).memsize) as usize);
        (*state).memsize = 0;
    }

    if p <= b_end.sub(16) {
        let limit = b_end.sub(16);
        let mut v1 = (*state).v[0];
        let mut v2 = (*state).v[1];
        let mut v3 = (*state).v[2];
        let mut v4 = (*state).v[3];
        loop {
            v1 = xxh32_round(v1, read32(p));
            p = p.add(4);
            v2 = xxh32_round(v2, read32(p));
            p = p.add(4);
            v3 = xxh32_round(v3, read32(p));
            p = p.add(4);
            v4 = xxh32_round(v4, read32(p));
            p = p.add(4);
            if p > limit {
                break;
            }
        }
        (*state).v[0] = v1;
        (*state).v[1] = v2;
        (*state).v[2] = v3;
        (*state).v[3] = v4;
    }

    if p < b_end {
        core::ptr::copy_nonoverlapping(
            p,
            (*state).mem32.as_mut_ptr() as *mut u8,
            (b_end as usize) - (p as usize),
        );
        (*state).memsize = ((b_end as usize) - (p as usize)) as u32;
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
    xxh32_finalize(h32, (*state).mem32.as_ptr() as *const u8, (*state).memsize as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(
    dst: *mut XXH32_canonical_t,
    mut hash: u32,
) {
    hash = hash.to_be();
    core::ptr::copy_nonoverlapping(
        &hash as *const u32 as *const u8,
        (*dst).digest.as_mut_ptr(),
        4,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> u32 {
    let mut v = 0u32;
    core::ptr::copy_nonoverlapping((*src).digest.as_ptr(), &mut v as *mut u32 as *mut u8, 4);
    u32::from_be(v)
}

// ===== XXH64 =====
#[inline]
fn xxh64_round(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(XXH_PRIME64_1)
}
#[inline]
fn xxh64_merge_round(mut acc: u64, val: u64) -> u64 {
    let val = xxh64_round(0, val);
    acc ^= val;
    acc = acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
    acc
}
#[inline]
fn xxh64_avalanche(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(XXH_PRIME64_2);
    h ^= h >> 29;
    h = h.wrapping_mul(XXH_PRIME64_3);
    h ^= h >> 32;
    h
}

unsafe fn xxh64_finalize(mut h: u64, mut p: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, read64(p));
        p = p.add(8);
        h ^= k1;
        h = h.rotate_left(27).wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h ^= (read32(p) as u64).wrapping_mul(XXH_PRIME64_1);
        p = p.add(4);
        h = h.rotate_left(23).wrapping_mul(XXH_PRIME64_2).wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h ^= (*p as u64).wrapping_mul(XXH_PRIME64_5);
        p = p.add(1);
        h = h.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h)
}

unsafe fn xxh64_endian(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end = p.add(len);
    let mut h64: u64;
    if len >= 32 {
        let limit = b_end.sub(32);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        loop {
            v1 = xxh64_round(v1, read64(p));
            p = p.add(8);
            v2 = xxh64_round(v2, read64(p));
            p = p.add(8);
            v3 = xxh64_round(v3, read64(p));
            p = p.add(8);
            v4 = xxh64_round(v4, read64(p));
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
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }
    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize(h64, p, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64(input: *const c_void, len: usize, seed: u64) -> u64 {
    xxh64_endian(input as *const u8, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_createState() -> *mut XXH64_state_t {
    super::allocations::malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_freeState(statePtr: *mut XXH64_state_t) -> i32 {
    super::allocations::free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_copyState(dst: *mut XXH64_state_t, src: *const XXH64_state_t) {
    core::ptr::copy_nonoverlapping(src, dst, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_reset(statePtr: *mut XXH64_state_t, seed: u64) -> i32 {
    let mut state: XXH64_state_t = core::mem::zeroed();
    state.v[0] = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
    state.v[1] = seed.wrapping_add(XXH_PRIME64_2);
    state.v[2] = seed;
    state.v[3] = seed.wrapping_sub(XXH_PRIME64_1);
    core::ptr::copy_nonoverlapping(
        &state as *const XXH64_state_t as *const u8,
        statePtr as *mut u8,
        core::mem::size_of::<XXH64_state_t>() - core::mem::size_of::<u64>(),
    );
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_update(
    state: *mut XXH64_state_t,
    input: *const c_void,
    len: usize,
) -> i32 {
    if input.is_null() {
        return XXH_OK;
    }
    let mut p = input as *const u8;
    let b_end = p.add(len);
    (*state).total_len = (*state).total_len.wrapping_add(len as u64);

    if ((*state).memsize as usize + len) < 32 {
        core::ptr::copy_nonoverlapping(
            p,
            ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize),
            len,
        );
        (*state).memsize += len as u32;
        return XXH_OK;
    }

    if (*state).memsize != 0 {
        core::ptr::copy_nonoverlapping(
            p,
            ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize),
            (32 - (*state).memsize) as usize,
        );
        let m = (*state).mem64.as_ptr() as *const u8;
        (*state).v[0] = xxh64_round((*state).v[0], read64(m));
        (*state).v[1] = xxh64_round((*state).v[1], read64(m.add(8)));
        (*state).v[2] = xxh64_round((*state).v[2], read64(m.add(16)));
        (*state).v[3] = xxh64_round((*state).v[3], read64(m.add(24)));
        p = p.add((32 - (*state).memsize) as usize);
        (*state).memsize = 0;
    }

    if p.add(32) <= b_end {
        let limit = b_end.sub(32);
        let mut v1 = (*state).v[0];
        let mut v2 = (*state).v[1];
        let mut v3 = (*state).v[2];
        let mut v4 = (*state).v[3];
        loop {
            v1 = xxh64_round(v1, read64(p));
            p = p.add(8);
            v2 = xxh64_round(v2, read64(p));
            p = p.add(8);
            v3 = xxh64_round(v3, read64(p));
            p = p.add(8);
            v4 = xxh64_round(v4, read64(p));
            p = p.add(8);
            if p > limit {
                break;
            }
        }
        (*state).v[0] = v1;
        (*state).v[1] = v2;
        (*state).v[2] = v3;
        (*state).v[3] = v4;
    }

    if p < b_end {
        core::ptr::copy_nonoverlapping(
            p,
            (*state).mem64.as_mut_ptr() as *mut u8,
            (b_end as usize) - (p as usize),
        );
        (*state).memsize = ((b_end as usize) - (p as usize)) as u32;
    }
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(state: *const XXH64_state_t) -> u64 {
    let mut h64: u64;
    if (*state).total_len >= 32 {
        let v1 = (*state).v[0];
        let v2 = (*state).v[1];
        let v3 = (*state).v[2];
        let v4 = (*state).v[3];
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
        h64 = (*state).v[2].wrapping_add(XXH_PRIME64_5);
    }
    h64 = h64.wrapping_add((*state).total_len);
    xxh64_finalize(h64, (*state).mem64.as_ptr() as *const u8, (*state).memsize as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, mut hash: u64) {
    hash = hash.to_be();
    core::ptr::copy_nonoverlapping(
        &hash as *const u64 as *const u8,
        (*dst).digest.as_mut_ptr(),
        8,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    let mut v = 0u64;
    core::ptr::copy_nonoverlapping((*src).digest.as_ptr(), &mut v as *mut u64 as *mut u8, 8);
    u64::from_be(v)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_XXH_versionNumber() -> u32 {
    XXH_VERSION_NUMBER
}
