//! Rust translation of the C library in `c_src/` (a trimmed copy of stb_ds.h
//! plus the `strkey` / `str_put` helpers from its unit-test section).
//!
//! The translation is deliberately literal: every arithmetic quirk of the C
//! code (implicit `int` promotions that sign-extend into `size_t`, wrapping
//! multiplies, the `printf` call that passes a whole struct for `%s`, ...) is
//! reproduced so the shared library is bit-for-bit compatible with the C one.
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// libc bindings (kept so that memory blocks and stdio state are shared with C)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// STBDS_REALLOC(c,p,s) -> realloc(p,s) ; STBDS_FREE(c,p) -> free(p)
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}

#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

#[inline]
unsafe fn c_memset(dst: *mut u8, val: u8, n: usize) {
    unsafe { ptr::write_bytes(dst, val, n) }
}

#[inline]
unsafe fn c_memmove(dst: *mut u8, src: *const u8, n: usize) {
    unsafe { ptr::copy(src, dst, n) }
}

// ---------------------------------------------------------------------------
// Data structures (layout-identical to the C ones)
// ---------------------------------------------------------------------------

/// `stbds_array_header`
#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

/// `stbds_string_block`
#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

/// `struct stbds_string_arena`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

/// `stbds_hash_bucket`
#[repr(C)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

/// `stbds_hash_index`
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

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

// enum { STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA }
const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = 64;
const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>(); // 32

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// Layout checks against the C structs (x86-64 SysV: 32/16/24/128/104 bytes).
const _: () = {
    use core::mem::{align_of, size_of};
    assert!(size_of::<stbds_array_header>() == 32 && align_of::<stbds_array_header>() == 8);
    assert!(size_of::<stbds_string_block>() == 16);
    assert!(size_of::<stbds_string_arena>() == 24);
    assert!(size_of::<stbds_hash_bucket>() == 128);
    assert!(size_of::<stbds_hash_index>() == 104);
    assert!(size_of::<usize>() == 8, "STBDS_SIPHASH_2_4 requires a 64-bit build");
};

