//! Rust translation of the C library in `c_src/` (an stb_ds.h-based
//! dynamic array / hash map implementation plus two small test helpers).
//!
//! The translation is intentionally a literal, pointer-for-pointer port of the
//! C code: the same allocations (via libc `realloc`/`free`), the same memory
//! layouts, the same integer wrap-around / sign-extension quirks and the same
//! order of side effects.  Behaviour (including reproduced C quirks) must match
//! byte for byte.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free/memset/... directly)
// ---------------------------------------------------------------------------

extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(p: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn abort() -> !;
}

/// `STBDS_ASSERT` == `assert()`.  None of these can fire during valid use of
/// the library; if one ever does, terminate like `assert()` does.
#[inline(always)]
fn stbds_assert(cond: bool) {
    if !cond {
        unsafe { abort() }
    }
}

// ---------------------------------------------------------------------------
// Data structures (must match the C layouts exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

const HDR_SIZE: usize = core::mem::size_of::<stbds_array_header>(); // 32

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

#[repr(C)]
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

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

// Compile-time layout checks mirroring the C struct sizes.
const _: () = assert!(core::mem::size_of::<stbds_array_header>() == 32);
const _: () = assert!(core::mem::size_of::<stbds_hash_bucket>() == 128);
const _: () = assert!(core::mem::size_of::<stbds_string_block>() == 16);
const _: () = assert!(core::mem::size_of::<stbds_string_arena>() == 24);
const _: () = assert!(core::mem::size_of::<stbds_hash_index>() == 104);
const _: () = assert!(core::mem::size_of::<usize>() == 8);

// ---------------------------------------------------------------------------
// Small pointer helpers (byte arithmetic, exactly like the C macros)
// ---------------------------------------------------------------------------

#[inline(always)]
fn byte_add(p: *mut c_void, n: usize) -> *mut c_void {
    (p as *mut u8).wrapping_add(n) as *mut c_void
}

#[inline(always)]
fn byte_sub(p: *mut c_void, n: usize) -> *mut c_void {
    (p as *mut u8).wrapping_sub(n) as *mut c_void
}

/// `stbds_header(t)`
#[inline(always)]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    byte_sub(t, HDR_SIZE) as *mut stbds_array_header
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline(always)]
fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    byte_add(x, elemsize)
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline(always)]
fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    byte_sub(x, elemsize)
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

/// `stbds_temp(t)` (lvalue write)
#[inline(always)]
unsafe fn set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

/// `stbds_temp_key(t)` (lvalue write): `*(char **) stbds_header(t)->hash_table`
#[inline(always)]
unsafe fn set_temp_key(t: *mut c_void, v: *mut c_char) {
    let tbl = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *tbl = v;
}

/// `stbds_hash_table(a)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ---------------------------------------------------------------------------
// Dynamic array
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

    let old: *mut c_void = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };

    let raw = realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HDR_SIZE),
    );
    let b = byte_add(raw, HDR_SIZE);
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
// Hashing
// ---------------------------------------------------------------------------

static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x31415926);

#[inline(always)]
fn hash_seed_get() -> usize {
    STBDS_HASH_SEED.load(Ordering::Relaxed)
}

