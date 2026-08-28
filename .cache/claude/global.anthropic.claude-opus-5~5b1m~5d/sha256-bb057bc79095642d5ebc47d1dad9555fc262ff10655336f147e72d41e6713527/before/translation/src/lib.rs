//! Rust translation of the C library in `c_src/` (an inlined copy of stb_ds.h
//! plus the `strkey` / `str_put` driver code from `src/lib.c`).
//!
//! The translation is deliberately literal: every quirk of the original C
//! (including implicit `int` sign-extension in the SipHash byte loading, the
//! missing `temp_key` update in the wrap-around branch of `stbds_hmput_key`,
//! and the `printf("%s %d\n", struct_by_value, ...)` call in `str_put`) is
//! reproduced bit for bit.
//!
//! All heap memory is managed through libc `realloc`/`free` so that pointers
//! handed out by / passed into this library remain interchangeable with the
//! ones from the original C implementation.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (STBDS_REALLOC / STBDS_FREE and the string/stdio helpers)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

#[inline(always)]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    realloc(p, size)
}

#[inline(always)]
unsafe fn stbds_free(p: *mut c_void) {
    free(p)
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

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Macro helpers
// ---------------------------------------------------------------------------

/// `stbds_header(t)` -- `((stbds_array_header *) (t) - 1)`
#[inline(always)]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    ((t as usize).wrapping_sub(HEADER_SIZE)) as *mut stbds_array_header
}

/// `stbds_arrlen(a)`
#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if !a.is_null() {
        (*stbds_header(a)).length as isize
    } else {
        0
    }
}

/// `stbds_arrcap(a)`
#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    }
}

/// `stbds_temp(t)` (read)
#[inline(always)]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

/// `stbds_temp(t) = v`
#[inline(always)]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `stbds_temp_key(t) = v` -- `(*(char **) stbds_header(t)->hash_table) = v`
#[inline(always)]
unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *ht = v;
}

/// `stbds_hash_table(a)` -- `((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_HASH_TO_ARR(x,elemsize)` -- `((char *) (x) - (elemsize))`
#[inline(always)]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    ((x as usize).wrapping_sub(elemsize)) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x,elemsize)` -- `((char *) (x) + (elemsize))`
#[inline(always)]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    ((x as usize).wrapping_add(elemsize)) as *mut c_void
}

/// `(char *) a + elemsize*i + keyoffset`
#[inline(always)]
fn elem_ptr(a: *mut c_void, elemsize: usize, i: usize, keyoffset: usize) -> *mut c_void {
    ((a as usize)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(keyoffset)) as *mut c_void
}

/// `STBDS_ALIGN_FWD(n,a)` -- `(((n) + (a) - 1) & ~((a)-1))`
#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// `STBDS_INDEX_IN_USE(x)` -- `((x) >= 0)`
#[inline(always)]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

#[inline(always)]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
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
    let mut min_cap = min_cap;
    let b: *mut c_void;
    let min_len: usize = (stbds_arrlen(a) as usize).wrapping_add(addlen);

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

    let old: *mut c_void = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };
    let raw = stbds_realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    );
    b = ((raw as usize).wrapping_add(HEADER_SIZE)) as *mut c_void;
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
    stbds_free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// hash seed / hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline(always)]
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

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
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
    let t: *mut stbds_hash_index = stbds_realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = stbds_align_fwd(
        (t as usize).wrapping_add(core::mem::size_of::<stbds_hash_index>()),
        STBDS_CACHE_LINE_SIZE,
    ) as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        memset(
            ptr::addr_of_mut!((*t).string) as *mut c_void,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b: *mut stbds_hash_bucket = (*t).storage.add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
                j += 1;
            }
            j = 0;
            while j < STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
                j += 1;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let mut i: usize = 0;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob: *mut stbds_hash_bucket = (*ot).storage.add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
                        z = 0;
                        let mut hit = false;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                hit = true;
                                break;
                            }
                            z += 1;
                        }
                        if hit {
                            break 'done;
                        }

                        pos = pos.wrapping_add(step);
                        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }

    t
}

// ---------------------------------------------------------------------------
// hash functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
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

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

