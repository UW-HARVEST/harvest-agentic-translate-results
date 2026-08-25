#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{self, null_mut};

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

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, size: usize) -> c_int;
    fn memmove(destination: *mut c_void, source: *const c_void, size: usize) -> *mut c_void;
    fn memset(destination: *mut c_void, value: c_int, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn sprintf(destination: *mut c_char, format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    (array as *mut ArrayHeader).sub(1)
}

#[inline]
unsafe fn arr_len(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        (*header(array)).length
    }
}

#[inline]
unsafe fn arr_cap(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        (*header(array)).capacity
    }
}

#[inline]
unsafe fn hash_to_arr(hash: *mut c_void, element_size: usize) -> *mut c_void {
    (hash as *mut u8).sub(element_size).cast()
}

#[inline]
unsafe fn arr_to_hash(array: *mut c_void, element_size: usize) -> *mut c_void {
    (array as *mut u8).add(element_size).cast()
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
    let minimum_length = arr_len(array).wrapping_add(add_length);

    if minimum_length > minimum_capacity {
        minimum_capacity = minimum_length;
    }

    if minimum_capacity <= arr_cap(array) {
        return array;
    }

    if minimum_capacity < 2usize.wrapping_mul(arr_cap(array)) {
        minimum_capacity = 2usize.wrapping_mul(arr_cap(array));
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
    let result = (allocation as *mut u8)
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

#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
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
        ((table.add(1) as usize).wrapping_add(CACHE_LINE_SIZE - 1)) & !(CACHE_LINE_SIZE - 1);

    (*table).storage = storage_address as *mut HashBucket;
    (*table).slot_count = slot_count;
    (*table).slot_count_log2 = integer_log2(slot_count);
    (*table).tombstone_count = 0;
    (*table).used_count = 0;
    (*table).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);
    (*table).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);
    (*table).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= BUCKET_LENGTH {
        (*table).used_count_shrink_threshold = 0;
    }
    assert!(
        (*table)
            .used_count_threshold
            .wrapping_add((*table).tombstone_count_threshold)
            < (*table).slot_count
    );

    if !old.is_null() {
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*old).string),
            ptr::addr_of_mut!((*table).string),
            1,
        );
        (*table).seed = (*old).seed;
    } else {
        ptr::write_bytes(ptr::addr_of_mut!((*table).string), 0, 1);
        (*table).seed = HASH_SEED;
        HASH_SEED = HASH_SEED
            .wrapping_mul(0x27bb_2ee6_87b0_b0fd)
            .wrapping_add(0xb504_f32d);
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
            for old_index in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_index] >= 0 {
                    let hash = (*old_bucket).hash[old_index];
                    let mut position = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                        let limit = position & BUCKET_MASK;
                        for index in limit..BUCKET_LENGTH {
                            if (*bucket).hash[index] == HASH_EMPTY {
                                (*bucket).hash[index] = hash;
                                (*bucket).index[index] = (*old_bucket).index[old_index];
                                break 'probe;
                            }
                        }
                        for index in 0..limit {
                            if (*bucket).hash[index] == HASH_EMPTY {
                                (*bucket).hash[index] = hash;
                                (*bucket).index[index] = (*old_bucket).index[old_index];
                                break 'probe;
                            }
                        }
                        position = position.wrapping_add(step);
                        step = step.wrapping_add(BUCKET_LENGTH);
                        position &= (*table).slot_count.wrapping_sub(1);
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

    let mut offset: usize = 0;
    while offset.wrapping_add(size_of::<usize>()) <= length {
        let low = (*data_pointer.add(0) as u32)
            | ((*data_pointer.add(1) as u32) << 8)
            | ((*data_pointer.add(2) as u32) << 16)
            | ((*data_pointer.add(3) as u32) << 24);
        let high = (*data_pointer.add(4) as u32)
            | ((*data_pointer.add(5) as u32) << 8)
            | ((*data_pointer.add(6) as u32) << 16)
            | ((*data_pointer.add(7) as u32) << 24);
        let data = (low as i32 as isize as usize) | ((high as usize) << 32);

        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        offset = offset.wrapping_add(size_of::<usize>());
        data_pointer = data_pointer.add(size_of::<usize>());
    }

    let mut data = length << (usize::BITS - 8);
    let remaining = length.wrapping_sub(offset);
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
        let shifted = (*data_pointer.add(3) as u32) << 24;
        data |= shifted as i32 as isize as usize;
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
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
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

unsafe fn keys_equal(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored_key = (array as *mut u8)
        .add(element_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *stored_key.cast::<*mut c_char>()) == 0
    } else {
        memcmp(key, stored_key.cast(), key_size) == 0
    }
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
                let key = *(array as *mut u8)
                    .add(element_size.wrapping_mul(index))
                    .cast::<*mut c_void>();
                free(key);
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*table).string));
    }
    free((*header(array)).hash_table);
    free(header(array).cast());
}

