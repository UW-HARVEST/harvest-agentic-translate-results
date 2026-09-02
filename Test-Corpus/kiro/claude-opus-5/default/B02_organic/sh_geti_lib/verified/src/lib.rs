//! Rust translation of c_src/src/lib.c (stb_ds.h implementation + `sh_geti`/`strkey` test code).
//!
//! The translation is deliberately literal: it mirrors the original pointer
//! arithmetic, the C integer-promotion / sign-extension quirks, and the exact
//! order of side effects and error checks.  It does NOT fix any bug present in
//! the original C.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (we deliberately use the C allocator + C stdio so that memory
// and output are interchangeable with the original library).
// ---------------------------------------------------------------------------
extern "C" {
    fn realloc(p: *mut c_void, s: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn abort() -> !;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

/// Mirrors `STBDS_ASSERT` == `assert` for a build *without* `NDEBUG`.
macro_rules! stbds_assert {
    ($cond:expr) => {
        if !($cond) {
            unsafe { abort() }
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
pub struct stbds_string_arena {
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

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const HDRSIZE: usize = core::mem::size_of::<stbds_array_header>();

// Layout sanity checks (compile-time).
const _: () = assert!(HDRSIZE == 32);
const _: () = assert!(core::mem::size_of::<stbds_hash_bucket>() == 128);
const _: () = assert!(core::mem::size_of::<stbds_string_arena>() == 24);
const _: () = assert!(core::mem::size_of::<stbds_string_block>() == 16);
const _: () = assert!(core::mem::size_of::<stbds_hash_index>() == 104);
const _: () = assert!(core::mem::size_of::<usize>() == 8);

#[inline]
fn is_index_in_use(x: isize) -> bool {
    x >= 0
}

/// `stbds_header(t)` -- `((stbds_array_header *) (t) - 1)`
#[inline]
fn hdr(a: *mut c_void) -> *mut stbds_array_header {
    (a as *mut u8).wrapping_sub(HDRSIZE) as *mut stbds_array_header
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline]
fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline]
fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).wrapping_add(elemsize) as *mut c_void
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*hdr(a)).length as isize
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*hdr(a)).capacity
    }
}

/// `stbds_hash_table(a)`
#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*hdr(a)).hash_table as *mut stbds_hash_index
}

/// `stbds_temp(t)` (as an lvalue helper pair)
#[inline]
unsafe fn temp_get(a: *mut c_void) -> isize {
    (*hdr(a)).temp
}

#[inline]
unsafe fn temp_set(a: *mut c_void, v: isize) {
    (*hdr(a)).temp = v;
}

