#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_unsafe
)]
use std::ffi::c_int;
use std::ptr;

extern "C" {
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn malloc(size: usize) -> *mut u8;
    fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> c_int;
    fn memset(s: *mut u8, c: c_int, n: usize) -> *mut u8;
    fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn strcmp(s1: *const i8, s2: *const i8) -> c_int;
    fn strlen(s: *const i8) -> usize;
    fn sprintf(s: *mut i8, format: *const i8, ...) -> c_int;
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = 7;
const STBDS_CACHE_LINE_SIZE: usize = 64;
const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;
const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;
const STBDS_HM_BINARY: i32 = 0;
const STBDS_HM_STRING: i32 = 1;
const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;
const STBDS_SIZE_T_BITS: u32 = 64;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [u8; 8],
}

#[repr(C)]
struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct StbdsHashIndex {
    temp_key: *mut i8,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StbdsStringArena,
    storage: *mut StbdsHashBucket,
}

static mut stbds_hash_seed: usize = 0x31415926;

unsafe fn header(t: *mut u8) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).offset(-1)
}

unsafe fn arr_len(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*header(a)).length as isize }
}

unsafe fn arr_cap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*header(a)).capacity }
}

unsafe fn stbds_temp(a: *mut u8) -> &'static mut isize {
    &mut (*header(a)).temp
}

unsafe fn stbds_temp_key(a: *mut u8) -> &'static mut *mut i8 {
    &mut *((*header(a)).hash_table as *mut *mut i8)
}

unsafe fn hash_to_arr(a: *mut u8, elemsize: usize) -> *mut u8 {
    a.sub(elemsize)
}

unsafe fn arr_to_hash(a: *mut u8, elemsize: usize) -> *mut u8 {
    a.add(elemsize)
}

unsafe fn hash_table(a: *mut u8) -> *mut StbdsHashIndex {
    (*header(a)).hash_table as *mut StbdsHashIndex
}

fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap: usize) -> *mut u8 {
    let mut min_cap = min_cap;
    let min_len = (arr_len(a) as usize) + addlen;
    if min_len > min_cap { min_cap = min_len; }
    if min_cap <= arr_cap(a) { return a; }
    if min_cap < 2 * arr_cap(a) {
        min_cap = 2 * arr_cap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }
    let alloc_size = elemsize * min_cap + std::mem::size_of::<StbdsArrayHeader>();
    let b: *mut u8 = if a.is_null() {
        realloc(ptr::null_mut(), alloc_size)
    } else {
        realloc(header(a) as *mut u8, alloc_size)
    };
    let data = b.add(std::mem::size_of::<StbdsArrayHeader>());
    if a.is_null() {
        (*header(data)).length = 0;
        (*header(data)).hash_table = ptr::null_mut();
        (*header(data)).temp = 0;
    }
    (*header(data)).capacity = min_cap;
    data
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) {
    free(header(a) as *mut u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut i8, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut p = str as *mut u8;
    while *p != 0 {
        hash = hash.rotate_left(9).wrapping_add(*p as usize);
        p = p.add(1);
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

unsafe fn stbds_siphash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize = (((0x736f6d65_usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = (((0x646f7261_usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = (((0x6c796765_usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = (((0x74656462_usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;
    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1); v1 = v1.rotate_left(13); v1 ^= v0; v0 = v0.rotate_left(STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3); v3 = v3.rotate_left(16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = v1.rotate_left(17); v1 ^= v2; v2 = v2.rotate_left(STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3); v3 = v3.rotate_left(21); v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dp = d.add(i);
        let lo = (*dp.add(0) as i32) | ((*dp.add(1) as i32) << 8) | ((*dp.add(2) as i32) << 16) | ((*dp.add(3) as i32) << 24);
        let hi = (*dp.add(4) as i32) | ((*dp.add(5) as i32) << 8) | ((*dp.add(6) as i32) << 16) | ((*dp.add(7) as i32) << 24);
        let mut data: usize = lo as usize;
        data |= ((hi as usize) << 16) << 16;
        v3 ^= data;
        for _ in 0..2 { sipround!(); }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }
    let dp = d.add(i);
    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    if rem >= 7 { data |= ((*dp.add(6) as usize) << 24) << 24; }
    if rem >= 6 { data |= ((*dp.add(5) as usize) << 20) << 20; }
    if rem >= 5 { data |= ((*dp.add(4) as usize) << 16) << 16; }
    if rem >= 4 { data |= ((*dp.add(3) as i32) << 24) as usize; }
    if rem >= 3 { data |= ((*dp.add(2) as i32) << 16) as usize; }
    if rem >= 2 { data |= ((*dp.add(1) as i32) << 8) as usize; }
    if rem >= 1 { data |= *dp.add(0) as usize; }
    v3 ^= data;
    for _ in 0..2 { sipround!(); }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 { sipround!(); }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<StbdsHashBucket>()
        + std::mem::size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut StbdsHashIndex;
    (*t).storage = align_fwd(
        (t as *mut u8).add(std::mem::size_of::<StbdsHashIndex>()) as usize,
        STBDS_CACHE_LINE_SIZE,
    ) as *mut StbdsHashBucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;
    (*t).used_count_threshold = slot_count - (slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;
    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }
    if !ot.is_null() {
        (*t).string = ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        memset(&mut (*t).string as *mut StbdsStringArena as *mut u8, 0, std::mem::size_of::<StbdsStringArena>());
        (*t).seed = stbds_hash_seed;
        let (a, b): (usize, usize);
        let mut temp: usize;
        // stbds_load_32_or_64(a,temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
        temp = 0x87b0b0fd_usize ^ 2147001325_usize;
        temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
        let mut va: usize = 0x27bb2ee6_usize;
        va <<= 16; va <<= 16;
        va ^= temp ^ 2147001325_usize;
        a = va;
        // stbds_load_32_or_64(b,temp, 715136305, 0, 0xb504f32d)
        temp = 0xb504f32d_usize ^ 715136305_usize;
        temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
        let mut vb: usize = 0_usize;
        vb <<= 16; vb <<= 16;
        vb ^= temp ^ 715136305_usize;
        b = vb;
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }
    {
        for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
            let bucket = &mut *(*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                bucket.hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                bucket.index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let h = ob.hash[j];
                    let mut pos = stbds_probe_position(h, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = &mut *(*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = h;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }
                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = h;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }
                        pos += step;
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }
    t
}

unsafe fn stbds_is_key_equal(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: i32, i: isize) -> bool {
    if mode >= STBDS_HM_STRING {
        let p = *(a.add(elemsize * i as usize + keyoffset) as *const *const i8);
        strcmp(key as *const i8, p) == 0
    } else {
        memcmp(key, a.add(elemsize * i as usize + keyoffset), keysize) == 0
    }
}

unsafe fn stbds_hm_find_slot(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: i32) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut i8, (*table).seed)
    } else {
        stbds_siphash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    if hash < 2 { hash += 2; }
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }
        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }
        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, temp: *mut isize, mode: i32) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let a2 = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*header(a2)).length += 1;
        memset(a2, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(a2, elemsize);
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = hash_table(raw_a);
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = &*(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                *temp = b.index[slot as usize & STBDS_BUCKET_MASK];
            }
        }
        return a;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: i32) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    *stbds_temp(hash_to_arr(p, elemsize)) = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() { hash_to_arr(a, elemsize) } else { ptr::null_mut() };
        let a2 = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*header(a2)).length += 1;
        memset(a2, 0, elemsize);
        return arr_to_hash(a2, elemsize);
    }
    a
}

unsafe fn stbds_strdup(s: *mut i8) -> *mut i8 {
    let len = strlen(s as *const i8) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut i8;
    memmove(p as *mut u8, s as *const u8, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: i32) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a = a;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }
    let raw_a = a;
    let mut a = hash_to_arr(a, elemsize);
    let mut table = hash_table(a);
    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut u8);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*header(a)).hash_table = nt as *mut u8;
        table = nt;
    }
    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut i8, (*table).seed)
        } else {
            stbds_siphash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        if hash < 2 { hash += 2; }
        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
        let mut tombstone: isize = -1;
        loop {
            let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                        *stbds_temp(a) = bucket.index[i];
                        if mode >= STBDS_HM_STRING {
                            *stbds_temp_key(a) = *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset) as *mut *mut i8);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    // goto found_empty_slot
                    return hmput_key_found_empty(a, raw_a, elemsize, key, keysize, mode, table, hash, pos, tombstone);
                } else if tombstone < 0 {
                    if bucket.index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                        *stbds_temp(a) = bucket.index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    return hmput_key_found_empty(a, raw_a, elemsize, key, keysize, mode, table, hash, pos, tombstone);
                } else if tombstone < 0 {
                    if bucket.index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }
            pos += step;
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }
    }
}

