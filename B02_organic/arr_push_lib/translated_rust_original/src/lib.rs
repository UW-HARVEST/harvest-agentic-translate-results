#![allow(
    non_camel_case_types,
    non_snake_case,
    unused_parens,
    clippy::missing_safety_doc,
    dead_code,
)]

use std::ffi::c_int;
use std::ptr;

// ── Constants ──────────────────────────────────────────────────────────
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

// ── Structs ────────────────────────────────────────────────────────────
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
pub struct stbds_string_block {
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

// ── Globals ────────────────────────────────────────────────────────────
static mut STBDS_HASH_SEED: usize = 0x31415926;

// ── Helpers ────────────────────────────────────────────────────────────
#[inline]
fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).offset(-1) }
}

#[inline]
fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { unsafe { (*stbds_header(a)).length as isize } }
}

#[inline]
fn stbds_arrlenu(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { unsafe { (*stbds_header(a)).length } }
}

#[inline]
fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { unsafe { (*stbds_header(a)).capacity } }
}

#[inline]
fn stbds_temp(a: *mut u8) -> &'static mut isize {
    unsafe { &mut (*stbds_header(a)).temp }
}

#[inline]
fn stbds_temp_key(a: *mut u8) -> *mut *mut u8 {
    unsafe { (*stbds_header(a)).hash_table as *mut *mut u8 }
}

#[inline]
fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// C-compatible realloc: if ptr is null, acts like malloc; size 0 with non-null frees
unsafe fn c_realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    if size == 0 {
        if !ptr.is_null() {
            // Can't dealloc without knowing layout, use libc
            libc::free(ptr as *mut libc::c_void);
        }
        return ptr::null_mut();
    }
    if ptr.is_null() {
        libc::malloc(size) as *mut u8
    } else {
        libc::realloc(ptr as *mut libc::c_void, size) as *mut u8
    }
}

unsafe fn c_free(ptr: *mut u8) {
    if !ptr.is_null() {
        libc::free(ptr as *mut libc::c_void);
    }
}

// ── stbds_arrgrowf ────────────────────────────────────────────────────
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

    let old_ptr = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut u8
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

// ── stbds_arrfreef ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) {
    c_free(stbds_header(a) as *mut u8);
}

// ── stbds_rand_seed ───────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// ── Hash functions ────────────────────────────────────────────────────
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut u8, seed: usize) -> usize {
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

unsafe fn stbds_siphash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
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
            v0 = v0.wrapping_add(v1); v1 = rotate_left(v1, 13); v1 ^= v0; v0 = rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3); v3 = rotate_left(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = rotate_left(v1, 17); v1 ^= v2; v2 = rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3); v3 = rotate_left(v3, 21); v3 ^= v0;
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
            | ((*dp.add(7) as usize) << 24)) << 16 << 16;
        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS { sipround!(); }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let dp = d.add(i);
    let rem = len - i;
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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ── stbds_make_hash_index ─────────────────────────────────────────────
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        * std::mem::size_of::<stbds_hash_bucket>()
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
        (*t).seed = STBDS_HASH_SEED;
        let (a, b): (usize, usize);
        // stbds_load_32_or_64 for a: v32=2147001325, v64_hi=0x27bb2ee6, v64_lo=0x87b0b0fd
        {
            let v32: usize = 2147001325;
            let v64_hi: usize = 0x27bb2ee6;
            let v64_lo: usize = 0x87b0b0fd;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            let mut var: usize = v64_hi;
            var <<= 16; var <<= 16;
            a = var ^ temp ^ v32;
        }
        // stbds_load_32_or_64 for b: v32=715136305, v64_hi=0, v64_lo=0xb504f32d
        {
            let v32: usize = 715136305;
            let v64_hi: usize = 0;
            let v64_lo: usize = 0xb504f32d;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            let mut var: usize = v64_hi;
            var <<= 16; var <<= 16;
            b = var ^ temp ^ v32;
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

// ── stbds_is_key_equal ────────────────────────────────────────────────
unsafe fn stbds_is_key_equal(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    keyoffset: usize, mode: c_int, i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_key_ptr = (a.add(elemsize * i as usize + keyoffset)) as *mut *mut u8;
        libc::strcmp(key as *const i8, *stored_key_ptr as *const i8) == 0
    } else {
        libc::memcmp(
            key as *const libc::c_void,
            a.add(elemsize * i as usize + keyoffset) as *const libc::c_void,
            keysize,
        ) == 0
    }
}

// ── stbds_hm_find_slot ───────────────────────────────────────────────
unsafe fn stbds_hm_find_slot(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    keyoffset: usize, mode: c_int,
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

// ── stbds_hmfree_func ─────────────────────────────────────────────────
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

// ── stbds_hmget_key_ts ────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    temp: *mut isize, mode: c_int,
) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let arr = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(arr)).length += 1;
        ptr::write_bytes(arr, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr.add(elemsize); // STBDS_ARR_TO_HASH
    } else {
        let raw_a = a.sub(elemsize); // STBDS_HASH_TO_ARR
        let table = stbds_hash_table(raw_a);
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

// ── stbds_hmget_key ───────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int,
) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let raw = p.sub(elemsize); // STBDS_HASH_TO_ARR
    *stbds_temp(raw) = temp;
    p
}

