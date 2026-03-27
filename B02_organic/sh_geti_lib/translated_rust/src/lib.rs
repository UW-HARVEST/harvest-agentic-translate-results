#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    unused_assignments,
    clippy::missing_safety_doc
)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ── constants ──────────────────────────────────────────────────────────
const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;
const STBDS_SIZE_T_BITS: usize = 64;

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

// ── global mutable state ───────────────────────────────────────────────
static mut stbds_hash_seed: usize = 0x31415926;

// ── C-compatible structs ───────────────────────────────────────────────
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

// ── helper inlines ─────────────────────────────────────────────────────
#[inline(always)]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() { 0 } else { (*stbds_header(a)).length as isize }
}

#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() { 0 } else { (*stbds_header(a)).capacity }
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline(always)]
fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

#[inline(always)]
fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// temp_key: first pointer-sized value at hash_table
#[inline(always)]
unsafe fn stbds_temp_key(a: *mut c_void) -> *mut *mut c_char {
    (*stbds_header(a)).hash_table as *mut *mut c_char
}

extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

// ── stbds_rand_seed ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

// ── stbds_hash_string ──────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str as *const u8;
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

// ── siphash ────────────────────────────────────────────────────────────
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1); v1 = rotate_left(v1, 13); v1 ^= v0; v0 = rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3); v3 = rotate_left(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = rotate_left(v1, 17); v1 ^= v2; v2 = rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3); v3 = rotate_left(v3, 21); v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + 8 <= len {
        let dp = d.add(i);
        data = *dp.add(0) as usize
            | ((*dp.add(1) as usize) << 8)
            | ((*dp.add(2) as usize) << 16)
            | ((*dp.add(3) as usize) << 24);
        data |= ((*dp.add(4) as usize)
            | ((*dp.add(5) as usize) << 8)
            | ((*dp.add(6) as usize) << 16)
            | ((*dp.add(7) as usize) << 24)) << 16 << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS { sipround!(); }
        v0 ^= data;
        i += 8;
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    let dp = d.add(i);
    // C fallthrough switch
    if rem >= 7 { data |= (*dp.add(6) as usize) << 24 << 24; }
    if rem >= 6 { data |= (*dp.add(5) as usize) << 20 << 20; }
    if rem >= 5 { data |= (*dp.add(4) as usize) << 16 << 16; }
    if rem >= 4 { data |= (*dp.add(3) as usize) << 24; }
    if rem >= 3 { data |= (*dp.add(2) as usize) << 16; }
    if rem >= 2 { data |= (*dp.add(1) as usize) << 8; }
    if rem >= 1 { data |= *dp.add(0) as usize; }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS { sipround!(); }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS { sipround!(); }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ── stbds_unit_tests (stub) ────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {}

// ── stbds_arrgrowf ─────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void, elemsize: usize, addlen: usize, min_cap_arg: usize,
) -> *mut c_void {
    let mut min_cap = min_cap_arg;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap { min_cap = min_len; }
    if min_cap <= stbds_arrcap(a) { return a; }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if !a.is_null() { stbds_header(a) as *mut c_void } else { ptr::null_mut() };
    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b_raw = realloc(old, alloc_size);
    let b = (b_raw as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

// ── stbds_arrfreef ─────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

// ── internal hash helpers ──────────────────────────────────────────────
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 { slot_count >>= 1; n += 1; }
    n
}

unsafe fn stbds_make_hash_index(
    slot_count: usize, ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let num_buckets = slot_count >> STBDS_BUCKET_SHIFT;
    let alloc_size = num_buckets * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    let after_t = (t as *mut u8).add(std::mem::size_of::<stbds_hash_index>());
    (*t).storage = align_fwd(after_t as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
        memset(&mut (*t).string as *mut stbds_string_arena as *mut c_void, 0,
               std::mem::size_of::<stbds_string_arena>());
        (*t).seed = stbds_hash_seed;
        let a: usize;
        let b: usize;
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

    // zero-init buckets
    for i in 0..num_buckets {
        let bucket = &mut *(*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH { bucket.hash[j] = STBDS_HASH_EMPTY; }
        for j in 0..STBDS_BUCKET_LENGTH { bucket.index[j] = STBDS_INDEX_EMPTY; }
    }

    // rehash from old table
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let old_num_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..old_num_buckets {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let hash = ob.hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
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

// ── stbds_is_key_equal ─────────────────────────────────────────────────
unsafe fn stbds_is_key_equal(
    a: *mut c_void, elemsize: usize, key: *mut c_void,
    keysize: usize, keyoffset: usize, mode: c_int, i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored = *(((a as *mut u8).offset(elemsize as isize * i).add(keyoffset)) as *const *const c_char);
        strcmp(key as *const c_char, stored) == 0
    } else {
        memcmp(key, (a as *mut u8).offset(elemsize as isize * i).add(keyoffset) as *const c_void, keysize) == 0
    }
}

// ── stbds_hmfree_func ──────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() { return; }
    let ht = stbds_hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_void);
                free(p);
            }
        }
        stbds_strreset(&mut (*ht).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

// ── stbds_hm_find_slot ─────────────────────────────────────────────────
unsafe fn stbds_hm_find_slot(
    a: *mut c_void, elemsize: usize, key: *mut c_void,
    keysize: usize, keyoffset: usize, mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
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

// ── stbds_hmget_key_ts ─────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void, elemsize: usize, key: *mut c_void,
    keysize: usize, temp: *mut isize, mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        memset(new_a, 0, elemsize);
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
                *temp = b.index[slot as usize & STBDS_BUCKET_MASK];
            }
        }
        return a;
    }
}

