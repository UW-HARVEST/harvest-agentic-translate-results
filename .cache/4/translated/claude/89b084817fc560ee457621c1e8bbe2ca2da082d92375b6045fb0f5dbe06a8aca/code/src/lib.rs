//! Rust translation of the C library in `c_src/` (stb_ds.h implementation +
//! the `hm_geti` / `strkey` test helpers).
//!
//! The translation is deliberately literal: memory layouts, pointer
//! arithmetic, integer wrap-around and even the quirks / bugs of the original
//! C code (e.g. the sign-extension of `d[3] << 24` in the siphash loader and
//! the seed cancelling itself out) are reproduced exactly so that the produced
//! shared object is a drop-in ABI/behaviour replacement for the C one.

#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// libc bindings (no external crates: declared by hand)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn abort() -> !;
}

/// `#define STBDS_REALLOC(c,p,s) realloc(p,s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    realloc(p, s)
}

/// `#define STBDS_FREE(c,p) free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    free(p)
}

// ---------------------------------------------------------------------------
// STBDS_ASSERT (== <assert.h> assert, NDEBUG is not defined in the C build)
// ---------------------------------------------------------------------------

#[cold]
#[inline(never)]
fn stbds_assert_fail(expr: &str) -> ! {
    // The C build uses plain assert(); on failure it prints a diagnostic to
    // stderr and aborts.  Reproduce the abort (the exact glibc wording embeds
    // the absolute build-machine path of lib.c and therefore cannot be
    // reproduced verbatim).
    eprintln!("Assertion failed: {}", expr);
    unsafe { abort() }
}

macro_rules! STBDS_ASSERT {
    ($cond:expr) => {
        if !($cond) {
            crate::stbds_assert_fail(stringify!($cond));
        }
    };
}

// ---------------------------------------------------------------------------
// Data structures (layout-compatible with the C originals)
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

// Compile-time verification of the C layout (x86-64 / LP64).
const _: () = {
    assert!(size_of::<stbds_array_header>() == 32);
    assert!(size_of::<stbds_string_block>() == 16);
    assert!(size_of::<stbds_string_arena>() == 24);
    assert!(size_of::<stbds_hash_bucket>() == 128);
    assert!(size_of::<stbds_hash_index>() == 104);
    assert!(size_of::<usize>() == 8);
};

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

#[allow(dead_code)] // part of the C enum, unused there as well
const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

// ---------------------------------------------------------------------------
// Pointer / macro helpers
// ---------------------------------------------------------------------------

/// `#define stbds_header(t) ((stbds_array_header *) (t) - 1)`
#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).wrapping_sub(1)
}

/// Byte offset of a pointer, using C's wrap-around (unsigned) semantics.
#[inline]
fn byte_off(p: *mut c_void, off: usize) -> *mut c_void {
    (p as *mut u8).wrapping_add(off) as *mut c_void
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
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

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    }
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
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

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// `#define STBDS_ROTATE_LEFT(val, n)`
#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n)`
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
    // size_t min_len = stbds_arrlen(a) + addlen;   (ptrdiff_t + size_t -> size_t)
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
    let b = stbds_realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),
    );
    let b = byte_off(b, size_of::<stbds_array_header>());
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
// Hash seed / hash index
// ---------------------------------------------------------------------------

/// `static size_t stbds_hash_seed=0x31415926;`
static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x31415926);

#[inline]
fn hash_seed_get() -> usize {
    STBDS_HASH_SEED.load(Ordering::Relaxed)
}

