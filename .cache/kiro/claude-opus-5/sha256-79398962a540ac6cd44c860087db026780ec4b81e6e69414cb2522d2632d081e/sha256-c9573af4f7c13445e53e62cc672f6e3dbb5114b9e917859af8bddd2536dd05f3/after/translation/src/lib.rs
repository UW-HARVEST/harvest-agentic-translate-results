//! Rust translation of `c_src/src/lib.c` (stb_ds, Sean Barrett — MIT / Unlicense).
//!
//! This is a 1:1 behavioural translation.  Every quirk of the original C —
//! including implementation-defined sign-extension in the SipHash byte loader,
//! the self-cancelling `hash ^= hash ^ rot(hash)` lines, the seed that cancels
//! itself out in `stbds_siphash_bytes`, and the odd block bookkeeping in
//! `stbds_stralloc` — is reproduced exactly rather than "fixed".
//!
//! Memory layout of every struct is `#[repr(C)]` and matches the C definitions
//! byte for byte, and allocation is done through libc `realloc`/`free` exactly
//! like `STBDS_REALLOC` / `STBDS_FREE` do, so buffers handed out by this
//! library are interchangeable with the C ones.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (STBDS_REALLOC / STBDS_FREE / string.h / stdio.h)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn abort() -> !;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
}

// ---------------------------------------------------------------------------
// STBDS_ASSERT
//
// `#define STBDS_ASSERT assert` and the CMake build defines no `NDEBUG`, so the
// C asserts are LIVE: a violated one writes a diagnostic to stderr and calls
// `abort()` (SIGABRT).  Several of them are reachable through the public API
// (e.g. `stbds_hmdel_key` with `mode > STBDS_HM_STRING`), so the translation
// must terminate the same way instead of continuing with a bad index.
// ---------------------------------------------------------------------------

#[cold]
#[inline(never)]
unsafe fn stbds_assert_fail(msg: &'static str) -> ! {
    write(2, msg.as_ptr() as *const c_void, msg.len());
    abort()
}

macro_rules! stbds_assert {
    ($cond:expr, $msg:literal) => {
        if !($cond) {
            stbds_assert_fail(concat!("Assertion `", $msg, "' failed.\n"))
        }
    };
}

/// `STBDS_REALLOC(c, p, s)` => `realloc(p, s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    realloc(p, size)
}

/// `STBDS_FREE(c, p)` => `free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    free(p)
}

/// `memmove` / `memcpy`
#[inline]
unsafe fn memmove_bytes(dst: *mut u8, src: *const u8, n: usize) {
    ptr::copy(src, dst, n);
}

/// `memset(p, 0, n)`
#[inline]
unsafe fn memzero(p: *mut u8, n: usize) {
    ptr::write_bytes(p, 0, n);
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

/// `STBDS_SIZE_T_BITS` — `sizeof(size_t) * 8`
const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// `typedef int STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds[sizeof(size_t)==8 ? 1 : -1];`
const _: () = assert!(size_of::<usize>() == 8);

// `STBDS_INDEX_IN_USE(x)  ((x) >= 0)`
#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

// `STBDS_ALIGN_FWD(n,a)   (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

// `STBDS_ROTATE_LEFT / STBDS_ROTATE_RIGHT`
#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ---------------------------------------------------------------------------
// Structures (layout-compatible with the C originals)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
// Accessor macros
// ---------------------------------------------------------------------------

/// `stbds_header(t)  ((stbds_array_header *) (t) - 1)`
#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).wrapping_sub(1)
}

/// `stbds_temp(t)` — read
#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

/// `stbds_temp(t) = v`
#[inline]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)` — read
#[inline]
unsafe fn stbds_temp_key(t: *mut c_void) -> *mut c_char {
    *((*stbds_header(t)).hash_table as *mut *mut c_char)
}

/// `stbds_temp_key(t) = v`
#[inline]
unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
}

/// `stbds_arrcap(a)  ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `stbds_arrlen(a)  ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `stbds_hash_table(a)  ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `(char *) a + elemsize * i` — unsigned (wrapping) arithmetic, as in C.
#[inline]
fn elem_at(a: *mut c_void, elemsize: usize, i: usize) -> *mut u8 {
    (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i))
}

// ---------------------------------------------------------------------------
// Dynamic array growth
// ---------------------------------------------------------------------------

