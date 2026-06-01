// Translation of c_src/src/lib.c -- byte-identical reimplementation of stb_ds + str_put.
// Public C ABI functions are exported with #[no_mangle] / extern "C" using their final
// linker symbol names (matches `nm -D` of the C .so).

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ----------------------- Constants -----------------------

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

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

// ----------------------- Types -----------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
}

#[repr(C)]
#[derive(Copy, Clone)]
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

// ----------------------- Helpers -----------------------

#[inline(always)]
unsafe fn header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline(always)]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as isize
    }
}

#[inline(always)]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

#[inline(always)]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline(always)]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline(always)]
unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*header(a)).hash_table as *mut stbds_hash_index
}

#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize {
    ((n) + (a) - 1) & !((a) - 1)
}

#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

// ----------------------- Globals -----------------------

static mut STBDS_HASH_SEED: usize = 0x31415926;

// ----------------------- arrgrowf -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len: usize = (arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= arrcap(a) {
        return a;
    }

    if min_cap < 2 * arrcap(a) {
        min_cap = 2 * arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_header_ptr: *mut c_void = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };
    let new_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(core::mem::size_of::<stbds_array_header>());
    let b_raw = libc::realloc(old_header_ptr, new_size);
    let b = (b_raw as *mut u8).add(core::mem::size_of::<stbds_array_header>()) as *mut c_void;
    if a.is_null() {
        (*header(b)).length = 0;
        (*header(b)).hash_table = ptr::null_mut();
        (*header(b)).temp = 0;
    }
    (*header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    libc::free(header(a) as *mut c_void);
}

// ----------------------- rand_seed -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// ----------------------- log2 / probe_position -----------------------

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

// ----------------------- make_hash_index -----------------------

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
        + core::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = libc::realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    let after_t = t.add(1) as usize;
    (*t).storage = align_fwd(after_t, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
    debug_assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        // memset string to zero
        (*t).string = stbds_string_arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64 macro:
        // a = (v64_hi << 32) ^ ((v64_lo ^ v32) << 16 << 16 >> 16 >> 16) ^ v32
        // For 64-bit:
        //   temp = v64_lo ^ v32; temp = (temp << 32) >> 32 = lower 32 bits of (v64_lo ^ v32)
        //   var = v64_hi << 32; var ^= temp ^ v32
        let a_seed: usize = {
            let v32: usize = 2147001325usize;
            let v64_hi: usize = 0x27bb2ee6usize;
            let v64_lo: usize = 0x87b0b0fdusize;
            let temp: usize = ((v64_lo ^ v32) << 16) << 16 >> 16 >> 16;
            ((v64_hi << 16) << 16) ^ temp ^ v32
        };
        let b_seed: usize = {
            let v32: usize = 715136305usize;
            let v64_hi: usize = 0usize;
            let v64_lo: usize = 0xb504f32dusize;
            let temp: usize = ((v64_lo ^ v32) << 16) << 16 >> 16 >> 16;
            ((v64_hi << 16) << 16) ^ temp ^ v32
        };
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a_seed).wrapping_add(b_seed);
    }

    {
        let n_buckets = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..n_buckets {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let n_buckets_old = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..n_buckets_old {
            let ob = (*ot).storage.add(i);
            'outer: for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let z_start = pos & STBDS_BUCKET_MASK;
                        for z in z_start..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                continue 'outer;
                            }
                        }
                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                continue 'outer;
                            }
                        }
                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }

    t
}

// ----------------------- hash_string -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut p = str as *mut u8;
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

// ----------------------- siphash_bytes -----------------------

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;
    let mut data: usize;

    let mut v0: usize = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1: usize = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2: usize = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3: usize = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    macro_rules! sipround {
        () => {{
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
        }};
    }

    let mut i: usize = 0;
    while i + core::mem::size_of::<usize>() <= len {
        // C: data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        //    data |= (size_t)(d[4]|(d[5]<<8)|(d[6]<<16)|(d[7]<<24)) << 16 << 16;
        // The C uses int promotion for the lower part. d[3] << 24 in C with d as
        // unsigned char promotes to int, value 0..255 << 24 fits in int. So OK.
        // For 64-bit size_t, the higher half is computed as a size_t cast then shifted.
        let lo: u32 = (*d.add(0)) as u32
            | ((*d.add(1)) as u32) << 8
            | ((*d.add(2)) as u32) << 16
            | ((*d.add(3)) as u32) << 24;
        let hi: u32 = (*d.add(4)) as u32
            | ((*d.add(5)) as u32) << 8
            | ((*d.add(6)) as u32) << 16
            | ((*d.add(7)) as u32) << 24;
        // Sign-extend lo as int per C semantics: data = (int) lo, then data |= (size_t)hi << 32
        // Actually data is size_t. C: data = d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24).
        // d[3]<<24 is int (since d[3] promotes to int). For unsigned char with high bit,
        // d[3]<<24 may overflow; this is actually UB but in practice produces a negative int.
        // Then assigning to size_t sign-extends.
        // To match: take lo as i32, sign-extend to isize, cast to usize.
        data = (lo as i32) as isize as usize;
        data |= (hi as usize) << 16 << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        i += core::mem::size_of::<usize>();
        d = d.add(core::mem::size_of::<usize>());
    }
    data = len << (STBDS_SIZE_T_BITS - 8);
    // Switch with fall-through.
    let rem = len - i;
    // Use sequential ifs to mimic fall-through.
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
        // C: data |= (d[3] << 24); — d[3] is unsigned char promoted to int.
        let v = ((*d.add(3)) as i32) << 24;
        data |= v as isize as usize;
    }
    if rem >= 3 {
        data |= ((*d.add(2)) as usize) << 16;
    }
    if rem >= 2 {
        data |= ((*d.add(1)) as usize) << 8;
    }
    if rem >= 1 {
        data |= (*d.add(0)) as usize;
    }
    // case 0: break (no-op)

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ----------------------- is_key_equal -----------------------

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
        // strcmp((char *) key, * (char **) ((char *) a + elemsize*i + keyoffset)) == 0
        let stored_ptr_loc = (a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char;
        let stored = *stored_ptr_loc;
        libc::strcmp(key as *const c_char, stored) == 0
    } else {
        let stored = (a as *mut u8).add(elemsize * i + keyoffset) as *const c_void;
        libc::memcmp(key as *const c_void, stored, keysize) == 0
    }
}

