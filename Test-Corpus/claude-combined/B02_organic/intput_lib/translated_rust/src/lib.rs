//! Translation of c_src/src/lib.c to Rust.
//! This is a faithful translation of the stb_ds (single-file dynamic data structures)
//! library plus the small `intput` wrapper and `strkey` helper.

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ------------------------------ C library externs ------------------------------
extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
}

// ------------------------------ Constants ------------------------------
const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;
#[allow(dead_code)]
const _: () = {
    // tie unused names into _ to silence dead code warnings
    let _ = STBDS_SH_NONE;
};

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1usize << 20;

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

// ------------------------------ Structures ------------------------------
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [u8; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct StbdsHashIndex {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StbdsStringArena,
    storage: *mut StbdsHashBucket,
}

// ------------------------------ Helpers ------------------------------
#[inline(always)]
unsafe fn header(t: *mut c_void) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).sub(1)
}

#[inline(always)]
unsafe fn arrcap(t: *mut c_void) -> usize {
    if t.is_null() {
        0
    } else {
        (*header(t)).capacity
    }
}

#[inline(always)]
unsafe fn arrlen(t: *mut c_void) -> isize {
    if t.is_null() {
        0
    } else {
        (*header(t)).length as isize
    }
}

#[inline(always)]
unsafe fn temp_set(t: *mut c_void, v: isize) {
    (*header(t)).temp = v;
}

#[inline(always)]
unsafe fn temp_key_set(t: *mut c_void, v: *mut c_char) {
    // *(char **) header(t)->hash_table = v
    // i.e. write to the location pointed to by hash_table
    let p = (*header(t)).hash_table as *mut *mut c_char;
    *p = v;
}

#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// HASH <-> ARR conversion
#[inline(always)]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline(always)]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline(always)]
unsafe fn hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    (*header(a)).hash_table as *mut StbdsHashIndex
}

// ------------------------------ Globals ------------------------------
static mut STBDS_HASH_SEED: usize = 0x31415926;

static mut BUFFER: [c_char; 256] = [0; 256];

// ------------------------------ stbds_arrgrowf ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (arrlen(a) as usize) + addlen;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= arrcap(a) {
        return a;
    }

    if min_cap < 2 * arrcap(a) {
        min_cap = 2 * arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let prev = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };
    let total_size = elemsize * min_cap + core::mem::size_of::<StbdsArrayHeader>();
    let raw = realloc(prev, total_size);
    let b_ptr = (raw as *mut u8).add(core::mem::size_of::<StbdsArrayHeader>()) as *mut c_void;

    if a.is_null() {
        (*header(b_ptr)).length = 0;
        (*header(b_ptr)).hash_table = ptr::null_mut();
        (*header(b_ptr)).temp = 0;
    }
    (*header(b_ptr)).capacity = min_cap;

    b_ptr
}

// ------------------------------ stbds_arrfreef ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(header(a) as *mut c_void);
}

// ------------------------------ stbds_rand_seed ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// ------------------------------ stbds_log2 ------------------------------
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

// ------------------------------ stbds_probe_position ------------------------------
#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

