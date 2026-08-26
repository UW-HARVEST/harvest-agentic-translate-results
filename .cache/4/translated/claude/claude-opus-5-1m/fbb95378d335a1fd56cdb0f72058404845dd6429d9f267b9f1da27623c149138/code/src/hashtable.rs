//! Translation of `src/hashtable.c`.

use crate::lookup3::{hashlittle, hashmask, hashsize};
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::types::*;
use core::ffi::{c_char, c_int, c_void};
use core::mem::offset_of;

const INITIAL_HASHTABLE_ORDER: usize = 3;

type list_t = hashtable_list;
type pair_t = hashtable_pair;
type bucket_t = hashtable_bucket;

#[inline]
unsafe fn list_to_pair(list: *mut list_t) -> *mut pair_t {
    (list as *mut u8).sub(offset_of!(pair_t, list)) as *mut pair_t
}

#[inline]
unsafe fn ordered_list_to_pair(list: *mut list_t) -> *mut pair_t {
    (list as *mut u8).sub(offset_of!(pair_t, ordered_list)) as *mut pair_t
}

#[inline]
unsafe fn hash_str(key: *const c_char, len: usize) -> usize {
    hashlittle(
        key as *const c_void,
        len,
        core::ptr::read_volatile(core::ptr::addr_of!(crate::hashtable_seed::hashtable_seed)),
    ) as usize
}

#[inline]
unsafe fn list_init(list: *mut list_t) {
    (*list).next = list;
    (*list).prev = list;
}

#[inline]
unsafe fn list_insert(list: *mut list_t, node: *mut list_t) {
    (*node).next = list;
    (*node).prev = (*list).prev;
    (*(*list).prev).next = node;
    (*list).prev = node;
}

#[inline]
unsafe fn list_remove(list: *mut list_t) {
    (*(*list).prev).next = (*list).next;
    (*(*list).next).prev = (*list).prev;
}

#[inline]
unsafe fn bucket_is_empty(hashtable: *mut hashtable_t, bucket: *mut bucket_t) -> bool {
    (*bucket).first == core::ptr::addr_of_mut!((*hashtable).list) && (*bucket).first == (*bucket).last
}

unsafe fn insert_to_bucket(hashtable: *mut hashtable_t, bucket: *mut bucket_t, list: *mut list_t) {
    if bucket_is_empty(hashtable, bucket) {
        list_insert(core::ptr::addr_of_mut!((*hashtable).list), list);
        (*bucket).last = list;
        (*bucket).first = list;
    } else {
        list_insert((*bucket).first, list);
        (*bucket).first = list;
    }
}

unsafe fn hashtable_find_pair(
    hashtable: *mut hashtable_t,
    bucket: *mut bucket_t,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> *mut pair_t {
    let mut list: *mut list_t;
    let mut pair: *mut pair_t;

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
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> c_int {
    let pair: *mut pair_t;
    let bucket: *mut bucket_t;
    let index: usize;

    index = hash & hashmask((*hashtable).order);
    bucket = (*hashtable).buckets.add(index);

    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return -1;
    }

    let plist = core::ptr::addr_of_mut!((*pair).list);

    if plist == (*bucket).first && plist == (*bucket).last {
        (*bucket).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*bucket).first = (*bucket).last;
    } else if plist == (*bucket).first {
        (*bucket).first = (*pair).list.next;
    } else if plist == (*bucket).last {
        (*bucket).last = (*pair).list.prev;
    }

    list_remove(core::ptr::addr_of_mut!((*pair).list));
    list_remove(core::ptr::addr_of_mut!((*pair).ordered_list));
    json_decref((*pair).value);

    jsonp_free(pair as *mut c_void);
    (*hashtable).size -= 1;

    0
}

unsafe fn hashtable_do_clear(hashtable: *mut hashtable_t) {
    let mut list: *mut list_t;
    let mut next: *mut list_t;
    let mut pair: *mut pair_t;

    list = (*hashtable).list.next;
    while list != core::ptr::addr_of_mut!((*hashtable).list) {
        next = (*list).next;
        pair = list_to_pair(list);
        json_decref((*pair).value);
        jsonp_free(pair as *mut c_void);
        list = next;
    }
}

