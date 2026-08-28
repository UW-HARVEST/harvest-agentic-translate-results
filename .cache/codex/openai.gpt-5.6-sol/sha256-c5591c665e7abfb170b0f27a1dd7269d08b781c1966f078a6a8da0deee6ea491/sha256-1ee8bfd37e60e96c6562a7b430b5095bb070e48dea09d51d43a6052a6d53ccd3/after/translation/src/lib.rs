#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> c_int;
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
}

const BUCKET_LENGTH: usize = 8;
const BUCKET_SHIFT: usize = 3;
const BUCKET_MASK: usize = BUCKET_LENGTH - 1;
const CACHE_LINE_SIZE: usize = 64;

const INDEX_EMPTY: isize = -1;
const INDEX_DELETED: isize = -2;
const HASH_EMPTY: usize = 0;
const HASH_DELETED: usize = 1;

const HM_STRING: c_int = 1;
const SH_DEFAULT: u8 = 1;
const SH_STRDUP: u8 = 2;
const SH_ARENA: u8 = 3;

#[repr(C)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct HashBucket {
    hash: [usize; BUCKET_LENGTH],
    index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
struct HashIndex {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StringArena,
    storage: *mut HashBucket,
}

static mut STBDS_HASH_SEED: usize = 0x3141_5926;
static mut BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(a: *mut c_void) -> *mut ArrayHeader {
    (a as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader
}

#[inline]
unsafe fn arr_len(a: *mut c_void) -> usize {
    if a.is_null() { 0 } else { (*header(a)).length }
}

#[inline]
unsafe fn arr_cap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut HashIndex {
    (*header(a)).hash_table as *mut HashIndex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = arr_len(a).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= arr_cap(a) {
        return a;
    }
    if min_cap < 2usize.wrapping_mul(arr_cap(a)) {
        min_cap = 2usize.wrapping_mul(arr_cap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };
    let bytes = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<ArrayHeader>());
    let allocation = realloc(old, bytes);
    let b = (allocation as *mut u8).add(size_of::<ArrayHeader>()) as *mut c_void;

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
    free(header(a) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
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

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let bucket_bytes = (slot_count >> BUCKET_SHIFT).wrapping_mul(size_of::<HashBucket>());
    let bytes = bucket_bytes
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(ptr::null_mut(), bytes) as *mut HashIndex;
    let storage_address = ((table.add(1) as usize) + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);

    (*table).storage = storage_address as *mut HashBucket;
    (*table).slot_count = slot_count;
    (*table).slot_count_log2 = log2(slot_count);
    (*table).tombstone_count = 0;
    (*table).used_count = 0;
    (*table).used_count_threshold = slot_count - (slot_count >> 2);
    (*table).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*table).used_count_shrink_threshold = slot_count >> 2;
    if slot_count <= BUCKET_LENGTH {
        (*table).used_count_shrink_threshold = 0;
    }

    if !old.is_null() {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
    } else {
        ptr::write_bytes(
            ptr::addr_of_mut!((*table).string) as *mut u8,
            0,
            size_of::<StringArena>(),
        );
        (*table).seed = STBDS_HASH_SEED;
        let a = 0x27bb_2ee6_87b0_b0fdusize;
        let b = 0x0000_0000_b504_f32dusize;
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    for bucket_number in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = (*table).storage.add(bucket_number);
        for i in 0..BUCKET_LENGTH {
            (*bucket).hash[i] = HASH_EMPTY;
        }
        for i in 0..BUCKET_LENGTH {
            (*bucket).index[i] = INDEX_EMPTY;
        }
    }

    if !old.is_null() {
        (*table).used_count = (*old).used_count;
        for bucket_number in 0..((*old).slot_count >> BUCKET_SHIFT) {
            let old_bucket = (*old).storage.add(bucket_number);
            for i in 0..BUCKET_LENGTH {
                if (*old_bucket).index[i] >= 0 {
                    let hash = (*old_bucket).hash[i];
                    let mut pos = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
                        for z in (pos & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[z] == HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*old_bucket).index[i];
                                break 'probe;
                            }
                        }
                        for z in 0..(pos & BUCKET_MASK) {
                            if (*bucket).hash[z] == HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*old_bucket).index[i];
                                break 'probe;
                            }
                        }
                        pos = pos.wrapping_add(step);
                        step = step.wrapping_add(BUCKET_LENGTH);
                        pos &= (*table).slot_count - 1;
                    }
                }
            }
        }
    }
    table
}

#[inline]
fn rotate_left(value: usize, amount: u32) -> usize {
    value.rotate_left(amount)
}

#[inline]
fn rotate_right(value: usize, amount: u32) -> usize {
    value.rotate_right(amount)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while *string != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*(string as *const u8) as usize);
        string = string.add(1);
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

