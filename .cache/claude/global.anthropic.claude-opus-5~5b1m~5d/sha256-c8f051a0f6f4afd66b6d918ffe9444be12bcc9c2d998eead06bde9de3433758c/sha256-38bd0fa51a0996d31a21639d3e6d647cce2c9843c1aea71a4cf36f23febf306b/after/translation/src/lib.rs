//! Rust translation of the C library in `c_src/` (stb_ds.h based dynamic array /
//! hash map implementation plus the small `strkey` / `arr_ins` helpers).
//!
//! The translation is byte-for-byte behaviour compatible with the C original:
//!
//! * every memory block keeps the exact same layout (the public C header macros
//!   poke directly at `stbds_array_header`, so the header must stay 32 bytes with
//!   the same field order),
//! * every allocation goes through libc `realloc`/`free` so that blocks created
//!   here can be released by the C macros (`arrfree`, ...) and vice versa,
//! * all integer arithmetic reproduces the C semantics exactly, including the
//!   sign-extension quirks of `d[3] << 24` in the siphash loader, the
//!   `hash ^= hash ^ ROTR(...)` no-ops and the wrap-around behaviour.
//!
//! No bugs of the original were fixed.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/// `STBDS_REALLOC(c,p,s)` -> `realloc(p,s)`
#[inline(always)]
unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    unsafe { realloc(p, s) }
}

/// `STBDS_FREE(c,p)` -> `free(p)`
#[inline(always)]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

#[inline(always)]
unsafe fn c_memset(dst: *mut c_void, val: u8, n: usize) {
    unsafe { ptr::write_bytes(dst as *mut u8, val, n) }
}

// ---------------------------------------------------------------------------
// Data layout (must match the C structures bit for bit)
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
#[derive(Clone, Copy)]
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

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

const HEADER_SIZE: usize = size_of::<stbds_array_header>();

// Compile time layout checks mirroring the C struct sizes on LP64.
const _: () = assert!(size_of::<stbds_array_header>() == 32);
const _: () = assert!(size_of::<stbds_string_block>() == 16);
const _: () = assert!(size_of::<stbds_string_arena>() == 24);
const _: () = assert!(size_of::<stbds_hash_bucket>() == 128);
const _: () = assert!(size_of::<stbds_hash_index>() == 104);
const _: () = assert!(size_of::<usize>() == 8, "siphash 2-4 requires a 64 bit build");

// ---------------------------------------------------------------------------
// pointer / accessor helpers (all plain address arithmetic, like the C macros)
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *) (t) - 1)`
#[inline(always)]
fn stbds_header(t: *const c_void) -> *mut stbds_array_header {
    (t as usize).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `(char *) p + n`
#[inline(always)]
fn byte_add(p: *const c_void, n: usize) -> *mut c_void {
    (p as usize).wrapping_add(n) as *mut c_void
}

/// `(char *) p - n`
#[inline(always)]
fn byte_sub(p: *const c_void, n: usize) -> *mut c_void {
    (p as usize).wrapping_sub(n) as *mut c_void
}

/// `STBDS_HASH_TO_ARR(x,elemsize)`
#[inline(always)]
fn stbds_hash_to_arr(x: *const c_void, elemsize: usize) -> *mut c_void {
    byte_sub(x, elemsize)
}

/// `STBDS_ARR_TO_HASH(x,elemsize)`
#[inline(always)]
fn stbds_arr_to_hash(x: *const c_void, elemsize: usize) -> *mut c_void {
    byte_add(x, elemsize)
}

/// `stbds_arrlen(a)`
#[inline(always)]
unsafe fn stbds_arrlen(a: *const c_void) -> isize {
    if !a.is_null() {
        unsafe { (*stbds_header(a)).length as isize }
    } else {
        0
    }
}

/// `stbds_arrcap(a)`
#[inline(always)]
unsafe fn stbds_arrcap(a: *const c_void) -> usize {
    if !a.is_null() {
        unsafe { (*stbds_header(a)).capacity }
    } else {
        0
    }
}

/// `stbds_temp(t) = v`
#[inline(always)]
unsafe fn set_stbds_temp(t: *const c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

/// `stbds_temp_key(t) = v`  (`*(char **) stbds_header(t)->hash_table`)
#[inline(always)]
unsafe fn set_stbds_temp_key(t: *const c_void, v: *mut c_char) {
    unsafe {
        let table = (*stbds_header(t)).hash_table as *mut *mut c_char;
        *table = v;
    }
}

/// `stbds_hash_table(a)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *const c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `STBDS_ROTATE_LEFT(val, n)`
#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ROTATE_RIGHT(val, n)`
#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ALIGN_FWD(n,a)`
#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