/// `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    // `stbds_array_header temp={0}; (void) sizeof(temp);` — no observable effect.
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

    let mut b = stbds_realloc(
        if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        },
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),
    );
    b = (b as *mut u8).wrapping_add(size_of::<stbds_array_header>()) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

/// `void stbds_arrfreef(void *a)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    stbds_free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// Hash seed / index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

/// `void stbds_rand_seed(size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
///
/// `v64_lo` and `v32` are the `unsigned int` / `int` literals from the C source;
/// their xor is computed in 32 bits (as C's usual arithmetic conversions do)
/// before being widened to `size_t`.
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u64, v64_lo: u32) -> usize {
    let mut temp: u64 = (v64_lo ^ v32) as u64;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;

    let mut var: u64 = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ (v32 as u64);

    var as usize
}

/// `static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)`
#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

/// `static size_t stbds_log2(size_t slot_count)`
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// `static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)`
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = stbds_realloc(
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
    stbds_assert!(
        (*t)
            .used_count_threshold
            .wrapping_add((*t).tombstone_count_threshold)
            < (*t).slot_count,
        "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count"
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        memzero(
            (&raw mut (*t).string) as *mut u8,
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
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
                // `done: ;`
            }
            i += 1;
        }
    }

    t
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// `size_t stbds_hash_string(char *str, size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut s = str_ as *const u8;
    let mut hash: usize = seed;
    while *s != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.wrapping_add(1);
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

/// `STBDS_SIPROUND()`
#[inline]
fn stbds_sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    let half = STBDS_SIZE_T_BITS / 2;
    *v0 = v0.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = stbds_rotate_left(*v0, half);

    *v2 = v2.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 16);
    *v3 ^= *v2;

    *v2 = v2.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = stbds_rotate_left(*v2, half);

    *v0 = v0.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 21);
    *v3 ^= *v0;
}

/// `static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)`
///
/// Note the C loader builds the low half of `data` as an `int` expression, so a
/// byte >= 0x80 in `d[3]`/`d[7]` makes the value negative and the conversion to
/// `size_t` sign-extends.  That behaviour is deliberately reproduced here.
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
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
    let mut i: usize = 0;
    while i.wrapping_add(size_of::<usize>()) <= len {
        // `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);`  (int -> size_t)
        let lo: u32 = (*d.wrapping_add(0) as u32)
            | ((*d.wrapping_add(1) as u32) << 8)
            | ((*d.wrapping_add(2) as u32) << 16)
            | ((*d.wrapping_add(3) as u32) << 24);
        data = ((lo as i32) as i64) as usize;

        // `data |= (size_t)(d[4] | (d[5]<<8) | (d[6]<<16) | (d[7]<<24)) << 16 << 16;`
        let hi: u32 = (*d.wrapping_add(4) as u32)
            | ((*d.wrapping_add(5) as u32) << 8)
            | ((*d.wrapping_add(6) as u32) << 16)
            | ((*d.wrapping_add(7) as u32) << 24);
        data |= ((((hi as i32) as i64) as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i = i.wrapping_add(size_of::<usize>());
        d = d.wrapping_add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // `switch (len - i)` with fall-through from case 7 down to case 1.
    let rem = len.wrapping_sub(i);
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
        // `data |= (d[3] << 24);` — int expression, sign-extends when d[3] >= 0x80
        data |= (((*d.wrapping_add(3) as i32).wrapping_shl(24)) as i64) as usize;
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

/// `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

/// `static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize,
///                               size_t keyoffset, int mode, size_t i)`
unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> c_int {
    let slot = elem_at(a, elemsize, i).wrapping_add(keyoffset);
    if mode >= STBDS_HM_STRING {
        (0 == strcmp(key as *const c_char, *(slot as *mut *mut c_char))) as c_int
    } else {
        (0 == memcmp(key, slot as *const c_void, keysize)) as c_int
    }
}

// ---------------------------------------------------------------------------
// Hash map
// ---------------------------------------------------------------------------

/// `void stbds_hmfree_func(void *a, size_t elemsize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                stbds_free(*(elem_at(a, elemsize, i) as *mut *mut c_void));
                i += 1;
            }
        }
        stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
    }
    stbds_free((*stbds_header(a)).hash_table);
    stbds_free(stbds_header(a) as *mut c_void);
}

/// `static ptrdiff_t stbds_hm_find_slot(void *a, size_t elemsize, void *key,
///                                     size_t keysize, size_t keyoffset, int mode)`
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
        hash = hash.wrapping_add(2);
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

