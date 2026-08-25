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

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(left: *const c_void, right: *const c_void, n: usize) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn snprintf(dest: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array.cast::<ArrayHeader>().sub(1)
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
unsafe fn hash_table(raw_array: *mut c_void) -> *mut HashIndex {
    (*header(raw_array)).hash_table.cast()
}

#[inline]
unsafe fn hash_to_array(hash_array: *mut c_void, element_size: usize) -> *mut c_void {
    hash_array.cast::<u8>().sub(element_size).cast()
}

#[inline]
unsafe fn array_to_hash(raw_array: *mut c_void, element_size: usize) -> *mut c_void {
    raw_array.cast::<u8>().add(element_size).cast()
}

#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

fn integer_log2(mut value: usize) -> usize {
    let mut result = 0;
    while value > 1 {
        value >>= 1;
        result += 1;
    }
    result
}

fn load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp = v64_lo ^ v32;
    temp = temp.wrapping_shl(16);
    temp = temp.wrapping_shl(16);
    temp >>= 16;
    temp >>= 16;

    let mut value = v64_hi.wrapping_shl(16);
    value = value.wrapping_shl(16);
    value ^ temp ^ v32
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
    (*table).used_count_threshold = slot_count - (slot_count >> 2);
    (*table).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*table).used_count_shrink_threshold = slot_count >> 2;
    if slot_count <= BUCKET_LENGTH {
        (*table).used_count_shrink_threshold = 0;
    }
    assert!(
        (*table).used_count_threshold + (*table).tombstone_count_threshold < (*table).slot_count
    );

    if !old.is_null() {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
    } else {
        (*table).string = StringArena {
            storage: null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*table).seed = HASH_SEED;
        let a = load_32_or_64(2_147_001_325, 0x27bb_2ee6, 0x87b0_b0fd);
        let b = load_32_or_64(715_136_305, 0, 0xb504_f32d);
        HASH_SEED = HASH_SEED.wrapping_mul(a).wrapping_add(b);
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
            for old_slot_index in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_slot_index] >= 0 {
                    let hash = (*old_bucket).hash[old_slot_index];
                    let mut position = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot_index];
                                break 'probe;
                            }
                        }
                        for slot in 0..(position & BUCKET_MASK) {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot_index];
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
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    element_size: usize,
    add_length: usize,
    minimum_capacity: usize,
) -> *mut c_void {
    let minimum_length = array_len(array).wrapping_add(add_length);
    let mut capacity = minimum_capacity;
    if minimum_length > capacity {
        capacity = minimum_length;
    }
    if capacity <= array_capacity(array) {
        return array;
    }
    if capacity < 2usize.wrapping_mul(array_capacity(array)) {
        capacity = 2usize.wrapping_mul(array_capacity(array));
    } else if capacity < 4 {
        capacity = 4;
    }

    let allocation = if array.is_null() {
        null_mut()
    } else {
        header(array).cast()
    };
    let allocation_size = element_size
        .wrapping_mul(capacity)
        .wrapping_add(size_of::<ArrayHeader>());
    let grown_header = realloc(allocation, allocation_size).cast::<ArrayHeader>();
    let grown_array = grown_header.add(1).cast::<c_void>();
    if array.is_null() {
        (*grown_header).length = 0;
        (*grown_header).hash_table = null_mut();
        (*grown_header).temp = 0;
    }
    (*grown_header).capacity = capacity;
    grown_array
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
pub unsafe extern "C" fn stbds_hash_string(string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut cursor = string.cast::<u8>();
    while *cursor != 0 {
        hash = hash.rotate_left(9).wrapping_add(*cursor as usize);
        cursor = cursor.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash = hash ^ hash ^ hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash = hash ^ hash ^ hash.rotate_right(11);
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

    let mut consumed = 0usize;
    while consumed.wrapping_add(size_of::<usize>()) <= length {
        let low = (*data_pointer.add(0) as u32)
            | ((*data_pointer.add(1) as u32) << 8)
            | ((*data_pointer.add(2) as u32) << 16)
            | ((*data_pointer.add(3) as u32) << 24);
        let high = (*data_pointer.add(4) as u32)
            | ((*data_pointer.add(5) as u32) << 8)
            | ((*data_pointer.add(6) as u32) << 16)
            | ((*data_pointer.add(7) as u32) << 24);
        let mut data = (low as i32 as isize) as usize;
        data |= (high as usize) << 32;

        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        consumed += size_of::<usize>();
        data_pointer = data_pointer.add(size_of::<usize>());
    }

    let mut data = length << (usize::BITS - 8);
    let remaining = length - consumed;
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
        let shifted = ((*data_pointer.add(3) as u32) << 24) as i32;
        data |= (shifted as isize) as usize;
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

unsafe fn is_key_equal(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored_key = array
        .cast::<u8>()
        .add(element_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *stored_key.cast::<*mut c_char>()) == 0
    } else {
        memcmp(key, stored_key.cast(), key_size) == 0
    }
}

unsafe fn find_hash_slot(
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

    let mut step = BUCKET_LENGTH;
    let mut position = probe_position(hash, (*table).slot_count);
    loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    array,
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
                return INDEX_EMPTY;
            }
        }
        for index in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    array,
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
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    temporary: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        let raw_array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        (*header(raw_array)).length += 1;
        ptr::write_bytes(raw_array.cast::<u8>(), 0, element_size);
        *temporary = INDEX_EMPTY;
        array_to_hash(raw_array, element_size)
    } else {
        let raw_array = hash_to_array(array, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temporary = INDEX_EMPTY;
        } else {
            let slot = find_hash_slot(array, element_size, key, key_size, 0, mode);
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
        let raw_array = stbds_arrgrowf(
            if array.is_null() {
                null_mut()
            } else {
                hash_to_array(array, element_size)
            },
            element_size,
            0,
            1,
        );
        (*header(raw_array)).length += 1;
        ptr::write_bytes(raw_array.cast::<u8>(), 0, element_size);
        array = array_to_hash(raw_array, element_size);
    }
    array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let length = strlen(string) + 1;
    let duplicate = realloc(null_mut(), length).cast::<c_char>();
    memmove(duplicate.cast(), string.cast(), length);
    duplicate
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
        let raw_array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        ptr::write_bytes(raw_array.cast::<u8>(), 0, element_size);
        (*header(raw_array)).length += 1;
        array = array_to_hash(raw_array, element_size);
    }

    let raw_hash_array = array;
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

    let mut step = BUCKET_LENGTH;
    let mut position = probe_position(hash, (*table).slot_count);
    let mut tombstone = INDEX_EMPTY;
    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                if is_key_equal(
                    raw_hash_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[slot];
                    if mode >= HM_STRING {
                        (*table).temp_key = *raw_hash_array
                            .cast::<u8>()
                            .add(element_size.wrapping_mul((*bucket).index[slot] as usize))
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
                if is_key_equal(
                    raw_hash_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[slot] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[slot];
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
    assert!((index as usize).wrapping_add(1) <= array_capacity(raw_array));
    (*header(raw_array)).length = (index + 1) as usize;
    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = index - 1;
    (*header(raw_array)).temp = index - 1;

    let destination = raw_array
        .cast::<u8>()
        .add(element_size.wrapping_mul(index as usize));
    match (*table).string.mode {
        SH_STRDUP => {
            let stored_key = duplicate_string(key.cast());
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        SH_ARENA => {
            let stored_key = stbds_stralloc(&mut (*table).string, key.cast());
            *destination.cast::<*mut c_char>() = stored_key;
            (*table).temp_key = stored_key;
        }
        SH_DEFAULT => {
            *destination.cast::<*mut c_char>() = key.cast();
            (*table).temp_key = key.cast();
        }
        _ => {
            memmove(destination.cast(), key, key_size);
        }
    }
    array_to_hash(raw_array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let raw_array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
    ptr::write_bytes(raw_array.cast::<u8>(), 0, element_size);
    (*header(raw_array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, null_mut());
    (*header(raw_array)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    array_to_hash(raw_array, element_size)
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

    let mut slot = find_hash_slot(array, element_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return array;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_index = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = array_len(raw_array) as isize - 2;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored_key = *array
            .cast::<u8>()
            .add(element_size.wrapping_mul(old_index as usize))
            .cast::<*mut c_char>();
        free(stored_key.cast());
    }

    if old_index != final_index {
        memmove(
            array
                .cast::<u8>()
                .add(element_size.wrapping_mul(old_index as usize))
                .cast(),
            array
                .cast::<u8>()
                .add(element_size.wrapping_mul(final_index as usize))
                .cast(),
            element_size,
        );

        let moved_key = array
            .cast::<u8>()
            .add(element_size.wrapping_mul(old_index as usize))
            .add(key_offset);
        slot = if mode == HM_STRING {
            find_hash_slot(
                array,
                element_size,
                *moved_key.cast::<*mut c_char>().cast::<*mut c_void>(),
                key_size,
                key_offset,
                mode,
            )
        } else {
            find_hash_slot(
                array,
                element_size,
                moved_key.cast(),
                key_size,
                key_offset,
                mode,
            )
        };
        assert!(slot >= 0);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_index = slot as usize & BUCKET_MASK;
        assert_eq!((*bucket).index[bucket_index], final_index);
        (*bucket).index[bucket_index] = old_index;
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
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, element_size: usize) {
    if array.is_null() {
        return;
    }
    let table = hash_table(array);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for index in 1..array_len(array) {
                let string = *array
                    .cast::<u8>()
                    .add(element_size.wrapping_mul(index))
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
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let length = strlen(string) + 1;
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
    ptr::write_bytes(arena.cast::<u8>(), 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    snprintf(buffer, 256, c"test_%d".as_ptr(), number);
    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(number: c_int) {
    let mut array: *mut c_int = null_mut();
    for index in 0..4usize {
        for value in [number, 2, 3, 4] {
            let raw = array.cast::<c_void>();
            if raw.is_null() || array_len(raw) + 1 > array_capacity(raw) {
                array = stbds_arrgrowf(raw, size_of::<c_int>(), 1, 0).cast();
            }
            let length = (*header(array.cast())).length;
            *array.add(length) = value;
            (*header(array.cast())).length += 1;
        }
        let length = (*header(array.cast())).length;
        memmove(
            array.add(index).cast(),
            array.add(index + 1).cast(),
            size_of::<c_int>() * (length - 1 - index),
        );
        (*header(array.cast())).length -= 1;
        free(header(array.cast()).cast());
        array = null_mut();

        for value in [number, 2, 3, 4] {
            let raw = array.cast::<c_void>();
            if raw.is_null() || array_len(raw) + 1 > array_capacity(raw) {
                array = stbds_arrgrowf(raw, size_of::<c_int>(), 1, 0).cast();
            }
            let length = (*header(array.cast())).length;
            *array.add(length) = value;
            (*header(array.cast())).length += 1;
        }
        let last = (*header(array.cast())).length - 1;
        *array.add(index) = *array.add(last);
        (*header(array.cast())).length -= 1;
        free(header(array.cast()).cast());
        array = null_mut();
    }
}
