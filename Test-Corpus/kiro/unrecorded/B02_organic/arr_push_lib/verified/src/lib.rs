#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    dead_code,
    private_interfaces,
    clippy::all
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
const STBDS_SIZE_T_BITS: usize = std::mem::size_of::<usize>() * 8;

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
    hash_table: *mut u8,
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
// Helpers
// ============================================================

unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

unsafe fn stbds_temp(t: *mut u8) -> &'static mut isize {
    &mut (*stbds_header(t)).temp
}

unsafe fn stbds_temp_key(t: *mut u8) -> &'static mut *mut u8 {
    // *(char **) stbds_header(t)->hash_table
    &mut *((*stbds_header(t)).hash_table as *mut *mut u8)
}

unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*stbds_header(a)).length as isize }
}

unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*stbds_header(a)).capacity }
}

unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn stbds_hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.sub(elemsize) }
}

fn stbds_arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.add(elemsize) }
}

fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// realloc/free wrappers using system allocator
unsafe fn c_realloc(p: *mut u8, size: usize) -> *mut u8 {
    if p.is_null() {
        if size == 0 { return ptr::null_mut(); }
        let layout = Layout::from_size_align_unchecked(size, 16);
        alloc::alloc(layout)
    } else {
        if size == 0 {
            // Can't know old size, but C realloc(p,0) frees. We'll just return null.
            return ptr::null_mut();
        }
        // We don't know the old layout size. Use libc realloc directly.
        libc::realloc(p as *mut libc::c_void, size) as *mut u8
    }
}

unsafe fn c_free(p: *mut u8) {
    if !p.is_null() {
        libc::free(p as *mut libc::c_void);
    }
}

unsafe fn c_malloc(size: usize) -> *mut u8 {
    libc::malloc(size) as *mut u8
}

// ============================================================
// Static seed
// ============================================================

static mut stbds_hash_seed: usize = 0x31415926;

// ============================================================
// Functions
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

unsafe fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut stbds_hash_index) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE - 1;
    let t = c_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;

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
    assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = stbds_hash_seed;

        let a: usize;
        let b: usize;
        let mut temp: usize;

        // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
        temp = 0x87b0b0fdusize ^ 2147001325usize;
        temp = temp.wrapping_shl(16).wrapping_shl(16).wrapping_shr(16).wrapping_shr(16);
        let mut a_val: usize = 0x27bb2ee6usize;
        a_val = a_val.wrapping_shl(16).wrapping_shl(16);
        a_val ^= temp ^ 2147001325usize;

        // stbds_load_32_or_64(b,temp, 715136305, 0, 0xb504f32d)
        temp = 0xb504f32dusize ^ 715136305usize;
        temp = temp.wrapping_shl(16).wrapping_shl(16).wrapping_shr(16).wrapping_shr(16);
        let mut b_val: usize = 0usize;
        b_val = b_val.wrapping_shl(16).wrapping_shl(16);
        b_val ^= temp ^ 715136305usize;

        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a_val).wrapping_add(b_val);
    }

    // Initialize buckets
    {
        for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
            let b = &mut *(*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                b.hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                b.index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }

    // Rehash from old table
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use(ob.index[j]) {
                    let hash = ob.hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
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

                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }

    t
}

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

