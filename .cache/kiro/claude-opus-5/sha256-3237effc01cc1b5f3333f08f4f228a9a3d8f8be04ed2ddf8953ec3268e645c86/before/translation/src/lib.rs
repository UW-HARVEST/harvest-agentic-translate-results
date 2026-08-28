//! Faithful Rust translation of `c_src/src/lib.c` (a trimmed-down copy of
//! `stb_ds.h` plus the `hm_geti` exercise function from its unit tests).
//!
//! The layout of every structure, the order of every operation, and every
//! integer-conversion quirk of the original C is reproduced exactly so that the
//! shared library behaves (and fails) byte-for-byte like the C build.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_unsafe)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free/memmove/... directly)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

const FILE_NAME: &[u8] = b"src/lib.c\0";

/// `STBDS_ASSERT` == `assert`; NDEBUG is not defined by the CMake build, so the
/// checks are live and route through glibc's `__assert_fail`.
macro_rules! stbds_assert {
    ($cond:expr, $text:literal, $line:expr, $func:literal) => {
        if !$cond {
            unsafe {
                __assert_fail(
                    concat!($text, "\0").as_ptr() as *const c_char,
                    FILE_NAME.as_ptr() as *const c_char,
                    $line,
                    concat!($func, "\0").as_ptr() as *const c_char,
                )
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------

#[repr(C)]
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
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: u32 = 3;
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

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

// ---------------------------------------------------------------------------
// Array-header helpers (the `stbds_header` / `stbds_arrlen` / ... macros)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).offset(-1) }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `stbds_temp(t)` lvalue read.
#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

/// `stbds_temp(t) = v`.
#[inline]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

/// `stbds_temp_key(t) = v` -- writes through the first field of the hash index.
#[inline]
unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) = v }
}

#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

// ---------------------------------------------------------------------------
// Dynamic array growth
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

        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_header(a) as *mut c_void
        };
        let raw = realloc(
            old,
            elemsize
                .wrapping_mul(min_cap)
                .wrapping_add(size_of::<stbds_array_header>()),
        );
        b = (raw as *mut u8).add(size_of::<stbds_array_header>()) as *mut c_void;
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
    unsafe { free(stbds_header(a) as *mut c_void) }
}

