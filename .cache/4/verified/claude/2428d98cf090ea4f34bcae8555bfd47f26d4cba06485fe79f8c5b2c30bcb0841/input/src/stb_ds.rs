//! Literal translation of the stb_ds implementation part of `c_src/src/lib.c`.
//!
//! Every function keeps the exact order of operations, the C integer
//! wrap-around semantics and the (intentional) sign-extension quirks of the
//! original code.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::ffi::*;

// ---------------------------------------------------------------------------
// #defines
// ---------------------------------------------------------------------------

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3;
pub const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
pub const STBDS_CACHE_LINE_SIZE: usize = 64;

pub const STBDS_INDEX_EMPTY: isize = -1;
pub const STBDS_INDEX_DELETED: isize = -2;

pub const STBDS_HASH_EMPTY: usize = 0;
pub const STBDS_HASH_DELETED: usize = 1;

pub const STBDS_SIZE_T_BITS: u32 = 64;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

/// `#define STBDS_INDEX_IN_USE(x) ((x) >= 0)`
#[inline(always)]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a).wrapping_sub(1) & !(a.wrapping_sub(1))
}

/// `#define STBDS_ROTATE_LEFT(val, n) (((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n))))`
#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n) (((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n))))`
#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ---------------------------------------------------------------------------
// Array header access helpers ("stbds_header(t)", "stbds_temp(t)", ...)
// ---------------------------------------------------------------------------

/// `#define stbds_header(t) ((stbds_array_header *) (t) - 1)`
#[inline(always)]
pub fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).wrapping_sub(1)
}

/// `#define stbds_arrlen(a) ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline(always)]
pub(crate) unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if !a.is_null() {
        (*stbds_header(a)).length as isize
    } else {
        0
    }
}

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline(always)]
pub(crate) unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    }
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline(always)]
pub(crate) unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline(always)]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *ht = v;
}

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char*) (x) - (elemsize))`
#[inline(always)]
pub(crate) fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char*) (x) + (elemsize))`
#[inline(always)]
pub(crate) fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `(char *) a + elemsize*i + keyoffset`
#[inline(always)]
pub(crate) fn elem_at(a: *mut c_void, elemsize: usize, i: usize, keyoffset: usize) -> *mut u8 {
    (a as *mut u8)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(keyoffset)
}

// ---------------------------------------------------------------------------
// dynamic array
// ---------------------------------------------------------------------------

/// ```c
/// void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    // stbds_array_header temp={0}; (void) sizeof(temp);
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

    let old: *mut c_void = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };

    let mut b = realloc(old, elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE));
    b = (b as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

/// ```c
/// void stbds_arrfreef(void *a)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// hash index
// ---------------------------------------------------------------------------

/// `static size_t stbds_hash_seed=0x31415926;`
static mut STBDS_HASH_SEED: usize = 0x3141_5926;

/// ```c
/// void stbds_rand_seed(size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

/// ```c
/// static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)
/// ```
#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
}

/// ```c
/// static size_t stbds_log2(size_t slot_count)
/// ```
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// ```c
/// stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
/// stbds_load_32_or_64(b, temp,  715136305,          0, 0xb504f32d);
/// ```
/// (the intermediate `v64_lo ^ v32` is computed in `unsigned int`)
#[inline(always)]
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

