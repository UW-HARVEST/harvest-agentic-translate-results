//! Rust translation of c_src/src/lib.c.
//!
//! The C source contains a copy of stb_ds (an open-addressed hash table /
//! growable-array library) and a public entry point `sh_geti(int num)`.
//! This module re-exports every C-visible symbol with identical semantics.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memset(p: *mut c_void, v: c_int, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Layout-compatible structs.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    // C struct uses default alignment (8 bytes on x86_64): 16 + 8 + 1 + 1 + 6 padding = 24.
    _pad: [u8; 6],
}

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

// ---------------------------------------------------------------------------
// Constants.
// ---------------------------------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // log2(8)
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

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

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

#[inline(always)]
unsafe fn header(a: *mut c_void) -> *mut stbds_array_header {
    unsafe { (a as *mut stbds_array_header).sub(1) }
}

#[inline(always)]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length as isize }
    }
}

#[inline(always)]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).capacity }
    }
}

#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

// ---------------------------------------------------------------------------
// Global hash seed.
// ---------------------------------------------------------------------------

// The C code stores a single mutable global. To preserve byte-identical
// behavior we use a mutable static and access it without synchronization,
// matching the C semantics (single-threaded use).
static mut STBDS_HASH_SEED: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

// ---------------------------------------------------------------------------
// Array functions.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    unsafe {
        let cur_len = arrlen(a) as usize;
        let cur_cap = arrcap(a);
        let min_len = cur_len + addlen;

        if min_len > min_cap {
            min_cap = min_len;
        }
        if min_cap <= cur_cap {
            return a;
        }
        if min_cap < 2 * cur_cap {
            min_cap = 2 * cur_cap;
        } else if min_cap < 4 {
            min_cap = 4;
        }

        let prev = if a.is_null() {
            ptr::null_mut()
        } else {
            header(a) as *mut c_void
        };
        let total = elemsize * min_cap + core::mem::size_of::<stbds_array_header>();
        let mut b = realloc(prev, total) as *mut u8;
        b = b.add(core::mem::size_of::<stbds_array_header>());
        let bv = b as *mut c_void;
        if a.is_null() {
            (*header(bv)).length = 0;
            (*header(bv)).hash_table = ptr::null_mut();
            (*header(bv)).temp = 0;
        }
        (*header(bv)).capacity = min_cap;
        bv
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        free(header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// Hash functions.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut s = str_ as *mut u8;
        let mut hash: usize = seed;
        while *s != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*s as usize);
            s = s.add(1);
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

