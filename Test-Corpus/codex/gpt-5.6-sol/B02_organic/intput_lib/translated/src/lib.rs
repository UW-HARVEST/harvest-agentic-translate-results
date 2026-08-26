#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memcmp(lhs: *const c_void, rhs: *const c_void, count: usize) -> c_int;
    fn memmove(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
    fn strcmp(lhs: *const c_char, rhs: *const c_char) -> c_int;
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

#[derive(Clone, Copy)]
#[repr(C)]
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

#[inline]
unsafe fn element(base: *mut c_void, element_size: usize, index: usize) -> *mut u8 {
    base.cast::<u8>().add(element_size.wrapping_mul(index))
}

#[inline]
fn probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
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

    let allocation = realloc(
        if array.is_null() {
            ptr::null_mut()
        } else {
            header(array).cast()
        },
        element_size
            .wrapping_mul(capacity)
            .wrapping_add(size_of::<ArrayHeader>()),
    );
    let grown = allocation
        .cast::<u8>()
        .add(size_of::<ArrayHeader>())
        .cast::<c_void>();

    if array.is_null() {
        (*header(grown)).length = 0;
        (*header(grown)).hash_table = ptr::null_mut();
        (*header(grown)).temp = 0;
    }
    (*header(grown)).capacity = capacity;
    grown
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(array: *mut c_void) {
    free(header(array).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    HASH_SEED = seed;
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>();
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

    if !old.is_null() {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
    } else {
        ptr::write_bytes(
            ptr::addr_of_mut!((*table).string).cast::<u8>(),
            0,
            size_of::<StbdsStringArena>(),
        );
        (*table).seed = HASH_SEED;

        let multiplier = if usize::BITS == 64 {
            0x27bb_2ee6_87b0_b0fdusize
        } else {
            2_147_001_325usize
        };
        let increment = if usize::BITS == 64 {
            0x0000_0000_b504_f32dusize
        } else {
            715_136_305usize
        };
        HASH_SEED = HASH_SEED
            .wrapping_mul(multiplier)
            .wrapping_add(increment);
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
                    let mut position =
                        probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
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
pub unsafe extern "C" fn stbds_hash_string(mut value: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while *value != 0 {
        hash = hash
            .rotate_left(9)
            .wrapping_add(*value.cast::<u8>() as usize);
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
        let low_word = (*data_bytes.add(0) as u32)
            | ((*data_bytes.add(1) as u32) << 8)
            | ((*data_bytes.add(2) as u32) << 16)
            | ((*data_bytes.add(3) as u32) << 24);
        // C evaluates this word as a signed int before converting it to size_t.
        let mut data = (low_word as i32 as isize) as usize;
        data |= ((*data_bytes.add(4) as usize)
            | ((*data_bytes.add(5) as usize) << 8)
            | ((*data_bytes.add(6) as usize) << 16)
            | ((*data_bytes.add(7) as usize) << 24))
            << 32;

        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        offset += size_of::<usize>();
        data_bytes = data_bytes.add(size_of::<usize>());
    }

    let mut data = length << (usize::BITS - 8);
    let remaining = length - offset;
    if remaining >= 7 {
        data |= (*data_bytes.add(6) as usize) << 48;
    }
    if remaining >= 6 {
        data |= (*data_bytes.add(5) as usize) << 40;
    }
    if remaining >= 5 {
        data |= (*data_bytes.add(4) as usize) << 32;
    }
    if remaining >= 4 {
        let signed_word = ((*data_bytes.add(3) as u32) << 24) as i32;
        data |= (signed_word as isize) as usize;
    }
    if remaining >= 3 {
        data |= (*data_bytes.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        data |= (*data_bytes.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        data |= *data_bytes as usize;
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
    data: *mut c_void,
    length: usize,
    seed: usize,
) -> usize {
    siphash_bytes(data, length, seed)
}

unsafe fn keys_equal(
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored_key = element(hash_array, element_size, index).add(key_offset);
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
                free(*element(array, element_size, index).cast::<*mut c_void>());
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
    let raw_array = hash_to_array(hash_array, element_size);
    let table = hash_table(raw_array);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    let mut step = BUCKET_LENGTH;
    if hash < 2 {
        hash += 2;
    }
    let mut position = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                if keys_equal(
                    hash_array,
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

        let limit = position & BUCKET_MASK;
        for slot in 0..limit {
            if (*bucket).hash[slot] == hash {
                if keys_equal(
                    hash_array,
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
    mut hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    temporary: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        let raw_array = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
        (*header(raw_array)).length += 1;
        memset(raw_array, 0, element_size);
        *temporary = INDEX_EMPTY;
        hash_array = array_to_hash(raw_array, element_size);
    } else {
        let raw_array = hash_to_array(hash_array, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temporary = INDEX_EMPTY;
        } else {
            let slot = find_slot(hash_array, element_size, key, key_size, 0, mode);
            if slot < 0 {
                *temporary = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
                *temporary = (*bucket).index[(slot as usize) & BUCKET_MASK];
            }
        }
    }
    hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temporary = 0;
    let result = stbds_hmget_key_ts(
        hash_array,
        element_size,
        key,
        key_size,
        &mut temporary,
        mode,
    );
    (*header(hash_to_array(result, element_size))).temp = temporary;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut hash_array: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if hash_array.is_null()
        || (*header(hash_to_array(hash_array, element_size))).length == 0
    {
        let raw_array = stbds_arrgrowf(
            if hash_array.is_null() {
                ptr::null_mut()
            } else {
                hash_to_array(hash_array, element_size)
            },
            element_size,
            0,
            1,
        );
        (*header(raw_array)).length += 1;
        memset(raw_array, 0, element_size);
        hash_array = array_to_hash(raw_array, element_size);
    }
    hash_array
}

unsafe fn duplicate_string(value: *mut c_char) -> *mut c_char {
    let length = strlen(value) + 1;
    let duplicate = realloc(ptr::null_mut(), length).cast::<c_char>();
    memmove(duplicate.cast(), value.cast(), length);
    duplicate
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
        let raw_array = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
        memset(raw_array, 0, element_size);
        (*header(raw_array)).length += 1;
        hash_array = array_to_hash(raw_array, element_size);
    }

    let raw_hash_array = hash_array;
    let mut raw_array = hash_to_array(hash_array, element_size);
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

    let mut step = BUCKET_LENGTH;
    let mut position = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    let mut tombstone: isize = -1;

    'search: loop {
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[slot] == hash {
                if keys_equal(
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
                        (*table).temp_key =
                            *element(raw_hash_array, element_size, (*bucket).index[slot] as usize)
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

        let limit = position & BUCKET_MASK;
        for slot in 0..limit {
            if (*bucket).hash[slot] == hash {
                if keys_equal(
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

    let insertion_index = array_len(raw_array) as isize;
    if (insertion_index as usize) + 1 > array_capacity(raw_array) {
        raw_array = stbds_arrgrowf(raw_array, element_size, 1, 0);
    }
    (*header(raw_array)).length = insertion_index as usize + 1;

    let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
    (*bucket).hash[position & BUCKET_MASK] = hash;
    (*bucket).index[position & BUCKET_MASK] = insertion_index - 1;
    (*header(raw_array)).temp = insertion_index - 1;

    let destination = element(raw_array, element_size, insertion_index as usize);
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            (*table).temp_key = stored;
            *destination.cast::<*mut c_char>() = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key.cast());
            (*table).temp_key = stored;
            *destination.cast::<*mut c_char>() = stored;
        }
        SH_DEFAULT => {
            (*table).temp_key = key.cast();
            *destination.cast::<*mut c_char>() = key.cast();
        }
        _ => {
            memcpy(destination.cast(), key, key_size);
        }
    }

    array_to_hash(raw_array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(
    element_size: usize,
    mode: c_int,
) -> *mut c_void {
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
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        return ptr::null_mut();
    }

    let raw_array = hash_to_array(hash_array, element_size);
    let mut table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return hash_array;
    }

    let mut slot = find_slot(
        hash_array,
        element_size,
        key,
        key_size,
        key_offset,
        mode,
    );
    if slot < 0 {
        return hash_array;
    }

    let mut bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
    let mut bucket_slot = (slot as usize) & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_slot];
    let final_index = array_len(raw_array) as isize - 2;

    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_slot] = HASH_DELETED;
    (*bucket).index[bucket_slot] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        free(
            *element(hash_array, element_size, old_index as usize).cast::<*mut c_void>(),
        );
    }

    if old_index != final_index {
        memmove(
            element(hash_array, element_size, old_index as usize).cast(),
            element(hash_array, element_size, final_index as usize).cast(),
            element_size,
        );

        let moved_key = if mode == HM_STRING {
            (*element(hash_array, element_size, old_index as usize)
                .add(key_offset)
                .cast::<*mut c_char>())
            .cast()
        } else {
            element(hash_array, element_size, old_index as usize)
                .add(key_offset)
                .cast()
        };
        slot = find_slot(
            hash_array,
            element_size,
            moved_key,
            key_size,
            key_offset,
            mode,
        );
        bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
        bucket_slot = (slot as usize) & BUCKET_MASK;
        (*bucket).index[bucket_slot] = old_index;
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
    value: *mut c_char,
) -> *mut c_char {
    let length = strlen(value) + 1;
    if length > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << (((*arena).block as usize) >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block += 1;
        }

        if length > block_size {
            let block = realloc(
                ptr::null_mut(),
                size_of::<StringBlock>() - 8 + length,
            )
            .cast::<StringBlock>();
            memmove((*block).storage.as_mut_ptr().cast(), value.cast(), length);
            if !(*arena).storage.is_null() {
                (*block).next = (*(*arena).storage).next;
                (*(*arena).storage).next = block;
            } else {
                (*block).next = ptr::null_mut();
                (*arena).storage = block;
                (*arena).remaining = 0;
            }
            return (*block).storage.as_mut_ptr();
        }

        let block = realloc(
            ptr::null_mut(),
            size_of::<StringBlock>() - 8 + block_size,
        )
        .cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let destination = (*(*arena).storage)
        .storage
        .as_mut_ptr()
        .add((*arena).remaining - length);
    (*arena).remaining -= length;
    memmove(destination.cast(), value.cast(), length);
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
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    sprintf(buffer, STRKEY_FORMAT.as_ptr().cast(), number);
    buffer
}

#[repr(C)]
struct IntPair {
    key: c_int,
    value: c_int,
}

unsafe fn put_int(mut map: *mut IntPair, key: c_int, value: c_int) -> *mut IntPair {
    map = stbds_hmput_key(
        map.cast(),
        size_of::<IntPair>(),
        ptr::addr_of!(key).cast_mut().cast(),
        size_of::<c_int>(),
        0,
    )
    .cast();
    let raw_array = map.sub(1);
    let index = (*header(raw_array.cast())).temp as usize;
    (*map.add(index)).key = key;
    (*map.add(index)).value = value;
    map
}

unsafe fn get_int(mut map: *mut IntPair, key: c_int) -> (*mut IntPair, c_int) {
    map = stbds_hmget_key(
        map.cast(),
        size_of::<IntPair>(),
        ptr::addr_of!(key).cast_mut().cast(),
        size_of::<c_int>(),
        0,
    )
    .cast();
    let raw_array = map.sub(1);
    let index = (*header(raw_array.cast())).temp;
    (map, (*map.offset(index)).value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(number: c_int) {
    let mut map = ptr::null_mut();
    map = put_int(map, number, 7);
    map = put_int(map, 11, 3);
    map = put_int(map, 9, number);
    if cfg!(debug_assertions) {
        let mut value;
        (map, value) = get_int(map, 9);
        debug_assert!(value == number);
        (map, value) = get_int(map, 11);
        debug_assert!(value == 3);
        (map, value) = get_int(map, number);
        debug_assert!(value == 7);
    }
    let _ = map;
}