// ---------------------------------------------------------------------------
// dynamic array growth
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

        let old = if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        };
        let b = stbds_realloc(
            old,
            elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
        );
        let b = byte_add(b, HEADER_SIZE);
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
// hash index
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { stbds_hash_seed = seed }
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

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline(always)]
fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp: usize;
    let mut var: usize;
    temp = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    var = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ v32;
    var
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = stbds_realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT)
                .wrapping_mul(size_of::<stbds_hash_bucket>())
                .wrapping_add(size_of::<stbds_hash_index>())
                .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
        ) as *mut stbds_hash_index;

        (*t).storage = align_fwd(
            (t as usize).wrapping_add(size_of::<stbds_hash_index>()),
            STBDS_CACHE_LINE_SIZE,
        ) as *mut stbds_hash_bucket;
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
        debug_assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            c_memset(
                &mut (*t).string as *mut stbds_string_arena as *mut c_void,
                0,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = stbds_hash_seed;
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
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
                            for z in 0..limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'probe;
                                }
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
}

// ---------------------------------------------------------------------------
// hashing
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut str_ = str_;
        let mut hash = seed;
        while *str_ != 0 {
            let c = *str_ as u8 as usize;
            hash = rotate_left(hash, 9).wrapping_add(c);
            str_ = str_.add(1);
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
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline(always)]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
    *v3 ^= *v0;
}

/// Reproduces `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);` where the
/// right hand side is computed in `int` and then (possibly sign-) extended to
/// `size_t` -- for `d[3] >= 0x80` the C expression is negative and therefore sets
/// every high bit of the resulting `size_t`.
#[inline(always)]
unsafe fn load_int_le32(d: *const u8) -> usize {
    unsafe {
        let v: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32) << 24);
        v as i64 as u64 as usize
    }
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;
        let mut i: usize;
        let mut data: usize;

        let mut v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        let mut v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        let mut v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        let mut v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v0 ^= 0x0706050403020100u64 as usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;
        v2 ^= 0x0706050403020100u64 as usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908u64 as usize ^ !seed;

        i = 0;
        while i.wrapping_add(size_of::<usize>()) <= len {
            data = load_int_le32(d);
            // discarded if size_t == 4
            data |= (load_int_le32(d.add(4)) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
            }
            v0 ^= data;

            i = i.wrapping_add(size_of::<usize>());
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        // C switch with fall-through: case k performs the work of cases k..1.
        let rem = len.wrapping_sub(i);
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
            // int expression -> sign extended when d[3] >= 0x80
            data |= (((*d.add(3) as i32) << 24) as i64 as u64) as usize;
        }
        if rem >= 3 {
            data |= ((*d.add(2) as i32) << 16) as usize;
        }
        if rem >= 2 {
            data |= ((*d.add(1) as i32) << 8) as usize;
        }
        if rem >= 1 {
            data |= *d.add(0) as usize;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// hash map
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
        let slot = byte_add(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset));
        if mode >= STBDS_HM_STRING {
            (0 == strcmp(key as *const c_char, *(slot as *mut *const c_char))) as c_int
        } else {
            (0 == memcmp(key, slot, keysize)) as c_int
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
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    stbds_free(*(byte_add(a, elemsize.wrapping_mul(i)) as *mut *mut c_void));
                    i += 1;
                }
            }
            stbds_strreset(&mut (*stbds_hash_table(a)).string);
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
            for i in 0..limit {
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
            }

            pos = pos.wrapping_add(step);
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
            pos &= (*table).slot_count.wrapping_sub(1);
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
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            c_memset(a, 0, elemsize);
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
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        set_stbds_temp(stbds_hash_to_arr(p, elemsize), temp);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
            let base = if !a.is_null() {
                stbds_hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            };
            let na = stbds_arrgrowf(base, elemsize, 0, 1);
            (*stbds_header(na)).length += 1;
            c_memset(na, 0, elemsize);
            a = stbds_arr_to_hash(na, elemsize);
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
        let keyoffset: usize = 0;
        let mut a = a;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            c_memset(a, 0, elemsize);
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
                            set_stbds_temp(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let kp = byte_add(
                                    raw_a,
                                    elemsize
                                        .wrapping_mul((*bucket).index[i] as usize)
                                        .wrapping_add(keyoffset),
                                ) as *mut *mut c_char;
                                set_stbds_temp_key(a, *kp);
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
                            set_stbds_temp(a, (*bucket).index[i]);
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

                debug_assert!((i as usize) + 1 <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                set_stbds_temp(a, i - 1);

                let slot =
                    byte_add(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *slot = p;
                        set_stbds_temp_key(a, p);
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                        *slot = p;
                        set_stbds_temp_key(a, p);
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *slot = p;
                        set_stbds_temp_key(a, p);
                    }
                    _ => {
                        memcpy(slot as *mut c_void, key, keysize);
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
        c_memset(a, 0, elemsize);
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
        set_stbds_temp(raw_a, 0);
        if table.is_null() {
            return a;
        }

        let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }

        let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        debug_assert!(slot < (*table).slot_count as isize);
        (*table).used_count = (*table).used_count.wrapping_sub(1);
        (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
        set_stbds_temp(raw_a, 1);
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            stbds_free(
                *(byte_add(a, elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_void),
            );
        }

        if old_index != final_index {
            memmove(
                byte_add(a, elemsize.wrapping_mul(old_index as usize)),
                byte_add(a, elemsize.wrapping_mul(final_index as usize)),
                elemsize,
            );

            if mode == STBDS_HM_STRING {
                let kp = byte_add(
                    a,
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                ) as *mut *mut c_void;
                slot = stbds_hm_find_slot(a, elemsize, *kp, keysize, keyoffset, mode);
            } else {
                let kp = byte_add(
                    a,
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                );
                slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
            }
            debug_assert!(slot >= 0);
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            debug_assert!((*b).index[i] == final_index);
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

        a
    }
}

// ---------------------------------------------------------------------------
// string storage
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, str_ as *const c_void, len);
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
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    (size_of::<stbds_string_block>() - 8).wrapping_add(len),
                ) as *mut stbds_string_block;
                memmove(
                    (*sb).storage.as_mut_ptr() as *mut c_void,
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
                return (*sb).storage.as_mut_ptr();
            } else {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    (size_of::<stbds_string_block>() - 8).wrapping_add(blocksize),
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        debug_assert!(len <= (*a).remaining);
        let base = (*(*a).storage).storage.as_mut_ptr() as usize;
        let p = base.wrapping_add((*a).remaining).wrapping_sub(len) as *mut c_char;
        (*a).remaining = (*a).remaining.wrapping_sub(len);
        memmove(p as *mut c_void, str_ as *const c_void, len);
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
        c_memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// test helpers exported by the C library
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

/// `sprintf(buffer, "test_%d", n); return buffer;`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = &raw mut buffer as *mut c_char;
        let mut pos: usize = 0;

        for &c in b"test_" {
            *buf.add(pos) = c as c_char;
            pos += 1;
        }

        let v = n as i64;
        if v < 0 {
            *buf.add(pos) = b'-' as c_char;
            pos += 1;
        }
        let mut mag = v.unsigned_abs();
        let mut digits = [0u8; 20];
        let mut ndigits = 0usize;
        loop {
            digits[ndigits] = b'0' + (mag % 10) as u8;
            ndigits += 1;
            mag /= 10;
            if mag == 0 {
                break;
            }
        }
        while ndigits > 0 {
            ndigits -= 1;
            *buf.add(pos) = digits[ndigits] as c_char;
            pos += 1;
        }
        *buf.add(pos) = 0;

        buf
    }
}