#[inline(always)]
fn hash_seed_set(v: usize) {
    STBDS_HASH_SEED.store(v, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    hash_seed_set(seed);
}

#[inline(always)]
fn rotl(val: usize, n: u32) -> usize {
    // ((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n)))
    (val.wrapping_shl(n)) | (val.wrapping_shr(STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
fn rotr(val: usize, n: u32) -> usize {
    // ((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n)))
    (val.wrapping_shr(n)) | (val.wrapping_shl(STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
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
    let mut hash = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = rotl(hash, 9).wrapping_add(*s as usize);
        s = s.wrapping_add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash = hash ^ (hash ^ rotr(hash, 31));
    hash = hash.wrapping_mul(21);
    hash = hash ^ (hash ^ rotr(hash, 11));
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16 << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = ((0x646f7261usize << 16 << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = ((0x6c796765usize << 16 << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = ((0x74656462usize << 16 << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    macro_rules! siproundf {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = rotl(v1, 13);
            v1 ^= v0;
            v0 = rotl(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = rotl(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotl(v1, 17);
            v1 ^= v2;
            v2 = rotl(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = rotl(v3, 21);
            v3 ^= v0;
        }};
    }

    let mut i: usize = 0;
    while i + core::mem::size_of::<usize>() <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        //
        // NOTE: in C these are `int` expressions; `d[3] << 24` can set the sign
        // bit, and the resulting negative `int` is then sign-extended when
        // stored into the `size_t` `data`.  That quirk is part of the observable
        // behaviour, so it is reproduced here.
        let lo: i32 = (*d.wrapping_add(0) as i32)
            | ((*d.wrapping_add(1) as i32).wrapping_shl(8))
            | ((*d.wrapping_add(2) as i32).wrapping_shl(16))
            | ((*d.wrapping_add(3) as i32).wrapping_shl(24));
        data = lo as isize as usize;

        let hi: i32 = (*d.wrapping_add(4) as i32)
            | ((*d.wrapping_add(5) as i32).wrapping_shl(8))
            | ((*d.wrapping_add(6) as i32).wrapping_shl(16))
            | ((*d.wrapping_add(7) as i32).wrapping_shl(24));
        data |= (hi as isize as usize).wrapping_shl(16).wrapping_shl(16);

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siproundf!();
        }
        v0 ^= data;

        i += core::mem::size_of::<usize>();
        d = d.wrapping_add(core::mem::size_of::<usize>());
    }

    data = len.wrapping_shl(STBDS_SIZE_T_BITS - 8);
    // switch (len - i) with fall-through from 7 down to 1
    let rem = len.wrapping_sub(i);
    if rem >= 7 {
        data |= ((*d.wrapping_add(6) as usize).wrapping_shl(24)).wrapping_shl(24);
    }
    if rem >= 6 {
        data |= ((*d.wrapping_add(5) as usize).wrapping_shl(20)).wrapping_shl(20);
    }
    if rem >= 5 {
        data |= ((*d.wrapping_add(4) as usize).wrapping_shl(16)).wrapping_shl(16);
    }
    if rem >= 4 {
        data |= ((*d.wrapping_add(3) as i32).wrapping_shl(24)) as isize as usize;
    }
    if rem >= 3 {
        data |= ((*d.wrapping_add(2) as i32).wrapping_shl(16)) as isize as usize;
    }
    if rem >= 2 {
        data |= ((*d.wrapping_add(1) as i32).wrapping_shl(8)) as isize as usize;
    }
    if rem >= 1 {
        data |= *d.wrapping_add(0) as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siproundf!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siproundf!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// Hash index
// ---------------------------------------------------------------------------

#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a).wrapping_sub(1)) & !(a.wrapping_sub(1))
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline(always)]
fn load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp: usize = v64_lo ^ v32;
    temp = temp.wrapping_shl(16);
    temp = temp.wrapping_shl(16);
    temp = temp.wrapping_shr(16);
    temp = temp.wrapping_shr(16);
    let mut var: usize = v64_hi;
    var = var.wrapping_shl(16);
    var = var.wrapping_shl(16);
    var ^= temp ^ v32;
    var
}

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

    (*t).storage = align_fwd(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE)
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
    stbds_assert(
        (*t).used_count_threshold.wrapping_add((*t).tombstone_count_threshold) < (*t).slot_count,
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
        (*t).seed = hash_seed_get();
        let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = load_32_or_64(715136305, 0, 0xb504f32d);
        hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b = (*t).storage.wrapping_add(i);
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
            let ob = (*ot).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket = (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
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
        let slot = byte_add(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *mut *mut c_char;
        0 == strcmp(key as *const c_char, *slot)
    } else {
        0 == memcmp(
            key as *const c_void,
            byte_add(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *const c_void,
            keysize,
        )
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
                let slot = byte_add(a, elemsize.wrapping_mul(i)) as *mut *mut c_char;
                free(*slot as *mut c_void);
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
    let raw_a = hash_to_arr(a, elemsize);
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
        let bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                ) {
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
                ) {
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
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize)
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
                    .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
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
    set_temp(hash_to_arr(p, elemsize), temp);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let base = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        a = stbds_arrgrowf(base, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
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
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    raw_a = a;
    a = hash_to_arr(a, elemsize);

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
        (*stbds_header(a)).hash_table = nt as *mut c_void;
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
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
                    ) {
                        set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let slot = byte_add(
                                raw_a,
                                elemsize
                                    .wrapping_mul((*bucket).index[i] as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char;
                            set_temp_key(a, *slot);
                        }
                        return arr_to_hash(a, elemsize);
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
                    ) {
                        set_temp(a, (*bucket).index[i]);
                        return arr_to_hash(a, elemsize);
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
            raw_a = arr_to_hash(a, elemsize);
            let _ = raw_a;

            stbds_assert((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            set_temp(a, i - 1);

            let dst = byte_add(a, elemsize.wrapping_mul(i as usize));
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let s = stbds_strdup(key as *mut c_char);
                    *(dst as *mut *mut c_char) = s;
                    set_temp_key(a, s);
                }
                STBDS_SH_ARENA => {
                    let s = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *(dst as *mut *mut c_char) = s;
                    set_temp_key(a, s);
                }
                STBDS_SH_DEFAULT => {
                    let s = key as *mut c_char;
                    *(dst as *mut *mut c_char) = s;
                    set_temp_key(a, s);
                }
                _ => {
                    memcpy(dst, key as *const c_void, keysize);
                }
            }
        }
        arr_to_hash(a, elemsize)
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
    arr_to_hash(a, elemsize)
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
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    set_temp(raw_a, 0);
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table)
        .storage
        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
    let old_index = (*b).index[i as usize];
    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
    stbds_assert(slot < (*table).slot_count as isize);
    (*table).used_count = (*table).used_count.wrapping_sub(1);
    (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
    set_temp(raw_a, 1);
    (*b).hash[i as usize] = STBDS_HASH_DELETED;
    (*b).index[i as usize] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let slot_ptr =
            byte_add(a, elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_char;
        free(*slot_ptr as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            byte_add(a, elemsize.wrapping_mul(old_index as usize)),
            byte_add(a, elemsize.wrapping_mul(final_index as usize)) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let kp = byte_add(
                a,
                elemsize
                    .wrapping_mul(old_index as usize)
                    .wrapping_add(keyoffset),
            ) as *mut *mut c_char;
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                *kp as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        } else {
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                byte_add(
                    a,
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                ),
                keysize,
                keyoffset,
                mode,
            );
        }
        stbds_assert(slot >= 0);
        b = (*table)
            .storage
            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        stbds_assert((*b).index[i as usize] == final_index);
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

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(str_) + 1;
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

    stbds_assert(len <= (*a).remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len);
    (*a).remaining -= len;
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
// Test helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

/// `static char buffer[256];` used by `strkey`.
static mut STRKEY_BUFFER: [u8; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    // sprintf(buffer, "test_%d", n);
    let buf = ptr::addr_of_mut!(STRKEY_BUFFER) as *mut u8;
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    memcpy(
        buf as *mut c_void,
        bytes.as_ptr() as *const c_void,
        bytes.len(),
    );
    *buf.wrapping_add(bytes.len()) = 0;
    buf as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let elemsize = core::mem::size_of::<c_int>();

    // STBDS_ASSERT(arrlen(arr)==0);
    stbds_assert(stbds_arrlen(arr as *mut c_void) == 0);

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            // arrpush(arr, j) == stbds_arrmaybegrow(arr,1), arr[len++] = j
            if arr.is_null()
                || (*stbds_header(arr as *mut c_void)).length + 1
                    > (*stbds_header(arr as *mut c_void)).capacity
            {
                arr = stbds_arrgrowf(arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
            }
            let len = (*stbds_header(arr as *mut c_void)).length;
            *arr.wrapping_add(len) = j;
            (*stbds_header(arr as *mut c_void)).length = len + 1;
            j += 1;
        }
        // arrfree(arr)
        if !arr.is_null() {
            free(stbds_header(arr as *mut c_void) as *mut c_void);
        }
        arr = ptr::null_mut();
        i = i.wrapping_add(50);
    }
}