unsafe fn stbds_siphash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575usize) ^ seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6dusize) ^ !seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261usize) ^ seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573usize) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1); v1 = stbds_rotate_left(v1, 13); v1 ^= v0; v0 = stbds_rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3); v3 = stbds_rotate_left(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = stbds_rotate_left(v1, 17); v1 ^= v2; v2 = stbds_rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3); v3 = stbds_rotate_left(v3, 21); v3 ^= v0;
        };
    }

    let sz = std::mem::size_of::<usize>();
    let mut i: usize = 0;
    while i + sz <= len {
        let dp = d.add(i);
        data = ((*dp.add(0) as i32)
            | ((*dp.add(1) as i32) << 8)
            | ((*dp.add(2) as i32) << 16)
            | ((*dp.add(3) as i32) << 24)) as usize;
        data |= (((*dp.add(4) as i32)
            | ((*dp.add(5) as i32) << 8)
            | ((*dp.add(6) as i32) << 16)
            | ((*dp.add(7) as i32) << 24)) as usize) << 16 << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        i += sz;
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    let dp = d.add(i);
    // C fallthrough switch
    if rem >= 7 { data |= (*dp.add(6) as usize) << 24 << 24; }
    if rem >= 6 { data |= (*dp.add(5) as usize) << 20 << 20; }
    if rem >= 5 { data |= (*dp.add(4) as usize) << 16 << 16; }
    if rem >= 4 { data |= ((*dp.add(3) as i32) << 24) as usize; }
    if rem >= 3 { data |= ((*dp.add(2) as i32) << 16) as usize; }
    if rem >= 2 { data |= ((*dp.add(1) as i32) << 8) as usize; }
    if rem >= 1 { data |= *dp.add(0) as usize; }

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

unsafe fn stbds_is_key_equal(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    keyoffset: usize, mode: c_int, i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_key_ptr = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        libc::strcmp(key as *const i8, stored_key_ptr as *const i8) == 0
    } else {
        libc::memcmp(
            key as *const libc::c_void,
            a.add(elemsize * i as usize + keyoffset) as *const libc::c_void,
            keysize,
        ) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut u8, elemsize: usize, addlen: usize, min_cap: usize,
) -> *mut u8 {
    let _temp = stbds_array_header {
        length: 0, capacity: 0, hash_table: ptr::null_mut(), temp: 0,
    };
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);
    let mut min_cap = min_cap;

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

    let old_ptr = if !a.is_null() {
        stbds_header(a) as *mut u8
    } else {
        ptr::null_mut()
    };
    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b_raw = c_realloc(old_ptr, alloc_size);
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
    c_free(stbds_header(a) as *mut u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() { return; }
    let ht = stbds_hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*stbds_header(a)).length {
                let p = *(a.add(elemsize * i) as *mut *mut u8);
                c_free(p);
            }
        }
        stbds_strreset(&mut (*ht).string);
    }
    c_free((*stbds_header(a)).hash_table);
    c_free(stbds_header(a) as *mut u8);
}

