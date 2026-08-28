use std::ffi::{c_char, c_int, c_void};
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

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<ArrayHeader>().sub(1) }
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
unsafe fn array_cap(array: *mut c_void) -> usize {
    if array.is_null() {
        0
    } else {
        unsafe { (*header(array)).capacity }
    }
}

#[inline]
unsafe fn hash_to_array(hash: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { hash.cast::<u8>().sub(elem_size).cast() }
}

#[inline]
unsafe fn array_to_hash(array: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { array.cast::<u8>().add(elem_size).cast() }
}

#[inline]
unsafe fn hash_table(array: *mut c_void) -> *mut HashIndex {
    unsafe { (*header(array)).hash_table.cast() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    array: *mut c_void,
    elem_size: usize,
    add_len: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = unsafe { array_len(array) }.wrapping_add(add_len);
    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= unsafe { array_cap(array) } {
        return array;
    }

    if min_cap < 2usize.wrapping_mul(unsafe { array_cap(array) }) {
        min_cap = 2usize.wrapping_mul(unsafe { array_cap(array) });
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let allocation = if array.is_null() {
        ptr::null_mut()
    } else {
        unsafe { header(array).cast() }
    };
    let base = unsafe {
        realloc(
            allocation,
            elem_size
                .wrapping_mul(min_cap)
                .wrapping_add(size_of::<ArrayHeader>()),
        )
    };
    let result = unsafe { base.cast::<u8>().add(size_of::<ArrayHeader>()).cast() };

    if array.is_null() {
        unsafe {
            (*header(result)).length = 0;
            (*header(result)).hash_table = ptr::null_mut();
            (*header(result)).temp = 0;
        }
    }
    unsafe {
        (*header(result)).capacity = min_cap;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(array: *mut c_void) {
    unsafe { free(header(array).cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe { HASH_SEED = seed };
}

#[inline]
fn rotate_left(value: usize, count: u32) -> usize {
    value.rotate_left(count)
}

#[inline]
fn rotate_right(value: usize, count: u32) -> usize {
    value.rotate_right(count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut string: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while unsafe { *string } != 0 {
        hash = rotate_left(hash, 9).wrapping_add(unsafe { *string as u8 as usize });
        string = unsafe { string.add(1) };
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

#[inline]
fn sip_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, usize::BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, usize::BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
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

    let mut i = 0;
    while i + size_of::<usize>() <= len {
        let low = unsafe {
            (*data_ptr.add(0) as i32
                | (*data_ptr.add(1) as i32) << 8
                | (*data_ptr.add(2) as i32) << 16
                | (*data_ptr.add(3) as i32) << 24) as isize as usize
        };
        let high = unsafe {
            *data_ptr.add(4) as usize
                | (*data_ptr.add(5) as usize) << 8
                | (*data_ptr.add(6) as usize) << 16
                | (*data_ptr.add(7) as usize) << 24
        };
        let word = low | (high << 32);

        v3 ^= word;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= word;

        i += size_of::<usize>();
        data_ptr = unsafe { data_ptr.add(size_of::<usize>()) };
    }

    let mut word = len << (usize::BITS - 8);
    let remaining = len - i;
    if remaining >= 7 {
        word |= (unsafe { *data_ptr.add(6) } as usize) << 48;
    }
    if remaining >= 6 {
        word |= (unsafe { *data_ptr.add(5) } as usize) << 40;
    }
    if remaining >= 5 {
        word |= (unsafe { *data_ptr.add(4) } as usize) << 32;
    }
    if remaining >= 4 {
        word |= ((unsafe { *data_ptr.add(3) } as i32) << 24) as isize as usize;
    }
    if remaining >= 3 {
        word |= (unsafe { *data_ptr.add(2) } as usize) << 16;
    }
    if remaining >= 2 {
        word |= (unsafe { *data_ptr.add(1) } as usize) << 8;
    }
    if remaining >= 1 {
        word |= unsafe { *data_ptr } as usize;
    }

    v3 ^= word;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= word;
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
            ptr::copy_nonoverlapping(
                ptr::addr_of!((*old).string),
                ptr::addr_of_mut!((*table).string),
                1,
            );
            (*table).seed = (*old).seed;
        } else {
            ptr::write_bytes(ptr::addr_of_mut!((*table).string), 0, 1);
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
            HASH_SEED = HASH_SEED.wrapping_mul(multiplier).wrapping_add(increment);
        }

        for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
            let bucket = (*table).storage.add(bucket_index);
            (*bucket).hash.fill(HASH_EMPTY);
            (*bucket).index.fill(INDEX_EMPTY);
        }
    }

    if !old.is_null() {
        unsafe {
            (*table).used_count = (*old).used_count;
        }
        for old_bucket_index in 0..(unsafe { (*old).slot_count } >> BUCKET_SHIFT) {
            let old_bucket = unsafe { (*old).storage.add(old_bucket_index) };
            for old_slot in 0..BUCKET_LENGTH {
                let old_index = unsafe { (*old_bucket).index[old_slot] };
                if old_index >= 0 {
                    let hash = unsafe { (*old_bucket).hash[old_slot] };
                    let mut position = probe_position(hash, slot_count);
                    let mut step = BUCKET_LENGTH;
                    loop {
                        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
                        let start = position & BUCKET_MASK;
                        let mut destination = None;
                        for slot in start..BUCKET_LENGTH {
                            if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                                destination = Some(slot);
                                break;
                            }
                        }
                        if destination.is_none() {
                            for slot in 0..start {
                                if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                                    destination = Some(slot);
                                    break;
                                }
                            }
                        }
                        if let Some(slot) = destination {
                            unsafe {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = old_index;
                            }
                            break;
                        }
                        position = position.wrapping_add(step) & (slot_count - 1);
                        step = step.wrapping_add(BUCKET_LENGTH);
                    }
                }
            }
        }
    }

    table
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
    let stored = unsafe {
        array
            .cast::<u8>()
            .add(elem_size.wrapping_mul(index))
            .add(key_offset)
    };
    if mode >= HM_STRING {
        let stored_string = unsafe { stored.cast::<*mut c_char>().read_unaligned() };
        unsafe { strcmp(key.cast(), stored_string) == 0 }
    } else {
        unsafe {
            std::slice::from_raw_parts(key.cast::<u8>(), key_size)
                == std::slice::from_raw_parts(stored, key_size)
        }
    }
}

unsafe fn find_slot(
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

    let mut position = probe_position(hash, unsafe { (*table).slot_count });
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = unsafe { (*table).storage.add(position >> BUCKET_SHIFT) };
        let start = position & BUCKET_MASK;

        for slot in start..BUCKET_LENGTH {
            if unsafe { (*bucket).hash[slot] } == hash {
                if unsafe {
                    is_key_equal(
                        hash_array,
                        elem_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        (*bucket).index[slot] as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                return -1;
            }
        }

        for slot in 0..start {
            if unsafe { (*bucket).hash[slot] } == hash {
                if unsafe {
                    is_key_equal(
                        hash_array,
                        elem_size,
                        key,
                        key_size,
                        key_offset,
                        mode,
                        (*bucket).index[slot] as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                return -1;
            }
        }

        position = position.wrapping_add(step) & (unsafe { (*table).slot_count } - 1);
        step = step.wrapping_add(BUCKET_LENGTH);
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
        array = unsafe { stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1) };
        unsafe {
            (*header(array)).length += 1;
            ptr::write_bytes(array, 0, elem_size);
            *temp = INDEX_EMPTY;
            array_to_hash(array, elem_size)
        }
    } else {
        let raw_array = unsafe { hash_to_array(array, elem_size) };
        let table = unsafe { hash_table(raw_array) };
        if table.is_null() {
            unsafe { *temp = INDEX_EMPTY };
        } else {
            let slot = unsafe { find_slot(array, elem_size, key, key_size, 0, mode) };
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
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
    let result = unsafe { stbds_hmget_key_ts(array, elem_size, key, key_size, &mut temp, mode) };
    let raw_array = unsafe { hash_to_array(result, elem_size) };
    unsafe { (*header(raw_array)).temp = temp };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut array: *mut c_void,
    elem_size: usize,
) -> *mut c_void {
    if array.is_null() || unsafe { (*header(hash_to_array(array, elem_size))).length } == 0 {
        let raw_array = if array.is_null() {
            ptr::null_mut()
        } else {
            unsafe { hash_to_array(array, elem_size) }
        };
        array = unsafe { stbds_arrgrowf(raw_array, elem_size, 0, 1) };
        unsafe {
            (*header(array)).length += 1;
            ptr::write_bytes(array, 0, elem_size);
            array = array_to_hash(array, elem_size);
        }
    }
    array
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = unsafe { strlen(string) } + 1;
    let copy = unsafe { realloc(ptr::null_mut(), len).cast::<c_char>() };
    unsafe { ptr::copy(string, copy, len) };
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
            ptr::write_bytes(raw_array, 0, elem_size);
            (*header(raw_array)).length += 1;
            hash_array = array_to_hash(raw_array, elem_size);
        }
    }

    let comparison_array = hash_array;
    let mut raw_array = unsafe { hash_to_array(hash_array, elem_size) };
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
        let start = position & BUCKET_MASK;

        for slot in start..BUCKET_LENGTH {
            if unsafe { (*bucket).hash[slot] } == hash {
                let index = unsafe { (*bucket).index[slot] };
                if unsafe {
                    is_key_equal(
                        comparison_array,
                        elem_size,
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
                            (*table).temp_key = comparison_array
                                .cast::<u8>()
                                .add(elem_size.wrapping_mul(index as usize))
                                .cast::<*mut c_char>()
                                .read_unaligned();
                        }
                        return array_to_hash(raw_array, elem_size);
                    }
                }
            } else if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && unsafe { (*bucket).index[slot] } == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        for slot in 0..start {
            if unsafe { (*bucket).hash[slot] } == hash {
                let index = unsafe { (*bucket).index[slot] };
                if unsafe {
                    is_key_equal(
                        comparison_array,
                        elem_size,
                        key,
                        key_size,
                        0,
                        mode,
                        index as usize,
                    )
                } {
                    unsafe {
                        (*header(raw_array)).temp = index;
                        return array_to_hash(raw_array, elem_size);
                    }
                }
            } else if unsafe { (*bucket).hash[slot] } == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && unsafe { (*bucket).index[slot] } == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        position = position.wrapping_add(step) & (unsafe { (*table).slot_count } - 1);
        step = step.wrapping_add(BUCKET_LENGTH);
    }

    if tombstone >= 0 {
        position = tombstone as usize;
        unsafe { (*table).tombstone_count -= 1 };
    }
    unsafe { (*table).used_count += 1 };

    let insertion_index = unsafe { array_len(raw_array) } as isize;
    if insertion_index as usize + 1 > unsafe { array_cap(raw_array) } {
        raw_array = unsafe { stbds_arrgrowf(raw_array, elem_size, 1, 0) };
    }
    unsafe {
        assert!(insertion_index as usize + 1 <= array_cap(raw_array));
        (*header(raw_array)).length = insertion_index as usize + 1;
        let bucket = (*table).storage.add(position >> BUCKET_SHIFT);
        (*bucket).hash[position & BUCKET_MASK] = hash;
        (*bucket).index[position & BUCKET_MASK] = insertion_index - 1;
        (*header(raw_array)).temp = insertion_index - 1;

        let destination = raw_array
            .cast::<u8>()
            .add(elem_size.wrapping_mul(insertion_index as usize));
        match (*table).string.mode {
            SH_STRDUP => {
                let stored = duplicate_string(key.cast());
                destination.cast::<*mut c_char>().write_unaligned(stored);
                (*table).temp_key = stored;
            }
            SH_ARENA => {
                let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key.cast());
                destination.cast::<*mut c_char>().write_unaligned(stored);
                (*table).temp_key = stored;
            }
            SH_DEFAULT => {
                destination
                    .cast::<*mut c_char>()
                    .write_unaligned(key.cast());
                (*table).temp_key = key.cast();
            }
            _ => ptr::copy(key.cast::<u8>(), destination, key_size),
        }
        array_to_hash(raw_array, elem_size)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let array = unsafe { stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1) };
    unsafe {
        ptr::write_bytes(array, 0, elem_size);
        (*header(array)).length = 1;
        let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
        (*header(array)).hash_table = table.cast();
        (*table).string.mode = mode as u8;
        array_to_hash(array, elem_size)
    }
}

#[unsafe(no_mangle)]
#[allow(unused_comparisons)]
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

    let raw_array = unsafe { hash_to_array(hash_array, elem_size) };
    let mut table = unsafe { hash_table(raw_array) };
    unsafe { (*header(raw_array)).temp = 0 };
    if table.is_null() {
        return hash_array;
    }

    let mut slot = unsafe { find_slot(hash_array, elem_size, key, key_size, key_offset, mode) };
    if slot < 0 {
        return hash_array;
    }

    let mut bucket = unsafe { (*table).storage.add(slot as usize >> BUCKET_SHIFT) };
    let mut bucket_slot = slot as usize & BUCKET_MASK;
    let old_index = unsafe { (*bucket).index[bucket_slot] };
    let final_index = unsafe { array_len(raw_array) } as isize - 2;

    unsafe {
        assert!(slot < (*table).slot_count as isize);
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_array)).temp = 1;
        assert!((*table).used_count >= 0);
        (*bucket).hash[bucket_slot] = HASH_DELETED;
        (*bucket).index[bucket_slot] = INDEX_DELETED;
    }

    if mode == HM_STRING && unsafe { (*table).string.mode } == SH_STRDUP {
        let string = unsafe {
            hash_array
                .cast::<u8>()
                .add(elem_size.wrapping_mul(old_index as usize))
                .cast::<*mut c_char>()
                .read_unaligned()
        };
        unsafe { free(string.cast()) };
    }

    if old_index != final_index {
        unsafe {
            ptr::copy(
                hash_array
                    .cast::<u8>()
                    .add(elem_size.wrapping_mul(final_index as usize)),
                hash_array
                    .cast::<u8>()
                    .add(elem_size.wrapping_mul(old_index as usize)),
                elem_size,
            );
        }

        let moved_key = if mode == HM_STRING {
            unsafe {
                hash_array
                    .cast::<u8>()
                    .add(elem_size.wrapping_mul(old_index as usize))
                    .add(key_offset)
                    .cast::<*mut c_char>()
                    .read_unaligned()
                    .cast()
            }
        } else {
            unsafe {
                hash_array
                    .cast::<u8>()
                    .add(elem_size.wrapping_mul(old_index as usize))
                    .add(key_offset)
                    .cast()
            }
        };
        slot = unsafe { find_slot(hash_array, elem_size, moved_key, key_size, key_offset, mode) };
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
        let replacement = unsafe { make_hash_index((*table).slot_count >> 1, table) };
        unsafe {
            (*header(raw_array)).hash_table = replacement.cast();
            free(table.cast());
        }
        table = replacement;
    } else if unsafe { (*table).tombstone_count > (*table).tombstone_count_threshold } {
        let replacement = unsafe { make_hash_index((*table).slot_count, table) };
        unsafe {
            (*header(raw_array)).hash_table = replacement.cast();
            free(table.cast());
        }
        table = replacement;
    }
    let _ = table;

    hash_array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = unsafe { strlen(string) } + 1;
    if len > unsafe { (*arena).remaining } {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << (unsafe { (*arena).block } >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            unsafe { (*arena).block += 1 };
        }

        if len > block_size {
            let block = unsafe {
                realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + len).cast::<StringBlock>()
            };
            unsafe {
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

    assert!(len <= unsafe { (*arena).remaining });
    let destination = unsafe {
        ptr::addr_of_mut!((*(*arena).storage).storage)
            .cast::<c_char>()
            .add((*arena).remaining - len)
    };
    unsafe {
        (*arena).remaining -= len;
        ptr::copy(string, destination, len);
    }
    destination
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut current = unsafe { (*arena).storage };
    while !current.is_null() {
        let next = unsafe { (*current).next };
        unsafe { free(current.cast()) };
        current = next;
    }
    unsafe { ptr::write_bytes(arena, 0, 1) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(array: *mut c_void, elem_size: usize) {
    if array.is_null() {
        return;
    }
    let table = unsafe { hash_table(array) };
    if !table.is_null() {
        if unsafe { (*table).string.mode } == SH_STRDUP {
            for index in 1..unsafe { (*header(array)).length } {
                let string = unsafe {
                    array
                        .cast::<u8>()
                        .add(elem_size.wrapping_mul(index))
                        .cast::<*mut c_char>()
                        .read_unaligned()
                };
                unsafe { free(string.cast()) };
            }
        }
        unsafe { stbds_strreset(ptr::addr_of_mut!((*table).string)) };
    }
    unsafe {
        free((*header(array)).hash_table);
        free(header(array).cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(BUFFER).cast::<c_char>();
    unsafe {
        sprintf(buffer, FORMAT.as_ptr().cast(), number);
    }
    buffer
}

#[repr(C)]
struct IntEntry {
    key: c_int,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(number: c_int) {
    let mut map = ptr::null_mut::<IntEntry>();

    unsafe fn put(map: &mut *mut IntEntry, key: c_int, value: c_int) {
        let mut key_argument = key;
        let key_ptr = ptr::addr_of_mut!(key_argument);
        *map = unsafe {
            stbds_hmput_key(
                (*map).cast(),
                size_of::<IntEntry>(),
                key_ptr.cast(),
                size_of::<c_int>(),
                0,
            )
            .cast()
        };
        let raw_array = unsafe { (*map).sub(1).cast::<c_void>() };
        let index = unsafe { (*header(raw_array)).temp };
        unsafe {
            (*(*map).add(index as usize)).key = key;
            (*(*map).add(index as usize)).value = value;
        }
    }

    unsafe fn get(map: &mut *mut IntEntry, key: c_int) -> c_int {
        let mut key_argument = key;
        *map = unsafe {
            stbds_hmget_key(
                (*map).cast(),
                size_of::<IntEntry>(),
                ptr::addr_of_mut!(key_argument).cast(),
                size_of::<c_int>(),
                0,
            )
            .cast()
        };
        let raw_array = unsafe { (*map).sub(1).cast::<c_void>() };
        let index = unsafe { (*header(raw_array)).temp };
        unsafe { (*(*map).offset(index)).value }
    }

    unsafe {
        put(&mut map, number, 7);
        put(&mut map, 11, 3);
        put(&mut map, 9, number);
        assert_eq!(get(&mut map, 9), number);
        assert_eq!(get(&mut map, 11), 3);
        assert_eq!(get(&mut map, number), 7);
    }
}
