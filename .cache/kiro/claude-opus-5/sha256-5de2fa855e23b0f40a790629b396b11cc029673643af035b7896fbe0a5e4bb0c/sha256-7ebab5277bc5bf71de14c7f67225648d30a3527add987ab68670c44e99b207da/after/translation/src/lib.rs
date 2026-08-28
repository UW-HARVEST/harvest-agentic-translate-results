//! Rust translation of c_src/src/lib.c (a vendored copy of stb_ds.h plus the
//! `strkey` / `arr_del` helpers).
//!
//! The translation is deliberately literal: it reproduces the original pointer
//! arithmetic, the C integer promotion/sign-extension quirks and the exact
//! order of operations so that the resulting shared object behaves
//! byte-for-byte like the C build.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// ---------------------------------------------------------------------------
// helpers replacing the C library string/memory routines
// ---------------------------------------------------------------------------

#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

#[inline]
unsafe fn c_strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0usize;
    loop {
        let ca = *(a.add(i) as *const u8);
        let cb = *(b.add(i) as *const u8);
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

#[inline]
unsafe fn c_memcmp(a: *const u8, b: *const u8, len: usize) -> c_int {
    let mut i = 0usize;
    while i < len {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// bit helpers (STBDS_SIZE_T_BITS / rotates)
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ---------------------------------------------------------------------------
// data structures
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

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

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

#[inline]
fn STBDS_INDEX_IN_USE(x: isize) -> bool {
    x >= 0
}

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

// ---------------------------------------------------------------------------
// array header accessors (stbds_header / stbds_arrlen / stbds_arrcap / ...)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline]
unsafe fn stbds_set_temp(a: *mut c_void, v: isize) {
    (*stbds_header(a)).temp = v;
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_set_temp_key(a: *mut c_void, v: *mut c_char) {
    *((*stbds_header(a)).hash_table as *mut *mut c_char) = v;
}

#[inline]
unsafe fn stbds_hash_table_of(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline]
unsafe fn STBDS_HASH_TO_ARR(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn STBDS_ARR_TO_HASH(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < stbds_arrcap(a).wrapping_mul(2) {
        min_cap = stbds_arrcap(a).wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };

    let mut b = realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),
    );
    b = (b as *mut u8).add(size_of::<stbds_array_header>()) as *mut c_void;

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

// ---------------------------------------------------------------------------
// hash seed / hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline]
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

/// Faithful translation of `stbds_load_32_or_64`.
#[inline]
fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp: usize = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ v32;
    var
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
            + size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1,
    ) as *mut stbds_hash_index;

    (*t).storage = STBDS_ALIGN_FWD(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(
            ptr::addr_of_mut!((*t).string) as *mut u8,
            0,
            size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i = 0usize;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let mut i = 0usize;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if STBDS_INDEX_IN_USE((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
            i += 1;
        }
    }

    t
}

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
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

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut data: usize;

    let mut v0: usize = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= (0x0706050403020100u64 as usize) ^ seed;
    v1 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;
    v2 ^= (0x0706050403020100u64 as usize) ^ seed;
    v3 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;

    macro_rules! siproundptr {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = rotate_left(v1, 13);
            v1 ^= v0;
            v0 = rotate_left(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotate_left(v1, 17);
            v1 ^= v2;
            v2 = rotate_left(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = rotate_left(v3, 21);
            v3 ^= v0;
        }};
    }

    let mut i = 0usize;
    while i + size_of::<usize>() <= len {
        // Reproduces the C expression, including the `int` overflow /
        // sign-extension behaviour of `d[3] << 24`.
        let lo: c_int = (*d.add(0) as c_int)
            | ((*d.add(1) as c_int) << 8)
            | ((*d.add(2) as c_int) << 16)
            | ((*d.add(3) as c_int) << 24);
        data = lo as isize as usize;

        let hi: c_int = (*d.add(4) as c_int)
            | ((*d.add(5) as c_int) << 8)
            | ((*d.add(6) as c_int) << 16)
            | ((*d.add(7) as c_int) << 24);
        data |= ((hi as isize as usize) << 16) << 16; // discarded if size_t == 4

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siproundptr!();
        }
        v0 ^= data;

        i += size_of::<usize>();
        d = d.add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // C switch with fallthrough from `len - i` down to 0.
    if rem >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        data |= (((*d.add(3) as c_int) << 24) as isize) as usize;
    }
    if rem >= 3 {
        data |= (((*d.add(2) as c_int) << 16) as isize) as usize;
    }
    if rem >= 2 {
        data |= (((*d.add(1) as c_int) << 8) as isize) as usize;
    }
    if rem >= 1 {
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siproundptr!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siproundptr!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// hash map internals
// ---------------------------------------------------------------------------

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> c_int {
    let slot = (a as *mut u8)
        .offset((elemsize as isize).wrapping_mul(i))
        .add(keyoffset);
    if mode >= STBDS_HM_STRING {
        (c_strcmp(key as *const c_char, *(slot as *mut *mut c_char)) == 0) as c_int
    } else {
        (c_memcmp(key as *const u8, slot, keysize) == 0) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table_of(a).is_null() {
        if (*stbds_hash_table_of(a)).string.mode == STBDS_SH_STRDUP {
            let mut i = 1usize;
            while i < (*stbds_header(a)).length {
                free(*((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_char) as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table_of(a)).string));
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
    let table = stbds_hash_table_of(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i],
                ) != 0
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        let limit = pos & STBDS_BUCKET_MASK;
        let mut i = 0usize;
        while i < limit {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i],
                ) != 0
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        STBDS_ARR_TO_HASH(a, elemsize)
    } else {
        let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

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
    stbds_set_temp(STBDS_HASH_TO_ARR(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0 {
        let old = if !a.is_null() {
            STBDS_HASH_TO_ARR(a, elemsize)
        } else {
            ptr::null_mut()
        };
        a = stbds_arrgrowf(old, elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c_strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }

    raw_a = a;
    a = STBDS_HASH_TO_ARR(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                STBDS_SH_NONE
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = nt as *mut c_void;
    }

    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut pos: usize;
        let mut tombstone: isize = -1;
        let mut bucket: *mut stbds_hash_bucket;

        if hash < 2 {
            hash += 2;
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'found_empty_slot: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let mut i = pos & STBDS_BUCKET_MASK;
            while i < STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i],
                    ) != 0
                    {
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let v = *((raw_a as *mut u8)
                                .offset((elemsize as isize).wrapping_mul((*bucket).index[i]))
                                .add(keyoffset) as *mut *mut c_char);
                            stbds_set_temp_key(a, v);
                        }
                        return STBDS_ARR_TO_HASH(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
                i += 1;
            }

            let limit = pos & STBDS_BUCKET_MASK;
            let mut i = 0usize;
            while i < limit {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i],
                    ) != 0
                    {
                        stbds_set_temp(a, (*bucket).index[i]);
                        return STBDS_ARR_TO_HASH(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
                i += 1;
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        // found_empty_slot:
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
        }
        (*table).used_count = (*table).used_count.wrapping_add(1);

        {
            let i: isize = stbds_arrlen(a);
            if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = STBDS_ARR_TO_HASH(a, elemsize);

            assert!((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = i.wrapping_add(1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i.wrapping_sub(1);
            stbds_set_temp(a, i.wrapping_sub(1));

            let slot = (a as *mut u8).wrapping_offset((elemsize as isize).wrapping_mul(i));
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *(slot as *mut *mut c_char) = p;
                    stbds_set_temp_key(a, p);
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *(slot as *mut *mut c_char) = p;
                    stbds_set_temp_key(a, p);
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *(slot as *mut *mut c_char) = p;
                    stbds_set_temp_key(a, p);
                }
                _ => {
                    ptr::copy_nonoverlapping(key as *const u8, slot, keysize);
                }
            }
        }
        STBDS_ARR_TO_HASH(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    STBDS_ARR_TO_HASH(a, elemsize)
}

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

    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    stbds_set_temp(raw_a, 0);
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
    let mut i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
    let old_index = (*b).index[i as usize];
    let final_index = stbds_arrlen(raw_a).wrapping_sub(1).wrapping_sub(1);
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count = (*table).used_count.wrapping_sub(1);
    (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
    stbds_set_temp(raw_a, 1);
    b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
    (*b).hash[i as usize] = STBDS_HASH_DELETED;
    (*b).index[i as usize] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(
            *((a as *mut u8).offset((elemsize as isize).wrapping_mul(old_index))
                as *mut *mut c_char) as *mut c_void,
        );
    }

    if old_index != final_index {
        ptr::copy(
            (a as *const u8).offset((elemsize as isize).wrapping_mul(final_index)),
            (a as *mut u8).offset((elemsize as isize).wrapping_mul(old_index)),
            elemsize,
        );

        let moved = (a as *mut u8)
            .offset((elemsize as isize).wrapping_mul(old_index))
            .add(keyoffset);
        slot = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(
                a,
                elemsize,
                *(moved as *mut *mut c_char) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            )
        } else {
            stbds_hm_find_slot(a, elemsize, moved as *mut c_void, keysize, keyoffset, mode)
        };
        assert!(slot >= 0);
        b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        assert!((*b).index[i as usize] == final_index);
        (*b).index[i as usize] = old_index;
    }
    (*stbds_header(raw_a)).length = (*stbds_header(raw_a)).length.wrapping_sub(1);

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

    a
}

// ---------------------------------------------------------------------------
// string arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = c_strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            let storage = ptr::addr_of_mut!((*sb).storage) as *mut c_char;
            ptr::copy(str_ as *const u8, storage as *mut u8, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return storage;
        } else {
            let sb = realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>() - 8 + blocksize,
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char).add((*a).remaining - len);
    (*a).remaining -= len;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
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
    ptr::write_bytes(a as *mut u8, 0, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// test helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    // sprintf(buffer, "test_%d", n)
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    let p = ptr::addr_of_mut!(buffer) as *mut c_char;
    ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
    *p.add(bytes.len()) = 0;
    p
}

/// `stbds_arrmaybegrow(a,n)` followed by nothing else.
#[inline]
unsafe fn arrmaybegrow_int(a: &mut *mut c_int, n: usize) {
    let p = *a as *mut c_void;
    if p.is_null() || (*stbds_header(p)).length.wrapping_add(n) > (*stbds_header(p)).capacity {
        *a = stbds_arrgrowf(p, size_of::<c_int>(), n, 0) as *mut c_int;
    }
}

/// `arrpush(a,v)`
#[inline]
unsafe fn arrpush_int(a: &mut *mut c_int, v: c_int) {
    arrmaybegrow_int(a, 1);
    let h = stbds_header(*a as *mut c_void);
    let len = (*h).length;
    *(*a).add(len) = v;
    (*h).length = len.wrapping_add(1);
}

/// `arrfree(a)`
#[inline]
unsafe fn arrfree_int(a: &mut *mut c_int) {
    if !(*a).is_null() {
        free(stbds_header(*a as *mut c_void) as *mut c_void);
    }
    *a = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();

    let mut i: c_int = 0;
    while i < 4 {
        arrpush_int(&mut arr, num);
        arrpush_int(&mut arr, 2);
        arrpush_int(&mut arr, 3);
        arrpush_int(&mut arr, 4);
        // arrdel(arr, i) == arrdeln(arr, i, 1)
        {
            let h = stbds_header(arr as *mut c_void);
            let n = 1usize;
            let count = (*h).length.wrapping_sub(n).wrapping_sub(i as usize);
            ptr::copy(
                arr.offset(i as isize + n as isize),
                arr.offset(i as isize),
                count,
            );
            (*h).length = (*h).length.wrapping_sub(n);
        }
        arrfree_int(&mut arr);

        arrpush_int(&mut arr, num);
        arrpush_int(&mut arr, 2);
        arrpush_int(&mut arr, 3);
        arrpush_int(&mut arr, 4);
        // arrdelswap(arr, i)
        {
            let h = stbds_header(arr as *mut c_void);
            let last = *arr.add((*h).length.wrapping_sub(1));
            *arr.offset(i as isize) = last;
            (*h).length = (*h).length.wrapping_sub(1);
        }
        arrfree_int(&mut arr);

        i += 1;
    }
}

#[cfg(test)]
mod layout_tests {
    //! Layout parity with the C structs (values from a C probe on this ABI).
    use super::*;

    #[test]
    fn struct_layout_matches_c() {
        assert_eq!(size_of::<stbds_array_header>(), 32);
        assert_eq!(size_of::<stbds_string_block>(), 16);
        assert_eq!(size_of::<stbds_string_arena>(), 24);
        assert_eq!(size_of::<stbds_hash_bucket>(), 128);
        assert_eq!(size_of::<stbds_hash_index>(), 104);
        let t: stbds_hash_index = unsafe { std::mem::zeroed() };
        let base = &t as *const _ as usize;
        assert_eq!(ptr::addr_of!(t.string) as usize - base, 72);
        assert_eq!(ptr::addr_of!(t.storage) as usize - base, 96);
    }
}
