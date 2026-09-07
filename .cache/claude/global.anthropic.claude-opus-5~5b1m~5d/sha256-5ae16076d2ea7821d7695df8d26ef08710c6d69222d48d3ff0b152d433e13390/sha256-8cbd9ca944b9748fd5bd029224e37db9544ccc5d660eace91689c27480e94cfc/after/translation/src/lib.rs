//! Rust translation of the C library in `c_src/` (an stb_ds.h derived
//! dynamic-array / hash-map implementation plus the small test helpers
//! `strkey()` and `arr_push()`).
//!
//! The translation is bit-for-bit faithful to the C original:
//!   * all data structures use the identical `#[repr(C)]` layout so that the
//!     `stb_ds.h` macros (which poke at the array header directly) keep working,
//!   * all allocations go through the C library's `realloc`/`free` so that
//!     memory allocated here can be released by C code (and vice versa),
//!   * every arithmetic quirk of the original (including implicit `int`
//!     promotions that sign-extend into `size_t`) is reproduced exactly,
//!   * the order of all checks/branches is preserved.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (STBDS_REALLOC / STBDS_FREE map onto realloc/free)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// `STBDS_REALLOC(c,p,s)  ->  realloc(p,s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    realloc(p, size)
}

/// `STBDS_FREE(c,p)  ->  free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    free(p)
}

// ---------------------------------------------------------------------------
// Small C string / memory helpers (equivalent to strlen/memcmp/strcmp/...)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    let mut p = s as *const u8;
    while *p != 0 {
        n += 1;
        p = p.add(1);
    }
    n
}

/// `0 == strcmp(a, b)`
#[inline]
unsafe fn c_str_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut pa = a as *const u8;
    let mut pb = b as *const u8;
    loop {
        let ca = *pa;
        let cb = *pb;
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        pa = pa.add(1);
        pb = pb.add(1);
    }
}

/// `0 == memcmp(a, b, n)`
#[inline]
unsafe fn c_mem_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    let mut i: usize = 0;
    while i < n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
        i += 1;
    }
    true
}

/// `memmove(dst, src, n)`
#[inline]
unsafe fn c_memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        ptr::copy(src, dst, n);
    }
}

/// `memcpy(dst, src, n)`
#[inline]
unsafe fn c_memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        ptr::copy_nonoverlapping(src, dst, n);
    }
}

/// `memset(dst, 0, n)`
#[inline]
unsafe fn c_memzero(dst: *mut u8, n: usize) {
    if n != 0 {
        ptr::write_bytes(dst, 0, n);
    }
}

/// `STBDS_ASSERT(x)` -- the original relies on `assert()`, which is compiled
/// out in release builds; the conditions are internal invariants only.
#[inline]
fn stbds_assert(_cond: bool) {}

// ---------------------------------------------------------------------------
// Data structures (layout identical to the C originals)
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
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Clone, Copy)]
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

// Compile-time confirmation that the layouts match the C compiler's.
const _: () = assert!(core::mem::size_of::<stbds_array_header>() == 32);
const _: () = assert!(core::mem::size_of::<stbds_string_block>() == 16);
const _: () = assert!(core::mem::size_of::<stbds_string_arena>() == 24);
const _: () = assert!(core::mem::size_of::<stbds_hash_bucket>() == 128);
const _: () = assert!(core::mem::size_of::<stbds_hash_index>() == 104);
const _: () = assert!(core::mem::offset_of!(stbds_hash_index, string) == 72);
const _: () = assert!(core::mem::offset_of!(stbds_hash_index, storage) == 96);
const _: () = assert!(core::mem::offset_of!(stbds_string_arena, block) == 16);
const _: () = assert!(core::mem::offset_of!(stbds_string_arena, mode) == 17);

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

// ---------------------------------------------------------------------------
// Header / array accessor helpers (the stb_ds.h macros)
// ---------------------------------------------------------------------------

