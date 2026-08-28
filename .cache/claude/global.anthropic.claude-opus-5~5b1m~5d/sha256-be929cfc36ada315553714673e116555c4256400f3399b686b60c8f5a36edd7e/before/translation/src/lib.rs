//! Rust translation of c_src/src/lib.c (stb_ds.h implementation + test helpers).
//!
//! This crate reproduces the complete public ABI of the C shared library:
//!
//!   stbds_arrgrowf, stbds_arrfreef, stbds_rand_seed, stbds_hash_string,
//!   stbds_hash_bytes, stbds_hmfree_func, stbds_hmget_key_ts, stbds_hmget_key,
//!   stbds_hmput_default, stbds_hmput_key, stbds_shmode_func, stbds_hmdel_key,
//!   stbds_stralloc, stbds_strreset, strkey, intput
//!
//! Behaviour (including the original C's integer-promotion quirks) is
//! reproduced bit-for-bit; no bugs are "fixed".

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (STBDS_REALLOC / STBDS_FREE map onto realloc / free, and
// STBDS_ASSERT maps onto assert()).
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// `__FILE__` as the C compiler saw it (see build.rs), NUL terminated.
static FILE_NAME: &[u8] = {
    const F: &str = concat!(env!("STBDS_C_FILE"), "\0");
    F.as_bytes()
};

/// Emulates `assert(cond)` from the C source: on failure glibc prints the
/// diagnostic and raises SIGABRT.
macro_rules! stbds_assert {
    ($cond:expr, $expr_str:expr, $line:expr, $func:expr) => {
        if !($cond) {
            __assert_fail(
                $expr_str.as_ptr() as *const c_char,
                FILE_NAME.as_ptr() as *const c_char,
                $line as c_uint,
                $func.as_ptr() as *const c_char,
            );
        }
    };
}

// ---------------------------------------------------------------------------
// Structures (layout-compatible with the C definitions)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
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

const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const HDR_SIZE: usize = size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Small helpers mirroring the C macros
// ---------------------------------------------------------------------------

#[inline(always)]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    // ((stbds_array_header *) (t) - 1)
    (t as *mut u8).wrapping_sub(HDR_SIZE) as *mut stbds_array_header
}

#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline(always)]
unsafe fn stbds_set_temp(a: *mut c_void, v: isize) {
    (*stbds_header(a)).temp = v;
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline(always)]
unsafe fn stbds_set_temp_key(a: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
    ptr::write_unaligned(ht, v);
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// STBDS_HASH_TO_ARR(x,elemsize)
#[inline(always)]
fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// STBDS_ARR_TO_HASH(x,elemsize)
#[inline(always)]
fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

#[inline(always)]
fn elem_ptr(a: *mut c_void, elemsize: usize, i: usize) -> *mut u8 {
    (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i))
}

#[inline(always)]
unsafe fn read_char_ptr(p: *const u8) -> *mut c_char {
    ptr::read_unaligned(p as *const *mut c_char)
}

#[inline(always)]
unsafe fn write_char_ptr(p: *mut u8, v: *mut c_char) {
    ptr::write_unaligned(p as *mut *mut c_char, v)
}

unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    let mut p = s as *const u8;
    while *p != 0 {
        n += 1;
        p = p.add(1);
    }
    n
}

/// `0 == strcmp(a, b)`
unsafe fn c_str_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut x = a as *const u8;
    let mut y = b as *const u8;
    loop {
        let ca = *x;
        let cb = *y;
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        x = x.add(1);
        y = y.add(1);
    }
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
    // size_t min_len = stbds_arrlen(a) + addlen;  (ptrdiff_t converted to size_t)
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
    let raw = realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HDR_SIZE),
    );
    let b = (raw as *mut u8).wrapping_add(HDR_SIZE) as *mut c_void;
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
// hash seed / hash index
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
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

/// stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)
#[inline(always)]
fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ v32;
    var
}

