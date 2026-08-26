//! Rust translation of `c_src/src/lib.c` (an `stb_ds.h` derived hash-map /
//! dynamic-array implementation plus the `sh_puts` / `strkey` helpers).
//!
//! The translation is intentionally a literal, pointer-for-pointer rendering of
//! the C original: memory layouts, allocation sizes, integer promotions,
//! wrap-around arithmetic and even the C bugs (e.g. `hash ^= hash ^ rot(...)`,
//! the sign-extending `d[3] << 24` inside the sip-hash tail) are reproduced so
//! that the resulting `cdylib` is ABI- and byte-output-compatible with the C
//! shared library.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings.  The C code uses realloc/free/mem*/str*/printf/sprintf, so we
// call the very same libc entry points to keep allocation and stdio behaviour
// (buffering, interleaving with a C caller) byte-identical.
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
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// The C translation unit's `__FILE__`, supplied by `build.rs` (see the note
/// there): glibc's `assert` embeds it in the abort message.
const ASSERT_FILE: &str = concat!(env!("STBDS_ASSERT_FILE"), "\0");

/// Mirrors `assert()` from glibc (`STBDS_ASSERT`).
macro_rules! stbds_assert {
    ($cond:expr, $text:expr, $line:expr, $func:expr) => {
        if !($cond) {
            unsafe {
                __assert_fail(
                    concat!($text, "\0").as_ptr() as *const c_char,
                    ASSERT_FILE.as_ptr() as *const c_char,
                    $line as c_uint,
                    concat!($func, "\0").as_ptr() as *const c_char,
                )
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Types (layout-identical to the C structs)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Copy, Clone)]
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

#[allow(dead_code)]
const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();
const HASH_INDEX_SIZE: usize = core::mem::size_of::<stbds_hash_index>();
const HASH_BUCKET_SIZE: usize = core::mem::size_of::<stbds_hash_bucket>();
const STRING_BLOCK_SIZE: usize = core::mem::size_of::<stbds_string_block>();
const STRING_ARENA_SIZE: usize = core::mem::size_of::<stbds_string_arena>();

// Guard the layout assumptions the C code makes.
const _: () = assert!(HEADER_SIZE == 32);
const _: () = assert!(HASH_BUCKET_SIZE == 128);
const _: () = assert!(HASH_INDEX_SIZE == 104);
const _: () = assert!(STRING_BLOCK_SIZE == 16);
const _: () = assert!(STRING_ARENA_SIZE == 24);
const _: () = assert!(core::mem::size_of::<usize>() == 8);

// ---------------------------------------------------------------------------
// Small helpers corresponding to the stb_ds macros
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *) (t) - 1)`
#[inline(always)]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `stbds_arrcap(a)`
#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `stbds_arrlen(a)`
#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `stbds_hash_table(a)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_HASH_TO_ARR(x,elemsize)`
#[inline(always)]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x,elemsize)`
#[inline(always)]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// byte offset from a `void *`
#[inline(always)]
fn byte_off(p: *mut c_void, off: usize) -> *mut u8 {
    (p as *mut u8).wrapping_add(off)
}

/// `stbds_temp_key(t)` lvalue == `*(char **) stbds_header(t)->hash_table`
#[inline(always)]
unsafe fn stbds_temp_key_ptr(t: *mut c_void) -> *mut *mut c_char {
    (*stbds_header(t)).hash_table as *mut *mut c_char
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
    let raw = realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(HEADER_SIZE),
    );
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
// hash seed / hash index construction
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
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// `STBDS_ALIGN_FWD(n,a)`
#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a).wrapping_sub(1) & !a.wrapping_sub(1)
}

