//! Rust translation of c_src/src/lib.c (stb_ds.h implementation, public domain / MIT,
//! Sean Barrett) plus the `strkey` / `arr_del` helpers defined in that file.
//!
//! The translation is deliberately literal: every quirk of the original C
//! (including integer promotion / sign-extension artefacts in the SipHash
//! byte-gathering code, and the missing `temp_key` store in the second probe
//! loop of `stbds_hmput_key`) is reproduced bit-for-bit.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (we use the platform allocator so that memory returned by this
// library is interchangeable with memory from the original C library).
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
// Data structures (layout must match the C structs exactly).
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_hash_index {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: stbds_string_arena,
    pub storage: *mut stbds_hash_bucket,
}

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

/// `sizeof(stbds_array_header)`
const HDR_SIZE: usize = core::mem::size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Macro helpers from the header.
// ---------------------------------------------------------------------------

/// `stbds_header(t)  ((stbds_array_header *) (t) - 1)`
#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(HDR_SIZE) as *mut stbds_array_header
}

/// `stbds_arrlen(a)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

/// `stbds_arrcap(a)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `stbds_temp(t) = v`
#[inline]
unsafe fn set_stbds_temp(t: *mut c_void, v: isize) {
    unsafe {
        (*stbds_header(t)).temp = v;
    }
}

/// `stbds_temp_key(t) = v`  ->  `*(char **) stbds_header(t)->hash_table = v`
#[inline]
unsafe fn set_stbds_temp_key(t: *mut c_void, v: *mut c_char) {
    unsafe {
        *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
    }
}

/// `STBDS_HASH_TO_ARR(x,elemsize)  ((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x,elemsize)  ((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `stbds_hash_table(a)  ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `(char *) a + elemsize*i + keyoffset`
#[inline]
fn elem_at(a: *mut c_void, elemsize: usize, i: usize, keyoffset: usize) -> *mut u8 {
    (a as *mut u8)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(keyoffset)
}

/// `STBDS_ALIGN_FWD(n,a)   (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a).wrapping_sub(1) & !(a - 1)
}

/// `STBDS_ROTATE_LEFT(val, n)`
#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ROTATE_RIGHT(val, n)`
#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
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
    unsafe {
        let mut min_cap = min_cap;
        let b: *mut c_void;
        // size_t min_len = stbds_arrlen(a) + addlen;
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
        let raw = stbds_realloc(old, elemsize.wrapping_mul(min_cap).wrapping_add(HDR_SIZE));
        b = (raw as *mut u8).wrapping_add(HDR_SIZE) as *mut c_void;
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
        stbds_free(stbds_header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// Hash seed / random state
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        stbds_hash_seed = seed;
    }
}

// ---------------------------------------------------------------------------
// stbds_probe_position / stbds_log2 / stbds_make_hash_index
// ---------------------------------------------------------------------------

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

/// Reproduces the `stbds_load_32_or_64` macro.
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    // var = v64_hi, var <<= 16, var <<= 16
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    // var ^= temp ^ v32
    var ^= temp ^ (v32 as usize);
    var
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t: *mut stbds_hash_index = stbds_realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT)
                .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
                .wrapping_add(core::mem::size_of::<stbds_hash_index>())
                .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
        ) as *mut stbds_hash_index;

        (*t).storage =
            stbds_align_fwd(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
        // STBDS_ASSERT(t->used_count_threshold + t->tombstone_count_threshold < t->slot_count);
        // The C .so is built without NDEBUG, so this assert is live there and
        // must be live here too.
        assert!(
            (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold)
                < (*t).slot_count,
            "stbds_make_hash_index: used_count_threshold + tombstone_count_threshold < slot_count"
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
                core::mem::size_of::<stbds_string_arena>(),
            );
            (*t).seed = stbds_hash_seed;
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
            stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
        }

        {
            let nbuckets = slot_count >> STBDS_BUCKET_SHIFT;
            let mut i: usize = 0;
            while i < nbuckets {
                let b = (*t).storage.wrapping_add(i);
                let mut j: usize = 0;
                while j < STBDS_BUCKET_LENGTH {
                    (*b).hash[j] = STBDS_HASH_EMPTY;
                    j += 1;
                }
                let mut j: usize = 0;
                while j < STBDS_BUCKET_LENGTH {
                    (*b).index[j] = STBDS_INDEX_EMPTY;
                    j += 1;
                }
                i += 1;
            }
        }

        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            let nbuckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
            let mut i: usize = 0;
            while i < nbuckets {
                let ob = (*ot).storage.wrapping_add(i);
                let mut j: usize = 0;
                while j < STBDS_BUCKET_LENGTH {
                    if (*ob).index[j] >= 0 {
                        let hash = (*ob).hash[j];
                        let mut pos =
                            stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                        let mut step = STBDS_BUCKET_LENGTH;
                        'done: loop {
                            let bucket = (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                            let mut z: usize = 0;
                            while z < limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'done;
                                }
                                z += 1;
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
}

// ---------------------------------------------------------------------------
// stbds_hash_string
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash: usize = seed;
        let mut s = str_ as *const u8;
        while *s != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*s as usize);
            s = s.wrapping_add(1);
        }

        hash ^= seed;
        hash = (!hash).wrapping_add(hash << 18);
        hash = hash ^ hash ^ rotate_right(hash, 31);
        hash = hash.wrapping_mul(21);
        hash = hash ^ hash ^ rotate_right(hash, 11);
        hash = hash.wrapping_add(hash << 6);
        hash ^= rotate_right(hash, 22);
        hash.wrapping_add(seed)
    }
}

