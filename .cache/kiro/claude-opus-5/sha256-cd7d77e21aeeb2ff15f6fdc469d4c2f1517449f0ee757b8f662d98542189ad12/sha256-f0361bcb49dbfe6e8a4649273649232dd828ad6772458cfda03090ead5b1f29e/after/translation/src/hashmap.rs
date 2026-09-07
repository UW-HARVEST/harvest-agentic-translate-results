//! Hash-map core: index construction, slot probing, and the
//! `stbds_hm*` / `stbds_shmode_func` public entry points.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::arr::stbds_arrgrowf;
use crate::hash::{stbds_hash_bytes, stbds_hash_string, stbds_hash_seed};
use crate::strings::{stbds_stralloc, stbds_strdup, stbds_strreset};
use crate::{
    align_fwd, arr_to_hash, arrcap, arrlen, free, hash_table, hash_to_arr, header, memcmp, realloc,
    stbds_assert, stbds_hash_bucket, stbds_hash_index, stbds_string_arena, strcmp, temp_key_set,
    temp_set, STBDS_BUCKET_LENGTH, STBDS_BUCKET_MASK, STBDS_BUCKET_SHIFT, STBDS_CACHE_LINE_SIZE,
    STBDS_HASH_DELETED, STBDS_HASH_EMPTY, STBDS_HM_STRING, STBDS_INDEX_DELETED, STBDS_INDEX_EMPTY,
    STBDS_SH_ARENA, STBDS_SH_DEFAULT, STBDS_SH_STRDUP,
};

/// `STBDS_INDEX_IN_USE(x)`
#[inline(always)]
fn index_in_use(x: isize) -> bool {
    x >= 0
}

/// ```c
/// static size_t stbds_probe_position(size_t hash, size_t slot_count, size_t slot_log2)
/// ```
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count.wrapping_sub(1))
}

/// ```c
/// static size_t stbds_log2(size_t slot_count)
/// ```
fn stbds_log2(slot_count: usize) -> usize {
    let mut slot_count = slot_count;
    let mut n: usize = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)`
#[inline]
fn load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ v32;
    var
}

/// ```c
/// static stbds_hash_index *stbds_make_hash_index(size_t slot_count, stbds_hash_index *ot)
/// ```
pub(crate) unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let t: *mut stbds_hash_index = realloc(
        ptr::null_mut(),
        (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
            + core::mem::size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1,
    ) as *mut stbds_hash_index;

    (*t).storage =
        align_fwd(t.wrapping_add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
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
        (*t).used_count_threshold + (*t).tombstone_count_threshold < (*t).slot_count
    );

    if !ot.is_null() {
        // t->string = ot->string;
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*ot).string),
            ptr::addr_of_mut!((*t).string),
            1,
        );
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(
            ptr::addr_of_mut!((*t).string) as *mut u8,
            0,
            core::mem::size_of::<stbds_string_arena>(),
        );
        (*t).seed = stbds_hash_seed;
        let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = load_32_or_64(715136305, 0, 0xb504f32d);
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }

    {
        let mut i: usize = 0;
        while i < slot_count >> STBDS_BUCKET_SHIFT {
            let b: *mut stbds_hash_bucket = (*t).storage.wrapping_add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
                j += 1;
            }
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                (*b).index[j] = STBDS_INDEX_EMPTY;
                j += 1;
            }
            i += 1;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let mut i: usize = 0;
        while i < (*ot).slot_count >> STBDS_BUCKET_SHIFT {
            let ob: *mut stbds_hash_bucket = (*ot).storage.wrapping_add(i);
            let mut j: usize = 0;
            while j < STBDS_BUCKET_LENGTH {
                if index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'probe: loop {
                        let bucket: *mut stbds_hash_bucket =
                            (*t).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

                        let mut z = pos & STBDS_BUCKET_MASK;
                        while z < STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'probe;
                            }
                            z += 1;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        let mut z: usize = 0;
                        while z < limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'probe;
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

/// ```c
/// static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize,
///                               size_t keyoffset, int mode, size_t i)
/// ```
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
        let slot = (a as *mut u8)
            .wrapping_add(elemsize.wrapping_mul(i))
            .wrapping_add(keyoffset) as *mut *mut c_char;
        (0 == strcmp(key as *const c_char, *slot as *const c_char)) as c_int
    } else {
        let slot = (a as *mut u8)
            .wrapping_add(elemsize.wrapping_mul(i))
            .wrapping_add(keyoffset) as *const c_void;
        (0 == memcmp(key as *const c_void, slot, keysize)) as c_int
    }
}

/// ```c
/// void stbds_hmfree_func(void *a, size_t elemsize)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    if !hash_table(a).is_null() {
        if (*hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {
            let mut i: usize = 1;
            while i < (*header(a)).length {
                let p = *((a as *mut u8).wrapping_add(elemsize.wrapping_mul(i))
                    as *mut *mut c_void);
                free(p);
                i += 1;
            }
        }
        stbds_strreset(ptr::addr_of_mut!((*hash_table(a)).string));
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut c_void);
}

