#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    unused_mut,
    unused_unsafe,
    unused_parens,
    clippy::all
)]

use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = 7;
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
const STBDS_SIZE_T_BITS: usize = 64;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;
const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

static mut stbds_hash_seed: usize = 0x31415926;

static mut buffer: [c_char; 256] = [0; 256];

#[repr(C)]
pub struct StbdsArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct StbdsStringBlock {
    pub next: *mut StbdsStringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct StbdsStringArena {
    pub storage: *mut StbdsStringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
pub struct StbdsHashBucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct StbdsHashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: StbdsStringArena,
    pub storage: *mut StbdsHashBucket,
}

#[inline(always)]
unsafe fn stbds_header(t: *mut c_void) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).sub(1)
}

#[inline(always)]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline(always)]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline(always)]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    (*stbds_header(a)).hash_table as *mut StbdsHashIndex
}

#[inline(always)]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n.wrapping_add(a).wrapping_sub(1)) & !(a.wrapping_sub(1))
}

#[inline(always)]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.wrapping_shl(n) | val.wrapping_shr(STBDS_SIZE_T_BITS as u32 - n)
}

#[inline(always)]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.wrapping_shr(n) | val.wrapping_shl(STBDS_SIZE_T_BITS as u32 - n)
}

fn stbds_load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp: usize = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ v32;
    var
}

#[inline(always)]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

