#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::c_int;
use std::ptr;

// ============================================================
// Constants
// ============================================================

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

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() * 8) as u32;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// ============================================================
// Structs
// ============================================================

#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut std::ffi::c_void,
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

// ============================================================
// Global state
// ============================================================

static mut stbds_hash_seed: usize = 0x31415926;

// ============================================================
// Helper macros as inline functions
// ============================================================

#[inline(always)]
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline(always)]
unsafe fn stbds_temp(t: *mut u8) -> &'static mut isize {
    &mut (*stbds_header(t)).temp
}

#[inline(always)]
unsafe fn stbds_temp_key(t: *mut u8) -> *mut *mut u8 {
    (*stbds_header(t)).hash_table as *mut *mut u8
}

#[inline(always)]
fn stbds_hash_to_arr(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.sub(elemsize) }
}

#[inline(always)]
fn stbds_arr_to_hash(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.add(elemsize) }
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline(always)]
unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline(always)]
unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline(always)]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline(always)]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// ============================================================
// Core functions
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut std::ffi::c_void {
    let a = a as *mut u8;
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) + addlen as isize) as usize;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a as *mut std::ffi::c_void;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b = if !a.is_null() {
        libc::realloc(stbds_header(a) as *mut std::ffi::c_void, alloc_size)
    } else {
        libc::realloc(ptr::null_mut(), alloc_size)
    };
    let b = (b as *mut u8).add(std::mem::size_of::<stbds_array_header>());
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut std::ffi::c_void) {
    libc::free(stbds_header(a as *mut u8) as *mut std::ffi::c_void);
}

