//! Rust translation of the C library in `c_src/` (stb_ds.h implementation by
//! Sean Barrett, plus the `strkey` / `arr_del` test helpers).
//!
//! The translation is deliberately literal: allocation strategy, integer
//! wrap-around, sign-extension quirks, evaluation order and error/branch order
//! are all reproduced exactly as the C does them (including behaviour that
//! would normally be considered a bug).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free/mem*/str* directly)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Types (must match the C layouts byte for byte)
// ---------------------------------------------------------------------------

#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct stbds_string_arena {
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

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

#[allow(dead_code)]
const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

// ---------------------------------------------------------------------------
// Pointer/`stbds_*` macro helpers
// ---------------------------------------------------------------------------

#[inline(always)]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    // ((stbds_array_header *) (t) - 1)
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `(char *) p + off` with wrap-around (offsets may be "negative" as usize).
#[inline(always)]
fn byte_off(p: *mut c_void, off: usize) -> *mut u8 {
    (p as *mut u8).wrapping_add(off)
}

#[inline(always)]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

#[inline(always)]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
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
unsafe fn stbds_temp_set(a: *mut c_void, v: isize) {
    (*stbds_header(a)).temp = v;
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline(always)]
unsafe fn stbds_temp_key_set(a: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
    *ht = v;
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
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

    let mut b = realloc(
        if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        },
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    );
    b = (b as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;

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
// Hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a).wrapping_sub(1)) & !(a.wrapping_sub(1))
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline(always)]
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
    let t: *mut stbds_hash_index = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
            + core::mem::size_of::<stbds_hash_index>()
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

    if !ot.is_null() {
        (*t).string = (*ot).string;
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
                if (*ob).index[j] >= 0 {
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
// Hash functions
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = hash
            .rotate_left(9)
            .wrapping_add(*s as usize);
        s = s.wrapping_add(1);
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

#[inline(always)]
fn stbds_sipround(v: &mut [usize; 4]) {
    let half = STBDS_SIZE_T_BITS / 2;
    v[0] = v[0].wrapping_add(v[1]);
    v[1] = v[1].rotate_left(13);
    v[1] ^= v[0];
    v[0] = v[0].rotate_left(half);

    v[2] = v[2].wrapping_add(v[3]);
    v[3] = v[3].rotate_left(16);
    v[3] ^= v[2];

    v[2] = v[2].wrapping_add(v[1]);
    v[1] = v[1].rotate_left(17);
    v[1] ^= v[2];
    v[2] = v[2].rotate_left(half);

    v[0] = v[0].wrapping_add(v[3]);
    v[3] = v[3].rotate_left(21);
    v[3] ^= v[0];
}

/// Reproduces C's `d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24)` computed in
/// `int` and then converted to `size_t` — i.e. sign-extended when the top
/// byte has its high bit set.
#[inline(always)]
unsafe fn load_le32_sign_extended(d: *const u8) -> usize {
    let raw: u32 = (*d.wrapping_add(0) as u32)
        | ((*d.wrapping_add(1) as u32) << 8)
        | ((*d.wrapping_add(2) as u32) << 16)
        | ((*d.wrapping_add(3) as u32) << 24);
    (raw as i32) as isize as usize
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v: [usize; 4] = [0; 4];

    v[0] = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v[1] = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v[2] = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v[3] = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v[0] ^= 0x0706050403020100usize ^ seed;
    v[1] ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v[2] ^= 0x0706050403020100usize ^ seed;
    v[3] ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let sz = core::mem::size_of::<usize>();
    let mut i: usize = 0;
    let mut data: usize;
    while i + sz <= len {
        data = load_le32_sign_extended(d);
        data |= (load_le32_sign_extended(d.wrapping_add(4)) << 16) << 16;

        v[3] ^= data;
        let mut j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround(&mut v);
            j += 1;
        }
        v[0] ^= data;

        i += sz;
        d = d.wrapping_add(sz);
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // Fall-through `switch` from case 7 down to case 1.
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
        // `d[3] << 24` is an `int` in C: sign-extends into size_t.
        data |= ((((*d.wrapping_add(3) as u32) << 24) as i32) as isize) as usize;
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

    v[3] ^= data;
    let mut j = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        stbds_sipround(&mut v);
        j += 1;
    }
    v[0] ^= data;
    v[2] ^= 0xff;
    let mut j = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        stbds_sipround(&mut v);
        j += 1;
    }

    v[0] ^ v[1] ^ v[2] ^ v[3]
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
        let slot = byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *mut *mut c_char;
        (0 == strcmp(key as *const c_char, *slot)) as c_int
    } else {
        let slot = byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *const c_void;
        (0 == memcmp(key as *const c_void, slot, keysize)) as c_int
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
                let p = byte_off(a, elemsize.wrapping_mul(i)) as *mut *mut c_char;
                free(*p as *mut c_void);
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
                    .wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
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
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let mut b = stbds_arrgrowf(
            if !a.is_null() {
                stbds_hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            },
            elemsize,
            0,
            1,
        );
        (*stbds_header(b)).length += 1;
        memset(b, 0, elemsize);
        b = stbds_arr_to_hash(b, elemsize);
        return b;
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
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
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
                STBDS_SH_DEFAULT
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
        hash += 2;
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
                        let kp = byte_off(
                            raw_a,
                            elemsize
                                .wrapping_mul((*bucket).index[i] as usize)
                                .wrapping_add(keyoffset),
                        ) as *mut *mut c_char;
                        stbds_temp_key_set(a, *kp);
                    }
                    return stbds_arr_to_hash(a, elemsize);
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
                    (*bucket).index[i] as usize,
                ) != 0
                {
                    stbds_temp_set(a, (*bucket).index[i]);
                    return stbds_arr_to_hash(a, elemsize);
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
        raw_a = stbds_arr_to_hash(a, elemsize);

        (*stbds_header(a)).length = (i + 1) as usize;
        bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        stbds_temp_set(a, i - 1);

        let dest = byte_off(a, elemsize.wrapping_mul(i as usize));
        match (*table).string.mode {
            STBDS_SH_STRDUP => {
                let p = stbds_strdup(key as *mut c_char);
                *(dest as *mut *mut c_char) = p;
                stbds_temp_key_set(a, p);
            }
            STBDS_SH_ARENA => {
                let p = stbds_stralloc(
                    ptr::addr_of_mut!((*table).string),
                    key as *mut c_char,
                );
                *(dest as *mut *mut c_char) = p;
                stbds_temp_key_set(a, p);
            }
            STBDS_SH_DEFAULT => {
                let p = key as *mut c_char;
                *(dest as *mut *mut c_char) = p;
                stbds_temp_key_set(a, p);
            }
            _ => {
                memcpy(dest as *mut c_void, key as *const c_void, keysize);
            }
        }
    }

    let _ = raw_a;
    stbds_arr_to_hash(a, elemsize)
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
    stbds_temp_set(raw_a, 0);
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table)
        .storage
        .wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
    let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
    let old_index: isize = (*b).index[i as usize];
    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    stbds_temp_set(raw_a, 1);
    (*b).hash[i as usize] = STBDS_HASH_DELETED;
    (*b).index[i as usize] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = byte_off(a, elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char;
        free(*p as *mut c_void);
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
            slot = stbds_hm_find_slot(a, elemsize, *kp as *mut c_void, keysize, keyoffset, mode);
        } else {
            let kp = byte_off(
                a,
                elemsize
                    .wrapping_mul(old_index as usize)
                    .wrapping_add(keyoffset),
            ) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
        }
        b = (*table)
            .storage
            .wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
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

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: u32 = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: u32 = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(str_) + 1;
    if len > (*a).remaining {
        // `512u << (blocksize>>1)` is computed in `unsigned int` in C.
        let shift = ((*a).block as usize) >> 1;
        let blocksize: usize =
            STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl(shift as u32) as usize;

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX as usize {
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
// Test helpers exported by the C library
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(buffer) as *mut c_char;
    sprintf(buf, c"test_%d".as_ptr(), n);
    buf
}

/// `stbds_arrput(arr, v)` for `int *arr`.
#[inline(always)]
unsafe fn arrput_int(arr: &mut *mut c_int, v: c_int) {
    let elemsize = core::mem::size_of::<c_int>();
    // stbds_arrmaybegrow(a, 1)
    if (*arr).is_null()
        || (*stbds_header(*arr as *mut c_void)).length + 1
            > (*stbds_header(*arr as *mut c_void)).capacity
    {
        *arr = stbds_arrgrowf(*arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
    }
    let h = stbds_header(*arr as *mut c_void);
    *(*arr).wrapping_add((*h).length) = v;
    (*h).length += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    let elemsize = core::mem::size_of::<c_int>();
    let mut arr: *mut c_int = ptr::null_mut();

    let mut i: isize = 0;
    while i < 4 {
        arrput_int(&mut arr, num);
        arrput_int(&mut arr, 2);
        arrput_int(&mut arr, 3);
        arrput_int(&mut arr, 4);

        // stbds_arrdel(arr, i) == stbds_arrdeln(arr, i, 1)
        {
            let h = stbds_header(arr as *mut c_void);
            let n: usize = 1;
            memmove(
                arr.wrapping_offset(i) as *mut c_void,
                arr.wrapping_offset(i + n as isize) as *const c_void,
                elemsize.wrapping_mul((*h).length.wrapping_sub(n).wrapping_sub(i as usize)),
            );
            (*h).length -= n;
        }

        // stbds_arrfree(arr)
        if !arr.is_null() {
            free(stbds_header(arr as *mut c_void) as *mut c_void);
        }
        arr = ptr::null_mut();

        arrput_int(&mut arr, num);
        arrput_int(&mut arr, 2);
        arrput_int(&mut arr, 3);
        arrput_int(&mut arr, 4);

        // stbds_arrdelswap(arr, i)
        {
            let h = stbds_header(arr as *mut c_void);
            let last = *arr.wrapping_add((*h).length - 1);
            *arr.wrapping_offset(i) = last;
            (*h).length -= 1;
        }

        if !arr.is_null() {
            free(stbds_header(arr as *mut c_void) as *mut c_void);
        }
        arr = ptr::null_mut();

        i += 1;
    }
    let _ = arr;
}
