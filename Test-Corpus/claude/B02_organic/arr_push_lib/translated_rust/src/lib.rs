//! Rust translation of c_src/src/lib.c.
//!
//! This translation reproduces every symbol that the C shared library exports,
//! so that an external caller loading the Rust .so via libloading sees the
//! identical ABI: the public `arr_push` plus all the internal `stbds_*`
//! dynamic-array / hash-map helpers and the `strkey` test helper.
//!
//! Memory layout matches the C version exactly: arrays are heap allocations
//! whose first `sizeof(stbds_array_header)` bytes hold the header and the
//! returned pointer is offset to the first element. All allocations go
//! through libc realloc/free, just like the C code's STBDS_REALLOC/STBDS_FREE.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_uchar, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// libc bindings (mirrors what STBDS_REALLOC / STBDS_FREE expand to).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(buf: *mut c_char, format: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Constants (mirror the #defines in lib.c).
// ---------------------------------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 -> 3
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

#[inline]
fn STBDS_INDEX_IN_USE(x: isize) -> bool {
    x >= 0
}

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() as u32) * 8;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[inline]
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// ---------------------------------------------------------------------------
// Header / index types.
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
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: c_uchar,
    pub mode: c_uchar,
}

#[repr(C)]
#[derive(Copy, Clone)]
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

// ---------------------------------------------------------------------------
// Helpers: header / hash-table access (mirroring the C macros).
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).sub(1) }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    unsafe {
        if a.is_null() {
            0
        } else {
            (*stbds_header(a)).length as isize
        }
    }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    unsafe {
        if a.is_null() {
            0
        } else {
            (*stbds_header(a)).capacity
        }
    }
}

#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> *mut isize {
    unsafe { &mut (*stbds_header(t)).temp as *mut isize }
}

#[inline]
unsafe fn stbds_temp_key(t: *mut c_void) -> *mut *mut c_char {
    unsafe { (*stbds_header(t)).hash_table as *mut *mut c_char }
}

#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

// ---------------------------------------------------------------------------
// Rotate helpers.
// ---------------------------------------------------------------------------

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    // Match the C macro: ((val) << n) | ((val) >> (BITS - n))
    // Use wrapping/u32-safe shifts; when n == 0 this would shift by BITS and
    // be UB in C, but the macro is never invoked with n == 0 in this file.
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf and stbds_arrfreef.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    unsafe {
        let min_len = stbds_arrlen(a) as usize + addlen;

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

        let old_header = if a.is_null() {
            ptr::null_mut::<c_void>()
        } else {
            stbds_header(a) as *mut c_void
        };

        let new_size = elemsize * min_cap + size_of::<stbds_array_header>();
        let raw = realloc(old_header, new_size);
        // b = (char*) b + sizeof(stbds_array_header)
        let b = (raw as *mut u8).add(size_of::<stbds_array_header>()) as *mut c_void;
        if a.is_null() {
            (*stbds_header(b)).length = 0;
            (*stbds_header(b)).hash_table = ptr::null_mut();
            (*stbds_header(b)).temp = 0;
        }
        (*stbds_header(b)).capacity = min_cap;
        b
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        free(stbds_header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// Random seed.
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

// ---------------------------------------------------------------------------
// Hash bucket / hash index machinery.
// ---------------------------------------------------------------------------

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

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
            + size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1;
        let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;

        // t->storage = ALIGN_FWD((size_t)(t+1), CACHE_LINE_SIZE)
        let after_index = (t as usize) + size_of::<stbds_hash_index>();
        (*t).storage = STBDS_ALIGN_FWD(after_index, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;

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
                &mut (*t).string as *mut stbds_string_arena as *mut c_void,
                0,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            // Mirror the C `stbds_load_32_or_64` pair that updates STBDS_HASH_SEED.
            // Compute a and b on a 64-bit size_t target.
            let a: usize = compute_load_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b: usize = compute_load_64(715136305, 0, 0xb504f32d);
            STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
        }

        // Initialize all buckets.
        let nbuckets = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..nbuckets {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
        }

        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            let old_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
            for i in 0..old_buckets {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if STBDS_INDEX_IN_USE((*ob).index[j]) {
                        let hash = (*ob).hash[j];
                        let mut pos =
                            stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                        let mut step = STBDS_BUCKET_LENGTH;
                        'rehash_loop: loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                            let limit_start = pos & STBDS_BUCKET_MASK;
                            let mut placed = false;
                            for z in limit_start..STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    placed = true;
                                    break;
                                }
                            }
                            if placed {
                                break 'rehash_loop;
                            }

                            for z in 0..limit_start {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    placed = true;
                                    break;
                                }
                            }
                            if placed {
                                break 'rehash_loop;
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
}

#[inline]
fn compute_load_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    // Replicates the stbds_load_32_or_64 macro on a 64-bit `size_t`.
    //   temp = v64_lo ^ v32; temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16
    //   var  = v64_hi; var <<= 16; var <<= 16
    //   var ^= temp ^ v32
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
// String hash.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        while *str_ != 0 {
            // C: hash = ROTATE_LEFT(hash, 9) + (unsigned char) *str++;
            hash = stbds_rotate_left(hash, 9).wrapping_add(*str_ as u8 as usize);
            str_ = str_.add(1);
        }
        hash ^= seed;
        // hash = (~hash) + (hash << 18)
        hash = (!hash).wrapping_add(hash << 18);
        // hash ^= hash ^ ROTATE_RIGHT(hash, 31)
        hash ^= hash ^ stbds_rotate_right(hash, 31);
        // hash = hash * 21
        hash = hash.wrapping_mul(21);
        hash ^= hash ^ stbds_rotate_right(hash, 11);
        hash = hash.wrapping_add(hash << 6);
        hash ^= stbds_rotate_right(hash, 22);
        hash.wrapping_add(seed)
    }
}

