#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(clippy::missing_safety_doc)]
#![allow(static_mut_refs)]

use libc::{c_char, c_int, c_void, free, memcmp, memcpy, memmove, memset, ptrdiff_t, realloc,
           size_t, sprintf, strcmp, strlen};

// ============================================================================
// Constants
// ============================================================================

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: ptrdiff_t = -1;
const STBDS_INDEX_DELETED: ptrdiff_t = -2;

const STBDS_HASH_EMPTY: size_t = 0;
const STBDS_HASH_DELETED: size_t = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: size_t = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: size_t = 1 << 20;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<size_t>() as u32) * 8;

// ============================================================================
// Struct layouts (must match C exactly)
// ============================================================================

#[repr(C)]
struct StbdsArrayHeader {
    length: size_t,
    capacity: size_t,
    hash_table: *mut c_void,
    temp: ptrdiff_t,
}

#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: size_t,
    block: u8,
    mode: u8,
    // padding to 24 bytes (compiler will pad to align)
}

#[repr(C)]
struct StbdsHashBucket {
    hash: [size_t; STBDS_BUCKET_LENGTH],
    index: [ptrdiff_t; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct StbdsHashIndex {
    temp_key: *mut c_char,
    slot_count: size_t,
    used_count: size_t,
    used_count_threshold: size_t,
    used_count_shrink_threshold: size_t,
    tombstone_count: size_t,
    tombstone_count_threshold: size_t,
    seed: size_t,
    slot_count_log2: size_t,
    string: StbdsStringArena,
    storage: *mut StbdsHashBucket,
}

// ============================================================================
// Global state
// ============================================================================

static mut STBDS_HASH_SEED: size_t = 0x31415926;

// ============================================================================
// Helpers (header pointer arithmetic)
// ============================================================================

#[inline]
unsafe fn header(a: *mut c_void) -> *mut StbdsArrayHeader {
    (a as *mut StbdsArrayHeader).offset(-1)
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> size_t {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> ptrdiff_t {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as ptrdiff_t
    }
}

// ============================================================================
// stbds_arrgrowf
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: size_t,
    addlen: size_t,
    mut min_cap: size_t,
) -> *mut c_void {
    let min_len = (arrlen(a) as size_t).wrapping_add(addlen);
    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= arrcap(a) {
        return a;
    }
    if min_cap < arrcap(a).wrapping_mul(2) {
        min_cap = arrcap(a).wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_ptr: *mut c_void = if a.is_null() {
        std::ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };
    let alloc_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(std::mem::size_of::<StbdsArrayHeader>());
    let raw = realloc(old_ptr, alloc_size);
    let b = (raw as *mut u8).add(std::mem::size_of::<StbdsArrayHeader>()) as *mut c_void;
    if a.is_null() {
        let h = header(b);
        (*h).length = 0;
        (*h).hash_table = std::ptr::null_mut();
        (*h).temp = 0;
    }
    (*header(b)).capacity = min_cap;
    b
}

// ============================================================================
// stbds_arrfreef
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(header(a) as *mut c_void);
}

// ============================================================================
// stbds_rand_seed
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: size_t) {
    STBDS_HASH_SEED = seed;
}

// ============================================================================
// stbds_log2 / stbds_probe_position
// ============================================================================

fn stbds_log2(mut slot_count: size_t) -> size_t {
    let mut n: size_t = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline]
fn stbds_probe_position(hash: size_t, slot_count: size_t, _slot_log2: size_t) -> size_t {
    hash & (slot_count - 1)
}

// ============================================================================
// stbds_make_hash_index
// ============================================================================

unsafe fn stbds_make_hash_index(
    slot_count: size_t,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        .wrapping_mul(std::mem::size_of::<StbdsHashBucket>())
        .wrapping_add(std::mem::size_of::<StbdsHashIndex>())
        .wrapping_add(STBDS_CACHE_LINE_SIZE - 1);
    let t = realloc(std::ptr::null_mut(), alloc_size) as *mut StbdsHashIndex;

    let after_t = t.add(1) as size_t;
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
        memset(
            &mut (*t).string as *mut StbdsStringArena as *mut c_void,
            0,
            std::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = STBDS_HASH_SEED;
        // Equivalent of stbds_load_32_or_64: a = 0x27bb2ee687b0b0fd, b = 0xb504f32d
        let a: size_t = 0x27bb2ee687b0b0fd;
        let b: size_t = 0xb504f32d;
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    let n_buckets = slot_count >> STBDS_BUCKET_SHIFT;
    for i in 0..n_buckets {
        let bk = (*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bk).hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bk).index[j] = STBDS_INDEX_EMPTY;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let n_old_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..n_old_buckets {
            let ob = (*ot).storage.add(i);
            'inner: for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    loop {
                        let bk = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let z_start = pos & STBDS_BUCKET_MASK;
                        for z in z_start..STBDS_BUCKET_LENGTH {
                            if (*bk).hash[z] == 0 {
                                (*bk).hash[z] = hash;
                                (*bk).index[z] = (*ob).index[j];
                                continue 'inner;
                            }
                        }
                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bk).hash[z] == 0 {
                                (*bk).hash[z] = hash;
                                (*bk).index[z] = (*ob).index[j];
                                continue 'inner;
                            }
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

// ============================================================================
// stbds_hash_string
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut s: *mut c_char, seed: size_t) -> size_t {
    let mut hash: size_t = seed;
    while *s != 0 {
        hash = hash.rotate_left(9).wrapping_add(*s as u8 as size_t);
        s = s.add(1);
    }
    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ hash.rotate_right(11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= hash.rotate_right(22);
    hash.wrapping_add(seed)
}

// ============================================================================
// stbds_siphash_bytes
// ============================================================================

#[inline(always)]
fn siphash_round(v0: &mut size_t, v1: &mut size_t, v2: &mut size_t, v3: &mut size_t) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    let mut d = p as *mut u8;
    let mut v0: size_t = 0x736f6d6570736575_usize ^ seed;
    let mut v1: size_t = 0x646f72616e646f6d_usize ^ !seed;
    let mut v2: size_t = 0x6c7967656e657261_usize ^ seed;
    let mut v3: size_t = 0x7465646279746573_usize ^ !seed;

    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;

    let sz = std::mem::size_of::<size_t>();
    let mut i: size_t = 0;
    while i + sz <= len {
        // data = d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24);
        // data |= (size_t)(d[4] | (d[5]<<8) | (d[6]<<16) | (d[7]<<24)) << 16 << 16;
        // The C does the inner OR as `int`, then casts to size_t (sign-extends),
        // then shifts << 32. For low half, the cast also sign-extends.
        let val_lo: i32 = (*d.add(0) as i32)
            | ((*d.add(1) as i32) << 8)
            | ((*d.add(2) as i32) << 16)
            | ((*d.add(3) as i32) << 24);
        let val_hi: i32 = (*d.add(4) as i32)
            | ((*d.add(5) as i32) << 8)
            | ((*d.add(6) as i32) << 16)
            | ((*d.add(7) as i32) << 24);
        let mut data: size_t = val_lo as i64 as size_t;
        data |= ((val_hi as i64 as size_t) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += sz;
        d = d.add(sz);
    }

    let mut data: size_t = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    if rem >= 7 {
        data |= ((*d.add(6) as size_t) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.add(5) as size_t) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.add(4) as size_t) << 16) << 16;
    }
    if rem >= 4 {
        data |= ((*d.add(3) as i32) << 24) as i64 as size_t;
    }
    if rem >= 3 {
        data |= ((*d.add(2) as i32) << 16) as i64 as size_t;
    }
    if rem >= 2 {
        data |= ((*d.add(1) as i32) << 8) as i64 as size_t;
    }
    if rem >= 1 {
        data |= *d.add(0) as i64 as size_t;
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

// ============================================================================
// stbds_hash_bytes
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    stbds_siphash_bytes(p, len, seed)
}

// ============================================================================
// stbds_is_key_equal
// ============================================================================

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
    i: size_t,
) -> c_int {
    let elem_ptr = (a as *mut u8).add(elemsize.wrapping_mul(i).wrapping_add(keyoffset));
    if mode >= STBDS_HM_STRING {
        let str_ptr = *(elem_ptr as *mut *mut c_char);
        if strcmp(key as *const c_char, str_ptr) == 0 {
            1
        } else {
            0
        }
    } else if memcmp(key, elem_ptr as *const c_void, keysize) == 0 {
        1
    } else {
        0
    }
}

// ============================================================================
// stbds_hmfree_func
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: size_t) {
    if a.is_null() {
        return;
    }
    let table = (*header(a)).hash_table as *mut StbdsHashIndex;
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let length = (*header(a)).length;
            for i in 1..length {
                let str_ptr = *((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_void);
                free(str_ptr);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut c_void);
}

// ============================================================================
// stbds_hm_find_slot
// ============================================================================

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
) -> ptrdiff_t {
    let raw_a = (a as *mut u8).sub(elemsize) as *mut c_void;
    let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
    let mut hash: size_t = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: size_t = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as size_t,
                ) != 0
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
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
                    (*bucket).index[i] as size_t,
                ) != 0
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
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

