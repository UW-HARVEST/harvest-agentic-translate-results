#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::ptr::{copy, copy_nonoverlapping, null_mut, write_bytes};

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, size: usize) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn sprintf(target: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
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

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array.cast::<u8>().sub(size_of::<ArrayHeader>()).cast()
}

#[inline]
unsafe fn array_length(array: *mut c_void) -> usize {
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
    let minimum_length = array_length(array).wrapping_add(add_length);
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

    let allocation = realloc(
        if array.is_null() {
            null_mut()
        } else {
            header(array).cast()
        },
        element_size
            .wrapping_mul(minimum_capacity)
            .wrapping_add(size_of::<ArrayHeader>()),
    );
    let result = allocation
        .cast::<u8>()
        .add(size_of::<ArrayHeader>())
        .cast::<c_void>();

    if array.is_null() {
        (*header(result)).length = 0;
        (*header(result)).hash_table = null_mut();
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
        hash = hash.rotate_left(9).wrapping_add((*string as u8) as usize);
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
unsafe fn siphash_bytes(pointer: *mut c_void, length: usize, seed: usize) -> usize {
    let mut data_pointer = pointer.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    #[inline]
    fn round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
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

    let mut offset = 0usize;
    while offset.wrapping_add(size_of::<usize>()) <= length {
        // Match the source's signed-int promotion before conversion to size_t.
        let low = ((*data_pointer.add(0) as i32)
            | ((*data_pointer.add(1) as i32) << 8)
            | ((*data_pointer.add(2) as i32) << 16)
            | ((*data_pointer.add(3) as i32) << 24)) as usize;
        let high = ((*data_pointer.add(4) as i32)
            | ((*data_pointer.add(5) as i32) << 8)
            | ((*data_pointer.add(6) as i32) << 16)
            | ((*data_pointer.add(7) as i32) << 24)) as usize;
        let data = low | high.wrapping_shl(32);

        v3 ^= data;
        round(&mut v0, &mut v1, &mut v2, &mut v3);
        round(&mut v0, &mut v1, &mut v2, &mut v3);
        v0 ^= data;

        offset += size_of::<usize>();
        data_pointer = data_pointer.add(size_of::<usize>());
    }

    let mut data = length << (usize::BITS as usize - 8);
    let remaining = length - offset;
    if remaining >= 7 {
        data |= (*data_pointer.add(6) as usize) << 48;
    }
    if remaining >= 6 {
        data |= (*data_pointer.add(5) as usize) << 40;
    }
    if remaining >= 5 {
        data |= (*data_pointer.add(4) as usize) << 32;
    }
    if remaining >= 4 {
        data |= ((*data_pointer.add(3) as i32) << 24) as usize;
    }
    if remaining >= 3 {
        data |= (*data_pointer.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        data |= (*data_pointer.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        data |= *data_pointer as usize;
    }

    v3 ^= data;
    round(&mut v0, &mut v1, &mut v2, &mut v3);
    round(&mut v0, &mut v1, &mut v2, &mut v3);
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(
    pointer: *mut c_void,
    length: usize,
    seed: usize,
) -> usize {
    siphash_bytes(pointer, length, seed)
}

#[inline]
fn log2(mut slot_count: usize) -> usize {
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
    let table = realloc(null_mut(), allocation_size).cast::<HashIndex>();
    let storage_address =
        ((table.add(1) as usize + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)) as *mut HashBucket;

    (*table).storage = storage_address;
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
        std::ptr::copy_nonoverlapping(&(*old).string, &mut (*table).string, 1);
        (*table).seed = (*old).seed;
    } else {
        (*table).string = zeroed();
        (*table).seed = HASH_SEED;
        let a = 0x27bb_2ee6_87b0_b0fdusize;
        let b = 0x0000_0000_b504_f32dusize;
        HASH_SEED = HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = (*table).storage.add(bucket_index);
        for item in 0..BUCKET_LENGTH {
            (*bucket).hash[item] = HASH_EMPTY;
        }
        for item in 0..BUCKET_LENGTH {
            (*bucket).index[item] = INDEX_EMPTY;
        }
    }

    if !old.is_null() {
        (*table).used_count = (*old).used_count;
        for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
            let old_bucket = (*old).storage.add(old_bucket_index);
            for item in 0..BUCKET_LENGTH {
                if (*old_bucket).index[item] >= 0 {
                    let hash = (*old_bucket).hash[item];
                    let mut position = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                        for candidate in (position & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[candidate] == HASH_EMPTY {
                                (*bucket).hash[candidate] = hash;
                                (*bucket).index[candidate] = (*old_bucket).index[item];
                                break 'probe;
                            }
                        }
                        for candidate in 0..(position & BUCKET_MASK) {
                            if (*bucket).hash[candidate] == HASH_EMPTY {
                                (*bucket).hash[candidate] = hash;
                                (*bucket).index[candidate] = (*old_bucket).index[item];
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

unsafe fn key_is_equal(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let item_key = array
        .cast::<u8>()
        .add(element_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *item_key.cast::<*mut c_char>()) == 0
    } else {
        memcmp(key, item_key.cast(), key_size) == 0
    }
}

unsafe fn find_slot(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = hash_to_array(array, element_size);
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
        for item in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[item] == hash {
                if key_is_equal(
                    array,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[item] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + item) as isize;
                }
            } else if (*bucket).hash[item] == HASH_EMPTY {
                return -1;
            }
        }
        for item in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[item] == hash {
                if key_is_equal(
                    array,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[item] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + item) as isize;
                }
            } else if (*bucket).hash[item] == HASH_EMPTY {
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
    mut array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    temporary: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        (*header(array)).length += 1;
        write_bytes(array.cast::<u8>(), 0, element_size);
        *temporary = INDEX_EMPTY;
        array_to_hash(array, element_size)
    } else {
        let raw_array = hash_to_array(array, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temporary = INDEX_EMPTY;
        } else {
            let slot = find_slot(array, element_size, key, key_size, 0, mode);
            if slot < 0 {
                *temporary = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                *temporary = (*bucket).index[slot as usize & BUCKET_MASK];
            }
        }
        array
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temporary = 0isize;
    let result = stbds_hmget_key_ts(array, element_size, key, key_size, &mut temporary, mode);
    (*header(hash_to_array(result, element_size))).temp = temporary;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut array: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if array.is_null() || (*header(hash_to_array(array, element_size))).length == 0 {
        let raw_array = if array.is_null() {
            null_mut()
        } else {
            hash_to_array(array, element_size)
        };
        array = stbds_arrgrowf(raw_array, element_size, 0, 1);
        (*header(array)).length += 1;
        write_bytes(array.cast::<u8>(), 0, element_size);
        array = array_to_hash(array, element_size);
    }
    array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let length = strlen(string) + 1;
    let result = realloc(null_mut(), length).cast::<c_char>();
    copy(string, result, length);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        let raw = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        write_bytes(raw.cast::<u8>(), 0, element_size);
        (*header(raw)).length += 1;
        array = array_to_hash(raw, element_size);
    }

    let mut raw_visible = array;
    let mut raw_array = hash_to_array(array, element_size);
    let mut table = hash_table(raw_array);
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
        for item in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[item] == hash {
                if key_is_equal(
                    raw_visible,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[item] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[item];
                    if mode >= HM_STRING {
                        (*table).temp_key = *raw_visible
                            .cast::<u8>()
                            .add(element_size * (*bucket).index[item] as usize)
                            .cast::<*mut c_char>();
                    }
                    return array_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[item] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + item;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[item] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + item) as isize;
            }
        }

        for item in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[item] == hash {
                if key_is_equal(
                    raw_visible,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[item] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[item];
                    return array_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[item] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + item;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[item] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + item) as isize;
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

    let item = array_length(raw_array) as isize;
    if item as usize + 1 > array_capacity(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, element_size, 1, 0);
    }
    raw_visible = array_to_hash(raw_array, element_size);
    (*header(raw_array)).length = item as usize + 1;

    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = item - 1;
    (*header(raw_array)).temp = item - 1;

    let item_pointer = raw_array
        .cast::<u8>()
        .add(element_size.wrapping_mul(item as usize));
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            *item_pointer.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(&mut (*table).string, key.cast());
            *item_pointer.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        SH_DEFAULT => {
            let stored = key.cast::<c_char>();
            *item_pointer.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        _ => copy_nonoverlapping(key.cast::<u8>(), item_pointer, key_size),
    }
    raw_visible
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
    write_bytes(array.cast::<u8>(), 0, element_size);
    (*header(array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, null_mut());
    (*header(array)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    array_to_hash(array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        return null_mut();
    }

    let raw_array = hash_to_array(array, element_size);
    let mut table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return array;
    }

    let mut slot = find_slot(array, element_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return array;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_item = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_item];
    let final_index = array_length(raw_array) as isize - 2;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_item] = HASH_DELETED;
    (*bucket).index[bucket_item] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored = *array
            .cast::<u8>()
            .add(element_size * old_index as usize)
            .cast::<*mut c_char>();
        free(stored.cast());
    }

    if old_index != final_index {
        copy(
            array.cast::<u8>().add(element_size * final_index as usize),
            array.cast::<u8>().add(element_size * old_index as usize),
            element_size,
        );
        let moved_key = array
            .cast::<u8>()
            .add(element_size * old_index as usize)
            .add(key_offset);
        slot = if mode == HM_STRING {
            find_slot(
                array,
                element_size,
                *moved_key.cast::<*mut c_char>() as *mut c_void,
                key_size,
                key_offset,
                mode,
            )
        } else {
            find_slot(
                array,
                element_size,
                moved_key.cast(),
                key_size,
                key_offset,
                mode,
            )
        };
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_item = slot as usize & BUCKET_MASK;
        (*bucket).index[bucket_item] = old_index;
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
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    const BLOCK_SIZE_MIN: usize = 512;
    const BLOCK_SIZE_MAX: usize = 1 << 20;

    let length = strlen(string) + 1;
    if length > (*arena).remaining {
        let block_size = BLOCK_SIZE_MIN << ((*arena).block >> 1);
        if block_size < BLOCK_SIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if length > block_size {
            let block = realloc(
                null_mut(),
                size_of::<StringBlock>()
                    .wrapping_sub(8)
                    .wrapping_add(length),
            )
            .cast::<StringBlock>();
            copy(string, (*block).storage.as_mut_ptr(), length);
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return (*block).storage.as_mut_ptr();
        }

        let block = realloc(
            null_mut(),
            size_of::<StringBlock>()
                .wrapping_sub(8)
                .wrapping_add(block_size),
        )
        .cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let result = (*(*arena).storage)
        .storage
        .as_mut_ptr()
        .add((*arena).remaining - length);
    (*arena).remaining -= length;
    copy(string, result, length);
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
    write_bytes(arena.cast::<u8>(), 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, element_size: usize) {
    if array.is_null() {
        return;
    }
    let table = hash_table(array);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for item in 1..array_length(array) {
                let string = *array
                    .cast::<u8>()
                    .add(element_size * item)
                    .cast::<*mut c_char>();
                free(string.cast());
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*header(array)).hash_table);
    free(header(array).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = std::ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    sprintf(buffer, c"test_%d".as_ptr(), number);
    buffer
}

#[repr(C)]
struct HelxoEntry {
    key: *mut c_char,
    value: c_char,
}

unsafe fn helxo_put(mut hash: *mut c_void, key: *mut c_char, value: c_char) -> *mut c_void {
    hash = stbds_hmput_key(
        hash,
        size_of::<HelxoEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );
    let raw_array = hash_to_array(hash, size_of::<HelxoEntry>());
    let index = (*header(raw_array)).temp as usize;
    (*hash.cast::<HelxoEntry>().add(index)).value = value;
    hash
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    let mut name = *b"jen\0";
    let mut hash = null_mut();
    hash = helxo_put(hash, c"bob".as_ptr().cast_mut(), b'h' as c_char);
    hash = helxo_put(hash, c"sally".as_ptr().cast_mut(), b'e' as c_char);
    hash = helxo_put(hash, c"fred".as_ptr().cast_mut(), b'l' as c_char);
    hash = helxo_put(hash, c"jen".as_ptr().cast_mut(), b'x' as c_char);
    hash = helxo_put(hash, c"doug".as_ptr().cast_mut(), b'o' as c_char);
    hash = helxo_put(hash, name.as_mut_ptr().cast(), letter);

    let raw_array = hash_to_array(hash, size_of::<HelxoEntry>());
    for index in 0..(array_length(raw_array) - 1) {
        let entry = hash.cast::<HelxoEntry>().add(index);
        printf(c"%s %c\n".as_ptr(), (*entry).key, (*entry).value as c_int);
    }
    stbds_hmfree_func(raw_array, size_of::<HelxoEntry>());
}
