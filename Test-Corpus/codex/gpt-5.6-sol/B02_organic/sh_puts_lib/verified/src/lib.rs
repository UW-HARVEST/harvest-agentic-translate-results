#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::{align_of, size_of};
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

const ARENA_BLOCKSIZE_MIN: usize = 512;
const ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

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
    fn memcmp(left: *const c_void, right: *const c_void, len: usize) -> c_int;
    fn memmove(dest: *mut c_void, src: *const c_void, len: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(a: *mut c_void) -> *mut ArrayHeader {
    a.cast::<ArrayHeader>().sub(1)
}

#[inline]
unsafe fn arr_len(a: *mut c_void) -> usize {
    if a.is_null() { 0 } else { (*header(a)).length }
}

#[inline]
unsafe fn arr_cap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elem_size: usize) -> *mut c_void {
    a.cast::<u8>().sub(elem_size).cast()
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elem_size: usize) -> *mut c_void {
    a.cast::<u8>().add(elem_size).cast()
}

#[inline]
unsafe fn hash_table(raw: *mut c_void) -> *mut HashIndex {
    (*header(raw)).hash_table.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elem_size: usize,
    add_len: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = arr_len(a).wrapping_add(add_len);
    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= arr_cap(a) {
        return a;
    }
    if min_cap < 2usize.wrapping_mul(arr_cap(a)) {
        min_cap = 2usize.wrapping_mul(arr_cap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a).cast()
    };
    let allocation_size = elem_size
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<ArrayHeader>());
    let allocation = realloc(old, allocation_size);
    let b = allocation
        .cast::<u8>()
        .add(size_of::<ArrayHeader>())
        .cast::<c_void>();

    if a.is_null() {
        (*header(b)).length = 0;
        (*header(b)).hash_table = ptr::null_mut();
        (*header(b)).temp = 0;
    }
    (*header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(header(a).cast());
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
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let bucket_bytes = (slot_count >> BUCKET_SHIFT).wrapping_mul(size_of::<HashBucket>());
    let allocation_size = bucket_bytes
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>();

    let storage_address =
        (table.add(1) as usize).wrapping_add(CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);
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
        (*table)
            .used_count_threshold
            .wrapping_add((*table).tombstone_count_threshold)
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
        ptr::write_bytes(
            ptr::addr_of_mut!((*table).string).cast::<u8>(),
            0,
            size_of::<StringArena>(),
        );
        (*table).seed = HASH_SEED;

        let a = if size_of::<usize>() == 8 {
            0x27bb_2ee6_87b0_b0fdu64 as usize
        } else {
            2_147_001_325usize
        };
        let b = if size_of::<usize>() == 8 {
            0x0000_0000_b504_f32du64 as usize
        } else {
            715_136_305usize
        };
        HASH_SEED = HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    for i in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = (*table).storage.add(i);
        for j in 0..BUCKET_LENGTH {
            (*bucket).hash[j] = HASH_EMPTY;
        }
        for j in 0..BUCKET_LENGTH {
            (*bucket).index[j] = INDEX_EMPTY;
        }
    }

    if !old.is_null() {
        (*table).used_count = (*old).used_count;
        for i in 0..((*old).slot_count >> BUCKET_SHIFT) {
            let old_bucket = (*old).storage.add(i);
            for j in 0..BUCKET_LENGTH {
                if (*old_bucket).index[j] >= 0 {
                    let hash = (*old_bucket).hash[j];
                    let mut pos = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
                        for z in (pos & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[z] == HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*old_bucket).index[j];
                                break 'probe;
                            }
                        }
                        for z in 0..(pos & BUCKET_MASK) {
                            if (*bucket).hash[z] == HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*old_bucket).index[j];
                                break 'probe;
                            }
                        }
                        pos = pos.wrapping_add(step) & ((*table).slot_count - 1);
                        step = step.wrapping_add(BUCKET_LENGTH);
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
        hash = hash.rotate_left(9).wrapping_add((*string as u8) as usize);
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

unsafe fn siphash_bytes(data_pointer: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_pointer = data_pointer.cast::<u8>();
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
        let low_word = (*data_pointer as u32)
            | ((*data_pointer.add(1) as u32) << 8)
            | ((*data_pointer.add(2) as u32) << 16)
            | ((*data_pointer.add(3) as u32) << 24);
        // The C expression is evaluated as signed int before conversion to size_t.
        let mut data = (low_word as i32 as isize) as usize;
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
        data_pointer = data_pointer.add(size_of::<usize>());
        i += size_of::<usize>();
    }

    let mut data = len << (usize::BITS - 8);
    let remaining = len - i;
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
        data |= (((*data_pointer.add(3) as i32) << 24) as isize) as usize;
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
    siphash_bytes(data, len, seed)
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored = a
        .cast::<u8>()
        .add(elem_size.wrapping_mul(index))
        .add(key_offset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *(stored.cast::<*mut c_char>())) == 0
    } else {
        memcmp(key, stored.cast(), key_size) == 0
    }
}

