//! Rust translation of `c_src/src/lib.c` (an inlined copy of `stb_ds.h` plus the
//! `strkey` / `str_put` helpers).
//!
//! The translation is intentionally literal: memory layout, allocation sizes,
//! probing order, hash arithmetic (including the C integer-promotion /
//! sign-extension quirks in `stbds_siphash_bytes`) and the exact `printf` call
//! are reproduced so the shared library is byte-for-byte output compatible.

#![allow(non_camel_case_types)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free/printf directly)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    realloc(p, size)
}

#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    free(p)
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

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

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

// ---------------------------------------------------------------------------
// Data structures (layouts identical to the C originals)
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
#[derive(Clone, Copy)]
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

// ---------------------------------------------------------------------------
// Array header helpers (the stbds_header / stbds_arrlen / ... macros)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline]
unsafe fn stbds_temp_set(a: *mut c_void, v: isize) {
    (*stbds_header(a)).temp = v;
}

#[inline]
unsafe fn stbds_temp_get(a: *mut c_void) -> isize {
    (*stbds_header(a)).temp
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_set(a: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
    *ht = v;
}

#[inline]
unsafe fn stbds_temp_key_get(a: *mut c_void) -> *mut c_char {
    let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
    *ht
}

#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).offset(-(elemsize as isize)) as *mut c_void
}

#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ---------------------------------------------------------------------------
// Small C string / memory helpers
// ---------------------------------------------------------------------------

unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// `0 == strcmp(a, b)`
unsafe fn c_str_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut i = 0usize;
    loop {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        i += 1;
    }
}

/// `0 == memcmp(a, b, n)`
unsafe fn c_mem_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    for i in 0..n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
    }
    true
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
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

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

    let old = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };
    let mut b = stbds_realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),
    );
    b = (b as *mut u8).add(size_of::<stbds_array_header>()) as *mut c_void;
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
// Hash seed / probing helpers
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
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

// ---------------------------------------------------------------------------
// stbds_make_hash_index
// ---------------------------------------------------------------------------

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = stbds_realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
            + size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1,
    ) as *mut stbds_hash_index;

    (*t).storage =
        stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
    assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        (*t).string = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*t).seed = STBDS_HASH_SEED;
        let a = stbds_load_32_or_64(2147001325, 0x27bb_2ee6, 0x87b0_b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504_f32d);
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i = 0usize;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
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
        let mut i = 0usize;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
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
    let mut hash = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash = hash ^ (hash ^ stbds_rotate_right(hash, 31));
    hash = hash.wrapping_mul(21);
    hash = hash ^ (hash ^ stbds_rotate_right(hash, 11));
    hash = hash.wrapping_add(hash << 6);
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

