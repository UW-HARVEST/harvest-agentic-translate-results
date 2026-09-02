//! Hash-map back-end and hash functions of `stb_ds`.

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::arr::stbds_arrgrowf;
use crate::c;
use crate::strings::{stbds_stralloc, stbds_strdup, stbds_strreset};
use crate::types::*;

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

/// `void stbds_rand_seed(size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    *STBDS_HASH_SEED.get() = seed;
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
fn load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    // temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16
    let mut temp: usize = (v64_lo ^ v32) as usize;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    // var = v64_hi, var <<= 16, var <<= 16
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    // var ^= temp ^ v32
    var ^= temp ^ (v32 as usize);
    var
}

// ---------------------------------------------------------------------------
// Probing helpers
// ---------------------------------------------------------------------------

/// `static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)`
#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & slot_count.wrapping_sub(1)
}

/// `static size_t stbds_log2(size_t slot_count)`
fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Hash index construction / rehash
// ---------------------------------------------------------------------------

/// `static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)`
pub unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut HashIndex) -> *mut HashIndex {
    let t = c::realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(size_of::<HashBucket>())
            .wrapping_add(size_of::<HashIndex>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),
    ) as *mut HashIndex;

    (*t).storage =
        stbds_align_fwd(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut HashBucket;
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
    stbds_assert!(
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count,
        "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count",
        401,
        "stbds_make_hash_index"
    );

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        c::memset(
            (&raw mut (*t).string) as *mut c_void,
            0,
            size_of::<StringArena>(),
        );
        (*t).seed = *STBDS_HASH_SEED.get();
        let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = load_32_or_64(715136305, 0, 0xb504f32d);
        *STBDS_HASH_SEED.get() = (*STBDS_HASH_SEED.get()).wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b = (*t).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let mut i: usize = 0;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob = (*ot).storage.wrapping_add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'done: loop {
                        let bucket = (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z = 0usize;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'done;
                            }
                            z += 1;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count.wrapping_sub(1);
                    }
                }
            }
            i += 1;
        }
    }

    t
}

// ---------------------------------------------------------------------------
// Hash functions
// ---------------------------------------------------------------------------