unsafe fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = libc::realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
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
        (*t).string = ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = stbds_hash_seed;
        let (a, b): (usize, usize);
        // stbds_load_32_or_64 for a: v32=2147001325, v64_hi=0x27bb2ee6, v64_lo=0x87b0b0fd
        {
            let v32: usize = 2147001325;
            let v64_hi: usize = 0x27bb2ee6;
            let v64_lo: usize = 0x87b0b0fd;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut va: usize = v64_hi;
            va <<= 16;
            va <<= 16;
            va ^= temp ^ v32;
            a = va;
        }
        // stbds_load_32_or_64 for b: v32=715136305, v64_hi=0, v64_lo=0xb504f32d
        {
            let v32: usize = 715136305;
            let v64_hi: usize = 0;
            let v64_lo: usize = 0xb504f32d;
            let mut temp: usize = v64_lo ^ v32;
            temp <<= 16;
            temp <<= 16;
            temp >>= 16;
            temp >>= 16;
            let mut vb: usize = v64_hi;
            vb <<= 16;
            vb <<= 16;
            vb ^= temp ^ v32;
            b = vb;
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
                if stbds_index_in_use(ob.index[j]) {
                    let hash = ob.hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = &mut *(*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let start = pos & STBDS_BUCKET_MASK;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }

                        for z in 0..start {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
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

// ============================================================
// Hash functions
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str;
    while *s != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_u64 as usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;
    v2 ^= 0x0706050403020100_u64 as usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 as usize ^ !seed;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = rotate_left(v1, 13);
            v1 ^= v0;
            v0 = rotate_left(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotate_left(v1, 17);
            v1 ^= v2;
            v2 = rotate_left(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = rotate_left(v3, 21);
            v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dp = d.add(i);
        data = *dp.add(0) as usize
            | ((*dp.add(1) as usize) << 8)
            | ((*dp.add(2) as usize) << 16)
            | ((*dp.add(3) as usize) << 24);
        data |= ((*dp.add(4) as usize)
            | ((*dp.add(5) as usize) << 8)
            | ((*dp.add(6) as usize) << 16)
            | ((*dp.add(7) as usize) << 24))
            << 16
            << 16;

        v3 ^= data;
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let remaining = len - i;
    let dp = d.add(i);
    // C fallthrough switch
    if remaining >= 7 {
        data |= (*dp.add(6) as usize) << 48;
    }
    if remaining >= 6 {
        // case 6: data |= ((size_t) d[5] << 20) << 20;
        data |= (*dp.add(5) as usize) << 40;
    }
    if remaining >= 5 {
        // case 5: data |= ((size_t) d[4] << 16) << 16;
        data |= (*dp.add(4) as usize) << 32;
    }
    if remaining >= 4 {
        data |= (*dp.add(3) as usize) << 24;
    }
    if remaining >= 3 {
        data |= (*dp.add(2) as usize) << 16;
    }
    if remaining >= 2 {
        data |= (*dp.add(1) as usize) << 8;
    }
    if remaining >= 1 {
        data |= *dp.add(0) as usize;
    }

    v3 ^= data;
    for _ in 0..2 {
        sipround!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(
    p: *mut std::ffi::c_void,
    len: usize,
    seed: usize,
) -> usize {
    stbds_siphash_bytes(p as *const u8, len, seed)
}

// ============================================================
// Key comparison and hash map core
// ============================================================

unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored_key_ptr = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        libc::strcmp(key as *const i8, stored_key_ptr as *const i8) == 0
    } else {
        libc::memcmp(
            key as *const std::ffi::c_void,
            a.add(elemsize * i as usize + keyoffset) as *const std::ffi::c_void,
            keysize,
        ) == 0
    }
}

unsafe fn stbds_hm_find_slot(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key as *mut std::ffi::c_void, keysize, (*table).seed)
    };
    let hash = if hash < 2 { hash + 2 } else { hash };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i])
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, bucket.index[i])
                {
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
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut std::ffi::c_void, elemsize: usize) {
    let a = a as *mut u8;
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*stbds_header(a)).length {
                libc::free(*(a.add(elemsize * i) as *const *mut u8) as *mut std::ffi::c_void);
            }
        }
        stbds_strreset(&mut (*stbds_hash_table(a)).string);
    }
    libc::free((*stbds_header(a)).hash_table);
    libc::free(stbds_header(a) as *mut std::ffi::c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let a = a as *mut u8;
    let key = key as *mut u8;
    let keyoffset: usize = 0;
    if a.is_null() {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return stbds_arr_to_hash(a, elemsize) as *mut std::ffi::c_void;
    } else {
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
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
        return a as *mut std::ffi::c_void;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    *stbds_temp(stbds_hash_to_arr(p as *mut u8, elemsize)) = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    a: *mut std::ffi::c_void,
    elemsize: usize,
) -> *mut std::ffi::c_void {
    let a = a as *mut u8;
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let raw = if !a.is_null() {
            stbds_hash_to_arr(a, elemsize) as *mut std::ffi::c_void
        } else {
            ptr::null_mut()
        };
        let new_a = stbds_arrgrowf(raw, elemsize, 0, 1) as *mut u8;
        (*stbds_header(new_a)).length += 1;
        ptr::write_bytes(new_a, 0, elemsize);
        return stbds_arr_to_hash(new_a, elemsize) as *mut std::ffi::c_void;
    }
    a as *mut std::ffi::c_void
}