/// ```c
/// static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)
/// ```
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
    stbds_assert!(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count,
        b"t->used_count_threshold + t->tombstone_count_threshold < t->slot_count\0",
        401,
        b"stbds_make_hash_index\0"
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(
            ptr::addr_of_mut!((*t).string) as *mut u8,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = STBDS_HASH_SEED;
        let a = stbds_load_32_or_64(2147001325, 0x27bb_2ee6, 0x87b0_b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504_f32d);
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
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
        let mut i: usize = 0;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob = (*ot).storage.wrapping_add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket = (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z: usize = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        let limit: usize = pos & STBDS_BUCKET_MASK;
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

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

/// ```c
/// size_t stbds_hash_string(char *str, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        // hash = STBDS_ROTATE_LEFT(hash, 9) + (unsigned char) *str++;
        hash = rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.wrapping_add(1);
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

macro_rules! stbds_sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {
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
    };
}

/// ```c
/// static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)
/// ```
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let mut j: usize;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = (((0x736f_6d65usize << 16) << 16).wrapping_add(0x7073_6575)) ^ seed;
    v1 = (((0x646f_7261usize << 16) << 16).wrapping_add(0x6e64_6f6d)) ^ !seed;
    v2 = (((0x6c79_6765usize << 16) << 16).wrapping_add(0x6e65_7261)) ^ seed;
    v3 = (((0x7465_6462usize << 16) << 16).wrapping_add(0x7974_6573)) ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    i = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // NOTE: this is an `int` expression in C; for d[3] >= 0x80 the result is
        // negative and the conversion to size_t sign extends, setting all upper
        // 32 bits.  Reproduced verbatim (bug-compatible).
        let lo: c_int = (*d.wrapping_add(0) as c_int)
            | ((*d.wrapping_add(1) as c_int) << 8)
            | ((*d.wrapping_add(2) as c_int) << 16)
            | ((*d.wrapping_add(3) as c_int) << 24);
        data = lo as usize;
        let hi: c_int = (*d.wrapping_add(4) as c_int)
            | ((*d.wrapping_add(5) as c_int) << 8)
            | ((*d.wrapping_add(6) as c_int) << 16)
            | ((*d.wrapping_add(7) as c_int) << 24);
        data |= ((hi as usize) << 16) << 16;

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
    // switch (len - i) with fall-through
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
        // `(d[3] << 24)` is an int -> sign extends into the upper 32 bits.
        data |= ((*d.wrapping_add(3) as c_int) << 24) as usize;
    }
    if rem >= 3 {
        data |= ((*d.wrapping_add(2) as c_int) << 16) as usize;
    }
    if rem >= 2 {
        data |= ((*d.wrapping_add(1) as c_int) << 8) as usize;
    }
    if rem >= 1 {
        data |= (*d.wrapping_add(0) as c_int) as usize;
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

/// ```c
/// size_t stbds_hash_bytes(void *p, size_t len, size_t seed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

/// ```c
/// static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize,
///                              size_t keyoffset, int mode, size_t i)
/// ```
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
        let stored = *(elem_at(a, elemsize, i, keyoffset) as *mut *mut c_char);
        (0 == strcmp(key as *const c_char, stored)) as c_int
    } else {
        (0 == memcmp(
            key,
            elem_at(a, elemsize, i, keyoffset) as *const c_void,
            keysize,
        )) as c_int
    }
}

// ---------------------------------------------------------------------------
// hash map
// ---------------------------------------------------------------------------

/// ```c
/// void stbds_hmfree_func(void *a, size_t elemsize)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                free(*(elem_at(a, elemsize, i, 0) as *mut *mut c_char) as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

/// ```c
/// static ptrdiff_t stbds_hm_find_slot(void *a, size_t elemsize, void *key,
///                                     size_t keysize, size_t keyoffset, int mode)
/// ```
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
    let mut hash: usize = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: usize = STBDS_BUCKET_LENGTH;
    let mut limit: usize;
    let mut i: usize;
    let mut pos: usize;
    let mut bucket: *mut stbds_hash_bucket;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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

/// ```c
/// void * stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize,
///                           ptrdiff_t *temp, int mode)
/// ```
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
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
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

/// ```c
/// void * stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
/// ```
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

/// ```c
/// void * stbds_hmput_default(void *a, size_t elemsize)
/// ```
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
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

