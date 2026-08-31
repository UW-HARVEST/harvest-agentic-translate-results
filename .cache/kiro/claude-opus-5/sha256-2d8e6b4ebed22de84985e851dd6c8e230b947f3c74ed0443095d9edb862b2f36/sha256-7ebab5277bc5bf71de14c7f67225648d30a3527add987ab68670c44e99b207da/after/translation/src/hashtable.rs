//! Translation of `src/hashtable.c`.

use crate::lookup3::{hashlittle, hashmask, hashsize};
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::types::*;
use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::Ordering;

const INITIAL_HASHTABLE_ORDER: usize = 3;

#[inline]
unsafe fn hash_str(key: *const c_char, len: usize) -> usize {
    hashlittle(
        key as *const u8,
        len,
        crate::hashtable_seed::hashtable_seed.load(Ordering::Relaxed),
    ) as usize
}

#[inline]
unsafe fn list_init(list: *mut HashtableList) {
    (*list).next = list;
    (*list).prev = list;
}

#[inline]
unsafe fn list_insert(list: *mut HashtableList, node: *mut HashtableList) {
    (*node).next = list;
    (*node).prev = (*list).prev;
    (*(*list).prev).next = node;
    (*list).prev = node;
}

#[inline]
unsafe fn list_remove(list: *mut HashtableList) {
    (*(*list).prev).next = (*list).next;
    (*(*list).next).prev = (*list).prev;
}

#[inline]
unsafe fn bucket_is_empty(hashtable: *mut HashtableT, bucket: *mut HashtableBucket) -> bool {
    (*bucket).first == &mut (*hashtable).list as *mut HashtableList
        && (*bucket).first == (*bucket).last
}

unsafe fn insert_to_bucket(
    hashtable: *mut HashtableT,
    bucket: *mut HashtableBucket,
    list: *mut HashtableList,
) {
    if bucket_is_empty(hashtable, bucket) {
        list_insert(&mut (*hashtable).list, list);
        (*bucket).last = list;
        (*bucket).first = list;
    } else {
        list_insert((*bucket).first, list);
        (*bucket).first = list;
    }
}