#[inline(always)]
unsafe fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *mut u8;
        let mut v0: usize = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
        let mut v1: usize = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
        let mut v2: usize = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        let mut v3: usize = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        macro_rules! sipround {
            () => {
                v0 = v0.wrapping_add(v1);
                v1 = rotate_left(v1, 13);
                v1 ^= v0;
                v0 = rotate_left(v0, STBDS_SIZE_T_BITS / 2);
                v2 = v2.wrapping_add(v3);
                v3 = rotate_left(v3, 16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = rotate_left(v1, 17);
                v1 ^= v2;
                v2 = rotate_left(v2, STBDS_SIZE_T_BITS / 2);
                v0 = v0.wrapping_add(v3);
                v3 = rotate_left(v3, 21);
                v3 ^= v0;
            };
        }

        let stride = core::mem::size_of::<usize>();
        let mut i: usize = 0;
        let mut data: usize;
        while i + stride <= len {
            // C: data = d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24);
            //    data |= (size_t)(d[4] | (d[5]<<8) | (d[6]<<16) | (d[7]<<24)) << 16 << 16;
            // The shifts on `int` (d[k] << 24) are int-type; replicate as such.
            // In Rust we use wrapping arithmetic on usize which is fine because
            // the bit pattern is what matters.
            let lo32 = (*d.add(0) as u32)
                | ((*d.add(1) as u32) << 8)
                | ((*d.add(2) as u32) << 16)
                | ((*d.add(3) as u32) << 24);
            let hi32 = (*d.add(4) as u32)
                | ((*d.add(5) as u32) << 8)
                | ((*d.add(6) as u32) << 16)
                | ((*d.add(7) as u32) << 24);
            // C: int implicit conversion to size_t. d[k] is unsigned char so
            // promoted to int; (d[k]<<24) for d[3] is a 32-bit int; extension
            // to size_t sign-extends if the resulting int is negative. Match
            // by using i32 then sign-extending to isize as size_t.
            let mut combined: usize = lo32 as i32 as isize as usize;
            combined |= ((hi32 as usize) << 16) << 16;
            data = combined;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                sipround!();
            }
            v0 ^= data;
            d = d.add(stride);
            i += stride;
        }

        data = len << (STBDS_SIZE_T_BITS - 8);
        // Switch with fall-through. d here points past the loop's last full
        // word read; remaining bytes are at d[0..len-i].
        let rem = len - i;
        // Using fallthrough switch from C:
        if rem >= 7 {
            data |= (*d.add(6) as usize) << 24 << 24;
        }
        if rem >= 6 {
            data |= (*d.add(5) as usize) << 20 << 20;
        }
        if rem >= 5 {
            data |= (*d.add(4) as usize) << 16 << 16;
        }
        if rem >= 4 {
            // C: (d[3] << 24) — int-typed; bit 31 may be set, sign-extends to
            // size_t. Reproduce that exactly.
            let v = ((*d.add(3) as u32) << 24) as i32 as isize as usize;
            data |= v;
        }
        if rem >= 3 {
            data |= (*d.add(2) as usize) << 16;
        }
        if rem >= 2 {
            data |= (*d.add(1) as usize) << 8;
        }
        if rem >= 1 {
            data |= *d.add(0) as usize;
        }
        // case 0: break — nothing.

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            sipround!();
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { siphash_bytes(p, len, seed) }
}

// ---------------------------------------------------------------------------
// Probe and helpers.
// ---------------------------------------------------------------------------

#[inline(always)]
fn probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn log2(mut slot_count: usize) -> usize {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    unsafe {
        if mode >= STBDS_HM_STRING {
            let kp = (a as *mut u8).add(elemsize * i as usize + keyoffset) as *mut *mut c_char;
            strcmp(key as *mut c_char, *kp) == 0
        } else {
            let kp = (a as *mut u8).add(elemsize * i as usize + keyoffset);
            memcmp(key, kp as *const c_void, keysize) == 0
        }
    }
}

#[inline(always)]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline(always)]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}

#[inline(always)]
unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*header(a)).hash_table as *mut stbds_hash_index }
}

// ---------------------------------------------------------------------------
// Make hash index.
// ---------------------------------------------------------------------------

