#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{copy_nonoverlapping, null_mut};

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcmp(lhs: *const c_void, rhs: *const c_void, len: usize) -> c_int;
    fn memmove(dst: *mut c_void, src: *const c_void, len: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, value: c_int, len: usize) -> *mut c_void;
    fn strcmp(lhs: *const c_char, rhs: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn sprintf(dst: *mut c_char, format: *const c_char, ...) -> c_int;
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
#[derive(Copy, Clone)]
pub struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
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

static mut HASH_SEED: usize = 0x31415926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];
static STRKEY_FORMAT: &[u8] = b"test_%d\0";

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    (array as *mut ArrayHeader).sub(1)
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
    (hash as *mut u8).sub(element_size).cast()
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, element_size: usize) -> *mut c_void {
    (array as *mut u8).add(element_size).cast()
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    (*header(array)).hash_table.cast()
}

#[inline]
unsafe fn set_temp_key(array: *mut c_void, key: *mut c_char) {
    let table = (*header(array)).hash_table as *mut *mut c_char;
    *table = key;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    element_size: usize,
    add_len: usize,
    mut min_capacity: usize,
) -> *mut c_void {
    let min_len = array_len(array).wrapping_add(add_len);

    if min_len > min_capacity {
        min_capacity = min_len;
    }
    if min_capacity <= array_capacity(array) {
        return array;
    }
    if min_capacity < 2usize.wrapping_mul(array_capacity(array)) {
        min_capacity = 2usize.wrapping_mul(array_capacity(array));
    } else if min_capacity < 4 {
        min_capacity = 4;
    }

    let allocation = realloc(
        if array.is_null() {
            null_mut()
        } else {
            header(array).cast()
        },
        element_size
            .wrapping_mul(min_capacity)
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
    (*header(result)).capacity = min_capacity;
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

fn integer_log2(mut slot_count: usize) -> usize {
    let mut result = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        result += 1;
    }
    result
}

#[inline]
fn load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut result = v64_hi;
    result <<= 16;
    result <<= 16;
    result ^ temp ^ v32
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(null_mut(), allocation_size).cast::<HashIndex>();
    let after_table = table.add(1) as usize;
    (*table).storage =
        ((after_table + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)) as *mut HashBucket;
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
        copy_nonoverlapping(
            std::ptr::addr_of!((*old).string).cast::<u8>(),
            std::ptr::addr_of_mut!((*table).string).cast::<u8>(),
            size_of::<StringArena>(),
        );
        (*table).seed = (*old).seed;
    } else {
        memset(
            std::ptr::addr_of_mut!((*table).string).cast(),
            0,
            size_of::<StringArena>(),
        );
        (*table).seed = HASH_SEED;
        let a = load_32_or_64(2_147_001_325, 0x27bb2ee6, 0x87b0b0fd);
        let b = load_32_or_64(715_136_305, 0, 0xb504f32d);
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
            .wrapping_add(*(string as *mut u8) as usize);
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

unsafe fn siphash_bytes(pointer: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_pointer = pointer.cast::<u8>();
    let mut v0 = ((0x736f6d65usize << 32) + 0x70736575) ^ seed;
    let mut v1 = ((0x646f7261usize << 32) + 0x6e646f6d) ^ !seed;
    let mut v2 = ((0x6c796765usize << 32) + 0x6e657261) ^ seed;
    let mut v3 = ((0x74656462usize << 32) + 0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut consumed = 0;
    while consumed + size_of::<usize>() <= len {
        let low_word = *data_pointer as u32
            | ((*data_pointer.add(1) as u32) << 8)
            | ((*data_pointer.add(2) as u32) << 16)
            | ((*data_pointer.add(3) as u32) << 24);
        let mut data = low_word as i32 as isize as usize;
        data |= ((*data_pointer.add(4) as usize)
            | ((*data_pointer.add(5) as usize) << 8)
            | ((*data_pointer.add(6) as usize) << 16)
            | ((*data_pointer.add(7) as usize) << 24))
            << 32;

        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        consumed += size_of::<usize>();
        data_pointer = data_pointer.add(size_of::<usize>());
    }

    let remaining = len - consumed;
    let mut data = len << (usize::BITS as usize - 8);
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
        let low_word = *data_pointer as u32
            | ((*data_pointer.add(1) as u32) << 8)
            | ((*data_pointer.add(2) as u32) << 16)
            | ((*data_pointer.add(3) as u32) << 24);
        data |= low_word as i32 as isize as usize;
    } else if remaining >= 3 {
        data |= (*data_pointer.add(2) as usize) << 16;
        data |= (*data_pointer.add(1) as usize) << 8;
        data |= *data_pointer as usize;
    } else if remaining >= 2 {
        data |= (*data_pointer.add(1) as usize) << 8;
        data |= *data_pointer as usize;
    } else if remaining >= 1 {
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
pub unsafe extern "C" fn stbds_hash_bytes(pointer: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(pointer, len, seed)
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
    let element_key = (array as *mut u8)
        .add(element_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *(element_key as *mut *mut c_char)) == 0
    } else {
        memcmp(key, element_key.cast(), key_size) == 0
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
                let key = *(array.cast::<u8>().add(element_size * index) as *mut *mut c_void);
                free(key);
            }
        }
        stbds_strreset(std::ptr::addr_of_mut!((*table).string));
    }
    free((*header(array)).hash_table);
    free(header(array).cast());
}

