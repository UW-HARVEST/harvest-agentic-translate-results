//! Hash map implementation: `stbds_hmfree_func`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmput_key`,
//! `stbds_hmdel_key`, `stbds_shmode_func`.

use core::ffi::{c_char, c_int, c_void};

use crate::arena::{stbds_stralloc, stbds_strdup, stbds_strreset};
use crate::array::stbds_arrgrowf;
use crate::ffi::*;
use crate::hash::{stbds_hash_bytes, stbds_hash_string, stbds_make_hash_index, stbds_probe_position};

/// ```c
/// static int stbds_is_key_equal(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode, size_t i)
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
        (0 == strcmp(key as *const c_char, *slot)) as c_int
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
pub extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        if !stbds_hash_table(a).is_null() {
            if (*stbds_hash_table(a)).string.mode as c_int == STBDS_SH_STRDUP {
                let mut i: usize = 1;
                while i < (*stbds_header(a)).length {
                    let p = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i)) as *mut *mut c_char;
                    free(*p as *mut c_void);
                    i += 1;
                }
            }
            stbds_strreset(core::ptr::addr_of_mut!((*stbds_hash_table(a)).string));
        }
        free((*stbds_header(a)).hash_table);
        free(stbds_header(a) as *mut c_void);
    }
}

/// ```c
/// static ptrdiff_t stbds_hm_find_slot(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)
/// ```
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
    let mut hash: usize = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: usize = STBDS_BUCKET_LENGTH;
    let mut pos: usize;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket: *mut StbdsHashBucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);

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
        pos &= (*table).slot_count - 1;
    }
}

/// ```c
/// void * stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset: usize = 0;
        if a.is_null() {
            let a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
            (*stbds_header(a)).length += 1;
            core::ptr::write_bytes(a as *mut u8, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            stbds_arr_to_hash(a, elemsize)
        } else {
            let table: *mut StbdsHashIndex;
            let raw_a = stbds_hash_to_arr(a, elemsize);
            table = (*stbds_header(raw_a)).hash_table as *mut StbdsHashIndex;
            if table.is_null() {
                *temp = -1;
            } else {
                let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    *temp = STBDS_INDEX_EMPTY;
                } else {
                    let b: *mut StbdsHashBucket =
                        (*table).storage.wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
                }
            }
            a
        }
    }
}

/// ```c
/// void * stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let mut temp: isize = 0;
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        stbds_temp_set(stbds_hash_to_arr(p, elemsize), temp);
        p
    }
}

/// ```c
/// void * stbds_hmput_default(void *a, size_t elemsize)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        let mut a = a;
        if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
            a = stbds_arrgrowf(
                if !a.is_null() {
                    stbds_hash_to_arr(a, elemsize)
                } else {
                    core::ptr::null_mut()
                },
                elemsize,
                0,
                1,
            );
            (*stbds_header(a)).length += 1;
            core::ptr::write_bytes(a as *mut u8, 0, elemsize);
            a = stbds_arr_to_hash(a, elemsize);
        }
        a
    }
}

