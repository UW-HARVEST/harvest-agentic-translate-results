//! Rust translation of `c_src/src/lib.c` (a vendored copy of stb_ds.h plus the
//! `helxo` demo entry point).
//!
//! The translation is deliberately literal: the data structures keep the exact
//! C memory layout, the allocation strategy goes through libc `realloc`/`free`
//! just like the original `STBDS_REALLOC`/`STBDS_FREE` macros, and the integer
//! arithmetic reproduces the original wrapping / sign-extension behaviour
//! (including the places where the C code relies on implementation defined
//! conversions). Output is emitted through libc `printf` so buffering and byte
//! output match the C library exactly.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

/// `STBDS_REALLOC(NULL, p, s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}

/// `STBDS_FREE(NULL, p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

// ---------------------------------------------------------------------------
// Layout-compatible structures
// ---------------------------------------------------------------------------

#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

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

/// `STBDS_HM_BINARY`. Not referenced by `helxo` (which only uses the string
/// hash map) but part of the public `stbds_*` calling convention.
#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

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
    (n.wrapping_add(a).wrapping_sub(1)) & !(a - 1)
}

// ---------------------------------------------------------------------------
// Array helpers (the `stbds_header` / `stbds_arrlen` / ... macro family)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).offset(-1) }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

/// `stbds_temp(t)`
#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> *mut isize {
    unsafe { ptr::addr_of_mut!((*stbds_header(t)).temp) }
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key(t: *mut c_void) -> *mut *mut c_char {
    unsafe { (*stbds_header(t)).hash_table as *mut *mut c_char }
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut c_char).sub(elemsize) as *mut c_void }
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut c_char).add(elemsize) as *mut c_void }
}

/// `stbds_hash_table(a)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
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
    unsafe {
        let mut min_cap = min_cap;
        let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

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

        let raw = stbds_realloc(
            if a.is_null() {
                ptr::null_mut()
            } else {
                stbds_header(a) as *mut c_void
            },
            elemsize
                .wrapping_mul(min_cap)
                .wrapping_add(size_of::<stbds_array_header>()),
        );
        let b = (raw as *mut c_char).add(size_of::<stbds_array_header>()) as *mut c_void;
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
    unsafe { stbds_free(stbds_header(a) as *mut c_void) }
}