unsafe fn make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let bytes = (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
            + core::mem::size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1;
        let t = realloc(ptr::null_mut(), bytes) as *mut stbds_hash_index;
        let after_t = (t as usize) + core::mem::size_of::<stbds_hash_index>();
        (*t).storage = align_fwd(after_t, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
        (*t).slot_count = slot_count;
        (*t).slot_count_log2 = log2(slot_count);
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
            // memset(&t->string, 0, sizeof(t->string));
            memset(
                &mut (*t).string as *mut stbds_string_arena as *mut c_void,
                0,
                core::mem::size_of::<stbds_string_arena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            // stbds_load_32_or_64: on 64-bit, var = v64_hi << 32 ^ temp ^ v32
            //   temp = v64_lo ^ v32; temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            //   var = v64_hi; var <<= 16; var <<= 16;
            //   var ^= temp ^ v32
            // So var = (v64_hi << 32) ^ ((v64_lo ^ v32) low-32-bits) ^ v32
            //       = (v64_hi << 32) | v64_lo (when masked to low 32)
            // Specifically: temp ends up as (v64_lo ^ v32) & 0xFFFFFFFF.
            // Then var = (v64_hi << 32) ^ ((v64_lo ^ v32) & 0xffffffff) ^ v32
            //         = (v64_hi << 32) ^ (((v64_lo ^ v32) & 0xffffffff) ^ v32)
            //         = (v64_hi << 32) ^ (v64_lo & 0xffffffff)  (since v32 is low 32-bit
            //                                                    so XORs cancel low 32 bits)
            // Actually let's just literally implement the C code:
            let v32_a: usize = 2147001325;
            let v64_hi_a: usize = 0x27bb2ee6;
            let v64_lo_a: usize = 0x87b0b0fd;
            let mut temp: usize;
            let a: usize;
            temp = v64_lo_a ^ v32_a;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut va = v64_hi_a;
            va <<= 16;
            va <<= 16;
            va ^= temp ^ v32_a;
            a = va;

            let v32_b: usize = 715136305;
            let v64_hi_b: usize = 0;
            let v64_lo_b: usize = 0xb504f32d;
            let b: usize;
            temp = v64_lo_b ^ v32_b;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut vb = v64_hi_b;
            vb <<= 16;
            vb <<= 16;
            vb ^= temp ^ v32_b;
            b = vb;

            STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
        }

        // Initialize buckets.
        let nb = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..nb {
            let bucket = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bucket).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bucket).index[j] = STBDS_INDEX_EMPTY;
            }
        }

        // Re-insert from old table.
        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            let onb = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
            for i in 0..onb {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    let idx = (*ob).index[j];
                    if idx >= 0 {
                        let hash = (*ob).hash[j];
                        let mut pos = probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                        let mut step = STBDS_BUCKET_LENGTH;
                        'outer: loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                            // first half: from pos & MASK to BUCKET_LENGTH
                            let mut placed = false;
                            let start = pos & STBDS_BUCKET_MASK;
                            for z in start..STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = idx;
                                    placed = true;
                                    break;
                                }
                            }
                            if placed {
                                break 'outer;
                            }
                            let limit = pos & STBDS_BUCKET_MASK;
                            for z in 0..limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = idx;
                                    placed = true;
                                    break;
                                }
                            }
                            if placed {
                                break 'outer;
                            }
                            pos += step;
                            step += STBDS_BUCKET_LENGTH;
                            pos &= (*t).slot_count - 1;
                        }
                    }
                }
            }
        }

        t
    }
}

// ---------------------------------------------------------------------------
// hmfree_func.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        let table = hash_table(a);
        if !table.is_null() {
            if (*table).string.mode == STBDS_SH_STRDUP as u8 {
                let len = (*header(a)).length;
                for i in 1..len {
                    let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                    free(*p as *mut c_void);
                }
            }
            stbds_strreset(&mut (*table).string);
        }
        free((*header(a)).hash_table);
        free(header(a) as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// hm_find_slot.
// ---------------------------------------------------------------------------

unsafe fn hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
        let raw_a = hash_to_arr(a, elemsize);
        let table = hash_table(raw_a);
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        if hash < 2 {
            hash += 2;
        }
        let mut pos = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }
            pos += step;
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }
    }
}

// ---------------------------------------------------------------------------
// hmget_key_ts / hmget_key.
// ---------------------------------------------------------------------------

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
        let keyoffset = 0;
        if a.is_null() {
            let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*header(a)).length += 1;
            memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            arr_to_hash(a, elemsize)
        } else {
            let raw_a = hash_to_arr(a, elemsize);
            let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
            if table.is_null() {
                *temp = -1;
            } else {
                let slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    *temp = STBDS_INDEX_EMPTY;
                } else {
                    let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[slot as usize & STBDS_BUCKET_MASK];
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
        let raw = hash_to_arr(p, elemsize);
        (*header(raw)).temp = temp;
        p
    }
}

