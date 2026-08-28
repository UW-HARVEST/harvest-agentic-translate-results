#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn snprintf(dst: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
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

#[derive(Clone, Copy)]
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

static mut HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(a: *mut c_void) -> *mut ArrayHeader {
    (a as *mut ArrayHeader).sub(1)
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).sub(elemsize).cast()
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).add(elemsize).cast()
}

#[inline]
unsafe fn hash_table(raw_a: *mut c_void) -> *mut HashIndex {
    (*header(raw_a)).hash_table.cast()
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = arr_len(a).wrapping_add(addlen);
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
    let allocation_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<ArrayHeader>());
    let allocation = realloc(old, allocation_size);
    let b = (allocation as *mut u8)
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

fn integer_log2(mut value: usize) -> usize {
    let mut n = 0;
    while value > 1 {
        value >>= 1;
        n += 1;
    }
    n
}

unsafe fn make_hash_index(slot_count: usize, old: *mut HashIndex) -> *mut HashIndex {
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>();
    let storage_address = ((table.add(1) as usize) + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);

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
        (*table).used_count_threshold + (*table).tombstone_count_threshold < (*table).slot_count
    );

    if old.is_null() {
        (*table).string = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*table).seed = HASH_SEED;
        HASH_SEED = HASH_SEED
            .wrapping_mul(0x27bb_2ee6_87b0_b0fd)
            .wrapping_add(0xb504_f32d);
    } else {
        (*table).string = (*old).string;
        (*table).seed = (*old).seed;
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
            for old_slot in 0..BUCKET_LENGTH {
                if (*old_bucket).index[old_slot] >= 0 {
                    let hash = (*old_bucket).hash[old_slot];
                    let mut pos = probe_position(hash, (*table).slot_count);
                    let mut step = BUCKET_LENGTH;
                    'probe: loop {
                        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
                        let limit = pos & BUCKET_MASK;
                        for slot in limit..BUCKET_LENGTH {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot];
                                break 'probe;
                            }
                        }
                        for slot in 0..limit {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot];
                                break 'probe;
                            }
                        }
                        pos = pos.wrapping_add(step);
                        step = step.wrapping_add(BUCKET_LENGTH);
                        pos &= (*table).slot_count - 1;
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