/// `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize,
///                           ptrdiff_t *temp, int mode)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memzero(a as *mut u8, elemsize);
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

/// `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)`
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

/// `void *stbds_hmput_default(void *a, size_t elemsize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
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
        memzero(a as *mut u8, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

/// `static char *stbds_strdup(char *str)`
unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_).wrapping_add(1);
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    memmove_bytes(p as *mut u8, str_ as *const u8, len);
    p
}

/// `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memzero(a as *mut u8, elemsize);
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
                STBDS_SH_DEFAULT as u8
            } else {
                0
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = nt as *mut c_void;
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

    // `for (;;) { ... }` with `goto found_empty_slot`
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
                    stbds_set_temp(a, (*bucket).index[i]);
                    if mode >= STBDS_HM_STRING {
                        let kp = *(elem_at(raw_a, elemsize, (*bucket).index[i] as usize)
                            .wrapping_add(keyoffset) as *mut *mut c_char);
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
            "(size_t) i+1 <= stbds_arrcap(a)"
        );
        (*stbds_header(a)).length = (i + 1) as usize;
        bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        stbds_set_temp(a, i - 1);

        let key_slot = elem_at(a, elemsize, i as usize) as *mut *mut c_char;
        match (*table).string.mode as c_int {
            STBDS_SH_STRDUP => {
                let p = stbds_strdup(key as *mut c_char);
                *key_slot = p;
                stbds_set_temp_key(a, p);
            }
            STBDS_SH_ARENA => {
                let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                *key_slot = p;
                stbds_set_temp_key(a, p);
            }
            STBDS_SH_DEFAULT => {
                let p = key as *mut c_char;
                *key_slot = p;
                stbds_set_temp_key(a, p);
            }
            _ => {
                // default: memcpy((char *) a + elemsize*i, key, keysize);
                memmove_bytes(
                    elem_at(a, elemsize, i as usize),
                    key as *const u8,
                    keysize,
                );
            }
        }
    }

    stbds_arr_to_hash(a, elemsize)
}

/// `void *stbds_shmode_func(size_t elemsize, int mode)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memzero(a as *mut u8, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    stbds_arr_to_hash(a, elemsize)
}

/// `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize,
///                        size_t keyoffset, int mode)`
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
    let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
    let old_index: isize = (*b).index[i as usize];
    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
    stbds_assert!(
        slot < (*table).slot_count as isize,
        "slot < (ptrdiff_t) table->slot_count"
    );
    (*table).used_count = (*table).used_count.wrapping_sub(1);
    (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
    stbds_set_temp(raw_a, 1);
    // STBDS_ASSERT(table->used_count >= 0) -- `used_count` is `size_t`, so the
    // C comparison is a tautology and can never fire; deliberately omitted.
    (*b).hash[i as usize] = STBDS_HASH_DELETED;
    (*b).index[i as usize] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
        stbds_free(*(elem_at(a, elemsize, old_index as usize) as *mut *mut c_void));
    }

    if old_index != final_index {
        ptr::copy(
            elem_at(a, elemsize, final_index as usize) as *const u8,
            elem_at(a, elemsize, old_index as usize),
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let k = *(elem_at(a, elemsize, old_index as usize).wrapping_add(keyoffset)
                as *mut *mut c_char);
            slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
        } else {
            let k = elem_at(a, elemsize, old_index as usize).wrapping_add(keyoffset);
            slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
        }
        // STBDS_ASSERT(slot >= 0);
        stbds_assert!(slot >= 0, "slot >= 0");
        b = (*table)
            .storage
            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        stbds_assert!((*b).index[i as usize] == final_index, "b->index[i] == final_index");
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

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

/// `char *stbds_stralloc(stbds_string_arena *a, char *str)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let len = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        // `(size_t) 512u << (blocksize >> 1)`.  For every arena the library
        // itself creates, `block` saturates at 22 so the shift count never
        // exceeds 11.  A caller-forged arena with `block > 127` would make the
        // C expression a shift-count overflow (UB); gcc on x86-64 emits `shl`,
        // whose count is taken mod 64, so mask to match instead of panicking.
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << ((blocksize >> 1) & (STBDS_SIZE_T_BITS as usize - 1));

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = stbds_realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            memmove_bytes(
                (&raw mut (*sb).storage) as *mut u8,
                str_ as *const u8,
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

    // STBDS_ASSERT(len <= a->remaining);
    stbds_assert!(len <= (*a).remaining, "len <= a->remaining");
    let p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining.wrapping_sub(len) as isize as usize);
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    memmove_bytes(p as *mut u8, str_ as *const u8, len);
    p
}