unsafe fn hmput_key_found_empty(
    mut a: *mut u8, mut raw_a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize,
    mode: i32, table: *mut StbdsHashIndex, hash: usize, mut pos: usize, tombstone: isize,
) -> *mut u8 {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;
    let i = arr_len(a);
    if (i as usize) + 1 > arr_cap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
        raw_a = arr_to_hash(a, elemsize);
    }
    (*header(a)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(a) = i - 1;
    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key as *mut i8);
            *(a.add(elemsize * i as usize) as *mut *mut i8) = dup;
            *stbds_temp_key(a) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(&mut (*table).string as *mut StbdsStringArena as *mut u8, key as *mut i8);
            *(a.add(elemsize * i as usize) as *mut *mut i8) = s;
            *stbds_temp_key(a) = s;
        }
        STBDS_SH_DEFAULT => {
            *(a.add(elemsize * i as usize) as *mut *mut i8) = key as *mut i8;
            *stbds_temp_key(a) = key as *mut i8;
        }
        _ => {
            memmove(a.add(elemsize * i as usize), key, keysize);
        }
    }
    arr_to_hash(a, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() { return; }
    let ht = hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*header(a)).length {
                free(*(a.add(elemsize * i) as *mut *mut u8));
            }
        }
        stbds_strreset(&mut (*ht).string as *mut StbdsStringArena as *mut u8);
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: i32) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*header(a)).hash_table = h as *mut u8;
    arr_to_hash(a, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: i32) -> *mut u8 {
    if a.is_null() { return ptr::null_mut(); }
    let raw_a = hash_to_arr(a, elemsize);
    let mut table = hash_table(raw_a);
    *stbds_temp(raw_a) = 0;
    if table.is_null() { return a; }
    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 { return a; }
    let b = &mut *(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = arr_len(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;
    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(*(a.add(elemsize * old_index as usize) as *mut *mut u8));
    }
    if old_index != final_index {
        memmove(
            a.add(elemsize * old_index as usize),
            a.add(elemsize * final_index as usize),
            elemsize,
        );
        let slot2 = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(a, elemsize, *(a.add(elemsize * old_index as usize + keyoffset) as *mut *mut u8), keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(a, elemsize, a.add(elemsize * old_index as usize + keyoffset), keysize, keyoffset, mode)
        };
        let b2 = &mut *(*table).storage.add(slot2 as usize >> STBDS_BUCKET_SHIFT);
        let i2 = slot2 as usize & STBDS_BUCKET_MASK;
        b2.index[i2] = old_index;
    }
    (*header(raw_a)).length -= 1;
    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count, table) as *mut u8;
        free(table as *mut u8);
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut u8, str: *mut i8) -> *mut i8 {
    let a = a as *mut StbdsStringArena;
    let len = strlen(str as *const i8) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }
        if len > blocksize {
            let sb = realloc(ptr::null_mut(), std::mem::size_of::<StbdsStringBlock>() - 8 + len) as *mut StbdsStringBlock;
            memmove((*sb).storage.as_mut_ptr(), str as *const u8, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr() as *mut i8;
        } else {
            let sb = realloc(ptr::null_mut(), std::mem::size_of::<StbdsStringBlock>() - 8 + blocksize) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len) as *mut i8;
    (*a).remaining -= len;
    memmove(p as *mut u8, str as *const u8, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut u8) {
    let a = a as *mut StbdsStringArena;
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut u8);
        x = y;
    }
    memset(a as *mut u8, 0, std::mem::size_of::<StbdsStringArena>());
}

static mut STRKEY_BUFFER: [u8; 256] = [0u8; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut i8 {
    sprintf(STRKEY_BUFFER.as_mut_ptr() as *mut i8, b"test_%d\0".as_ptr() as *const i8, n);
    STRKEY_BUFFER.as_mut_ptr() as *mut i8
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    unsafe {
        let mut arr: *mut c_int = ptr::null_mut();
        let elemsize = std::mem::size_of::<c_int>();
        for i in 0..4u32 {
            // arrpush x4
            for &v in &[num, 2, 3, 4] {
                if arr.is_null() || (*header(arr as *mut u8)).length + 1 > (*header(arr as *mut u8)).capacity {
                    arr = stbds_arrgrowf(arr as *mut u8, elemsize, 1, 0) as *mut c_int;
                }
                let idx = (*header(arr as *mut u8)).length;
                *arr.add(idx) = v;
                (*header(arr as *mut u8)).length += 1;
            }
            // arrdel(arr, i)
            {
                let hdr = header(arr as *mut u8);
                let count = (*hdr).length - 1 - i as usize;
                ptr::copy(arr.add(i as usize + 1), arr.add(i as usize), count);
                (*hdr).length -= 1;
            }
            // arrfree
            stbds_arrfreef(arr as *mut u8);
            arr = ptr::null_mut();

            // arrpush x4
            for &v in &[num, 2, 3, 4] {
                if arr.is_null() || (*header(arr as *mut u8)).length + 1 > (*header(arr as *mut u8)).capacity {
                    arr = stbds_arrgrowf(arr as *mut u8, elemsize, 1, 0) as *mut c_int;
                }
                let idx = (*header(arr as *mut u8)).length;
                *arr.add(idx) = v;
                (*header(arr as *mut u8)).length += 1;
            }
            // arrdelswap(arr, i)
            {
                let hdr = header(arr as *mut u8);
                let last = (*hdr).length - 1;
                *arr.add(i as usize) = *arr.add(last);
                (*hdr).length -= 1;
            }
            // arrfree
            stbds_arrfreef(arr as *mut u8);
            arr = ptr::null_mut();
        }
    }
}