/// Mirrors `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` evaluated in
/// `int` arithmetic and then converted to `size_t` (i.e. sign-extended when
/// the top byte has its high bit set -- exactly what the C code does).
#[inline(always)]
unsafe fn load_int_le32_sign_extended(d: *const u8) -> usize {
    let v: u32 = (*d.add(0) as u32)
        | ((*d.add(1) as u32) << 8)
        | ((*d.add(2) as u32) << 16)
        | ((*d.add(3) as u32) << 24);
    (v as i32) as isize as usize
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let mut j: usize;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = stbds_rotate_left(v1, 13);
            v1 ^= v0;
            v0 = stbds_rotate_left(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = stbds_rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = stbds_rotate_left(v1, 17);
            v1 ^= v2;
            v2 = stbds_rotate_left(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = stbds_rotate_left(v3, 21);
            v3 ^= v0;
        }};
    }

    i = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        data = load_int_le32_sign_extended(d);
        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        data |= (load_int_le32_sign_extended(d.add(4)) << 16) << 16;

        v3 ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            sipround!();
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let tail = len.wrapping_sub(i);
    // Fall-through switch from the C original.
    if tail >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if tail >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if tail >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if tail >= 4 {
        // `data |= (d[3] << 24);` -- int arithmetic, hence sign extension.
        data |= (((*d.add(3) as u32) << 24) as i32) as isize as usize;
    }
    if tail >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if tail >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if tail >= 1 {
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    j = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        sipround!();
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    j = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        sipround!();
        j += 1;
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
    i: usize,
) -> c_int {
    if mode >= STBDS_HM_STRING {
        let stored = *(elem_ptr(a, elemsize, i, keyoffset) as *mut *mut c_char);
        (strcmp(key as *const c_char, stored) == 0) as c_int
    } else {
        (memcmp(
            key as *const c_void,
            elem_ptr(a, elemsize, i, keyoffset) as *const c_void,
            keysize,
        ) == 0) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                stbds_free(*(elem_ptr(a, elemsize, i, 0) as *mut *mut c_char) as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));
    }
    stbds_free((*stbds_header(a)).hash_table);
    stbds_free(stbds_header(a) as *mut c_void);
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut limit: usize;
    let mut i: usize;
    let mut pos: usize;
    let mut bucket: *mut stbds_hash_bucket;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        i = pos & STBDS_BUCKET_MASK;
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
                    return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        limit = pos & STBDS_BUCKET_MASK;
        i = 0;
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
                    return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        pos = pos.wrapping_add(step);
        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
        pos &= (*table).slot_count.wrapping_sub(1);
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
    let keyoffset: usize = 0;
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        stbds_arr_to_hash(a, elemsize)
    } else {
        let table: *mut stbds_hash_index;
        let raw_a = stbds_hash_to_arr(a, elemsize);
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
    stbds_set_temp(stbds_hash_to_arr(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
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

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
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
    let mut a = a;
    let keyoffset: usize = 0;
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
        let nt: *mut stbds_hash_index;
        let slot_count: usize;

        slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            stbds_free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT as u8
            } else {
                STBDS_SH_NONE as u8
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
            hash = hash.wrapping_add(2);
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'found_empty_slot: loop {
            let limit: usize;
            let mut i: usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            i = pos & STBDS_BUCKET_MASK;
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
                            let kp = *(elem_ptr(
                                raw_a,
                                elemsize,
                                (*bucket).index[i] as usize,
                                keyoffset,
                            ) as *mut *mut c_char);
                            stbds_set_temp_key(a, kp);
                        }
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                }
                i += 1;
            }

            limit = pos & STBDS_BUCKET_MASK;
            i = 0;
            let mut empty_found = false;
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
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    empty_found = true;
                    break;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                }
                i += 1;
            }
            if empty_found {
                break 'found_empty_slot;
            }

            pos = pos.wrapping_add(step);
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
            pos &= (*table).slot_count.wrapping_sub(1);
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
            raw_a = stbds_arr_to_hash(a, elemsize);
            let _ = raw_a;

            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_set_temp(a, i - 1);

            let slot: *mut *mut c_char = elem_ptr(a, elemsize, i as usize, 0) as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *slot = p;
                    stbds_set_temp_key(a, p);
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *slot = p;
                    stbds_set_temp_key(a, p);
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *slot = p;
                    stbds_set_temp_key(a, p);
                }
                _ => {
                    memcpy(slot as *mut c_void, key as *const c_void, keysize);
                }
            }
        }
        stbds_arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    let h: *mut stbds_hash_index;
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    stbds_arr_to_hash(a, elemsize)
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
        ptr::null_mut()
    } else {
        let table: *mut stbds_hash_index;
        let raw_a = stbds_hash_to_arr(a, elemsize);
        table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_set_temp(raw_a, 0);
        if table.is_null() {
            a
        } else {
            let mut slot: isize;
            slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b: *mut stbds_hash_bucket =
                    (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                stbds_set_temp(raw_a, 1);
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
                    stbds_free(
                        *(elem_ptr(a, elemsize, old_index as usize, 0) as *mut *mut c_char)
                            as *mut c_void,
                    );
                }

                if old_index != final_index {
                    memmove(
                        elem_ptr(a, elemsize, old_index as usize, 0),
                        elem_ptr(a, elemsize, final_index as usize, 0) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        let k = *(elem_ptr(a, elemsize, old_index as usize, keyoffset)
                            as *mut *mut c_char);
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            k as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            elem_ptr(a, elemsize, old_index as usize, keyoffset),
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    (*b).index[i as usize] = old_index;
                }
                (*stbds_header(raw_a)).length -= 1;

                if (*table).used_count < (*table).used_count_shrink_threshold
                    && (*table).slot_count > STBDS_BUCKET_LENGTH
                {
                    (*stbds_header(raw_a)).hash_table =
                        stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
                    stbds_free(table as *mut c_void);
                } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
                    (*stbds_header(raw_a)).hash_table =
                        stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
                    stbds_free(table as *mut c_void);
                }

                a
            }
        }
    }
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
    let len = strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = stbds_realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            memmove(
                ptr::addr_of_mut!((*sb).storage) as *mut c_void,
                str_ as *const c_void,
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
            let sb = stbds_realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    p = ((ptr::addr_of_mut!((*(*a).storage).storage) as usize)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len)) as *mut c_char;
    (*a).remaining -= len;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x: *mut stbds_string_block;
    let mut y: *mut stbds_string_block;
    x = (*a).storage;
    while !x.is_null() {
        y = (*x).next;
        stbds_free(x as *mut c_void);
        x = y;
    }
    memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}