// ── stbds_hmput_default ───────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*stbds_header(a.sub(elemsize))).length == 0 {
        let raw = if !a.is_null() { a.sub(elemsize) } else { ptr::null_mut() };
        let arr = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(arr)).length += 1;
        ptr::write_bytes(arr, 0, elemsize);
        return arr.add(elemsize);
    }
    a
}

// ── stbds_strdup (internal) ───────────────────────────────────────────
unsafe fn stbds_strdup_internal(str: *mut u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;
    let p = c_realloc(ptr::null_mut(), len);
    ptr::copy(str, p, len);
    p
}

// ── stbds_hmput_key ───────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int,
) -> *mut u8 {
    let keyoffset: usize = 0;

    let a = if a.is_null() {
        let arr = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(arr, 0, elemsize);
        (*stbds_header(arr)).length += 1;
        arr.add(elemsize) // ARR_TO_HASH
    } else {
        a
    };

    let raw_a = a;
    let arr = a.sub(elemsize); // HASH_TO_ARR

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
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*stbds_header(arr)).hash_table = nt as *mut u8;
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

    let found_pos: usize;
    'search: loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    *stbds_temp(arr) = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *stbds_temp_key(arr) = *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset) as *mut *mut u8);
                    }
                    return arr.add(elemsize);
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
                    *stbds_temp(arr) = bucket.index[i];
                    return arr.add(elemsize);
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

    let final_pos = if tombstone >= 0 {
        (*table).tombstone_count -= 1;
        tombstone as usize
    } else {
        found_pos
    };
    (*table).used_count += 1;

    let i = stbds_arrlen(arr);
    let mut arr = arr;
    if (i as usize + 1) > stbds_arrcap(arr) {
        arr = stbds_arrgrowf(arr, elemsize, 1, 0);
    }
    let raw_a = arr.add(elemsize); // ARR_TO_HASH

    assert!((i as usize + 1) <= stbds_arrcap(arr));
    (*stbds_header(arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(final_pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[final_pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[final_pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(arr) = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dst = arr.add(elemsize * i as usize) as *mut *mut u8;
            *dst = stbds_strdup_internal(key);
            *stbds_temp_key(arr) = *dst;
        }
        STBDS_SH_ARENA => {
            let dst = arr.add(elemsize * i as usize) as *mut *mut u8;
            *dst = stbds_stralloc(&mut (*table).string, key) as *mut u8;
            *stbds_temp_key(arr) = *dst;
        }
        STBDS_SH_DEFAULT => {
            let dst = arr.add(elemsize * i as usize) as *mut *mut u8;
            *dst = key;
            *stbds_temp_key(arr) = *dst;
        }
        _ => {
            ptr::copy_nonoverlapping(key, arr.add(elemsize * i as usize), keysize);
        }
    }

    raw_a
}

// ── stbds_shmode_func ─────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut u8;
    a.add(elemsize) // ARR_TO_HASH
}

// ── stbds_hmdel_key ───────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    keyoffset: usize, mode: c_int,
) -> *mut u8 {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = a.sub(elemsize);
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
    let i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;

    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 (always true for usize)

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
            let k = *(a.add(elemsize * old_index as usize + keyoffset) as *mut *mut u8);
            stbds_hm_find_slot(a, elemsize, k, keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(
                a, elemsize,
                a.add(elemsize * old_index as usize + keyoffset),
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

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        c_free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut u8;
        c_free(table as *mut u8);
    }

    a
}

// ── stbds_stralloc ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;
    if len > (*a).remaining {
        let blocksize_shift = (*a).block >> 1;
        let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_shift as usize);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }
        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            ptr::copy_nonoverlapping(str, (*sb).storage.as_mut_ptr(), len);
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
    ptr::copy_nonoverlapping(str, p, len);
    p
}

// ── stbds_strreset ────────────────────────────────────────────────────
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

// ── stbds_unit_tests (stub) ───────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {
    // Not implemented in the C source's lib.c either (only declared extern)
}

// ── arr_push ──────────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let arr_ptr = &mut arr as *mut *mut c_int;

    assert!(stbds_arrlen(arr as *mut u8) == 0);

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            // arrpush(arr, j) expands to:
            // stbds_arrmaybegrow then arr[header->length++] = j
            let a = *arr_ptr as *mut u8;
            let elemsize = std::mem::size_of::<c_int>();
            if a.is_null()
                || (*stbds_header(a)).length + 1 > (*stbds_header(a)).capacity
            {
                *arr_ptr = stbds_arrgrowf(a, elemsize, 1, 0) as *mut c_int;
            }
            let a = *arr_ptr as *mut u8;
            let idx = (*stbds_header(a)).length;
            (*stbds_header(a)).length = idx + 1;
            *(*arr_ptr).add(idx) = j;
            j += 1;
        }
        // arrfree(arr)
        let a = *arr_ptr as *mut u8;
        if !a.is_null() {
            c_free(stbds_header(a) as *mut u8);
        }
        *arr_ptr = ptr::null_mut();
        i += 50;
    }
}
