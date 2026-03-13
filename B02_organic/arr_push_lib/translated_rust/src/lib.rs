#![allow(
    non_snake_case,
    non_camel_case_types,
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

const STBDS_SIZE_T_BITS: usize = std::mem::size_of::<usize>() * 8;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

// ============================================================
// Structs
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
pub struct stbds_string_arena {
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
// Global state
// ============================================================

static mut stbds_hash_seed: usize = 0x31415926;

// ============================================================
// Helper functions (inline equivalents of C macros)
// ============================================================

unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

unsafe fn stbds_realloc(p: *mut u8, size: usize) -> *mut u8 {
    if p.is_null() {
        if size == 0 {
            return ptr::null_mut();
        }
        let layout = Layout::from_size_align_unchecked(size, 8);
        alloc::alloc(layout)
    } else {
        // We don't know the old size, so we use realloc with a generous old layout.
        // In C this is just realloc(p, size). We approximate by using size as old size too.
        // Actually, we need to use libc realloc for exact C semantics.
        libc_realloc(p, size)
    }
}

unsafe fn libc_realloc(p: *mut u8, size: usize) -> *mut u8 {
    extern "C" {
        fn realloc(p: *mut u8, size: usize) -> *mut u8;
        fn free(p: *mut u8);
        fn malloc(size: usize) -> *mut u8;
    }
    if p.is_null() {
        malloc(size)
    } else {
        realloc(p, size)
    }
}

unsafe fn stbds_free(p: *mut u8) {
    extern "C" {
        fn free(p: *mut u8);
    }
    free(p);
}

fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// ============================================================
// stbds_rand_seed
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
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
    let arr_len: isize = if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    };
    let arr_cap: usize = if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    };

    let min_len = arr_len as usize + addlen;
    let mut min_cap = min_cap;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= arr_cap {
        return a;
    }

    if min_cap < 2 * arr_cap {
        min_cap = 2 * arr_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b = if a.is_null() {
        libc_realloc(ptr::null_mut(), alloc_size)
    } else {
        libc_realloc(stbds_header(a) as *mut u8, alloc_size)
    };

    let b = b.add(std::mem::size_of::<stbds_array_header>());

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }

    (*stbds_header(b)).capacity = min_cap;

    b
}

// ============================================================
// stbds_arrfreef
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) {
    stbds_free(stbds_header(a) as *mut u8);
}

// ============================================================
// stbds_log2, stbds_probe_position
// ============================================================

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

// ============================================================
// stbds_load_32_or_64
// ============================================================

fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    // On 64-bit: temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16
    // This zeroes out the upper 32 bits of (v64_lo ^ v32) on 64-bit, keeping lower 32.
    // var = v64_hi << 32, var ^= temp ^ v32
    let mut temp: usize = v64_lo ^ v32;
    temp = temp.wrapping_shl(16).wrapping_shl(16).wrapping_shr(16).wrapping_shr(16);
    let mut var: usize = v64_hi;
    var = var.wrapping_shl(16).wrapping_shl(16);
    var ^= temp ^ v32;
    var
}

// ============================================================
// stbds_make_hash_index
// ============================================================

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = libc_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;

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
        (*t).string = ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = stbds_hash_seed;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    // Initialize buckets
    {
        let num_buckets = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..num_buckets {
            let bucket = &mut *(*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                bucket.hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                bucket.index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }

    // Rehash from old table
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let num_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..num_buckets {
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
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash ^= hash ^ stbds_rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ stbds_rotate_right(hash, 11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// ============================================================
// stbds_siphash_bytes / stbds_hash_bytes
// ============================================================

unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;

    let mut v0: usize = ((0x736f6d65_usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261_usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765_usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462_usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;

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

    let mut i = 0usize;
    while i + std::mem::size_of::<usize>() <= len {
        let dd = d.add(i);
        let mut data: usize = *dd.add(0) as usize
            | (*dd.add(1) as usize) << 8
            | (*dd.add(2) as usize) << 16
            | (*dd.add(3) as usize) << 24;
        data |= ((*dd.add(4) as usize)
            | (*dd.add(5) as usize) << 8
            | (*dd.add(6) as usize) << 16
            | (*dd.add(7) as usize) << 24)
            .wrapping_shl(16)
            .wrapping_shl(16);

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let mut data: usize = len.wrapping_shl((STBDS_SIZE_T_BITS - 8) as u32);
    let dd = d.add(i);
    let rem = len - i;
    // Fallthrough switch
    if rem >= 7 {
        data |= (*dd.add(6) as usize).wrapping_shl(24).wrapping_shl(24);
    }
    if rem >= 6 {
        data |= (*dd.add(5) as usize).wrapping_shl(20).wrapping_shl(20);
    }
    if rem >= 5 {
        data |= (*dd.add(4) as usize).wrapping_shl(16).wrapping_shl(16);
    }
    if rem >= 4 {
        data |= (*dd.add(3) as usize) << 24;
    }
    if rem >= 3 {
        data |= (*dd.add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*dd.add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *dd.add(0) as usize;
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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut std::ffi::c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p as *const u8, len, seed)
}

// ============================================================
// stbds_is_key_equal
// ============================================================

unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let str_ptr = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        libc_strcmp(key as *const i8, str_ptr as *const i8) == 0
    } else {
        libc_memcmp(
            key as *const std::ffi::c_void,
            a.add(elemsize * i as usize + keyoffset) as *const std::ffi::c_void,
            keysize,
        ) == 0
    }
}

extern "C" {
    fn strcmp(a: *const i8, b: *const i8) -> c_int;
    fn memcmp(a: *const std::ffi::c_void, b: *const std::ffi::c_void, n: usize) -> c_int;
    fn memmove(dst: *mut std::ffi::c_void, src: *const std::ffi::c_void, n: usize) -> *mut std::ffi::c_void;
    fn memset(s: *mut std::ffi::c_void, c: c_int, n: usize) -> *mut std::ffi::c_void;
    fn strlen(s: *const i8) -> usize;
    fn sprintf(s: *mut i8, fmt: *const i8, ...) -> c_int;
}

unsafe fn libc_strcmp(a: *const i8, b: *const i8) -> c_int {
    strcmp(a, b)
}

unsafe fn libc_memcmp(a: *const std::ffi::c_void, b: *const std::ffi::c_void, n: usize) -> c_int {
    memcmp(a, b, n)
}

// ============================================================
// STBDS_HASH_TO_ARR / STBDS_ARR_TO_HASH
// ============================================================

unsafe fn hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.sub(elemsize)
}

unsafe fn arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.add(elemsize)
}

unsafe fn hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ============================================================
// stbds_hmfree_func
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut std::ffi::c_void, elemsize: usize) {
    let a = a as *mut u8;
    if a.is_null() {
        return;
    }
    let ht = hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let p = *(a.add(elemsize * i) as *const *mut u8);
                stbds_free(p);
            }
        }
        stbds_strreset(&mut (*ht).string);
    }
    stbds_free((*stbds_header(a)).hash_table as *mut u8);
    stbds_free(stbds_header(a) as *mut u8);
}