// ---------------------------------------------------------------------------
// hmput_default.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
            let prev = if a.is_null() {
                ptr::null_mut()
            } else {
                hash_to_arr(a, elemsize)
            };
            let new_a = stbds_arrgrowf(prev, elemsize, 0, 1);
            (*header(new_a)).length += 1;
            memset(new_a, 0, elemsize);
            arr_to_hash(new_a, elemsize)
        } else {
            a
        }
    }
}

// ---------------------------------------------------------------------------
// strdup (internal).
// ---------------------------------------------------------------------------

unsafe fn stbds_strdup_internal(s: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(s) + 1;
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, s as *const c_void, len);
        p
    }
}

// ---------------------------------------------------------------------------
// hmput_key.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset = 0;
        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }

        let mut raw_a = a;
        a = hash_to_arr(a, elemsize);

        let mut table = (*header(a)).hash_table as *mut stbds_hash_index;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count * 2
            };
            let nt = make_hash_index(slot_count, table);
            if !table.is_null() {
                free(table as *mut c_void);
            } else {
                (*nt).string.mode = if mode >= STBDS_HM_STRING {
                    STBDS_SH_DEFAULT as u8
                } else {
                    0
                };
            }
            (*header(a)).hash_table = nt as *mut c_void;
            table = nt;
        }

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
        let mut pos = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        let pos_found: usize;
        'outer: loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
                        (*header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            // stbds_temp_key(a) = ...
                            let kp = (raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char;
                            // stbds_temp_key(a) = *(char **) hash_table[0] cast: macro accesses
                            //   *(char **) stbds_header(t)->hash_table.
                            // hash_table points at stbds_hash_index whose first member is
                            // temp_key (char *). So writing *(char**)hash_table writes
                            // table->temp_key.
                            (*table).temp_key = *kp;
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos_found = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i])
                    {
                        (*header(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos_found = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }
            pos += step;
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        let mut pos = pos_found;
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        let i = arrlen(a);
        if (i + 1) as usize > arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = arr_to_hash(a, elemsize);

        (*header(a)).length = (i + 1) as usize;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*header(a)).temp = i - 1;

        match (*table).string.mode as c_int {
            v if v == STBDS_SH_STRDUP => {
                let new_p = stbds_strdup_internal(key as *mut c_char);
                let dst = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dst = new_p;
                (*table).temp_key = new_p;
            }
            v if v == STBDS_SH_ARENA => {
                let new_p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                let dst = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dst = new_p;
                (*table).temp_key = new_p;
            }
            v if v == STBDS_SH_DEFAULT => {
                let dst = (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dst = key as *mut c_char;
                (*table).temp_key = key as *mut c_char;
            }
            _ => {
                memcpy(
                    (a as *mut u8).add(elemsize * i as usize) as *mut c_void,
                    key as *const c_void,
                    keysize,
                );
            }
        }

        let _ = raw_a;
        arr_to_hash(a, elemsize)
    }
}

// ---------------------------------------------------------------------------
// shmode_func.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length = 1;
        let h = make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        arr_to_hash(a, elemsize)
    }
}

// ---------------------------------------------------------------------------
// hmdel_key.
// ---------------------------------------------------------------------------

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
        let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
        (*header(raw_a)).temp = 0;
        if table.is_null() {
            return a;
        }
        let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }
        let mut b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = arrlen(raw_a) - 1 - 1;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_a)).temp = 1;
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
            let p = (a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char;
            free(*p as *mut c_void);
        }

        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
                (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
                elemsize,
            );

            slot = if mode == STBDS_HM_STRING {
                let kp = (a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                    as *mut *mut c_char;
                hm_find_slot(a, elemsize, *kp as *mut c_void, keysize, keyoffset, mode)
            } else {
                let kp = (a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                    as *mut c_void;
                hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode)
            };
            b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            (*b).index[i] = old_index;
        }
        (*header(raw_a)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > STBDS_BUCKET_LENGTH
        {
            (*header(raw_a)).hash_table =
                make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
            free(table as *mut c_void);
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            (*header(raw_a)).hash_table =
                make_hash_index((*table).slot_count, table) as *mut c_void;
            free(table as *mut c_void);
        }

        a
    }
}

