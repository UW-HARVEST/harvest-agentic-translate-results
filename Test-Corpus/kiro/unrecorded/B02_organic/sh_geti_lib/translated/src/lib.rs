#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    clippy::missing_safety_doc
)]

use std::ffi::c_int;
use std::ptr;

extern "C" {
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn malloc(size: usize) -> *mut u8;
    fn memset(s: *mut u8, c: c_int, n: usize) -> *mut u8;
    fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> c_int;
    fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn strcmp(s1: *const u8, s2: *const u8) -> c_int;
    fn strlen(s: *const u8) -> usize;
    fn sprintf(s: *mut u8, format: *const u8, ...) -> c_int;
    fn printf(format: *const u8, ...) -> c_int;
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;
const STBDS_SIZE_T_BITS: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

static mut stbds_hash_seed: usize = 0x31415926;

#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

#[repr(C)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [u8; 8],
}

#[repr(C)]
struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct stbds_hash_index {
    temp_key: *mut u8,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: stbds_string_arena,
    storage: *mut stbds_hash_bucket,
}

// Helper: get header pointer from array pointer
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if !a.is_null() { (*stbds_header(a)).length as isize } else { 0 }
}

unsafe fn stbds_arrlenu(a: *mut u8) -> usize {
    if !a.is_null() { (*stbds_header(a)).length } else { 0 }
}

unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if !a.is_null() { (*stbds_header(a)).capacity } else { 0 }
}

fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

unsafe fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap_arg: usize) -> *mut u8 {
    let mut min_cap = min_cap_arg;
    let min_len = (stbds_arrlen(a) + addlen as isize) as usize;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if !a.is_null() { stbds_header(a) as *mut u8 } else { ptr::null_mut() };
    let b = realloc(old, elemsize * min_cap + std::mem::size_of::<stbds_array_header>());
    let b = b.add(std::mem::size_of::<stbds_array_header>());
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut stbds_hash_index) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    (*t).storage = stbds_align_fwd(
        (t.add(1)) as usize,
        STBDS_CACHE_LINE_SIZE,
    ) as *mut stbds_hash_bucket;
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
    assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);

    if !ot.is_null() {
        (*t).string = std::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        memset(&mut (*t).string as *mut stbds_string_arena as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
        (*t).seed = stbds_hash_seed;
        // stbds_load_32_or_64 for a
        let a: usize;
        {
            let v32: usize = 2147001325;
            let v64_hi: usize = 0x27bb2ee6;
            let v64_lo: usize = 0x87b0b0fd;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            let mut var: usize = v64_hi;
            var <<= 16; var <<= 16;
            a = var ^ temp ^ v32;
        }
        let b: usize;
        {
            let v32: usize = 715136305;
            let v64_hi: usize = 0;
            let v64_lo: usize = 0xb504f32d;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
            let mut var: usize = v64_hi;
            var <<= 16; var <<= 16;
            b = var ^ temp ^ v32;
        }
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    // Initialize buckets
    for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
        let bucket = &mut *(*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            bucket.hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            bucket.index[j] = STBDS_INDEX_EMPTY;
        }
    }

    // Rehash from old table
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let hash = ob.hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = &mut *(*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                            z += 1;
                        }
                        let limit = pos & STBDS_BUCKET_MASK;
                        z = 0;
                        while z < limit {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                            z += 1;
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

unsafe fn stbds_hash_string_fn(str: *const u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str;
    while *p != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*p as usize);
        p = p.add(1);
    }
    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ stbds_rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ stbds_rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1); v1 = stbds_rotate_left(v1, 13); v1 ^= v0; v0 = stbds_rotate_left(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3); v3 = stbds_rotate_left(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = stbds_rotate_left(v1, 17); v1 ^= v2; v2 = stbds_rotate_left(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3); v3 = stbds_rotate_left(v3, 21); v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dd = d.add(i);
        let mut data: usize = *dd.add(0) as usize
            | ((*dd.add(1) as usize) << 8)
            | ((*dd.add(2) as usize) << 16)
            | ((*dd.add(3) as usize) << 24);
        data |= ((*dd.add(4) as usize)
            | ((*dd.add(5) as usize) << 8)
            | ((*dd.add(6) as usize) << 16)
            | ((*dd.add(7) as usize) << 24)) << 16 << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let dd = d.add(i);
    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    // Fall-through switch
    if rem >= 7 { data |= (*dd.add(6) as usize) << 24 << 24; }
    if rem >= 6 { data |= (*dd.add(5) as usize) << 20 << 20; }
    if rem >= 5 { data |= (*dd.add(4) as usize) << 16 << 16; }
    if rem >= 4 { data |= (*dd.add(3) as usize) << 24; }
    if rem >= 3 { data |= (*dd.add(2) as usize) << 16; }
    if rem >= 2 { data |= (*dd.add(1) as usize) << 8; }
    if rem >= 1 { data |= *dd.add(0) as usize; }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        sipround!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        sipround!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

unsafe fn stbds_hash_bytes_fn(p: *const u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

unsafe fn stbds_is_key_equal(a: *mut u8, elemsize: usize, key: *const u8, keysize: usize, keyoffset: usize, mode: c_int, i: isize) -> bool {
    if mode >= STBDS_HM_STRING {
        let key_ptr = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        strcmp(key, key_ptr) == 0
    } else {
        memcmp(key, a.add(elemsize * i as usize + keyoffset), keysize) == 0
    }
}

unsafe fn stbds_hm_find_slot(a: *mut u8, elemsize: usize, key: *const u8, keysize: usize, keyoffset: usize, mode: c_int) -> isize {
    let raw_a = a.sub(elemsize);
    let table = stbds_hash_table(raw_a);
    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string_fn(key, (*table).seed)
    } else {
        stbds_hash_bytes_fn(key, keysize, (*table).seed)
    };
    let mut hash = hash;
    if hash < 2 { hash += 2; }
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }
        let limit = pos & STBDS_BUCKET_MASK;
        i = 0;
        while i < limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }
        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

