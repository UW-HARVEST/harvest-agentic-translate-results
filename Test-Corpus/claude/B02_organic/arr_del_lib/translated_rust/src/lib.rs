// Rust translation of c_src/src/lib.c
//
// This is a near-1:1 port of the stb_ds dynamic-array / hash-map subset
// that the C source uses. The C library exports many `stbds_*` symbols;
// to make the Rust .so a drop-in replacement we re-export the same names
// with identical signatures and behavior.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::missing_transmute_annotations)]
#![allow(unused_assignments)]

use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

type size_t = usize;
type ptrdiff_t = isize;

// ---------------------------------------------------------------------------
// libc bindings (allocation, mem*, str*)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: size_t) -> c_int;
    fn memset(dst: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> size_t;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Constants & types matching the C code
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_array_header {
    pub length: size_t,
    pub capacity: size_t,
    pub hash_table: *mut c_void,
    pub temp: ptrdiff_t,
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: size_t,
    pub block: u8,
    pub mode: u8,
}

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: ptrdiff_t = -1;
const STBDS_INDEX_DELETED: ptrdiff_t = -2;

const STBDS_HASH_EMPTY: size_t = 0;
const STBDS_HASH_DELETED: size_t = 1;

#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [size_t; STBDS_BUCKET_LENGTH],
    pub index: [ptrdiff_t; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_hash_index {
    pub temp_key: *mut c_char,
    pub slot_count: size_t,
    pub used_count: size_t,
    pub used_count_threshold: size_t,
    pub used_count_shrink_threshold: size_t,
    pub tombstone_count: size_t,
    pub tombstone_count_threshold: size_t,
    pub seed: size_t,
    pub slot_count_log2: size_t,
    pub string: stbds_string_arena,
    pub storage: *mut stbds_hash_bucket,
}

const HEADER_SIZE: usize = std::mem::size_of::<stbds_array_header>();

#[inline]
unsafe fn stbds_header(a: *mut c_void) -> *mut stbds_array_header {
    (a as *mut u8).sub(HEADER_SIZE) as *mut stbds_array_header
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> ptrdiff_t {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as ptrdiff_t
    }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> size_t {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: size_t) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: size_t) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ---------------------------------------------------------------------------
// Random seed
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: size_t = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: size_t) {
    STBDS_HASH_SEED = seed;
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: size_t,
    addlen: size_t,
    mut min_cap: size_t,
) -> *mut c_void {
    let min_len = (stbds_arrlen(a) as size_t).wrapping_add(addlen);

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

    let old_ptr: *mut c_void = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };

    let mut b = realloc(old_ptr, elemsize * min_cap + HEADER_SIZE);
    b = (b as *mut u8).add(HEADER_SIZE) as *mut c_void;

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

// ---------------------------------------------------------------------------
// stbds_log2
// ---------------------------------------------------------------------------

unsafe fn stbds_log2(mut slot_count: size_t) -> size_t {
    let mut n: size_t = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// stbds_probe_position
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_probe_position(hash: size_t, slot_count: size_t, _slot_log2: size_t) -> size_t {
    hash & (slot_count - 1)
}

// ---------------------------------------------------------------------------
// stbds_make_hash_index
// ---------------------------------------------------------------------------

unsafe fn stbds_align_fwd(n: size_t, a: size_t) -> size_t {
    (n + (a - 1)) & !(a - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: size_t,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;

    let raw_after = (t as *mut u8).add(std::mem::size_of::<stbds_hash_index>()) as size_t;
    let aligned = stbds_align_fwd(raw_after, STBDS_CACHE_LINE_SIZE);
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
        (*t).string = stbds_string_arena {
            storage: (*ot).string.storage,
            remaining: (*ot).string.remaining,
            block: (*ot).string.block,
            mode: (*ot).string.mode,
        };
        (*t).seed = (*ot).seed;
    } else {
        memset(
            &mut (*t).string as *mut _ as *mut c_void,
            0,
            std::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = STBDS_HASH_SEED;

        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
        // For 64-bit: a = (0x27bb2ee6 << 32) ^ (0x87b0b0fd ^ 2147001325 << 32 >> 32 ... )
        // Direct semantics, then the result on 64-bit ends up as
        //   a = ((0x27bb2ee6 << 32) ^ ((0x87b0b0fd ^ 2147001325) & 0xffffffff)) ^ 2147001325
        // Actually simpler: read the macro line by line.
        //   temp = v64_lo ^ v32 = 0x87b0b0fd ^ 2147001325
        //   temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16; -- truncates to low 32 bits
        //   var = v64_hi = 0x27bb2ee6
        //   var <<= 16; var <<= 16; -> shifts to high 32 bits
        //   var ^= temp ^ v32 -> low 32 bits become temp ^ v32 = (v64_lo ^ v32) ^ v32 = v64_lo
        // So for 64-bit: a = (v64_hi << 32) | (v64_lo & 0xFFFFFFFF) effectively.
        let a: size_t = ((0x27bb2ee6u64 << 32) as size_t) | (0x87b0b0fdu64 as size_t);
        let b: size_t = ((0u64 << 32) as size_t) | (0xb504f32du64 as size_t);

        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: size_t = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
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
        let mut i: size_t = 0;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
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
                        let mut z2: size_t = 0;
                        while z2 < limit {
                            if (*bucket).hash[z2] == 0 {
                                (*bucket).hash[z2] = hash;
                                (*bucket).index[z2] = (*ob).index[j];
                                break 'outer;
                            }
                            z2 += 1;
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
// stbds_hash_string
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<size_t>() * 8) as u32;

#[inline]
fn rotl(val: size_t, n: u32) -> size_t {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotr(val: size_t, n: u32) -> size_t {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str: *mut c_char, seed: size_t) -> size_t {
    let mut hash: size_t = seed;
    while *str != 0 {
        hash = rotl(hash, 9).wrapping_add(*(str as *mut u8) as size_t);
        str = str.add(1);
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

// ---------------------------------------------------------------------------
// siphash
// ---------------------------------------------------------------------------

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline]
fn siphash_round(v0: &mut size_t, v1: &mut size_t, v2: &mut size_t, v3: &mut size_t) {
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

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    let mut d = p as *mut u8;

    let mut v0: size_t = (((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: size_t = (((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: size_t = (((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: size_t = (((0x74656462usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i: size_t = 0;
    while i + std::mem::size_of::<size_t>() <= len {
        let d0 = *d as size_t;
        let d1 = *d.add(1) as size_t;
        let d2 = *d.add(2) as size_t;
        let d3 = *d.add(3) as size_t;
        let d4 = *d.add(4) as size_t;
        let d5 = *d.add(5) as size_t;
        let d6 = *d.add(6) as size_t;
        let d7 = *d.add(7) as size_t;
        let mut data: size_t = d0 | (d1 << 8) | (d2 << 16) | (d3 << 24);
        data |= ((d4 | (d5 << 8) | (d6 << 16) | (d7 << 24)) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += std::mem::size_of::<size_t>();
        d = d.add(std::mem::size_of::<size_t>());
    }

    let mut data: size_t = len << (STBDS_SIZE_T_BITS - 8);
    // Mirror the C switch fallthrough.
    let tail = len - i;
    if tail >= 7 {
        data |= ((*d.add(6) as size_t) << 24) << 24;
    }
    if tail >= 6 {
        data |= ((*d.add(5) as size_t) << 20) << 20;
    }
    if tail >= 5 {
        data |= ((*d.add(4) as size_t) << 16) << 16;
    }
    if tail >= 4 {
        // The C cast (d[3] << 24) is on `int`. Replicate exactly:
        // unsigned char << 24 in C is computed in `int`, gets sign-extended
        // when ORed into `size_t`. *d.add(3) is u8, cast to i32 and shifted is fine
        // because top bit can never be set (u8 max is 0xFF). After shift, the
        // result is at most 0xFF00_0000 which is positive in i32. So no SE issue.
        data |= (*d.add(3) as size_t) << 24;
    }
    if tail >= 3 {
        data |= (*d.add(2) as size_t) << 16;
    }
    if tail >= 2 {
        data |= (*d.add(1) as size_t) << 8;
    }
    if tail >= 1 {
        data |= *d as size_t;
    }
    // tail == 0 -> nothing

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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// stbds_is_key_equal
// ---------------------------------------------------------------------------

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
    i: ptrdiff_t,
) -> c_int {
    if mode >= STBDS_HM_STRING {
        let elem_ptr = (a as *mut u8).add(elemsize.wrapping_mul(i as size_t) + keyoffset);
        let stored: *mut c_char = *(elem_ptr as *mut *mut c_char);
        if strcmp(key as *const c_char, stored) == 0 {
            1
        } else {
            0
        }
    } else {
        let elem_ptr = (a as *mut u8).add(elemsize.wrapping_mul(i as size_t) + keyoffset);
        if memcmp(key, elem_ptr as *const c_void, keysize) == 0 {
            1
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hmfree_func
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: size_t) {
    if a.is_null() {
        return;
    }
    let table = stbds_hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let mut i: size_t = 1;
            while i < (*stbds_header(a)).length {
                let elem_ptr = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                free(*elem_ptr as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// stbds_hm_find_slot
// ---------------------------------------------------------------------------

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
) -> ptrdiff_t {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
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
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) != 0 {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        let limit = pos & STBDS_BUCKET_MASK;
        let mut i2: size_t = 0;
        while i2 < limit {
            if (*bucket).hash[i2] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i2]) != 0 {
                    return ((pos & !STBDS_BUCKET_MASK) + i2) as ptrdiff_t;
                }
            } else if (*bucket).hash[i2] == STBDS_HASH_EMPTY {
                return -1;
            }
            i2 += 1;
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

// ---------------------------------------------------------------------------
// stbds_hmget_key_ts
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    temp: *mut ptrdiff_t,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: size_t = 0;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return stbds_arr_to_hash(a, elemsize);
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
                let b = (*table).storage.add(slot as size_t >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[slot as size_t & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

// ---------------------------------------------------------------------------
// stbds_hmget_key
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    mode: c_int,
) -> *mut c_void {
    let mut temp: ptrdiff_t = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let raw = stbds_hash_to_arr(p, elemsize);
    (*stbds_header(raw)).temp = temp;
    p
}

// ---------------------------------------------------------------------------
// stbds_hmput_default
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: size_t) -> *mut c_void {
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let raw = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

// ---------------------------------------------------------------------------
// stbds_strdup
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str: *mut c_char) -> *mut c_char {
    let len = strlen(str) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ---------------------------------------------------------------------------
// stbds_hmput_key
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: size_t = 0;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
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
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                STBDS_SH_NONE
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
    let mut tombstone: ptrdiff_t = -1;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    'outer: loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) != 0 {
                    (*stbds_header(a)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        // stbds_temp_key(a) = *(char**)((char*)raw_a + elemsize*idx + keyoffset)
                        let stored = *((raw_a as *mut u8).add(elemsize * (*bucket).index[i] as size_t + keyoffset) as *mut *mut c_char);
                        // temp_key is stored at hash_table->temp_key (stbds_temp_key macro
                        // uses `(*(char **) stbds_header(t)->hash_table)` which writes the
                        // first field of the hash_index struct)
                        *(((*stbds_header(a)).hash_table) as *mut *mut c_char) = stored;
                    }
                    return stbds_arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'outer;
            } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
            }
            i += 1;
        }

        let limit = pos & STBDS_BUCKET_MASK;
        let mut i2: size_t = 0;
        while i2 < limit {
            if (*bucket).hash[i2] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i2]) != 0 {
                    (*stbds_header(a)).temp = (*bucket).index[i2];
                    return stbds_arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i2] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i2;
                break 'outer;
            } else if tombstone < 0 && (*bucket).index[i2] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i2) as ptrdiff_t;
            }
            i2 += 1;
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    // found_empty_slot
    if tombstone >= 0 {
        pos = tombstone as size_t;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    {
        let i: ptrdiff_t = stbds_arrlen(a);
        if (i + 1) as size_t > stbds_arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = stbds_arr_to_hash(a, elemsize);

        (*stbds_header(a)).length = (i + 1) as size_t;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*stbds_header(a)).temp = i - 1;

        match (*table).string.mode {
            x if x == STBDS_SH_STRDUP => {
                let dup = stbds_strdup(key as *mut c_char);
                let elem_ptr = (a as *mut u8).add(elemsize * i as size_t) as *mut *mut c_char;
                *elem_ptr = dup;
                *(((*stbds_header(a)).hash_table) as *mut *mut c_char) = dup;
            }
            x if x == STBDS_SH_ARENA => {
                let alloced = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                let elem_ptr = (a as *mut u8).add(elemsize * i as size_t) as *mut *mut c_char;
                *elem_ptr = alloced;
                *(((*stbds_header(a)).hash_table) as *mut *mut c_char) = alloced;
            }
            x if x == STBDS_SH_DEFAULT => {
                let elem_ptr = (a as *mut u8).add(elemsize * i as size_t) as *mut *mut c_char;
                *elem_ptr = key as *mut c_char;
                *(((*stbds_header(a)).hash_table) as *mut *mut c_char) = key as *mut c_char;
            }
            _ => {
                let elem_ptr = (a as *mut u8).add(elemsize * i as size_t) as *mut c_void;
                memcpy(elem_ptr, key as *const c_void, keysize);
            }
        }
    }

    stbds_arr_to_hash(a, elemsize)
}

// ---------------------------------------------------------------------------
// stbds_shmode_func
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: size_t, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    stbds_arr_to_hash(a, elemsize)
}

// ---------------------------------------------------------------------------
// stbds_hmdel_key
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
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
    let mut b = (*table).storage.add(slot as size_t >> STBDS_BUCKET_SHIFT);
    let mut i = slot as size_t & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let stored = *((a as *mut u8).add(elemsize * old_index as size_t) as *mut *mut c_char);
        free(stored as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize * old_index as size_t) as *mut c_void,
            (a as *mut u8).add(elemsize * final_index as size_t) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let stored_key = *((a as *mut u8).add(elemsize * old_index as size_t + keyoffset) as *mut *mut c_char);
            slot = stbds_hm_find_slot(a, elemsize, stored_key as *mut c_void, keysize, keyoffset, mode);
        } else {
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                (a as *mut u8).add(elemsize * old_index as size_t + keyoffset) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        }
        b = (*table).storage.add(slot as size_t >> STBDS_BUCKET_SHIFT);
        i = slot as size_t & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// ---------------------------------------------------------------------------
// stbds_stralloc
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: size_t = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: size_t = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut c_char) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(str) + 1;
    if len > (*a).remaining {
        let mut blocksize: size_t = (*a).block as size_t;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // sizeof(stbds_string_block) - 8 + len = sizeof(*next ptr) + len
            let alloc_size = std::mem::size_of::<*mut stbds_string_block>() + len;
            let sb = realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            memmove((*sb).storage.as_mut_ptr() as *mut c_void, str as *const c_void, len);
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
            let alloc_size = std::mem::size_of::<*mut stbds_string_block>() + blocksize;
            let sb = realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ---------------------------------------------------------------------------
// stbds_strreset
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// strkey
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let fmt = b"test_%d\0".as_ptr() as *const c_char;
    sprintf(BUFFER.as_mut_ptr(), fmt, n);
    BUFFER.as_mut_ptr()
}

// ---------------------------------------------------------------------------
// arr_del — the public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    let mut arr: *mut c_int;

    let mut i: c_int = 0;
    while i < 4 {
        // First sequence: arrpush num,2,3,4; arrdel(arr, i); arrfree.
        arr = ptr::null_mut();
        push_i32(&mut arr, num);
        push_i32(&mut arr, 2);
        push_i32(&mut arr, 3);
        push_i32(&mut arr, 4);
        arrdel_i32(&mut arr, i as size_t);
        if !arr.is_null() {
            stbds_arrfreef(arr as *mut c_void);
            arr = ptr::null_mut();
        }

        // Second sequence: arrpush num,2,3,4; arrdelswap(arr, i); arrfree.
        push_i32(&mut arr, num);
        push_i32(&mut arr, 2);
        push_i32(&mut arr, 3);
        push_i32(&mut arr, 4);
        arrdelswap_i32(&mut arr, i as size_t);
        if !arr.is_null() {
            stbds_arrfreef(arr as *mut c_void);
            arr = ptr::null_mut();
        }

        let _ = arr; // silence unused
        i += 1;
    }
}

unsafe fn push_i32(arr: &mut *mut c_int, value: c_int) {
    // stbds_arrmaybegrow + length++
    let elemsize = std::mem::size_of::<c_int>();
    let need_grow = (*arr).is_null()
        || (*stbds_header(*arr as *mut c_void)).length + 1 > (*stbds_header(*arr as *mut c_void)).capacity;
    if need_grow {
        *arr = stbds_arrgrowf(*arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
    }
    let h = stbds_header(*arr as *mut c_void);
    let len = (*h).length;
    *(*arr).add(len) = value;
    (*h).length = len + 1;
}

unsafe fn arrdel_i32(arr: &mut *mut c_int, i: size_t) {
    // stbds_arrdeln(a,i,1):
    //   memmove(&a[i], &a[i+1], sizeof *a * (length - 1 - i)); length -= 1;
    let elemsize = std::mem::size_of::<c_int>();
    let h = stbds_header(*arr as *mut c_void);
    let length = (*h).length;
    let dst = (*arr).add(i) as *mut c_void;
    let src = (*arr).add(i + 1) as *const c_void;
    memmove(dst, src, elemsize * (length - 1 - i));
    (*h).length -= 1;
}

unsafe fn arrdelswap_i32(arr: &mut *mut c_int, i: size_t) {
    // stbds_arrdelswap(a,i): a[i] = arrlast(a); length -= 1;
    let h = stbds_header(*arr as *mut c_void);
    let last = *(*arr).add((*h).length - 1);
    *(*arr).add(i) = last;
    (*h).length -= 1;
}