// ---------------------------------------------------------------------------
// String arena.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    s: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = strlen(s) + 1;
        if len > (*a).remaining {
            let bs0 = (*a).block as usize;
            let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (bs0 >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                // sizeof(stbds_string_block) = 16; sizeof(*sb) - 8 = 8 (next pointer
                // size).
                let total = core::mem::size_of::<stbds_string_block>() - 8 + len;
                let sb = realloc(ptr::null_mut(), total) as *mut stbds_string_block;
                memmove(
                    (*sb).storage.as_mut_ptr() as *mut c_void,
                    s as *const c_void,
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
                let total = core::mem::size_of::<stbds_string_block>() - 8 + blocksize;
                let sb = realloc(ptr::null_mut(), total) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
        (*a).remaining -= len;
        memmove(p as *mut c_void, s as *const c_void, len);
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
        memset(a as *mut c_void, 0, core::mem::size_of::<stbds_string_arena>());
    }
}

// ---------------------------------------------------------------------------
// strkey + sh_geti.
// ---------------------------------------------------------------------------

// 256-byte mutable buffer matching `static char buffer[256];` in C.
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let fmt = b"test_%d\0";
        let p = &raw mut STRKEY_BUFFER as *mut c_char;
        sprintf(p, fmt.as_ptr() as *const c_char, n);
        p
    }
}

// Map entry layout used by sh_geti: { char *key; int value; } with default
// alignment yields 8 + 4 + 4 padding = 16 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
struct ShEntry {
    key: *mut c_char,
    value: c_int,
    _pad: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    unsafe {
        let mut strmap: *mut ShEntry = ptr::null_mut();
        let mut sa = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
            _pad: [0; 6],
        };
        let elemsize = core::mem::size_of::<ShEntry>();

        // First loop: stralloc each strkey(i), then strreset.
        for i in 0..num {
            stbds_stralloc(&mut sa, strkey(i));
        }
        stbds_strreset(&mut sa);

        for j in 0..2 {
            // shgeti(strmap,"foo") == -1
            let foo = b"foo\0";
            let _ = shgeti(&mut strmap, foo.as_ptr() as *mut c_char);

            if j == 0 {
                // sh_new_strdup(strmap)
                strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP) as *mut ShEntry;
            } else {
                // sh_new_arena(strmap)
                strmap = stbds_shmode_func(elemsize, STBDS_SH_ARENA) as *mut ShEntry;
            }

            let _ = shgeti(&mut strmap, foo.as_ptr() as *mut c_char);

            // shdefault(strmap, -2): writes value at strmap[-1].
            (*strmap.offset(-1)).value = -2;

            let _ = shgeti(&mut strmap, foo.as_ptr() as *mut c_char);

            // shput(strmap, strkey(i), i*3) for i in 0..num step 2.
            let mut i = 0;
            while i < num {
                shput(&mut strmap, strkey(i), i.wrapping_mul(3));
                i += 2;
            }

            // print loop.
            let len_signed = shlen(strmap);
            let fmt = b"%s %d\n\0";
            let mut z: isize = 0;
            while z < len_signed {
                let entry = strmap.offset(z);
                printf(
                    fmt.as_ptr() as *const c_char,
                    (*entry).key,
                    (*entry).value,
                );
                z += 1;
            }

            // for (i=0; i<num; i++) shget(strmap, strkey(i)) checks (no observable
            // effect, but we still execute to match allocations/seed updates).
            let mut i = 0;
            while i < num {
                let _ = shget(&mut strmap, strkey(i));
                i += 1;
            }
            // for (i=2; i<num; i+=4) shdel(strmap, strkey(i));
            let mut i = 2;
            while i < num {
                shdel(&mut strmap, strkey(i));
                i += 4;
            }
            // for (i=0; i<num; i++) shget — no observable effect.
            let mut i = 0;
            while i < num {
                let _ = shget(&mut strmap, strkey(i));
                i += 1;
            }
            // for (i=0; i<num; i++) shdel(strmap, strkey(i));
            let mut i = 0;
            while i < num {
                shdel(&mut strmap, strkey(i));
                i += 1;
            }
            // for (i=0; i<num; i++) shget — no observable effect.
            let mut i = 0;
            while i < num {
                let _ = shget(&mut strmap, strkey(i));
                i += 1;
            }