unsafe fn stbds_hmget_key_ts(a: *mut u8, elemsize: usize, key: *const u8, keysize: usize, temp: *mut isize, mode: c_int) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let a2 = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a2)).length += 1;
        memset(a2, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return a2.add(elemsize);
    } else {
        let raw_a = a.sub(elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = &*(*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                *temp = b.index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        return a;
    }
}

unsafe fn stbds_hmget_key(a: *mut u8, elemsize: usize, key: *const u8, keysize: usize, mode: c_int) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*stbds_header(p.sub(elemsize))).temp = temp;
    p
}

unsafe fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*stbds_header(a.sub(elemsize))).length == 0 {
        let raw = if !a.is_null() { a.sub(elemsize) } else { ptr::null_mut() };
        let a2 = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*stbds_header(a2)).length += 1;
        memset(a2, 0, elemsize);
        return a2.add(elemsize);
    }
    a
}

unsafe fn stbds_strdup(str: *const u8) -> *mut u8 {
    let len = strlen(str) + 1;
    let p = realloc(ptr::null_mut(), len);
    memmove(p, str, len);
    p
}

unsafe fn stbds_stralloc_fn(a: *mut stbds_string_arena, str: *const u8) -> *mut u8 {
    let len = strlen(str) + 1;
    if len > (*a).remaining {
        let block_exp = (*a).block;
        let mut blocksize = (STBDS_STRING_ARENA_BLOCKSIZE_MIN) << (block_exp as usize >> 1);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }
        if len > blocksize {
            let sb = realloc(ptr::null_mut(), std::mem::size_of::<stbds_string_block>() - 8 + len) as *mut stbds_string_block;
            memmove((*sb).storage.as_mut_ptr(), str, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let sb = realloc(ptr::null_mut(), std::mem::size_of::<stbds_string_block>() - 8 + blocksize) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }
    assert!(len <= (*a).remaining);
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p, str, len);
    p
}

