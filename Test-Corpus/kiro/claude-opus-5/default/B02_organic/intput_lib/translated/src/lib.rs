//! Rust translation of `c_src/src/lib.c` (an inlined copy of the `stb_ds.h`
//! implementation plus the small `strkey` / `intput` helpers).
//!
//! The translation is deliberately literal: memory layouts, allocation
//! patterns, integer wrap-around, sign-extension quirks and the exact order of
//! checks are all preserved so that the shared object behaves identically to
//! the C build (including its bugs).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// libc bindings (the C code uses realloc/free directly, so we must too:
// STBDS_REALLOC(c,p,s) -> realloc(p,s), STBDS_FREE(c,p) -> free(p))
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// assert() emulation: glibc prints
//   "<progname>: <file>:<line>: <func>: Assertion `<expr>' failed.\n"
// on stderr and then raises SIGABRT.
// ---------------------------------------------------------------------------

fn progname() -> String {
    match std::fs::read("/proc/self/cmdline") {
        Ok(bytes) => {
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let argv0 = String::from_utf8_lossy(&bytes[..end]).into_owned();
            match argv0.rfind('/') {
                Some(i) => argv0[i + 1..].to_string(),
                None => argv0,
            }
        }
        Err(_) => String::new(),
    }
}

#[cold]
fn assert_fail(expr: &str, file: &str, line: u32, func: &str) -> ! {
    use std::io::Write;
    let prog = progname();
    let msg = format!(
        "{}{}{}:{}: {}{}Assertion `{}' failed.\n",
        prog,
        if prog.is_empty() { "" } else { ": " },
        file,
        line,
        func,
        if func.is_empty() { "" } else { ": " },
        expr
    );
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(msg.as_bytes());
    let _ = lock.flush();
    std::process::abort();
}

