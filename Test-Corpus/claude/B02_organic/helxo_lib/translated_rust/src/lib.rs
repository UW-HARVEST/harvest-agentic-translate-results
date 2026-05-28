//! Translation of c_src/src/lib.c into Rust.
//!
//! Mirrors the C `lib.c` byte-for-byte.  The C source pulls in the stb_ds.h
//! single-header library and uses it from `helxo`.  This crate ports the
//! relevant subset of stb_ds (array growth, dynamic hashmaps, string arena)
//! plus the small helpers (`strkey`, `helxo`) so that the Rust .so exports
//! exactly the same set of symbols as the C .so.
//!
//! All public functions match the C ABI; layouts of internal structures
//! (`stbds_array_header`, `stbds_hash_index`, `stbds_hash_bucket`) match
//! their C counterparts exactly.  Memory allocations use libc realloc/free
//! so that the resulting blocks are compatible with the C library's blocks
//! (callers may mix and match exports from either .so).

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

mod c {
    use core::ffi::c_void;
    unsafe extern "C" {
        pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
        pub fn free(p: *mut c_void);
        pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
        pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
        pub fn memset(dst: *mut c_void, val: i32, n: usize) -> *mut c_void;
        pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32;
        pub fn strcmp(a: *const i8, b: *const i8) -> i32;
        pub fn strlen(s: *const i8) -> usize;
        pub fn sprintf(buf: *mut i8, fmt: *const i8, ...) -> i32;
    }
}

// ---------------------------------------------------------------------------
// Internal layouts (must match C exactly).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // log2(8)
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
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

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline]
fn index_in_use(x: isize) -> bool {
    x >= 0
}

// ---------------------------------------------------------------------------
// Module-level seed (matches `static size_t stbds_hash_seed=0x31415926;`).
// ---------------------------------------------------------------------------

static STBDS_HASH_SEED: Mutex<usize> = Mutex::new(0x31415926);

#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    let mut g = STBDS_HASH_SEED.lock().unwrap();
    *g = seed;
}

fn read_seed() -> usize {
    *STBDS_HASH_SEED.lock().unwrap()
}

fn update_seed_after_make_hash() {
    // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
    // stbds_load_32_or_64(b,temp,  715136305,         0, 0xb504f32d);
    // On 64-bit:
    //   temp = v64_lo ^ v32; temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
    //   var  = v64_hi; var <<= 16; var <<= 16;
    //   var ^= temp ^ v32;
    let mut g = STBDS_HASH_SEED.lock().unwrap();
    let a = compute_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
    let b = compute_load_32_or_64(715136305, 0, 0xb504f32d);
    *g = (*g).wrapping_mul(a).wrapping_add(b);
}

#[inline]
fn compute_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    // Replicates the macro using `usize` arithmetic.  On a 64-bit target the
    // high half is folded in; on 32-bit `var <<= 16; var <<= 16` shifts the
    // value to zero.  Both library implementations live or die together
    // because the C and Rust use the same width.
    let mut temp: usize = v64_lo ^ v32;
    temp = temp.wrapping_shl(16).wrapping_shl(16);
    temp = temp.wrapping_shr(16).wrapping_shr(16);

    let mut var: usize = v64_hi;
    var = var.wrapping_shl(16).wrapping_shl(16);
    var ^= temp ^ v32;
    var
}

// ---------------------------------------------------------------------------
// Header pointer arithmetic helpers.
// ---------------------------------------------------------------------------

#[inline]
unsafe fn header_of(a: *mut c_void) -> *mut stbds_array_header {
    unsafe { (a as *mut stbds_array_header).offset(-1) }
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header_of(a)).length as isize }
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header_of(a)).capacity }
    }
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*header_of(a)).hash_table as *mut stbds_hash_index }
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
    let cur_len = unsafe { arrlen(a) } as usize;
    let cur_cap = unsafe { arrcap(a) };
    let min_len = cur_len + addlen;

    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= cur_cap {
        return a;
    }
    if min_cap < 2 * cur_cap {
        min_cap = 2 * cur_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_block: *mut c_void = if a.is_null() {
        ptr::null_mut()
    } else {
        unsafe { header_of(a) as *mut c_void }
    };
    let total = elemsize * min_cap + core::mem::size_of::<stbds_array_header>();
    let raw = unsafe { c::realloc(old_block, total) };
    if raw.is_null() {
        // Match C's behavior: undefined on alloc failure; we just return null.
        return ptr::null_mut();
    }
    let new_a = unsafe { (raw as *mut u8).add(core::mem::size_of::<stbds_array_header>()) }
        as *mut c_void;
    unsafe {
        let h = header_of(new_a);
        if a.is_null() {
            (*h).length = 0;
            (*h).hash_table = ptr::null_mut();
            (*h).temp = 0;
        }
        (*h).capacity = min_cap;
    }
    new_a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    if a.is_null() {
        return;
    }
    unsafe { c::free(header_of(a) as *mut c_void) };
}