unsafe fn stbds_strreset_fn(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut u8);
        x = y;
    }
    memset(a as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
}

unsafe fn stbds_hmput_key(a_arg: *mut u8, elemsize: usize, key: *const u8, keysize: usize, mode: c_int) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a: *mut u8;
    let raw_a: *mut u8;

    if a_arg.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = a.add(elemsize);
    } else {
        a = a_arg;
    }

    raw_a = a;
    a = a.sub(elemsize); // raw_a points to hash space, a points to array

    let mut table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut u8);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*stbds_header(a)).hash_table = nt as *mut u8;
        table = nt;
    }

    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string_fn(key, (*table).seed)
    } else {
        stbds_hash_bytes_fn(key, keysize, (*table).seed)
    };
    let mut hash = hash;
    if hash < 2 { hash += 2; }
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    let found_pos: usize;

    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(a)).temp = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        // stbds_temp_key(a) = key stored at that index
                        let key_in_arr = *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset) as *const *mut u8);
                        *((*stbds_header(a)).hash_table as *mut *mut u8) = key_in_arr;
                    }
                    return a.add(elemsize);
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 {
                if bucket.index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            i += 1;
        }
        let limit = pos & STBDS_BUCKET_MASK;
        i = 0;
        while i < limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*stbds_header(a)).temp = bucket.index[i];
                    return a.add(elemsize);
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 {
                if bucket.index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            i += 1;
        }
        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    // found_empty_slot:
    let mut final_pos = found_pos;
    if tombstone >= 0 {
        final_pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let arr_i = stbds_arrlen(a) as isize;
    if (arr_i as usize + 1) > stbds_arrcap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    let raw_a2 = a.add(elemsize);

    assert!((arr_i as usize + 1) <= stbds_arrcap(a));
    (*stbds_header(a)).length = (arr_i + 1) as usize;
    let bucket = &mut *(*table).storage.add(final_pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[final_pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[final_pos & STBDS_BUCKET_MASK] = arr_i - 1;
    (*stbds_header(a)).temp = arr_i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key);
            *(a.add(elemsize * arr_i as usize) as *mut *mut u8) = dup;
            *((*stbds_header(a)).hash_table as *mut *mut u8) = dup;
        }
        STBDS_SH_ARENA => {
            let alloc = stbds_stralloc_fn(&mut (*table).string, key);
            *(a.add(elemsize * arr_i as usize) as *mut *mut u8) = alloc;
            *((*stbds_header(a)).hash_table as *mut *mut u8) = alloc;
        }
        STBDS_SH_DEFAULT => {
            *(a.add(elemsize * arr_i as usize) as *mut *mut u8) = key as *mut u8;
            *((*stbds_header(a)).hash_table as *mut *mut u8) = key as *mut u8;
        }
        _ => {
            memmove(a.add(elemsize * arr_i as usize), key, keysize);
        }
    }

    a.add(elemsize)
}

unsafe fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() { return; }
    let ht = stbds_hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            let len = (*stbds_header(a)).length;
            for i in 1..len {
                free(*(a.add(elemsize * i) as *const *mut u8));
            }
        }
        stbds_strreset_fn(&mut (*ht).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut u8);
}

unsafe fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut u8;
    a.add(elemsize)
}

unsafe fn stbds_hmdel_key(a: *mut u8, elemsize: usize, key: *const u8, keysize: usize, keyoffset: usize, mode: c_int) -> *mut u8 {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = a.sub(elemsize);
    let mut table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    (*stbds_header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }
    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }
    let b = &mut *(*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 always true for usize
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(*(a.add(elemsize * old_index as usize) as *const *mut u8));
    }

    if old_index != final_index {
        memmove(a.add(elemsize * old_index as usize), a.add(elemsize * final_index as usize), elemsize);

        let slot2 = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(a, elemsize, *(a.add(elemsize * old_index as usize + keyoffset) as *const *const u8), keysize, keyoffset, mode)
        } else {
            stbds_hm_find_slot(a, elemsize, a.add(elemsize * old_index as usize + keyoffset), keysize, keyoffset, mode)
        };
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add((slot2 as usize) >> STBDS_BUCKET_SHIFT);
        let i2 = (slot2 as usize) & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count, table) as *mut u8;
        free(table as *mut u8);
    }

    a
}