// ============================================================
// stbds_hmput_key
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let keyoffset: usize = 0;
    let key = key as *mut u8;

    let mut a = a as *mut u8;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
        ptr::write_bytes(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = stbds_arr_to_hash(a, elemsize);
    }

    let raw_a = a;
    let a_arr = stbds_hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a_arr)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            libc::free(table as *mut std::ffi::c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*stbds_header(a_arr)).hash_table = nt as *mut std::ffi::c_void;
        table = nt;
    }

    let hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key as *mut std::ffi::c_void, keysize, (*table).seed)
    };
    let hash = if hash < 2 { hash + 2 } else { hash };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut tombstone: isize = -1;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    let found_pos: usize;

    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                    *stbds_temp(a_arr) = bucket.index[i];
                    if mode >= STBDS_HM_STRING {
                        *stbds_temp_key(a_arr) =
                            *(raw_a.add(elemsize * bucket.index[i] as usize + keyoffset) as *mut *mut u8);
                    }
                    return stbds_arr_to_hash(a_arr, elemsize) as *mut std::ffi::c_void;
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
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
                    *stbds_temp(a_arr) = bucket.index[i];
                    return stbds_arr_to_hash(a_arr, elemsize) as *mut std::ffi::c_void;
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
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

    // found_empty_slot:
    let final_pos = if tombstone >= 0 {
        (*table).tombstone_count -= 1;
        tombstone as usize
    } else {
        found_pos
    };
    (*table).used_count += 1;

    let i = stbds_arrlen(a_arr);
    let mut a_arr = a_arr;
    if (i as usize + 1) > stbds_arrcap(a_arr) {
        a_arr = stbds_arrgrowf(a_arr as *mut std::ffi::c_void, elemsize, 1, 0) as *mut u8;
    }
    let raw_a = stbds_arr_to_hash(a_arr, elemsize);

    assert!((i as usize + 1) <= stbds_arrcap(a_arr));
    (*stbds_header(a_arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(final_pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[final_pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[final_pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(a_arr) = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key);
            *(a_arr.add(elemsize * i as usize) as *mut *mut u8) = dup;
            *stbds_temp_key(a_arr) = dup;
        }
        STBDS_SH_ARENA => {
            let s = stbds_stralloc(
                &mut (*table).string,
                key as *mut i8,
            ) as *mut u8;
            *(a_arr.add(elemsize * i as usize) as *mut *mut u8) = s;
            *stbds_temp_key(a_arr) = s;
        }
        STBDS_SH_DEFAULT => {
            *(a_arr.add(elemsize * i as usize) as *mut *mut u8) = key;
            *stbds_temp_key(a_arr) = key;
        }
        _ => {
            libc::memcpy(
                a_arr.add(elemsize * i as usize) as *mut std::ffi::c_void,
                key as *const std::ffi::c_void,
                keysize,
            );
        }
    }

    stbds_arr_to_hash(a_arr, elemsize) as *mut std::ffi::c_void
}

// ============================================================
// stbds_shmode_func, stbds_hmdel_key, string functions
// ============================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(
    elemsize: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1) as *mut u8;
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut std::ffi::c_void;
    stbds_arr_to_hash(a, elemsize) as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut std::ffi::c_void,
    elemsize: usize,
    key: *mut std::ffi::c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut std::ffi::c_void {
    let a = a as *mut u8;
    let key = key as *mut u8;
    if a.is_null() {
        return ptr::null_mut();
    }

    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    *stbds_temp(raw_a) = 0;
    if table.is_null() {
        return a as *mut std::ffi::c_void;
    }

    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a as *mut std::ffi::c_void;
    }

    let b = &mut *(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = b.index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    assert!((*table).used_count < usize::MAX); // used_count >= 0 always true for usize
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        libc::free(*(a.add(elemsize * old_index as usize) as *const *mut u8) as *mut std::ffi::c_void);
    }

    if old_index != final_index {
        libc::memmove(
            a.add(elemsize * old_index as usize) as *mut std::ffi::c_void,
            a.add(elemsize * final_index as usize) as *const std::ffi::c_void,
            elemsize,
        );

        let slot2 = if mode == STBDS_HM_STRING {
            stbds_hm_find_slot(
                a,
                elemsize,
                *(a.add(elemsize * old_index as usize + keyoffset) as *mut *mut u8),
                keysize,
                keyoffset,
                mode,
            )
        } else {
            stbds_hm_find_slot(
                a,
                elemsize,
                a.add(elemsize * old_index as usize + keyoffset),
                keysize,
                keyoffset,
                mode,
            )
        };
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add(slot2 as usize >> STBDS_BUCKET_SHIFT);
        let i2 = slot2 as usize & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut std::ffi::c_void;
        libc::free(table as *mut std::ffi::c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut std::ffi::c_void;
        libc::free(table as *mut std::ffi::c_void);
    }

    a as *mut std::ffi::c_void
}