// ---------------------------------------------------------------------------
// SipHash-2-4 byte hash.
// ---------------------------------------------------------------------------

#[inline(always)]
fn siphash_round(v: &mut [usize; 4]) {
    let half = STBDS_SIZE_T_BITS / 2;
    v[0] = v[0].wrapping_add(v[1]);
    v[1] = stbds_rotate_left(v[1], 13);
    v[1] ^= v[0];
    v[0] = stbds_rotate_left(v[0], half);
    v[2] = v[2].wrapping_add(v[3]);
    v[3] = stbds_rotate_left(v[3], 16);
    v[3] ^= v[2];
    v[2] = v[2].wrapping_add(v[1]);
    v[1] = stbds_rotate_left(v[1], 17);
    v[1] ^= v[2];
    v[2] = stbds_rotate_left(v[2], half);
    v[0] = v[0].wrapping_add(v[3]);
    v[3] = stbds_rotate_left(v[3], 21);
    v[3] ^= v[0];
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *mut u8;
        let mut v: [usize; 4] = [0; 4];

        // v0 = ((((size_t) 0x736f6d65 << 16) << 16) + 0x70736575) ^  seed;
        v[0] = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        // v1 = ((((size_t) 0x646f7261 << 16) << 16) + 0x6e646f6d) ^ ~seed;
        v[1] = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        v[2] = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        v[3] = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v[0] ^= 0x0706050403020100usize ^ seed;
        v[1] ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v[2] ^= 0x0706050403020100usize ^ seed;
        v[3] ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let step = size_of::<usize>();
        let mut i = 0usize;
        while i + step <= len {
            // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
            // data |= ((d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16) << 16;
            let lo: u32 = (*d as u32)
                | ((*d.add(1) as u32) << 8)
                | ((*d.add(2) as u32) << 16)
                | ((*d.add(3) as u32) << 24);
            let mut data = lo as usize;
            let hi: u32 = (*d.add(4) as u32)
                | ((*d.add(5) as u32) << 8)
                | ((*d.add(6) as u32) << 16)
                | ((*d.add(7) as u32) << 24);
            data |= ((hi as usize) << 16) << 16;

            v[3] ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round(&mut v);
            }
            v[0] ^= data;

            i += step;
            d = d.add(step);
        }
        // Last block with length encoded in top byte.
        let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
        // C uses fall-through switch on (len - i):
        let rem = len - i;
        // case 7: data |= ((size_t) d[6] << 24) << 24;
        if rem >= 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        // case 6: data |= ((size_t) d[5] << 20) << 20;
        if rem >= 6 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        // case 5: data |= ((size_t) d[4] << 16) << 16;
        if rem >= 5 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        // case 4: data |= (d[3] << 24);  -- C uses int promotion of u8 then shift-by-24
        if rem >= 4 {
            data |= (*d.add(3) as u32 as usize) << 24;
        }
        if rem >= 3 {
            data |= (*d.add(2) as u32 as usize) << 16;
        }
        if rem >= 2 {
            data |= (*d.add(1) as u32 as usize) << 8;
        }
        if rem >= 1 {
            data |= *d as u32 as usize;
        }

        v[3] ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v);
        }
        v[0] ^= data;
        v[2] ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            siphash_round(&mut v);
        }

        v[0] ^ v[1] ^ v[2] ^ v[3]
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// Key equality.
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
    unsafe {
        if mode >= STBDS_HM_STRING {
            // 0 == strcmp(key, *(char**) (a + elemsize*i + keyoffset))
            let stored_key_ptr =
                (a as *mut u8).add(elemsize.wrapping_mul(i as usize) + keyoffset) as *mut *mut c_char;
            strcmp(key as *const c_char, *stored_key_ptr) == 0
        } else {
            let region = (a as *mut u8).add(elemsize.wrapping_mul(i as usize) + keyoffset)
                as *const c_void;
            memcmp(key, region, keysize) == 0
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hmfree_func.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        let table = stbds_hash_table(a);
        if !table.is_null() {
            if (*table).string.mode as c_int == STBDS_SH_STRDUP {
                let len = (*stbds_header(a)).length;
                let mut i = 1usize;
                while i < len {
                    let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                    free(*p as *mut c_void);
                    i += 1;
                }
            }
            stbds_strreset(&mut (*table).string as *mut stbds_string_arena);
        }
        free((*stbds_header(a)).hash_table);
        free(stbds_header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// Hash slot finder.
// ---------------------------------------------------------------------------

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
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

            // Forward sweep
            let limit = pos & STBDS_BUCKET_MASK;
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
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
            }

            for i in 0..limit {
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
            }

            pos += step;
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }
    }
}

