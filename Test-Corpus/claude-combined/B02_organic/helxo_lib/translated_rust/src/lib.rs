// Rust translation of stb_ds-style C library + helxo function.
//
// All public C functions are exposed via #[unsafe(no_mangle)] and extern "C".
// Logic mirrors the C code byte-for-byte where observable.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

// -------------- Constants matching C macros --------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

#[allow(dead_code)]
const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() * 8) as u32;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// -------------- Layout-compatible structures ---------------

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
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
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

// -------------- Hash seed (mutable global) --------------

static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x31415926);

// -------------- Allocation helpers (use libc malloc/realloc/free
// to match C ABI behavior) --------------

unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    libc::realloc(p, s) as *mut c_void
}

unsafe fn stbds_free(p: *mut c_void) {
    libc::free(p)
}

// -------------- Helpers for accessing the array header --------------

#[inline(always)]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline(always)]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline(always)]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline(always)]
unsafe fn stbds_temp(t: *mut c_void) -> isize {
    (*stbds_header(t)).temp
}

#[inline(always)]
unsafe fn stbds_set_temp(t: *mut c_void, v: isize) {
    (*stbds_header(t)).temp = v;
}

#[inline(always)]
unsafe fn stbds_temp_key_set(t: *mut c_void, v: *mut c_char) {
    let ht = (*stbds_header(t)).hash_table as *mut *mut c_char;
    *ht = v;
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
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// -------------- arrgrowf / arrfreef --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_ptr: *mut c_void = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };
    let new_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(std::mem::size_of::<stbds_array_header>());
    let mut b = stbds_realloc(old_ptr, new_size);
    b = (b as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;

    if a.is_null() {
        let h = stbds_header(b);
        (*h).length = 0;
        (*h).hash_table = ptr::null_mut();
        (*h).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    stbds_free(stbds_header(a) as *mut c_void);
}

// -------------- rand_seed --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED.store(seed, Ordering::Relaxed);
}

// -------------- log2 helpers --------------

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline(always)]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// -------------- make_hash_index --------------

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let raw = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    let t = raw;

    // Align storage forward to STBDS_CACHE_LINE_SIZE
    let after = (t.add(1)) as usize;
    let aligned = (after + STBDS_CACHE_LINE_SIZE - 1) & !(STBDS_CACHE_LINE_SIZE - 1);
    (*t).storage = aligned as *mut stbds_hash_bucket;
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

    if !ot.is_null() {
        (*t).string = std::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        // memset(&t->string, 0, sizeof(t->string))
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        let cur_seed = STBDS_HASH_SEED.load(Ordering::Relaxed);
        (*t).seed = cur_seed;
        // Compute new seed:
        //   stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
        //   stbds_load_32_or_64(b,temp,  715136305,          0, 0xb504f32d);
        //   stbds_hash_seed = stbds_hash_seed * a + b;
        // For 64-bit size_t: a = (v64_hi << 32) | v64_lo; b = (v64_hi << 32) | v64_lo
        let a: usize = if std::mem::size_of::<usize>() == 8 {
            ((0x27bb2ee6usize) << 32) | 0x87b0b0fdusize
        } else {
            2147001325usize
        };
        let b: usize = if std::mem::size_of::<usize>() == 8 {
            (0usize << 32) | 0xb504f32dusize
        } else {
            715136305usize
        };
        let new_seed = cur_seed.wrapping_mul(a).wrapping_add(b);
        STBDS_HASH_SEED.store(new_seed, Ordering::Relaxed);
    }

    // Initialize bucket hashes/indices
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

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let old_n = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..old_n {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
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

// -------------- hash_string / hash_bytes --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut p = str as *const u8;
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

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    let nseed = !seed;

    v0 = ((((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = ((((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d)) ^ nseed;
    v2 = ((((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = ((((0x74656462usize) << 16) << 16).wrapping_add(0x79746573)) ^ nseed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ nseed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ nseed;

    macro_rules! siphash_round {
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

    let szt = std::mem::size_of::<usize>();
    let mut i: usize = 0;
    while i + szt <= len {
        let b0 = *d as u32;
        let b1 = *d.add(1) as u32;
        let b2 = *d.add(2) as u32;
        let b3 = *d.add(3) as u32;
        let lo = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
        data = lo as usize;
        let b4 = *d.add(4) as u32;
        let b5 = *d.add(5) as u32;
        let b6 = *d.add(6) as u32;
        let b7 = *d.add(7) as u32;
        let hi = b4 | (b5 << 8) | (b6 << 16) | (b7 << 24);
        // (size_t) (hi) << 16 << 16 - discarded if size_t==4
        if szt == 8 {
            data |= ((hi as usize) << 16) << 16;
        } else {
            // discarded in 32-bit
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round!();
        }
        v0 ^= data;

        i += szt;
        d = d.add(szt);
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // C uses fall-through cases; replicate with shifts that match
    // exactly what each case writes. Each case ORs a byte (or word) shifted
    // to specific position, using d[k] from the local pointer (still pointing
    // at the start of the tail, since we incremented above).
    //
    // case 7: data |= ((size_t) d[6] << 24) << 24;
    // case 6: data |= ((size_t) d[5] << 20) << 20;
    // case 5: data |= ((size_t) d[4] << 16) << 16;
    // case 4: data |= (d[3] << 24);
    // case 3: data |= (d[2] << 16);
    // case 2: data |= (d[1] << 8);
    // case 1: data |= d[0];
    // case 0: break;
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
        // (d[3] << 24) - in C, d[3] is unsigned char promoted to int; shifted by 24 yields int.
        // Cast result to size_t (via |=). For signed int 0xff << 24 would be UB but we treat as
        // wrapping u32→usize.
        let v = (*d.add(3) as u32).wrapping_shl(24) as i32 as isize as usize;
        data |= v;
    }
    if rem >= 3 {
        let v = (*d.add(2) as u32).wrapping_shl(16) as i32 as isize as usize;
        data |= v;
    }
    if rem >= 2 {
        let v = (*d.add(1) as u32).wrapping_shl(8) as i32 as isize as usize;
        data |= v;
    }
    if rem >= 1 {
        data |= *d as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siphash_round!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siphash_round!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// -------------- key equality test --------------

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    let entry_ptr = (a as *mut u8).add((elemsize as isize * i + keyoffset as isize) as usize);
    if mode >= STBDS_HM_STRING {
        // strcmp((char*)key, *(char**)entry_ptr) == 0
        let entry_key = *(entry_ptr as *mut *mut c_char);
        libc::strcmp(key as *const c_char, entry_key) == 0
    } else {
        libc::memcmp(key, entry_ptr as *mut c_void, keysize) == 0
    }
}

// -------------- hmfree_func --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                let p = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                stbds_free(*p as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string as *mut stbds_string_arena);
    }
    stbds_free((*stbds_header(a)).hash_table);
    stbds_free(stbds_header(a) as *mut c_void);
}

// -------------- hm_find_slot --------------

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
        hash = hash.wrapping_add(2);
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
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

// -------------- hmget_key_ts / hmget_key --------------

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
                let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
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
    stbds_set_temp(hash_to_arr(p, elemsize), temp);
    p
}

// -------------- hmput_default --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let raw_in: *mut c_void = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(raw_in, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

// -------------- strdup --------------

unsafe fn stbds_strdup_internal(s: *mut c_char) -> *mut c_char {
    let len = libc::strlen(s) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;
    libc::memmove(p as *mut c_void, s as *const c_void, len);
    p
}

// -------------- hmput_key --------------

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
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

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
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*stbds_header(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    {
        let mut hash: usize = if mode >= STBDS_HM_STRING {
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

        'search: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            // First half: from (pos & MASK) to BUCKET_LENGTH
            let start = pos & STBDS_BUCKET_MASK;
            let mut found_empty = false;
            let mut empty_at_idx: usize = 0;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i],
                    ) {
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let kp = (raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char;
                            stbds_temp_key_set(a, *kp);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    found_empty = true;
                    empty_at_idx = i;
                    break;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }
            if found_empty {
                pos = (pos & !STBDS_BUCKET_MASK) + empty_at_idx;
                break 'search;
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i],
                    ) {
                        stbds_set_temp(a, (*bucket).index[i]);
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    found_empty = true;
                    empty_at_idx = i;
                    break;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }
            if found_empty {
                pos = (pos & !STBDS_BUCKET_MASK) + empty_at_idx;
                break 'search;
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        // found_empty_slot label
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        let i = stbds_arrlen(a);
        if (i as usize) + 1 > stbds_arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = arr_to_hash(a, elemsize);
        // (raw_a unused after this except for invariants)
        let _ = raw_a;

        (*stbds_header(a)).length = (i as usize) + 1;
        bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        stbds_set_temp(a, i - 1);

        let dst = (a as *mut u8).add(elemsize * (i as usize));
        match (*table).string.mode {
            x if x == STBDS_SH_STRDUP => {
                let dup = stbds_strdup_internal(key as *mut c_char);
                *(dst as *mut *mut c_char) = dup;
                stbds_temp_key_set(a, dup);
            }
            x if x == STBDS_SH_ARENA => {
                let p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                *(dst as *mut *mut c_char) = p;
                stbds_temp_key_set(a, p);
            }
            x if x == STBDS_SH_DEFAULT => {
                *(dst as *mut *mut c_char) = key as *mut c_char;
                stbds_temp_key_set(a, key as *mut c_char);
            }
            _ => {
                libc::memcpy(dst as *mut c_void, key as *const c_void, keysize);
            }
        }

        arr_to_hash(a, elemsize)
    }
}

// -------------- shmode_func --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

// -------------- hmdel_key --------------

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
    stbds_set_temp(raw_a, 0);
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
    let final_index = stbds_arrlen(raw_a) - 1 - 1;

    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    stbds_set_temp(raw_a, 1);
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = (a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char;
        stbds_free(*p as *mut c_void);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
            (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let kp = *((a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                as *mut *mut c_char) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
        } else {
            let kp = (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
        }
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        let nt = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*stbds_header(raw_a)).hash_table = nt as *mut c_void;
        stbds_free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let nt = stbds_make_hash_index((*table).slot_count, table);
        (*stbds_header(raw_a)).hash_table = nt as *mut c_void;
        stbds_free(table as *mut c_void);
    }

    a
}

// -------------- stralloc --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    s: *mut c_char,
) -> *mut c_char {
    let len = libc::strlen(s) + 1;
    if len > (*a).remaining {
        let blocksize_pre = (*a).block as usize;
        let blocksize: usize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_pre >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let alloc_size =
                std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            libc::memmove(
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
            let alloc_size =
                std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
            // unused after assigning to remaining; suppress warning
            let _ = blocksize;
        }
    }

    let p = (*(*a).storage)
        .storage
        .as_mut_ptr()
        .add((*a).remaining - len);
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, s as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut c_void);
        x = y;
    }
    ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
}

// -------------- strkey --------------

// 256-byte static buffer matching the C `static char buffer[256]`.
// Use a Mutex-guarded raw buffer to allow returning a stable pointer.
// The C version is not thread safe (it's a plain global), so we don't
// need to be either; use a static UnsafeCell.

#[repr(C)]
struct StaticBuffer {
    data: [c_char; 256],
}
unsafe impl Sync for StaticBuffer {}

static mut BUFFER: StaticBuffer = StaticBuffer { data: [0; 256] };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf_ptr = (&raw mut BUFFER).cast::<c_char>();
    let fmt = b"test_%d\0".as_ptr() as *const c_char;
    libc::sprintf(buf_ptr, fmt, n);
    buf_ptr
}

// -------------- helxo --------------
//
// helxo(letter):
//   struct { char *key; char value; } *hash = NULL;
//   char name[4] = "jen";
//   shput(hash, "bob", 'h');
//   shput(hash, "sally", 'e');
//   shput(hash, "fred", 'l');
//   shput(hash, "jen", 'x');
//   shput(hash, "doug", 'o');
//   shput(hash, name, letter);
//   for (int z=0; z < shlen(hash); ++z)
//      printf("%s %c\n", hash[z], hash[z].value);
//   shfree(hash);
//
// shput is implemented via stbds_hmput_key with mode=STBDS_HM_STRING and elemsize=sizeof(struct).
// The struct {char*; char} has size 16, alignment 8, key at offset 0, value at offset 8.

#[repr(C)]
struct HelxoEntry {
    key: *mut c_char,
    value: c_char,
}

const HELXO_ELEMSIZE: usize = std::mem::size_of::<HelxoEntry>();

unsafe fn shput_helxo(hash: *mut c_void, key: *const c_char, value: c_char) -> *mut c_void {
    // sizeof((t)->key) = 8 (size of char*); we pass keysize=8 but mode=STRING ignores it.
    let new_hash = stbds_hmput_key(
        hash,
        HELXO_ELEMSIZE,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        STBDS_HM_STRING,
    );
    let temp_idx = stbds_temp(hash_to_arr(new_hash, HELXO_ELEMSIZE));
    let entry = (new_hash as *mut HelxoEntry).offset(temp_idx);
    (*entry).value = value;
    new_hash
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    let mut hash: *mut c_void = ptr::null_mut();
    let mut name: [c_char; 4] = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];

    let bob = b"bob\0".as_ptr() as *const c_char;
    let sally = b"sally\0".as_ptr() as *const c_char;
    let fred = b"fred\0".as_ptr() as *const c_char;
    let jen = b"jen\0".as_ptr() as *const c_char;
    let doug = b"doug\0".as_ptr() as *const c_char;

    hash = shput_helxo(hash, bob, b'h' as c_char);
    hash = shput_helxo(hash, sally, b'e' as c_char);
    hash = shput_helxo(hash, fred, b'l' as c_char);
    hash = shput_helxo(hash, jen, b'x' as c_char);
    hash = shput_helxo(hash, doug, b'o' as c_char);
    hash = shput_helxo(hash, name.as_mut_ptr(), letter);

    let len = if hash.is_null() {
        0
    } else {
        // shlen = arr length - 1 (subtract reserved slot)
        (*stbds_header(hash_to_arr(hash, HELXO_ELEMSIZE))).length as isize - 1
    };

    let fmt = b"%s %c\n\0".as_ptr() as *const c_char;
    for z in 0..len {
        let entry_ptr = (hash as *mut HelxoEntry).offset(z);
        // Match C: printf("%s %c\n", hash[z], hash[z].value)
        // hash[z] is the struct passed by value. On x86_64 SysV, the struct fits
        // in two integer slots: first slot = key pointer, second slot = value
        // (with padding). Then hash[z].value is the third slot.
        // %s reads the first slot (key pointer); %c reads the next slot
        // (which is the second slot of the struct, containing value in low byte).
        //
        // To reproduce byte-identically without varargs struct passing, just
        // pass key + value directly.
        libc::printf(fmt, (*entry_ptr).key, (*entry_ptr).value as c_int);
    }

    if !hash.is_null() {
        stbds_hmfree_func(hash_to_arr(hash, HELXO_ELEMSIZE), HELXO_ELEMSIZE);
    }
}