// ── stbds_hmget_key ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void, elemsize: usize, key: *mut c_void,
    keysize: usize, mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*stbds_header(hash_to_arr(p, elemsize))).temp = temp;
    p
}

// ── stbds_hmput_default ────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() { hash_to_arr(a, elemsize) } else { ptr::null_mut() };
        let new_a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        memset(new_a, 0, elemsize);
        return arr_to_hash(new_a, elemsize);
    }
    a
}

// ── stbds_strdup (internal) ────────────────────────────────────────────
unsafe fn stbds_strdup_internal(str: *mut c_char) -> *mut c_char {
    let len = strlen(str) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ── stbds_hmput_key ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a_in: *mut c_void, elemsize: usize, key: *mut c_void,
    keysize: usize, mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut a: *mut c_void;
    let raw_a: *mut c_void;

    if a_in.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    } else {
        a = a_in;
    }

    raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*stbds_header(a)).hash_table = nt as *mut c_void;
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

    let found_pos: usize;
    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(a)).temp = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *stbds_temp_key(a) = *((raw_a as *mut u8).offset(elemsize as isize * bucket.index[i]).add(keyoffset) as *mut *mut c_char);
                    }
                    return arr_to_hash(a, elemsize);
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
                    (*stbds_header(a)).temp = bucket.index[i];
                    return arr_to_hash(a, elemsize);
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

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    // found_empty_slot:
    let mut final_pos = found_pos;
    if tombstone >= 0 {
        final_pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(a) as isize;
    if (i as usize + 1) > stbds_arrcap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    let raw_a2 = arr_to_hash(a, elemsize);

    assert!((i as usize + 1) <= stbds_arrcap(a));
    (*stbds_header(a)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(final_pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[final_pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[final_pos & STBDS_BUCKET_MASK] = i - 1;
    (*stbds_header(a)).temp = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup_internal(key as *mut c_char);
            *((a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char) = dup;
            *stbds_temp_key(a) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *((a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char) = s;
            *stbds_temp_key(a) = s;
        }
        STBDS_SH_DEFAULT => {
            *((a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char) = key as *mut c_char;
            *stbds_temp_key(a) = key as *mut c_char;
        }
        _ => {
            memmove(
                (a as *mut u8).add(elemsize * i as usize) as *mut c_void,
                key as *const c_void,
                keysize,
            );
        }
    }

    let _ = raw_a2;
    arr_to_hash(a, elemsize)
}

// ── stbds_shmode_func ──────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut c_void;
    arr_to_hash(a, elemsize)
}

// ── stbds_hmdel_key ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void, elemsize: usize, key: *mut c_void,
    keysize: usize, keyoffset: usize, mode: c_int,
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
    let i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = stbds_arrlen(raw_a) as isize - 1 - 1;

    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 always true for usize
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

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

        let slot2 = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(
                a, elemsize,
                *((a as *mut u8).offset(elemsize as isize * old_index).add(keyoffset) as *mut *mut c_void),
                keysize, keyoffset, mode,
            )
        } else {
            stbds_hm_find_slot(
                a, elemsize,
                (a as *mut u8).offset(elemsize as isize * old_index).add(keyoffset) as *mut c_void,
                keysize, keyoffset, mode,
            )
        };
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add((slot2 as usize) >> STBDS_BUCKET_SHIFT);
        let i2 = slot2 as usize & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
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

// ── stbds_stralloc ─────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut c_char) -> *mut c_char {
    let len = strlen(str) + 1;
    if len > (*a).remaining {
        let blocksize_exp = (*a).block;
        let mut blocksize = (STBDS_STRING_ARENA_BLOCKSIZE_MIN as usize) << (blocksize_exp as usize >> 1);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }
        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            memmove((*sb).storage.as_mut_ptr() as *mut c_void, str as *const c_void, len);
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

// ── stbds_strreset ─────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, std::mem::size_of::<stbds_string_arena>());
}

// ── sh_geti ────────────────────────────────────────────────────────────
// Reproduces the C struct layout: { char *key; int value; } → 16 bytes on 64-bit
// with key at offset 0, value at offset 8.
// The stbds array element at index -1 (the "default") also has this layout.

const STRMAP_ELEMSIZE: usize = 16; // sizeof(struct { char *key; int value; }) with padding
const STRMAP_KEY_OFFSET: usize = 0;
const STRMAP_VALUE_OFFSET: usize = 8;

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let fmt = b"test_%d\0".as_ptr() as *const c_char;
    sprintf(BUFFER.as_mut_ptr(), fmt, n);
    BUFFER.as_mut_ptr()
}

