// Translation of xxhash.c / xxhash.h (xxHash 0.6.5) to Rust.
//
// Compiled originally with -DXXH_NAMESPACE=LZ4_, so every public symbol is
// exported with an `LZ4_` prefix. Target: x86_64 Linux (little-endian,
// unaligned accesses allowed, default XXH_FORCE_MEMORY_ACCESS==0 which uses
// memcpy-based reads -- safe for any alignment). Because of that:
//   - XXH_CPU_LITTLE_ENDIAN is always 1 (true) on this target.
//   - XXH_FORCE_ALIGN_CHECK is 0 on x86/x86_64, so the "aligned" fast path in
//     XXH32()/XXH64() is never taken; only the "unaligned" endian_align call
//     with XXH_littleEndian is exercised.
//   - Since reads always go through the memcpy-style XXH_read32/XXH_read64,
//     the `align` parameter (aligned vs unaligned) makes no actual behavioral
//     difference; both paths are identical in codegen terms. We simply always
//     use unaligned little-endian reads.
//   - XXH_ACCEPT_NULL_INPUT_POINTER defaults to 0, so NULL input pointers are
//     NOT specially handled in XXH32()/XXH64() (they would dereference and
//     segfault on nonzero length, matching the C behavior exactly), and in
//     the streaming XXH32_update/XXH64_update, a NULL input pointer causes
//     XXH_ERROR to be returned.
//   - XXH_FORCE_NATIVE_FORMAT defaults to 0, irrelevant since we're already
//     little-endian.