// ---------------------------------------------------------------------------
// Hash index
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { STBDS_HASH_SEED = seed }
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
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
        stbds_assert!(
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
            "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count",
            401,
            "stbds_make_hash_index"
        );
        if !ot.is_null() {
            (*t).string = stbds_string_arena {
                storage: (*ot).string.storage,
                remaining: (*ot).string.remaining,
                block: (*ot).string.block,
                mode: (*ot).string.mode,
            };
            (*t).seed = (*ot).seed;
        } else {
            memset(
                &raw mut (*t).string as *mut c_void,
                0,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
            let mut temp: usize;
            temp = (0x87b0b0fdu32 ^ 2147001325u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut a: usize = 0x27bb2ee6usize;
            a <<= 16;
            a <<= 16;
            a ^= temp ^ 2147001325usize;
            // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d)
            temp = (0xb504f32du32 ^ 715136305u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut b: usize = 0usize;
            b <<= 16;
            b <<= 16;
            b ^= temp ^ 715136305usize;
            STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
        }

        {
            let mut i: usize = 0;
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
            let mut i: usize = 0;
            while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if stbds_index_in_use((*ob).index[j]) {
                        let hash = (*ob).hash[j];
                        let mut pos =
                            stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                        let mut step = STBDS_BUCKET_LENGTH;
                        loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                            let mut placed = false;
                            let mut z = pos & STBDS_BUCKET_MASK;
                            while z < STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    placed = true;
                                    break;
                                }
                                z += 1;
                            }
                            if placed {
                                break;
                            }

                            let limit = pos & STBDS_BUCKET_MASK;
                            let mut z = 0usize;
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
                                break;
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
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut s = str;
        while *s != 0 {
            hash = stbds_rotate_left(hash, 9).wrapping_add(*s as u8 as usize);
            s = s.add(1);
        }

        hash ^= seed;
        hash = (!hash).wrapping_add(hash.wrapping_shl(18));
        hash ^= hash ^ stbds_rotate_right(hash, 31);
        hash = hash.wrapping_mul(21);
        hash ^= hash ^ stbds_rotate_right(hash, 11);
        hash = hash.wrapping_add(hash.wrapping_shl(6));
        hash ^= stbds_rotate_right(hash, 22);
        hash.wrapping_add(seed)
    }
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline]
fn stbds_sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = stbds_rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = stbds_rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *mut u8;
        let mut data: usize;

        let mut v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        let mut v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        let mut v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        let mut v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut i: usize = 0;
        while i + size_of::<usize>() <= len {
            // C: int arithmetic, then (possibly sign-extending) conversion to size_t
            let w: u32 = (*d as u32)
                | ((*d.add(1) as u32) << 8)
                | ((*d.add(2) as u32) << 16)
                | ((*d.add(3) as u32) << 24);
            data = (w as i32) as i64 as usize;
            let w2: u32 = (*d.add(4) as u32)
                | ((*d.add(5) as u32) << 8)
                | ((*d.add(6) as u32) << 16)
                | ((*d.add(7) as u32) << 24);
            data |= (((w2 as i32) as i64 as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }
        data = len << (STBDS_SIZE_T_BITS - 8);
        // switch (len - i) with fall-through
        let n = len - i;
        if n >= 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        if n >= 6 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        if n >= 5 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        if n >= 4 {
            data |= ((*d.add(3) as i32).wrapping_shl(24)) as i64 as usize;
        }
        if n >= 3 {
            data |= ((*d.add(2) as i32) << 16) as i64 as usize;
        }
        if n >= 2 {
            data |= ((*d.add(1) as i32) << 8) as i64 as usize;
        }
        if n >= 1 {
            data |= (*d as i32) as i64 as usize;
        }
        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> c_int {
    unsafe {
        let slot = (a as *mut u8)
            .add(elemsize.wrapping_mul(i))
            .add(keyoffset) as *mut c_void;
        if mode >= STBDS_HM_STRING {
            (0 == strcmp(key as *const c_char, *(slot as *mut *mut c_char))) as c_int
        } else {
            (0 == memcmp(key, slot, keysize)) as c_int
        }
    }
}

// ---------------------------------------------------------------------------
// Hash map
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        if !stbds_hash_table(a).is_null() {
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    free(*((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_char)
                        as *mut c_void);
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
        }
        free((*stbds_header(a)).hash_table);
        free(stbds_header(a) as *mut c_void);
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
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = stbds_hash_table(raw_a);
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut pos: usize;

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
        let keyoffset: usize = 0;
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            stbds_arr_to_hash(a, elemsize)
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
        stbds_set_temp(stbds_hash_to_arr(p, elemsize), temp);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
            a = stbds_arrgrowf(
                if !a.is_null() {
                    stbds_hash_to_arr(a, elemsize)
                } else {
                    ptr::null_mut()
                },
                elemsize,
                0,
                1,
            );
            (*stbds_header(a)).length += 1;
            memset(a, 0, elemsize);
            a = stbds_arr_to_hash(a, elemsize);
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
        let keyoffset: usize = 0;
        let mut a = a;
        #[allow(unused_assignments)]
        let mut raw_a: *mut c_void;
        let mut table: *mut stbds_hash_index;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*stbds_header(a)).length += 1;
            a = stbds_arr_to_hash(a, elemsize);
        }

        raw_a = a;
        a = stbds_hash_to_arr(a, elemsize);

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
                    STBDS_SH_DEFAULT as u8
                } else {
                    0
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
                        ) != 0
                        {
                            stbds_set_temp(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                stbds_set_temp_key(
                                    a,
                                    *((raw_a as *mut u8)
                                        .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                        .add(keyoffset) as *mut *mut c_char),
                                );
                            }
                            return stbds_arr_to_hash(a, elemsize);
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
                        ) != 0
                        {
                            stbds_set_temp(a, (*bucket).index[i]);
                            return stbds_arr_to_hash(a, elemsize);
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
                raw_a = stbds_arr_to_hash(a, elemsize);
                let _ = raw_a;

                stbds_assert!(
                    (i as usize).wrapping_add(1) <= stbds_arrcap(a),
                    "(size_t) i+1 <= stbds_arrcap(a)",
                    778,
                    "stbds_hmput_key"
                );
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_set_temp(a, i - 1);

                let key_slot =
                    (a as *mut u8).add(elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
                match (*table).string.mode as c_int {
                    STBDS_SH_STRDUP => {
                        *key_slot = stbds_strdup(key as *mut c_char);
                        stbds_set_temp_key(a, *key_slot);
                    }
                    STBDS_SH_ARENA => {
                        *key_slot = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                        stbds_set_temp_key(a, *key_slot);
                    }
                    STBDS_SH_DEFAULT => {
                        *key_slot = key as *mut c_char;
                        stbds_set_temp_key(a, *key_slot);
                    }
                    _ => {
                        memcpy(key_slot as *mut c_void, key, keysize);
                    }
                }
            }
            stbds_arr_to_hash(a, elemsize)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        stbds_arr_to_hash(a, elemsize)
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
            return ptr::null_mut();
        }
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_set_temp(raw_a, 0);
        if table.is_null() {
            return a;
        }

        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }

        let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        let old_index = (*b).index[i as usize];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        stbds_assert!(
            slot < (*table).slot_count as isize,
            "slot < (ptrdiff_t) table->slot_count",
            828,
            "stbds_hmdel_key"
        );
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        stbds_set_temp(raw_a, 1);
        stbds_assert!(
            true, /* table->used_count >= 0 is always true for size_t */
            "table->used_count >= 0",
            832,
            "stbds_hmdel_key"
        );
        (*b).hash[i as usize] = STBDS_HASH_DELETED;
        (*b).index[i as usize] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
            free(
                *((a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char)
                    as *mut c_void,
            );
        }

        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
                (a as *mut u8).add(elemsize.wrapping_mul(final_index as usize)) as *const c_void,
                elemsize,
            );

            let moved_key = (a as *mut u8)
                .add(elemsize.wrapping_mul(old_index as usize))
                .add(keyoffset);
            if mode == STBDS_HM_STRING {
                slot = stbds_hm_find_slot(
                    a,
                    elemsize,
                    *(moved_key as *mut *mut c_char) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                );
            } else {
                slot = stbds_hm_find_slot(a, elemsize, moved_key as *mut c_void, keysize, keyoffset, mode);
            }
            stbds_assert!(slot >= 0, "slot >= 0", 846, "stbds_hmdel_key");
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
            stbds_assert!(
                (*b).index[i as usize] == final_index,
                "b->index[i] == final_index",
                849,
                "stbds_hmdel_key"
            );
            (*b).index[i as usize] = old_index;
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

        a
    }
}