// ----------------------- hmfree_func -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !hash_table(a).is_null() {
        if (*hash_table(a)).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*header(a)).length {
                let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                libc::free(*p as *mut c_void);
            }
        }
        stbds_strreset(&mut (*hash_table(a)).string);
    }
    libc::free((*header(a)).hash_table);
    libc::free(header(a) as *mut c_void);
}

// ----------------------- hm_find_slot -----------------------

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table(raw_a);
    let mut hash: usize = if mode >= STBDS_HM_STRING {
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
        let i_start = pos & STBDS_BUCKET_MASK;
        for i in i_start..STBDS_BUCKET_LENGTH {
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
                ) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

// ----------------------- hmget_key_ts -----------------------

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
        (*header(a)).length += 1;
        libc::memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(a, elemsize);
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

// ----------------------- hmget_key -----------------------

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
    (*header(hash_to_arr(p, elemsize))).temp = temp;
    p
}

// ----------------------- hmput_default -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let in_arr = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(in_arr, elemsize, 0, 1);
        (*header(a)).length += 1;
        libc::memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

// ----------------------- strdup -----------------------

unsafe fn stbds_strdup(str: *mut c_char) -> *mut c_char {
    let len = libc::strlen(str) + 1;
    let p = libc::realloc(ptr::null_mut(), len) as *mut c_char;
    libc::memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ----------------------- hmput_key -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        libc::memset(a, 0, elemsize);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count: usize = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            libc::free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                STBDS_SH_NONE
            };
        }
        (*header(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    {
        let mut hash: usize = if mode >= STBDS_HM_STRING {
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

        'main_loop: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let i_start = pos & STBDS_BUCKET_MASK;
            for i in i_start..STBDS_BUCKET_LENGTH {
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
                        (*header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            // stbds_temp_key(a) = *(char**)((char*)raw_a + elemsize*idx + keyoffset)
                            let p = (raw_a as *mut u8)
                                .add(elemsize * ((*bucket).index[i] as usize) + keyoffset)
                                as *mut *mut c_char;
                            *((*header(a)).hash_table as *mut *mut c_char) = *p;
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'main_loop;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
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
                        (*header(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'main_loop;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
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
            let i: isize = arrlen(a);
            if (i as usize) + 1 > arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);

            debug_assert!((i as usize) + 1 <= arrcap(a));
            (*header(a)).length = (i as usize) + 1;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            (*header(a)).temp = i - 1;

            let dest = (a as *mut u8).add(elemsize * (i as usize)) as *mut *mut c_char;
            // table->string.mode
            let mode_val = (*table).string.mode;
            match mode_val {
                STBDS_SH_STRDUP => {
                    let dup = stbds_strdup(key as *mut c_char);
                    *dest = dup;
                    *((*header(a)).hash_table as *mut *mut c_char) = dup;
                }
                STBDS_SH_ARENA => {
                    let allocated = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                    *dest = allocated;
                    *((*header(a)).hash_table as *mut *mut c_char) = allocated;
                }
                STBDS_SH_DEFAULT => {
                    *dest = key as *mut c_char;
                    *((*header(a)).hash_table as *mut *mut c_char) = key as *mut c_char;
                }
                _ => {
                    libc::memcpy(
                        (a as *mut u8).add(elemsize * (i as usize)) as *mut c_void,
                        key,
                        keysize,
                    );
                }
            }
            // suppress unused
            let _ = raw_a;
        }
        arr_to_hash(a, elemsize)
    }
}

// ----------------------- shmode_func -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    libc::memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

// ----------------------- hmdel_key -----------------------

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
    let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
    (*header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index: isize = arrlen(raw_a) - 1 - 1;
    debug_assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    debug_assert!((*table).used_count <= isize::MAX as usize);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = (a as *mut u8).add(elemsize * (old_index as usize)) as *mut *mut c_char;
        libc::free(*p as *mut c_void);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).add(elemsize * (old_index as usize)) as *mut c_void,
            (a as *mut u8).add(elemsize * (final_index as usize)) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let key_loc =
                (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset) as *mut *mut c_char;
            slot = stbds_hm_find_slot(a, elemsize, *key_loc as *mut c_void, keysize, keyoffset, mode);
        } else {
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        }
        debug_assert!(slot >= 0);
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        debug_assert!((*b).index[i] == final_index);
        (*b).index[i] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        libc::free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        libc::free(table as *mut c_void);
    }

    a
}

// ----------------------- stralloc -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut c_char,
) -> *mut c_char {
    let len: usize = libc::strlen(str) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // sizeof(stbds_string_block) = 16 (pointer + 8 bytes); -8 for storage. So overhead is 8.
            // C: sizeof(*sb) - 8 + len
            let alloc_size = core::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = libc::realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            libc::memmove((*sb).storage.as_mut_ptr() as *mut c_void, str as *const c_void, len);
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
            let alloc_size = core::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = libc::realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    debug_assert!(len <= (*a).remaining);
    let p = ((*(*a).storage).storage.as_mut_ptr()).add((*a).remaining - len);
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, str as *const c_void, len);
    p
}