/// `void stbds_strreset(stbds_string_arena *a)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut c_void);
        x = y;
    }
    memzero(a as *mut u8, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// Test / demo entry points
// ---------------------------------------------------------------------------

/// `static char buffer[256];`
static mut buffer: [c_char; 256] = [0; 256];

/// `char *strkey(int n)` — `sprintf(buffer, "test_%d", n); return buffer;`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let dst = (&raw mut buffer) as *mut u8;

    const PREFIX: &[u8] = b"test_";
    let mut w: usize = 0;
    while w < PREFIX.len() {
        *dst.add(w) = PREFIX[w];
        w += 1;
    }

    // "%d" for an `int`, including INT_MIN.
    let mut digits = [0u8; 12];
    let mut nd: usize = 0;
    let neg = n < 0;
    let mut mag: u32 = if neg {
        (n as i64).unsigned_abs() as u32
    } else {
        n as u32
    };
    loop {
        digits[nd] = b'0' + (mag % 10) as u8;
        nd += 1;
        mag /= 10;
        if mag == 0 {
            break;
        }
    }
    if neg {
        *dst.add(w) = b'-';
        w += 1;
    }
    while nd > 0 {
        nd -= 1;
        *dst.add(w) = digits[nd];
        w += 1;
    }
    *dst.add(w) = 0;

    dst as *mut c_char
}

/// The anonymous `struct { char *key; int value; }` used by `sh_puts`.
#[repr(C)]
#[derive(Copy, Clone)]
struct sh_puts_entry {
    key: *mut c_char,
    value: c_int,
}

/// `void sh_puts(int num)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    const ELEMSIZE: usize = size_of::<sh_puts_entry>();

    let mut strmap: *mut sh_puts_entry = ptr::null_mut();
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i: c_int = 0;
    while i < num {
        stbds_stralloc(&raw mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&raw mut sa);

    {
        // `s.key = "a", s.value = num;`
        static A_LITERAL: [c_char; 2] = [b'a' as c_char, 0];
        let s = sh_puts_entry {
            key: (&raw const A_LITERAL) as *mut c_char,
            value: num,
        };

        // sh_new_arena(strmap)
        strmap = stbds_shmode_func(ELEMSIZE, STBDS_SH_ARENA) as *mut sh_puts_entry;

        // shputs(strmap, s)
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            ELEMSIZE,
            s.key as *mut c_void,
            size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut sh_puts_entry;
        let raw = stbds_hash_to_arr(strmap as *mut c_void, ELEMSIZE);
        *strmap.wrapping_offset(stbds_temp(raw)) = s;
        (*strmap.wrapping_offset(stbds_temp(raw))).key = stbds_temp_key(raw);

        // STBDS_ASSERT(*strmap[0].key == 'a');
        // STBDS_ASSERT(strmap[0].key != s.key);
        // STBDS_ASSERT(strmap[0].value == s.value);
        stbds_assert!(*(*strmap).key == b'a' as c_char, "*strmap[0].key == 'a'");
        stbds_assert!((*strmap).key != s.key, "strmap[0].key != s.key");
        stbds_assert!((*strmap).value == s.value, "strmap[0].value == s.value");

        // for (int z=0; z < shlen(strmap); ++z)
        //     printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // `strmap[z]` is a 16-byte {pointer, int} struct passed by value in the
        // variadic call; under the SysV AMD64 ABI it occupies two INTEGER
        // eightbytes, so "%s" consumes .key and "%d" consumes .value -- the
        // trailing `strmap[z].value` argument is never read by the format.
        let shlen: isize = if !strmap.is_null() {
            (*stbds_header(raw)).length as isize - 1
        } else {
            0
        };
        let mut z: c_int = 0;
        while (z as isize) < shlen {
            let e = *strmap.wrapping_offset(z as isize);
            printf(c"%s %d\n".as_ptr(), e.key, e.value);
            z += 1;
        }

        // shfree(strmap)
        if !strmap.is_null() {
            stbds_hmfree_func(stbds_hash_to_arr(strmap as *mut c_void, ELEMSIZE), ELEMSIZE);
        }
        strmap = ptr::null_mut();
    }

    let _ = STBDS_HM_BINARY;
    let _ = STBDS_SH_NONE;
    let _ = stbds_hmput_default;
}
