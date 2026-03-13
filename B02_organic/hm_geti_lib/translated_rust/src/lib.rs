#![allow(
    non_camel_case_types,
    non_snake_case,
    unused_variables,
    unused_assignments,
    unused_mut,
    clippy::all
)]

use std::alloc::{self, Layout};
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

const STBDS_HM_BINARY: i32 = 0;
const STBDS_HM_STRING: i32 = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: usize = std::mem::size_of::<usize>() * 8;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

static mut STBDS_HASH_SEED: usize = 0x31415926;

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
// Helpers: malloc/realloc/free wrappers
// ============================================================
unsafe fn c_realloc(p: *mut u8, size: usize) -> *mut u8 {
    if p.is_null() {
        c_malloc(size)
    } else {
        // We stored the layout at (ptr - 16)
        let meta = p.sub(16) as *const usize;
        let old_size = *meta;
        let old_align = *meta.add(1);
        let old_layout = Layout::from_size_align_unchecked(old_size + 16, old_align);
        let raw = p.sub(16);
        let new_raw = alloc::realloc(raw, old_layout, size + 16);
        if new_raw.is_null() {
            return ptr::null_mut();
        }
        let meta = new_raw as *mut usize;
        *meta = size;
        // align stays the same
        new_raw.add(16)
    }
}

unsafe fn c_malloc(size: usize) -> *mut u8 {
    let align = 16usize;
    let total = size + 16;
    let layout = Layout::from_size_align_unchecked(total, align);
    let raw = alloc::alloc(layout);
    if raw.is_null() {
        return ptr::null_mut();
    }
    let meta = raw as *mut usize;
    *meta = size;
    *meta.add(1) = align;
    raw.add(16)
}

unsafe fn c_free(p: *mut u8) {
    if p.is_null() {
        return;
    }
    let meta = p.sub(16) as *const usize;
    let size = *meta;
    let align = *meta.add(1);
    let layout = Layout::from_size_align_unchecked(size + 16, align);
    alloc::dealloc(p.sub(16), layout);
}

// ============================================================
// Header access helpers
// ============================================================
#[inline(always)]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).sub(1)
}

#[inline(always)]
unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline(always)]
unsafe fn stbds_arrlenu(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length
    }
}

#[inline(always)]
unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline(always)]
unsafe fn stbds_temp(a: *mut u8) -> &'static mut isize {
    &mut (*stbds_header(a)).temp
}

#[inline(always)]
unsafe fn stbds_temp_key(a: *mut u8) -> &'static mut *mut u8 {
    // *(char **) stbds_header(t)->hash_table
    &mut *((*stbds_header(a)).hash_table as *mut *mut u8)
}

// ============================================================
// HASH_TO_ARR / ARR_TO_HASH
// ============================================================
#[inline(always)]
unsafe fn hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.sub(elemsize)
}

#[inline(always)]
unsafe fn arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.add(elemsize)
}

