//! Transliteration of `common/xxhash.h` + `common/xxhash.c`.
//!
//! `common/xxhash.c` is just:
//! ```c
//! #define XXH_STATIC_LINKING_ONLY
//! #define XXH_IMPLEMENTATION
//! #include "xxhash.h"
//! ```
//!
//! zstd's local adaptation at the top of `xxhash.h` sets:
//!   * `XXH_NO_XXH3`  -> the whole XXH3 / XXH128 family is compiled out.
//!   * `XXH_NAMESPACE ZSTD_` -> every public symbol gets a `ZSTD_` prefix.
//!
//! `XXH_NO_LONG_LONG` is *not* defined, so XXH64 is present.
//! `XXH_NO_STREAM` / `XXH_NO_STDLIB` are *not* defined, so the streaming API and
//! `malloc`/`free` based state allocation are present.
//!
//! Build configuration reproduced here (gcc/clang on little-endian x86_64):
//!   * `XXH_FORCE_MEMORY_ACCESS == 1` (gcc, non-old-ARM)  -> `__attribute__((aligned(1)))`
//!     loads, i.e. plain unaligned loads.
//!   * `XXH_CPU_LITTLE_ENDIAN == 1`  (`__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__`)
//!   * `XXH_SIZE_OPT == 0`
//!   * `XXH_FORCE_ALIGN_CHECK == 0`  (`__x86_64__`)
//!   * `XXH32_ENDJMP == 0`

#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use crate::mem::{free, malloc, memcpy, memset};
use core::ffi::{c_int, c_uint, c_void};
use core::ptr::{addr_of, addr_of_mut};

/* *************************************
*  Version
***************************************/
pub const XXH_VERSION_MAJOR: c_uint = 0;
pub const XXH_VERSION_MINOR: c_uint = 8;
pub const XXH_VERSION_RELEASE: c_uint = 2;
pub const XXH_VERSION_NUMBER: c_uint =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

/* *************************************
*  typedef enum { XXH_OK = 0, XXH_ERROR } XXH_errorcode;
***************************************/
pub type XXH_errorcode = c_int;
pub const XXH_OK: XXH_errorcode = 0;
pub const XXH_ERROR: XXH_errorcode = 1;

/* *************************************
*  Hash types
***************************************/
pub type XXH32_hash_t = u32;
pub type XXH64_hash_t = u64;

pub type xxh_u8 = u8;
pub type xxh_u32 = XXH32_hash_t;
pub type xxh_u64 = XXH64_hash_t;

/* struct XXH32_state_s */
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH32_state_t {
    /// Total length hashed, modulo 2^32
    pub total_len_32: XXH32_hash_t,
    /// Whether the hash is >= 16 (handles `total_len_32` overflow)
    pub large_len: XXH32_hash_t,
    /// Accumulator lanes
    pub v: [XXH32_hash_t; 4],
    /// Internal buffer for partial reads. Treated as `unsigned char[16]`.
    pub mem32: [XXH32_hash_t; 4],
    /// Amount of data in `mem32`
    pub memsize: XXH32_hash_t,
    /// Reserved field. Do not read nor write to it.
    pub reserved: XXH32_hash_t,
}

/* struct XXH64_state_s */
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH64_state_t {
    /// Total length hashed. This is always 64-bit.
    pub total_len: XXH64_hash_t,
    /// Accumulator lanes
    pub v: [XXH64_hash_t; 4],
    /// Internal buffer for partial reads. Treated as `unsigned char[32]`.
    pub mem64: [XXH64_hash_t; 4],
    /// Amount of data in `mem64`
    pub memsize: XXH32_hash_t,
    /// Reserved field, needed for padding anyways
    pub reserved32: XXH32_hash_t,
    /// Reserved field. Do not read or write to it.
    pub reserved64: XXH64_hash_t,
}

/* typedef struct { unsigned char digest[4]; } XXH32_canonical_t; */
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH32_canonical_t {
    pub digest: [u8; 4],
}

/* typedef struct { unsigned char digest[sizeof(XXH64_hash_t)]; } XXH64_canonical_t; */
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XXH64_canonical_t {
    pub digest: [u8; 8],
}