// ============================================================================
// stbds_hmget_key_ts
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    temp: *mut ptrdiff_t,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: size_t = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf(std::ptr::null_mut(), elemsize, 0, 1);
        (*header(new_a)).length += 1;
        memset(new_a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        (new_a as *mut u8).add(elemsize) as *mut c_void
    } else {
        let raw_a = (a as *mut u8).sub(elemsize) as *mut c_void;
        let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table).storage.add((slot as size_t) >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[(slot as size_t) & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

// ============================================================================
// stbds_hmget_key
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    mode: c_int,
) -> *mut c_void {
    let mut temp: ptrdiff_t = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    let arr = (p as *mut u8).sub(elemsize) as *mut c_void;
    (*header(arr)).temp = temp;
    p
}

// ============================================================================
// stbds_hmput_default
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: size_t) -> *mut c_void {
    let len_zero = if a.is_null() {
        true
    } else {
        let arr = (a as *mut u8).sub(elemsize) as *mut c_void;
        (*header(arr)).length == 0
    };
    if a.is_null() || len_zero {
        let raw_in = if a.is_null() {
            std::ptr::null_mut()
        } else {
            (a as *mut u8).sub(elemsize) as *mut c_void
        };
        let new_a = stbds_arrgrowf(raw_in, elemsize, 0, 1);
        (*header(new_a)).length += 1;
        memset(new_a, 0, elemsize);
        return (new_a as *mut u8).add(elemsize) as *mut c_void;
    }
    a
}

