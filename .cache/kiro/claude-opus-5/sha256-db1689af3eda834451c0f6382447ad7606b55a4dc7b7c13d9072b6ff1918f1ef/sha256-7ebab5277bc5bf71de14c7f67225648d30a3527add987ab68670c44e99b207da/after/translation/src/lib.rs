//! Rust translation of `c_src/src/lib.c` — a trimmed copy of Sean Barrett's
//! `stb_ds.h` dynamic array / hash map implementation plus the `str_dups`
//! driver function declared in `c_src/include/lib.h`.
//!
//! The translation is deliberately literal: it keeps the same memory layout,
//! the same allocator (libc `realloc`/`free`), the same probing order, the
//! same integer-overflow / sign-extension quirks and the same order of
//! validation so that the produced shared object is a drop-in replacement
//! that yields byte-identical output.

#![allow(non_camel_case_types)]

use core::ffi::{CStr, c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings
//
// The C code uses `realloc`/`free` through STBDS_REALLOC/STBDS_FREE and uses
// `printf`/`sprintf` for output. We bind those directly so allocation and
// stdio buffering behaviour are unchanged.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn abort() -> !;
    #[link_name = "printf"]
    fn printf_ss(fmt: *const c_char, a: *const c_char, b: c_int) -> c_int;
    #[link_name = "sprintf"]
    fn sprintf_i(buf: *mut c_char, fmt: *const c_char, n: c_int) -> c_int;
}

/// `STBDS_ASSERT` is `assert` and `NDEBUG` is not defined by the CMake build,
/// so a failing check aborts the process.
#[inline]
fn stbds_assert(cond: bool) {
    if !cond {
        unsafe { abort() }
    }
}

#[inline]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}

#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

#[inline]
unsafe fn memmove_bytes(dst: *mut u8, src: *const u8, n: usize) {
    unsafe { ptr::copy(src, dst, n) }
}

#[inline]
unsafe fn memset_zero(dst: *mut u8, n: usize) {
    unsafe { ptr::write_bytes(dst, 0, n) }
}

#[inline]
unsafe fn strlen(s: *const c_char) -> usize {
    unsafe { CStr::from_ptr(s).to_bytes().len() }
}

