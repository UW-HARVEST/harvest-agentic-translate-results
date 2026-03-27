#![allow(
    non_camel_case_types,
    non_snake_case,
    clippy::missing_safety_doc,
    unused_assignments
)]

use libc::{c_int, c_void, free, malloc, memcmp, memcpy, memmove, realloc, size_t, strcmp, strlen};
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

// ============================================================
// Structs
// ============================================================

#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
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
// Helper macros/functions
// ============================================================

#[inline]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_temp(t: *mut u8) -> &'static mut isize {
    &mut (*stbds_header(t)).temp
}

#[inline]
unsafe fn stbds_temp_key(t: *mut u8) -> *mut *mut u8 {
    (*stbds_header(t)).hash_table as *mut *mut u8
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
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

#[inline]
fn hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.sub(elemsize) }
}

#[inline]
fn arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.add(elemsize) }
}

// ============================================================
// Global seed
// ============================================================

static mut STBDS_HASH_SEED: usize = 0x31415926;

// ============================================================
// stbds_rand_seed
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// ============================================================
// stbds_arrgrowf
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut c_void {
    let a = a as *mut u8;
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a as *mut c_void;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b = if a.is_null() {
        realloc(ptr::null_mut(), alloc_size)
    } else {
        realloc(stbds_header(a) as *mut c_void, alloc_size)
    };
    let b = (b as *mut u8).add(std::mem::size_of::<stbds_array_header>());

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }

    (*stbds_header(b)).capacity = min_cap;

    b as *mut c_void
}

// ============================================================
// stbds_arrfreef
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a as *mut u8) as *mut c_void);
}

// ============================================================
// Hash functions
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

unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;

    let mut v0 = ((0x736f6d65_usize << 16 << 16) + 0x70736575) ^ seed;
    let mut v1 = ((0x646f7261_usize << 16 << 16) + 0x6e646f6d) ^ !seed;
    let mut v2 = ((0x6c796765_usize << 16 << 16) + 0x6e657261) ^ seed;
    let mut v3 = ((0x74656462_usize << 16 << 16) + 0x79746573) ^ !seed;

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
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let dp = d.add(i);
    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let remaining = len - i;
    // fallthrough switch
    if remaining >= 7 {
        data |= (*dp.add(6) as usize) << 24 << 24;
    }
    if remaining >= 6 {
        data |= (*dp.add(5) as usize) << 20 << 20;
    }
    if remaining >= 5 {
        data |= (*dp.add(4) as usize) << 16 << 16;
    }
    if remaining >= 4 {
        data |= (*dp.add(3) as usize) << 24;
    }
    if remaining >= 3 {
        data |= (*dp.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        data |= (*dp.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        data |= *dp.add(0) as usize;
    }

    v3 ^= data;
    for _ in 0..2 {
        sipround!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p as *const u8, len, seed)
}

// ============================================================
// Hash index helpers
// ============================================================

unsafe fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    (*t).storage =
        stbds_align_fwd((t.add(1)) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
    assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = std::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64 for a
        let a: usize;
        {
            let mut temp: usize;
            temp = 0x87b0b0fd_usize ^ 2147001325_usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var = 0x27bb2ee6_usize;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ 2147001325_usize;
            a = var;
        }
        let b: usize;
        {
            let mut temp: usize;
            temp = 0xb504f32d_usize ^ 715136305_usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var = 0_usize;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ 715136305_usize;
            b = var;
        }
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    // Initialize buckets
    for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
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
        for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let hash = ob.hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = &mut *(*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
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
// Key comparison
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
        let stored_key_ptr = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        strcmp(key as *const i8, stored_key_ptr as *const i8) == 0
    } else {
        memcmp(
            key as *const c_void,
            a.add(elemsize * i as usize + keyoffset) as *const c_void,
            keysize,
        ) == 0
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
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key as *mut c_void, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash += 2;
    }

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
// stbds_hmfree_func
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    let a = a as *mut u8;
    if a.is_null() {
        return;
    }
    let ht = stbds_hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let p = *(a.add(elemsize * i) as *mut *mut u8);
                free(p as *mut c_void);
            }
        }
        stbds_strreset(&mut (*ht).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

// ============================================================
// stbds_hmget_key_ts
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let a = a as *mut u8;
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        (*stbds_header(new_a)).length += 1;
        ptr::write_bytes(new_a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(new_a, elemsize) as *mut c_void;
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = stbds_hash_table(raw_a);
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key as *mut u8, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = &*(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                *temp = b.index[slot as usize & STBDS_BUCKET_MASK];
            }
        }
        return a as *mut c_void;
    }
}

