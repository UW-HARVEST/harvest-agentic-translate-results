//! Rust translation of the C library in `c_src/` (stb_ds.h implementation part
//! plus the small test helpers `strkey` / `arr_ins`).
//!
//! The translation is deliberately literal: every arithmetic wrap-around,
//! integer promotion quirk and evaluation order of the original C code is
//! reproduced so that the resulting shared object behaves byte-for-byte like
//! the C shared object.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]
#![allow(private_interfaces)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// C runtime helpers (STBDS_REALLOC / STBDS_FREE map to realloc / free)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn abort() -> !;
}

/// `STBDS_ASSERT` is `assert` from <assert.h>. The library as built by
/// `c_src/CMakeLists.txt` is compiled without `NDEBUG`, so a failing assertion
/// aborts the process (glibc's `abort()` does not flush stdio buffers, exactly
/// like the C build).
#[inline]
unsafe fn stbds_assert(cond: bool) {
    if !cond {
        abort();
    }
}

/// `STBDS_REALLOC(c, p, s)` == `realloc(p, s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    realloc(p, size)
}

/// `STBDS_FREE(c, p)` == `free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    free(p)
}

// ---------------------------------------------------------------------------
// Structures (layout identical to the C originals)
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
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
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

// enum { STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA };
const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[inline]
fn STBDS_INDEX_IN_USE(x: isize) -> bool {
    x >= 0
}

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const HEADER_SIZE: usize = size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Small pointer / macro helpers
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *) (t) - 1)`
#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `STBDS_HASH_TO_ARR(x, elemsize)` == `((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x, elemsize)` == `((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `stbds_arrcap(a)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    }
}

/// `stbds_arrlen(a)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if !a.is_null() {
        (*stbds_header(a)).length as isize
    } else {
        0
    }
}

/// `stbds_temp(t)` (read)
#[inline]
unsafe fn stbds_temp_get(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

/// `stbds_temp(t) = v`
#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `stbds_temp_key(t) = v`  (== `*(char **) stbds_header(t)->hash_table = v`)
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *ht = v;
}

/// `stbds_hash_table(a)` == `((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline]
fn byte_ptr(p: *mut c_void, off: usize) -> *mut u8 {
    (p as *mut u8).wrapping_add(off)
}

#[inline]
unsafe fn c_memset(p: *mut c_void, value: u8, n: usize) {
    ptr::write_bytes(p as *mut u8, value, n);
}

#[inline]
unsafe fn c_memmove(dst: *mut c_void, src: *const c_void, n: usize) {
    ptr::copy(src as *const u8, dst as *mut u8, n);
}

#[inline]
unsafe fn c_memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int {
    let mut i = 0usize;
    while i < n {
        let x = *(a as *const u8).add(i);
        let y = *(b as *const u8).add(i);
        if x != y {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
    0
}

#[inline]
unsafe fn c_strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0usize;
    loop {
        let x = *(a as *const u8).add(i);
        let y = *(b as *const u8).add(i);
        if x != y {
            return x as c_int - y as c_int;
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}

#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *(s as *const u8).add(n) != 0 {
        n += 1;
    }
    n
}

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
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
    let mut b: *mut c_void;
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

    b = stbds_realloc(
        if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        },
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    );
    b = (b as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    } else {
        // STBDS_STATS(++stbds_array_grow);
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    stbds_free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// hash seed / hash index
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

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

/// `STBDS_ALIGN_FWD(n, a)` == `(((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a).wrapping_sub(1) & !(a.wrapping_sub(1))
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t: *mut stbds_hash_index = stbds_realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(size_of::<stbds_hash_bucket>())
            .wrapping_add(size_of::<stbds_hash_index>())
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
    stbds_assert(
        (*t)
            .used_count_threshold
            .wrapping_add((*t).tombstone_count_threshold)
            < (*t).slot_count,
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        c_memset(
            (&mut (*t).string) as *mut stbds_string_arena as *mut c_void,
            0,
            size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let a = stbds_load_32_or_64(2147001325u32, 0x27bb2ee6u32, 0x87b0b0fdu32);
        // stbds_load_32_or_64(b, temp,  715136305,          0, 0xb504f32d);
        let b = stbds_load_32_or_64(715136305u32, 0u32, 0xb504f32du32);
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
                if STBDS_INDEX_IN_USE((*ob).index[j]) {
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

/// ```c
/// #define stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)     \
///   temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16, \
///   var = v64_hi, var <<= 16, var <<= 16,                         \
///   var ^= temp ^ v32
/// ```
/// Note: `v64_lo ^ v32` is computed in 32-bit `unsigned int` arithmetic in C
/// (the hex literals do not fit in `int`, the decimal ones do), then widened.
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
// hash functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut str_ = str_ as *const u8;
    while *str_ != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*str_ as usize);
        str_ = str_.add(1);
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

macro_rules! stbds_sipround {
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = stbds_rotate_left($v1, 13);
        $v1 ^= $v0;
        $v0 = stbds_rotate_left($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = stbds_rotate_left($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = stbds_rotate_left($v1, 17);
        $v1 ^= $v2;
        $v2 = stbds_rotate_left($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = stbds_rotate_left($v3, 21);
        $v3 ^= $v0;
    }};
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

    v0 = ((((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed) as usize;
    v1 = ((((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed) as usize;
    v2 = ((((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed) as usize;
    v3 = ((((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed) as usize;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    i = 0;
    while i.wrapping_add(size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // (computed as `int` in C, then sign-extended into size_t)
        let lo: c_int = (*d.add(0) as c_int)
            | ((*d.add(1) as c_int) << 8)
            | ((*d.add(2) as c_int) << 16)
            | ((*d.add(3) as c_int) << 24);
        data = lo as isize as usize;
        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: c_int = (*d.add(4) as c_int)
            | ((*d.add(5) as c_int) << 8)
            | ((*d.add(6) as c_int) << 16)
            | ((*d.add(7) as c_int) << 24);
        data |= ((hi as isize as usize) << 16) << 16;

        v3 ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround!(v0, v1, v2, v3);
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(size_of::<usize>());
        d = d.add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) with C fall-through semantics
    let rem = len.wrapping_sub(i);
    if rem == 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 && rem <= 7 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 && rem <= 7 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 && rem <= 7 {
        // data |= (d[3] << 24);   -- `int` arithmetic, sign-extended
        data |= (((*d.add(3) as c_int) << 24) as isize) as usize;
    }
    if rem >= 3 && rem <= 7 {
        data |= (((*d.add(2) as c_int) << 16) as isize) as usize;
    }
    if rem >= 2 && rem <= 7 {
        data |= (((*d.add(1) as c_int) << 8) as isize) as usize;
    }
    if rem >= 1 && rem <= 7 {
        data |= ((*d.add(0) as c_int) as isize) as usize;
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
        let slot = byte_ptr(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *mut *mut c_char;
        (0 == c_strcmp(key as *const c_char, *slot)) as c_int
    } else {
        (0 == c_memcmp(
            key as *const c_void,
            byte_ptr(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *const c_void,
            keysize,
        )) as c_int
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
                stbds_free(*(byte_ptr(a, elemsize.wrapping_mul(i)) as *mut *mut c_void));
                i += 1;
            }
        }
        stbds_strreset(&mut (*stbds_hash_table(a)).string);
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
    let mut a = a;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        c_memset(a, 0, elemsize);
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
                let b: *mut stbds_hash_bucket =
                    (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
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
    stbds_temp_set(stbds_hash_to_arr(p, elemsize), temp);
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
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        c_memset(a, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c_strlen(str_).wrapping_add(1);
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    c_memmove(p as *mut c_void, str_ as *const c_void, len);
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
    let keyoffset: usize = 0;
    let mut a = a;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        c_memset(a, 0, elemsize);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
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
                        stbds_temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let src = byte_ptr(
                                raw_a,
                                elemsize
                                    .wrapping_mul((*bucket).index[i] as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char;
                            stbds_temp_key_set(a, *src);
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

            stbds_assert((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_temp_set(a, i - 1);

            let slot = byte_ptr(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    *slot = v;
                    stbds_temp_key_set(a, v);
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                    *slot = v;
                    stbds_temp_key_set(a, v);
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    *slot = v;
                    stbds_temp_key_set(a, v);
                }
                _ => {
                    c_memmove(
                        byte_ptr(a, elemsize.wrapping_mul(i as usize)) as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
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
    c_memset(a, 0, elemsize);
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
        stbds_temp_set(raw_a, 0);
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
                let mut i: c_int = (slot as usize & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                stbds_assert(slot < (*table).slot_count as isize);
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                stbds_temp_set(raw_a, 1);
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
                    stbds_free(
                        *(byte_ptr(a, elemsize.wrapping_mul(old_index as usize))
                            as *mut *mut c_void),
                    );
                }

                if old_index != final_index {
                    c_memmove(
                        byte_ptr(a, elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
                        byte_ptr(a, elemsize.wrapping_mul(final_index as usize)) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        let kp = *(byte_ptr(
                            a,
                            elemsize
                                .wrapping_mul(old_index as usize)
                                .wrapping_add(keyoffset),
                        ) as *mut *mut c_char);
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            kp as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            byte_ptr(
                                a,
                                elemsize
                                    .wrapping_mul(old_index as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    stbds_assert(slot >= 0);
                    b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = (slot as usize & STBDS_BUCKET_MASK) as c_int;
                    stbds_assert((*b).index[i as usize] == final_index);
                    (*b).index[i as usize] = old_index;
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
    let len = c_strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb: *mut stbds_string_block = stbds_realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(len),
            ) as *mut stbds_string_block;
            c_memmove(
                (*sb).storage.as_mut_ptr() as *mut c_void,
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
            return (*sb).storage.as_mut_ptr();
        } else {
            let sb: *mut stbds_string_block = stbds_realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(blocksize),
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    stbds_assert(len <= (*a).remaining);
    p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len) as *mut c_char;
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    c_memmove(p as *mut c_void, str_ as *const c_void, len);
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
    c_memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// test helpers exported by the C library
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    // sprintf(buffer, "test_%d", n);
    let buf = (&raw mut buffer) as *mut u8;
    let mut tmp = [0u8; 32];
    let mut pos = 0usize;
    for &b in b"test_" {
        tmp[pos] = b;
        pos += 1;
    }
    // %d formatting of a C int
    let mut num = n as i64;
    if num < 0 {
        tmp[pos] = b'-';
        pos += 1;
        num = -num;
    }
    let start = pos;
    if num == 0 {
        tmp[pos] = b'0';
        pos += 1;
    } else {
        let mut digits = [0u8; 20];
        let mut nd = 0usize;
        while num > 0 {
            digits[nd] = b'0' + (num % 10) as u8;
            nd += 1;
            num /= 10;
        }
        while nd > 0 {
            nd -= 1;
            tmp[pos] = digits[nd];
            pos += 1;
        }
    }
    let _ = start;
    ptr::copy_nonoverlapping(tmp.as_ptr(), buf, pos);
    *buf.add(pos) = 0;
    buf as *mut c_char
}

// --- helpers reproducing the stb_ds array macros for `int *` arrays ---------

#[inline]
unsafe fn i32_arrmaybegrow(arr: &mut *mut c_int, n: usize) {
    let a = *arr as *mut c_void;
    if a.is_null()
        || (*stbds_header(a)).length.wrapping_add(n) > (*stbds_header(a)).capacity
    {
        *arr = stbds_arrgrowf(a, size_of::<c_int>(), n, 0) as *mut c_int;
    }
}

/// `arrpush(arr, v)`
#[inline]
unsafe fn i32_arrpush(arr: &mut *mut c_int, v: c_int) {
    i32_arrmaybegrow(arr, 1);
    let h = stbds_header(*arr as *mut c_void);
    let len = (*h).length;
    *(*arr).add(len) = v;
    (*h).length = len.wrapping_add(1);
}

/// `stbds_arrins(arr, i, v)`
#[inline]
unsafe fn i32_arrins(arr: &mut *mut c_int, i: usize, v: c_int) {
    // stbds_arrinsn(a,i,n): (stbds_arraddn(a,n), memmove(...))
    // stbds_arraddn(a,1) -> maybegrow + length += 1
    i32_arrmaybegrow(arr, 1);
    let h = stbds_header(*arr as *mut c_void);
    (*h).length = (*h).length.wrapping_add(1);
    let count = size_of::<c_int>().wrapping_mul((*h).length.wrapping_sub(1).wrapping_sub(i));
    c_memmove(
        (*arr).add(i + 1) as *mut c_void,
        (*arr).add(i) as *const c_void,
        count,
    );
    *(*arr).add(i) = v;
}

/// `arrfree(arr)`
#[inline]
unsafe fn i32_arrfree(arr: &mut *mut c_int) {
    let a = *arr as *mut c_void;
    if !a.is_null() {
        stbds_free(stbds_header(a) as *mut c_void);
    }
    *arr = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let mut i: c_int = 0;

    while i < 5 {
        i32_arrpush(&mut arr, 1);
        i32_arrpush(&mut arr, 2);
        i32_arrpush(&mut arr, 3);
        i32_arrpush(&mut arr, 4);
        i32_arrins(&mut arr, i as usize, num);
        stbds_assert(*arr.add(i as usize) == num);
        if i < 4 {
            stbds_assert(*arr.add(4) == 4);
        }
        i32_arrfree(&mut arr);
        i += 1;
    }
}