// ============================================================
// stbds_hm_find_slot
// ============================================================

unsafe fn stbds_hm_find_slot(
    a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut u8, (*table).seed)
    } else {
        stbds_hash_bytes(key as *mut std::ffi::c_void, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        for i in 0..start {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
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
// stbds_hmget_key_ts
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let mut a = a as *mut u8;
    let key = key as *const u8;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a as *mut std::ffi::c_void, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(a, elemsize) as *mut std::ffi::c_void;
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, 0, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = &*(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                *temp = b.index[slot as usize & STBDS_BUCKET_MASK];
            }
        }
        return a as *mut std::ffi::c_void;
    }
}

// ============================================================
// stbds_hmget_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let raw = hash_to_arr(p as *mut u8, elemsize);
    (*stbds_header(raw)).temp = temp;
    p
}

// ============================================================
// stbds_hmput_default
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    a: *mut std::ffi::c_void,
    elemsize: usize,
) -> *mut std::ffi::c_void {
    let mut a = a as *mut u8;
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a as *mut std::ffi::c_void, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a as *mut std::ffi::c_void
}

// ============================================================
// stbds_strdup (internal)
// ============================================================

unsafe fn stbds_strdup_internal(str: *const u8) -> *mut u8 {
    let len = strlen(str as *const i8) + 1;
    let p = libc_realloc(ptr::null_mut(), len);
    memmove(p as *mut std::ffi::c_void, str as *const std::ffi::c_void, len);
    p
}

// ============================================================
// stbds_hmput_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let keyoffset: usize = 0;
    let mut a = a as *mut u8;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a as *mut std::ffi::c_void, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

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
        (*stbds_header(a)).hash_table = nt as *mut std::ffi::c_void;
        table = nt;
    }

    let key = key as *const u8;
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut u8, (*table).seed)
    } else {
        stbds_hash_bytes(key as *mut std::ffi::c_void, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(a)).temp = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut u8;
                        *temp_key_ptr = *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset) as *const *mut u8);
                    }
                    return arr_to_hash(a, elemsize) as *mut std::ffi::c_void;
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                // goto found_empty_slot
                return stbds_hmput_key_found_empty(a, raw_a, elemsize, key, keysize, keyoffset, mode, table, hash, pos, tombstone);
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
                    (*stbds_header(a)).temp = bucket.index[i];
                    return arr_to_hash(a, elemsize) as *mut std::ffi::c_void;
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                return stbds_hmput_key_found_empty(a, raw_a, elemsize, key, keysize, keyoffset, mode, table, hash, pos, tombstone);
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
    mut a: *mut u8,
    mut raw_a: *const u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    _keyoffset: usize,
    mode: c_int,
    table: *mut stbds_hash_index,
    hash: usize,
    mut pos: usize,
    tombstone: isize,
) -> *mut std::ffi::c_void {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = (*stbds_header(a)).length as isize;
    if (i as usize) + 1 > (*stbds_header(a)).capacity {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
        raw_a = arr_to_hash(a, elemsize);
    }

    assert!((i as usize) + 1 <= (*stbds_header(a)).capacity);
    (*stbds_header(a)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*stbds_header(a)).temp = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup_internal(key);
            *(a.add(elemsize * i as usize) as *mut *mut u8) = dup;
            let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut u8;
            *temp_key_ptr = dup;
        }
        STBDS_SH_ARENA => {
            let ht = (*stbds_header(a)).hash_table as *mut stbds_hash_index;
            let alloc = stbds_stralloc(&mut (*ht).string, key as *mut u8);
            *(a.add(elemsize * i as usize) as *mut *mut u8) = alloc as *mut u8;
            let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut u8;
            *temp_key_ptr = alloc as *mut u8;
        }
        STBDS_SH_DEFAULT => {
            *(a.add(elemsize * i as usize) as *mut *const u8) = key;
            let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *const u8;
            *temp_key_ptr = key;
        }
        _ => {
            memmove(
                a.add(elemsize * i as usize) as *mut std::ffi::c_void,
                key as *const std::ffi::c_void,
                keysize,
            );
        }
    }

    arr_to_hash(a, elemsize) as *mut std::ffi::c_void
}