unsafe fn hashtable_find_pair(
    hashtable: *mut HashtableT,
    bucket: *mut HashtableBucket,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> *mut HashtablePair {
    let mut list: *mut HashtableList;
    let mut pair: *mut HashtablePair;

    if bucket_is_empty(hashtable, bucket) {
        return core::ptr::null_mut();
    }

    list = (*bucket).first;
    loop {
        pair = list_to_pair(list);
        if (*pair).hash == hash
            && (*pair).key_len == key_len
            && memcmp(
                (*pair).key.as_ptr() as *const c_void,
                key as *const c_void,
                key_len,
            ) == 0
        {
            return pair;
        }

        if list == (*bucket).last {
            break;
        }

        list = (*list).next;
    }

    core::ptr::null_mut()
}

/* returns 0 on success, -1 if key was not found */
unsafe fn hashtable_do_del(
    hashtable: *mut HashtableT,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> c_int {
    let pair: *mut HashtablePair;
    let bucket: *mut HashtableBucket;
    let index: usize;

    index = hash & hashmask((*hashtable).order);
    bucket = (*hashtable).buckets.add(index);

    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return -1;
    }

    let pair_list = &mut (*pair).list as *mut HashtableList;

    if pair_list == (*bucket).first && pair_list == (*bucket).last {
        (*bucket).last = &mut (*hashtable).list;
        (*bucket).first = (*bucket).last;
    } else if pair_list == (*bucket).first {
        (*bucket).first = (*pair).list.next;
    } else if pair_list == (*bucket).last {
        (*bucket).last = (*pair).list.prev;
    }

    list_remove(&mut (*pair).list);
    list_remove(&mut (*pair).ordered_list);
    json_decref((*pair).value);

    jsonp_free(pair as *mut c_void);
    (*hashtable).size -= 1;

    0
}

unsafe fn hashtable_do_clear(hashtable: *mut HashtableT) {
    let mut list: *mut HashtableList;
    let mut next: *mut HashtableList;
    let mut pair: *mut HashtablePair;

    list = (*hashtable).list.next;
    while list != &mut (*hashtable).list as *mut HashtableList {
        next = (*list).next;
        pair = list_to_pair(list);
        json_decref((*pair).value);
        jsonp_free(pair as *mut c_void);
        list = next;
    }
}

unsafe fn hashtable_do_rehash(hashtable: *mut HashtableT) -> c_int {
    let mut list: *mut HashtableList;
    let mut next: *mut HashtableList;
    let mut pair: *mut HashtablePair;
    let new_size: usize;
    let new_order: usize;
    let new_buckets: *mut HashtableBucket;

    new_order = (*hashtable).order + 1;
    new_size = hashsize(new_order);

    new_buckets =
        jsonp_malloc(new_size * core::mem::size_of::<HashtableBucket>()) as *mut HashtableBucket;
    if new_buckets.is_null() {
        return -1;
    }

    jsonp_free((*hashtable).buckets as *mut c_void);
    (*hashtable).buckets = new_buckets;
    (*hashtable).order = new_order;

    for i in 0..new_size {
        (*(*hashtable).buckets.add(i)).last = &mut (*hashtable).list;
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
    }

    list = (*hashtable).list.next;
    list_init(&mut (*hashtable).list);

    while list != &mut (*hashtable).list as *mut HashtableList {
        next = (*list).next;
        pair = list_to_pair(list);
        let index = (*pair).hash % new_size;
        insert_to_bucket(
            hashtable,
            (*hashtable).buckets.add(index),
            &mut (*pair).list,
        );
        list = next;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_init(hashtable: *mut HashtableT) -> c_int {
    (*hashtable).size = 0;
    (*hashtable).order = INITIAL_HASHTABLE_ORDER;
    (*hashtable).buckets = jsonp_malloc(
        hashsize((*hashtable).order) * core::mem::size_of::<HashtableBucket>(),
    ) as *mut HashtableBucket;
    if (*hashtable).buckets.is_null() {
        return -1;
    }

    list_init(&mut (*hashtable).list);
    list_init(&mut (*hashtable).ordered_list);

    for i in 0..hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).last = &mut (*hashtable).list;
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_close(hashtable: *mut HashtableT) {
    hashtable_do_clear(hashtable);
    jsonp_free((*hashtable).buckets as *mut c_void);
}

unsafe fn init_pair(
    value: *mut JsonT,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> *mut HashtablePair {
    let pair: *mut HashtablePair;

    /* offsetof(...) returns the size of pair_t without the last,
    flexible member. This way, the correct amount is allocated. */

    if key_len >= usize::MAX - PAIR_KEY_OFFSET {
        /* Avoid an overflow if the key is very long */
        return core::ptr::null_mut();
    }

    pair = jsonp_malloc(PAIR_KEY_OFFSET + key_len + 1) as *mut HashtablePair;

    if pair.is_null() {
        return core::ptr::null_mut();
    }

    (*pair).hash = hash;
    let key_ptr = (*pair).key.as_mut_ptr();
    memcpy(key_ptr as *mut c_void, key as *const c_void, key_len);
    *key_ptr.add(key_len) = 0;
    (*pair).key_len = key_len;
    (*pair).value = value;

    list_init(&mut (*pair).list);
    list_init(&mut (*pair).ordered_list);

    pair
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_set(
    hashtable: *mut HashtableT,
    key: *const c_char,
    key_len: usize,
    value: *mut JsonT,
) -> c_int {
    let mut pair: *mut HashtablePair;
    let bucket: *mut HashtableBucket;
    let hash: usize;
    let index: usize;

    /* rehash if the load ratio exceeds 1 */
    if (*hashtable).size >= hashsize((*hashtable).order) && hashtable_do_rehash(hashtable) != 0 {
        return -1;
    }

    hash = hash_str(key, key_len);
    index = hash & hashmask((*hashtable).order);
    bucket = (*hashtable).buckets.add(index);
    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);

    if !pair.is_null() {
        json_decref((*pair).value);
        (*pair).value = value;
    } else {
        pair = init_pair(value, key, key_len, hash);

        if pair.is_null() {
            return -1;
        }

        insert_to_bucket(hashtable, bucket, &mut (*pair).list);
        list_insert(&mut (*hashtable).ordered_list, &mut (*pair).ordered_list);

        (*hashtable).size += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_get(
    hashtable: *mut HashtableT,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let pair: *mut HashtablePair;
    let hash: usize;
    let bucket: *mut HashtableBucket;

    hash = hash_str(key, key_len);
    bucket = (*hashtable).buckets.add(hash & hashmask((*hashtable).order));

    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return core::ptr::null_mut();
    }

    (*pair).value as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_del(
    hashtable: *mut HashtableT,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    let hash = hash_str(key, key_len);
    hashtable_do_del(hashtable, key, key_len, hash)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_clear(hashtable: *mut HashtableT) {
    hashtable_do_clear(hashtable);

    for i in 0..hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).last = &mut (*hashtable).list;
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
    }

    list_init(&mut (*hashtable).list);
    list_init(&mut (*hashtable).ordered_list);
    (*hashtable).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter(hashtable: *mut HashtableT) -> *mut c_void {
    hashtable_iter_next(hashtable, &mut (*hashtable).ordered_list as *mut _ as *mut c_void)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_at(
    hashtable: *mut HashtableT,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let pair: *mut HashtablePair;
    let hash: usize;
    let bucket: *mut HashtableBucket;

    hash = hash_str(key, key_len);
    bucket = (*hashtable).buckets.add(hash & hashmask((*hashtable).order));

    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return core::ptr::null_mut();
    }

    &mut (*pair).ordered_list as *mut _ as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_next(
    hashtable: *mut HashtableT,
    iter: *mut c_void,
) -> *mut c_void {
    let list = iter as *mut HashtableList;
    if (*list).next == &mut (*hashtable).ordered_list as *mut HashtableList {
        return core::ptr::null_mut();
    }
    (*list).next as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key(iter: *mut c_void) -> *mut c_void {
    let pair = ordered_list_to_pair(iter as *mut HashtableList);
    (*pair).key.as_mut_ptr() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key_len(iter: *mut c_void) -> usize {
    let pair = ordered_list_to_pair(iter as *mut HashtableList);
    (*pair).key_len
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_value(iter: *mut c_void) -> *mut c_void {
    let pair = ordered_list_to_pair(iter as *mut HashtableList);
    (*pair).value as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_set(iter: *mut c_void, value: *mut JsonT) {
    let pair = ordered_list_to_pair(iter as *mut HashtableList);

    json_decref((*pair).value);
    (*pair).value = value;
}
