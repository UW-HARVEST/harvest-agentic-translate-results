#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    unused_assignments,
    clippy::missing_safety_doc
)]

use std::ptr;

use libc::{c_char, c_int, c_void, free, memcmp, memmove, printf, realloc, strcmp, strlen};

// --- Constants ---
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

// --- Structs (C-compatible layout) ---
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
    storage: [c_char; 8],
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

// --- Global state ---
static mut stbds_hash_seed: usize = 0x31415926;

// --- Helper macros as inline fns ---
#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() { 0 } else { (*stbds_header(a)).length as isize }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() { 0 } else { (*stbds_header(a)).capacity }
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
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
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

// --- Public API functions ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str as *mut u8;
    while *s != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
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

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let d = p as *mut u8;
    let mut v0: usize = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1); v1 = stbds_rotate_left(v1, 13); v1 ^= v0; v0 = stbds_rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3); v3 = stbds_rotate_left(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = stbds_rotate_left(v1, 17); v1 ^= v2; v2 = stbds_rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3); v3 = stbds_rotate_left(v3, 21); v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dp = d.add(i);
        // C uses int arithmetic: d[3]<<24 sign-extends when d[3]>=128
        let lo = *dp.add(0) as i32
            | ((*dp.add(1) as i32) << 8)
            | ((*dp.add(2) as i32) << 16)
            | ((*dp.add(3) as i32) << 24);
        let hi = *dp.add(4) as i32
            | ((*dp.add(5) as i32) << 8)
            | ((*dp.add(6) as i32) << 16)
            | ((*dp.add(7) as i32) << 24);
        let mut data: usize = lo as usize; // sign-extends like C int->size_t
        data |= (hi as usize) << 16 << 16;

        v3 ^= data;
        for _ in 0..2 { sipround!(); }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let dp = d.add(i);
    let rem = len - i;
    // C fallthrough switch — case 4 uses int promotion (d[3] << 24 sign-extends)
    if rem >= 7 { data |= ((*dp.add(6) as usize) << 24) << 24; }
    if rem >= 6 { data |= ((*dp.add(5) as usize) << 20) << 20; }
    if rem >= 5 { data |= ((*dp.add(4) as usize) << 16) << 16; }
    if rem >= 4 { data |= ((*dp.add(3) as i32) << 24) as usize; }
    if rem >= 3 { data |= (*dp.add(2) as usize) << 16; }
    if rem >= 2 { data |= (*dp.add(1) as usize) << 8; }
    if rem >= 1 { data |= *dp.add(0) as usize; }

    v3 ^= data;
    for _ in 0..2 { sipround!(); }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 { sipround!(); }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// --- Array grow/free ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void, elemsize: usize, addlen: usize, min_cap: usize,
) -> *mut c_void {
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap { min_cap = min_len; }
    if min_cap <= stbds_arrcap(a) { return a; }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let raw = if a.is_null() {
        realloc(ptr::null_mut(), alloc_size)
    } else {
        realloc(stbds_header(a) as *mut c_void, alloc_size)
    };
    let b = (raw as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

// --- Hash index ---

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

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut stbds_hash_index) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
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
        (*t).string = std::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        std::ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = stbds_hash_seed;
        let (a, b): (usize, usize);
        // stbds_load_32_or_64 for a: v32=2147001325, v64_hi=0x27bb2ee6, v64_lo=0x87b0b0fd
        {
            let mut temp: usize;
            temp = 0x87b0b0fd_usize ^ 2147001325_usize;
            temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            let mut va = 0x27bb2ee6_usize;
            va <<= 16; va <<= 16;
            a = va ^ temp ^ 2147001325_usize;
        }
        // stbds_load_32_or_64 for b: v32=715136305, v64_hi=0, v64_lo=0xb504f32d
        {
            let mut temp: usize;
            temp = 0xb504f32d_usize ^ 715136305_usize;
            temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            let mut vb = 0_usize;
            vb <<= 16; vb <<= 16;
            b = vb ^ temp ^ 715136305_usize;
        }
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
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

// --- Key comparison ---

unsafe fn stbds_is_key_equal(
    a: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize,
    keyoffset: usize, mode: c_int, i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_key_ptr = *((a as *mut u8).offset(elemsize as isize * i + keyoffset as isize) as *mut *mut c_char);
        strcmp(key as *const c_char, stored_key_ptr) == 0
    } else {
        memcmp(
            key,
            (a as *mut u8).offset(elemsize as isize * i + keyoffset as isize) as *const c_void,
            keysize,
        ) == 0
    }
}

