//! Hashing + hash-index construction, translated from `c_src/src/lib.c`.

use core::ffi::{c_char, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::cffi::{memset, realloc};
use crate::types::*;

/// `static size_t stbds_hash_seed=0x31415926;`
///
/// Modelled with a relaxed atomic so that the (single threaded) read-modify-write
/// sequences of the C original are reproduced without relying on `static mut`.
static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x3141_5926);

#[inline]
fn hash_seed_get() -> usize {
    STBDS_HASH_SEED.load(Ordering::Relaxed)
}

#[inline]
fn hash_seed_set(v: usize) {
    STBDS_HASH_SEED.store(v, Ordering::Relaxed);
}

/// ```c
/// void stbds_rand_seed(size_t seed) { stbds_hash_seed = seed; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    hash_seed_set(seed);
}

/// `#define STBDS_SIZE_T_BITS ((sizeof (size_t)) * 8)`
pub const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

/// ```c
/// #define stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo) \
///   temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16, \
///   var = v64_hi, var <<= 16, var <<= 16, \
///   var ^= temp ^ v32
/// ```
#[inline]
fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp: usize = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ v32;
    var
}

/// ```c
/// static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)
/// { size_t pos; STBDS_NOTUSED(slot_log2); pos = hash & (slot_count-1); return pos; }
/// ```
#[inline]
pub fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
}

/// ```c
/// static size_t stbds_log2(size_t slot_count)
/// { size_t n=0; while (slot_count > 1) { slot_count >>= 1; ++n; } return n; }
/// ```
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// ```c
/// static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)
/// ```
pub unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let t: *mut StbdsHashIndex = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<StbdsHashBucket>())
            .wrapping_add(core::mem::size_of::<StbdsHashIndex>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut StbdsHashIndex;

    (*t).storage =
        stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count - (slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }
    stbds_assert!(
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
        "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count\0",
        401,
        "stbds_make_hash_index\0"
    );

    if !ot.is_null() {
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*ot).string) as *const u8,
            ptr::addr_of_mut!((*t).string) as *mut u8,
            core::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = (*ot).seed;
    } else {
        memset(
            ptr::addr_of_mut!((*t).string) as *mut c_void,
            0,
            core::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = hash_seed_get();
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b: *mut StbdsHashBucket = (*t).storage.add(i);
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
            let ob: *mut StbdsHashBucket = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash: usize = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step: usize = STBDS_BUCKET_LENGTH;
                    'place: loop {
                        let bucket: *mut StbdsHashBucket =
                            (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'place; // goto done;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'place; // goto done;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
                // done: ;
            }
            i += 1;
        }
    }

    t
}

/// `#define STBDS_ROTATE_LEFT(val, n)  (((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n))))`
#[inline]
fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n) (((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n))))`
#[inline]
fn rotr(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

/// ```c
/// size_t stbds_hash_string(char *str, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = rotl(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotr(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotr(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

/// ```c
/// static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)
/// ```
///
/// Note the deliberate faithfulness to the C integer promotions: the little
/// endian loads are built from `int` sub-expressions, so `d[3] << 24` /
/// `d[7] << 24` are *signed* and sign-extend when converted to `size_t`.
/// That quirk is part of the observable hash value and must be preserved.
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! siproundc {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = rotl(v1, 13);
            v1 ^= v0;
            v0 = rotl(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = rotl(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotl(v1, 17);
            v1 ^= v2;
            v2 = rotl(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = rotl(v3, 21);
            v3 ^= v0;
        }};
    }

    let sz = core::mem::size_of::<usize>();
    let mut i: usize = 0;
    while i + sz <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);   /* int math */
        let lo: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32) << 24);
        data = lo as isize as usize; // sign extension, exactly as C does
        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: i32 = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | ((*d.add(7) as i32) << 24);
        data |= ((hi as isize as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siproundc!();
        }
        v0 ^= data;

        i += sz;
        d = d.add(sz);
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) { ... } with C fall-through semantics
    let rem = len.wrapping_sub(i);
    if rem >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        // (d[3] << 24) is int arithmetic -> sign extends into size_t
        data |= ((*d.add(3) as i32) << 24) as isize as usize;
    }
    if rem >= 3 {
        data |= ((*d.add(2) as i32) << 16) as isize as usize;
    }
    if rem >= 2 {
        data |= ((*d.add(1) as i32) << 8) as isize as usize;
    }
    if rem >= 1 {
        data |= (*d.add(0) as i32) as isize as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siproundc!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siproundc!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// ```c
/// size_t stbds_hash_bytes(void *p, size_t len, size_t seed) { return stbds_siphash_bytes(p,len,seed); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}