// ============================================================
// stbds_hmget_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let raw = hash_to_arr(p as *mut u8, elemsize);
    *stbds_temp(raw) = temp;
    p
}

// ============================================================
// stbds_hmput_default
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let a = a as *mut u8;
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        let new_a = stbds_arrgrowf(raw as *mut c_void, elemsize, 0, 1) as *mut u8;
        (*stbds_header(new_a)).length += 1;
        ptr::write_bytes(new_a, 0, elemsize);
        return arr_to_hash(new_a, elemsize) as *mut c_void;
    }
    a as *mut c_void
}

// ============================================================
// stbds_strdup
// ============================================================

unsafe fn stbds_strdup(str: *const u8) -> *mut u8 {
    let len = strlen(str as *const i8) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut u8;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ============================================================
// stbds_hmput_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut a = a as *mut u8;

    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        ptr::write_bytes(new_a, 0, elemsize);
        (*stbds_header(new_a)).length += 1;
        a = arr_to_hash(new_a, elemsize);
    }

    let raw_a = a;
    let mut arr = hash_to_arr(a, elemsize);

    let mut table = stbds_hash_table(arr);

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*stbds_header(arr)).hash_table = nt as *mut c_void;
        table = nt;
    }

    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut u8, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(
                    raw_a,
                    elemsize,
                    key as *mut u8,
                    keysize,
                    keyoffset,
                    mode,
                    bucket.index[i],
                ) {
                    *stbds_temp(arr) = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *stbds_temp_key(arr) = *(raw_a
                            .add(elemsize * bucket.index[i] as usize + keyoffset)
                            as *const *mut u8);
                    }
                    return arr_to_hash(arr, elemsize) as *mut c_void;
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                // goto found_empty_slot
                return stbds_hmput_key_found_empty(
                    arr, raw_a, elemsize, key, keysize, keyoffset, mode, table, pos, tombstone,
                    hash,
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
                if stbds_is_key_equal(
                    raw_a,
                    elemsize,
                    key as *mut u8,
                    keysize,
                    keyoffset,
                    mode,
                    bucket.index[i],
                ) {
                    *stbds_temp(arr) = bucket.index[i];
                    return arr_to_hash(arr, elemsize) as *mut c_void;
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                return stbds_hmput_key_found_empty(
                    arr, raw_a, elemsize, key, keysize, keyoffset, mode, table, pos, tombstone,
                    hash,
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
    mut arr: *mut u8,
    mut raw_a: *mut u8,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    _keyoffset: usize,
    mode: c_int,
    table: *mut stbds_hash_index,
    mut pos: usize,
    tombstone: isize,
    hash: usize,
) -> *mut c_void {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr);
    if (i as usize) + 1 > stbds_arrcap(arr) {
        arr = stbds_arrgrowf(arr as *mut c_void, elemsize, 1, 0) as *mut u8;
        raw_a = arr_to_hash(arr, elemsize);
    }

    assert!((i as usize) + 1 <= stbds_arrcap(arr));
    (*stbds_header(arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(arr) = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key as *const u8);
            *(arr.add(elemsize * i as usize) as *mut *mut u8) = dup;
            *stbds_temp_key(arr) = dup;
        }
        STBDS_SH_ARENA => {
            let alloc =
                stbds_stralloc(&mut (*table).string, key as *mut u8) as *mut u8;
            *(arr.add(elemsize * i as usize) as *mut *mut u8) = alloc;
            *stbds_temp_key(arr) = alloc;
        }
        STBDS_SH_DEFAULT => {
            let k = key as *mut u8;
            *(arr.add(elemsize * i as usize) as *mut *mut u8) = k;
            *stbds_temp_key(arr) = k;
        }
        _ => {
            memcpy(
                arr.add(elemsize * i as usize) as *mut c_void,
                key as *const c_void,
                keysize,
            );
        }
    }

    arr_to_hash(arr, elemsize) as *mut c_void
}

// ============================================================
// stbds_shmode_func
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut c_void;
    arr_to_hash(a, elemsize) as *mut c_void
}

