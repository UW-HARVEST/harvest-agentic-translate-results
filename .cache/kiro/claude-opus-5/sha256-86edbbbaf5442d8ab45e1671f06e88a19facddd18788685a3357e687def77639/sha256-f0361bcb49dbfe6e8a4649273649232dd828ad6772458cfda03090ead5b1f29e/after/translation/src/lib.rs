//! Rust translation of `c_src/src/lib.c` (an stb_ds.h implementation plus the
//! `strkey` / `intput` helpers).
//!
//! The goal is bit-exact behavioural equivalence with the C original, including
//! its quirks (signed-int overflow in the siphash byte gather, the missing
//! `temp_key` update in the wrap-around half of the put loop, the leak in
//! `intput`, ...). Nothing is "fixed".
//!
//! Only 64-bit `size_t` is supported, matching the C file's
//! `STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds` static assertion.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free/strlen/strcmp/memcmp directly)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
}

/// `#define STBDS_REALLOC(c,p,s) realloc(p,s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}

/// `#define STBDS_FREE(c,p) free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

/// `#define STBDS_ASSERT assert` — asserts are enabled in the C build (no
/// `NDEBUG`), and a failing `assert` aborts the process.
macro_rules! stbds_assert {
    ($cond:expr) => {
        if !($cond) {
            std::process::abort();
        }
    };
}

// ---------------------------------------------------------------------------
// Data layout
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
struct stbds_string_arena {
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
fn stbds_index_in_use(x: isize) -> bool {
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

const HEADER_SIZE: usize = size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Header / array accessor macros
// ---------------------------------------------------------------------------

/// `#define stbds_header(t)  ((stbds_array_header *) (t) - 1)`
#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut u8).sub(HEADER_SIZE) as *mut stbds_array_header }
}

/// `#define stbds_arrlen(a) ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v };
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) = v };
}

/// `#define stbds_hash_table(a)  ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char*) (x) - (elemsize))`
#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char*) (x) + (elemsize))`
#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn elem_ptr(a: *mut c_void, elemsize: usize, i: usize) -> *mut u8 {
    unsafe { (a as *mut u8).add(elemsize.wrapping_mul(i)) }
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

        let old = if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        };
        let raw = stbds_realloc(
            old,
            elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
        );
        let b = (raw as *mut u8).add(HEADER_SIZE) as *mut c_void;
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
// Hash seed / index construction
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { STBDS_HASH_SEED = seed };
}

/// ```c
/// #define stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo) \
///   temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16, \
///   var = v64_hi, var <<= 16, var <<= 16, \
///   var ^= temp ^ v32
/// ```
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

