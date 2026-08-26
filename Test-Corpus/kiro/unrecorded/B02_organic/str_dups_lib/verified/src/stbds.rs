#![allow(
    non_camel_case_types,
    non_snake_case,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::c_int;
use std::ptr;

const BUCKET_LENGTH: usize = 8;
const BUCKET_SHIFT: usize = 3;
const BUCKET_MASK: usize = BUCKET_LENGTH - 1;
const CACHE_LINE_SIZE: usize = 64;
const INDEX_EMPTY: isize = -1;
const INDEX_DELETED: isize = -2;
const HASH_EMPTY: usize = 0;
const HASH_DELETED: usize = 1;
const HM_STRING: i32 = 1;
const SH_DEFAULT: u8 = 1;
const SH_STRDUP: u8 = 2;
const SH_ARENA: u8 = 3;
const SIZE_T_BITS: u32 = 64;
const STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

pub static mut HASH_SEED: usize = 0x31415926;

#[repr(C)]
pub struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

#[repr(C)]
pub struct StringBlock {
    next: *mut StringBlock,
    storage: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
pub struct HashBucket {
    hash: [usize; BUCKET_LENGTH],
    index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
pub struct HashIndex {
    temp_key: *mut u8,
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
pub struct StrMapEntry {
    key: *mut u8,
    value: c_int,
}

pub unsafe fn hdr(t: *mut u8) -> *mut ArrayHeader {
    (t as *mut ArrayHeader).offset(-1)
}

pub unsafe fn arr_len(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*hdr(a)).length as isize }
}

pub unsafe fn arr_cap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*hdr(a)).capacity }
}

pub unsafe fn h2a(x: *mut u8, es: usize) -> *mut u8 { x.sub(es) }
pub unsafe fn a2h(x: *mut u8, es: usize) -> *mut u8 { x.add(es) }
pub unsafe fn htab(a: *mut u8) -> *mut HashIndex { (*hdr(a)).hash_table as *mut HashIndex }

pub unsafe fn arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap_in: usize) -> *mut u8 {
    let min_len = (arr_len(a) as usize).wrapping_add(addlen);
    let mut mc = min_cap_in;
    if min_len > mc { mc = min_len; }
    if mc <= arr_cap(a) { return a; }
    if mc < 2 * arr_cap(a) { mc = 2 * arr_cap(a); }
    else if mc < 4 { mc = 4; }
    let old = if !a.is_null() { hdr(a) as *mut u8 } else { ptr::null_mut() };
    let b_raw = libc::realloc(old as *mut _, elemsize * mc + std::mem::size_of::<ArrayHeader>()) as *mut u8;
    let b = b_raw.add(std::mem::size_of::<ArrayHeader>());
    if a.is_null() {
        (*hdr(b)).length = 0;
        (*hdr(b)).hash_table = ptr::null_mut();
        (*hdr(b)).temp = 0;
    }
    (*hdr(b)).capacity = mc;
    b
}

fn log2(mut n: usize) -> usize {
    let mut r = 0;
    while n > 1 { n >>= 1; r += 1; }
    r
}

fn probe_pos(hash: usize, sc: usize, _: usize) -> usize { hash & (sc - 1) }
fn rotl(v: usize, n: u32) -> usize { v.rotate_left(n) }
fn rotr(v: usize, n: u32) -> usize { v.rotate_right(n) }

fn hash_string(s: *const u8, seed: usize) -> usize {
    let mut h = seed;
    let mut p = s;
    unsafe { while *p != 0 { h = rotl(h, 9).wrapping_add(*p as usize); p = p.add(1); } }
    h ^= seed;
    h = (!h).wrapping_add(h << 18);
    h ^= h ^ rotr(h, 31);
    h = h.wrapping_mul(21);
    h ^= h ^ rotr(h, 11);
    h = h.wrapping_add(h << 6);
    h ^= rotr(h, 22);
    h.wrapping_add(seed)
}