/* *************************************
*  Tuning parameters, resolved for this build
***************************************/
pub const XXH_CPU_LITTLE_ENDIAN: c_int = 1;
pub const XXH_FORCE_ALIGN_CHECK: c_int = 0;
pub const XXH32_ENDJMP: c_int = 0;

/* *************************************
*  Memory related functions
***************************************/
#[inline(always)]
pub unsafe fn XXH_malloc(s: usize) -> *mut c_void {
    malloc(s)
}

#[inline(always)]
pub unsafe fn XXH_free(p: *mut c_void) {
    free(p)
}

#[inline(always)]
pub unsafe fn XXH_memcpy(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void {
    memcpy(dest, src, size)
}

/* ***************************
*  Memory reads
*****************************/

/// `typedef enum { XXH_aligned, XXH_unaligned } XXH_alignment;`
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum XXH_alignment {
    XXH_aligned,
    XXH_unaligned,
}
pub use XXH_alignment::{XXH_aligned, XXH_unaligned};

/// `XXH_FORCE_MEMORY_ACCESS == 1`:
/// ```c
/// typedef __attribute__((aligned(1))) xxh_u32 xxh_unalign32;
/// return *((const xxh_unalign32*)ptr);
/// ```
#[inline(always)]
pub unsafe fn XXH_read32(ptr: *const xxh_u8) -> xxh_u32 {
    core::ptr::read_unaligned(ptr as *const xxh_u32)
}

/// `XXH_swap32` == `__builtin_bswap32`
#[inline(always)]
pub fn XXH_swap32(x: xxh_u32) -> xxh_u32 {
    x.swap_bytes()
}

/// `XXH_CPU_LITTLE_ENDIAN ? XXH_read32(ptr) : XXH_swap32(XXH_read32(ptr))`
#[inline(always)]
pub unsafe fn XXH_readLE32(ptr: *const xxh_u8) -> xxh_u32 {
    u32::from_le(XXH_read32(ptr))
}

/// `XXH_CPU_LITTLE_ENDIAN ? XXH_swap32(XXH_read32(ptr)) : XXH_read32(ptr)`
#[inline(always)]
pub unsafe fn XXH_readBE32(ptr: *const xxh_u8) -> xxh_u32 {
    u32::from_be(XXH_read32(ptr))
}

/// With `XXH_FORCE_MEMORY_ACCESS == 1` both branches lower to the same
/// little-endian 32-bit load, so the `align` hint has no observable effect.
#[inline(always)]
pub unsafe fn XXH_readLE32_align(ptr: *const xxh_u8, align: XXH_alignment) -> xxh_u32 {
    if align == XXH_unaligned {
        XXH_readLE32(ptr)
    } else {
        u32::from_le(core::ptr::read_unaligned(ptr as *const xxh_u32))
    }
}

/// `#define XXH_get32bits(p) XXH_readLE32_align(p, align)`
#[inline(always)]
unsafe fn XXH_get32bits(p: *const xxh_u8, align: XXH_alignment) -> xxh_u32 {
    XXH_readLE32_align(p, align)
}

/* *************************************
*  Misc
***************************************/
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

/* *******************************************************************
*  32-bit hash functions
*********************************************************************/
pub const XXH_PRIME32_1: xxh_u32 = 0x9E3779B1;
pub const XXH_PRIME32_2: xxh_u32 = 0x85EBCA77;
pub const XXH_PRIME32_3: xxh_u32 = 0xC2B2AE3D;
pub const XXH_PRIME32_4: xxh_u32 = 0x27D4EB2F;
pub const XXH_PRIME32_5: xxh_u32 = 0x165667B1;

/// Normal stripe processing routine.
pub fn XXH32_round(mut acc: xxh_u32, input: xxh_u32) -> xxh_u32 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME32_2));
    acc = acc.rotate_left(13);
    acc = acc.wrapping_mul(XXH_PRIME32_1);
    acc
}

/// Mixes all bits to finalize the hash.
pub fn XXH32_avalanche(mut hash: xxh_u32) -> xxh_u32 {
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(XXH_PRIME32_2);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(XXH_PRIME32_3);
    hash ^= hash >> 16;
    hash
}