// ============================================================================
// stbds_strdup
// ============================================================================

unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    let len = strlen(s) + 1;
    let p = realloc(std::ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

// ============================================================================
// stbds_hmput_key
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: size_t = 0;

    if a.is_null() {
        a = stbds_arrgrowf(std::ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length += 1;
        a = (a as *mut u8).add(elemsize) as *mut c_void;
    }

    let mut raw_a = a;
    let mut a_arr = (a as *mut u8).sub(elemsize) as *mut c_void;

    let mut table = (*header(a_arr)).hash_table as *mut StbdsHashIndex;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count: size_t = if table.is_null() {
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
        (*header(a_arr)).hash_table = nt as *mut c_void;
        table = nt;
    }

    let mut hash: size_t = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: size_t = STBDS_BUCKET_LENGTH;
    let mut tombstone: ptrdiff_t = -1;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    let final_pos: size_t;
    'outer: loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as size_t,
                ) != 0
                {
                    (*header(a_arr)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        let key_at = *((raw_a as *mut u8).add(
                            elemsize
                                .wrapping_mul((*bucket).index[i] as size_t)
                                .wrapping_add(keyoffset),
                        ) as *mut *mut c_char);
                        (*table).temp_key = key_at;
                    }
                    return (a_arr as *mut u8).add(elemsize) as *mut c_void;
                }
            } else if (*bucket).hash[i] == 0 {
                final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'outer;
            } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
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
                    (*bucket).index[i] as size_t,
                ) != 0
                {
                    (*header(a_arr)).temp = (*bucket).index[i];
                    return (a_arr as *mut u8).add(elemsize) as *mut c_void;
                }
            } else if (*bucket).hash[i] == 0 {
                final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'outer;
            } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
            }
        }
        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    let mut pos = final_pos;
    if tombstone >= 0 {
        pos = tombstone as size_t;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = arrlen(a_arr);
    if (i as size_t) + 1 > arrcap(a_arr) {
        a_arr = stbds_arrgrowf(a_arr, elemsize, 1, 0);
    }
    raw_a = (a_arr as *mut u8).add(elemsize) as *mut c_void;

    (*header(a_arr)).length = (i + 1) as size_t;
    let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
    (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*header(a_arr)).temp = i - 1;

    let dst_ptr = (a_arr as *mut u8).add(elemsize.wrapping_mul(i as size_t));

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key as *mut c_char);
            *(dst_ptr as *mut *mut c_char) = dup;
            (*table).temp_key = dup;
        }
        STBDS_SH_ARENA => {
            let alloc = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *(dst_ptr as *mut *mut c_char) = alloc;
            (*table).temp_key = alloc;
        }
        STBDS_SH_DEFAULT => {
            *(dst_ptr as *mut *mut c_char) = key as *mut c_char;
            (*table).temp_key = key as *mut c_char;
        }
        _ => {
            memcpy(dst_ptr as *mut c_void, key as *const c_void, keysize);
        }
    }

    raw_a
}