unsafe fn find_slot(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw = hash_to_arr(a, elem_size);
    let table = hash_table(raw);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, key_size, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut pos = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
        for i in (pos & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    a,
                    elem_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((pos & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return -1;
            }
        }
        for i in 0..(pos & BUCKET_MASK) {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    a,
                    elem_size,
                    key,
                    key_size,
                    key_offset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((pos & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return -1;
            }
        }
        pos = pos.wrapping_add(step) & ((*table).slot_count - 1);
        step = step.wrapping_add(BUCKET_LENGTH);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elem_size: usize) {
    if a.is_null() {
        return;
    }
    let table = hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for i in 1..(*header(a)).length {
                free(*(a.cast::<u8>().add(elem_size * i).cast::<*mut c_void>()));
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*table).string));
    }
    free((*header(a)).hash_table);
    free(header(a).cast());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1);
        (*header(a)).length += 1;
        ptr::write_bytes(a, 0, elem_size);
        *temp = INDEX_EMPTY;
        arr_to_hash(a, elem_size)
    } else {
        let raw = hash_to_arr(a, elem_size);
        let table = hash_table(raw);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = find_slot(a, elem_size, key, key_size, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
                *temp = (*bucket).index[(slot as usize) & BUCKET_MASK];
            }
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0;
    let result = stbds_hmget_key_ts(a, elem_size, key, key_size, &mut temp, mode);
    (*header(hash_to_arr(result, elem_size))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elem_size: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elem_size))).length == 0 {
        a = stbds_arrgrowf(
            if a.is_null() {
                ptr::null_mut()
            } else {
                hash_to_arr(a, elem_size)
            },
            elem_size,
            0,
            1,
        );
        (*header(a)).length += 1;
        ptr::write_bytes(a, 0, elem_size);
        a = arr_to_hash(a, elem_size);
    }
    a
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let copy = realloc(ptr::null_mut(), len).cast::<c_char>();
    memmove(copy.cast(), string.cast(), len);
    copy
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1);
        ptr::write_bytes(a, 0, elem_size);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elem_size);
    }

    let raw_a = a;
    a = hash_to_arr(a, elem_size);
    let mut table = hash_table(a);
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
        (*header(a)).hash_table = new_table.cast();
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
    let mut pos = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    let mut tombstone: isize = -1;

    'search: loop {
        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
        for i in (pos & BUCKET_MASK)..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    raw_a,
                    elem_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    (*header(a)).temp = (*bucket).index[i];
                    if mode >= HM_STRING {
                        (*table).temp_key = *(raw_a
                            .cast::<u8>()
                            .add(elem_size * ((*bucket).index[i] as usize))
                            .cast::<*mut c_char>());
                    }
                    return arr_to_hash(a, elem_size);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
        }
        for i in 0..(pos & BUCKET_MASK) {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    raw_a,
                    elem_size,
                    key,
                    key_size,
                    0,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    (*header(a)).temp = (*bucket).index[i];
                    return arr_to_hash(a, elem_size);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
        }
        pos = pos.wrapping_add(step) & ((*table).slot_count - 1);
        step = step.wrapping_add(BUCKET_LENGTH);
    }

    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = arr_len(a);
    if i + 1 > arr_cap(a) {
        a = stbds_arrgrowf(a, elem_size, 1, 0);
    }
    assert!(i.wrapping_add(1) <= arr_cap(a));
    (*header(a)).length = i + 1;

    let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
    (*bucket).hash[pos & BUCKET_MASK] = hash;
    (*bucket).index[pos & BUCKET_MASK] = i as isize - 1;
    (*header(a)).temp = i as isize - 1;

    let destination = a.cast::<u8>().add(elem_size * i);
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
            *destination.cast::<*mut c_char>() = key.cast();
            (*table).temp_key = key.cast();
        }
        _ => {
            ptr::copy_nonoverlapping(key.cast::<u8>(), destination, key_size);
        }
    }
    arr_to_hash(a, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1);
    ptr::write_bytes(a, 0, elem_size);
    (*header(a)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    arr_to_hash(a, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw = hash_to_arr(a, elem_size);
    let mut table = hash_table(raw);
    (*header(raw)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = find_slot(a, elem_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return a;
    }

    let mut bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
    let mut bucket_index = (slot as usize) & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = arr_len(raw) as isize - 2;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw)).temp = 1;
    assert!((*table).used_count <= usize::MAX);
    (*bucket).hash[bucket_index] = HASH_DELETED;
    (*bucket).index[bucket_index] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        free(
            *(a.cast::<u8>()
                .add(elem_size * old_index as usize)
                .cast::<*mut c_void>()),
        );
    }

    if old_index != final_index {
        memmove(
            a.cast::<u8>().add(elem_size * old_index as usize).cast(),
            a.cast::<u8>().add(elem_size * final_index as usize).cast(),
            elem_size,
        );
        let moved_key = if mode == HM_STRING {
            *(a.cast::<u8>()
                .add(elem_size * old_index as usize + key_offset)
                .cast::<*mut c_void>())
        } else {
            a.cast::<u8>()
                .add(elem_size * old_index as usize + key_offset)
                .cast()
        };
        slot = find_slot(a, elem_size, moved_key, key_size, key_offset, mode);
        assert!(slot >= 0);
        bucket = (*table).storage.add((slot as usize) >> BUCKET_SHIFT);
        bucket_index = (slot as usize) & BUCKET_MASK;
        assert!((*bucket).index[bucket_index] == final_index);
        (*bucket).index[bucket_index] = old_index;
    }
    (*header(raw)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let replacement = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw)).hash_table = replacement.cast();
        free(table.cast());
        table = replacement;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let replacement = make_hash_index((*table).slot_count, table);
        (*header(raw)).hash_table = replacement.cast();
        free(table.cast());
        table = replacement;
    }
    let _ = table;
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = strlen(string) + 1;
    if len > (*arena).remaining {
        let block_size = ARENA_BLOCKSIZE_MIN << ((*arena).block >> 1);
        if block_size < ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > block_size {
            let allocation_size = size_of::<StringBlock>() - 8 + len;
            let block = realloc(ptr::null_mut(), allocation_size).cast::<StringBlock>();
            memmove((*block).storage.as_mut_ptr().cast(), string.cast(), len);
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

        let allocation_size = size_of::<StringBlock>() - 8 + block_size;
        let block = realloc(ptr::null_mut(), allocation_size).cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = block_size;
    }

    assert!(len <= (*arena).remaining);
    let destination = (*(*arena).storage)
        .storage
        .as_mut_ptr()
        .add((*arena).remaining - len);
    (*arena).remaining -= len;
    memmove(destination.cast(), string.cast(), len);
    destination
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(arena: *mut StringArena) {
    let mut block = (*arena).storage;
    while !block.is_null() {
        let next = (*block).next;
        free(block.cast());
        block = next;
    }
    ptr::write_bytes(arena.cast::<u8>(), 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(number: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    sprintf(buffer, FORMAT.as_ptr().cast(), number);
    buffer
}

#[repr(C)]
struct StringMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    let mut arena: StringArena = std::mem::zeroed();
    let mut i = 0;
    while i < num {
        stbds_stralloc(&mut arena, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut arena);

    const KEY: &[u8] = b"a\0";
    const PRINT_FORMAT: &[u8] = b"%s %d\n\0";
    let mut map = stbds_shmode_func(size_of::<StringMapEntry>(), SH_ARENA as c_int);
    map = stbds_hmput_key(
        map,
        size_of::<StringMapEntry>(),
        KEY.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );
    let raw = hash_to_arr(map, size_of::<StringMapEntry>());
    let index = (*header(raw)).temp as usize;
    let entry = map.cast::<StringMapEntry>().add(index);
    (*entry).value = num;
    assert!(*(*map.cast::<StringMapEntry>()).key == b'a' as c_char);
    assert!((*map.cast::<StringMapEntry>()).key != KEY.as_ptr().cast_mut().cast());
    assert!((*map.cast::<StringMapEntry>()).value == num);

    let length = (*header(raw)).length - 1;
    for z in 0..length {
        let entry = map.cast::<StringMapEntry>().add(z);
        printf(PRINT_FORMAT.as_ptr().cast(), (*entry).key, (*entry).value);
    }
    stbds_hmfree_func(raw, size_of::<StringMapEntry>());
}

const _: () = {
    assert!(size_of::<ArrayHeader>() == 4 * size_of::<usize>());
    assert!(align_of::<HashIndex>() == align_of::<usize>());
};
