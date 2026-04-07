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

// ---- Constants ----
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
const STBDS_SIZE_T_BITS: usize = 64;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;
const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

static mut stbds_hash_seed: usize = 0x31415926;
static mut strkey_buffer: [u8; 256] = [0u8; 256];

// ---- Structs ----
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
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
pub struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [i8; 8],
}

#[repr(C)]
struct stbds_hash_index {
    temp_key: *mut i8,
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

// ---- Internal helpers ----
unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*stbds_header(a)).length as isize }
}

unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*stbds_header(a)).capacity }
}

unsafe fn stbds_realloc(p: *mut u8, s: usize) -> *mut u8 {
    libc::realloc(p as *mut libc::c_void, s) as *mut u8
}

unsafe fn stbds_free(p: *mut u8) {
    libc::free(p as *mut libc::c_void);
}

#[inline]
unsafe fn stbds_hash_table(a: *mut u8) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline]
unsafe fn stbds_temp(a: *mut u8) -> &'static mut isize {
    &mut (*stbds_header(a)).temp
}

#[inline]
unsafe fn stbds_temp_key(a: *mut u8) -> &'static mut *mut i8 {
    &mut *((*stbds_header(a)).hash_table as *mut *mut i8)
}

#[inline]
fn STBDS_HASH_TO_ARR(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.sub(elemsize) }
}

#[inline]
fn STBDS_ARR_TO_HASH(x: *mut u8, elemsize: usize) -> *mut u8 {
    unsafe { x.add(elemsize) }
}

