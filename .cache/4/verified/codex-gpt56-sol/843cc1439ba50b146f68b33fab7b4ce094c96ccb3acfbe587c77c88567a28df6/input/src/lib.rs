#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const BUCKET_LENGTH: usize = 8;
const BUCKET_SHIFT: usize = 3;
const BUCKET_MASK: usize = BUCKET_LENGTH - 1;
const CACHE_LINE_SIZE: usize = 64;

const HM_STRING: c_int = 1;
const SH_DEFAULT: u8 = 1;
const SH_STRDUP: u8 = 2;
const SH_ARENA: u8 = 3;

const INDEX_EMPTY: isize = -1;
const INDEX_DELETED: isize = -2;
const HASH_EMPTY: usize = 0;
const HASH_DELETED: usize = 1;

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
#[derive(Clone, Copy)]
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

unsafe extern "C" {
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, size: usize) -> c_int;
    fn memcpy(destination: *mut c_void, source: *const c_void, size: usize) -> *mut c_void;
    fn memmove(destination: *mut c_void, source: *const c_void, size: usize) -> *mut c_void;
    fn memset(destination: *mut c_void, value: c_int, size: usize) -> *mut c_void;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn sprintf(destination: *mut c_char, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut BUFFER: [c_char; 256] = [0; 256];
static STRKEY_FORMAT: &[u8; 8] = b"test_%d\0";

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array
        .cast::<u8>()
        .sub(size_of::<ArrayHeader>())
        .cast::<ArrayHeader>()
}

#[inline]
unsafe fn array_len(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        (*header(array)).length
    }
}

#[inline]
unsafe fn array_capacity(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        (*header(array)).capacity
    }
}

