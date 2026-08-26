//! Hash-map implementation: key lookup, insertion, deletion and teardown.

use core::ffi::{c_char, c_int, c_void};
use core::ptr::null_mut;

use crate::*;

/// ```c
/// static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize,
///                              size_t keyoffset, int mode, size_t i)
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
        let other =
            *(byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *mut *mut c_char);
        (0 == strcmp(key as *const c_char, other)) as c_int
    } else {
        (0 == memcmp(
            key as *const c_void,
            byte_off(a, elemsize.wrapping_mul(i).wrapping_add(keyoffset)) as *const c_void,
            keysize,
        )) as c_int
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
    if !stbds_hash_table(a).is_null() {
        if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {
            let mut i: usize = 1;
            while i < (*stbds_header(a)).length {
                STBDS_FREE(*(byte_off(a, elemsize.wrapping_mul(i)) as *mut *mut c_char)
                    as *mut c_void);
                i += 1;
            }
        }
        stbds_strreset(&raw mut (*stbds_hash_table(a)).string);
    }
    STBDS_FREE((*stbds_header(a)).hash_table);
    STBDS_FREE(stbds_header(a) as *mut c_void);
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
    let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        // STBDS_STATS(++stbds_hash_probes);
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
                    return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
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
                    return ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
            i += 1;
        }

        pos = pos.wrapping_add(step);
        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
        pos &= (*table).slot_count.wrapping_sub(1);
    }
}

/// ```c
/// void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize,
///                          ptrdiff_t *temp, int mode)
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
        let a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        STBDS_ARR_TO_HASH(a, elemsize)
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
/// void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
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
    stbds_set_temp(STBDS_HASH_TO_ARR(p, elemsize), temp);
    p
}

/// ```c
/// void *stbds_hmput_default(void *a, size_t elemsize)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    let mut a = a;
    if a.is_null() || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0 {
        a = stbds_arrgrowf(
            if !a.is_null() {
                STBDS_HASH_TO_ARR(a, elemsize)
            } else {
                null_mut()
            },
            elemsize,
            0,
            1,
        );
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        memset(a, 0, elemsize);
        a = STBDS_ARR_TO_HASH(a, elemsize);
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
    #[allow(unused_assignments)]
    let mut raw_a: *mut c_void;
    let mut table: *mut stbds_hash_index;

    if a.is_null() {
        a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length = (*stbds_header(a)).length.wrapping_add(1);
        a = STBDS_ARR_TO_HASH(a, elemsize);
    }

    raw_a = a;
    a = STBDS_HASH_TO_ARR(a, elemsize);

    table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count.wrapping_mul(2)
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            STBDS_FREE(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                0
            };
        }
        table = nt;
        (*stbds_header(a)).hash_table = nt as *mut c_void;
        // STBDS_STATS(++stbds_hash_grow);
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
            hash = hash.wrapping_add(2);
        }

        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        'found_empty_slot: loop {
            // STBDS_STATS(++stbds_hash_probes);
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
                        stbds_set_temp(a, (*bucket).index[i]);
                        if mode >= STBDS_HM_STRING {
                            stbds_set_temp_key(
                                a,
                                *(byte_off(
                                    raw_a,
                                    elemsize
                                        .wrapping_mul((*bucket).index[i] as usize)
                                        .wrapping_add(keyoffset),
                                ) as *mut *mut c_char),
                            );
                        }
                        return STBDS_ARR_TO_HASH(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
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
                        stbds_set_temp(a, (*bucket).index[i]);
                        return STBDS_ARR_TO_HASH(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK).wrapping_add(i);
                    break 'found_empty_slot;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK).wrapping_add(i)) as isize;
                    }
                }
                i += 1;
            }

            pos = pos.wrapping_add(step);
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
            pos &= (*table).slot_count.wrapping_sub(1);
        }

        // found_empty_slot:
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
        }
        (*table).used_count = (*table).used_count.wrapping_add(1);

        {
            let i: isize = stbds_arrlen(a);
            if (i as usize).wrapping_add(1) > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = STBDS_ARR_TO_HASH(a, elemsize);
            let _ = raw_a;

            STBDS_ASSERT((i as usize).wrapping_add(1) <= stbds_arrcap(a));
            (*stbds_header(a)).length = (i + 1) as usize;
            bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            stbds_set_temp(a, i - 1);

            let slot = byte_off(a, elemsize.wrapping_mul(i as usize)) as *mut *mut c_char;
            match (*table).string.mode {
                STBDS_SH_STRDUP => {
                    *slot = stbds_strdup(key as *mut c_char);
                    stbds_set_temp_key(a, *slot);
                }
                STBDS_SH_ARENA => {
                    *slot = stbds_stralloc(&raw mut (*table).string, key as *mut c_char);
                    stbds_set_temp_key(a, *slot);
                }
                STBDS_SH_DEFAULT => {
                    *slot = key as *mut c_char;
                    stbds_set_temp_key(a, *slot);
                }
                _ => {
                    memcpy(
                        byte_off(a, elemsize.wrapping_mul(i as usize)) as *mut c_void,
                        key as *const c_void,
                        keysize,
                    );
                }
            }
        }
        STBDS_ARR_TO_HASH(a, elemsize)
    }
}

