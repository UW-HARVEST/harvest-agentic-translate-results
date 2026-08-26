#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{addr_of_mut, null_mut};

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

#[link(name = "c")]
unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn strlen(value: *const c_char) -> usize;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
}

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
pub struct StbdsStringArena {
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
    string: StbdsStringArena,
    storage: *mut HashBucket,
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];
static STRKEY_FORMAT: &[u8] = b"test_%d\0";

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array.cast::<ArrayHeader>().sub(1)
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
unsafe fn hash_table(raw_array: *mut c_void) -> *mut HashIndex {
    (*header(raw_array)).hash_table.cast::<HashIndex>()
}

#[inline]
unsafe fn hash_to_arr(hash_array: *mut c_void, elem_size: usize) -> *mut c_void {
    hash_array.cast::<u8>().sub(elem_size).cast()
}

#[inline]
unsafe fn arr_to_hash(raw_array: *mut c_void, elem_size: usize) -> *mut c_void {
    raw_array.cast::<u8>().add(elem_size).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    elem_size: usize,
    add_len: usize,
    requested_cap: usize,
) -> *mut c_void {
    let mut min_cap = requested_cap;
    let min_len = arr_len(array).wrapping_add(add_len);

    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= arr_cap(array) {
        return array;
    }
    if min_cap < 2usize.wrapping_mul(arr_cap(array)) {
        min_cap = 2usize.wrapping_mul(arr_cap(array));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let allocation = realloc(
        if array.is_null() {
            null_mut()
        } else {
            header(array).cast()
        },
        elem_size
            .wrapping_mul(min_cap)
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
    (*header(result)).capacity = min_cap;
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

    if !old.is_null() {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
    } else {
        (*table).string = StbdsStringArena {
            storage: null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*table).seed = HASH_SEED;
        let a = if usize::BITS == 64 {
            0x27bb_2ee6_87b0_b0fdusize
        } else {
            2_147_001_325usize
        };
        let b = if usize::BITS == 64 {
            0x0000_0000_b504_f32dusize
        } else {
            715_136_305usize
        };
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
            for old_index in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_index] >= 0 {
                    let hash = (*old_bucket).hash[old_index];
                    let mut position = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;

                    'place: loop {
                        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[index] == HASH_EMPTY {
                                (*bucket).hash[index] = hash;
                                (*bucket).index[index] = (*old_bucket).index[old_index];
                                break 'place;
                            }
                        }
                        for index in 0..(position & BUCKET_MASK) {
                            if (*bucket).hash[index] == HASH_EMPTY {
                                (*bucket).hash[index] = hash;
                                (*bucket).index[index] = (*old_bucket).index[old_index];
                                break 'place;
                            }
                        }
                        position = position.wrapping_add(step) & ((*table).slot_count - 1);
                        step = step.wrapping_add(BUCKET_LENGTH);
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
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ hash.rotate_right(11);
    hash = hash.wrapping_add(hash << 6);
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

unsafe fn siphash_bytes(data_pointer: *mut c_void, length: usize, seed: usize) -> usize {
    let mut data_bytes = data_pointer.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0;
    while offset + size_of::<usize>() <= length {
        let low = ((*data_bytes.add(0) as u32)
            | ((*data_bytes.add(1) as u32) << 8)
            | ((*data_bytes.add(2) as u32) << 16)
            | ((*data_bytes.add(3) as u32) << 24)) as i32;
        let high = (*data_bytes.add(4) as usize)
            | ((*data_bytes.add(5) as usize) << 8)
            | ((*data_bytes.add(6) as usize) << 16)
            | ((*data_bytes.add(7) as usize) << 24);
        let block = (low as isize as usize) | (high << 32);

        v3 ^= block;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= block;
        offset += size_of::<usize>();
        data_bytes = data_bytes.add(size_of::<usize>());
    }

    let remaining = length - offset;
    let mut tail = length << (usize::BITS - 8);
    if remaining >= 7 {
        tail |= (*data_bytes.add(6) as usize) << 48;
    }
    if remaining >= 6 {
        tail |= (*data_bytes.add(5) as usize) << 40;
    }
    if remaining >= 5 {
        tail |= (*data_bytes.add(4) as usize) << 32;
    }
    if remaining >= 4 {
        let shifted = ((*data_bytes.add(3) as u32) << 24) as i32;
        tail |= shifted as isize as usize;
    }
    if remaining >= 3 {
        tail |= (*data_bytes.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        tail |= (*data_bytes.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        tail |= *data_bytes as usize;
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
pub unsafe extern "C" fn stbds_hash_bytes(data: *mut c_void, length: usize, seed: usize) -> usize {
    siphash_bytes(data, length, seed)
}

unsafe fn is_key_equal(
    array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored_key = array
        .cast::<u8>()
        .add(elem_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast::<c_char>(), *stored_key.cast::<*mut c_char>()) == 0
    } else {
        libc_memcmp(key.cast_const(), stored_key.cast(), key_size) == 0
    }
}

#[inline]
unsafe fn libc_memcmp(left: *const c_void, right: *const c_void, count: usize) -> c_int {
    unsafe extern "C" {
        fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> c_int;
    }
    memcmp(left, right, count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(raw_array: *mut c_void, elem_size: usize) {
    if raw_array.is_null() {
        return;
    }

    let table = hash_table(raw_array);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for index in 1..(*header(raw_array)).length {
                let string = *raw_array
                    .cast::<u8>()
                    .add(elem_size.wrapping_mul(index))
                    .cast::<*mut c_void>();
                free(string);
            }
        }
        stbds_strreset(addr_of_mut!((*table).string));
    }
    free((*header(raw_array)).hash_table);
    free(header(raw_array).cast());
}

unsafe fn hm_find_slot(
    hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = hash_to_arr(hash_array, elem_size);
    let table = hash_table(raw_array);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast::<c_char>(), (*table).seed)
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
        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    hash_array,
                    elem_size,
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
        for index in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    hash_array,
                    elem_size,
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
        position = position.wrapping_add(step) & ((*table).slot_count - 1);
        step = step.wrapping_add(BUCKET_LENGTH);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        let raw_array = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
        (*header(raw_array)).length += 1;
        memset(raw_array, 0, elem_size);
        *temp = INDEX_EMPTY;
        arr_to_hash(raw_array, elem_size)
    } else {
        let raw_array = hash_to_arr(hash_array, elem_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(hash_array, elem_size, key, key_size, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                *temp = (*bucket).index[slot as usize & BUCKET_MASK];
            }
        }
        hash_array
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0;
    let result = stbds_hmget_key_ts(hash_array, elem_size, key, key_size, &mut temp, mode);
    (*header(hash_to_arr(result, elem_size))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    hash_array: *mut c_void,
    elem_size: usize,
) -> *mut c_void {
    if hash_array.is_null() || (*header(hash_to_arr(hash_array, elem_size))).length == 0 {
        let raw_array = stbds_arrgrowf(
            if hash_array.is_null() {
                null_mut()
            } else {
                hash_to_arr(hash_array, elem_size)
            },
            elem_size,
            0,
            1,
        );
        (*header(raw_array)).length += 1;
        memset(raw_array, 0, elem_size);
        arr_to_hash(raw_array, elem_size)
    } else {
        hash_array
    }
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let length = strlen(string) + 1;
    let result = realloc(null_mut(), length).cast::<c_char>();
    memmove(result.cast(), string.cast(), length);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        let raw_array = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
        memset(raw_array, 0, elem_size);
        (*header(raw_array)).length += 1;
        hash_array = arr_to_hash(raw_array, elem_size);
    }

    let mut raw_hash_array = hash_array;
    let mut raw_array = hash_to_arr(hash_array, elem_size);
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
        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    raw_hash_array,
                    elem_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[index];
                    if mode >= HM_STRING {
                        (*table).temp_key = *raw_hash_array
                            .cast::<u8>()
                            .add(elem_size.wrapping_mul((*bucket).index[index] as usize))
                            .cast::<*mut c_char>();
                    }
                    return arr_to_hash(raw_array, elem_size);
                }
            } else if (*bucket).hash[index] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + index;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[index] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + index) as isize;
            }
        }

        for index in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    raw_hash_array,
                    elem_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[index];
                    return arr_to_hash(raw_array, elem_size);
                }
            } else if (*bucket).hash[index] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + index;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[index] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + index) as isize;
            }
        }

        position = position.wrapping_add(step) & ((*table).slot_count - 1);
        step = step.wrapping_add(BUCKET_LENGTH);
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let element_index = arr_len(raw_array) as isize;
    if element_index as usize + 1 > arr_cap(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, elem_size, 1, 0);
    }
    raw_hash_array = arr_to_hash(raw_array, elem_size);
    (*header(raw_array)).length = element_index as usize + 1;

    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = element_index - 1;
    (*header(raw_array)).temp = element_index - 1;

    let destination = raw_array
        .cast::<u8>()
        .add(elem_size.wrapping_mul(element_index as usize));
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            *destination.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(addr_of_mut!((*table).string), key.cast());
            *destination.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        SH_DEFAULT => {
            let stored = key.cast::<c_char>();
            *destination.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        _ => {
            memcpy(destination.cast(), key.cast_const(), key_size);
        }
    }

    raw_hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let raw_array = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
    memset(raw_array, 0, elem_size);
    (*header(raw_array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, null_mut());
    (*header(raw_array)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    arr_to_hash(raw_array, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        return null_mut();
    }

    let raw_array = hash_to_arr(hash_array, elem_size);
    let mut table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return hash_array;
    }

    let mut slot = hm_find_slot(hash_array, elem_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return hash_array;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_index = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = arr_len(raw_array) as isize - 2;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let string = *hash_array
            .cast::<u8>()
            .add(elem_size.wrapping_mul(old_index as usize))
            .cast::<*mut c_void>();
        free(string);
    }

    if old_index != final_index {
        let destination = hash_array
            .cast::<u8>()
            .add(elem_size.wrapping_mul(old_index as usize));
        let source = hash_array
            .cast::<u8>()
            .add(elem_size.wrapping_mul(final_index as usize));
        memmove(destination.cast(), source.cast(), elem_size);

        let moved_key = if mode == HM_STRING {
            (*destination.add(key_offset).cast::<*mut c_char>()).cast::<c_void>()
        } else {
            destination.add(key_offset).cast::<c_void>()
        };
        slot = hm_find_slot(hash_array, elem_size, moved_key, key_size, key_offset, mode);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_index = slot as usize & BUCKET_MASK;
        (*bucket).index[bucket_index] = old_index;
    }

    (*header(raw_array)).length -= 1;
    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let replacement = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_array)).hash_table = replacement.cast();
        free(table.cast());
        table = replacement;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let replacement = make_hash_index((*table).slot_count, table);
        (*header(raw_array)).hash_table = replacement.cast();
        free(table.cast());
        table = replacement;
    }
    let _ = table;

    hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StbdsStringArena,
    string: *mut c_char,
) -> *mut c_char {
    let length = strlen(string) + 1;
    if length > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << (((*arena).block as usize) >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if length > block_size {
            let block =
                realloc(null_mut(), size_of::<StringBlock>() - 8 + length).cast::<StringBlock>();
            memmove(addr_of_mut!((*block).storage).cast(), string.cast(), length);
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return addr_of_mut!((*block).storage).cast::<c_char>();
        }

        let block =
            realloc(null_mut(), size_of::<StringBlock>() - 8 + block_size).cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let destination = addr_of_mut!((*(*arena).storage).storage)
        .cast::<c_char>()
        .add((*arena).remaining - length);
    (*arena).remaining -= length;
    memmove(destination.cast(), string.cast(), length);
    destination
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StbdsStringArena) {
    let mut block = (*arena).storage;
    while !block.is_null() {
        let next = (*block).next;
        free(block.cast());
        block = next;
    }
    memset(arena.cast(), 0, size_of::<StbdsStringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    sprintf(buffer, STRKEY_FORMAT.as_ptr().cast(), number);
    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(number: c_int) {
    let mut array: *mut c_int = null_mut();
    let mut outer: c_int = 0;
    while outer < number {
        let mut inner: c_int = 0;
        while inner < outer {
            let raw = array.cast::<c_void>();
            if raw.is_null() || arr_len(raw) + 1 > arr_cap(raw) {
                array = stbds_arrgrowf(raw, size_of::<c_int>(), 1, 0).cast();
            }
            let index = (*header(array.cast())).length;
            *array.add(index) = inner;
            (*header(array.cast())).length += 1;
            inner = inner.wrapping_add(1);
        }
        if !array.is_null() {
            free(header(array.cast()).cast());
            array = null_mut();
        }
        outer = outer.wrapping_add(50);
    }
}
