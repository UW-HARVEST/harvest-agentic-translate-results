#![allow(
    non_camel_case_types, non_snake_case, unused_assignments,
    unused_variables, unused_mut, static_mut_refs,
)]

use std::ptr;
use std::ffi::c_int;

extern "C" {
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> c_int;
    fn strcmp(s1: *const u8, s2: *const u8) -> c_int;
    fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn memset(s: *mut u8, c: c_int, n: usize) -> *mut u8;
    fn strlen(s: *const u8) -> usize;
    fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;
const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;
const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;
const STBDS_SIZE_T_BITS: usize = std::mem::size_of::<usize>() * 8;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;
const HDR_SIZE: usize = std::mem::size_of::<stbds_array_header>();

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
pub struct stbds_string_arena {
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

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[inline(always)]
unsafe fn hdr(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}
#[inline(always)]
unsafe fn arr_len(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*hdr(a)).length as isize }
}
#[inline(always)]
unsafe fn arr_cap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*hdr(a)).capacity }
}
#[inline(always)]
fn align_fwd(n: usize, a: usize) -> usize { (n + a - 1) & !(a - 1) }

// STBDS_HASH_TO_ARR(x,elemsize) = (x as *mut u8).sub(elemsize)
// STBDS_ARR_TO_HASH(x,elemsize) = (x as *mut u8).add(elemsize)

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap: usize) -> *mut u8 {
    let mut min_cap = min_cap;
    let min_len = (arr_len(a) as usize).wrapping_add(addlen);
    if min_len > min_cap { min_cap = min_len; }
    if min_cap <= arr_cap(a) { return a; }
    if min_cap < 2 * arr_cap(a) { min_cap = 2 * arr_cap(a); }
    else if min_cap < 4 { min_cap = 4; }
    let b = realloc(
        if a.is_null() { ptr::null_mut() } else { hdr(a) as *mut u8 },
        elemsize * min_cap + HDR_SIZE,
    ).add(HDR_SIZE);
    if a.is_null() {
        (*hdr(b)).length = 0;
        (*hdr(b)).hash_table = ptr::null_mut();
        (*hdr(b)).temp = 0;
    }
    (*hdr(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) { free(hdr(a) as *mut u8); }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) { STBDS_HASH_SEED = seed; }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(s: *mut u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = s;
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

unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize = (((0x736f6d65_usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = (((0x646f7261_usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = (((0x6c796765_usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = (((0x74656462_usize) << 16) << 16).wrapping_add(0x79746573) ^ !seed;
    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    macro_rules! sipround { () => {
        v0 = v0.wrapping_add(v1); v1 = v1.rotate_left(13); v1 ^= v0; v0 = v0.rotate_left((STBDS_SIZE_T_BITS/2) as u32);
        v2 = v2.wrapping_add(v3); v3 = v3.rotate_left(16); v3 ^= v2;
        v2 = v2.wrapping_add(v1); v1 = v1.rotate_left(17); v1 ^= v2; v2 = v2.rotate_left((STBDS_SIZE_T_BITS/2) as u32);
        v0 = v0.wrapping_add(v3); v3 = v3.rotate_left(21); v3 ^= v0;
    };}
    let mut i: usize = 0;
    while i + 8 <= len {
        let dp = d.add(i);
        let mut data: usize = *dp as usize | ((*dp.add(1) as usize) << 8) | ((*dp.add(2) as usize) << 16) | ((*dp.add(3) as usize) << 24);
        data |= ((*dp.add(4) as usize) | ((*dp.add(5) as usize) << 8) | ((*dp.add(6) as usize) << 16) | ((*dp.add(7) as usize) << 24)) << 16 << 16;
        v3 ^= data;
        for _ in 0..2 { sipround!(); }
        v0 ^= data;
        i += 8;
    }
    let dp = d.add(i);
    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let rem = len - i;
    if rem >= 7 { data |= ((*dp.add(6) as usize) << 24) << 24; }
    if rem >= 6 { data |= ((*dp.add(5) as usize) << 20) << 20; }
    if rem >= 5 { data |= ((*dp.add(4) as usize) << 16) << 16; }
    if rem >= 4 { data |= (*dp.add(3) as usize) << 24; }
    if rem >= 3 { data |= (*dp.add(2) as usize) << 16; }
    if rem >= 2 { data |= (*dp.add(1) as usize) << 8; }
    if rem >= 1 { data |= *dp as usize; }
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

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize { hash & (slot_count - 1) }

fn stbds_log2(mut sc: usize) -> usize { let mut n = 0; while sc > 1 { sc >>= 1; n += 1; } n }

unsafe fn stbds_is_key_equal(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: c_int, i: isize) -> bool {
    if mode >= STBDS_HM_STRING {
        strcmp(key, *(a.add(elemsize * i as usize).add(keyoffset) as *const *const u8)) == 0
    } else {
        memcmp(key, a.add(elemsize * i as usize).add(keyoffset), keysize) == 0
    }
}

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut stbds_hash_index) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>() + STBDS_CACHE_LINE_SIZE - 1;
    let t = realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    (*t).storage = align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;
    (*t).used_count_threshold = slot_count - (slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;
    if slot_count <= STBDS_BUCKET_LENGTH { (*t).used_count_shrink_threshold = 0; }
    assert!((*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count);
    if !ot.is_null() {
        ptr::copy_nonoverlapping(&(*ot).string as *const _ as *const u8, &mut (*t).string as *mut _ as *mut u8, std::mem::size_of::<stbds_string_arena>());
        (*t).seed = (*ot).seed;
    } else {
        memset(&mut (*t).string as *mut _ as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
        (*t).seed = STBDS_HASH_SEED;
        let a_val: usize; let b_val: usize;
        { let v32: usize = 2147001325; let v64_hi: usize = 0x27bb2ee6; let v64_lo: usize = 0x87b0b0fd;
          let mut temp = v64_lo ^ v32; temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
          let mut r = v64_hi; r <<= 16; r <<= 16; r ^= temp ^ v32; a_val = r; }
        { let v32: usize = 715136305; let v64_hi: usize = 0; let v64_lo: usize = 0xb504f32d;
          let mut temp = v64_lo ^ v32; temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
          let mut r = v64_hi; r <<= 16; r <<= 16; r ^= temp ^ v32; b_val = r; }
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a_val).wrapping_add(b_val);
    }
    for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
        let b = &mut *(*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH { b.hash[j] = STBDS_HASH_EMPTY; }
        for j in 0..STBDS_BUCKET_LENGTH { b.index[j] = STBDS_INDEX_EMPTY; }
    }
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
                        for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 { bucket.hash[z] = hash; bucket.index[z] = ob.index[j]; break 'outer; }
                        }
                        for z in 0..(pos & STBDS_BUCKET_MASK) {
                            if bucket.hash[z] == 0 { bucket.hash[z] = hash; bucket.index[z] = ob.index[j]; break 'outer; }
                        }
                        pos += step; step += STBDS_BUCKET_LENGTH; pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }
    t
}

unsafe fn stbds_hm_find_slot(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: c_int) -> isize {
    let raw_a = a.sub(elemsize);
    let table = (*hdr(raw_a)).hash_table as *mut stbds_hash_index;
    let mut hash = if mode >= STBDS_HM_STRING { stbds_hash_string(key, (*table).seed) } else { stbds_siphash_bytes(key, keysize, (*table).seed) };
    if hash < 2 { hash += 2; }
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY { return -1; }
        }
        for i in 0..(pos & STBDS_BUCKET_MASK) {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY { return -1; }
        }
        pos += step; step += STBDS_BUCKET_LENGTH; pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() { return; }
    let ht = (*hdr(a)).hash_table as *mut stbds_hash_index;
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*hdr(a)).length { free(*(a.add(elemsize * i) as *mut *mut u8)); }
        }
        stbds_strreset(&mut (*ht).string);
    }
    free((*hdr(a)).hash_table);
    free(hdr(a) as *mut u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, temp: *mut isize, mode: c_int) -> *mut u8 {
    if a.is_null() {
        let arr = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*hdr(arr)).length += 1;
        memset(arr, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr.add(elemsize)
    } else {
        let raw_a = a.sub(elemsize);
        let table = (*hdr(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, 0, mode);
            if slot < 0 { *temp = STBDS_INDEX_EMPTY; }
            else {
                let b = &*(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                *temp = b.index[slot as usize & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*hdr(p.sub(elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*hdr(a.sub(elemsize))).length == 0 {
        let raw = if !a.is_null() { a.sub(elemsize) } else { ptr::null_mut() };
        let arr = stbds_arrgrowf(raw, elemsize, 0, 1);
        (*hdr(arr)).length += 1;
        memset(arr, 0, elemsize);
        arr.add(elemsize)
    } else { a }
}

unsafe fn stbds_strdup_internal(s: *const u8) -> *mut u8 {
    let len = strlen(s) + 1;
    let p = realloc(ptr::null_mut(), len);
    memmove(p, s, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(a_in: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a: *mut u8;

    if a_in.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*hdr(a)).length += 1;
        a = a.add(elemsize);
    } else {
        a = a_in;
    }

    let raw_a = a;
    let mut arr = a.sub(elemsize);

    let mut table = (*hdr(arr)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() { free(table as *mut u8); }
        else { (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 }; }
        (*hdr(arr)).hash_table = nt as *mut u8;
        table = nt;
    }

    let mut hash = if mode >= STBDS_HM_STRING { stbds_hash_string(key, (*table).seed) } else { stbds_siphash_bytes(key, keysize, (*table).seed) };
    if hash < 2 { hash += 2; }
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    // Search loop - find existing or empty slot
    // Use a variable to store the found empty position
    let empty_pos: usize;
    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*hdr(arr)).temp = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *((*hdr(arr)).hash_table as *mut *mut u8) =
                            *(raw_a.add(elemsize * bucket.index[i] as usize).add(keyoffset) as *mut *mut u8);
                    }
                    return arr.add(elemsize);
                }
            } else if bucket.hash[i] == 0 {
                empty_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && bucket.index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }
        for i in 0..(pos & STBDS_BUCKET_MASK) {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    (*hdr(arr)).temp = bucket.index[i];
                    return arr.add(elemsize);
                }
            } else if bucket.hash[i] == 0 {
                empty_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && bucket.index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }
        pos += step; step += STBDS_BUCKET_LENGTH; pos &= (*table).slot_count - 1;
    }

    // found_empty_slot:
    let pos = if tombstone >= 0 { (*table).tombstone_count -= 1; tombstone as usize } else { empty_pos };
    (*table).used_count += 1;

    let i = arr_len(arr);
    if (i as usize + 1) > arr_cap(arr) {
        arr = stbds_arrgrowf(arr, elemsize, 1, 0);
        // raw_a changes because arr may have moved
    }
    let raw_a2 = arr.add(elemsize);

    assert!((i as usize + 1) <= arr_cap(arr));
    (*hdr(arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*hdr(arr)).temp = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup_internal(key);
            *(arr.add(elemsize * i as usize) as *mut *mut u8) = dup;
            *((*hdr(arr)).hash_table as *mut *mut u8) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(&mut (*table).string, key);
            *(arr.add(elemsize * i as usize) as *mut *mut u8) = s;
            *((*hdr(arr)).hash_table as *mut *mut u8) = s;
        }
        STBDS_SH_DEFAULT => {
            *(arr.add(elemsize * i as usize) as *mut *mut u8) = key;
            *((*hdr(arr)).hash_table as *mut *mut u8) = key;
        }
        _ => {
            memcpy(arr.add(elemsize * i as usize), key, keysize);
        }
    }
    arr.add(elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*hdr(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*hdr(a)).hash_table = h as *mut u8;
    a.add(elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: c_int) -> *mut u8 {
    if a.is_null() { return ptr::null_mut(); }
    let raw_a = a.sub(elemsize);
    let mut table = (*hdr(raw_a)).hash_table as *mut stbds_hash_index;
    (*hdr(raw_a)).temp = 0;
    if table.is_null() { return a; }

    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 { return a; }

    let b = &mut *(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = arr_len(raw_a) - 1 - 1;
    assert!(slot < (*table).slot_count as isize);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*hdr(raw_a)).temp = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 (always true for usize, but matches C assert)
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(*(a.add(elemsize * old_index as usize) as *mut *mut u8));
    }

    if old_index != final_index {
        memmove(a.add(elemsize * old_index as usize), a.add(elemsize * final_index as usize), elemsize);
        let slot2: isize;
        if mode == STBDS_HM_STRING {
            slot2 = stbds_hm_find_slot(a, elemsize, *(a.add(elemsize * old_index as usize).add(keyoffset) as *mut *mut u8), keysize, keyoffset, mode);
        } else {
            slot2 = stbds_hm_find_slot(a, elemsize, a.add(elemsize * old_index as usize).add(keyoffset), keysize, keyoffset, mode);
        }
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add(slot2 as usize >> STBDS_BUCKET_SHIFT);
        let i2 = slot2 as usize & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }
    (*hdr(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*hdr(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*hdr(raw_a)).hash_table = stbds_make_hash_index((*table).slot_count, table) as *mut u8;
        free(table as *mut u8);
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str_ptr: *mut u8) -> *mut u8 {
    let len = strlen(str_ptr) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX { (*a).block += 1; }
        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            memmove((*sb).storage.as_mut_ptr() as *mut u8, str_ptr, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr() as *mut u8;
        } else {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }
    assert!(len <= (*a).remaining);
    let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8).add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p, str_ptr, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut u8);
        x = y;
    }
    memset(a as *mut u8, 0, std::mem::size_of::<stbds_string_arena>());
}

// ── strkey / intput ────────────────────────────────────────────────────────

static mut BUFFER: [u8; 256] = [0u8; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut u8 {
    extern "C" { fn sprintf(s: *mut u8, format: *const u8, ...) -> c_int; }
    sprintf(BUFFER.as_mut_ptr(), b"test_%d\0".as_ptr(), n);
    BUFFER.as_mut_ptr()
}

#[repr(C)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(num: c_int) {
    let elemsize = std::mem::size_of::<IntMapEntry>();
    let mut intmap: *mut u8 = ptr::null_mut();

    // hmput(intmap, num, 7)
    {
        let k = num;
        intmap = stbds_hmput_key(intmap, elemsize, &k as *const c_int as *mut u8, std::mem::size_of::<c_int>(), STBDS_HM_BINARY);
        let idx = (*hdr(intmap.sub(elemsize))).temp;
        let entry = intmap.add(elemsize * idx as usize) as *mut IntMapEntry;
        (*entry).key = k;
        (*entry).value = 7;
    }
    // hmput(intmap, 11, 3)
    {
        let k: c_int = 11;
        intmap = stbds_hmput_key(intmap, elemsize, &k as *const c_int as *mut u8, std::mem::size_of::<c_int>(), STBDS_HM_BINARY);
        let idx = (*hdr(intmap.sub(elemsize))).temp;
        let entry = intmap.add(elemsize * idx as usize) as *mut IntMapEntry;
        (*entry).key = k;
        (*entry).value = 3;
    }
    // hmput(intmap, 9, num)
    {
        let k: c_int = 9;
        intmap = stbds_hmput_key(intmap, elemsize, &k as *const c_int as *mut u8, std::mem::size_of::<c_int>(), STBDS_HM_BINARY);
        let idx = (*hdr(intmap.sub(elemsize))).temp;
        let entry = intmap.add(elemsize * idx as usize) as *mut IntMapEntry;
        (*entry).key = k;
        (*entry).value = num;
    }
    // assert hmget(intmap, 9) == num
    {
        let k: c_int = 9;
        intmap = stbds_hmget_key(intmap, elemsize, &k as *const c_int as *mut u8, std::mem::size_of::<c_int>(), STBDS_HM_BINARY);
        let idx = (*hdr(intmap.sub(elemsize))).temp;
        let entry = intmap.add(elemsize * idx as usize) as *mut IntMapEntry;
        assert!((*entry).value == num);
    }
    // assert hmget(intmap, 11) == 3
    {
        let k: c_int = 11;
        intmap = stbds_hmget_key(intmap, elemsize, &k as *const c_int as *mut u8, std::mem::size_of::<c_int>(), STBDS_HM_BINARY);
        let idx = (*hdr(intmap.sub(elemsize))).temp;
        let entry = intmap.add(elemsize * idx as usize) as *mut IntMapEntry;
        assert!((*entry).value == 3);
    }
    // assert hmget(intmap, num) == 7
    {
        let k = num;
        intmap = stbds_hmget_key(intmap, elemsize, &k as *const c_int as *mut u8, std::mem::size_of::<c_int>(), STBDS_HM_BINARY);
        let idx = (*hdr(intmap.sub(elemsize))).temp;
        let entry = intmap.add(elemsize * idx as usize) as *mut IntMapEntry;
        assert!((*entry).value == 7);
    }
}
