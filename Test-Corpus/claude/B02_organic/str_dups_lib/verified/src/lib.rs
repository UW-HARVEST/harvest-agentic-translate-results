// Translation of c_src/src/lib.c to Rust.
//
// We faithfully reproduce the stb_ds-style routines exposed by the C
// shared library so that the symbol set and runtime behaviour match
// byte-for-byte.

#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use std::sync::Mutex;

extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// ------------------------------------------------------------------
// Layout of the array header that prefixes every dynamic array.
// Must match struct { size_t length; size_t capacity; void *hash_table;
//                    ptrdiff_t temp; } in the C source.
// ------------------------------------------------------------------
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[inline]
unsafe fn header(a: *mut c_void) -> *mut StbdsArrayHeader {
    (a as *mut StbdsArrayHeader).offset(-1)
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as isize
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

// ------------------------------------------------------------------
// stbds_string_arena / stbds_string_block (must match C layout).
// ------------------------------------------------------------------
#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl Default for StbdsStringArena {
    fn default() -> Self {
        Self {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

// ------------------------------------------------------------------
// Hash table layout.
// ------------------------------------------------------------------
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

const STBDS_SH_NONE: c_int = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct StbdsHashIndex {
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

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
const fn rot_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
const fn rot_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

const STBDS_SIZE_T_BITS: usize = std::mem::size_of::<usize>() * 8;

// ------------------------------------------------------------------
// Global hash seed; matches the C `static size_t stbds_hash_seed`.
// ------------------------------------------------------------------
static HASH_SEED: Mutex<usize> = Mutex::new(0x31415926);

#[no_mangle]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    *HASH_SEED.lock().unwrap() = seed;
}

// ------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef
// ------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = arrlen(a) as usize + addlen;
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

    let old_ptr = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };

    let total = elemsize * min_cap + std::mem::size_of::<StbdsArrayHeader>();
    let raw = realloc(old_ptr, total);
    let b = (raw as *mut u8).add(std::mem::size_of::<StbdsArrayHeader>()) as *mut c_void;

    if a.is_null() {
        let h = header(b);
        (*h).length = 0;
        (*h).hash_table = ptr::null_mut();
        (*h).temp = 0;
    }
    (*header(b)).capacity = min_cap;

    b
}

#[no_mangle]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(header(a) as *mut c_void);
}

// ------------------------------------------------------------------
// Hash functions.
// ------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn stbds_hash_string(mut str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while *str_ != 0 {
        hash = rot_left(hash, 9).wrapping_add(*(str_ as *const u8) as usize);
        str_ = str_.add(1);
    }
    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rot_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rot_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rot_right(hash, 22);
    hash.wrapping_add(seed)
}