// ============================================================
// stbds_arrgrowf
// ============================================================
unsafe fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap: usize) -> *mut u8 {
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    let double_cap = stbds_arrcap(a).wrapping_mul(2);
    if min_cap < double_cap {
        min_cap = double_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(std::mem::size_of::<stbds_array_header>());
    let b = if a.is_null() {
        c_realloc(ptr::null_mut(), alloc_size)
    } else {
        c_realloc(stbds_header(a) as *mut u8, alloc_size)
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
unsafe fn stbds_arrfreef(a: *mut u8) {
    c_free(stbds_header(a) as *mut u8);
}

// ============================================================
// Alignment helper
// ============================================================
#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a).wrapping_sub(1)) & !a.wrapping_sub(1)
}

// ============================================================
// stbds_probe_position
// ============================================================
#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
}

// ============================================================
// stbds_log2
// ============================================================
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

// ============================================================
// Rotate helpers
// ============================================================
#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// ============================================================
// stbds_hash_string
// ============================================================
unsafe fn stbds_hash_string_impl(str: *const u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str;
    while *p != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*p as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// ============================================================
// stbds_siphash_bytes
// ============================================================
unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = rotate_left(v1, 13);
            v1 ^= v0;
            v0 = rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3);
            v3 = rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotate_left(v1, 17);
            v1 ^= v2;
            v2 = rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3);
            v3 = rotate_left(v3, 21);
            v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dp = d.add(i);
        data = *dp.add(0) as usize
            | ((*dp.add(1) as usize) << 8)
            | ((*dp.add(2) as usize) << 16)
            | ((*dp.add(3) as usize) << 24);
        data |= ((*dp.add(4) as usize
            | ((*dp.add(5) as usize) << 8)
            | ((*dp.add(6) as usize) << 16)
            | ((*dp.add(7) as usize) << 24))
            << 16)
            << 16;

        v3 ^= data;
        for _j in 0..2 {
            sipround!();
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    let dp = d.add(i);
    // C fallthrough switch
    if rem >= 7 {
        data |= ((*dp.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*dp.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*dp.add(4) as usize) << 16) << 16;
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
    for _j in 0..2 {
        sipround!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _j in 0..4 {
        sipround!();
    }

    v0 ^ v1 ^ v2 ^ v3
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
    mode: i32,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let str_ptr = *(a.add(elemsize.wrapping_mul(i as usize).wrapping_add(keyoffset)) as *const *const u8);
        libc_strcmp(key, str_ptr) == 0
    } else {
        libc_memcmp(
            key,
            a.add(elemsize.wrapping_mul(i as usize).wrapping_add(keyoffset)),
            keysize,
        ) == 0
    }
}

unsafe fn libc_strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return (ca as i32) - (cb as i32);
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

unsafe fn libc_memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return (ca as i32) - (cb as i32);
        }
    }
    0
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
    let t = c_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    (*t).storage = align_fwd(
        (t.add(1)) as usize,
        STBDS_CACHE_LINE_SIZE,
    ) as *mut stbds_hash_bucket;
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
    assert!(
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
    );

    if !ot.is_null() {
        (*t).string = ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64 for a
        let mut temp: usize;
        let a: usize;
        temp = 0x87b0b0fd ^ 2147001325u32 as usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        let mut a_val = 0x27bb2ee6usize;
        a_val <<= 16;
        a_val <<= 16;
        a_val ^= temp ^ 2147001325u32 as usize;
        a = a_val;

        // stbds_load_32_or_64 for b
        let b: usize;
        temp = 0xb504f32d ^ 715136305u32 as usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        let mut b_val = 0usize;
        b_val <<= 16;
        b_val <<= 16;
        b_val ^= temp ^ 715136305u32 as usize;
        b = b_val;

        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
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

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
            }
        }
    }

    t
}

// ============================================================
// stbds_hmfree_func
// ============================================================
unsafe fn stbds_hmfree_func_impl(a: *mut u8, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let ht = stbds_hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let str_ptr = *(a.add(elemsize * i) as *mut *mut u8);
                c_free(str_ptr);
            }
        }
        stbds_strreset_impl(&mut (*ht).string);
    }
    c_free((*stbds_header(a)).hash_table as *mut u8);
    c_free(stbds_header(a) as *mut u8);
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
    mode: i32,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string_impl(key, (*table).seed)
    } else {
        stbds_siphash_bytes(key, keysize, (*table).seed)
    };
    let hash = if hash < 2 { hash + 2 } else { hash };
    let mut step = STBDS_BUCKET_LENGTH;
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

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count.wrapping_sub(1);
    }
}

// ============================================================
// stbds_hmget_key_ts
// ============================================================
unsafe fn stbds_hmget_key_ts_impl(
    a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    temp: *mut isize,
    mode: i32,
) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let arr = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(arr)).length += 1;
        ptr::write_bytes(arr, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(arr, elemsize);
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = stbds_hash_table(raw_a);
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
unsafe fn stbds_hmget_key_impl(
    a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    mode: i32,
) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts_impl(a, elemsize, key, keysize, &mut temp, mode);
    *stbds_temp(hash_to_arr(p, elemsize)) = temp;
    p
}

// ============================================================
// stbds_hmput_default
// ============================================================
unsafe fn stbds_hmput_default_impl(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        let arr = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(arr)).length += 1;
        ptr::write_bytes(arr, 0, elemsize);
        return arr_to_hash(arr, elemsize);
    }
    a
}

