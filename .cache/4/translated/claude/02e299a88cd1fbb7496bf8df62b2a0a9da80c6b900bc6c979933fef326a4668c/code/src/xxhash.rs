//! Translation of `c_src/src/xxhash.c` (compiled with `XXH_NAMESPACE=LZ4_`).

use crate::common::{free, malloc, mem_copy, mem_init};
use core::ffi::{c_uint, c_void};

pub const XXH_VERSION_MAJOR: u32 = 0;
pub const XXH_VERSION_MINOR: u32 = 6;
pub const XXH_VERSION_RELEASE: u32 = 5;
pub const XXH_VERSION_NUMBER: u32 =
    XXH_VERSION_MAJOR * 100 * 100 + XXH_VERSION_MINOR * 100 + XXH_VERSION_RELEASE;

/* XXH_errorcode */
pub const XXH_OK: c_uint = 0;
pub const XXH_ERROR: c_uint = 1;
pub type XXH_errorcode = c_uint;

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

#[inline(always)]
unsafe fn XXH_read32(p: *const u8) -> u32 {
    core::ptr::read_unaligned(p as *const u32)
}

#[inline(always)]
unsafe fn XXH_read64(p: *const u8) -> u64 {
    core::ptr::read_unaligned(p as *const u64)
}

#[inline(always)]
fn XXH_swap32(x: u32) -> u32 {
    x.swap_bytes()
}

#[inline(always)]
fn XXH_swap64(x: u64) -> u64 {
    x.swap_bytes()
}

#[inline(always)]
fn XXH_rotl32(x: u32, r: u32) -> u32 {
    (x << r) | (x >> (32 - r))
}

#[inline(always)]
fn XXH_rotl64(x: u64, r: u32) -> u64 {
    (x << r) | (x >> (64 - r))
}

#[inline(always)]
fn XXH_CPU_LITTLE_ENDIAN() -> bool {
    cfg!(target_endian = "little")
}

/* XXH_endianness */
const XXH_bigEndian: i32 = 0;
const XXH_littleEndian: i32 = 1;

/* XXH_alignment */
const XXH_aligned: i32 = 0;
const XXH_unaligned: i32 = 1;

#[inline(always)]
unsafe fn XXH_readLE32_align(p: *const u8, endian: i32, align: i32) -> u32 {
    if align == XXH_unaligned {
        if endian == XXH_littleEndian {
            XXH_read32(p)
        } else {
            XXH_swap32(XXH_read32(p))
        }
    } else {
        if endian == XXH_littleEndian {
            *(p as *const u32)
        } else {
            XXH_swap32(*(p as *const u32))
        }
    }
}

#[inline(always)]
unsafe fn XXH_readLE32(p: *const u8, endian: i32) -> u32 {
    XXH_readLE32_align(p, endian, XXH_unaligned)
}

#[inline(always)]
unsafe fn XXH_readBE32(p: *const u8) -> u32 {
    if XXH_CPU_LITTLE_ENDIAN() {
        XXH_swap32(XXH_read32(p))
    } else {
        XXH_read32(p)
    }
}

#[inline(always)]
unsafe fn XXH_readLE64_align(p: *const u8, endian: i32, align: i32) -> u64 {
    if align == XXH_unaligned {
        if endian == XXH_littleEndian {
            XXH_read64(p)
        } else {
            XXH_swap64(XXH_read64(p))
        }
    } else {
        if endian == XXH_littleEndian {
            *(p as *const u64)
        } else {
            XXH_swap64(*(p as *const u64))
        }
    }
}

#[inline(always)]
unsafe fn XXH_readLE64(p: *const u8, endian: i32) -> u64 {
    XXH_readLE64_align(p, endian, XXH_unaligned)
}

