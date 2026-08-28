//! Rust translation of `c_src/src/lib.c` (an embedded copy of `stb_ds.h`
//! plus the small `strkey`/`arr_push` driver helpers).
//!
//! The translation is deliberately literal: memory layouts, allocation
//! strategy (libc `realloc`/`free`), pointer arithmetic, iteration order and
//! integer wrap-around behaviour all mirror the original C, including the
//! places where the C relies on implementation-defined signed overflow /
//! sign-extension (see `siphash_bytes`).

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// libc allocation (STBDS_REALLOC / STBDS_FREE)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// `STBDS_ASSERT` == `assert`: abort when the condition does not hold.
#[inline]
fn stbds_assert(cond: bool) {
    if !cond {
        std::process::abort();
    }
}

// ---------------------------------------------------------------------------
// Types
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

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const HEADER_SIZE: usize = size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Array header helpers
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
#[allow(dead_code)]
unsafe fn stbds_temp_get(t: *mut u8) -> isize {
    (*stbds_header(t)).temp
}

#[inline]
unsafe fn stbds_temp_set(t: *mut u8, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut u8, v: *mut c_char) {
    let ht = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *ht = v;
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
unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline]
unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline]
unsafe fn hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.sub(elemsize)
}

#[inline]
unsafe fn arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.add(elemsize)
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

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

    if min_cap < stbds_arrcap(a).wrapping_mul(2) {
        min_cap = stbds_arrcap(a).wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };
    let mut b = realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    ) as *mut u8;
    b = b.add(HEADER_SIZE);
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a as *mut u8) as *mut c_void);
}

// ---------------------------------------------------------------------------
// Seed / hash index construction
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[inline]
unsafe fn hash_seed_get() -> usize {
    *(&raw const STBDS_HASH_SEED)
}

#[inline]
unsafe fn hash_seed_set(v: usize) {
    *(&raw mut STBDS_HASH_SEED) = v;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    hash_seed_set(seed);
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // temp = v64_lo ^ v32 (computed in 32 bits in C), then <<16 <<16 >>16 >>16
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ (v32 as usize);
    var
}

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(size_of::<stbds_hash_bucket>())
            .wrapping_add(size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage =
        stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
    stbds_assert(
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        (*t).string = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*t).seed = hash_seed_get();
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));
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
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'probe;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        let mut placed = false;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                placed = true;
                                break;
                            }
                            z += 1;
                        }
                        if placed {
                            break 'probe;
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
// Hashing
// ---------------------------------------------------------------------------

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut str_ = str_;
    while *str_ != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*str_ as u8 as usize);
        str_ = str_.add(1);
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

    let mut v0: usize = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! siproundfn {
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

    let mut data: usize;
    let mut i = 0usize;
    while i + size_of::<usize>() <= len {
        // Reproduce the C expression, including the signed-int overflow on
        // `d[3] << 24` / `d[7] << 24` and the sign-extension to size_t.
        let lo: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32) << 24);
        data = lo as isize as usize;
        let hi: i32 = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | ((*d.add(7) as i32) << 24);
        data |= ((hi as isize as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siproundfn!();
        }
        v0 ^= data;

        i += size_of::<usize>();
        d = d.add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // switch (len - i) with fall-through from case 7 down to case 1.
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
        data |= (((*d.add(3) as i32) << 24) as isize) as usize;
    }
    if rem >= 3 {
        data |= (((*d.add(2) as i32) << 16) as isize) as usize;
    }
    if rem >= 2 {
        data |= (((*d.add(1) as i32) << 8) as isize) as usize;
    }
    if rem >= 1 {
        data |= (*d.add(0) as i32) as isize as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siproundfn!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siproundfn!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// Key comparison
// ---------------------------------------------------------------------------

#[inline]
unsafe fn c_strcmp_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut i = 0usize;
    loop {
        let ca = *a.add(i) as u8;
        let cb = *b.add(i) as u8;
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        i += 1;
    }
}

#[inline]
unsafe fn c_memcmp_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    for i in 0..n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
    }
    true
}

unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> c_int {
    let slot = a.add(elemsize.wrapping_mul(i)).add(keyoffset);
    if mode >= STBDS_HM_STRING {
        let stored = *(slot as *mut *mut c_char);
        (c_strcmp_eq(key as *const c_char, stored)) as c_int
    } else {
        (c_memcmp_eq(key as *const u8, slot as *const u8, keysize)) as c_int
    }
}