// ============================================================
// stbds_strdup
// ============================================================
unsafe fn stbds_strdup_impl(str: *const u8) -> *mut u8 {
    let mut len = 0usize;
    while *str.add(len) != 0 {
        len += 1;
    }
    len += 1;
    let p = c_realloc(ptr::null_mut(), len);
    ptr::copy(str, p, len);
    p
}

// ============================================================
// stbds_hmput_key
// ============================================================
unsafe fn stbds_hmput_key_impl(
    a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    mode: i32,
) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a = a;

    if a.is_null() {
        let arr = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(arr, 0, elemsize);
        (*stbds_header(arr)).length += 1;
        a = arr_to_hash(arr, elemsize);
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
            c_free(table as *mut u8);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*stbds_header(arr)).hash_table = nt as *mut std::ffi::c_void;
        table = nt;
    }

    {
        let hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string_impl(key, (*table).seed)
        } else {
            stbds_siphash_bytes(key, keysize, (*table).seed)
        };
        let hash = if hash < 2 { hash + 2 } else { hash };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut tombstone: isize = -1;
        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        loop {
            let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i],
                    ) {
                        *stbds_temp(arr) = bucket.index[i];
                        if mode >= STBDS_HM_STRING {
                            *stbds_temp_key(arr) = *(raw_a
                                .add(elemsize.wrapping_mul(bucket.index[i] as usize).wrapping_add(keyoffset))
                                as *const *mut u8);
                        }
                        return arr_to_hash(arr, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    // goto found_empty_slot
                    return stbds_hmput_key_found_empty(
                        arr, raw_a, elemsize, key, keysize, keyoffset, mode, table, hash, pos,
                        tombstone,
                    );
                } else if tombstone < 0 {
                    if bucket.index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            let limit = start;
            for i in 0..limit {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i],
                    ) {
                        *stbds_temp(arr) = bucket.index[i];
                        return arr_to_hash(arr, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    return stbds_hmput_key_found_empty(
                        arr, raw_a, elemsize, key, keysize, keyoffset, mode, table, hash, pos,
                        tombstone,
                    );
                } else if tombstone < 0 {
                    if bucket.index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count.wrapping_sub(1);
        }
    }
}

unsafe fn stbds_hmput_key_found_empty(
    mut arr: *mut u8,
    mut raw_a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    keyoffset: usize,
    mode: i32,
    table: *mut stbds_hash_index,
    hash: usize,
    mut pos: usize,
    tombstone: isize,
) -> *mut u8 {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr) as isize;
    if (i as usize) + 1 > stbds_arrcap(arr) {
        arr = stbds_arrgrowf(arr, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(arr, elemsize);

    assert!((i as usize) + 1 <= stbds_arrcap(arr));
    (*stbds_header(arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(arr) = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup_impl(key);
            *(arr.add(elemsize * (i as usize)) as *mut *mut u8) = dup;
            *stbds_temp_key(arr) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc_impl(&mut (*table).string, key);
            *(arr.add(elemsize * (i as usize)) as *mut *mut u8) = s;
            *stbds_temp_key(arr) = s;
        }
        STBDS_SH_DEFAULT => {
            *(arr.add(elemsize * (i as usize)) as *mut *mut u8) = key as *mut u8;
            *stbds_temp_key(arr) = key as *mut u8;
        }
        _ => {
            ptr::copy_nonoverlapping(key, arr.add(elemsize * (i as usize)), keysize);
        }
    }

    arr_to_hash(arr, elemsize)
}

// ============================================================
// stbds_shmode_func
// ============================================================
unsafe fn stbds_shmode_func_impl(elemsize: usize, mode: i32) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut std::ffi::c_void;
    arr_to_hash(a, elemsize)
}