#[inline]
fn hash_seed_set(v: usize) {
    STBDS_HASH_SEED.store(v, Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    hash_seed_set(seed);
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

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
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
    STBDS_ASSERT!(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        memset(
            ptr::addr_of_mut!((*t).string) as *mut c_void,
            0,
            size_of::<stbds_string_arena>(),
        );
        (*t).seed = hash_seed_get();
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));
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
                        let mut z = 0usize;
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
// Hash functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
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

macro_rules! STBDS_SIPROUND {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
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

/// Reproduces the C expression `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)`
/// which is evaluated in `int` arithmetic (so the result is sign extended when
/// it is later converted to `size_t`).
#[inline]
unsafe fn load32_as_c_int(d: *const u8) -> i32 {
    let v: u32 = (*d.wrapping_add(0) as u32)
        | ((*d.wrapping_add(1) as u32) << 8)
        | ((*d.wrapping_add(2) as u32) << 16)
        | ((*d.wrapping_add(3) as u32) << 24);
    v as i32
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let j: usize;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;
    let _ = j;

    v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    i = 0;
    while i.wrapping_add(size_of::<usize>()) <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        data = load32_as_c_int(d) as isize as usize;
        // data |= (size_t) (d[4] | ... | (d[7] << 24)) << 16 << 16;
        data |= ((load32_as_c_int(d.wrapping_add(4)) as isize as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            STBDS_SIPROUND!(v0, v1, v2, v3);
        }
        v0 ^= data;

        i = i.wrapping_add(size_of::<usize>());
        d = d.wrapping_add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len.wrapping_sub(i);
    // switch (len - i) with fall-through from `case 7` down to `case 0`.
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
        // `data |= (d[3] << 24);`  -- int arithmetic, hence sign extension.
        data |= (((*d.wrapping_add(3) as u32) << 24) as i32) as isize as usize;
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
        STBDS_SIPROUND!(v0, v1, v2, v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        STBDS_SIPROUND!(v0, v1, v2, v3);
    }

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
) -> c_int {
    if mode >= STBDS_HM_STRING {
        let stored = *(byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset))
            as *mut *mut c_char);
        (0 == strcmp(key as *const c_char, stored)) as c_int
    } else {
        (0 == memcmp(
            key as *const c_void,
            byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *const c_void,
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
                stbds_free(
                    *(byte_off(a, elemsize.wrapping_mul(i)) as *mut *mut c_char) as *mut c_void,
                );
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
    let mut pos: usize;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

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
                let b = (*table).storage.wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
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
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
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
                        stbds_temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let stored = *(byte_off(
                                raw_a,
                                elemsize
                                    .wrapping_mul((*bucket).index[i] as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char);
                            stbds_temp_key_set(a, stored);
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
            pos &= (*table).slot_count - 1;
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

            STBDS_ASSERT!((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i.wrapping_add(1)) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i.wrapping_sub(1);
            stbds_temp_set(a, i.wrapping_sub(1));

            let slot: *mut *mut c_char =
                byte_off(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *slot = p;
                    stbds_temp_key_set(a, p);
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(ptr::addr_of_mut!((*table).string), key as *mut c_char);
                    *slot = p;
                    stbds_temp_key_set(a, p);
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *slot = p;
                    stbds_temp_key_set(a, p);
                }
                _ => {
                    memcpy(
                        byte_off(a, elemsize.wrapping_mul(i as usize)),
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
                let mut b = (*table).storage.wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1).wrapping_sub(1);
                STBDS_ASSERT!(slot < (*table).slot_count as isize);
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                stbds_temp_set(raw_a, 1);
                // STBDS_ASSERT(table->used_count >= 0) -- always true for size_t.
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
                    stbds_free(
                        *(byte_off(a, elemsize.wrapping_mul(old_index as usize))
                            as *mut *mut c_char) as *mut c_void,
                    );
                }

                if old_index != final_index {
                    memmove(
                        byte_off(a, elemsize.wrapping_mul(old_index as usize)),
                        byte_off(a, elemsize.wrapping_mul(final_index as usize)) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            *(byte_off(
                                a,
                                elemsize
                                    .wrapping_mul(old_index as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            byte_off(
                                a,
                                elemsize
                                    .wrapping_mul(old_index as usize)
                                    .wrapping_add(keyoffset),
                            ),
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    STBDS_ASSERT!(slot >= 0);
                    b = (*table).storage.wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    STBDS_ASSERT!((*b).index[i as usize] == final_index);
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
// String arena
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_).wrapping_add(1);
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize =
            STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = stbds_realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>().wrapping_sub(8).wrapping_add(len),
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
                size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(blocksize),
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    STBDS_ASSERT!(len <= (*a).remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_offset((*a).remaining as isize)
        .wrapping_offset(-(len as isize));
    (*a).remaining = (*a).remaining.wrapping_sub(len);
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
    memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// Test helpers exported by the C library
// ---------------------------------------------------------------------------

/// `static char buffer[256];`
static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(BUFFER) as *mut c_char;
    sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
    buf
}

// --- macro emulation for the `hm_geti` test ---------------------------------
//
// In `hm_geti` the table type is `struct { int key; int value; } *`, i.e.
// elemsize == 8 and sizeof(key) == 4, keyoffset == 0.

const HM_ELEMSIZE: usize = 8;
const HM_KEYSIZE: usize = 4;

/// `stbds_temp((t)-1)`
#[inline]
unsafe fn hm_temp(t: *mut c_void) -> isize {
    stbds_temp_get(stbds_hash_to_arr(t, HM_ELEMSIZE))
}

/// `&(t)[idx]`
#[inline]
fn hm_elem(t: *mut c_void, idx: isize) -> *mut u8 {
    (t as *mut u8).wrapping_offset(idx.wrapping_mul(HM_ELEMSIZE as isize))
}

#[inline]
unsafe fn hm_set_key(t: *mut c_void, idx: isize, k: c_int) {
    *(hm_elem(t, idx) as *mut c_int) = k;
}

#[inline]
unsafe fn hm_set_value(t: *mut c_void, idx: isize, v: c_int) {
    *(hm_elem(t, idx).wrapping_add(4) as *mut c_int) = v;
}

#[inline]
unsafe fn hm_get_value(t: *mut c_void, idx: isize) -> c_int {
    *(hm_elem(t, idx).wrapping_add(4) as *const c_int)
}

/// `hmgeti(t,k)`
#[inline]
unsafe fn hm_hmgeti(t: &mut *mut c_void, k: c_int) -> isize {
    let mut kv: c_int = k;
    *t = stbds_hmget_key(
        *t,
        HM_ELEMSIZE,
        ptr::addr_of_mut!(kv) as *mut c_void,
        HM_KEYSIZE,
        STBDS_HM_BINARY,
    );
    hm_temp(*t)
}

/// `hmgeti_ts(t,k,temp)`
#[inline]
unsafe fn hm_hmgeti_ts(t: &mut *mut c_void, k: c_int, temp: &mut isize) -> isize {
    let mut kv: c_int = k;
    *t = stbds_hmget_key_ts(
        *t,
        HM_ELEMSIZE,
        ptr::addr_of_mut!(kv) as *mut c_void,
        HM_KEYSIZE,
        temp as *mut isize,
        STBDS_HM_BINARY,
    );
    *temp
}

/// `hmget(t,k)`
#[inline]
unsafe fn hm_hmget(t: &mut *mut c_void, k: c_int) -> c_int {
    let _ = hm_hmgeti(t, k);
    hm_get_value(*t, hm_temp(*t))
}

/// `hmget_ts(t,k,temp)`
#[inline]
unsafe fn hm_hmget_ts(t: &mut *mut c_void, k: c_int, temp: &mut isize) -> c_int {
    let _ = hm_hmgeti_ts(t, k, temp);
    hm_get_value(*t, *temp)
}

/// `hmput(t,k,v)`
#[inline]
unsafe fn hm_hmput(t: &mut *mut c_void, k: c_int, v: c_int) {
    let mut kv: c_int = k;
    *t = stbds_hmput_key(
        *t,
        HM_ELEMSIZE,
        ptr::addr_of_mut!(kv) as *mut c_void,
        HM_KEYSIZE,
        STBDS_HM_BINARY,
    );
    hm_set_key(*t, hm_temp(*t), k);
    hm_set_value(*t, hm_temp(*t), v);
}

/// `hmdel(t,k)`
#[inline]
unsafe fn hm_hmdel(t: &mut *mut c_void, k: c_int) -> isize {
    let mut kv: c_int = k;
    *t = stbds_hmdel_key(
        *t,
        HM_ELEMSIZE,
        ptr::addr_of_mut!(kv) as *mut c_void,
        HM_KEYSIZE,
        0, // STBDS_OFFSETOF((t),key)
        STBDS_HM_BINARY,
    );
    if !(*t).is_null() {
        hm_temp(*t)
    } else {
        0
    }
}

/// `hmdefault(t,v)`
#[inline]
unsafe fn hm_hmdefault(t: &mut *mut c_void, v: c_int) {
    *t = stbds_hmput_default(*t, HM_ELEMSIZE);
    hm_set_value(*t, -1, v);
}

/// `hmfree(t)`
#[inline]
unsafe fn hm_hmfree(t: &mut *mut c_void) {
    if !(*t).is_null() {
        stbds_hmfree_func(stbds_hash_to_arr(*t, HM_ELEMSIZE), HM_ELEMSIZE);
    }
    *t = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    let mut intmap: *mut c_void = ptr::null_mut();
    let mut temp: isize = 0;
    let mut i: c_int;

    i = 1;
    STBDS_ASSERT!(hm_hmgeti(&mut intmap, i) == -1);
    hm_hmdefault(&mut intmap, -2);
    STBDS_ASSERT!(hm_hmgeti(&mut intmap, i) == -1);
    STBDS_ASSERT!(hm_hmget(&mut intmap, i) == -2);

    i = 0;
    while i < num {
        hm_hmput(&mut intmap, i, i.wrapping_mul(5));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            STBDS_ASSERT!(hm_hmget(&mut intmap, i) == -2);
        } else {
            STBDS_ASSERT!(hm_hmget(&mut intmap, i) == i.wrapping_mul(5));
        }
        if (i & 1) != 0 {
            STBDS_ASSERT!(hm_hmget_ts(&mut intmap, i, &mut temp) == -2);
        } else {
            STBDS_ASSERT!(hm_hmget_ts(&mut intmap, i, &mut temp) == i.wrapping_mul(5));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        hm_hmput(&mut intmap, i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            STBDS_ASSERT!(hm_hmget(&mut intmap, i) == -2);
        } else {
            STBDS_ASSERT!(hm_hmget(&mut intmap, i) == i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 2;
    while i < num {
        let _ = hm_hmdel(&mut intmap, i);
        i = i.wrapping_add(4);
    }

    i = 0;
    while i < num {
        if (i & 3) != 0 {
            STBDS_ASSERT!(hm_hmget(&mut intmap, i) == -2);
        } else {
            STBDS_ASSERT!(hm_hmget(&mut intmap, i) == i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        let _ = hm_hmdel(&mut intmap, i);
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        STBDS_ASSERT!(hm_hmget(&mut intmap, i) == -2);
        i = i.wrapping_add(1);
    }

    hm_hmfree(&mut intmap);

    i = 0;
    while i < num {
        hm_hmput(&mut intmap, i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    hm_hmfree(&mut intmap);
}