#[inline]
unsafe fn hash_to_array(hash: *mut c_void, element_size: usize) -> *mut c_void {
    hash.cast::<u8>().sub(element_size).cast()
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, element_size: usize) -> *mut c_void {
    array.cast::<u8>().add(element_size).cast()
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    (*header(array)).hash_table.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    element_size: usize,
    add_length: usize,
    mut minimum_capacity: usize,
) -> *mut c_void {
    let minimum_length = array_len(array).wrapping_add(add_length);
    if minimum_length > minimum_capacity {
        minimum_capacity = minimum_length;
    }

    let old_capacity = array_capacity(array);
    if minimum_capacity <= old_capacity {
        return array;
    }

    if minimum_capacity < old_capacity.wrapping_mul(2) {
        minimum_capacity = old_capacity.wrapping_mul(2);
    } else if minimum_capacity < 4 {
        minimum_capacity = 4;
    }

    let old_allocation = if array.is_null() {
        ptr::null_mut()
    } else {
        header(array).cast()
    };
    let allocation_size = element_size
        .wrapping_mul(minimum_capacity)
        .wrapping_add(size_of::<ArrayHeader>());
    let allocation = realloc(old_allocation, allocation_size);
    let result = allocation
        .cast::<u8>()
        .add(size_of::<ArrayHeader>())
        .cast::<c_void>();

    if array.is_null() {
        (*header(result)).length = 0;
        (*header(result)).hash_table = ptr::null_mut();
        (*header(result)).temp = 0;
    }
    (*header(result)).capacity = minimum_capacity;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(array: *mut c_void) {
    free(header(array).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    HASH_SEED = seed;
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
    let old = hash;
    hash ^= old ^ old.rotate_right(31);
    hash = hash.wrapping_mul(21);
    let old = hash;
    hash ^= old ^ old.rotate_right(11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= hash.rotate_right(22);
    hash.wrapping_add(seed)
}

#[inline]
fn sip_round(
    mut v0: usize,
    mut v1: usize,
    mut v2: usize,
    mut v3: usize,
) -> (usize, usize, usize, usize) {
    v0 = v0.wrapping_add(v1);
    v1 = v1.rotate_left(13);
    v1 ^= v0;
    v0 = v0.rotate_left(usize::BITS / 2);
    v2 = v2.wrapping_add(v3);
    v3 = v3.rotate_left(16);
    v3 ^= v2;
    v2 = v2.wrapping_add(v1);
    v1 = v1.rotate_left(17);
    v1 ^= v2;
    v2 = v2.rotate_left(usize::BITS / 2);
    v0 = v0.wrapping_add(v3);
    v3 = v3.rotate_left(21);
    v3 ^= v0;
    (v0, v1, v2, v3)
}

unsafe fn siphash_bytes(data_pointer: *mut c_void, length: usize, seed: usize) -> usize {
    let data_bytes = data_pointer.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0usize;
    while offset.wrapping_add(size_of::<usize>()) <= length {
        let low_bits = (*data_bytes.add(offset) as u32)
            | ((*data_bytes.add(offset + 1) as u32) << 8)
            | ((*data_bytes.add(offset + 2) as u32) << 16)
            | ((*data_bytes.add(offset + 3) as u32) << 24);
        let high_bits = (*data_bytes.add(offset + 4) as u32)
            | ((*data_bytes.add(offset + 5) as u32) << 8)
            | ((*data_bytes.add(offset + 6) as u32) << 16)
            | ((*data_bytes.add(offset + 7) as u32) << 24);
        let low = low_bits as i32 as isize as usize;
        let high = high_bits as i32 as isize as usize;
        let block = low | (high << 32);

        v3 ^= block;
        for _ in 0..2 {
            (v0, v1, v2, v3) = sip_round(v0, v1, v2, v3);
        }
        v0 ^= block;
        offset += size_of::<usize>();
    }

    let remaining = length - offset;
    let mut tail = length.wrapping_shl(usize::BITS - 8);
    if remaining >= 7 {
        tail |= (*data_bytes.add(offset + 6) as usize) << 48;
    }
    if remaining >= 6 {
        tail |= (*data_bytes.add(offset + 5) as usize) << 40;
    }
    if remaining >= 5 {
        tail |= (*data_bytes.add(offset + 4) as usize) << 32;
    }
    if remaining >= 4 {
        let fourth_byte = ((*data_bytes.add(offset + 3) as u32) << 24) as i32;
        tail |= fourth_byte as isize as usize;
    }
    if remaining >= 3 {
        tail |= (*data_bytes.add(offset + 2) as usize) << 16;
    }
    if remaining >= 2 {
        tail |= (*data_bytes.add(offset + 1) as usize) << 8;
    }
    if remaining >= 1 {
        tail |= *data_bytes.add(offset) as usize;
    }

    v3 ^= tail;
    for _ in 0..2 {
        (v0, v1, v2, v3) = sip_round(v0, v1, v2, v3);
    }
    v0 ^= tail;
    v2 ^= 0xff;
    for _ in 0..4 {
        (v0, v1, v2, v3) = sip_round(v0, v1, v2, v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(data: *mut c_void, length: usize, seed: usize) -> usize {
    siphash_bytes(data, length, seed)
}

#[inline]
fn integer_log2(mut slot_count: usize) -> usize {
    let mut result = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        result += 1;
    }
    result
}

#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>();
    let storage_address = (table.add(1) as usize + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);

    (*table).storage = storage_address as *mut HashBucket;
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

    if old.is_null() {
        ptr::write_bytes(
            ptr::addr_of_mut!((*table).string).cast::<u8>(),
            0,
            size_of::<StringArena>(),
        );
        (*table).seed = HASH_SEED;
        let a = 0x27bb_2ee6_87b0_b0fdusize;
        let b = 0x0000_0000_b504_f32dusize;
        HASH_SEED = HASH_SEED.wrapping_mul(a).wrapping_add(b);
    } else {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
    }

    for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = (*table).storage.add(bucket_index);
        for index in 0..BUCKET_LENGTH {
            (*bucket).hash[index] = HASH_EMPTY;
        }
        for index in 0..BUCKET_LENGTH {
            (*bucket).index[index] = INDEX_EMPTY;
        }
    }

    if !old.is_null() {
        (*table).used_count = (*old).used_count;
        for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
            let old_bucket = (*old).storage.add(old_bucket_index);
            for old_slot in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_slot] >= 0 {
                    let hash = (*old_bucket).hash[old_slot];
                    let mut position = probe_position(hash, slot_count);
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
                        for slot in 0..(position & BUCKET_MASK) {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot];
                                break 'probe;
                            }
                        }
                        position = position.wrapping_add(step);
                        step = step.wrapping_add(BUCKET_LENGTH);
                        position &= slot_count - 1;
                    }
                }
            }
        }
    }
    table
}

