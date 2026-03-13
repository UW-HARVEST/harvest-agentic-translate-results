#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    clippy::missing_safety_doc,
    dead_code,
    static_mut_refs,
    unused_mut,
    unused_assignments,
    unused_variables,
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

// ── types ──────────────────────────────────────────────────────────────
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

// ── global state ───────────────────────────────────────────────────────
static mut stbds_hash_seed: usize = 0x31415926;
static mut buffer: [u8; 256] = [0u8; 256];

// ── helpers ────────────────────────────────────────────────────────────
#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).offset(-1) }
}

#[inline]
fn stbds_temp(t: *mut c_void) -> &'static mut isize {
    unsafe { &mut (*stbds_header(t)).temp }
}

#[inline]
fn stbds_temp_key(t: *mut c_void) -> *mut *mut c_char {
    unsafe { (*stbds_header(t)).hash_table as *mut *mut c_char }
}

#[inline]
fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() { 0 } else { unsafe { (*stbds_header(a)).length as isize } }
}

#[inline]
fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() { 0 } else { unsafe { (*stbds_header(a)).capacity } }
}

#[inline]
fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

#[inline]
fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
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

// ── libc wrappers ──────────────────────────────────────────────────────
unsafe fn c_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    libc::realloc(p, s) as *mut c_void
}
unsafe fn c_free(p: *mut c_void) {
    libc::free(p as *mut libc::c_void);
}

// ── stbds_rand_seed ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

// ── stbds_arrgrowf ────────────────────────────────────────────────────
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

    let old = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };
    let b = c_realloc(
        old,
        elemsize * min_cap + std::mem::size_of::<stbds_array_header>(),
    );
    let b = (b as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;
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
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    c_free(stbds_header(a) as *mut c_void);
}

// ── stbds_hash_string ─────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    stbds_hash_string_impl(str, seed)
}

unsafe fn stbds_hash_string_impl(str: *mut c_char, seed: usize) -> usize {
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

// ── stbds_siphash_bytes (stbds_hash_bytes) ────────────────────────────
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let d_start = p as *const u8;
    let mut d = d_start;

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
    while i + 8 <= len {
        let mut data: usize = *d.add(0) as usize
            | ((*d.add(1) as usize) << 8)
            | ((*d.add(2) as usize) << 16)
            | ((*d.add(3) as usize) << 24);
        data |= ((*d.add(4) as usize)
            | ((*d.add(5) as usize) << 8)
            | ((*d.add(6) as usize) << 16)
            | ((*d.add(7) as usize) << 24))
            << 16
            << 16;

        v3 ^= data;
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;
        i += 8;
        d = d.add(8);
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    match len - i {
        7 => {
            data |= ((*d.add(6) as usize) << 24) << 24;
            data |= ((*d.add(5) as usize) << 20) << 20;
            data |= ((*d.add(4) as usize) << 16) << 16;
            data |= (*d.add(3) as usize) << 24;
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        6 => {
            data |= ((*d.add(5) as usize) << 20) << 20;
            data |= ((*d.add(4) as usize) << 16) << 16;
            data |= (*d.add(3) as usize) << 24;
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        5 => {
            data |= ((*d.add(4) as usize) << 16) << 16;
            data |= (*d.add(3) as usize) << 24;
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        4 => {
            data |= (*d.add(3) as usize) << 24;
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        3 => {
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        2 => {
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        1 => {
            data |= *d.add(0) as usize;
        }
        0 => {}
        _ => {}
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

// ── stbds_is_key_equal ────────────────────────────────────────────────
unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_ptr = *((a as *const u8).offset(elemsize as isize * i + keyoffset as isize)
            as *const *const c_char);
        libc::strcmp(key as *const c_char, stored_ptr) == 0
    } else {
        libc::memcmp(
            key,
            (a as *const u8).offset(elemsize as isize * i + keyoffset as isize) as *const c_void,
            keysize,
        ) == 0
    }
}

// ── stbds_hm_find_slot ───────────────────────────────────────────────
unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string_impl(key as *mut c_char, (*table).seed)
    } else {
        stbds_siphash_bytes(key, keysize, (*table).seed)
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
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i])
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        for i in 0..start {
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

// ── stbds_hmfree_func ─────────────────────────────────────────────────
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
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_void);
                c_free(p);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    c_free((*stbds_header(a)).hash_table);
    c_free(stbds_header(a) as *mut c_void);
}

// ── stbds_hmget_key_ts ────────────────────────────────────────────────
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
        let a2 = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a2)).length += 1;
        std::ptr::write_bytes(a2 as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(a2, elemsize);
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

// ── stbds_hmget_key ───────────────────────────────────────────────────
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
    *stbds_temp(hash_to_arr(p, elemsize)) = temp;
    p
}

// ── stbds_hmput_default ───────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        let a2 = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(a2)).length += 1;
        std::ptr::write_bytes(a2 as *mut u8, 0, elemsize);
        return arr_to_hash(a2, elemsize);
    }
    a
}