/// `0 == strcmp(a, b)`
#[inline]
unsafe fn str_eq(a: *const c_char, b: *const c_char) -> bool {
    unsafe {
        let mut i = 0isize;
        loop {
            let ca = *a.offset(i) as u8;
            let cb = *b.offset(i) as u8;
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
#[inline]
unsafe fn mem_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    unsafe {
        for i in 0..n {
            if *a.add(i) != *b.add(i) {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Data layout (must match the C structs bit for bit)
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

const STBDS_HM_BINARY: c_int = 0;
const _: c_int = STBDS_HM_BINARY; // mirrors the C macro; unused by this driver
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

// ---------------------------------------------------------------------------
// Array header accessors: `stbds_header(t)` is `((stbds_array_header *)(t) - 1)`
// ---------------------------------------------------------------------------

#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
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

/// `stbds_temp(t)` — `stbds_header(t)->temp`
#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

#[inline]
unsafe fn stbds_temp_get(t: *mut c_void) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

/// `stbds_temp_key(t)` — `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_ptr(t: *mut c_void) -> *mut *mut c_char {
    unsafe { (*stbds_header(t)).hash_table as *mut *mut c_char }
}

/// `STBDS_HASH_TO_ARR(x, elemsize)`
#[inline]
fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

/// `STBDS_ARR_TO_HASH(x, elemsize)`
#[inline]
fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
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
        // size_t min_len = stbds_arrlen(a) + addlen;
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

        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_header(a) as *mut c_void
        };
        // STBDS_REALLOC(c,p,s) == realloc(p,s)
        let mut b = stbds_realloc(
            old,
            elemsize
                .wrapping_mul(min_cap)
                .wrapping_add(size_of::<stbds_array_header>()),
        );

        b = (b as *mut u8).add(size_of::<stbds_array_header>()) as *mut c_void;
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
// Hash seed / index construction
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x3141_5926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { STBDS_HASH_SEED = seed }
}

#[inline]
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
            (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
                + size_of::<stbds_hash_index>()
                + STBDS_CACHE_LINE_SIZE
                - 1,
        ) as *mut stbds_hash_index;

        let past_end = (t as usize).wrapping_add(size_of::<stbds_hash_index>());
        (*t).storage = stbds_align_fwd(past_end, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
        stbds_assert(
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
        );

        if !ot.is_null() {
            ptr::write(
                &raw mut (*t).string,
                stbds_string_arena {
                    storage: (*ot).string.storage,
                    remaining: (*ot).string.remaining,
                    block: (*ot).string.block,
                    mode: (*ot).string.mode,
                },
            );
            (*t).seed = (*ot).seed;
        } else {
            memset_zero(
                (&raw mut (*t).string) as *mut u8,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
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
fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotr(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut s = str_;
        while *s != 0 {
            hash = rotl(hash, 9).wrapping_add(*s as u8 as usize);
            s = s.add(1);
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
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! siproundv {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotl($v1, 13);
        $v1 ^= $v0;
        $v0 = rotl($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotl($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotl($v1, 17);
        $v1 ^= $v2;
        $v2 = rotl($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotl($v3, 21);
        $v3 ^= $v0;
    }};
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;

        let mut v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        let mut v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        let mut v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        let mut v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut data: usize;
        let mut i = 0usize;
        while i + size_of::<usize>() <= len {
            // The C expression is evaluated in `int` and then converted to
            // size_t, which sign-extends when byte 3 (resp. byte 7) has its
            // high bit set. Reproduced exactly.
            let lo = (d.add(0).read() as i32)
                | ((d.add(1).read() as i32) << 8)
                | ((d.add(2).read() as i32) << 16)
                | ((d.add(3).read() as i32) << 24);
            data = lo as usize;
            let hi = (d.add(4).read() as i32)
                | ((d.add(5).read() as i32) << 8)
                | ((d.add(6).read() as i32) << 16)
                | ((d.add(7).read() as i32) << 24);
            data |= ((hi as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siproundv!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        // switch (len - i) with fall-through from case 7 down to case 1
        let rem = len - i;
        if rem >= 7 {
            data |= ((d.add(6).read() as usize) << 24) << 24;
        }
        if rem >= 6 {
            data |= ((d.add(5).read() as usize) << 20) << 20;
        }
        if rem >= 5 {
            data |= ((d.add(4).read() as usize) << 16) << 16;
        }
        if rem >= 4 {
            data |= ((d.add(3).read() as i32) << 24) as usize;
        }
        if rem >= 3 {
            data |= ((d.add(2).read() as i32) << 16) as usize;
        }
        if rem >= 2 {
            data |= ((d.add(1).read() as i32) << 8) as usize;
        }
        if rem >= 1 {
            data |= d.add(0).read() as usize;
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siproundv!(v0, v1, v2, v3);
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            siproundv!(v0, v1, v2, v3);
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
            str_eq(key as *const c_char, *(slot as *mut *mut c_char))
        } else {
            mem_eq(key as *const u8, slot, keysize)
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
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
                let mut i = 1usize;
                while i < (*stbds_header(a)).length {
                    let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                    stbds_free(*p as *mut c_void);
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
            memset_zero(a as *mut u8, elemsize);
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
        stbds_temp_set(hash_to_arr(p, elemsize), temp);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
            let base = if !a.is_null() {
                hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            };
            let g = stbds_arrgrowf(base, elemsize, 0, 1);
            (*stbds_header(g)).length += 1;
            memset_zero(g as *mut u8, elemsize);
            a = arr_to_hash(g, elemsize);
        }
        a
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        memmove_bytes(p as *mut u8, str_ as *const u8, len);
        p
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
            let g = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset_zero(g as *mut u8, elemsize);
            (*stbds_header(g)).length += 1;
            a = arr_to_hash(g, elemsize);
        }

        let mut raw_a = a;
        let mut a = hash_to_arr(a, elemsize);

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
                    STBDS_SH_DEFAULT as u8
                } else {
                    STBDS_SH_NONE as u8
                };
            }
            table = nt;
            (*stbds_header(a)).hash_table = nt as *mut c_void;
        }

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
                            let src = (raw_a as *mut u8)
                                .offset((elemsize as isize) * (*bucket).index[i])
                                .add(keyoffset) as *mut *mut c_char;
                            *stbds_temp_key_ptr(a) = *src;
                        }
                        return arr_to_hash(a, elemsize);
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
                        return arr_to_hash(a, elemsize);
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
            let i = stbds_arrlen(a);
            if (i as usize) + 1 > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);
            let _ = raw_a;

            stbds_assert((i as usize) + 1 <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_temp_set(a, i - 1);

            let elem_key_slot =
                (a as *mut u8).offset((elemsize as isize).wrapping_mul(i)) as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let dup = stbds_strdup(key as *mut c_char);
                    *elem_key_slot = dup;
                    *stbds_temp_key_ptr(a) = dup;
                }
                STBDS_SH_ARENA => {
                    let s = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                    *elem_key_slot = s;
                    *stbds_temp_key_ptr(a) = s;
                }
                STBDS_SH_DEFAULT => {
                    let s = key as *mut c_char;
                    *elem_key_slot = s;
                    *stbds_temp_key_ptr(a) = s;
                }
                _ => {
                    memmove_bytes(
                        (a as *mut u8).offset((elemsize as isize).wrapping_mul(i)),
                        key as *const u8,
                        keysize,
                    );
                }
            }
        }
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset_zero(a as *mut u8, elemsize);
        (*stbds_header(a)).length = 1;
        let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        arr_to_hash(a, elemsize)
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
        let raw_a = hash_to_arr(a, elemsize);
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
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        stbds_assert(slot < (*table).slot_count as isize);
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        stbds_temp_set(raw_a, 1);
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
            let p = (a as *mut u8).offset((elemsize as isize).wrapping_mul(old_index))
                as *mut *mut c_char;
            stbds_free(*p as *mut c_void);
        }

        if old_index != final_index {
            memmove_bytes(
                (a as *mut u8).offset((elemsize as isize).wrapping_mul(old_index)),
                (a as *const u8).offset((elemsize as isize).wrapping_mul(final_index)),
                elemsize,
            );

            let moved = (a as *mut u8)
                .offset((elemsize as isize).wrapping_mul(old_index))
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
            stbds_assert(slot >= 0);
            b = (*table).storage.offset(slot >> STBDS_BUCKET_SHIFT);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            stbds_assert((*b).index[i] == final_index);
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
// String arena
// ---------------------------------------------------------------------------

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

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                memmove_bytes(
                    (&raw mut (*sb).storage) as *mut c_char as *mut u8,
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
                    size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        stbds_assert(len <= (*a).remaining);
        let p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .add((*a).remaining)
            .sub(len);
        (*a).remaining -= len;
        memmove_bytes(p as *mut u8, str_ as *const u8, len);
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
        memset_zero(a as *mut u8, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// Driver code
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut BUFFER) as *mut c_char;
        sprintf_i(buf, c"test_%d".as_ptr(), n);
        buf
    }
}

/// `struct { char *key; int value; }` — the anonymous string-map element type
/// used by `str_dups`.
#[repr(C)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(num: c_int) {
    unsafe {
        let elemsize = size_of::<StrMapEntry>();
        let mut strmap: *mut StrMapEntry;
        let mut sa = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };

        let mut i: c_int = 0;
        while i < num {
            stbds_stralloc(&raw mut sa, strkey(i));
            i += 1;
        }
        stbds_strreset(&raw mut sa);

        {
            let s = StrMapEntry {
                key: c"a".as_ptr() as *mut c_char,
                value: num,
            };

            // sh_new_strdup(strmap)
            strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP) as *mut StrMapEntry;

            // shputs(strmap, s)
            strmap = stbds_hmput_key(
                strmap as *mut c_void,
                elemsize,
                s.key as *mut c_void,
                size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            ) as *mut StrMapEntry;
            let raw_a = strmap.offset(-1) as *mut c_void;
            let t = stbds_temp_get(raw_a);
            ptr::write(
                strmap.offset(t),
                StrMapEntry {
                    key: s.key,
                    value: s.value,
                },
            );
            let t = stbds_temp_get(raw_a);
            (*strmap.offset(t)).key = *stbds_temp_key_ptr(raw_a);

            stbds_assert(*(*strmap.offset(0)).key == b'a' as c_char);
            stbds_assert((*strmap.offset(0)).key != s.key);
            stbds_assert((*strmap.offset(0)).value == s.value);

            // shlen(strmap)
            let len = if strmap.is_null() {
                0
            } else {
                (*stbds_header(strmap.offset(-1) as *mut c_void)).length as isize - 1
            };

            let mut z: isize = 0;
            while z < len {
                let e = strmap.offset(z);
                // The C source passes the whole struct where `%s` is expected:
                //   printf("%s %d\n", strmap[z], strmap[z].value);
                // Under the SysV x86-64 ABI the 16-byte struct occupies the
                // next two integer argument registers, so `%s` consumes
                // `key` and `%d` consumes `value`; the third argument is
                // never read. Reproduced literally here.
                printf_ss(c"%s %d\n".as_ptr(), (*e).key, (*e).value);
                z += 1;
            }

            // shfree(strmap)
            if !strmap.is_null() {
                stbds_hmfree_func(strmap.offset(-1) as *mut c_void, elemsize);
            }
            strmap = ptr::null_mut();
            let _ = strmap;
        }
    }
}
