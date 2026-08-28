#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

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
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn memcmp(left: *const c_void, right: *const c_void, len: usize) -> c_int;
    fn sprintf(dst: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn abort() -> !;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array.cast::<u8>().sub(size_of::<ArrayHeader>()).cast()
}

#[inline]
unsafe fn hash_table(raw_array: *mut c_void) -> *mut HashIndex {
    (*header(raw_array)).hash_table.cast()
}

#[inline]
unsafe fn array_to_hash(raw_array: *mut c_void, element_size: usize) -> *mut c_void {
    raw_array.cast::<u8>().add(element_size).cast()
}

#[inline]
unsafe fn hash_to_array(map: *mut c_void, element_size: usize) -> *mut c_void {
    map.cast::<u8>().sub(element_size).cast()
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
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

#[inline]
unsafe fn bucket_at(table: *mut HashIndex, slot: usize) -> *mut HashBucket {
    (*table).storage.add(slot >> BUCKET_SHIFT)
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

    let old_capacity = array_capacity(array);
    if min_capacity <= old_capacity {
        return array;
    }

    if min_capacity < old_capacity.wrapping_mul(2) {
        min_capacity = old_capacity.wrapping_mul(2);
    } else if min_capacity < 4 {
        min_capacity = 4;
    }

    let old_allocation = if array.is_null() {
        ptr::null_mut()
    } else {
        header(array).cast()
    };
    let allocation_size = element_size
        .wrapping_mul(min_capacity)
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

fn hash_index_log2(mut slot_count: usize) -> usize {
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
    let table = realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>();
    let storage_address = (table.add(1) as usize + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);

    (*table).storage = storage_address as *mut HashBucket;
    (*table).slot_count = slot_count;
    (*table).slot_count_log2 = hash_index_log2(slot_count);
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
        ptr::write_bytes(ptr::addr_of_mut!((*table).string), 0, 1);
        (*table).seed = HASH_SEED;
        HASH_SEED = HASH_SEED
            .wrapping_mul(0x27bb_2ee6_87b0_b0fd)
            .wrapping_add(0xb504_f32d);
    }

    for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = (*table).storage.add(bucket_index);
        for i in 0..BUCKET_LENGTH {
            (*bucket).hash[i] = HASH_EMPTY;
        }
        for i in 0..BUCKET_LENGTH {
            (*bucket).index[i] = INDEX_EMPTY;
        }
    }

    if !old.is_null() {
        (*table).used_count = (*old).used_count;
        for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
            let old_bucket = (*old).storage.add(old_bucket_index);
            for i in 0..BUCKET_LENGTH {
                if (*old_bucket).index[i] >= 0 {
                    let hash = (*old_bucket).hash[i];
                    let mut position = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = bucket_at(table, position);
                        for z in (position & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[z] == HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*old_bucket).index[i];
                                break 'probe;
                            }
                        }
                        for z in 0..(position & BUCKET_MASK) {
                            if (*bucket).hash[z] == HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*old_bucket).index[i];
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

unsafe fn siphash_bytes(data: *mut c_void, len: usize, seed: usize) -> usize {
    let mut bytes = data.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0;
    while offset + size_of::<usize>() <= len {
        let low = u32::from(*bytes)
            | (u32::from(*bytes.add(1)) << 8)
            | (u32::from(*bytes.add(2)) << 16)
            | (u32::from(*bytes.add(3)) << 24);
        let high = u32::from(*bytes.add(4))
            | (u32::from(*bytes.add(5)) << 8)
            | (u32::from(*bytes.add(6)) << 16)
            | (u32::from(*bytes.add(7)) << 24);
        // The C expressions promote bytes to signed int before byte 3 is
        // shifted. GCC sign-extends that 32-bit result when assigning size_t.
        let mut word = (low as i32 as isize) as usize;
        word |= (high as usize) << 32;
        v3 ^= word;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= word;
        offset += size_of::<usize>();
        bytes = bytes.add(size_of::<usize>());
    }

    let mut tail = len << (usize::BITS as usize - 8);
    let remaining = len - offset;
    for i in 4..remaining {
        tail |= (*bytes.add(i) as usize) << (i * 8);
    }
    if remaining >= 4 {
        let promoted = (u32::from(*bytes.add(3)) << 24) as i32;
        tail |= (promoted as isize) as usize;
    }
    if remaining >= 3 {
        tail |= (*bytes.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        tail |= (*bytes.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        tail |= *bytes as usize;
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

unsafe fn hm_find_slot(
    map: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = hash_to_array(map, element_size);
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
        let bucket = bucket_at(table, position);
        for i in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    map,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return INDEX_EMPTY;
            }
        }
        for i in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    map,
                    element_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((position & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
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
    mut map: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if map.is_null() {
        map = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
        (*header(map)).length += 1;
        ptr::write_bytes(map.cast::<u8>(), 0, element_size);
        *temp = INDEX_EMPTY;
        array_to_hash(map, element_size)
    } else {
        let raw_array = hash_to_array(map, element_size);
        let table = hash_table(raw_array);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(map, element_size, key, key_size, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = bucket_at(table, slot as usize);
                *temp = (*bucket).index[slot as usize & BUCKET_MASK];
            }
        }
        map
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    map: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0;
    let result = stbds_hmget_key_ts(map, element_size, key, key_size, &mut temp, mode);
    (*header(hash_to_array(result, element_size))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut map: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if map.is_null() || (*header(hash_to_array(map, element_size))).length == 0 {
        let raw_array = if map.is_null() {
            ptr::null_mut()
        } else {
            hash_to_array(map, element_size)
        };
        let grown = stbds_arrgrowf(raw_array, element_size, 0, 1);
        (*header(grown)).length += 1;
        ptr::write_bytes(grown.cast::<u8>(), 0, element_size);
        map = array_to_hash(grown, element_size);
    }
    map
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let duplicate = realloc(ptr::null_mut(), len).cast::<c_char>();
    ptr::copy(string, duplicate, len);
    duplicate
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut map: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if map.is_null() {
        let raw_array = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
        ptr::write_bytes(raw_array.cast::<u8>(), 0, element_size);
        (*header(raw_array)).length += 1;
        map = array_to_hash(raw_array, element_size);
    }

    let mut public_array = map;
    let mut raw_array = hash_to_array(map, element_size);
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
    let mut tombstone = INDEX_EMPTY;
    'search: loop {
        let bucket = bucket_at(table, position);
        for i in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    public_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[i];
                    if mode >= HM_STRING {
                        (*table).temp_key = *public_array
                            .cast::<u8>()
                            .add(element_size.wrapping_mul((*bucket).index[i] as usize))
                            .cast::<*mut c_char>();
                    }
                    return array_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + i) as isize;
            }
        }

        for i in 0..(position & BUCKET_MASK) {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    public_array,
                    element_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    (*header(raw_array)).temp = (*bucket).index[i];
                    return array_to_hash(raw_array, element_size);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + i) as isize;
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

    let bucket = bucket_at(table, position);
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
            let stored = key.cast::<c_char>();
            *destination.cast::<*mut c_char>() = stored;
            (*table).temp_key = stored;
        }
        _ => ptr::copy_nonoverlapping(key.cast::<u8>(), destination, key_size),
    }

    public_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let raw_array = stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1);
    ptr::write_bytes(raw_array.cast::<u8>(), 0, element_size);
    (*header(raw_array)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
    (*header(raw_array)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    array_to_hash(raw_array, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    map: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let raw_array = hash_to_array(map, element_size);
    let mut table = hash_table(raw_array);
    (*header(raw_array)).temp = 0;
    if table.is_null() {
        return map;
    }

    let mut slot = hm_find_slot(map, element_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return map;
    }

    let mut bucket = bucket_at(table, slot as usize);
    let mut bucket_index = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = array_len(raw_array) as isize - 2;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_array)).temp = 1;
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored = *map
            .cast::<u8>()
            .add(element_size.wrapping_mul(old_index as usize))
            .cast::<*mut c_char>();
        free(stored.cast());
    }

    if old_index != final_index {
        let destination = map
            .cast::<u8>()
            .add(element_size.wrapping_mul(old_index as usize));
        let source = map
            .cast::<u8>()
            .add(element_size.wrapping_mul(final_index as usize));
        ptr::copy(source, destination, element_size);

        let moved_key = destination.add(key_offset);
        slot = if mode == HM_STRING {
            hm_find_slot(
                map,
                element_size,
                *moved_key.cast::<*mut c_char>().cast(),
                key_size,
                key_offset,
                mode,
            )
        } else {
            hm_find_slot(
                map,
                element_size,
                moved_key.cast(),
                key_size,
                key_offset,
                mode,
            )
        };
        bucket = bucket_at(table, slot as usize);
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
        table = new_table;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = make_hash_index((*table).slot_count, table);
        (*header(raw_array)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    }
    let _ = table;

    map
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = strlen(string) + 1;
    if len > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << (usize::from((*arena).block) >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > block_size {
            let block =
                realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + len).cast::<StringBlock>();
            ptr::copy(string, ptr::addr_of_mut!((*block).storage).cast(), len);
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

        let block = realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + block_size)
            .cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    let result = ptr::addr_of_mut!((*(*arena).storage).storage)
        .cast::<c_char>()
        .add((*arena).remaining - len);
    (*arena).remaining -= len;
    ptr::copy(string, result, len);
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
    ptr::write_bytes(arena, 0, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(raw_array: *mut c_void, element_size: usize) {
    if raw_array.is_null() {
        return;
    }

    let table = hash_table(raw_array);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for i in 1..(*header(raw_array)).length {
                let string = *raw_array
                    .cast::<u8>()
                    .add(element_size.wrapping_mul(i))
                    .cast::<*mut c_char>();
                free(string.cast());
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*table).string));
    }
    free((*header(raw_array)).hash_table);
    free(header(raw_array).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    let buffer = ptr::addr_of_mut!(BUFFER).cast::<c_char>();
    sprintf(buffer, b"test_%d\0".as_ptr().cast(), number);
    buffer
}

#[inline]
unsafe fn require(condition: bool) {
    if !condition {
        abort();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(number: c_int) {
    let mut arena: StringArena = std::mem::zeroed();
    for i in 0..number {
        stbds_stralloc(&mut arena, strkey(i));
    }
    stbds_strreset(&mut arena);

    let source = StringEntry {
        key: b"a\0".as_ptr().cast_mut().cast(),
        value: number,
    };
    let element_size = size_of::<StringEntry>();
    let mut map = stbds_shmode_func(element_size, SH_ARENA as c_int);
    map = stbds_hmput_key(
        map,
        element_size,
        source.key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );

    let raw_array = hash_to_array(map, element_size);
    let index = (*header(raw_array)).temp as usize;
    let destination = map.cast::<StringEntry>().add(index);
    *destination = source;
    (*destination).key = (*hash_table(raw_array)).temp_key;

    require(*(*map.cast::<StringEntry>()).key == b'a' as c_char);
    require((*map.cast::<StringEntry>()).key != source.key);
    require((*map.cast::<StringEntry>()).value == source.value);

    let length = (*header(raw_array)).length as isize - 1;
    for i in 0..length {
        let entry = map.cast::<StringEntry>().offset(i);
        printf(b"%s %d\n\0".as_ptr().cast(), (*entry).key, (*entry).value);
    }

    stbds_hmfree_func(raw_array, element_size);
}
