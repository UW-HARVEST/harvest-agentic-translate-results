//! Rust translation of c_src/src/lib.c (stb_ds.h implementation section plus the
//! `strkey` / `sh_puts` helpers).
//!
//! The C sources are compiled into one shared library whose public ABI is:
//!
//!   stbds_arrgrowf   stbds_arrfreef   stbds_rand_seed   stbds_hash_string
//!   stbds_hash_bytes stbds_hmget_key_ts stbds_hmget_key stbds_hmput_default
//!   stbds_shmode_func stbds_hmdel_key stbds_stralloc   stbds_hmput_key
//!   stbds_strreset   stbds_hmfree_func strkey          sh_puts
//!
//! Every one of those symbols is re-exported here with the identical signature,
//! identical memory layout for all shared structures, and identical (including
//! buggy / implementation-defined) arithmetic behaviour.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(static_mut_refs)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

// ---------------------------------------------------------------------------
// libc bindings.  The C code uses realloc()/free() for every allocation and
// printf()/sprintf() for its output, so we must use exactly the same functions
// (same heap, same stdio stream & buffering) rather than Rust equivalents.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `#define STBDS_REALLOC(c,p,s) realloc(p,s)`
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    unsafe { realloc(p, s) }
}

/// `#define STBDS_FREE(c,p) free(p)`
#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    unsafe { free(p) }
}

// `STBDS_ASSERT` -> `assert`.  Reproduced as a no-op (an NDEBUG build); every
// assertion in this library holds for all well-formed uses.
macro_rules! stbds_assert {
    ($cond:expr) => {
        let _ = &$cond;
    };
}

// ---------------------------------------------------------------------------
// Structures.  Layouts must match the C definitions byte for byte because
// callers hand these pointers back to us across the ABI.
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct {
///   size_t length; size_t capacity; void *hash_table; ptrdiff_t temp;
/// } stbds_array_header;
/// ```
#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

/// ```c
/// typedef struct stbds_string_block {
///   struct stbds_string_block *next; char storage[8];
/// } stbds_string_block;
/// ```
#[repr(C)]
pub struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

/// ```c
/// struct stbds_string_arena {
///   stbds_string_block *storage; size_t remaining;
///   unsigned char block; unsigned char mode;
/// };
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

/// ```c
/// typedef struct {
///   size_t hash[STBDS_BUCKET_LENGTH]; ptrdiff_t index[STBDS_BUCKET_LENGTH];
/// } stbds_hash_bucket;
/// ```
#[repr(C)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

/// ```c
/// typedef struct {
///   char *temp_key; size_t slot_count, used_count, used_count_threshold,
///   used_count_shrink_threshold, tombstone_count, tombstone_count_threshold,
///   seed, slot_count_log2; stbds_string_arena string; stbds_hash_bucket *storage;
/// } stbds_hash_index;
/// ```
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
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 ? 3 : 2
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

// enum { STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA };
const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

const HEADER_SIZE: usize = size_of::<stbds_array_header>(); // 32

/// `#define STBDS_INDEX_IN_USE(x) ((x) >= 0)`
#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

/// `#define stbds_header(t) ((stbds_array_header *) (t) - 1)`
///
/// Wrapping arithmetic keeps the behaviour defined even for `t == NULL`, which
/// the C code does perform (e.g. `stbds_arrfreef(NULL)`).
#[inline]
fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut u8).wrapping_sub(HEADER_SIZE) as *mut stbds_array_header
}

/// `#define stbds_arrlen(a) ((a) ? (ptrdiff_t) stbds_header(a)->length : 0)`
#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).length as isize }
    }
}

/// `#define stbds_arrcap(a) ((a) ? stbds_header(a)->capacity : 0)`
#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*stbds_header(a)).capacity }
    }
}

/// `#define stbds_temp(t) stbds_header(t)->temp`
#[inline]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    unsafe { (*stbds_header(t)).temp }
}

#[inline]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    unsafe {
        (*stbds_header(t)).temp = v;
    }
}