unsafe fn key_is_equal(
    entries: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored_key = entries
        .cast::<u8>()
        .add(element_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *stored_key.cast::<*mut c_char>()) == 0
    } else {
        memcmp(key, stored_key.cast(), key_size) == 0
    }
}

unsafe fn find_slot(
    entries: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = hash_to_array(entries, element_size);
    let table = hash_table(raw_array);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut position = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                if key_is_equal(
                    entries,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                return -1;
            }
        }

        for slot in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[slot] == hash {
                if key_is_equal(
                    entries,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                return -1;
            }
        }
        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut entries: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    temporary: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if entries.is_null() {
        entries = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
        (*header(entries)).length += 1;
        memset(entries, 0, element_size);
        *temporary = INDEX_EMPTY;
        array_to_hash(entries, element_size)
    } else {
        let raw_array = hash_to_array(entries, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temporary = INDEX_EMPTY;
        } else {
            let slot = find_slot(entries, element_size, key, key_size, 0, mode);
            if slot < 0 {
                *temporary = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                *temporary = (*bucket).index[slot as usize & BUCKET_MASK];
            }
        }
        entries
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    entries: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temporary = 0isize;
    let result = stbds_hmget_key_ts(entries, element_size, key, key_size, &mut temporary, mode);
    let raw_array = hash_to_array(result, element_size);
    (*header(raw_array)).temp = temporary;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut entries: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if entries.is_null() || (*header(hash_to_array(entries, element_size))).length == 0 {
        let raw_array = if entries.is_null() {
            ptr::null_mut()
        } else {
            hash_to_array(entries, element_size)
        };
        entries = stbds_arrgrowf(raw_array, element_size, 0, 1);
        (*header(entries)).length += 1;
        memset(entries, 0, element_size);
        entries = array_to_hash(entries, element_size);
    }
    entries
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let mut length = 0usize;
    while *string.add(length) != 0 {
        length += 1;
    }
    length += 1;
    let duplicate = realloc(ptr::null_mut(), length).cast::<c_char>();
    memmove(duplicate.cast(), string.cast(), length);
    duplicate
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut entries: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if entries.is_null() {
        entries = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
        memset(entries, 0, element_size);
        (*header(entries)).length += 1;
        entries = array_to_hash(entries, element_size);
    }

    let mut raw_entries = entries;
    let mut raw_array = hash_to_array(entries, element_size);
    let mut table = hash_table(raw_array);

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let new_table = make_hash_index(slot_count, table);
        if table.is_null() {
            (*new_table).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 };
        } else {
            free(table.cast());
        }
        (*header(raw_array)).hash_table = new_table.cast();
        table = new_table;
    }

    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut position = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    let mut tombstone = -1isize;

    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                let index = (*bucket).index[slot];
                if key_is_equal(
                    raw_entries,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    index as usize,
                ) {
                    (*header(raw_array)).temp = index;
                    if mode >= HM_STRING {
                        (*table).temp_key = *raw_entries
                            .cast::<u8>()
                            .add(element_size.wrapping_mul(index as usize))
                            .cast::<*mut c_char>();
                    }
                    return array_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        for slot in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[slot] == hash {
                let index = (*bucket).index[slot];
                if key_is_equal(
                    raw_entries,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    index as usize,
                ) {
                    (*header(raw_array)).temp = index;
                    return array_to_hash(raw_array, element_size);
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

    let index = array_len(raw_array) as isize;
    if (index as usize).wrapping_add(1) > array_capacity(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, element_size, 1, 0);
    }
    raw_entries = array_to_hash(raw_array, element_size);
    (*header(raw_array)).length = (index as usize).wrapping_add(1);

    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = index - 1;
    (*header(raw_array)).temp = index - 1;

    let destination = raw_array
        .cast::<u8>()
        .add(element_size.wrapping_mul(index as usize));
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            *destination.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key.cast());
            *destination.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        SH_DEFAULT => {
            *destination.cast::<*mut c_char>() = key.cast();
            (*table).temp_key = key.cast();
        }
        _ => {
            memcpy(destination.cast(), key, key_size);
        }
    }
    raw_entries
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
    memset(array, 0, element_size);
    (*header(array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
    (*header(array)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    array_to_hash(array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    entries: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if entries.is_null() {
        return ptr::null_mut();
    }

    let raw_array = hash_to_array(entries, element_size);
    let mut table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return entries;
    }

    let mut slot = find_slot(entries, element_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return entries;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_slot = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_slot];
    let final_index = array_len(raw_array).wrapping_sub(1).wrapping_sub(1) as isize;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_slot] = HASH_DELETED;
    (*bucket).index[bucket_slot] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored = *entries
            .cast::<u8>()
            .add(element_size.wrapping_mul(old_index as usize))
            .cast::<*mut c_void>();
        free(stored);
    }

    if old_index != final_index {
        let old_entry = entries
            .cast::<u8>()
            .add(element_size.wrapping_mul(old_index as usize));
        let final_entry = entries
            .cast::<u8>()
            .add(element_size.wrapping_mul(final_index as usize));
        memmove(old_entry.cast(), final_entry.cast(), element_size);

        let moved_key = if mode == HM_STRING {
            *old_entry.add(key_offset).cast::<*mut c_char>() as *mut c_void
        } else {
            old_entry.add(key_offset).cast()
        };
        slot = find_slot(entries, element_size, moved_key, key_size, key_offset, mode);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_slot = slot as usize & BUCKET_MASK;
        (*bucket).index[bucket_slot] = old_index;
    }
    (*header(raw_array)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let new_table = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_array)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = make_hash_index((*table).slot_count, table);
        (*header(raw_array)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    }
    let _ = table;
    entries
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let mut length = 0usize;
    while *string.add(length) != 0 {
        length += 1;
    }
    length += 1;

    if length > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl(((*arena).block >> 1) as u32);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if length > block_size {
            let allocation_size = size_of::<StringBlock>() - 8 + length;
            let block = realloc(ptr::null_mut(), allocation_size).cast::<StringBlock>();
            memmove(
                ptr::addr_of_mut!((*block).storage).cast(),
                string.cast(),
                length,
            );
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = ptr::null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return ptr::addr_of_mut!((*block).storage).cast();
        }

        let allocation_size = size_of::<StringBlock>() - 8 + block_size;
        let block = realloc(ptr::null_mut(), allocation_size).cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let destination = ptr::addr_of_mut!((*(*arena).storage).storage)
        .cast::<c_char>()
        .add((*arena).remaining - length);
    (*arena).remaining -= length;
    memmove(destination.cast(), string.cast(), length);
    destination
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut block = (*arena).storage;
    while !block.is_null() {
        let next = (*block).next;
        free(block.cast());
        block = next;
    }
    memset(arena.cast(), 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, element_size: usize) {
    if array.is_null() {
        return;
    }
    let table = hash_table(array);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for index in 1..(*header(array)).length {
                let string = *array
                    .cast::<u8>()
                    .add(element_size.wrapping_mul(index))
                    .cast::<*mut c_void>();
                free(string);
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*table).string));
    }
    free((*header(array)).hash_table);
    free(header(array).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = ptr::addr_of_mut!(BUFFER).cast::<c_char>();
    sprintf(buffer, STRKEY_FORMAT.as_ptr().cast(), number);
    buffer
}