/// `stbds_header(t)  ->  ((stbds_array_header *) (t) - 1)`
#[inline]
fn stbds_header(a: *mut c_void) -> *mut stbds_array_header {
    (a as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `stbds_arrlen(a)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `stbds_arrcap(a)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `stbds_temp(t) = v`
#[inline]
unsafe fn set_stbds_temp(a: *mut c_void, v: isize) {
    (*stbds_header(a)).temp = v;
}

/// `stbds_temp_key(t) = v`  (i.e. `*(char **) stbds_header(t)->hash_table = v`)
#[inline]
unsafe fn set_stbds_temp_key(a: *mut c_void, v: *mut c_char) {
    *((*stbds_header(a)).hash_table as *mut *mut c_char) = v;
}

/// `stbds_hash_table(a)  ->  ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_HASH_TO_ARR(x,elemsize)  ->  ((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x,elemsize)  ->  ((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `(char *) a + elemsize * i + keyoffset`
#[inline]
fn elem_ptr(a: *mut c_void, elemsize: usize, i: usize, keyoffset: usize) -> *mut u8 {
    (a as *mut u8)
        .wrapping_add(elemsize.wrapping_mul(i))
        .wrapping_add(keyoffset)
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
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    );
    let b = (b as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;
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
// Hash seed / hash index construction
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[inline]
unsafe fn hash_seed_get() -> usize {
    *(&raw const STBDS_HASH_SEED)
}

#[inline]
unsafe fn hash_seed_set(v: usize) {
    *(&raw mut STBDS_HASH_SEED) = v;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    hash_seed_set(seed);
}

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

/// `STBDS_ALIGN_FWD(n,a)  ->  (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

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

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t: *mut stbds_hash_index = stbds_realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = stbds_align_fwd(
        (t as *mut u8).wrapping_add(core::mem::size_of::<stbds_hash_index>()) as usize,
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
    stbds_assert(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count,
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        c_memzero(
            (&raw mut (*t).string) as *mut u8,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = hash_seed_get();
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let mut temp: usize;
        temp = (0x87b0b0fdu32 ^ 2147001325u32) as usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        let mut a: usize = 0x27bb2ee6usize;
        a <<= 16;
        a <<= 16;
        a ^= temp ^ 2147001325usize;
        // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d);
        temp = (0xb504f32du32 ^ 715136305u32) as usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        let mut b: usize = 0usize;
        b <<= 16;
        b <<= 16;
        b ^= temp ^ 715136305usize;
        hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));
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
            let ob: *mut stbds_hash_bucket = (*ot).storage.add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'probe: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
                        let mut z: usize = 0;
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
                j += 1;
            }
            i += 1;
        }
    }

    t
}

// ---------------------------------------------------------------------------
// Hash functions
// ---------------------------------------------------------------------------

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

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

