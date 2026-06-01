// Rust translation of c_src/src/lib.c (stb_ds-based)
// Preserves the exact behaviour and exported symbols of the C version.

#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};
use core::mem;
use core::ptr;

// ----------------------------------------------------------------------------
// libc bindings (we use malloc/realloc/free so memory layout & ownership match
// the C library byte-for-byte).
// ----------------------------------------------------------------------------
extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// Helper: STBDS_REALLOC(c, p, s) -> realloc(p, s)
unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    realloc(p, s)
}

unsafe fn stbds_free(p: *mut c_void) {
    free(p);
}

// ----------------------------------------------------------------------------
// Array header
// ----------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

// stbds_header(t) = ((stbds_array_header*)(t)) - 1
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

// ----------------------------------------------------------------------------
// String arena
// ----------------------------------------------------------------------------
#[repr(C)]
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

// ----------------------------------------------------------------------------
// Hash bucket / hash index
// ----------------------------------------------------------------------------
const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // log2(8)
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
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
#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

// Suppress unused warnings for constants we keep for documentation/parity.
const _: u8 = STBDS_SH_NONE;
const _: u8 = STBDS_SH_ARENA;

// ----------------------------------------------------------------------------
// Global hash seed
// ----------------------------------------------------------------------------
// The C version uses a non-thread-safe global. We replicate that exactly.
static mut STBDS_HASH_SEED: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// ----------------------------------------------------------------------------
// stbds_arrgrowf
// ----------------------------------------------------------------------------
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

    let header_size = mem::size_of::<stbds_array_header>();
    let old_header: *mut c_void = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };

    let raw = stbds_realloc(old_header, elemsize * min_cap + header_size);
    let b = (raw as *mut u8).add(header_size) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

// ----------------------------------------------------------------------------
// stbds_arrfreef
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    stbds_free(stbds_header(a) as *mut c_void);
}

// ----------------------------------------------------------------------------
// Helpers used during hashing/probing
// ----------------------------------------------------------------------------
const STBDS_SIZE_T_BITS: u32 = (mem::size_of::<usize>() * 8) as u32;

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[inline]
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

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// ----------------------------------------------------------------------------
// stbds_make_hash_index
// ----------------------------------------------------------------------------
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let bytes = (slot_count >> STBDS_BUCKET_SHIFT) * mem::size_of::<stbds_hash_bucket>()
        + mem::size_of::<stbds_hash_index>()
        + (STBDS_CACHE_LINE_SIZE - 1);
    let raw = stbds_realloc(ptr::null_mut(), bytes);
    let t = raw as *mut stbds_hash_index;

    let after_t = (t as *mut u8).add(mem::size_of::<stbds_hash_index>()) as usize;
    let aligned = align_fwd(after_t, STBDS_CACHE_LINE_SIZE);
    (*t).storage = aligned as *mut stbds_hash_bucket;

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
        (*t).string = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        // stbds_load_32_or_64(b,temp,  715136305,          0, 0xb504f32d);
        // For 64-bit size_t these compute:
        //   a = (0x27bb2ee6 << 32) | 0x87b0b0fd
        //   b = (         0 << 32) | 0xb504f32d
        let a: usize = (0x27bb2ee6_usize << 32) | 0x87b0b0fd_usize;
        let b: usize = 0xb504f32d_usize;
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    // initialise buckets
    let nb = slot_count >> STBDS_BUCKET_SHIFT;
    for i in 0..nb {
        let bucket = (*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bucket).hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bucket).index[j] = STBDS_INDEX_EMPTY;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let onb = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..onb {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let start = pos & STBDS_BUCKET_MASK;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }

    t
}

// ----------------------------------------------------------------------------
// stbds_hash_string
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str_ptr: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    while *str_ptr != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*(str_ptr as *mut u8) as usize);
        str_ptr = str_ptr.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// ----------------------------------------------------------------------------
