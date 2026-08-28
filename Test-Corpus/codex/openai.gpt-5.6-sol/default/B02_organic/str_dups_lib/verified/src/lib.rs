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
struct StringMapEntry {
    key: *mut c_char,
    value: c_int,
}

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> c_int;
    fn memmove(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn printf(format: *const c_char, ...) -> c_int;
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x31415926;
static mut BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<u8>().sub(size_of::<ArrayHeader>()).cast() }
}

#[inline]
unsafe fn array_len(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        unsafe { (*header(array)).length }
    }
}

#[inline]
unsafe fn array_capacity(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        unsafe { (*header(array)).capacity }
    }
}

#[inline]
unsafe fn hash_to_array(hash: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { hash.cast::<u8>().sub(element_size).cast() }
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { array.cast::<u8>().add(element_size).cast() }
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    unsafe { (*header(array)).hash_table.cast() }
}

#[inline]
unsafe fn entry_at(base: *mut c_void, element_size: usize, index: usize) -> *mut u8 {
    unsafe { base.cast::<u8>().add(element_size.wrapping_mul(index)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    element_size: usize,
    add_len: usize,
    mut min_capacity: usize,
) -> *mut c_void {
    let min_len = unsafe { array_len(array) }.wrapping_add(add_len);
    if min_len > min_capacity {
        min_capacity = min_len;
    }
    if min_capacity <= unsafe { array_capacity(array) } {
        return array;
    }

    let old_capacity = unsafe { array_capacity(array) };
    if min_capacity < old_capacity.wrapping_mul(2) {
        min_capacity = old_capacity.wrapping_mul(2);
    } else if min_capacity < 4 {
        min_capacity = 4;
    }

    let allocation_size = element_size
        .wrapping_mul(min_capacity)
        .wrapping_add(size_of::<ArrayHeader>());
    let allocation = unsafe {
        realloc(
            if array.is_null() {
                ptr::null_mut()
            } else {
                header(array).cast()
            },
            allocation_size,
        )
    };
    let result = unsafe {
        allocation
            .cast::<u8>()
            .add(size_of::<ArrayHeader>())
            .cast::<c_void>()
    };
    if array.is_null() {
        unsafe {
            (*header(result)).length = 0;
            (*header(result)).hash_table = ptr::null_mut();
            (*header(result)).temp = 0;
        }
    }
    unsafe { (*header(result)).capacity = min_capacity };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(array: *mut c_void) {
    unsafe { free(header(array).cast()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { HASH_SEED = seed };
}

#[inline]
fn table_log2(mut slot_count: usize) -> usize {
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
    let table = unsafe { realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>() };
    let storage_address = (unsafe { table.add(1) } as usize).wrapping_add(CACHE_LINE_SIZE - 1)
        & !(CACHE_LINE_SIZE - 1);

    unsafe {
        (*table).storage = storage_address as *mut HashBucket;
        (*table).slot_count = slot_count;
        (*table).slot_count_log2 = table_log2(slot_count);
        (*table).tombstone_count = 0;
        (*table).used_count = 0;
        (*table).used_count_threshold = slot_count - (slot_count >> 2);
        (*table).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
        (*table).used_count_shrink_threshold = slot_count >> 2;
        if slot_count <= BUCKET_LENGTH {
            (*table).used_count_shrink_threshold = 0;
        }
        assert!(
            (*table).used_count_threshold + (*table).tombstone_count_threshold
                < (*table).slot_count
        );

        if !old.is_null() {
            ptr::copy_nonoverlapping(&(*old).string, &mut (*table).string, 1);
            (*table).seed = (*old).seed;
        } else {
            ptr::write_bytes(&mut (*table).string, 0, 1);
            (*table).seed = HASH_SEED;
            HASH_SEED = HASH_SEED
                .wrapping_mul(0x27bb2ee687b0b0fd)
                .wrapping_add(0x00000000b504f32d);
        }

        for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
            let bucket = (*table).storage.add(bucket_index);
            for index in 0..BUCKET_LENGTH {
                (*bucket).hash[index] = HASH_EMPTY;
                (*bucket).index[index] = INDEX_EMPTY;
            }
        }

        if !old.is_null() {
            (*table).used_count = (*old).used_count;
            for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
                let old_bucket = (*old).storage.add(old_bucket_index);
                for old_slot_index in 0..BUCKET_LENGTH {
                    if (*old_bucket).index[old_slot_index] >= 0 {
                        insert_rehashed(
                            table,
                            (*old_bucket).hash[old_slot_index],
                            (*old_bucket).index[old_slot_index],
                        );
                    }
                }
            }
        }
    }
    table
}

unsafe fn insert_rehashed(table: *mut HashIndex, hash: usize, index: isize) {
    let mut position = probe_position(hash, unsafe { (*table).slot_count });
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
        let limit = position & BUCKET_MASK;

        for slot in limit..BUCKET_LENGTH {
            if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                unsafe {
                    (*bucket).hash[slot] = hash;
                    (*bucket).index[slot] = index;
                }
                return;
            }
        }
        for slot in 0..limit {
            if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                unsafe {
                    (*bucket).hash[slot] = hash;
                    (*bucket).index[slot] = index;
                }
                return;
            }
        }

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= unsafe { (*table).slot_count - 1 };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while unsafe { *string } != 0 {
        hash = hash
            .rotate_left(9)
            .wrapping_add(unsafe { *string as u8 as usize });
        string = unsafe { string.add(1) };
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

unsafe fn siphash_bytes(pointer: *mut c_void, length: usize, seed: usize) -> usize {
    let mut data_pointer = pointer.cast::<u8>();
    let mut v0 = 0x736f6d6570736575usize ^ seed;
    let mut v1 = 0x646f72616e646f6dusize ^ !seed;
    let mut v2 = 0x6c7967656e657261usize ^ seed;
    let mut v3 = 0x7465646279746573usize ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut offset = 0;
    while offset + size_of::<usize>() <= length {
        let low = unsafe {
            (*data_pointer.add(0) as u32)
                | ((*data_pointer.add(1) as u32) << 8)
                | ((*data_pointer.add(2) as u32) << 16)
                | ((*data_pointer.add(3) as u32) << 24)
        };
        let high = unsafe {
            (*data_pointer.add(4) as u32)
                | ((*data_pointer.add(5) as u32) << 8)
                | ((*data_pointer.add(6) as u32) << 16)
                | ((*data_pointer.add(7) as u32) << 24)
        };
        let data = (low as i32 as isize as usize) | ((high as usize) << 32);
        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        offset += size_of::<usize>();
        data_pointer = unsafe { data_pointer.add(size_of::<usize>()) };
    }

    let mut data = length << (usize::BITS as usize - 8);
    let remaining = length - offset;
    if remaining >= 7 {
        data |= (unsafe { *data_pointer.add(6) } as usize) << 48;
    }
    if remaining >= 6 {
        data |= (unsafe { *data_pointer.add(5) } as usize) << 40;
    }
    if remaining >= 5 {
        data |= (unsafe { *data_pointer.add(4) } as usize) << 32;
    }
    if remaining >= 4 {
        let shifted = (unsafe { *data_pointer.add(3) } as u32) << 24;
        data |= shifted as i32 as isize as usize;
    }
    if remaining >= 3 {
        data |= (unsafe { *data_pointer.add(2) } as usize) << 16;
    }
    if remaining >= 2 {
        data |= (unsafe { *data_pointer.add(1) } as usize) << 8;
    }
    if remaining >= 1 {
        data |= unsafe { *data_pointer } as usize;
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
    unsafe { siphash_bytes(pointer, length, seed) }
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
    let stored_key = unsafe { entry_at(array, element_size, index).add(key_offset) };
    if mode >= HM_STRING {
        unsafe { strcmp(key.cast(), *stored_key.cast::<*mut c_char>()) == 0 }
    } else {
        unsafe { memcmp(key, stored_key.cast(), key_size) == 0 }
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
    let raw_array = unsafe { hash_to_array(array, element_size) };
    let table = unsafe { hash_table(raw_array) };
    let mut hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key.cast(), (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if hash < 2 {
        hash += 2;
    }

    let mut position = probe_position(hash, unsafe { (*table).slot_count });
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
        let limit = position & BUCKET_MASK;
        for slot in limit..BUCKET_LENGTH {
            let bucket_hash = unsafe { (*bucket).hash[slot] };
            if bucket_hash == hash {
                let index = unsafe { (*bucket).index[slot] };
                if unsafe {
                    is_key_equal(
                        array,
                        element_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        index as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if bucket_hash == HASH_EMPTY {
                return -1;
            }
        }
        for slot in 0..limit {
            let bucket_hash = unsafe { (*bucket).hash[slot] };
            if bucket_hash == hash {
                let index = unsafe { (*bucket).index[slot] };
                if unsafe {
                    is_key_equal(
                        array,
                        element_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        index as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if bucket_hash == HASH_EMPTY {
                return -1;
            }
        }
        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= unsafe { (*table).slot_count - 1 };
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
        array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
        unsafe {
            (*header(array)).length += 1;
            memset(array, 0, element_size);
            *temp = INDEX_EMPTY;
            array_to_hash(array, element_size)
        }
    } else {
        let raw_array = unsafe { hash_to_array(array, element_size) };
        let table = unsafe { hash_table(raw_array) };
        if table.is_null() {
            unsafe { *temp = INDEX_EMPTY };
        } else {
            let slot = unsafe { find_slot(array, element_size, key, key_size, 0, mode) };
            if slot < 0 {
                unsafe { *temp = INDEX_EMPTY };
            } else {
                let bucket = unsafe { (*table).storage.add(slot as usize >> BUCKET_SHIFT) };
                unsafe { *temp = (*bucket).index[slot as usize & BUCKET_MASK] };
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
    let mut temp = 0isize;
    let result = unsafe { stbds_hmget_key_ts(array, element_size, key, key_size, &mut temp, mode) };
    let raw_array = unsafe { hash_to_array(result, element_size) };
    unsafe { (*header(raw_array)).temp = temp };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut array: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if array.is_null() || unsafe { (*header(hash_to_array(array, element_size))).length } == 0 {
        let raw_array = if array.is_null() {
            ptr::null_mut()
        } else {
            unsafe { hash_to_array(array, element_size) }
        };
        array = unsafe { stbds_arrgrowf(raw_array, element_size, 0, 1) };
        unsafe {
            (*header(array)).length += 1;
            memset(array, 0, element_size);
            array_to_hash(array, element_size)
        }
    } else {
        array
    }
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let length = unsafe { strlen(string) }.wrapping_add(1);
    let result = unsafe { realloc(ptr::null_mut(), length).cast::<c_char>() };
    unsafe { memmove(result.cast(), string.cast(), length) };
    result
}

unsafe fn set_temp_key(array: *mut c_void, key: *mut c_char) {
    let table = unsafe { (*header(array)).hash_table.cast::<HashIndex>() };
    unsafe { (*table).temp_key = key };
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
        array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
        unsafe {
            memset(array, 0, element_size);
            (*header(array)).length += 1;
            array = array_to_hash(array, element_size);
        }
    }

    let raw_hash_array = array;
    let mut raw_array = unsafe { hash_to_array(array, element_size) };
    let mut table = unsafe { hash_table(raw_array) };

    if table.is_null() || unsafe { (*table).used_count >= (*table).used_count_threshold } {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            unsafe { (*table).slot_count.wrapping_mul(2) }
        };
        let new_table = unsafe { make_hash_index(slot_count, table) };
        if !table.is_null() {
            unsafe { free(table.cast()) };
        } else {
            unsafe {
                (*new_table).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 };
            }
        }
        table = new_table;
        unsafe { (*header(raw_array)).hash_table = table.cast() };
    }

    let mut hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key.cast(), (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if hash < 2 {
        hash += 2;
    }

    let mut position = probe_position(hash, unsafe { (*table).slot_count });
    let mut step = BUCKET_LENGTH;
    let mut tombstone = -1isize;

    'search: loop {
        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
        let limit = position & BUCKET_MASK;

        for slot in limit..BUCKET_LENGTH {
            let bucket_hash = unsafe { (*bucket).hash[slot] };
            if bucket_hash == hash {
                let index = unsafe { (*bucket).index[slot] };
                if unsafe {
                    is_key_equal(
                        raw_hash_array,
                        element_size,
                        key,
                        key_size,
                        0,
                        mode,
                        index as usize,
                    )
                } {
                    unsafe {
                        (*header(raw_array)).temp = index;
                        if mode >= HM_STRING {
                            let stored_key =
                                *entry_at(raw_hash_array, element_size, index as usize)
                                    .cast::<*mut c_char>();
                            set_temp_key(raw_array, stored_key);
                        }
                        return array_to_hash(raw_array, element_size);
                    }
                }
            } else if bucket_hash == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && unsafe { (*bucket).index[slot] } == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        for slot in 0..limit {
            let bucket_hash = unsafe { (*bucket).hash[slot] };
            if bucket_hash == hash {
                let index = unsafe { (*bucket).index[slot] };
                if unsafe {
                    is_key_equal(
                        raw_hash_array,
                        element_size,
                        key,
                        key_size,
                        0,
                        mode,
                        index as usize,
                    )
                } {
                    unsafe {
                        (*header(raw_array)).temp = index;
                        return array_to_hash(raw_array, element_size);
                    }
                }
            } else if bucket_hash == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && unsafe { (*bucket).index[slot] } == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= unsafe { (*table).slot_count - 1 };
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        unsafe { (*table).tombstone_count -= 1 };
    }
    unsafe { (*table).used_count += 1 };

    let index = unsafe { array_len(raw_array) } as isize;
    if (index as usize).wrapping_add(1) > unsafe { array_capacity(raw_array) } {
        raw_array = unsafe { stbds_arrgrowf(raw_array, element_size, 1, 0) };
    }
    unsafe {
        assert!((index as usize).wrapping_add(1) <= array_capacity(raw_array));
        (*header(raw_array)).length = index as usize + 1;
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        (*bucket).hash[position & BUCKET_MASK] = hash;
        (*bucket).index[position & BUCKET_MASK] = index - 1;
        (*header(raw_array)).temp = index - 1;

        let destination = entry_at(raw_array, element_size, index as usize);
        match (*table).string.mode {
            SH_STRDUP => {
                let stored = duplicate_string(key.cast());
                *destination.cast::<*mut c_char>() = stored;
                set_temp_key(raw_array, stored);
            }
            SH_ARENA => {
                let stored = stbds_stralloc(&mut (*table).string, key.cast());
                *destination.cast::<*mut c_char>() = stored;
                set_temp_key(raw_array, stored);
            }
            SH_DEFAULT => {
                let stored = key.cast::<c_char>();
                *destination.cast::<*mut c_char>() = stored;
                set_temp_key(raw_array, stored);
            }
            _ => {
                ptr::copy_nonoverlapping(key.cast::<u8>(), destination, key_size);
            }
        }
        array_to_hash(raw_array, element_size)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
    unsafe {
        memset(array, 0, element_size);
        (*header(array)).length = 1;
        let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
        (*header(array)).hash_table = table.cast();
        (*table).string.mode = mode as u8;
        array_to_hash(array, element_size)
    }
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
        return ptr::null_mut();
    }

    let raw_array = unsafe { hash_to_array(array, element_size) };
    let table = unsafe { hash_table(raw_array) };
    unsafe { (*header(raw_array)).temp = 0 };
    if table.is_null() {
        return array;
    }

    let mut slot = unsafe { find_slot(array, element_size, key, key_size, key_offset, mode) };
    if slot < 0 {
        return array;
    }

    let mut bucket = unsafe { (*table).storage.add(slot as usize >> BUCKET_SHIFT) };
    let mut bucket_slot = slot as usize & BUCKET_MASK;
    let old_index = unsafe { (*bucket).index[bucket_slot] };
    let final_index = unsafe { array_len(raw_array) }.wrapping_sub(2) as isize;
    unsafe {
        assert!(slot < (*table).slot_count as isize);
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_array)).temp = 1;
        (*bucket).hash[bucket_slot] = HASH_DELETED;
        (*bucket).index[bucket_slot] = INDEX_DELETED;
    }

    if mode == HM_STRING && unsafe { (*table).string.mode == SH_STRDUP } {
        unsafe {
            free((*entry_at(array, element_size, old_index as usize).cast::<*mut c_char>()).cast())
        };
    }

    if old_index != final_index {
        unsafe {
            memmove(
                entry_at(array, element_size, old_index as usize).cast(),
                entry_at(array, element_size, final_index as usize).cast(),
                element_size,
            );
        }
        slot = if mode == HM_STRING {
            let moved_key = unsafe {
                *entry_at(array, element_size, old_index as usize)
                    .add(key_offset)
                    .cast::<*mut c_char>()
            };
            unsafe {
                find_slot(
                    array,
                    element_size,
                    moved_key.cast(),
                    key_size,
                    key_offset,
                    mode,
                )
            }
        } else {
            let moved_key =
                unsafe { entry_at(array, element_size, old_index as usize).add(key_offset) };
            unsafe {
                find_slot(
                    array,
                    element_size,
                    moved_key.cast(),
                    key_size,
                    key_offset,
                    mode,
                )
            }
        };
        assert!(slot >= 0);
        bucket = unsafe { (*table).storage.add(slot as usize >> BUCKET_SHIFT) };
        bucket_slot = slot as usize & BUCKET_MASK;
        unsafe {
            assert_eq!((*bucket).index[bucket_slot], final_index);
            (*bucket).index[bucket_slot] = old_index;
        }
    }
    unsafe { (*header(raw_array)).length -= 1 };

    if unsafe {
        (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > BUCKET_LENGTH
    } {
        let new_table = unsafe { make_hash_index((*table).slot_count >> 1, table) };
        unsafe {
            (*header(raw_array)).hash_table = new_table.cast();
            free(table.cast());
        }
    } else if unsafe { (*table).tombstone_count > (*table).tombstone_count_threshold } {
        let new_table = unsafe { make_hash_index((*table).slot_count, table) };
        unsafe {
            (*header(raw_array)).hash_table = new_table.cast();
            free(table.cast());
        }
    }

    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let length = unsafe { strlen(string) }.wrapping_add(1);
    if length > unsafe { (*arena).remaining } {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << (unsafe { (*arena).block } as usize >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            unsafe { (*arena).block = (*arena).block.wrapping_add(1) };
        }

        if length > block_size {
            let block = unsafe {
                realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + length)
                    .cast::<StringBlock>()
            };
            unsafe {
                memmove((*block).storage.as_mut_ptr().cast(), string.cast(), length);
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
        } else {
            let block = unsafe {
                realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + block_size)
                    .cast::<StringBlock>()
            };
            unsafe {
                (*block).next = (*arena).storage;
                (*arena).storage = block;
                (*arena).remaining = block_size;
            }
        }
    }

    assert!(length <= unsafe { (*arena).remaining });
    let result = unsafe {
        (*(*arena).storage)
            .storage
            .as_mut_ptr()
            .add((*arena).remaining - length)
    };
    unsafe {
        (*arena).remaining -= length;
        memmove(result.cast(), string.cast(), length);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut block = unsafe { (*arena).storage };
    while !block.is_null() {
        let next = unsafe { (*block).next };
        unsafe { free(block.cast()) };
        block = next;
    }
    unsafe { memset(arena.cast(), 0, size_of::<StringArena>()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn strkey(number: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    unsafe {
        let buffer = ptr::addr_of_mut!(BUFFER).cast::<c_char>();
        sprintf(buffer, FORMAT.as_ptr().cast(), number);
        buffer
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(number: c_int) {
    const A: &[u8] = b"a\0";
    const FORMAT: &[u8] = b"%s %d\n\0";

    unsafe {
        let mut arena = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut index = 0;
        while index < number {
            stbds_stralloc(&mut arena, strkey(index));
            index += 1;
        }
        stbds_strreset(&mut arena);

        let source = StringMapEntry {
            key: A.as_ptr().cast_mut().cast(),
            value: number,
        };
        let mut map = stbds_shmode_func(size_of::<StringMapEntry>(), SH_STRDUP as c_int)
            .cast::<StringMapEntry>();
        map = stbds_hmput_key(
            map.cast(),
            size_of::<StringMapEntry>(),
            source.key.cast(),
            size_of::<*mut c_char>(),
            HM_STRING,
        )
        .cast();

        let raw_array = hash_to_array(map.cast(), size_of::<StringMapEntry>());
        let map_index = (*header(raw_array)).temp as usize;
        *map.add(map_index) = source;
        (*map.add(map_index)).key = (*hash_table(raw_array)).temp_key;
        assert_eq!(*(*map).key, b'a' as c_char);
        assert_ne!((*map).key, source.key);
        assert_eq!((*map).value, source.value);

        let length = (*header(raw_array)).length as isize - 1;
        let mut output_index = 0isize;
        while output_index < length {
            let entry = map.offset(output_index);
            printf(FORMAT.as_ptr().cast(), (*entry).key, (*entry).value);
            output_index += 1;
        }

        stbds_hmfree_func(raw_array, size_of::<StringMapEntry>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, element_size: usize) {
    if array.is_null() {
        return;
    }
    let table = unsafe { hash_table(array) };
    if !table.is_null() {
        if unsafe { (*table).string.mode == SH_STRDUP } {
            let length = unsafe { (*header(array)).length };
            for index in 1..length {
                let string = unsafe { *entry_at(array, element_size, index).cast::<*mut c_char>() };
                unsafe { free(string.cast()) };
            }
        }
        unsafe { stbds_strreset(&mut (*table).string) };
    }
    unsafe {
        free((*header(array)).hash_table);
        free(header(array).cast());
    }
}
