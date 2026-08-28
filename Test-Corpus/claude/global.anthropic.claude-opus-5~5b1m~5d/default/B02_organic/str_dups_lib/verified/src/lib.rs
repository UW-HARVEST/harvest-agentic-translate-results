//! Rust translation of the C library in `c_src/` (an stb_ds-derived data
//! structure library plus the `str_dups` / `strkey` test helpers).
//!
//! The translation is deliberately literal: every arithmetic quirk, integer
//! promotion, sign extension and check ordering of the original C is
//! reproduced, including behaviour that would be considered a bug.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]
#![allow(unused_assignments)]

use core::ptr;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free/str*/mem* and printf/sprintf)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn abort() -> !;
}

// ---------------------------------------------------------------------------
// `STBDS_ASSERT` == `assert` from <assert.h>.
//
// The C library is compiled WITHOUT `-DNDEBUG` (see `c_src/CMakeLists.txt`,
// which sets no build type), so `nm -D` on the C `.so` really does show
// `U __assert_fail`: a failing assertion prints a diagnostic and `abort()`s.
// That is externally observable behaviour (SIGABRT), so the translation has to
// reproduce it rather than silently continuing with corrupted state.
// ---------------------------------------------------------------------------

#[cold]
#[inline(never)]
unsafe fn stbds_assert_fail(expr: &str, line: u32) -> ! {
    // Written straight to fd 2, mirroring glibc's `__assert_fail`, without
    // relying on any Rust runtime state.
    let msg = format!("lib.c:{}: Assertion `{}' failed.\n", line, expr);
    write(2, msg.as_ptr() as *const c_void, msg.len());
    abort()
}

macro_rules! STBDS_ASSERT {
    ($cond:expr, $line:expr) => {
        if !($cond) {
            stbds_assert_fail(stringify!($cond), $line);
        }
    };
}

// ---------------------------------------------------------------------------
// Types (layouts must match the C structs exactly)
// ---------------------------------------------------------------------------

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
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Copy, Clone)]
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

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

const HDR_SIZE: usize = core::mem::size_of::<stbds_array_header>();

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1usize << 20;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

// ---------------------------------------------------------------------------
// Small helpers mirroring the C macros
// ---------------------------------------------------------------------------

#[inline(always)]
fn padd(p: *mut c_void, off: usize) -> *mut c_void {
    (p as *mut u8).wrapping_add(off) as *mut c_void
}

#[inline(always)]
fn psub(p: *mut c_void, off: usize) -> *mut c_void {
    (p as *mut u8).wrapping_sub(off) as *mut c_void
}

/// `stbds_header(t)`
#[inline(always)]
fn stbds_header(a: *mut c_void) -> *mut stbds_array_header {
    psub(a, HDR_SIZE) as *mut stbds_array_header
}

/// `stbds_arrlen(a)`
#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `stbds_arrcap(a)`
#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// `stbds_hash_table(a)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline(always)]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    psub(x, elemsize)
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline(always)]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    padd(x, elemsize)
}

