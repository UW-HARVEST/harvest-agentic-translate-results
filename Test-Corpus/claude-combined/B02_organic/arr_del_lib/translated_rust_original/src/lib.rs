#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_assignments, unused_parens, dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = 7;
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

#[repr(C)]
#[derive(Clone, Copy)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
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

#[inline]
unsafe fn header_ptr(t: *mut c_void) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).offset(-1)
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header_ptr(a)).length
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header_ptr(a)).capacity
    }
}

#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).offset(-(elemsize as isize)) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

static mut STBDS_HASH_SEED: usize = 0x31415926;

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
    let cur_arrlen = arrlen(a);
    let cur_arrcap = arrcap(a);
    let min_len = cur_arrlen.wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= cur_arrcap {
        return a;
    }

    if min_cap < 2usize.wrapping_mul(cur_arrcap) {
        min_cap = 2usize.wrapping_mul(cur_arrcap);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let prev = if a.is_null() {
        ptr::null_mut()
    } else {
        header_ptr(a) as *mut c_void
    };
    let alloc_size = elemsize.wrapping_mul(min_cap) + std::mem::size_of::<StbdsArrayHeader>();
    let raw = libc::realloc(prev, alloc_size) as *mut u8;
    let b = raw.add(std::mem::size_of::<StbdsArrayHeader>()) as *mut c_void;
    if a.is_null() {
        let h = header_ptr(b);
        (*h).length = 0;
        (*h).hash_table = ptr::null_mut();
        (*h).temp = 0;
    }
    (*header_ptr(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    libc::free(header_ptr(a) as *mut c_void);
}

#[inline]
fn stbds_log2_(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        * std::mem::size_of::<StbdsHashBucket>()
        + std::mem::size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = libc::realloc(ptr::null_mut(), alloc_size) as *mut StbdsHashIndex;

    let after_t = t.add(1) as usize;
    let aligned = (after_t + STBDS_CACHE_LINE_SIZE - 1) & !(STBDS_CACHE_LINE_SIZE - 1);
    (*t).storage = aligned as *mut StbdsHashBucket;

    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2_(slot_count);
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
        ptr::write_bytes(
            &mut (*t).string as *mut StbdsStringArena as *mut u8,
            0,
            std::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = STBDS_HASH_SEED;
        // a, b on 64-bit:
        // a = 0x27bb2ee687b0b0fd, b = 0xb504f32d
        let a: usize = 0x27bb2ee687b0b0fd;
        let b: usize = 0xb504f32d;
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    let n_buckets = slot_count >> STBDS_BUCKET_SHIFT;
    for i in 0..n_buckets {
        let b_ptr = (*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            (*b_ptr).hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            (*b_ptr).index[j] = STBDS_INDEX_EMPTY;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let ot_n = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..ot_n {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if (*ob).index[j] >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let pos_low = pos & STBDS_BUCKET_MASK;
                        for z in pos_low..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                        }
                        for z in 0..pos_low {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str_ as *mut u8;
    while *s != 0 {
        hash = hash.rotate_left(9).wrapping_add(*s as usize);
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

#[inline(always)]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;

    let mut v0: usize = 0x736f6d6570736575usize ^ seed;
    let mut v1: usize = 0x646f72616e646f6dusize ^ !seed;
    let mut v2: usize = 0x6c7967656e657261usize ^ seed;
    let mut v3: usize = 0x7465646279746573usize ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let sz = std::mem::size_of::<usize>();
    let mut i: usize = 0;
    let mut data: usize;

    while i + sz <= len {
        let b0 = *d.add(0) as i32;
        let b1 = *d.add(1) as i32;
        let b2 = *d.add(2) as i32;
        let b3 = *d.add(3) as i32;
        // Reproduce signed-extension behavior of C's shift-into-sign-bit
        let lo: usize = (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) as usize;
        let b4 = *d.add(4) as i32;
        let b5 = *d.add(5) as i32;
        let b6 = *d.add(6) as i32;
        let b7 = *d.add(7) as i32;
        let hi: usize = (b4 | (b5 << 8) | (b6 << 16) | (b7 << 24)) as usize;
        data = lo | (hi << 32);

        v3 ^= data;
        for _ in 0..2 {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += sz;
        d = d.add(sz);
    }

    data = len << (64 - 8);

    let rem = len - i;
    if rem >= 7 {
        data |= ((*d.add(6)) as usize) << 48;
    }
    if rem >= 6 {
        data |= ((*d.add(5)) as usize) << 40;
    }
    if rem >= 5 {
        data |= ((*d.add(4)) as usize) << 32;
    }
    if rem >= 4 {
        let b3 = *d.add(3) as i32;
        data |= (b3 << 24) as usize;
    }
    if rem >= 3 {
        let b2 = *d.add(2) as i32;
        data |= (b2 << 16) as usize;
    }
    if rem >= 2 {
        let b1 = *d.add(1) as i32;
        data |= (b1 << 8) as usize;
    }
    if rem >= 1 {
        data |= (*d.add(0)) as usize;
    }

    v3 ^= data;
    for _ in 0..2 {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
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
    i: isize,
) -> bool {
    let elem_addr = (a as *mut u8).offset((elemsize as isize) * i + (keyoffset as isize));
    if mode >= STBDS_HM_STRING {
        let stored_key = *(elem_addr as *mut *mut c_char);
        libc::strcmp(key as *const c_char, stored_key) == 0
    } else {
        libc::memcmp(key, elem_addr as *const c_void, keysize) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let h = header_ptr(a);
    let table = (*h).hash_table as *mut StbdsHashIndex;
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let length = (*h).length;
            for i in 1..length {
                let pp = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                libc::free(*pp as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    libc::free((*h).hash_table);
    libc::free(h as *mut c_void);
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
    let table = (*header_ptr(raw_a)).hash_table as *mut StbdsHashIndex;
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos: usize;

    if hash < 2 {
        hash += 2;
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        let pos_low = pos & STBDS_BUCKET_MASK;

        for i in pos_low..STBDS_BUCKET_LENGTH {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*header_ptr(a)).length += 1;
        libc::memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(a, elemsize);
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header_ptr(raw_a)).hash_table as *mut StbdsHashIndex;
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
        return a;
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
    (*header_ptr(hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut a: *mut c_void,
    elemsize: usize,
) -> *mut c_void {
    let needs_alloc = if a.is_null() {
        true
    } else {
        (*header_ptr(hash_to_arr(a, elemsize))).length == 0
    };
    if needs_alloc {
        let prev = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(prev, elemsize, 0, 1);
        (*header_ptr(a)).length += 1;
        libc::memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = libc::strlen(str_) + 1;
    let p = libc::realloc(ptr::null_mut(), len) as *mut c_char;
    libc::memmove(p as *mut c_void, str_ as *const c_void, len);
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
    let keyoffset = 0usize;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        libc::memset(a, 0, elemsize);
        (*header_ptr(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*header_ptr(a)).hash_table as *mut StbdsHashIndex;

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
        (*header_ptr(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos: usize;
    let mut tombstone: isize = -1;
    let mut bucket: *mut StbdsHashBucket;

    if hash < 2 {
        hash += 2;
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    'find: loop {
        bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let pos_low = pos & STBDS_BUCKET_MASK;
        for i in pos_low..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    (*header_ptr(a)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        let kp = (raw_a as *mut u8)
                            .offset((elemsize as isize) * (*bucket).index[i] + (keyoffset as isize))
                            as *mut *mut c_char;
                        (*table).temp_key = *kp;
                    }
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'find;
            } else if tombstone < 0 {
                if (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    (*header_ptr(a)).temp = (*bucket).index[i];
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'find;
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

    let i = arrlen(a) as isize;
    if (i as usize) + 1 > arrcap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(a, elemsize);

    (*header_ptr(a)).length = (i as usize) + 1;
    bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
    (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*header_ptr(a)).temp = i - 1;

    let elem_addr = (a as *mut u8).offset((elemsize as isize) * i);
    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let new_key = stbds_strdup(key as *mut c_char);
            *(elem_addr as *mut *mut c_char) = new_key;
            (*table).temp_key = new_key;
        }
        STBDS_SH_ARENA => {
            let new_key = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *(elem_addr as *mut *mut c_char) = new_key;
            (*table).temp_key = new_key;
        }
        STBDS_SH_DEFAULT => {
            *(elem_addr as *mut *mut c_char) = key as *mut c_char;
            (*table).temp_key = key as *mut c_char;
        }
        _ => {
            libc::memcpy(elem_addr as *mut c_void, key as *const c_void, keysize);
        }
    }

    let _ = raw_a;
    arr_to_hash(a, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    libc::memset(a, 0, elemsize);
    (*header_ptr(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header_ptr(a)).hash_table = h as *mut c_void;
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
    let table = (*header_ptr(raw_a)).hash_table as *mut StbdsHashIndex;
    (*header_ptr(raw_a)).temp = 0;
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
    let final_index = (arrlen(raw_a) as isize) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header_ptr(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let kp = (a as *mut u8).offset((elemsize as isize) * old_index) as *mut *mut c_char;
        libc::free(*kp as *mut c_void);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).offset((elemsize as isize) * old_index) as *mut c_void,
            (a as *mut u8).offset((elemsize as isize) * final_index) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let kp = (a as *mut u8)
                .offset((elemsize as isize) * old_index + (keyoffset as isize))
                as *mut *mut c_char;
            slot = stbds_hm_find_slot(a, elemsize, *kp as *mut c_void, keysize, keyoffset, mode);
        } else {
            let kp = (a as *mut u8).offset((elemsize as isize) * old_index + (keyoffset as isize))
                as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
        }
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*header_ptr(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*header_ptr(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        libc::free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header_ptr(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        libc::free(table as *mut c_void);
    }

    let _ = table;
    a
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
    str_: *mut c_char,
) -> *mut c_char {
    let len = libc::strlen(str_) + 1;
    if len > (*a).remaining {
        let blocksize_init = (*a).block as usize;
        let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_init >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // sizeof(stbds_string_block) - 8 + len
            let alloc_len = std::mem::size_of::<StbdsStringBlock>() - 8 + len;
            let sb = libc::realloc(ptr::null_mut(), alloc_len) as *mut StbdsStringBlock;
            libc::memmove(
                (*sb).storage.as_mut_ptr() as *mut c_void,
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
            return (*sb).storage.as_mut_ptr() as *mut c_char;
        } else {
            let alloc_len = std::mem::size_of::<StbdsStringBlock>() - 8 + blocksize;
            let sb = libc::realloc(ptr::null_mut(), alloc_len) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = (*(*a).storage)
        .storage
        .as_mut_ptr()
        .add((*a).remaining - len);
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, str_ as *const c_void, len);
    p as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        libc::free(x as *mut c_void);
        x = y;
    }
    ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<StbdsStringArena>());
}

// Test helper: strkey buffer
static mut STRKEY_BUFFER: [u8; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let p = &raw mut STRKEY_BUFFER as *mut c_char;
    libc::sprintf(p, b"test_%d\0".as_ptr() as *const c_char, n);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let elemsize = std::mem::size_of::<c_int>();

    for i in 0..4usize {
        // Push num, 2, 3, 4
        let push = |arr_p: &mut *mut c_int, v: c_int| unsafe {
            let a_void = *arr_p as *mut c_void;
            let need_grow = if a_void.is_null() {
                true
            } else {
                (*header_ptr(a_void)).length + 1 > (*header_ptr(a_void)).capacity
            };
            if need_grow {
                *arr_p = stbds_arrgrowf(a_void, elemsize, 1, 0) as *mut c_int;
            }
            let h = header_ptr(*arr_p as *mut c_void);
            *(*arr_p).add((*h).length) = v;
            (*h).length += 1;
        };

        push(&mut arr, num);
        push(&mut arr, 2);
        push(&mut arr, 3);
        push(&mut arr, 4);

        // arrdel(arr, i): memmove(arr+i, arr+i+1, sizeof(int)*(length-1-i)); length -= 1
        {
            let h = header_ptr(arr as *mut c_void);
            let length = (*h).length;
            libc::memmove(
                arr.add(i) as *mut c_void,
                arr.add(i + 1) as *const c_void,
                elemsize * (length - 1 - i),
            );
            (*h).length -= 1;
        }

        // arrfree(arr)
        if !arr.is_null() {
            libc::free(header_ptr(arr as *mut c_void) as *mut c_void);
            arr = ptr::null_mut();
        }

        // Push num, 2, 3, 4
        push(&mut arr, num);
        push(&mut arr, 2);
        push(&mut arr, 3);
        push(&mut arr, 4);

        // arrdelswap(arr, i): a[i] = a[length-1]; length -= 1
        {
            let h = header_ptr(arr as *mut c_void);
            *arr.add(i) = *arr.add((*h).length - 1);
            (*h).length -= 1;
        }

        // arrfree(arr)
        if !arr.is_null() {
            libc::free(header_ptr(arr as *mut c_void) as *mut c_void);
            arr = ptr::null_mut();
        }
    }
}