#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(size_of::<stbds_hash_bucket>())
            .wrapping_add(size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = stbds_align_fwd(
        (t as usize).wrapping_add(size_of::<stbds_hash_index>()),
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
            size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i = 0usize;
        while i < (slot_count >> STBDS_BUCKET_SHIFT) {
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
        while i < ((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = (*ot).storage.add(i);
            let mut j = 0usize;
            while j < STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
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
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'probe;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = hash.rotate_left(9).wrapping_add(*s as usize);
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

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! stbds_siphash_round {
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

    let mut i = 0usize;
    while i.wrapping_add(size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // (int arithmetic, then converted to size_t == sign extension)
        let lo: c_int = (*d.add(0) as c_int)
            | ((*d.add(1) as c_int) << 8)
            | ((*d.add(2) as c_int) << 16)
            | ((*d.add(3) as c_int) << 24);
        data = lo as isize as usize;
        let hi: c_int = (*d.add(4) as c_int)
            | ((*d.add(5) as c_int) << 8)
            | ((*d.add(6) as c_int) << 16)
            | ((*d.add(7) as c_int) << 24);
        data |= ((hi as isize as usize) << 16) << 16;

        v3 ^= data;
        let mut j = 0usize;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_siphash_round!(v0, v1, v2, v3);
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(size_of::<usize>());
        d = d.add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len.wrapping_sub(i);
    // switch with fall-through: case 7 .. case 1
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
        data |= (((*d.add(3) as c_int) << 24) as isize) as usize;
    }
    if rem >= 3 {
        data |= (((*d.add(2) as c_int) << 16) as isize) as usize;
    }
    if rem >= 2 {
        data |= (((*d.add(1) as c_int) << 8) as isize) as usize;
    }
    if rem >= 1 {
        data |= ((*d.add(0) as c_int) as isize) as usize;
    }

    v3 ^= data;
    let mut j = 0usize;
    while j < STBDS_SIPHASH_C_ROUNDS {
        stbds_siphash_round!(v0, v1, v2, v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    let mut j = 0usize;
    while j < STBDS_SIPHASH_D_ROUNDS {
        stbds_siphash_round!(v0, v1, v2, v3);
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
    let slot = (a as *mut u8)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(keyoffset);
    if mode >= STBDS_HM_STRING {
        c_str_eq(key as *const c_char, read_char_ptr(slot)) as c_int
    } else {
        let mut eq = true;
        let ka = key as *const u8;
        let mut n = 0usize;
        while n < keysize {
            if *ka.add(n) != *slot.add(n) {
                eq = false;
                break;
            }
            n += 1;
        }
        eq as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i = 1usize;
            while i < (*stbds_header(a)).length {
                free(read_char_ptr(elem_ptr(a, elemsize, i)) as *mut c_void);
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
    stbds_set_temp(hash_to_arr(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let base = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        a = stbds_arrgrowf(base, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c_strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
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
    let mut a = a;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
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
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
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
            hash += 2;
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
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let v = read_char_ptr(
                                (raw_a as *mut u8)
                                    .wrapping_add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                    .wrapping_add(keyoffset),
                            );
                            stbds_set_temp_key(a, v);
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

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count.wrapping_sub(1);
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
            raw_a = arr_to_hash(a, elemsize);
            let _ = raw_a;

            stbds_assert!(
                (i as usize).wrapping_add(1) <= stbds_arrcap(a),
                b"(size_t) i+1 <= stbds_arrcap(a)\0",
                778,
                b"stbds_hmput_key\0"
            );
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_set_temp(a, i - 1);

            let dst = elem_ptr(a, elemsize, i as usize);
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    write_char_ptr(dst, v);
                    stbds_set_temp_key(a, v);
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    write_char_ptr(dst, v);
                    stbds_set_temp_key(a, v);
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    write_char_ptr(dst, v);
                    stbds_set_temp_key(a, v);
                }
                _ => {
                    ptr::copy_nonoverlapping(key as *const u8, dst, keysize);
                }
            }
        }
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
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
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_set_temp(raw_a, 0);
        if table.is_null() {
            a
        } else {
            let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index = (*b).index[i as usize];
                let final_index = stbds_arrlen(raw_a) - 1 - 1;
                stbds_assert!(
                    slot < (*table).slot_count as isize,
                    b"slot < (ptrdiff_t) table->slot_count\0",
                    828,
                    b"stbds_hmdel_key\0"
                );
                (*table).used_count -= 1;
                (*table).tombstone_count += 1;
                stbds_set_temp(raw_a, 1);
                // STBDS_ASSERT(table->used_count >= 0) -- always true for size_t
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                    free(read_char_ptr(elem_ptr(a, elemsize, old_index as usize)) as *mut c_void);
                }

                if old_index != final_index {
                    ptr::copy(
                        elem_ptr(a, elemsize, final_index as usize) as *const u8,
                        elem_ptr(a, elemsize, old_index as usize),
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        let k = read_char_ptr(
                            elem_ptr(a, elemsize, old_index as usize).wrapping_add(keyoffset),
                        );
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            k as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        let k = elem_ptr(a, elemsize, old_index as usize).wrapping_add(keyoffset);
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            k as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    stbds_assert!(slot >= 0, b"slot >= 0\0", 846, b"stbds_hmdel_key\0");
                    b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    stbds_assert!(
                        (*b).index[i as usize] == final_index,
                        b"b->index[i] == final_index\0",
                        849,
                        b"stbds_hmdel_key\0"
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
    let len = c_strlen(str_) + 1;
    if len > (*a).remaining {
        let blocksize0 = (*a).block as usize;
        let blocksize =
            STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize0 >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                (size_of::<stbds_string_block>() - 8).wrapping_add(len),
            ) as *mut stbds_string_block;
            ptr::copy(
                str_ as *const u8,
                ptr::addr_of_mut!((*sb).storage) as *mut u8,
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
                (size_of::<stbds_string_block>() - 8).wrapping_add(blocksize),
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
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut u8)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len) as *mut c_char;
    (*a).remaining -= len;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
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
    ptr::write_bytes(a as *mut u8, 0, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// test helpers (strkey / intput)
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

/// `sprintf(dst, "%d", n)` for a plain int; returns the number of digits
/// written (excluding the NUL, which is not written here).
unsafe fn write_int_decimal(dst: *mut u8, n: c_int) -> usize {
    let mut tmp = [0u8; 24];
    let mut v = n as i64;
    let neg = v < 0;
    if neg {
        v = -v;
    }
    let mut ndigits = 0usize;
    if v == 0 {
        tmp[0] = b'0';
        ndigits = 1;
    } else {
        while v > 0 {
            tmp[ndigits] = b'0' + (v % 10) as u8;
            v /= 10;
            ndigits += 1;
        }
    }
    let mut w = 0usize;
    if neg {
        *dst = b'-';
        w = 1;
    }
    let mut k = ndigits;
    while k > 0 {
        k -= 1;
        *dst.add(w) = tmp[k];
        w += 1;
    }
    w
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(buffer) as *mut u8;
    let prefix: &[u8; 5] = b"test_";
    ptr::copy_nonoverlapping(prefix.as_ptr(), buf, 5);
    let w = write_int_decimal(buf.add(5), n);
    *buf.add(5 + w) = 0;
    buf as *mut c_char
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

const INTMAP_ELEMSIZE: usize = size_of::<IntMapEntry>();
const INTMAP_KEYSIZE: usize = size_of::<c_int>();

/// `stbds_temp((t)-1)` for an `IntMapEntry *` hash-map pointer.
#[inline(always)]
unsafe fn intmap_temp(t: *mut IntMapEntry) -> isize {
    (*stbds_header(hash_to_arr(t as *mut c_void, INTMAP_ELEMSIZE))).temp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(num: c_int) {
    // struct { int key; int value; } *intmap = NULL;  followed by  intmap = NULL;
    let mut intmap: *mut IntMapEntry;
    intmap = ptr::null_mut();

    // hmput(intmap, num, 7);
    {
        let mut k: c_int = num;
        intmap = stbds_hmput_key(
            intmap as *mut c_void,
            INTMAP_ELEMSIZE,
            ptr::addr_of_mut!(k) as *mut c_void,
            INTMAP_KEYSIZE,
            0,
        ) as *mut IntMapEntry;
        (*intmap.offset(intmap_temp(intmap))).key = num;
        (*intmap.offset(intmap_temp(intmap))).value = 7;
    }

    // hmput(intmap, 11, 3);
    {
        let mut k: c_int = 11;
        intmap = stbds_hmput_key(
            intmap as *mut c_void,
            INTMAP_ELEMSIZE,
            ptr::addr_of_mut!(k) as *mut c_void,
            INTMAP_KEYSIZE,
            0,
        ) as *mut IntMapEntry;
        (*intmap.offset(intmap_temp(intmap))).key = 11;
        (*intmap.offset(intmap_temp(intmap))).value = 3;
    }

    // hmput(intmap, 9, num);
    {
        let mut k: c_int = 9;
        intmap = stbds_hmput_key(
            intmap as *mut c_void,
            INTMAP_ELEMSIZE,
            ptr::addr_of_mut!(k) as *mut c_void,
            INTMAP_KEYSIZE,
            0,
        ) as *mut IntMapEntry;
        (*intmap.offset(intmap_temp(intmap))).key = 9;
        (*intmap.offset(intmap_temp(intmap))).value = num;
    }

    // STBDS_ASSERT(hmget(intmap, 9) == num);
    {
        let v = intmap_hmget(&mut intmap, 9);
        stbds_assert!(
            v == num,
            b"hmget(intmap, 9) == num\0",
            953,
            b"intput\0"
        );
    }

    // STBDS_ASSERT(hmget(intmap, 11) == 3);
    {
        let v = intmap_hmget(&mut intmap, 11);
        stbds_assert!(
            v == 3,
            b"hmget(intmap, 11) == 3\0",
            954,
            b"intput\0"
        );
    }

    // STBDS_ASSERT(hmget(intmap, num) == 7);
    {
        let v = intmap_hmget(&mut intmap, num);
        stbds_assert!(
            v == 7,
            b"hmget(intmap, num) == 7\0",
            955,
            b"intput\0"
        );
    }
}

/// `hmget(intmap, k)` == `stbds_hmgetp(intmap,k)->value`
unsafe fn intmap_hmget(intmap: &mut *mut IntMapEntry, k: c_int) -> c_int {
    let mut key: c_int = k;
    *intmap = stbds_hmget_key(
        *intmap as *mut c_void,
        INTMAP_ELEMSIZE,
        ptr::addr_of_mut!(key) as *mut c_void,
        INTMAP_KEYSIZE,
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    let t = intmap_temp(*intmap);
    (*(*intmap).offset(t)).value
}