unsafe fn stbds_strdup(str: *const u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;
    let p = libc::realloc(ptr::null_mut(), len) as *mut u8;
    libc::memmove(p as *mut std::ffi::c_void, str as *const std::ffi::c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str: *mut i8,
) -> *mut i8 {
    let len = libc::strlen(str) + 1;
    if len > (*a).remaining {
        let blocksize_shift = (*a).block >> 1;
        let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_shift as usize);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = libc::realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            libc::memmove(
                (*sb).storage.as_mut_ptr() as *mut std::ffi::c_void,
                str as *const std::ffi::c_void,
                len,
            );
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
            let sb_size = std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = libc::realloc(ptr::null_mut(), sb_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    libc::memmove(
        p as *mut std::ffi::c_void,
        str as *const std::ffi::c_void,
        len,
    );
    p as *mut i8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        libc::free(x as *mut std::ffi::c_void);
        x = y;
    }
    ptr::write_bytes(a, 0, 1);
}

// ============================================================
// Public API: hm_geti (the only function in lib.h)
// ============================================================

// Reproduces the C struct: struct { int key; int value; }
#[repr(C)]
#[derive(Copy, Clone)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

// Helper macros translated as inline functions operating on *mut IntMapEntry
// These mirror the C macro expansions for the specific IntMapEntry type.

const ENTRY_SIZE: usize = std::mem::size_of::<IntMapEntry>();

/// hmgeti(intmap, k) -> index or -1
unsafe fn hm_hmgeti(intmap: *mut IntMapEntry, k: c_int) -> (*mut IntMapEntry, isize) {
    let mut key_val = k;
    let p = stbds_hmget_key(
        intmap as *mut std::ffi::c_void,
        ENTRY_SIZE,
        &mut key_val as *mut c_int as *mut std::ffi::c_void,
        std::mem::size_of::<c_int>(),
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    let raw_a = stbds_hash_to_arr(p as *mut u8, ENTRY_SIZE);
    let temp = *stbds_temp(raw_a);
    (p, temp)
}

/// hmget(intmap, k) -> value (uses default if not found)
unsafe fn hm_hmget(intmap: *mut IntMapEntry, k: c_int) -> (*mut IntMapEntry, c_int) {
    let (p, _) = hm_hmgeti(intmap, k);
    let raw_a = stbds_hash_to_arr(p as *mut u8, ENTRY_SIZE);
    let temp = *stbds_temp(raw_a);
    let val = (*p.offset(temp)).value;
    (p, val)
}

/// hmget_ts(intmap, k, &mut temp) -> value
unsafe fn hm_hmget_ts(intmap: *mut IntMapEntry, k: c_int) -> (*mut IntMapEntry, c_int, isize) {
    let mut key_val = k;
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(
        intmap as *mut std::ffi::c_void,
        ENTRY_SIZE,
        &mut key_val as *mut c_int as *mut std::ffi::c_void,
        std::mem::size_of::<c_int>(),
        &mut temp,
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    let val = (*p.offset(temp)).value;
    (p, val, temp)
}

/// hmput(intmap, k, v)
unsafe fn hm_hmput(intmap: *mut IntMapEntry, k: c_int, v: c_int) -> *mut IntMapEntry {
    let mut key_val = k;
    let p = stbds_hmput_key(
        intmap as *mut std::ffi::c_void,
        ENTRY_SIZE,
        &mut key_val as *mut c_int as *mut std::ffi::c_void,
        std::mem::size_of::<c_int>(),
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    let raw_a = stbds_hash_to_arr(p as *mut u8, ENTRY_SIZE);
    let temp = *stbds_temp(raw_a);
    (*p.offset(temp)).key = k;
    (*p.offset(temp)).value = v;
    p
}

/// hmdefault(intmap, v)
unsafe fn hm_hmdefault(intmap: *mut IntMapEntry, v: c_int) -> *mut IntMapEntry {
    let p = stbds_hmput_default(
        intmap as *mut std::ffi::c_void,
        ENTRY_SIZE,
    ) as *mut IntMapEntry;
    (*p.offset(-1)).value = v;
    p
}

/// hmdel(intmap, k)
unsafe fn hm_hmdel(intmap: *mut IntMapEntry, k: c_int) -> (*mut IntMapEntry, isize) {
    let mut key_val = k;
    let p = stbds_hmdel_key(
        intmap as *mut std::ffi::c_void,
        ENTRY_SIZE,
        &mut key_val as *mut c_int as *mut std::ffi::c_void,
        std::mem::size_of::<c_int>(),
        0, // keyoffset = STBDS_OFFSETOF for first field = 0
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    let temp = if !p.is_null() {
        let raw_a = stbds_hash_to_arr(p as *mut u8, ENTRY_SIZE);
        *stbds_temp(raw_a)
    } else {
        0
    };
    (p, temp)
}

/// hmfree(intmap)
unsafe fn hm_hmfree(intmap: *mut IntMapEntry) {
    if !intmap.is_null() {
        stbds_hmfree_func(
            intmap.offset(-1) as *mut std::ffi::c_void,
            ENTRY_SIZE,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    let mut intmap: *mut IntMapEntry = ptr::null_mut();

    let mut i: c_int;

    // i = 1; STBDS_ASSERT(hmgeti(intmap,i) == -1);
    i = 1;
    {
        let (p, idx) = hm_hmgeti(intmap, i);
        intmap = p;
        assert!(idx == -1);
    }

    // hmdefault(intmap, -2);
    intmap = hm_hmdefault(intmap, -2);

    // STBDS_ASSERT(hmgeti(intmap, i) == -1);
    {
        let (p, idx) = hm_hmgeti(intmap, i);
        intmap = p;
        assert!(idx == -1);
    }

    // STBDS_ASSERT(hmget(intmap, i) == -2);
    {
        let (p, val) = hm_hmget(intmap, i);
        intmap = p;
        assert!(val == -2);
    }

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*5);
    i = 0;
    while i < num {
        intmap = hm_hmput(intmap, i, i * 5);
        i += 2;
    }

    // for (i=0; i < num; i+=1) { ... }
    i = 0;
    while i < num {
        if (i & 1) != 0 {
            let (p, val) = hm_hmget(intmap, i);
            intmap = p;
            assert!(val == -2);
        } else {
            let (p, val) = hm_hmget(intmap, i);
            intmap = p;
            assert!(val == i * 5);
        }
        if (i & 1) != 0 {
            let (p, val, _temp) = hm_hmget_ts(intmap, i);
            intmap = p;
            assert!(val == -2);
        } else {
            let (p, val, _temp) = hm_hmget_ts(intmap, i);
            intmap = p;
            assert!(val == i * 5);
        }
        i += 1;
    }

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*3);
    i = 0;
    while i < num {
        intmap = hm_hmput(intmap, i, i * 3);
        i += 2;
    }

    // for (i=0; i < num; i+=1) ...
    i = 0;
    while i < num {
        if (i & 1) != 0 {
            let (p, val) = hm_hmget(intmap, i);
            intmap = p;
            assert!(val == -2);
        } else {
            let (p, val) = hm_hmget(intmap, i);
            intmap = p;
            assert!(val == i * 3);
        }
        i += 1;
    }

    // for (i=2; i < num; i+=4) hmdel(intmap, i);
    i = 2;
    while i < num {
        let (p, _) = hm_hmdel(intmap, i);
        intmap = p;
        i += 4;
    }

    // for (i=0; i < num; i+=1) ...
    i = 0;
    while i < num {
        if (i & 3) != 0 {
            let (p, val) = hm_hmget(intmap, i);
            intmap = p;
            assert!(val == -2);
        } else {
            let (p, val) = hm_hmget(intmap, i);
            intmap = p;
            assert!(val == i * 3);
        }
        i += 1;
    }

    // for (i=0; i < num; i+=1) hmdel(intmap, i);
    i = 0;
    while i < num {
        let (p, _) = hm_hmdel(intmap, i);
        intmap = p;
        i += 1;
    }

    // for (i=0; i < num; i+=1) STBDS_ASSERT(hmget(intmap, i) == -2);
    i = 0;
    while i < num {
        let (p, val) = hm_hmget(intmap, i);
        intmap = p;
        assert!(val == -2);
        i += 1;
    }

    // hmfree(intmap);
    hm_hmfree(intmap);
    intmap = ptr::null_mut();

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*3);
    i = 0;
    while i < num {
        intmap = hm_hmput(intmap, i, i * 3);
        i += 2;
    }

    // hmfree(intmap);
    hm_hmfree(intmap);
    // intmap = ptr::null_mut(); // end of function
}