unsafe fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut data_ptr = p.cast::<u8>();
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
        // The C expression shifts byte 3 as a signed int before converting to
        // size_t. GCC sign-extends a high-bit byte here, despite the C UB.
        let low = (*data_ptr.add(0) as u32)
            | ((*data_ptr.add(1) as u32) << 8)
            | ((*data_ptr.add(2) as u32) << 16)
            | ((*data_ptr.add(3) as u32) << 24);
        let high = (*data_ptr.add(4) as u32)
            | ((*data_ptr.add(5) as u32) << 8)
            | ((*data_ptr.add(6) as u32) << 16)
            | ((*data_ptr.add(7) as u32) << 24);
        let data = (low as i32 as isize as usize) | ((high as usize) << 32);
        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        offset += size_of::<usize>();
        data_ptr = data_ptr.add(size_of::<usize>());
    }

    let mut data = len << (usize::BITS as usize - 8);
    let remaining = len - offset;
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
        data |= ((*data_ptr.add(3) as u32) << 24) as i32 as isize as usize;
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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    index: usize,
) -> bool {
    let stored = (a as *mut u8)
        .add(elemsize.wrapping_mul(index))
        .add(keyoffset);
    if mode >= HM_STRING {
        strcmp(key.cast(), *(stored as *mut *mut c_char)) == 0
    } else {
        memcmp(key, stored.cast(), keysize) == 0
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
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table(raw_a);
    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut pos = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
        let limit = pos & BUCKET_MASK;
        for i in limit..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((pos & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return -1;
            }
        }
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as usize,
                ) {
                    return ((pos & !BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                return -1;
            }
        }
        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= (*table).slot_count - 1;
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
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*header(a)).length += 1;
        ptr::write_bytes(a, 0, elemsize);
        *temp = INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = hash_table(raw_a);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = find_slot(a, elemsize, key, keysize, 0, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
                *temp = (*bucket).index[slot as usize & BUCKET_MASK];
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
    let result = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*header(hash_to_arr(result, elemsize))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*header(a)).length += 1;
        ptr::write_bytes(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let result = realloc(ptr::null_mut(), len).cast::<c_char>();
    ptr::copy(string, result, len);
    result
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
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a, 0, elemsize);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);
    let mut table = hash_table(a);
    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let new_table = make_hash_index(slot_count, table);
        if table.is_null() {
            (*new_table).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 };
        } else {
            free(table.cast());
        }
        (*header(a)).hash_table = new_table.cast();
        table = new_table;
    }

    let mut hash = if mode >= HM_STRING {
        stbds_hash_string(key.cast(), (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut pos = probe_position(hash, (*table).slot_count);
    let mut step = BUCKET_LENGTH;
    let mut tombstone = -1isize;
    'search: loop {
        let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
        let limit = pos & BUCKET_MASK;
        for i in limit..BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                let index = (*bucket).index[i];
                if is_key_equal(raw_a, elemsize, key, keysize, 0, mode, index as usize) {
                    (*header(a)).temp = index;
                    if mode >= HM_STRING {
                        (*table).temp_key = *((raw_a as *mut u8).add(elemsize * index as usize)
                            as *mut *mut c_char);
                    }
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
        }
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                let index = (*bucket).index[i];
                if is_key_equal(raw_a, elemsize, key, keysize, 0, mode, index as usize) {
                    (*header(a)).temp = index;
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == HASH_EMPTY {
                pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && (*bucket).index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
        }
        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= (*table).slot_count - 1;
    }

    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let index = (*header(a)).length as isize;
    if index as usize + 1 > (*header(a)).capacity {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    assert!(index as usize + 1 <= (*header(a)).capacity);
    raw_a = arr_to_hash(a, elemsize);
    (*header(a)).length = index as usize + 1;
    let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
    (*bucket).hash[pos & BUCKET_MASK] = hash;
    (*bucket).index[pos & BUCKET_MASK] = index - 1;
    (*header(a)).temp = index - 1;

    let destination = (a as *mut u8).add(elemsize * index as usize);
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(&mut (*table).string, key.cast());
            *(destination as *mut *mut c_char) = stored;
            (*table).temp_key = stored;
        }
        SH_DEFAULT => {
            *(destination as *mut *mut c_char) = key.cast();
            (*table).temp_key = key.cast();
        }
        _ => ptr::copy(key.cast::<u8>(), destination, keysize),
    }
    raw_a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a, 0, elemsize);
    (*header(a)).length = 1;
    let table = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = table.cast();
    (*table).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
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

    let raw_a = hash_to_arr(a, elemsize);
    let mut table = hash_table(raw_a);
    (*header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_slot = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_slot];
    let final_index = (*header(raw_a)).length as isize - 2;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    (*bucket).hash[bucket_slot] = HASH_DELETED;
    (*bucket).index[bucket_slot] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        let stored = *((a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_void);
        free(stored);
    }

    if old_index != final_index {
        let destination = (a as *mut u8).add(elemsize * old_index as usize);
        let source = (a as *mut u8).add(elemsize * final_index as usize);
        ptr::copy(source, destination, elemsize);

        let moved_key = if mode == HM_STRING {
            *(destination.add(keyoffset) as *mut *mut c_void)
        } else {
            destination.add(keyoffset).cast()
        };
        slot = find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
        assert!(slot >= 0);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_slot = slot as usize & BUCKET_MASK;
        assert!((*bucket).index[bucket_slot] == final_index);
        (*bucket).index[bucket_slot] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > BUCKET_LENGTH
    {
        let new_table = make_hash_index((*table).slot_count >> 1, table);
        (*header(raw_a)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let new_table = make_hash_index((*table).slot_count, table);
        (*header(raw_a)).hash_table = new_table.cast();
        free(table.cast());
        table = new_table;
    }
    let _ = table;
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == SH_STRDUP {
            for index in 1..(*header(a)).length {
                let string = *((a as *mut u8).add(elemsize * index) as *mut *mut c_void);
                free(string);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*header(a)).hash_table);
    free(header(a).cast());
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
        let blocksize = STRING_ARENA_BLOCKSIZE_MIN << (((*arena).block as usize) >> 1);
        if blocksize < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > blocksize {
            let block =
                realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + len).cast::<StringBlock>();
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

        let block = realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + blocksize)
            .cast::<StringBlock>();
        (*block).next = (*arena).storage;
        (*arena).storage = block;
        (*arena).remaining = blocksize;
    }

    assert!(len <= (*arena).remaining);
    let result = ((*arena).storage as *mut u8)
        .add(size_of::<*mut StringBlock>())
        .add((*arena).remaining - len)
        .cast::<c_char>();
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
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    const FORMAT: &[u8] = b"test_%d\0";
    let buffer = ptr::addr_of_mut!(STRKEY_BUFFER).cast::<c_char>();
    snprintf(buffer, 256, FORMAT.as_ptr().cast(), n);
    buffer
}

unsafe fn push_i32(mut array: *mut c_int, value: c_int) -> *mut c_int {
    if array.is_null() || (*header(array.cast())).length + 1 > (*header(array.cast())).capacity {
        array = stbds_arrgrowf(array.cast(), size_of::<c_int>(), 1, 0).cast();
    }
    let index = (*header(array.cast())).length;
    *array.add(index) = value;
    (*header(array.cast())).length += 1;
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    for insertion_index in 0..5usize {
        let mut array: *mut c_int = ptr::null_mut();
        array = push_i32(array, 1);
        array = push_i32(array, 2);
        array = push_i32(array, 3);
        array = push_i32(array, 4);

        if (*header(array.cast())).length + 1 > (*header(array.cast())).capacity {
            array = stbds_arrgrowf(array.cast(), size_of::<c_int>(), 1, 0).cast::<c_int>();
        }
        let old_length = (*header(array.cast())).length;
        (*header(array.cast())).length += 1;
        ptr::copy(
            array.add(insertion_index),
            array.add(insertion_index + 1),
            old_length - insertion_index,
        );
        *array.add(insertion_index) = num;

        assert_eq!(*array.add(insertion_index), num);
        if insertion_index < 4 {
            assert_eq!(*array.add(4), 4);
        }
        free(header(array.cast()).cast());
    }
}
