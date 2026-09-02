//! Rust translation of the C library in `c_src/` (an stb_ds.h derivative plus the
//! `helxo`/`strkey` demo entry points).
//!
//! The translation is deliberately literal: allocation is done with the C
//! allocator, structure layouts are `#[repr(C)]` clones of the originals, and
//! integer arithmetic reproduces the original's wrapping / sign-extension
//! behaviour (including the places where the C is arguably buggy).

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uchar, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc
// ---------------------------------------------------------------------------

unsafe extern "C" {
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

/// `STBDS_REALLOC(c,p,s)` -> `realloc(p,s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    unsafe { realloc(p, s) }
}

/// `STBDS_FREE(c,p)` -> `free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

// ---------------------------------------------------------------------------
// Layout-compatible structures
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
    block: c_uchar,
    mode: c_uchar,
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
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const HDR_SIZE: usize = size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Array header helpers (the `stbds_header` / `stbds_arrlen` / ... macros)
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

#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) = v }
}

/// `STBDS_HASH_TO_ARR(x,elemsize)`
#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

/// `STBDS_ARR_TO_HASH(x,elemsize)`
#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

/// `stbds_hash_table(a)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

#[inline]
unsafe fn elem_ptr(a: *mut c_void, elemsize: usize, i: isize) -> *mut u8 {
    unsafe { (a as *mut u8).offset((elemsize as isize).wrapping_mul(i)) }
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
        let raw = stbds_realloc(old, elemsize.wrapping_mul(min_cap).wrapping_add(HDR_SIZE));
        let b = (raw as *mut u8).add(HDR_SIZE) as *mut c_void;
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
    unsafe { stbds_free(stbds_header(a) as *mut c_void) }
}

// ---------------------------------------------------------------------------
// Hash seed / hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { stbds_hash_seed = seed }
}

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