#[inline]
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

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = stbds_realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
                + size_of::<stbds_hash_index>()
                + STBDS_CACHE_LINE_SIZE
                - 1,
        ) as *mut stbds_hash_index;

        (*t).storage = stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE)
            as *mut stbds_hash_bucket;
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
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
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
            ptr::write_bytes(&raw mut (*t).string as *mut u8, 0, size_of::<stbds_string_arena>());
            (*t).seed = STBDS_HASH_SEED;
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
            STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
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
                        'search: loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                            let mut z = pos & STBDS_BUCKET_MASK;
                            while z < STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'search;
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
                                break 'search;
                            }

                            pos = pos.wrapping_add(step);
                            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
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
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut p = str_;
        while *p != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*p as u8 as usize);
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
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! siphash_round {
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {{
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

        let mut v0: usize = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
        let mut v1: usize = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
        let mut v2: usize = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
        let mut v3: usize = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut data: usize;

        let mut i = 0usize;
        while i + size_of::<usize>() <= len {
            // Faithful reproduction of the C expression, including the signed
            // `int` overflow / sign-extension when byte 3 has its top bit set:
            //   data  = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
            //   data |= (size_t)(d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
            let lo: i32 = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | ((*d.add(3) as i32).wrapping_shl(24));
            let hi: i32 = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | ((*d.add(7) as i32).wrapping_shl(24));
            data = lo as isize as usize;
            data |= ((hi as isize as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        // switch (len - i) with fall-through from `case 7` down to `case 1`.
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
            data |= ((*d.add(3) as i32).wrapping_shl(24)) as isize as usize;
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
            siphash_round!(v0, v1, v2, v3);
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            siphash_round!(v0, v1, v2, v3);
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// Key comparison / hash map internals
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
    unsafe {
        let slot = (a as *mut u8)
            .offset(elemsize.wrapping_mul(i as usize) as isize)
            .add(keyoffset);
        if mode >= STBDS_HM_STRING {
            (0 == strcmp(key as *const c_char, *(slot as *mut *mut c_char))) as c_int
        } else {
            (0 == memcmp(key, slot as *const c_void, keysize)) as c_int
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
                    stbds_free(*(elem_ptr(a, elemsize, i) as *mut *mut c_void));
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string as *mut c_void);
        }
        stbds_free((*stbds_header(a)).hash_table);
        stbds_free(stbds_header(a) as *mut c_void);
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
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
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
            ptr::write_bytes(a as *mut u8, 0, elemsize);
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
            (*stbds_header(g)).length += 1;
            ptr::write_bytes(g as *mut u8, 0, elemsize);
            a = arr_to_hash(g, elemsize);
        }
        a
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
        p
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

        if a.is_null() {
            let g = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            ptr::write_bytes(g as *mut u8, 0, elemsize);
            (*stbds_header(g)).length += 1;
            a = arr_to_hash(g, elemsize);
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
                stbds_free(table as *mut c_void);
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
            let mut tombstone: isize = -1;
            let mut bucket: *mut stbds_hash_bucket;

            if hash < 2 {
                hash += 2;
            }

            let mut pos =
                stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

            'probe: loop {
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
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let src = *(elem_ptr(
                                    raw_a,
                                    elemsize,
                                    (*bucket).index[i] as usize,
                                )
                                .add(keyoffset) as *mut *mut c_char);
                                stbds_temp_key_set(a, src);
                            }
                            return arr_to_hash(a, elemsize);
                        }
                    } else if (*bucket).hash[i] == 0 {
                        pos = (pos & !STBDS_BUCKET_MASK) + i;
                        break 'probe;
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
                        ) != 0
                        {
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
                    break 'probe;
                }

                pos = pos.wrapping_add(step);
                step = step.wrapping_add(STBDS_BUCKET_LENGTH);
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
                    a = stbds_arrgrowf(a, elemsize, 1, 0);
                }
                raw_a = arr_to_hash(a, elemsize);
                let _ = raw_a;

                stbds_assert!((i as usize) + 1 <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let dst = elem_ptr(a, elemsize, i as usize) as *mut *mut c_char;
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *dst = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(
                            &raw mut (*table).string as *mut c_void,
                            key as *mut c_char,
                        );
                        *dst = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *dst = p;
                        stbds_temp_key_set(a, p);
                    }
                    _ => {
                        ptr::copy_nonoverlapping(
                            key as *const u8,
                            elem_ptr(a, elemsize, i as usize),
                            keysize,
                        );
                    }
                }
            }
            arr_to_hash(a, elemsize)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        arr_to_hash(a, elemsize)
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
            let raw_a = hash_to_arr(a, elemsize);
            let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
            stbds_temp_set(raw_a, 0);
            if table.is_null() {
                a
            } else {
                let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    a
                } else {
                    let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
                    let old_index = (*b).index[i];
                    let final_index = stbds_arrlen(raw_a) - 1 - 1;
                    stbds_assert!(slot < (*table).slot_count as isize);
                    (*table).used_count -= 1;
                    (*table).tombstone_count += 1;
                    stbds_temp_set(raw_a, 1);
                    stbds_assert!((*table).used_count as isize >= 0);
                    (*b).hash[i] = STBDS_HASH_DELETED;
                    (*b).index[i] = STBDS_INDEX_DELETED;

                    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                        stbds_free(
                            *(elem_ptr(a, elemsize, old_index as usize) as *mut *mut c_void),
                        );
                    }

                    if old_index != final_index {
                        ptr::copy(
                            elem_ptr(a, elemsize, final_index as usize) as *const u8,
                            elem_ptr(a, elemsize, old_index as usize),
                            elemsize,
                        );

                        let moved_key = elem_ptr(a, elemsize, old_index as usize).add(keyoffset);
                        slot = if mode == STBDS_HM_STRING {
                            stbds_hm_find_slot(
                                a,
                                elemsize,
                                *(moved_key as *mut *mut c_void),
                                keysize,
                                keyoffset,
                                mode,
                            )
                        } else {
                            stbds_hm_find_slot(
                                a,
                                elemsize,
                                moved_key as *mut c_void,
                                keysize,
                                keyoffset,
                                mode,
                            )
                        };
                        stbds_assert!(slot >= 0);
                        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                        i = (slot as usize) & STBDS_BUCKET_MASK;
                        stbds_assert!((*b).index[i] == final_index);
                        (*b).index[i] = old_index;
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
}

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut c_void, str_: *mut c_char) -> *mut c_char {
    unsafe {
        let a = a as *mut stbds_string_arena;
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            // `(size_t) 512 << (blocksize >> 1)`; on x86-64 the shift count is
            // taken modulo 64, which `wrapping_shl` reproduces.
            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                // sizeof(*sb) - 8 + len
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                ptr::copy(
                    str_ as *const u8,
                    (&raw mut (*sb).storage) as *mut u8,
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
                    size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        stbds_assert!(len <= (*a).remaining);
        let p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .add((*a).remaining)
            .sub(len);
        (*a).remaining -= len;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut c_void) {
    unsafe {
        let a = a as *mut stbds_string_arena;
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            stbds_free(x as *mut c_void);
            x = y;
        }
        ptr::write_bytes(a as *mut u8, 0, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// strkey / intput
// ---------------------------------------------------------------------------

static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

/// `char *strkey(int n) { sprintf(buffer, "test_%d", n); return buffer; }`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut STRKEY_BUFFER) as *mut c_char;

        let mut digits = [0u8; 20];
        let mut ndigits = 0usize;
        let mut v = (n as i64).unsigned_abs();
        if v == 0 {
            digits[0] = b'0';
            ndigits = 1;
        } else {
            while v > 0 {
                digits[ndigits] = b'0' + (v % 10) as u8;
                ndigits += 1;
                v /= 10;
            }
        }

        let mut o = 0usize;
        for &c in b"test_" {
            *buf.add(o) = c as c_char;
            o += 1;
        }
        if n < 0 {
            *buf.add(o) = b'-' as c_char;
            o += 1;
        }
        let mut k = ndigits;
        while k > 0 {
            k -= 1;
            *buf.add(o) = digits[k] as c_char;
            o += 1;
        }
        *buf.add(o) = 0;

        buf
    }
}

/// The hash-map element type used by `intput`:
/// `struct { int key; int value; }`
#[repr(C)]
struct intput_entry {
    key: c_int,
    value: c_int,
}

const INTPUT_ELEMSIZE: usize = size_of::<intput_entry>();
const INTPUT_KEYSIZE: usize = size_of::<c_int>();

/// `stbds_temp((t)-1)` for the `intput` map.
#[inline]
unsafe fn intput_temp(t: *mut intput_entry) -> isize {
    unsafe { stbds_temp(t.sub(1) as *mut c_void) }
}

/// Expansion of `hmput(intmap, k, v)`.
#[inline]
unsafe fn intput_hmput(t: *mut intput_entry, k: c_int, v: c_int) -> *mut intput_entry {
    unsafe {
        // (t) = stbds_hmput_key((t), sizeof *(t), (void*) (int[1]){k}, sizeof (t)->key, 0)
        let mut key_tmp: [c_int; 1] = [k];
        let t = stbds_hmput_key(
            t as *mut c_void,
            INTPUT_ELEMSIZE,
            key_tmp.as_mut_ptr() as *mut c_void,
            INTPUT_KEYSIZE,
            0,
        ) as *mut intput_entry;
        (*t.offset(intput_temp(t))).key = k;
        (*t.offset(intput_temp(t))).value = v;
        t
    }
}

/// Expansion of `hmget(intmap, k)`; returns the (possibly updated) table and
/// the fetched value.
#[inline]
unsafe fn intput_hmget(t: *mut intput_entry, k: c_int) -> (*mut intput_entry, c_int) {
    unsafe {
        let mut key_tmp: [c_int; 1] = [k];
        let t = stbds_hmget_key(
            t as *mut c_void,
            INTPUT_ELEMSIZE,
            key_tmp.as_mut_ptr() as *mut c_void,
            INTPUT_KEYSIZE,
            STBDS_HM_BINARY,
        ) as *mut intput_entry;
        let v = (*t.offset(intput_temp(t))).value;
        (t, v)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(num: c_int) {
    unsafe {
        let mut intmap: *mut intput_entry = ptr::null_mut();

        intmap = ptr::null_mut();
        intmap = intput_hmput(intmap, num, 7);
        intmap = intput_hmput(intmap, 11, 3);
        intmap = intput_hmput(intmap, 9, num);

        let (m, v) = intput_hmget(intmap, 9);
        intmap = m;
        stbds_assert!(v == num);

        let (m, v) = intput_hmget(intmap, 11);
        intmap = m;
        stbds_assert!(v == 3);

        let (m, v) = intput_hmget(intmap, num);
        intmap = m;
        stbds_assert!(v == 7);

        // The C original never frees `intmap`; neither do we.
        let _ = intmap;
    }
}
