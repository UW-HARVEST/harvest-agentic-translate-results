#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{addr_of_mut, null_mut};

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> c_int;
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
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

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    (array as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader
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
unsafe fn hash_to_arr(hash: *mut c_void, elem_size: usize) -> *mut c_void {
    (hash as *mut u8).sub(elem_size) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(array: *mut c_void, elem_size: usize) -> *mut c_void {
    (array as *mut u8).add(elem_size) as *mut c_void
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    (*header(array)).hash_table as *mut HashIndex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    elem_size: usize,
    add_len: usize,
    mut min_cap: usize,
) -> *mut c_void {
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

    let allocation_size = elem_size
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<ArrayHeader>());
    let old = if array.is_null() {
        null_mut()
    } else {
        header(array) as *mut c_void
    };
    let allocation = realloc(old, allocation_size);
    let result = (allocation as *mut u8).add(size_of::<ArrayHeader>()) as *mut c_void;

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
    free(header(array) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    HASH_SEED = seed;
}

#[inline]
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

#[inline]
fn log2(mut slot_count: usize) -> usize {
    let mut result = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        result += 1;
    }
    result
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let bucket_bytes = (slot_count >> BUCKET_SHIFT).wrapping_mul(size_of::<HashBucket>());
    let allocation_size = bucket_bytes
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(null_mut(), allocation_size) as *mut HashIndex;
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
            for old_index in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_index] >= 0 {
                    let value_hash = (*old_bucket).hash[old_index];
                    let mut position = value_hash & (slot_count - 1);
                    let mut step = BUCKET_LENGTH;

                    'probe: loop {
                        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[index] == HASH_EMPTY {
                                (*bucket).hash[index] = value_hash;
                                (*bucket).index[index] = (*old_bucket).index[old_index];
                                break 'probe;
                            }
                        }
                        for index in 0..(position & BUCKET_MASK) {
                            if (*bucket).hash[index] == HASH_EMPTY {
                                (*bucket).hash[index] = value_hash;
                                (*bucket).index[index] = (*old_bucket).index[old_index];
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut value: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while *value != 0 {
        hash = hash.rotate_left(9).wrapping_add(*value as u8 as usize);
        value = value.add(1);
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
fn sip_round(values: &mut [usize; 4]) {
    values[0] = values[0].wrapping_add(values[1]);
    values[1] = values[1].rotate_left(13);
    values[1] ^= values[0];
    values[0] = values[0].rotate_left(usize::BITS / 2);
    values[2] = values[2].wrapping_add(values[3]);
    values[3] = values[3].rotate_left(16);
    values[3] ^= values[2];
    values[2] = values[2].wrapping_add(values[1]);
    values[1] = values[1].rotate_left(17);
    values[1] ^= values[2];
    values[2] = values[2].rotate_left(usize::BITS / 2);
    values[0] = values[0].wrapping_add(values[3]);
    values[3] = values[3].rotate_left(21);
    values[3] ^= values[0];
}

unsafe fn siphash_bytes(pointer: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_pointer = pointer as *const u8;
    let mut values = [
        0x736f_6d65_7073_6575usize ^ seed,
        0x646f_7261_6e64_6f6dusize ^ !seed,
        0x6c79_6765_6e65_7261usize ^ seed,
        0x7465_6462_7974_6573usize ^ !seed,
    ];
    values[0] ^= 0x0706_0504_0302_0100usize ^ seed;
    values[1] ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    values[2] ^= 0x0706_0504_0302_0100usize ^ seed;
    values[3] ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0;
    while offset + size_of::<usize>() <= len {
        let low = (*data_pointer as i32)
            | ((*data_pointer.add(1) as i32) << 8)
            | ((*data_pointer.add(2) as i32) << 16)
            | ((*data_pointer.add(3) as i32) << 24);
        let high = (*data_pointer.add(4) as usize)
            | ((*data_pointer.add(5) as usize) << 8)
            | ((*data_pointer.add(6) as usize) << 16)
            | ((*data_pointer.add(7) as usize) << 24);
        let data = (low as usize) | (high << 32);
        values[3] ^= data;
        sip_round(&mut values);
        sip_round(&mut values);
        values[0] ^= data;
        offset += size_of::<usize>();
        data_pointer = data_pointer.add(size_of::<usize>());
    }

    let mut data = len.wrapping_shl(usize::BITS - 8);
    let remaining = len - offset;
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
    values[3] ^= data;
    sip_round(&mut values);
    sip_round(&mut values);
    values[0] ^= data;
    values[2] ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut values);
    }
    values[0] ^ values[1] ^ values[2] ^ values[3]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(pointer: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(pointer, len, seed)
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
    let stored_key = (array as *mut u8)
        .add(elem_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key as *const c_char, *(stored_key as *mut *mut c_char)) == 0
    } else {
        memcmp(key, stored_key as *const c_void, key_size) == 0
    }
}

unsafe fn hm_find_slot(
    array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = hash_to_arr(array, elem_size);
    let table = hash_table(raw_array);
    let mut value_hash = if mode >= HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    if value_hash < 2 {
        value_hash += 2;
    }

    let mut position = value_hash & ((*table).slot_count - 1);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[index] == value_hash {
                if is_key_equal(
                    array,
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
            if (*bucket).hash[index] == value_hash {
                if is_key_equal(
                    array,
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

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        array = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
        (*header(array)).length += 1;
        memset(array, 0, elem_size);
        *temp = INDEX_EMPTY;
        arr_to_hash(array, elem_size)
    } else {
        let raw_array = hash_to_arr(array, elem_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(array, elem_size, key, key_size, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                *temp = (*bucket).index[slot as usize & BUCKET_MASK];
            }
        }
        array
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
    let result = stbds_hmget_key_ts(array, elem_size, key, key_size, &mut temp, mode);
    (*header(hash_to_arr(result, elem_size))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut array: *mut c_void,
    elem_size: usize,
) -> *mut c_void {
    if array.is_null() || (*header(hash_to_arr(array, elem_size))).length == 0 {
        let raw_array = if array.is_null() {
            null_mut()
        } else {
            hash_to_arr(array, elem_size)
        };
        let grown = stbds_arrgrowf(raw_array, elem_size, 0, 1);
        (*header(grown)).length += 1;
        memset(grown, 0, elem_size);
        array = arr_to_hash(grown, elem_size);
    }
    array
}

unsafe fn duplicate_string(value: *mut c_char) -> *mut c_char {
    let len = strlen(value) + 1;
    let result = realloc(null_mut(), len) as *mut c_char;
    memmove(result as *mut c_void, value as *const c_void, len);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        let raw_array = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
        memset(raw_array, 0, elem_size);
        (*header(raw_array)).length += 1;
        array = arr_to_hash(raw_array, elem_size);
    }

    let raw_hash_array = array;
    let mut raw_array = hash_to_arr(array, elem_size);
    let mut table = hash_table(raw_array);

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
        (*header(raw_array)).hash_table = new_table as *mut c_void;
        table = new_table;
    }

    let mut value_hash = if mode >= HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    if value_hash < 2 {
        value_hash += 2;
    }

    let mut position = value_hash & ((*table).slot_count - 1);
    let mut step = BUCKET_LENGTH;
    let mut tombstone = -1isize;

    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for index in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[index] == value_hash {
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
                        (*table).temp_key = *((raw_hash_array as *mut u8)
                            .add(elem_size * (*bucket).index[index] as usize)
                            as *mut *mut c_char);
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
            if (*bucket).hash[index] == value_hash {
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

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= (*table).slot_count - 1;
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let index = arr_len(raw_array) as isize;
    if index as usize + 1 > arr_cap(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, elem_size, 1, 0);
    }
    (*header(raw_array)).length = index as usize + 1;

    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = value_hash;
    (*bucket).index[position & BUCKET_MASK] = index - 1;
    (*header(raw_array)).temp = index - 1;

    let destination = (raw_array as *mut u8).add(elem_size * index as usize);
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key as *mut c_char);
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        SH_DEFAULT => {
            let stored = key as *mut c_char;
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        _ => {
            memcpy(destination as *mut c_void, key as *const c_void, key_size);
        }
    }
    arr_to_hash(raw_array, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let array = stbds_arrgrowf(null_mut(), elem_size, 0, 1);
    memset(array, 0, elem_size);
    (*header(array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, null_mut());
    (*header(array)).hash_table = table as *mut c_void;
    (*table).string.mode = mode as u8;
    arr_to_hash(array, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        return null_mut();
    }

    let raw_array = hash_to_arr(array, elem_size);
    let table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return array;
    }

    let mut slot = hm_find_slot(array, elem_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return array;
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
        let stored = *((array as *mut u8).add(elem_size * old_index as usize) as *mut *mut c_char);
        free(stored as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (array as *mut u8).add(elem_size * old_index as usize) as *mut c_void,
            (array as *mut u8).add(elem_size * final_index as usize) as *const c_void,
            elem_size,
        );

        let moved_key = (array as *mut u8)
            .add(elem_size * old_index as usize)
            .add(key_offset);
        slot = if mode == HM_STRING {
            hm_find_slot(
                array,
                elem_size,
                *(moved_key as *mut *mut c_char) as *mut c_void,
                key_size,
                key_offset,
                mode,
            )
        } else {
            hm_find_slot(
                array,
                elem_size,
                moved_key as *mut c_void,
                key_size,
                key_offset,
                mode,
            )
        };
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_index = slot as usize & BUCKET_MASK;
        (*bucket).index[bucket_index] = old_index;
    }
    (*header(raw_array)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let new_table = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_array)).hash_table = new_table as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = make_hash_index((*table).slot_count, table);
        (*header(raw_array)).hash_table = new_table as *mut c_void;
        free(table as *mut c_void);
    }
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    value: *mut c_char,
) -> *mut c_char {
    const BLOCK_SIZE_MIN: usize = 512;
    const BLOCK_SIZE_MAX: usize = 1 << 20;

    let len = strlen(value) + 1;
    if len > (*arena).remaining {
        let block_size = BLOCK_SIZE_MIN << ((*arena).block >> 1);
        if block_size < BLOCK_SIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > block_size {
            let allocation_size = size_of::<StringBlock>() - 8 + len;
            let block = realloc(null_mut(), allocation_size) as *mut StringBlock;
            memmove(
                (*block).storage.as_mut_ptr() as *mut c_void,
                value as *const c_void,
                len,
            );
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

        let allocation_size = size_of::<StringBlock>() - 8 + block_size;
        let block = realloc(null_mut(), allocation_size) as *mut StringBlock;
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let result = (*(*arena).storage)
        .storage
        .as_mut_ptr()
        .add((*arena).remaining - len);
    (*arena).remaining -= len;
    memmove(result as *mut c_void, value as *const c_void, len);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut block = (*arena).storage;
    while !block.is_null() {
        let next = (*block).next;
        free(block as *mut c_void);
        block = next;
    }
    memset(arena as *mut c_void, 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, elem_size: usize) {
    if array.is_null() {
        return;
    }
    let table = hash_table(array);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for index in 1..(*header(array)).length {
                let stored = *((array as *mut u8).add(elem_size * index) as *mut *mut c_char);
                free(stored as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*header(array)).hash_table);
    free(header(array) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = addr_of_mut!(STRKEY_BUFFER) as *mut c_char;
    let prefix = b"test_";
    for (index, byte) in prefix.iter().enumerate() {
        *buffer.add(index) = *byte as c_char;
    }

    let mut position = prefix.len();
    let negative = number < 0;
    let mut magnitude = if negative {
        (-(number as i64)) as u64
    } else {
        number as u64
    };
    if negative {
        *buffer.add(position) = b'-' as c_char;
        position += 1;
    }

    let digits_start = position;
    loop {
        *buffer.add(position) = (b'0' + (magnitude % 10) as u8) as c_char;
        position += 1;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }
    let mut left = digits_start;
    let mut right = position - 1;
    while left < right {
        let temp = *buffer.add(left);
        *buffer.add(left) = *buffer.add(right);
        *buffer.add(right) = temp;
        left += 1;
        right -= 1;
    }
    *buffer.add(position) = 0;
    buffer
}

#[repr(C)]
struct IntEntry {
    key: c_int,
    value: c_int,
}

unsafe fn put_int(map: &mut *mut IntEntry, key: c_int, value: c_int) {
    let mut key_copy = key;
    *map = stbds_hmput_key(
        *map as *mut c_void,
        size_of::<IntEntry>(),
        &mut key_copy as *mut c_int as *mut c_void,
        size_of::<c_int>(),
        0,
    ) as *mut IntEntry;
    let raw = hash_to_arr(*map as *mut c_void, size_of::<IntEntry>());
    let index = (*header(raw)).temp;
    (*map.offset(index)).key = key;
    (*map.offset(index)).value = value;
}

unsafe fn delete_int(map: &mut *mut IntEntry, key: c_int) {
    let mut key_copy = key;
    *map = stbds_hmdel_key(
        *map as *mut c_void,
        size_of::<IntEntry>(),
        &mut key_copy as *mut c_int as *mut c_void,
        size_of::<c_int>(),
        0,
        0,
    ) as *mut IntEntry;
}

unsafe fn free_int_map(map: &mut *mut IntEntry) {
    if !(*map).is_null() {
        stbds_hmfree_func(
            hash_to_arr(*map as *mut c_void, size_of::<IntEntry>()),
            size_of::<IntEntry>(),
        );
        *map = null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    let mut map: *mut IntEntry = null_mut();
    map = stbds_hmput_default(map as *mut c_void, size_of::<IntEntry>()) as *mut IntEntry;
    (*map.offset(-1)).value = -2;

    let mut index = 0;
    while index < num {
        put_int(&mut map, index, index.wrapping_mul(5));
        index = index.wrapping_add(2);
    }
    index = 0;
    while index < num {
        put_int(&mut map, index, index.wrapping_mul(3));
        index = index.wrapping_add(2);
    }
    index = 2;
    while index < num {
        delete_int(&mut map, index);
        index = index.wrapping_add(4);
    }
    index = 0;
    while index < num {
        delete_int(&mut map, index);
        index = index.wrapping_add(1);
    }
    free_int_map(&mut map);

    index = 0;
    while index < num {
        put_int(&mut map, index, index.wrapping_mul(3));
        index = index.wrapping_add(2);
    }
    free_int_map(&mut map);
}