unsafe fn hm_find_slot(
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
        let limit = position & BUCKET_MASK;
        for index in limit..BUCKET_LENGTH {
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
        for index in 0..limit {
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
    mut array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if array.is_null() {
        array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        (*header(array)).length += 1;
        memset(array, 0, element_size);
        *temp = INDEX_EMPTY;
        array_to_hash(array, element_size)
    } else {
        let raw_array = hash_to_array(array, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(array, element_size, key, key_size, 0, mode);
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
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0;
    let result = stbds_hmget_key_ts(array, element_size, key, key_size, &mut temp, mode);
    (*header(hash_to_array(result, element_size))).temp = temp;
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
        memset(array, 0, element_size);
        array = array_to_hash(array, element_size);
    }
    array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let copy = realloc(null_mut(), len).cast::<c_char>();
    memmove(copy.cast(), string.cast(), len);
    copy
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
        array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
        memset(array, 0, element_size);
        (*header(array)).length += 1;
        array = array_to_hash(array, element_size);
    }

    let mut public_array = array;
    let mut raw_array = hash_to_array(array, element_size);
    let mut table = hash_table(raw_array);

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
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
    let mut tombstone = INDEX_EMPTY;

    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        let limit = position & BUCKET_MASK;
        for index in limit..BUCKET_LENGTH {
            if (*bucket).hash[index] == hash {
                if is_key_equal(
                    public_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[index];
                    if mode >= HM_STRING {
                        let existing_key = *(public_array
                            .cast::<u8>()
                            .add(element_size * (*bucket).index[index] as usize)
                            as *mut *mut c_char);
                        set_temp_key(raw_array, existing_key);
                    }
                    return array_to_hash(raw_array, element_size);
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
                if is_key_equal(
                    public_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[index] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[index];
                    return array_to_hash(raw_array, element_size);
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

    let index = array_len(raw_array) as isize;
    if index as usize + 1 > array_capacity(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, element_size, 1, 0);
    }
    public_array = array_to_hash(raw_array, element_size);
    (*header(raw_array)).length = index as usize + 1;
    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = index - 1;
    (*header(raw_array)).temp = index - 1;

    let destination = raw_array.cast::<u8>().add(element_size * index as usize);
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            *(destination as *mut *mut c_char) = stored;
            set_temp_key(raw_array, stored);
        }
        SH_ARENA => {
            let stored = stbds_stralloc(std::ptr::addr_of_mut!((*table).string), key.cast());
            *(destination as *mut *mut c_char) = stored;
            set_temp_key(raw_array, stored);
        }
        SH_DEFAULT => {
            *(destination as *mut *mut c_char) = key.cast();
            set_temp_key(raw_array, key.cast());
        }
        _ => {
            copy_nonoverlapping(key.cast::<u8>(), destination, key_size);
        }
    }
    public_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = stbds_arrgrowf(null_mut(), element_size, 0, 1);
    memset(array, 0, element_size);
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
    let table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return array;
    }

    let mut slot = hm_find_slot(array, element_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return array;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_index = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = array_len(raw_array) as isize - 2;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored =
            *(array.cast::<u8>().add(element_size * old_index as usize) as *mut *mut c_void);
        free(stored);
    }

    if old_index != final_index {
        let destination = array.cast::<u8>().add(element_size * old_index as usize);
        let source = array.cast::<u8>().add(element_size * final_index as usize);
        memmove(destination.cast(), source.cast(), element_size);

        let moved_key = if mode == HM_STRING {
            *(destination.add(key_offset) as *mut *mut c_char) as *mut c_void
        } else {
            destination.add(key_offset).cast()
        };
        slot = hm_find_slot(array, element_size, moved_key, key_size, key_offset, mode);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_index = slot as usize & BUCKET_MASK;
        (*bucket).index[bucket_index] = old_index;
    }
    (*header(raw_array)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let new_table = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_array)).hash_table = new_table.cast();
        free(table.cast());
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = make_hash_index((*table).slot_count, table);
        (*header(raw_array)).hash_table = new_table.cast();
        free(table.cast());
    }
    array
}

const STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = strlen(string) + 1;
    if len > (*arena).remaining {
        let mut block_size = (*arena).block as usize;
        block_size = STRING_ARENA_BLOCKSIZE_MIN << (block_size >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > block_size {
            let block =
                realloc(null_mut(), size_of::<StringBlock>() - 8 + len).cast::<StringBlock>();
            memmove(
                std::ptr::addr_of_mut!((*block).storage).cast(),
                string.cast(),
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
            return std::ptr::addr_of_mut!((*block).storage).cast();
        }

        let block =
            realloc(null_mut(), size_of::<StringBlock>() - 8 + block_size).cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let result = std::ptr::addr_of_mut!((*(*arena).storage).storage)
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
    let buffer = std::ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    sprintf(buffer, STRKEY_FORMAT.as_ptr().cast(), number);
    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(number: c_int) {
    for index in 0..4usize {
        let mut array: *mut c_int = null_mut();

        for value in [number, 2, 3, 4] {
            array = stbds_arrgrowf(array.cast(), size_of::<c_int>(), 1, 0).cast();
            let length = (*header(array.cast())).length;
            *array.add(length) = value;
            (*header(array.cast())).length = length + 1;
        }
        memmove(
            array.add(index).cast(),
            array.add(index + 1).cast(),
            size_of::<c_int>() * ((*header(array.cast())).length - 1 - index),
        );
        (*header(array.cast())).length -= 1;
        free(header(array.cast()).cast());

        array = null_mut();
        for value in [number, 2, 3, 4] {
            array = stbds_arrgrowf(array.cast(), size_of::<c_int>(), 1, 0).cast();
            let length = (*header(array.cast())).length;
            *array.add(length) = value;
            (*header(array.cast())).length = length + 1;
        }
        *array.add(index) = *array.add((*header(array.cast())).length - 1);
        (*header(array.cast())).length -= 1;
        free(header(array.cast()).cast());
    }
}