macro_rules! stbds_assert {
    ($cond:expr, $expr:expr, $line:expr, $func:expr) => {
        if !($cond) {
            assert_fail($expr, "src/lib.c", $line, $func);
        }
    };
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

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

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// ---------------------------------------------------------------------------
// Data structures (layout-compatible with the C originals)
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

const HEADER_SIZE: usize = core::mem::size_of::<stbds_array_header>();

// ---------------------------------------------------------------------------
// Small helpers mirroring the stb_ds macros
// ---------------------------------------------------------------------------

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
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

#[inline]
unsafe fn stbds_temp_set(a: *mut c_void, v: isize) {
    unsafe { (*stbds_header(a)).temp = v }
}

#[inline]
unsafe fn stbds_temp_get(a: *mut c_void) -> isize {
    unsafe { (*stbds_header(a)).temp }
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`
#[inline]
unsafe fn stbds_temp_key_set(a: *mut c_void, v: *mut c_char) {
    unsafe { *((*stbds_header(a)).hash_table as *mut *mut c_char) = v }
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*stbds_header(a)).hash_table as *mut stbds_hash_index }
}

#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).offset(-(elemsize as isize)) as *mut c_void }
}

#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a - 1)) & !(a - 1)
}

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

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

/// `0 == strcmp(a, b)`
#[inline]
unsafe fn c_streq(a: *const c_char, b: *const c_char) -> bool {
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

/// `0 == memcmp(a, b, n)`
#[inline]
unsafe fn c_memeq(a: *const u8, b: *const u8, n: usize) -> bool {
    unsafe {
        for i in 0..n {
            if *a.add(i) != *b.add(i) {
                return false;
            }
        }
    }
    true
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
        let raw = realloc(old, elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE));
        b = (raw as *mut u8).add(HEADER_SIZE) as *mut c_void;
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
        free(stbds_header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// Hash index
// ---------------------------------------------------------------------------

static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x31415926);

#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED.store(seed, Ordering::Relaxed);
}

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

/// Reproduces the `stbds_load_32_or_64` macro on a 64-bit target.
///
/// `temp = v64_lo ^ v32` is evaluated in 32-bit `unsigned int` arithmetic in C,
/// then widened, shifted left 32 and back right 32.
fn stbds_load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ (v32 as usize);
    var
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let t = realloc(
            ptr::null_mut(),
            (slot_count >> STBDS_BUCKET_SHIFT)
                .wrapping_mul(core::mem::size_of::<stbds_hash_bucket>())
                .wrapping_add(core::mem::size_of::<stbds_hash_index>())
                .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
        ) as *mut stbds_hash_index;

        (*t).storage =
            align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
            (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
            "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count",
            401,
            "stbds_make_hash_index"
        );
        if !ot.is_null() {
            (*t).string = (*ot).string;
            (*t).seed = (*ot).seed;
        } else {
            ptr::write_bytes(
                &mut (*t).string as *mut stbds_string_arena as *mut u8,
                0,
                core::mem::size_of::<stbds_string_arena>(),
            );
            (*t).seed = STBDS_HASH_SEED.load(Ordering::Relaxed);
            let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
            let seed = STBDS_HASH_SEED.load(Ordering::Relaxed);
            STBDS_HASH_SEED.store(seed.wrapping_mul(a).wrapping_add(b), Ordering::Relaxed);
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
// Hashing
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash = seed;
        let mut p = str_ as *const u8;
        while *p != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*p as usize);
            p = p.add(1);
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

macro_rules! sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotate_left($v1, 13);
        $v1 ^= $v0;
        $v0 = rotate_left($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotate_left($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotate_left($v1, 17);
        $v1 ^= $v2;
        $v2 = rotate_left($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotate_left($v3, 21);
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
        let mut data: usize;

        v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
        v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
        v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
        v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        let mut i = 0usize;
        while i + core::mem::size_of::<usize>() <= len {
            // The C code builds the low half with `int` arithmetic; `d[3] << 24`
            // can set the sign bit, and the resulting negative `int` is then
            // sign-extended when assigned to `size_t`.  Reproduced here.
            let lo: i32 = (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | ((*d.add(3) as i32) << 24);
            data = lo as isize as usize;
            let hi: i32 = (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | ((*d.add(7) as i32) << 24);
            data |= ((hi as isize as usize) << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                sipround!(v0, v1, v2, v3);
            }
            v0 ^= data;

            i += core::mem::size_of::<usize>();
            d = d.add(core::mem::size_of::<usize>());
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        if rem <= 7 {
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
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            sipround!(v0, v1, v2, v3);
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
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
    unsafe {
        let slot = (a as *mut u8).add(elemsize.wrapping_mul(i).wrapping_add(keyoffset));
        if mode >= STBDS_HM_STRING {
            c_streq(key as *const c_char, *(slot as *mut *mut c_char))
        } else {
            c_memeq(key as *const u8, slot, keysize)
        }
    }
}

// ---------------------------------------------------------------------------
// Hash map internals
// ---------------------------------------------------------------------------

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
                    free(
                        *((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_char)
                            as *mut c_void,
                    );
                    i += 1;
                }
            }
            stbds_strreset(&mut (*stbds_hash_table(a)).string);
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
                        (*bucket).index[i] as usize,
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
            let base = if !a.is_null() {
                hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            };
            a = stbds_arrgrowf(base, elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            ptr::write_bytes(a as *mut u8, 0, elemsize);
            a = arr_to_hash(a, elemsize);
        }
        a
    }
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
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
            ptr::write_bytes(a as *mut u8, 0, elemsize);
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
            let mut tombstone: isize = -1;
            let mut bucket: *mut stbds_hash_bucket;

            if hash < 2 {
                hash += 2;
            }

            let mut pos =
                stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

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
                        ) {
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let kp = *((raw_a as *mut u8).add(
                                    elemsize
                                        .wrapping_mul((*bucket).index[i] as usize)
                                        .wrapping_add(keyoffset),
                                ) as *mut *mut c_char);
                                stbds_temp_key_set(a, kp);
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
                let mut found = false;
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
                            stbds_temp_set(a, (*bucket).index[i]);
                            return arr_to_hash(a, elemsize);
                        }
                    } else if (*bucket).hash[i] == 0 {
                        pos = (pos & !STBDS_BUCKET_MASK) + i;
                        found = true;
                        break;
                    } else if tombstone < 0 {
                        if (*bucket).index[i] == STBDS_INDEX_DELETED {
                            tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                        }
                    }
                    i += 1;
                }
                if found {
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
                    a = stbds_arrgrowf(a, elemsize, 1, 0);
                }
                raw_a = arr_to_hash(a, elemsize);

                stbds_assert!(
                    (i as usize).wrapping_add(1) <= stbds_arrcap(a),
                    "(size_t) i+1 <= stbds_arrcap(a)",
                    778,
                    "stbds_hmput_key"
                );
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let elem = (a as *mut u8).add(elemsize.wrapping_mul(i as usize));
                match (*table).string.mode {
                    STBDS_SH_STRDUP => {
                        let s = stbds_strdup(key as *mut c_char);
                        *(elem as *mut *mut c_char) = s;
                        stbds_temp_key_set(a, s);
                    }
                    STBDS_SH_ARENA => {
                        let s = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                        *(elem as *mut *mut c_char) = s;
                        stbds_temp_key_set(a, s);
                    }
                    STBDS_SH_DEFAULT => {
                        let s = key as *mut c_char;
                        *(elem as *mut *mut c_char) = s;
                        stbds_temp_key_set(a, s);
                    }
                    _ => {
                        ptr::copy_nonoverlapping(key as *const u8, elem, keysize);
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
        let final_index = stbds_arrlen(raw_a) - 1 - 1;
        stbds_assert!(
            slot < (*table).slot_count as isize,
            "slot < (ptrdiff_t) table->slot_count",
            828,
            "stbds_hmdel_key"
        );
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        stbds_temp_set(raw_a, 1);
        stbds_assert!(
            true, // table->used_count >= 0 is vacuously true for size_t
            "table->used_count >= 0",
            832,
            "stbds_hmdel_key"
        );
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            free(
                *((a as *mut u8).add(elemsize.wrapping_mul(old_index as usize))
                    as *mut *mut c_char) as *mut c_void,
            );
        }

        if old_index != final_index {
            ptr::copy(
                (a as *const u8).add(elemsize.wrapping_mul(final_index as usize)),
                (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)),
                elemsize,
            );

            let slot_key = (a as *mut u8)
                .add(elemsize.wrapping_mul(old_index as usize).wrapping_add(keyoffset));
            if mode == STBDS_HM_STRING {
                slot = stbds_hm_find_slot(
                    a,
                    elemsize,
                    *(slot_key as *mut *mut c_char) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                );
            } else {
                slot = stbds_hm_find_slot(a, elemsize, slot_key as *mut c_void, keysize, keyoffset, mode);
            }
            stbds_assert!(slot >= 0, "slot >= 0", 846, "stbds_hmdel_key");
            b = (*table).storage.add((slot >> STBDS_BUCKET_SHIFT) as usize);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            stbds_assert!(
                (*b).index[i] == final_index,
                "b->index[i] == final_index",
                849,
                "stbds_hmdel_key"
            );
            (*b).index[i] = old_index;
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

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = c_strlen(str_) + 1;
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        ptr::copy(str_ as *const u8, p as *mut u8, len);
        p
    }
}

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

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = realloc(
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
                let sb = realloc(
                    ptr::null_mut(),
                    core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        stbds_assert!(
            len <= (*a).remaining,
            "len <= a->remaining",
            913,
            "stbds_stralloc"
        );
        p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
            .add((*a).remaining.wrapping_sub(len)) as *mut c_char;
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
            free(x as *mut c_void);
            x = y;
        }
        ptr::write_bytes(
            a as *mut u8,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
    }
}

// ---------------------------------------------------------------------------
// Library entry points
// ---------------------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = (&raw mut BUFFER) as *mut c_char;
        let s = format!("test_{}", n);
        let bytes = s.as_bytes();
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
        buf
    }
}

#[repr(C)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

const INTMAP_ELEMSIZE: usize = core::mem::size_of::<IntMapEntry>();
const INTMAP_KEYSIZE: usize = core::mem::size_of::<c_int>();

/// `stbds_temp((t)-1)` for the int map: the `temp` field of the header of the
/// raw (default-element-prefixed) array.
#[inline]
unsafe fn intmap_temp(t: *mut IntMapEntry) -> isize {
    unsafe { stbds_temp_get(hash_to_arr(t as *mut c_void, INTMAP_ELEMSIZE)) }
}

/// `hmput(intmap, k, v)`
#[inline]
unsafe fn intmap_put(t: *mut IntMapEntry, k: c_int, v: c_int) -> *mut IntMapEntry {
    unsafe {
        // STBDS_ADDRESSOF((t)->key, (k)) == (int[1]){k}
        let mut key: [c_int; 1] = [k];
        let t = stbds_hmput_key(
            t as *mut c_void,
            INTMAP_ELEMSIZE,
            key.as_mut_ptr() as *mut c_void,
            INTMAP_KEYSIZE,
            0,
        ) as *mut IntMapEntry;
        (*t.offset(intmap_temp(t))).key = k;
        (*t.offset(intmap_temp(t))).value = v;
        t
    }
}

/// `hmget(intmap, k)`, which also rewrites `t`
#[inline]
unsafe fn intmap_get(t: *mut IntMapEntry, k: c_int) -> (*mut IntMapEntry, c_int) {
    unsafe {
        let mut key: [c_int; 1] = [k];
        let t = stbds_hmget_key(
            t as *mut c_void,
            INTMAP_ELEMSIZE,
            key.as_mut_ptr() as *mut c_void,
            INTMAP_KEYSIZE,
            STBDS_HM_BINARY,
        ) as *mut IntMapEntry;
        let v = (*t.offset(intmap_temp(t))).value;
        (t, v)
    }
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn intput(num: c_int) {
    unsafe {
        let mut intmap: *mut IntMapEntry = ptr::null_mut();

        intmap = ptr::null_mut();
        intmap = intmap_put(intmap, num, 7);
        intmap = intmap_put(intmap, 11, 3);
        intmap = intmap_put(intmap, 9, num);

        let (t, v) = intmap_get(intmap, 9);
        intmap = t;
        stbds_assert!(v == num, "hmget(intmap, 9) == num", 953, "intput");

        let (t, v) = intmap_get(intmap, 11);
        intmap = t;
        stbds_assert!(v == 3, "hmget(intmap, 11) == 3", 954, "intput");

        let (t, v) = intmap_get(intmap, num);
        intmap = t;
        stbds_assert!(v == 7, "hmget(intmap, num) == 7", 955, "intput");

        // The C code never frees `intmap`; neither do we.
        let _ = intmap;
    }
}