#[inline]
fn sip_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, usize::BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, usize::BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn siphash_bytes(data: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = data as *const u8;
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut i = 0;
    while i + size_of::<usize>() <= len {
        let low = (*d as u32)
            | ((*d.add(1) as u32) << 8)
            | ((*d.add(2) as u32) << 16)
            | ((*d.add(3) as u32) << 24);
        let high = (*d.add(4) as u32)
            | ((*d.add(5) as u32) << 8)
            | ((*d.add(6) as u32) << 16)
            | ((*d.add(7) as u32) << 24);
        let mut block = (low as i32 as isize) as usize;
        block |= ((high as i32 as isize) as usize) << 32;
        v3 ^= block;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= block;
        i += size_of::<usize>();
        d = d.add(size_of::<usize>());
    }

    let mut block = len << (usize::BITS - 8);
    match len - i {
        7 => {
            block |= (*d.add(6) as usize) << 48;
            block |= (*d.add(5) as usize) << 40;
            block |= (*d.add(4) as usize) << 32;
            block |= (((*d.add(3) as u32) << 24) as i32 as isize) as usize;
            block |= (*d.add(2) as usize) << 16;
            block |= (*d.add(1) as usize) << 8;
            block |= *d as usize;
        }
        6 => {
            block |= (*d.add(5) as usize) << 40;
            block |= (*d.add(4) as usize) << 32;
            block |= (((*d.add(3) as u32) << 24) as i32 as isize) as usize;
            block |= (*d.add(2) as usize) << 16;
            block |= (*d.add(1) as usize) << 8;
            block |= *d as usize;
        }
        5 => {
            block |= (*d.add(4) as usize) << 32;
            block |= (((*d.add(3) as u32) << 24) as i32 as isize) as usize;
            block |= (*d.add(2) as usize) << 16;
            block |= (*d.add(1) as usize) << 8;
            block |= *d as usize;
        }
        4 => {
            block |= (((*d.add(3) as u32) << 24) as i32 as isize) as usize;
            block |= (*d.add(2) as usize) << 16;
            block |= (*d.add(1) as usize) << 8;
            block |= *d as usize;
        }
        3 => {
            block |= (*d.add(2) as usize) << 16;
            block |= (*d.add(1) as usize) << 8;
            block |= *d as usize;
        }
        2 => {
            block |= (*d.add(1) as usize) << 8;
            block |= *d as usize;
        }
        1 => block |= *d as usize,
        0 => {}
        _ => unreachable!(),
    }

    v3 ^= block;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= block;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(data: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(data, len, seed)
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let item_key = (a as *mut u8)
        .add(elemsize.wrapping_mul(index))
        .add(keyoffset);
    if mode >= HM_STRING {
        strcmp(key as *const c_char, *(item_key as *mut *mut c_char)) == 0
    } else {
        memcmp(key, item_key as *const c_void, keysize) == 0
    }
}

unsafe fn hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table(raw_a);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut pos = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
        for i in (pos & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((pos & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return -1;
            }
        }

        for i in 0..(pos & BUCKET_MASK) {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((pos & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return -1;
            }
        }

        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for i in 1..(*header(a)).length {
                let key = *((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_void);
                free(key);
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*table).string));
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut c_void);
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
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut HashIndex;
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(a, elemsize, key, keysize, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
                *temp = (*bucket).index[(slot as usize) & BUCKET_MASK];
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
    let mut temp = 0;
    let result = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*header(hash_to_arr(result, elemsize))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*header(a)).length += 1;
        memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let result = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(result as *mut c_void, string as *const c_void, len);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);
    let mut table = (*header(a)).hash_table as *mut HashIndex;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let new_table = make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*new_table).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 };
        }
        (*header(a)).hash_table = new_table as *mut c_void;
        table = new_table;
    }

    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut pos = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    let mut tombstone = -1isize;

    'search: loop {
        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
        for i in (pos & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    0,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    (*header(a)).temp = (*bucket).index[i];
                    if mode >= HM_STRING {
                        (*table).temp_key = *((raw_a as *mut u8)
                            .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                            as *mut *mut c_char);
                    }
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
        }

        for i in 0..(pos & BUCKET_MASK) {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    0,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    (*header(a)).temp = (*bucket).index[i];
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
        }

        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= (*table).slot_count - 1;
    }

    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let index = arr_len(a) as isize;
    if (index as usize).wrapping_add(1) > arr_cap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(a, elemsize);
    (*header(a)).length = (index + 1) as usize;
    let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
    (*bucket).hash[pos & BUCKET_MASK] = hash;
    (*bucket).index[pos & BUCKET_MASK] = index - 1;
    (*header(a)).temp = index - 1;

    let destination = (a as *mut u8).add(elemsize.wrapping_mul(index as usize));
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = stbds_strdup(key as *mut c_char);
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key as *mut c_char);
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        SH_DEFAULT => {
            *(destination as *mut *mut c_char) = key as *mut c_char;
            (*table).temp_key = key as *mut c_char;
        }
        _ => {
            memcpy(destination as *mut c_void, key as *const c_void, keysize);
        }
    }
    raw_a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = table as *mut c_void;
    (*table).string.mode = mode as u8;
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
    let mut table = (*header(raw_a)).hash_table as *mut HashIndex;
    (*header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
    let mut bucket_index = (slot as usize) & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = arr_len(raw_a) as isize - 2;

    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored =
            *((a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)) as *mut *mut c_void);
        free(stored);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
            (a as *mut u8).add(elemsize.wrapping_mul(final_index as usize)) as *const c_void,
            elemsize,
        );

        let moved_item = (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize));
        let moved_key = if mode == HM_STRING {
            *((moved_item.add(keyoffset)) as *mut *mut c_void)
        } else {
            moved_item.add(keyoffset) as *mut c_void
        };
        slot = hm_find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
        bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
        bucket_index = (slot as usize) & BUCKET_MASK;
        (*bucket).index[bucket_index] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let replacement = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_a)).hash_table = replacement as *mut c_void;
        free(table as *mut c_void);
        table = replacement;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let replacement = make_hash_index((*table).slot_count, table);
        (*header(raw_a)).hash_table = replacement as *mut c_void;
        free(table as *mut c_void);
        table = replacement;
    }
    let _ = table;
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    const BLOCKSIZE_MIN: usize = 512;
    const BLOCKSIZE_MAX: usize = 1 << 20;

    let len = strlen(string) + 1;
    if len > (*arena).remaining {
        let mut blocksize = (*arena).block as usize;
        blocksize = BLOCKSIZE_MIN << (blocksize >> 1);
        if blocksize < BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > blocksize {
            let allocation_size = size_of::<StringBlock>() - 8 + len;
            let block = realloc(ptr::null_mut(), allocation_size) as *mut StringBlock;
            memmove(
                ptr::addr_of_mut!((*block).storage) as *mut c_void,
                string as *const c_void,
                len,
            );
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = ptr::null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return ptr::addr_of_mut!((*block).storage) as *mut c_char;
        }

        let allocation_size = size_of::<StringBlock>() - 8 + blocksize;
        let block = realloc(ptr::null_mut(), allocation_size) as *mut StringBlock;
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = blocksize;
    }

    let destination = (ptr::addr_of_mut!((*(*arena).storage).storage) as *mut c_char)
        .add((*arena).remaining - len);
    (*arena).remaining -= len;
    memmove(destination as *mut c_void, string as *const c_void, len);
    destination
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut current = (*arena).storage;
    while !current.is_null() {
        let next = (*current).next;
        free(current as *mut c_void);
        current = next;
    }
    memset(arena as *mut c_void, 0, size_of::<StringArena>());
}

