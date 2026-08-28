//! Faithful Rust translation of `c_src/src/lib.c`.
//!
//! The C file is an amalgamation of Sean Barrett's `stb_ds.h` implementation
//! (dynamic arrays + hash maps) together with the `sh_geti` string-hash-map
//! exerciser exported through `include/lib.h`.
//!
//! The translation keeps the original memory layout, allocation strategy
//! (libc `realloc`/`free`), pointer arithmetic and *all* of the original
//! quirks (signed-overflow driven sign extension inside the siphash byte
//! loader, the missing `temp_key` store in the wrapped-around probe loop of
//! `stbds_hmput_key`, the leaked array in `sh_geti`, ...) so that the produced
//! output is byte identical.
//!
//! `printf`/`sprintf` from libc are used directly so that stdio buffering and
//! integer formatting match the C build exactly.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use core::mem::size_of;
use core::ptr;
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// #define STBDS_REALLOC(c,p,s) realloc(p,s)
// #define STBDS_FREE(c,p)      free(p)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn STBDS_REALLOC(p: *mut c_void, s: usize) -> *mut c_void {
    unsafe { realloc(p, s) }
}

#[inline]
unsafe fn STBDS_FREE(p: *mut c_void) {
    unsafe { free(p) }
}

/// `STBDS_ASSERT` maps to `assert` in the C source and `NDEBUG` is *not*
/// defined by the CMake build, so the checks are live.
macro_rules! STBDS_ASSERT {
    ($cond:expr) => {
        assert!($cond)
    };
}

// ---------------------------------------------------------------------------
// Layout types
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
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Clone, Copy)]
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

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const HDR_SIZE: usize = size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Small C library helpers (kept local so no external crate is required)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn strlen(s: *const c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

#[inline]
unsafe fn strcmp_eq(a: *const c_char, b: *const c_char) -> bool {
    unsafe {
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
}

#[inline]
unsafe fn memcmp_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    unsafe {
        let mut i = 0usize;
        while i < n {
            if *a.add(i) != *b.add(i) {
                return false;
            }
            i += 1;
        }
        true
    }
}

#[inline]
unsafe fn memmove(dst: *mut u8, src: *const u8, n: usize) {
    unsafe { ptr::copy(src, dst, n) }
}

#[inline]
unsafe fn memset0(dst: *mut u8, n: usize) {
    unsafe { ptr::write_bytes(dst, 0, n) }
}

// ---------------------------------------------------------------------------
// Array header accessors (stbds_header / stbds_arrlen / ...)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).sub(1) }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    unsafe {
        if a.is_null() {
            0
        } else {
            (*stbds_header(a)).capacity
        }
    }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    unsafe {
        if a.is_null() {
            0
        } else {
            (*stbds_header(a)).length as isize
        }
    }
}

/// `stbds_temp(t)` -> `stbds_header(t)->temp`
#[inline]
unsafe fn stbds_temp_get(t: *mut c_void) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe {
        (*stbds_header(t)).temp = v;
    }
}

/// `stbds_temp_key(t)` -> `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    unsafe {
        *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
    }
}