// ============================================================
// stbds_hmdel_key
// ============================================================
unsafe fn stbds_hmdel_key_impl(
    a: *mut u8,
    elemsize: usize,
    key: *const u8,
    keysize: usize,
    keyoffset: usize,
    mode: i32,
) -> *mut u8 {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    *stbds_temp(raw_a) = 0;
    if table.is_null() {
        return a;
    }

    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let b = &mut *(*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = stbds_arrlen(raw_a) as isize - 1 - 1;
    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 always true for usize
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let str_ptr = *(a.add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut u8);
        c_free(str_ptr);
    }

    if old_index != final_index {
        ptr::copy(
            a.add(elemsize.wrapping_mul(final_index as usize)),
            a.add(elemsize.wrapping_mul(old_index as usize)),
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            let key_ptr = *(a.add(
                elemsize.wrapping_mul(old_index as usize).wrapping_add(keyoffset),
            ) as *const *const u8);
            stbds_hm_find_slot(a, elemsize, key_ptr, keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(
                a,
                elemsize,
                a.add(elemsize.wrapping_mul(old_index as usize).wrapping_add(keyoffset)),
                keysize,
                keyoffset,
                mode,
            )
        };
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add((slot2 as usize) >> STBDS_BUCKET_SHIFT);
        let i2 = (slot2 as usize) & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut std::ffi::c_void;
        c_free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut std::ffi::c_void;
        c_free(table as *mut u8);
    }

    a
}

// ============================================================
// stbds_stralloc
// ============================================================
unsafe fn stbds_stralloc_impl(a: &mut stbds_string_arena, str: *const u8) -> *mut u8 {
    let mut len = 0usize;
    while *str.add(len) != 0 {
        len += 1;
    }
    len += 1;

    if len > a.remaining {
        let blocksize_shift = a.block >> 1;
        let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_shift as usize);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            a.block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            ptr::copy(str, (*sb).storage.as_mut_ptr(), len);
            if !a.storage.is_null() {
                (*sb).next = (*a.storage).next;
                (*a.storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                a.storage = sb;
                a.remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = a.storage;
            a.storage = sb;
            a.remaining = blocksize;
        }
    }

    assert!(len <= a.remaining);
    let p = (*a.storage).storage.as_mut_ptr().add(a.remaining - len);
    a.remaining -= len;
    ptr::copy(str, p, len);
    p
}

// ============================================================
// stbds_strreset
// ============================================================
unsafe fn stbds_strreset_impl(a: &mut stbds_string_arena) {
    let mut x = a.storage;
    while !x.is_null() {
        let y = (*x).next;
        c_free(x as *mut u8);
        x = y;
    }
    ptr::write_bytes(a as *mut stbds_string_arena, 0, 1);
}

// ============================================================
// Public C API: hm_geti
// ============================================================

// Helper struct matching C: struct { int key; int value; }
#[repr(C)]
#[derive(Copy, Clone)]
struct IntMapEntry {
    key: i32,
    value: i32,
}

// Macro-like helpers for the hashmap operations on IntMapEntry
// These replicate the C macro expansions for the specific IntMapEntry type.

const INTMAP_ELEMSIZE: usize = std::mem::size_of::<IntMapEntry>();
const INTMAP_KEYSIZE: usize = std::mem::size_of::<i32>();

unsafe fn intmap_hmgeti(intmap: &mut *mut u8, k: i32) -> isize {
    let key_bytes = k.to_ne_bytes();
    *intmap = stbds_hmget_key_impl(
        *intmap,
        INTMAP_ELEMSIZE,
        key_bytes.as_ptr(),
        INTMAP_KEYSIZE,
        STBDS_HM_BINARY,
    );
    let arr = hash_to_arr(*intmap, INTMAP_ELEMSIZE);
    (*stbds_header(arr)).temp
}

unsafe fn intmap_hmget(intmap: &mut *mut u8, k: i32) -> i32 {
    let key_bytes = k.to_ne_bytes();
    *intmap = stbds_hmget_key_impl(
        *intmap,
        INTMAP_ELEMSIZE,
        key_bytes.as_ptr(),
        INTMAP_KEYSIZE,
        STBDS_HM_BINARY,
    );
    let arr = hash_to_arr(*intmap, INTMAP_ELEMSIZE);
    let idx = (*stbds_header(arr)).temp;
    let entry = &*((*intmap).add(INTMAP_ELEMSIZE.wrapping_mul(idx as usize)) as *const IntMapEntry);
    entry.value
}

unsafe fn intmap_hmget_ts(intmap: &mut *mut u8, k: i32, temp: &mut isize) -> i32 {
    let key_bytes = k.to_ne_bytes();
    *intmap = stbds_hmget_key_ts_impl(
        *intmap,
        INTMAP_ELEMSIZE,
        key_bytes.as_ptr(),
        INTMAP_KEYSIZE,
        temp,
        STBDS_HM_BINARY,
    );
    let entry = &*((*intmap).add(INTMAP_ELEMSIZE.wrapping_mul(*temp as usize)) as *const IntMapEntry);
    entry.value
}

unsafe fn intmap_hmput(intmap: &mut *mut u8, k: i32, v: i32) {
    let key_bytes = k.to_ne_bytes();
    *intmap = stbds_hmput_key_impl(
        *intmap,
        INTMAP_ELEMSIZE,
        key_bytes.as_ptr(),
        INTMAP_KEYSIZE,
        STBDS_HM_BINARY,
    );
    let arr = hash_to_arr(*intmap, INTMAP_ELEMSIZE);
    let idx = (*stbds_header(arr)).temp;
    let entry = &mut *((*intmap).add(INTMAP_ELEMSIZE.wrapping_mul(idx as usize)) as *mut IntMapEntry);
    entry.key = k;
    entry.value = v;
}

unsafe fn intmap_hmdefault(intmap: &mut *mut u8, v: i32) {
    *intmap = stbds_hmput_default_impl(*intmap, INTMAP_ELEMSIZE);
    // (t)[-1].value = v
    let entry = &mut *((*intmap).sub(INTMAP_ELEMSIZE) as *mut IntMapEntry);
    entry.value = v;
}

unsafe fn intmap_hmdel(intmap: &mut *mut u8, k: i32) {
    let key_bytes = k.to_ne_bytes();
    *intmap = stbds_hmdel_key_impl(
        *intmap,
        INTMAP_ELEMSIZE,
        key_bytes.as_ptr(),
        INTMAP_KEYSIZE,
        0, // STBDS_OFFSETOF((t),key) = 0 since key is first field
        STBDS_HM_BINARY,
    );
}

unsafe fn intmap_hmfree(intmap: &mut *mut u8) {
    if !(*intmap).is_null() {
        stbds_hmfree_func_impl((*intmap).sub(INTMAP_ELEMSIZE), INTMAP_ELEMSIZE);
    }
    *intmap = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: std::ffi::c_int) {
    unsafe {
        let mut intmap: *mut u8 = ptr::null_mut();
        let mut temp: isize = 0;

        let mut i: i32;
        i = 1;
        assert!(intmap_hmgeti(&mut intmap, i) == -1);
        intmap_hmdefault(&mut intmap, -2);
        assert!(intmap_hmgeti(&mut intmap, i) == -1);
        assert!(intmap_hmget(&mut intmap, i) == -2);
        i = 0;
        while i < num {
            intmap_hmput(&mut intmap, i, i * 5);
            i += 2;
        }
        i = 0;
        while i < num {
            if i & 1 != 0 {
                assert!(intmap_hmget(&mut intmap, i) == -2);
            } else {
                assert!(intmap_hmget(&mut intmap, i) == i * 5);
            }
            if i & 1 != 0 {
                assert!(intmap_hmget_ts(&mut intmap, i, &mut temp) == -2);
            } else {
                assert!(intmap_hmget_ts(&mut intmap, i, &mut temp) == i * 5);
            }
            i += 1;
        }
        i = 0;
        while i < num {
            intmap_hmput(&mut intmap, i, i * 3);
            i += 2;
        }
        i = 0;
        while i < num {
            if i & 1 != 0 {
                assert!(intmap_hmget(&mut intmap, i) == -2);
            } else {
                assert!(intmap_hmget(&mut intmap, i) == i * 3);
            }
            i += 1;
        }
        i = 2;
        while i < num {
            intmap_hmdel(&mut intmap, i);
            i += 4;
        }
        i = 0;
        while i < num {
            if i & 3 != 0 {
                assert!(intmap_hmget(&mut intmap, i) == -2);
            } else {
                assert!(intmap_hmget(&mut intmap, i) == i * 3);
            }
            i += 1;
        }
        i = 0;
        while i < num {
            intmap_hmdel(&mut intmap, i);
            i += 1;
        }
        i = 0;
        while i < num {
            assert!(intmap_hmget(&mut intmap, i) == -2);
            i += 1;
        }
        intmap_hmfree(&mut intmap);
        i = 0;
        while i < num {
            intmap_hmput(&mut intmap, i, i * 3);
            i += 2;
        }
        intmap_hmfree(&mut intmap);
    }
}