unsafe fn find_slot(
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = hash_to_arr(hash_array, element_size);
    let table = hash_table(raw_array);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    let mut step = BUCKET_LENGTH;
    let mut position = probe_position(hash, (*table).slot_count);
    loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        let limit = position & BUCKET_MASK;
        for index in limit..BUCKET_LENGTH {
            if (*bucket).hash[index] == hash {
                if keys_equal(
                    hash_array,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + index) as isize;
                }
            } else if (*bucket).hash[index] == HASH_EMPTY {
                return -1;
            }
        }
        for index in 0..limit {
            if (*bucket).hash[index] == hash {
                if keys_equal(
                    hash_array,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + index) as isize;
                }
            } else if (*bucket).hash[index] == HASH_EMPTY {
                return -1;
            }
        }
        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count.wrapping_sub(1);
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
        (*header(array)).length = (*header(array)).length.wrapping_add(1);
        memset(array, 0, element_size);
        *temporary = INDEX_EMPTY;
        arr_to_hash(array, element_size)
    } else {
        let raw_array = hash_to_arr(array, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temporary = INDEX_EMPTY;
        } else {
            let slot = find_slot(array, element_size, key, key_size, 0, mode);
            if slot < 0 {
                *temporary = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
                *temporary = (*bucket).index[(slot as usize) & BUCKET_MASK];
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
    let mut temporary = 0;
    let result = stbds_hmget_key_ts(
        array,
        element_size,
        key,
        key_size,
        ptr::addr_of_mut!(temporary),
        mode,
    );
    (*header(hash_to_arr(result, element_size))).temp = temporary;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut array: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if array.is_null() || (*header(hash_to_arr(array, element_size))).length == 0 {
        array = stbds_arrgrowf(
            if array.is_null() {
                null_mut()
            } else {
                hash_to_arr(array, element_size)
            },
            element_size,
            0,
            1,
        );
        (*header(array)).length = (*header(array)).length.wrapping_add(1);
        memset(array, 0, element_size);
        array = arr_to_hash(array, element_size);
    }
    array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let length = strlen(string).wrapping_add(1);
    let result = realloc(null_mut(), length).cast::<c_char>();
    memmove(result.cast(), string.cast(), length);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        let raw = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        memset(raw, 0, element_size);
        (*header(raw)).length = (*header(raw)).length.wrapping_add(1);
        hash_array = arr_to_hash(raw, element_size);
    }

    let raw_hash_array = hash_array;
    let mut raw_array = hash_to_arr(hash_array, element_size);
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
        hash = hash.wrapping_add(2);
    }

    let mut step = BUCKET_LENGTH;
    let mut position = probe_position(hash, (*table).slot_count);
    let mut tombstone: isize = -1;

    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        let limit = position & BUCKET_MASK;
        for index in limit..BUCKET_LENGTH {
            if (*bucket).hash[index] == hash {
                if keys_equal(
                    raw_hash_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[index];
                    if mode >= HM_STRING {
                        (*table).temp_key = *(raw_hash_array as *mut u8)
                            .add(element_size.wrapping_mul((*bucket).index[index] as usize))
                            .cast::<*mut c_char>();
                    }
                    return arr_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[index] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + index;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[index] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + index) as isize;
            }
        }
        for index in 0..limit {
            if (*bucket).hash[index] == hash {
                if keys_equal(
                    raw_hash_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[index];
                    return arr_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[index] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + index;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[index] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + index) as isize;
            }
        }
        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count.wrapping_sub(1);
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
    }
    (*table).used_count = (*table).used_count.wrapping_add(1);

    let index = arr_len(raw_array) as isize;
    if (index as usize).wrapping_add(1) > arr_cap(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, element_size, 1, 0);
    }
    assert!((index as usize).wrapping_add(1) <= arr_cap(raw_array));
    (*header(raw_array)).length = (index as usize).wrapping_add(1);

    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = index - 1;
    (*header(raw_array)).temp = index - 1;

    let destination = (raw_array as *mut u8).add(element_size.wrapping_mul(index as usize));
    match (*table).string.mode {
        SH_STRDUP => {
            let stored_key = duplicate_string(key.cast());
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        SH_ARENA => {
            let stored_key = stbds_stralloc(ptr::addr_of_mut!((*table).string), key.cast());
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        SH_DEFAULT => {
            *destination.cast::<*mut c_char>() = key.cast();
            (*table).temp_key = key.cast();
        }
        _ => {
            ptr::copy_nonoverlapping(key.cast::<u8>(), destination, key_size);
        }
    }

    arr_to_hash(raw_array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
    memset(array, 0, element_size);
    (*header(array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, null_mut());
    (*header(array)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    arr_to_hash(array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        return null_mut();
    }

    let raw_array = hash_to_arr(hash_array, element_size);
    let table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return hash_array;
    }

    let mut slot = find_slot(hash_array, element_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return hash_array;
    }

    let mut bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
    let mut bucket_index = (slot as usize) & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = arr_len(raw_array) as isize - 2;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count = (*table).used_count.wrapping_sub(1);
    (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored_key = *(hash_array as *mut u8)
            .add(element_size.wrapping_mul(old_index as usize))
            .cast::<*mut c_void>();
        free(stored_key);
    }

    if old_index != final_index {
        let destination =
            (hash_array as *mut u8).add(element_size.wrapping_mul(old_index as usize));
        let source = (hash_array as *mut u8).add(element_size.wrapping_mul(final_index as usize));
        memmove(destination.cast(), source.cast(), element_size);

        let moved_key = if mode == HM_STRING {
            *(destination.add(key_offset).cast::<*mut c_void>())
        } else {
            destination.add(key_offset).cast()
        };
        slot = find_slot(
            hash_array,
            element_size,
            moved_key,
            key_size,
            key_offset,
            mode,
        );
        assert!(slot >= 0);
        bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
        bucket_index = (slot as usize) & BUCKET_MASK;
        assert!((*bucket).index[bucket_index] == final_index);
        (*bucket).index[bucket_index] = old_index;
    }
    (*header(raw_array)).length = (*header(raw_array)).length.wrapping_sub(1);

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let replacement = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_array)).hash_table = replacement.cast();
        free(table.cast());
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let replacement = make_hash_index((*table).slot_count, table);
        (*header(raw_array)).hash_table = replacement.cast();
        free(table.cast());
    }

    hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let length = strlen(string).wrapping_add(1);
    if length > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << ((*arena).block >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
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
            memmove(
                ptr::addr_of_mut!((*block).storage).cast(),
                string.cast(),
                length,
            );
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return ptr::addr_of_mut!((*block).storage).cast();
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

    assert!(length <= (*arena).remaining);
    let result = ptr::addr_of_mut!((*(*arena).storage).storage)
        .cast::<c_char>()
        .add((*arena).remaining - length);
    (*arena).remaining -= length;
    memmove(result.cast(), string.cast(), length);
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
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    sprintf(buffer, FORMAT.as_ptr().cast(), number);
    buffer
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(number: c_int) {
    const KEY: &[u8] = b"a\0";
    const FORMAT: &[u8] = b"%s %d\n\0";

    let mut arena: StringArena = std::mem::zeroed();
    for index in 0..number {
        stbds_stralloc(ptr::addr_of_mut!(arena), strkey(index));
    }
    stbds_strreset(ptr::addr_of_mut!(arena));

    let source = StringMapEntry {
        key: KEY.as_ptr() as *mut c_char,
        value: number,
    };
    let mut map = stbds_shmode_func(size_of::<StringMapEntry>(), SH_STRDUP as c_int);
    map = stbds_hmput_key(
        map,
        size_of::<StringMapEntry>(),
        source.key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );

    let raw_array = hash_to_arr(map, size_of::<StringMapEntry>());
    let index = (*header(raw_array)).temp as usize;
    *map.cast::<StringMapEntry>().add(index) = source;
    (*map.cast::<StringMapEntry>().add(index)).key = (*hash_table(raw_array)).temp_key;

    assert!(*(*map.cast::<StringMapEntry>()).key == b'a' as c_char);
    assert!((*map.cast::<StringMapEntry>()).key != source.key);
    assert!((*map.cast::<StringMapEntry>()).value == source.value);

    let length = (*header(raw_array)).length as isize - 1;
    for index in 0..length {
        let entry = map.cast::<StringMapEntry>().offset(index);
        printf(FORMAT.as_ptr().cast(), (*entry).key, (*entry).value);
    }

    stbds_hmfree_func(raw_array, size_of::<StringMapEntry>());
}