// ---------------------------------------------------------------------------
// Macro helpers
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *)(t) - 1)`
#[inline]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).offset(-1) }
}

/// `stbds_arrlen(a)`
#[inline]
unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

/// `stbds_arrcap(a)`
#[inline]
unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `stbds_temp(t)` (read)
#[inline]
unsafe fn stbds_temp_get(t: *mut u8) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

/// `stbds_temp(t) = v`
#[inline]
unsafe fn stbds_temp_set(t: *mut u8, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

/// `stbds_temp_key(t)` == `(*(char **) stbds_header(t)->hash_table)` (write)
#[inline]
unsafe fn stbds_temp_key_set(t: *mut u8, v: *mut c_char) {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) = v }
}

/// `stbds_hash_table(a)`
#[inline]
unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `STBDS_HASH_TO_ARR(x,elemsize)`
#[inline]
unsafe fn hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.sub(elemsize) }
}

/// `STBDS_ARR_TO_HASH(x,elemsize)`
#[inline]
unsafe fn arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.add(elemsize) }
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
        let a = a as *mut u8;
        let mut min_cap = min_cap;
        let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

        if min_len > min_cap {
            min_cap = min_len;
        }

        if min_cap <= stbds_arrcap(a) {
            return a as *mut c_void;
        }

        let cap2 = stbds_arrcap(a).wrapping_mul(2);
        if min_cap < cap2 {
            min_cap = cap2;
        } else if min_cap < 4 {
            min_cap = 4;
        }

        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_header(a) as *mut c_void
        };
        let raw = stbds_realloc(
            old,
            elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
        );
        let b = (raw as *mut u8).add(HEADER_SIZE);
        if a.is_null() {
            (*stbds_header(b)).length = 0;
            (*stbds_header(b)).hash_table = ptr::null_mut();
            (*stbds_header(b)).temp = 0;
        }
        (*stbds_header(b)).capacity = min_cap;

        b as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        stbds_free(stbds_header(a as *mut u8) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x3141_5926);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED.store(seed, Ordering::Relaxed);
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    // var = v64_hi, var <<= 16, var <<= 16
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    // var ^= temp ^ v32
    var ^= temp ^ (v32 as usize);
    var
}

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
    let mut n: usize = 0;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash: usize = seed;
        let mut s = str_ as *const u8;
        while *s != 0 {
            hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);
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

macro_rules! siphash_round {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = stbds_rotate_left($v1, 13);
        $v1 ^= $v0;
        $v0 = stbds_rotate_left($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = stbds_rotate_left($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = stbds_rotate_left($v1, 17);
        $v1 ^= $v2;
        $v2 = stbds_rotate_left($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = stbds_rotate_left($v3, 21);
        $v3 ^= $v0;
    }};
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;
        let mut v0: usize;
        let mut v1: usize;
        let mut v2: usize;
        let mut v3: usize;

        v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut i: usize = 0;
        let mut data: usize;
        while i + core::mem::size_of::<usize>() <= len {
            // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
            // (int arithmetic in C: d[3]<<24 may become negative and then
            //  sign-extends when converted to size_t)
            let lo: i32 = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | ((*d.add(3) as i32) << 24);
            data = lo as i64 as usize;
            let hi: i32 = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | ((*d.add(7) as i32) << 24);
            data |= ((hi as i64 as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i += core::mem::size_of::<usize>();
            d = d.add(core::mem::size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        // switch (len - i) with fall-through
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
            data |= (((*d.add(3) as i32) << 24) as i64) as usize;
        }
        if rem >= 3 {
            data |= (((*d.add(2) as i32) << 16) as i64) as usize;
        }
        if rem >= 2 {
            data |= (((*d.add(1) as i32) << 8) as i64) as usize;
        }
        if rem >= 1 {
            data |= (*d.add(0) as i32) as i64 as usize;
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round!(v0, v1, v2, v3);
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            siphash_round!(v0, v1, v2, v3);
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// hash index construction
// ---------------------------------------------------------------------------

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = stbds_realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT)
                .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
                .wrapping_add(core::mem::size_of::<stbds_hash_index>())
                .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
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

        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            c_memset(
                (&raw mut (*t).string) as *mut u8,
                0,
                core::mem::size_of::<stbds_string_arena>(),
            );
            let old_seed = STBDS_HASH_SEED.load(Ordering::Relaxed);
            (*t).seed = old_seed;
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
            STBDS_HASH_SEED.store(
                old_seed.wrapping_mul(a).wrapping_add(b),
                Ordering::Relaxed,
            );
        }

        {
            let mut i: usize = 0;
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
            let mut i: usize = 0;
            while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if (*ob).index[j] >= 0 {
                        let hash = (*ob).hash[j];
                        let mut pos =
                            stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                        let mut step = STBDS_BUCKET_LENGTH;
                        'probe: loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                            let mut z = pos & STBDS_BUCKET_MASK;
                            while z < STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'probe;
                                }
                                z += 1;
                            }

                            let limit = pos & STBDS_BUCKET_MASK;
                            let mut z = 0usize;
                            while z < limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'probe;
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
// hash map internals
// ---------------------------------------------------------------------------

unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> c_int {
    unsafe {
        if mode >= STBDS_HM_STRING {
            let stored = *(a.add(elemsize.wrapping_mul(i).wrapping_add(keyoffset))
                as *mut *mut c_char);
            (0 == strcmp(key as *const c_char, stored)) as c_int
        } else {
            (0 == memcmp(
                key,
                a.add(elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *const c_void,
                keysize,
            )) as c_int
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        let a = a as *mut u8;
        if a.is_null() {
            return;
        }
        if !stbds_hash_table(a).is_null() {
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    stbds_free(
                        *(a.add(elemsize.wrapping_mul(i)) as *mut *mut c_char) as *mut c_void,
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
    a: *mut u8,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
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
        let keyoffset: usize = 0;
        let a = a as *mut u8;
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
            (*stbds_header(a)).length += 1;
            c_memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            arr_to_hash(a, elemsize) as *mut c_void
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
                    let b = (*table)
                        .storage
                        .add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
                }
            }
            a as *mut c_void
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
        stbds_temp_set(hash_to_arr(p as *mut u8, elemsize), temp);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a as *mut u8;
        if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
            let base = if a.is_null() {
                ptr::null_mut()
            } else {
                hash_to_arr(a, elemsize) as *mut c_void
            };
            a = stbds_arrgrowf(base, elemsize, 0, 1) as *mut u8;
            (*stbds_header(a)).length += 1;
            c_memset(a, 0, elemsize);
            a = arr_to_hash(a, elemsize);
        }
        a as *mut c_void
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        c_memmove(p as *mut u8, str_ as *const u8, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a_in: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset: usize = 0;
        let mut a = a_in as *mut u8;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
            c_memset(a, 0, elemsize);
            (*stbds_header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }

        // NOTE: mirrors the C code, where `raw_a` holds the *hash* pointer and
        // `a` is re-pointed at the raw array.
        let mut raw_a: *mut u8 = a;
        let mut a: *mut u8 = hash_to_arr(a, elemsize);

        let mut table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count.wrapping_mul(2)
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
            (*stbds_header(a)).hash_table = nt as *mut c_void;
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

            if hash < 2 {
                hash += 2;
            }

            let mut pos =
                stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

            'found_empty_slot: loop {
                let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

                let mut i = pos & STBDS_BUCKET_MASK;
                let mut escaped = false;
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
                                let existing = *(raw_a.add(
                                    elemsize
                                        .wrapping_mul((*bucket).index[i] as usize)
                                        .wrapping_add(keyoffset),
                                ) as *mut *mut c_char);
                                stbds_temp_key_set(a, existing);
                            }
                            return arr_to_hash(a, elemsize) as *mut c_void;
                        }
                    } else if (*bucket).hash[i] == 0 {
                        pos = (pos & !STBDS_BUCKET_MASK) + i;
                        escaped = true;
                        break;
                    } else if tombstone < 0 {
                        if (*bucket).index[i] == STBDS_INDEX_DELETED {
                            tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                        }
                    }
                    i += 1;
                }
                if escaped {
                    break 'found_empty_slot;
                }

                let limit = pos & STBDS_BUCKET_MASK;
                let mut i = 0usize;
                let mut escaped = false;
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
                            return arr_to_hash(a, elemsize) as *mut c_void;
                        }
                    } else if (*bucket).hash[i] == 0 {
                        pos = (pos & !STBDS_BUCKET_MASK) + i;
                        escaped = true;
                        break;
                    } else if tombstone < 0 {
                        if (*bucket).index[i] == STBDS_INDEX_DELETED {
                            tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                        }
                    }
                    i += 1;
                }
                if escaped {
                    break 'found_empty_slot;
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
                if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                    a = stbds_arrgrowf(a as *mut c_void, elemsize, 1, 0) as *mut u8;
                }
                raw_a = arr_to_hash(a, elemsize);
                let _ = raw_a;

                (*stbds_header(a)).length = (i + 1) as usize;
                let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let slot = a.add(elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let v = stbds_strdup(key as *mut c_char);
                        *slot = v;
                        stbds_temp_key_set(a, v);
                    }
                    STBDS_SH_ARENA => {
                        let v = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                        *slot = v;
                        stbds_temp_key_set(a, v);
                    }
                    STBDS_SH_DEFAULT => {
                        let v = key as *mut c_char;
                        *slot = v;
                        stbds_temp_key_set(a, v);
                    }
                    _ => {
                        c_memmove(
                            a.add(elemsize.wrapping_mul(i as usize)),
                            key as *const u8,
                            keysize,
                        );
                    }
                }
            }
            arr_to_hash(a, elemsize) as *mut c_void
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        c_memset(a, 0, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        arr_to_hash(a, elemsize) as *mut c_void
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
        let a = a as *mut u8;
        if a.is_null() {
            return ptr::null_mut();
        }
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_temp_set(raw_a, 0);
        if table.is_null() {
            return a as *mut c_void;
        }

        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a as *mut c_void;
        }

        let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        stbds_temp_set(raw_a, 1);
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            stbds_free(
                *(a.add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char)
                    as *mut c_void,
            );
        }

        if old_index != final_index {
            c_memmove(
                a.add(elemsize.wrapping_mul(old_index as usize)),
                a.add(elemsize.wrapping_mul(final_index as usize)) as *const u8,
                elemsize,
            );

            if mode == STBDS_HM_STRING {
                let k = *(a.add(
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                ) as *mut *mut c_char);
                slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
            } else {
                let k = a.add(
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                );
                slot = stbds_hm_find_slot(a, elemsize, k as *mut c_void, keysize, keyoffset, mode);
            }
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            (*b).index[i] = old_index;
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

        a as *mut c_void
    }
}

// ---------------------------------------------------------------------------
// string arena
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let p: *mut c_char;
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize: usize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    core::mem::size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                c_memmove(
                    (&raw mut (*sb).storage) as *mut u8,
                    str_ as *const u8,
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
                    core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .add((*a).remaining)
            .sub(len);
        (*a).remaining -= len;
        c_memmove(p as *mut u8, str_ as *const u8, len);
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
        c_memset(
            a as *mut u8,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
    }
}

// ---------------------------------------------------------------------------
// test helpers exported by the library
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut BUFFER) as *mut c_char;
        sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
        buf
    }
}

/// The struct used by `str_put`: `struct { char *key; int value; }`
#[repr(C)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    unsafe {
        const ELEMSIZE: usize = core::mem::size_of::<StrMapEntry>(); // 16
        const KEYSIZE: usize = core::mem::size_of::<*mut c_char>(); // 8

        let mut strmap: *mut u8 = ptr::null_mut();
        let mut sa = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };

        let mut i: c_int = 0;
        while i < num {
            stbds_stralloc(&mut sa, strkey(i));
            i += 1;
        }
        stbds_strreset(&mut sa);

        {
            // s.key = "a", s.value = num;
            let s = StrMapEntry {
                key: b"a\0".as_ptr() as *mut c_char,
                value: num,
            };

            // shputs(strmap, s)
            strmap = stbds_hmput_key(
                strmap as *mut c_void,
                ELEMSIZE,
                s.key as *mut c_void,
                KEYSIZE,
                STBDS_HM_STRING,
            ) as *mut u8;
            let raw = hash_to_arr(strmap, ELEMSIZE); // (t)-1
            let t0 = stbds_temp_get(raw);
            let elem = strmap.add(ELEMSIZE.wrapping_mul(t0 as usize)) as *mut StrMapEntry;
            (*elem).key = s.key;
            (*elem).value = s.value;
            let t1 = stbds_temp_get(hash_to_arr(strmap, ELEMSIZE));
            let elem1 = strmap.add(ELEMSIZE.wrapping_mul(t1 as usize)) as *mut StrMapEntry;
            // (t)[temp].key = stbds_temp_key((t)-1)
            (*elem1).key = *((*stbds_header(hash_to_arr(strmap, ELEMSIZE))).hash_table
                as *mut *mut c_char);

            // for (z=0; z < shlen(strmap); ++z)
            //    printf("%s %d\n", strmap[z], strmap[z].value);
            //
            // The first vararg is the whole 16-byte struct; per the SysV AMD64
            // ABI it occupies two INTEGER registers, so "%s" consumes the
            // `key` pointer and "%d" consumes the low half of the second
            // eightbyte, i.e. `value`. The trailing `strmap[z].value` argument
            // is never read by the format string.
            let mut z: isize = 0;
            while z < stbds_hmlen(strmap, ELEMSIZE) {
                let e = strmap.add(ELEMSIZE.wrapping_mul(z as usize)) as *mut StrMapEntry;
                printf(
                    b"%s %d\n\0".as_ptr() as *const c_char,
                    (*e).key,
                    (*e).value,
                );
                z += 1;
            }

            // shfree(strmap)
            if !strmap.is_null() {
                stbds_hmfree_func(hash_to_arr(strmap, ELEMSIZE) as *mut c_void, ELEMSIZE);
            }
            strmap = ptr::null_mut();
            let _ = strmap;
        }
    }
}

/// `stbds_hmlen(t)` == `((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)`
#[inline]
unsafe fn stbds_hmlen(t: *mut u8, elemsize: usize) -> isize {
    unsafe {
        if t.is_null() {
            0
        } else {
            (*stbds_header(hash_to_arr(t, elemsize))).length as isize - 1
        }
    }
}
