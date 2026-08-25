#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const BUCKET_LENGTH: usize = 8;
const BUCKET_SHIFT: usize = 3;
const BUCKET_MASK: usize = BUCKET_LENGTH - 1;
const CACHE_LINE_SIZE: usize = 64;

const HM_STRING: c_int = 1;
const SH_STRDUP: u8 = 2;
const SH_ARENA: u8 = 3;

const INDEX_EMPTY: isize = -1;
const INDEX_DELETED: isize = -2;
const HASH_EMPTY: usize = 0;
const HASH_DELETED: usize = 1;

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
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, value: c_int, n: usize) -> *mut c_void;
    fn memcmp(lhs: *const c_void, rhs: *const c_void, n: usize) -> c_int;
    fn strcmp(lhs: *const c_char, rhs: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(dst: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn abort() -> !;
}

static mut HASH_SEED: usize = 0x3141_5926;
static mut BUFFER: [c_char; 256] = [0; 256];

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
unsafe fn hash_table(raw_a: *mut c_void) -> *mut HashIndex {
    (*header(raw_a)).hash_table.cast()
}

#[inline]
unsafe fn temp_key(raw_a: *mut c_void) -> *mut *mut c_char {
    (*header(raw_a)).hash_table.cast()
}

#[inline]
unsafe fn c_assert(condition: bool) {
    if !condition {
        abort();
    }
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
    let b = allocation.cast::<u8>().add(size_of::<ArrayHeader>()).cast();

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
    let allocation_size = (slot_count >> BUCKET_SHIFT)
        .wrapping_mul(size_of::<HashBucket>())
        .wrapping_add(size_of::<HashIndex>())
        .wrapping_add(CACHE_LINE_SIZE - 1);
    let table = realloc(ptr::null_mut(), allocation_size).cast::<HashIndex>();
    let storage_addr =
        ((table.add(1) as usize).wrapping_add(CACHE_LINE_SIZE - 1)) & !(CACHE_LINE_SIZE - 1);

    (*table).storage = storage_addr as *mut HashBucket;
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
    c_assert(
        (*table).used_count_threshold + (*table).tombstone_count_threshold < (*table).slot_count,
    );

    if !old.is_null() {
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*old).string),
            ptr::addr_of_mut!((*table).string),
            1,
        );
        (*table).seed = (*old).seed;
    } else {
        memset(
            ptr::addr_of_mut!((*table).string).cast(),
            0,
            size_of::<StringArena>(),
        );
        (*table).seed = HASH_SEED;
        let a = if usize::BITS == 64 {
            0x27bb_2ee6_87b0_b0fdusize
        } else {
            2_147_001_325usize
        };
        let b = if usize::BITS == 64 {
            0x0000_0000_b504_f32dusize
        } else {
            715_136_305usize
        };
        HASH_SEED = HASH_SEED.wrapping_mul(a).wrapping_add(b);
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
                        for slot in (pos & BUCKET_MASK)..BUCKET_LENGTH {
                            if (*bucket).hash[slot] == HASH_EMPTY {
                                (*bucket).hash[slot] = hash;
                                (*bucket).index[slot] = (*old_bucket).index[old_slot];
                                break 'probe;
                            }
                        }
                        for slot in 0..(pos & BUCKET_MASK) {
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
        hash = hash.rotate_left(9).wrapping_add((*string as u8) as usize);
        string = string.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash = hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash = hash.rotate_right(11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= hash.rotate_right(22);
    hash.wrapping_add(seed)
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

    #[inline]
    fn round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
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

    let mut offset = 0;
    while offset + size_of::<usize>() <= len {
        let low = i32::from_le_bytes([
            *data_ptr,
            *data_ptr.add(1),
            *data_ptr.add(2),
            *data_ptr.add(3),
        ]) as isize as usize;
        let high = u32::from_le_bytes([
            *data_ptr.add(4),
            *data_ptr.add(5),
            *data_ptr.add(6),
            *data_ptr.add(7),
        ]) as usize;
        let data = low | (high << 32);

        v3 ^= data;
        for _ in 0..2 {
            round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        offset += size_of::<usize>();
        data_ptr = data_ptr.add(size_of::<usize>());
    }

    let remaining = len - offset;
    let mut data = len << (usize::BITS - 8);
    for i in 0..remaining.min(3) {
        data |= (*data_ptr.add(i) as usize) << (i * 8);
    }
    if remaining >= 4 {
        data |= ((*data_ptr.add(3) as i32).wrapping_shl(24)) as isize as usize;
    }
    for i in 4..remaining {
        data |= (*data_ptr.add(i) as usize) << (i * 8);
    }
    v3 ^= data;
    for _ in 0..2 {
        round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
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

unsafe fn hm_find_slot(
    a: *mut c_void,
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    key_offset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elem_size);
    let table = hash_table(raw_a);
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
        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= (*table).slot_count - 1;
    }
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
        memset(a, 0, elem_size);
        *temp = INDEX_EMPTY;
        arr_to_hash(a, elem_size)
    } else {
        let raw_a = hash_to_arr(a, elem_size);
        let table = hash_table(raw_a);
        if table.is_null() {
            *temp = INDEX_EMPTY;
        } else {
            let slot = hm_find_slot(a, elem_size, key, key_size, 0, mode);
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
    elem_size: usize,
    key: *mut c_void,
    key_size: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
    let result = stbds_hmget_key_ts(a, elem_size, key, key_size, &mut temp, mode);
    (*header(hash_to_arr(result, elem_size))).temp = temp;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elem_size: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elem_size))).length == 0 {
        let raw = if a.is_null() {
            ptr::null_mut()
        } else {
            hash_to_arr(a, elem_size)
        };
        a = stbds_arrgrowf(raw, elem_size, 0, 1);
        (*header(a)).length += 1;
        memset(a, 0, elem_size);
        a = arr_to_hash(a, elem_size);
    }
    a
}

unsafe fn duplicate_string(string: *mut c_char) -> *mut c_char {
    let len = strlen(string) + 1;
    let result = realloc(ptr::null_mut(), len).cast::<c_char>();
    memmove(result.cast(), string.cast(), len);
    result
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
        memset(a, 0, elem_size);
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
            (*new_table).string.mode = if mode >= HM_STRING { 1 } else { 0 };
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
                let index = (*bucket).index[i];
                if is_key_equal(raw_a, elem_size, key, key_size, 0, mode, index as usize) {
                    (*header(a)).temp = index;
                    if mode >= HM_STRING {
                        *temp_key(a) = *(raw_a
                            .cast::<u8>()
                            .add(elem_size * index as usize)
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
                let index = (*bucket).index[i];
                if is_key_equal(raw_a, elem_size, key, key_size, 0, mode, index as usize) {
                    (*header(a)).temp = index;
                    return arr_to_hash(a, elem_size);
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
        a = stbds_arrgrowf(a, elem_size, 1, 0);
    }
    c_assert(index as usize + 1 <= (*header(a)).capacity);
    (*header(a)).length = index as usize + 1;

    let bucket = (*table).storage.add(pos >> BUCKET_SHIFT);
    (*bucket).hash[pos & BUCKET_MASK] = hash;
    (*bucket).index[pos & BUCKET_MASK] = index - 1;
    (*header(a)).temp = index - 1;

    let destination = a.cast::<u8>().add(elem_size * index as usize);
    match (*table).string.mode {
        SH_STRDUP => {
            let stored = duplicate_string(key.cast());
            *destination.cast::<*mut c_char>() = stored;
            *temp_key(a) = stored;
        }
        SH_ARENA => {
            let stored = stbds_stralloc(ptr::addr_of_mut!((*table).string), key.cast());
            *destination.cast::<*mut c_char>() = stored;
            *temp_key(a) = stored;
        }
        1 => {
            let stored = key.cast::<c_char>();
            *destination.cast::<*mut c_char>() = stored;
            *temp_key(a) = stored;
        }
        _ => {
            memmove(destination.cast(), key, key_size);
        }
    }
    arr_to_hash(a, elem_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elem_size: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elem_size, 0, 1);
    memset(a, 0, elem_size);
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

    let raw_a = hash_to_arr(a, elem_size);
    let mut table = hash_table(raw_a);
    (*header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = hm_find_slot(a, elem_size, key, key_size, key_offset, mode);
    if slot < 0 {
        return a;
    }

    let mut bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
    let mut bucket_index = slot as usize & BUCKET_MASK;
    let old_index = (*bucket).index[bucket_index];
    let final_index = (*header(raw_a)).length as isize - 2;
    c_assert(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
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
                .add(elem_size * old_index as usize)
                .add(key_offset)
                .cast::<*mut c_void>())
        } else {
            a.cast::<u8>()
                .add(elem_size * old_index as usize)
                .add(key_offset)
                .cast()
        };
        slot = hm_find_slot(a, elem_size, moved_key, key_size, key_offset, mode);
        c_assert(slot >= 0);
        bucket = (*table).storage.add(slot as usize >> BUCKET_SHIFT);
        bucket_index = slot as usize & BUCKET_MASK;
        c_assert((*bucket).index[bucket_index] == final_index);
        (*bucket).index[bucket_index] = old_index;
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
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    string: *mut c_char,
) -> *mut c_char {
    let len = strlen(string) + 1;
    if len > (*arena).remaining {
        let block_size = STRING_ARENA_BLOCKSIZE_MIN << (((*arena).block as usize) >> 1);
        if block_size < STRING_ARENA_BLOCKSIZE_MAX {
            (*arena).block = (*arena).block.wrapping_add(1);
        }

        if len > block_size {
            let block =
                realloc(ptr::null_mut(), size_of::<StringBlock>() - 8 + len).cast::<StringBlock>();
            memmove(
                ptr::addr_of_mut!((*block).storage).cast(),
                string.cast(),
                len,
            );
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

    c_assert(len <= (*arena).remaining);
    let result = ptr::addr_of_mut!((*(*arena).storage).storage)
        .cast::<c_char>()
        .add((*arena).remaining - len);
    (*arena).remaining -= len;
    memmove(result.cast(), string.cast(), len);
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
    memset(arena.cast(), 0, size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buffer = ptr::addr_of_mut!(BUFFER).cast::<c_char>();
    sprintf(buffer, c"test_%d".as_ptr(), n);
    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let mut map: *mut StringMapEntry = ptr::null_mut();
    let mut entry = StringMapEntry {
        key: ptr::null_mut(),
        value: 0,
    };
    let mut arena = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i = 0;
    while i < num {
        stbds_stralloc(&mut arena, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut arena);

    entry.key = c"a".as_ptr().cast_mut();
    entry.value = num;
    map = stbds_hmput_key(
        map.cast(),
        size_of::<StringMapEntry>(),
        entry.key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    )
    .cast();
    let raw_map = map.sub(1);
    let index = (*header(raw_map.cast())).temp as usize;
    *map.add(index) = entry;
    (*map.add(index)).key = *temp_key(raw_map.cast());

    c_assert(*(*map).key == b'a' as c_char);
    c_assert((*map).key == entry.key);
    c_assert((*map).value == entry.value);

    let length = (*header(raw_map.cast())).length as isize - 1;
    let mut z = 0isize;
    while z < length {
        let item = *map.offset(z);
        printf(c"%s %d\n".as_ptr(), item.key, item.value);
        z += 1;
    }
    stbds_hmfree_func(raw_map.cast(), size_of::<StringMapEntry>());
}