// ----------------------- strreset -----------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        libc::free(x as *mut c_void);
        x = y;
    }
    libc::memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}

// ----------------------- strkey + str_put -----------------------

// static char buffer[256] in the C source -- replicate as a static mut.
static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let fmt = b"test_%d\0";
    let buf_ptr = (&raw mut BUFFER) as *mut c_char;
    libc::sprintf(buf_ptr, fmt.as_ptr() as *const c_char, n);
    buf_ptr
}

// Rust translation of str_put. The C struct used is:
//   struct { char *key; int value; }
// Total size 16 bytes (8 + 4 + 4 padding) on 64-bit; representable in Rust with
// #[repr(C)] for layout matching the in-array slot.
#[repr(C)]
#[derive(Copy, Clone)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let mut strmap: *mut StrMapEntry = ptr::null_mut();
    let mut sa: stbds_string_arena = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    for i in 0..num {
        stbds_stralloc(&mut sa, strkey(i));
    }
    stbds_strreset(&mut sa);

    {
        // s.key = "a"; s.value = num;
        let key_a = b"a\0".as_ptr() as *mut c_char;
        let s = StrMapEntry {
            key: key_a,
            value: num,
        };

        // shputs(strmap, s):
        // (t) = stbds_hmput_key_wrapper((t), sizeof *(t), (void*) (s).key, sizeof (t)->key, STBDS_HM_STRING);
        // (t)[stbds_temp((t)-1)] = (s);
        // (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1);
        let elemsize = core::mem::size_of::<StrMapEntry>();
        let keysize = core::mem::size_of::<*mut c_char>();
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            s.key as *mut c_void,
            keysize,
            STBDS_HM_STRING,
        ) as *mut StrMapEntry;
        let t_arr = hash_to_arr(strmap as *mut c_void, elemsize);
        let temp = (*header(t_arr)).temp;
        // Copy struct s into strmap[temp]
        *strmap.offset(temp) = s;
        // strmap[temp].key = stbds_temp_key(strmap-1)  -- which is *(char**)hash_table.
        let hash_t = (*header(t_arr)).hash_table as *mut *mut c_char;
        (*strmap.offset(temp)).key = *hash_t;

        debug_assert!(*((*strmap.offset(0)).key) == b'a' as c_char);
        debug_assert!((*strmap.offset(0)).key == s.key);
        debug_assert!((*strmap.offset(0)).value == s.value);

        // shlen(strmap) -> stbds_hmlen
        let length = (*header(t_arr)).length;
        let z_count: isize = if length == 0 { 0 } else { length as isize - 1 };

        // for (int z=0; z < shlen(strmap); ++z)
        //     printf("%s %d\n", strmap[z], strmap[z].value);
        // The struct is passed by value as variadic. On x86_64 SysV, struct
        // {char*, int} of size 16 is split into two registers: key in RSI,
        // value (sign-extended? actually padding) in RDX. The format string
        // reads RSI for %s (= key) and RDX for %d (= value low 4 bytes).
        // Then strmap[z].value goes in RCX (unused).
        // Net effect: prints "<key> <value>\n".
        for z in 0..z_count {
            let entry = *strmap.offset(z);
            libc::printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                entry.key,
                entry.value,
            );
        }

        // shfree(strmap) => stbds_hmfree(strmap)
        // ((p) != NULL ? stbds_hmfree_func((p)-1, sizeof*(p)),0 : 0), (p)=NULL
        if !strmap.is_null() {
            stbds_hmfree_func(t_arr, elemsize);
        }
        // strmap = NULL (not used after)
    }
}