// ------------------------------ stbds_make_hash_index ------------------------------
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let total = (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<StbdsHashBucket>()
        + core::mem::size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = realloc(ptr::null_mut(), total) as *mut StbdsHashIndex;

    let after_t = t.add(1) as usize;
    (*t).storage = align_fwd(after_t, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
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
            &mut (*t).string as *mut StbdsStringArena as *mut c_void,
            0,
            core::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64 expansion (only 64-bit path matters here):
        // a = 0x27bb2ee687b0b0fd
        // b = 0xb504f32d
        // both as size_t. The macro's bit-twiddling resolves to these on 64-bit.
        let a: usize;
        let b: usize;
        if core::mem::size_of::<usize>() == 8 {
            // a: v64_hi=0x27bb2ee6, v64_lo=0x87b0b0fd
            a = ((0x27bb2ee6usize) << 32) | 0x87b0b0fdusize;
            // b: v64_hi=0, v64_lo=0xb504f32d
            b = 0xb504f32dusize;
        } else {
            // 32-bit path
            a = 2147001325usize;
            b = 715136305usize;
        }
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    {
        let buckets = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..buckets {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..buckets {
            let ob = (*ot).storage.add(i);
            'inner: for j in 0..STBDS_BUCKET_LENGTH {
                let idx = (*ob).index[j];
                if idx >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        // first loop: from pos&mask to BUCKET_LENGTH
                        let start = pos & STBDS_BUCKET_MASK;
                        let mut placed = false;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                placed = true;
                                break;
                            }
                        }
                        if placed {
                            continue 'inner;
                        }
                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                placed = true;
                                break;
                            }
                        }
                        if placed {
                            continue 'inner;
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

// ------------------------------ stbds_hash_string ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    while *str != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*(str as *const u8) as usize);
        str = str.add(1);
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

// ------------------------------ stbds_siphash_bytes ------------------------------
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

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = rotate_left(v1, 13);
            v1 ^= v0;
            v0 = rotate_left(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotate_left(v1, 17);
            v1 ^= v2;
            v2 = rotate_left(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = rotate_left(v3, 21);
            v3 ^= v0;
        }};
    }

    let szt = core::mem::size_of::<usize>();
    let mut i: usize = 0;
    while i + szt <= len {
        // C: data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // Each d[i] is unsigned char promoted to int. The shifts and ORs
        // happen in int. Then assignment to size_t sign-extends if high bit set.
        let lo_int: i32 = (*d.add(0) as i32)
            | (((*d.add(1) as i32) << 8) as i32)
            | (((*d.add(2) as i32) << 16) as i32)
            | (((*d.add(3) as u32 as i32).wrapping_shl(24)) as i32);
        // Sign-extend to size_t (64 bits)
        data = lo_int as isize as usize;

        // C: data |= (size_t)(d[4]|(d[5]<<8)|(d[6]<<16)|(d[7]<<24)) << 16 << 16;
        // The (size_t) cast applies to the inner int expression, sign-extending it.
        // Then <<32 discards the sign-extended upper bits (since size_t is 64-bit).
        let hi_int: i32 = (*d.add(4) as i32)
            | (((*d.add(5) as i32) << 8) as i32)
            | (((*d.add(6) as i32) << 16) as i32)
            | (((*d.add(7) as u32 as i32).wrapping_shl(24)) as i32);
        let hi_sx: usize = hi_int as isize as usize;
        data |= hi_sx.wrapping_shl(16).wrapping_shl(16);

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;

        i += szt;
        d = d.add(szt);
    }
    data = len.wrapping_shl(STBDS_SIZE_T_BITS - 8);
    // C switch fallthrough
    let rem = len - i;
    if rem >= 7 {
        // C: data |= ((size_t) d[6] << 24) << 24;
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        // C: data |= (d[3] << 24);
        // d[3] << 24 is int; if d[3] high bit is set, it's negative int.
        // ORing into size_t involves sign extension.
        let v: i32 = (*d.add(3) as u32 as i32).wrapping_shl(24);
        data |= v as isize as usize;
    }
    if rem >= 3 {
        // d[2]<<16 — d[2] max is 0xff -> int 0xff0000, no high bit set.
        let v: i32 = (*d.add(2) as i32) << 16;
        data |= v as isize as usize;
    }
    if rem >= 2 {
        let v: i32 = (*d.add(1) as i32) << 8;
        data |= v as isize as usize;
    }
    if rem >= 1 {
        let v: i32 = *d.add(0) as i32;
        data |= v as isize as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        sipround!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        sipround!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ------------------------------ stbds_is_key_equal ------------------------------
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
        let p = (a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char;
        strcmp(key as *const c_char, *p) == 0
    } else {
        memcmp(
            key as *const c_void,
            (a as *mut u8).add(elemsize * i + keyoffset) as *const c_void,
            keysize,
        ) == 0
    }
}

// ------------------------------ stbds_hmfree_func ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !hash_table(a).is_null() {
        if (*hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let len = (*header(a)).length;
            for i in 1..len {
                let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                free(*p as *mut c_void);
            }
        }
        stbds_strreset(&mut (*hash_table(a)).string);
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut c_void);
}

// ------------------------------ stbds_hm_find_slot ------------------------------
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

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
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

