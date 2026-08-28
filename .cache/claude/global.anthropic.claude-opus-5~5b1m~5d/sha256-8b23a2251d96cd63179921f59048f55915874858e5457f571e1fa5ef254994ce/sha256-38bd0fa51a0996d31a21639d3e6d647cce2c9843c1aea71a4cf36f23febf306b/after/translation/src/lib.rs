//! Rust translation of the C library in `c_src/`.
//!
//! The C library is a single translation unit (`src/lib.c`) that embeds the
//! implementation part of `stb_ds.h` (public domain, see `c_src/license.txt`)
//! together with two extra functions (`strkey`, `helxo`).
//!
//! Every public symbol exported by the C shared object is re-exported here with
//! the identical linker name, signature and observable behaviour (including the
//! quirks / latent bugs of the original code, which are reproduced verbatim).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings
//
// The C code allocates through `realloc`/`free` (STBDS_REALLOC / STBDS_FREE),
// so the Rust translation must use the very same allocator: callers of this
// library free array/hash-map headers with plain `free()` through the
// `stbds_arrfree` / `stbds_hmfree` macros.
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

// ---------------------------------------------------------------------------
// Data structures (layout-identical to the C ones)
// ---------------------------------------------------------------------------

/// `typedef struct { size_t length; size_t capacity; void *hash_table; ptrdiff_t temp; } stbds_array_header;`
#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

/// `typedef struct stbds_string_block { struct stbds_string_block *next; char storage[8]; } stbds_string_block;`
#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

/// `struct stbds_string_arena { stbds_string_block *storage; size_t remaining; unsigned char block; unsigned char mode; };`
#[repr(C)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

/// `typedef struct { size_t hash[8]; ptrdiff_t index[8]; } stbds_hash_bucket;`
#[repr(C)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

/// `typedef struct { ... } stbds_hash_index;`
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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // (STBDS_BUCKET_LENGTH == 8 ? 3 : 2)
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

#[allow(dead_code)]
const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Small helpers mirroring the C macros
// ---------------------------------------------------------------------------

/// `#define stbds_header(t)  ((stbds_array_header *) (t) - 1)`
#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `#define stbds_arrlen(a)  ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `#define stbds_arrcap(a)  ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `#define stbds_temp(t)    stbds_header(t)->temp`
#[inline]
unsafe fn stbds_temp_get(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
}