// ---------------------------------------------------------------------------
// driver code from src/lib.c
// ---------------------------------------------------------------------------

/// `struct { char *key; int value; }` used by `str_put`.
#[repr(C)]
#[derive(Clone, Copy)]
struct str_map_entry {
    key: *mut c_char,
    value: c_int,
}

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(buffer) as *mut c_char;
    sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
    buf
}

static A_KEY: [u8; 2] = [b'a', 0];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let mut strmap: *mut str_map_entry = ptr::null_mut();
    let s: str_map_entry;
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut i: c_int;

    i = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    {
        s = str_map_entry {
            key: A_KEY.as_ptr() as *mut c_char,
            value: num,
        };

        // shputs(strmap, s):
        //   (t) = stbds_hmput_key((t), sizeof *(t), (void*) (s).key, sizeof (s).key, STBDS_HM_STRING),
        //   (t)[stbds_temp((t)-1)] = (s),
        //   (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1)
        let elemsize = core::mem::size_of::<str_map_entry>();
        let keysize = core::mem::size_of::<*mut c_char>();
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            s.key as *mut c_void,
            keysize,
            STBDS_HM_STRING,
        ) as *mut str_map_entry;
        let arr = stbds_hash_to_arr(strmap as *mut c_void, elemsize);
        let t = stbds_temp(arr);
        *strmap.offset(t) = s;
        let tk = *((*stbds_header(arr)).hash_table as *mut *mut c_char);
        (*strmap.offset(t)).key = tk;

        // assert(*strmap[0].key == 'a');
        // assert(strmap[0].key == s.key);
        // assert(strmap[0].value == s.value);

        // for (int z=0; z < shlen(strmap); ++z)
        //     printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // `strmap[z]` is a 16-byte struct passed by value: on the SysV x86-64
        // ABI its two eightbytes go into the two varargs slots that "%s" and
        // "%d" consume, i.e. the key pointer and the value.
        let mut z: c_int = 0;
        while (z as isize) < shlen(strmap, elemsize) {
            let e = *strmap.offset(z as isize);
            printf(b"%s %d\n\0".as_ptr() as *const c_char, e.key, e.value);
            z += 1;
        }

        // shfree(strmap):
        //   ((void) ((p) != NULL ? stbds_hmfree_func((p)-1,sizeof*(p)),0 : 0),(p)=NULL)
        if !strmap.is_null() {
            stbds_hmfree_func(stbds_hash_to_arr(strmap as *mut c_void, elemsize), elemsize);
        }
        strmap = ptr::null_mut();
        let _ = strmap;
    }
}

/// `stbds_shlen(t)` == `stbds_hmlen(t)` --
/// `((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)`
#[inline(always)]
unsafe fn shlen(t: *mut str_map_entry, elemsize: usize) -> isize {
    if !t.is_null() {
        (*stbds_header(stbds_hash_to_arr(t as *mut c_void, elemsize))).length as isize - 1
    } else {
        0
    }
}