// ============================================================================
// stbds_shmode_func
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: size_t, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(std::ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, std::ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    (a as *mut u8).add(elemsize) as *mut c_void
}

// ============================================================================
// stbds_hmdel_key
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    let raw_a = (a as *mut u8).sub(elemsize) as *mut c_void;
    let table = (*header(raw_a)).hash_table as *mut StbdsHashIndex;
    (*header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table).storage.add((slot as size_t) >> STBDS_BUCKET_SHIFT);
    let mut i = (slot as size_t) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index: ptrdiff_t = arrlen(raw_a) - 1 - 1;

    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *((a as *mut u8).add(elemsize.wrapping_mul(old_index as size_t))
            as *mut *mut c_void);
        free(p);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize.wrapping_mul(old_index as size_t)) as *mut c_void,
            (a as *mut u8).add(elemsize.wrapping_mul(final_index as size_t)) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let key_str = *((a as *mut u8).add(
                elemsize
                    .wrapping_mul(old_index as size_t)
                    .wrapping_add(keyoffset),
            ) as *mut *mut c_char);
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                key_str as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        } else {
            let key_ptr = (a as *mut u8).add(
                elemsize
                    .wrapping_mul(old_index as size_t)
                    .wrapping_add(keyoffset),
            ) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, key_ptr, keysize, keyoffset, mode);
        }

        b = (*table).storage.add((slot as size_t) >> STBDS_BUCKET_SHIFT);
        i = (slot as size_t) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        let new_table = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_a)).hash_table = new_table as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = stbds_make_hash_index((*table).slot_count, table);
        (*header(raw_a)).hash_table = new_table as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// ============================================================================
// stbds_stralloc
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
    s: *mut c_char,
) -> *mut c_char {
    let len = strlen(s) + 1;
    if len > (*a).remaining {
        let blocksize_in = (*a).block as size_t;
        let blocksize: size_t = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_in >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let alloc_size = std::mem::size_of::<StbdsStringBlock>() - 8 + len;
            let sb = realloc(std::ptr::null_mut(), alloc_size) as *mut StbdsStringBlock;
            memmove(
                (*sb).storage.as_mut_ptr() as *mut c_void,
                s as *const c_void,
                len,
            );
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = std::ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let alloc_size = std::mem::size_of::<StbdsStringBlock>() - 8 + blocksize;
            let sb = realloc(std::ptr::null_mut(), alloc_size) as *mut StbdsStringBlock;
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

// ============================================================================
// stbds_strreset
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, std::mem::size_of::<StbdsStringArena>());
}

// ============================================================================
// strkey
// ============================================================================

static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let fmt = b"test_%d\0".as_ptr() as *const c_char;
    sprintf(STRKEY_BUFFER.as_mut_ptr(), fmt, n);
    STRKEY_BUFFER.as_mut_ptr()
}

// ============================================================================
// hm_geti  (test driver translated from C)
// Element layout: struct { int key; int value; }  -> 8 bytes, key at 0, value at 4
// ============================================================================

#[inline]
unsafe fn intmap_temp(intmap: *mut c_void, elemsize: size_t) -> ptrdiff_t {
    let arr = (intmap as *mut u8).sub(elemsize) as *mut c_void;
    (*header(arr)).temp
}

#[inline]
unsafe fn intmap_value(intmap: *mut c_void, elemsize: size_t, idx: ptrdiff_t) -> *mut c_int {
    (intmap as *mut u8).offset(idx as isize * elemsize as isize + 4) as *mut c_int
}