#[inline]
fn STBDS_ALIGN_FWD(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn STBDS_ROTATE_LEFT(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn STBDS_ROTATE_RIGHT(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[inline]
fn STBDS_INDEX_IN_USE(x: isize) -> bool {
    x >= 0
}

// ---- stbds_arrgrowf ----
unsafe fn stbds_arrgrowf_internal(
    a: *mut u8,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut u8 {
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= stbds_arrcap(a) {
        return a;
    }
    let double_cap = stbds_arrcap(a).wrapping_mul(2);
    if min_cap < double_cap {
        min_cap = double_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(core::mem::size_of::<stbds_array_header>());
    let old_ptr = if !a.is_null() {
        stbds_header(a) as *mut u8
    } else {
        ptr::null_mut()
    };
    let b_raw = stbds_realloc(old_ptr, alloc_size);
    let b = b_raw.add(core::mem::size_of::<stbds_array_header>());
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut u8,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut u8 {
    stbds_arrgrowf_internal(a, elemsize, addlen, min_cap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) {
    stbds_free(stbds_header(a) as *mut u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

unsafe fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

// ---- stbds_make_hash_index ----
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT)
        * core::mem::size_of::<stbds_hash_bucket>()
        + core::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    (*t).storage = STBDS_ALIGN_FWD(
        t.add(1) as usize,
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

    if !ot.is_null() {
        (*t).string = core::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        core::ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = stbds_hash_seed;
        // stbds_load_32_or_64 for a
        let mut temp: usize;
        let mut a: usize;
        temp = 0x87b0b0fd_usize ^ 2147001325_usize;
        temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
        a = 0x27bb2ee6_usize;
        a <<= 16; a <<= 16;
        a ^= temp ^ 2147001325_usize;
        // stbds_load_32_or_64 for b
        let mut b: usize;
        temp = 0xb504f32d_usize ^ 715136305_usize;
        temp <<= 16; temp <<= 16; temp >>= 16; temp >>= 16;
        b = 0_usize;
        b <<= 16; b <<= 16;
        b ^= temp ^ 715136305_usize;
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
                if STBDS_INDEX_IN_USE(ob.index[j]) {
                    let hash = ob.hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
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

// ---- Hash functions ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut i8, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str as *mut u8;
    while *s != 0 {
        hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
    }
    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ STBDS_ROTATE_RIGHT(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= STBDS_ROTATE_RIGHT(hash, 22);
    hash.wrapping_add(seed)
}

unsafe fn stbds_siphash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
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
            v0 = v0.wrapping_add(v1); v1 = STBDS_ROTATE_LEFT(v1, 13); v1 ^= v0; v0 = STBDS_ROTATE_LEFT(v0, (STBDS_SIZE_T_BITS / 2) as u32);
            v2 = v2.wrapping_add(v3); v3 = STBDS_ROTATE_LEFT(v3, 16); v3 ^= v2;
            v2 = v2.wrapping_add(v1); v1 = STBDS_ROTATE_LEFT(v1, 17); v1 ^= v2; v2 = STBDS_ROTATE_LEFT(v2, (STBDS_SIZE_T_BITS / 2) as u32);
            v0 = v0.wrapping_add(v3); v3 = STBDS_ROTATE_LEFT(v3, 21); v3 ^= v0;
        };
    }

    let mut i: usize = 0;
    while i + core::mem::size_of::<usize>() <= len {
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
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        i += core::mem::size_of::<usize>();
    }

    // Tail bytes - C uses fallthrough switch
    data = len << (STBDS_SIZE_T_BITS - 8);
    let remaining = len - i;
    let dp = d.add(i);
    if remaining >= 7 { data |= (*dp.add(6) as usize) << 24 << 24; }
    if remaining >= 6 { data |= (*dp.add(5) as usize) << 20 << 20; }
    if remaining >= 5 { data |= (*dp.add(4) as usize) << 16 << 16; }
    if remaining >= 4 { data |= (*dp.add(3) as usize) << 24; }
    if remaining >= 3 { data |= (*dp.add(2) as usize) << 16; }
    if remaining >= 2 { data |= (*dp.add(1) as usize) << 8; }
    if remaining >= 1 { data |= *dp.add(0) as usize; }

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---- Key comparison ----
unsafe fn stbds_is_key_equal(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: i32,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let stored = *(a.add(elemsize * i as usize + keyoffset) as *const *const i8);
        libc::strcmp(key as *const i8, stored) == 0
    } else {
        libc::memcmp(
            key as *const libc::c_void,
            a.add(elemsize * i as usize + keyoffset) as *const libc::c_void,
            keysize,
        ) == 0
    }
}

// ---- stbds_hm_find_slot ----
unsafe fn stbds_hm_find_slot(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: i32,
) -> isize {
    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
    let table = stbds_hash_table(raw_a);
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

// ---- stbds_strdup ----
unsafe fn stbds_strdup(str: *mut i8) -> *mut i8 {
    let len = libc::strlen(str as *const i8) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut i8;
    libc::memmove(p as *mut libc::c_void, str as *const libc::c_void, len);
    p
}

// ---- stbds_hmfree_func ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() { return; }
    let ht = stbds_hash_table(a);
    if !ht.is_null() {
        if (*ht).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*stbds_header(a)).length {
                stbds_free(*(a.add(elemsize * i) as *mut *mut u8));
            }
        }
        stbds_strreset_internal(&mut (*ht).string);
    }
    stbds_free((*stbds_header(a)).hash_table);
    stbds_free(stbds_header(a) as *mut u8);
}

// ---- stbds_hmget_key_ts ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    temp: *mut isize,
    mode: i32,
) -> *mut u8 {
    let keyoffset: usize = 0;
    if a.is_null() {
        let new_a = stbds_arrgrowf_internal(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        libc::memset(new_a as *mut libc::c_void, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return STBDS_ARR_TO_HASH(new_a, elemsize);
    } else {
        let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
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
        return a;
    }
}

// ---- stbds_hmget_key ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    mode: i32,
) -> *mut u8 {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    *stbds_temp(STBDS_HASH_TO_ARR(p, elemsize)) = temp;
    p
}

// ---- stbds_hmput_default ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    if a.is_null() || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0 {
        let base = if !a.is_null() {
            STBDS_HASH_TO_ARR(a, elemsize)
        } else {
            ptr::null_mut()
        };
        let new_a = stbds_arrgrowf_internal(base, elemsize, 0, 1);
        (*stbds_header(new_a)).length += 1;
        libc::memset(new_a as *mut libc::c_void, 0, elemsize);
        return STBDS_ARR_TO_HASH(new_a, elemsize);
    }
    a
}

// ---- stbds_hmput_key ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    mode: i32,
) -> *mut u8 {
    let keyoffset: usize = 0;
    let mut a = a;
    let raw_a: *mut u8;

    if a.is_null() {
        let new_a = stbds_arrgrowf_internal(ptr::null_mut(), elemsize, 0, 1);
        libc::memset(new_a as *mut libc::c_void, 0, elemsize);
        (*stbds_header(new_a)).length += 1;
        a = STBDS_ARR_TO_HASH(new_a, elemsize);
    }

    let mut raw_a_val = a;
    let mut arr = STBDS_HASH_TO_ARR(a, elemsize);

    let mut table = (*stbds_header(arr)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            stbds_free(table as *mut u8);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
        }
        (*stbds_header(arr)).hash_table = nt as *mut u8;
        table = nt;
    }

    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut i8, (*table).seed)
        } else {
            stbds_siphash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut tombstone: isize = -1;

        if hash < 2 { hash += 2; }

        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        loop {
            let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(raw_a_val, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                        *stbds_temp(arr) = bucket.index[i];
                        if mode >= STBDS_HM_STRING {
                            *stbds_temp_key(arr) = *(raw_a_val.add(elemsize * bucket.index[i] as usize + keyoffset) as *const *mut i8);
                        }
                        return STBDS_ARR_TO_HASH(arr, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    // goto found_empty_slot
                    return stbds_hmput_key_found_empty(arr, raw_a_val, elemsize, key, keysize, keyoffset, mode, table, hash, pos, tombstone);
                } else if tombstone < 0 {
                    if bucket.index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(raw_a_val, elemsize, key, keysize, keyoffset, mode, bucket.index[i]) {
                        *stbds_temp(arr) = bucket.index[i];
                        return STBDS_ARR_TO_HASH(arr, elemsize);
                    }
                } else if bucket.hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    return stbds_hmput_key_found_empty(arr, raw_a_val, elemsize, key, keysize, keyoffset, mode, table, hash, pos, tombstone);
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

unsafe fn stbds_hmput_key_found_empty(
    mut arr: *mut u8,
    mut raw_a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: i32,
    table: *mut stbds_hash_index,
    hash: usize,
    mut pos: usize,
    tombstone: isize,
) -> *mut u8 {
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(arr);
    if (i as usize) + 1 > stbds_arrcap(arr) {
        arr = stbds_arrgrowf_internal(arr, elemsize, 1, 0);
    }
    raw_a = STBDS_ARR_TO_HASH(arr, elemsize);

    (*stbds_header(arr)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
    *stbds_temp(arr) = i - 1;

    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let dup = stbds_strdup(key as *mut i8);
            *(arr.add(elemsize * i as usize) as *mut *mut i8) = dup;
            *stbds_temp_key(arr) = dup;
        }
        STBDS_SH_ARENA => {
            let alloc = stbds_stralloc_internal(&mut (*table).string, key as *mut i8);
            *(arr.add(elemsize * i as usize) as *mut *mut i8) = alloc;
            *stbds_temp_key(arr) = alloc;
        }
        STBDS_SH_DEFAULT => {
            *(arr.add(elemsize * i as usize) as *mut *mut i8) = key as *mut i8;
            *stbds_temp_key(arr) = key as *mut i8;
        }
        _ => {
            libc::memcpy(
                arr.add(elemsize * i as usize) as *mut libc::c_void,
                key as *const libc::c_void,
                keysize,
            );
        }
    }
    STBDS_ARR_TO_HASH(arr, elemsize)
}

// ---- stbds_hmdel_key ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut u8,
    elemsize: usize,
    key: *mut u8,
    keysize: usize,
    keyoffset: usize,
    mode: i32,
) -> *mut u8 {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
    let mut table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    *stbds_temp(raw_a) = 0;
    if table.is_null() {
        return a;
    }
    let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }
    let b = &mut *(*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let bi = slot as usize & STBDS_BUCKET_MASK;
    let old_index = b.index[bi];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;

    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    b.hash[bi] = STBDS_HASH_DELETED;
    b.index[bi] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        stbds_free(*(a.add(elemsize * old_index as usize) as *mut *mut u8));
    }

    if old_index != final_index {
        libc::memmove(
            a.add(elemsize * old_index as usize) as *mut libc::c_void,
            a.add(elemsize * final_index as usize) as *const libc::c_void,
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
        let b2 = &mut *(*table).storage.add(slot2 as usize >> STBDS_BUCKET_SHIFT);
        let bi2 = slot2 as usize & STBDS_BUCKET_MASK;
        b2.index[bi2] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut u8;
        stbds_free(table as *mut u8);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut u8;
        stbds_free(table as *mut u8);
    }

    a
}

// ---- stbds_shmode_func ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: i32) -> *mut u8 {
    let a = stbds_arrgrowf_internal(ptr::null_mut(), elemsize, 0, 1);
    libc::memset(a as *mut libc::c_void, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut u8;
    STBDS_ARR_TO_HASH(a, elemsize)
}

// ---- stbds_stralloc / stbds_strreset ----
unsafe fn stbds_stralloc_internal(a: *mut stbds_string_arena, str: *mut i8) -> *mut i8 {
    let len = libc::strlen(str as *const i8) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }
        if len > blocksize {
            let sb = stbds_realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            libc::memmove(
                (*sb).storage.as_mut_ptr() as *mut libc::c_void,
                str as *const libc::c_void,
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
            let sb = stbds_realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }
    let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
        .add((*a).remaining - len) as *mut i8;
    (*a).remaining -= len;
    libc::memmove(p as *mut libc::c_void, str as *const libc::c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds_string_arena, str: *mut i8) -> *mut i8 {
    stbds_stralloc_internal(a, str)
}

unsafe fn stbds_strreset_internal(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut u8);
        x = y;
    }
    libc::memset(a as *mut libc::c_void, 0, core::mem::size_of::<stbds_string_arena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    stbds_strreset_internal(a);
}

// ---- strkey ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: i32) -> *mut i8 {
    let fmt = b"test_%d\0";
    libc::sprintf(
        strkey_buffer.as_mut_ptr() as *mut i8,
        fmt.as_ptr() as *const i8,
        n,
    );
    strkey_buffer.as_mut_ptr() as *mut i8
}

// ---- arr_ins ----
#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    unsafe {
        let elemsize = core::mem::size_of::<c_int>();
        let mut arr: *mut c_int = ptr::null_mut();

        for i in 0..5i32 {
            for v in 1..=4i32 {
                let a = arr as *mut u8;
                if a.is_null() || (*stbds_header(a)).length + 1 > (*stbds_header(a)).capacity {
                    arr = stbds_arrgrowf_internal(a, elemsize, 1, 0) as *mut c_int;
                }
                let a = arr as *mut u8;
                let idx = (*stbds_header(a)).length;
                (*stbds_header(a)).length += 1;
                *arr.add(idx) = v;
            }

            // stbds_arrins(arr, i, num) -> arrinsn(arr,i,1), arr[i]=num
            {
                let n: usize = 1;
                let a = arr as *mut u8;
                if a.is_null() || (*stbds_header(a)).length + n > (*stbds_header(a)).capacity {
                    arr = stbds_arrgrowf_internal(a, elemsize, n, 0) as *mut c_int;
                }
                let a = arr as *mut u8;
                (*stbds_header(a)).length += n;
                let len = (*stbds_header(a)).length;
                let count = len - n - (i as usize);
                ptr::copy(arr.add(i as usize), arr.add(i as usize + n), count);
                *arr.add(i as usize) = num;
            }

            assert_eq!(*arr.add(i as usize), num);
            if i < 4 {
                assert_eq!(*arr.add(4), 4);
            }

            if !arr.is_null() {
                stbds_free(stbds_header(arr as *mut u8) as *mut u8);
            }
            arr = ptr::null_mut();
        }
    }
}
