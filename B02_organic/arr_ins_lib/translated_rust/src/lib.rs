#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::missing_safety_doc
)]

use std::alloc::{self, Layout};
use std::ffi::c_int;
use std::ptr;

// ============================================================
// Constants
// ============================================================
const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: usize = 64;
const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

static mut stbds_hash_seed: usize = 0x31415926;

// ============================================================
// Structs — must match C layout exactly
// ============================================================
#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut std::ffi::c_void,
    temp: isize,
}

#[repr(C)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [u8; 8],
}

#[repr(C)]
struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct stbds_hash_index {
    temp_key: *mut u8,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: stbds_string_arena,
    storage: *mut stbds_hash_bucket,
}

// ============================================================
// Helper: C-compatible malloc/realloc/free via libc
// ============================================================
extern "C" {
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn malloc(size: usize) -> *mut u8;
    fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> c_int;
    fn strcmp(s1: *const u8, s2: *const u8) -> c_int;
    fn strlen(s: *const u8) -> usize;
    fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn memset(s: *mut u8, c: c_int, n: usize) -> *mut u8;
    fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn sprintf(s: *mut u8, format: *const u8, ...) -> c_int;
}

fn stbds_realloc(p: *mut u8, s: usize) -> *mut u8 {
    unsafe { realloc(p, s) }
}

fn stbds_free(p: *mut u8) {
    unsafe { free(p) }
}

// ============================================================
// Header access helpers
// ============================================================
#[inline]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline]
unsafe fn stbds_arrlenu(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length
    }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ============================================================
// stbds_arrgrowf
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut u8,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut u8 {
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let old_ptr = if !a.is_null() {
        stbds_header(a) as *mut u8
    } else {
        ptr::null_mut()
    };
    let b_raw = stbds_realloc(old_ptr, alloc_size);
    let b = b_raw.add(std::mem::size_of::<stbds_array_header>());

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }

    (*stbds_header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) {
    stbds_free(stbds_header(a) as *mut u8);
}

// ============================================================
// stbds_rand_seed
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

// ============================================================
// Hash helpers
// ============================================================
#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// ============================================================
// stbds_hash_string
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str;
    while *p != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*p as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ stbds_rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ stbds_rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// ============================================================
// stbds_siphash_bytes
// ============================================================
unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = stbds_rotate_left(v1, 13);
            v1 ^= v0;
            v0 = stbds_rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3);
            v3 = stbds_rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = stbds_rotate_left(v1, 17);
            v1 ^= v2;
            v2 = stbds_rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3);
            v3 = stbds_rotate_left(v3, 21);
            v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dp = d.add(i);
        let mut data: usize = *dp.add(0) as usize
            | ((*dp.add(1) as usize) << 8)
            | ((*dp.add(2) as usize) << 16)
            | ((*dp.add(3) as usize) << 24);
        data |= ((*dp.add(4) as usize)
            | ((*dp.add(5) as usize) << 8)
            | ((*dp.add(6) as usize) << 16)
            | ((*dp.add(7) as usize) << 24))
            << 16
            << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let dp = d.add(i);
    let rem = len - i;
    // fallthrough switch
    if rem >= 7 {
        data |= (*dp.add(6) as usize) << 24 << 24;
    }
    if rem >= 6 {
        data |= (*dp.add(5) as usize) << 20 << 20;
    }
    if rem >= 5 {
        data |= (*dp.add(4) as usize) << 16 << 16;
    }
    if rem >= 4 {
        data |= (*dp.add(3) as usize) << 24;
    }
    if rem >= 3 {
        data |= (*dp.add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*dp.add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *dp.add(0) as usize;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ============================================================
// stbds_make_hash_index
// ============================================================
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let num_buckets = slot_count >> STBDS_BUCKET_SHIFT;
    let alloc_size = num_buckets * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    (*t).storage = stbds_align_fwd(
        (t.add(1)) as usize,
        STBDS_CACHE_LINE_SIZE,
    ) as *mut stbds_hash_bucket;
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
    assert!(
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
    );

    if !ot.is_null() {
        (*t).string = std::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        memset(
            &mut (*t).string as *mut stbds_string_arena as *mut u8,
            0,
            std::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        let (a, b): (usize, usize);
        // stbds_load_32_or_64 for a: v32=2147001325, v64_hi=0x27bb2ee6, v64_lo=0x87b0b0fd
        {
            let v32: usize = 2147001325;
            let v64_hi: usize = 0x27bb2ee6;
            let v64_lo: usize = 0x87b0b0fd;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var: usize = v64_hi;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ v32;
            a = var;
        }
        // stbds_load_32_or_64 for b: v32=715136305, v64_hi=0, v64_lo=0xb504f32d
        {
            let v32: usize = 715136305;
            let v64_hi: usize = 0;
            let v64_lo: usize = 0xb504f32d;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var: usize = v64_hi;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ v32;
            b = var;
        }
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    // Initialize buckets
    for i in 0..num_buckets {
        let bucket = &mut *(*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            bucket.hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            bucket.index[j] = STBDS_INDEX_EMPTY;
        }
    }

    // Rehash from old table
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let old_num_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..old_num_buckets {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let hash = ob.hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = &mut *(*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let start = pos & STBDS_BUCKET_MASK;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }
                        for z in 0..start {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }
                        pos += step;
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }

    t
}

// ============================================================
// stbds_is_key_equal
// ============================================================
unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let str_ptr = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        strcmp(key, str_ptr) == 0
    } else {
        memcmp(key, a.add(elemsize * i as usize + keyoffset), keysize) == 0
    }
}

// ============================================================
// stbds_hm_find_slot
// ============================================================
unsafe fn stbds_hm_find_slot(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = a.sub(elemsize); // STBDS_HASH_TO_ARR
    let table = stbds_hash_table(raw_a);
    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let hash = if hash < 2 { hash + 2 } else { hash };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i])
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i])
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

