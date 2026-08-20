//! Hashing (`stbds_hash_string`, `stbds_hash_bytes`, siphash-2-4), the random
//! seed, and hash-index construction/rehashing.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr::null_mut;

use crate::*;

/// `static size_t stbds_hash_seed=0x31415926;`
static mut stbds_hash_seed: usize = 0x3141_5926;

#[inline]
pub(crate) unsafe fn hash_seed_get() -> usize {
    *(&raw const stbds_hash_seed)
}

#[inline]
pub(crate) unsafe fn hash_seed_set(v: usize) {
    *(&raw mut stbds_hash_seed) = v;
}

/// ```c
/// void stbds_rand_seed(size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    hash_seed_set(seed);
}

/// `#define STBDS_ROTATE_LEFT(val, n) (((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n))))`
///
/// `wrapping_shl`/`wrapping_shr` reproduce the x86 shift semantics that the C
/// compiler emits for the (technically undefined) `n == 0` case.
#[inline]
pub(crate) fn STBDS_ROTATE_LEFT(val: usize, n: u32) -> usize {
    val.wrapping_shl(n) | val.wrapping_shr(STBDS_SIZE_T_BITS.wrapping_sub(n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n) (((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n))))`
#[inline]
pub(crate) fn STBDS_ROTATE_RIGHT(val: usize, n: u32) -> usize {
    val.wrapping_shr(n) | val.wrapping_shl(STBDS_SIZE_T_BITS.wrapping_sub(n))
}

/// ```c
/// static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)
/// ```
pub(crate) fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    // STBDS_NOTUSED(slot_log2);
    hash & slot_count.wrapping_sub(1)
}

/// ```c
/// static size_t stbds_log2(size_t slot_count)
/// ```
pub(crate) fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// Expansion of the `stbds_load_32_or_64` macro for a 64-bit `size_t`:
///
/// ```c
/// temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16,
/// var = v64_hi, var <<= 16, var <<= 16,
/// var ^= temp ^ v32
/// ```
///
/// `v32` is an `int` constant (positive here, so widening is value preserving)
/// and `v64_lo`/`v64_hi` are `unsigned int` constants, so the xor happens in
/// 32-bit arithmetic and is then zero-extended.
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ (v32 as usize);
    var
}

/// ```c
/// static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)
/// ```
pub(crate) unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t: *mut stbds_hash_index = STBDS_REALLOC(
        null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(size_of::<stbds_hash_bucket>())
            .wrapping_add(size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = STBDS_ALIGN_FWD(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE)
        as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }
    STBDS_ASSERT(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count,
    );
    // STBDS_STATS(++stbds_hash_alloc);
    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        memset(
            (&raw mut (*t).string) as *mut c_void,
            0,
            size_of::<stbds_string_arena>(),
        );
        (*t).seed = hash_seed_get();
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b: *mut stbds_hash_bucket = (*t).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let mut i: usize = 0;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob: *mut stbds_hash_bucket = (*ot).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if STBDS_INDEX_IN_USE((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    // STBDS_STATS(++stbds_rehash_items);
                    'outer: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
                        // STBDS_STATS(++stbds_rehash_probes);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer; // goto done;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer; // goto done;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
                // done: ;
            }
            i += 1;
        }
    }

    t
}

/// ```c
/// size_t stbds_hash_string(char *str, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut str_ = str_ as *const u8;
    while *str_ != 0 {
        // hash = STBDS_ROTATE_LEFT(hash, 9) + (unsigned char) *str++;
        hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*str_ as usize);
        str_ = str_.wrapping_add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= STBDS_ROTATE_RIGHT(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

/// ```c
/// #define STBDS_SIPROUND() do { ... } while (0)
/// ```
#[inline(always)]
fn STBDS_SIPROUND(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = STBDS_ROTATE_LEFT(*v1, 13);
    *v1 ^= *v0;
    *v0 = STBDS_ROTATE_LEFT(*v0, STBDS_SIZE_T_BITS / 2);

    *v2 = v2.wrapping_add(*v3);
    *v3 = STBDS_ROTATE_LEFT(*v3, 16);
    *v3 ^= *v2;

    *v2 = v2.wrapping_add(*v1);
    *v1 = STBDS_ROTATE_LEFT(*v1, 17);
    *v1 ^= *v2;
    *v2 = STBDS_ROTATE_LEFT(*v2, STBDS_SIZE_T_BITS / 2);

    *v0 = v0.wrapping_add(*v3);
    *v3 = STBDS_ROTATE_LEFT(*v3, 21);
    *v3 ^= *v0;
}

/// ```c
/// static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)
/// ```
///
/// The `data` assembly reproduces C's integer promotions exactly: the four
/// low bytes are combined in `int` arithmetic, so a byte >= 0x80 at index 3
/// makes the `int` negative and the conversion to `size_t` sign-extends.
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i: usize = 0;
    while i.wrapping_add(size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);   /* int math */
        let lo: u32 = (*d.wrapping_add(0) as u32)
            | ((*d.wrapping_add(1) as u32) << 8)
            | ((*d.wrapping_add(2) as u32) << 16)
            | ((*d.wrapping_add(3) as u32) << 24);
        data = (lo as i32) as isize as usize; // sign extension of the int value
        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: u32 = (*d.wrapping_add(4) as u32)
            | ((*d.wrapping_add(5) as u32) << 8)
            | ((*d.wrapping_add(6) as u32) << 16)
            | ((*d.wrapping_add(7) as u32) << 24);
        data |= ((hi as usize) << 16) << 16;

        v3 ^= data;
        for _j in 0..STBDS_SIPHASH_C_ROUNDS {
            STBDS_SIPROUND(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i = i.wrapping_add(size_of::<usize>());
        d = d.wrapping_add(size_of::<usize>());
    }

    data = len.wrapping_shl(STBDS_SIZE_T_BITS - 8);
    let rem = len.wrapping_sub(i);
    // switch (len - i) with fall-through
    if rem == 7 {
        data |= ((*d.wrapping_add(6) as usize) << 24) << 24;
    }
    if rem >= 6 && rem <= 7 {
        data |= ((*d.wrapping_add(5) as usize) << 20) << 20;
    }
    if rem >= 5 && rem <= 7 {
        data |= ((*d.wrapping_add(4) as usize) << 16) << 16;
    }
    if rem >= 4 && rem <= 7 {
        // data |= (d[3] << 24);  /* int expression -> sign-extended to size_t */
        data |= (((*d.wrapping_add(3) as u32) << 24) as i32) as isize as usize;
    }
    if rem >= 3 && rem <= 7 {
        data |= (*d.wrapping_add(2) as usize) << 16;
    }
    if rem >= 2 && rem <= 7 {
        data |= (*d.wrapping_add(1) as usize) << 8;
    }
    if rem >= 1 && rem <= 7 {
        data |= *d.wrapping_add(0) as usize;
    }

    v3 ^= data;
    for _j in 0..STBDS_SIPHASH_C_ROUNDS {
        STBDS_SIPROUND(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _j in 0..STBDS_SIPHASH_D_ROUNDS {
        STBDS_SIPROUND(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// ```c
/// size_t stbds_hash_bytes(void *p, size_t len, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}