unsafe fn push_int(array: &mut *mut c_int, value: c_int) {
    if (*array).is_null()
        || (*header((*array).cast())).length + 1 > (*header((*array).cast())).capacity
    {
        *array = stbds_arrgrowf((*array).cast(), size_of::<c_int>(), 1, 0).cast();
    }
    let length = (*header((*array).cast())).length;
    *(*array).add(length) = value;
    (*header((*array).cast())).length += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(number: c_int) {
    for insertion_index in 0..5usize {
        let mut array: *mut c_int = ptr::null_mut();
        push_int(&mut array, 1);
        push_int(&mut array, 2);
        push_int(&mut array, 3);
        push_int(&mut array, 4);

        let old_length = (*header(array.cast())).length;
        if old_length + 1 > (*header(array.cast())).capacity {
            array = stbds_arrgrowf(array.cast(), size_of::<c_int>(), 1, 0).cast();
        }
        (*header(array.cast())).length += 1;
        memmove(
            array.add(insertion_index + 1).cast(),
            array.add(insertion_index).cast(),
            size_of::<c_int>() * (old_length - insertion_index),
        );
        *array.add(insertion_index) = number;

        assert_eq!(*array.add(insertion_index), number);
        if insertion_index < 4 {
            assert_eq!(*array.add(4), 4);
        }
        free(header(array.cast()).cast());
    }
}