/// `#define stbds_temp_key(t) (*(char **) stbds_header(t)->hash_table)`
#[inline]
unsafe fn stbds_temp_key(t: *mut c_void) -> *mut c_char {
    unsafe { *((*stbds_header(t)).hash_table as *mut *mut c_char) }
}

#[inline]
unsafe fn stbds_set_temp_key(t: *mut c_void, v: *mut c_char) {
    unsafe {
        *((*stbds_header(t)).hash_table as *mut *mut c_char) = v;
    }
}

/// `#define stbds_hash_table(a) ((stbds_hash_index *) stbds_header(a)->hash_table)`
#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

/// `#define STBDS_HASH_TO_ARR(x,elemsize) ((char *) (x) - (elemsize))`
#[inline]
fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `#define STBDS_ARR_TO_HASH(x,elemsize) ((char *) (x) + (elemsize))`
#[inline]
fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).wrapping_add(elemsize) as *mut c_void
}

#[inline]
fn byte_ptr(a: *mut c_void, off: usize) -> *mut u8 {
    (a as *mut u8).wrapping_add(off)
}

// C library string/memory primitives, reimplemented so no extra dependency is
// needed.  Semantics are identical to the C originals.
#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

#[inline]
unsafe fn c_strcmp_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut i = 0usize;
    unsafe {
        loop {
            let ca = *a.add(i) as u8;
            let cb = *b.add(i) as u8;
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

#[inline]
unsafe fn c_memcmp_eq(a: *const u8, b: *const u8, n: usize) -> bool {
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

#[inline]
unsafe fn c_memmove(dst: *mut u8, src: *const u8, n: usize) {
    unsafe { ptr::copy(src, dst, n) }
}

#[inline]
unsafe fn c_memcpy(dst: *mut u8, src: *const u8, n: usize) {
    unsafe { ptr::copy_nonoverlapping(src, dst, n) }
}

#[inline]
unsafe fn c_memset0(dst: *mut u8, n: usize) {
    unsafe { ptr::write_bytes(dst, 0, n) }
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

        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_header(a) as *mut c_void
        };
        b = stbds_realloc(
            old,
            elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
        );
        let b = (b as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;
        if a.is_null() {
            (*stbds_header(b)).length = 0;
            (*stbds_header(b)).hash_table = ptr::null_mut();
            (*stbds_header(b)).temp = 0;
        } else {
            // STBDS_STATS(++stbds_array_grow) -- compiled out
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

// ---------------------------------------------------------------------------
// Hash seed / hash index construction
// ---------------------------------------------------------------------------

static mut stbds_hash_seed: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        stbds_hash_seed = seed;
    }
}

/// ```c
/// static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)
/// ```
#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

/// ```c
/// static size_t stbds_log2(size_t slot_count)
/// ```
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
            (slot_count >> STBDS_BUCKET_SHIFT)
                .wrapping_mul(size_of::<stbds_hash_bucket>())
                .wrapping_add(size_of::<stbds_hash_index>())
                .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
        ) as *mut stbds_hash_index;

        (*t).storage = stbds_align_fwd(
            (t as *mut u8).wrapping_add(size_of::<stbds_hash_index>()) as usize,
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
        stbds_assert!(
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
        );

        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            c_memset0(
                (&raw mut (*t).string) as *mut u8,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = stbds_hash_seed;
            // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
            // stbds_load_32_or_64(b,temp,  715136305,          0, 0xb504f32d);
            // -> on a 64-bit size_t these evaluate to (hi << 32) | lo.
            let a: usize = 0x27bb2ee6_87b0b0fd;
            let b: usize = 0x00000000_b504f32d;
            stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
        }

        {
            let mut i = 0usize;
            while i < slot_count >> STBDS_BUCKET_SHIFT {
                let bucket = (*t).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    (*bucket).hash[j] = STBDS_HASH_EMPTY;
                }
                for j in 0..STBDS_BUCKET_LENGTH {
                    (*bucket).index[j] = STBDS_INDEX_EMPTY;
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

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() * 8) as u32;

/// `#define STBDS_ROTATE_LEFT(val, n) (((val) << (n)) | ((val) >> (BITS - (n))))`
#[inline]
fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

/// `#define STBDS_ROTATE_RIGHT(val, n) (((val) >> (n)) | ((val) << (BITS - (n))))`
#[inline]
fn rotr(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut s = str_ as *const u8;
        while *s != 0 {
            hash = rotl(hash, 9).wrapping_add(*s as usize);
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

macro_rules! siphash_round {
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {{
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
        let mut data: usize;

        let mut v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
        let mut v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
        let mut v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
        let mut v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut i = 0usize;
        while i + size_of::<usize>() <= len {
            // The C source builds the low half with `int` arithmetic:
            //   data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
            // which becomes a *negative int* when d[3] >= 0x80 and is therefore
            // sign-extended when stored into the size_t.  Reproduced verbatim.
            let lo: i32 = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | ((*d.add(3) as i32) << 24);
            data = lo as isize as usize;
            let hi: i32 = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | ((*d.add(7) as i32) << 24);
            data |= ((hi as isize as usize) << 16) << 16; // discarded if size_t == 4

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                siphash_round!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }
        data = len << (STBDS_SIZE_T_BITS - 8);
        // switch (len - i) with fall-through from 7 down to 0.
        let rem = len - i;
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
            // `data |= (d[3] << 24);` -- int arithmetic, sign-extended.
            data |= ((*d.add(3) as i32) << 24) as isize as usize;
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
) -> c_int {
    unsafe {
        let slot = (a as *mut u8)
            .wrapping_offset((elemsize as isize).wrapping_mul(i))
            .wrapping_add(keyoffset);
        if mode >= STBDS_HM_STRING {
            (c_strcmp_eq(key as *const c_char, *(slot as *mut *mut c_char)) as c_int) as c_int
        } else {
            c_memcmp_eq(key as *const u8, slot as *const u8, keysize) as c_int
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
                    stbds_free(
                        *(byte_ptr(a, elemsize.wrapping_mul(i)) as *mut *mut c_char) as *mut c_void,
                    );
                    i += 1;
                }
            }
            stbds_strreset(&raw mut (*stbds_hash_table(a)).string as *mut stbds_string_arena);
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
                        (*bucket).index[i],
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
                        (*bucket).index[i],
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
        let keyoffset = 0usize;
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            c_memset0(a as *mut u8, elemsize);
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
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        stbds_set_temp(stbds_hash_to_arr(p, elemsize), temp);
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
            c_memset0(a as *mut u8, elemsize);
            a = stbds_arr_to_hash(a, elemsize);
        }
        a
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = c_strlen(str_) + 1;
        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
        c_memmove(p as *mut u8, str_ as *const u8, len);
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
        let mut raw_a: *mut c_void;
        let mut table: *mut stbds_hash_index;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            c_memset0(a as *mut u8, elemsize);
            (*stbds_header(a)).length += 1;
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
                stbds_free(table as *mut c_void);
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
                        ) != 0
                        {
                            stbds_set_temp(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let src = byte_ptr(
                                    raw_a,
                                    (elemsize.wrapping_mul((*bucket).index[i] as usize))
                                        .wrapping_add(keyoffset),
                                ) as *mut *mut c_char;
                                stbds_set_temp_key(a, *src);
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
                let mut found_empty = false;
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
                        ) != 0
                        {
                            stbds_set_temp(a, (*bucket).index[i]);
                            return stbds_arr_to_hash(a, elemsize);
                        }
                    } else if (*bucket).hash[i] == 0 {
                        pos = (pos & !STBDS_BUCKET_MASK) + i;
                        found_empty = true;
                        break;
                    } else if tombstone < 0 {
                        if (*bucket).index[i] == STBDS_INDEX_DELETED {
                            tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                        }
                    }
                    i += 1;
                }
                if found_empty {
                    break 'found_empty_slot;
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

                stbds_assert!((i as usize) + 1 <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_set_temp(a, i - 1);

                let dst =
                    byte_ptr(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
                match (*table).string.mode as c_int {
                    STBDS_SH_STRDUP => {
                        let v = stbds_strdup(key as *mut c_char);
                        *dst = v;
                        stbds_set_temp_key(a, v);
                    }
                    STBDS_SH_ARENA => {
                        let v = stbds_stralloc(
                            &raw mut (*table).string as *mut stbds_string_arena,
                            key as *mut c_char,
                        );
                        *dst = v;
                        stbds_set_temp_key(a, v);
                    }
                    STBDS_SH_DEFAULT => {
                        let v = key as *mut c_char;
                        *dst = v;
                        stbds_set_temp_key(a, v);
                    }
                    _ => {
                        c_memcpy(dst as *mut u8, key as *const u8, keysize);
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
        c_memset0(a as *mut u8, elemsize);
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
            ptr::null_mut()
        } else {
            let raw_a = stbds_hash_to_arr(a, elemsize);
            let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
            stbds_set_temp(raw_a, 0);
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
            let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
            stbds_assert!(slot < (*table).slot_count as isize);
            (*table).used_count -= 1;
            (*table).tombstone_count += 1;
            stbds_set_temp(raw_a, 1);
            (*b).hash[i as usize] = STBDS_HASH_DELETED;
            (*b).index[i as usize] = STBDS_INDEX_DELETED;

            if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
                stbds_free(
                    *(byte_ptr(a, elemsize.wrapping_mul(old_index as usize))
                        as *mut *mut c_char) as *mut c_void,
                );
            }

            if old_index != final_index {
                c_memmove(
                    byte_ptr(a, elemsize.wrapping_mul(old_index as usize)),
                    byte_ptr(a, elemsize.wrapping_mul(final_index as usize)) as *const u8,
                    elemsize,
                );

                if mode == STBDS_HM_STRING {
                    let kp = byte_ptr(
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
                    let kp = byte_ptr(
                        a,
                        elemsize
                            .wrapping_mul(old_index as usize)
                            .wrapping_add(keyoffset),
                    );
                    slot = stbds_hm_find_slot(a, elemsize, kp as *mut c_void, keysize, keyoffset, mode);
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
        let len = c_strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN
                .checked_shl((blocksize >> 1) as u32)
                .unwrap_or_else(|| STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32));

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = stbds_realloc(
                    ptr::null_mut(),
                    size_of::<stbds_string_block>()
                        .wrapping_sub(8)
                        .wrapping_add(len),
                ) as *mut stbds_string_block;
                c_memmove(
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
                    size_of::<stbds_string_block>()
                        .wrapping_sub(8)
                        .wrapping_add(blocksize),
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        stbds_assert!(len <= (*a).remaining);
        p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
            .wrapping_add((*a).remaining as isize as usize)
            .wrapping_sub(len);
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
        c_memset0(a as *mut u8, size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// Test / demo helpers from the bottom of lib.c
// ---------------------------------------------------------------------------

/// `static char buffer[256];`
static mut BUFFER: [u8; 256] = [0; 256];

/// ```c
/// char *strkey(int n) { sprintf(buffer, "test_%d", n); return buffer; }
/// ```
///
/// Formatted without touching the heap, so that the sequence of
/// malloc/realloc/free calls made by this library is exactly the C one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let dst = (&raw mut BUFFER) as *mut u8;
        let mut w = 0usize;
        for &c in b"test_" {
            *dst.add(w) = c;
            w += 1;
        }
        // "%d" of an int
        let mut mag: u64 = (n as i64).unsigned_abs();
        if n < 0 {
            *dst.add(w) = b'-';
            w += 1;
        }
        let mut digits = [0u8; 20];
        let mut nd = 0usize;
        loop {
            digits[nd] = b'0' + (mag % 10) as u8;
            nd += 1;
            mag /= 10;
            if mag == 0 {
                break;
            }
        }
        while nd > 0 {
            nd -= 1;
            *dst.add(w) = digits[nd];
            w += 1;
        }
        *dst.add(w) = 0;
        dst as *mut c_char
    }
}

/// The string-hash-map entry type declared locally inside `sh_puts`:
/// `struct { char *key; int value; }`
#[repr(C)]
#[derive(Clone, Copy)]
struct sh_puts_entry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    unsafe {
        let elemsize = size_of::<sh_puts_entry>(); // sizeof *strmap
        let mut strmap: *mut sh_puts_entry = ptr::null_mut();
        let mut s = sh_puts_entry {
            key: ptr::null_mut(),
            value: 0,
        };
        let mut sa = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };

        // for (i=0; i < num; ++i) stralloc(&sa, strkey(i));
        let mut i: c_int = 0;
        while i < num {
            stbds_stralloc(&mut sa, strkey(i));
            i += 1;
        }
        // strreset(&sa);
        stbds_strreset(&mut sa);

        {
            // s.key = "a", s.value = num;
            s.key = c"a".as_ptr() as *mut c_char;
            s.value = num;

            // sh_new_arena(strmap);
            strmap = stbds_shmode_func(elemsize, STBDS_SH_ARENA) as *mut sh_puts_entry;

            // shputs(strmap, s);
            //   (t) = stbds_hmput_key((t), sizeof *(t), (void*)(s).key, sizeof (s).key, STBDS_HM_STRING),
            //   (t)[stbds_temp((t)-1)] = (s),
            //   (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1)
            strmap = stbds_hmput_key(
                strmap as *mut c_void,
                elemsize,
                s.key as *mut c_void,
                size_of::<*mut c_char>(), // sizeof (s).key
                STBDS_HM_STRING,
            ) as *mut sh_puts_entry;
            let raw = strmap.wrapping_sub(1) as *mut c_void;
            let t1 = stbds_temp(raw);
            *strmap.offset(t1) = s;
            let t2 = stbds_temp(raw);
            (*strmap.offset(t2)).key = stbds_temp_key(raw);

            stbds_assert!(*(*strmap.offset(0)).key == b'a' as c_char);
            stbds_assert!((*strmap.offset(0)).key != s.key);
            stbds_assert!((*strmap.offset(0)).value == s.value);

            // for (int z=0; z < shlen(strmap); ++z)
            //     printf("%s %d\n", strmap[z], strmap[z].value);
            //
            // `strmap[z]` is a 16-byte struct passed to a variadic function: in
            // the SysV AMD64 ABI it occupies the next two INTEGER argument
            // registers, so %s consumes its `key` member and %d consumes the
            // low 32 bits of its second eightbyte, i.e. `value`.  The explicit
            // third argument is never consumed by the format string.
            let mut z: c_int = 0;
            while (z as isize) < stbds_hmlen(strmap as *mut c_void, elemsize) {
                let e = *strmap.offset(z as isize);
                printf(c"%s %d\n".as_ptr(), e.key, e.value);
                z += 1;
            }

            // shfree(strmap);
            if !strmap.is_null() {
                stbds_hmfree_func(strmap.wrapping_sub(1) as *mut c_void, elemsize);
            }
            #[allow(unused_assignments)]
            {
                strmap = ptr::null_mut();
            }
        }
    }
}

/// `#define stbds_hmlen(t) ((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)`
#[inline]
unsafe fn stbds_hmlen(t: *mut c_void, elemsize: usize) -> isize {
    unsafe {
        if t.is_null() {
            0
        } else {
            (*stbds_header(stbds_hash_to_arr(t, elemsize))).length as isize - 1
        }
    }
}

// Documented for completeness; the C header defines the whole macro set even
// where this translation unit never needs a given value.
#[allow(dead_code)]
const _UNUSED_CONSTS: (c_int, c_int) = (STBDS_HM_BINARY, STBDS_SH_NONE);