// The strmap element: { char *key; int value; }
// On 64-bit: key is 8 bytes (pointer), value is 4 bytes, then 4 bytes padding = 16 bytes
const STRMAP_ELEM_SIZE: usize = 16;
const STRMAP_KEY_SIZE: usize = 8; // sizeof(char*)
const STRMAP_KEY_OFFSET: usize = 0;
const STRMAP_VALUE_OFFSET: usize = 8;

static mut BUFFER: [u8; 256] = [0u8; 256];

unsafe fn strkey(n: c_int) -> *mut u8 {
    sprintf(BUFFER.as_mut_ptr(), b"test_%d\0".as_ptr(), n);
    BUFFER.as_mut_ptr()
}

// Macro-like helpers for the string hash map operations
// shgeti: get index of key in string hash map
unsafe fn shgeti(strmap: *mut u8, key: *const u8) -> (*mut u8, isize) {
    let s = stbds_hmget_key(strmap, STRMAP_ELEM_SIZE, key, STRMAP_KEY_SIZE, STBDS_HM_STRING);
    let raw = s.sub(STRMAP_ELEM_SIZE);
    let temp = (*stbds_header(raw)).temp;
    (s, temp)
}

// shput: put key/value into string hash map
unsafe fn shput(strmap: *mut u8, key: *const u8, value: c_int) -> *mut u8 {
    let s = stbds_hmput_key(strmap, STRMAP_ELEM_SIZE, key, STRMAP_KEY_SIZE, STBDS_HM_STRING);
    let raw = s.sub(STRMAP_ELEM_SIZE);
    let temp = (*stbds_header(raw)).temp;
    // s[temp].value = value
    let elem = s.add(STRMAP_ELEM_SIZE * temp as usize);
    *(elem.add(STRMAP_VALUE_OFFSET) as *mut c_int) = value;
    s
}

// shget: get value for key
unsafe fn shget(strmap: *mut u8, key: *const u8) -> (*mut u8, c_int) {
    let (s, temp) = shgeti(strmap, key);
    let raw = s.sub(STRMAP_ELEM_SIZE);
    let t = (*stbds_header(raw)).temp;
    let elem = s.add(STRMAP_ELEM_SIZE * t as usize);
    let value = *(elem.add(STRMAP_VALUE_OFFSET) as *const c_int);
    (s, value)
}

// shdefault: set default value
unsafe fn shdefault(strmap: *mut u8, v: c_int) -> *mut u8 {
    let s = stbds_hmput_default(strmap, STRMAP_ELEM_SIZE);
    // s[-1].value = v
    let elem = s.sub(STRMAP_ELEM_SIZE);
    *(elem.add(STRMAP_VALUE_OFFSET) as *mut c_int) = v;
    s
}

// shdel: delete key
unsafe fn shdel(strmap: *mut u8, key: *const u8) -> (*mut u8, isize) {
    let s = stbds_hmdel_key(strmap, STRMAP_ELEM_SIZE, key, STRMAP_KEY_SIZE, STRMAP_KEY_OFFSET, STBDS_HM_STRING);
    let temp = if !s.is_null() {
        (*stbds_header(s.sub(STRMAP_ELEM_SIZE))).temp
    } else {
        0
    };
    (s, temp)
}

// shlen: length of hash map
unsafe fn shlen(strmap: *mut u8) -> isize {
    if !strmap.is_null() {
        (*stbds_header(strmap.sub(STRMAP_ELEM_SIZE))).length as isize - 1
    } else {
        0
    }
}

