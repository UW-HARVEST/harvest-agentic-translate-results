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
    fn memcmp(left: *const c_void, right: *const c_void, size: usize) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { (array as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader }
}

#[inline]
unsafe fn hash_to_array(hash_array: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { (hash_array as *mut u8).sub(element_size) as *mut c_void }
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { (array as *mut u8).add(element_size) as *mut c_void }
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    unsafe { (*header(array)).hash_table as *mut HashIndex }
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

    let allocation = element_size
        .wrapping_mul(min_capacity)
        .wrapping_add(size_of::<ArrayHeader>());
    let old_header = if array.is_null() {
        ptr::null_mut()
    } else {
        unsafe { header(array) as *mut c_void }
    };
    let base = unsafe { realloc(old_header, allocation) };
    let result = unsafe { (base as *mut u8).add(size_of::<ArrayHeader>()) as *mut c_void };

    if array.is_null() {
        unsafe {
            (*header(result)).length = 0;
            (*header(result)).hash_table = ptr::null_mut();
            (*header(result)).temp = 0;
        }
    }
    unsafe {
        (*header(result)).capacity = min_capacity;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(array: *mut c_void) {
    unsafe {
        free(header(array) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        HASH_SEED = seed;
    }
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

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = unsafe { realloc(ptr::null_mut(), allocation) as *mut HashIndex };
    let storage_address =
        ((unsafe { table.add(1) } as usize) + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);
    let storage = storage_address as *mut HashBucket;

    unsafe {
        (*table).storage = storage;
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

        if old.is_null() {
            (*table).string = StringArena {
                storage: ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            (*table).seed = HASH_SEED;

            let low_a = 0x87b0_b0fdusize ^ 2_147_001_325usize;
            let a = (0x27bb_2ee6usize << 32) ^ (low_a << 32 >> 32) ^ 2_147_001_325usize;
            let low_b = 0xb504_f32dusize ^ 715_136_305usize;
            let b = (low_b << 32 >> 32) ^ 715_136_305usize;
            HASH_SEED = HASH_SEED.wrapping_mul(a).wrapping_add(b);
        } else {
            (*table).string = (*old).string;
            (*table).seed = (*old).seed;
        }

        for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
            storage.add(bucket_index).write(HashBucket {
                hash: [HASH_EMPTY; BUCKET_LENGTH],
                index: [INDEX_EMPTY; BUCKET_LENGTH],
            });
        }

        if !old.is_null() {
            (*table).used_count = (*old).used_count;
            for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
                let old_bucket = &*(*old).storage.add(old_bucket_index);
                for old_slot in 0..BUCKET_LENGTH {
                    if old_bucket.index[old_slot] >= 0 {
                        let item_hash = old_bucket.hash[old_slot];
                        let mut position =
                            probe_position(item_hash, slot_count, (*table).slot_count_log2);
                        let mut step = BUCKET_LENGTH;

                        'probe: loop {
                            let bucket = &mut *storage.add(position >> BUCKET_SHIFT);
                            let limit = position & BUCKET_MASK;
                            for slot in limit..BUCKET_LENGTH {
                                if bucket.hash[slot] == HASH_EMPTY {
                                    bucket.hash[slot] = item_hash;
                                    bucket.index[slot] = old_bucket.index[old_slot];
                                    break 'probe;
                                }
                            }
                            for slot in 0..limit {
                                if bucket.hash[slot] == HASH_EMPTY {
                                    bucket.hash[slot] = item_hash;
                                    bucket.index[slot] = old_bucket.index[old_slot];
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
    }

    table
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    unsafe {
        while *string != 0 {
            hash = hash.rotate_left(9).wrapping_add((*string as u8) as usize);
            string = string.add(1);
        }
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

unsafe fn siphash_bytes(data_pointer: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data = data_pointer as *const u8;
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
        let low = unsafe {
            (*data.add(0) as i32)
                | ((*data.add(1) as i32) << 8)
                | ((*data.add(2) as i32) << 16)
                | (*data.add(3) as i32).wrapping_shl(24)
        };
        let high = unsafe {
            (*data.add(4) as i32)
                | ((*data.add(5) as i32) << 8)
                | ((*data.add(6) as i32) << 16)
                | (*data.add(7) as i32).wrapping_shl(24)
        };
        let word = (low as isize as usize) | ((high as isize as usize) << 32);
        v3 ^= word;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= word;
        offset += size_of::<usize>();
        data = unsafe { data.add(size_of::<usize>()) };
    }

    let mut final_word = len << (usize::BITS as usize - 8);
    let remaining = len - offset;
    unsafe {
        if remaining >= 7 {
            final_word |= (*data.add(6) as usize) << 48;
        }
        if remaining >= 6 {
            final_word |= (*data.add(5) as usize) << 40;
        }
        if remaining >= 5 {
            final_word |= (*data.add(4) as usize) << 32;
        }
        if remaining >= 4 {
            final_word |= (*data.add(3) as i32).wrapping_shl(24) as isize as usize;
        }
        if remaining >= 3 {
            final_word |= (*data.add(2) as usize) << 16;
        }
        if remaining >= 2 {
            final_word |= (*data.add(1) as usize) << 8;
        }
        if remaining >= 1 {
            final_word |= *data as usize;
        }
    }

    v3 ^= final_word;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= final_word;
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

unsafe fn is_key_equal(
    array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored_key = unsafe {
        (array as *mut u8)
            .add(element_size.wrapping_mul(index))
            .add(key_offset)
    };
    if mode >= HM_STRING {
        unsafe {
            strcmp(
                key as *const c_char,
                *(stored_key as *const *mut c_char) as *const c_char,
            ) == 0
        }
    } else {
        unsafe { memcmp(key, stored_key as *const c_void, key_size) == 0 }
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
    let raw_array = unsafe { hash_to_array(array, element_size) };
    let table = unsafe { hash_table(raw_array) };
    let mut item_hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key as *mut c_char, (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if item_hash < 2 {
        item_hash += 2;
    }

    let mut position = probe_position(item_hash, unsafe { (*table).slot_count }, unsafe {
        (*table).slot_count_log2
    });
    let mut step = BUCKET_LENGTH;

    loop {
        let bucket = unsafe { &*(*table).storage.add(position >> BUCKET_SHIFT) };
        let limit = position & BUCKET_MASK;
        for slot in limit..BUCKET_LENGTH {
            if bucket.hash[slot] == item_hash {
                if unsafe {
                    is_key_equal(
                        array,
                        element_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                return -1;
            }
        }
        for slot in 0..limit {
            if bucket.hash[slot] == item_hash {
                if unsafe {
                    is_key_equal(
                        array,
                        element_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
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
            ptr::write_bytes(array as *mut u8, 0, element_size);
            *temp = INDEX_EMPTY;
            array_to_hash(array, element_size)
        }
    } else {
        let raw_array = unsafe { hash_to_array(array, element_size) };
        let table = unsafe { hash_table(raw_array) };
        unsafe {
            if table.is_null() {
                *temp = INDEX_EMPTY;
            } else {
                let slot = find_hash_slot(array, element_size, key, key_size, 0, mode);
                if slot < 0 {
                    *temp = INDEX_EMPTY;
                } else {
                    let bucket = &*(*table).storage.add(slot as usize >> BUCKET_SHIFT);
                    *temp = bucket.index[slot as usize & BUCKET_MASK];
                }
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
    unsafe {
        (*header(raw_array)).temp = temp;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut array: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    let needs_default =
        array.is_null() || unsafe { (*header(hash_to_array(array, element_size))).length == 0 };
    if needs_default {
        let raw_array = if array.is_null() {
            ptr::null_mut()
        } else {
            unsafe { hash_to_array(array, element_size) }
        };
        array = unsafe { stbds_arrgrowf(raw_array, element_size, 0, 1) };
        unsafe {
            (*header(array)).length += 1;
            ptr::write_bytes(array as *mut u8, 0, element_size);
            array = array_to_hash(array, element_size);
        }
    }
    array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = unsafe { strlen(string) } + 1;
    let result = unsafe { realloc(ptr::null_mut(), len) as *mut c_char };
    unsafe {
        ptr::copy(string, result, len);
    }
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
        let raw = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
        unsafe {
            ptr::write_bytes(raw as *mut u8, 0, element_size);
            (*header(raw)).length += 1;
            array = array_to_hash(raw, element_size);
        }
    }

    let mut raw_hash_array = array;
    let mut raw_array = unsafe { hash_to_array(array, element_size) };
    let mut table = unsafe { hash_table(raw_array) };

    if table.is_null() || unsafe { (*table).used_count >= (*table).used_count_threshold } {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            unsafe { (*table).slot_count.wrapping_mul(2) }
        };
        let new_table = unsafe { make_hash_index(slot_count, table) };
        unsafe {
            if table.is_null() {
                (*new_table).string.mode = if mode >= HM_STRING { 1 } else { 0 };
            } else {
                free(table as *mut c_void);
            }
            (*header(raw_array)).hash_table = new_table as *mut c_void;
        }
        table = new_table;
    }

    let mut item_hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key as *mut c_char, (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if item_hash < 2 {
        item_hash += 2;
    }

    let mut position = probe_position(item_hash, unsafe { (*table).slot_count }, unsafe {
        (*table).slot_count_log2
    });
    let mut step = BUCKET_LENGTH;
    let mut tombstone = -1isize;

    'search: loop {
        let bucket = unsafe { &mut *(*table).storage.add(position >> BUCKET_SHIFT) };
        let limit = position & BUCKET_MASK;
        for slot in limit..BUCKET_LENGTH {
            if bucket.hash[slot] == item_hash {
                if unsafe {
                    is_key_equal(
                        raw_hash_array,
                        element_size,
                        key,
                        key_size,
                        0,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    unsafe {
                        (*header(raw_array)).temp = bucket.index[slot];
                        if mode >= HM_STRING {
                            (*table).temp_key = *((raw_hash_array as *mut u8)
                                .add(element_size * bucket.index[slot] as usize)
                                as *mut *mut c_char);
                        }
                        return array_to_hash(raw_array, element_size);
                    }
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && bucket.index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }
        for slot in 0..limit {
            if bucket.hash[slot] == item_hash {
                if unsafe {
                    is_key_equal(
                        raw_hash_array,
                        element_size,
                        key,
                        key_size,
                        0,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    unsafe {
                        (*header(raw_array)).temp = bucket.index[slot];
                        return array_to_hash(raw_array, element_size);
                    }
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && bucket.index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }
        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= unsafe { (*table).slot_count - 1 };
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        unsafe {
            (*table).tombstone_count -= 1;
        }
    }
    unsafe {
        (*table).used_count += 1;
    }

    let index = unsafe { array_len(raw_array) } as isize;
    if (index as usize).wrapping_add(1) > unsafe { array_capacity(raw_array) } {
        raw_array = unsafe { stbds_arrgrowf(raw_array, element_size, 1, 0) };
    }
    raw_hash_array = unsafe { array_to_hash(raw_array, element_size) };

    unsafe {
        (*header(raw_array)).length = (index + 1) as usize;
        let bucket = &mut *(*table).storage.add(position >> BUCKET_SHIFT);
        bucket.hash[position & BUCKET_MASK] = item_hash;
        bucket.index[position & BUCKET_MASK] = index - 1;
        (*header(raw_array)).temp = index - 1;

        let destination = (raw_array as *mut u8).add(element_size.wrapping_mul(index as usize));
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
            1 => {
                let stored = key as *mut c_char;
                *(destination as *mut *mut c_char) = stored;
                (*table).temp_key = stored;
            }
            _ => {
                ptr::copy_nonoverlapping(key as *const u8, destination, key_size);
            }
        }
    }

    raw_hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
    unsafe {
        ptr::write_bytes(array as *mut u8, 0, element_size);
        (*header(array)).length = 1;
        let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
        (*header(array)).hash_table = table as *mut c_void;
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
    let mut table = unsafe { hash_table(raw_array) };
    unsafe {
        (*header(raw_array)).temp = 0;
    }
    if table.is_null() {
        return array;
    }

    let mut slot = unsafe { find_hash_slot(array, element_size, key, key_size, key_offset, mode) };
    if slot < 0 {
        return array;
    }

    unsafe {
        let mut bucket = &mut *(*table).storage.add(slot as usize >> BUCKET_SHIFT);
        let mut bucket_slot = slot as usize & BUCKET_MASK;
        let old_index = bucket.index[bucket_slot];
        let final_index = array_len(raw_array) as isize - 2;

        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_array)).temp = 1;
        bucket.hash[bucket_slot] = HASH_DELETED;
        bucket.index[bucket_slot] = INDEX_DELETED;

        if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
            let stored =
                *((array as *mut u8).add(element_size * old_index as usize) as *mut *mut c_void);
            free(stored);
        }

        if old_index != final_index {
            let destination = (array as *mut u8).add(element_size * old_index as usize);
            let source = (array as *mut u8).add(element_size * final_index as usize);
            ptr::copy(source, destination, element_size);

            slot = if mode == HM_STRING {
                let moved_key = *((destination.add(key_offset)) as *mut *mut c_void);
                find_hash_slot(array, element_size, moved_key, key_size, key_offset, mode)
            } else {
                find_hash_slot(
                    array,
                    element_size,
                    destination.add(key_offset) as *mut c_void,
                    key_size,
                    key_offset,
                    mode,
                )
            };
            bucket = &mut *(*table).storage.add(slot as usize >> BUCKET_SHIFT);
            bucket_slot = slot as usize & BUCKET_MASK;
            bucket.index[bucket_slot] = old_index;
        }
        (*header(raw_array)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > BUCKET_LENGTH
        {
            let replacement = make_hash_index((*table).slot_count >> 1, table);
            (*header(raw_array)).hash_table = replacement as *mut c_void;
            free(table as *mut c_void);
            table = replacement;
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            let replacement = make_hash_index((*table).slot_count, table);
            (*header(raw_array)).hash_table = replacement as *mut c_void;
            free(table as *mut c_void);
            table = replacement;
        }
    }
    let _ = table;
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = unsafe { strlen(string) } + 1;
    unsafe {
        if len > (*arena).remaining {
            let block_size = STRING_ARENA_BLOCKSIZE_MIN << ((*arena).block as usize >> 1);
            if block_size < STRING_ARENA_BLOCKSIZE_MAX {
                (*arena).block = (*arena).block.wrapping_add(1);
            }

            if len > block_size {
                let allocation = size_of::<StringBlock>() - 8 + len;
                let block = realloc(ptr::null_mut(), allocation) as *mut StringBlock;
                ptr::copy(string, (*block).storage.as_mut_ptr(), len);
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

            let allocation = size_of::<StringBlock>() - 8 + block_size;
            let block = realloc(ptr::null_mut(), allocation) as *mut StringBlock;
            (*block).next = (*arena).storage;
            (*arena).storage = block;
            (*arena).remaining = block_size;
        }

        let destination = (*(*arena).storage)
            .storage
            .as_mut_ptr()
            .add((*arena).remaining - len);
        (*arena).remaining -= len;
        ptr::copy(string, destination, len);
        destination
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    unsafe {
        let mut block = (*arena).storage;
        while !block.is_null() {
            let next = (*block).next;
            free(block as *mut c_void);
            block = next;
        }
        ptr::write_bytes(arena as *mut u8, 0, size_of::<StringArena>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, element_size: usize) {
    if array.is_null() {
        return;
    }
    unsafe {
        let table = hash_table(array);
        if !table.is_null() {
            if (*table).string.mode == SH_STRDUP {
                for index in 1..(*header(array)).length {
                    let stored =
                        *((array as *mut u8).add(element_size * index) as *mut *mut c_void);
                    free(stored);
                }
            }
            stbds_strreset(&mut (*table).string);
        }
        free((*header(array)).hash_table);
        free(header(array) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    unsafe {
        let buffer = ptr::addr_of_mut!(STRKEY_BUFFER) as *mut c_char;
        sprintf(buffer, FORMAT.as_ptr() as *const c_char, number);
        buffer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(number: c_int) {
    let mut array: *mut c_int = ptr::null_mut();
    let mut outer: c_int = 0;
    while outer < number {
        let mut inner: c_int = 0;
        while inner < outer {
            let raw = array as *mut c_void;
            if raw.is_null()
                || unsafe { (*header(raw)).length.wrapping_add(1) > (*header(raw)).capacity }
            {
                array = unsafe { stbds_arrgrowf(raw, size_of::<c_int>(), 1, 0) as *mut c_int };
            }
            unsafe {
                let item = (*header(array as *mut c_void)).length;
                *array.add(item) = inner;
                (*header(array as *mut c_void)).length += 1;
            }
            inner += 1;
        }
        if !array.is_null() {
            unsafe {
                free(header(array as *mut c_void) as *mut c_void);
            }
            array = ptr::null_mut();
        }
        outer += 50;
    }
}