// ---------------------------------------------------------------------------
// hmget_key_ts / hmget_key.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset = 0usize;
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
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
                    let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[slot as usize & STBDS_BUCKET_MASK];
                }
            }
            return a;
        }
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
    unsafe {
        let mut temp: isize = 0;
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        *stbds_temp(stbds_hash_to_arr(p, elemsize)) = temp;
        p
    }
}

// ---------------------------------------------------------------------------
// hmput_default.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
            let prev = if a.is_null() {
                ptr::null_mut::<c_void>()
            } else {
                stbds_hash_to_arr(a, elemsize)
            };
            a = stbds_arrgrowf(prev, elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            memset(a, 0, elemsize);
            a = stbds_arr_to_hash(a, elemsize);
        }
        a
    }
}

// ---------------------------------------------------------------------------
// stbds_strdup (file-static helper, not exported).
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(s) + 1;
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, s as *const c_void, len);
        p
    }
}

// ---------------------------------------------------------------------------
// hmput_key.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset = 0usize;

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
                    STBDS_SH_DEFAULT as c_uchar
                } else {
                    STBDS_SH_NONE as c_uchar
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
        let bucket;

        // Probe loop: emulate goto with a labeled loop.
        let final_pos: usize;
        'probe: loop {
            let limit = pos & STBDS_BUCKET_MASK;
            let cur_bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*cur_bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*cur_bucket).index[i],
                    ) {
                        *stbds_temp(a) = (*cur_bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            let p = (raw_a as *mut u8).add(
                                elemsize * (*cur_bucket).index[i] as usize + keyoffset,
                            ) as *mut *mut c_char;
                            *stbds_temp_key(a) = *p;
                        }
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*cur_bucket).hash[i] == 0 {
                    final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                    bucket = cur_bucket;
                    break 'probe;
                } else if tombstone < 0
                    && (*cur_bucket).index[i] == STBDS_INDEX_DELETED
                {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }

            for i in 0..limit {
                if (*cur_bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*cur_bucket).index[i],
                    ) {
                        *stbds_temp(a) = (*cur_bucket).index[i];
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*cur_bucket).hash[i] == 0 {
                    final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                    bucket = cur_bucket;
                    break 'probe;
                } else if tombstone < 0
                    && (*cur_bucket).index[i] == STBDS_INDEX_DELETED
                {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }

            pos += step;
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        // found_empty_slot:
        let mut pos = final_pos;
        let mut bucket_used = bucket;

        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        let i = stbds_arrlen(a);
        if (i as usize + 1) > stbds_arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = stbds_arr_to_hash(a, elemsize);

        (*stbds_header(a)).length = (i + 1) as usize;
        bucket_used = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket_used).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket_used).index[pos & STBDS_BUCKET_MASK] = i - 1;
        *stbds_temp(a) = i - 1;

        let _ = bucket_used; // suppress unused warning
        let _ = raw_a;

        match (*table).string.mode as c_int {
            x if x == STBDS_SH_STRDUP => {
                let dest = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                let dup = stbds_strdup(key as *mut c_char);
                *dest = dup;
                *stbds_temp_key(a) = dup;
            }
            x if x == STBDS_SH_ARENA => {
                let dest = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                let alloc = stbds_stralloc(
                    &mut (*table).string as *mut stbds_string_arena,
                    key as *mut c_char,
                );
                *dest = alloc;
                *stbds_temp_key(a) = alloc;
            }
            x if x == STBDS_SH_DEFAULT => {
                let dest = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dest = key as *mut c_char;
                *stbds_temp_key(a) = key as *mut c_char;
            }
            _ => {
                let dest = (a as *mut u8).add(elemsize * i as usize) as *mut c_void;
                memcpy(dest, key as *const c_void, keysize);
            }
        }
        stbds_arr_to_hash(a, elemsize)
    }
}