#[inline(always)]
unsafe fn XXH_readBE64(p: *const u8) -> u64 {
    if XXH_CPU_LITTLE_ENDIAN() {
        XXH_swap64(XXH_read64(p))
    } else {
        XXH_read64(p)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH_versionNumber() -> c_uint {
    XXH_VERSION_NUMBER
}

/* *******************************************************************
*  32-bit hash functions
*********************************************************************/
const PRIME32_1: u32 = 2654435761;
const PRIME32_2: u32 = 2246822519;
const PRIME32_3: u32 = 3266489917;
const PRIME32_4: u32 = 668265263;
const PRIME32_5: u32 = 374761393;

#[inline(always)]
fn XXH32_round(seed: u32, input: u32) -> u32 {
    let mut seed = seed.wrapping_add(input.wrapping_mul(PRIME32_2));
    seed = XXH_rotl32(seed, 13);
    seed = seed.wrapping_mul(PRIME32_1);
    seed
}

#[inline(always)]
fn XXH32_avalanche(h32: u32) -> u32 {
    let mut h32 = h32;
    h32 ^= h32 >> 15;
    h32 = h32.wrapping_mul(PRIME32_2);
    h32 ^= h32 >> 13;
    h32 = h32.wrapping_mul(PRIME32_3);
    h32 ^= h32 >> 16;
    h32
}

unsafe fn XXH32_finalize(
    h32_in: u32,
    ptr: *const u8,
    len: usize,
    endian: i32,
    align: i32,
) -> u32 {
    let mut h32 = h32_in;
    let mut p = ptr;

    macro_rules! process1 {
        () => {{
            h32 = h32.wrapping_add((*p as u32).wrapping_mul(PRIME32_5));
            p = p.wrapping_add(1);
            h32 = XXH_rotl32(h32, 11).wrapping_mul(PRIME32_1);
        }};
    }
    macro_rules! process4 {
        () => {{
            h32 = h32.wrapping_add(XXH_readLE32_align(p, endian, align).wrapping_mul(PRIME32_3));
            p = p.wrapping_add(4);
            h32 = XXH_rotl32(h32, 17).wrapping_mul(PRIME32_4);
        }};
    }

    match len & 15 {
        12 => {
            process4!();
            process4!();
            process4!();
            return XXH32_avalanche(h32);
        }
        8 => {
            process4!();
            process4!();
            return XXH32_avalanche(h32);
        }
        4 => {
            process4!();
            return XXH32_avalanche(h32);
        }

        13 => {
            process4!();
            process4!();
            process4!();
            process1!();
            return XXH32_avalanche(h32);
        }
        9 => {
            process4!();
            process4!();
            process1!();
            return XXH32_avalanche(h32);
        }
        5 => {
            process4!();
            process1!();
            return XXH32_avalanche(h32);
        }

        14 => {
            process4!();
            process4!();
            process4!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        10 => {
            process4!();
            process4!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        6 => {
            process4!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }

        15 => {
            process4!();
            process4!();
            process4!();
            process1!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        11 => {
            process4!();
            process4!();
            process1!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        7 => {
            process4!();
            process1!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        3 => {
            process1!();
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        2 => {
            process1!();
            process1!();
            return XXH32_avalanche(h32);
        }
        1 => {
            process1!();
            return XXH32_avalanche(h32);
        }
        0 => {
            return XXH32_avalanche(h32);
        }
        _ => {}
    }
    h32
}

#[inline(always)]
unsafe fn XXH32_endian_align(
    input: *const c_void,
    len: usize,
    seed: u32,
    endian: i32,
    align: i32,
) -> u32 {
    let mut p = input as *const u8;
    let bEnd = p.wrapping_add(len);
    let mut h32: u32;

    if len >= 16 {
        let limit = bEnd.wrapping_sub(15);
        let mut v1: u32 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
        let mut v2: u32 = seed.wrapping_add(PRIME32_2);
        let mut v3: u32 = seed.wrapping_add(0);
        let mut v4: u32 = seed.wrapping_sub(PRIME32_1);

        loop {
            v1 = XXH32_round(v1, XXH_readLE32_align(p, endian, align));
            p = p.wrapping_add(4);
            v2 = XXH32_round(v2, XXH_readLE32_align(p, endian, align));
            p = p.wrapping_add(4);
            v3 = XXH32_round(v3, XXH_readLE32_align(p, endian, align));
            p = p.wrapping_add(4);
            v4 = XXH32_round(v4, XXH_readLE32_align(p, endian, align));
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

    XXH32_finalize(h32, p, len & 15, endian, align)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32(input: *const c_void, len: usize, seed: c_uint) -> c_uint {
    let endian_detected: i32 = if XXH_CPU_LITTLE_ENDIAN() {
        XXH_littleEndian
    } else {
        XXH_bigEndian
    };

    /* XXH_FORCE_ALIGN_CHECK == 0 on x86/x86_64 */
    if XXH_FORCE_ALIGN_CHECK {
        if ((input as usize) & 3) == 0 {
            if endian_detected == XXH_littleEndian {
                return XXH32_endian_align(input, len, seed, XXH_littleEndian, XXH_aligned);
            } else {
                return XXH32_endian_align(input, len, seed, XXH_bigEndian, XXH_aligned);
            }
        }
    }

    if endian_detected == XXH_littleEndian {
        XXH32_endian_align(input, len, seed, XXH_littleEndian, XXH_unaligned)
    } else {
        XXH32_endian_align(input, len, seed, XXH_bigEndian, XXH_unaligned)
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const XXH_FORCE_ALIGN_CHECK: bool = false;
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
const XXH_FORCE_ALIGN_CHECK: bool = true;

/*======   Hash streaming   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_createState() -> *mut XXH32_state_t {
    malloc(core::mem::size_of::<XXH32_state_t>()) as *mut XXH32_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_freeState(statePtr: *mut XXH32_state_t) -> XXH_errorcode {
    free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_copyState(
    dstState: *mut XXH32_state_t,
    srcState: *const XXH32_state_t,
) {
    mem_copy(
        dstState as *mut u8,
        srcState as *const u8,
        core::mem::size_of::<XXH32_state_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_reset(
    statePtr: *mut XXH32_state_t,
    seed: c_uint,
) -> XXH_errorcode {
    let mut state = core::mem::MaybeUninit::<XXH32_state_t>::uninit();
    let sp = state.as_mut_ptr();
    mem_init(sp as *mut u8, 0, core::mem::size_of::<XXH32_state_t>());
    (*sp).v1 = seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2);
    (*sp).v2 = seed.wrapping_add(PRIME32_2);
    (*sp).v3 = seed.wrapping_add(0);
    (*sp).v4 = seed.wrapping_sub(PRIME32_1);
    /* do not write into reserved */
    mem_copy(
        statePtr as *mut u8,
        sp as *const u8,
        core::mem::size_of::<XXH32_state_t>() - core::mem::size_of::<u32>(),
    );
    XXH_OK
}

#[inline(always)]
unsafe fn XXH32_update_endian(
    state: *mut XXH32_state_t,
    input: *const c_void,
    len: usize,
    endian: i32,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_ERROR;
    }

    {
        let mut p = input as *const u8;
        let bEnd = p.wrapping_add(len);

        (*state).total_len_32 = (*state).total_len_32.wrapping_add(len as u32);
        (*state).large_len |= ((len >= 16) as u32) | (((*state).total_len_32 >= 16) as u32);

        if ((*state).memsize as usize) + len < 16 {
            mem_copy(
                ((*state).mem32.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                input as *const u8,
                len,
            );
            (*state).memsize = (*state).memsize.wrapping_add(len as u32);
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            mem_copy(
                ((*state).mem32.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                input as *const u8,
                16 - (*state).memsize as usize,
            );
            {
                let mut p32 = (*state).mem32.as_ptr() as *const u8;
                (*state).v1 = XXH32_round((*state).v1, XXH_readLE32(p32, endian));
                p32 = p32.wrapping_add(4);
                (*state).v2 = XXH32_round((*state).v2, XXH_readLE32(p32, endian));
                p32 = p32.wrapping_add(4);
                (*state).v3 = XXH32_round((*state).v3, XXH_readLE32(p32, endian));
                p32 = p32.wrapping_add(4);
                (*state).v4 = XXH32_round((*state).v4, XXH_readLE32(p32, endian));
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
                v1 = XXH32_round(v1, XXH_readLE32(p, endian));
                p = p.wrapping_add(4);
                v2 = XXH32_round(v2, XXH_readLE32(p, endian));
                p = p.wrapping_add(4);
                v3 = XXH32_round(v3, XXH_readLE32(p, endian));
                p = p.wrapping_add(4);
                v4 = XXH32_round(v4, XXH_readLE32(p, endian));
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
            mem_copy(
                (*state).mem32.as_mut_ptr() as *mut u8,
                p,
                bEnd as usize - p as usize,
            );
            (*state).memsize = (bEnd as usize - p as usize) as u32;
        }
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_update(
    state_in: *mut XXH32_state_t,
    input: *const c_void,
    len: usize,
) -> XXH_errorcode {
    if XXH_CPU_LITTLE_ENDIAN() {
        XXH32_update_endian(state_in, input, len, XXH_littleEndian)
    } else {
        XXH32_update_endian(state_in, input, len, XXH_bigEndian)
    }
}

#[inline(always)]
unsafe fn XXH32_digest_endian(state: *const XXH32_state_t, endian: i32) -> u32 {
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
        (*state).mem32.as_ptr() as *const u8,
        (*state).memsize as usize,
        endian,
        XXH_aligned,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_digest(state_in: *const XXH32_state_t) -> c_uint {
    if XXH_CPU_LITTLE_ENDIAN() {
        XXH32_digest_endian(state_in, XXH_littleEndian)
    } else {
        XXH32_digest_endian(state_in, XXH_bigEndian)
    }
}

/*======   Canonical representation   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_canonicalFromHash(
    dst: *mut XXH32_canonical_t,
    hash: c_uint,
) {
    let mut hash = hash;
    if XXH_CPU_LITTLE_ENDIAN() {
        hash = XXH_swap32(hash);
    }
    mem_copy(dst as *mut u8, &hash as *const u32 as *const u8, 4);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH32_hashFromCanonical(src: *const XXH32_canonical_t) -> c_uint {
    XXH_readBE32(src as *const u8)
}

/* *******************************************************************
*  64-bit hash functions
*********************************************************************/
const PRIME64_1: u64 = 11400714785074694791;
const PRIME64_2: u64 = 14029467366897019727;
const PRIME64_3: u64 = 1609587929392839161;
const PRIME64_4: u64 = 9650029242287828579;
const PRIME64_5: u64 = 2870177450012600261;

#[inline(always)]
fn XXH64_round(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = XXH_rotl64(acc, 31);
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
fn XXH64_avalanche(h64: u64) -> u64 {
    let mut h64 = h64;
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

unsafe fn XXH64_finalize(
    h64_in: u64,
    ptr: *const u8,
    len: usize,
    endian: i32,
    align: i32,
) -> u64 {
    let mut h64 = h64_in;
    let mut p = ptr;

    macro_rules! process1_64 {
        () => {{
            h64 ^= (*p as u64).wrapping_mul(PRIME64_5);
            p = p.wrapping_add(1);
            h64 = XXH_rotl64(h64, 11).wrapping_mul(PRIME64_1);
        }};
    }
    macro_rules! process4_64 {
        () => {{
            h64 ^= (XXH_readLE32_align(p, endian, align) as u64).wrapping_mul(PRIME64_1);
            p = p.wrapping_add(4);
            h64 = XXH_rotl64(h64, 23)
                .wrapping_mul(PRIME64_2)
                .wrapping_add(PRIME64_3);
        }};
    }
    macro_rules! process8_64 {
        () => {{
            let k1 = XXH64_round(0, XXH_readLE64_align(p, endian, align));
            p = p.wrapping_add(8);
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
            return XXH64_avalanche(h64);
        }
        16 => {
            process8_64!();
            process8_64!();
            return XXH64_avalanche(h64);
        }
        8 => {
            process8_64!();
            return XXH64_avalanche(h64);
        }

        28 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            return XXH64_avalanche(h64);
        }
        20 => {
            process8_64!();
            process8_64!();
            process4_64!();
            return XXH64_avalanche(h64);
        }
        12 => {
            process8_64!();
            process4_64!();
            return XXH64_avalanche(h64);
        }
        4 => {
            process4_64!();
            return XXH64_avalanche(h64);
        }

        25 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        17 => {
            process8_64!();
            process8_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        9 => {
            process8_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }

        29 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        21 => {
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        13 => {
            process8_64!();
            process4_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        5 => {
            process4_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }

        26 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        18 => {
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        10 => {
            process8_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }

        30 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        22 => {
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        14 => {
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        6 => {
            process4_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }

        27 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        19 => {
            process8_64!();
            process8_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        11 => {
            process8_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }

        31 => {
            process8_64!();
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        23 => {
            process8_64!();
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        15 => {
            process8_64!();
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        7 => {
            process4_64!();
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        3 => {
            process1_64!();
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        2 => {
            process1_64!();
            process1_64!();
            return XXH64_avalanche(h64);
        }
        1 => {
            process1_64!();
            return XXH64_avalanche(h64);
        }
        0 => {
            return XXH64_avalanche(h64);
        }
        _ => {}
    }

    0
}

#[inline(always)]
unsafe fn XXH64_endian_align(
    input: *const c_void,
    len: usize,
    seed: u64,
    endian: i32,
    align: i32,
) -> u64 {
    let mut p = input as *const u8;
    let bEnd = p.wrapping_add(len);
    let mut h64: u64;

    if len >= 32 {
        let limit = bEnd.wrapping_sub(32);
        let mut v1: u64 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
        let mut v2: u64 = seed.wrapping_add(PRIME64_2);
        let mut v3: u64 = seed.wrapping_add(0);
        let mut v4: u64 = seed.wrapping_sub(PRIME64_1);

        loop {
            v1 = XXH64_round(v1, XXH_readLE64_align(p, endian, align));
            p = p.wrapping_add(8);
            v2 = XXH64_round(v2, XXH_readLE64_align(p, endian, align));
            p = p.wrapping_add(8);
            v3 = XXH64_round(v3, XXH_readLE64_align(p, endian, align));
            p = p.wrapping_add(8);
            v4 = XXH64_round(v4, XXH_readLE64_align(p, endian, align));
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

    XXH64_finalize(h64, p, len, endian, align)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64(
    input: *const c_void,
    len: usize,
    seed: u64,
) -> u64 {
    let endian_detected: i32 = if XXH_CPU_LITTLE_ENDIAN() {
        XXH_littleEndian
    } else {
        XXH_bigEndian
    };

    if XXH_FORCE_ALIGN_CHECK {
        if ((input as usize) & 7) == 0 {
            if endian_detected == XXH_littleEndian {
                return XXH64_endian_align(input, len, seed, XXH_littleEndian, XXH_aligned);
            } else {
                return XXH64_endian_align(input, len, seed, XXH_bigEndian, XXH_aligned);
            }
        }
    }

    if endian_detected == XXH_littleEndian {
        XXH64_endian_align(input, len, seed, XXH_littleEndian, XXH_unaligned)
    } else {
        XXH64_endian_align(input, len, seed, XXH_bigEndian, XXH_unaligned)
    }
}

/*======   Hash Streaming   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_createState() -> *mut XXH64_state_t {
    malloc(core::mem::size_of::<XXH64_state_t>()) as *mut XXH64_state_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_freeState(statePtr: *mut XXH64_state_t) -> XXH_errorcode {
    free(statePtr as *mut c_void);
    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_copyState(
    dstState: *mut XXH64_state_t,
    srcState: *const XXH64_state_t,
) {
    mem_copy(
        dstState as *mut u8,
        srcState as *const u8,
        core::mem::size_of::<XXH64_state_t>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_reset(
    statePtr: *mut XXH64_state_t,
    seed: u64,
) -> XXH_errorcode {
    let mut state = core::mem::MaybeUninit::<XXH64_state_t>::uninit();
    let sp = state.as_mut_ptr();
    mem_init(sp as *mut u8, 0, core::mem::size_of::<XXH64_state_t>());
    (*sp).v1 = seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2);
    (*sp).v2 = seed.wrapping_add(PRIME64_2);
    (*sp).v3 = seed.wrapping_add(0);
    (*sp).v4 = seed.wrapping_sub(PRIME64_1);
    /* do not write into reserved */
    mem_copy(
        statePtr as *mut u8,
        sp as *const u8,
        core::mem::size_of::<XXH64_state_t>() - core::mem::size_of::<[u32; 2]>(),
    );
    XXH_OK
}

#[inline(always)]
unsafe fn XXH64_update_endian(
    state: *mut XXH64_state_t,
    input: *const c_void,
    len: usize,
    endian: i32,
) -> XXH_errorcode {
    if input.is_null() {
        return XXH_ERROR;
    }

    {
        let mut p = input as *const u8;
        let bEnd = p.wrapping_add(len);

        (*state).total_len = (*state).total_len.wrapping_add(len as u64);

        if ((*state).memsize as usize) + len < 32 {
            mem_copy(
                ((*state).mem64.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                input as *const u8,
                len,
            );
            (*state).memsize = (*state).memsize.wrapping_add(len as u32);
            return XXH_OK;
        }

        if (*state).memsize != 0 {
            mem_copy(
                ((*state).mem64.as_mut_ptr() as *mut u8).wrapping_add((*state).memsize as usize),
                input as *const u8,
                32 - (*state).memsize as usize,
            );
            let m = (*state).mem64.as_ptr() as *const u8;
            (*state).v1 = XXH64_round((*state).v1, XXH_readLE64(m.wrapping_add(0), endian));
            (*state).v2 = XXH64_round((*state).v2, XXH_readLE64(m.wrapping_add(8), endian));
            (*state).v3 = XXH64_round((*state).v3, XXH_readLE64(m.wrapping_add(16), endian));
            (*state).v4 = XXH64_round((*state).v4, XXH_readLE64(m.wrapping_add(24), endian));
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
                v1 = XXH64_round(v1, XXH_readLE64(p, endian));
                p = p.wrapping_add(8);
                v2 = XXH64_round(v2, XXH_readLE64(p, endian));
                p = p.wrapping_add(8);
                v3 = XXH64_round(v3, XXH_readLE64(p, endian));
                p = p.wrapping_add(8);
                v4 = XXH64_round(v4, XXH_readLE64(p, endian));
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
            mem_copy(
                (*state).mem64.as_mut_ptr() as *mut u8,
                p,
                bEnd as usize - p as usize,
            );
            (*state).memsize = (bEnd as usize - p as usize) as u32;
        }
    }

    XXH_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_update(
    state_in: *mut XXH64_state_t,
    input: *const c_void,
    len: usize,
) -> XXH_errorcode {
    if XXH_CPU_LITTLE_ENDIAN() {
        XXH64_update_endian(state_in, input, len, XXH_littleEndian)
    } else {
        XXH64_update_endian(state_in, input, len, XXH_bigEndian)
    }
}

#[inline(always)]
unsafe fn XXH64_digest_endian(state: *const XXH64_state_t, endian: i32) -> u64 {
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
        (*state).mem64.as_ptr() as *const u8,
        (*state).total_len as usize,
        endian,
        XXH_aligned,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_digest(state_in: *const XXH64_state_t) -> u64 {
    if XXH_CPU_LITTLE_ENDIAN() {
        XXH64_digest_endian(state_in, XXH_littleEndian)
    } else {
        XXH64_digest_endian(state_in, XXH_bigEndian)
    }
}

/*====== Canonical representation   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_canonicalFromHash(dst: *mut XXH64_canonical_t, hash: u64) {
    let mut hash = hash;
    if XXH_CPU_LITTLE_ENDIAN() {
        hash = XXH_swap64(hash);
    }
    mem_copy(dst as *mut u8, &hash as *const u64 as *const u8, 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4_XXH64_hashFromCanonical(src: *const XXH64_canonical_t) -> u64 {
    XXH_readBE64(src as *const u8)
}
