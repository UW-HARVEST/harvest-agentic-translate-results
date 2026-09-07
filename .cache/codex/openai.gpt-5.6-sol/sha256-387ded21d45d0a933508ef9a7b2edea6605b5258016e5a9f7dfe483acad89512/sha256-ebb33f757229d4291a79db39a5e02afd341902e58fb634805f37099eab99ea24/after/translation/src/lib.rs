#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{self, addr_of_mut, null_mut};

unsafe extern "C" {
    fn abort() -> !;
    fn free(ptr: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, len: usize) -> c_int;
    fn memmove(dest: *mut c_void, src: *const c_void, len: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: c_int, len: usize) -> *mut c_void;
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

const STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

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

#[repr(C)]
#[derive(Clone, Copy)]
struct StringMapEntry {
    key: *mut c_char,
    value: c_int,
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut BUFFER: [c_char; 256] = [0; 256];

const FOO: &[u8] = b"foo\0";
const STRKEY_FORMAT: &[u8] = b"test_%d\0";
const PRINT_FORMAT: &[u8] = b"%s %d\n\0";

#[inline]
unsafe fn array_header(a: *mut c_void) -> *mut ArrayHeader {
    a.cast::<ArrayHeader>().sub(1)
}

#[inline]
unsafe fn array_len(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*array_header(a)).length
    }
}

#[inline]
unsafe fn array_capacity(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*array_header(a)).capacity
    }
}

#[inline]
unsafe fn hash_to_array(a: *mut c_void, elem_size: usize) -> *mut c_void {
    a.cast::<u8>().sub(elem_size).cast()
}