fn siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let mut v0: usize = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;
    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    macro_rules! sr {
        () => {
            v0 = v0.wrapping_add(v1); v1 = rotl(v1, 13); v1 ^= v0; v0 = rotl(v0, SIZE_T_BITS/2);
            v2 = v2.wrapping_add(v3); v3 = rotl(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = rotl(v1, 17); v1 ^= v2; v2 = rotl(v2, SIZE_T_BITS/2);
            v0 = v0.wrapping_add(v3); v3 = rotl(v3, 21); v3 ^= v0;
        };
    }
    unsafe {
        let mut i: usize = 0;
        while i + 8 <= len {
            let d = p.add(i);
            let lo = *d as usize | (*d.add(1) as usize) << 8 | (*d.add(2) as usize) << 16 | (*d.add(3) as usize) << 24;
            let hi = *d.add(4) as usize | (*d.add(5) as usize) << 8 | (*d.add(6) as usize) << 16 | (*d.add(7) as usize) << 24;
            let data = lo | (hi << 16 << 16);
            v3 ^= data; for _ in 0..2 { sr!(); } v0 ^= data;
            i += 8;
        }
        let mut data: usize = len << (SIZE_T_BITS - 8);
        let d = p.add(i);
        let rem = len - i;
        if rem >= 7 { data |= (*d.add(6) as usize) << 24 << 24; }
        if rem >= 6 { data |= (*d.add(5) as usize) << 20 << 20; }
        if rem >= 5 { data |= (*d.add(4) as usize) << 16 << 16; }
        if rem >= 4 { data |= (*d.add(3) as usize) << 24; }
        if rem >= 3 { data |= (*d.add(2) as usize) << 16; }
        if rem >= 2 { data |= (*d.add(1) as usize) << 8; }
        if rem >= 1 { data |= *d as usize; }
        v3 ^= data; for _ in 0..2 { sr!(); } v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..4 { sr!(); }
    }
    v0 ^ v1 ^ v2 ^ v3
}

unsafe fn is_key_equal(a: *mut u8, es: usize, key: *const u8, ks: usize, ko: usize, mode: i32, i: isize) -> bool {
    if mode >= HM_STRING {
        let stored = *(a.add(es * i as usize + ko) as *const *const i8);
        libc::strcmp(key as *const i8, stored) == 0
    } else {
        libc::memcmp(key as *const _, a.add(es * i as usize + ko) as *const _, ks) == 0
    }
}