// ============================================================
// stbds_strdup
// ============================================================
unsafe fn stbds_strdup(str: *const u8) -> *mut u8 {
    let len = strlen(str) + 1;
    let p = stbds_realloc(ptr::null_mut(), len);
    memmove(p, str, len);
    p
}

// ============================================================
// stbds_stralloc
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut u8) -> *mut u8 {
    let len = strlen(str) + 1;
    if len > (*a).remaining {
        let blocksize_shift = (*a).block >> 1;
        let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_shift as usize);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = stbds_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            memmove((*sb).storage.as_mut_ptr(), str, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = stbds_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p, str, len);
    p
}

// ============================================================
// stbds_strreset
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut u8);
        x = y;
    }
    memset(
        a as *mut u8,
        0,
        std::mem::size_of::<stbds_string_arena>(),
    );
}

// ============================================================
// stbds_hmfree_func
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = stbds_hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                stbds_free(*(a.add(elemsize * i) as *const *mut u8));
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    stbds_free((*stbds_header(a)).hash_table as *mut u8);
    stbds_free(stbds_header(a) as *mut u8);
}

// ============================================================
// stbds_hmget_key_ts
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        memset(new_a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return new_a.add(elemsize); // STBDS_ARR_TO_HASH
    } else {
        let raw_a = a.sub(elemsize); // STBDS_HASH_TO_ARR
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = &*(*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                *temp = b.index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        return a;
    }
}

// ============================================================
// stbds_hmget_key
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    mode: c_int,
) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let raw_p = p.sub(elemsize); // STBDS_HASH_TO_ARR
    (*stbds_header(raw_p)).temp = temp;
    p
}

// ============================================================
// stbds_hmput_default
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*stbds_header(a.sub(elemsize))).length == 0 {
        let raw = if !a.is_null() {
            a.sub(elemsize)
        } else {
            ptr::null_mut()
        };
        let new_a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        memset(new_a, 0, elemsize);
        return new_a.add(elemsize);
    }
    a
}

// ============================================================
// stbds_hmput_key
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    mode: c_int,
) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a = a;

    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(new_a, 0, elemsize);
        (*stbds_header(new_a)).length += 1;
        a = new_a.add(elemsize); // ARR_TO_HASH
    }

    let raw_a = a;
    let arr_a = a.sub(elemsize); // HASH_TO_ARR

    let mut table = (*stbds_header(arr_a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            stbds_free(table as *mut u8);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*stbds_header(arr_a)).hash_table = nt as *mut std::ffi::c_void;
        table = nt;
    }

    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let hash = if hash < 2 { hash + 2 } else { hash };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    let mut tombstone: isize = -1;

    // Search loop
    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(arr_a)).temp = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        let temp_key_ptr =
                            (*stbds_header(arr_a)).hash_table as *mut *mut u8;
                        *temp_key_ptr = *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset)
                            as *const *mut u8);
                    }
                    return arr_a.add(elemsize); // ARR_TO_HASH
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                // goto found_empty_slot
                return stbds_hmput_key_found_empty(
                    arr_a, raw_a, elemsize, key, keysize, keyoffset, mode, table, pos, tombstone,
                );
            } else if tombstone < 0 {
                if bucket.index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(arr_a)).temp = bucket.index[i];
                    return arr_a.add(elemsize);
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                return stbds_hmput_key_found_empty(
                    arr_a, raw_a, elemsize, key, keysize, keyoffset, mode, table, pos, tombstone,
                );
            } else if tombstone < 0 {
                if bucket.index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
        }

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

unsafe fn stbds_hmput_key_found_empty(
    mut arr_a: *mut u8,
    raw_a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    _keyoffset: usize,
    mode: c_int,
    table: *mut stbds_hash_index,
    mut pos: usize,
    tombstone: isize,
) -> *mut u8 {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr_a);
    if (i as usize) + 1 > stbds_arrcap(arr_a) {
        arr_a = stbds_arrgrowf(arr_a, elemsize, 1, 0);
    }
    let new_raw_a = arr_a.add(elemsize); // ARR_TO_HASH

    assert!((i as usize) + 1 <= stbds_arrcap(arr_a));
    (*stbds_header(arr_a)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    let slot = pos & STBDS_BUCKET_MASK;
    bucket.hash[slot] = if pos == tombstone as usize {
        // recalculate - but actually the hash was already computed
        // In the C code, `hash` is in scope. We need to recompute or pass it.
        // Actually looking at the C code more carefully, `hash` is used directly.
        // We need to pass hash. Let me fix this by restructuring.
        // For now this path uses the hash variable from the caller.
        // This is a limitation - let me restructure.
        0 // placeholder - will fix
    } else {
        0 // placeholder
    };
    // Actually, I need to restructure to pass hash. Let me do that differently.
    // This approach won't work cleanly. Let me rewrite stbds_hmput_key as a single function.
    0 as *mut u8 // placeholder
}
