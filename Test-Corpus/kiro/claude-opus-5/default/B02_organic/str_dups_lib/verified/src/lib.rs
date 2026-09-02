#![allow(nonstandard_style)]
//! Rust translation of `c_src/src/lib.c` (an amalgamation of `stb_ds.h` plus the
//! `str_dups` driver from `stb_ds`'s unit-test block).
//!
//! The translation is deliberately literal: every arithmetic quirk, integer
//! promotion, sign extension and ordering detail of the original C is
//! reproduced, including behaviour that is arguably buggy (see the notes on
//! `siphash_bytes` and `hmput_key`). Allocation goes through libc `realloc`/
//! `free` so that blocks stay interchangeable with the C implementation.
//!
//! Public ABI (16 symbols, matching `nm -D` on the C shared object):
//!   stbds_arrgrowf, stbds_arrfreef, stbds_rand_seed, stbds_hash_string,
//!   stbds_hash_bytes, stbds_hmget_key_ts, stbds_hmget_key,
//!   stbds_hmput_default, stbds_shmode_func, stbds_hmdel_key, stbds_stralloc,
//!   stbds_hmput_key, stbds_strreset, stbds_hmfree_func, strkey, str_dups

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    /// glibc's expansion target for `assert()`. `NDEBUG` is not defined in the
    /// C build (CMakeLists sets no build type), so the asserts are live and a
    /// violation must abort rather than silently continue.
    fn __assert_fail(
        expr: *const c_char,
        file: *const c_char,
        line: u32,
        func: *const c_char,
    ) -> !;
}

/// `#define STBDS_ASSERT assert`
macro_rules! stbds_assert {
    ($cond:expr, $expr_str:literal, $line:literal, $func:literal) => {
        if !($cond) {
            __assert_fail(
                concat!($expr_str, "\0").as_ptr() as *const c_char,
                b"src/lib.c\0".as_ptr() as *const c_char,
                $line,
                concat!($func, "\0").as_ptr() as *const c_char,
            );
        }
    };
}

/// `#define STBDS_REALLOC(c,p,s) realloc(p,s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    realloc(p, size)
}

/// `#define STBDS_FREE(c,p) free(p)`
#[inline]
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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

/// `(sizeof (size_t)) * 8`
const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// `typedef int STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds[sizeof(size_t)==8?1:-1];`
const _: () = assert!(size_of::<usize>() == 8);

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

// ---------------------------------------------------------------------------
// Small helpers mirroring the C macros
// ---------------------------------------------------------------------------

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

/// `#define stbds_header(t) ((stbds_array_header *) (t) - 1)`
#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).wrapping_sub(1)
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

#[inline]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
}

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    }
}

/// `#define stbds_arrlen(a) ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if !a.is_null() {
        (*stbds_header(a)).length as isize
    } else {
        0
    }
}

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char*) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char*) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `(char *) a + elemsize*i + keyoffset` with C's `size_t` wrap-around semantics.
#[inline]
fn elem_at(a: *mut c_void, elemsize: usize, i: usize, keyoffset: usize) -> *mut u8 {
    (a as *mut u8)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(keyoffset)
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// `#define STBDS_ROTATE_LEFT(val, n) (((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n))))`
#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n) (((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n))))`
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
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),
    );
    b = (raw as *mut u8).wrapping_add(size_of::<stbds_array_header>()) as *mut c_void;
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
// Hash seed / index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
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