// ---------------------------------------------------------------------------
// Hash map internals
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    let a = a as *mut u8;
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i = 1usize;
            while i < (*stbds_header(a)).length {
                free(*(a.add(elemsize.wrapping_mul(i)) as *mut *mut c_char) as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

unsafe fn stbds_hm_find_slot(
    a: *mut u8,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
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
                    (*bucket).index[i] as usize,
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
                    (*bucket).index[i] as usize,
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
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
    let a = a as *mut u8;
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize) as *mut c_void
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
                let b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a as *mut c_void
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
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode) as *mut u8;
    stbds_temp_set(hash_to_arr(p, elemsize), temp);
    p as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a as *mut u8;
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let base = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize) as *mut c_void
        };
        a = stbds_arrgrowf(base, elemsize, 0, 1) as *mut u8;
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;

    let mut a = a as *mut u8;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        ptr::write_bytes(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
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
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                STBDS_SH_NONE
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = table as *mut c_void;
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

        'found_empty_slot: loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        stbds_temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let stored = *(raw_a
                                .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                .add(keyoffset)
                                as *mut *mut c_char);
                            stbds_temp_key_set(a, stored);
                        }
                        return arr_to_hash(a, elemsize) as *mut c_void;
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
            let mut found_empty = false;
            while i < limit {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        stbds_temp_set(a, (*bucket).index[i]);
                        return arr_to_hash(a, elemsize) as *mut c_void;
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    found_empty = true;
                    break;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
                i += 1;
            }
            if found_empty {
                break 'found_empty_slot;
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        // found_empty_slot:
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        {
            let i: isize = stbds_arrlen(a);
            if (i as usize) + 1 > stbds_arrcap(a) {
                a = stbds_arrgrowf(a as *mut c_void, elemsize, 1, 0) as *mut u8;
            }
            raw_a = arr_to_hash(a, elemsize);

            stbds_assert((i as usize) + 1 <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_temp_set(a, i - 1);

            let dest = a.add(elemsize.wrapping_mul(i as usize));
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *(dest as *mut *mut c_char) = p;
                    stbds_temp_key_set(a, p);
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                    *(dest as *mut *mut c_char) = p;
                    stbds_temp_key_set(a, p);
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *(dest as *mut *mut c_char) = p;
                    stbds_temp_key_set(a, p);
                }
                _ => {
                    ptr::copy_nonoverlapping(key as *const u8, dest, keysize);
                }
            }
            let _ = raw_a;
        }
        arr_to_hash(a, elemsize) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize) as *mut c_void
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
    let a = a as *mut u8;
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    stbds_temp_set(raw_a, 0);
    if table.is_null() {
        return a as *mut c_void;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a as *mut c_void;
    }

    let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    stbds_assert(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    stbds_temp_set(raw_a, 1);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(*(a.add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char) as *mut c_void);
    }

    if old_index != final_index {
        ptr::copy(
            a.add(elemsize.wrapping_mul(final_index as usize)),
            a.add(elemsize.wrapping_mul(old_index as usize)),
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let k = *(a
                .add(elemsize.wrapping_mul(old_index as usize))
                .add(keyoffset) as *mut *mut c_char);
            slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
        } else {
            let k = a
                .add(elemsize.wrapping_mul(old_index as usize))
                .add(keyoffset);
            slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
        }
        stbds_assert(slot >= 0);
        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        stbds_assert((*b).index[i] == final_index);
        (*b).index[i] = old_index;
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

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c_strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
    p
}

#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// Address of the flexible `storage` member of a `stbds_string_block`.
#[inline]
unsafe fn sb_storage(sb: *mut stbds_string_block) -> *mut c_char {
    (&raw mut (*sb).storage) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let len = c_strlen(str_) + 1;
    if len > (*a).remaining {
        let blocksize = (*a).block as usize;

        let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            ptr::copy(str_ as *const u8, sb_storage(sb) as *mut u8, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return sb_storage(sb);
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

    stbds_assert(len <= (*a).remaining);
    let p = sb_storage((*a).storage).add((*a).remaining).sub(len);
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
// Driver helpers
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = (&raw mut BUFFER) as *mut c_char;
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
    *buf.add(bytes.len()) = 0;
    buf
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();

    stbds_assert(stbds_arrlen(arr as *mut u8) == 0);

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            // stbds_arrput(arr, j)
            let need_grow = arr.is_null()
                || (*stbds_header(arr as *mut u8)).length + 1
                    > (*stbds_header(arr as *mut u8)).capacity;
            if need_grow {
                arr = stbds_arrgrowf(arr as *mut c_void, size_of::<c_int>(), 1, 0) as *mut c_int;
            }
            let h = stbds_header(arr as *mut u8);
            *arr.add((*h).length) = j;
            (*h).length += 1;
            j += 1;
        }
        // stbds_arrfree(arr)
        if !arr.is_null() {
            free(stbds_header(arr as *mut u8) as *mut c_void);
        }
        arr = ptr::null_mut();

        i = i.wrapping_add(50);
    }
}