unsafe fn make_hash_index(slot_count: usize, ot: *mut HashIndex) -> *mut HashIndex {
    let nb = slot_count >> BUCKET_SHIFT;
    let sz = nb * std::mem::size_of::<HashBucket>() + std::mem::size_of::<HashIndex>() + CACHE_LINE_SIZE - 1;
    let t = libc::realloc(ptr::null_mut(), sz) as *mut HashIndex;
    let after = (t as usize) + std::mem::size_of::<HashIndex>();
    (*t).storage = ((after + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)) as *mut HashBucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;
    (*t).used_count_threshold = slot_count - (slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*t).used_count_shrink_threshold = if slot_count <= BUCKET_LENGTH { 0 } else { slot_count >> 2 };

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut StringArena as *mut u8, 0, std::mem::size_of::<StringArena>());
        (*t).seed = HASH_SEED;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd)
        let mut temp: usize;
        temp = 0x87b0b0fd_usize ^ 2147001325_usize;
        temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
        let mut a_val: usize = 0x27bb2ee6_usize;
        a_val <<= 16; a_val <<= 16;
        a_val ^= temp ^ 2147001325_usize;
        // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d)
        temp = 0xb504f32d_usize ^ 715136305_usize;
        temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
        let mut b_val: usize = 0_usize;
        b_val <<= 16; b_val <<= 16;
        b_val ^= temp ^ 715136305_usize;
        HASH_SEED = HASH_SEED.wrapping_mul(a_val).wrapping_add(b_val);
    }

    for i in 0..nb {
        let b = &mut *(*t).storage.add(i);
        for j in 0..BUCKET_LENGTH { b.hash[j] = HASH_EMPTY; }
        for j in 0..BUCKET_LENGTH { b.index[j] = INDEX_EMPTY; }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        for i in 0..((*ot).slot_count >> BUCKET_SHIFT) {
            let ob = &*(*ot).storage.add(i);
            for j in 0..BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let hash = ob.hash[j];
                    let mut pos = probe_pos(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = BUCKET_LENGTH;
                    'rehash: loop {
                        let bucket = &mut *(*t).storage.add(pos >> BUCKET_SHIFT);
                        let mut z = pos & BUCKET_MASK;
                        while z < BUCKET_LENGTH {
                            if bucket.hash[z] == 0 { bucket.hash[z] = hash; bucket.index[z] = ob.index[j]; break 'rehash; }
                            z += 1;
                        }
                        let lim = pos & BUCKET_MASK;
                        z = 0;
                        while z < lim {
                            if bucket.hash[z] == 0 { bucket.hash[z] = hash; bucket.index[z] = ob.index[j]; break 'rehash; }
                            z += 1;
                        }
                        pos += step; step += BUCKET_LENGTH; pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }
    t
}

unsafe fn hm_find_slot(a: *mut u8, es: usize, key: *const u8, ks: usize, ko: usize, mode: i32) -> isize {
    let raw_a = h2a(a, es);
    let table = htab(raw_a);
    let mut hv = if mode >= HM_STRING { hash_string(key, (*table).seed) } else { siphash_bytes(key, ks, (*table).seed) };
    if hv < 2 { hv += 2; }
    let mut step = BUCKET_LENGTH;
    let mut pos = probe_pos(hv, (*table).slot_count, (*table).slot_count_log2);
    loop {
        let bucket = &*(*table).storage.add(pos >> BUCKET_SHIFT);
        let mut i = pos & BUCKET_MASK;
        while i < BUCKET_LENGTH {
            if bucket.hash[i] == hv {
                if is_key_equal(a, es, key, ks, ko, mode, bucket.index[i]) { return ((pos & !BUCKET_MASK) + i) as isize; }
            } else if bucket.hash[i] == HASH_EMPTY { return -1; }
            i += 1;
        }
        let lim = pos & BUCKET_MASK;
        i = 0;
        while i < lim {
            if bucket.hash[i] == hv {
                if is_key_equal(a, es, key, ks, ko, mode, bucket.index[i]) { return ((pos & !BUCKET_MASK) + i) as isize; }
            } else if bucket.hash[i] == HASH_EMPTY { return -1; }
            i += 1;
        }
        pos += step; step += BUCKET_LENGTH; pos &= (*table).slot_count - 1;
    }
}

pub unsafe fn hmfree_func(a: *mut u8, es: usize) {
    if a.is_null() { return; }
    let ht = htab(a);
    if !ht.is_null() {
        if (*ht).string.mode == SH_STRDUP {
            for i in 1..(*hdr(a)).length {
                libc::free(*(a.add(es * i) as *const *mut u8) as *mut _);
            }
        }
        strreset(&mut (*ht).string);
    }
    libc::free((*hdr(a)).hash_table as *mut _);
    libc::free(hdr(a) as *mut _);
}

unsafe fn hmget_key_ts_inner(a: *mut u8, es: usize, key: *const u8, ks: usize, temp: &mut isize, mode: i32) -> *mut u8 {
    let ko: usize = 0;
    if a.is_null() {
        let b = arrgrowf(ptr::null_mut(), es, 0, 1);
        (*hdr(b)).length += 1;
        ptr::write_bytes(b, 0, es);
        *temp = INDEX_EMPTY;
        return a2h(b, es);
    } else {
        let raw_a = h2a(a, es);
        let table = (*hdr(raw_a)).hash_table as *mut HashIndex;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = hm_find_slot(a, es, key, ks, ko, mode);
            if slot < 0 {
                *temp = INDEX_EMPTY;
            } else {
                let b = &(*table).storage.add((slot as usize) >> BUCKET_SHIFT);
                *temp = (**b).index[slot as usize & BUCKET_MASK];
            }
        }
        return a;
    }
}

unsafe fn hmput_default_inner(a: *mut u8, es: usize) -> *mut u8 {
    if a.is_null() || (*hdr(h2a(a, es))).length == 0 {
        let raw = if !a.is_null() { h2a(a, es) } else { ptr::null_mut() };
        let b = arrgrowf(raw, es, 0, 1);
        (*hdr(b)).length += 1;
        ptr::write_bytes(b, 0, es);
        return a2h(b, es);
    }
    a
}

unsafe fn hmdel_key_inner(a: *mut u8, es: usize, key: *const u8, ks: usize, ko: usize, mode: i32) -> *mut u8 {
    if a.is_null() { return ptr::null_mut(); }
    let raw_a = h2a(a, es);
    let table = (*hdr(raw_a)).hash_table as *mut HashIndex;
    (*hdr(raw_a)).temp = 0;
    if table.is_null() { return a; }
    let slot = hm_find_slot(a, es, key, ks, ko, mode);
    if slot < 0 { return a; }
    let b = &mut *(*table).storage.add((slot as usize) >> BUCKET_SHIFT);
    let si = slot as usize & BUCKET_MASK;
    let old_index = b.index[si];
    let final_index = arr_len(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*hdr(raw_a)).temp = 1;
    b.hash[si] = HASH_DELETED;
    b.index[si] = INDEX_DELETED;

    if mode == HM_STRING && (*table).string.mode == SH_STRDUP {
        libc::free(*(a.add(es * old_index as usize) as *const *mut u8) as *mut _);
    }

    if old_index != final_index {
        libc::memmove(
            a.add(es * old_index as usize) as *mut _,
            a.add(es * final_index as usize) as *const _,
            es,
        );
        let new_slot = if mode == HM_STRING {
            hm_find_slot(a, es, *(a.add(es * old_index as usize + ko) as *const *const u8), ks, ko, mode)
        } else {
            hm_find_slot(a, es, a.add(es * old_index as usize + ko), ks, ko, mode)
        };
        assert!(new_slot >= 0);
        let nb = &mut *(*table).storage.add((new_slot as usize) >> BUCKET_SHIFT);
        let ni = new_slot as usize & BUCKET_MASK;
        assert!(nb.index[ni] == final_index);
        nb.index[ni] = old_index;
    }
    (*hdr(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > BUCKET_LENGTH {
        (*hdr(raw_a)).hash_table = make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        libc::free(table as *mut _);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*hdr(raw_a)).hash_table = make_hash_index((*table).slot_count, table) as *mut u8;
        libc::free(table as *mut _);
    }
    a
}

pub unsafe fn hmput_key(a_in: *mut u8, es: usize, key: *const u8, ks: usize, mode: i32) -> *mut u8 {
    let ko: usize = 0;
    let mut a: *mut u8;
    if a_in.is_null() {
        a = arrgrowf(ptr::null_mut(), es, 0, 1);
        ptr::write_bytes(a, 0, es);
        (*hdr(a)).length += 1;
        a = a2h(a, es);
    } else {
        a = a_in;
    }
    let raw_a = a;
    let a_arr = h2a(a, es);
    let mut table = (*hdr(a_arr)).hash_table as *mut HashIndex;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let sc = if table.is_null() { BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = make_hash_index(sc, table);
        if !table.is_null() { libc::free(table as *mut _); }
        else { (*nt).string.mode = if mode >= HM_STRING { SH_DEFAULT } else { 0 }; }
        (*hdr(a_arr)).hash_table = nt as *mut u8;
        table = nt;
    }

    let mut hv = if mode >= HM_STRING { hash_string(key, (*table).seed) } else { siphash_bytes(key, ks, (*table).seed) };
    if hv < 2 { hv += 2; }
    let mut step = BUCKET_LENGTH;
    let mut tombstone: isize = -1;
    let mut pos = probe_pos(hv, (*table).slot_count, (*table).slot_count_log2);

    let found_pos: usize;
    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> BUCKET_SHIFT);
        let mut i = pos & BUCKET_MASK;
        while i < BUCKET_LENGTH {
            if bucket.hash[i] == hv {
                if is_key_equal(raw_a, es, key, ks, ko, mode, bucket.index[i]) {
                    (*hdr(a_arr)).temp = bucket.index[i];
                    if mode >= HM_STRING {
                        let stored = *(raw_a.add(es * bucket.index[i] as usize + ko) as *const *mut u8);
                        *((*hdr(a_arr)).hash_table as *mut *mut u8) = stored;
                    }
                    return a2h(a_arr, es);
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && bucket.index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
            i += 1;
        }
        let lim = pos & BUCKET_MASK;
        i = 0;
        while i < lim {
            if bucket.hash[i] == hv {
                if is_key_equal(raw_a, es, key, ks, ko, mode, bucket.index[i]) {
                    (*hdr(a_arr)).temp = bucket.index[i];
                    return a2h(a_arr, es);
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && bucket.index[i] == INDEX_DELETED {
                tombstone = ((pos & !BUCKET_MASK) + i) as isize;
            }
            i += 1;
        }
        pos += step; step += BUCKET_LENGTH; pos &= (*table).slot_count - 1;
    }

    // found_empty_slot:
    let mut fp = found_pos;
    if tombstone >= 0 { fp = tombstone as usize; (*table).tombstone_count -= 1; }
    (*table).used_count += 1;

    let idx = arr_len(a_arr);
    let mut a_arr = a_arr;
    if (idx as usize) + 1 > arr_cap(a_arr) {
        a_arr = arrgrowf(a_arr, es, 1, 0);
    }
    let raw_a2 = a2h(a_arr, es);
    (*hdr(a_arr)).length = (idx + 1) as usize;
    let bucket = &mut *(*table).storage.add(fp >> BUCKET_SHIFT);
    bucket.hash[fp & BUCKET_MASK] = hv;
    bucket.index[fp & BUCKET_MASK] = idx - 1;
    (*hdr(a_arr)).temp = idx - 1;

    match (*table).string.mode {
        SH_STRDUP => {
            let dup = stbds_strdup(key);
            *(a_arr.add(es * idx as usize) as *mut *mut u8) = dup;
            *((*hdr(a_arr)).hash_table as *mut *mut u8) = dup;
        }
        SH_ARENA => {
            let s = stralloc(&mut (*table).string, key);
            *(a_arr.add(es * idx as usize) as *mut *mut u8) = s;
            *((*hdr(a_arr)).hash_table as *mut *mut u8) = s;
        }
        SH_DEFAULT => {
            *(a_arr.add(es * idx as usize) as *mut *mut u8) = key as *mut u8;
            *((*hdr(a_arr)).hash_table as *mut *mut u8) = key as *mut u8;
        }
        _ => {
            libc::memcpy(a_arr.add(es * idx as usize) as *mut _, key as *const _, ks);
        }
    }

    a2h(a_arr, es)
}

pub unsafe fn shmode_func(es: usize, mode: i32) -> *mut u8 {
    let a = arrgrowf(ptr::null_mut(), es, 0, 1);
    ptr::write_bytes(a, 0, es);
    (*hdr(a)).length = 1;
    let h = make_hash_index(BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*hdr(a)).hash_table = h as *mut u8;
    a2h(a, es)
}

unsafe fn stbds_strdup(s: *const u8) -> *mut u8 {
    let len = libc::strlen(s as *const i8) + 1;
    let p = libc::realloc(ptr::null_mut(), len) as *mut u8;
    libc::memmove(p as *mut _, s as *const _, len);
    p
}

pub unsafe fn stralloc(arena: &mut StringArena, s: *const u8) -> *mut u8 {
    let len = libc::strlen(s as *const i8) + 1;
    if len > arena.remaining {
        let bs_shift = arena.block >> 1;
        let mut blocksize = STRING_ARENA_BLOCKSIZE_MIN << (bs_shift as usize);
        if blocksize < STRING_ARENA_BLOCKSIZE_MAX { arena.block += 1; }
        if len > blocksize {
            let sb = libc::realloc(ptr::null_mut(), std::mem::size_of::<StringBlock>() - 8 + len) as *mut StringBlock;
            libc::memmove((*sb).storage.as_mut_ptr() as *mut _, s as *const _, len);
            if !arena.storage.is_null() {
                (*sb).next = (*arena.storage).next;
                (*arena.storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                arena.storage = sb;
                arena.remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let sb = libc::realloc(ptr::null_mut(), std::mem::size_of::<StringBlock>() - 8 + blocksize) as *mut StringBlock;
            (*sb).next = arena.storage;
            arena.storage = sb;
            arena.remaining = blocksize;
        }
    }
    assert!(len <= arena.remaining);
    let p = (*arena.storage).storage.as_mut_ptr().add(arena.remaining - len);
    arena.remaining -= len;
    libc::memmove(p as *mut _, s as *const _, len);
    p
}

pub unsafe fn strreset(arena: &mut StringArena) {
    let mut x = arena.storage;
    while !x.is_null() {
        let y = (*x).next;
        libc::free(x as *mut _);
        x = y;
    }
    ptr::write_bytes(arena as *mut StringArena as *mut u8, 0, std::mem::size_of::<StringArena>());
}

// Helper: sprintf into static buffer, return pointer
static mut BUFFER: [u8; 256] = [0u8; 256];

pub unsafe fn strkey(n: c_int) -> *const u8 {
    let written = libc::snprintf(BUFFER.as_mut_ptr() as *mut i8, 256, b"test_%d\0".as_ptr() as *const i8, n);
    BUFFER.as_ptr()
}

// Export wrappers matching C signatures
pub unsafe fn rand_seed(seed: usize) {
    HASH_SEED = seed;
}

pub fn hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
}

pub fn hash_string_export(s: *mut u8, seed: usize) -> usize {
    hash_string(s, seed)
}

pub unsafe fn arrfreef(a: *mut u8) {
    libc::free(hdr(a) as *mut _);
}

pub unsafe fn hmget_key_export(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: i32) -> *mut u8 {
    let mut temp: isize = 0;
    let p = hmget_key_ts_inner(a, elemsize, key, keysize, &mut temp, mode);
    (*hdr(h2a(p, elemsize))).temp = temp;
    p
}

pub unsafe fn hmget_key_ts_export(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, temp: *mut isize, mode: i32) -> *mut u8 {
    hmget_key_ts_inner(a, elemsize, key, keysize, &mut *temp, mode)
}

pub unsafe fn hmput_default_export(a: *mut u8, elemsize: usize) -> *mut u8 {
    hmput_default_inner(a, elemsize)
}

pub unsafe fn hmdel_key_export(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: i32) -> *mut u8 {
    hmdel_key_inner(a, elemsize, key, keysize, keyoffset, mode)
}

pub fn str_dups_impl(num: c_int) {
    unsafe {
        let es = std::mem::size_of::<StrMapEntry>();
        let mut strmap: *mut u8 = ptr::null_mut();
        let mut sa = StringArena { storage: ptr::null_mut(), remaining: 0, block: 0, mode: 0 };

        // Allocate arena entries
        for i in 0..num {
            stralloc(&mut sa, strkey(i));
        }
        strreset(&mut sa);

        // sh_new_strdup(strmap) => strmap = shmode_func(elemsize, SH_STRDUP)
        strmap = shmode_func(es, SH_STRDUP as i32);

        // shputs(strmap, s) where s = {key="a", value=num}
        // shputs does: hmput_key_wrapper with HM_STRING, then assigns struct, then sets key to temp_key
        let key_a = b"a\0".as_ptr();
        strmap = hmput_key(strmap, es, key_a, std::mem::size_of::<*mut u8>(), HM_STRING);
        let temp = (*hdr(h2a(strmap, es))).temp;
        let entry = strmap.add(es * temp as usize) as *mut StrMapEntry;
        // s.key = "a", s.value = num => (t)[temp] = s
        // But shputs first assigns the whole struct, then overwrites key with temp_key
        (*entry).value = num;
        // shputs sets key = stbds_temp_key which is *(char**) hash_table (the temp_key field of HashIndex)
        let temp_key = *((*hdr(h2a(strmap, es))).hash_table as *const *mut u8);
        (*entry).key = temp_key;

        // Assertions
        assert!(*(*( strmap as *mut StrMapEntry)).key == b'a');
        assert!((*( strmap as *mut StrMapEntry)).key != key_a as *mut u8);
        assert!((*( strmap as *mut StrMapEntry)).value == num);

        // shlen(strmap) = header(strmap-1)->length - 1
        let raw = h2a(strmap, es);
        let len = (*hdr(raw)).length as isize - 1;

        for z in 0..len {
            let e = &*(strmap.add(es * z as usize) as *const StrMapEntry);
            libc::printf(b"%s %d\n\0".as_ptr() as *const i8, e.key, e.value);
        }

        // shfree(strmap) = hmfree
        hmfree_func(h2a(strmap, es), es);
    }
}