// ---------------------------------------------------------------------------
// String storage
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str) + 1;
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, str as *const c_void, len);
        p
    }
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut c_char,
) -> *mut c_char {
    unsafe {
        let p: *mut c_char;
        let len = strlen(str) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = realloc(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                memmove(
                    (&raw mut (*sb).storage) as *mut c_void,
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
                return (&raw mut (*sb).storage) as *mut c_char;
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

        stbds_assert!(
            len <= (*a).remaining,
            "len <= a->remaining",
            913,
            "stbds_stralloc"
        );
        p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .add((*a).remaining)
            .sub(len);
        (*a).remaining -= len;
        memmove(p as *mut c_void, str as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            free(x as *mut c_void);
            x = y;
        }
        memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// Test helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        sprintf(
            (&raw mut BUFFER) as *mut c_char,
            c"test_%d".as_ptr(),
            n,
        );
        (&raw mut BUFFER) as *mut c_char
    }
}

/// The element type of `hm_geti`'s local `intmap`: `struct { int key; int value; }`
#[repr(C)]
#[derive(Copy, Clone)]
struct IntMap {
    key: c_int,
    value: c_int,
}

const IM_ELEMSIZE: usize = size_of::<IntMap>();
const IM_KEYSIZE: usize = size_of::<c_int>();

/// `stbds_temp((t)-1)`
#[inline]
unsafe fn im_temp(t: *mut IntMap) -> isize {
    unsafe { stbds_temp(t.offset(-1) as *mut c_void) }
}

#[inline]
unsafe fn im_hmgeti(t: &mut *mut IntMap, k: c_int) -> isize {
    unsafe {
        let key: [c_int; 1] = [k];
        *t = stbds_hmget_key(
            *t as *mut c_void,
            IM_ELEMSIZE,
            key.as_ptr() as *mut c_void,
            IM_KEYSIZE,
            STBDS_HM_BINARY,
        ) as *mut IntMap;
        im_temp(*t)
    }
}

#[inline]
unsafe fn im_hmget(t: &mut *mut IntMap, k: c_int) -> c_int {
    unsafe {
        im_hmgeti(t, k);
        (*(*t).offset(im_temp(*t))).value
    }
}

#[inline]
unsafe fn im_hmgeti_ts(t: &mut *mut IntMap, k: c_int, temp: &mut isize) -> isize {
    unsafe {
        let key: [c_int; 1] = [k];
        *t = stbds_hmget_key_ts(
            *t as *mut c_void,
            IM_ELEMSIZE,
            key.as_ptr() as *mut c_void,
            IM_KEYSIZE,
            temp,
            STBDS_HM_BINARY,
        ) as *mut IntMap;
        *temp
    }
}

#[inline]
unsafe fn im_hmget_ts(t: &mut *mut IntMap, k: c_int, temp: &mut isize) -> c_int {
    unsafe {
        im_hmgeti_ts(t, k, temp);
        (*(*t).offset(*temp)).value
    }
}

#[inline]
unsafe fn im_hmput(t: &mut *mut IntMap, k: c_int, v: c_int) {
    unsafe {
        let key: [c_int; 1] = [k];
        *t = stbds_hmput_key(
            *t as *mut c_void,
            IM_ELEMSIZE,
            key.as_ptr() as *mut c_void,
            IM_KEYSIZE,
            0,
        ) as *mut IntMap;
        (*(*t).offset(im_temp(*t))).key = k;
        (*(*t).offset(im_temp(*t))).value = v;
    }
}

#[inline]
unsafe fn im_hmdefault(t: &mut *mut IntMap, v: c_int) {
    unsafe {
        *t = stbds_hmput_default(*t as *mut c_void, IM_ELEMSIZE) as *mut IntMap;
        (*(*t).offset(-1)).value = v;
    }
}

#[inline]
unsafe fn im_hmdel(t: &mut *mut IntMap, k: c_int) -> isize {
    unsafe {
        let key: [c_int; 1] = [k];
        *t = stbds_hmdel_key(
            *t as *mut c_void,
            IM_ELEMSIZE,
            key.as_ptr() as *mut c_void,
            IM_KEYSIZE,
            0, // STBDS_OFFSETOF((t),key)
            STBDS_HM_BINARY,
        ) as *mut IntMap;
        if !(*t).is_null() { im_temp(*t) } else { 0 }
    }
}

#[inline]
unsafe fn im_hmfree(t: &mut *mut IntMap) {
    unsafe {
        if !(*t).is_null() {
            stbds_hmfree_func((*t).offset(-1) as *mut c_void, IM_ELEMSIZE);
        }
        *t = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    unsafe {
        let mut intmap: *mut IntMap = ptr::null_mut();
        let mut temp: isize = 0;
        let mut i: c_int;

        i = 1;
        stbds_assert!(
            im_hmgeti(&mut intmap, i) == -1,
            "hmgeti(intmap,i) == -1",
            952,
            "hm_geti"
        );
        im_hmdefault(&mut intmap, -2);
        stbds_assert!(
            im_hmgeti(&mut intmap, i) == -1,
            "hmgeti(intmap, i) == -1",
            954,
            "hm_geti"
        );
        stbds_assert!(
            im_hmget(&mut intmap, i) == -2,
            "hmget (intmap, i) == -2",
            955,
            "hm_geti"
        );
        i = 0;
        while i < num {
            im_hmput(&mut intmap, i, i.wrapping_mul(5));
            i += 2;
        }
        i = 0;
        while i < num {
            if i & 1 != 0 {
                stbds_assert!(
                    im_hmget(&mut intmap, i) == -2,
                    "hmget(intmap, i) == -2",
                    959,
                    "hm_geti"
                );
            } else {
                stbds_assert!(
                    im_hmget(&mut intmap, i) == i.wrapping_mul(5),
                    "hmget(intmap, i) == i*5",
                    960,
                    "hm_geti"
                );
            }
            if i & 1 != 0 {
                stbds_assert!(
                    im_hmget_ts(&mut intmap, i, &mut temp) == -2,
                    "hmget_ts(intmap, i, temp) == -2",
                    961,
                    "hm_geti"
                );
            } else {
                stbds_assert!(
                    im_hmget_ts(&mut intmap, i, &mut temp) == i.wrapping_mul(5),
                    "hmget_ts(intmap, i, temp) == i*5",
                    962,
                    "hm_geti"
                );
            }
            i += 1;
        }
        i = 0;
        while i < num {
            im_hmput(&mut intmap, i, i.wrapping_mul(3));
            i += 2;
        }
        i = 0;
        while i < num {
            if i & 1 != 0 {
                stbds_assert!(
                    im_hmget(&mut intmap, i) == -2,
                    "hmget(intmap, i) == -2",
                    967,
                    "hm_geti"
                );
            } else {
                stbds_assert!(
                    im_hmget(&mut intmap, i) == i.wrapping_mul(3),
                    "hmget(intmap, i) == i*3",
                    968,
                    "hm_geti"
                );
            }
            i += 1;
        }
        i = 2;
        while i < num {
            im_hmdel(&mut intmap, i);
            i += 4;
        }
        i = 0;
        while i < num {
            if i & 3 != 0 {
                stbds_assert!(
                    im_hmget(&mut intmap, i) == -2,
                    "hmget(intmap, i) == -2",
                    972,
                    "hm_geti"
                );
            } else {
                stbds_assert!(
                    im_hmget(&mut intmap, i) == i.wrapping_mul(3),
                    "hmget(intmap, i) == i*3",
                    973,
                    "hm_geti"
                );
            }
            i += 1;
        }
        i = 0;
        while i < num {
            im_hmdel(&mut intmap, i);
            i += 1;
        }
        i = 0;
        while i < num {
            stbds_assert!(
                im_hmget(&mut intmap, i) == -2,
                "hmget(intmap, i) == -2",
                977,
                "hm_geti"
            );
            i += 1;
        }
        im_hmfree(&mut intmap);
        i = 0;
        while i < num {
            im_hmput(&mut intmap, i, i.wrapping_mul(3));
            i += 2;
        }
        im_hmfree(&mut intmap);

        let _ = STBDS_SH_NONE;
        let _ = STBDS_HASH_DELETED;
    }
}