#[inline(always)]
unsafe fn stbds_sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = stbds_rotate_left(*v0, (STBDS_SIZE_T_BITS / 2) as u32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = stbds_rotate_left(*v2, (STBDS_SIZE_T_BITS / 2) as u32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;
    let mut i: usize;
    let mut j: usize;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((0x736f6d65_usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261_usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765_usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462_usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;

    i = 0;
    while i.wrapping_add(core::mem::size_of::<usize>()) <= len {
        data = *d.add(0) as usize
            | (*d.add(1) as usize) << 8
            | (*d.add(2) as usize) << 16
            | (*d.add(3) as usize) << 24;
        data |= ((*d.add(4) as usize)
            | ((*d.add(5) as usize) << 8)
            | ((*d.add(6) as usize) << 16)
            | ((*d.add(7) as usize) << 24))
            << 16
            << 16;

        v3 ^= data;
        j = 0;
        while j < STBDS_SIPHASH_C_ROUNDS {
            stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            j += 1;
        }
        v0 ^= data;

        i += core::mem::size_of::<usize>();
        d = d.add(core::mem::size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let remain = len.wrapping_sub(i);
    // C fallthrough switch
    if remain >= 7 {
        data |= (*d.add(6) as usize) << 24 << 24;
    }
    if remain >= 6 {
        data |= (*d.add(5) as usize) << 20 << 20;
    }
    if remain >= 5 {
        data |= (*d.add(4) as usize) << 16 << 16;
    }
    if remain >= 4 {
        data |= (*d.add(3) as usize) << 24;
    }
    if remain >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if remain >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if remain >= 1 {
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    j = 0;
    while j < STBDS_SIPHASH_C_ROUNDS {
        stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    j = 0;
    while j < STBDS_SIPHASH_D_ROUNDS {
        stbds_sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut s = str as *mut u8;
    while *s != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);
        s = s.add(1);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds_hash_seed = seed;
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut StbdsHashIndex,
) -> *mut StbdsHashIndex {
    let t: *mut StbdsHashIndex = realloc(
        core::ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            * core::mem::size_of::<StbdsHashBucket>()
            + core::mem::size_of::<StbdsHashIndex>()
            + STBDS_CACHE_LINE_SIZE
            - 1,
    ) as *mut StbdsHashIndex;

    (*t).storage = stbds_align_fwd(
        t.add(1) as usize,
        STBDS_CACHE_LINE_SIZE,
    ) as *mut StbdsHashBucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }
    assert!(
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
    );

    if !ot.is_null() {
        (*t).string = core::ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        memset(
            &mut (*t).string as *mut StbdsStringArena as *mut c_void,
            0,
            core::mem::size_of::<StbdsStringArena>(),
        );
        (*t).seed = stbds_hash_seed;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let bucket = &mut *(*t).storage.add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                bucket.hash[j] = STBDS_HASH_EMPTY;
                j += 1;
            }
            j = 0;
            while j < STBDS_BUCKET_LENGTH {
                bucket.index[j] = STBDS_INDEX_EMPTY;
                j += 1;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        let mut i: usize = 0;
        (*t).used_count = (*ot).used_count;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob = &*(*ot).storage.add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                if stbds_index_in_use(ob.index[j]) {
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

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }

    t
}

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    if mode >= STBDS_HM_STRING {
        let str_ptr = *((a as *mut u8).offset(elemsize as isize * i + keyoffset as isize)
            as *mut *mut c_char);
        0 == strcmp(key as *const c_char, str_ptr as *const c_char)
    } else {
        0 == memcmp(
            key as *const c_void,
            (a as *mut u8).offset(elemsize as isize * i + keyoffset as isize) as *const c_void,
            keysize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut c_void {
    let mut min_cap = min_cap;
    let arr_len: isize = if !a.is_null() {
        (*stbds_header(a)).length as isize
    } else {
        0
    };
    let arr_cap: usize = if !a.is_null() {
        (*stbds_header(a)).capacity
    } else {
        0
    };
    let min_len = (arr_len as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= arr_cap {
        return a;
    }

    if min_cap < 2_usize.wrapping_mul(arr_cap) {
        min_cap = 2_usize.wrapping_mul(arr_cap);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let b = realloc(
        if !a.is_null() {
            stbds_header(a) as *mut c_void
        } else {
            core::ptr::null_mut()
        },
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(core::mem::size_of::<StbdsArrayHeader>()),
    );
    let b = (b as *mut u8).add(core::mem::size_of::<StbdsArrayHeader>()) as *mut c_void;
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = core::ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                free(
                    *((a as *mut u8).add(elemsize.wrapping_mul(i)) as *mut *mut c_void)
                        as *mut c_void,
                );
                i += 1;
            }
        }
        stbds_strreset(&mut (*stbds_hash_table(a)).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos: usize;

    if hash < 2 {
        hash += 2;
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

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

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count.wrapping_sub(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    if a.is_null() {
        let a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return stbds_arr_to_hash(a, elemsize);
    } else {
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut StbdsHashIndex;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*stbds_header(stbds_hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null()
        || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0
    {
        let arr = stbds_arrgrowf(
            if !a.is_null() {
                stbds_hash_to_arr(a, elemsize)
            } else {
                core::ptr::null_mut()
            },
            elemsize,
            0,
            1,
        );
        (*stbds_header(arr)).length += 1;
        memset(arr, 0, elemsize);
        return stbds_arr_to_hash(arr, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str: *mut c_char) -> *mut c_char {
    let len = strlen(str as *const c_char).wrapping_add(1);
    let p = realloc(core::ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut raw_a: *mut c_void;
    let mut table: *mut StbdsHashIndex;
    let mut a = a;

    if a.is_null() {
        a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = stbds_arr_to_hash(a, elemsize);
    }

    raw_a = a;
    a = stbds_hash_to_arr(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut StbdsHashIndex;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        (*stbds_header(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut pos: usize;
        let mut tombstone: isize = -1;

        if hash < 2 {
            hash += 2;
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        let found_pos: usize;
        'search: loop {
            let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let mut i = pos & STBDS_BUCKET_MASK;
            while i < STBDS_BUCKET_LENGTH {
                if bucket.hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i],
                    ) {
                        (*stbds_header(a)).temp = bucket.index[i];
                        if mode >= STBDS_HM_STRING {
                            let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut c_char;
                            *temp_key_ptr = *((raw_a as *mut u8)
                                .offset(elemsize as isize * bucket.index[i] + keyoffset as isize)
                                as *mut *mut c_char);
                        }
                        return stbds_arr_to_hash(a, elemsize);
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
                    if stbds_is_key_equal(
                        raw_a, elemsize, key, keysize, keyoffset, mode, bucket.index[i],
                    ) {
                        (*stbds_header(a)).temp = bucket.index[i];
                        return stbds_arr_to_hash(a, elemsize);
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

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count.wrapping_sub(1);
        }

        pos = found_pos;
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
        }
        (*table).used_count = (*table).used_count.wrapping_add(1);

        {
            let i: isize = if !a.is_null() {
                (*stbds_header(a)).length as isize
            } else {
                0
            };
            if (i as usize).wrapping_add(1) > if !a.is_null() {
                (*stbds_header(a)).capacity
            } else {
                0
            } {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = stbds_arr_to_hash(a, elemsize);

            assert!((i as usize).wrapping_add(1) <= (*stbds_header(a)).capacity);
            (*stbds_header(a)).length = (i + 1) as usize;
            let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            bucket.hash[pos & STBDS_BUCKET_MASK] = hash;
            bucket.index[pos & STBDS_BUCKET_MASK] = i - 1;
            (*stbds_header(a)).temp = i - 1;

            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    let dest =
                        (a as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char;
                    *dest = stbds_strdup(key as *mut c_char);
                    let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut c_char;
                    *temp_key_ptr = *dest;
                }
                STBDS_SH_ARENA => {
                    let dest =
                        (a as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char;
                    *dest = stbds_stralloc(
                        &mut (*table).string,
                        key as *mut c_char,
                    );
                    let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut c_char;
                    *temp_key_ptr = *dest;
                }
                STBDS_SH_DEFAULT => {
                    let dest =
                        (a as *mut u8).offset(elemsize as isize * i) as *mut *mut c_char;
                    *dest = key as *mut c_char;
                    let temp_key_ptr = (*stbds_header(a)).hash_table as *mut *mut c_char;
                    *temp_key_ptr = *dest;
                }
                _ => {
                    memmove(
                        (a as *mut u8).offset(elemsize as isize * i) as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        stbds_arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, core::ptr::null_mut());
    (*h).string.mode = mode as u8;
    (*stbds_header(a)).hash_table = h as *mut c_void;
    stbds_arr_to_hash(a, elemsize)
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
        return core::ptr::null_mut();
    }

    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut StbdsHashIndex;
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
    let final_index = ((*stbds_header(raw_a)).length as isize) - 1 - 1;

    assert!((slot as usize) < (*table).slot_count);
    (*table).used_count = (*table).used_count.wrapping_sub(1);
    (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
    (*stbds_header(raw_a)).temp = 1;
    assert!((*table).used_count as isize >= 0);
    b.hash[i] = STBDS_HASH_DELETED;
    b.index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        free(
            *((a as *mut u8).offset(elemsize as isize * old_index) as *mut *mut c_void)
                as *mut c_void,
        );
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).offset(elemsize as isize * old_index) as *mut c_void,
            (a as *mut u8).offset(elemsize as isize * final_index) as *const c_void,
            elemsize,
        );

        let slot2: isize;
        if mode == STBDS_HM_STRING {
            slot2 = stbds_hm_find_slot(
                a,
                elemsize,
                *((a as *mut u8).offset(elemsize as isize * old_index + keyoffset as isize)
                    as *mut *mut c_void),
                keysize,
                keyoffset,
                mode,
            );
        } else {
            slot2 = stbds_hm_find_slot(
                a,
                elemsize,
                (a as *mut u8).offset(elemsize as isize * old_index + keyoffset as isize)
                    as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        }
        assert!(slot2 >= 0);
        let b2 = &mut *(*table).storage.add((slot2 as usize) >> STBDS_BUCKET_SHIFT);
        let i2 = (slot2 as usize) & STBDS_BUCKET_MASK;
        assert!(b2.index[i2] == final_index);
        b2.index[i2] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut StbdsStringArena, str: *mut c_char) -> *mut c_char {
    let len = strlen(str as *const c_char).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize_exp = (*a).block as usize;
        let mut blocksize: usize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_exp >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                core::ptr::null_mut(),
                core::mem::size_of::<StbdsStringBlock>() - 8 + len,
            ) as *mut StbdsStringBlock;
            memmove(
                (*sb).storage.as_mut_ptr() as *mut c_void,
                str as *const c_void,
                len,
            );
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = core::ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let sb = realloc(
                core::ptr::null_mut(),
                core::mem::size_of::<StbdsStringBlock>() - 8 + blocksize,
            ) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    assert!(len <= (*a).remaining);
    let p = (*(*a).storage)
        .storage
        .as_mut_ptr()
        .offset((*a).remaining as isize - len as isize);
    (*a).remaining -= len;
    memmove(p as *mut c_void, str as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<StbdsStringArena>(),
    );
}

unsafe fn strkey(n: c_int) -> *mut c_char {
    sprintf(
        buffer.as_mut_ptr(),
        b"test_%d\0".as_ptr() as *const c_char,
        n,
    );
    buffer.as_mut_ptr()
}

#[repr(C)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let mut strmap: *mut StrMapEntry = core::ptr::null_mut();
    let mut sa: StbdsStringArena = core::mem::zeroed();
    let mut i: c_int;

    i = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    {
        let s_key = b"a\0".as_ptr() as *mut c_char;
        let s_value = num;

        // shputs(strmap, s): hmput_key_wrapper then assign struct then fixup key
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            core::mem::size_of::<StrMapEntry>(),
            &s_key as *const *mut c_char as *mut c_void,
            core::mem::size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut StrMapEntry;
        let temp_idx = (*stbds_header(
            (strmap as *mut u8).sub(core::mem::size_of::<StrMapEntry>()) as *mut c_void,
        ))
        .temp;
        (*strmap.offset(temp_idx)).key = s_key;
        (*strmap.offset(temp_idx)).value = s_value;
        // fixup key from temp_key
        let temp_key = *((*stbds_header(
            (strmap as *mut u8).sub(core::mem::size_of::<StrMapEntry>()) as *mut c_void,
        ))
        .hash_table as *mut *mut c_char);
        (*strmap.offset(temp_idx)).key = temp_key;

        assert!(*(*strmap.offset(0)).key == b'a' as c_char);
        assert!((*strmap.offset(0)).key == s_key);
        assert!((*strmap.offset(0)).value == s_value);

        // shlen(strmap) = header(strmap-1)->length - 1
        let raw_a = (strmap as *mut u8).sub(core::mem::size_of::<StrMapEntry>()) as *mut c_void;
        let len = (*stbds_header(raw_a)).length as isize - 1;

        let mut z: isize = 0;
        while z < len {
            let entry = &*strmap.offset(z);
            printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                entry.key,
                entry.value,
            );
            z += 1;
        }

        // shfree(strmap): hmfree_func(strmap-1, sizeof*strmap), strmap=NULL
        stbds_hmfree_func(
            (strmap as *mut u8).sub(core::mem::size_of::<StrMapEntry>()) as *mut c_void,
            core::mem::size_of::<StrMapEntry>(),
        );
        strmap = core::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_unit_tests() {}
