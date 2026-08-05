//! Translation of hashtable.c
#![allow(non_upper_case_globals)]

use crate::hashtable_seed::hashtable_seed;
use crate::lookup3::hashlittle;
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::types::*;
use crate::types::json_decref;
use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::Ordering;

extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
}

const INITIAL_HASHTABLE_ORDER: usize = 3;

type list_t = hashtable_list;
type pair_t = hashtable_pair;
type bucket_t = hashtable_bucket;

#[inline]
fn hashsize(n: usize) -> usize {
    1usize << n
}
#[inline]
fn hashmask(n: usize) -> usize {
    hashsize(n) - 1
}

#[inline]
unsafe fn hash_str(key: *const c_char, len: usize) -> usize {
    hashlittle(
        key as *const c_void,
        len,
        hashtable_seed.load(Ordering::Relaxed),
    ) as usize
}

// container_of for list -> pair (list member is at offset 0)
#[inline]
unsafe fn list_to_pair(list: *mut list_t) -> *mut pair_t {
    // offsetof(pair_t, list) == 0
    (list as *mut u8).offset(-(offset_of_list())) as *mut pair_t
}
#[inline]
unsafe fn ordered_list_to_pair(list: *mut list_t) -> *mut pair_t {
    (list as *mut u8).offset(-(offset_of_ordered_list())) as *mut pair_t
}

#[inline]
fn offset_of_list() -> isize {
    // list is the first member
    core::mem::offset_of!(pair_t, list) as isize
}
#[inline]
fn offset_of_ordered_list() -> isize {
    core::mem::offset_of!(pair_t, ordered_list) as isize
}
#[inline]
fn offset_of_key() -> usize {
    core::mem::offset_of!(pair_t, key)
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
    (*bucket).first == &mut (*hashtable).list as *mut list_t && (*bucket).first == (*bucket).last
}

unsafe fn insert_to_bucket(hashtable: *mut hashtable_t, bucket: *mut bucket_t, list: *mut list_t) {
    if bucket_is_empty(hashtable, bucket) {
        list_insert(&mut (*hashtable).list, list);
        (*bucket).first = list;
        (*bucket).last = list;
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
    if bucket_is_empty(hashtable, bucket) {
        return ptr::null_mut();
    }

    let mut list = (*bucket).first;
    loop {
        let pair = list_to_pair(list);
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

    ptr::null_mut()
}

/* returns 0 on success, -1 if key was not found */
unsafe fn hashtable_do_del(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> c_int {
    let index = hash & hashmask((*hashtable).order);
    let bucket = (*hashtable).buckets.add(index);

    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return -1;
    }

    let pair_list = &mut (*pair).list as *mut list_t;
    let hlist = &mut (*hashtable).list as *mut list_t;

    if pair_list == (*bucket).first && pair_list == (*bucket).last {
        (*bucket).first = hlist;
        (*bucket).last = hlist;
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

unsafe fn hashtable_do_clear(hashtable: *mut hashtable_t) {
    let mut list = (*hashtable).list.next;
    let hlist = &mut (*hashtable).list as *mut list_t;
    while list != hlist {
        let next = (*list).next;
        let pair = list_to_pair(list);
        json_decref((*pair).value);
        jsonp_free(pair as *mut c_void);
        list = next;
    }
}

unsafe fn hashtable_do_rehash(hashtable: *mut hashtable_t) -> c_int {
    let new_order = (*hashtable).order + 1;
    let new_size = hashsize(new_order);

    let new_buckets = jsonp_malloc(new_size * size_of::<bucket_t>()) as *mut bucket_t;
    if new_buckets.is_null() {
        return -1;
    }

    jsonp_free((*hashtable).buckets as *mut c_void);
    (*hashtable).buckets = new_buckets;
    (*hashtable).order = new_order;

    let hlist = &mut (*hashtable).list as *mut list_t;
    for i in 0..new_size {
        (*(*hashtable).buckets.add(i)).first = hlist;
        (*(*hashtable).buckets.add(i)).last = hlist;
    }

    let mut list = (*hashtable).list.next;
    list_init(&mut (*hashtable).list);

    while list != hlist {
        let next = (*list).next;
        let pair = list_to_pair(list);
        let index = (*pair).hash % new_size;
        insert_to_bucket(hashtable, (*hashtable).buckets.add(index), &mut (*pair).list);
        list = next;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_init(hashtable: *mut hashtable_t) -> c_int {
    (*hashtable).size = 0;
    (*hashtable).order = INITIAL_HASHTABLE_ORDER;
    (*hashtable).buckets =
        jsonp_malloc(hashsize((*hashtable).order) * size_of::<bucket_t>()) as *mut bucket_t;
    if (*hashtable).buckets.is_null() {
        return -1;
    }

    list_init(&mut (*hashtable).list);
    list_init(&mut (*hashtable).ordered_list);

    let hlist = &mut (*hashtable).list as *mut list_t;
    for i in 0..hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).first = hlist;
        (*(*hashtable).buckets.add(i)).last = hlist;
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
    if key_len >= usize::MAX - offset_of_key() {
        /* Avoid an overflow if the key is very long */
        return ptr::null_mut();
    }

    let pair = jsonp_malloc(offset_of_key() + key_len + 1) as *mut pair_t;

    if pair.is_null() {
        return ptr::null_mut();
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

    list_init(&mut (*pair).list);
    list_init(&mut (*pair).ordered_list);

    pair
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_set(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    /* rehash if the load ratio exceeds 1 */
    if (*hashtable).size >= hashsize((*hashtable).order) {
        if hashtable_do_rehash(hashtable) != 0 {
            return -1;
        }
    }

    let hash = hash_str(key, key_len);
    let index = hash & hashmask((*hashtable).order);
    let bucket = (*hashtable).buckets.add(index);
    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);

    if !pair.is_null() {
        json_decref((*pair).value);
        (*pair).value = value;
    } else {
        let pair = init_pair(value, key, key_len, hash);

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
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let hash = hash_str(key, key_len);
    let bucket = (*hashtable).buckets.add(hash & hashmask((*hashtable).order));

    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return ptr::null_mut();
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
    hashtable_do_clear(hashtable);

    let hlist = &mut (*hashtable).list as *mut list_t;
    for i in 0..hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).first = hlist;
        (*(*hashtable).buckets.add(i)).last = hlist;
    }

    list_init(&mut (*hashtable).list);
    list_init(&mut (*hashtable).ordered_list);
    (*hashtable).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter(hashtable: *mut hashtable_t) -> *mut c_void {
    hashtable_iter_next(hashtable, &mut (*hashtable).ordered_list as *mut list_t as *mut c_void)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_at(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let hash = hash_str(key, key_len);
    let bucket = (*hashtable).buckets.add(hash & hashmask((*hashtable).order));

    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
    if pair.is_null() {
        return ptr::null_mut();
    }

    &mut (*pair).ordered_list as *mut list_t as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_next(
    hashtable: *mut hashtable_t,
    iter: *mut c_void,
) -> *mut c_void {
    let list = iter as *mut list_t;
    if (*list).next == &mut (*hashtable).ordered_list as *mut list_t {
        return ptr::null_mut();
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

// hashtable_key_to_iter macro helper used by value.c's json_object_key_to_iter
#[inline]
pub unsafe fn hashtable_key_to_iter(key: *const c_char) -> *mut c_void {
    // &(container_of(key_, struct hashtable_pair, key)->ordered_list)
    let pair = (key as *mut u8).offset(-(offset_of_key() as isize)) as *mut pair_t;
    &mut (*pair).ordered_list as *mut list_t as *mut c_void
}