#[inline(always)]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
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
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < stbds_arrcap(a).wrapping_mul(2) {
        min_cap = stbds_arrcap(a).wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };
    let mut b = realloc(old, elemsize.wrapping_mul(min_cap).wrapping_add(HDR_SIZE));
    b = padd(b, HDR_SIZE);
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut s = str_;
    let mut hash = seed;
    while *s != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*(s as *const u8) as usize);
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
    let mut d = p as *const u8;
    let mut data: usize;

    let mut v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

    let mut i: usize = 0;
    while i + core::mem::size_of::<usize>() <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // All operands are promoted to `int`, so the result is a *signed* int
        // that is then sign-extended when stored into the size_t `data`.
        let lo: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32).wrapping_shl(24));
        data = lo as i64 as u64 as usize;
        let hi: i32 = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | ((*d.add(7) as i32).wrapping_shl(24));
        data |= ((((hi as i64 as u64 as usize) << 16) as usize) << 16) as usize;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round!(v0, v1, v2, v3);
        }
        v0 ^= data;

        i += core::mem::size_of::<usize>();
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len.wrapping_shl(STBDS_SIZE_T_BITS - 8);
    let rem = len.wrapping_sub(i);
    // Fallthrough switch: case 7 falls into 6, 5, ... 1.
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
        data |= ((*d.add(3) as i32).wrapping_shl(24)) as i64 as u64 as usize;
    }
    if rem >= 3 {
        data |= ((*d.add(2) as i32) << 16) as i64 as u64 as usize;
    }
    if rem >= 2 {
        data |= ((*d.add(1) as i32) << 8) as i64 as u64 as usize;
    }
    if rem >= 1 {
        data |= (*d.add(0) as i32) as i64 as u64 as usize;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// hash index construction
// ---------------------------------------------------------------------------

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;

    (*t).storage = stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }
    // c_src/src/lib.c:401
    STBDS_ASSERT!(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count,
        401
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        memset(
            ptr::addr_of_mut!((*t).string) as *mut c_void,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let a: usize = {
            let mut temp: usize = (0x87b0b0fdu32 ^ 2147001325u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var: usize = 0x27bb2ee6usize;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ 2147001325usize;
            var
        };
        // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d);
        let b: usize = {
            let mut temp: usize = (0xb504f32du32 ^ 715136305u32) as usize;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut var: usize = 0usize;
            var <<= 16;
            var <<= 16;
            var ^= temp ^ 715136305usize;
            var
        };
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
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
                    'search: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'search;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'search;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
            }
            i += 1;
        }
    }

    t
}