#[inline]
unsafe fn intmap_key(intmap: *mut c_void, elemsize: size_t, idx: ptrdiff_t) -> *mut c_int {
    (intmap as *mut u8).offset(idx as isize * elemsize as isize) as *mut c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    const ELEMSIZE: size_t = 8; // sizeof(struct { int key; int value; })
    const KEYSIZE: size_t = 4; // sizeof(int)

    let mut intmap: *mut c_void = std::ptr::null_mut();
    let mut temp: ptrdiff_t = 0;
    let mut i: c_int;

    // i = 1; assert(hmgeti(intmap, i) == -1);
    i = 1;
    let mut k = i;
    intmap = stbds_hmget_key(
        intmap,
        ELEMSIZE,
        &mut k as *mut c_int as *mut c_void,
        KEYSIZE,
        STBDS_HM_BINARY,
    );
    assert!(intmap_temp(intmap, ELEMSIZE) == -1);

    // hmdefault(intmap, -2);
    intmap = stbds_hmput_default(intmap, ELEMSIZE);
    *intmap_value(intmap, ELEMSIZE, -1) = -2;

    // assert(hmgeti(intmap, i) == -1);
    let mut k = i;
    intmap = stbds_hmget_key(
        intmap,
        ELEMSIZE,
        &mut k as *mut c_int as *mut c_void,
        KEYSIZE,
        STBDS_HM_BINARY,
    );
    assert!(intmap_temp(intmap, ELEMSIZE) == -1);

    // assert(hmget(intmap, i) == -2);
    let mut k = i;
    intmap = stbds_hmget_key(
        intmap,
        ELEMSIZE,
        &mut k as *mut c_int as *mut c_void,
        KEYSIZE,
        STBDS_HM_BINARY,
    );
    let t = intmap_temp(intmap, ELEMSIZE);
    assert!(*intmap_value(intmap, ELEMSIZE, t) == -2);

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*5);
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmput_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        *intmap_key(intmap, ELEMSIZE, t) = i;
        *intmap_value(intmap, ELEMSIZE, t) = i.wrapping_mul(5);
        i += 2;
    }

    // for (i=0; i < num; i+=1) {
    //     hmget and hmget_ts checks
    // }
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmget_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        let val = *intmap_value(intmap, ELEMSIZE, t);
        if i & 1 != 0 {
            assert!(val == -2);
        } else {
            assert!(val == i.wrapping_mul(5));
        }

        // hmget_ts: same but uses the local temp variable
        let mut k = i;
        intmap = stbds_hmget_key_ts(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            &mut temp,
            STBDS_HM_BINARY,
        );
        let val = *intmap_value(intmap, ELEMSIZE, temp);
        if i & 1 != 0 {
            assert!(val == -2);
        } else {
            assert!(val == i.wrapping_mul(5));
        }
        i += 1;
    }

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*3);
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmput_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        *intmap_key(intmap, ELEMSIZE, t) = i;
        *intmap_value(intmap, ELEMSIZE, t) = i.wrapping_mul(3);
        i += 2;
    }

    // for (i=0; i < num; i+=1) check
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmget_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        let val = *intmap_value(intmap, ELEMSIZE, t);
        if i & 1 != 0 {
            assert!(val == -2);
        } else {
            assert!(val == i.wrapping_mul(3));
        }
        i += 1;
    }

    // for (i=2; i < num; i+=4) hmdel(intmap, i);
    i = 2;
    while i < num {
        let mut k = i;
        intmap = stbds_hmdel_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            0,
            STBDS_HM_BINARY,
        );
        i += 4;
    }

    // for (i=0; i < num; i+=1) check
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmget_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        let val = *intmap_value(intmap, ELEMSIZE, t);
        if i & 3 != 0 {
            assert!(val == -2);
        } else {
            assert!(val == i.wrapping_mul(3));
        }
        i += 1;
    }

    // for (i=0; i < num; i+=1) hmdel(intmap, i);
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmdel_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            0,
            STBDS_HM_BINARY,
        );
        i += 1;
    }

    // for (i=0; i < num; i+=1) assert(hmget(intmap, i) == -2);
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmget_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        let val = *intmap_value(intmap, ELEMSIZE, t);
        assert!(val == -2);
        i += 1;
    }

    // hmfree(intmap);
    if !intmap.is_null() {
        let arr = (intmap as *mut u8).sub(ELEMSIZE) as *mut c_void;
        stbds_hmfree_func(arr, ELEMSIZE);
    }
    intmap = std::ptr::null_mut();

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*3);
    i = 0;
    while i < num {
        let mut k = i;
        intmap = stbds_hmput_key(
            intmap,
            ELEMSIZE,
            &mut k as *mut c_int as *mut c_void,
            KEYSIZE,
            STBDS_HM_BINARY,
        );
        let t = intmap_temp(intmap, ELEMSIZE);
        *intmap_key(intmap, ELEMSIZE, t) = i;
        *intmap_value(intmap, ELEMSIZE, t) = i.wrapping_mul(3);
        i += 2;
    }

    // hmfree(intmap);
    if !intmap.is_null() {
        let arr = (intmap as *mut u8).sub(ELEMSIZE) as *mut c_void;
        stbds_hmfree_func(arr, ELEMSIZE);
    }
    let _ = intmap;
}
