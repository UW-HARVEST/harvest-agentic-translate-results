//! Rust translation of `c_src/src/lib.c` (an excerpt of stb_ds.h plus the
//! `arr_ins` unit-test helper).
//!
//! The translation is deliberately literal: memory layouts, allocation calls,
//! integer wrap-around and even the latent sign-extension quirks of the
//! original C are reproduced so that observable behaviour is byte-identical.

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// libc bindings (STBDS_REALLOC / STBDS_FREE map onto realloc / free)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn abort() -> !;
}

/// `STBDS_ASSERT` is `assert` in the C source (NDEBUG is not defined), i.e. a
/// failing check terminates the process with SIGABRT.
fn stbds_assert_fail() -> ! {
    unsafe { abort() }
}

macro_rules! stbds_assert {
    ($cond:expr) => {
        if !($cond) {
            stbds_assert_fail()
        }
    };
}

/// `STBDS_REALLOC(NULL, p, s)` == `realloc(p, s)`
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}

/// `STBDS_FREE(NULL, p)` == `free(p)`
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

// ---------------------------------------------------------------------------
// Data structures (layouts must match the C definitions exactly)
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

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

// ---------------------------------------------------------------------------
// Array header accessors (the `stbds_header` / `stbds_arrlen` ... macros)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).sub(1) }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `stbds_temp(t)` -- lvalue access to the array header's `temp` field.
#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

/// `stbds_temp_key(t)` -- `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) = v }
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
// Small C string / memory helpers
// ---------------------------------------------------------------------------

unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// `0 == strcmp(a, b)`
unsafe fn c_str_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut i = 0usize;
    unsafe {
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
}

/// `0 == memcmp(a, b, n)`
unsafe fn c_mem_eq(a: *const c_void, b: *const c_void, n: usize) -> bool {
    unsafe {
        std::slice::from_raw_parts(a as *const u8, n) == std::slice::from_raw_parts(b as *const u8, n)
    }
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
    unsafe {
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
            if !a.is_null() {
                stbds_header(a) as *mut c_void
            } else {
                ptr::null_mut()
            },
            elemsize
                .wrapping_mul(min_cap)
                .wrapping_add(size_of::<stbds_array_header>()),
        );
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
    unsafe { stbds_free(stbds_header(a) as *mut c_void) }
}

// ---------------------------------------------------------------------------
// Hash seed / probing helpers
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { STBDS_HASH_SEED = seed }
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

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