// ---------------------------------------------------------------------------
// hash map internals
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
    if mode >= STBDS_HM_STRING {
        let slot = padd(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *mut *mut c_char;
        (0 == strcmp(key as *const c_char, *slot as *const c_char)) as c_int
    } else {
        let slot = padd(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset));
        (0 == memcmp(key as *const c_void, slot as *const c_void, keysize)) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                free(*(padd(a, elemsize.wrapping_mul(i)) as *mut *mut c_char) as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));
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
                    return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
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
                    return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        pos = pos.wrapping_add(step);
        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
        pos &= (*table).slot_count.wrapping_sub(1);
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
    let keyoffset: usize = 0;
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
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
                let b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
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
    (*stbds_header(stbds_hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
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
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        memset(a, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_).wrapping_add(1);
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut a = a;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        a = stbds_arr_to_hash(a, elemsize);
    }

    raw_a = a;
    a = stbds_hash_to_arr(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
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
        let mut pos: usize;
        let mut tombstone: isize = -1;
        let mut bucket: *mut stbds_hash_bucket;

        if hash < 2 {
            hash = hash.wrapping_add(2);
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

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
                        (*stbds_header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            let src = padd(
                                raw_a,
                                elemsize
                                    .wrapping_mul((*bucket).index[i] as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char;
                            (*stbds_hash_table(a)).temp_key = *src;
                        }
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
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
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        (*stbds_header(a)).temp = (*bucket).index[i];
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                }
                i += 1;
            }

            pos = pos.wrapping_add(step);
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
            pos &= (*table).slot_count.wrapping_sub(1);
        }

        // found_empty_slot:
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
        }
        (*table).used_count = (*table).used_count.wrapping_add(1);

        {
            let i: isize = stbds_arrlen(a);
            if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = stbds_arr_to_hash(a, elemsize);
            let _ = raw_a;

            // c_src/src/lib.c:778
            STBDS_ASSERT!((i as usize).wrapping_add(1) <= stbds_arrcap(a), 778);
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            (*stbds_header(a)).temp = i - 1;

            let dst = padd(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    *dst = v;
                    (*stbds_hash_table(a)).temp_key = v;
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *dst = v;
                    (*stbds_hash_table(a)).temp_key = v;
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    *dst = v;
                    (*stbds_hash_table(a)).temp_key = v;
                }
                _ => {
                    memcpy(
                        padd(a, elemsize.wrapping_mul(i as usize)),
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        stbds_arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
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
        ptr::null_mut()
    } else {
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        (*stbds_header(raw_a)).temp = 0;
        if table.is_null() {
            a
        } else {
            let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                // c_src/src/lib.c:828
                STBDS_ASSERT!(slot < (*table).slot_count as isize, 828);
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                (*stbds_header(raw_a)).temp = 1;
                // c_src/src/lib.c:832 -- `STBDS_ASSERT(table->used_count >= 0)`
                // is vacuously true in C because `used_count` is a `size_t`
                // (it stays true even after the `--used_count` above wraps to
                // SIZE_MAX), so there is no runtime check to emit here.
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                    free(
                        *(padd(a, elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char)
                            as *mut c_void,
                    );
                }

                if old_index != final_index {
                    memmove(
                        padd(a, elemsize.wrapping_mul(old_index as usize)),
                        padd(a, elemsize.wrapping_mul(final_index as usize)) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        let k = *(padd(
                            a,
                            elemsize
                                .wrapping_mul(old_index as usize)
                                .wrapping_add(keyoffset),
                        ) as *mut *mut c_char);
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            k as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        let k = padd(
                            a,
                            elemsize
                                .wrapping_mul(old_index as usize)
                                .wrapping_add(keyoffset),
                        );
                        slot = stbds_hm_find_slot(a, elemsize, k, keysize, keyoffset, mode);
                    }
                    // c_src/src/lib.c:846
                    STBDS_ASSERT!(slot >= 0, 846);
                    b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    // c_src/src/lib.c:849
                    STBDS_ASSERT!((*b).index[i as usize] == final_index, 849);
                    (*b).index[i as usize] = old_index;
                }
                (*stbds_header(raw_a)).length = (*stbds_header(raw_a)).length.wrapping_sub(1);

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
    let p: *mut c_char;
    let len = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(len),
            ) as *mut stbds_string_block;
            memmove(
                ptr::addr_of_mut!((*sb).storage) as *mut c_void,
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
            return ptr::addr_of_mut!((*sb).storage) as *mut c_char;
        } else {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(blocksize),
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    // c_src/src/lib.c:913
    STBDS_ASSERT!(len <= (*a).remaining, 913);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len);
    (*a).remaining = (*a).remaining.wrapping_sub(len);
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
    memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}

// ---------------------------------------------------------------------------
// test helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(buffer) as *mut c_char;
    sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);
    buf
}

#[repr(C)]
#[derive(Copy, Clone)]
struct str_dups_entry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(num: c_int) {
    let elemsize = core::mem::size_of::<str_dups_entry>();

    let mut strmap: *mut str_dups_entry = ptr::null_mut();
    let mut s = str_dups_entry {
        key: ptr::null_mut(),
        value: 0,
    };
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
        s.key = b"a\0".as_ptr() as *mut c_char;
        s.value = num;

        // sh_new_strdup(strmap)
        strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP as c_int) as *mut str_dups_entry;

        // shputs(strmap, s)
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            s.key as *mut c_void,
            core::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut str_dups_entry;
        let raw = psub(strmap as *mut c_void, elemsize);
        let t = (*stbds_header(raw)).temp;
        *strmap.offset(t) = s;
        (*strmap.offset(t)).key = (*stbds_hash_table(raw)).temp_key;

        // c_src/src/lib.c:960-962
        STBDS_ASSERT!(*(*strmap.offset(0)).key == b'a' as c_char, 960);
        STBDS_ASSERT!((*strmap.offset(0)).key != s.key, 961);
        STBDS_ASSERT!((*strmap.offset(0)).value == s.value, 962);

        // for (int z=0; z < shlen(strmap); ++z)
        //   printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // `strmap[z]` is a 16-byte struct passed by value in a variadic call:
        // on the SysV AMD64 ABI it occupies two integer parameter slots, so
        // %s consumes the `key` pointer and %d consumes the low 32 bits of
        // the second eightbyte, i.e. `value`. The third argument is unused.
        let mut z: c_int = 0;
        while (z as isize) < stbds_hmlen(strmap as *mut c_void, elemsize) {
            let e = *strmap.offset(z as isize);
            printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                e.key,
                e.value,
            );
            z += 1;
        }

        // shfree(strmap)
        if !strmap.is_null() {
            stbds_hmfree_func(psub(strmap as *mut c_void, elemsize), elemsize);
        }
        strmap = ptr::null_mut();
        let _ = strmap;
    }
}

/// `stbds_hmlen(t)` == `t ? (ptrdiff_t) stbds_header(t-1)->length - 1 : 0`
#[inline(always)]
unsafe fn stbds_hmlen(t: *mut c_void, elemsize: usize) -> isize {
    if !t.is_null() {
        ((*stbds_header(psub(t, elemsize))).length as isize) - 1
    } else {
        0
    }
}