// ---------------------------------------------------------------------------
// Hash seed / hash index construction
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { STBDS_HASH_SEED = seed }
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

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
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
        let t = stbds_realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
                + size_of::<stbds_hash_index>()
                + STBDS_CACHE_LINE_SIZE
                - 1,
        ) as *mut stbds_hash_index;

        (*t).storage = stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE)
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
        assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

        if !ot.is_null() {
            ptr::copy_nonoverlapping(
                ptr::addr_of!((*ot).string),
                ptr::addr_of_mut!((*t).string),
                1,
            );
            (*t).seed = (*ot).seed;
        } else {
            ptr::write_bytes(
                ptr::addr_of_mut!((*t).string) as *mut u8,
                0,
                size_of::<stbds_string_arena>(),
            );
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
                        'done: loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

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
                                break 'done;
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
    unsafe {
        let mut hash = seed;
        let mut s = str_;
        while *s != 0 {
            hash = stbds_rotate_left(hash, 9).wrapping_add(*s as u8 as usize);
            s = s.add(1);
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
}

/// Reproduces the C expression `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)`
/// where each `d[i]` is promoted to `int`: the result is a (possibly negative)
/// `int` which is then sign extended when converted to `size_t`.
#[inline]
unsafe fn stbds_load_le32_as_int(d: *const u8) -> c_int {
    unsafe {
        (*d.add(0) as c_int)
            | ((*d.add(1) as c_int) << 8)
            | ((*d.add(2) as c_int) << 16)
            | ((*d.add(3) as c_int).wrapping_shl(24))
    }
}

#[inline]
fn stbds_int_to_size(v: c_int) -> usize {
    v as isize as usize
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;
        let mut data: usize;

        let mut v0 = ((((0x736f_6d65usize) << 16) << 16).wrapping_add(0x7073_6575)) ^ seed;
        let mut v1 = ((((0x646f_7261usize) << 16) << 16).wrapping_add(0x6e64_6f6d)) ^ !seed;
        let mut v2 = ((((0x6c79_6765usize) << 16) << 16).wrapping_add(0x6e65_7261)) ^ seed;
        let mut v3 = ((((0x7465_6462usize) << 16) << 16).wrapping_add(0x7974_6573)) ^ !seed;

        v0 ^= 0x0706_0504_0302_0100u64 as usize ^ seed;
        v1 ^= 0x0f0e_0d0c_0b0a_0908u64 as usize ^ !seed;
        v2 ^= 0x0706_0504_0302_0100u64 as usize ^ seed;
        v3 ^= 0x0f0e_0d0c_0b0a_0908u64 as usize ^ !seed;

        macro_rules! siphash_round {
            () => {{
                v0 = v0.wrapping_add(v1);
                v1 = stbds_rotate_left(v1, 13);
                v1 ^= v0;
                v0 = stbds_rotate_left(v0, STBDS_SIZE_T_BITS / 2);
                v2 = v2.wrapping_add(v3);
                v3 = stbds_rotate_left(v3, 16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = stbds_rotate_left(v1, 17);
                v1 ^= v2;
                v2 = stbds_rotate_left(v2, STBDS_SIZE_T_BITS / 2);
                v0 = v0.wrapping_add(v3);
                v3 = stbds_rotate_left(v3, 21);
                v3 ^= v0;
            }};
        }

        let mut i = 0usize;
        while i + size_of::<usize>() <= len {
            data = stbds_int_to_size(stbds_load_le32_as_int(d));
            data |= (stbds_int_to_size(stbds_load_le32_as_int(d.add(4))) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round!();
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        // Fall-through switch: every case from `len - i` down to 1 executes.
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
            data |= stbds_int_to_size((*d.add(3) as c_int).wrapping_shl(24));
        }
        if rem >= 3 {
            data |= stbds_int_to_size((*d.add(2) as c_int) << 16);
        }
        if rem >= 2 {
            data |= stbds_int_to_size((*d.add(1) as c_int) << 8);
        }
        if rem >= 1 {
            data |= stbds_int_to_size(*d.add(0) as c_int);
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round!();
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            siphash_round!();
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
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
    unsafe {
        let slot = (a as *mut c_char).add(elemsize.wrapping_mul(i)).add(keyoffset);
        if mode >= STBDS_HM_STRING {
            (0 == strcmp(key as *const c_char, *(slot as *mut *const c_char))) as c_int
        } else {
            let x = key as *const u8;
            let y = slot as *const u8;
            let mut n = 0usize;
            while n < keysize {
                if *x.add(n) != *y.add(n) {
                    return 0;
                }
                n += 1;
            }
            1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        if !stbds_hash_table(a).is_null() {
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
                let mut i = 1usize;
                while i < (*stbds_header(a)).length {
                    stbds_free(*((a as *mut c_char).add(elemsize * i) as *mut *mut c_void));
                    i += 1;
                }
            }
            stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));
        }
        stbds_free((*stbds_header(a)).hash_table);
        stbds_free(stbds_header(a) as *mut c_void);
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
            pos &= (*table).slot_count - 1;
        }
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
    unsafe {
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
                    *temp = (*b).index[slot as usize & STBDS_BUCKET_MASK];
                }
            }
            a
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
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
                    0
                };
            }
            table = nt;
            (*stbds_header(a)).hash_table = table as *mut c_void;
        }

        {
            let mut hash = if mode >= STBDS_HM_STRING {
                stbds_hash_string(key as *mut c_char, (*table).seed)
            } else {
                stbds_hash_bytes(key, keysize, (*table).seed)
            };
            let mut step = STBDS_BUCKET_LENGTH;
            let mut tombstone: isize = -1;
            let mut bucket: *mut stbds_hash_bucket;

            if hash < 2 {
                hash = hash.wrapping_add(2);
            }

            let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

            'found_empty_slot: loop {
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
                            (*bucket).index[i] as usize,
                        ) != 0
                        {
                            *stbds_temp(a) = (*bucket).index[i];
                            if mode >= STBDS_HM_STRING {
                                *stbds_temp_key(a) = *((raw_a as *mut c_char)
                                    .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                    .add(keyoffset) as *mut *mut c_char);
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
                let mut empty_at: Option<usize> = None;
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
                            *stbds_temp(a) = (*bucket).index[i];
                            return stbds_arr_to_hash(a, elemsize);
                        }
                    } else if (*bucket).hash[i] == 0 {
                        empty_at = Some(i);
                        break;
                    } else if tombstone < 0 {
                        if (*bucket).index[i] == STBDS_INDEX_DELETED {
                            tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                        }
                    }
                    i += 1;
                }
                if let Some(i) = empty_at {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'found_empty_slot;
                }

                pos = pos.wrapping_add(step);
                step += STBDS_BUCKET_LENGTH;
                pos &= (*table).slot_count - 1;
            }

            // found_empty_slot:
            if tombstone >= 0 {
                pos = tombstone as usize;
                (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
            }
            (*table).used_count += 1;

            {
                let i = stbds_arrlen(a);
                if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                    a = stbds_arrgrowf(a, elemsize, 1, 0);
                }
                raw_a = stbds_arr_to_hash(a, elemsize);

                assert!((i as usize) + 1 <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                *stbds_temp(a) = i - 1;

                let dst = (a as *mut c_char).add(elemsize.wrapping_mul(i as usize));
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *(dst as *mut *mut c_char) = p;
                        *stbds_temp_key(a) = p;
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(
                            ptr::addr_of_mut!((*table).string),
                            key as *mut c_char,
                        );
                        *(dst as *mut *mut c_char) = p;
                        *stbds_temp_key(a) = p;
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *(dst as *mut *mut c_char) = p;
                        *stbds_temp_key(a) = p;
                    }
                    _ => {
                        ptr::copy_nonoverlapping(key as *const u8, dst as *mut u8, keysize);
                    }
                }
                let _ = raw_a;
            }
            stbds_arr_to_hash(a, elemsize)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        stbds_arr_to_hash(a, elemsize)
    }
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

        let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i = (slot as usize & STBDS_BUCKET_MASK) as c_int;
        let old_index = (*b).index[i as usize];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        assert!(slot < (*table).slot_count as isize);
        (*table).used_count = (*table).used_count.wrapping_sub(1);
        (*table).tombstone_count += 1;
        *stbds_temp(raw_a) = 1;
        b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        (*b).hash[i as usize] = STBDS_HASH_DELETED;
        (*b).index[i as usize] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            stbds_free(*((a as *mut c_char).add(elemsize.wrapping_mul(old_index as usize))
                as *mut *mut c_void));
        }

        if old_index != final_index {
            ptr::copy(
                (a as *const c_char).add(elemsize.wrapping_mul(final_index as usize)) as *const u8,
                (a as *mut c_char).add(elemsize.wrapping_mul(old_index as usize)) as *mut u8,
                elemsize,
            );

            let moved = (a as *mut c_char)
                .add(elemsize.wrapping_mul(old_index as usize))
                .add(keyoffset);
            slot = if mode == STBDS_HM_STRING {
                stbds_hm_find_slot(
                    a,
                    elemsize,
                    *(moved as *mut *mut c_char) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                )
            } else {
                stbds_hm_find_slot(a, elemsize, moved as *mut c_void, keysize, keyoffset, mode)
            };
            assert!(slot >= 0);
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = (slot as usize & STBDS_BUCKET_MASK) as c_int;
            assert!((*b).index[i as usize] == final_index);
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

// ---------------------------------------------------------------------------
// String storage
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
        p
    }
}