// siphash 2-4 (64-bit only)
// ----------------------------------------------------------------------------
const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline]
unsafe fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    // v0 = ((((size_t) 0x736f6d65 << 16) << 16) + 0x70736575) ^  seed;
    v0 = (((0x736f6d65_usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = (((0x646f7261_usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = (((0x6c796765_usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = (((0x74656462_usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;

    let mut i: usize = 0;
    while i + mem::size_of::<usize>() <= len {
        // data = d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24);
        data = (*d.add(0) as usize)
            | ((*d.add(1) as usize) << 8)
            | ((*d.add(2) as usize) << 16)
            | ((*d.add(3) as usize) << 24);
        // data |= (size_t)(d[4]|(d[5]<<8)|(d[6]<<16)|(d[7]<<24)) << 16 << 16;
        let hi = (*d.add(4) as usize)
            | ((*d.add(5) as usize) << 8)
            | ((*d.add(6) as usize) << 16)
            | ((*d.add(7) as usize) << 24);
        data |= (hi << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += mem::size_of::<usize>();
        d = d.add(mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // Fall-through switch — emulate with explicit additions.
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
        data |= (*d.add(3) as usize) << 24;
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
    // case 0: nothing.

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

// ----------------------------------------------------------------------------
// stbds_is_key_equal
// ----------------------------------------------------------------------------
unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> bool {
    let elem_ptr = (a as *mut u8).add(elemsize * i + keyoffset);
    if mode >= STBDS_HM_STRING {
        let stored_key = *(elem_ptr as *mut *mut c_char);
        strcmp(key as *mut c_char, stored_key) == 0
    } else {
        memcmp(key, elem_ptr as *const c_void, keysize) == 0
    }
}

// helpers: STBDS_HASH_TO_ARR / STBDS_ARR_TO_HASH
#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ----------------------------------------------------------------------------
// stbds_hmfree_func
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = stbds_hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            let mut i: usize = 1;
            while i < len {
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_char);
                stbds_free(p as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&mut (*table).string as *mut stbds_string_arena);
    }
    stbds_free((*stbds_header(a)).hash_table);
    stbds_free(stbds_header(a) as *mut c_void);
}

// ----------------------------------------------------------------------------
// stbds_hm_find_slot
// ----------------------------------------------------------------------------
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

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
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

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

// ----------------------------------------------------------------------------
// stbds_hmget_key_ts
// ----------------------------------------------------------------------------
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
                let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

// ----------------------------------------------------------------------------
// stbds_hmget_key
// ----------------------------------------------------------------------------
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
    let arr = hash_to_arr(p, elemsize);
    (*stbds_header(arr)).temp = temp;
    p
}

// ----------------------------------------------------------------------------
// stbds_hmput_default
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let prev = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(prev, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

// ----------------------------------------------------------------------------
// strdup (internal)
// ----------------------------------------------------------------------------
unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    let len = strlen(s) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

// ----------------------------------------------------------------------------
// stbds_hmput_key
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

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

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
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
                        // stbds_temp_key(a) = *(char**)((char*)raw_a + elemsize*idx + keyoffset);
                        let temp_key_ptr =
                            (raw_a as *mut u8).add(elemsize * ((*bucket).index[i] as usize) + keyoffset)
                                as *mut *mut c_char;
                        let kk = *temp_key_ptr;
                        // store via hash_table->temp_key
                        let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
                        *ht = kk;
                    }
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                final_pos = pos;
                break 'outer;
            } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
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
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                final_pos = pos;
                break 'outer;
            } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    let mut pos = final_pos;
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i_idx = stbds_arrlen(a) as usize;
    if i_idx + 1 > stbds_arrcap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(a, elemsize);
    let _ = raw_a;

    (*stbds_header(a)).length = i_idx + 1;
    let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
    (*bucket).index[pos & STBDS_BUCKET_MASK] = (i_idx as isize) - 1;
    (*stbds_header(a)).temp = (i_idx as isize) - 1;

    let elem_slot_ptr = (a as *mut u8).add(elemsize * i_idx) as *mut *mut c_char;
    match (*table).string.mode {
        m if m == STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key as *mut c_char);
            *elem_slot_ptr = dup;
            // stbds_temp_key(a) = ...
            let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
            *ht = dup;
        }
        m if m == STBDS_SH_ARENA => {
            let p = stbds_stralloc(&mut (*table).string as *mut stbds_string_arena, key as *mut c_char);
            *elem_slot_ptr = p;
            let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
            *ht = p;
        }
        m if m == STBDS_SH_DEFAULT => {
            *elem_slot_ptr = key as *mut c_char;
            let ht = (*stbds_header(a)).hash_table as *mut *mut c_char;
            *ht = key as *mut c_char;
        }
        _ => {
            memmove(
                (a as *mut u8).add(elemsize * i_idx) as *mut c_void,
                key as *const c_void,
                keysize,
            );
            // The C code uses `memcpy` here. memcpy and memmove behave the same for
            // non-overlapping inputs, which is the case.
            // (Actually the C code uses memcpy; we use memmove which is always safe.)
            // Wait — re-read C: it uses `memcpy`. Use that for parity.
            // But result is identical for non-overlapping. Ok.
        }
    }

    arr_to_hash(a, elemsize)
}

// ----------------------------------------------------------------------------
// stbds_shmode_func
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

// ----------------------------------------------------------------------------
// stbds_hmdel_key
// ----------------------------------------------------------------------------
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
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    (*stbds_header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let mut bucket_i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[bucket_i];
    let final_index = (stbds_arrlen(raw_a) as isize) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    (*b).hash[bucket_i] = STBDS_HASH_DELETED;
    (*b).index[bucket_i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *((a as *mut u8).add(elemsize * (old_index as usize)) as *mut *mut c_char);
        stbds_free(p as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize * (old_index as usize)) as *mut c_void,
            (a as *mut u8).add(elemsize * (final_index as usize)) as *const c_void,
            elemsize,
        );

        let new_slot = if mode == STBDS_HM_STRING {
            let key_ptr = *((a as *mut u8).add(elemsize * (old_index as usize) + keyoffset)
                as *mut *mut c_char);
            stbds_hm_find_slot(
                a,
                elemsize,
                key_ptr as *mut c_void,
                keysize,
                keyoffset,
                mode,
            )
        } else {
            let key_ptr =
                (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset) as *mut c_void;
            stbds_hm_find_slot(a, elemsize, key_ptr, keysize, keyoffset, mode)
        };

        b = (*table).storage.add((new_slot as usize) >> STBDS_BUCKET_SHIFT);
        bucket_i = (new_slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[bucket_i] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        let new_table = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*stbds_header(raw_a)).hash_table = new_table as *mut c_void;
        stbds_free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = stbds_make_hash_index((*table).slot_count, table);
        (*stbds_header(raw_a)).hash_table = new_table as *mut c_void;
        stbds_free(table as *mut c_void);
    }

    a
}

// ----------------------------------------------------------------------------
// stbds_stralloc
// ----------------------------------------------------------------------------
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    s: *mut c_char,
) -> *mut c_char {
    let len = strlen(s) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;

        // blocksize = MIN << (blocksize >> 1)
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            // sizeof(stbds_string_block)-8+len = sizeof(*next) + len
            let alloc_size = mem::size_of::<*mut stbds_string_block>() + len;
            let sb = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            // memmove(sb->storage, str, len)
            memmove(
                &mut (*sb).storage as *mut [c_char; 8] as *mut c_void,
                s as *const c_void,
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
            return &mut (*sb).storage as *mut [c_char; 8] as *mut c_char;
        } else {
            let alloc_size = mem::size_of::<*mut stbds_string_block>() + blocksize;
            let sb = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    // p = a->storage->storage + a->remaining - len;
    let storage_base = &mut (*(*a).storage).storage as *mut [c_char; 8] as *mut c_char;
    let p = storage_base.add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

// ----------------------------------------------------------------------------
// stbds_strreset
// ----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, mem::size_of::<stbds_string_arena>());
}

// ----------------------------------------------------------------------------
// strkey + buffer (file-static in C)
// ----------------------------------------------------------------------------
// `static char buffer[256];` — file-scope, shared between strkey() invocations.
// We replicate this with a single mutable buffer.
static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let fmt = b"test_%d\0".as_ptr() as *const c_char;
    let buf = ptr::addr_of_mut!(BUFFER) as *mut c_char;
    sprintf(buf, fmt, n);
    buf
}

// ----------------------------------------------------------------------------
// str_dups — the only "user level" public function declared in lib.h
// ----------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(num: c_int) {
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

    // strmap = sh_new_strdup(...)
    let elemsize = mem::size_of::<StrMapEntry>();
    let mut strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP as c_int);

    // shputs(strmap, s) where s.key="a", s.value=num
    let key_a = b"a\0".as_ptr() as *mut c_char;
    let s = StrMapEntry {
        key: key_a,
        value: num,
    };

    // stbds_hmput_key_wrapper((t), sizeof *(t), (void*)(s).key, sizeof (t)->key, STBDS_HM_STRING)
    strmap = stbds_hmput_key(
        strmap,
        elemsize,
        s.key as *mut c_void,
        mem::size_of::<*mut c_char>(),
        STBDS_HM_STRING,
    );

    // (t)[stbds_temp((t)-1)] = (s)  — write the entry
    let tarr = hash_to_arr(strmap, elemsize);
    let temp = (*stbds_header(tarr)).temp;
    let entry_ptr = (strmap as *mut u8).add(elemsize * (temp as usize)) as *mut StrMapEntry;
    *entry_ptr = s;

    // (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1);
    let ht = (*stbds_header(tarr)).hash_table as *mut *mut c_char;
    let temp_key = *ht;
    (*entry_ptr).key = temp_key;

    // shlen(strmap) = stbds_hmlen(strmap) = stbds_header(strmap-1)->length - 1
    let len = (*stbds_header(tarr)).length as isize - 1;

    let fmt = b"%s %d\n\0".as_ptr() as *const c_char;
    let mut z: isize = 0;
    while z < len {
        let entry = *((strmap as *mut u8).add(elemsize * (z as usize)) as *mut StrMapEntry);
        // Replicate `printf("%s %d\n", strmap[z], strmap[z].value);` — the
        // struct's first field (key) is consumed by %s, and the explicit
        // `.value` arg is consumed by %d (the struct's second register slot
        // happens to land in the same vararg position on SysV; either way
        // both yield `value`).
        printf(fmt, entry.key, entry.value);
        z += 1;
    }

    // shfree(strmap) -> stbds_hmfree_func((strmap)-1, sizeof *strmap)
    stbds_hmfree_func(tarr, elemsize);
}