// shfree
unsafe fn shfree(strmap: *mut u8) {
    if !strmap.is_null() {
        stbds_hmfree_func(strmap.sub(STRMAP_ELEM_SIZE), STRMAP_ELEM_SIZE);
    }
}

// sh_new_strdup
unsafe fn sh_new_strdup(strmap: *mut u8) -> *mut u8 {
    stbds_shmode_func(STRMAP_ELEM_SIZE, STBDS_SH_STRDUP as c_int)
}

// sh_new_arena
unsafe fn sh_new_arena(strmap: *mut u8) -> *mut u8 {
    stbds_shmode_func(STRMAP_ELEM_SIZE, STBDS_SH_ARENA as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    unsafe {
        // String arena test
        let mut sa = std::mem::zeroed::<stbds_string_arena>();
        for i in 0..num {
            stbds_stralloc_fn(&mut sa, strkey(i));
        }
        stbds_strreset_fn(&mut sa);

        for j in 0..2 {
            let mut strmap: *mut u8 = ptr::null_mut();

            // ASSERT(shgeti(strmap,"foo") == -1)
            let (s, idx) = shgeti(strmap, b"foo\0".as_ptr());
            strmap = s;
            assert!(idx == -1);

            if j == 0 {
                strmap = sh_new_strdup(strmap);
            } else {
                strmap = sh_new_arena(strmap);
            }

            // ASSERT(shgeti(strmap,"foo") == -1)
            let (s, idx) = shgeti(strmap, b"foo\0".as_ptr());
            strmap = s;
            assert!(idx == -1);

            strmap = shdefault(strmap, -2);

            // ASSERT(shgeti(strmap,"foo") == -1)
            let (s, idx) = shgeti(strmap, b"foo\0".as_ptr());
            strmap = s;
            assert!(idx == -1);

            // for (i=0; i < num; i+=2) shput(strmap, strkey(i), i*3);
            let mut i = 0;
            while i < num {
                strmap = shput(strmap, strkey(i), i * 3);
                i += 2;
            }

            // for (int z=0; z < shlen(strmap); ++z) printf("%s %d\n", strmap[z], strmap[z].value);
            // Note: strmap[z] in C with the struct type prints the key (char*) via %s
            // strmap points to element[1] (index 0 in user space), so strmap[z] is element at offset z
            let len = shlen(strmap);
            for z in 0..len {
                let elem = strmap.add(STRMAP_ELEM_SIZE * z as usize);
                let key_ptr = *(elem as *const *const u8);
                let value = *(elem.add(STRMAP_VALUE_OFFSET) as *const c_int);
                printf(b"%s %d\n\0".as_ptr(), key_ptr, value);
            }

            // Verify: odd indices should return -2, even should return i*3
            i = 0;
            while i < num {
                let (s, val) = shget(strmap, strkey(i));
                strmap = s;
                if i & 1 != 0 {
                    assert!(val == -2);
                } else {
                    assert!(val == i * 3);
                }
                i += 1;
            }

            // Delete every 4th starting at 2
            i = 2;
            while i < num {
                let (s, _) = shdel(strmap, strkey(i));
                strmap = s;
                i += 4;
            }

            // Verify: i&3 != 0 should be -2, i&3 == 0 should be i*3
            i = 0;
            while i < num {
                let (s, val) = shget(strmap, strkey(i));
                strmap = s;
                if i & 3 != 0 {
                    assert!(val == -2);
                } else {
                    assert!(val == i * 3);
                }
                i += 1;
            }

            // Delete all
            i = 0;
            while i < num {
                let (s, _) = shdel(strmap, strkey(i));
                strmap = s;
                i += 1;
            }

            // Verify all return -2
            i = 0;
            while i < num {
                let (s, val) = shget(strmap, strkey(i));
                strmap = s;
                assert!(val == -2);
                i += 1;
            }

            shfree(strmap);
        }
    }
}
