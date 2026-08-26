use std::ffi::{c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
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

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(lhs: *const c_char, rhs: *const c_char) -> c_int;
    fn memcmp(lhs: *const c_void, rhs: *const c_void, count: usize) -> c_int;
    fn sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
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
    unsafe { (array as *mut ArrayHeader).sub(1) }
}

#[inline]
unsafe fn array_length(array: *mut c_void) -> usize {
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
    unsafe { (hash as *mut u8).sub(element_size).cast() }
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { (array as *mut u8).add(element_size).cast() }
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    unsafe { (*header(array)).hash_table.cast() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    element_size: usize,
    add_length: usize,
    mut minimum_capacity: usize,
) -> *mut c_void {
    let minimum_length = unsafe { array_length(array) }.wrapping_add(add_length);
    if minimum_length > minimum_capacity {
        minimum_capacity = minimum_length;
    }

    let old_capacity = unsafe { array_capacity(array) };
    if minimum_capacity <= old_capacity {
        return array;
    }

    if minimum_capacity < old_capacity.wrapping_mul(2) {
        minimum_capacity = old_capacity.wrapping_mul(2);
    } else if minimum_capacity < 4 {
        minimum_capacity = 4;
    }

    let allocation = unsafe {
        realloc(
            if array.is_null() {
                ptr::null_mut()
            } else {
                header(array).cast()
            },
            element_size
                .wrapping_mul(minimum_capacity)
                .wrapping_add(size_of::<ArrayHeader>()),
        )
    };
    let result = unsafe {
        (allocation as *mut u8)
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
    unsafe {
        (*header(result)).capacity = minimum_capacity;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(array: *mut c_void) {
    unsafe {
        free(header(array).cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        HASH_SEED = seed;
    }
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

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = unsafe { realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>() };
    let storage_address =
        ((unsafe { table.add(1) } as usize) + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);

    unsafe {
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
            (*table).used_count_threshold + (*table).tombstone_count_threshold
                < (*table).slot_count
        );

        if !old.is_null() {
            (*table).string = (*old).string;
            (*table).seed = (*old).seed;
        } else {
            (*table).string = zeroed();
            (*table).seed = HASH_SEED;

            let a32 = 2_147_001_325usize;
            let a = ((0x27bb_2ee6usize) << 32) ^ ((0x87b0_b0fdusize ^ a32) & 0xffff_ffff) ^ a32;
            let b32 = 715_136_305usize;
            let b = ((0usize) << 32) ^ ((0xb504_f32dusize ^ b32) & 0xffff_ffff) ^ b32;
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
            for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
                let old_bucket = (*old).storage.add(old_bucket_index);
                for old_item in 0..BUCKET_LENGTH {
                    if (*old_bucket).index[old_item] >= 0 {
                        let item_hash = (*old_bucket).hash[old_item];
                        let mut position = probe_position(item_hash, (*table).slot_count);
                        let mut step = BUCKET_LENGTH;

                        'probe: loop {
                            let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
                            for item in (position & BUCKET_MASK)..BUCKET_LENGTH {
                                if (*bucket).hash[item] == HASH_EMPTY {
                                    (*bucket).hash[item] = item_hash;
                                    (*bucket).index[item] = (*old_bucket).index[old_item];
                                    break 'probe;
                                }
                            }

                            let limit = position & BUCKET_MASK;
                            for item in 0..limit {
                                if (*bucket).hash[item] == HASH_EMPTY {
                                    (*bucket).hash[item] = item_hash;
                                    (*bucket).index[item] = (*old_bucket).index[old_item];
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
    }
    table
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

unsafe fn siphash_bytes(data_pointer: *mut c_void, length: usize, seed: usize) -> usize {
    let mut data_pointer = data_pointer.cast::<u8>();
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut offset = 0usize;
    while offset + size_of::<usize>() <= length {
        let bytes = unsafe { std::slice::from_raw_parts(data_pointer, 8) };
        let low = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let high = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        // Match the C integer promotions: d[3] << 24 is a signed int
        // before conversion to size_t, and GCC sign-extends that value.
        let word = (low as i32 as isize as usize) | ((high as usize) << 32);
        v3 ^= word;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= word;
        offset += size_of::<usize>();
        data_pointer = unsafe { data_pointer.add(size_of::<usize>()) };
    }

    let mut tail = length.wrapping_shl(56);
    let remaining = length - offset;
    unsafe {
        if remaining >= 7 {
            tail |= (*data_pointer.add(6) as usize) << 48;
        }
        if remaining >= 6 {
            tail |= (*data_pointer.add(5) as usize) << 40;
        }
        if remaining >= 5 {
            tail |= (*data_pointer.add(4) as usize) << 32;
        }
        if remaining >= 4 {
            let promoted = ((*data_pointer.add(3) as u32) << 24) as i32;
            tail |= promoted as isize as usize;
        }
        if remaining >= 3 {
            tail |= (*data_pointer.add(2) as usize) << 16;
        }
        if remaining >= 2 {
            tail |= (*data_pointer.add(1) as usize) << 8;
        }
        if remaining >= 1 {
            tail |= *data_pointer as usize;
        }
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
    unsafe { siphash_bytes(data, length, seed) }
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
    let item_key = unsafe {
        (array as *mut u8)
            .add(element_size.wrapping_mul(index))
            .add(key_offset)
    };
    if mode >= HM_STRING {
        unsafe { strcmp(key.cast(), *(item_key.cast::<*mut c_char>())) == 0 }
    } else {
        unsafe { memcmp(key, item_key.cast(), key_size) == 0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, element_size: usize) {
    if array.is_null() {
        return;
    }

    let table = unsafe { hash_table(array) };
    if !table.is_null() {
        unsafe {
            if (*table).string.mode == SH_STRDUP {
                for index in 1..(*header(array)).length {
                    let string_pointer = (array as *mut u8)
                        .add(element_size.wrapping_mul(index))
                        .cast::<*mut c_void>();
                    free(*string_pointer);
                }
            }
            stbds_strreset(ptr::addr_of_mut!((*table).string));
        }
    }

    unsafe {
        free((*header(array)).hash_table);
        free(header(array).cast());
    }
}

unsafe fn find_hash_slot(
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_array = unsafe { hash_to_array(hash_array, element_size) };
    let table = unsafe { hash_table(raw_array) };
    let mut item_hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key.cast(), (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if item_hash < 2 {
        item_hash += 2;
    }

    let mut step = BUCKET_LENGTH;
    let mut position = unsafe { probe_position(item_hash, (*table).slot_count) };
    loop {
        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
        for item in (position & BUCKET_MASK)..BUCKET_LENGTH {
            unsafe {
                if (*bucket).hash[item] == item_hash {
                    if is_key_equal(
                        hash_array,
                        element_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        (*bucket).index[item] as usize,
                    ) {
                        return ((position & !BUCKET_MASK) + item) as isize;
                    }
                } else if (*bucket).hash[item] == HASH_EMPTY {
                    return INDEX_EMPTY;
                }
            }
        }

        let limit = position & BUCKET_MASK;
        for item in 0..limit {
            unsafe {
                if (*bucket).hash[item] == item_hash {
                    if is_key_equal(
                        hash_array,
                        element_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        (*bucket).index[item] as usize,
                    ) {
                        return ((position & !BUCKET_MASK) + item) as isize;
                    }
                } else if (*bucket).hash[item] == HASH_EMPTY {
                    return INDEX_EMPTY;
                }
            }
        }

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= unsafe { (*table).slot_count - 1 };
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
        hash_array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
        unsafe {
            (*header(hash_array)).length += 1;
            ptr::write_bytes(hash_array, 0, element_size);
            *temporary = INDEX_EMPTY;
            array_to_hash(hash_array, element_size)
        }
    } else {
        let raw_array = unsafe { hash_to_array(hash_array, element_size) };
        let table = unsafe { hash_table(raw_array) };
        unsafe {
            if table.is_null() {
                *temporary = INDEX_EMPTY;
            } else {
                let slot = find_hash_slot(hash_array, element_size, key, key_size, 0, mode);
                if slot < 0 {
                    *temporary = INDEX_EMPTY;
                } else {
                    let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                    *temporary = (*bucket).index[slot as usize & BUCKET_MASK];
                }
            }
        }
        hash_array
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    hash_array: *mut c_void,
    element_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temporary = 0isize;
    let result = unsafe {
        stbds_hmget_key_ts(
            hash_array,
            element_size,
            key,
            key_size,
            &mut temporary,
            mode,
        )
    };
    let raw_array = unsafe { hash_to_array(result, element_size) };
    unsafe {
        (*header(raw_array)).temp = temporary;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut hash_array: *mut c_void,
    element_size: usize,
) -> *mut c_void {
    if hash_array.is_null()
        || unsafe { (*header(hash_to_array(hash_array, element_size))).length == 0 }
    {
        let raw_array = if hash_array.is_null() {
            ptr::null_mut()
        } else {
            unsafe { hash_to_array(hash_array, element_size) }
        };
        hash_array = unsafe { stbds_arrgrowf(raw_array, element_size, 0, 1) };
        unsafe {
            (*header(hash_array)).length += 1;
            ptr::write_bytes(hash_array, 0, element_size);
            hash_array = array_to_hash(hash_array, element_size);
        }
    }
    hash_array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let length = unsafe { strlen(string) + 1 };
    let result = unsafe { realloc(ptr::null_mut(), length).cast::<c_char>() };
    unsafe {
        ptr::copy(string, result, length);
    }
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
        let array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
        unsafe {
            ptr::write_bytes(array, 0, element_size);
            (*header(array)).length += 1;
            hash_array = array_to_hash(array, element_size);
        }
    }

    let raw_hash_array = hash_array;
    let mut array = unsafe { hash_to_array(hash_array, element_size) };
    let mut table = unsafe { hash_table(array) };

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
            (*header(array)).hash_table = new_table.cast();
            table = new_table;
        }
    }

    let mut item_hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key.cast(), (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, key_size, (*table).seed) }
    };
    if item_hash < 2 {
        item_hash += 2;
    }

    let mut step = BUCKET_LENGTH;
    let mut position = unsafe { probe_position(item_hash, (*table).slot_count) };
    let mut tombstone = INDEX_EMPTY;

    'search: loop {
        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
        for item in (position & BUCKET_MASK)..BUCKET_LENGTH {
            unsafe {
                if (*bucket).hash[item] == item_hash {
                    if is_key_equal(
                        raw_hash_array,
                        element_size,
                        key,
                        key_size,
                        0,
                        mode,
                        (*bucket).index[item] as usize,
                    ) {
                        (*header(array)).temp = (*bucket).index[item];
                        if mode >= HM_STRING {
                            (*table).temp_key = *((raw_hash_array as *mut u8)
                                .add(element_size.wrapping_mul((*bucket).index[item] as usize))
                                .cast::<*mut c_char>());
                        }
                        return array_to_hash(array, element_size);
                    }
                } else if (*bucket).hash[item] == HASH_EMPTY {
                    position = (position & !BUCKET_MASK) + item;
                    break 'search;
                } else if tombstone < 0 && (*bucket).index[item] == INDEX_DELETED {
                    tombstone = ((position & !BUCKET_MASK) + item) as isize;
                }
            }
        }

        let limit = position & BUCKET_MASK;
        for item in 0..limit {
            unsafe {
                if (*bucket).hash[item] == item_hash {
                    if is_key_equal(
                        raw_hash_array,
                        element_size,
                        key,
                        key_size,
                        0,
                        mode,
                        (*bucket).index[item] as usize,
                    ) {
                        (*header(array)).temp = (*bucket).index[item];
                        return array_to_hash(array, element_size);
                    }
                } else if (*bucket).hash[item] == HASH_EMPTY {
                    position = (position & !BUCKET_MASK) + item;
                    break 'search;
                } else if tombstone < 0 && (*bucket).index[item] == INDEX_DELETED {
                    tombstone = ((position & !BUCKET_MASK) + item) as isize;
                }
            }
        }

        position = position.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        position &= unsafe { (*table).slot_count - 1 };
    }

    unsafe {
        if tombstone >= 0 {
            position = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        let item_index = array_length(array) as isize;
        if item_index as usize + 1 > array_capacity(array) {
            array = stbds_arrgrowf(array, element_size, 1, 0);
        }
        assert!(item_index as usize + 1 <= array_capacity(array));
        (*header(array)).length = item_index as usize + 1;
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        (*bucket).hash[position & BUCKET_MASK] = item_hash;
        (*bucket).index[position & BUCKET_MASK] = item_index - 1;
        (*header(array)).temp = item_index - 1;

        let item_key = (array as *mut u8)
            .add(element_size.wrapping_mul(item_index as usize))
            .cast::<*mut c_char>();
        match (*table).string.mode {
            SH_STRDUP => {
                let stored = duplicate_string(key.cast());
                *item_key = stored;
                (*table).temp_key = stored;
            }
            SH_ARENA => {
                let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key.cast());
                *item_key = stored;
                (*table).temp_key = stored;
            }
            SH_DEFAULT => {
                let stored = key.cast::<c_char>();
                *item_key = stored;
                (*table).temp_key = stored;
            }
            _ => {
                ptr::copy_nonoverlapping(key.cast::<u8>(), item_key.cast::<u8>(), key_size);
            }
        }
        array_to_hash(array, element_size)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(element_size: usize, mode: c_int) -> *mut c_void {
    let array = unsafe { stbds_arrgrowf(ptr::null_mut(), element_size, 0, 1) };
    unsafe {
        ptr::write_bytes(array, 0, element_size);
        (*header(array)).length = 1;
        let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
        (*header(array)).hash_table = table.cast();
        (*table).string.mode = mode as u8;
        array_to_hash(array, element_size)
    }
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

    let raw_array = unsafe { hash_to_array(hash_array, element_size) };
    let mut table = unsafe { hash_table(raw_array) };
    unsafe {
        (*header(raw_array)).temp = 0;
    }
    if table.is_null() {
        return hash_array;
    }

    let mut slot =
        unsafe { find_hash_slot(hash_array, element_size, key, key_size, key_offset, mode) };
    if slot < 0 {
        return hash_array;
    }

    unsafe {
        let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        let mut bucket_item = slot as usize & BUCKET_MASK;
        let old_index = (*bucket).index[bucket_item];
        let final_index = array_length(raw_array) as isize - 2;
        assert!(slot < (*table).slot_count as isize);
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_array)).temp = 1;
        (*bucket).hash[bucket_item] = HASH_DELETED;
        (*bucket).index[bucket_item] = INDEX_DELETED;

        if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
            let stored = *((hash_array as *mut u8)
                .add(element_size.wrapping_mul(old_index as usize))
                .cast::<*mut c_void>());
            free(stored);
        }

        if old_index != final_index {
            ptr::copy(
                (hash_array as *mut u8).add(element_size.wrapping_mul(final_index as usize)),
                (hash_array as *mut u8).add(element_size.wrapping_mul(old_index as usize)),
                element_size,
            );

            let moved_key = (hash_array as *mut u8)
                .add(element_size.wrapping_mul(old_index as usize))
                .add(key_offset);
            let lookup_key = if mode == HM_STRING {
                *(moved_key.cast::<*mut c_void>())
            } else {
                moved_key.cast()
            };
            slot = find_hash_slot(
                hash_array,
                element_size,
                lookup_key,
                key_size,
                key_offset,
                mode,
            );
            assert!(slot >= 0);
            bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
            bucket_item = slot as usize & BUCKET_MASK;
            assert!((*bucket).index[bucket_item] == final_index);
            (*bucket).index[bucket_item] = old_index;
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
    }
    hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let length = unsafe { strlen(string) + 1 };
    unsafe {
        if length > (*arena).remaining {
            let block_size = 512usize << ((*arena).block >> 1);
            if block_size < (1usize << 20) {
                (*arena).block = (*arena).block.wrapping_add(1);
            }

            if length > block_size {
                let block = realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + length)
                    .cast::<StringBlock>();
                ptr::copy(string, ptr::addr_of_mut!((*block).storage).cast(), length);
                if !(*arena).storage.is_null() {
                    (*block).next = (*(*arena).storage).next;
                    (*(*arena).storage).next = block;
                } else {
                    (*block).next = ptr::null_mut();
                    (*arena).storage = block;
                    (*arena).remaining = 0;
                }
                return ptr::addr_of_mut!((*block).storage).cast();
            } else {
                let block = realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + block_size)
                    .cast::<StringBlock>();
                (*block).next = (*arena).storage;
                (*arena).storage = block;
                (*arena).remaining = block_size;
            }
        }

        assert!(length <= (*arena).remaining);
        let result = ptr::addr_of_mut!((*(*arena).storage).storage)
            .cast::<c_char>()
            .add((*arena).remaining - length);
        (*arena).remaining -= length;
        ptr::copy(string, result, length);
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
        ptr::write_bytes(arena, 0, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    unsafe {
        sprintf(buffer, FORMAT.as_ptr().cast(), number);
    }
    buffer
}

#[repr(C)]
struct StringMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[inline]
unsafe fn string_map_index(map: &mut *mut StringMapEntry, key: *mut c_char) -> isize {
    *map = unsafe {
        stbds_hmget_key(
            (*map).cast(),
            size_of::<StringMapEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            HM_STRING,
        )
        .cast()
    };
    let raw_array = unsafe {
        ((*map).cast::<u8>())
            .sub(size_of::<StringMapEntry>())
            .cast::<c_void>()
    };
    unsafe { (*header(raw_array)).temp }
}

#[inline]
unsafe fn string_map_get(map: &mut *mut StringMapEntry, key: *mut c_char) -> c_int {
    let index = unsafe { string_map_index(map, key) };
    unsafe { (*(*map).offset(index)).value }
}

#[inline]
unsafe fn string_map_length(map: *mut StringMapEntry) -> isize {
    if map.is_null() {
        0
    } else {
        let raw_array = unsafe {
            map.cast::<u8>()
                .sub(size_of::<StringMapEntry>())
                .cast::<c_void>()
        };
        unsafe { (*header(raw_array)).length as isize - 1 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(number: c_int) {
    const FOO: &[u8] = b"foo\0";
    const PRINT_FORMAT: &[u8] = b"%s %d\n\0";

    let mut string_map: *mut StringMapEntry = ptr::null_mut();
    let mut arena: StringArena = unsafe { zeroed() };

    let mut index = 0;
    while index < number {
        let key = unsafe { strkey(index) };
        unsafe {
            stbds_stralloc(&mut arena, key);
        }
        index += 1;
    }
    unsafe {
        stbds_strreset(&mut arena);
    }

    for pass in 0..2 {
        assert_eq!(
            unsafe { string_map_index(&mut string_map, FOO.as_ptr() as *mut c_char) },
            INDEX_EMPTY
        );
        string_map = unsafe {
            stbds_shmode_func(
                size_of::<StringMapEntry>(),
                if pass == 0 {
                    SH_STRDUP as c_int
                } else {
                    SH_ARENA as c_int
                },
            )
            .cast()
        };
        assert_eq!(
            unsafe { string_map_index(&mut string_map, FOO.as_ptr() as *mut c_char) },
            INDEX_EMPTY
        );

        string_map =
            unsafe { stbds_hmput_default(string_map.cast(), size_of::<StringMapEntry>()).cast() };
        unsafe {
            (*string_map.offset(-1)).value = -2;
        }
        assert_eq!(
            unsafe { string_map_index(&mut string_map, FOO.as_ptr() as *mut c_char) },
            INDEX_EMPTY
        );

        index = 0;
        while index < number {
            let key = unsafe { strkey(index) };
            string_map = unsafe {
                stbds_hmput_key(
                    string_map.cast(),
                    size_of::<StringMapEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                )
                .cast()
            };
            let raw_array = unsafe {
                string_map
                    .cast::<u8>()
                    .sub(size_of::<StringMapEntry>())
                    .cast::<c_void>()
            };
            let map_index = unsafe { (*header(raw_array)).temp };
            unsafe {
                (*string_map.offset(map_index)).value = index.wrapping_mul(3);
            }
            index = index.wrapping_add(2);
        }

        let mut print_index = 0isize;
        while print_index < unsafe { string_map_length(string_map) } {
            let entry = unsafe { &*string_map.offset(print_index) };
            unsafe {
                printf(PRINT_FORMAT.as_ptr().cast(), entry.key, entry.value);
            }
            print_index += 1;
        }

        index = 0;
        while index < number {
            let actual = unsafe { string_map_get(&mut string_map, strkey(index)) };
            if index & 1 != 0 {
                assert_eq!(actual, -2);
            } else {
                assert_eq!(actual, index.wrapping_mul(3));
            }
            index += 1;
        }

        index = 2;
        while index < number {
            let key = unsafe { strkey(index) };
            string_map = unsafe {
                stbds_hmdel_key(
                    string_map.cast(),
                    size_of::<StringMapEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                )
                .cast()
            };
            index = index.wrapping_add(4);
        }

        index = 0;
        while index < number {
            let actual = unsafe { string_map_get(&mut string_map, strkey(index)) };
            if index & 3 != 0 {
                assert_eq!(actual, -2);
            } else {
                assert_eq!(actual, index.wrapping_mul(3));
            }
            index += 1;
        }

        index = 0;
        while index < number {
            let key = unsafe { strkey(index) };
            string_map = unsafe {
                stbds_hmdel_key(
                    string_map.cast(),
                    size_of::<StringMapEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                )
                .cast()
            };
            index += 1;
        }

        index = 0;
        while index < number {
            assert_eq!(
                unsafe { string_map_get(&mut string_map, strkey(index)) },
                -2
            );
            index += 1;
        }

        let raw_array = unsafe {
            string_map
                .cast::<u8>()
                .sub(size_of::<StringMapEntry>())
                .cast::<c_void>()
        };
        unsafe {
            stbds_hmfree_func(raw_array, size_of::<StringMapEntry>());
        }
        string_map = ptr::null_mut();
    }
}