/// `STBDS_HASH_TO_ARR(x,elemsize)`
#[inline]
unsafe fn STBDS_HASH_TO_ARR(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

/// `STBDS_ARR_TO_HASH(x,elemsize)`
#[inline]
unsafe fn STBDS_ARR_TO_HASH(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

/// `stbds_hash_table(a)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
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
    unsafe {
        let mut min_cap = min_cap;
        let b: *mut c_void;
        let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

        if min_len > min_cap {
            min_cap = min_len;
        }

        if min_cap <= stbds_arrcap(a) {
            return a;
        }

        if min_cap < 2usize.wrapping_mul(stbds_arrcap(a)) {
            min_cap = 2usize.wrapping_mul(stbds_arrcap(a));
        } else if min_cap < 4 {
            min_cap = 4;
        }

        let old = if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        };
        let raw = STBDS_REALLOC(old, elemsize.wrapping_mul(min_cap).wrapping_add(HDR_SIZE));
        b = (raw as *mut u8).add(HDR_SIZE) as *mut c_void;
        if a.is_null() {
            (*stbds_header(b)).length = 0;
            (*stbds_header(b)).hash_table = ptr::null_mut();
            (*stbds_header(b)).temp = 0;
        }
        (*stbds_header(b)).capacity = min_cap;

        b
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        STBDS_FREE(stbds_header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// hash seeding / hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        *(&raw mut stbds_hash_seed) = seed;
    }
}

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
    unsafe {
        let t = STBDS_REALLOC(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
                + size_of::<stbds_hash_index>()
                + STBDS_CACHE_LINE_SIZE
                - 1,
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
        STBDS_ASSERT!(
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
        );
        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            memset0(
                (&raw mut (*t).string) as *mut u8,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = *(&raw const stbds_hash_seed);

            // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let mut temp: usize;
            let a: usize;
            let b: usize;

            temp = (0x87b0b0fdu32 ^ 2147001325u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var: usize = 0x27bb2ee6usize;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ 2147001325usize;
            a = var;

            // stbds_load_32_or_64(b,temp,  715136305,          0, 0xb504f32d);
            temp = (0xb504f32du32 ^ 715136305u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var: usize = 0usize;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ 715136305usize;
            b = var;

            *(&raw mut stbds_hash_seed) = (*(&raw const stbds_hash_seed))
                .wrapping_mul(a)
                .wrapping_add(b);
        }

        {
            let mut i = 0usize;
            while i < slot_count >> STBDS_BUCKET_SHIFT {
                let b = (*t).storage.add(i);
                let mut j = 0usize;
                while j < STBDS_BUCKET_LENGTH {
                    (*b).hash[j] = STBDS_HASH_EMPTY;
                    j += 1;
                }
                let mut j = 0usize;
                while j < STBDS_BUCKET_LENGTH {
                    (*b).index[j] = STBDS_INDEX_EMPTY;
                    j += 1;
                }
                i += 1;
            }
        }

        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            let mut i = 0usize;
            while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
                let ob = (*ot).storage.add(i);
                let mut j = 0usize;
                while j < STBDS_BUCKET_LENGTH {
                    if STBDS_INDEX_IN_USE((*ob).index[j]) {
                        let hash = (*ob).hash[j];
                        let mut pos =
                            stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
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
                    j += 1;
                }
                i += 1;
            }
        }

        t
    }
}

// ---------------------------------------------------------------------------
// hashing
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
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut str_ = str_ as *const u8;
        while *str_ != 0 {
            hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*str_ as usize);
            str_ = str_.add(1);
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
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;
        let mut v0: usize;
        let mut v1: usize;
        let mut v2: usize;
        let mut v3: usize;
        let mut data: usize;

        v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
        v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
        v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
        v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        macro_rules! STBDS_SIPROUND {
            () => {{
                v0 = v0.wrapping_add(v1);
                v1 = STBDS_ROTATE_LEFT(v1, 13);
                v1 ^= v0;
                v0 = STBDS_ROTATE_LEFT(v0, STBDS_SIZE_T_BITS / 2);
                v2 = v2.wrapping_add(v3);
                v3 = STBDS_ROTATE_LEFT(v3, 16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = STBDS_ROTATE_LEFT(v1, 17);
                v1 ^= v2;
                v2 = STBDS_ROTATE_LEFT(v2, STBDS_SIZE_T_BITS / 2);
                v0 = v0.wrapping_add(v3);
                v3 = STBDS_ROTATE_LEFT(v3, 21);
                v3 ^= v0;
            }};
        }

        // The C code builds the 32-bit halves with `int` arithmetic, so a byte
        // >= 0x80 in the most significant position makes the intermediate
        // result negative and the conversion to size_t sign extends it. That
        // behaviour is part of the hash and is reproduced here.
        let mut i = 0usize;
        while i + size_of::<usize>() <= len {
            let lo = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | ((*d.add(3) as i32).wrapping_shl(24));
            data = lo as isize as usize;
            let hi = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | ((*d.add(7) as i32).wrapping_shl(24));
            data |= ((hi as isize as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                STBDS_SIPROUND!();
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        // switch (len - i) with fallthrough
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
            data |= (*d.add(3) as i32).wrapping_shl(24) as isize as usize;
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
            STBDS_SIPROUND!();
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            STBDS_SIPROUND!();
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// hash map machinery
// ---------------------------------------------------------------------------

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> bool {
    unsafe {
        if mode >= STBDS_HM_STRING {
            let stored =
                *((a as *mut u8).add(elemsize.wrapping_mul(i)).add(keyoffset) as *mut *mut c_char);
            strcmp_eq(key as *const c_char, stored as *const c_char)
        } else {
            memcmp_eq(
                key as *const u8,
                (a as *mut u8).add(elemsize.wrapping_mul(i)).add(keyoffset) as *const u8,
                keysize,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        if !stbds_hash_table(a).is_null() {
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
                let mut i = 1usize;
                while i < (*stbds_header(a)).length {
                    STBDS_FREE(
                        *((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_char)
                            as *mut c_void,
                    );
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
        }
        STBDS_FREE((*stbds_header(a)).hash_table);
        STBDS_FREE(stbds_header(a) as *mut c_void);
    }
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
        let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
        let table = stbds_hash_table(raw_a);
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut pos;

        if hash < 2 {
            hash += 2;
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

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
    unsafe {
        let keyoffset = 0usize;
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            memset0(a as *mut u8, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            STBDS_ARR_TO_HASH(a, elemsize)
        } else {
            let table: *mut stbds_hash_index;
            let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
            table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
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
            a
        }
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
    unsafe {
        let mut temp: isize = 0;
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        stbds_temp_set(STBDS_HASH_TO_ARR(p, elemsize), temp);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0 {
            a = stbds_arrgrowf(
                if !a.is_null() {
                    STBDS_HASH_TO_ARR(a, elemsize)
                } else {
                    ptr::null_mut()
                },
                elemsize,
                0,
                1,
            );
            (*stbds_header(a)).length += 1;
            memset0(a as *mut u8, elemsize);
            a = STBDS_ARR_TO_HASH(a, elemsize);
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset = 0usize;
        let mut a = a;
        let mut raw_a: *mut c_void;
        let mut table: *mut stbds_hash_index;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset0(a as *mut u8, elemsize);
            (*stbds_header(a)).length += 1;
            a = STBDS_ARR_TO_HASH(a, elemsize);
        }

        raw_a = a;
        a = STBDS_HASH_TO_ARR(a, elemsize);

        table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let nt: *mut stbds_hash_index;
            let slot_count: usize;

            slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count * 2
            };
            nt = stbds_make_hash_index(slot_count, table);
            if !table.is_null() {
                STBDS_FREE(table as *mut c_void);
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
            let mut pos;
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
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                stbds_temp_key_set(
                                    a,
                                    *((raw_a as *mut u8)
                                        .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                        .add(keyoffset) as *mut *mut c_char),
                                );
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
                            (*bucket).index[i] as usize,
                        ) {
                            stbds_temp_set(a, (*bucket).index[i]);
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
                (*table).tombstone_count -= 1;
            }
            (*table).used_count += 1;

            {
                let i: isize = stbds_arrlen(a);
                if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                    a = stbds_arrgrowf(a, elemsize, 1, 0);
                }
                raw_a = STBDS_ARR_TO_HASH(a, elemsize);

                STBDS_ASSERT!((i as usize).wrapping_add(1) <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let slot =
                    (a as *mut u8).add(elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *slot = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                        *slot = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *slot = p;
                        stbds_temp_key_set(a, p);
                    }
                    _ => {
                        memmove(
                            (a as *mut u8).add(elemsize.wrapping_mul(i as usize)),
                            key as *const u8,
                            keysize,
                        );
                    }
                }
                let _ = raw_a;
            }
            STBDS_ARR_TO_HASH(a, elemsize)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        let h: *mut stbds_hash_index;
        memset0(a as *mut u8, elemsize);
        (*stbds_header(a)).length = 1;
        h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        STBDS_ARR_TO_HASH(a, elemsize)
    }
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
    unsafe {
        if a.is_null() {
            ptr::null_mut()
        } else {
            let table: *mut stbds_hash_index;
            let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
            table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
            stbds_temp_set(raw_a, 0);
            if table.is_null() {
                a
            } else {
                let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    a
                } else {
                    let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    let mut i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    let old_index = (*b).index[i as usize];
                    let final_index = stbds_arrlen(raw_a) - 1 - 1;
                    STBDS_ASSERT!(slot < (*table).slot_count as isize);
                    (*table).used_count -= 1;
                    (*table).tombstone_count += 1;
                    stbds_temp_set(raw_a, 1);
                    STBDS_ASSERT!((*table).used_count as isize >= 0);
                    (*b).hash[i as usize] = STBDS_HASH_DELETED;
                    (*b).index[i as usize] = STBDS_INDEX_DELETED;

                    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                        STBDS_FREE(
                            *((a as *mut u8).add(elemsize.wrapping_mul(old_index as usize))
                                as *mut *mut c_char) as *mut c_void,
                        );
                    }

                    if old_index != final_index {
                        memmove(
                            (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)),
                            (a as *mut u8).add(elemsize.wrapping_mul(final_index as usize)),
                            elemsize,
                        );

                        if mode == STBDS_HM_STRING {
                            slot = stbds_hm_find_slot(
                                a,
                                elemsize,
                                *((a as *mut u8)
                                    .add(elemsize.wrapping_mul(old_index as usize))
                                    .add(keyoffset) as *mut *mut c_char) as *mut c_void,
                                keysize,
                                keyoffset,
                                mode,
                            );
                        } else {
                            slot = stbds_hm_find_slot(
                                a,
                                elemsize,
                                (a as *mut u8)
                                    .add(elemsize.wrapping_mul(old_index as usize))
                                    .add(keyoffset) as *mut c_void,
                                keysize,
                                keyoffset,
                                mode,
                            );
                        }
                        STBDS_ASSERT!(slot >= 0);
                        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                        STBDS_ASSERT!((*b).index[i as usize] == final_index);
                        (*b).index[i as usize] = old_index;
                    }
                    (*stbds_header(raw_a)).length -= 1;

                    if (*table).used_count < (*table).used_count_shrink_threshold
                        && (*table).slot_count > STBDS_BUCKET_LENGTH
                    {
                        (*stbds_header(raw_a)).hash_table =
                            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
                        STBDS_FREE(table as *mut c_void);
                    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
                        (*stbds_header(raw_a)).hash_table =
                            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
                        STBDS_FREE(table as *mut c_void);
                    }

                    a
                }
            }
        }
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = STBDS_REALLOC(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut u8, str_ as *const u8, len);
        p
    }
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let p: *mut c_char;
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = STBDS_REALLOC(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                memmove(
                    (&raw mut (*sb).storage) as *mut c_char as *mut u8,
                    str_ as *const u8,
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
                return (&raw mut (*sb).storage) as *mut c_char;
            } else {
                let sb = STBDS_REALLOC(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        STBDS_ASSERT!(len <= (*a).remaining);
        p = ((&raw mut (*(*a).storage).storage) as *mut c_char).add((*a).remaining - len);
        (*a).remaining -= len;
        memmove(p as *mut u8, str_ as *const u8, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x: *mut stbds_string_block;
        let mut y: *mut stbds_string_block;
        x = (*a).storage;
        while !x.is_null() {
            y = (*x).next;
            STBDS_FREE(x as *mut c_void);
            x = y;
        }
        memset0(a as *mut u8, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// test driver
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut buffer) as *mut c_char;
        sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
        buf
    }
}

/// Element type of the anonymous struct used by `sh_geti`:
/// `struct { char *key; int value; }`
#[repr(C)]
#[derive(Clone, Copy)]
struct strmap_entry {
    key: *mut c_char,
    value: c_int,
}

const ELEMSIZE: usize = size_of::<strmap_entry>();
const KEYSIZE: usize = size_of::<*mut c_char>();

/// `stbds_temp((t)-1)` for the string map
#[inline]
unsafe fn sh_temp(t: *mut strmap_entry) -> isize {
    unsafe { stbds_temp_get(t.sub(1) as *mut c_void) }
}

#[inline]
unsafe fn shgeti(t: &mut *mut strmap_entry, k: *mut c_char) -> isize {
    unsafe {
        *t = stbds_hmget_key(
            *t as *mut c_void,
            ELEMSIZE,
            k as *mut c_void,
            KEYSIZE,
            STBDS_HM_STRING,
        ) as *mut strmap_entry;
        sh_temp(*t)
    }
}

#[inline]
unsafe fn shput(t: &mut *mut strmap_entry, k: *mut c_char, v: c_int) {
    unsafe {
        *t = stbds_hmput_key(
            *t as *mut c_void,
            ELEMSIZE,
            k as *mut c_void,
            KEYSIZE,
            STBDS_HM_STRING,
        ) as *mut strmap_entry;
        let idx = sh_temp(*t);
        (*(*t).offset(idx)).value = v;
    }
}

#[inline]
unsafe fn shget(t: &mut *mut strmap_entry, k: *mut c_char) -> c_int {
    unsafe {
        shgeti(t, k);
        (*(*t).offset(sh_temp(*t))).value
    }
}

#[inline]
unsafe fn shdel(t: &mut *mut strmap_entry, k: *mut c_char) -> isize {
    unsafe {
        *t = stbds_hmdel_key(
            *t as *mut c_void,
            ELEMSIZE,
            k as *mut c_void,
            KEYSIZE,
            0,
            STBDS_HM_STRING,
        ) as *mut strmap_entry;
        if !(*t).is_null() {
            sh_temp(*t)
        } else {
            0
        }
    }
}

#[inline]
unsafe fn shdefault(t: &mut *mut strmap_entry, v: c_int) {
    unsafe {
        *t = stbds_hmput_default(*t as *mut c_void, ELEMSIZE) as *mut strmap_entry;
        (*(*t).offset(-1)).value = v;
    }
}

#[inline]
unsafe fn shlen(t: *mut strmap_entry) -> isize {
    unsafe {
        if !t.is_null() {
            (*stbds_header(t.sub(1) as *mut c_void)).length as isize - 1
        } else {
            0
        }
    }
}

#[inline]
unsafe fn shfree(t: &mut *mut strmap_entry) {
    unsafe {
        if !(*t).is_null() {
            stbds_hmfree_func((*t).sub(1) as *mut c_void, ELEMSIZE);
        }
        *t = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    unsafe {
        let mut strmap: *mut strmap_entry = ptr::null_mut();
        let mut sa = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut i: c_int;
        let mut j: c_int;

        let foo = b"foo\0".as_ptr() as *mut c_char;

        i = 0;
        while i < num {
            stbds_stralloc(&mut sa, strkey(i));
            i += 1;
        }
        stbds_strreset(&mut sa);

        j = 0;
        while j < 2 {
            STBDS_ASSERT!(shgeti(&mut strmap, foo) == -1);
            if j == 0 {
                strmap = stbds_shmode_func(ELEMSIZE, STBDS_SH_STRDUP as c_int) as *mut strmap_entry;
            } else {
                strmap = stbds_shmode_func(ELEMSIZE, STBDS_SH_ARENA as c_int) as *mut strmap_entry;
            }
            STBDS_ASSERT!(shgeti(&mut strmap, foo) == -1);
            shdefault(&mut strmap, -2);
            STBDS_ASSERT!(shgeti(&mut strmap, foo) == -1);

            i = 0;
            while i < num {
                shput(&mut strmap, strkey(i), i.wrapping_mul(3));
                i += 2;
            }

            // printf("%s %d\n", strmap[z], strmap[z].value);
            //
            // The first argument is the whole 16-byte struct: on the SysV
            // AMD64 ABI it occupies two INTEGER eightbytes, so "%s" consumes
            // the key pointer and "%d" consumes the low half of the second
            // eightbyte, which is `value`. The trailing `strmap[z].value`
            // argument is never read by the format string. This reproduces
            // that register layout exactly.
            let fmt = b"%s %d\n\0".as_ptr() as *const c_char;
            let mut z: c_int = 0;
            while (z as isize) < shlen(strmap) {
                let e = strmap.offset(z as isize);
                printf(fmt, (*e).key, (*e).value as u32 as u64, (*e).value);
                z += 1;
            }

            i = 0;
            while i < num {
                if i & 1 != 0 {
                    STBDS_ASSERT!(shget(&mut strmap, strkey(i)) == -2);
                } else {
                    STBDS_ASSERT!(shget(&mut strmap, strkey(i)) == i.wrapping_mul(3));
                }
                i += 1;
            }
            i = 2;
            while i < num {
                shdel(&mut strmap, strkey(i));
                i += 4;
            }
            i = 0;
            while i < num {
                if i & 3 != 0 {
                    STBDS_ASSERT!(shget(&mut strmap, strkey(i)) == -2);
                } else {
                    STBDS_ASSERT!(shget(&mut strmap, strkey(i)) == i.wrapping_mul(3));
                }
                i += 1;
            }
            i = 0;
            while i < num {
                shdel(&mut strmap, strkey(i));
                i += 1;
            }
            i = 0;
            while i < num {
                STBDS_ASSERT!(shget(&mut strmap, strkey(i)) == -2);
                i += 1;
            }

            shfree(&mut strmap);
            j += 1;
        }
    }
}
