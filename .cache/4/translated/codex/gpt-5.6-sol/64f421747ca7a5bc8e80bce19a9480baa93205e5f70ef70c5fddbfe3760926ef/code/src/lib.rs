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
    storage: [u8; 8],
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
    #[link_name = "realloc"]
    fn c_realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn c_free(ptr: *mut c_void);
    #[link_name = "sprintf"]
    fn c_sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [u8; 256] = [0; 256];

#[inline]
unsafe fn header(a: *mut c_void) -> *mut ArrayHeader {
    unsafe { (a as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader }
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn table_for_raw(a: *mut c_void) -> *mut HashIndex {
    unsafe { (*header(a)).hash_table as *mut HashIndex }
}

#[inline]
unsafe fn array_length(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length }
    }
}

#[inline]
unsafe fn array_capacity(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).capacity }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = unsafe { array_length(a) }.wrapping_add(addlen);
    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= unsafe { array_capacity(a) } {
        return a;
    }

    let old_capacity = unsafe { array_capacity(a) };
    if min_cap < 2usize.wrapping_mul(old_capacity) {
        min_cap = 2usize.wrapping_mul(old_capacity);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let allocation = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<ArrayHeader>());
    let old_header = if a.is_null() {
        ptr::null_mut()
    } else {
        unsafe { header(a) as *mut c_void }
    };
    let base = unsafe { c_realloc(old_header, allocation) };
    let b = unsafe { (base as *mut u8).add(size_of::<ArrayHeader>()) as *mut c_void };

    if a.is_null() {
        unsafe {
            (*header(b)).length = 0;
            (*header(b)).hash_table = ptr::null_mut();
            (*header(b)).temp = 0;
        }
    }
    unsafe {
        (*header(b)).capacity = min_cap;
    }
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe { c_free(header(a) as *mut c_void) };
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

