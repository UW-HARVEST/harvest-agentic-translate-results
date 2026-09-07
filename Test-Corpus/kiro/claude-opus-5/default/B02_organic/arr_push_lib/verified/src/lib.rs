//! Rust translation of the C library in `c_src/` (an stb_ds.h based dynamic
//! array / hash map implementation by Sean Barrett, plus two test helpers).
//!
//! The translation is intentionally literal: every arithmetic quirk, integer
//! promotion, sign-extension and evaluation order of the original C is
//! reproduced, including behaviour that is arguably a bug in the C source.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings
//
// The C code uses `realloc`/`free` directly (STBDS_REALLOC / STBDS_FREE), and
// pointers produced by this library may be released by callers with `free`,
// so we must use the very same allocator.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

#[inline(always)]
unsafe fn stbds_realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}

#[inline(always)]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

/// `assert()` from <assert.h>. Aborts on failure like the C runtime does.
macro_rules! stbds_assert {
    ($cond:expr) => {
        if !($cond) {
            std::process::abort();
        }
    };
}

// ---------------------------------------------------------------------------
// Statics (the C file has two file-scope mutable objects)
// ---------------------------------------------------------------------------

struct SyncCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncCell<T> {}

/// `static size_t stbds_hash_seed = 0x31415926;`
static STBDS_HASH_SEED: SyncCell<usize> = SyncCell(UnsafeCell::new(0x3141_5926));

/// `static char buffer[256];`
static BUFFER: SyncCell<[u8; 256]> = SyncCell(UnsafeCell::new([0u8; 256]));

// ---------------------------------------------------------------------------
// Data structures (layouts must match the C definitions exactly)
// ---------------------------------------------------------------------------

/// `typedef struct { size_t length, capacity; void *hash_table; ptrdiff_t temp; } stbds_array_header;`
#[repr(C)]
#[derive(Clone, Copy)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>(); // 32

/// `typedef struct stbds_string_block { struct stbds_string_block *next; char storage[8]; } stbds_string_block;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

/// `struct stbds_string_arena { stbds_string_block *storage; size_t remaining; unsigned char block, mode; };`
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

/// `typedef struct { size_t hash[8]; ptrdiff_t index[8]; } stbds_hash_bucket;`
#[repr(C)]
#[derive(Clone, Copy)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

/// `typedef struct { ... } stbds_hash_index;`
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

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

// ---------------------------------------------------------------------------
// Macro helpers from the C source
// ---------------------------------------------------------------------------

/// `#define stbds_header(t)  ((stbds_array_header *) (t) - 1)`
#[inline(always)]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).offset(-1) }
}

/// `#define stbds_arrcap(a)  ((a) ? stbds_header(a)->capacity : 0)`
#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `#define stbds_arrlen(a)  ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

/// `#define stbds_temp(t)  stbds_header(t)->temp`
#[inline(always)]
unsafe fn stbds_temp_set(t: *mut c_void, v: isize) {
    unsafe { (*stbds_header(t)).temp = v }
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline(always)]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) = v }
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline(always)]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline(always)]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

/// `#define stbds_hash_table(a)  ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

#[inline(always)]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    // ((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n)))
    val.rotate_left(n)
}

#[inline(always)]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    // ((val) >> (n)) | ((val) << (STBDS_SIZE_T_BITS - (n)))
    val.rotate_right(n)
}

/// `strlen`
#[inline(always)]
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
#[inline(always)]
unsafe fn c_str_equal(a: *const c_char, b: *const c_char) -> bool {
    let mut i = 0usize;
    unsafe {
        loop {
            let x = *(a.add(i) as *const u8);
            let y = *(b.add(i) as *const u8);
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }
}

/// `0 == memcmp(a, b, n)`
#[inline(always)]
unsafe fn c_mem_equal(a: *const u8, b: *const u8, n: usize) -> bool {
    let mut i = 0usize;
    unsafe {
        while i < n {
            if *a.add(i) != *b.add(i) {
                return false;
            }
            i += 1;
        }
    }
    true
}

// ===========================================================================
// stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

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

        if min_cap < stbds_arrcap(a).wrapping_mul(2) {
            min_cap = stbds_arrcap(a).wrapping_mul(2);
        } else if min_cap < 4 {
            min_cap = 4;
        }

        let old: *mut c_void = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_header(a) as *mut c_void
        };
        // STBDS_REALLOC(c,p,s) expands to realloc(p,s) -- the context argument
        // is discarded.
        b = stbds_realloc(
            old,
            elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
        );

        let b = (b as *mut u8).add(HEADER_SIZE) as *mut c_void;
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
    unsafe {
        stbds_free(stbds_header(a) as *mut c_void);
    }
}

// ===========================================================================
// Hash seed / hash index construction
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        *STBDS_HASH_SEED.0.get() = seed;
    }
}