#[inline]
unsafe fn array_to_hash(a: *mut c_void, elem_size: usize) -> *mut c_void {
    a.cast::<u8>().add(elem_size).cast()
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut HashIndex {
    (*array_header(a)).hash_table.cast()
}

#[inline]
unsafe fn c_assert(condition: bool) {
    if !condition {
        abort();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elem_size: usize,
    add_len: usize,
    mut min_capacity: usize,
) -> *mut c_void {
    let min_len = array_len(a).wrapping_add(add_len);

    if min_len > min_capacity {
        min_capacity = min_len;
    }

    if min_capacity <= array_capacity(a) {
        return a;
    }

    if min_capacity < 2usize.wrapping_mul(array_capacity(a)) {
        min_capacity = 2usize.wrapping_mul(array_capacity(a));
    } else if min_capacity < 4 {
        min_capacity = 4;
    }

    let allocation = if a.is_null() {
        null_mut()
    } else {
        array_header(a).cast()
    };
    let b = realloc(
        allocation,
        elem_size
            .wrapping_mul(min_capacity)
            .wrapping_add(size_of::<ArrayHeader>()),
    )
    .cast::<u8>()
    .add(size_of::<ArrayHeader>())
    .cast::<c_void>();

    if a.is_null() {
        (*array_header(b)).length = 0;
        (*array_header(b)).hash_table = null_mut();
        (*array_header(b)).temp = 0;
    }
    (*array_header(b)).capacity = min_capacity;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(array_header(a).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    HASH_SEED = seed;
}

#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

fn integer_log2(mut slot_count: usize) -> usize {
    let mut result = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        result += 1;
    }
    result
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(null_mut(), allocation_size).cast::<HashIndex>();
    let storage_address =
        ((table.add(1) as usize + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)) as *mut HashBucket;

    (*table).storage = storage_address;
    (*table).slot_count = slot_count;
    (*table).slot_count_log2 = integer_log2(slot_count);
    (*table).tombstone_count = 0;
    (*table).used_count = 0;
    (*table).used_count_threshold = slot_count - (slot_count >> 2);
    (*table).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*table).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= BUCKET_LENGTH {
        (*table).used_count_shrink_threshold = 0;
    }
    c_assert(
        (*table).used_count_threshold + (*table).tombstone_count_threshold < (*table).slot_count,
    );

    if !old.is_null() {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
    } else {
        memset(
            addr_of_mut!((*table).string).cast(),
            0,
            size_of::<StringArena>(),
        );
        (*table).seed = HASH_SEED;
        HASH_SEED = HASH_SEED
            .wrapping_mul(0x27bb_2ee6_87b0_b0fd)
            .wrapping_add(0x0000_0000_b504_f32d);
    }

    for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = (*table).storage.add(bucket_index);
        for slot in 0..BUCKET_LENGTH {
            (*bucket).hash[slot] = HASH_EMPTY;
        }
        for slot in 0..BUCKET_LENGTH {
            (*bucket).index[slot] = INDEX_EMPTY;
        }
    }

    if !old.is_null() {
        (*table).used_count = (*old).used_count;
        for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
            let old_bucket = (*old).storage.add(old_bucket_index);
            for old_slot in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_slot] >= 0 {
                    let hash = (*old_bucket).hash[old_slot];
                    let mut position = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;

                    'probe: loop {
                        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot];
                                break 'probe;
                            }
                        }

                        let limit = position & BUCKET_MASK;
                        for slot in 0..limit {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot];
                                break 'probe;
                            }
                        }

                        position = position.wrapping_add(step);
                        step = step.wrapping_add(BUCKET_LENGTH);
                        position &= (*table).slot_count - 1;
                    }
                }
            }
        }
    }

    table
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while *string != 0 {
        hash = hash
            .rotate_left(9)
            .wrapping_add(*string.cast::<u8>() as usize);
        string = string.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash ^= hash ^ hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ hash.rotate_right(11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= hash.rotate_right(22);
    hash.wrapping_add(seed)
}

#[inline]
fn sip_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(usize::BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(usize::BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

unsafe fn siphash_bytes(data_pointer: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_bytes = data_pointer.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0usize;
    while offset + size_of::<usize>() <= len {
        let low = (*data_bytes.add(0) as u32)
            | ((*data_bytes.add(1) as u32) << 8)
            | ((*data_bytes.add(2) as u32) << 16)
            | ((*data_bytes.add(3) as u32) << 24);
        let high = (*data_bytes.add(4) as u32)
            | ((*data_bytes.add(5) as u32) << 8)
            | ((*data_bytes.add(6) as u32) << 16)
            | ((*data_bytes.add(7) as u32) << 24);
        let word = (low as i32 as isize as usize) | ((high as usize) << 32);

        v3 ^= word;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= word;
        offset += size_of::<usize>();
        data_bytes = data_bytes.add(size_of::<usize>());
    }

    let mut tail = len << (usize::BITS - 8);
    match len - offset {
        7 => {
            tail |= (*data_bytes.add(6) as usize) << 48;
            tail |= (*data_bytes.add(5) as usize) << 40;
            tail |= (*data_bytes.add(4) as usize) << 32;
            tail |= (*data_bytes.add(3) as i32).wrapping_shl(24) as isize as usize;
            tail |= (*data_bytes.add(2) as usize) << 16;
            tail |= (*data_bytes.add(1) as usize) << 8;
            tail |= *data_bytes as usize;
        }
        6 => {
            tail |= (*data_bytes.add(5) as usize) << 40;
            tail |= (*data_bytes.add(4) as usize) << 32;
            tail |= (*data_bytes.add(3) as i32).wrapping_shl(24) as isize as usize;
            tail |= (*data_bytes.add(2) as usize) << 16;
            tail |= (*data_bytes.add(1) as usize) << 8;
            tail |= *data_bytes as usize;
        }
        5 => {
            tail |= (*data_bytes.add(4) as usize) << 32;
            tail |= (*data_bytes.add(3) as i32).wrapping_shl(24) as isize as usize;
            tail |= (*data_bytes.add(2) as usize) << 16;
            tail |= (*data_bytes.add(1) as usize) << 8;
            tail |= *data_bytes as usize;
        }
        4 => {
            tail |= (*data_bytes.add(3) as i32).wrapping_shl(24) as isize as usize;
            tail |= (*data_bytes.add(2) as usize) << 16;
            tail |= (*data_bytes.add(1) as usize) << 8;
            tail |= *data_bytes as usize;
        }
        3 => {
            tail |= (*data_bytes.add(2) as usize) << 16;
            tail |= (*data_bytes.add(1) as usize) << 8;
            tail |= *data_bytes as usize;
        }
        2 => {
            tail |= (*data_bytes.add(1) as usize) << 8;
            tail |= *data_bytes as usize;
        }
        1 => tail |= *data_bytes as usize,
        0 => {}
        _ => unreachable!(),
    }

    v3 ^= tail;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= tail;
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
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored = a
        .cast::<u8>()
        .add(elem_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *stored.cast::<*mut c_char>()) == 0
    } else {
        memcmp(key, stored.cast(), key_size) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elem_size: usize) {
    if a.is_null() {
        return;
    }

    let table = hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for index in 1..(*array_header(a)).length {
                free(*a.cast::<u8>().add(elem_size * index).cast::<*mut c_void>());
            }
        }
        stbds_strreset(addr_of_mut!((*table).string));
    }
    free((*array_header(a)).hash_table);
    free(array_header(a).cast());
}

unsafe fn hm_find_slot(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_array(a, elem_size);
    let table = hash_table(raw_a);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    let mut step = BUCKET_LENGTH;

    if hash < 2 {
        hash += 2;
    }

    let mut position = probe_position(hash, (*table).slot_count);
    loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                if is_key_equal(
                    a,
                    elem_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                return INDEX_EMPTY;
            }
        }

        let limit = position & BUCKET_MASK;
        for slot in 0..limit {
            if (*bucket).hash[slot] == hash {
                if is_key_equal(
                    a,
                    elem_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                return INDEX_EMPTY;
            }
        }

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
        (*array_header(a)).length += 1;
        memset(a, 0, elem_size);
        *temp = INDEX_EMPTY;
        array_to_hash(a, elem_size)
    } else {
        let raw_a = hash_to_array(a, elem_size);
        let table = hash_table(raw_a);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(a, elem_size, key, key_size, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                *temp = (*bucket).index[slot as usize & BUCKET_MASK];
            }
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
    let result = stbds_hmget_key_ts(a, elem_size, key, key_size, &mut temp, mode);
    (*array_header(hash_to_array(result, elem_size))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elem_size: usize) -> *mut c_void {
    if a.is_null() || (*array_header(hash_to_array(a, elem_size))).length == 0 {
        let raw = if a.is_null() {
            null_mut()
        } else {
            hash_to_array(a, elem_size)
        };
        a = stbds_arrgrowf(raw, elem_size, 0, 1);
        (*array_header(a)).length += 1;
        memset(a, 0, elem_size);
        a = array_to_hash(a, elem_size);
    }
    a
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let result = realloc(null_mut(), len).cast::<c_char>();
    memmove(result.cast(), string.cast(), len);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
        memset(a, 0, elem_size);
        (*array_header(a)).length += 1;
        a = array_to_hash(a, elem_size);
    }

    let mut raw_a = a;
    a = hash_to_array(a, elem_size);
    let mut table = hash_table(a);

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let new_table = make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table.cast());
        } else {
            (*new_table).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 };
        }
        table = new_table;
        (*array_header(a)).hash_table = table.cast();
    }

    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    let mut step = BUCKET_LENGTH;
    let mut tombstone = INDEX_EMPTY;

    if hash < 2 {
        hash += 2;
    }

    let mut position = probe_position(hash, (*table).slot_count);
    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                if is_key_equal(
                    raw_a,
                    elem_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    (*array_header(a)).temp = (*bucket).index[slot];
                    if mode >= HM_STRING {
                        (*table).temp_key = *raw_a
                            .cast::<u8>()
                            .add(elem_size * (*bucket).index[slot] as usize)
                            .cast::<*mut c_char>();
                    }
                    return array_to_hash(a, elem_size);
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        let limit = position & BUCKET_MASK;
        for slot in 0..limit {
            if (*bucket).hash[slot] == hash {
                if is_key_equal(
                    raw_a,
                    elem_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    (*array_header(a)).temp = (*bucket).index[slot];
                    return array_to_hash(a, elem_size);
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count - 1;
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let index = array_len(a) as isize;
    if index as usize + 1 > array_capacity(a) {
        a = stbds_arrgrowf(a, elem_size, 1, 0);
    }
    raw_a = array_to_hash(a, elem_size);

    c_assert(index as usize + 1 <= array_capacity(a));
    (*array_header(a)).length = index as usize + 1;
    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = index - 1;
    (*array_header(a)).temp = index - 1;

    let destination = a.cast::<u8>().add(elem_size * index as usize);
    match (*table).string.mode {
        SH_STRDUP => {
            let stored_key = duplicate_string(key.cast());
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        SH_ARENA => {
            let stored_key = stbds_stralloc(addr_of_mut!((*table).string), key.cast());
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        SH_DEFAULT => {
            let stored_key = key.cast();
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        _ => {
            ptr::copy_nonoverlapping(key.cast::<u8>(), destination, key_size);
        }
    }

    raw_a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
    memset(a, 0, elem_size);
    (*array_header(a)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, null_mut());
    (*array_header(a)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    array_to_hash(a, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        return null_mut();
    }

    let raw_a = hash_to_array(a, elem_size);
    let mut table = hash_table(raw_a);
    (*array_header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = hm_find_slot(a, elem_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return a;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_slot = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_slot];
    let final_index = array_len(raw_a) as isize - 2;
    c_assert(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*array_header(raw_a)).temp = 1;
    (*bucket).hash[bucket_slot] = HASH_DELETED;
    (*bucket).index[bucket_slot] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        free(
            *a.cast::<u8>()
                .add(elem_size * old_index as usize)
                .cast::<*mut c_void>(),
        );
    }

    if old_index != final_index {
        memmove(
            a.cast::<u8>().add(elem_size * old_index as usize).cast(),
            a.cast::<u8>().add(elem_size * final_index as usize).cast(),
            elem_size,
        );

        let moved_key = a
            .cast::<u8>()
            .add(elem_size * old_index as usize)
            .add(key_offset);
        let lookup_key = if mode == HM_STRING {
            *moved_key.cast::<*mut c_void>()
        } else {
            moved_key.cast()
        };
        slot = hm_find_slot(a, elem_size, lookup_key, key_size, key_offset, mode);
        c_assert(slot >= 0);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_slot = slot as usize & BUCKET_MASK;
        c_assert((*bucket).index[bucket_slot] == final_index);
        (*bucket).index[bucket_slot] = old_index;
    }
    (*array_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let new_table = make_hash_index((*table).slot_count >> 1, table);
        (*array_header(raw_a)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = make_hash_index((*table).slot_count, table);
        (*array_header(raw_a)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    }
    let _ = table;

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = strlen(string) + 1;
    if len > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << ((*arena).block >> 1);

        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > block_size {
            let block =
                realloc(null_mut(), size_of::<StringBlock>() - 8 + len).cast::<StringBlock>();
            let storage = addr_of_mut!((*block).storage).cast::<c_char>();
            memmove(storage.cast(), string.cast(), len);
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return storage;
        } else {
            let block = realloc(null_mut(), size_of::<StringBlock>() - 8 + block_size)
                .cast::<StringBlock>();
            (*block).next = (*arena).storage;
            (*arena).storage = block;
            (*arena).remaining = block_size;
        }
    }

    c_assert(len <= (*arena).remaining);
    let result = addr_of_mut!((*(*arena).storage).storage)
        .cast::<c_char>()
        .add((*arena).remaining - len);
    (*arena).remaining -= len;
    memmove(result.cast(), string.cast(), len);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut current = (*arena).storage;
    while !current.is_null() {
        let next = (*current).next;
        free(current.cast());
        current = next;
    }
    memset(arena.cast(), 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = addr_of_mut!(BUFFER).cast::<c_char>();
    sprintf(buffer, STRKEY_FORMAT.as_ptr().cast::<c_char>(), number);
    buffer
}

unsafe fn string_map_index(map: *mut StringMapEntry) -> isize {
    let raw = hash_to_array(map.cast(), size_of::<StringMapEntry>());
    (*array_header(raw)).temp
}

unsafe fn string_map_get_index(map: &mut *mut StringMapEntry, key: *mut c_char) -> isize {
    *map = stbds_hmget_key(
        (*map).cast(),
        size_of::<StringMapEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    )
    .cast();
    string_map_index(*map)
}

unsafe fn string_map_get(map: &mut *mut StringMapEntry, key: *mut c_char) -> c_int {
    let index = string_map_get_index(map, key);
    (*map.offset(index)).value
}

unsafe fn string_map_put(map: &mut *mut StringMapEntry, key: *mut c_char, value: c_int) {
    *map = stbds_hmput_key(
        (*map).cast(),
        size_of::<StringMapEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    )
    .cast();
    let index = string_map_index(*map);
    (*map.offset(index)).value = value;
}

unsafe fn string_map_delete(map: &mut *mut StringMapEntry, key: *mut c_char) {
    *map = stbds_hmdel_key(
        (*map).cast(),
        size_of::<StringMapEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        0,
        HM_STRING,
    )
    .cast();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let mut string_map: *mut StringMapEntry = null_mut();
    let mut arena = StringArena {
        storage: null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut index = 0;
    while index < num {
        stbds_stralloc(&mut arena, strkey(index));
        index += 1;
    }
    stbds_strreset(&mut arena);

    for pass in 0..2 {
        c_assert(
            string_map_get_index(&mut string_map, FOO.as_ptr().cast::<c_char>().cast_mut())
                == INDEX_EMPTY,
        );

        string_map = stbds_shmode_func(
            size_of::<StringMapEntry>(),
            if pass == 0 {
                SH_STRDUP as c_int
            } else {
                SH_ARENA as c_int
            },
        )
        .cast();

        c_assert(
            string_map_get_index(&mut string_map, FOO.as_ptr().cast::<c_char>().cast_mut())
                == INDEX_EMPTY,
        );

        string_map = stbds_hmput_default(string_map.cast(), size_of::<StringMapEntry>()).cast();
        (*string_map.offset(-1)).value = -2;

        c_assert(
            string_map_get_index(&mut string_map, FOO.as_ptr().cast::<c_char>().cast_mut())
                == INDEX_EMPTY,
        );

        index = 0;
        while index < num {
            string_map_put(&mut string_map, strkey(index), index.wrapping_mul(3));
            index = index.wrapping_add(2);
        }

        let raw = hash_to_array(string_map.cast(), size_of::<StringMapEntry>());
        let map_len = (*array_header(raw)).length as isize - 1;
        for output_index in 0..map_len {
            let entry = *string_map.offset(output_index);
            printf(
                PRINT_FORMAT.as_ptr().cast::<c_char>(),
                entry.key,
                entry.value,
            );
        }

        index = 0;
        while index < num {
            let actual = string_map_get(&mut string_map, strkey(index));
            if index & 1 != 0 {
                c_assert(actual == -2);
            } else {
                c_assert(actual == index.wrapping_mul(3));
            }
            index += 1;
        }

        index = 2;
        while index < num {
            string_map_delete(&mut string_map, strkey(index));
            index = index.wrapping_add(4);
        }

        index = 0;
        while index < num {
            let actual = string_map_get(&mut string_map, strkey(index));
            if index & 3 != 0 {
                c_assert(actual == -2);
            } else {
                c_assert(actual == index.wrapping_mul(3));
            }
            index += 1;
        }

        index = 0;
        while index < num {
            string_map_delete(&mut string_map, strkey(index));
            index += 1;
        }

        index = 0;
        while index < num {
            c_assert(string_map_get(&mut string_map, strkey(index)) == -2);
            index += 1;
        }

        let raw = hash_to_array(string_map.cast(), size_of::<StringMapEntry>());
        stbds_hmfree_func(raw, size_of::<StringMapEntry>());
        string_map = null_mut();
    }
}