unsafe fn hashtable_do_rehash(hashtable: *mut hashtable_t) -> c_int {
    let mut list: *mut list_t;
    let mut next: *mut list_t;
    let mut pair: *mut pair_t;
    let mut i: usize;
    let mut index: usize;
    let new_size: usize;
    let new_order: usize;
    let new_buckets: *mut hashtable_bucket;

    new_order = (*hashtable).order + 1;
    new_size = hashsize(new_order);

    new_buckets =
        jsonp_malloc(new_size * core::mem::size_of::<bucket_t>()) as *mut hashtable_bucket;
    if new_buckets.is_null() {
        return -1;
    }

    jsonp_free((*hashtable).buckets as *mut c_void);
    (*hashtable).buckets = new_buckets;
    (*hashtable).order = new_order;

    i = 0;
    while i < new_size {
        (*(*hashtable).buckets.add(i)).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
        i += 1;
    }

    list = (*hashtable).list.next;
    list_init(core::ptr::addr_of_mut!((*hashtable).list));

    while list != core::ptr::addr_of_mut!((*hashtable).list) {
        next = (*list).next;
        pair = list_to_pair(list);
        index = (*pair).hash % new_size;
        insert_to_bucket(
            hashtable,
            (*hashtable).buckets.add(index),
            core::ptr::addr_of_mut!((*pair).list),
        );
        list = next;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_init(hashtable: *mut hashtable_t) -> c_int {
    let mut i: usize;

    (*hashtable).size = 0;
    (*hashtable).order = INITIAL_HASHTABLE_ORDER;
    (*hashtable).buckets =
        jsonp_malloc(hashsize((*hashtable).order) * core::mem::size_of::<bucket_t>())
            as *mut hashtable_bucket;
    if (*hashtable).buckets.is_null() {
        return -1;
    }

    list_init(core::ptr::addr_of_mut!((*hashtable).list));
    list_init(core::ptr::addr_of_mut!((*hashtable).ordered_list));

    i = 0;
    while i < hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
        i += 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_close(hashtable: *mut hashtable_t) {
    hashtable_do_clear(hashtable);
    jsonp_free((*hashtable).buckets as *mut c_void);
}

unsafe fn init_pair(
    value: *mut json_t,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> *mut pair_t {
    let pair: *mut pair_t;

    /* offsetof(...) returns the size of pair_t without the last,
    flexible member. This way, the correct amount is allocated. */

    if key_len >= usize::MAX - offset_of!(pair_t, key) {
        /* Avoid an overflow if the key is very long */
        return core::ptr::null_mut();
    }

    pair = jsonp_malloc(offset_of!(pair_t, key) + key_len + 1) as *mut pair_t;

    if pair.is_null() {
        return core::ptr::null_mut();
    }

    (*pair).hash = hash;
    memcpy(
        (*pair).key.as_mut_ptr() as *mut c_void,
        key as *const c_void,
        key_len,
    );
    *(*pair).key.as_mut_ptr().add(key_len) = 0;
    (*pair).key_len = key_len;
    (*pair).value = value;

    list_init(core::ptr::addr_of_mut!((*pair).list));
    list_init(core::ptr::addr_of_mut!((*pair).ordered_list));

    pair
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_set(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    let mut pair: *mut pair_t;
    let bucket: *mut bucket_t;
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

        insert_to_bucket(hashtable, bucket, core::ptr::addr_of_mut!((*pair).list));
        list_insert(
            core::ptr::addr_of_mut!((*hashtable).ordered_list),
            core::ptr::addr_of_mut!((*pair).ordered_list),
        );

        (*hashtable).size += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_get(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let pair: *mut pair_t;
    let hash: usize;
    let bucket: *mut bucket_t;

    hash = hash_str(key, key_len);
    bucket = (*hashtable)
        .buckets
        .add(hash & hashmask((*hashtable).order));

    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return core::ptr::null_mut();
    }

    (*pair).value as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_del(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    let hash = hash_str(key, key_len);
    hashtable_do_del(hashtable, key, key_len, hash)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_clear(hashtable: *mut hashtable_t) {
    let mut i: usize;

    hashtable_do_clear(hashtable);

    i = 0;
    while i < hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
        i += 1;
    }

    list_init(core::ptr::addr_of_mut!((*hashtable).list));
    list_init(core::ptr::addr_of_mut!((*hashtable).ordered_list));
    (*hashtable).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter(hashtable: *mut hashtable_t) -> *mut c_void {
    hashtable_iter_next(
        hashtable,
        core::ptr::addr_of_mut!((*hashtable).ordered_list) as *mut c_void,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_at(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let pair: *mut pair_t;
    let hash: usize;
    let bucket: *mut bucket_t;

    hash = hash_str(key, key_len);
    bucket = (*hashtable)
        .buckets
        .add(hash & hashmask((*hashtable).order));

    pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return core::ptr::null_mut();
    }

    core::ptr::addr_of_mut!((*pair).ordered_list) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_next(
    hashtable: *mut hashtable_t,
    iter: *mut c_void,
) -> *mut c_void {
    let list = iter as *mut list_t;
    if (*list).next == core::ptr::addr_of_mut!((*hashtable).ordered_list) {
        return core::ptr::null_mut();
    }
    (*list).next as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key(iter: *mut c_void) -> *mut c_void {
    let pair = ordered_list_to_pair(iter as *mut list_t);
    (*pair).key.as_mut_ptr() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key_len(iter: *mut c_void) -> usize {
    let pair = ordered_list_to_pair(iter as *mut list_t);
    (*pair).key_len
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_value(iter: *mut c_void) -> *mut c_void {
    let pair = ordered_list_to_pair(iter as *mut list_t);
    (*pair).value as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_set(iter: *mut c_void, value: *mut json_t) {
    let pair = ordered_list_to_pair(iter as *mut list_t);

    json_decref((*pair).value);
    (*pair).value = value;
}

/// `hashtable_key_to_iter` macro from `hashtable.h`.
#[inline]
pub unsafe fn hashtable_key_to_iter(key: *const c_char) -> *mut c_void {
    let pair = (key as *mut u8).sub(offset_of!(pair_t, key)) as *mut pair_t;
    core::ptr::addr_of_mut!((*pair).ordered_list) as *mut c_void
}