#[inline]
fn integer_log2(mut slot_count: usize) -> usize {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = unsafe { c_realloc(ptr::null_mut(), allocation) as *mut HashIndex };
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

        if !old.is_null() {
            (*table).string = (*old).string;
            (*table).seed = (*old).seed;
        } else {
            ptr::write_bytes(
                ptr::addr_of_mut!((*table).string) as *mut u8,
                0,
                size_of::<StringArena>(),
            );
            (*table).seed = HASH_SEED;
            HASH_SEED = HASH_SEED
                .wrapping_mul(0x27bb_2ee6_87b0_b0fd)
                .wrapping_add(0xb504_f32d);
        }

        for bucket_index in 0..(slot_count >> BUCKET_SHIFT) {
            let bucket = &mut *(*table).storage.add(bucket_index);
            for value in &mut bucket.hash {
                *value = HASH_EMPTY;
            }
            for value in &mut bucket.index {
                *value = INDEX_EMPTY;
            }
        }

        if !old.is_null() {
            (*table).used_count = (*old).used_count;
            for old_bucket_index in 0..((*old).slot_count >> BUCKET_SHIFT) {
                let old_bucket = &*(*old).storage.add(old_bucket_index);
                for old_slot in 0..BUCKET_LENGTH {
                    if old_bucket.index[old_slot] >= 0 {
                        let hash = old_bucket.hash[old_slot];
                        let mut position = probe_position(hash, (*table).slot_count);
                        let mut step = BUCKET_LENGTH;
                        'probe: loop {
                            let bucket = &mut *(*table).storage.add(position >> BUCKET_SHIFT);
                            for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
                                if bucket.hash[slot] == HASH_EMPTY {
                                    bucket.hash[slot] = hash;
                                    bucket.index[slot] = old_bucket.index[old_slot];
                                    break 'probe;
                                }
                            }
                            for slot in 0..(position & BUCKET_MASK) {
                                if bucket.hash[slot] == HASH_EMPTY {
                                    bucket.hash[slot] = hash;
                                    bucket.index[slot] = old_bucket.index[old_slot];
                                    break 'probe;
                                }
                            }
                            position = position.wrapping_add(step) & ((*table).slot_count - 1);
                            step = step.wrapping_add(BUCKET_LENGTH);
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
            hash = hash.rotate_left(9).wrapping_add(*string as u8 as usize);
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

unsafe fn siphash_bytes(data_pointer: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_pointer = data_pointer as *const u8;
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;

    v0 ^= 0x0706_0504_0302_0100 ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908 ^ !seed;
    v2 ^= 0x0706_0504_0302_0100 ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908 ^ !seed;

    let mut offset = 0;
    while offset + size_of::<usize>() <= len {
        let low_32 = unsafe {
            *data_pointer.add(0) as u32
                | (*data_pointer.add(1) as u32) << 8
                | (*data_pointer.add(2) as u32) << 16
                | (*data_pointer.add(3) as u32) << 24
        };
        // Match GCC's sign extension of the C expression `d[3] << 24`.
        let low = low_32 as i32 as isize as usize;
        let high = unsafe {
            *data_pointer.add(4) as usize
                | (*data_pointer.add(5) as usize) << 8
                | (*data_pointer.add(6) as usize) << 16
                | (*data_pointer.add(7) as usize) << 24
        };
        let word = low | (high << 32);
        v3 ^= word;
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        v0 ^= word;
        offset += size_of::<usize>();
        data_pointer = unsafe { data_pointer.add(size_of::<usize>()) };
    }

    let mut word = len.wrapping_shl(56);
    let remaining = len - offset;
    if remaining >= 7 {
        word |= unsafe { (*data_pointer.add(6) as usize) << 48 };
    }
    if remaining >= 6 {
        word |= unsafe { (*data_pointer.add(5) as usize) << 40 };
    }
    if remaining >= 5 {
        word |= unsafe { (*data_pointer.add(4) as usize) << 32 };
    }
    if remaining >= 4 {
        let shifted = unsafe { (*data_pointer.add(3) as u32) << 24 };
        word |= shifted as i32 as isize as usize;
    }
    if remaining >= 3 {
        word |= unsafe { (*data_pointer.add(2) as usize) << 16 };
    }
    if remaining >= 2 {
        word |= unsafe { (*data_pointer.add(1) as usize) << 8 };
    }
    if remaining >= 1 {
        word |= unsafe { *data_pointer as usize };
    }

    v3 ^= word;
    sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    v0 ^= word;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { siphash_bytes(p, len, seed) }
}

unsafe fn c_strings_equal(mut left: *const c_char, mut right: *const c_char) -> bool {
    unsafe {
        loop {
            if *left != *right {
                return false;
            }
            if *left == 0 {
                return true;
            }
            left = left.add(1);
            right = right.add(1);
        }
    }
}

unsafe fn key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored = unsafe {
        (a as *mut u8)
            .add(elemsize.wrapping_mul(index))
            .add(keyoffset)
    };
    if mode >= HM_STRING {
        let stored_string = unsafe { *(stored as *mut *mut c_char) };
        unsafe { c_strings_equal(key as *const c_char, stored_string) }
    } else {
        unsafe {
            std::slice::from_raw_parts(key as *const u8, keysize)
                == std::slice::from_raw_parts(stored, keysize)
        }
    }
}

unsafe fn find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw = unsafe { hash_to_arr(a, elemsize) };
    let table = unsafe { table_for_raw(raw) };
    let mut hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key as *mut c_char, (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, keysize, (*table).seed) }
    };
    if hash < 2 {
        hash += 2;
    }

    let mut position = unsafe { probe_position(hash, (*table).slot_count) };
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = unsafe { &*(*table).storage.add(position >> BUCKET_SHIFT) };
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if bucket.hash[slot] == hash {
                if unsafe {
                    key_equal(
                        a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                return INDEX_EMPTY;
            }
        }
        for slot in 0..(position & BUCKET_MASK) {
            if bucket.hash[slot] == hash {
                if unsafe {
                    key_equal(
                        a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    return ((position & !BUCKET_MASK) + slot) as isize;
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                return INDEX_EMPTY;
            }
        }
        position = unsafe { position.wrapping_add(step) & ((*table).slot_count - 1) };
        step = step.wrapping_add(BUCKET_LENGTH);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = unsafe { stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) };
        unsafe {
            (*header(a)).length += 1;
            ptr::write_bytes(a as *mut u8, 0, elemsize);
            *temp = INDEX_EMPTY;
            arr_to_hash(a, elemsize)
        }
    } else {
        let raw = unsafe { hash_to_arr(a, elemsize) };
        let table = unsafe { table_for_raw(raw) };
        unsafe {
            if table.is_null() {
                *temp = INDEX_EMPTY;
            } else {
                let slot = find_slot(a, elemsize, key, keysize, 0, mode);
                if slot < 0 {
                    *temp = INDEX_EMPTY;
                } else {
                    let bucket = &*(*table).storage.add(slot as usize >> BUCKET_SHIFT);
                    *temp = bucket.index[slot as usize & BUCKET_MASK];
                }
            }
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
    let result = unsafe { stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode) };
    let raw = unsafe { hash_to_arr(result, elemsize) };
    unsafe {
        (*header(raw)).temp = temp;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    let needs_default = a.is_null() || unsafe { (*header(hash_to_arr(a, elemsize))).length == 0 };
    if needs_default {
        let raw = if a.is_null() {
            ptr::null_mut()
        } else {
            unsafe { hash_to_arr(a, elemsize) }
        };
        a = unsafe { stbds_arrgrowf(raw, elemsize, 0, 1) };
        unsafe {
            (*header(a)).length += 1;
            ptr::write_bytes(a as *mut u8, 0, elemsize);
            a = arr_to_hash(a, elemsize);
        }
    }
    a
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let mut len = 0;
    unsafe {
        while *string.add(len) != 0 {
            len += 1;
        }
        len += 1;
        let copy = c_realloc(ptr::null_mut(), len) as *mut c_char;
        ptr::copy(string, copy, len);
        copy
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = unsafe { stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) };
        unsafe {
            ptr::write_bytes(a as *mut u8, 0, elemsize);
            (*header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }
    }

    let raw_a = a;
    let mut array = unsafe { hash_to_arr(a, elemsize) };
    let mut table = unsafe { table_for_raw(array) };

    if table.is_null() || unsafe { (*table).used_count >= (*table).used_count_threshold } {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            unsafe { (*table).slot_count.wrapping_mul(2) }
        };
        let new_table = unsafe { make_hash_index(slot_count, table) };
        if !table.is_null() {
            unsafe { c_free(table as *mut c_void) };
        } else {
            unsafe {
                (*new_table).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 };
            }
        }
        table = new_table;
        unsafe {
            (*header(array)).hash_table = table as *mut c_void;
        }
    }

    let mut hash = if mode >= HM_STRING {
        unsafe { stbds_hash_string(key as *mut c_char, (*table).seed) }
    } else {
        unsafe { stbds_hash_bytes(key, keysize, (*table).seed) }
    };
    if hash < 2 {
        hash += 2;
    }

    let mut position = unsafe { probe_position(hash, (*table).slot_count) };
    let mut step = BUCKET_LENGTH;
    let mut tombstone = INDEX_EMPTY;

    'search: loop {
        let bucket = unsafe { &mut *(*table).storage.add(position >> BUCKET_SHIFT) };
        for slot in (position & BUCKET_MASK)..BUCKET_LENGTH {
            if bucket.hash[slot] == hash {
                if unsafe {
                    key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        0,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    unsafe {
                        (*header(array)).temp = bucket.index[slot];
                        if mode >= HM_STRING {
                            (*table).temp_key = *((raw_a as *mut u8)
                                .add(elemsize * bucket.index[slot] as usize)
                                as *mut *mut c_char);
                        }
                        return arr_to_hash(array, elemsize);
                    }
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && bucket.index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        for slot in 0..(position & BUCKET_MASK) {
            if bucket.hash[slot] == hash {
                if unsafe {
                    key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        0,
                        mode,
                        bucket.index[slot] as usize,
                    )
                } {
                    unsafe {
                        (*header(array)).temp = bucket.index[slot];
                        return arr_to_hash(array, elemsize);
                    }
                }
            } else if bucket.hash[slot] == HASH_EMPTY {
                position = (position & !BUCKET_MASK) + slot;
                break 'search;
            } else if tombstone < 0 && bucket.index[slot] == INDEX_DELETED {
                tombstone = ((position & !BUCKET_MASK) + slot) as isize;
            }
        }

        position = unsafe { position.wrapping_add(step) & ((*table).slot_count - 1) };
        step = step.wrapping_add(BUCKET_LENGTH);
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

    let index = unsafe { array_length(array) as isize };
    if index as usize + 1 > unsafe { array_capacity(array) } {
        array = unsafe { stbds_arrgrowf(array, elemsize, 1, 0) };
    }
    unsafe {
        (*header(array)).length = index as usize + 1;
        let bucket = &mut *(*table).storage.add(position >> BUCKET_SHIFT);
        bucket.hash[position & BUCKET_MASK] = hash;
        bucket.index[position & BUCKET_MASK] = index - 1;
        (*header(array)).temp = index - 1;

        let destination = (array as *mut u8).add(elemsize * index as usize);
        match (*table).string.mode {
            SH_STRDUP => {
                let stored = duplicate_string(key as *mut c_char);
                *(destination as *mut *mut c_char) = stored;
                (*table).temp_key = stored;
            }
            SH_ARENA => {
                let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key as *mut c_char);
                *(destination as *mut *mut c_char) = stored;
                (*table).temp_key = stored;
            }
            SH_DEFAULT => {
                *(destination as *mut *mut c_char) = key as *mut c_char;
                (*table).temp_key = key as *mut c_char;
            }
            _ => {
                ptr::copy(key as *const u8, destination, keysize);
            }
        }
        arr_to_hash(array, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let array = unsafe { stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) };
    unsafe {
        ptr::write_bytes(array as *mut u8, 0, elemsize);
        (*header(array)).length = 1;
        let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
        (*header(array)).hash_table = table as *mut c_void;
        (*table).string.mode = mode as u8;
        arr_to_hash(array, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw = unsafe { hash_to_arr(a, elemsize) };
    let mut table = unsafe { table_for_raw(raw) };
    unsafe {
        (*header(raw)).temp = 0;
    }
    if table.is_null() {
        return a;
    }

    let mut slot = unsafe { find_slot(a, elemsize, key, keysize, keyoffset, mode) };
    if slot < 0 {
        return a;
    }

    unsafe {
        let mut bucket = &mut *(*table).storage.add(slot as usize >> BUCKET_SHIFT);
        let mut bucket_slot = slot as usize & BUCKET_MASK;
        let old_index = bucket.index[bucket_slot];
        let final_index = array_length(raw) as isize - 2;

        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw)).temp = 1;
        bucket.hash[bucket_slot] = HASH_DELETED;
        bucket.index[bucket_slot] = INDEX_DELETED;

        if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
            let stored = *((a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_void);
            c_free(stored);
        }

        if old_index != final_index {
            ptr::copy(
                (a as *mut u8).add(elemsize * final_index as usize),
                (a as *mut u8).add(elemsize * old_index as usize),
                elemsize,
            );

            let moved_key = if mode == HM_STRING {
                *((a as *mut u8)
                    .add(elemsize * old_index as usize)
                    .add(keyoffset) as *mut *mut c_void)
            } else {
                (a as *mut u8)
                    .add(elemsize * old_index as usize)
                    .add(keyoffset) as *mut c_void
            };
            slot = find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
            bucket = &mut *(*table).storage.add(slot as usize >> BUCKET_SHIFT);
            bucket_slot = slot as usize & BUCKET_MASK;
            bucket.index[bucket_slot] = old_index;
        }
        (*header(raw)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > BUCKET_LENGTH
        {
            let new_table = make_hash_index((*table).slot_count >> 1, table);
            (*header(raw)).hash_table = new_table as *mut c_void;
            c_free(table as *mut c_void);
            table = new_table;
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            let new_table = make_hash_index((*table).slot_count, table);
            (*header(raw)).hash_table = new_table as *mut c_void;
            c_free(table as *mut c_void);
            table = new_table;
        }
        let _ = table;
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = unsafe { table_for_raw(a) };
    unsafe {
        if !table.is_null() {
            if (*table).string.mode == SH_STRDUP {
                for index in 1..(*header(a)).length {
                    let stored = *((a as *mut u8).add(elemsize * index) as *mut *mut c_void);
                    c_free(stored);
                }
            }
            stbds_strreset(ptr::addr_of_mut!((*table).string));
        }
        c_free((*header(a)).hash_table);
        c_free(header(a) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let mut len = 0usize;
    unsafe {
        while *string.add(len) != 0 {
            len += 1;
        }
        len += 1;

        if len > (*arena).remaining {
            let blocksize = 512usize << ((*arena).block >> 1);
            if blocksize < (1usize << 20) {
                (*arena).block = (*arena).block.wrapping_add(1);
            }

            if len > blocksize {
                let block = c_realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + len)
                    as *mut StringBlock;
                ptr::copy(string as *const u8, (*block).storage.as_mut_ptr(), len);
                if !(*arena).storage.is_null() {
                    (*block).next = (*(*arena).storage).next;
                    (*(*arena).storage).next = block;
                } else {
                    (*block).next = ptr::null_mut();
                    (*arena).storage = block;
                    (*arena).remaining = 0;
                }
                return (*block).storage.as_mut_ptr() as *mut c_char;
            }

            let block = c_realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + blocksize)
                as *mut StringBlock;
            (*block).next = (*arena).storage;
            (*arena).storage = block;
            (*arena).remaining = blocksize;
        }

        let destination = (*(*arena).storage)
            .storage
            .as_mut_ptr()
            .add((*arena).remaining - len);
        (*arena).remaining -= len;
        ptr::copy(string as *const u8, destination, len);
        destination as *mut c_char
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    unsafe {
        let mut block = (*arena).storage;
        while !block.is_null() {
            let next = (*block).next;
            c_free(block as *mut c_void);
            block = next;
        }
        ptr::write_bytes(arena as *mut u8, 0, size_of::<StringArena>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER) as *mut u8 as *mut c_char;
    unsafe {
        c_sprintf(buffer, FORMAT.as_ptr() as *const c_char, n);
    }
    buffer
}

#[repr(C)]
struct IntEntry {
    key: c_int,
    value: c_int,
}

unsafe fn int_map_get_index(map: &mut *mut IntEntry, key: c_int) -> isize {
    *map = unsafe {
        stbds_hmget_key(
            *map as *mut c_void,
            size_of::<IntEntry>(),
            ptr::addr_of!(key) as *mut c_void,
            size_of::<c_int>(),
            0,
        ) as *mut IntEntry
    };
    let raw = unsafe { (*map).sub(1) as *mut c_void };
    unsafe { (*header(raw)).temp }
}

unsafe fn int_map_get_index_ts(map: &mut *mut IntEntry, key: c_int) -> isize {
    let mut temp = 0isize;
    *map = unsafe {
        stbds_hmget_key_ts(
            *map as *mut c_void,
            size_of::<IntEntry>(),
            ptr::addr_of!(key) as *mut c_void,
            size_of::<c_int>(),
            &mut temp,
            0,
        ) as *mut IntEntry
    };
    temp
}

unsafe fn int_map_get(map: &mut *mut IntEntry, key: c_int) -> c_int {
    let index = unsafe { int_map_get_index(map, key) };
    unsafe { (*(*map).offset(index)).value }
}

unsafe fn int_map_get_ts(map: &mut *mut IntEntry, key: c_int) -> c_int {
    let index = unsafe { int_map_get_index_ts(map, key) };
    unsafe { (*(*map).offset(index)).value }
}

unsafe fn int_map_put(map: &mut *mut IntEntry, key: c_int, value: c_int) {
    *map = unsafe {
        stbds_hmput_key(
            *map as *mut c_void,
            size_of::<IntEntry>(),
            ptr::addr_of!(key) as *mut c_void,
            size_of::<c_int>(),
            0,
        ) as *mut IntEntry
    };
    let raw = unsafe { (*map).sub(1) as *mut c_void };
    let index = unsafe { (*header(raw)).temp };
    unsafe {
        (*(*map).offset(index)).key = key;
        (*(*map).offset(index)).value = value;
    }
}

unsafe fn int_map_delete(map: &mut *mut IntEntry, key: c_int) {
    *map = unsafe {
        stbds_hmdel_key(
            *map as *mut c_void,
            size_of::<IntEntry>(),
            ptr::addr_of!(key) as *mut c_void,
            size_of::<c_int>(),
            0,
            0,
        ) as *mut IntEntry
    };
}

unsafe fn int_map_free(map: &mut *mut IntEntry) {
    if !(*map).is_null() {
        unsafe {
            stbds_hmfree_func((*map).sub(1) as *mut c_void, size_of::<IntEntry>());
        }
        *map = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    let mut map: *mut IntEntry = ptr::null_mut();

    assert_eq!(unsafe { int_map_get_index(&mut map, 1) }, -1);
    map =
        unsafe { stbds_hmput_default(map as *mut c_void, size_of::<IntEntry>()) as *mut IntEntry };
    unsafe {
        (*map.offset(-1)).value = -2;
    }
    assert_eq!(unsafe { int_map_get_index(&mut map, 1) }, -1);
    assert_eq!(unsafe { int_map_get(&mut map, 1) }, -2);

    let mut i = 0;
    while i < num {
        unsafe { int_map_put(&mut map, i, i.wrapping_mul(5)) };
        i = i.wrapping_add(2);
    }
    i = 0;
    while i < num {
        let expected = if i & 1 != 0 { -2 } else { i.wrapping_mul(5) };
        assert_eq!(unsafe { int_map_get(&mut map, i) }, expected);
        assert_eq!(unsafe { int_map_get_ts(&mut map, i) }, expected);
        i = i.wrapping_add(1);
    }
    i = 0;
    while i < num {
        unsafe { int_map_put(&mut map, i, i.wrapping_mul(3)) };
        i = i.wrapping_add(2);
    }
    i = 0;
    while i < num {
        let expected = if i & 1 != 0 { -2 } else { i.wrapping_mul(3) };
        assert_eq!(unsafe { int_map_get(&mut map, i) }, expected);
        i = i.wrapping_add(1);
    }
    i = 2;
    while i < num {
        unsafe { int_map_delete(&mut map, i) };
        i = i.wrapping_add(4);
    }
    i = 0;
    while i < num {
        let expected = if i & 3 != 0 { -2 } else { i.wrapping_mul(3) };
        assert_eq!(unsafe { int_map_get(&mut map, i) }, expected);
        i = i.wrapping_add(1);
    }
    i = 0;
    while i < num {
        unsafe { int_map_delete(&mut map, i) };
        i = i.wrapping_add(1);
    }
    i = 0;
    while i < num {
        assert_eq!(unsafe { int_map_get(&mut map, i) }, -2);
        i = i.wrapping_add(1);
    }
    unsafe { int_map_free(&mut map) };

    i = 0;
    while i < num {
        unsafe { int_map_put(&mut map, i, i.wrapping_mul(3)) };
        i = i.wrapping_add(2);
    }
    unsafe { int_map_free(&mut map) };
}