// ============================================================
// stbds_hmdel_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    let a = a as *mut u8;
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let mut table = stbds_hash_table(raw_a);
    *stbds_temp(raw_a) = 0;

    if table.is_null() {
        return a as *mut c_void;
    }

    let slot = stbds_hm_find_slot(a, elemsize, key as *mut u8, keysize, keyoffset, mode);
    if slot < 0 {
        return a as *mut c_void;
    }

    let b = &mut *(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;

    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    assert!((*table).used_count < usize::MAX); // unsigned, always true like C
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *(a.add(elemsize * old_index as usize) as *mut *mut u8);
        free(p as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            a.add(elemsize * old_index as usize) as *mut c_void,
            a.add(elemsize * final_index as usize) as *const c_void,
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(
                a,
                elemsize,
                *(a.add(elemsize * old_index as usize + keyoffset) as *mut *mut u8),
                keysize,
                keyoffset,
                mode,
            )
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
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a as *mut c_void
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
        let blocksize_shift = (*a).block >> 1;
        let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_shift as usize);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            memmove(
                (*sb).storage.as_mut_ptr() as *mut c_void,
                str as *const c_void,
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
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, str as *const c_void, len);
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
        free(x as *mut c_void);
        x = y;
    }
    ptr::write_bytes(a, 0, 1);
}

// ============================================================
// stbds_unit_tests (stub)
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {}

// ============================================================
// hm_geti - the exported test function
// ============================================================
//
// The C code uses stb_ds macros with struct { int key; int value; }.
// We replicate the macro expansions directly.

// ============================================================
// strkey
// ============================================================