// ---------------------------------------------------------------------------
// SipHash
// ---------------------------------------------------------------------------

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! stbds_sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotate_left($v1, 13);
        $v1 ^= $v0;
        $v0 = rotate_left($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotate_left($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotate_left($v1, 17);
        $v1 ^= $v2;
        $v2 = rotate_left($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotate_left($v3, 21);
        $v3 ^= $v0;
    }};
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
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

        i = 0;
        while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
            // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
            //
            // NOTE: in C this expression has type `int`; if d[3] >= 0x80 the
            // result is negative and the conversion to `size_t` sign-extends.
            // That behaviour is reproduced verbatim here.
            let lo: u32 = (*d.wrapping_add(0) as u32)
                | ((*d.wrapping_add(1) as u32) << 8)
                | ((*d.wrapping_add(2) as u32) << 16)
                | ((*d.wrapping_add(3) as u32) << 24);
            data = (lo as i32) as usize;

            // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
            let hi: u32 = (*d.wrapping_add(4) as u32)
                | ((*d.wrapping_add(5) as u32) << 8)
                | ((*d.wrapping_add(6) as u32) << 16)
                | ((*d.wrapping_add(7) as u32) << 24);
            data |= (((hi as i32) as usize) << 16) << 16;

            v3 ^= data;
            j = 0;
            while j < STBDS_SIPHASH_C_ROUNDS {
                stbds_sipround!(v0, v1, v2, v3);
                j += 1;
            }
            v0 ^= data;

            i = i.wrapping_add(core::mem::size_of::<usize>());
            d = d.wrapping_add(core::mem::size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len.wrapping_sub(i);
        // switch (len - i) with fall-through from case 7 down to case 1.
        if rem >= 7 {
            data |= ((*d.wrapping_add(6) as usize) << 24) << 24;
        }
        if rem >= 6 {
            data |= ((*d.wrapping_add(5) as usize) << 20) << 20;
        }
        if rem >= 5 {
            data |= ((*d.wrapping_add(4) as usize) << 16) << 16;
        }
        if rem >= 4 {
            // `d[3] << 24` is an `int` in C -> may sign-extend when widened.
            data |= (((*d.wrapping_add(3) as u32) << 24) as i32) as usize;
        }
        if rem >= 3 {
            data |= (*d.wrapping_add(2) as usize) << 16;
        }
        if rem >= 2 {
            data |= (*d.wrapping_add(1) as usize) << 8;
        }
        if rem >= 1 {
            data |= *d.wrapping_add(0) as usize;
        }

        v3 ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround!(v0, v1, v2, v3);
            j += 1;
        }
        v0 ^= data;
        v2 ^= 0xff;
        j = 0;
        while j < STBDS_SIPHASH_D_ROUNDS {
            stbds_sipround!(v0, v1, v2, v3);
            j += 1;
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// stbds_is_key_equal
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
    unsafe {
        if mode >= STBDS_HM_STRING {
            let stored = *(elem_at(a, elemsize, i, keyoffset) as *mut *mut c_char);
            (strcmp(key as *const c_char, stored) == 0) as c_int
        } else {
            (memcmp(
                key as *const c_void,
                elem_at(a, elemsize, i, keyoffset) as *const c_void,
                keysize,
            ) == 0) as c_int
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
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    stbds_free(*(elem_at(a, elemsize, i, 0) as *mut *mut c_char) as *mut c_void);
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
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = stbds_hash_table(raw_a);
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut pos: usize;
        let mut bucket: *mut stbds_hash_bucket;

        if hash < 2 {
            hash = hash.wrapping_add(2);
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        loop {
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                        return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
                i += 1;
            }

            let limit = pos & STBDS_BUCKET_MASK;
            let mut i: usize = 0;
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
                    let b = (*table)
                        .storage
                        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
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
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &raw mut temp, mode);
        set_stbds_temp(stbds_hash_to_arr(p, elemsize), temp);
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
        if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
            let old = if !a.is_null() {
                stbds_hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            };
            a = stbds_arrgrowf(old, elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            memset(a, 0, elemsize);
            a = stbds_arr_to_hash(a, elemsize);
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
                stbds_free(table as *mut c_void);
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
                hash = hash.wrapping_add(2);
            }

            pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

            'found_empty_slot: loop {
                bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                            set_stbds_temp(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let k = *(elem_at(
                                    raw_a,
                                    elemsize,
                                    (*bucket).index[i] as usize,
                                    keyoffset,
                                ) as *mut *mut c_char);
                                set_stbds_temp_key(a, k);
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

                let limit = pos & STBDS_BUCKET_MASK;
                let mut i: usize = 0;
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
                            // NOTE: unlike the loop above, the original C does
                            // *not* update stbds_temp_key here.
                            set_stbds_temp(a, (*bucket).index[i]);
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

                // STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a));
                assert!(
                    (i as usize).wrapping_add(1) <= stbds_arrcap(a),
                    "stbds_hmput_key: i+1 <= stbds_arrcap(a)"
                );
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                set_stbds_temp(a, i - 1);

                let dst = elem_at(a, elemsize, i as usize, 0) as *mut *mut c_char;
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *dst = p;
                        set_stbds_temp_key(a, p);
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                        *dst = p;
                        set_stbds_temp_key(a, p);
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *dst = p;
                        set_stbds_temp_key(a, p);
                    }
                    _ => {
                        memcpy(
                            elem_at(a, elemsize, i as usize, 0) as *mut c_void,
                            key as *const c_void,
                            keysize,
                        );
                    }
                }
            }
            stbds_arr_to_hash(a, elemsize)
        }
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
        (*h).string.mode = mode as u8;
        stbds_arr_to_hash(a, elemsize)
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

        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        set_stbds_temp(raw_a, 0);
        if table.is_null() {
            return a;
        }

        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }

        let mut b = (*table)
            .storage
            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        let old_index: isize = (*b).index[i as usize];
        let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
        // STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count);
        assert!(
            (slot as usize) < (*table).slot_count,
            "stbds_hmdel_key: slot < table->slot_count"
        );
        (*table).used_count = (*table).used_count.wrapping_sub(1);
        (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
        set_stbds_temp(raw_a, 1);
        // STBDS_ASSERT(table->used_count >= 0);  -- always true for size_t in C
        (*b).hash[i as usize] = STBDS_HASH_DELETED;
        (*b).index[i as usize] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            stbds_free(
                *(elem_at(a, elemsize, old_index as usize, 0) as *mut *mut c_char) as *mut c_void,
            );
        }

        if old_index != final_index {
            memmove(
                elem_at(a, elemsize, old_index as usize, 0) as *mut c_void,
                elem_at(a, elemsize, final_index as usize, 0) as *const c_void,
                elemsize,
            );

            if mode == STBDS_HM_STRING {
                let k = *(elem_at(a, elemsize, old_index as usize, keyoffset)
                    as *mut *mut c_char);
                slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
            } else {
                let k = elem_at(a, elemsize, old_index as usize, keyoffset);
                slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
            }
            // STBDS_ASSERT(slot >= 0);
            assert!(slot >= 0, "stbds_hmdel_key: slot >= 0");
            b = (*table)
                .storage
                .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
            // STBDS_ASSERT(b->index[i] == final_index);
            assert!(
                (*b).index[i as usize] == final_index,
                "stbds_hmdel_key: b->index[i] == final_index"
            );
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

// ---------------------------------------------------------------------------
// stbds_strdup / stbds_stralloc / stbds_strreset
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
        let p: *mut c_char;
        let len = strlen(str_).wrapping_add(1);
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
                    core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        // STBDS_ASSERT(len <= a->remaining);
        assert!(len <= (*a).remaining, "stbds_stralloc: len <= a->remaining");
        p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .wrapping_add((*a).remaining as isize as usize)
            .wrapping_sub(len);
        (*a).remaining = (*a).remaining.wrapping_sub(len);
        memmove(p as *mut c_void, str_ as *const c_void, len);
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
            stbds_free(x as *mut c_void);
            x = y;
        }
        memset(
            a as *mut c_void,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
    }
}