#[inline]
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

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    let mut temp = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ (v32 as usize);
    var
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = stbds_realloc(
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

        (*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);
        (*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);
        (*t).used_count_shrink_threshold = slot_count >> 2;

        if slot_count <= STBDS_BUCKET_LENGTH {
            (*t).used_count_shrink_threshold = 0;
        }

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
            (*t).seed = stbds_hash_seed;
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
            stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
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
                            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                            pos &= (*t).slot_count.wrapping_sub(1);
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
// Hash functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut s = str_ as *const c_uchar;
        let mut hash: usize = seed;
        while *s != 0 {
            hash = hash
                .rotate_left(9)
                .wrapping_add(*s as usize);
            s = s.add(1);
        }

        hash ^= seed;
        hash = (!hash).wrapping_add(hash << 18);
        hash ^= hash ^ hash.rotate_right(31);
        hash = hash.wrapping_mul(21);
        hash ^= hash ^ hash.rotate_right(11);
        hash = hash.wrapping_add(hash << 6);
        hash ^= hash.rotate_right(22);
        hash.wrapping_add(seed)
    }
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! stbds_sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = $v1.rotate_left(13);
        $v1 ^= $v0;
        $v0 = $v0.rotate_left(STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = $v3.rotate_left(16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = $v1.rotate_left(17);
        $v1 ^= $v2;
        $v2 = $v2.rotate_left(STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = $v3.rotate_left(21);
        $v3 ^= $v0;
    }};
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const c_uchar;

        let mut v0: usize = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
        let mut v1: usize = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
        let mut v2: usize = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
        let mut v3: usize = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut i: usize = 0;
        let mut data: usize;
        while i.wrapping_add(size_of::<usize>()) <= len {
            // `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` is computed in `int`
            // and then converted to `size_t`, so a byte >= 0x80 at d[3] sign-extends.
            let lo = (*d.add(0) as u32)
                | ((*d.add(1) as u32) << 8)
                | ((*d.add(2) as u32) << 16)
                | ((*d.add(3) as u32) << 24);
            data = (lo as i32) as isize as usize;
            let hi = (*d.add(4) as u32)
                | ((*d.add(5) as u32) << 8)
                | ((*d.add(6) as u32) << 16)
                | ((*d.add(7) as u32) << 24);
            data |= (((hi as i32) as isize as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                stbds_sipround!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i = i.wrapping_add(size_of::<usize>());
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        // The C uses a fall-through switch on `len - i`.
        let rem = len.wrapping_sub(i);
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
            // `d[3] << 24` is an `int` expression: sign-extends when d[3] >= 0x80.
            data |= (((*d.add(3) as u32) << 24) as i32) as isize as usize;
        }
        if rem >= 3 {
            data |= ((*d.add(2) as u32) << 16) as usize;
        }
        if rem >= 2 {
            data |= ((*d.add(1) as u32) << 8) as usize;
        }
        if rem >= 1 {
            data |= *d.add(0) as usize;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// Key comparison
// ---------------------------------------------------------------------------

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    unsafe {
        if mode >= STBDS_HM_STRING {
            let slot = elem_ptr(a, elemsize, i).add(keyoffset) as *mut *mut c_char;
            0 == strcmp(key as *const c_char, *slot)
        } else {
            0 == memcmp(
                key as *const c_void,
                elem_ptr(a, elemsize, i).add(keyoffset) as *const c_void,
                keysize,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hmfree_func
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        if !stbds_hash_table(a).is_null() {
            if (*stbds_hash_table(a)).string.mode as c_int == STBDS_SH_STRDUP {
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    stbds_free(
                        *(elem_ptr(a, elemsize, i as isize) as *mut *mut c_char) as *mut c_void,
                    );
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
        }
        stbds_free((*stbds_header(a)).hash_table);
        stbds_free(stbds_header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// stbds_hm_find_slot
// ---------------------------------------------------------------------------

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
        let raw_a = hash_to_arr(a, elemsize);
        let table = stbds_hash_table(raw_a);
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;

        if hash < 2 {
            hash = hash.wrapping_add(2);
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
                        (*bucket).index[i],
                    ) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
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
}

// ---------------------------------------------------------------------------
// stbds_hmget_key_ts / stbds_hmget_key
// ---------------------------------------------------------------------------

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
            (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
            memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            arr_to_hash(a, elemsize)
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
        stbds_temp_set(hash_to_arr(p, elemsize), temp);
        p
    }
}

// ---------------------------------------------------------------------------
// stbds_hmput_default
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
            let base = if !a.is_null() {
                hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            };
            let g = stbds_arrgrowf(base, elemsize, 0, 1);
            (*stbds_header(g)).length = (*stbds_header(g)).length.wrapping_add(1);
            memset(g, 0, elemsize);
            a = arr_to_hash(g, elemsize);
        }
        a
    }
}

// ---------------------------------------------------------------------------
// stbds_hmput_key
// ---------------------------------------------------------------------------

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
        let mut raw_a: *mut c_void;
        let mut table: *mut stbds_hash_index;

        if a.is_null() {
            let g = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(g, 0, elemsize);
            (*stbds_header(g)).length = (*stbds_header(g)).length.wrapping_add(1);
            a = arr_to_hash(g, elemsize);
        }

        raw_a = a;
        a = hash_to_arr(a, elemsize);

        table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count.wrapping_mul(2)
            };
            let nt = stbds_make_hash_index(slot_count, table);
            if !table.is_null() {
                stbds_free(table as *mut c_void);
            } else {
                (*nt).string.mode = if mode >= STBDS_HM_STRING {
                    STBDS_SH_DEFAULT as c_uchar
                } else {
                    0
                };
            }
            table = nt;
            (*stbds_header(a)).hash_table = table as *mut c_void;
        }

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

        'outer: loop {
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
                    ) {
                        stbds_temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let v = *(elem_ptr(raw_a, elemsize, (*bucket).index[i]).add(keyoffset)
                                as *mut *mut c_char);
                            stbds_temp_key_set(a, v);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
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
                        (*bucket).index[i],
                    ) {
                        stbds_temp_set(a, (*bucket).index[i]);
                        return arr_to_hash(a, elemsize);
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
                break 'outer;
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
            raw_a = arr_to_hash(a, elemsize);
            let _ = raw_a;

            (*stbds_header(a)).length = i.wrapping_add(1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i.wrapping_sub(1);
            stbds_temp_set(a, i.wrapping_sub(1));

            let dst = elem_ptr(a, elemsize, i) as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *dst = p;
                    stbds_temp_key_set(a, p);
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                    *dst = p;
                    stbds_temp_key_set(a, p);
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *dst = p;
                    stbds_temp_key_set(a, p);
                }
                _ => {
                    memcpy(
                        elem_ptr(a, elemsize, i) as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        arr_to_hash(a, elemsize)
    }
}

// ---------------------------------------------------------------------------
// stbds_shmode_func
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as c_uchar;
        arr_to_hash(a, elemsize)
    }
}