/// Offset of `stbds_string_block::storage`, i.e. `sizeof(*sb) - 8`.
const STRING_BLOCK_STORAGE_OFFSET: usize = size_of::<stbds_string_block>() - 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block += 1;
            }

            if len > blocksize {
                let sb = stbds_realloc(ptr::null_mut(), STRING_BLOCK_STORAGE_OFFSET + len)
                    as *mut stbds_string_block;
                let storage = ptr::addr_of_mut!((*sb).storage) as *mut c_char;
                ptr::copy(str_ as *const u8, storage as *mut u8, len);
                if !(*a).storage.is_null() {
                    (*sb).next = (*(*a).storage).next;
                    (*(*a).storage).next = sb;
                } else {
                    (*sb).next = ptr::null_mut();
                    (*a).storage = sb;
                    (*a).remaining = 0;
                }
                return storage;
            } else {
                let sb = stbds_realloc(ptr::null_mut(), STRING_BLOCK_STORAGE_OFFSET + blocksize)
                    as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        assert!(len <= (*a).remaining);
        let p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
            .add((*a).remaining)
            .sub(len);
        (*a).remaining -= len;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            stbds_free(x as *mut c_void);
            x = y;
        }
        ptr::write_bytes(a as *mut u8, 0, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// strkey / helxo
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = ptr::addr_of_mut!(BUFFER) as *mut c_char;
        sprintf(buf, c"test_%d".as_ptr(), n);
        buf
    }
}

