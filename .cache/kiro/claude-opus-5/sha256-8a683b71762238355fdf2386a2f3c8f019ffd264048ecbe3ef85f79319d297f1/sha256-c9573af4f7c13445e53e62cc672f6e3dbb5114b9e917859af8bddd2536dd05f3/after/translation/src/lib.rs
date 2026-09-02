//! Rust translation of the C library in `c_src/` (stb_ds.h single-header data
//! structures library by Sean Barrett, plus the `strkey` / `arr_ins` helpers).
//!
//! The translation is intentionally literal: it reproduces the exact memory
//! layout, allocation strategy, evaluation order, integer wrap-around and even
//! the implementation-defined sign-extension quirks of the original C so that
//! the resulting shared object is ABI- and behaviour-compatible.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use core::ptr;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free directly through STBDS_REALLOC /
// STBDS_FREE, so we must use the very same allocator).
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Small C runtime helpers, implemented so that zero-length operations are safe.
// ---------------------------------------------------------------------------

#[inline]
unsafe fn c_memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        ptr::copy(src, dst, n);
    }
}

#[inline]
unsafe fn c_memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        ptr::copy(src, dst, n);
    }
}

#[inline]
unsafe fn c_memset0(dst: *mut u8, n: usize) {
    if n != 0 {
        ptr::write_bytes(dst, 0, n);
    }
}

#[inline]
unsafe fn c_memcmp_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    let mut i = 0usize;
    while i < n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
        i += 1;
    }
    true
}

#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

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

// ---------------------------------------------------------------------------
// Data structures (layout-identical to the C originals)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

const STBDS_HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>(); // 32

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

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