// ── stbds_strdup (internal) ───────────────────────────────────────────
unsafe fn stbds_strdup_impl(str: *const c_char) -> *mut c_char {
    let len = libc::strlen(str) + 1;
    let p = c_realloc(ptr::null_mut(), len) as *mut c_char;
    libc::memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ── stbds_hmput_key ───────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;

    let mut a = if a.is_null() {
        let a2 = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        std::ptr::write_bytes(a2 as *mut u8, 0, elemsize);
        (*stbds_header(a2)).length += 1;
        arr_to_hash(a2, elemsize)
    } else {
        a
    };

    let raw_a = a;
    let mut arr = hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(arr)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            c_free(table as *mut c_void);
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
        stbds_hash_string_impl(key as *mut c_char, (*table).seed)
    } else {
        stbds_siphash_bytes(key, keysize, (*table).seed)
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
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    bucket.index[i],
                ) {
                    *stbds_temp(arr) = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *stbds_temp_key(arr) = *((raw_a as *const u8)
                            .offset(elemsize as isize * bucket.index[i] + keyoffset as isize)
                            as *const *mut c_char);
                    }
                    return arr_to_hash(arr, elemsize);
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                // goto found_empty_slot
                return stbds_hmput_key_found_empty(
                    arr, raw_a, elemsize, key, keysize, table, hash, pos, tombstone, mode,
                );
            } else if tombstone < 0 && bucket.index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    bucket.index[i],
                ) {
                    *stbds_temp(arr) = bucket.index[i];
                    return arr_to_hash(arr, elemsize);
                }
            } else if bucket.hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                return stbds_hmput_key_found_empty(
                    arr, raw_a, elemsize, key, keysize, table, hash, pos, tombstone, mode,
                );
            } else if tombstone < 0 && bucket.index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

unsafe fn stbds_hmput_key_found_empty(
    mut arr: *mut c_void,
    mut raw_a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    table: *mut stbds_hash_index,
    hash: usize,
    mut pos: usize,
    tombstone: isize,
    mode: c_int,
) -> *mut c_void {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr);
    if (i as usize) + 1 > stbds_arrcap(arr) {
        arr = stbds_arrgrowf(arr, elemsize, 1, 0);
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
            let dup = stbds_strdup_impl(key as *const c_char);
            let slot = (arr as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char;
            *slot = dup;
            *stbds_temp_key(arr) = dup;
        }
        STBDS_SH_ARENA => {
            let s =
                stbds_stralloc(&mut (*table).string, key as *mut c_char);
            let slot = (arr as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char;
            *slot = s;
            *stbds_temp_key(arr) = s;
        }
        STBDS_SH_DEFAULT => {
            let slot = (arr as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char;
            *slot = key as *mut c_char;
            *stbds_temp_key(arr) = key as *mut c_char;
        }
        _ => {
            libc::memcpy(
                (arr as *mut u8).offset(elemsize as isize * i) as *mut c_void,
                key,
                keysize,
            );
        }
    }

    arr_to_hash(arr, elemsize)
}

// ── stbds_shmode_func ─────────────────────────────────────────────────
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

// ── stbds_hmdel_key ───────────────────────────────────────────────────
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
        let p = *((a as *mut u8).offset(elemsize as isize * old_index) as *mut *mut c_void);
        c_free(p);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).offset(elemsize as isize * old_index) as *mut c_void,
            (a as *mut u8).offset(elemsize as isize * final_index) as *const c_void,
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(
                a,
                elemsize,
                *((a as *mut u8).offset(elemsize as isize * old_index + keyoffset as isize)
                    as *mut *mut c_void),
                keysize,
                keyoffset,
                mode,
            )
        } else {
            stbds_hm_find_slot(
                a,
                elemsize,
                (a as *mut u8).offset(elemsize as isize * old_index + keyoffset as isize)
                    as *mut c_void,
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
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        c_free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        c_free(table as *mut c_void);
    }

    a
}