/// Faithful expansion of
/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)` on a 64-bit `size_t`,
/// where `v32`/`v64_hi` are `int` constants and `v64_lo` is an `unsigned int`
/// constant.
#[inline(always)]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // temp = v64_lo ^ v32;  temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
    let mut temp: usize = (v64_lo ^ v32) as usize; // unsigned int arithmetic
    temp = temp << 16;
    temp = temp << 16;
    temp = temp >> 16;
    temp = temp >> 16;
    // var = v64_hi; var <<= 16; var <<= 16; var ^= temp ^ v32;
    let mut var: usize = v64_hi as usize;
    var = var << 16;
    var = var << 16;
    var ^= temp ^ (v32 as usize);
    var
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t: *mut stbds_hash_index = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(HASH_BUCKET_SIZE)
            .wrapping_add(HASH_INDEX_SIZE)
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;
    (*t).storage = stbds_align_fwd(
        (t as usize).wrapping_add(HASH_INDEX_SIZE),
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
            STRING_ARENA_SIZE,
        );
        (*t).seed = stbds_hash_seed;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b: *mut stbds_hash_bucket = (*t).storage.wrapping_add(i);
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
            let ob: *mut stbds_hash_bucket = (*ot).storage.wrapping_add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash: usize = (*ob).hash[j];
                    let mut pos: usize =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step: usize = STBDS_BUCKET_LENGTH;
                    'probe: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z: usize = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'probe;
                            }
                            z += 1;
                        }

                        let limit: usize = pos & STBDS_BUCKET_MASK;
                        z = 0;
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

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

// `STBDS_ROTATE_LEFT` / `STBDS_ROTATE_RIGHT` compile to x86 `shl`/`shr`, which
// mask the shift count with `& 63`.  `wrapping_shl`/`wrapping_shr` reproduce
// that masking exactly and, unlike `<<`/`>>`, never panic in a debug build.
#[inline(always)]
fn rotl(val: usize, n: u32) -> usize {
    val.wrapping_shl(n) | val.wrapping_shr(STBDS_SIZE_T_BITS.wrapping_sub(n))
}

#[inline(always)]
fn rotr(val: usize, n: u32) -> usize {
    val.wrapping_shr(n) | val.wrapping_shl(STBDS_SIZE_T_BITS.wrapping_sub(n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *mut u8;
    while *s != 0 {
        hash = rotl(hash, 9).wrapping_add(*s as usize);
        s = s.wrapping_add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash = hash ^ (hash ^ rotr(hash, 31));
    hash = hash.wrapping_mul(21);
    hash = hash ^ (hash ^ rotr(hash, 11));
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline(always)]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;
    let mut i: usize;
    let mut j: usize;
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
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);`
        // The right hand side is `int`; when d[3] >= 0x80 it becomes negative
        // and sign-extends into `size_t`.  Reproduce that exactly.
        let lo: u32 = (*d.wrapping_add(0) as u32)
            | ((*d.wrapping_add(1) as u32) << 8)
            | ((*d.wrapping_add(2) as u32) << 16)
            | ((*d.wrapping_add(3) as u32) << 24);
        data = (lo as i32) as isize as usize;
        let hi: u32 = (*d.wrapping_add(4) as u32)
            | ((*d.wrapping_add(5) as u32) << 8)
            | ((*d.wrapping_add(6) as u32) << 16)
            | ((*d.wrapping_add(7) as u32) << 24);
        data |= (((hi as i32) as isize as usize) << 16) << 16;

        v3 ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.wrapping_add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len.wrapping_sub(i);
    // Fall-through switch on `len - i` (0..=7).
    if rem == 7 {
        data |= ((*d.wrapping_add(6) as usize) << 24) << 24;
    }
    if rem >= 6 && rem <= 7 {
        data |= ((*d.wrapping_add(5) as usize) << 20) << 20;
    }
    if rem >= 5 && rem <= 7 {
        data |= ((*d.wrapping_add(4) as usize) << 16) << 16;
    }
    if rem >= 4 && rem <= 7 {
        // `d[3] << 24` is `int` arithmetic -> sign extension on conversion.
        data |= (((*d.wrapping_add(3) as u32) << 24) as i32) as isize as usize;
    }
    if rem >= 3 && rem <= 7 {
        data |= (*d.wrapping_add(2) as usize) << 16;
    }
    if rem >= 2 && rem <= 7 {
        data |= (*d.wrapping_add(1) as usize) << 8;
    }
    if rem >= 1 && rem <= 7 {
        data |= *d.wrapping_add(0) as usize;
    }

    v3 ^= data;
    j = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    j = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
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
        let slot = byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *mut *mut c_char;
        (0 == strcmp(key as *const c_char, *slot)) as c_int
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
                let slot = byte_off(a, elemsize.wrapping_mul(i)) as *mut *mut c_char;
                free(*slot as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&mut (*stbds_hash_table(a)).string);
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
    let raw_a: *mut c_void = stbds_hash_to_arr(a, elemsize);
    let table: *mut stbds_hash_index = stbds_hash_table(raw_a);
    let mut hash: usize = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: usize = STBDS_BUCKET_LENGTH;
    let mut pos: usize;
    let bucket: *mut stbds_hash_bucket;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket: *mut stbds_hash_bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

        let mut i: usize = pos & STBDS_BUCKET_MASK;
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

        let limit: usize = pos & STBDS_BUCKET_MASK;
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
    let mut a = a;
    let keyoffset: usize = 0;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        stbds_arr_to_hash(a, elemsize)
    } else {
        let table: *mut stbds_hash_index;
        let raw_a: *mut c_void = stbds_hash_to_arr(a, elemsize);
        table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot: isize = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b: *mut stbds_hash_bucket = (*table)
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
    (*stbds_header(stbds_hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null()
        || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0
    {
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
    let mut a = a;
    let keyoffset: usize = 0;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
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
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

            let mut i: usize = pos & STBDS_BUCKET_MASK;
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
                        (*stbds_header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            let src = byte_off(
                                raw_a,
                                elemsize
                                    .wrapping_mul((*bucket).index[i] as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char;
                            *stbds_temp_key_ptr(a) = *src;
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

            let limit: usize = pos & STBDS_BUCKET_MASK;
            i = 0;
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
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        (*stbds_header(a)).temp = (*bucket).index[i];
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    found_empty = true;
                    break;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                }
                i += 1;
            }
            if found_empty {
                break 'found_empty_slot;
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
                "(size_t) i+1 <= stbds_arrcap(a)",
                778,
                "stbds_hmput_key"
            );
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            (*stbds_header(a)).temp = i - 1;

            let slot = byte_off(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
            let mode_str = (*table).string.mode as c_int;
            if mode_str == STBDS_SH_STRDUP {
                let v = stbds_strdup(key as *mut c_char);
                *slot = v;
                *stbds_temp_key_ptr(a) = v;
            } else if mode_str == STBDS_SH_ARENA {
                let v = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                *slot = v;
                *stbds_temp_key_ptr(a) = v;
            } else if mode_str == STBDS_SH_DEFAULT {
                let v = key as *mut c_char;
                *slot = v;
                *stbds_temp_key_ptr(a) = v;
            } else {
                memcpy(slot as *mut c_void, key as *const c_void, keysize);
            }
        }
        stbds_arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a: *mut c_void = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    let h: *mut stbds_hash_index;
    memset(a, 0, elemsize);
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
        return ptr::null_mut();
    } else {
        let table: *mut stbds_hash_index;
        let raw_a: *mut c_void = stbds_hash_to_arr(a, elemsize);
        table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        (*stbds_header(raw_a)).temp = 0;
        if table.is_null() {
            return a;
        } else {
            let mut slot: isize;
            slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                return a;
            } else {
                let mut b: *mut stbds_hash_bucket = (*table)
                    .storage
                    .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                stbds_assert!(
                    slot < (*table).slot_count as isize,
                    "slot < (ptrdiff_t) table->slot_count",
                    828,
                    "stbds_hmdel_key"
                );
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                (*stbds_header(raw_a)).temp = 1;
                // STBDS_ASSERT(table->used_count >= 0) -- always true for size_t
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING
                    && (*table).string.mode == STBDS_SH_STRDUP as u8
                {
                    let slotp =
                        byte_off(a, elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char;
                    free(*slotp as *mut c_void);
                }

                if old_index != final_index {
                    memmove(
                        byte_off(a, elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
                        byte_off(a, elemsize.wrapping_mul(final_index as usize)) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        let kp = byte_off(
                            a,
                            elemsize
                                .wrapping_mul(old_index as usize)
                                .wrapping_add(keyoffset),
                        ) as *mut *mut c_char;
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            *kp as *mut c_void,
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
                            ) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    stbds_assert!(slot >= 0, "slot >= 0", 846, "stbds_hmdel_key");
                    b = (*table)
                        .storage
                        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    stbds_assert!(
                        (*b).index[i as usize] == final_index,
                        "b->index[i] == final_index",
                        849,
                        "stbds_hmdel_key"
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

                return a;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// string arena
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len: usize = strlen(str_).wrapping_add(1);
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len: usize = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        // `(size_t) 512u << (a->block >> 1)` — `a->block` is an `unsigned char`,
        // so the shift count can reach 127.  gcc emits `shl`, which masks the
        // count with `& 63`; `wrapping_shl` reproduces that exactly (and, unlike
        // `<<`, does not panic in a debug build).
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                STRING_BLOCK_SIZE.wrapping_sub(8).wrapping_add(len),
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
            let sb = realloc(
                ptr::null_mut(),
                STRING_BLOCK_SIZE.wrapping_sub(8).wrapping_add(blocksize),
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
        free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, STRING_ARENA_SIZE);
}

// ---------------------------------------------------------------------------
// strkey / sh_puts
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = (&raw mut buffer) as *mut c_char;
    sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
    buf
}

/// `struct { char *key; int value; }` from `sh_puts`.
#[repr(C)]
#[derive(Copy, Clone)]
struct sh_puts_entry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    let elemsize: usize = core::mem::size_of::<sh_puts_entry>(); // 16
    let mut strmap: *mut sh_puts_entry = ptr::null_mut();
    let mut s = sh_puts_entry {
        key: ptr::null_mut(),
        value: 0,
    };
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut i: c_int;

    i = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    {
        s.key = b"a\0".as_ptr() as *mut c_char;
        s.value = num;

        // sh_new_arena(strmap)
        strmap = stbds_shmode_func(elemsize, STBDS_SH_ARENA) as *mut sh_puts_entry;

        // shputs(strmap, s)
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            s.key as *mut c_void,
            core::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut sh_puts_entry;
        let raw = stbds_hash_to_arr(strmap as *mut c_void, elemsize);
        *strmap.wrapping_offset((*stbds_header(raw)).temp) = s;
        (*strmap.wrapping_offset((*stbds_header(raw)).temp)).key = *stbds_temp_key_ptr(raw);

        stbds_assert!(
            *(*strmap.wrapping_offset(0)).key == b'a' as c_char,
            "*strmap[0].key == 'a'",
            959,
            "sh_puts"
        );
        stbds_assert!(
            (*strmap.wrapping_offset(0)).key != s.key,
            "strmap[0].key != s.key",
            960,
            "sh_puts"
        );
        stbds_assert!(
            (*strmap.wrapping_offset(0)).value == s.value,
            "strmap[0].value == s.value",
            961,
            "sh_puts"
        );

        // for (int z=0; z < shlen(strmap); ++z)
        //     printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // `strmap[z]` is a 16-byte struct passed through `...`; under the
        // SysV AMD64 ABI it occupies two INTEGER eightbytes, so `%s` consumes
        // the `key` pointer and `%d` consumes the `value` field.  The third
        // argument (`strmap[z].value`) is never read by the format string.
        let mut z: c_int = 0;
        while (z as isize)
            < (if !strmap.is_null() {
                (*stbds_header(stbds_hash_to_arr(strmap as *mut c_void, elemsize))).length as isize
                    - 1
            } else {
                0
            })
        {
            let e = *strmap.wrapping_offset(z as isize);
            printf(b"%s %d\n\0".as_ptr() as *const c_char, e.key, e.value);
            z += 1;
        }

        // shfree(strmap)
        if !strmap.is_null() {
            stbds_hmfree_func(stbds_hash_to_arr(strmap as *mut c_void, elemsize), elemsize);
        }
        strmap = ptr::null_mut();
    }
}