// Macro-like helpers that mirror the C macros for the strmap type
// shgeti(strmap, k) → hmget_key_wrapper then read temp
unsafe fn sh_geti_shgeti(strmap: &mut *mut c_void, k: *const c_char) -> isize {
    *strmap = stbds_hmget_key(*strmap, STRMAP_ELEMSIZE, k as *mut c_void, STRMAP_ELEMSIZE, STBDS_HM_STRING);
    (*stbds_header(hash_to_arr(*strmap, STRMAP_ELEMSIZE))).temp
}

// shput(strmap, k, v)
unsafe fn sh_geti_shput(strmap: &mut *mut c_void, k: *const c_char, v: c_int) {
    *strmap = stbds_hmput_key(*strmap, STRMAP_ELEMSIZE, k as *mut c_void, STRMAP_ELEMSIZE, STBDS_HM_STRING);
    let raw = hash_to_arr(*strmap, STRMAP_ELEMSIZE);
    let temp = (*stbds_header(raw)).temp;
    let elem = (*strmap as *mut u8).offset(STRMAP_ELEMSIZE as isize * temp) as *mut u8;
    *(elem.add(STRMAP_VALUE_OFFSET) as *mut c_int) = v;
}

// shget(strmap, k) → value
unsafe fn sh_geti_shget(strmap: &mut *mut c_void, k: *const c_char) -> c_int {
    // shgetp then ->value
    let _ = sh_geti_shgeti(strmap, k);
    let raw = hash_to_arr(*strmap, STRMAP_ELEMSIZE);
    let temp = (*stbds_header(raw)).temp;
    let elem = (*strmap as *mut u8).offset(STRMAP_ELEMSIZE as isize * temp) as *const u8;
    *(elem.add(STRMAP_VALUE_OFFSET) as *const c_int)
}

// shdel(strmap, k)
unsafe fn sh_geti_shdel(strmap: &mut *mut c_void, k: *const c_char) {
    *strmap = stbds_hmdel_key(
        *strmap, STRMAP_ELEMSIZE, k as *mut c_void, STRMAP_ELEMSIZE,
        STRMAP_KEY_OFFSET, STBDS_HM_STRING,
    );
}