/// `size_t stbds_hash_string(char *str, size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut s = str_ as *const u8;
    while *s != 0 {
        hash = rotl(hash, 9).wrapping_add(*s as usize);
        s = s.wrapping_add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotr(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotr(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

macro_rules! siphash_round {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotl($v1, 13);
        $v1 ^= $v0;
        $v0 = rotl($v0, STBDS_SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotl($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotl($v1, 17);
        $v1 ^= $v2;
        $v2 = rotl($v2, STBDS_SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotl($v3, 21);
        $v3 ^= $v0;
    }};
}

/// `static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)`
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut data: usize;

    let mut v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i: usize = 0;
    while i + size_of::<usize>() <= len {
        // `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);`
        // The right-hand side has type `int`, so the result is *sign extended*
        // when stored into the `size_t`.  This is reproduced verbatim.
        let lo = (*d.wrapping_add(0) as i32)
            | ((*d.wrapping_add(1) as i32) << 8)
            | ((*d.wrapping_add(2) as i32) << 16)
            | ((*d.wrapping_add(3) as i32) << 24);
        data = sx(lo);
        let hi = (*d.wrapping_add(4) as i32)
            | ((*d.wrapping_add(5) as i32) << 8)
            | ((*d.wrapping_add(6) as i32) << 16)
            | ((*d.wrapping_add(7) as i32) << 24);
        data |= (sx(hi) << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round!(v0, v1, v2, v3);
        }
        v0 ^= data;

        i += size_of::<usize>();
        d = d.wrapping_add(size_of::<usize>());
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // `switch (len - i)` with deliberate fall-through from 7 down to 0.
    let rem = len.wrapping_sub(i);
    if rem >= 7 {
        data |= ((*d.wrapping_add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.wrapping_add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.wrapping_add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        data |= sx((*d.wrapping_add(3) as i32) << 24);
    }
    if rem >= 3 {
        data |= sx((*d.wrapping_add(2) as i32) << 16);
    }
    if rem >= 2 {
        data |= sx((*d.wrapping_add(1) as i32) << 8);
    }
    if rem >= 1 {
        data |= sx(*d.wrapping_add(0) as i32);
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siphash_round!(v0, v1, v2, v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siphash_round!(v0, v1, v2, v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// Key comparison
// ---------------------------------------------------------------------------

/// `static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode, size_t i)`
unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> c_int {
    if mode >= STBDS_HM_STRING {
        let stored = elem_at(a, elemsize, i, keyoffset) as *mut *mut c_char;
        (0 == c::strcmp(key as *const c_char, *stored)) as c_int
    } else {
        (0 == c::memcmp(
            key as *const c_void,
            elem_at(a, elemsize, i, keyoffset) as *const c_void,
            keysize,
        )) as c_int
    }
}

// ---------------------------------------------------------------------------
// Teardown
// ---------------------------------------------------------------------------

/// `void stbds_hmfree_func(void *a, size_t elemsize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                let slot = elem_at(a, elemsize, i, 0) as *mut *mut c_char;
                c::free(*slot as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
    }
    c::free((*stbds_header(a)).hash_table);
    c::free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

/// `static ptrdiff_t stbds_hm_find_slot(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)`
unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash += 2;
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

        let mut i = pos & STBDS_BUCKET_MASK;
        while i < STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as usize,
                ) != 0
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        let limit = pos & STBDS_BUCKET_MASK;
        let mut i = 0usize;
        while i < limit {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i] as usize,
                ) != 0
                {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count.wrapping_sub(1);
    }
}

/// `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)`
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
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        c::memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut HashIndex;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table)
                    .storage
                    .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

/// `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)`
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
    *stbds_temp_ptr(hash_to_arr(p, elemsize)) = temp;
    p
}

// ---------------------------------------------------------------------------
// Insertion
// ---------------------------------------------------------------------------

/// `void *stbds_hmput_default(void *a, size_t elemsize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        a = stbds_arrgrowf(
            if !a.is_null() {
                hash_to_arr(a, elemsize)
            } else {
                ptr::null_mut()
            },
            elemsize,
            0,
            1,
        );
        (*stbds_header(a)).length += 1;
        c::memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

/// `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;
    let mut a = a;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        c::memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a)).hash_table as *mut HashIndex;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            c::free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT as u8
            } else {
                0
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = table as *mut c_void;
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
        let mut bucket: *mut HashBucket;

        if hash < 2 {
            hash += 2;
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'found_empty_slot: loop {
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

            let mut i = pos & STBDS_BUCKET_MASK;
            while i < STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        *stbds_temp_ptr(a) = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            *stbds_temp_key_ptr(a) = *(elem_at(
                                raw_a,
                                elemsize,
                                (*bucket).index[i] as usize,
                                keyoffset,
                            ) as *mut *mut c_char);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
                i += 1;
            }

            let limit = pos & STBDS_BUCKET_MASK;
            let mut i = 0usize;
            while i < limit {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) != 0
                    {
                        *stbds_temp_ptr(a) = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
                i += 1;
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count.wrapping_sub(1);
        }

        // found_empty_slot:
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        {
            let i: isize = stbds_arrlen(a);
            if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);

            stbds_assert!(
                (i as usize).wrapping_add(1) <= stbds_arrcap(a),
                "(size_t) i+1 <= stbds_arrcap(a)",
                778,
                "stbds_hmput_key"
            );
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            *stbds_temp_ptr(a) = i - 1;

            let dst = elem_at(a, elemsize, i as usize, 0);
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let p = stbds_strdup(key as *mut c_char);
                    *(dst as *mut *mut c_char) = p;
                    *stbds_temp_key_ptr(a) = p;
                }
                STBDS_SH_ARENA => {
                    let p = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                    *(dst as *mut *mut c_char) = p;
                    *stbds_temp_key_ptr(a) = p;
                }
                STBDS_SH_DEFAULT => {
                    let p = key as *mut c_char;
                    *(dst as *mut *mut c_char) = p;
                    *stbds_temp_key_ptr(a) = p;
                }
                _ => {
                    c::memcpy(dst as *mut c_void, key as *const c_void, keysize);
                }
            }
        }
        let _ = raw_a;
        arr_to_hash(a, elemsize)
    }
}

/// `void *stbds_shmode_func(size_t elemsize, int mode)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    c::memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

// ---------------------------------------------------------------------------
// Deletion
// ---------------------------------------------------------------------------

/// `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)`
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
    let table = (*stbds_header(raw_a)).hash_table as *mut HashIndex;
    *stbds_temp_ptr(raw_a) = 0;
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table)
        .storage
        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
    let old_index = (*b).index[i as usize];
    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
    stbds_assert!(
        slot < (*table).slot_count as isize,
        "slot < (ptrdiff_t) table->slot_count",
        828,
        "stbds_hmdel_key"
    );
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp_ptr(raw_a) = 1;
    // `STBDS_ASSERT(table->used_count >= 0);` — always true for size_t.
    stbds_assert!(
        true,
        "table->used_count >= 0",
        832,
        "stbds_hmdel_key"
    );
    (*b).hash[i as usize] = STBDS_HASH_DELETED;
    (*b).index[i as usize] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
        let p = elem_at(a, elemsize, old_index as usize, 0) as *mut *mut c_char;
        c::free(*p as *mut c_void);
    }

    if old_index != final_index {
        c::memmove(
            elem_at(a, elemsize, old_index as usize, 0) as *mut c_void,
            elem_at(a, elemsize, final_index as usize, 0) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let stored =
                *(elem_at(a, elemsize, old_index as usize, keyoffset) as *mut *mut c_char);
            slot = stbds_hm_find_slot(a, elemsize, stored as *mut c_void, keysize, keyoffset, mode);
        } else {
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                elem_at(a, elemsize, old_index as usize, keyoffset) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        }
        stbds_assert!(slot >= 0, "slot >= 0", 846, "stbds_hmdel_key");
        b = (*table)
            .storage
            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        stbds_assert!(
            (*b).index[i as usize] == final_index,
            "b->index[i] == final_index",
            849,
            "stbds_hmdel_key"
        );
        (*b).index[i as usize] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        c::free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        c::free(table as *mut c_void);
    }

    a
}