/// ```c
/// void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
/// ```
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
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        a = stbds_arr_to_hash(a, elemsize);
    }

    raw_a = a;
    a = stbds_hash_to_arr(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count: usize = if table.is_null() {
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
        (*stbds_header(a)).hash_table = nt as *mut c_void;
    }

    {
        let mut hash: usize = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step: usize = STBDS_BUCKET_LENGTH;
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
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                            let v = *(elem_at(
                                raw_a,
                                elemsize,
                                (*bucket).index[i] as usize,
                                keyoffset,
                            ) as *mut *mut c_char);
                            stbds_temp_key_set(a, v);
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

            stbds_assert!(
                (i as usize).wrapping_add(1) <= stbds_arrcap(a),
                b"(size_t) i+1 <= stbds_arrcap(a)\0",
                778,
                b"stbds_hmput_key\0"
            );
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_temp_set(a, i - 1);

            let mode_field = (*table).string.mode as c_int;
            if mode_field == STBDS_SH_STRDUP {
                let v = stbds_strdup(key as *mut c_char);
                *(elem_at(a, elemsize, i as usize, 0) as *mut *mut c_char) = v;
                stbds_temp_key_set(a, v);
            } else if mode_field == STBDS_SH_ARENA {
                let v = stbds_stralloc(
                    ptr::addr_of_mut!((*table).string),
                    key as *mut c_char,
                );
                *(elem_at(a, elemsize, i as usize, 0) as *mut *mut c_char) = v;
                stbds_temp_key_set(a, v);
            } else if mode_field == STBDS_SH_DEFAULT {
                let v = key as *mut c_char;
                *(elem_at(a, elemsize, i as usize, 0) as *mut *mut c_char) = v;
                stbds_temp_key_set(a, v);
            } else {
                memcpy(
                    elem_at(a, elemsize, i as usize, 0) as *mut c_void,
                    key as *const c_void,
                    keysize,
                );
            }
        }
        let _ = raw_a;
        stbds_arr_to_hash(a, elemsize)
    }
}

/// ```c
/// void * stbds_shmode_func(size_t elemsize, int mode)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    stbds_arr_to_hash(a, elemsize)
}

/// ```c
/// void * stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize,
///                        size_t keyoffset, int mode)
/// ```
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
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_temp_set(raw_a, 0);
        if table.is_null() {
            a
        } else {
            let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b = (*table)
                    .storage
                    .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                stbds_assert!(
                    slot < (*table).slot_count as isize,
                    b"slot < (ptrdiff_t) table->slot_count\0",
                    828,
                    b"stbds_hmdel_key\0"
                );
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                stbds_temp_set(raw_a, 1);
                // STBDS_ASSERT(table->used_count >= 0) -- always true for size_t
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
                    free(
                        *(elem_at(a, elemsize, old_index as usize, 0) as *mut *mut c_char)
                            as *mut c_void,
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
                            elem_at(a, elemsize, old_index as usize, keyoffset) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    stbds_assert!(slot >= 0, b"slot >= 0\0", 846, b"stbds_hmdel_key\0");
                    b = (*table)
                        .storage
                        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    stbds_assert!(
                        (*b).index[i as usize] == final_index,
                        b"b->index[i] == final_index\0",
                        849,
                        b"stbds_hmdel_key\0"
                    );
                    (*b).index[i as usize] = old_index;
                }
                (*stbds_header(raw_a)).length = (*stbds_header(raw_a)).length.wrapping_sub(1);

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

/// ```c
/// static char *stbds_strdup(char *str)
/// ```
unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_).wrapping_add(1);
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// ```c
/// char *stbds_stralloc(stbds_string_arena *a, char *str)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len: usize = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(len),
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
                core::mem::size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(blocksize),
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    stbds_assert!(
        len <= (*a).remaining,
        b"len <= a->remaining\0",
        913,
        b"stbds_stralloc\0"
    );
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining.wrapping_sub(len) as usize);
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// ```c
/// void stbds_strreset(stbds_string_arena *a)
/// ```
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
    ptr::write_bytes(
        a as *mut u8,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}
