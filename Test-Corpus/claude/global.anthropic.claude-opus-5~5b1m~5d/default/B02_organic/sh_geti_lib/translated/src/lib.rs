//! Faithful Rust translation of the C library in `c_src/` (an stb_ds derived
//! hash-map / dynamic-array implementation plus the `sh_geti` / `strkey`
//! driver helpers).
//!
//! The translation is intentionally literal: every arithmetic quirk, implicit
//! integer conversion, order of operations, and (mis)behaviour of the original
//! C is reproduced, including the sign-extension bug in the SipHash byte
//! loading and the `printf("%s %d\n", struct, struct.value)` call in
//! `sh_geti`.
//!
//! Memory management uses libc `realloc`/`free` exactly like the C code so
//! that the header-behind-the-pointer trick and the pointer arithmetic remain
//! byte-for-byte compatible with the C implementation.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (the C library links against libc + libm)
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
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn abort() -> !;
}

/// `assert()` from <assert.h>.  NDEBUG is not defined for the C build, so the
/// assertions are live; a failing assertion terminates the process.
macro_rules! STBDS_ASSERT {
    ($cond:expr) => {
        if !($cond) {
            unsafe { abort() }
        }
    };
}

// ---------------------------------------------------------------------------
// Types
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

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

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

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Small helpers mirroring the C macros
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *) (t) - 1)`
#[inline(always)]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
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

/// `stbds_arrlen(a)`
#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// `stbds_temp(t)` (lvalue read)
#[inline(always)]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

/// `stbds_temp(t) = v`
#[inline(always)]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `stbds_temp_key(t) = v`, i.e. `*(char **) stbds_header(t)->hash_table = v`
#[inline(always)]
unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
}

/// `stbds_hash_table(a)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline(always)]
fn STBDS_HASH_TO_ARR(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline(always)]
fn STBDS_ARR_TO_HASH(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `(char *) a + elemsize * i`
#[inline(always)]
fn elem_at(a: *mut c_void, elemsize: usize, i: usize) -> *mut u8 {
    (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i))
}

/// `STBDS_ALIGN_FWD(n, a)`
#[inline(always)]
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