/// Expansion of the C body:
/// ```c
/// void arr_ins(int num) {
///   int *arr=NULL; int i,j;
///   for (i=0; i < 5; ++i) {
///     arrpush(arr,1); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
///     stbds_arrins(arr,i,num);
///     STBDS_ASSERT(arr[i] == num);
///     if (i < 4) STBDS_ASSERT(arr[4] == 4);
///     arrfree(arr);
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    unsafe {
        const ELEMSIZE: usize = size_of::<c_int>();

        let mut arr: *mut c_int = ptr::null_mut();

        for i in 0..5isize {
            // arrpush(arr, 1..4)
            for v in 1..=4 as c_int {
                // stbds_arrmaybegrow(a,1)
                if arr.is_null()
                    || (*stbds_header(arr as *const c_void)).length + 1
                        > (*stbds_header(arr as *const c_void)).capacity
                {
                    arr = stbds_arrgrowf(arr as *mut c_void, ELEMSIZE, 1, 0) as *mut c_int;
                }
                // (a)[stbds_header(a)->length++] = v
                let h = stbds_header(arr as *const c_void);
                *arr.add((*h).length) = v;
                (*h).length += 1;
            }

            // stbds_arrins(arr,i,num) == stbds_arrinsn(arr,i,1), arr[i] = num
            //   stbds_arrinsn -> stbds_arraddn(arr,1) then memmove
            {
                // stbds_arraddn(a,1) == (void) stbds_arraddnindex(a,1)
                if arr.is_null()
                    || (*stbds_header(arr as *const c_void)).length + 1
                        > (*stbds_header(arr as *const c_void)).capacity
                {
                    arr = stbds_arrgrowf(arr as *mut c_void, ELEMSIZE, 1, 0) as *mut c_int;
                }
                let h = stbds_header(arr as *const c_void);
                (*h).length += 1;

                // memmove(&a[i+1], &a[i], sizeof *a * (length - 1 - i))
                let n: usize = 1;
                let count = (*h)
                    .length
                    .wrapping_sub(n)
                    .wrapping_sub(i as usize);
                memmove(
                    arr.offset(i + n as isize) as *mut c_void,
                    arr.offset(i) as *const c_void,
                    ELEMSIZE.wrapping_mul(count),
                );
                *arr.offset(i) = num;
            }

            debug_assert!(*arr.offset(i) == num);
            if i < 4 {
                debug_assert!(*arr.offset(4) == 4);
            }

            // arrfree(arr)
            if !arr.is_null() {
                stbds_free(stbds_header(arr as *const c_void) as *mut c_void);
            }
            arr = ptr::null_mut();
        }
    }
}