/// Literal translation of the `stbds_load_32_or_64` macro.
///
/// `v64_lo` / `v64_hi` are `unsigned int` typed hex literals in the C source and
/// `v32` is a (positive) `int`, so every intermediate stays zero-extended.
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

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t: *mut stbds_hash_index = stbds_realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
            + size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1,
    ) as *mut stbds_hash_index;
    (*t).storage = stbds_align_fwd(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE)
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
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
        "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count",
        401,
        "stbds_make_hash_index"
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        memset(
            &mut (*t).string as *mut stbds_string_arena as *mut c_void,
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
            let b = (*t).storage.wrapping_add(i);
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
            let ob = (*ot).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    loop {
                        let bucket = (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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

// ---------------------------------------------------------------------------
// Hashing
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
    // `hash ^= hash ^ STBDS_ROTATE_RIGHT(hash,31);` -- written out literally.
    hash = hash ^ (hash ^ stbds_rotate_right(hash, 31));
    hash = hash.wrapping_mul(21);
    hash = hash ^ (hash ^ stbds_rotate_right(hash, 11));
    hash = hash.wrapping_add(hash << 6);
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

macro_rules! stbds_siprounds {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident, $rounds:expr) => {
        for _ in 0..$rounds {
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
        }
    };
}

/// Note on fidelity: in the C source both halves of the 64-bit `data` word are
/// assembled with *`int`* arithmetic (`d[3] << 24` promotes to `int`), so when
/// byte 3 has its high bit set the result is a negative `int` that gets
/// **sign extended** on conversion to `size_t`, filling the whole upper half of
/// `data` with ones. The same happens for the `case 4:` tail byte. This is
/// observable (e.g. lengths 12..15 of a buffer whose byte 11 is >= 0x80 all hash
/// identically), so it is reproduced here exactly rather than "fixed".
///
/// Note also that all four `v0..v3` initialisers XOR `seed` (or `~seed`) in
/// twice, so the seed cancels out completely and this hash ignores it.
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

    i = 0;
    while i + size_of::<usize>() <= len {
        // `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);` -- int, then
        // sign-extended into size_t.
        let lo: i32 = ((*d.add(0) as u32)
            | ((*d.add(1) as u32) << 8)
            | ((*d.add(2) as u32) << 16)
            | ((*d.add(3) as u32) << 24)) as i32;
        data = lo as i64 as u64 as usize;
        let hi: i32 = ((*d.add(4) as u32)
            | ((*d.add(5) as u32) << 8)
            | ((*d.add(6) as u32) << 16)
            | ((*d.add(7) as u32) << 24)) as i32;
        data |= ((hi as i64 as u64 as usize) << 16) << 16;

        v3 ^= data;
        stbds_siprounds!(v0, v1, v2, v3, STBDS_SIPHASH_C_ROUNDS);
        v0 ^= data;

        i += size_of::<usize>();
        d = d.add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // Fall-through switch: `case 7` runs 7,6,5,4,3,2,1; `case 4` runs 4,3,2,1; ...
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
        // `data |= (d[3] << 24);` -- int expression, sign-extends into size_t.
        data |= (((*d.add(3) as u32) << 24) as i32) as i64 as u64 as usize;
    }
    if rem >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    stbds_siprounds!(v0, v1, v2, v3, STBDS_SIPHASH_C_ROUNDS);
    v0 ^= data;
    v2 ^= 0xff;
    stbds_siprounds!(v0, v1, v2, v3, STBDS_SIPHASH_D_ROUNDS);

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// Hash map internals
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
    if mode >= STBDS_HM_STRING {
        0 == strcmp(
            key as *const c_char,
            *(elem_at(a, elemsize, i, keyoffset) as *mut *mut c_char),
        )
    } else {
        0 == memcmp(
            key,
            elem_at(a, elemsize, i, keyoffset) as *const c_void,
            keysize,
        )
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
                stbds_free(*(elem_at(a, elemsize, i, 0) as *mut *mut c_char) as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&mut (*stbds_hash_table(a)).string as *mut stbds_string_arena);
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

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
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
            hash += 2;
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'probe: loop {
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
                    ) {
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            stbds_set_temp_key(
                                a,
                                *(elem_at(
                                    raw_a,
                                    elemsize,
                                    (*bucket).index[i] as usize,
                                    keyoffset,
                                ) as *mut *mut c_char),
                            );
                        }
                        return stbds_arr_to_hash(a, elemsize);
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
                        // NB: unlike the loop above, the C code does *not* update
                        // stbds_temp_key here. Preserved verbatim.
                        stbds_set_temp(a, (*bucket).index[i]);
                        return stbds_arr_to_hash(a, elemsize);
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
            if (i as usize) + 1 > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = stbds_arr_to_hash(a, elemsize);

            stbds_assert!(
                (i as usize) + 1 <= stbds_arrcap(a),
                "(size_t) i+1 <= stbds_arrcap(a)",
                778,
                "stbds_hmput_key"
            );
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_set_temp(a, i - 1);

            let slot = elem_at(a, elemsize, i as usize, 0) as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    *slot = v;
                    stbds_set_temp_key(a, v);
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(
                        &mut (*table).string as *mut stbds_string_arena,
                        key as *mut c_char,
                    );
                    *slot = v;
                    stbds_set_temp_key(a, v);
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    *slot = v;
                    stbds_set_temp_key(a, v);
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
        let _ = raw_a;
        stbds_arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
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

    let mut b = (*table)
        .storage
        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
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
    // STBDS_ASSERT(table->used_count >= 0);  (lib.c:832 -- size_t, always true)
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
        stbds_free(*(elem_at(a, elemsize, old_index as usize, 0) as *mut *mut c_char) as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            elem_at(a, elemsize, old_index as usize, 0) as *mut c_void,
            elem_at(a, elemsize, final_index as usize, 0) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                *(elem_at(a, elemsize, old_index as usize, keyoffset) as *mut *mut c_char)
                    as *mut c_void,
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
        stbds_assert!(slot >= 0, "slot >= 0", 846, "stbds_hmdel_key");
        b = (*table)
            .storage
            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        stbds_assert!(
            (*b).index[i] == final_index,
            "b->index[i] == final_index",
            849,
            "stbds_hmdel_key"
        );
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

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

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
            (*a).block += 1;
        }

        if len > blocksize {
            let sb = stbds_realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            memmove(
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
            let sb = stbds_realloc(
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
    p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len) as *mut c_char;
    (*a).remaining -= len;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// Test-block helpers that the C library also exports
// ---------------------------------------------------------------------------

/// `static char buffer[256];`
static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(buffer) as *mut c_char;
    sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
    buf
}

/// `struct { char *key; int value; }` -- the anonymous struct inside `str_dups`.
#[repr(C)]
#[derive(Clone, Copy)]
struct str_dups_entry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(num: c_int) {
    let elemsize = size_of::<str_dups_entry>();
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i: c_int = 0;
    while i < num {
        stbds_stralloc(&mut sa as *mut stbds_string_arena, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa as *mut stbds_string_arena);

    {
        let s = str_dups_entry {
            key: b"a\0".as_ptr() as *mut c_char,
            value: num,
        };

        // sh_new_strdup(strmap)
        let mut strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP) as *mut str_dups_entry;

        // shputs(strmap, s)
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            s.key as *mut c_void,
            size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut str_dups_entry;
        let raw = stbds_hash_to_arr(strmap as *mut c_void, elemsize);
        *strmap.offset(stbds_temp(raw)) = s;
        (*strmap.offset(stbds_temp(raw))).key =
            *((*stbds_header(raw)).hash_table as *mut *mut c_char);

        // STBDS_ASSERT(*strmap[0].key == 'a');
        // STBDS_ASSERT(strmap[0].key != s.key);
        // STBDS_ASSERT(strmap[0].value == s.value);
        stbds_assert!(
            *(*strmap.offset(0)).key == b'a' as c_char,
            "*strmap[0].key == 'a'",
            960,
            "str_dups"
        );
        stbds_assert!(
            (*strmap.offset(0)).key != s.key,
            "strmap[0].key != s.key",
            961,
            "str_dups"
        );
        stbds_assert!(
            (*strmap.offset(0)).value == s.value,
            "strmap[0].value == s.value",
            962,
            "str_dups"
        );

        // for (int z=0; z < shlen(strmap); ++z)
        //     printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // The struct is passed by value to a variadic function; under the SysV
        // AMD64 ABI its two eightbytes land in the first two vararg slots, so
        // "%s" consumes `.key` and "%d" consumes the low half of the second
        // eightbyte, i.e. `.value`. Verified against the C build.
        let shlen: isize = (*stbds_header(raw)).length as isize - 1;
        let mut z: c_int = 0;
        while (z as isize) < shlen {
            let e = &*strmap.offset(z as isize);
            printf(b"%s %d\n\0".as_ptr() as *const c_char, e.key, e.value);
            z += 1;
        }

        // shfree(strmap)
        stbds_hmfree_func(raw, elemsize);
    }
}

// Referenced only so the constants mirror the C enum exactly.
const _: c_int = STBDS_SH_NONE;
const _: c_int = STBDS_HM_BINARY;