/// `STBDS_ROTATE_LEFT(val, n)`
#[inline(always)]
fn STBDS_ROTATE_LEFT(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ROTATE_RIGHT(val, n)`
#[inline(always)]
fn STBDS_ROTATE_RIGHT(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ---------------------------------------------------------------------------
// Dynamic array growth
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let b: *mut c_void;
    let min_len: usize = (stbds_arrlen(a) as usize).wrapping_add(addlen);

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

    let old: *mut c_void = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };
    let raw = realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(HEADER_SIZE),
    );
    b = (raw as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;
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
// Hash index
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
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
    let t: *mut stbds_hash_index;
    t = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(core::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut stbds_hash_index;
    (*t).storage = STBDS_ALIGN_FWD(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE)
        as *mut stbds_hash_bucket;
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
    STBDS_ASSERT!(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count
    );
    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        let a: usize;
        let b: usize;
        let mut temp: usize;
        memset(
            ptr::addr_of_mut!((*t).string) as *mut c_void,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        temp = 0x87b0_b0fdusize ^ 2147001325usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        let mut var = 0x27bb_2ee6usize;
        var <<= 16;
        var <<= 16;
        var ^= temp ^ 2147001325usize;
        a = var;
        // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d);
        temp = 0xb504_f32dusize ^ 715136305usize;
        temp <<= 16;
        temp <<= 16;
        temp >>= 16;
        temp >>= 16;
        let mut var2 = 0usize;
        var2 <<= 16;
        var2 <<= 16;
        var2 ^= temp ^ 715136305usize;
        b = var2;
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b: *mut stbds_hash_bucket = (*t).storage.wrapping_add(i);
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
            let ob: *mut stbds_hash_bucket = (*ot).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash: usize = (*ob).hash[j];
                    let mut pos: usize =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step: usize = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z: usize = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        let limit: usize = pos & STBDS_BUCKET_MASK;
                        let mut z: usize = 0;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
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
// Hashing
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s: *const u8 = str_ as *const u8;
    while *s != 0 {
        hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as usize);
        s = s.wrapping_add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= STBDS_ROTATE_RIGHT(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline(always)]
fn stbds_sipround(v: &mut [usize; 4]) {
    v[0] = v[0].wrapping_add(v[1]);
    v[1] = STBDS_ROTATE_LEFT(v[1], 13);
    v[1] ^= v[0];
    v[0] = STBDS_ROTATE_LEFT(v[0], STBDS_SIZE_T_BITS / 2);
    v[2] = v[2].wrapping_add(v[3]);
    v[3] = STBDS_ROTATE_LEFT(v[3], 16);
    v[3] ^= v[2];
    v[2] = v[2].wrapping_add(v[1]);
    v[1] = STBDS_ROTATE_LEFT(v[1], 17);
    v[1] ^= v[2];
    v[2] = STBDS_ROTATE_LEFT(v[2], STBDS_SIZE_T_BITS / 2);
    v[0] = v[0].wrapping_add(v[3]);
    v[3] = STBDS_ROTATE_LEFT(v[3], 21);
    v[3] ^= v[0];
}

/// The C code builds `data` out of `int` sub-expressions; when the top byte has
/// its high bit set the `int` becomes negative and the conversion to `size_t`
/// sign-extends.  That behaviour is part of the observable output and is
/// reproduced here.
#[inline(always)]
fn c_int_to_size_t(v: u32) -> usize {
    ((v as i32) as i64) as usize
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d: *const u8 = p as *const u8;
    let mut i: usize;
    let mut j: usize;
    let mut v: [usize; 4] = [0; 4];
    let mut data: usize;

    v[0] = ((0x736f_6d65usize << 16) << 16).wrapping_add(0x7073_6575) ^ seed;
    v[1] = ((0x646f_7261usize << 16) << 16).wrapping_add(0x6e64_6f6d) ^ !seed;
    v[2] = ((0x6c79_6765usize << 16) << 16).wrapping_add(0x6e65_7261) ^ seed;
    v[3] = ((0x7465_6462usize << 16) << 16).wrapping_add(0x7974_6573) ^ !seed;

    v[0] ^= 0x0706_0504_0302_0100usize ^ seed;
    v[1] ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v[2] ^= 0x0706_0504_0302_0100usize ^ seed;
    v[3] ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    i = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        let lo: u32 = (*d.wrapping_add(0) as u32)
            | ((*d.wrapping_add(1) as u32) << 8)
            | ((*d.wrapping_add(2) as u32) << 16)
            | ((*d.wrapping_add(3) as u32) << 24);
        data = c_int_to_size_t(lo);
        let hi: u32 = (*d.wrapping_add(4) as u32)
            | ((*d.wrapping_add(5) as u32) << 8)
            | ((*d.wrapping_add(6) as u32) << 16)
            | ((*d.wrapping_add(7) as u32) << 24);
        data |= (c_int_to_size_t(hi) << 16) << 16;

        v[3] ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround(&mut v);
            j += 1;
        }
        v[0] ^= data;

        i = i.wrapping_add(core::mem::size_of::<usize>());
        d = d.wrapping_add(core::mem::size_of::<usize>());
    }
    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch (len - i) with C fall-through semantics
    let rem: usize = len.wrapping_sub(i);
    if rem >= 7 {
        data |= ((*d.wrapping_add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.wrapping_add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.wrapping_add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        data |= c_int_to_size_t((*d.wrapping_add(3) as u32) << 24);
    }
    if rem >= 3 {
        data |= (*d.wrapping_add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*d.wrapping_add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *d.wrapping_add(0) as usize;
    }

    v[3] ^= data;
    j = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        stbds_sipround(&mut v);
        j += 1;
    }
    v[0] ^= data;
    v[2] ^= 0xff;
    j = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        stbds_sipround(&mut v);
        j += 1;
    }

    v[0] ^ v[1] ^ v[2] ^ v[3]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
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
    if mode >= STBDS_HM_STRING {
        (0 == strcmp(
            key as *const c_char,
            *(elem_at(a, elemsize, i).wrapping_add(keyoffset) as *mut *mut c_char),
        )) as c_int
    } else {
        (0 == memcmp(
            key as *const c_void,
            elem_at(a, elemsize, i).wrapping_add(keyoffset) as *const c_void,
            keysize,
        )) as c_int
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
                free(*(elem_at(a, elemsize, i) as *mut *mut c_char) as *mut c_void);
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
    let raw_a: *mut c_void = STBDS_HASH_TO_ARR(a, elemsize);
    let table: *mut stbds_hash_index = stbds_hash_table(raw_a);
    let mut hash: usize = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: usize = STBDS_BUCKET_LENGTH;
    let mut pos: usize;
    let mut bucket: *mut stbds_hash_bucket;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let limit: usize;
        bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

        let mut i: usize = pos & STBDS_BUCKET_MASK;
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

        limit = pos & STBDS_BUCKET_MASK;
        let mut i: usize = 0;
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
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count.wrapping_sub(1);
    }
}

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
        STBDS_ARR_TO_HASH(a, elemsize)
    } else {
        let table: *mut stbds_hash_index;
        let raw_a: *mut c_void = STBDS_HASH_TO_ARR(a, elemsize);
        table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot: isize = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b: *mut stbds_hash_bucket =
                    (*table).storage.wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
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
    let p: *mut c_void = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    stbds_set_temp(STBDS_HASH_TO_ARR(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null()
        || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0
    {
        a = stbds_arrgrowf(
            if !a.is_null() {
                STBDS_HASH_TO_ARR(a, elemsize)
            } else {
                ptr::null_mut()
            },
            elemsize,
            0,
            1,
        );
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }

    raw_a = a;
    a = STBDS_HASH_TO_ARR(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let nt: *mut stbds_hash_index;
        let slot_count: usize;

        slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = nt as *mut c_void;
    }

    {
        let mut hash: usize = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step: usize = STBDS_BUCKET_LENGTH;
        let mut pos: usize;
        let mut tombstone: isize = -1;
        let mut bucket: *mut stbds_hash_bucket;

        if hash < 2 {
            hash = hash.wrapping_add(2);
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'found_empty_slot: loop {
            let limit: usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

            let mut i: usize = pos & STBDS_BUCKET_MASK;
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
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let kp = *(elem_at(
                                raw_a,
                                elemsize,
                                (*bucket).index[i] as usize,
                            )
                            .wrapping_add(keyoffset)
                                as *mut *mut c_char);
                            stbds_set_temp_key(a, kp);
                        }
                        return STBDS_ARR_TO_HASH(a, elemsize);
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

            limit = pos & STBDS_BUCKET_MASK;
            let mut i: usize = 0;
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
                        stbds_set_temp(a, (*bucket).index[i]);
                        return STBDS_ARR_TO_HASH(a, elemsize);
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
            step += STBDS_BUCKET_LENGTH;
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
            raw_a = STBDS_ARR_TO_HASH(a, elemsize);

            STBDS_ASSERT!((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_set_temp(a, i - 1);

            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let s = stbds_strdup(key as *mut c_char);
                    *(elem_at(a, elemsize, i as usize) as *mut *mut c_char) = s;
                    stbds_set_temp_key(a, s);
                }
                STBDS_SH_ARENA => {
                    let s = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *(elem_at(a, elemsize, i as usize) as *mut *mut c_char) = s;
                    stbds_set_temp_key(a, s);
                }
                STBDS_SH_DEFAULT => {
                    let s = key as *mut c_char;
                    *(elem_at(a, elemsize, i as usize) as *mut *mut c_char) = s;
                    stbds_set_temp_key(a, s);
                }
                _ => {
                    memcpy(
                        elem_at(a, elemsize, i as usize) as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        let _ = raw_a;
        STBDS_ARR_TO_HASH(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a: *mut c_void = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    let h: *mut stbds_hash_index;
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    STBDS_ARR_TO_HASH(a, elemsize)
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
        let table: *mut stbds_hash_index;
        let raw_a: *mut c_void = STBDS_HASH_TO_ARR(a, elemsize);
        table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_set_temp(raw_a, 0);
        if table.is_null() {
            a
        } else {
            let mut slot: isize;
            slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b: *mut stbds_hash_bucket =
                    (*table).storage.wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index: isize = (*b).index[i as usize];
                let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                STBDS_ASSERT!(slot < (*table).slot_count as isize);
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                stbds_set_temp(raw_a, 1);
                // STBDS_ASSERT(table->used_count >= 0) -- always true for size_t
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                    free(
                        *(elem_at(a, elemsize, old_index as usize) as *mut *mut c_char)
                            as *mut c_void,
                    );
                }

                if old_index != final_index {
                    memmove(
                        elem_at(a, elemsize, old_index as usize) as *mut c_void,
                        elem_at(a, elemsize, final_index as usize) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            *(elem_at(a, elemsize, old_index as usize).wrapping_add(keyoffset)
                                as *mut *mut c_char) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            elem_at(a, elemsize, old_index as usize).wrapping_add(keyoffset)
                                as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    STBDS_ASSERT!(slot >= 0);
                    b = (*table)
                        .storage
                        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    STBDS_ASSERT!((*b).index[i as usize] == final_index);
                    (*b).index[i as usize] = old_index;
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
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len: usize = strlen(str_).wrapping_add(1);
    let p: *mut c_char = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len: usize = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb: *mut stbds_string_block = realloc(
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
            let sb: *mut stbds_string_block = realloc(
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

    STBDS_ASSERT!(len <= (*a).remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining as isize as usize)
        .wrapping_sub(len);
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x: *mut stbds_string_block;
    let mut y: *mut stbds_string_block;
    x = (*a).storage;
    while !x.is_null() {
        y = (*x).next;
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
// Test/driver helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(buffer) as *mut c_char;
    sprintf(buf, c"test_%d".as_ptr(), n);
    buf
}

/// `struct { char *key; int value; }` from `sh_geti`.
#[repr(C)]
#[derive(Copy, Clone)]
struct sh_geti_entry {
    key: *mut c_char,
    value: c_int,
}

const SH_ES: usize = core::mem::size_of::<sh_geti_entry>();
const SH_KEYSIZE: usize = core::mem::size_of::<*mut c_char>();

/// `(t) - 1` for the user-visible hash-map pointer, i.e. the raw array base.
#[inline(always)]
fn sh_raw(t: *mut sh_geti_entry) -> *mut c_void {
    (t as *mut u8).wrapping_sub(SH_ES) as *mut c_void
}

/// `stbds_shlen(t)`
#[inline(always)]
unsafe fn sh_len(t: *mut sh_geti_entry) -> isize {
    if !t.is_null() {
        (*stbds_header(sh_raw(t))).length as isize - 1
    } else {
        0
    }
}

/// `stbds_shgeti(t, k)`
#[inline(always)]
unsafe fn sh_geti_macro(t: &mut *mut sh_geti_entry, k: *const c_char) -> isize {
    *t = stbds_hmget_key(
        *t as *mut c_void,
        SH_ES,
        k as *mut c_void,
        SH_KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut sh_geti_entry;
    stbds_temp(sh_raw(*t))
}

/// `stbds_shput(t, k, v)`
#[inline(always)]
unsafe fn sh_put(t: &mut *mut sh_geti_entry, k: *const c_char, v: c_int) {
    *t = stbds_hmput_key(
        *t as *mut c_void,
        SH_ES,
        k as *mut c_void,
        SH_KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut sh_geti_entry;
    let idx = stbds_temp(sh_raw(*t));
    (*(*t).wrapping_offset(idx)).value = v;
}

/// `stbds_shget(t, k)`
#[inline(always)]
unsafe fn sh_get(t: &mut *mut sh_geti_entry, k: *const c_char) -> c_int {
    sh_geti_macro(t, k);
    let idx = stbds_temp(sh_raw(*t));
    (*(*t).wrapping_offset(idx)).value
}

/// `stbds_shdel(t, k)`
#[inline(always)]
unsafe fn sh_del(t: &mut *mut sh_geti_entry, k: *const c_char) -> isize {
    *t = stbds_hmdel_key(
        *t as *mut c_void,
        SH_ES,
        k as *mut c_void,
        SH_KEYSIZE,
        0, // STBDS_OFFSETOF(t, key)
        STBDS_HM_STRING,
    ) as *mut sh_geti_entry;
    if !(*t).is_null() {
        stbds_temp(sh_raw(*t))
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let mut strmap: *mut sh_geti_entry = ptr::null_mut();
    let mut sa: stbds_string_arena = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut i: c_int;
    let mut j: c_int;

    let foo: *const c_char = c"foo".as_ptr();

    i = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    j = 0;
    while j < 2 {
        STBDS_ASSERT!(sh_geti_macro(&mut strmap, foo) == -1);
        if j == 0 {
            // sh_new_strdup(strmap)
            strmap = stbds_shmode_func(SH_ES, STBDS_SH_STRDUP as c_int) as *mut sh_geti_entry;
        } else {
            // sh_new_arena(strmap)
            strmap = stbds_shmode_func(SH_ES, STBDS_SH_ARENA as c_int) as *mut sh_geti_entry;
        }
        STBDS_ASSERT!(sh_geti_macro(&mut strmap, foo) == -1);
        // shdefault(strmap, -2)
        strmap = stbds_hmput_default(strmap as *mut c_void, SH_ES) as *mut sh_geti_entry;
        (*strmap.wrapping_offset(-1)).value = -2;
        STBDS_ASSERT!(sh_geti_macro(&mut strmap, foo) == -1);

        i = 0;
        while i < num {
            sh_put(&mut strmap, strkey(i), i.wrapping_mul(3));
            i += 2;
        }

        let mut z: c_int = 0;
        while (z as isize) < sh_len(strmap) {
            let e: sh_geti_entry = *strmap.wrapping_offset(z as isize);
            // printf("%s %d\n", strmap[z], strmap[z].value);
            //
            // The struct is passed by value: on the SysV AMD64 ABI it occupies
            // two INTEGER eightbytes, so "%s" consumes the `key` member and
            // "%d" consumes the low half of the second eightbyte, i.e. the
            // `value` member.  The trailing `strmap[z].value` argument is never
            // consumed by the format string.
            printf(c"%s %d\n".as_ptr(), e.key, e.value);
            z += 1;
        }

        i = 0;
        while i < num {
            if i & 1 != 0 {
                STBDS_ASSERT!(sh_get(&mut strmap, strkey(i)) == -2);
            } else {
                STBDS_ASSERT!(sh_get(&mut strmap, strkey(i)) == i.wrapping_mul(3));
            }
            i += 1;
        }
        i = 2;
        while i < num {
            sh_del(&mut strmap, strkey(i));
            i += 4;
        }
        i = 0;
        while i < num {
            if i & 3 != 0 {
                STBDS_ASSERT!(sh_get(&mut strmap, strkey(i)) == -2);
            } else {
                STBDS_ASSERT!(sh_get(&mut strmap, strkey(i)) == i.wrapping_mul(3));
            }
            i += 1;
        }
        i = 0;
        while i < num {
            sh_del(&mut strmap, strkey(i));
            i += 1;
        }
        i = 0;
        while i < num {
            STBDS_ASSERT!(sh_get(&mut strmap, strkey(i)) == -2);
            i += 1;
        }

        // shfree(strmap)
        if !strmap.is_null() {
            stbds_hmfree_func(sh_raw(strmap), SH_ES);
        }
        strmap = ptr::null_mut();

        j += 1;
    }
    let _ = STBDS_SH_NONE;
    let _ = STBDS_HM_BINARY;
}