/// `#define stbds_hash_table(a)  ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline]
fn STBDS_HASH_TO_ARR(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline]
fn STBDS_ARR_TO_HASH(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// `#define STBDS_ROTATE_LEFT(val, n) (((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n))))`
#[inline]
fn STBDS_ROTATE_LEFT(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n) (((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n))))`
#[inline]
fn STBDS_ROTATE_RIGHT(val: usize, n: u32) -> usize {
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
    mut min_cap: usize,
) -> *mut c_void {
    let b: *mut c_void;
    let min_len: usize = (stbds_arrlen(a) as usize).wrapping_add(addlen);

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

    let old: *mut c_void = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };
    let raw = realloc(old, elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE));
    b = (raw as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;
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
// hash seed / rng
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

/// ```c
/// #define stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)   \
///   temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16, \
///   var = v64_hi, var <<= 16, var <<= 16,                       \
///   var ^= temp ^ v32
/// ```
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // In C `v64_lo ^ v32` is computed in `unsigned int` (32 bits) and then
    // widened to size_t, so the intermediate is naturally 32 bits wide.
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
// hash index construction
// ---------------------------------------------------------------------------

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
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
    let t: *mut stbds_hash_index = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = STBDS_ALIGN_FWD(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE)
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
    // STBDS_ASSERT(t->used_count_threshold + t->tombstone_count_threshold < t->slot_count);

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
            let ob: *mut stbds_hash_bucket = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

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

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut str_ = str_;
    while *str_ != 0 {
        hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*str_ as u8 as usize);
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

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
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

    i = 0;
    while i + core::mem::size_of::<usize>() <= len {
        // `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);`
        //
        // In C this expression has type `int`; when d[3] >= 0x80 the result is
        // negative and the conversion to size_t sign-extends it, setting the
        // whole upper half of `data`.  That behaviour is reproduced here.
        let w0: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | (((*d.add(3) as u32) << 24) as i32);
        data = w0 as i64 as usize;

        let w1: i32 = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | (((*d.add(7) as u32) << 24) as i32);
        data |= ((w1 as i64 as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            STBDS_SIPROUND!();
        }
        v0 ^= data;

        i += core::mem::size_of::<usize>();
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) with C fall-through semantics
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
        // `data |= (d[3] << 24);` -- int expression, sign-extended into size_t
        data |= ((((*d.add(3) as u32) << 24) as i32) as i64) as usize;
    }
    if rem >= 3 {
        data |= (((*d.add(2) as i32) << 16) as i64) as usize;
    }
    if rem >= 2 {
        data |= (((*d.add(1) as i32) << 8) as i64) as usize;
    }
    if rem >= 1 {
        data |= *d.add(0) as usize;
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
    if mode >= STBDS_HM_STRING {
        let stored = *((a as *mut u8)
            .wrapping_offset(elemsize.wrapping_mul(i as usize) as isize)
            .wrapping_add(keyoffset) as *mut *mut c_char);
        (0 == strcmp(key as *mut c_char, stored)) as c_int
    } else {
        (0 == memcmp(
            key,
            (a as *mut u8)
                .wrapping_offset(elemsize.wrapping_mul(i as usize) as isize)
                .wrapping_add(keyoffset) as *const c_void,
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
                free(*((a as *mut u8).wrapping_add(elemsize.wrapping_mul(i)) as *mut *mut c_char)
                    as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));
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
        hash += 2;
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
    stbds_temp_set(STBDS_HASH_TO_ARR(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
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
        memset(a, 0, elemsize);
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }
    a
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
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;
    let mut a = a;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
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
            (*table).slot_count.wrapping_mul(2)
        };
        nt = stbds_make_hash_index(slot_count, table);
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
                        (*bucket).index[i],
                    ) != 0
                    {
                        stbds_temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let stored = *((raw_a as *mut u8)
                                .wrapping_offset(
                                    elemsize.wrapping_mul((*bucket).index[i] as usize) as isize,
                                )
                                .wrapping_add(keyoffset) as *mut *mut c_char);
                            stbds_temp_key_set(a, stored);
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
            let _ = raw_a;

            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_temp_set(a, i - 1);

            let slot = (a as *mut u8).wrapping_offset(elemsize.wrapping_mul(i as usize) as isize)
                as *mut *mut c_char;
            match (*table).string.mode as c_int {
                x if x == STBDS_SH_STRDUP => {
                    *slot = stbds_strdup(key as *mut c_char);
                    stbds_temp_key_set(a, *slot);
                }
                x if x == STBDS_SH_ARENA => {
                    *slot = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    stbds_temp_key_set(a, *slot);
                }
                x if x == STBDS_SH_DEFAULT => {
                    *slot = key as *mut c_char;
                    stbds_temp_key_set(a, *slot);
                }
                _ => {
                    memcpy(
                        (a as *mut u8).wrapping_offset(elemsize.wrapping_mul(i as usize) as isize)
                            as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        STBDS_ARR_TO_HASH(a, elemsize)
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
        ptr::null_mut()
    } else {
        let table: *mut stbds_hash_index;
        let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
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
                    (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                (*table).used_count -= 1;
                (*table).tombstone_count += 1;
                stbds_temp_set(raw_a, 1);
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
                    free(*((a as *mut u8)
                        .wrapping_offset(elemsize.wrapping_mul(old_index as usize) as isize)
                        as *mut *mut c_char) as *mut c_void);
                }

                if old_index != final_index {
                    memmove(
                        (a as *mut u8)
                            .wrapping_offset(elemsize.wrapping_mul(old_index as usize) as isize)
                            as *mut c_void,
                        (a as *mut u8)
                            .wrapping_offset(elemsize.wrapping_mul(final_index as usize) as isize)
                            as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        let k = *((a as *mut u8)
                            .wrapping_offset(elemsize.wrapping_mul(old_index as usize) as isize)
                            .wrapping_add(keyoffset) as *mut *mut c_char);
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            k as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        let k = (a as *mut u8)
                            .wrapping_offset(elemsize.wrapping_mul(old_index as usize) as isize)
                            .wrapping_add(keyoffset);
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            k as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
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
    }
}

// ---------------------------------------------------------------------------
// string arena
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

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
            let sb = realloc(
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
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    // STBDS_ASSERT(len <= a->remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len);
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
        free(x as *mut c_void);
        x = y;
    }
    memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}

// ---------------------------------------------------------------------------
// `strkey` and `helxo`
// ---------------------------------------------------------------------------

/// `static char buffer[256];`
static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    sprintf(
        ptr::addr_of_mut!(buffer) as *mut c_char,
        c"test_%d".as_ptr(),
        n,
    );
    ptr::addr_of_mut!(buffer) as *mut c_char
}

/// The anonymous struct used by `helxo`: `struct { char *key; char value; }`
#[repr(C)]
struct HelxoEntry {
    key: *mut c_char,
    value: c_char,
}

/// `shput(hash, k, v)` expanded for `HelxoEntry`:
/// ```c
/// (t) = stbds_hmput_key((t), sizeof *(t), (void*)(k), sizeof (t)->key, STBDS_HM_STRING),
/// (t)[stbds_temp((t)-1)].value = (v)
/// ```
unsafe fn helxo_shput(t: &mut *mut HelxoEntry, k: *mut c_char, v: c_char) {
    *t = stbds_hmput_key(
        *t as *mut c_void,
        core::mem::size_of::<HelxoEntry>(),
        k as *mut c_void,
        core::mem::size_of::<*mut c_char>(),
        STBDS_HM_STRING,
    ) as *mut HelxoEntry;
    let idx = stbds_temp_get(t.wrapping_offset(-1) as *mut c_void);
    (*t.wrapping_offset(idx)).value = v;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    {
        let mut hash: *mut HelxoEntry = ptr::null_mut();
        let mut name: [c_char; 4] = [
            'j' as u8 as c_char,
            'e' as u8 as c_char,
            'n' as u8 as c_char,
            0,
        ];
        helxo_shput(&mut hash, c"bob".as_ptr() as *mut c_char, 'h' as u8 as c_char);
        helxo_shput(
            &mut hash,
            c"sally".as_ptr() as *mut c_char,
            'e' as u8 as c_char,
        );
        helxo_shput(
            &mut hash,
            c"fred".as_ptr() as *mut c_char,
            'l' as u8 as c_char,
        );
        helxo_shput(&mut hash, c"jen".as_ptr() as *mut c_char, 'x' as u8 as c_char);
        helxo_shput(
            &mut hash,
            c"doug".as_ptr() as *mut c_char,
            'o' as u8 as c_char,
        );

        helxo_shput(&mut hash, name.as_mut_ptr(), letter);

        // for (int z=0; z < shlen(hash); ++z)
        //     printf("%s %c\n", hash[z], hash[z].value);
        //
        // The first variadic argument is the whole 16-byte struct, which the
        // x86-64 SysV ABI passes in two integer registers: the first holds
        // `key` (consumed by %s), the second holds `value` in its low byte
        // (consumed by %c).  The trailing `hash[z].value` argument is never
        // read by the format string.  Passing (key, value) reproduces exactly
        // the same output.
        let len = helxo_shlen(hash);
        let mut z: isize = 0;
        while z < len {
            printf(
                c"%s %c\n".as_ptr(),
                (*hash.offset(z)).key,
                (*hash.offset(z)).value as c_int,
            );
            z += 1;
        }

        // shfree(hash)
        if !hash.is_null() {
            stbds_hmfree_func(
                hash.wrapping_offset(-1) as *mut c_void,
                core::mem::size_of::<HelxoEntry>(),
            );
        }
        hash = ptr::null_mut();
        let _ = hash;
        let _ = &mut name;
    }
}

/// `#define stbds_hmlen(t) ((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)`
unsafe fn helxo_shlen(t: *mut HelxoEntry) -> isize {
    if t.is_null() {
        0
    } else {
        (*stbds_header(t.wrapping_offset(-1) as *mut c_void)).length as isize - 1
    }
}