#[repr(C)]
struct StringMapEntry {
    key: *mut c_char,
    value: c_int,
}

static A_KEY: [u8; 2] = *b"a\0";
static DECIMAL_FORMAT: [u8; 8] = *b"test_%d\0";
static OUTPUT_FORMAT: [u8; 7] = *b"%s %d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let destination = ptr::addr_of_mut!(BUFFER) as *mut c_char;
    sprintf(
        destination,
        DECIMAL_FORMAT.as_ptr() as *const c_char,
        number,
    );
    destination
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let mut string_map: *mut StringMapEntry = ptr::null_mut();
    let mut arena = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i = 0;
    while i < num {
        let key = strkey(i);
        stbds_stralloc(&mut arena, key);
        i += 1;
    }
    stbds_strreset(&mut arena);

    let key = A_KEY.as_ptr() as *mut c_char;
    string_map = stbds_hmput_key(
        string_map as *mut c_void,
        size_of::<StringMapEntry>(),
        key as *mut c_void,
        size_of::<*mut c_char>(),
        HM_STRING,
    ) as *mut StringMapEntry;

    let raw_map = hash_to_arr(string_map as *mut c_void, size_of::<StringMapEntry>());
    let index = (*header(raw_map)).temp as usize;
    let entry = string_map.add(index);
    (*entry).key = key;
    (*entry).value = num;
    (*entry).key = (*(hash_table(raw_map))).temp_key;

    let length = (*header(raw_map)).length - 1;
    for item_number in 0..length {
        let item = string_map.add(item_number);
        printf(
            OUTPUT_FORMAT.as_ptr() as *const c_char,
            (*item).key,
            (*item).value,
        );
    }

    stbds_hmfree_func(raw_map, size_of::<StringMapEntry>());
}