// ============================================================
// stbds_shmode_func
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(
    elemsize: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a as *mut std::ffi::c_void, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut std::ffi::c_void;
    arr_to_hash(a, elemsize) as *mut std::ffi::c_void
}

// ============================================================
// stbds_hmdel_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let a = a as *mut u8;
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let mut table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    (*stbds_header(raw_a)).temp = 0;

    if table.is_null() {
        return a as *mut std::ffi::c_void;
    }

    let key = key as *const u8;
    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a as *mut std::ffi::c_void;
    }

    let b = &mut *(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = (*stbds_header(raw_a)).length as isize - 1 - 1;

    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 always true for usize

    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *(a.add(elemsize * old_index as usize) as *const *mut u8);
        stbds_free(p);
    }

    if old_index != final_index {
        memmove(
            a.add(elemsize * old_index as usize) as *mut std::ffi::c_void,
            a.add(elemsize * final_index as usize) as *const std::ffi::c_void,
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            let k = *(a.add(elemsize * old_index as usize + keyoffset) as *const *const u8);
            stbds_hm_find_slot(a, elemsize, k, keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(
                a,
                elemsize,
                a.add(elemsize * old_index as usize + keyoffset),
                keysize,
                keyoffset,
                mode,
            )
        };
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add(slot2 as usize >> STBDS_BUCKET_SHIFT);
        let i2 = slot2 as usize & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }

    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        let nt = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*stbds_header(raw_a)).hash_table = nt as *mut std::ffi::c_void;
        stbds_free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let nt = stbds_make_hash_index((*table).slot_count, table);
        (*stbds_header(raw_a)).hash_table = nt as *mut std::ffi::c_void;
        stbds_free(table as *mut u8);
    }

    a as *mut std::ffi::c_void
}

// ============================================================
// stbds_stralloc
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut u8,
) -> *mut u8 {
    let len = strlen(str as *const i8) + 1;
    if len > (*a).remaining {
        let blocksize_shift = (*a).block;
        let mut blocksize =
            (STBDS_STRING_ARENA_BLOCKSIZE_MIN).wrapping_shl((blocksize_shift >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = libc_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            memmove(
                (*sb).storage.as_mut_ptr() as *mut std::ffi::c_void,
                str as *const std::ffi::c_void,
                len,
            );
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
            let sb = libc_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(
        p as *mut std::ffi::c_void,
        str as *const std::ffi::c_void,
        len,
    );
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
        a as *mut std::ffi::c_void,
        0,
        std::mem::size_of::<stbds_string_arena>(),
    );
}

// ============================================================
// stbds_unit_tests (stub — not called by arr_push but declared extern)
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {
    // The C code declares this extern but the test body is not in the
    // provided source. Provide an empty stub.
}

// ============================================================
// arr_push — the public function
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();

    // STBDS_ASSERT(arrlen(arr)==0)
    assert!(stbds_arrlen(arr) == 0);

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            // arrpush(arr, j)
            arr = stbds_arrmaybegrow(arr, 1);
            let hdr = stbds_header(arr as *mut u8);
            let idx = (*hdr).length;
            (*hdr).length += 1;
            *arr.add(idx) = j;
            j += 1;
        }
        // arrfree(arr)
        if !arr.is_null() {
            stbds_free(stbds_header(arr as *mut u8) as *mut u8);
        }
        arr = ptr::null_mut();
        i += 50;
    }
}

// Helper: arrlen for typed pointer
unsafe fn stbds_arrlen(a: *const c_int) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a as *mut u8)).length as isize
    }
}

// Helper: arrmaybegrow — returns possibly-reallocated pointer
unsafe fn stbds_arrmaybegrow(a: *mut c_int, n: usize) -> *mut c_int {
    if a.is_null()
        || (*stbds_header(a as *mut u8)).length + n > (*stbds_header(a as *mut u8)).capacity
    {
        stbds_arrgrowf(a as *mut u8, std::mem::size_of::<c_int>(), n, 0) as *mut c_int
    } else {
        a
    }
}