/// Processes the last 0-15 bytes of `ptr`.
///
/// `XXH32_ENDJMP == 0` -> the compact rerolled version is used.
pub unsafe fn XXH32_finalize(
    mut hash: xxh_u32,
    mut ptr: *const xxh_u8,
    mut len: usize,
    align: XXH_alignment,
) -> xxh_u32 {
    /* Compact rerolled version; generally faster */
    len &= 15;
    while len >= 4 {
        /* XXH_PROCESS4 */
        hash = hash.wrapping_add(XXH_get32bits(ptr, align).wrapping_mul(XXH_PRIME32_3));
        ptr = ptr.add(4);
        hash = hash.rotate_left(17).wrapping_mul(XXH_PRIME32_4);
        len -= 4;
    }
    while len > 0 {
        /* XXH_PROCESS1 */
        hash = hash.wrapping_add((*ptr as xxh_u32).wrapping_mul(XXH_PRIME32_5));
        ptr = ptr.add(1);
        hash = hash.rotate_left(11).wrapping_mul(XXH_PRIME32_1);
        len -= 1;
    }
    XXH32_avalanche(hash)
}

/// The implementation for `XXH32()`.
pub unsafe fn XXH32_endian_align(
    mut input: *const xxh_u8,
    len: usize,
    seed: xxh_u32,
    align: XXH_alignment,
) -> xxh_u32 {
    let mut h32: xxh_u32;

    if len >= 16 {
        let bEnd: *const xxh_u8 = input.wrapping_add(len);
        let limit: *const xxh_u8 = bEnd.wrapping_sub(15);
        let mut v1: xxh_u32 = seed
            .wrapping_add(XXH_PRIME32_1)
            .wrapping_add(XXH_PRIME32_2);
        let mut v2: xxh_u32 = seed.wrapping_add(XXH_PRIME32_2);
        let mut v3: xxh_u32 = seed.wrapping_add(0);
        let mut v4: xxh_u32 = seed.wrapping_sub(XXH_PRIME32_1);

        loop {
            v1 = XXH32_round(v1, XXH_get32bits(input, align));
            input = input.add(4);
            v2 = XXH32_round(v2, XXH_get32bits(input, align));
            input = input.add(4);
            v3 = XXH32_round(v3, XXH_get32bits(input, align));
            input = input.add(4);
            v4 = XXH32_round(v4, XXH_get32bits(input, align));
            input = input.add(4);
            if !(input < limit) {
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

    h32 = h32.wrapping_add(len as xxh_u32);

    XXH32_finalize(h32, input, len & 15, align)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32(
    input: *const c_void,
    len: usize,
    seed: XXH32_hash_t,
) -> XXH32_hash_t {
    if XXH_FORCE_ALIGN_CHECK != 0 {
        if ((input as usize) & 3) == 0 {
            /* Input is 4-bytes aligned, leverage the speed benefit */
            return XXH32_endian_align(input as *const xxh_u8, len, seed, XXH_aligned);
        }
    }

    XXH32_endian_align(input as *const xxh_u8, len, seed, XXH_unaligned)
}

/*******   Hash streaming   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_createState() -> *mut XXH32_state_t {
    XXH_malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_freeState(statePtr: *mut XXH32_state_t) -> XXH_errorcode {
    XXH_free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_copyState(
    dstState: *mut XXH32_state_t,
    srcState: *const XXH32_state_t,
) {
    XXH_memcpy(
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
    memset(
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
        let mut p: *const xxh_u8 = input as *const xxh_u8;
        let bEnd: *const xxh_u8 = p.wrapping_add(len);

        (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as XXH32_hash_t);
        (*state).large_len |=
            ((len >= 16) as XXH32_hash_t) | (((*state).total_len_32 >= 16) as XXH32_hash_t);

        if ((*state).memsize as usize).wrapping_add(len) < 16 {
            /* fill in tmp buffer */
            XXH_memcpy(
                (addr_of_mut!((*state).mem32) as *mut xxh_u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                len,
            );
            (*state).memsize = (*state).memsize.wrapping_add(len as XXH32_hash_t);
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            /* some data left from previous update */
            XXH_memcpy(
                (addr_of_mut!((*state).mem32) as *mut xxh_u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                16 - (*state).memsize as usize,
            );
            {
                let mut p32: *const xxh_u32 = addr_of!((*state).mem32) as *const xxh_u32;
                (*state).v[0] = XXH32_round((*state).v[0], XXH_readLE32(p32 as *const xxh_u8));
                p32 = p32.add(1);
                (*state).v[1] = XXH32_round((*state).v[1], XXH_readLE32(p32 as *const xxh_u8));
                p32 = p32.add(1);
                (*state).v[2] = XXH32_round((*state).v[2], XXH_readLE32(p32 as *const xxh_u8));
                p32 = p32.add(1);
                (*state).v[3] = XXH32_round((*state).v[3], XXH_readLE32(p32 as *const xxh_u8));
            }
            p = p.add(16 - (*state).memsize as usize);
            (*state).memsize = 0;
        }

        if p <= bEnd.wrapping_sub(16) {
            let limit: *const xxh_u8 = bEnd.wrapping_sub(16);

            loop {
                (*state).v[0] = XXH32_round((*state).v[0], XXH_readLE32(p));
                p = p.add(4);
                (*state).v[1] = XXH32_round((*state).v[1], XXH_readLE32(p));
                p = p.add(4);
                (*state).v[2] = XXH32_round((*state).v[2], XXH_readLE32(p));
                p = p.add(4);
                (*state).v[3] = XXH32_round((*state).v[3], XXH_readLE32(p));
                p = p.add(4);
                if !(p <= limit) {
                    break;
                }
            }
        }

        if p < bEnd {
            XXH_memcpy(
                addr_of_mut!((*state).mem32) as *mut c_void,
                p as *const c_void,
                (bEnd as usize) - (p as usize),
            );
            (*state).memsize = ((bEnd as usize) - (p as usize)) as c_uint;
        }
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_digest(state: *const XXH32_state_t) -> XXH32_hash_t {
    let mut h32: xxh_u32;

    if (*state).large_len != 0 {
        h32 = (*state).v[0]
            .rotate_left(1)
            .wrapping_add((*state).v[1].rotate_left(7))
            .wrapping_add((*state).v[2].rotate_left(12))
            .wrapping_add((*state).v[3].rotate_left(18));
    } else {
        h32 = (*state).v[2] /* == seed */.wrapping_add(XXH_PRIME32_5);
    }

    h32 = h32.wrapping_add((*state).total_len_32);

    XXH32_finalize(
        h32,
        addr_of!((*state).mem32) as *const xxh_u8,
        (*state).memsize as usize,
        XXH_aligned,
    )
}

/*******   Canonical representation   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(
    dst: *mut XXH32_canonical_t,
    hash: XXH32_hash_t,
) {
    let mut hash: XXH32_hash_t = hash;
    if XXH_CPU_LITTLE_ENDIAN != 0 {
        hash = XXH_swap32(hash);
    }
    XXH_memcpy(
        dst as *mut c_void,
        addr_of!(hash) as *const c_void,
        core::mem::size_of::<XXH32_canonical_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(
    src: *const XXH32_canonical_t,
) -> XXH32_hash_t {
    XXH_readBE32(src as *const xxh_u8)
}

/* *******************************************************************
*  64-bit hash functions
*********************************************************************/
/*******   Memory access   *******/

/// `XXH_FORCE_MEMORY_ACCESS == 1`:
/// ```c
/// typedef __attribute__((aligned(1))) xxh_u64 xxh_unalign64;
/// return *((const xxh_unalign64*)ptr);
/// ```
#[inline(always)]
pub unsafe fn XXH_read64(ptr: *const xxh_u8) -> xxh_u64 {
    core::ptr::read_unaligned(ptr as *const xxh_u64)
}

/// `XXH_swap64` == `__builtin_bswap64`
#[inline(always)]
pub fn XXH_swap64(x: xxh_u64) -> xxh_u64 {
    x.swap_bytes()
}

/// `XXH_CPU_LITTLE_ENDIAN ? XXH_read64(ptr) : XXH_swap64(XXH_read64(ptr))`
#[inline(always)]
pub unsafe fn XXH_readLE64(ptr: *const xxh_u8) -> xxh_u64 {
    u64::from_le(XXH_read64(ptr))
}

/// `XXH_CPU_LITTLE_ENDIAN ? XXH_swap64(XXH_read64(ptr)) : XXH_read64(ptr)`
#[inline(always)]
pub unsafe fn XXH_readBE64(ptr: *const xxh_u8) -> xxh_u64 {
    u64::from_be(XXH_read64(ptr))
}

/// With `XXH_FORCE_MEMORY_ACCESS == 1` both branches lower to the same
/// little-endian 64-bit load, so the `align` hint has no observable effect.
#[inline(always)]
pub unsafe fn XXH_readLE64_align(ptr: *const xxh_u8, align: XXH_alignment) -> xxh_u64 {
    if align == XXH_unaligned {
        XXH_readLE64(ptr)
    } else {
        u64::from_le(core::ptr::read_unaligned(ptr as *const xxh_u64))
    }
}

/// `#define XXH_get64bits(p) XXH_readLE64_align(p, align)`
#[inline(always)]
unsafe fn XXH_get64bits(p: *const xxh_u8, align: XXH_alignment) -> xxh_u64 {
    XXH_readLE64_align(p, align)
}

/*******   xxh64   *******/
pub const XXH_PRIME64_1: xxh_u64 = 0x9E3779B185EBCA87;
pub const XXH_PRIME64_2: xxh_u64 = 0xC2B2AE3D27D4EB4F;
pub const XXH_PRIME64_3: xxh_u64 = 0x165667B19E3779F9;
pub const XXH_PRIME64_4: xxh_u64 = 0x85EBCA77C2B2AE63;
pub const XXH_PRIME64_5: xxh_u64 = 0x27D4EB2F165667C5;

pub fn XXH64_round(mut acc: xxh_u64, input: xxh_u64) -> xxh_u64 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc = acc.wrapping_mul(XXH_PRIME64_1);
    acc
}

pub fn XXH64_mergeRound(mut acc: xxh_u64, val: xxh_u64) -> xxh_u64 {
    let mut val: xxh_u64 = val;
    val = XXH64_round(0, val);
    acc ^= val;
    acc = acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
    acc
}

pub fn XXH64_avalanche(mut hash: xxh_u64) -> xxh_u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(XXH_PRIME64_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(XXH_PRIME64_3);
    hash ^= hash >> 32;
    hash
}

/// Processes the last 0-31 bytes of `ptr`.
pub unsafe fn XXH64_finalize(
    mut hash: xxh_u64,
    mut ptr: *const xxh_u8,
    mut len: usize,
    align: XXH_alignment,
) -> xxh_u64 {
    len &= 31;
    while len >= 8 {
        let k1: xxh_u64 = XXH64_round(0, XXH_get64bits(ptr, align));
        ptr = ptr.add(8);
        hash ^= k1;
        hash = hash
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        hash ^= (XXH_get32bits(ptr, align) as xxh_u64).wrapping_mul(XXH_PRIME64_1);
        ptr = ptr.add(4);
        hash = hash
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        hash ^= (*ptr as xxh_u64).wrapping_mul(XXH_PRIME64_5);
        ptr = ptr.add(1);
        hash = hash.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    XXH64_avalanche(hash)
}

/// The implementation for `XXH64()`.
pub unsafe fn XXH64_endian_align(
    mut input: *const xxh_u8,
    len: usize,
    seed: xxh_u64,
    align: XXH_alignment,
) -> xxh_u64 {
    let mut h64: xxh_u64;

    if len >= 32 {
        let bEnd: *const xxh_u8 = input.wrapping_add(len);
        let limit: *const xxh_u8 = bEnd.wrapping_sub(31);
        let mut v1: xxh_u64 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2: xxh_u64 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3: xxh_u64 = seed.wrapping_add(0);
        let mut v4: xxh_u64 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = XXH64_round(v1, XXH_get64bits(input, align));
            input = input.add(8);
            v2 = XXH64_round(v2, XXH_get64bits(input, align));
            input = input.add(8);
            v3 = XXH64_round(v3, XXH_get64bits(input, align));
            input = input.add(8);
            v4 = XXH64_round(v4, XXH_get64bits(input, align));
            input = input.add(8);
            if !(input < limit) {
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

    h64 = h64.wrapping_add(len as xxh_u64);

    XXH64_finalize(h64, input, len, align)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64(
    input: *const c_void,
    len: usize,
    seed: XXH64_hash_t,
) -> XXH64_hash_t {
    if XXH_FORCE_ALIGN_CHECK != 0 {
        if ((input as usize) & 7) == 0 {
            /* Input is aligned, let's leverage the speed advantage */
            return XXH64_endian_align(input as *const xxh_u8, len, seed, XXH_aligned);
        }
    }

    XXH64_endian_align(input as *const xxh_u8, len, seed, XXH_unaligned)
}

/*******   Hash Streaming   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_createState() -> *mut XXH64_state_t {
    XXH_malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_freeState(statePtr: *mut XXH64_state_t) -> XXH_errorcode {
    XXH_free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_copyState(
    dstState: *mut XXH64_state_t,
    srcState: *const XXH64_state_t,
) {
    XXH_memcpy(
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
    memset(
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
        let mut p: *const xxh_u8 = input as *const xxh_u8;
        let bEnd: *const xxh_u8 = p.wrapping_add(len);

        (*state).total_len = (*state).total_len.wrapping_add(len as xxh_u64);

        if ((*state).memsize as usize).wrapping_add(len) < 32 {
            /* fill in tmp buffer */
            XXH_memcpy(
                (addr_of_mut!((*state).mem64) as *mut xxh_u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                len,
            );
            (*state).memsize = (*state).memsize.wrapping_add(len as xxh_u32);
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            /* tmp buffer is full */
            XXH_memcpy(
                (addr_of_mut!((*state).mem64) as *mut xxh_u8).add((*state).memsize as usize)
                    as *mut c_void,
                input,
                32 - (*state).memsize as usize,
            );
            let mem64: *const xxh_u64 = addr_of!((*state).mem64) as *const xxh_u64;
            (*state).v[0] = XXH64_round((*state).v[0], XXH_readLE64(mem64.add(0) as *const xxh_u8));
            (*state).v[1] = XXH64_round((*state).v[1], XXH_readLE64(mem64.add(1) as *const xxh_u8));
            (*state).v[2] = XXH64_round((*state).v[2], XXH_readLE64(mem64.add(2) as *const xxh_u8));
            (*state).v[3] = XXH64_round((*state).v[3], XXH_readLE64(mem64.add(3) as *const xxh_u8));
            p = p.add(32 - (*state).memsize as usize);
            (*state).memsize = 0;
        }

        if p.wrapping_add(32) <= bEnd {
            let limit: *const xxh_u8 = bEnd.wrapping_sub(32);

            loop {
                (*state).v[0] = XXH64_round((*state).v[0], XXH_readLE64(p));
                p = p.add(8);
                (*state).v[1] = XXH64_round((*state).v[1], XXH_readLE64(p));
                p = p.add(8);
                (*state).v[2] = XXH64_round((*state).v[2], XXH_readLE64(p));
                p = p.add(8);
                (*state).v[3] = XXH64_round((*state).v[3], XXH_readLE64(p));
                p = p.add(8);
                if !(p <= limit) {
                    break;
                }
            }
        }

        if p < bEnd {
            XXH_memcpy(
                addr_of_mut!((*state).mem64) as *mut c_void,
                p as *const c_void,
                (bEnd as usize) - (p as usize),
            );
            (*state).memsize = ((bEnd as usize) - (p as usize)) as c_uint;
        }
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(state: *const XXH64_state_t) -> XXH64_hash_t {
    let mut h64: xxh_u64;

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
        h64 = (*state).v[2] /* seed */.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add((*state).total_len as xxh_u64);

    XXH64_finalize(
        h64,
        addr_of!((*state).mem64) as *const xxh_u8,
        (*state).total_len as usize,
        XXH_aligned,
    )
}

/******* Canonical representation   *******/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(
    dst: *mut XXH64_canonical_t,
    hash: XXH64_hash_t,
) {
    let mut hash: XXH64_hash_t = hash;
    if XXH_CPU_LITTLE_ENDIAN != 0 {
        hash = XXH_swap64(hash);
    }
    XXH_memcpy(
        dst as *mut c_void,
        addr_of!(hash) as *const c_void,
        core::mem::size_of::<XXH64_canonical_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(
    src: *const XXH64_canonical_t,
) -> XXH64_hash_t {
    XXH_readBE64(src as *const xxh_u8)
}