macro_rules! sipround {
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

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut data: usize;

    let mut v0 = (((0x736f_6d65usize << 16) << 16).wrapping_add(0x7073_6575)) ^ seed;
    let mut v1 = (((0x646f_7261usize << 16) << 16).wrapping_add(0x6e64_6f6d)) ^ !seed;
    let mut v2 = (((0x6c79_6765usize << 16) << 16).wrapping_add(0x6e65_7261)) ^ seed;
    let mut v3 = (((0x7465_6462usize << 16) << 16).wrapping_add(0x7974_6573)) ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    // Note: the C code builds the low half of `data` from `int` arithmetic, so a
    // byte >= 0x80 at offset 3 sign-extends into the upper 32 bits. That quirk is
    // reproduced here on purpose.
    let mut i = 0usize;
    while i + size_of::<usize>() <= len {
        let lo = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32) << 24);
        data = lo as isize as usize;
        let hi = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | ((*d.add(7) as i32) << 24);
        data |= ((hi as isize as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;

        i += size_of::<usize>();
        d = d.add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
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
        data |= ((*d.add(3) as i32) << 24) as isize as usize;
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
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        sipround!(v0, v1, v2, v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        sipround!(v0, v1, v2, v3);
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
    i: isize,
) -> bool {
    let slot = (a as *mut u8).offset(elemsize.wrapping_mul(i as usize) as isize).add(keyoffset);
    if mode >= STBDS_HM_STRING {
        c_str_eq(key as *const c_char, *(slot as *mut *mut c_char) as *const c_char)
    } else {
        c_mem_eq(key as *const u8, slot as *const u8, keysize)
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
                stbds_free(*((a as *mut u8).add(elemsize * i) as *mut *mut c_char) as *mut c_void);
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
    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut hash = hash;
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
                    (*bucket).index[i],
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
                    (*bucket).index[i],
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
    let keyoffset = 0usize;
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
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
        ptr::write_bytes(a as *mut u8, 0, elemsize);
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
    let keyoffset = 0usize;
    let mut a = a;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = stbds_arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = stbds_hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

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
                STBDS_SH_DEFAULT
            } else {
                STBDS_SH_NONE
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = table as *mut c_void;
    }

    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;
    let mut bucket: *mut stbds_hash_bucket;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    'find: loop {
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
                ) {
                    stbds_temp_set(a, (*bucket).index[i]);
                    if mode >= STBDS_HM_STRING {
                        let src = (raw_a as *mut u8)
                            .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                            .add(keyoffset) as *mut *mut c_char;
                        stbds_temp_key_set(a, *src);
                    }
                    return stbds_arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'find;
            } else if tombstone < 0 {
                if (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            i += 1;
        }

        let limit = pos & STBDS_BUCKET_MASK;
        let mut i = 0usize;
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
                    (*bucket).index[i],
                ) {
                    stbds_temp_set(a, (*bucket).index[i]);
                    return stbds_arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                found_empty = true;
                break;
            } else if tombstone < 0 {
                if (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            i += 1;
        }
        if found_empty {
            break 'find;
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
        let i = stbds_arrlen(a);
        if (i as usize) + 1 > stbds_arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = stbds_arr_to_hash(a, elemsize);

        assert!((i as usize) + 1 <= stbds_arrcap(a));
        (*stbds_header(a)).length = (i + 1) as usize;
        bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        stbds_temp_set(a, i - 1);

        let key_slot = (a as *mut u8).add(elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
        match (*table).string.mode {
            STBDS_SH_STRDUP => {
                *key_slot = stbds_strdup(key as *mut c_char);
                stbds_temp_key_set(a, *key_slot);
            }
            STBDS_SH_ARENA => {
                *key_slot = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                stbds_temp_key_set(a, *key_slot);
            }
            STBDS_SH_DEFAULT => {
                *key_slot = key as *mut c_char;
                stbds_temp_key_set(a, *key_slot);
            }
            _ => {
                ptr::copy_nonoverlapping(
                    key as *const u8,
                    (a as *mut u8).add(elemsize.wrapping_mul(i as usize)),
                    keysize,
                );
            }
        }
    }
    stbds_arr_to_hash(a, elemsize)
}

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
    stbds_temp_set(raw_a, 0);
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
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    stbds_temp_set(raw_a, 1);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        stbds_free(
            *((a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char)
                as *mut c_void,
        );
    }

    if old_index != final_index {
        ptr::copy(
            (a as *mut u8).add(elemsize.wrapping_mul(final_index as usize)),
            (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)),
            elemsize,
        );

        let moved_key = (a as *mut u8)
            .add(elemsize.wrapping_mul(old_index as usize))
            .add(keyoffset);
        slot = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(
                a,
                elemsize,
                *(moved_key as *mut *mut c_char) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            )
        } else {
            stbds_hm_find_slot(a, elemsize, moved_key as *mut c_void, keysize, keyoffset, mode)
        };
        assert!(slot >= 0);
        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        assert!((*b).index[i] == final_index);
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

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c_strlen(str_) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
    p
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
    let len = c_strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = stbds_realloc(
                ptr::null_mut(),
                size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            ptr::copy(
                str_ as *const u8,
                (*sb).storage.as_mut_ptr() as *mut u8,
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

    assert!(len <= (*a).remaining);
    p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8).add((*a).remaining - len) as *mut c_char;
    (*a).remaining -= len;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
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
    ptr::write_bytes(a as *mut u8, 0, size_of::<stbds_string_arena>());
}

// ---------------------------------------------------------------------------
// strkey / str_put
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

/// `char *strkey(int n)` — `sprintf(buffer, "test_%d", n)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(BUFFER) as *mut u8;
    let s = format_test_key(n);
    let bytes = s.as_bytes();
    ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
    *buf.add(bytes.len()) = 0;
    buf as *mut c_char
}

/// Renders `test_%d` the same way `sprintf` does for an `int`.
fn format_test_key(n: c_int) -> String {
    let mut s = String::with_capacity(20);
    s.push_str("test_");
    s.push_str(&n.to_string());
    s
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let elemsize = size_of::<StrMapEntry>();
    let keysize = size_of::<*mut c_char>();

    let mut strmap: *mut StrMapEntry = ptr::null_mut();
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i: c_int = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    {
        let s = StrMapEntry {
            key: b"a\0".as_ptr() as *mut c_char,
            value: num,
        };

        // shputs(strmap, s)
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            s.key as *mut c_void,
            keysize,
            STBDS_HM_STRING,
        ) as *mut StrMapEntry;
        let raw = strmap.offset(-1) as *mut c_void;
        *strmap.offset(stbds_temp_get(raw)) = s;
        (*strmap.offset(stbds_temp_get(raw))).key = stbds_temp_key_get(raw);

        assert!(*(*strmap.offset(0)).key == b'a' as c_char);
        assert!((*strmap.offset(0)).key == s.key);
        assert!((*strmap.offset(0)).value == s.value);

        // shlen(strmap)
        let len = (*stbds_header(raw)).length as isize - 1;
        let mut z: isize = 0;
        while z < len {
            // The C code passes the whole struct where `%s` is expected; under the
            // SysV AMD64 ABI that puts the key pointer in the first vararg slot and
            // the (padded) value in the second, which is what `%d` then reads.
            let e = *strmap.offset(z);
            printf(b"%s %d\n\0".as_ptr() as *const c_char, e.key, e.value);
            z += 1;
        }

        // shfree(strmap)
        if !strmap.is_null() {
            stbds_hmfree_func(strmap.offset(-1) as *mut c_void, elemsize);
        }
        strmap = ptr::null_mut();
    }
}
