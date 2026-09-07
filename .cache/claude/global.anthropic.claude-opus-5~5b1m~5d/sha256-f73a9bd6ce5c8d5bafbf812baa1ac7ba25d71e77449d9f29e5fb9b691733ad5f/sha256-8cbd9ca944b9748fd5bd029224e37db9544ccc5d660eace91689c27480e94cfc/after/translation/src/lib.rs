// Rust translation of c_src/src/lib.c (stb_ds.h implementation + test driver).
//
// The C library globs src/lib.c into a shared object exporting these 16 public
// symbols:
//   stbds_arrgrowf, stbds_arrfreef, stbds_rand_seed, stbds_hash_string,
//   stbds_hash_bytes, stbds_hmget_key_ts, stbds_hmget_key, stbds_hmput_default,
//   stbds_shmode_func, stbds_hmdel_key, stbds_stralloc, stbds_hmput_key,
//   stbds_strreset, stbds_hmfree_func, strkey, hm_geti
//
// Every quirk of the original (including integer-promotion sign-extension in
// the siphash byte gather, and the missing temp_key store in the second probe
// loop of stbds_hmput_key) is reproduced verbatim.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (STBDS_REALLOC(c,p,s) -> realloc(p,s), STBDS_FREE(c,p) -> free(p))
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
}

/// `assert()` from <assert.h>.  The original library is normally compiled with
/// assertions live; a failing assertion aborts the process.
#[inline]
fn STBDS_ASSERT(cond: bool) {
    if !cond {
        std::process::abort();
    }
}

// ---------------------------------------------------------------------------
// Types (verified byte-for-byte against the C layout)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
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

const SIZEOF_ARRAY_HEADER: usize = core::mem::size_of::<stbds_array_header>();
const SIZEOF_HASH_INDEX: usize = core::mem::size_of::<stbds_hash_index>();
const SIZEOF_HASH_BUCKET: usize = core::mem::size_of::<stbds_hash_bucket>();
const SIZEOF_STRING_ARENA: usize = core::mem::size_of::<stbds_string_arena>();
const SIZEOF_STRING_BLOCK: usize = core::mem::size_of::<stbds_string_block>();

// Compile-time layout checks mirroring the C ABI.
const _: () = assert!(SIZEOF_ARRAY_HEADER == 32);
const _: () = assert!(SIZEOF_HASH_INDEX == 104);
const _: () = assert!(SIZEOF_HASH_BUCKET == 128);
const _: () = assert!(SIZEOF_STRING_ARENA == 24);
const _: () = assert!(SIZEOF_STRING_BLOCK == 16);
// typedef int STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds[sizeof(size_t)==8?1:-1];
const _: () = assert!(core::mem::size_of::<usize>() == 8);

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const STBDS_SIZE_T_BITS: u32 = 64;

// ---------------------------------------------------------------------------
// Small helpers replacing the C macros
// ---------------------------------------------------------------------------

/// `stbds_header(t)` == `((stbds_array_header *) (t) - 1)`
#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(SIZEOF_ARRAY_HEADER) as *mut stbds_array_header
}

/// `stbds_arrcap(a)` == `((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if !a.is_null() {
        unsafe { (*stbds_header(a)).capacity }
    } else {
        0
    }
}

/// `stbds_arrlen(a)` == `((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if !a.is_null() {
        unsafe { (*stbds_header(a)).length as isize }
    } else {
        0
    }
}

/// `STBDS_HASH_TO_ARR(x,elemsize)` == `((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH(x,elemsize)` == `((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `stbds_hash_table(a)` == `((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `stbds_temp(t)` == `stbds_header(t)->temp` (read)
#[inline]
unsafe fn stbds_temp_get(t: *mut c_void) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

/// `stbds_temp(t) = v`
#[inline]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe {
        (*stbds_header(t)).temp = v;
    }
}

/// `stbds_temp_key(t) = v` == `(*(char **) stbds_header(t)->hash_table) = v`
#[inline]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    unsafe {
        *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
    }
}