unsafe fn stbds_hm_find_slot(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    keyoffset: usize, mode: c_int,
) -> isize {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 { hash += 2; }

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

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
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
        pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    temp: *mut isize, mode: c_int,
) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        ptr::write_bytes(new_a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return stbds_arr_to_hash(new_a, elemsize);
    } else {
        let raw_a = stbds_hash_to_arr(a, elemsize);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int,
) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    *stbds_temp(stbds_hash_to_arr(p, elemsize)) = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    let mut a = a;
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() { stbds_hash_to_arr(a, elemsize) } else { ptr::null_mut() };
        let new_a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        ptr::write_bytes(new_a, 0, elemsize);
        a = stbds_arr_to_hash(new_a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str: *mut u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;
    let p = c_realloc(ptr::null_mut(), len);
    ptr::copy(str, p, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int,
) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a = a;

    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(new_a, 0, elemsize);
        (*stbds_header(new_a)).length += 1;
        a = stbds_arr_to_hash(new_a, elemsize);
    }

    let raw_a = a;
    let a_arr = stbds_hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a_arr)).hash_table as *mut stbds_hash_index;

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
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*stbds_header(a_arr)).hash_table = nt as *mut u8;
        table = nt;
    }

    // Now do the insert/find
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;

    if hash < 2 { hash += 2; }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    let found_pos: usize;

    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    *stbds_temp(a_arr) = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *stbds_temp_key(a_arr) = *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset) as *const *mut u8);
                    }
                    return stbds_arr_to_hash(a_arr, elemsize);
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
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
                    *stbds_temp(a_arr) = bucket.index[i];
                    return stbds_arr_to_hash(a_arr, elemsize);
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 {
                if bucket.index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    // found_empty_slot:
    let mut pos = found_pos;
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let mut a_arr = a_arr; // may be reassigned
    let i = stbds_arrlen(a_arr);
    if (i as usize + 1) > stbds_arrcap(a_arr) {
        a_arr = stbds_arrgrowf(a_arr, elemsize, 1, 0);
    }
    let raw_a = stbds_arr_to_hash(a_arr, elemsize);

    assert!((i as usize + 1) <= stbds_arrcap(a_arr));
    (*stbds_header(a_arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(a_arr) = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key);
            *(a_arr.add(elemsize * i as usize) as *mut *mut u8) = dup;
            *stbds_temp_key(a_arr) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(&mut (*table).string, key as *mut u8);
            *(a_arr.add(elemsize * i as usize) as *mut *mut u8) = s;
            *stbds_temp_key(a_arr) = s;
        }
        STBDS_SH_DEFAULT => {
            *(a_arr.add(elemsize * i as usize) as *mut *mut u8) = key;
            *stbds_temp_key(a_arr) = key;
        }
        _ => {
            ptr::copy_nonoverlapping(key, a_arr.add(elemsize * i as usize), keysize);
        }
    }

    stbds_arr_to_hash(a_arr, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut u8;
    stbds_arr_to_hash(a, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    keyoffset: usize, mode: c_int,
) -> *mut u8 {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
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
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 always true for usize
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *(a.add(elemsize * old_index as usize) as *mut *mut u8);
        c_free(p);
    }

    if old_index != final_index {
        ptr::copy(
            a.add(elemsize * final_index as usize),
            a.add(elemsize * old_index as usize),
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            let k = *(a.add(elemsize * old_index as usize + keyoffset) as *const *mut u8);
            stbds_hm_find_slot(a, elemsize, k, keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(a, elemsize, a.add(elemsize * old_index as usize + keyoffset), keysize, keyoffset, mode)
        };
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add((slot2 as usize) >> STBDS_BUCKET_SHIFT);
        let i2 = (slot2 as usize) & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        c_free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count, table) as *mut u8;
        c_free(table as *mut u8);
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;
    if len > (*a).remaining {
        let blocksize: usize;
        let block_val = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (block_val >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            ptr::copy(str, (*sb).storage.as_mut_ptr(), len);
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
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    ptr::copy(str, p, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        c_free(x as *mut u8);
        x = y;
    }
    ptr::write_bytes(a, 0, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {
    // empty in original for this build
}

static mut buffer: [u8; 256] = [0u8; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut u8 {
    libc::snprintf(buffer.as_mut_ptr() as *mut i8, 256, b"test_%d\0".as_ptr() as *const i8, n);
    buffer.as_mut_ptr()
}

// ============================================================
// Helper aliases used by arr_push (inline the macro logic)
// ============================================================

// stbds_hash_to_arr / stbds_arr_to_hash already defined above as fns

// ============================================================
// The public function
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();

    assert!(stbds_arrlen(arr as *mut u8) == 0);

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            // arrpush(arr, j) => stbds_arrmaybegrow then store
            let a = arr as *mut u8;
            let elemsize = std::mem::size_of::<c_int>();
            if a.is_null() || (*stbds_header(a)).length + 1 > (*stbds_header(a)).capacity {
                arr = stbds_arrgrowf(a, elemsize, 1, 0) as *mut c_int;
            }
            let hdr = stbds_header(arr as *mut u8);
            let idx = (*hdr).length;
            (*hdr).length += 1;
            *arr.add(idx) = j;

            j += 1;
        }
        // arrfree(arr)
        if !arr.is_null() {
            c_free(stbds_header(arr as *mut u8) as *mut u8);
        }
        arr = ptr::null_mut();

        i += 50;
    }
}
