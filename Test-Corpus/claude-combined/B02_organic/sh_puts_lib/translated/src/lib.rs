// Rust translation of stb_ds.h hashmap library + sh_puts test function.
// Designed to be byte-identical with the C version. Uses libc allocation.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(static_mut_refs)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

// ---------- Constants ----------

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

#[allow(dead_code)]
const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_SIZE_T_BITS: u32 = 64;

// ---------- Structs (must match C layout exactly) ----------

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct StbdsHashIndex {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StbdsStringArena,
    storage: *mut StbdsHashBucket,
}

// ---------- Helpers (mirroring C macros) ----------

#[inline]
unsafe fn header(a: *mut c_void) -> *mut StbdsArrayHeader {
    (a as *mut StbdsArrayHeader).offset(-1)
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as isize
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    (*header(a)).hash_table as *mut StbdsHashIndex
}

#[inline]
fn rotate_left_sz(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right_sz(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// Mutable global hash seed (matches C `static size_t stbds_hash_seed`).
static mut STBDS_HASH_SEED: usize = 0x31415926;

// ---------- Public exports ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (arrlen(a) as usize).wrapping_add(addlen);

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

    let old_block = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };

    let new_size = elemsize * min_cap + size_of::<StbdsArrayHeader>();
    let raw = libc::realloc(old_block, new_size);
    let b = (raw as *mut u8).add(size_of::<StbdsArrayHeader>()) as *mut c_void;

    if a.is_null() {
        let h = header(b);
        (*h).length = 0;
        (*h).hash_table = ptr::null_mut();
        (*h).temp = 0;
    }
    (*header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    libc::free(header(a) as *mut c_void);
}

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<StbdsHashBucket>()
        + size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = libc::realloc(ptr::null_mut(), alloc_size) as *mut StbdsHashIndex;

    let after_t = t.add(1) as usize;
    let aligned = (after_t + STBDS_CACHE_LINE_SIZE - 1) & !(STBDS_CACHE_LINE_SIZE - 1);
    (*t).storage = aligned as *mut StbdsHashBucket;

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
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        // Initialize string arena to 0
        (*t).string = StbdsStringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64(a, ...): yields a = 0x27bb2ee687b0b0fd
        // stbds_load_32_or_64(b, ...): yields b = 0xb504f32d
        let a: usize = 0x27bb2ee687b0b0fd;
        let b: usize = 0xb504f32d;
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    // Initialize all buckets
    for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
        let bucket = (*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bucket).hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bucket).index[j] = STBDS_INDEX_EMPTY;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                let idx_j = (*ob).index[j];
                if idx_j >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let start = pos & STBDS_BUCKET_MASK;
                        let mut z = start;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                            z += 1;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    while *str_ != 0 {
        hash = rotate_left_sz(hash, 9).wrapping_add(*(str_ as *mut u8) as usize);
        str_ = str_.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right_sz(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right_sz(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right_sz(hash, 22);
    hash.wrapping_add(seed)
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;

    let mut v0: usize = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1: usize = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2: usize = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3: usize = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left(STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left(STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
            v3 ^= v0;
        };
    }

    let sz = size_of::<usize>();
    let mut i: usize = 0;
    while i + sz <= len {
        // C performs arithmetic in `int`, then implicitly converts to size_t —
        // which sign-extends the upper 32 bits if the int result is negative
        // (e.g. d[3] >= 0x80). Replicate that behavior exactly.
        let b0 = *d.add(0) as i32;
        let b1 = *d.add(1) as i32;
        let b2 = *d.add(2) as i32;
        let b3 = *d.add(3) as i32;
        let b4 = *d.add(4) as i32;
        let b5 = *d.add(5) as i32;
        let b6 = *d.add(6) as i32;
        let b7 = *d.add(7) as i32;
        let lo_int: i32 = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
        let hi_int: i32 = b4 | (b5 << 8) | (b6 << 16) | (b7 << 24);
        let mut data: usize = lo_int as i64 as usize;
        data |= (((hi_int as i64) as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;

        i += sz;
        d = d.add(sz);
    }
    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // C uses fall-through switch; replicate.
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
        // C: `d[3] << 24` is int arithmetic — if d[3] >= 0x80 the result is
        // negative and is sign-extended when OR'd into size_t.
        let v: i32 = (*d.add(3) as i32) << 24;
        data |= v as i64 as usize;
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
        let stored_key_ptr =
            *((a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char);
        libc::strcmp(key as *const c_char, stored_key_ptr) == 0
    } else {
        libc::memcmp(
            key,
            (a as *mut u8).add(elemsize * i + keyoffset) as *const c_void,
            keysize,
        ) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !hash_table(a).is_null() {
        let table = hash_table(a);
        if (*table).string.mode == STBDS_SH_STRDUP {
            let len = (*header(a)).length;
            for i in 1..len {
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_char);
                libc::free(p as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string as *mut StbdsStringArena);
    }
    libc::free((*header(a)).hash_table);
    libc::free(header(a) as *mut c_void);
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

        let start = pos & STBDS_BUCKET_MASK;
        let mut i = start;
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
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
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
    (*header(hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut a: *mut c_void,
    elemsize: usize,
) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let prev = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(prev, elemsize, 0, 1);
        (*header(a)).length += 1;
        libc::memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    let len = libc::strlen(s) + 1;
    let p = libc::realloc(ptr::null_mut(), len) as *mut c_char;
    libc::memmove(p as *mut c_void, s as *const c_void, len);
    p
}

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

    let mut table = (*header(a)).hash_table as *mut StbdsHashIndex;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
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
                0
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
        let mut tombstone: isize = -1;

        if hash < 2 {
            hash += 2;
        }

        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
        let mut bucket: *mut StbdsHashBucket;

        let pos_for_empty: usize;

        'main_loop: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            let mut i = start;
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
                        (*header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            // stbds_temp_key(a) = *(char**)((char*)raw_a + elemsize*idx + keyoffset)
                            let stored = *((raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char);
                            *((*header(a)).hash_table as *mut *mut c_char) = stored;
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos_for_empty = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'main_loop;
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
                    ) {
                        (*header(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos_for_empty = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'main_loop;
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

        let mut pos = pos_for_empty;
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        {
            let i = arrlen(a);
            if (i as usize) + 1 > arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);
            let _ = raw_a;

            (*header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            (*header(a)).temp = i - 1;

            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let dup = stbds_strdup(key as *mut c_char);
                    *((a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char) = dup;
                    *((*header(a)).hash_table as *mut *mut c_char) = dup;
                }
                STBDS_SH_ARENA => {
                    let arena = &mut (*table).string as *mut StbdsStringArena;
                    let alloc = stbds_stralloc(arena, key as *mut c_char);
                    *((a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char) = alloc;
                    *((*header(a)).hash_table as *mut *mut c_char) = alloc;
                }
                STBDS_SH_DEFAULT => {
                    let kp = key as *mut c_char;
                    *((a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char) = kp;
                    *((*header(a)).hash_table as *mut *mut c_char) = kp;
                }
                _ => {
                    libc::memcpy(
                        (a as *mut u8).add(elemsize * i as usize) as *mut c_void,
                        key,
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
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    libc::memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
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
    let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
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
    let final_index = arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *((a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char);
        libc::free(p as *mut c_void);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
            (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let key_ptr = *((a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                as *mut *mut c_char);
            slot = stbds_hm_find_slot(a, elemsize, key_ptr as *mut c_void, keysize, keyoffset, mode);
        } else {
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        }
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        let new_table = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_a)).hash_table = new_table as *mut c_void;
        libc::free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = stbds_make_hash_index((*table).slot_count, table);
        (*header(raw_a)).hash_table = new_table as *mut c_void;
        libc::free(table as *mut c_void);
    }

    a
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
    s: *mut c_char,
) -> *mut c_char {
    let len = libc::strlen(s) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // Allocate a "huge" block for one string
            let block_alloc_size = size_of::<StbdsStringBlock>() - 8 + len;
            let sb = libc::realloc(ptr::null_mut(), block_alloc_size) as *mut StbdsStringBlock;
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
            let block_alloc_size = size_of::<StbdsStringBlock>() - 8 + blocksize;
            let sb = libc::realloc(ptr::null_mut(), block_alloc_size) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = (*(*a).storage)
        .storage
        .as_mut_ptr()
        .add((*a).remaining)
        .sub(len);
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, s as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        libc::free(x as *mut c_void);
        x = y;
    }
    libc::memset(a as *mut c_void, 0, size_of::<StbdsStringArena>());
}

// ---------- Test buffer + strkey + sh_puts ----------

static mut BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf_ptr = BUFFER.as_mut_ptr();
    libc::sprintf(buf_ptr, b"test_%d\0".as_ptr() as *const c_char, n);
    buf_ptr
}

// Shape of the strmap struct used inside sh_puts.
// Layout: { char* key; int value; } -> 8 + 4 + 4 padding = 16 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
    _pad: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    let mut strmap: *mut StrMapEntry = ptr::null_mut();
    let mut sa = StbdsStringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    // for (i=0; i < num; ++i) stralloc(&sa, strkey(i));
    let mut i: c_int = 0;
    while i < num {
        stbds_stralloc(&mut sa as *mut StbdsStringArena, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa as *mut StbdsStringArena);

    {
        let s = StrMapEntry {
            key: b"a\0".as_ptr() as *mut c_char,
            value: num,
            _pad: 0,
        };
        // sh_new_arena(strmap)
        strmap = stbds_shmode_func(size_of::<StrMapEntry>(), STBDS_SH_ARENA as c_int)
            as *mut StrMapEntry;
        // shputs(strmap, s):
        // (t) = stbds_hmput_key((t), sizeof *t, (void*)s.key, sizeof t->key, STBDS_HM_STRING);
        // (t)[temp(t-1)] = s;
        // (t)[temp(t-1)].key = temp_key(t-1);
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            size_of::<StrMapEntry>(),
            s.key as *mut c_void,
            size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut StrMapEntry;

        let raw = hash_to_arr(strmap as *mut c_void, size_of::<StrMapEntry>());
        let temp_idx = (*header(raw)).temp;
        let temp_key = *((*header(raw)).hash_table as *mut *mut c_char);
        *strmap.offset(temp_idx) = s;
        (*strmap.offset(temp_idx)).key = temp_key;

        // for (z=0; z < shlen(strmap); ++z) printf("%s %d\n", strmap[z], strmap[z].value);
        // shlen(strmap) = stbds_header(strmap-1)->length - 1
        let len_minus_one = if strmap.is_null() {
            0
        } else {
            (*header(raw)).length as isize - 1
        };
        let mut z: isize = 0;
        while z < len_minus_one {
            let entry = &*strmap.offset(z);
            // The C printf(\"%s %d\\n\", strmap[z], strmap[z].value) effectively prints
            // entry.key then entry.value due to x86_64 ABI for the 16-byte struct.
            libc::printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                entry.key,
                entry.value,
            );
            z += 1;
        }

        // shfree(strmap):
        // ((p) != NULL ? stbds_hmfree_func((p)-1, sizeof *p), 0 : 0), (p)=NULL
        if !strmap.is_null() {
            stbds_hmfree_func(
                hash_to_arr(strmap as *mut c_void, size_of::<StrMapEntry>()),
                size_of::<StrMapEntry>(),
            );
        }
        strmap = ptr::null_mut();
        let _ = strmap;
    }
}