// ---------------------------------------------------------------------------
// stbds_make_hash_index
// ---------------------------------------------------------------------------

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
        stbds_assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

        if !ot.is_null() {
            (*t).string = stbds_string_arena {
                storage: (*ot).string.storage,
                remaining: (*ot).string.remaining,
                block: (*ot).string.block,
                mode: (*ot).string.mode,
            };
            (*t).seed = (*ot).seed;
        } else {
            ptr::write_bytes(&raw mut (*t).string as *mut u8, 0, size_of::<stbds_string_arena>());
            (*t).seed = STBDS_HASH_SEED;

            // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let mut temp: usize;
            let mut a: usize;
            temp = (0x87b0_b0fdu32 ^ 2147001325u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            a = 0x27bb_2ee6usize;
            a <<= 16;
            a <<= 16;
            a ^= temp ^ 2147001325usize;

            // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d);
            let mut b: usize;
            temp = (0xb504_f32du32 ^ 715136305u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            b = 0usize;
            b <<= 16;
            b <<= 16;
            b ^= temp ^ 715136305usize;

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
                            let mut z = 0usize;
                            while z < limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'outer;
                                }
                                z += 1;
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

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;

        let mut v0 = ((((0x736f_6d65usize) << 16) << 16) + 0x7073_6575) ^ seed;
        let mut v1 = ((((0x646f_7261usize) << 16) << 16) + 0x6e64_6f6d) ^ !seed;
        let mut v2 = ((((0x6c79_6765usize) << 16) << 16) + 0x6e65_7261) ^ seed;
        let mut v3 = ((((0x7465_6462usize) << 16) << 16) + 0x7974_6573) ^ !seed;

        v0 ^= 0x0706_0504_0302_0100usize ^ seed;
        v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
        v2 ^= 0x0706_0504_0302_0100usize ^ seed;
        v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

        macro_rules! sipround {
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

        let mut data: usize;
        let mut i = 0usize;
        while i + size_of::<usize>() <= len {
            // NOTE: reproduces the C expression exactly, including the
            // `d[3] << 24` int overflow whose result is sign-extended when
            // assigned to size_t.
            let lo = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | (*d.add(3) as i32).wrapping_shl(24);
            data = lo as isize as usize;
            let hi = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | (*d.add(7) as i32).wrapping_shl(24);
            data |= ((hi as isize as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                sipround!();
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        // switch (len - i) with fall-through from case 7 down to case 1
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
            data |= (*d.add(3) as i32).wrapping_shl(24) as isize as usize;
        }
        if rem >= 3 {
            data |= ((*d.add(2) as i32) << 16) as isize as usize;
        }
        if rem >= 2 {
            data |= ((*d.add(1) as i32) << 8) as isize as usize;
        }
        if rem >= 1 {
            data |= (*d.add(0) as i32) as isize as usize;
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
    i: isize,
) -> bool {
    unsafe {
        let slot = (a as *mut u8)
            .offset((elemsize as isize).wrapping_mul(i))
            .add(keyoffset);
        if mode >= STBDS_HM_STRING {
            c_str_eq(key as *const c_char, *(slot as *mut *mut c_char))
        } else {
            c_mem_eq(key, slot as *const c_void, keysize)
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
                    stbds_free(
                        *((a as *mut u8).add(elemsize * i) as *mut *mut c_char) as *mut c_void,
                    );
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
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
                    let b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
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
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &raw mut temp, mode);
        stbds_temp_set(stbds_hash_to_arr(p, elemsize), temp);
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
                    STBDS_SH_NONE
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
                hash += 2;
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
                            (*bucket).index[i],
                        ) {
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                stbds_temp_key_set(
                                    a,
                                    *((raw_a as *mut u8)
                                        .offset((elemsize as isize) * (*bucket).index[i])
                                        .add(keyoffset) as *mut *mut c_char),
                                );
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
                            (*bucket).index[i],
                        ) {
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
                pos &= (*table).slot_count - 1;
            }

            // found_empty_slot:
            if tombstone >= 0 {
                pos = tombstone as usize;
                (*table).tombstone_count -= 1;
            }
            (*table).used_count += 1;

            {
                let i: isize = stbds_arrlen(a);
                if (i as usize) + 1 > stbds_arrcap(a) {
                    a = stbds_arrgrowf(a, elemsize, 1, 0);
                }
                raw_a = stbds_arr_to_hash(a, elemsize);
                let _ = raw_a;

                stbds_assert!((i as usize) + 1 <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let key_slot = (a as *mut u8).offset((elemsize as isize) * i);
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *(key_slot as *mut *mut c_char) = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                        *(key_slot as *mut *mut c_char) = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *(key_slot as *mut *mut c_char) = p;
                        stbds_temp_key_set(a, p);
                    }
                    _ => {
                        ptr::copy_nonoverlapping(key as *const u8, key_slot, keysize);
                    }
                }
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
        stbds_temp_set(raw_a, 0);
        if table.is_null() {
            return a;
        }

        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }

        let mut b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
        let mut i: c_int = (slot & STBDS_BUCKET_MASK as isize) as c_int;
        let old_index = (*b).index[i as usize];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        stbds_assert!(slot < (*table).slot_count as isize);
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        stbds_temp_set(raw_a, 1);
        (*b).hash[i as usize] = STBDS_HASH_DELETED;
        (*b).index[i as usize] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            stbds_free(
                *((a as *mut u8).offset((elemsize as isize) * old_index) as *mut *mut c_char)
                    as *mut c_void,
            );
        }

        if old_index != final_index {
            ptr::copy(
                (a as *const u8).offset((elemsize as isize) * final_index),
                (a as *mut u8).offset((elemsize as isize) * old_index),
                elemsize,
            );

            if mode == STBDS_HM_STRING {
                slot = stbds_hm_find_slot(
                    a,
                    elemsize,
                    *((a as *mut u8)
                        .offset((elemsize as isize) * old_index)
                        .add(keyoffset) as *mut *mut c_char) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                );
            } else {
                slot = stbds_hm_find_slot(
                    a,
                    elemsize,
                    (a as *mut u8)
                        .offset((elemsize as isize) * old_index)
                        .add(keyoffset) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                );
            }
            stbds_assert!(slot >= 0);
            b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
            i = (slot & STBDS_BUCKET_MASK as isize) as c_int;
            stbds_assert!((*b).index[i as usize] == final_index);
            (*b).index[i as usize] = old_index;
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
}

// ---------------------------------------------------------------------------
// String pool
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = c_strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
        p
    }
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = c_strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block += 1;
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                ptr::copy(
                    str_ as *const u8,
                    (&raw mut (*sb).storage) as *mut c_char as *mut u8,
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
                return (&raw mut (*sb).storage) as *mut c_char;
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

        stbds_assert!(len <= (*a).remaining);
        let p = ((&raw mut (*(*a).storage).storage) as *mut c_char).add((*a).remaining - len);
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
// Test helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let s = format!("test_{}\0", n);
        let dst = (&raw mut BUFFER) as *mut c_char;
        ptr::copy_nonoverlapping(s.as_ptr(), dst as *mut u8, s.len());
        dst
    }
}

// --- int-array macro expansions used by arr_ins ----------------------------

/// `stbds_arrmaybegrow(a, n)`
unsafe fn arr_maybegrow(a: &mut *mut c_int, n: usize) {
    unsafe {
        let p = *a as *mut c_void;
        if p.is_null() || (*stbds_header(p)).length + n > (*stbds_header(p)).capacity {
            *a = stbds_arrgrowf(p, size_of::<c_int>(), n, 0) as *mut c_int;
        }
    }
}

/// `stbds_arrput(a, v)`
unsafe fn arr_put(a: &mut *mut c_int, v: c_int) {
    unsafe {
        arr_maybegrow(a, 1);
        let h = stbds_header(*a as *mut c_void);
        let len = (*h).length;
        *(*a).add(len) = v;
        (*h).length = len + 1;
    }
}

/// `stbds_arraddn(a, n)` (via stbds_arraddnindex)
unsafe fn arr_addn(a: &mut *mut c_int, n: usize) {
    unsafe {
        arr_maybegrow(a, n);
        if n != 0 {
            let h = stbds_header(*a as *mut c_void);
            (*h).length += n;
        }
    }
}

/// `stbds_arrinsn(a, i, n)`
unsafe fn arr_insn(a: &mut *mut c_int, i: usize, n: usize) {
    unsafe {
        arr_addn(a, n);
        let h = stbds_header(*a as *mut c_void);
        let count = (*h).length - n - i;
        ptr::copy(
            (*a).add(i) as *const c_int,
            (*a).add(i + n),
            count,
        );
    }
}

/// `stbds_arrins(a, i, v)`
unsafe fn arr_ins_at(a: &mut *mut c_int, i: usize, v: c_int) {
    unsafe {
        arr_insn(a, i, 1);
        *(*a).add(i) = v;
    }
}

/// `stbds_arrfree(a)`
unsafe fn arr_free(a: &mut *mut c_int) {
    unsafe {
        if !(*a).is_null() {
            stbds_free(stbds_header(*a as *mut c_void) as *mut c_void);
        }
        *a = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    unsafe {
        let mut arr: *mut c_int = ptr::null_mut();

        for i in 0..5usize {
            arr_put(&mut arr, 1);
            arr_put(&mut arr, 2);
            arr_put(&mut arr, 3);
            arr_put(&mut arr, 4);
            arr_ins_at(&mut arr, i, num);
            stbds_assert!(*arr.add(i) == num);
            if i < 4 {
                stbds_assert!(*arr.add(4) == 4);
            }
            arr_free(&mut arr);
        }
    }
}