// ---------------------------------------------------------------------------
// stbds_hmdel_key
// ---------------------------------------------------------------------------

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

        let raw_a = hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_temp_set(raw_a, 0);
        if table.is_null() {
            return a;
        }

        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }

        let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1).wrapping_sub(1);
        (*table).used_count = (*table).used_count.wrapping_sub(1);
        (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
        stbds_temp_set(raw_a, 1);
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode as c_int == STBDS_SH_STRDUP {
            stbds_free(*(elem_ptr(a, elemsize, old_index) as *mut *mut c_char) as *mut c_void);
        }

        if old_index != final_index {
            memmove(
                elem_ptr(a, elemsize, old_index) as *mut c_void,
                elem_ptr(a, elemsize, final_index) as *const c_void,
                elemsize,
            );

            if mode == STBDS_HM_STRING {
                let k = *(elem_ptr(a, elemsize, old_index).add(keyoffset) as *mut *mut c_char);
                slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
            } else {
                let k = elem_ptr(a, elemsize, old_index).add(keyoffset);
                slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
            }
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            (*b).index[i] = old_index;
        }
        (*stbds_header(raw_a)).length = (*stbds_header(raw_a)).length.wrapping_sub(1);

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

// ---------------------------------------------------------------------------
// String storage
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_).wrapping_add(1);
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, str_ as *const c_void, len);
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
        let len = strlen(str_).wrapping_add(1);
        if len > (*a).remaining {
            let blocksize0 = (*a).block as usize;

            let blocksize =
                STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize0 >> 1) as u32);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    (size_of::<stbds_string_block>() - 8).wrapping_add(len),
                ) as *mut stbds_string_block;
                memmove(
                    (&raw mut (*sb).storage) as *mut c_void,
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
                return (&raw mut (*sb).storage) as *mut c_char;
            } else {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    (size_of::<stbds_string_block>() - 8).wrapping_add(blocksize),
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .add((*a).remaining)
            .sub(len);
        (*a).remaining = (*a).remaining.wrapping_sub(len);
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            stbds_free(x as *mut c_void);
            x = y;
        }
        memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// strkey
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let b = (&raw mut buffer) as *mut c_char;
        sprintf(b, c"test_%d".as_ptr(), n);
        b
    }
}

// ---------------------------------------------------------------------------
// helxo
// ---------------------------------------------------------------------------

/// The anonymous `struct { char *key; char value; }` used by `helxo`.
#[repr(C)]
struct helxo_entry {
    key: *mut c_char,
    value: c_char,
}

const HELXO_ELEMSIZE: usize = size_of::<helxo_entry>();

/// `shput(hash, k, v)`
#[inline]
unsafe fn helxo_shput(
    t: *mut helxo_entry,
    key: *mut c_char,
    value: c_char,
) -> *mut helxo_entry {
    unsafe {
        let t = stbds_hmput_key(
            t as *mut c_void,
            HELXO_ELEMSIZE,
            key as *mut c_void,
            size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut helxo_entry;
        let raw = hash_to_arr(t as *mut c_void, HELXO_ELEMSIZE);
        let idx = (*stbds_header(raw)).temp;
        (*t.offset(idx)).value = value;
        t
    }
}

/// `shlen(hash)`
#[inline]
unsafe fn helxo_shlen(t: *mut helxo_entry) -> isize {
    unsafe {
        if t.is_null() {
            0
        } else {
            ((*stbds_header(hash_to_arr(t as *mut c_void, HELXO_ELEMSIZE))).length as isize).wrapping_sub(1)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    unsafe {
        let mut hash: *mut helxo_entry = ptr::null_mut();
        let mut name: [c_char; 4] = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];

        hash = helxo_shput(hash, c"bob".as_ptr() as *mut c_char, b'h' as c_char);
        hash = helxo_shput(hash, c"sally".as_ptr() as *mut c_char, b'e' as c_char);
        hash = helxo_shput(hash, c"fred".as_ptr() as *mut c_char, b'l' as c_char);
        hash = helxo_shput(hash, c"jen".as_ptr() as *mut c_char, b'x' as c_char);
        hash = helxo_shput(hash, c"doug".as_ptr() as *mut c_char, b'o' as c_char);

        hash = helxo_shput(hash, (&raw mut name) as *mut c_char, letter);

        // The C passes the whole element struct as the `%s` argument; under the
        // SysV ABI that places the `key` pointer in the first argument register
        // and the second eightbyte (whose low byte is `value`) in the next, so
        // `%s` prints the key and `%c` prints the value.
        let mut z: isize = 0;
        while z < helxo_shlen(hash) {
            let e = hash.offset(z);
            printf(c"%s %c\n".as_ptr(), (*e).key, (*e).value as c_int);
            z += 1;
        }

        // shfree(hash) == stbds_hmfree(hash), which is
        //   ((void) ((p) != NULL ? stbds_hmfree_func((p)-1,sizeof*(p)),0 : 0),(p)=NULL)
        // i.e. the free is guarded on a non-NULL table.
        if !hash.is_null() {
            stbds_hmfree_func(
                hash_to_arr(hash as *mut c_void, HELXO_ELEMSIZE),
                HELXO_ELEMSIZE,
            );
        }
        hash = ptr::null_mut();
        let _ = hash;

        let _ = STBDS_SH_NONE;
        let _ = STBDS_INDEX_EMPTY;
    }
}