            // shfree(strmap):
            shfree(&mut strmap);
        }
    }
}

// ---------------------------------------------------------------------------
// Inline helpers replicating the C macros that operate on `strmap`.
// ---------------------------------------------------------------------------

#[inline(always)]
unsafe fn shgeti(t: &mut *mut ShEntry, k: *mut c_char) -> isize {
    unsafe {
        let elemsize = core::mem::size_of::<ShEntry>();
        let key_size = core::mem::size_of::<*mut c_char>();
        let new_t = stbds_hmget_key(
            *t as *mut c_void,
            elemsize,
            k as *mut c_void,
            key_size,
            STBDS_HM_STRING,
        ) as *mut ShEntry;
        *t = new_t;
        // stbds_temp((t)-1)
        if !new_t.is_null() {
            let raw = hash_to_arr(new_t as *mut c_void, elemsize);
            (*header(raw)).temp
        } else {
            -1
        }
    }
}

#[inline(always)]
unsafe fn shput(t: &mut *mut ShEntry, k: *mut c_char, v: c_int) {
    unsafe {
        let elemsize = core::mem::size_of::<ShEntry>();
        let key_size = core::mem::size_of::<*mut c_char>();
        let new_t = stbds_hmput_key(
            *t as *mut c_void,
            elemsize,
            k as *mut c_void,
            key_size,
            STBDS_HM_STRING,
        ) as *mut ShEntry;
        *t = new_t;
        let raw = hash_to_arr(new_t as *mut c_void, elemsize);
        let temp = (*header(raw)).temp;
        // strmap[temp].value = v (no .key write because shput uses temp_key path
        // already populated by stbds_hmput_key).
        (*new_t.offset(temp)).value = v;
    }
}

#[inline(always)]
unsafe fn shget(t: &mut *mut ShEntry, k: *mut c_char) -> c_int {
    unsafe {
        let i = shgeti(t, k);
        let new_t = *t;
        (*new_t.offset(i)).value
    }
}

#[inline(always)]
unsafe fn shdel(t: &mut *mut ShEntry, k: *mut c_char) -> isize {
    unsafe {
        let elemsize = core::mem::size_of::<ShEntry>();
        let key_size = core::mem::size_of::<*mut c_char>();
        let keyoffset = 0;
        let new_t = stbds_hmdel_key(
            *t as *mut c_void,
            elemsize,
            k as *mut c_void,
            key_size,
            keyoffset,
            STBDS_HM_STRING,
        ) as *mut ShEntry;
        *t = new_t;
        if !new_t.is_null() {
            let raw = hash_to_arr(new_t as *mut c_void, elemsize);
            (*header(raw)).temp
        } else {
            0
        }
    }
}

#[inline(always)]
unsafe fn shlen(t: *mut ShEntry) -> isize {
    unsafe {
        if t.is_null() {
            0
        } else {
            let elemsize = core::mem::size_of::<ShEntry>();
            let raw = hash_to_arr(t as *mut c_void, elemsize);
            (*header(raw)).length as isize - 1
        }
    }
}

#[inline(always)]
unsafe fn shfree(t: &mut *mut ShEntry) {
    unsafe {
        if !(*t).is_null() {
            let elemsize = core::mem::size_of::<ShEntry>();
            let raw = hash_to_arr(*t as *mut c_void, elemsize);
            stbds_hmfree_func(raw, elemsize);
        }
        *t = ptr::null_mut();
    }
}