/// `stbds_temp_key(t)` -- `(*(char **) stbds_header(t)->hash_table)`
#[inline]
unsafe fn temp_key_set(a: *mut c_void, v: *mut c_char) {
    let table = (*hdr(a)).hash_table as *mut *mut c_char;
    *table = v;
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
    let min_len: usize = (arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= arrcap(a) {
        return a;
    }

    if min_cap < 2usize.wrapping_mul(arrcap(a)) {
        min_cap = 2usize.wrapping_mul(arrcap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old: *mut c_void = if !a.is_null() {
        hdr(a) as *mut c_void
    } else {
        ptr::null_mut()
    };
    let raw = realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HDRSIZE),
    );
    let b = (raw as *mut u8).wrapping_add(HDRSIZE) as *mut c_void;

    if a.is_null() {
        (*hdr(b)).length = 0;
        (*hdr(b)).hash_table = ptr::null_mut();
        (*hdr(b)).temp = 0;
    }
    (*hdr(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    // NOTE: the C code does not null-check `a` here; reproduced verbatim.
    free(hdr(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
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

/// `STBDS_ALIGN_FWD(n,a)`
#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// Reproduces the `stbds_load_32_or_64` macro on a 64-bit target.
#[inline]
fn load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    // var = v64_hi, var <<= 16, var <<= 16, var ^= temp ^ v32
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
    let t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = align_fwd(
        (t as usize).wrapping_add(core::mem::size_of::<stbds_hash_index>()),
        STBDS_CACHE_LINE_SIZE,
    ) as *mut stbds_hash_bucket;
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
    stbds_assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = stbds_string_arena {
            storage: (*ot).string.storage,
            remaining: (*ot).string.remaining,
            block: (*ot).string.block,
            mode: (*ot).string.mode,
        };
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(
            ptr::addr_of_mut!((*t).string) as *mut u8,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = STBDS_HASH_SEED;
        let a = load_32_or_64(2147001325u32, 0x27bb2ee6, 0x87b0b0fd);
        let b = load_32_or_64(715136305u32, 0, 0xb504f32d);
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < (slot_count >> STBDS_BUCKET_SHIFT) {
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
        let mut i: usize = 0;
        while i < ((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if is_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
            }
            i += 1;
        }
    }

    t
}

#[inline]
fn rotl(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotr(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = rotl(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotr(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotr(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, 32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, 32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = ((((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = ((((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = ((((0x74656462usize) << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i: usize = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // NOTE: the C source builds the low half with `int` arithmetic, so a
        // byte with the high bit set in d[3] yields a negative `int` which is
        // then *sign-extended* into `size_t`.  Reproduced exactly.
        let lo32: u32 = (*d.add(0) as u32)
            | ((*d.add(1) as u32) << 8)
            | ((*d.add(2) as u32) << 16)
            | ((*d.add(3) as u32) << 24);
        data = ((lo32 as i32) as isize) as usize;
        let hi32: u32 = (*d.add(4) as u32)
            | ((*d.add(5) as u32) << 8)
            | ((*d.add(6) as u32) << 16)
            | ((*d.add(7) as u32) << 24);
        data |= (hi32 as usize) << 32;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len << (core::mem::size_of::<usize>() * 8 - 8);
    let rem = len.wrapping_sub(i);
    // C `switch` with fall-through from `case 7` down to `case 1`.
    if rem == 7 || rem == 6 || rem == 5 || rem == 4 || rem == 3 || rem == 2 || rem == 1 {
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
            // `d[3] << 24` is `int` arithmetic in C, then sign-extended.
            data |= ((((*d.add(3) as u32) << 24) as i32) as isize) as usize;
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
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

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
        let slot = (a as *mut u8)
            .wrapping_add(elemsize.wrapping_mul(i))
            .wrapping_add(keyoffset) as *mut *mut c_char;
        (0 == strcmp(key as *const c_char, *slot)) as c_int
    } else {
        let slot = (a as *mut u8)
            .wrapping_add(elemsize.wrapping_mul(i))
            .wrapping_add(keyoffset) as *const c_void;
        (0 == memcmp(key as *const c_void, slot, keysize)) as c_int
    }
}

// ---------------------------------------------------------------------------
// hash map core
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !hash_table(a).is_null() {
        if (*hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i: usize = 1;
            while i < (*hdr(a)).length {
                let slot = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i)) as *mut *mut c_char;
                free(*slot as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*hash_table(a)).string));
    }
    free((*hdr(a)).hash_table);
    free(hdr(a) as *mut c_void);
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
    let table = hash_table(raw_a);
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
    let keyoffset: usize = 0;
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*hdr(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*hdr(raw_a)).hash_table as *mut stbds_hash_index;
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
    temp_set(hash_to_arr(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null() || (*hdr(hash_to_arr(a, elemsize))).length == 0 {
        let base = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        a = stbds_arrgrowf(base, elemsize, 0, 1);
        (*hdr(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
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

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*hdr(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*hdr(a)).hash_table as *mut stbds_hash_index;

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
                STBDS_SH_NONE
            };
        }
        table = nt;
        (*hdr(a)).hash_table = table as *mut c_void;
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
            hash = hash.wrapping_add(2);
        }

        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

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
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let slot = (raw_a as *mut u8)
                                .wrapping_add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                .wrapping_add(keyoffset) as *mut *mut c_char;
                            temp_key_set(a, *slot);
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
                        // NOTE: unlike the loop above, the original does not
                        // update `stbds_temp_key` here.  Reproduced verbatim.
                        temp_set(a, (*bucket).index[i]);
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
            let i: isize = arrlen(a);
            if (i as usize).wrapping_add(1) > arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);

            stbds_assert!((i as usize).wrapping_add(1) <= arrcap(a));
            (*hdr(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            temp_set(a, i - 1);

            let dst = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i as usize));
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *(dst as *mut *mut c_char) = p;
                    temp_key_set(a, p);
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *(dst as *mut *mut c_char) = p;
                    temp_key_set(a, p);
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *(dst as *mut *mut c_char) = p;
                    temp_key_set(a, p);
                }
                _ => {
                    memcpy(dst as *mut c_void, key as *const c_void, keysize);
                }
            }
        }
        let _ = raw_a;
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*hdr(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*hdr(a)).hash_table = h as *mut c_void;
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
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let table = (*hdr(raw_a)).hash_table as *mut stbds_hash_index;
    temp_set(raw_a, 0);
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index: isize = arrlen(raw_a) - 1 - 1;
    stbds_assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    temp_set(raw_a, 1);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let slotp = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(old_index as usize))
            as *mut *mut c_char;
        free(*slotp as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(final_index as usize))
                as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let slotp = (a as *mut u8)
                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                .wrapping_add(keyoffset) as *mut *mut c_char;
            slot = stbds_hm_find_slot(a, elemsize, *slotp as *mut c_void, keysize, keyoffset, mode);
        } else {
            let slotp = (a as *mut u8)
                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                .wrapping_add(keyoffset) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, slotp, keysize, keyoffset, mode);
        }
        stbds_assert!(slot >= 0);
        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        stbds_assert!((*b).index[i] == final_index);
        (*b).index[i] = old_index;
    }
    (*hdr(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*hdr(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*hdr(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
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

    stbds_assert!(len <= (*a).remaining);
    let p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining as isize as usize)
        .wrapping_sub(len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, str_ as *const c_void, len);
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
    ptr::write_bytes(
        a as *mut u8,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}

// ---------------------------------------------------------------------------
// test code from the bottom of lib.c
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(BUFFER) as *mut c_char;
    sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
    buf
}

/// `struct { char *key; int value; }` used by `sh_geti`.
#[repr(C)]
#[derive(Copy, Clone)]
struct ShEntry {
    key: *mut c_char,
    value: c_int,
}

const SH_ELEMSIZE: usize = core::mem::size_of::<ShEntry>();
const _: () = assert!(SH_ELEMSIZE == 16);
/// `sizeof (t)->key`
const SH_KEYSIZE: usize = core::mem::size_of::<*mut c_char>();

#[inline]
unsafe fn sh_temp(t: *mut ShEntry) -> isize {
    // stbds_temp((t)-1)
    temp_get(hash_to_arr(t as *mut c_void, SH_ELEMSIZE))
}

#[inline]
unsafe fn sh_len(t: *mut ShEntry) -> isize {
    if t.is_null() {
        0
    } else {
        (*hdr(hash_to_arr(t as *mut c_void, SH_ELEMSIZE))).length as isize - 1
    }
}

#[inline]
unsafe fn sh_geti_macro(t: &mut *mut ShEntry, k: *mut c_char) -> isize {
    *t = stbds_hmget_key(
        *t as *mut c_void,
        SH_ELEMSIZE,
        k as *mut c_void,
        SH_KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut ShEntry;
    sh_temp(*t)
}

#[inline]
unsafe fn sh_put(t: &mut *mut ShEntry, k: *mut c_char, v: c_int) {
    *t = stbds_hmput_key(
        *t as *mut c_void,
        SH_ELEMSIZE,
        k as *mut c_void,
        SH_KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut ShEntry;
    let idx = sh_temp(*t);
    (*(*t).offset(idx)).value = v;
}

#[inline]
unsafe fn sh_get(t: &mut *mut ShEntry, k: *mut c_char) -> c_int {
    let _ = sh_geti_macro(t, k);
    let idx = sh_temp(*t);
    (*(*t).offset(idx)).value
}

#[inline]
unsafe fn sh_del(t: &mut *mut ShEntry, k: *mut c_char) -> isize {
    *t = stbds_hmdel_key(
        *t as *mut c_void,
        SH_ELEMSIZE,
        k as *mut c_void,
        SH_KEYSIZE,
        0, // STBDS_OFFSETOF((t),key)
        STBDS_HM_STRING,
    ) as *mut ShEntry;
    if !(*t).is_null() {
        sh_temp(*t)
    } else {
        0
    }
}

#[inline]
unsafe fn sh_free(t: &mut *mut ShEntry) {
    if !(*t).is_null() {
        stbds_hmfree_func(hash_to_arr(*t as *mut c_void, SH_ELEMSIZE), SH_ELEMSIZE);
    }
    *t = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let mut strmap: *mut ShEntry = ptr::null_mut();
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let foo = b"foo\0".as_ptr() as *mut c_char;
    let fmt = b"%s %d\n\0".as_ptr() as *const c_char;

    let mut i: c_int = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    let mut j: c_int = 0;
    while j < 2 {
        stbds_assert!(sh_geti_macro(&mut strmap, foo) == -1);
        if j == 0 {
            strmap = stbds_shmode_func(SH_ELEMSIZE, STBDS_SH_STRDUP as c_int) as *mut ShEntry;
        } else {
            strmap = stbds_shmode_func(SH_ELEMSIZE, STBDS_SH_ARENA as c_int) as *mut ShEntry;
        }
        stbds_assert!(sh_geti_macro(&mut strmap, foo) == -1);
        // shdefault(strmap, -2)
        strmap = stbds_hmput_default(strmap as *mut c_void, SH_ELEMSIZE) as *mut ShEntry;
        (*strmap.offset(-1)).value = -2;
        stbds_assert!(sh_geti_macro(&mut strmap, foo) == -1);

        i = 0;
        while i < num {
            sh_put(&mut strmap, strkey(i), i.wrapping_mul(3));
            i += 2;
        }

        let mut z: c_int = 0;
        while (z as isize) < sh_len(strmap) {
            // The C code passes the whole struct by value to printf; under the
            // SysV x86-64 ABI that places `key` and `value` in the same two
            // argument registers the "%s %d" conversions read.
            let e = *strmap.offset(z as isize);
            printf(fmt, e.key, e.value);
            z += 1;
        }

        i = 0;
        while i < num {
            if i & 1 != 0 {
                stbds_assert!(sh_get(&mut strmap, strkey(i)) == -2);
            } else {
                stbds_assert!(sh_get(&mut strmap, strkey(i)) == i.wrapping_mul(3));
            }
            i += 1;
        }
        i = 2;
        while i < num {
            sh_del(&mut strmap, strkey(i));
            i += 4;
        }
        i = 0;
        while i < num {
            if i & 3 != 0 {
                stbds_assert!(sh_get(&mut strmap, strkey(i)) == -2);
            } else {
                stbds_assert!(sh_get(&mut strmap, strkey(i)) == i.wrapping_mul(3));
            }
            i += 1;
        }
        i = 0;
        while i < num {
            sh_del(&mut strmap, strkey(i));
            i += 1;
        }
        i = 0;
        while i < num {
            stbds_assert!(sh_get(&mut strmap, strkey(i)) == -2);
            i += 1;
        }

        sh_free(&mut strmap);
        j += 1;
    }
}