use core::ffi::{c_int, c_uint, c_ulonglong, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/* *************************************
 *  Types
 ***************************************/

pub type XXH32_hash_t = u32;
pub type XXH64_hash_t = u64;

// typedef enum { XXH_OK=0, XXH_ERROR } XXH_errorcode;
pub const XXH_OK: c_int = 0;
pub const XXH_ERROR: c_int = 1;

#[repr(C)]
pub struct XXH32_state_s {
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
pub type XXH32_state_t = XXH32_state_s;

#[repr(C)]
pub struct XXH64_state_s {
    pub total_len: u64,
    pub v1: u64,
    pub v2: u64,
    pub v3: u64,
    pub v4: u64,
    pub mem64: [u64; 4],
    pub memsize: u32,
    pub reserved: [u32; 2],
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

/* *************************************
 *  Memory reads (XXH_FORCE_MEMORY_ACCESS default == 0, memcpy-based, safe
 *  for any alignment; on this target reads are always effectively
 *  little-endian native reads since XXH_CPU_LITTLE_ENDIAN == 1)
 ***************************************/

#[inline(always)]
unsafe fn xxh_read32(mem_ptr: *const u8) -> u32 {
    unsafe { core::ptr::read_unaligned(mem_ptr as *const u32) }
}

#[inline(always)]
unsafe fn xxh_read64(mem_ptr: *const u8) -> u64 {
    unsafe { core::ptr::read_unaligned(mem_ptr as *const u64) }
}

#[inline(always)]
fn xxh_swap32(x: u32) -> u32 {
    x.swap_bytes()
}

#[inline(always)]
fn xxh_swap64(x: u64) -> u64 {
    x.swap_bytes()
}

// XXH_readLE32_align / XXH_readLE32: endian is always XXH_littleEndian on
// this target, and align makes no behavioral difference (memcpy-based read).
#[inline(always)]
unsafe fn xxh_get32bits(p: *const u8) -> u32 {
    unsafe { xxh_read32(p) }
}

#[inline(always)]
unsafe fn xxh_get64bits(p: *const u8) -> u64 {
    unsafe { xxh_read64(p) }
}

// XXH_readBE32/64 : CPU is little-endian, so this always swaps.
#[inline(always)]
unsafe fn xxh_read_be32(ptr: *const u8) -> u32 {
    unsafe { xxh_swap32(xxh_read32(ptr)) }
}

#[inline(always)]
unsafe fn xxh_read_be64(ptr: *const u8) -> u64 {
    unsafe { xxh_swap64(xxh_read64(ptr)) }
}

#[inline(always)]
fn xxh_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}

#[inline(always)]
fn xxh_rotl64(x: u64, r: u32) -> u64 {
    (x << r) | (x >> (64 - r))
}

/* *******************************************************************
 *  Version
 *********************************************************************/

const XXH_VERSION_MAJOR: c_uint = 0;
const XXH_VERSION_MINOR: c_uint = 6;
const XXH_VERSION_RELEASE: c_uint = 5;
const XXH_VERSION_NUMBER: c_uint =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

/* *******************************************************************
 *  32-bit hash functions
 *********************************************************************/

const PRIME32_1: u32 = 2654435761u32;
const PRIME32_2: u32 = 2246822519u32;
const PRIME32_3: u32 = 3266489917u32;
const PRIME32_4: u32 = 668265263u32;
const PRIME32_5: u32 = 374761393u32;

#[inline(always)]
fn xxh32_round(seed: u32, input: u32) -> u32 {
    let mut seed = seed.wrapping_add(input.wrapping_mul(PRIME32_2));
    seed = xxh_rotl32(seed, 13);
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

// Faithful transcription of XXH32_finalize's switch(len&15) fallthrough
// chain: it is equivalent to running PROCESS4 (len15/4) times followed by
// PROCESS1 (len15%4) times, then avalanche. (Verified case-by-case against
// the C switch statement.)
unsafe fn xxh32_finalize(mut h32: u32, ptr: *const u8, len: usize) -> u32 {
    let mut p = ptr;
    let len15 = len & 15;
    let n4 = len15 / 4;
    let n1 = len15 % 4;

    for _ in 0..n4 {
        h32 = h32.wrapping_add(unsafe { xxh_get32bits(p) }.wrapping_mul(PRIME32_3));
        p = unsafe { p.add(4) };
        h32 = xxh_rotl32(h32, 17).wrapping_mul(PRIME32_4);
    }

    for _ in 0..n1 {
        h32 = h32.wrapping_add((unsafe { *p } as u32).wrapping_mul(PRIME32_5));
        p = unsafe { p.add(1) };
        h32 = xxh_rotl32(h32, 11).wrapping_mul(PRIME32_1);
    }

    xxh32_avalanche(h32)
}

unsafe fn xxh32_endian_align(input: *const u8, len: usize, seed: u32) -> u32 {
    let mut p = input;
    let b_end = unsafe { p.add(len) };
    let h32: u32;

    if len >= 16 {
        let limit = unsafe { b_end.sub(15) };
        let mut v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
        let mut v2 = seed.wrapping_add(PRIME32_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(PRIME32_1);

        loop {
            v1 = xxh32_round(v1, unsafe { xxh_get32bits(p) });
            p = unsafe { p.add(4) };
            v2 = xxh32_round(v2, unsafe { xxh_get32bits(p) });
            p = unsafe { p.add(4) };
            v3 = xxh32_round(v3, unsafe { xxh_get32bits(p) });
            p = unsafe { p.add(4) };
            v4 = xxh32_round(v4, unsafe { xxh_get32bits(p) });
            p = unsafe { p.add(4) };
            if !(p < limit) {
                break;
            }
        }

        h32 = xxh_rotl32(v1, 1)
            .wrapping_add(xxh_rotl32(v2, 7))
            .wrapping_add(xxh_rotl32(v3, 12))
            .wrapping_add(xxh_rotl32(v4, 18));
    } else {
        h32 = seed.wrapping_add(PRIME32_5);
    }

    let h32 = h32.wrapping_add(len as u32);

    unsafe { xxh32_finalize(h32, p, len & 15) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32(input: *const c_void, len: usize, seed: c_uint) -> XXH32_hash_t {
    // XXH_FORCE_ALIGN_CHECK == 0 on x86_64: the aligned fast-path is never
    // taken. XXH_CPU_LITTLE_ENDIAN == 1 always on this target.
    unsafe { xxh32_endian_align(input as *const u8, len, seed) }
}

/*======   Hash streaming   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_createState() -> *mut XXH32_state_t {
    unsafe { malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_freeState(state_ptr: *mut XXH32_state_t) -> c_int {
    unsafe { free(state_ptr as *mut c_void) };
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_copyState(dst_state: *mut XXH32_state_t, src_state: *const XXH32_state_t) {
    unsafe {
        core::ptr::copy_nonoverlapping(src_state, dst_state, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_reset(state_ptr: *mut XXH32_state_t, seed: c_uint) -> c_int {
    let mut state: XXH32_state_t = XXH32_state_t {
        total_len_32: 0,
        large_len: 0,
        v1: 0,
        v2: 0,
        v3: 0,
        v4: 0,
        mem32: [0; 4],
        memsize: 0,
        reserved: 0,
    };
    state.v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
    state.v2 = seed.wrapping_add(PRIME32_2);
    state.v3 = seed.wrapping_add(0);
    state.v4 = seed.wrapping_sub(PRIME32_1);
    // do not write into reserved
    unsafe {
        core::ptr::copy_nonoverlapping(
            &state as *const XXH32_state_t as *const u8,
            state_ptr as *mut u8,
            core::mem::size_of::<XXH32_state_t>() - core::mem::size_of::<u32>(),
        );
    }
    XXH_OK
}

unsafe fn xxh32_update_endian(state: *mut XXH32_state_t, input: *const c_void, len: usize) -> c_int {
    if input.is_null() {
        return XXH_ERROR;
    }

    unsafe {
        let mut p = input as *const u8;
        let b_end = p.add(len);

        (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
        (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

        if ((*state).memsize as usize + len) < 16 {
            let dst = ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize);
            core::ptr::copy_nonoverlapping(p, dst, len);
            (*state).memsize += len as u32;
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            let dst = ((*state).mem32.as_mut_ptr() as *mut u8).add((*state).memsize as usize);
            let n = 16 - (*state).memsize as usize;
            core::ptr::copy_nonoverlapping(p, dst, n);
            {
                let mut p32 = (*state).mem32.as_ptr() as *const u8;
                (*state).v1 = xxh32_round((*state).v1, xxh_get32bits(p32));
                p32 = p32.add(4);
                (*state).v2 = xxh32_round((*state).v2, xxh_get32bits(p32));
                p32 = p32.add(4);
                (*state).v3 = xxh32_round((*state).v3, xxh_get32bits(p32));
                p32 = p32.add(4);
                (*state).v4 = xxh32_round((*state).v4, xxh_get32bits(p32));
            }
            p = p.add(16 - (*state).memsize as usize);
            (*state).memsize = 0;
        }

        if p <= b_end.sub(16) {
            let limit = b_end.sub(16);
            let mut v1 = (*state).v1;
            let mut v2 = (*state).v2;
            let mut v3 = (*state).v3;
            let mut v4 = (*state).v4;

            loop {
                v1 = xxh32_round(v1, xxh_get32bits(p));
                p = p.add(4);
                v2 = xxh32_round(v2, xxh_get32bits(p));
                p = p.add(4);
                v3 = xxh32_round(v3, xxh_get32bits(p));
                p = p.add(4);
                v4 = xxh32_round(v4, xxh_get32bits(p));
                p = p.add(4);
                if !(p <= limit) {
                    break;
                }
            }

            (*state).v1 = v1;
            (*state).v2 = v2;
            (*state).v3 = v3;
            (*state).v4 = v4;
        }

        if p < b_end {
            let n = b_end.offset_from(p) as usize;
            core::ptr::copy_nonoverlapping(p, (*state).mem32.as_mut_ptr() as *mut u8, n);
            (*state).memsize = n as u32;
        }
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_update(state_in: *mut XXH32_state_t, input: *const c_void, len: usize) -> c_int {
    unsafe { xxh32_update_endian(state_in, input, len) }
}

unsafe fn xxh32_digest_endian(state: *const XXH32_state_t) -> u32 {
    unsafe {
        let h32: u32;
        if (*state).large_len != 0 {
            h32 = xxh_rotl32((*state).v1, 1)
                .wrapping_add(xxh_rotl32((*state).v2, 7))
                .wrapping_add(xxh_rotl32((*state).v3, 12))
                .wrapping_add(xxh_rotl32((*state).v4, 18));
        } else {
            h32 = (*state).v3.wrapping_add(PRIME32_5);
        }

        let h32 = h32.wrapping_add((*state).total_len_32);

        xxh32_finalize(h32, (*state).mem32.as_ptr() as *const u8, (*state).memsize as usize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_digest(state_in: *const XXH32_state_t) -> XXH32_hash_t {
    unsafe { xxh32_digest_endian(state_in) }
}

/*======   Canonical representation   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_canonicalFromHash(dst: *mut XXH32_canonical_t, hash: XXH32_hash_t) {
    // XXH_CPU_LITTLE_ENDIAN is always true on this target.
    let hash = xxh_swap32(hash);
    unsafe {
        core::ptr::copy_nonoverlapping(&hash as *const u32 as *const u8, dst as *mut u8, 4);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> XXH32_hash_t {
    unsafe { xxh_read_be32(src as *const u8) }
}

/* *******************************************************************
 *  64-bit hash functions
 *********************************************************************/

const PRIME64_1: u64 = 11400714785074694791u64;
const PRIME64_2: u64 = 14029467366897019727u64;
const PRIME64_3: u64 = 1609587929392839161u64;
const PRIME64_4: u64 = 9650029242287828579u64;
const PRIME64_5: u64 = 2870177450012600261u64;

#[inline(always)]
fn xxh64_round(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = xxh_rotl64(acc, 31);
    acc = acc.wrapping_mul(PRIME64_1);
    acc
}

#[inline(always)]
fn xxh64_merge_round(acc: u64, val: u64) -> u64 {
    let val = xxh64_round(0, val);
    let acc = acc ^ val;
    let acc = acc.wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4);
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

// Faithful transcription of XXH64_finalize's switch(len&31) fallthrough
// chain: equivalent to running PROCESS8 (len32/8) times, then if the
// remainder r=len32%8 is >=4 running PROCESS4 once and subtracting 4 from r,
// then running PROCESS1 r times, then avalanche. (Verified case-by-case
// against the C switch statement.)
unsafe fn xxh64_finalize(mut h64: u64, ptr: *const u8, len: usize) -> u64 {
    let mut p = ptr;
    let len32 = len & 31;
    let n8 = len32 / 8;
    let mut r = len32 % 8;

    for _ in 0..n8 {
        let k1 = xxh64_round(0, unsafe { xxh_get64bits(p) });
        p = unsafe { p.add(8) };
        h64 ^= k1;
        h64 = xxh_rotl64(h64, 27).wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4);
    }

    if r >= 4 {
        h64 ^= (unsafe { xxh_get32bits(p) } as u64).wrapping_mul(PRIME64_1);
        p = unsafe { p.add(4) };
        h64 = xxh_rotl64(h64, 23).wrapping_mul(PRIME64_2).wrapping_add(PRIME64_3);
        r -= 4;
    }

    for _ in 0..r {
        h64 ^= (unsafe { *p } as u64).wrapping_mul(PRIME64_5);
        p = unsafe { p.add(1) };
        h64 = xxh_rotl64(h64, 11).wrapping_mul(PRIME64_1);
    }

    xxh64_avalanche(h64)
}

unsafe fn xxh64_endian_align(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end = unsafe { p.add(len) };
    let h64: u64;

    if len >= 32 {
        let limit = unsafe { b_end.sub(32) };
        let mut v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
        let mut v2 = seed.wrapping_add(PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(PRIME64_1);

        loop {
            v1 = xxh64_round(v1, unsafe { xxh_get64bits(p) });
            p = unsafe { p.add(8) };
            v2 = xxh64_round(v2, unsafe { xxh_get64bits(p) });
            p = unsafe { p.add(8) };
            v3 = xxh64_round(v3, unsafe { xxh_get64bits(p) });
            p = unsafe { p.add(8) };
            v4 = xxh64_round(v4, unsafe { xxh_get64bits(p) });
            p = unsafe { p.add(8) };
            if !(p <= limit) {
                break;
            }
        }

        let mut h = xxh_rotl64(v1, 1)
            .wrapping_add(xxh_rotl64(v2, 7))
            .wrapping_add(xxh_rotl64(v3, 12))
            .wrapping_add(xxh_rotl64(v4, 18));
        h = xxh64_merge_round(h, v1);
        h = xxh64_merge_round(h, v2);
        h = xxh64_merge_round(h, v3);
        h = xxh64_merge_round(h, v4);
        h64 = h;
    } else {
        h64 = seed.wrapping_add(PRIME64_5);
    }

    let h64 = h64.wrapping_add(len as u64);

    unsafe { xxh64_finalize(h64, p, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64(input: *const c_void, len: usize, seed: c_ulonglong) -> XXH64_hash_t {
    unsafe { xxh64_endian_align(input as *const u8, len, seed as u64) }
}

/*======   Hash Streaming   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_createState() -> *mut XXH64_state_t {
    unsafe { malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_freeState(state_ptr: *mut XXH64_state_t) -> c_int {
    unsafe { free(state_ptr as *mut c_void) };
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_copyState(dst_state: *mut XXH64_state_t, src_state: *const XXH64_state_t) {
    unsafe {
        core::ptr::copy_nonoverlapping(src_state, dst_state, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_reset(state_ptr: *mut XXH64_state_t, seed: c_ulonglong) -> c_int {
    let seed = seed as u64;
    let mut state: XXH64_state_t = XXH64_state_t {
        total_len: 0,
        v1: 0,
        v2: 0,
        v3: 0,
        v4: 0,
        mem64: [0; 4],
        memsize: 0,
        reserved: [0; 2],
    };
    state.v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
    state.v2 = seed.wrapping_add(PRIME64_2);
    state.v3 = seed.wrapping_add(0);
    state.v4 = seed.wrapping_sub(PRIME64_1);
    // do not write into reserved
    unsafe {
        core::ptr::copy_nonoverlapping(
            &state as *const XXH64_state_t as *const u8,
            state_ptr as *mut u8,
            core::mem::size_of::<XXH64_state_t>() - core::mem::size_of::<[u32; 2]>(),
        );
    }
    XXH_OK
}

unsafe fn xxh64_update_endian(state: *mut XXH64_state_t, input: *const c_void, len: usize) -> c_int {
    if input.is_null() {
        return XXH_ERROR;
    }

    unsafe {
        let mut p = input as *const u8;
        let b_end = p.add(len);

        (*state).total_len = (*state).total_len.wrapping_add(len as u64);

        if ((*state).memsize as usize + len) < 32 {
            let dst = ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize);
            core::ptr::copy_nonoverlapping(p, dst, len);
            (*state).memsize += len as u32;
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            let dst = ((*state).mem64.as_mut_ptr() as *mut u8).add((*state).memsize as usize);
            let n = 32 - (*state).memsize as usize;
            core::ptr::copy_nonoverlapping(p, dst, n);

            let mem64 = (*state).mem64.as_ptr();
            (*state).v1 = xxh64_round((*state).v1, xxh_get64bits(mem64.add(0) as *const u8));
            (*state).v2 = xxh64_round((*state).v2, xxh_get64bits(mem64.add(1) as *const u8));
            (*state).v3 = xxh64_round((*state).v3, xxh_get64bits(mem64.add(2) as *const u8));
            (*state).v4 = xxh64_round((*state).v4, xxh_get64bits(mem64.add(3) as *const u8));

            p = p.add(32 - (*state).memsize as usize);
            (*state).memsize = 0;
        }

        if p.add(32) <= b_end {
            let limit = b_end.sub(32);
            let mut v1 = (*state).v1;
            let mut v2 = (*state).v2;
            let mut v3 = (*state).v3;
            let mut v4 = (*state).v4;

            loop {
                v1 = xxh64_round(v1, xxh_get64bits(p));
                p = p.add(8);
                v2 = xxh64_round(v2, xxh_get64bits(p));
                p = p.add(8);
                v3 = xxh64_round(v3, xxh_get64bits(p));
                p = p.add(8);
                v4 = xxh64_round(v4, xxh_get64bits(p));
                p = p.add(8);
                if !(p <= limit) {
                    break;
                }
            }

            (*state).v1 = v1;
            (*state).v2 = v2;
            (*state).v3 = v3;
            (*state).v4 = v4;
        }

        if p < b_end {
            let n = b_end.offset_from(p) as usize;
            core::ptr::copy_nonoverlapping(p, (*state).mem64.as_mut_ptr() as *mut u8, n);
            (*state).memsize = n as u32;
        }
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_update(state_in: *mut XXH64_state_t, input: *const c_void, len: usize) -> c_int {
    unsafe { xxh64_update_endian(state_in, input, len) }
}

unsafe fn xxh64_digest_endian(state: *const XXH64_state_t) -> u64 {
    unsafe {
        let h64: u64;
        if (*state).total_len >= 32 {
            let v1 = (*state).v1;
            let v2 = (*state).v2;
            let v3 = (*state).v3;
            let v4 = (*state).v4;

            let mut h = xxh_rotl64(v1, 1)
                .wrapping_add(xxh_rotl64(v2, 7))
                .wrapping_add(xxh_rotl64(v3, 12))
                .wrapping_add(xxh_rotl64(v4, 18));
            h = xxh64_merge_round(h, v1);
            h = xxh64_merge_round(h, v2);
            h = xxh64_merge_round(h, v3);
            h = xxh64_merge_round(h, v4);
            h64 = h;
        } else {
            h64 = (*state).v3.wrapping_add(PRIME64_5);
        }

        let h64 = h64.wrapping_add((*state).total_len);

        xxh64_finalize(h64, (*state).mem64.as_ptr() as *const u8, (*state).total_len as usize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_digest(state_in: *const XXH64_state_t) -> XXH64_hash_t {
    unsafe { xxh64_digest_endian(state_in) }
}

/*====== Canonical representation   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, hash: XXH64_hash_t) {
    let hash = xxh_swap64(hash);
    unsafe {
        core::ptr::copy_nonoverlapping(&hash as *const u64 as *const u8, dst as *mut u8, 8);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> XXH64_hash_t {
    unsafe { xxh_read_be64(src as *const u8) }
}