/// ```c
/// void *stbds_shmode_func(size_t elemsize, int mode)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    STBDS_ARR_TO_HASH(a, elemsize)
}

/// ```c
/// void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize,
///                       size_t keyoffset, int mode)
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
        null_mut()
    } else {
        let raw_a = STBDS_HASH_TO_ARR(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        stbds_set_temp(raw_a, 0);
        if table.is_null() {
            a
        } else {
            let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                a
            } else {
                let mut b: *mut stbds_hash_bucket = (*table)
                    .storage
                    .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                let mut i: c_int = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                let old_index = (*b).index[i as usize];
                let final_index = stbds_arrlen(raw_a) - 1 - 1;
                STBDS_ASSERT(slot < (*table).slot_count as isize);
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                stbds_set_temp(raw_a, 1);
                // STBDS_ASSERT(table->used_count >= 0);   /* always true (unsigned) */
                (*b).hash[i as usize] = STBDS_HASH_DELETED;
                (*b).index[i as usize] = STBDS_INDEX_DELETED;

                if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
                    STBDS_FREE(
                        *(byte_off(a, elemsize.wrapping_mul(old_index as usize))
                            as *mut *mut c_char) as *mut c_void,
                    );
                }

                if old_index != final_index {
                    memmove(
                        byte_off(a, elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
                        byte_off(a, elemsize.wrapping_mul(final_index as usize)) as *const c_void,
                        elemsize,
                    );

                    if mode == STBDS_HM_STRING {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            *(byte_off(
                                a,
                                elemsize
                                    .wrapping_mul(old_index as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut *mut c_char) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            byte_off(
                                a,
                                elemsize
                                    .wrapping_mul(old_index as usize)
                                    .wrapping_add(keyoffset),
                            ) as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    STBDS_ASSERT(slot >= 0);
                    b = (*table)
                        .storage
                        .wrapping_add((slot >> STBDS_BUCKET_SHIFT) as usize);
                    i = ((slot as usize) & STBDS_BUCKET_MASK) as c_int;
                    STBDS_ASSERT((*b).index[i as usize] == final_index);
                    (*b).index[i as usize] = old_index;
                }
                (*stbds_header(raw_a)).length = (*stbds_header(raw_a)).length.wrapping_sub(1);

                if (*table).used_count < (*table).used_count_shrink_threshold
                    && (*table).slot_count > STBDS_BUCKET_LENGTH
                {
                    (*stbds_header(raw_a)).hash_table =
                        stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
                    STBDS_FREE(table as *mut c_void);
                    // STBDS_STATS(++stbds_hash_shrink);
                } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
                    (*stbds_header(raw_a)).hash_table =
                        stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
                    STBDS_FREE(table as *mut c_void);
                    // STBDS_STATS(++stbds_hash_rebuild);
                }

                a
            }
        }
    }
}
