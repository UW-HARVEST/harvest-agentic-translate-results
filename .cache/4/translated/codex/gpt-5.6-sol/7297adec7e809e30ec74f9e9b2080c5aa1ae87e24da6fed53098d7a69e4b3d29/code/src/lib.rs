#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn memcmp(lhs: *const c_void, rhs: *const c_void, count: usize) -> c_int;
    fn memcpy(dst: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
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

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(data: *mut c_void) -> *mut ArrayHeader {
    data.cast::<ArrayHeader>().wrapping_sub(1)
}

#[inline]
unsafe fn array_len(data: *mut c_void) -> isize {
    if data.is_null() {
        0
    } else {
        unsafe { (*header(data)).length as isize }
    }
}

#[inline]
unsafe fn array_cap(data: *mut c_void) -> usize {
    if data.is_null() {
        0
    } else {
        unsafe { (*header(data)).capacity }
    }
}

#[inline]
unsafe fn hash_to_array(hash: *mut c_void, elem_size: usize) -> *mut c_void {
    hash.cast::<u8>().wrapping_sub(elem_size).cast()
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, elem_size: usize) -> *mut c_void {
    array.cast::<u8>().wrapping_add(elem_size).cast()
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    unsafe { (*header(array)).hash_table.cast() }
}

#[inline]
unsafe fn arena_storage(block: *mut StringBlock) -> *mut c_char {
    unsafe { ptr::addr_of_mut!((*block).storage).cast() }
}

#[inline]
fn probe_position(hash: usize, slot_count: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
}

fn integer_log2(mut slot_count: usize) -> usize {
    let mut result = 0usize;
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
    let table = unsafe { realloc(ptr::null_mut(), allocation_size) }.cast::<HashIndex>();
    let after_table = table.wrapping_add(1) as usize;
    let aligned = after_table.wrapping_add(CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);

    unsafe {
        (*table).storage = aligned as *mut HashBucket;
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

        if !old.is_null() {
            (*table).string = (*old).string;
            (*table).seed = (*old).seed;
        } else {
            (*table).string = StringArena {
                storage: ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            (*table).seed = HASH_SEED;

            let a = if size_of::<usize>() == 8 {
                0x27bb_2ee6_87b0_b0fdusize
            } else {
                2_147_001_325usize
            };
            let b = if size_of::<usize>() == 8 {
                0x0000_0000_b504_f32dusize
            } else {
                715_136_305usize
            };
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
            for bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
                let old_bucket = (*old).storage.add(bucket_index);
                for item in 0..BUCKET_LENGTH {
                    if (*old_bucket).index[item] >= 0 {
                        let item_hash = (*old_bucket).hash[item];
                        let mut position = probe_position(item_hash, (*table).slot_count);
                        let mut step = BUCKET_LENGTH;
                        loop {
                            let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                            let limit = position & BUCKET_MASK;
                            let mut inserted = false;

                            for slot in limit..BUCKET_LENGTH {
                                if (*bucket).hash[slot] == HASH_EMPTY {
                                    (*bucket).hash[slot] = item_hash;
                                    (*bucket).index[slot] = (*old_bucket).index[item];
                                    inserted = true;
                                    break;
                                }
                            }
                            if !inserted {
                                for slot in 0..limit {
                                    if (*bucket).hash[slot] == HASH_EMPTY {
                                        (*bucket).hash[slot] = item_hash;
                                        (*bucket).index[slot] = (*old_bucket).index[item];
                                        inserted = true;
                                        break;
                                    }
                                }
                            }
                            if inserted {
                                break;
                            }

                            position = position.wrapping_add(step);
                            step = step.wrapping_add(BUCKET_LENGTH);
                            position &= (*table).slot_count.wrapping_sub(1);
                        }
                    }
                }
            }
        }
    }

    table
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    data: *mut c_void,
    elem_size: usize,
    add_len: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (unsafe { array_len(data) } as usize).wrapping_add(add_len);
    if min_len > min_cap {
        min_cap = min_len;
    }

    let old_cap = unsafe { array_cap(data) };
    if min_cap <= old_cap {
        return data;
    }
    if min_cap < old_cap.wrapping_mul(2) {
        min_cap = old_cap.wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_allocation = if data.is_null() {
        ptr::null_mut()
    } else {
        unsafe { header(data) }.cast()
    };
    let allocation_size = elem_size
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<ArrayHeader>());
    let allocation = unsafe { realloc(old_allocation, allocation_size) };
    let result = allocation
        .cast::<u8>()
        .wrapping_add(size_of::<ArrayHeader>())
        .cast::<c_void>();

    unsafe {
        if data.is_null() {
            (*header(result)).length = 0;
            (*header(result)).hash_table = ptr::null_mut();
            (*header(result)).temp = 0;
        }
        (*header(result)).capacity = min_cap;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(data: *mut c_void) {
    unsafe { free(header(data).cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { HASH_SEED = seed };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    unsafe {
        while *string != 0 {
            hash = hash
                .rotate_left(9)
                .wrapping_add(*string.cast::<u8>() as usize);
            string = string.add(1);
        }
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

unsafe fn siphash_bytes(data_ptr: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_ptr = data_ptr.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0usize;
    while offset.wrapping_add(size_of::<usize>()) <= len {
        let bytes = unsafe { std::slice::from_raw_parts(data_ptr, 8) };
        let low = ((bytes[0] as u32)
            | ((bytes[1] as u32) << 8)
            | ((bytes[2] as u32) << 16)
            | ((bytes[3] as u32) << 24)) as i32;
        let high = ((bytes[4] as u32)
            | ((bytes[5] as u32) << 8)
            | ((bytes[6] as u32) << 16)
            | ((bytes[7] as u32) << 24)) as i32;
        let mut data = low as isize as usize;
        data |= (high as isize as usize) << 32;

        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        offset = offset.wrapping_add(size_of::<usize>());
        data_ptr = data_ptr.wrapping_add(size_of::<usize>());
    }

    let remaining = len.wrapping_sub(offset);
    let mut data = len << (usize::BITS - 8);
    unsafe {
        if remaining >= 7 {
            data |= (*data_ptr.add(6) as usize) << 48;
        }
        if remaining >= 6 {
            data |= (*data_ptr.add(5) as usize) << 40;
        }
        if remaining >= 5 {
            data |= (*data_ptr.add(4) as usize) << 32;
        }
        if remaining >= 4 {
            let fourth = ((*data_ptr.add(3) as u32) << 24) as i32;
            data |= fourth as isize as usize;
        }
        if remaining >= 3 {
            data |= (*data_ptr.add(2) as usize) << 16;
        }
        if remaining >= 2 {
            data |= (*data_ptr.add(1) as usize) << 8;
        }
        if remaining >= 1 {
            data |= *data_ptr as usize;
        }
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
pub unsafe extern "C" fn stbds_hash_bytes(data: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { siphash_bytes(data, len, seed) }
}

unsafe fn key_is_equal(
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
        .wrapping_add(elem_size.wrapping_mul(index))
        .wrapping_add(key_offset);
    if mode >= HM_STRING {
        unsafe { strcmp(key.cast(), *stored_key.cast::<*mut c_char>()) == 0 }
    } else {
        unsafe { memcmp(key, stored_key.cast(), key_size) == 0 }
    }
}

unsafe fn hm_find_slot(
    hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = unsafe { hash_to_array(hash_array, elem_size) };
    let table = unsafe { hash_table(raw_array) };
    let mut hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key.cast(), (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if hash < 2 {
        hash += 2;
    }

    unsafe {
        let mut position = probe_position(hash, (*table).slot_count);
        let mut step = BUCKET_LENGTH;
        loop {
            let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
            let limit = position & BUCKET_MASK;

            for item in limit..BUCKET_LENGTH {
                if (*bucket).hash[item] == hash {
                    if key_is_equal(
                        hash_array,
                        elem_size,
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
            for item in 0..limit {
                if (*bucket).hash[item] == hash {
                    if key_is_equal(
                        hash_array,
                        elem_size,
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
            position &= (*table).slot_count.wrapping_sub(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut hash_array: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if hash_array.is_null() {
        hash_array = unsafe { stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1) };
        unsafe {
            (*header(hash_array)).length += 1;
            memset(hash_array, 0, elem_size);
            *temp = INDEX_EMPTY;
            array_to_hash(hash_array, elem_size)
        }
    } else {
        let raw_array = unsafe { hash_to_array(hash_array, elem_size) };
        let table = unsafe { hash_table(raw_array) };
        unsafe {
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
    let mut temp = 0isize;
    let result =
        unsafe { stbds_hmget_key_ts(hash_array, elem_size, key, key_size, &mut temp, mode) };
    let raw_array = unsafe { hash_to_array(result, elem_size) };
    unsafe { (*header(raw_array)).temp = temp };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut hash_array: *mut c_void,
    elem_size: usize,
) -> *mut c_void {
    if hash_array.is_null()
        || unsafe { (*header(hash_to_array(hash_array, elem_size))).length == 0 }
    {
        let raw_array = if hash_array.is_null() {
            ptr::null_mut()
        } else {
            unsafe { hash_to_array(hash_array, elem_size) }
        };
        let raw_array = unsafe { stbds_arrgrowf(raw_array, elem_size, 0, 1) };
        unsafe {
            (*header(raw_array)).length += 1;
            memset(raw_array, 0, elem_size);
            hash_array = array_to_hash(raw_array, elem_size);
        }
    }
    hash_array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = unsafe { strlen(string) }.wrapping_add(1);
    let copy = unsafe { realloc(ptr::null_mut(), len) }.cast::<c_char>();
    unsafe { memmove(copy.cast(), string.cast(), len) };
    copy
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
        let raw_array = unsafe { stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1) };
        unsafe {
            memset(raw_array, 0, elem_size);
            (*header(raw_array)).length += 1;
            hash_array = array_to_hash(raw_array, elem_size);
        }
    }

    let mut raw_hash_array = hash_array;
    let mut raw_array = unsafe { hash_to_array(hash_array, elem_size) };
    let mut table = unsafe { hash_table(raw_array) };

    unsafe {
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

        loop {
            let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
            let limit = position & BUCKET_MASK;
            let mut empty_position = None;

            for item in limit..BUCKET_LENGTH {
                if (*bucket).hash[item] == hash {
                    if key_is_equal(
                        raw_hash_array,
                        elem_size,
                        key,
                        key_size,
                        0,
                        mode,
                        (*bucket).index[item] as usize,
                    ) {
                        (*header(raw_array)).temp = (*bucket).index[item];
                        if mode >= HM_STRING {
                            (*table).temp_key = *raw_hash_array
                                .cast::<u8>()
                                .add(elem_size * (*bucket).index[item] as usize)
                                .cast::<*mut c_char>();
                        }
                        return array_to_hash(raw_array, elem_size);
                    }
                } else if (*bucket).hash[item] == HASH_EMPTY {
                    empty_position = Some((position & !BUCKET_MASK) + item);
                    break;
                } else if tombstone < 0 && (*bucket).index[item] == INDEX_DELETED {
                    tombstone = ((position & !BUCKET_MASK) + item) as isize;
                }
            }

            if empty_position.is_none() {
                for item in 0..limit {
                    if (*bucket).hash[item] == hash {
                        if key_is_equal(
                            raw_hash_array,
                            elem_size,
                            key,
                            key_size,
                            0,
                            mode,
                            (*bucket).index[item] as usize,
                        ) {
                            (*header(raw_array)).temp = (*bucket).index[item];
                            return array_to_hash(raw_array, elem_size);
                        }
                    } else if (*bucket).hash[item] == HASH_EMPTY {
                        empty_position = Some((position & !BUCKET_MASK) + item);
                        break;
                    } else if tombstone < 0 && (*bucket).index[item] == INDEX_DELETED {
                        tombstone = ((position & !BUCKET_MASK) + item) as isize;
                    }
                }
            }

            if let Some(empty) = empty_position {
                position = if tombstone >= 0 {
                    (*table).tombstone_count -= 1;
                    tombstone as usize
                } else {
                    empty
                };
                break;
            }

            position = position.wrapping_add(step);
            step = step.wrapping_add(BUCKET_LENGTH);
            position &= (*table).slot_count.wrapping_sub(1);
        }

        (*table).used_count += 1;
        let index = (*header(raw_array)).length as isize;
        if (index as usize).wrapping_add(1) > (*header(raw_array)).capacity {
            raw_array = stbds_arrgrowf(raw_array, elem_size, 1, 0);
        }
        raw_hash_array = array_to_hash(raw_array, elem_size);
        (*header(raw_array)).length = (index as usize).wrapping_add(1);

        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        (*bucket).hash[position & BUCKET_MASK] = hash;
        (*bucket).index[position & BUCKET_MASK] = index - 1;
        (*header(raw_array)).temp = index - 1;

        let destination = raw_array
            .cast::<u8>()
            .add(elem_size.wrapping_mul(index as usize));
        match (*table).string.mode {
            SH_STRDUP => {
                let stored = duplicate_string(key.cast());
                *destination.cast::<*mut c_char>() = stored;
                (*table).temp_key = stored;
            }
            SH_ARENA => {
                let stored = stbds_stralloc(&mut (*table).string, key.cast());
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
        raw_hash_array
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let array = unsafe { stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1) };
    unsafe {
        memset(array, 0, elem_size);
        (*header(array)).length = 1;
        let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
        (*header(array)).hash_table = table.cast();
        (*table).string.mode = mode as u8;
        array_to_hash(array, elem_size)
    }
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
        return ptr::null_mut();
    }

    unsafe {
        let raw_array = hash_to_array(hash_array, elem_size);
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
        let final_index = (*header(raw_array)).length as isize - 2;

        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_array)).temp = 1;
        (*bucket).hash[bucket_index] = HASH_DELETED;
        (*bucket).index[bucket_index] = INDEX_DELETED;

        if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
            let string = *hash_array
                .cast::<u8>()
                .add(elem_size * old_index as usize)
                .cast::<*mut c_void>();
            free(string);
        }

        if old_index != final_index {
            let destination = hash_array.cast::<u8>().add(elem_size * old_index as usize);
            let source = hash_array
                .cast::<u8>()
                .add(elem_size * final_index as usize);
            memmove(destination.cast(), source.cast(), elem_size);

            slot = if mode == HM_STRING {
                hm_find_slot(
                    hash_array,
                    elem_size,
                    *destination.add(key_offset).cast::<*mut c_void>(),
                    key_size,
                    key_offset,
                    mode,
                )
            } else {
                hm_find_slot(
                    hash_array,
                    elem_size,
                    destination.add(key_offset).cast(),
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(raw_array: *mut c_void, elem_size: usize) {
    if raw_array.is_null() {
        return;
    }

    unsafe {
        let table = hash_table(raw_array);
        if !table.is_null() {
            if (*table).string.mode == SH_STRDUP {
                for index in 1..(*header(raw_array)).length {
                    let string = *raw_array
                        .cast::<u8>()
                        .add(elem_size * index)
                        .cast::<*mut c_void>();
                    free(string);
                }
            }
            stbds_strreset(&mut (*table).string);
        }
        free((*header(raw_array)).hash_table);
        free(header(raw_array).cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = unsafe { strlen(string) }.wrapping_add(1);
    unsafe {
        if len > (*arena).remaining {
            let block_size = STRING_ARENA_BLOCKSIZE_MIN << ((*arena).block >> 1);
            if block_size < STRING_ARENA_BLOCKSIZE_MAX {
                (*arena).block = (*arena).block.wrapping_add(1);
            }

            if len > block_size {
                let allocation_size = size_of::<StringBlock>().wrapping_sub(8).wrapping_add(len);
                let block = realloc(ptr::null_mut(), allocation_size).cast::<StringBlock>();
                memmove(arena_storage(block).cast(), string.cast(), len);
                if !(*arena).storage.is_null() {
                    (*block).next = (*(*arena).storage).next;
                    (*(*arena).storage).next = block;
                } else {
                    (*block).next = ptr::null_mut();
                    (*arena).storage = block;
                    (*arena).remaining = 0;
                }
                return arena_storage(block);
            }

            let allocation_size = size_of::<StringBlock>()
                .wrapping_sub(8)
                .wrapping_add(block_size);
            let block = realloc(ptr::null_mut(), allocation_size).cast::<StringBlock>();
            (*block).next = (*arena).storage;
            (*arena).storage = block;
            (*arena).remaining = block_size;
        }

        let result = arena_storage((*arena).storage).add((*arena).remaining.wrapping_sub(len));
        (*arena).remaining -= len;
        memmove(result.cast(), string.cast(), len);
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    unsafe {
        let mut block = (*arena).storage;
        while !block.is_null() {
            let next = (*block).next;
            free(block.cast());
            block = next;
        }
        memset(arena.cast(), 0, size_of::<StringArena>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    unsafe { sprintf(buffer, FORMAT.as_ptr().cast(), number) };
    buffer
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringCharEntry {
    key: *mut c_char,
    value: c_char,
}

unsafe fn put_string_char(
    hash: *mut StringCharEntry,
    key: *mut c_char,
    value: c_char,
) -> *mut StringCharEntry {
    let result = unsafe {
        stbds_hmput_key(
            hash.cast(),
            size_of::<StringCharEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            HM_STRING,
        )
    }
    .cast::<StringCharEntry>();
    let raw_array = unsafe { hash_to_array(result.cast(), size_of::<StringCharEntry>()) };
    let index = unsafe { (*header(raw_array)).temp as usize };
    unsafe { (*result.add(index)).value = value };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    const BOB: &[u8] = b"bob\0";
    const SALLY: &[u8] = b"sally\0";
    const FRED: &[u8] = b"fred\0";
    const JEN: &[u8] = b"jen\0";
    const DOUG: &[u8] = b"doug\0";
    const FORMAT: &[u8] = b"%s %c\n\0";

    let mut hash: *mut StringCharEntry = ptr::null_mut();
    let mut name = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];
    unsafe {
        hash = put_string_char(hash, BOB.as_ptr().cast_mut().cast(), b'h' as c_char);
        hash = put_string_char(hash, SALLY.as_ptr().cast_mut().cast(), b'e' as c_char);
        hash = put_string_char(hash, FRED.as_ptr().cast_mut().cast(), b'l' as c_char);
        hash = put_string_char(hash, JEN.as_ptr().cast_mut().cast(), b'x' as c_char);
        hash = put_string_char(hash, DOUG.as_ptr().cast_mut().cast(), b'o' as c_char);
        hash = put_string_char(hash, name.as_mut_ptr(), letter);

        let raw_array = hash_to_array(hash.cast(), size_of::<StringCharEntry>());
        let length = (*header(raw_array)).length as isize - 1;
        for index in 0..length {
            let entry = *hash.offset(index);
            printf(FORMAT.as_ptr().cast(), entry.key, c_int::from(entry.value));
        }
        stbds_hmfree_func(raw_array, size_of::<StringCharEntry>());
    }
}
