#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::{c_char, c_int, c_void};
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

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() * 8) as u32;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// ============================================================
// Structs (C-compatible layout)
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
struct stbds_hash_index {
    temp_key: *mut c_char,
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

#[repr(C)]
pub struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

// ============================================================
// Global mutable state
// ============================================================
static mut stbds_hash_seed: usize = 0x31415926;

// ============================================================
// Helper: pointer to header (header is right before the array data)
// ============================================================
#[inline(always)]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline(always)]
unsafe fn stbds_arrlenu(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length
    }
}

#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// STBDS_HASH_TO_ARR: go from "hash pointer" (element[0] is default) back to raw array
#[inline(always)]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

/// STBDS_ARR_TO_HASH: go from raw array to "hash pointer"
#[inline(always)]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
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

    let old_ptr = if !a.is_null() {
        stbds_header(a) as *mut u8
    } else {
        ptr::null_mut()
    };
    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b_raw = libc::realloc(old_ptr as *mut c_void, alloc_size);
    let b = (b_raw as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;

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
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    libc::free(stbds_header(a) as *mut c_void);
}

// ============================================================
// stbds_rand_seed
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

// ============================================================
// stbds_hash_string
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut p = str as *const u8;
    while *p != 0 {
        hash = hash.rotate_left(9).wrapping_add(*p as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ hash.rotate_right(11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= hash.rotate_right(22);
    hash.wrapping_add(seed)
}

// ============================================================
// stbds_siphash_bytes (static in C, but needed by stbds_hash_bytes)
// ============================================================
unsafe fn stbds_siphash_bytes(p: *const c_void, len: usize, seed: usize) -> usize {
    let d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = (((0x736f6d65_usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = (((0x646f7261_usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = (((0x6c796765_usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = (((0x74656462_usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left(STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left(STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
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

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    let dp = d.add(i);
    // fallthrough switch
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

// ============================================================
// stbds_hash_bytes
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ============================================================
// Probe position / log2
// ============================================================
#[inline(always)]
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
    let t = libc::realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
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
        std::ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = stbds_hash_seed;
        let a: usize;
        let b: usize;
        let mut temp: usize;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
        temp = 0x87b0b0fd_usize ^ 2147001325_usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        a = {
            let mut v = 0x27bb2ee6_usize;
            v <<= 16;
            v <<= 16;
            v ^= temp ^ 2147001325_usize;
            v
        };
        // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d)
        temp = 0xb504f32d_usize ^ 715136305_usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        b = {
            let mut v = 0_usize;
            v <<= 16;
            v <<= 16;
            v ^= temp ^ 715136305_usize;
            v
        };
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
        let num_old_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..num_old_buckets {
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
    a: *mut c_void,
    elemsize: usize,
    key: *const c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_key_ptr =
            *((a as *const u8).offset(elemsize as isize * i + keyoffset as isize)
                as *const *const c_char);
        libc::strcmp(key as *const c_char, stored_key_ptr) == 0
    } else {
        libc::memcmp(
            key,
            (a as *const u8).offset(elemsize as isize * i + keyoffset as isize) as *const c_void,
            keysize,
        ) == 0
    }
}

// ============================================================
// stbds_hmfree_func
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = stbds_hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let key_ptr_loc =
                    (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                libc::free(*key_ptr_loc as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    libc::free((*stbds_header(a)).hash_table);
    libc::free(stbds_header(a) as *mut c_void);
}

// ============================================================
// stbds_hm_find_slot
// ============================================================
unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *const c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
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
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        std::ptr::write_bytes(new_a as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(new_a, elemsize);
    } else {
        let raw_a = hash_to_arr(a, elemsize);
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
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*stbds_header(hash_to_arr(p, elemsize))).temp = temp;
    p
}

// ============================================================
// stbds_hmput_default
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        let new_a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        std::ptr::write_bytes(new_a as *mut u8, 0, elemsize);
        return arr_to_hash(new_a, elemsize);
    }
    a
}

// ============================================================
// stbds_strdup (static helper)
// ============================================================
unsafe fn stbds_strdup(str: *const c_char) -> *mut c_char {
    let len = libc::strlen(str) + 1;
    let p = libc::realloc(ptr::null_mut(), len) as *mut c_char;
    libc::memmove(p as *mut c_void, str as *const c_void, len);
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
    let mut a = a;
    let raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        std::ptr::write_bytes(new_a as *mut u8, 0, elemsize);
        (*stbds_header(new_a)).length += 1;
        a = arr_to_hash(new_a, elemsize);
    }

    let raw_a_save = a;
    let arr = hash_to_arr(a, elemsize);

    table = (*stbds_header(arr)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            libc::free(table as *mut c_void);
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

    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
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
            let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a_save,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        bucket.index[i],
                    ) {
                        (*stbds_header(arr)).temp = bucket.index[i];
                        if mode >= STBDS_HM_STRING {
                            let key_loc = (raw_a_save as *const u8)
                                .offset(elemsize as isize * bucket.index[i] + keyoffset as isize)
                                as *const *mut c_char;
                            *((*stbds_header(arr)).hash_table as *mut *mut c_char) = *key_loc;
                        }
                        return arr_to_hash(arr, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    // goto found_empty_slot
                    return stbds_hmput_key_found_empty(
                        arr, elemsize, key, keysize, keyoffset, mode, table, raw_a_save, pos,
                        tombstone, hash,
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
                        raw_a_save,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        bucket.index[i],
                    ) {
                        (*stbds_header(arr)).temp = bucket.index[i];
                        return arr_to_hash(arr, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    return stbds_hmput_key_found_empty(
                        arr, elemsize, key, keysize, keyoffset, mode, table, raw_a_save, pos,
                        tombstone, hash,
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
}

/// Helper for the "found_empty_slot" label in stbds_hmput_key
unsafe fn stbds_hmput_key_found_empty(
    mut arr: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    table: *mut stbds_hash_index,
    mut raw_a: *mut c_void,
    mut pos: usize,
    tombstone: isize,
    hash: usize,
) -> *mut c_void {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr) as isize;
    if (i as usize + 1) > stbds_arrcap(arr) {
        arr = stbds_arrgrowf(arr, elemsize, 1, 0);
        raw_a = arr_to_hash(arr, elemsize);
    }

    assert!((i as usize + 1) <= stbds_arrcap(arr));
    (*stbds_header(arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*stbds_header(arr)).temp = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key as *const c_char);
            let key_loc =
                (arr as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
            *key_loc = dup;
            *((*stbds_header(arr)).hash_table as *mut *mut c_char) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(
                &mut (*table).string,
                key as *mut c_char,
            );
            let key_loc =
                (arr as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
            *key_loc = s;
            *((*stbds_header(arr)).hash_table as *mut *mut c_char) = s;
        }
        STBDS_SH_DEFAULT => {
            let key_loc =
                (arr as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
            *key_loc = key as *mut c_char;
            *((*stbds_header(arr)).hash_table as *mut *mut c_char) = key as *mut c_char;
        }
        _ => {
            libc::memcpy(
                (arr as *mut u8).add(elemsize * i as usize) as *mut c_void,
                key,
                keysize,
            );
        }
    }

    arr_to_hash(arr, elemsize)
}

// ============================================================
// stbds_shmode_func
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    std::ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut c_void;
    arr_to_hash(a, elemsize)
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
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    (*stbds_header(raw_a)).temp = 0;

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
    (*stbds_header(raw_a)).temp = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 (always true for usize, but matches C assert)
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let key_ptr_loc =
            (a as *mut u8).offset(elemsize as isize * old_index) as *mut *mut c_char;
        libc::free(*key_ptr_loc as *mut c_void);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).offset(elemsize as isize * old_index) as *mut c_void,
            (a as *mut u8).offset(elemsize as isize * final_index) as *mut c_void,
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            let key_ptr = *((a as *const u8).offset(elemsize as isize * old_index + keyoffset as isize)
                as *const *const c_void);
            stbds_hm_find_slot(a, elemsize, key_ptr, keysize, keyoffset, mode)
        } else {
            let key_ptr = (a as *const u8).offset(elemsize as isize * old_index + keyoffset as isize)
                as *const c_void;
            stbds_hm_find_slot(a, elemsize, key_ptr, keysize, keyoffset, mode)
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
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        libc::free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        libc::free(table as *mut c_void);
    }

    a
}

// ============================================================
// stbds_stralloc
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut c_char,
) -> *mut c_char {
    let len = libc::strlen(str) + 1;
    if len > (*a).remaining {
        let blocksize_exp = (*a).block;
        let mut blocksize =
            (STBDS_STRING_ARENA_BLOCKSIZE_MIN) << ((blocksize_exp as usize) >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // Allocate oversized block
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = libc::realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            libc::memmove(
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
            let sb = libc::realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
        .add((*a).remaining - len) as *mut c_char;
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, str as *const c_void, len);
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
        libc::free(x as *mut c_void);
        x = y;
    }
    std::ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
}

// ============================================================
// stbds_unit_tests (declared extern in C but not defined in this file)
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {
    // stub — declared but not defined in the C source's lib.c
}

// ============================================================
// sh_geti — the main public function
// ============================================================
// In C, sh_geti uses macros that operate on:
//   struct { char *key; int value; } *strmap
// elemsize = size_of::<StrMapEntry>() = 16 on 64-bit (8 ptr + 4 int + 4 pad)
// keysize = size_of::<*mut c_char>() = 8
// keyoffset = 0 (key is first field)

#[repr(C)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

static mut BUFFER: [u8; 256] = [0u8; 256];

unsafe fn strkey(n: c_int) -> *mut c_char {
    libc::sprintf(
        BUFFER.as_mut_ptr() as *mut c_char,
        b"test_%d\0".as_ptr() as *const c_char,
        n,
    );
    BUFFER.as_mut_ptr() as *mut c_char
}

/// Macro helpers inlined for sh_geti
const ELEMSIZE: usize = std::mem::size_of::<StrMapEntry>();
const KEYSIZE: usize = std::mem::size_of::<*mut c_char>();

/// shgeti(strmap, k) => hmget_key_wrapper then read temp
unsafe fn sh_geti_shgeti(strmap: *mut StrMapEntry, k: *const c_char) -> (*mut StrMapEntry, isize) {
    let result = stbds_hmget_key(
        strmap as *mut c_void,
        ELEMSIZE,
        k as *mut c_void,
        KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut StrMapEntry;
    let raw = hash_to_arr(result as *mut c_void, ELEMSIZE);
    let temp = (*stbds_header(raw)).temp;
    (result, temp)
}

/// shput(strmap, k, v)
unsafe fn sh_geti_shput(
    strmap: *mut StrMapEntry,
    k: *const c_char,
    v: c_int,
) -> *mut StrMapEntry {
    let result = stbds_hmput_key(
        strmap as *mut c_void,
        ELEMSIZE,
        k as *mut c_void,
        KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut StrMapEntry;
    let raw = hash_to_arr(result as *mut c_void, ELEMSIZE);
    let temp = (*stbds_header(raw)).temp;
    (*result.offset(temp)).value = v;
    result
}

/// shget(strmap, k) => shgetp(strmap,k)->value
unsafe fn sh_geti_shget(strmap: *mut StrMapEntry, k: *const c_char) -> (*mut StrMapEntry, c_int) {
    let (result, temp) = sh_geti_shgeti(strmap, k);
    let val = (*result.offset(temp)).value;
    (result, val)
}

/// shdel(strmap, k)
unsafe fn sh_geti_shdel(strmap: *mut StrMapEntry, k: *const c_char) -> (*mut StrMapEntry, isize) {
    let result = stbds_hmdel_key(
        strmap as *mut c_void,
        ELEMSIZE,
        k as *mut c_void,
        KEYSIZE,
        0, // STBDS_OFFSETOF((t),key) = 0 since key is first field
        STBDS_HM_STRING,
    ) as *mut StrMapEntry;
    if result.is_null() {
        (result, 0)
    } else {
        let raw = hash_to_arr(result as *mut c_void, ELEMSIZE);
        (result, (*stbds_header(raw)).temp)
    }
}

/// shdefault(strmap, v) => hmdefault
unsafe fn sh_geti_shdefault(strmap: *mut StrMapEntry, v: c_int) -> *mut StrMapEntry {
    let result = stbds_hmput_default(strmap as *mut c_void, ELEMSIZE) as *mut StrMapEntry;
    (*result.offset(-1)).value = v;
    result
}

/// sh_new_strdup(strmap) => shmode_func_wrapper
unsafe fn sh_geti_sh_new_strdup(strmap: *mut StrMapEntry) -> *mut StrMapEntry {
    stbds_shmode_func(ELEMSIZE, STBDS_SH_STRDUP as c_int) as *mut StrMapEntry
}

/// sh_new_arena(strmap) => shmode_func_wrapper
unsafe fn sh_geti_sh_new_arena(strmap: *mut StrMapEntry) -> *mut StrMapEntry {
    stbds_shmode_func(ELEMSIZE, STBDS_SH_ARENA as c_int) as *mut StrMapEntry
}

/// shfree(strmap) => hmfree
unsafe fn sh_geti_shfree(strmap: *mut StrMapEntry) {
    if !strmap.is_null() {
        stbds_hmfree_func(
            strmap.offset(-1) as *mut c_void,
            ELEMSIZE,
        );
    }
}

/// shlen(strmap) => hmlen
unsafe fn sh_geti_shlen(strmap: *mut StrMapEntry) -> isize {
    if strmap.is_null() {
        0
    } else {
        (*stbds_header(strmap.offset(-1) as *mut c_void)).length as isize - 1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let mut strmap: *mut StrMapEntry = ptr::null_mut();
    let mut sa: stbds_string_arena = std::mem::zeroed();

    for i in 0..num {
        stbds_stralloc(&mut sa, strkey(i));
    }
    stbds_strreset(&mut sa);

    for j in 0..2 {
        // STBDS_ASSERT(shgeti(strmap,"foo") == -1);
        {
            let (new_strmap, temp) = sh_geti_shgeti(strmap, b"foo\0".as_ptr() as *const c_char);
            strmap = new_strmap;
            assert!(temp == -1);
        }

        if j == 0 {
            strmap = sh_geti_sh_new_strdup(strmap);
        } else {
            strmap = sh_geti_sh_new_arena(strmap);
        }

        // STBDS_ASSERT(shgeti(strmap,"foo") == -1);
        {
            let (new_strmap, temp) = sh_geti_shgeti(strmap, b"foo\0".as_ptr() as *const c_char);
            strmap = new_strmap;
            assert!(temp == -1);
        }

        // shdefault(strmap, -2);
        strmap = sh_geti_shdefault(strmap, -2);

        // STBDS_ASSERT(shgeti(strmap,"foo") == -1);
        {
            let (new_strmap, temp) = sh_geti_shgeti(strmap, b"foo\0".as_ptr() as *const c_char);
            strmap = new_strmap;
            assert!(temp == -1);
        }

        // for (i=0; i < num; i+=2) shput(strmap, strkey(i), i*3);
        {
            let mut i = 0;
            while i < num {
                strmap = sh_geti_shput(strmap, strkey(i), i * 3);
                i += 2;
            }
        }

        // for (int z=0; z < shlen(strmap); ++z)
        //   printf("%s %d\n", strmap[z], strmap[z].value);
        // NOTE: In the C code, strmap[z] in the first %s position is actually
        // strmap[z].key (the first field, which is char*). The C compiler
        // passes the struct but printf reads it as char* from the first field.
        {
            let len = sh_geti_shlen(strmap);
            for z in 0..len {
                let entry = &*strmap.offset(z);
                libc::printf(
                    b"%s %d\n\0".as_ptr() as *const c_char,
                    entry.key,
                    entry.value,
                );
            }
        }

        // for (i=0; i < num; i+=1)
        //   if (i & 1) assert(shget == -2) else assert(shget == i*3)
        {
            for i in 0..num {
                let (new_strmap, val) = sh_geti_shget(strmap, strkey(i));
                strmap = new_strmap;
                if i & 1 != 0 {
                    assert!(val == -2);
                } else {
                    assert!(val == i * 3);
                }
            }
        }

        // for (i=2; i < num; i+=4) shdel(strmap, strkey(i));
        {
            let mut i = 2;
            while i < num {
                let (new_strmap, _) = sh_geti_shdel(strmap, strkey(i));
                strmap = new_strmap;
                i += 4;
            }
        }

        // for (i=0; i < num; i+=1)
        //   if (i & 3) assert(shget == -2) else assert(shget == i*3)
        {
            for i in 0..num {
                let (new_strmap, val) = sh_geti_shget(strmap, strkey(i));
                strmap = new_strmap;
                if i & 3 != 0 {
                    assert!(val == -2);
                } else {
                    assert!(val == i * 3);
                }
            }
        }

        // for (i=0; i < num; i+=1) shdel(strmap, strkey(i));
        {
            for i in 0..num {
                let (new_strmap, _) = sh_geti_shdel(strmap, strkey(i));
                strmap = new_strmap;
            }
        }

        // for (i=0; i < num; i+=1) assert(shget == -2)
        {
            for i in 0..num {
                let (new_strmap, val) = sh_geti_shget(strmap, strkey(i));
                strmap = new_strmap;
                assert!(val == -2);
            }
        }

        // shfree(strmap);
        sh_geti_shfree(strmap);
        strmap = ptr::null_mut();
    }
}