/// ```c
/// void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset: usize = 0;
        let mut a = a;
        let mut raw_a: *mut c_void;
        let mut table: *mut StbdsHashIndex;

        if a.is_null() {
            a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
            core::ptr::write_bytes(a as *mut u8, 0, elemsize);
            (*stbds_header(a)).length += 1;
            a = stbds_arr_to_hash(a, elemsize);
        }

        raw_a = a;
        a = stbds_hash_to_arr(a, elemsize);

        table = (*stbds_header(a)).hash_table as *mut StbdsHashIndex;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let nt: *mut StbdsHashIndex;
            let slot_count: usize;

            slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count * 2
            };
            nt = stbds_make_hash_index(slot_count, table);
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
            (*stbds_header(a)).hash_table = table as *mut c_void;
        }

        {
            let mut hash: usize = if mode >= STBDS_HM_STRING {
                stbds_hash_string(key as *mut c_char, (*table).seed)
            } else {
                stbds_hash_bytes(key, keysize, (*table).seed)
            };
            let mut step: usize = STBDS_BUCKET_LENGTH;
            let mut pos: usize;
            let mut tombstone: isize = -1;
            let mut bucket: *mut StbdsHashBucket;

            if hash < 2 {
                hash = hash.wrapping_add(2);
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
                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let src = (raw_a as *mut u8)
                                    .wrapping_add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                    .wrapping_add(keyoffset) as *mut *mut c_char;
                                stbds_temp_key_set(a, *src);
                            }
                            return stbds_arr_to_hash(a, elemsize);
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
                let mut empty_slot = false;
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
                            stbds_temp_set(a, (*bucket).index[i]);
                            return stbds_arr_to_hash(a, elemsize);
                        }
                    } else if (*bucket).hash[i] == 0 {
                        pos = (pos & !STBDS_BUCKET_MASK) + i;
                        empty_slot = true;
                        break;
                    } else if tombstone < 0 {
                        if (*bucket).index[i] == STBDS_INDEX_DELETED {
                            tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                        }
                    }
                    i += 1;
                }
                if empty_slot {
                    break 'found_empty_slot;
                }

                pos = pos.wrapping_add(step);
                step += STBDS_BUCKET_LENGTH;
                pos &= (*table).slot_count - 1;
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
                raw_a = stbds_arr_to_hash(a, elemsize);
                let _ = raw_a;

                stbds_assert!((i as usize) + 1 <= stbds_arrcap(a));
                (*stbds_header(a)).length = (i + 1) as usize;
                bucket = (*table).storage.wrapping_add(pos >> STBDS_BUCKET_SHIFT);
                (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
                (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
                stbds_temp_set(a, i - 1);

                let dst = (a as *mut u8).wrapping_add(elemsize.wrapping_mul(i as usize));
                match (*table).string.mode as c_int {
                    STBDS_SH_STRDUP => {
                        let v = stbds_strdup(key as *mut c_char);
                        *(dst as *mut *mut c_char) = v;
                        stbds_temp_key_set(a, v);
                    }
                    STBDS_SH_ARENA => {
                        let v = stbds_stralloc(
                            core::ptr::addr_of_mut!((*table).string),
                            key as *mut c_char,
                        );
                        *(dst as *mut *mut c_char) = v;
                        stbds_temp_key_set(a, v);
                    }
                    STBDS_SH_DEFAULT => {
                        let v = key as *mut c_char;
                        *(dst as *mut *mut c_char) = v;
                        stbds_temp_key_set(a, v);
                    }
                    _ => {
                        memmove_key(dst, key, keysize);
                    }
                }
            }
            stbds_arr_to_hash(a, elemsize)
        }
    }
}

/// `memcpy((char *) a + elemsize*i, key, keysize);`
#[inline]
unsafe fn memmove_key(dst: *mut u8, key: *mut c_void, keysize: usize) {
    memcpy(dst as *mut c_void, key as *const c_void, keysize);
}

/// ```c
/// void * stbds_shmode_func(size_t elemsize, int mode)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
        let h: *mut StbdsHashIndex;
        core::ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length = 1;
        h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, core::ptr::null_mut());
        (*stbds_header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        stbds_arr_to_hash(a, elemsize)
    }
}

/// ```c
/// void * stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            core::ptr::null_mut()
        } else {
            let table: *mut StbdsHashIndex;
            let raw_a = stbds_hash_to_arr(a, elemsize);
            table = (*stbds_header(raw_a)).hash_table as *mut StbdsHashIndex;
            stbds_temp_set(raw_a, 0);
            if table.is_null() {
                a
            } else {
                let mut slot: isize;
                slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    a
                } else {
                    let mut b: *mut StbdsHashBucket =
                        (*table).storage.wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
                    let mut i: usize = (slot as usize) & STBDS_BUCKET_MASK;
                    let old_index: isize = (*b).index[i];
                    let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;
                    stbds_assert!(slot < (*table).slot_count as isize);
                    (*table).used_count -= 1;
                    (*table).tombstone_count += 1;
                    stbds_temp_set(raw_a, 1);
                    stbds_assert!((*table).used_count as isize >= 0);
                    (*b).hash[i] = STBDS_HASH_DELETED;
                    (*b).index[i] = STBDS_INDEX_DELETED;

                    if mode == STBDS_HM_STRING && (*table).string.mode as c_int == STBDS_SH_STRDUP {
                        let p = (a as *mut u8)
                            .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                            as *mut *mut c_char;
                        free(*p as *mut c_void);
                    }

                    if old_index != final_index {
                        memmove(
                            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                as *mut c_void,
                            (a as *mut u8).wrapping_add(elemsize.wrapping_mul(final_index as usize))
                                as *const c_void,
                            elemsize,
                        );

                        if mode == STBDS_HM_STRING {
                            let kp = (a as *mut u8)
                                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                .wrapping_add(keyoffset) as *mut *mut c_char;
                            slot = stbds_hm_find_slot(
                                a,
                                elemsize,
                                *kp as *mut c_void,
                                keysize,
                                keyoffset,
                                mode,
                            );
                        } else {
                            let kp = (a as *mut u8)
                                .wrapping_add(elemsize.wrapping_mul(old_index as usize))
                                .wrapping_add(keyoffset) as *mut c_void;
                            slot = stbds_hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode);
                        }
                        stbds_assert!(slot >= 0);
                        b = (*table).storage.wrapping_offset(slot >> STBDS_BUCKET_SHIFT);
                        i = (slot as usize) & STBDS_BUCKET_MASK;
                        stbds_assert!((*b).index[i] == final_index);
                        (*b).index[i] = old_index;
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
            }
        }
    }
}
