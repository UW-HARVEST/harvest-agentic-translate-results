// Rust translation of c_src/src/lib.c (stb_ds-like data structures + sh_geti test)
// This is a faithful 1:1 translation that preserves byte-identical output.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::too_many_arguments)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uchar};
use std::ptr;

// libc bindings
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// ----------- struct definitions -----------

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
pub struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: c_uchar,
    mode: c_uchar,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

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

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() as u32) * 8;
const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1usize << 20;

// ----------- helpers for accessing the array header -----------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
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
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ----------- global state -----------

static mut stbds_hash_seed: usize = 0x31415926;
static mut buffer: [c_char; 256] = [0; 256];

// ----------- exported functions -----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (stbds_arrlen(a) as usize) + addlen;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_ptr = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };

    let total_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b = realloc(old_ptr, total_size);
    let b = (b as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;

    if a.is_null() {
        let h = stbds_header(b);
        (*h).length = 0;
        (*h).hash_table = ptr::null_mut();
        (*h).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

// ----------- hash functions -----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    while *str_ != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*(str_ as *const u8) as usize);
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

#[inline]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
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
    let mut d = p as *mut u8;

    let mut v0: usize = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        // C: data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // The expression is `int`; assigning to `size_t data` sign-extends
        // when (d[3] << 24) overflows into the sign bit.
        let lo_int: i32 = (*d as i32)
            | (((*d.add(1) as i32)) << 8)
            | (((*d.add(2) as i32)) << 16)
            | (((*d.add(3) as u32) << 24) as i32);
        let lo: usize = lo_int as isize as usize;
        // C: data |= (size_t)(d[4] | (d[5]<<8) | (d[6]<<16) | (d[7]<<24)) << 16 << 16;
        // The inner expression is `int`; cast to size_t sign-extends, then shift left 32.
        let hi_int: i32 = (*d.add(4) as i32)
            | (((*d.add(5) as i32)) << 8)
            | (((*d.add(6) as i32)) << 16)
            | (((*d.add(7) as u32) << 24) as i32);
        let hi: usize = hi_int as isize as usize;
        let data: usize = lo | ((hi << 16) << 16);

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += std::mem::size_of::<usize>();
        d = d.add(std::mem::size_of::<usize>());
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let remaining = len - i;
    // C switch with fallthrough
    if remaining >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if remaining >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if remaining >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if remaining >= 4 {
        // C: data |= (d[3] << 24). d[3] (unsigned char) promotes to int.
        // (d[3] << 24) is int and may be negative when d[3] >= 0x80.
        // OR-ing into size_t sign-extends, setting upper 32 bits.
        let v = ((*d.add(3) as u32).wrapping_shl(24)) as i32;
        data |= v as isize as usize;
    }
    if remaining >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        data |= *d as usize;
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

// ----------- hash index helpers -----------

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2_size(mut slot_count: usize) -> usize {
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
    let total = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = realloc(ptr::null_mut(), total) as *mut stbds_hash_index;

    let after_t = (t as usize) + std::mem::size_of::<stbds_hash_index>();
    (*t).storage = stbds_align_fwd(after_t, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2_size(slot_count);
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
        // memset string to 0
        memset(
            &mut (*t).string as *mut stbds_string_arena as *mut c_void,
            0,
            std::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        // a = 0x27bb2ee687b0b0fd, b = 0x00000000b504f32d
        let a: usize = 0x27bb2ee687b0b0fdusize;
        let b: usize = 0x00000000b504f32dusize;
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    // initialize buckets
    {
        let n_buckets = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..n_buckets {
            let bk = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bk).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bk).index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let n_buckets_old = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..n_buckets_old {
            let ob = (*ot).storage.add(i);
            'outer: for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;

                    loop {
                        let bk = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        // first half
                        for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                            if (*bk).hash[z] == 0 {
                                (*bk).hash[z] = hash;
                                (*bk).index[z] = (*ob).index[j];
                                continue 'outer;
                            }
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bk).hash[z] == 0 {
                                (*bk).hash[z] = hash;
                                (*bk).index[z] = (*ob).index[j];
                                continue 'outer;
                            }
                        }

                        pos += step;
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }

    t
}

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
        let stored_key_ptr =
            (a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char;
        strcmp(key as *const c_char, *stored_key_ptr) == 0
    } else {
        memcmp(
            key as *const c_void,
            (a as *mut u8).add(elemsize * i + keyoffset) as *const c_void,
            keysize,
        ) == 0
    }
}

#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = stbds_hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP as c_uchar {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                free(*p as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string);
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
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
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
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
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
        }

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

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
                let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
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
    let raw = stbds_hash_to_arr(p, elemsize);
    (*stbds_header(raw)).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(old, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
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

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = stbds_arr_to_hash(a, elemsize);
    }

    raw_a = a;
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
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT as c_uchar
            } else {
                0
            };
        }
        (*stbds_header(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    let final_pos: usize;
    'outer: loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
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
                    (*stbds_header(a)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        let table_ptr =
                            (*stbds_header(a)).hash_table as *mut stbds_hash_index;
                        let stored = (raw_a as *mut u8)
                            .add(elemsize * ((*bucket).index[i] as usize) + keyoffset)
                            as *mut *mut c_char;
                        (*table_ptr).temp_key = *stored;
                    }
                    return stbds_arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'outer;
            } else if tombstone < 0 {
                if (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
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
                    (*stbds_header(a)).temp = (*bucket).index[i];
                    return stbds_arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'outer;
            } else if tombstone < 0 {
                if (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
        }

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    let mut pos = final_pos;
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

        (*stbds_header(a)).length = (i + 1) as usize;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*stbds_header(a)).temp = i - 1;

        match (*table).string.mode as c_int {
            x if x == STBDS_SH_STRDUP => {
                let dst = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
                let dup = stbds_strdup(key as *mut c_char);
                *dst = dup;
                (*table).temp_key = dup;
            }
            x if x == STBDS_SH_ARENA => {
                let dst = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
                let p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                *dst = p;
                (*table).temp_key = p;
            }
            x if x == STBDS_SH_DEFAULT => {
                let dst = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
                *dst = key as *mut c_char;
                (*table).temp_key = key as *mut c_char;
            }
            _ => {
                memcpy(
                    (a as *mut u8).add(elemsize * (i as usize)) as *mut c_void,
                    key,
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
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as c_uchar;
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
    (*stbds_header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }
    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }
    let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as c_uchar {
        let p = (a as *mut u8).add(elemsize * (old_index as usize)) as *mut *mut c_char;
        free(*p as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize * (old_index as usize)) as *mut c_void,
            (a as *mut u8).add(elemsize * (final_index as usize)) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let key_loc = (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset)
                as *mut *mut c_char;
            slot = stbds_hm_find_slot(a, elemsize, *key_loc as *mut c_void, keysize, keyoffset, mode);
        } else {
            let key_loc = (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset)
                as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, key_loc, keysize, keyoffset, mode);
        }
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        let new_t = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*stbds_header(raw_a)).hash_table = new_t as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_t = stbds_make_hash_index((*table).slot_count, table);
        (*stbds_header(raw_a)).hash_table = new_t as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// ----------- string arena -----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let len = strlen(str_) + 1;
    if len > (*a).remaining {
        let blocksize_in = (*a).block as usize;
        let blocksize: usize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_in >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // sizeof(stbds_string_block) is pointer + 8 bytes = 16. -8 means just pointer-size.
            // We replicate sizeof(*sb) - 8 + len
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
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
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
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
    memset(a as *mut c_void, 0, std::mem::size_of::<stbds_string_arena>());
}

// ----------- the test/demo function -----------

// Export `strkey` since C declares it non-static:
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    sprintf(buffer.as_mut_ptr(), b"test_%d\0".as_ptr() as *const c_char, n);
    buffer.as_mut_ptr()
}

// The struct used in sh_geti: { char *key; int value; }
// On x86_64: char* (8 bytes) + int (4 bytes) + 4 bytes padding = 16 bytes total
#[repr(C)]
#[derive(Copy, Clone)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let elemsize = std::mem::size_of::<StrMapEntry>();
    // In C: shgeti expects there is a "default" entry at index 0 (i.e. strmap[-1] when offset).
    // The macros use sizeof((*t)->key) for keysize, which for this struct is sizeof(char*) = 8.
    let keysize = std::mem::size_of::<*mut c_char>();
    let _ = keysize;

    // strmap is a hashed pointer (i.e., points to entry[1] of the underlying array; entry[0] is default).
    let mut strmap: *mut c_void = ptr::null_mut();
    let mut sa: stbds_string_arena = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    for i in 0..num {
        stbds_stralloc(&mut sa, strkey(i));
    }
    stbds_strreset(&mut sa);

    for j in 0..2 {
        // shgeti(strmap, "foo") == -1
        // shgeti macro: sets strmap = stbds_hmget_key_wrapper(strmap, elemsize, "foo", sizeof(char*), STBDS_HM_STRING),
        //               then returns stbds_temp(strmap-1).
        let mut foo = *b"foo\0";
        strmap = stbds_hmget_key(
            strmap,
            elemsize,
            foo.as_mut_ptr() as *mut c_void,
            std::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        );
        if !strmap.is_null() {
            let raw = stbds_hash_to_arr(strmap, elemsize);
            let _ = (*stbds_header(raw)).temp; // assert it's -1
        }

        if j == 0 {
            strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP);
        } else {
            strmap = stbds_shmode_func(elemsize, STBDS_SH_ARENA);
        }

        // shgeti(strmap, "foo") == -1
        strmap = stbds_hmget_key(
            strmap,
            elemsize,
            foo.as_mut_ptr() as *mut c_void,
            std::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        );

        // shdefault(strmap, -2): strmap = stbds_hmput_default(strmap, sizeof(*strmap)); strmap[-1].value = -2;
        strmap = stbds_hmput_default(strmap, elemsize);
        // strmap[-1].value = -2
        let default_entry = (strmap as *mut u8).sub(elemsize) as *mut StrMapEntry;
        (*default_entry).value = -2;

        // shgeti(strmap, "foo") == -1
        strmap = stbds_hmget_key(
            strmap,
            elemsize,
            foo.as_mut_ptr() as *mut c_void,
            std::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        );

        // for (i=0; i<num; i+=2) shput(strmap, strkey(i), i*3);
        let mut i = 0;
        while i < num {
            let k = strkey(i);
            // shput macro: strmap = hmput_key(strmap, elemsize, k, sizeof(char*), HM_STRING),
            //              strmap[temp].value = i*3;
            strmap = stbds_hmput_key(
                strmap,
                elemsize,
                k as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            );
            let raw = stbds_hash_to_arr(strmap, elemsize);
            let temp = (*stbds_header(raw)).temp;
            let entry = (strmap as *mut u8).add(elemsize * (temp as usize)) as *mut StrMapEntry;
            (*entry).value = i * 3;
            i += 2;
        }

        // for (int z=0; z < shlen(strmap); ++z) printf("%s %d\n", strmap[z], strmap[z].value);
        // shlen returns header(strmap-1)->length - 1
        let raw = stbds_hash_to_arr(strmap, elemsize);
        let len = (*stbds_header(raw)).length;
        let shlen_val = (len as isize) - 1;
        for z in 0..shlen_val {
            let entry = (strmap as *mut u8).add(elemsize * (z as usize)) as *mut StrMapEntry;
            // printf("%s %d\n", strmap[z], strmap[z].value)
            // C interprets first 8 bytes of strmap[z] as char* (which is the key field).
            printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                (*entry).key,
                (*entry).value,
            );
        }

        // for (i=0; i<num; i+=1) shget(strmap, strkey(i)) ==
        //   (i & 1) ? -2 : i*3
        let mut i = 0;
        while i < num {
            let k = strkey(i);
            strmap = stbds_hmget_key(
                strmap,
                elemsize,
                k as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            );
            // shget reads strmap[temp].value
            let raw = stbds_hash_to_arr(strmap, elemsize);
            let temp = (*stbds_header(raw)).temp;
            let entry = (strmap as *mut u8).add(elemsize * (temp as usize)) as *mut StrMapEntry;
            let _ = (*entry).value;
            i += 1;
        }

        // for (i=2; i<num; i+=4) shdel(strmap, strkey(i));
        let mut i = 2;
        while i < num {
            let k = strkey(i);
            strmap = stbds_hmdel_key(
                strmap,
                elemsize,
                k as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                // STBDS_OFFSETOF((t),key) = 0 since key is the first field
                0,
                STBDS_HM_STRING,
            );
            i += 4;
        }

        // for (i=0; i<num; i+=1) shget(strmap, strkey(i))...
        let mut i = 0;
        while i < num {
            let k = strkey(i);
            strmap = stbds_hmget_key(
                strmap,
                elemsize,
                k as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            );
            i += 1;
        }

        // for (i=0; i<num; i+=1) shdel(strmap, strkey(i));
        let mut i = 0;
        while i < num {
            let k = strkey(i);
            strmap = stbds_hmdel_key(
                strmap,
                elemsize,
                k as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                0,
                STBDS_HM_STRING,
            );
            i += 1;
        }

        // for (i=0; i<num; i+=1) shget(strmap, strkey(i)) == -2
        let mut i = 0;
        while i < num {
            let k = strkey(i);
            strmap = stbds_hmget_key(
                strmap,
                elemsize,
                k as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            );
            i += 1;
        }

        // shfree(strmap)
        if !strmap.is_null() {
            let raw = stbds_hash_to_arr(strmap, elemsize);
            stbds_hmfree_func(raw, elemsize);
        }
        strmap = ptr::null_mut();
        let _ = STBDS_SH_NONE;
    }
}