unsafe fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;

    let mut v0: usize =
        (((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize =
        (((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize =
        (((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize =
        (((0x74656462usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left((STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left((STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
            v3 ^= v0;
        };
    }

    let sz = std::mem::size_of::<usize>();
    let mut i = 0usize;
    while i + sz <= len {
        // Match C exactly: the inner OR expressions have type `int`,
        // so `dN << 24` can produce a negative int, which then
        // sign-extends when widened to `size_t`.
        let b0 = *d.add(0) as i32;
        let b1 = *d.add(1) as i32;
        let b2 = *d.add(2) as i32;
        let b3 = *d.add(3) as i32;
        let b4 = *d.add(4) as i32;
        let b5 = *d.add(5) as i32;
        let b6 = *d.add(6) as i32;
        let b7 = *d.add(7) as i32;
        let lo_int: i32 = b0
            | b1.wrapping_shl(8)
            | b2.wrapping_shl(16)
            | b3.wrapping_shl(24);
        let hi_int: i32 = b4
            | b5.wrapping_shl(8)
            | b6.wrapping_shl(16)
            | b7.wrapping_shl(24);
        // Sign-extend lo_int to size_t (matches C `data = (int_expr)` -> size_t).
        let mut data: usize = lo_int as isize as usize;
        // Sign-extend hi_int to size_t, then shift up by 32 bits.
        let hi_us = hi_int as isize as usize;
        data |= (hi_us << 16) << 16;

        v3 ^= data;
        sipround!();
        sipround!();
        v0 ^= data;
        i += sz;
        d = d.add(sz);
    }

    let mut data: usize = (len << (STBDS_SIZE_T_BITS - 8)) as usize;
    let rem = len - i;
    // Fall-through switch in C — cumulative.
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
        // C: `data |= (d[3] << 24);` — d[3] promoted to int, shift in
        // int domain. If d[3] >= 0x80 the int is negative; assigning to
        // size_t sign-extends. Reproduce that.
        let v = (*d.add(3) as i32).wrapping_shl(24);
        data |= v as isize as usize;
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
    sipround!();
    sipround!();
    v0 ^= data;
    v2 ^= 0xff;
    sipround!();
    sipround!();
    sipround!();
    sipround!();

    v0 ^ v1 ^ v2 ^ v3
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
}

// ------------------------------------------------------------------
// Internal helpers.
// ------------------------------------------------------------------
#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

fn log2_count(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn make_hash_index(slot_count: usize, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let total = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<StbdsHashBucket>()
        + std::mem::size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let raw = realloc(ptr::null_mut(), total) as *mut StbdsHashIndex;
    let t = raw;
    (*t).storage = align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = log2_count(slot_count);
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
        (*t).string = StbdsStringArena::default();
        let mut seed_lock = HASH_SEED.lock().unwrap();
        (*t).seed = *seed_lock;
        let a: usize = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b: usize = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        *seed_lock = (*seed_lock).wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i = 0usize;
        while i < (slot_count >> STBDS_BUCKET_SHIFT) {
            let bucket = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bucket).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bucket).index[j] = STBDS_INDEX_EMPTY;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let mut i = 0usize;
        while i < ((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                let idx = (*ob).index[j];
                if idx >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = probe_position(hash, (*t).slot_count);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = idx;
                                break 'outer;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = idx;
                                break 'outer;
                            }
                            z += 1;
                        }

                        pos += step;
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

#[inline]
fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    // Macro from C:
    //   temp = v64_lo ^ v32; temp <<=16; temp <<=16; temp >>= 16; temp >>= 16;
    //   var = v64_hi; var <<= 16; var <<= 16;
    //   var ^= temp ^ v32;
    let mut temp = v64_lo ^ v32;
    temp = temp.wrapping_shl(16);
    temp = temp.wrapping_shl(16);
    temp = temp.wrapping_shr(16);
    temp = temp.wrapping_shr(16);
    let mut var = v64_hi;
    var = var.wrapping_shl(16);
    var = var.wrapping_shl(16);
    var ^= temp ^ v32;
    var
}

#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    (*header(a)).hash_table as *mut StbdsHashIndex
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_ptr = (a as *mut u8).add(elemsize * i as usize + keyoffset) as *mut *mut c_char;
        strcmp(key as *const c_char, *stored_ptr) == 0
    } else {
        memcmp(
            key,
            (a as *mut u8).add(elemsize * i as usize + keyoffset) as *const c_void,
            keysize,
        ) == 0
    }
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !hash_table(a).is_null() {
        if (*hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
            let mut i = 1usize;
            let len = (*header(a)).length;
            while i < len {
                let ptr = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                free(*ptr as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&mut (*hash_table(a)).string as *mut StbdsStringArena);
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut c_void);
}

unsafe fn hm_find_slot(
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
        hash += 2;
    }

    let mut pos = probe_position(hash, (*table).slot_count);

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
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
                if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
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
            let slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[slot as usize & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let raw = hash_to_arr(p, elemsize);
    (*header(raw)).temp = temp;
    p
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmput_default(
    mut a: *mut c_void,
    elemsize: usize,
) -> *mut c_void {
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

unsafe fn strdup_c(s: *mut c_char) -> *mut c_char {
    let len = strlen(s) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;

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
        let nt = make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT as u8
            } else {
                STBDS_SH_NONE as u8
            };
        }
        (*header(a)).hash_table = nt as *mut c_void;
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

    let mut pos = probe_position(hash, (*table).slot_count);

    let final_pos: usize;
    'find: loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                {
                    (*header(a)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        let stored = (raw_a as *mut u8)
                            .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                            as *mut *mut c_char;
                        // stbds_temp_key(a) = ...
                        let temp_key_ptr = (*header(a)).hash_table as *mut *mut c_char;
                        *temp_key_ptr = *stored;
                    }
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                final_pos = (pos & !STBDS_BUCKET_MASK) + i;
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
        while i < limit {
            if (*bucket).hash[i] == hash {
                if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                {
                    (*header(a)).temp = (*bucket).index[i];
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'find;
            } else if tombstone < 0 {
                if (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            i += 1;
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

    let i = arrlen(a);
    if (i + 1) as usize > arrcap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(a, elemsize);
    let _ = raw_a;

    (*header(a)).length = (i + 1) as usize;
    let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
    (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*header(a)).temp = i - 1;

    let dst = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
    match (*table).string.mode as c_int {
        x if x == STBDS_SH_STRDUP => {
            let dup = strdup_c(key as *mut c_char);
            *dst = dup;
            let temp_key_ptr = (*header(a)).hash_table as *mut *mut c_char;
            *temp_key_ptr = dup;
        }
        x if x == STBDS_SH_ARENA => {
            let alloc = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *dst = alloc;
            let temp_key_ptr = (*header(a)).hash_table as *mut *mut c_char;
            *temp_key_ptr = alloc;
        }
        x if x == STBDS_SH_DEFAULT => {
            *dst = key as *mut c_char;
            let temp_key_ptr = (*header(a)).hash_table as *mut *mut c_char;
            *temp_key_ptr = key as *mut c_char;
        }
        _ => {
            memcpy(
                (a as *mut u8).add(elemsize * i as usize) as *mut c_void,
                key,
                keysize,
            );
        }
    }
    arr_to_hash(a, elemsize)
}

#[no_mangle]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let h = make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

#[no_mangle]
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
    (*header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }
    let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }
    let mut b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index = arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
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
            let p = (a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                as *mut *mut c_char;
            slot = hm_find_slot(a, elemsize, *p as *mut c_void, keysize, keyoffset, mode);
        } else {
            let p = (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut c_void;
            slot = hm_find_slot(a, elemsize, p, keysize, keyoffset, mode);
        }
        b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*header(raw_a)).hash_table =
            make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header(raw_a)).hash_table = make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// ------------------------------------------------------------------
// String arena.
// ------------------------------------------------------------------
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[no_mangle]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
    s: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(s) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // sizeof(stbds_string_block)-8 + len.  In C, sizeof(stbds_string_block)
            // is sizeof(next pointer) + 8 (storage). On 64-bit ABI = 16; -8 = 8.
            // Equivalent: size_of::<*mut StbdsStringBlock>() + len.
            let total = std::mem::size_of::<*mut StbdsStringBlock>() + len;
            let sb = realloc(ptr::null_mut(), total) as *mut StbdsStringBlock;
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
            return (*sb).storage.as_mut_ptr();
        } else {
            let total = std::mem::size_of::<*mut StbdsStringBlock>() + blocksize;
            let sb = realloc(ptr::null_mut(), total) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    p = (*(*a).storage)
        .storage
        .as_mut_ptr()
        .add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

#[no_mangle]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, std::mem::size_of::<StbdsStringArena>());
}

// ------------------------------------------------------------------
// strkey() — exported by the C lib because there are no static markers.
// ------------------------------------------------------------------
static STRKEY_BUFFER: Mutex<[u8; 256]> = Mutex::new([0u8; 256]);

#[no_mangle]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    // C buffer is `static char buffer[256]`; we use a Mutex-guarded
    // global. For tests we only need to return a valid C string with the
    // formatted contents.
    let mut buf = STRKEY_BUFFER.lock().unwrap();
    let s = format!("test_{}\0", n);
    let bytes = s.as_bytes();
    let n = bytes.len().min(256);
    buf[..n].copy_from_slice(&bytes[..n]);
    // Ensure NUL termination.
    if n < 256 {
        buf[n] = 0;
    } else {
        buf[255] = 0;
    }
    // Return a stable pointer into the global buffer. Since callers only
    // read the value before the next call, this matches the C semantics
    // exactly. Leak the lock by leaving the underlying buffer in place —
    // we drop the guard at function exit but the storage is `'static`.
    // Get a raw pointer to the static array's contents.
    let ptr = buf.as_mut_ptr() as *mut c_char;
    drop(buf);
    ptr
}

// ------------------------------------------------------------------
// Public test driver.
// ------------------------------------------------------------------
#[repr(C)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[no_mangle]
pub unsafe extern "C" fn str_dups(num: c_int) {
    let mut sa = StbdsStringArena::default();
    for i in 0..num {
        // Use sprintf locally — we don't care about reading from strkey()
        // since our only goal is to feed strings to stbds_stralloc.
        let mut local: [c_char; 64] = [0; 64];
        let fmt = b"test_%d\0".as_ptr() as *const c_char;
        sprintf(local.as_mut_ptr(), fmt, i);
        stbds_stralloc(&mut sa, local.as_mut_ptr());
    }
    stbds_strreset(&mut sa);

    // sh_new_strdup + shputs + iteration
    let elemsize = std::mem::size_of::<StrMapEntry>();
    let mut strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP);
    let key_lit = b"a\0".as_ptr() as *mut c_char;
    let s = StrMapEntry {
        key: key_lit,
        value: num,
    };
    // shputs(strmap, s):
    //   t = stbds_hmput_key_wrapper(t, sizeof *t, (void*) s.key, sizeof t->key, STBDS_HM_STRING);
    //   t[stbds_temp(t-1)] = s;
    //   t[stbds_temp(t-1)].key = stbds_temp_key(t-1);
    strmap = stbds_hmput_key(
        strmap,
        elemsize,
        s.key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        STBDS_HM_STRING,
    );
    let raw = hash_to_arr(strmap, elemsize);
    let temp_idx = (*header(raw)).temp;
    let entry = (strmap as *mut u8).add(elemsize * temp_idx as usize) as *mut StrMapEntry;
    *entry = StrMapEntry { key: s.key, value: s.value };
    let temp_key = *((*header(raw)).hash_table as *mut *mut c_char);
    (*entry).key = temp_key;

    // The C `printf("%s %d\n", strmap[z], strmap[z].value)` reads the
    // first 8 bytes of strmap[z] (key pointer) as a `char*` for `%s` and
    // the 4-byte value field for `%d` (passed in the second variadic
    // slot due to integer promotion / SysV ABI quirks).
    let fmt = b"%s %d\n\0".as_ptr() as *const c_char;
    printf(fmt, (*entry).key, (*entry).value);

    stbds_hmfree_func(raw, elemsize);
}