// ── stbds_stralloc ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut c_char,
) -> *mut c_char {
    let len = libc::strlen(str) + 1;
    if len > (*a).remaining {
        let blocksize_shift = (*a).block >> 1;
        let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_shift as usize);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
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
            let sb = c_realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage)
        .storage
        .as_mut_ptr()
        .add((*a).remaining - len);
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ── stbds_strreset ────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        c_free(x as *mut c_void);
        x = y;
    }
    std::ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
}

// ── strkey helper ─────────────────────────────────────────────────────
unsafe fn strkey(n: c_int) -> *mut c_char {
    libc::sprintf(
        buffer.as_mut_ptr() as *mut c_char,
        b"test_%d\0".as_ptr() as *const c_char,
        n,
    );
    buffer.as_mut_ptr() as *mut c_char
}

// ── str_dups ──────────────────────────────────────────────────────────
// The struct used in str_dups: { char *key; int value; }
// On 64-bit with natural alignment this is 16 bytes (8 ptr + 4 int + 4 pad).
#[repr(C)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(num: c_int) {
    let mut strmap: *mut c_void = ptr::null_mut();
    let mut sa: stbds_string_arena = std::mem::zeroed();
    let elemsize = std::mem::size_of::<StrMapEntry>();

    // stralloc loop
    for i in 0..num {
        stbds_stralloc(&mut sa, strkey(i));
    }
    stbds_strreset(&mut sa);

    {
        // sh_new_strdup(strmap) => strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP)
        strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP as c_int);

        // shputs(strmap, s) where s = { key="a", value=num }
        // shputs expands to:
        //   strmap = stbds_hmput_key_wrapper(strmap, elemsize, (void*) s.key, sizeof(s.key), STBDS_HM_STRING)
        //   strmap[temp].key = stbds_temp_key(strmap-1)   (the strdup'd key)
        //   strmap[temp] = s   ... then strmap[temp].key = stbds_temp_key
        // Actually shputs does:
        //   (t) = hmput_key_wrapper(t, sizeof*(t), (void*)(s).key, sizeof (s).key, STBDS_HM_STRING),
        //   (t)[temp] = (s),
        //   (t)[temp].key = stbds_temp_key((t)-1)
        let s_key = b"a\0".as_ptr() as *mut c_char;
        let s_value = num;

        // sizeof (s).key = sizeof(char*) = 8
        strmap = stbds_hmput_key(
            strmap,
            elemsize,
            s_key as *mut c_void,
            std::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        );
        let arr = hash_to_arr(strmap, elemsize);
        let idx = *stbds_temp(arr);
        let entry = (strmap as *mut u8).offset(elemsize as isize * idx) as *mut StrMapEntry;
        // (t)[temp] = s
        (*entry).key = s_key;
        (*entry).value = s_value;
        // (t)[temp].key = stbds_temp_key((t)-1)
        (*entry).key = *stbds_temp_key(arr);

        // Assertions
        let entry0 = strmap as *const StrMapEntry;
        assert!(*(*entry0).key == b'a' as c_char);
        assert!((*entry0).key != s_key);
        assert!((*entry0).value == s_value);

        // for loop with printf bug:
        // printf("%s %d\n", strmap[z], strmap[z].value)
        // In C, passing a struct as vararg: the first field (char* key) ends up as the %s argument,
        // and the second field (int value) ends up as the %d argument on x86_64 ABI.
        // So this effectively prints: printf("%s %d\n", strmap[z].key, strmap[z].value)
        let len = {
            let arr2 = hash_to_arr(strmap, elemsize);
            let hdr = stbds_header(arr2);
            if (*hdr).length > 1 {
                (*hdr).length as isize - 1
            } else {
                0
            }
        };
        for z in 0..len {
            let e = (strmap as *const u8).offset(elemsize as isize * z) as *const StrMapEntry;
            // Reproduce the C bug: printf("%s %d\n", strmap[z], strmap[z].value)
            // On x86_64, passing the struct puts key in rsi (%s) and value in edx (%d)
            // which is identical to passing key, value separately.
            libc::printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                (*e).key,
                (*e).value,
            );
        }

        // shfree(strmap) => hmfree(strmap)
        // hmfree: stbds_hmfree_func((p)-1, sizeof*(p)), p=NULL
        if !strmap.is_null() {
            stbds_hmfree_func(
                hash_to_arr(strmap, elemsize),
                elemsize,
            );
            strmap = ptr::null_mut();
        }
    }
}