/// The anonymous struct used by `helxo`: `struct { char *key; char value; }`.
#[repr(C)]
#[derive(Copy, Clone)]
struct helxo_entry {
    key: *mut c_char,
    value: c_char,
}

const HELXO_ELEMSIZE: usize = size_of::<helxo_entry>();
const HELXO_KEYSIZE: usize = size_of::<*mut c_char>();

/// `shput(hash, k, v)`
#[inline]
unsafe fn helxo_shput(hash: &mut *mut helxo_entry, key: *mut c_char, value: c_char) {
    unsafe {
        *hash = stbds_hmput_key(
            *hash as *mut c_void,
            HELXO_ELEMSIZE,
            key as *mut c_void,
            HELXO_KEYSIZE,
            STBDS_HM_STRING,
        ) as *mut helxo_entry;
        let temp = *stbds_temp((*hash).sub(1) as *mut c_void);
        (*(*hash).offset(temp)).value = value;
    }
}

/// `shlen(hash)`
#[inline]
unsafe fn helxo_shlen(hash: *mut helxo_entry) -> isize {
    unsafe {
        if hash.is_null() {
            0
        } else {
            (*stbds_header(hash.sub(1) as *mut c_void)).length as isize - 1
        }
    }
}

/// `shfree(hash)`
#[inline]
unsafe fn helxo_shfree(hash: &mut *mut helxo_entry) {
    unsafe {
        if !hash.is_null() {
            stbds_hmfree_func((*hash).sub(1) as *mut c_void, HELXO_ELEMSIZE);
        }
        *hash = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    unsafe {
        let mut hash: *mut helxo_entry = ptr::null_mut();
        // char name[4] = "jen";
        let mut name: [c_char; 4] = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];

        helxo_shput(&mut hash, c"bob".as_ptr() as *mut c_char, b'h' as c_char);
        helxo_shput(&mut hash, c"sally".as_ptr() as *mut c_char, b'e' as c_char);
        helxo_shput(&mut hash, c"fred".as_ptr() as *mut c_char, b'l' as c_char);
        helxo_shput(&mut hash, c"jen".as_ptr() as *mut c_char, b'x' as c_char);
        helxo_shput(&mut hash, c"doug".as_ptr() as *mut c_char, b'o' as c_char);

        helxo_shput(&mut hash, name.as_mut_ptr(), letter);

        let mut z: c_int = 0;
        while (z as isize) < helxo_shlen(hash) {
            // The C code passes the whole struct where `printf` expects a
            // `char *` for `%s` and then reads the following vararg slot for
            // `%c`. Under the SysV x86-64 ABI the 16-byte struct occupies two
            // integer argument slots, so `%s` consumes `key` and `%c` consumes
            // the eightbyte whose low byte is `value`; the explicitly passed
            // `hash[z].value` argument is never read.
            let e = *hash.offset(z as isize);
            printf(c"%s %c\n".as_ptr(), e.key, e.value as c_int);
            z += 1;
        }

        helxo_shfree(&mut hash);
    }
}