// ---------------------------------------------------------------------------
// strkey
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        // sprintf(buffer, "test_%d", n);
        let buf = (&raw mut buffer) as *mut c_char as *mut u8;
        let mut out: usize = 0;
        for &c in b"test_" {
            *buf.wrapping_add(out) = c;
            out += 1;
        }
        // %d formatting of a C `int`
        let mut v = n as i32;
        if v < 0 {
            *buf.wrapping_add(out) = b'-';
            out += 1;
        }
        // Collect decimal digits of |v| without overflowing on i32::MIN.
        let mut digits = [0u8; 12];
        let mut nd = 0usize;
        if v == 0 {
            digits[0] = b'0';
            nd = 1;
        } else {
            while v != 0 {
                let d = (v % 10).unsigned_abs() as u8;
                digits[nd] = b'0' + d;
                nd += 1;
                v /= 10;
            }
        }
        while nd > 0 {
            nd -= 1;
            *buf.wrapping_add(out) = digits[nd];
            out += 1;
        }
        *buf.wrapping_add(out) = 0;

        (&raw mut buffer) as *mut c_char
    }
}

// ---------------------------------------------------------------------------
// arr_del  (the public entry point declared in include/lib.h)
// ---------------------------------------------------------------------------

/// `stbds_arrmaybegrow(a,1)` + `a[stbds_header(a)->length++] = v` for `int *`.
#[inline]
unsafe fn arrpush_int(arr: &mut *mut c_int, v: c_int) {
    unsafe {
        let elemsize = core::mem::size_of::<c_int>();
        if arr.is_null()
            || (*stbds_header(*arr as *mut c_void)).length + 1
                > (*stbds_header(*arr as *mut c_void)).capacity
        {
            *arr = stbds_arrgrowf(*arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
        }
        let h = stbds_header(*arr as *mut c_void);
        let idx = (*h).length;
        (*h).length = idx + 1;
        *arr.wrapping_add(idx) = v;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    unsafe {
        let elemsize = core::mem::size_of::<c_int>();
        let mut arr: *mut c_int = ptr::null_mut();

        let mut i: c_int = 0;
        while i < 4 {
            // arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
            arrpush_int(&mut arr, num);
            arrpush_int(&mut arr, 2);
            arrpush_int(&mut arr, 3);
            arrpush_int(&mut arr, 4);

            // arrdel(arr,i) -> arrdeln(arr,i,1)
            {
                let h = stbds_header(arr as *mut c_void);
                let n: usize = 1;
                let count = (*h).length.wrapping_sub(n).wrapping_sub(i as usize);
                memmove(
                    arr.wrapping_add(i as usize) as *mut c_void,
                    arr.wrapping_add((i as usize).wrapping_add(n)) as *const c_void,
                    elemsize.wrapping_mul(count),
                );
                (*h).length = (*h).length.wrapping_sub(n);
            }

            // arrfree(arr)
            if !arr.is_null() {
                stbds_free(stbds_header(arr as *mut c_void) as *mut c_void);
            }
            arr = ptr::null_mut();

            arrpush_int(&mut arr, num);
            arrpush_int(&mut arr, 2);
            arrpush_int(&mut arr, 3);
            arrpush_int(&mut arr, 4);

            // arrdelswap(arr,i) -> a[i] = arrlast(a), header(a)->length -= 1
            {
                let h = stbds_header(arr as *mut c_void);
                let last = *arr.wrapping_add((*h).length.wrapping_sub(1));
                *arr.wrapping_add(i as usize) = last;
                (*h).length = (*h).length.wrapping_sub(1);
            }

            // arrfree(arr)
            if !arr.is_null() {
                stbds_free(stbds_header(arr as *mut c_void) as *mut c_void);
            }
            arr = ptr::null_mut();

            i += 1;
        }
    }
}