/// ```c
/// static ptrdiff_t stbds_hm_find_slot(void *a, size_t elemsize, void *key,
///                                     size_t keysize, size_t keyoffset, int mode)
/// ```
unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table(raw_a);
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
        let bucket: *mut stbds_hash_bucket =
            (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
        let mut i: usize = 0;
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

/// ```c
/// void * stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize,
///                           ptrdiff_t *temp, int mode)
/// ```
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
        (*header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b: *mut stbds_hash_bucket = (*table)
                    .storage
                    .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        a
    }
}

/// ```c
/// void * stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
/// ```
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
    temp_set(hash_to_arr(p, elemsize), temp);
    p
}

/// ```c
/// void * stbds_hmput_default(void *a, size_t elemsize)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        let base = if !a.is_null() {
            hash_to_arr(a, elemsize)
        } else {
            ptr::null_mut()
        };
        let b = stbds_arrgrowf(base, elemsize, 0, 1);
        (*header(b)).length += 1;
        ptr::write_bytes(b as *mut u8, 0, elemsize);
        a = arr_to_hash(b, elemsize);
    }
    a
}

/// ```c
/// void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
/// ```
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
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    raw_a = a;
    a = hash_to_arr(a, elemsize);

    table = (*header(a)).hash_table as *mut stbds_hash_index;

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
                STBDS_SH_DEFAULT as u8
            } else {
                0
            };
        }
        table = nt;
        (*header(a)).hash_table = table as *mut c_void;
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
        let mut bucket: *mut stbds_hash_bucket;

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
                        temp_set(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            let v = *((raw_a as *mut u8)
                                .wrapping_add(
                                    elemsize.wrapping_mul((*bucket).index[i] as usize),
                                )
                                .wrapping_add(keyoffset) as *mut *mut c_char);
                            temp_key_set(a, v);
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
            let mut i: usize = 0;
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
                        temp_set(a, (*bucket).index[i]);
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
            (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
        }
        (*table).used_count = (*table).used_count.wrapping_add(1);

        {
            let i: isize = arrlen(a);
            if (i as usize).wrapping_add(1) > arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = arr_to_hash(a, elemsize);

            stbds_assert!((i as usize).wrapping_add(1) <= arrcap(a));
            (*header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            temp_set(a, i - 1);

            let slot = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i as usize))
                as *mut *mut c_char;
            match (*table).string.mode as c_int {
                STBDS_SH_STRDUP => {
                    let v = stbds_strdup(key as *mut c_char);
                    *slot = v;
                    temp_key_set(a, v);
                }
                STBDS_SH_ARENA => {
                    let v = stbds_stralloc(
                        ptr::addr_of_mut!((*table).string),
                        key as *mut c_char,
                    );
                    *slot = v;
                    temp_key_set(a, v);
                }
                STBDS_SH_DEFAULT => {
                    let v = key as *mut c_char;
                    *slot = v;
                    temp_key_set(a, v);
                }
                _ => {
                    ptr::copy_nonoverlapping(
                        key as *const u8,
                        (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i as usize)),
                        keysize,
                    );
                }
            }
        }
        arr_to_hash(a, elemsize)
    }
}

/// ```c
/// void * stbds_shmode_func(size_t elemsize, int mode)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
}

/// ```c
/// void * stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize,
///                        size_t keyoffset, int mode)
/// ```
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
    let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
    temp_set(raw_a, 0);
    if table.is_null() {
        return a;
    }

    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b: *mut stbds_hash_bucket = (*table)
        .storage
        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
    let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
    let old_index: isize = (*b).index[i as usize];
    let final_index: isize = arrlen(raw_a) - 1 - 1;
    stbds_assert!(slot < (*table).slot_count as isize);
    (*table).used_count = (*table).used_count.wrapping_sub(1);
    (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
    temp_set(raw_a, 1);
    // STBDS_ASSERT(table->used_count >= 0); -- always true for size_t
    (*b).hash[i as usize] = STBDS_HASH_DELETED;
    (*b).index[i as usize] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
        let p = *((a as *mut u8).wrapping_add(elemsize.wrapping_mul(old_index as usize))
            as *mut *mut c_void);
        free(p);
    }

    if old_index != final_index {
        ptr::copy(
            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(final_index as usize)),
            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(old_index as usize)),
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let moved_key = *((a as *mut u8)
                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                .wrapping_add(keyoffset) as *mut *mut c_char);
            slot = stbds_hm_find_slot(
                a,
                elemsize,
                moved_key as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
        } else {
            let moved_key = (a as *mut u8)
                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                .wrapping_add(keyoffset) as *mut c_void;
            slot = stbds_hm_find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
        }
        stbds_assert!(slot >= 0);
        b = (*table)
            .storage
            .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
        i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
        stbds_assert!((*b).index[i as usize] == final_index);
        (*b).index[i as usize] = old_index;
    }
    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}