/// `STBDS_ROTATE_LEFT(val, n)` == `(((val) << (n)) | ((val) >> (BITS - (n))))`
#[inline]
fn STBDS_ROTATE_LEFT(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ROTATE_RIGHT(val, n)` == `(((val) >> (n)) | ((val) << (BITS - (n))))`
#[inline]
fn STBDS_ROTATE_RIGHT(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

/// `STBDS_ALIGN_FWD(n,a)` == `(((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
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
    unsafe {
        let mut min_cap = min_cap;
        let b: *mut c_void;
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

        let old = if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            ptr::null_mut()
        };
        let raw = realloc(
            old,
            elemsize
                .wrapping_mul(min_cap)
                .wrapping_add(SIZEOF_ARRAY_HEADER),
        );
        b = (raw as *mut u8).wrapping_add(SIZEOF_ARRAY_HEADER) as *mut c_void;
        if a.is_null() {
            (*stbds_header(b)).length = 0;
            (*stbds_header(b)).hash_table = ptr::null_mut();
            (*stbds_header(b)).temp = 0;
        } else {
            // STBDS_STATS(++stbds_array_grow); -- compiled out
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
// Seeding / hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        stbds_hash_seed = seed;
    }
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    // STBDS_NOTUSED(slot_log2);
    let pos = hash & (slot_count.wrapping_sub(1));
    pos
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

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT)
                .wrapping_mul(SIZEOF_HASH_BUCKET)
                .wrapping_add(SIZEOF_HASH_INDEX)
                .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
        ) as *mut stbds_hash_index;

        // t->storage = (stbds_hash_bucket *) STBDS_ALIGN_FWD((size_t) (t+1), STBDS_CACHE_LINE_SIZE);
        (*t).storage = STBDS_ALIGN_FWD(
            (t as usize).wrapping_add(SIZEOF_HASH_INDEX),
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
        STBDS_ASSERT(
            (*t)
                .used_count_threshold
                .wrapping_add((*t).tombstone_count_threshold)
                < (*t).slot_count,
        );

        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            let a: usize;
            let b: usize;
            let mut temp: usize;
            memset(
                (&raw mut (*t).string) as *mut c_void,
                0,
                SIZEOF_STRING_ARENA,
            );
            (*t).seed = stbds_hash_seed;
            // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
            {
                let v32: usize = 2147001325;
                let v64_hi: usize = 0x27bb2ee6;
                let v64_lo: usize = 0x87b0b0fd;
                temp = v64_lo ^ v32;
                temp <<= 16;
                temp <<= 16;
                temp >>= 16;
                temp >>= 16;
                let mut var: usize = v64_hi;
                var <<= 16;
                var <<= 16;
                var ^= temp ^ v32;
                a = var;
            }
            // stbds_load_32_or_64(b,temp,  715136305,          0, 0xb504f32d);
            {
                let v32: usize = 715136305;
                let v64_hi: usize = 0;
                let v64_lo: usize = 0xb504f32d;
                temp = v64_lo ^ v32;
                temp <<= 16;
                temp <<= 16;
                temp >>= 16;
                temp >>= 16;
                let mut var: usize = v64_hi;
                var <<= 16;
                var <<= 16;
                var ^= temp ^ v32;
                b = var;
            }
            stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
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
                    // STBDS_INDEX_IN_USE(x) -> ((x) >= 0)
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
}

// ---------------------------------------------------------------------------
// Hash functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash: usize = seed;
        let mut s = str_;
        while *s != 0 {
            hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as u8 as usize);
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
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *mut u8;
        let mut v0: usize;
        let mut v1: usize;
        let mut v2: usize;
        let mut v3: usize;
        let mut data: usize;

        v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        // STBDS_SIPROUND()
        macro_rules! STBDS_SIPROUND {
            () => {{
                v0 = v0.wrapping_add(v1);
                v1 = STBDS_ROTATE_LEFT(v1, 13);
                v1 ^= v0;
                v0 = STBDS_ROTATE_LEFT(v0, STBDS_SIZE_T_BITS / 2);
                v2 = v2.wrapping_add(v3);
                v3 = STBDS_ROTATE_LEFT(v3, 16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = STBDS_ROTATE_LEFT(v1, 17);
                v1 ^= v2;
                v2 = STBDS_ROTATE_LEFT(v2, STBDS_SIZE_T_BITS / 2);
                v0 = v0.wrapping_add(v3);
                v3 = STBDS_ROTATE_LEFT(v3, 21);
                v3 ^= v0;
            }};
        }

        let mut i: usize = 0;
        while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
            // NOTE: the C expression `d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24)`
            // has type `int`; when d[3] >= 0x80 it becomes negative and the
            // conversion to size_t sign-extends, filling the top 32 bits with
            // ones.  That behaviour is preserved exactly here.
            let lo: i32 = (*d.wrapping_add(0) as i32)
                | ((*d.wrapping_add(1) as i32) << 8)
                | ((*d.wrapping_add(2) as i32) << 16)
                | ((*d.wrapping_add(3) as i32) << 24);
            data = lo as usize;
            let hi: i32 = (*d.wrapping_add(4) as i32)
                | ((*d.wrapping_add(5) as i32) << 8)
                | ((*d.wrapping_add(6) as i32) << 16)
                | ((*d.wrapping_add(7) as i32) << 24);
            data |= ((hi as usize) << 16) << 16; // discarded if size_t == 4

            v3 ^= data;
            for _j in 0..STBDS_SIPHASH_C_ROUNDS {
                STBDS_SIPROUND!();
            }
            v0 ^= data;

            i = i.wrapping_add(core::mem::size_of::<usize>());
            d = d.wrapping_add(core::mem::size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        // switch (len - i) with C fall-through semantics
        let rem = len.wrapping_sub(i);
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
            // `(d[3] << 24)` is an int -> may sign-extend into the high 32 bits.
            data |= ((*d.wrapping_add(3) as i32) << 24) as usize;
        }
        if rem >= 3 {
            data |= ((*d.wrapping_add(2) as i32) << 16) as usize;
        }
        if rem >= 2 {
            data |= ((*d.wrapping_add(1) as i32) << 8) as usize;
        }
        if rem >= 1 {
            data |= *d.wrapping_add(0) as i32 as usize;
        }

        v3 ^= data;
        for _j in 0..STBDS_SIPHASH_C_ROUNDS {
            STBDS_SIPROUND!();
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _j in 0..STBDS_SIPHASH_D_ROUNDS {
            STBDS_SIPROUND!();
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
        if mode >= STBDS_HM_STRING {
            let slot = (a as *mut u8)
                .wrapping_add(elemsize.wrapping_mul(i))
                .wrapping_add(keyoffset) as *mut *mut c_char;
            (0 == strcmp(key as *mut c_char, *slot)) as c_int
        } else {
            let slot = (a as *mut u8)
                .wrapping_add(elemsize.wrapping_mul(i))
                .wrapping_add(keyoffset) as *mut c_void;
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
            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    free(
                        *((a as *mut u8).wrapping_add(elemsize.wrapping_mul(i)) as *mut *mut c_char)
                            as *mut c_void,
                    );
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
        }
        free((*stbds_header(a)).hash_table);
        free(stbds_header(a) as *mut c_void);
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
        let mut pos: usize;
        let mut bucket: *mut stbds_hash_bucket;

        if hash < 2 {
            hash = hash.wrapping_add(2);
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        loop {
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
            memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            stbds_arr_to_hash(a, elemsize)
        } else {
            let table: *mut stbds_hash_index;
            let raw_a = stbds_hash_to_arr(a, elemsize);
            table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
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
            memset(a, 0, elemsize);
            a = stbds_arr_to_hash(a, elemsize);
        }
        a
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_).wrapping_add(1);
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, str_ as *const c_void, len);
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
        let keyoffset: usize = 0;
        let mut a = a;
        let mut raw_a: *mut c_void;
        let mut table: *mut stbds_hash_index;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*stbds_header(a)).length += 1;
            a = stbds_arr_to_hash(a, elemsize);
        }

        raw_a = a;
        a = stbds_hash_to_arr(a, elemsize);

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
                    STBDS_SH_DEFAULT as u8
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
                        ) != 0
                        {
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let kp = *((raw_a as *mut u8)
                                    .wrapping_add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                    .wrapping_add(keyoffset)
                                    as *mut *mut c_char);
                                stbds_temp_key_set(a, kp);
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
                            // NOTE: unlike the loop above, the original does not
                            // update stbds_temp_key here.  Preserved verbatim.
                            stbds_temp_set(a, (*bucket).index[i]);
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
                (*table).tombstone_count -= 1;
            }
            (*table).used_count += 1;

            {
                let i: isize = stbds_arrlen(a);
                if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                    a = stbds_arrgrowf(a, elemsize, 1, 0);
                }
                raw_a = stbds_arr_to_hash(a, elemsize);
                let _ = raw_a;

                STBDS_ASSERT((i as usize).wrapping_add(1) <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let dst = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i as usize));
                match (*table).string.mode as c_int {
                    STBDS_SH_STRDUP => {
                        let p = stbds_strdup(key as *mut c_char);
                        *(dst as *mut *mut c_char) = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_ARENA => {
                        let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                        *(dst as *mut *mut c_char) = p;
                        stbds_temp_key_set(a, p);
                    }
                    STBDS_SH_DEFAULT => {
                        let p = key as *mut c_char;
                        *(dst as *mut *mut c_char) = p;
                        stbds_temp_key_set(a, p);
                    }
                    _ => {
                        memcpy(dst as *mut c_void, key as *const c_void, keysize);
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
        let h: *mut stbds_hash_index;
        memset(a, 0, elemsize);
        (*stbds_header(a)).length = 1;
        h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
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
            ptr::null_mut()
        } else {
            let table: *mut stbds_hash_index;
            let raw_a = stbds_hash_to_arr(a, elemsize);
            table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
            stbds_temp_set(raw_a, 0);
            if table.is_null() {
                a
            } else {
                let mut slot: isize;
                slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    a
                } else {
                    let mut b = (*table)
                        .storage
                        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    let old_index = (*b).index[i as usize];
                    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                    STBDS_ASSERT(slot < (*table).slot_count as isize);
                    (*table).used_count -= 1;
                    (*table).tombstone_count += 1;
                    stbds_temp_set(raw_a, 1);
                    STBDS_ASSERT(true); // STBDS_ASSERT(table->used_count >= 0) -- size_t, always true
                    (*b).hash[i as usize] = STBDS_HASH_DELETED;
                    (*b).index[i as usize] = STBDS_INDEX_DELETED;

                    if mode == STBDS_HM_STRING
                        && (*table).string.mode == STBDS_SH_STRDUP as u8
                    {
                        free(
                            *((a as *mut u8)
                                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                as *mut *mut c_char) as *mut c_void,
                        );
                    }

                    if old_index != final_index {
                        memmove(
                            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                as *mut c_void,
                            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(final_index as usize))
                                as *const c_void,
                            elemsize,
                        );

                        if mode == STBDS_HM_STRING {
                            let kp = *((a as *mut u8)
                                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                .wrapping_add(keyoffset) as *mut *mut c_char);
                            slot = stbds_hm_find_slot(
                                a,
                                elemsize,
                                kp as *mut c_void,
                                keysize,
                                keyoffset,
                                mode,
                            );
                        } else {
                            let kp = (a as *mut u8)
                                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                .wrapping_add(keyoffset);
                            slot = stbds_hm_find_slot(
                                a,
                                elemsize,
                                kp as *mut c_void,
                                keysize,
                                keyoffset,
                                mode,
                            );
                        }
                        STBDS_ASSERT(slot >= 0);
                        b = (*table)
                            .storage
                            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                        STBDS_ASSERT((*b).index[i as usize] == final_index);
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
        let p: *mut c_char;
        let len = strlen(str_).wrapping_add(1);
        if len > (*a).remaining {
            let mut blocksize: usize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = realloc(
                    ptr::null_mut(),
                    SIZEOF_STRING_BLOCK.wrapping_sub(8).wrapping_add(len),
                ) as *mut stbds_string_block;
                memmove(
                    (&raw mut (*sb).storage) as *mut c_void,
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
                return (&raw mut (*sb).storage) as *mut c_char;
            } else {
                let sb = realloc(
                    ptr::null_mut(),
                    SIZEOF_STRING_BLOCK.wrapping_sub(8).wrapping_add(blocksize),
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        STBDS_ASSERT(len <= (*a).remaining);
        p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .wrapping_add((*a).remaining)
            .wrapping_sub(len);
        (*a).remaining = (*a).remaining.wrapping_sub(len);
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x: *mut stbds_string_block;
        let mut y: *mut stbds_string_block;
        x = (*a).storage;
        while !x.is_null() {
            y = (*x).next;
            free(x as *mut c_void);
            x = y;
        }
        memset(a as *mut c_void, 0, SIZEOF_STRING_ARENA);
    }
}

// ---------------------------------------------------------------------------
// Test driver: strkey / hm_geti
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut buffer) as *mut c_char;
        // sprintf(buffer, "test_%d", n);
        let s = format!("test_{}", n);
        let bytes = s.as_bytes();
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.wrapping_add(bytes.len()) = 0;
        buf
    }
}

/// The element type of `hm_geti`'s local map: `struct { int key; int value; }`.
#[repr(C)]
#[derive(Clone, Copy)]
struct hm_geti_entry {
    key: c_int,
    value: c_int,
}

const HM_GETI_ELEMSIZE: usize = core::mem::size_of::<hm_geti_entry>();
const HM_GETI_KEYSIZE: usize = core::mem::size_of::<c_int>();
const _: () = assert!(HM_GETI_ELEMSIZE == 8);
const _: () = assert!(HM_GETI_KEYSIZE == 4);

/// `stbds_temp((t)-1)` for the intmap element type.
#[inline]
unsafe fn intmap_temp(t: *mut hm_geti_entry) -> isize {
    unsafe {
        stbds_temp_get(stbds_hash_to_arr(t as *mut c_void, HM_GETI_ELEMSIZE))
    }
}

/// `&(t)[idx]`
#[inline]
fn intmap_at(t: *mut hm_geti_entry, idx: isize) -> *mut hm_geti_entry {
    (t as *mut u8).wrapping_offset(idx.wrapping_mul(HM_GETI_ELEMSIZE as isize))
        as *mut hm_geti_entry
}

/// `hmgeti(t, k)`
#[inline]
unsafe fn intmap_hmgeti(t: &mut *mut hm_geti_entry, k: c_int) -> isize {
    unsafe {
        let mut kk: c_int = k; // (int[1]){k}
        *t = stbds_hmget_key(
            *t as *mut c_void,
            HM_GETI_ELEMSIZE,
            (&raw mut kk) as *mut c_void,
            HM_GETI_KEYSIZE,
            STBDS_HM_BINARY,
        ) as *mut hm_geti_entry;
        intmap_temp(*t)
    }
}

/// `hmgeti_ts(t, k, temp)`
#[inline]
unsafe fn intmap_hmgeti_ts(t: &mut *mut hm_geti_entry, k: c_int, temp: &mut isize) -> isize {
    unsafe {
        let mut kk: c_int = k;
        *t = stbds_hmget_key_ts(
            *t as *mut c_void,
            HM_GETI_ELEMSIZE,
            (&raw mut kk) as *mut c_void,
            HM_GETI_KEYSIZE,
            temp as *mut isize,
            STBDS_HM_BINARY,
        ) as *mut hm_geti_entry;
        *temp
    }
}

/// `hmget(t, k)`
#[inline]
unsafe fn intmap_hmget(t: &mut *mut hm_geti_entry, k: c_int) -> c_int {
    unsafe {
        let _ = intmap_hmgeti(t, k);
        (*intmap_at(*t, intmap_temp(*t))).value
    }
}

/// `hmget_ts(t, k, temp)`
#[inline]
unsafe fn intmap_hmget_ts(t: &mut *mut hm_geti_entry, k: c_int, temp: &mut isize) -> c_int {
    unsafe {
        let _ = intmap_hmgeti_ts(t, k, temp);
        (*intmap_at(*t, *temp)).value
    }
}

/// `hmput(t, k, v)`
#[inline]
unsafe fn intmap_hmput(t: &mut *mut hm_geti_entry, k: c_int, v: c_int) {
    unsafe {
        let mut kk: c_int = k;
        *t = stbds_hmput_key(
            *t as *mut c_void,
            HM_GETI_ELEMSIZE,
            (&raw mut kk) as *mut c_void,
            HM_GETI_KEYSIZE,
            0,
        ) as *mut hm_geti_entry;
        (*intmap_at(*t, intmap_temp(*t))).key = k;
        (*intmap_at(*t, intmap_temp(*t))).value = v;
    }
}

/// `hmdel(t, k)`
#[inline]
unsafe fn intmap_hmdel(t: &mut *mut hm_geti_entry, k: c_int) -> isize {
    unsafe {
        let mut kk: c_int = k;
        *t = stbds_hmdel_key(
            *t as *mut c_void,
            HM_GETI_ELEMSIZE,
            (&raw mut kk) as *mut c_void,
            HM_GETI_KEYSIZE,
            0, // STBDS_OFFSETOF((t),key)
            STBDS_HM_BINARY,
        ) as *mut hm_geti_entry;
        if !(*t).is_null() { intmap_temp(*t) } else { 0 }
    }
}

/// `hmdefault(t, v)`
#[inline]
unsafe fn intmap_hmdefault(t: &mut *mut hm_geti_entry, v: c_int) {
    unsafe {
        *t = stbds_hmput_default(*t as *mut c_void, HM_GETI_ELEMSIZE) as *mut hm_geti_entry;
        (*intmap_at(*t, -1)).value = v;
    }
}

/// `hmfree(p)`
#[inline]
unsafe fn intmap_hmfree(t: &mut *mut hm_geti_entry) {
    unsafe {
        if !(*t).is_null() {
            stbds_hmfree_func(
                stbds_hash_to_arr(*t as *mut c_void, HM_GETI_ELEMSIZE),
                HM_GETI_ELEMSIZE,
            );
        }
        *t = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    unsafe {
        let mut intmap: *mut hm_geti_entry = ptr::null_mut();
        let mut temp: isize = 0;
        let mut i: c_int;

        i = 1;
        STBDS_ASSERT(intmap_hmgeti(&mut intmap, i) == -1);
        intmap_hmdefault(&mut intmap, -2);
        STBDS_ASSERT(intmap_hmgeti(&mut intmap, i) == -1);
        STBDS_ASSERT(intmap_hmget(&mut intmap, i) == -2);

        i = 0;
        while i < num {
            intmap_hmput(&mut intmap, i, i.wrapping_mul(5));
            i = i.wrapping_add(2);
        }

        i = 0;
        while i < num {
            if i & 1 != 0 {
                STBDS_ASSERT(intmap_hmget(&mut intmap, i) == -2);
            } else {
                STBDS_ASSERT(intmap_hmget(&mut intmap, i) == i.wrapping_mul(5));
            }
            if i & 1 != 0 {
                STBDS_ASSERT(intmap_hmget_ts(&mut intmap, i, &mut temp) == -2);
            } else {
                STBDS_ASSERT(intmap_hmget_ts(&mut intmap, i, &mut temp) == i.wrapping_mul(5));
            }
            i = i.wrapping_add(1);
        }

        i = 0;
        while i < num {
            intmap_hmput(&mut intmap, i, i.wrapping_mul(3));
            i = i.wrapping_add(2);
        }

        i = 0;
        while i < num {
            if i & 1 != 0 {
                STBDS_ASSERT(intmap_hmget(&mut intmap, i) == -2);
            } else {
                STBDS_ASSERT(intmap_hmget(&mut intmap, i) == i.wrapping_mul(3));
            }
            i = i.wrapping_add(1);
        }

        i = 2;
        while i < num {
            intmap_hmdel(&mut intmap, i);
            i = i.wrapping_add(4);
        }

        i = 0;
        while i < num {
            if i & 3 != 0 {
                STBDS_ASSERT(intmap_hmget(&mut intmap, i) == -2);
            } else {
                STBDS_ASSERT(intmap_hmget(&mut intmap, i) == i.wrapping_mul(3));
            }
            i = i.wrapping_add(1);
        }

        i = 0;
        while i < num {
            intmap_hmdel(&mut intmap, i);
            i = i.wrapping_add(1);
        }

        i = 0;
        while i < num {
            STBDS_ASSERT(intmap_hmget(&mut intmap, i) == -2);
            i = i.wrapping_add(1);
        }

        intmap_hmfree(&mut intmap);

        i = 0;
        while i < num {
            intmap_hmput(&mut intmap, i, i.wrapping_mul(3));
            i = i.wrapping_add(2);
        }

        intmap_hmfree(&mut intmap);

        // silence "unused" for constants mirrored from the C source
        let _ = (STBDS_SH_NONE, STBDS_SH_ARENA, STBDS_INDEX_EMPTY);
    }
}