// --- hmfree_func ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() { return; }
    // a is the raw array pointer (caller passes hash_view - 1 element = raw_array)
    if !stbds_hash_table(a).is_null() {
        let table = stbds_hash_table(a);
        if (*table).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            // In C: for (i=1; i < length; ++i) free(*(char**)((char*)a + elemsize*i))
            for i in 1..len {
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_void);
                free(p);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

// --- hm_find_slot ---

unsafe fn stbds_hm_find_slot(
    a: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize,
    keyoffset: usize, mode: c_int,
) -> isize {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 { hash += 2; }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
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

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

// --- hmget_key_ts ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize,
    temp: *mut isize, mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        std::ptr::write_bytes(new_a as *mut u8, 0, elemsize);
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

// --- hmget_key ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize, mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*stbds_header(stbds_hash_to_arr(p, elemsize))).temp = temp;
    p
}

// --- hmput_default ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() { stbds_hash_to_arr(a, elemsize) } else { ptr::null_mut() };
        let new_a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        std::ptr::write_bytes(new_a as *mut u8, 0, elemsize);
        return stbds_arr_to_hash(new_a, elemsize);
    }
    a
}

// --- strdup helper ---

unsafe fn stbds_strdup_internal(str: *mut c_char) -> *mut c_char {
    let len = strlen(str) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// --- hmput_key ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize, mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut a = a;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        std::ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = stbds_arr_to_hash(a, elemsize);
    }

    let raw_a = a;
    let arr_a = stbds_hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(arr_a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*stbds_header(arr_a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;

    if hash < 2 { hash += 2; }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(arr_a)).temp = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        let key_ptr = *((raw_a as *mut u8).offset(elemsize as isize * bucket.index[i] + keyoffset as isize) as *mut *mut c_char);
                        *((*stbds_header(arr_a)).hash_table as *mut *mut c_char) = key_ptr;
                    }
                    return stbds_arr_to_hash(arr_a, elemsize);
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                // goto found_empty_slot
                return stbds_hmput_key_found_empty(arr_a, raw_a, elemsize, key, keysize, mode, table, hash, pos, tombstone);
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
                    return stbds_arr_to_hash(arr_a, elemsize);
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                return stbds_hmput_key_found_empty(arr_a, raw_a, elemsize, key, keysize, mode, table, hash, pos, tombstone);
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
    mut arr_a: *mut c_void, raw_a: *mut c_void, elemsize: usize,
    key: *mut c_void, keysize: usize, mode: c_int,
    table: *mut stbds_hash_index, hash: usize, mut pos: usize, tombstone: isize,
) -> *mut c_void {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr_a);
    if (i as usize + 1) > stbds_arrcap(arr_a) {
        arr_a = stbds_arrgrowf(arr_a, elemsize, 1, 0);
    }
    let raw_a_new = stbds_arr_to_hash(arr_a, elemsize);
    let _ = raw_a_new; // raw_a updated

    assert!((i as usize + 1) <= stbds_arrcap(arr_a));
    (*stbds_header(arr_a)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*stbds_header(arr_a)).temp = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup_internal(key as *mut c_char);
            *((arr_a as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char) = dup;
            *((*stbds_header(arr_a)).hash_table as *mut *mut c_char) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *((arr_a as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char) = s;
            *((*stbds_header(arr_a)).hash_table as *mut *mut c_char) = s;
        }
        STBDS_SH_DEFAULT => {
            *((arr_a as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char) = key as *mut c_char;
            *((*stbds_header(arr_a)).hash_table as *mut *mut c_char) = key as *mut c_char;
        }
        _ => {
            memmove(
                (arr_a as *mut u8).offset(elemsize as isize * i) as *mut c_void,
                key,
                keysize,
            );
        }
    }

    stbds_arr_to_hash(arr_a, elemsize)
}

// --- shmode_func ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    std::ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut c_void;
    stbds_arr_to_hash(a, elemsize)
}

// --- hmdel_key ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize,
    keyoffset: usize, mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let mut table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    (*stbds_header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let b = &mut *(*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let bi = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = b.index[bi];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;

    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    b.hash[bi] = STBDS_HASH_DELETED;
    b.index[bi] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *((a as *mut u8).offset(elemsize as isize * old_index) as *mut *mut c_void);
        free(p);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).offset(elemsize as isize * old_index) as *mut c_void,
            (a as *mut u8).offset(elemsize as isize * final_index) as *const c_void,
            elemsize,
        );

        let new_slot = if mode == STBDS_HM_STRING {
            let moved_key = *((a as *mut u8).offset(elemsize as isize * old_index + keyoffset as isize) as *mut *mut c_char);
            stbds_hm_find_slot(a, elemsize, moved_key as *mut c_void, keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(
                a, elemsize,
                (a as *mut u8).offset(elemsize as isize * old_index + keyoffset as isize) as *mut c_void,
                keysize, keyoffset, mode,
            )
        };
        assert!(new_slot >= 0);
        let nb = &mut *(*table).storage.add((new_slot as usize) >> STBDS_BUCKET_SHIFT);
        let ni = (new_slot as usize) & STBDS_BUCKET_MASK;
        assert!(nb.index[ni] == final_index);
        nb.index[ni] = old_index;
    }

    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// --- stralloc / strreset ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut c_char) -> *mut c_char {
    let len = strlen(str) + 1;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    std::ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
}

