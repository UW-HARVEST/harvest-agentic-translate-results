//! Hashing and hash-index construction (`stbds_rand_seed`, `stbds_hash_string`,
//! `stbds_hash_bytes`, `stbds_make_hash_index`, ...).

use core::ffi::{c_char, c_void};

use crate::ffi::*;

/// `static size_t stbds_hash_seed=0x31415926;`
pub static mut STBDS_HASH_SEED: usize = 0x3141_5926;

/// ```c
/// void stbds_rand_seed(size_t seed) { stbds_hash_seed = seed; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

/// ```c
/// static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)
/// { pos = hash & (slot_count-1); return pos; }
/// ```
#[inline]
pub fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
}

/// ```c
/// static size_t stbds_log2(size_t slot_count)
/// ```
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
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
/// static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)
/// ```
pub unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let t: *mut StbdsHashIndex = realloc(
        core::ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<StbdsHashBucket>()
            + core::mem::size_of::<StbdsHashIndex>()
            + STBDS_CACHE_LINE_SIZE
            - 1,
    ) as *mut StbdsHashIndex;
    (*t).storage =
        stbds_align_fwd(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
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
    stbds_assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        // memset(&t->string, 0, sizeof(t->string));
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!((*t).string) as *mut u8,
            0,
            core::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let a: usize = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        // stbds_load_32_or_64(b,temp,  715136305,          0, 0xb504f32d);
        let b: usize = load_32_or_64(715136305, 0, 0xb504f32d);
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
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
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket: *mut StbdsHashBucket =
                            (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        let mut placed = false;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                placed = true;
                                break;
                            }
                            z += 1;
                        }
                        if placed {
                            break 'done;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
            i += 1;
        }
    }

    t
}

/// ```c
/// #define stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)                    \
///   temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16,      \
///   var = v64_hi, var <<= 16, var <<= 16,                                        \
///   var ^= temp ^ v32
/// ```
/// `v32` is an `int` literal, `v64_hi`/`v64_lo` are `unsigned int` literals; the
/// XOR happens in `unsigned int` and is then zero-extended into `size_t`.
#[inline]
fn load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
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
/// size_t stbds_hash_string(char *str, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
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
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

/// ```c
/// static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)
/// ```
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let (mut v0, mut v1, mut v2, mut v3): (usize, usize, usize, usize);
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    // STBDS_SIPROUND()
    macro_rules! sipround {
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

    i = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // The right-hand side is an `int` expression, so the store into the
        // `size_t` sign-extends when d[3] has its high bit set.
        let lo: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32) << 24);
        data = lo as i64 as u64 as usize;
        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: i32 = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | ((*d.add(7) as i32) << 24);
        data |= ((hi as i64 as u64 as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) { case 7: ... fallthrough ... case 0: break; }
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
        // `(d[3] << 24)` is an int: sign-extends into size_t.
        data |= ((*d.add(3) as i32) << 24) as i64 as u64 as usize;
    }
    if rem >= 3 {
        data |= ((*d.add(2) as i32) << 16) as i64 as u64 as usize;
    }
    if rem >= 2 {
        data |= ((*d.add(1) as i32) << 8) as i64 as u64 as usize;
    }
    if rem >= 1 {
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        sipround!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        sipround!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// ```c
/// size_t stbds_hash_bytes(void *p, size_t len, size_t seed) { return stbds_siphash_bytes(p,len,seed); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}