// shdefault(strmap, v) → hmput_default then set [-1].value
unsafe fn sh_geti_shdefault(strmap: &mut *mut c_void, v: c_int) {
    *strmap = stbds_hmput_default(*strmap, STRMAP_ELEMSIZE);
    let raw = hash_to_arr(*strmap, STRMAP_ELEMSIZE);
    // element at index -1 relative to the hash pointer = element 0 of raw array
    let default_elem = (*strmap as *mut u8).offset(-(STRMAP_ELEMSIZE as isize));
    *(default_elem.add(STRMAP_VALUE_OFFSET) as *mut c_int) = v;
}

// sh_new_strdup(strmap)
unsafe fn sh_geti_sh_new_strdup(strmap: &mut *mut c_void) {
    *strmap = stbds_shmode_func(STRMAP_ELEMSIZE, STBDS_SH_STRDUP as c_int);
}

// sh_new_arena(strmap)
unsafe fn sh_geti_sh_new_arena(strmap: &mut *mut c_void) {
    *strmap = stbds_shmode_func(STRMAP_ELEMSIZE, STBDS_SH_ARENA as c_int);
}

// shfree(strmap)
unsafe fn sh_geti_shfree(strmap: &mut *mut c_void) {
    if !(*strmap).is_null() {
        let raw = hash_to_arr(*strmap, STRMAP_ELEMSIZE);
        stbds_hmfree_func(raw, STRMAP_ELEMSIZE);
    }
    *strmap = ptr::null_mut();
}

// shlen(strmap) → hmlen
unsafe fn sh_geti_shlen(strmap: *mut c_void) -> isize {
    if strmap.is_null() { 0 }
    else {
        let raw = hash_to_arr(strmap, STRMAP_ELEMSIZE);
        (*stbds_header(raw)).length as isize - 1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let mut strmap: *mut c_void = ptr::null_mut();
    let mut sa: stbds_string_arena = std::mem::zeroed();

    for i in 0..num {
        stbds_stralloc(&mut sa, strkey(i));
    }
    stbds_strreset(&mut sa);

    for j in 0..2 {
        assert!(sh_geti_shgeti(&mut strmap, b"foo\0".as_ptr() as *const c_char) == -1);
        if j == 0 {
            sh_geti_sh_new_strdup(&mut strmap);
        } else {
            sh_geti_sh_new_arena(&mut strmap);
        }
        assert!(sh_geti_shgeti(&mut strmap, b"foo\0".as_ptr() as *const c_char) == -1);
        sh_geti_shdefault(&mut strmap, -2);
        assert!(sh_geti_shgeti(&mut strmap, b"foo\0".as_ptr() as *const c_char) == -1);

        let mut i = 0;
        while i < num {
            sh_geti_shput(&mut strmap, strkey(i), i * 3);
            i += 2;
        }

        // printf loop: for z in 0..shlen(strmap) printf("%s %d\n", strmap[z].key, strmap[z].value)
        let len = sh_geti_shlen(strmap);
        let fmt = b"%s %d\n\0".as_ptr() as *const c_char;
        for z in 0..len {
            let elem = (strmap as *mut u8).offset(STRMAP_ELEMSIZE as isize * z as isize);
            let key_ptr = *(elem.add(STRMAP_KEY_OFFSET) as *const *const c_char);
            let val = *(elem.add(STRMAP_VALUE_OFFSET) as *const c_int);
            printf(fmt, key_ptr, val);
        }

        i = 0;
        while i < num {
            if i & 1 != 0 {
                assert!(sh_geti_shget(&mut strmap, strkey(i)) == -2);
            } else {
                assert!(sh_geti_shget(&mut strmap, strkey(i)) == i * 3);
            }
            i += 1;
        }

        i = 2;
        while i < num {
            sh_geti_shdel(&mut strmap, strkey(i));
            i += 4;
        }

        i = 0;
        while i < num {
            if i & 3 != 0 {
                assert!(sh_geti_shget(&mut strmap, strkey(i)) == -2);
            } else {
                assert!(sh_geti_shget(&mut strmap, strkey(i)) == i * 3);
            }
            i += 1;
        }

        i = 0;
        while i < num {
            sh_geti_shdel(&mut strmap, strkey(i));
            i += 1;
        }

        i = 0;
        while i < num {
            assert!(sh_geti_shget(&mut strmap, strkey(i)) == -2);
            i += 1;
        }

        sh_geti_shfree(&mut strmap);
    }
}