// --- strkey ---

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    libc::sprintf(buffer.as_mut_ptr(), b"test_%d\0".as_ptr() as *const c_char, n);
    buffer.as_mut_ptr()
}

// --- helxo ---
// Hash map element: struct { char *key; char value; }

#[repr(C)]
struct ShEntry {
    key: *mut c_char,
    value: c_char,
}

// In C, `hash` is a pointer into the "user view" of the array:
//   hash = STBDS_ARR_TO_HASH(raw_arr, elemsize) = raw_arr + elemsize
// So hash[-1] is the default slot, hash[0..N-1] are user entries.
// stbds_hmput_key takes/returns the "hash view" pointer (raw+elemsize).
// stbds_temp(hash-1) = stbds_header(hash-1)->temp = header(raw_arr)->temp
// shlen(hash) = header(hash-1)->length - 1 = header(raw_arr)->length - 1
// shfree(hash) calls stbds_hmfree_func(hash-1, elemsize) = stbds_hmfree_func(raw_arr, elemsize)
//   but stbds_hmfree_func expects `a` to be the raw array pointer already offset by -elemsize
//   from the caller. Actually: #define stbds_hmfree(p) ((p)!=NULL ? stbds_hmfree_func((p)-1,sizeof*(p)),0:0)
//   So it passes hash-1 = raw_arr as `a` to stbds_hmfree_func.
//   Inside stbds_hmfree_func(a, elemsize): a IS the raw array. It does stbds_header(a) etc.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    let mut hash: *mut ShEntry = ptr::null_mut();
    let elemsize = std::mem::size_of::<ShEntry>();
    let keysize = std::mem::size_of::<*mut c_char>();

    macro_rules! shput {
        ($hash:expr, $k:expr, $v:expr) => {{
            // stbds_hmput_key takes the "hash view" = raw+elemsize, which is hash itself
            // (or null if hash is null). It returns the updated "hash view".
            let a_in: *mut c_void = if $hash.is_null() {
                ptr::null_mut()
            } else {
                $hash as *mut c_void
            };
            let result = stbds_hmput_key(a_in, elemsize, $k as *mut c_void, keysize, STBDS_HM_STRING);
            $hash = result as *mut ShEntry;
            // stbds_temp(hash-1) = header(raw_arr)->temp
            let raw_arr = ($hash as *mut u8).sub(elemsize) as *mut c_void;
            let temp = (*stbds_header(raw_arr)).temp;
            (*$hash.offset(temp)).value = $v;
        }};
    }

    macro_rules! shlen {
        ($hash:expr) => {{
            if $hash.is_null() {
                0isize
            } else {
                let raw_arr = ($hash as *mut u8).sub(elemsize) as *mut c_void;
                (*stbds_header(raw_arr)).length as isize - 1
            }
        }};
    }

    macro_rules! shfree {
        ($hash:expr) => {{
            if !$hash.is_null() {
                // stbds_hmfree(p) calls stbds_hmfree_func((p)-1, sizeof*(p))
                // (p)-1 = hash - 1 entry = raw_arr pointer
                let raw_arr = ($hash as *mut u8).sub(elemsize) as *mut c_void;
                stbds_hmfree_func(raw_arr, elemsize);
            }
            $hash = ptr::null_mut();
        }};
    }

    let name: [c_char; 4] = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];

    shput!(hash, b"bob\0".as_ptr(), b'h' as c_char);
    shput!(hash, b"sally\0".as_ptr(), b'e' as c_char);
    shput!(hash, b"fred\0".as_ptr(), b'l' as c_char);
    shput!(hash, b"jen\0".as_ptr(), b'x' as c_char);
    shput!(hash, b"doug\0".as_ptr(), b'o' as c_char);

    shput!(hash, name.as_ptr(), letter);

    let len = shlen!(hash);
    for z in 0..len {
        // C: printf("%s %c\n", hash[z], hash[z].value)
        // hash[z] is a struct passed by value; %s reads the first 8 bytes = key pointer
        let entry = &*hash.offset(z);
        printf(b"%s %c\n\0".as_ptr() as *const c_char, entry.key, entry.value as c_int);
    }

    shfree!(hash);
}