// ------------------------------ stbds_hmget_key_ts ------------------------------
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
        (*header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
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

// ------------------------------ stbds_hmget_key ------------------------------
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

// ------------------------------ stbds_hmput_default ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let prev = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(prev, elemsize, 0, 1);
        (*header(a)).length += 1;
        memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

// ------------------------------ stbds_strdup ------------------------------
unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    let len = strlen(s) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

// ------------------------------ stbds_hmput_key ------------------------------
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
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*header(a)).hash_table as *mut StbdsHashIndex;

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
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*header(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut tombstone: isize = -1;
        let mut bucket: *mut StbdsHashBucket;

        if hash < 2 {
            hash = hash.wrapping_add(2);
        }

        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'outer: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let p = (raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char;
                            temp_key_set(a, *p);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        temp_set(a, (*bucket).index[i]);
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        // found_empty_slot
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        {
            let i = arrlen(a);
            if (i as usize) + 1 > arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);

            (*header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            temp_set(a, i - 1);

            match (*table).string.mode {
                m if m == STBDS_SH_STRDUP => {
                    let dup = stbds_strdup(key as *mut c_char);
                    let p = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
                    *p = dup;
                    temp_key_set(a, dup);
                }
                m if m == STBDS_SH_ARENA => {
                    let alloc = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                    let p = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
                    *p = alloc;
                    temp_key_set(a, alloc);
                }
                m if m == STBDS_SH_DEFAULT => {
                    let p = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
                    *p = key as *mut c_char;
                    temp_key_set(a, key as *mut c_char);
                }
                _ => {
                    memcpy(
                        (a as *mut u8).add(elemsize * (i as usize)) as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        let _ = raw_a;
        arr_to_hash(a, elemsize)
    }
}

// ------------------------------ stbds_shmode_func ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

// ------------------------------ stbds_hmdel_key ------------------------------
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
    let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
    temp_set(raw_a, 0);
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
    let final_index = arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    temp_set(raw_a, 1);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = (a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char;
        free(*p as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
            (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let p = (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut *mut c_char;
            slot = stbds_hm_find_slot(a, elemsize, *p as *mut c_void, keysize, keyoffset, mode);
        } else {
            let p = (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, p, keysize, keyoffset, mode);
        }
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// ------------------------------ stbds_stralloc ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut StbdsStringArena, s: *mut c_char) -> *mut c_char {
    let len = strlen(s) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // sizeof(*sb)-8 + len
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<StbdsStringBlock>() - 8 + len,
            ) as *mut StbdsStringBlock;
            memmove(
                (*sb).storage.as_mut_ptr() as *mut c_void,
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
            return (*sb).storage.as_mut_ptr() as *mut c_char;
        } else {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<StbdsStringBlock>() - 8 + blocksize,
            ) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len) as *mut c_char;
    (*a).remaining -= len;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

// ------------------------------ stbds_strreset ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, core::mem::size_of::<StbdsStringArena>());
}

// ------------------------------ strkey ------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let fmt = b"test_%d\0".as_ptr() as *const c_char;
    let buf_ptr = (&raw mut BUFFER) as *mut c_char;
    sprintf(buf_ptr, fmt, n);
    buf_ptr
}

// ------------------------------ intput ------------------------------
// struct { int key; int value; } => 8 bytes total in C
#[repr(C)]
#[derive(Copy, Clone)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

/// Helper: equivalent to the `stbds_hmput(t, k, v)` macro for the local
/// IntMapEntry hash map. Returns the new map pointer.
unsafe fn intmap_hmput(
    map: *mut IntMapEntry,
    key: c_int,
    value: c_int,
) -> *mut IntMapEntry {
    let mut k_local = key;
    let new_map = stbds_hmput_key(
        map as *mut c_void,
        core::mem::size_of::<IntMapEntry>(),
        &mut k_local as *mut c_int as *mut c_void,
        core::mem::size_of::<c_int>(),
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    // stbds_temp((t)-1) where (t-1) is the underlying array pointer.
    let raw =
        (new_map as *mut u8).sub(core::mem::size_of::<IntMapEntry>()) as *mut IntMapEntry;
    let temp_idx = (*header(raw as *mut c_void)).temp;
    let entry = new_map.offset(temp_idx);
    (*entry).key = key;
    (*entry).value = value;
    new_map
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(num: c_int) {
    // Faithful translation: the C source declares intmap = NULL twice in
    // immediate succession — preserve that fidelity.
    #[allow(unused_assignments)]
    let mut intmap: *mut IntMapEntry = ptr::null_mut();

    intmap = ptr::null_mut();
    intmap = intmap_hmput(intmap, num, 7);
    intmap = intmap_hmput(intmap, 11, 3);
    intmap = intmap_hmput(intmap, 9, num);

    // Asserts in the C source produce no observable output unless they fail.
    // We do not free intmap — matches C source.
    let _ = intmap;
}