#[inline]
fn stbds_sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = stbds_rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = stbds_rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut data: usize;

    let mut v0: usize = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1: usize = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2: usize = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3: usize = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    let mut i: usize = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        // The C code builds these words with `int` arithmetic, so
        // `d[3] << 24` overflows into the sign bit and is then
        // sign-extended when widened to size_t.  Reproduce that exactly.
        let lo: u32 = (*d.add(0) as u32)
            | ((*d.add(1) as u32) << 8)
            | ((*d.add(2) as u32) << 16)
            | ((*d.add(3) as u32) << 24);
        data = ((lo as i32) as isize) as usize;
        let hi: u32 = (*d.add(4) as u32)
            | ((*d.add(5) as u32) << 8)
            | ((*d.add(6) as u32) << 16)
            | ((*d.add(7) as u32) << 24);
        data |= ((((hi as i32) as isize) as usize) << 16) << 16;

        v3 ^= data;
        let mut j: usize = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len.wrapping_sub(i);
    if rem <= 7 {
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
            data |= (((*d.add(3) as i32).wrapping_shl(24)) as isize) as usize;
        }
        if rem >= 3 {
            data |= (((*d.add(2) as i32) << 16) as isize) as usize;
        }
        if rem >= 2 {
            data |= (((*d.add(1) as i32) << 8) as isize) as usize;
        }
        if rem >= 1 {
            data |= ((*d.add(0) as i32) as isize) as usize;
        }
    }

    v3 ^= data;
    let mut j: usize = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    let mut j: usize = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
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
        let other = *(elem_ptr(a, elemsize, i, keyoffset) as *mut *mut c_char);
        c_str_eq(key as *const c_char, other) as c_int
    } else {
        c_mem_eq(
            key as *const u8,
            elem_ptr(a, elemsize, i, keyoffset) as *const u8,
            keysize,
        ) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                stbds_free(*(elem_ptr(a, elemsize, i, 0) as *mut *mut c_void));
                i += 1;
            }
        }
        stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
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
    let mut hash: usize = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: usize = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket: *mut stbds_hash_bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
        let mut i: usize = 0;
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
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        c_memzero(a as *mut u8, elemsize);
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
    set_stbds_temp(stbds_hash_to_arr(p, elemsize), temp);
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
        c_memzero(a as *mut u8, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c_strlen(str_) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    c_memmove(p as *mut u8, str_ as *const u8, len);
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
    #[allow(unused_assignments)]
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        c_memzero(a as *mut u8, elemsize);
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
            stbds_free(table as *mut c_void);
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

        'search: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
                        set_stbds_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let existing = *(elem_ptr(
                                raw_a,
                                elemsize,
                                (*bucket).index[i] as usize,
                                keyoffset,
                            ) as *mut *mut c_char);
                            set_stbds_temp_key(a, existing);
                        }
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'search;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                }
                i += 1;
            }

            let limit: usize = pos & STBDS_BUCKET_MASK;
            let mut i: usize = 0;
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
                        set_stbds_temp(a, (*bucket).index[i]);
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'search;
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
            let _ = raw_a;

            stbds_assert((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = i.wrapping_add(1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i.wrapping_sub(1);
            set_stbds_temp(a, i.wrapping_sub(1));

            let slot = elem_ptr(a, elemsize, i as usize, 0) as *mut *mut c_char;
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    *slot = v;
                    set_stbds_temp_key(a, v);
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                    *slot = v;
                    set_stbds_temp_key(a, v);
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    *slot = v;
                    set_stbds_temp_key(a, v);
                }
                _ => {
                    c_memcpy(
                        elem_ptr(a, elemsize, i as usize, 0),
                        key as *const u8,
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
    c_memzero(a as *mut u8, elemsize);
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
        set_stbds_temp(raw_a, 0);
        if table.is_null() {
            a
        } else {
            let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b: *mut stbds_hash_bucket =
                    (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1).wrapping_sub(1);
                stbds_assert(slot < (*table).slot_count as isize);
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                set_stbds_temp(raw_a, 1);
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                    stbds_free(*(elem_ptr(a, elemsize, old_index as usize, 0) as *mut *mut c_void));
                }

                if old_index != final_index {
                    c_memmove(
                        elem_ptr(a, elemsize, old_index as usize, 0),
                        elem_ptr(a, elemsize, final_index as usize, 0) as *const u8,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            *(elem_ptr(a, elemsize, old_index as usize, keyoffset)
                                as *mut *mut c_char) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            elem_ptr(a, elemsize, old_index as usize, keyoffset) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    stbds_assert(slot >= 0);
                    b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
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
// String arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// `sizeof(stbds_string_block) - 8` = size of the block header without the
/// inline 8-byte storage stub.
const STRING_BLOCK_HDR: usize = core::mem::size_of::<stbds_string_block>() - 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len: usize = c_strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb: *mut stbds_string_block =
                stbds_realloc(ptr::null_mut(), STRING_BLOCK_HDR.wrapping_add(len))
                    as *mut stbds_string_block;
            c_memmove(
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
            let sb: *mut stbds_string_block =
                stbds_realloc(ptr::null_mut(), STRING_BLOCK_HDR.wrapping_add(blocksize))
                    as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    stbds_assert(len <= (*a).remaining);
    p = ((&raw mut (*(*a).storage).storage) as *mut u8)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len) as *mut c_char;
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    c_memmove(p as *mut u8, str_ as *const u8, len);
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
    c_memzero(a as *mut u8, core::mem::size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// Test helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

/// `static char buffer[256];`
static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    // sprintf(buffer, "test_%d", n);
    let buf = (&raw mut BUFFER) as *mut u8;
    let mut tmp: [u8; 32] = [0; 32];
    let mut len: usize = 0;
    // "test_"
    for (i, c) in b"test_".iter().enumerate() {
        *buf.add(i) = *c;
        len += 1;
        let _ = i;
    }
    // decimal representation of the int, matching printf("%d")
    let mut val: i64 = n as i64;
    if val < 0 {
        *buf.add(len) = b'-';
        len += 1;
        val = -val;
    }
    let mut ndigits: usize = 0;
    if val == 0 {
        tmp[0] = b'0';
        ndigits = 1;
    } else {
        while val > 0 {
            tmp[ndigits] = b'0' + (val % 10) as u8;
            val /= 10;
            ndigits += 1;
        }
    }
    let mut k: usize = ndigits;
    while k > 0 {
        k -= 1;
        *buf.add(len) = tmp[k];
        len += 1;
    }
    *buf.add(len) = 0;

    buf as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    // int *arr = NULL;  (elemsize == sizeof(int) == 4)
    const ELEMSIZE: usize = core::mem::size_of::<c_int>();
    let mut arr: *mut c_void = ptr::null_mut();

    stbds_assert(stbds_arrlen(arr) == 0);
    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            // arrpush(arr, j)  ->  stbds_arrmaybegrow(arr,1),
            //                      arr[stbds_header(arr)->length++] = j
            if arr.is_null()
                || (*stbds_header(arr)).length.wrapping_add(1) > (*stbds_header(arr)).capacity
            {
                arr = stbds_arrgrowf(arr, ELEMSIZE, 1, 0);
            }
            let idx = (*stbds_header(arr)).length;
            (*stbds_header(arr)).length = idx.wrapping_add(1);
            *(arr as *mut c_int).add(idx) = j;
            j += 1;
        }
        // arrfree(arr)
        if !arr.is_null() {
            stbds_free(stbds_header(arr) as *mut c_void);
        }
        arr = ptr::null_mut();
        i = i.wrapping_add(50);
    }
    let _ = STBDS_SH_NONE;
    let _ = STBDS_HM_BINARY;
}