// ---------------------------------------------------------------------------
// Array header access helpers
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    // `((stbds_array_header *) (t) - 1)`; use wrapping so that the (bogus but
    // faithfully reproduced) `stbds_arrfreef(NULL)` case does not trip UB
    // checks before it reaches `free`.
    (t as *mut stbds_array_header).wrapping_sub(1)
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
unsafe fn stbds_set_temp(a: *mut u8, v: isize) {
    (*stbds_header(a)).temp = v;
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_set_temp_key(a: *mut u8, v: *mut c_char) {
    let table = stbds_hash_table(a);
    (*table).temp_key = v;
}

#[inline]
unsafe fn STBDS_HASH_TO_ARR(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.wrapping_sub(elemsize)
}

#[inline]
unsafe fn STBDS_ARR_TO_HASH(x: *mut u8, elemsize: usize) -> *mut u8 {
    x.wrapping_add(elemsize)
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

    if min_cap < 2usize.wrapping_mul(stbds_arrcap(a)) {
        min_cap = 2usize.wrapping_mul(stbds_arrcap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old: *mut c_void = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };

    let b = realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(STBDS_HEADER_SIZE),
    );
    let b = (b as *mut u8).wrapping_add(STBDS_HEADER_SIZE);

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
// Hash seed / index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    *(&raw mut stbds_hash_seed) = seed;
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
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage =
        STBDS_ALIGN_FWD(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
        c_memset0(
            ptr::addr_of_mut!((*t).string) as *mut u8,
            core::mem::size_of::<stbds_string_arena>(),
        );
        let seed_p = &raw mut stbds_hash_seed;
        (*t).seed = *seed_p;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
        // stbds_load_32_or_64(b, temp,  715136305,          0, 0xb504f32d)
        let a: usize = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b: usize = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        *seed_p = (*seed_p).wrapping_mul(a).wrapping_add(b);
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
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
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

/// Faithful expansion of the `stbds_load_32_or_64` macro for 64-bit `size_t`.
///
/// ```c
/// temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16,
/// var = v64_hi, var <<= 16, var <<= 16,
/// var ^= temp ^ v32
/// ```
///
/// `v64_lo` is an `unsigned int` literal and `v32` an `int` literal, so
/// `v64_lo ^ v32` has type `unsigned int` and is zero-extended into `size_t`.
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
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

// ---------------------------------------------------------------------------
// Hash functions
// ---------------------------------------------------------------------------

#[inline]
fn STBDS_ROTATE_LEFT(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn STBDS_ROTATE_RIGHT(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str;
    while *s != 0 {
        hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as u8 as usize);
        s = s.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= STBDS_ROTATE_RIGHT(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! stbds_sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = STBDS_ROTATE_LEFT($v1, 13);
        $v1 ^= $v0;
        $v0 = STBDS_ROTATE_LEFT($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = STBDS_ROTATE_LEFT($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = STBDS_ROTATE_LEFT($v1, 17);
        $v1 ^= $v2;
        $v2 = STBDS_ROTATE_LEFT($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = STBDS_ROTATE_LEFT($v3, 21);
        $v3 ^= $v0;
    }};
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;

    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;

    v0 = ((((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = ((((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = ((((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = ((((0x74656462usize) << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let elem = core::mem::size_of::<usize>();

    let mut i = 0usize;
    let mut data: usize;
    while i + elem <= len {
        // `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` is computed in
        // `int` and then converted to `size_t`; when `d[3] >= 0x80` the `int`
        // value is negative and the conversion sign-extends.  Reproduced.
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
            stbds_sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;

        i += elem;
        d = d.add(elem);
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) with fall-through from case 7 down to case 1
    let rem = len - i;
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
        // `(d[3] << 24)` is an `int`: sign-extends when d[3] >= 0x80.
        data |= ((*d.add(3) as i32) << 24) as isize as usize;
    }
    if rem >= 3 {
        data |= ((*d.add(2) as i32) << 16) as isize as usize;
    }
    if rem >= 2 {
        data |= ((*d.add(1) as i32) << 8) as isize as usize;
    }
    if rem >= 1 {
        data |= (*d.add(0) as i32) as isize as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        stbds_sipround!(v0, v1, v2, v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        stbds_sipround!(v0, v1, v2, v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// Hash map internals
// ---------------------------------------------------------------------------

unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        c_strcmp_eq(
            key as *const c_char,
            *(a.add(elemsize.wrapping_mul(i)).add(keyoffset) as *mut *mut c_char),
        )
    } else {
        c_memcmp_eq(
            key as *const u8,
            a.add(elemsize.wrapping_mul(i)).add(keyoffset),
            keysize,
        )
    }
}

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
                free(*(a.add(elemsize.wrapping_mul(i)) as *mut *mut c_void));
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));
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
    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
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
                ) {
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
                ) {
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
        c_memset0(a, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        STBDS_ARR_TO_HASH(a, elemsize) as *mut c_void
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
                let b = (*table)
                    .storage
                    .add((slot >> STBDS_BUCKET_SHIFT) as usize);
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
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    stbds_set_temp(STBDS_HASH_TO_ARR(p as *mut u8, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a as *mut u8;
    if a.is_null() || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0 {
        let base = if !a.is_null() {
            STBDS_HASH_TO_ARR(a, elemsize)
        } else {
            ptr::null_mut()
        };
        a = stbds_arrgrowf(base as *mut c_void, elemsize, 0, 1) as *mut u8;
        (*stbds_header(a)).length += 1;
        c_memset0(a, elemsize);
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }
    a as *mut c_void
}

unsafe fn stbds_strdup(str: *mut c_char) -> *mut c_char {
    let len = c_strlen(str) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    c_memmove(p as *mut u8, str as *const u8, len);
    p
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
    let mut raw_a: *mut u8;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        c_memset0(a, elemsize);
        (*stbds_header(a)).length += 1;
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }

    raw_a = a;
    a = STBDS_HASH_TO_ARR(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

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
                        (*bucket).index[i] as usize,
                    ) {
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let kp = *(raw_a
                                .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                .add(keyoffset) as *mut *mut c_char);
                            stbds_set_temp_key(a, kp);
                        }
                        return STBDS_ARR_TO_HASH(a, elemsize) as *mut c_void;
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
                        (*bucket).index[i] as usize,
                    ) {
                        stbds_set_temp(a, (*bucket).index[i]);
                        return STBDS_ARR_TO_HASH(a, elemsize) as *mut c_void;
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
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        {
            let i: isize = stbds_arrlen(a);
            if (i as usize) + 1 > stbds_arrcap(a) {
                a = stbds_arrgrowf(a as *mut c_void, elemsize, 1, 0) as *mut u8;
            }
            raw_a = STBDS_ARR_TO_HASH(a, elemsize);
            let _ = raw_a;

            assert!((i as usize) + 1 <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_set_temp(a, i - 1);

            let slot = a.add(elemsize.wrapping_mul(i as usize));
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    *(slot as *mut *mut c_char) = v;
                    stbds_set_temp_key(a, v);
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *(slot as *mut *mut c_char) = v;
                    stbds_set_temp_key(a, v);
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    *(slot as *mut *mut c_char) = v;
                    stbds_set_temp_key(a, v);
                }
                _ => {
                    c_memcpy(slot, key as *const u8, keysize);
                }
            }
        }
        STBDS_ARR_TO_HASH(a, elemsize) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
    c_memset0(a, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    STBDS_ARR_TO_HASH(a, elemsize) as *mut c_void
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
    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    stbds_set_temp(raw_a, 0);
    if table.is_null() {
        return a as *mut c_void;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a as *mut c_void;
    }

    let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i: usize = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    stbds_set_temp(raw_a, 1);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(*(a.add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_void));
    }

    if old_index != final_index {
        c_memmove(
            a.add(elemsize.wrapping_mul(old_index as usize)),
            a.add(elemsize.wrapping_mul(final_index as usize)),
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
        assert!(slot >= 0);
        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        assert!((*b).index[i] == final_index);
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

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = c_strlen(str) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        // `(size_t) 512 << (blocksize>>1)`: `a->block` is a public `unsigned
        // char` field, so the shift count can reach 127.  gcc emits `shlq %cl`,
        // which masks the count to 6 bits on x86-64; `wrapping_shl` reproduces
        // that exactly (a plain `<<` is a Rust arithmetic-overflow / LLVM poison).
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            c_memmove(
                ptr::addr_of_mut!((*sb).storage) as *mut u8,
                str as *const u8,
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
            return ptr::addr_of_mut!((*sb).storage) as *mut c_char;
        } else {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .add((*a).remaining)
        .sub(len);
    (*a).remaining -= len;
    c_memmove(p as *mut u8, str as *const u8, len);
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
    c_memset0(a as *mut u8, core::mem::size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// Test helpers exported by the C library
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    // sprintf(buffer, "test_%d", n)
    let buf = (&raw mut buffer) as *mut u8;
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    c_memcpy(buf, bytes.as_ptr(), bytes.len());
    *buf.add(bytes.len()) = 0;
    buf as *mut c_char
}

// --- dynamic-array macro helpers specialised for `int *`, as used by arr_ins

#[inline]
unsafe fn arr_maybegrow_int(arr: &mut *mut c_int, n: usize) {
    let a = *arr as *mut u8;
    if a.is_null() || (*stbds_header(a)).length + n > (*stbds_header(a)).capacity {
        *arr = stbds_arrgrowf(
            *arr as *mut c_void,
            core::mem::size_of::<c_int>(),
            n,
            0,
        ) as *mut c_int;
    }
}

#[inline]
unsafe fn arrput_int(arr: &mut *mut c_int, v: c_int) {
    arr_maybegrow_int(arr, 1);
    let h = stbds_header(*arr as *mut u8);
    let idx = (*h).length;
    (*h).length = idx + 1;
    *(*arr).add(idx) = v;
}

/// `stbds_arraddn(a,n)` -> `(void) stbds_arraddnindex(a,n)`
#[inline]
unsafe fn arraddn_int(arr: &mut *mut c_int, n: usize) {
    arr_maybegrow_int(arr, n);
    if n != 0 {
        let h = stbds_header(*arr as *mut u8);
        (*h).length += n;
    }
}

/// `stbds_arrinsn(a,i,n)`
#[inline]
unsafe fn arrinsn_int(arr: &mut *mut c_int, i: usize, n: usize) {
    arraddn_int(arr, n);
    let a = *arr;
    let len = (*stbds_header(a as *mut u8)).length;
    c_memmove(
        a.add(i + n) as *mut u8,
        a.add(i) as *const u8,
        core::mem::size_of::<c_int>() * (len - n - i),
    );
}

#[inline]
unsafe fn arrfree_int(arr: &mut *mut c_int) {
    let a = *arr as *mut u8;
    if !a.is_null() {
        free(stbds_header(a) as *mut c_void);
    }
    *arr = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();

    for i in 0..5usize {
        arrput_int(&mut arr, 1);
        arrput_int(&mut arr, 2);
        arrput_int(&mut arr, 3);
        arrput_int(&mut arr, 4);
        // stbds_arrins(arr, i, num)
        arrinsn_int(&mut arr, i, 1);
        *arr.add(i) = num;
        assert!(*arr.add(i) == num);
        if i < 4 {
            assert!(*arr.add(4) == 4);
        }
        arrfree_int(&mut arr);
    }
}