// ---------------------------------------------------------------------------
// shmode_func.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as c_uchar;
        stbds_arr_to_hash(a, elemsize)
    }
}

// ---------------------------------------------------------------------------
// hmdel_key.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            return ptr::null_mut();
        }
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        *stbds_temp(raw_a) = 0;
        if table.is_null() {
            return a;
        }
        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }
        let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        *stbds_temp(raw_a) = 1;
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode as c_int == STBDS_SH_STRDUP {
            let p = (a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char;
            free(*p as *mut c_void);
        }

        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
                (a as *const u8).add(elemsize * final_index as usize) as *const c_void,
                elemsize,
            );
            slot = if mode == STBDS_HM_STRING {
                let kp = (a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                    as *mut *mut c_char;
                stbds_hm_find_slot(a, elemsize, *kp as *mut c_void, keysize, keyoffset, mode)
            } else {
                let kp = (a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                    as *mut c_void;
                stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode)
            };
            let b2 = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            (*b2).index[i] = old_index;
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
}

// ---------------------------------------------------------------------------
// stralloc / strreset.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    s: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = strlen(s) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block += 1;
            }

            if len > blocksize {
                let alloc_size = size_of::<stbds_string_block>() - 8 + len;
                let sb = realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
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
                let alloc_size = size_of::<stbds_string_block>() - 8 + blocksize;
                let sb = realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
            .add((*a).remaining - len) as *mut c_char;
        (*a).remaining -= len;
        memmove(p as *mut c_void, s as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            free(x as *mut c_void);
            x = y;
        }
        memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// strkey: writes "test_%d" into a 256-byte file-scope static buffer and
// returns its address. Mirrors the C implementation (incl. shared buffer).
// ---------------------------------------------------------------------------

static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let fmt = b"test_%d\0".as_ptr() as *const c_char;
        let buf_ptr = std::ptr::addr_of_mut!(STRKEY_BUFFER) as *mut c_char;
        sprintf(buf_ptr, fmt, n);
        buf_ptr
    }
}

// ---------------------------------------------------------------------------
// arr_push: the only public symbol declared in lib.h.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    unsafe {
        // int *arr = NULL; STBDS_ASSERT(arrlen(arr)==0);
        // We faithfully reproduce the array-grow / array-free flow of the C
        // version, going through stbds_arrgrowf / stbds_arrfreef so any
        // observable side-effect (allocator calls, header layout) matches.
        let mut arr: *mut c_int = ptr::null_mut();
        // arrlen(NULL) == 0, so the assertion always trivially holds.

        let mut i: c_int = 0;
        while i < num {
            let mut j: c_int = 0;
            while j < i {
                // arrpush(arr, j) ==
                //   arrmaybegrow(arr, 1); arr[header->length++] = j;
                if arr.is_null()
                    || (*stbds_header(arr as *mut c_void)).length + 1
                        > (*stbds_header(arr as *mut c_void)).capacity
                {
                    arr = stbds_arrgrowf(
                        arr as *mut c_void,
                        size_of::<c_int>(),
                        1,
                        0,
                    ) as *mut c_int;
                }
                let len = (*stbds_header(arr as *mut c_void)).length;
                *arr.add(len) = j;
                (*stbds_header(arr as *mut c_void)).length = len + 1;
                j += 1;
            }
            // arrfree(arr): if arr != NULL, free(header(arr)); arr = NULL.
            if !arr.is_null() {
                stbds_arrfreef(arr as *mut c_void);
                arr = ptr::null_mut();
            }
            i += 50;
        }
    }
}