/// `static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)`
#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

/// `static size_t stbds_log2(size_t slot_count)`
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

/// `static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)`
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = stbds_realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
                + core::mem::size_of::<stbds_hash_index>()
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
        stbds_assert!(
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
        );

        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            // memset(&t->string, 0, sizeof(t->string));
            ptr::write_bytes(
                &mut (*t).string as *mut stbds_string_arena as *mut u8,
                0,
                core::mem::size_of::<stbds_string_arena>(),
            );
            (*t).seed = *STBDS_HASH_SEED.0.get();
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
            let seed = *STBDS_HASH_SEED.0.get();
            *STBDS_HASH_SEED.0.get() = seed.wrapping_mul(a).wrapping_add(b);
        }

        {
            let mut i = 0usize;
            while i < (slot_count >> STBDS_BUCKET_SHIFT) {
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
            while i < ((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if (*ob).index[j] >= 0 {
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

// ===========================================================================
// Hash functions
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut s = str_ as *const u8;
        while *s != 0 {
            hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);
            s = s.add(1);
        }

        hash ^= seed;
        hash = (!hash).wrapping_add(hash << 18);
        hash = hash ^ hash ^ stbds_rotate_right(hash, 31);
        hash = hash.wrapping_mul(21);
        hash = hash ^ hash ^ stbds_rotate_right(hash, 11);
        hash = hash.wrapping_add(hash << 6);
        hash ^= stbds_rotate_right(hash, 22);
        hash.wrapping_add(seed)
    }
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

macro_rules! stbds_sipround {
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

/// `static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)`
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *const u8;
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

        let mut i = 0usize;
        while i + core::mem::size_of::<usize>() <= len {
            // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
            // The RHS has type `int`; when d[3] >= 0x80 the result is negative
            // and sign-extends into the upper half of `size_t`.  Reproduced.
            let lo: i32 = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | (((*d.add(3) as u32) << 24) as i32);
            data = lo as isize as usize;

            let hi: i32 = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | (((*d.add(7) as u32) << 24) as i32);
            data |= ((hi as isize as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                stbds_sipround!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i += core::mem::size_of::<usize>();
            d = d.add(core::mem::size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        // switch (len - i) with C fall-through semantics
        if rem >= 7 {
            data |= (((*d.add(6)) as usize) << 24) << 24;
        }
        if rem >= 6 {
            data |= (((*d.add(5)) as usize) << 20) << 20;
        }
        if rem >= 5 {
            data |= (((*d.add(4)) as usize) << 16) << 16;
        }
        if rem >= 4 {
            // `data |= (d[3] << 24);` -- `int` expression, sign-extended.
            data |= ((((*d.add(3) as u32) << 24) as i32) as isize) as usize;
        }
        if rem >= 3 {
            data |= (((*d.add(2) as i32) << 16) as isize) as usize;
        }
        if rem >= 2 {
            data |= (((*d.add(1) as i32) << 8) as isize) as usize;
        }
        if rem >= 1 {
            data |= (*d.add(0) as i32) as isize as usize;
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            stbds_sipround!(v0, v1, v2, v3);
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

/// `static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode, size_t i)`
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
        let slot = (a as *mut u8)
            .offset(elemsize.wrapping_mul(i).wrapping_add(keyoffset) as isize);
        if mode >= STBDS_HM_STRING {
            let stored = *(slot as *mut *mut c_char);
            c_str_equal(key as *const c_char, stored) as c_int
        } else {
            c_mem_equal(key as *const u8, slot as *const u8, keysize) as c_int
        }
    }
}

// ===========================================================================
// Hash map implementation
// ===========================================================================

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
                    let pp = (a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_char;
                    stbds_free(*pp as *mut c_void);
                    i += 1;
                }
            }
            stbds_strreset(&mut (*stbds_hash_table(a)).string);
        }
        stbds_free((*stbds_header(a)).hash_table);
        stbds_free(stbds_header(a) as *mut c_void);
    }
}

/// `static ptrdiff_t stbds_hm_find_slot(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)`
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
            ptr::write_bytes(a as *mut u8, 0, elemsize);
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
        stbds_temp_set(hash_to_arr(p, elemsize), temp);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
            let base = if a.is_null() {
                ptr::null_mut()
            } else {
                hash_to_arr(a, elemsize)
            };
            let g = stbds_arrgrowf(base, elemsize, 0, 1);
            (*stbds_header(g)).length += 1;
            ptr::write_bytes(g as *mut u8, 0, elemsize);
            a = arr_to_hash(g, elemsize);
        }
        a
    }
}