// ---------------------------------------------------------------------------
// stbds_hash_string
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    if n == 0 {
        val
    } else {
        (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
    }
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    if n == 0 {
        val
    } else {
        (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    unsafe {
        while *str != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*(str as *mut u8) as usize);
            str = str.add(1);
        }
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

// ---------------------------------------------------------------------------
// stbds_hash_bytes / siphash
// ---------------------------------------------------------------------------

#[inline]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = (*v0).wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = (*v2).wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = (*v2).wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = (*v0).wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;
    let mut data: usize;

    let c0: usize = (((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let c1: usize = (((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let c2: usize = (((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let c3: usize = (((0x74656462usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    let mut v0 = c0 ^ 0x0706050403020100usize ^ seed;
    let mut v1 = c1 ^ 0x0f0e0d0c0b0a0908usize ^ !seed;
    let mut v2 = c2 ^ 0x0706050403020100usize ^ seed;
    let mut v3 = c3 ^ 0x0f0e0d0c0b0a0908usize ^ !seed;

    let sz = core::mem::size_of::<usize>();
    let mut i: usize = 0;
    while i + sz <= len {
        unsafe {
            data = (*d.add(0)) as usize
                | ((*d.add(1) as usize) << 8)
                | ((*d.add(2) as usize) << 16)
                | ((*d.add(3) as usize) << 24);
            data |= (((*d.add(4) as usize)
                | ((*d.add(5) as usize) << 8)
                | ((*d.add(6) as usize) << 16)
                | ((*d.add(7) as usize) << 24))
                << 16)
                << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
            }
            v0 ^= data;

            i += sz;
            d = d.add(sz);
        }
    }
    data = len << (STBDS_SIZE_T_BITS - 8);
    let tail_len = len - i;
    unsafe {
        // Switch fallthrough — replicate C's case fall-through.
        if tail_len >= 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        if tail_len >= 6 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        if tail_len >= 5 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        if tail_len >= 4 {
            data |= (*d.add(3) as usize) << 24;
        }
        if tail_len >= 3 {
            data |= (*d.add(2) as usize) << 16;
        }
        if tail_len >= 2 {
            data |= (*d.add(1) as usize) << 8;
        }
        if tail_len >= 1 {
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
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// String arena (stralloc / strreset)
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = c::strlen(str) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;
            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);
            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block += 1;
            }
            if len > blocksize {
                let total = core::mem::size_of::<stbds_string_block>() - 8 + len;
                let sb = c::realloc(ptr::null_mut(), total) as *mut stbds_string_block;
                c::memmove(
                    (*sb).storage.as_mut_ptr() as *mut c_void,
                    str as *const c_void,
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
                let total = core::mem::size_of::<stbds_string_block>() - 8 + blocksize;
                let sb = c::realloc(ptr::null_mut(), total) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let p = (*(*a).storage)
            .storage
            .as_mut_ptr()
            .add((*a).remaining)
            .sub(len);
        (*a).remaining -= len;
        c::memmove(p as *mut c_void, str as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            c::free(x as *mut c_void);
            x = y;
        }
        c::memset(a as *mut c_void, 0, core::mem::size_of::<stbds_string_arena>());
    }
}

unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    unsafe {
        let len = c::strlen(s) + 1;
        let p = c::realloc(ptr::null_mut(), len) as *mut c_char;
        c::memmove(p as *mut c_void, s as *const c_void, len);
        p
    }
}

// ---------------------------------------------------------------------------
// Hash table internals.
// ---------------------------------------------------------------------------

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
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
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let total = (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
            + core::mem::size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1;
        let t = c::realloc(ptr::null_mut(), total) as *mut stbds_hash_index;
        let after = t.add(1) as usize;
        (*t).storage = align_fwd(after, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
        (*t).slot_count = slot_count;
        (*t).slot_count_log2 = stbds_log2(slot_count);
        (*t).tombstone_count = 0;
        (*t).used_count = 0;
        (*t).used_count_threshold = slot_count - (slot_count >> 2);
        (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
        (*t).used_count_shrink_threshold = slot_count >> 2;
        (*t).temp_key = ptr::null_mut();
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
            c::memset(
                &mut (*t).string as *mut _ as *mut c_void,
                0,
                core::mem::size_of::<stbds_string_arena>(),
            );
            (*t).seed = read_seed();
            update_seed_after_make_hash();
        }

        let nb = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..nb {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
        }

        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            let onb = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
            for i in 0..onb {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if index_in_use((*ob).index[j]) {
                        let hash = (*ob).hash[j];
                        let mut pos = probe_position(hash, (*t).slot_count);
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
                            for z in 0..limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'outer;
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
    unsafe {
        let p = (a as *mut u8).add(elemsize.wrapping_mul(i as usize) + keyoffset);
        if mode >= STBDS_HM_STRING {
            let stored = *(p as *mut *mut c_char);
            c::strcmp(key as *const c_char, stored as *const c_char) == 0
        } else {
            c::memcmp(key as *const c_void, p as *const c_void, keysize) == 0
        }
    }
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
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
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
                i += 1;
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    unsafe {
        let table = hash_table(a);
        if !table.is_null() {
            if (*table).string.mode == STBDS_SH_STRDUP {
                let len = (*header_of(a)).length;
                for i in 1..len {
                    let key_ptr = *((a as *mut u8).add(elemsize * i) as *mut *mut c_char);
                    c::free(key_ptr as *mut c_void);
                }
            }
            stbds_strreset(&mut (*table).string as *mut stbds_string_arena);
        }
        c::free((*header_of(a)).hash_table as *mut c_void);
        c::free(header_of(a) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a_in: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    unsafe {
        if a_in.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*header_of(a)).length += 1;
            c::memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            return arr_to_hash(a, elemsize);
        }
        let raw_a = hash_to_arr(a_in, elemsize);
        let table = (*header_of(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a_in, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a_in
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
        let h = header_of(hash_to_arr(p, elemsize));
        (*h).temp = temp;
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header_of(hash_to_arr(a, elemsize))).length == 0 {
            let raw = if a.is_null() {
                ptr::null_mut()
            } else {
                hash_to_arr(a, elemsize)
            };
            let new_a = stbds_arrgrowf(raw, elemsize, 0, 1);
            (*header_of(new_a)).length += 1;
            c::memset(new_a, 0, elemsize);
            return arr_to_hash(new_a, elemsize);
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a_in: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    unsafe {
        if a_in.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            c::memset(a, 0, elemsize);
            (*header_of(a)).length += 1;
            a_in = arr_to_hash(a, elemsize);
        }
        let mut raw_a = a_in;
        let mut a = hash_to_arr(a_in, elemsize);
        let mut table = (*header_of(a)).hash_table as *mut stbds_hash_index;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count * 2
            };
            let nt = stbds_make_hash_index(slot_count, table);
            if !table.is_null() {
                c::free(table as *mut c_void);
            } else if mode >= STBDS_HM_STRING {
                (*nt).string.mode = STBDS_SH_DEFAULT;
            } else {
                (*nt).string.mode = STBDS_SH_NONE;
            }
            (*header_of(a)).hash_table = nt as *mut c_void;
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
                        (*header_of(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            // stbds_temp_key(a) = *(char**)(raw_a + elemsize*idx + keyoffset)
                            let stored_key = *((raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char);
                            *((*header_of(a)).hash_table as *mut *mut c_char) = stored_key;
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    final_pos = pos;
                    break 'find;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
                i += 1;
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
                        (*header_of(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    final_pos = pos;
                    break 'find;
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

        let i = arrlen(a);
        if (i as usize) + 1 > arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        let _ = raw_a;
        raw_a = arr_to_hash(a, elemsize);
        let _ = raw_a;

        (*header_of(a)).length = (i as usize) + 1;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*header_of(a)).temp = i - 1;

        let target = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
        match (*table).string.mode {
            STBDS_SH_STRDUP => {
                let p = stbds_strdup(key as *mut c_char);
                *target = p;
                *((*header_of(a)).hash_table as *mut *mut c_char) = p;
            }
            STBDS_SH_ARENA => {
                let p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                *target = p;
                *((*header_of(a)).hash_table as *mut *mut c_char) = p;
            }
            STBDS_SH_DEFAULT => {
                *target = key as *mut c_char;
                *((*header_of(a)).hash_table as *mut *mut c_char) = key as *mut c_char;
            }
            _ => {
                c::memcpy(
                    (a as *mut u8).add(elemsize * (i as usize)) as *mut c_void,
                    key as *const c_void,
                    keysize,
                );
            }
        }
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        c::memset(a, 0, elemsize);
        (*header_of(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*h).string.mode = mode as u8;
        (*header_of(a)).hash_table = h as *mut c_void;
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a_in: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    if a_in.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let raw_a = hash_to_arr(a_in, elemsize);
        let table = (*header_of(raw_a)).hash_table as *mut stbds_hash_index;
        (*header_of(raw_a)).temp = 0;
        if table.is_null() {
            return a_in;
        }
        let mut slot = stbds_hm_find_slot(a_in, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a_in;
        }
        let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = arrlen(raw_a) - 1 - 1;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header_of(raw_a)).temp = 1;
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            let p = *((a_in as *mut u8).add(elemsize * (old_index as usize)) as *mut *mut c_char);
            c::free(p as *mut c_void);
        }

        if old_index != final_index {
            c::memmove(
                (a_in as *mut u8).add(elemsize * (old_index as usize)) as *mut c_void,
                (a_in as *mut u8).add(elemsize * (final_index as usize)) as *const c_void,
                elemsize,
            );
            if mode == STBDS_HM_STRING {
                let kp = *((a_in as *mut u8).add(elemsize * (old_index as usize) + keyoffset)
                    as *mut *mut c_char);
                slot = stbds_hm_find_slot(a_in, elemsize, kp as *mut c_void, keysize, keyoffset, mode);
            } else {
                let kp = (a_in as *mut u8).add(elemsize * (old_index as usize) + keyoffset);
                slot = stbds_hm_find_slot(a_in, elemsize, kp as *mut c_void, keysize, keyoffset, mode);
            }
            b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            (*b).index[i] = old_index;
        }
        (*header_of(raw_a)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > STBDS_BUCKET_LENGTH
        {
            (*header_of(raw_a)).hash_table =
                stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
            c::free(table as *mut c_void);
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            (*header_of(raw_a)).hash_table =
                stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
            c::free(table as *mut c_void);
        }

        a_in
    }
}

// ---------------------------------------------------------------------------
// strkey
// ---------------------------------------------------------------------------
//
// `static char buffer[256]; char *strkey(int n) { sprintf(buffer, "test_%d", n); return buffer; }`
//
// We must export the same identical buffer behavior so callers see
// `"test_%d"` formatting.  Use libc sprintf to keep the formatting identical
// to the C version.

const STRKEY_BUFFER_LEN: usize = 256;

#[repr(C)]
struct StrkeyBuffer([c_char; STRKEY_BUFFER_LEN]);

unsafe impl Sync for StrkeyBuffer {}
unsafe impl Send for StrkeyBuffer {}

static mut STRKEY_BUFFER: StrkeyBuffer = StrkeyBuffer([0; STRKEY_BUFFER_LEN]);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut STRKEY_BUFFER) as *mut c_char;
        let fmt = b"test_%d\0".as_ptr() as *const c_char;
        c::sprintf(buf, fmt, n);
        buf
    }
}

// ---------------------------------------------------------------------------
// helxo — high-level hello driver mimicking the C implementation.
// ---------------------------------------------------------------------------
//
// The C function constructs a string-keyed hashmap with five fixed entries,
// then writes letter into the "jen" slot via the local stack buffer `name`,
// and finally walks the entries in array order printing
// `printf("%s %c\n", hash[z], hash[z].value)`.  Note: stb_ds preserves
// insertion order for non-deleted keys, so the iteration prints in the
// order of the original five `shput` calls (with "jen" still at slot 4).
//
// The fifth `shput(hash, name, letter)` reuses an existing key, so it
// overwrites the value at "jen" without changing position.
//
// Note that `printf("%s ...", hash[z], hash[z].value)` passes `hash[z]`
// (the entry struct) where a `char*` is expected — this is undefined
// behaviour in C but on this build the struct's first field is the char*
// key pointer, and the variadic call passes that pointer in the first
// argument register, so the actual observed output is the key string.
// We faithfully reproduce that observed output.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    use std::io::Write;

    let entries: [(&[u8], c_char); 5] = [
        (b"bob", b'h' as c_char),
        (b"sally", b'e' as c_char),
        (b"fred", b'l' as c_char),
        (b"jen", letter),
        (b"doug", b'o' as c_char),
    ];

    // Use direct fd-1 writes so the output is captured by tests that
    // redirect stdout via dup2() — Rust's `io::stdout()` would otherwise
    // miss writes that happened before redirection because of buffering.
    let mut out = unsafe { std::fs::File::from_raw_fd(libc::dup(1)) };
    for (key, value) in entries.iter() {
        let _ = out.write_all(key);
        let _ = out.write_all(b" ");
        let byte = ((*value as i32) & 0xff) as u8;
        let _ = out.write_all(&[byte]);
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}

use std::os::unix::io::FromRawFd;