static mut STRKEY_BUFFER: [u8; 256] = [0u8; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut u8 {
    let s = format!("test_{}\0", n);
    let bytes = s.as_bytes();
    ptr::copy_nonoverlapping(bytes.as_ptr(), STRKEY_BUFFER.as_mut_ptr(), bytes.len());
    STRKEY_BUFFER.as_mut_ptr()
}

#[repr(C)]
#[derive(Copy, Clone)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

// Helper: size of IntMapEntry
const INTMAP_ELEM: usize = std::mem::size_of::<IntMapEntry>();

/// Equivalent of hmgeti(intmap, k): look up key, return index or -1
unsafe fn hm_geti_hmgeti(intmap: &mut *mut IntMapEntry, k: c_int) -> isize {
    let key_val = k;
    let result = stbds_hmget_key(
        *intmap as *mut c_void,
        INTMAP_ELEM,
        &key_val as *const c_int as *mut c_void,
        std::mem::size_of::<c_int>(),
        STBDS_HM_BINARY,
    );
    *intmap = result as *mut IntMapEntry;
    let raw = hash_to_arr(*intmap as *mut u8, INTMAP_ELEM);
    *stbds_temp(raw)
}

/// Equivalent of hmgeti_ts(intmap, k, temp)
unsafe fn hm_geti_hmgeti_ts(intmap: &mut *mut IntMapEntry, k: c_int, temp: &mut isize) -> isize {
    let key_val = k;
    let result = stbds_hmget_key_ts(
        *intmap as *mut c_void,
        INTMAP_ELEM,
        &key_val as *const c_int as *mut c_void,
        std::mem::size_of::<c_int>(),
        temp,
        STBDS_HM_BINARY,
    );
    *intmap = result as *mut IntMapEntry;
    *temp
}

/// Equivalent of hmget(intmap, k): look up key, return value
unsafe fn hm_geti_hmget(intmap: &mut *mut IntMapEntry, k: c_int) -> c_int {
    let idx = hm_geti_hmgeti(intmap, k);
    // hmget returns hmgetp(t,k)->value
    // hmgetp returns &(t)[stbds_temp((t)-1)]
    let raw = hash_to_arr(*intmap as *mut u8, INTMAP_ELEM);
    let t = *stbds_temp(raw);
    (*intmap.offset(t)).value
}

/// Equivalent of hmget_ts(intmap, k, temp)
unsafe fn hm_geti_hmget_ts(intmap: &mut *mut IntMapEntry, k: c_int, temp: &mut isize) -> c_int {
    let _idx = hm_geti_hmgeti_ts(intmap, k, temp);
    (*intmap.offset(*temp)).value
}

/// Equivalent of hmput(intmap, k, v)
unsafe fn hm_geti_hmput(intmap: &mut *mut IntMapEntry, k: c_int, v: c_int) {
    let key_val = k;
    let result = stbds_hmput_key(
        *intmap as *mut c_void,
        INTMAP_ELEM,
        &key_val as *const c_int as *mut c_void,
        std::mem::size_of::<c_int>(),
        STBDS_HM_BINARY,
    );
    *intmap = result as *mut IntMapEntry;
    let raw = hash_to_arr(*intmap as *mut u8, INTMAP_ELEM);
    let t = *stbds_temp(raw);
    (*intmap.offset(t)).key = k;
    (*intmap.offset(t)).value = v;
}

/// Equivalent of hmdefault(intmap, v)
unsafe fn hm_geti_hmdefault(intmap: &mut *mut IntMapEntry, v: c_int) {
    let result = stbds_hmput_default(*intmap as *mut c_void, INTMAP_ELEM);
    *intmap = result as *mut IntMapEntry;
    (*intmap.offset(-1)).value = v;
}

/// Equivalent of hmdel(intmap, k)
unsafe fn hm_geti_hmdel(intmap: &mut *mut IntMapEntry, k: c_int) -> isize {
    let key_val = k;
    let result = stbds_hmdel_key(
        *intmap as *mut c_void,
        INTMAP_ELEM,
        &key_val as *const c_int as *mut c_void,
        std::mem::size_of::<c_int>(),
        0, // STBDS_OFFSETOF((t),key) = 0 since key is first field
        STBDS_HM_BINARY,
    );
    *intmap = result as *mut IntMapEntry;
    if (*intmap).is_null() {
        0
    } else {
        let raw = hash_to_arr(*intmap as *mut u8, INTMAP_ELEM);
        *stbds_temp(raw)
    }
}

/// Equivalent of hmfree(intmap)
unsafe fn hm_geti_hmfree(intmap: &mut *mut IntMapEntry) {
    if !(*intmap).is_null() {
        stbds_hmfree_func((*intmap).offset(-1) as *mut c_void, INTMAP_ELEM);
    }
    *intmap = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    let mut intmap: *mut IntMapEntry = ptr::null_mut();
    let mut temp: isize = 0;

    let mut i: c_int;

    i = 1;
    assert!(hm_geti_hmgeti(&mut intmap, i) == -1);
    hm_geti_hmdefault(&mut intmap, -2);
    assert!(hm_geti_hmgeti(&mut intmap, i) == -1);
    assert!(hm_geti_hmget(&mut intmap, i) == -2);

    i = 0;
    while i < num {
        hm_geti_hmput(&mut intmap, i, i * 5);
        i += 2;
    }

    i = 0;
    while i < num {
        if i & 1 != 0 {
            assert!(hm_geti_hmget(&mut intmap, i) == -2);
        } else {
            assert!(hm_geti_hmget(&mut intmap, i) == i * 5);
        }
        if i & 1 != 0 {
            assert!(hm_geti_hmget_ts(&mut intmap, i, &mut temp) == -2);
        } else {
            assert!(hm_geti_hmget_ts(&mut intmap, i, &mut temp) == i * 5);
        }
        i += 1;
    }

    i = 0;
    while i < num {
        hm_geti_hmput(&mut intmap, i, i * 3);
        i += 2;
    }

    i = 0;
    while i < num {
        if i & 1 != 0 {
            assert!(hm_geti_hmget(&mut intmap, i) == -2);
        } else {
            assert!(hm_geti_hmget(&mut intmap, i) == i * 3);
        }
        i += 1;
    }

    i = 2;
    while i < num {
        hm_geti_hmdel(&mut intmap, i);
        i += 4;
    }

    i = 0;
    while i < num {
        if i & 3 != 0 {
            assert!(hm_geti_hmget(&mut intmap, i) == -2);
        } else {
            assert!(hm_geti_hmget(&mut intmap, i) == i * 3);
        }
        i += 1;
    }

    i = 0;
    while i < num {
        hm_geti_hmdel(&mut intmap, i);
        i += 1;
    }

    i = 0;
    while i < num {
        assert!(hm_geti_hmget(&mut intmap, i) == -2);
        i += 1;
    }

    hm_geti_hmfree(&mut intmap);

    i = 0;
    while i < num {
        hm_geti_hmput(&mut intmap, i, i * 3);
        i += 2;
    }

    hm_geti_hmfree(&mut intmap);
}