/// `static char *stbds_strdup(char *str)`
unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = c_strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
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
            let g = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            ptr::write_bytes(g as *mut u8, 0, elemsize);
            (*stbds_header(g)).length += 1;
            a = arr_to_hash(g, elemsize);
        }

        raw_a = a;
        a = hash_to_arr(a, elemsize);

        table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

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
            let mut pos: usize;
            let mut tombstone: isize = -1;
            let mut bucket: *mut stbds_hash_bucket;

            if hash < 2 {
                hash += 2;
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
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let src = (raw_a as *mut u8).add(
                                    elemsize
                                        .wrapping_mul((*bucket).index[i] as usize)
                                        .wrapping_add(keyoffset),
                                ) as *mut *mut c_char;
                                stbds_temp_key_set(a, *src);
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
                            (*bucket).index[i] as usize,
                        ) != 0
                        {
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
                raw_a = arr_to_hash(a, elemsize);
                let _ = raw_a;

                stbds_assert!((i as usize).wrapping_add(1) <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let slot_ptr =
                    (a as *mut u8).add(elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let v = stbds_strdup(key as *mut c_char);
                        *slot_ptr = v;
                        stbds_temp_key_set(a, v);
                    }
                    STBDS_SH_ARENA => {
                        let v = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                        *slot_ptr = v;
                        stbds_temp_key_set(a, v);
                    }
                    STBDS_SH_DEFAULT => {
                        let v = key as *mut c_char;
                        *slot_ptr = v;
                        stbds_temp_key_set(a, v);
                    }
                    _ => {
                        ptr::copy_nonoverlapping(
                            key as *const u8,
                            (a as *mut u8).add(elemsize.wrapping_mul(i as usize)),
                            keysize,
                        );
                    }
                }
            }
            arr_to_hash(a, elemsize)
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

        let mut b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
        stbds_assert!(slot < (*table).slot_count as isize);
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        stbds_temp_set(raw_a, 1);
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            let pp = (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize))
                as *mut *mut c_char;
            stbds_free(*pp as *mut c_void);
        }

        if old_index != final_index {
            ptr::copy(
                (a as *const u8).add(elemsize.wrapping_mul(final_index as usize)),
                (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)),
                elemsize,
            );

            if mode == STBDS_HM_STRING {
                let kp = (a as *mut u8).add(
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                ) as *mut *mut c_char;
                slot = stbds_hm_find_slot(
                    a,
                    elemsize,
                    (*kp) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                );
            } else {
                let kp = (a as *mut u8).add(
                    elemsize
                        .wrapping_mul(old_index as usize)
                        .wrapping_add(keyoffset),
                ) as *mut c_void;
                slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
            }
            stbds_assert!(slot >= 0);
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            stbds_assert!((*b).index[i] == final_index);
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

// ===========================================================================
// String arena
// ===========================================================================

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let p: *mut c_char;
        let len = c_strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    core::mem::size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
                ptr::copy(
                    str_ as *const u8,
                    (*sb).storage.as_mut_ptr() as *mut u8,
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
                    core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        stbds_assert!(len <= (*a).remaining);
        p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
            .add((*a).remaining - len) as *mut c_char;
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
        ptr::write_bytes(
            a as *mut u8,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
    }
}

// ===========================================================================
// Test helpers exported by the library
// ===========================================================================

/// `char *strkey(int n)` -- `sprintf(buffer, "test_%d", n); return buffer;`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = BUFFER.0.get() as *mut u8;
        let s = format!("test_{}", n);
        let bytes = s.as_bytes();
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
        *buf.add(bytes.len()) = 0;
        buf as *mut c_char
    }
}

/// ```c
/// void arr_push(int num)
/// {
///   int *arr=NULL;
///   int i,j;
///   STBDS_ASSERT(arrlen(arr)==0);
///   for (i=0; i < num; i += 50) {
///     for (j=0; j < i; ++j)
///       arrpush(arr,j);
///     arrfree(arr);
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    unsafe {
        let mut arr: *mut c_int = ptr::null_mut();
        let elemsize = core::mem::size_of::<c_int>();

        stbds_assert!(stbds_arrlen(arr as *mut c_void) == 0);

        let mut i: c_int = 0;
        while i < num {
            let mut j: c_int = 0;
            while j < i {
                // stbds_arrmaybegrow(arr, 1)
                if arr.is_null()
                    || (*stbds_header(arr as *mut c_void)).length + 1
                        > (*stbds_header(arr as *mut c_void)).capacity
                {
                    arr = stbds_arrgrowf(arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
                }
                // arr[stbds_header(arr)->length++] = j;
                let h = stbds_header(arr as *mut c_void);
                let idx = (*h).length;
                (*h).length = idx + 1;
                *arr.add(idx) = j;
                j += 1;
            }
            // stbds_arrfree(arr)
            if !arr.is_null() {
                stbds_free(stbds_header(arr as *mut c_void) as *mut c_void);
            }
            arr = ptr::null_mut();
            i += 50;
        }
    }
}
