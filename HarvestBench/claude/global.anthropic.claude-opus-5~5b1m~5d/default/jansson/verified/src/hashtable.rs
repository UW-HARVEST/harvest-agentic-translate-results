//! Translation of `src/hashtable.c` / `src/hashtable.h`.

use core::ffi::{c_char, c_int, c_void};

use crate::ffi;
use crate::jansson::{json_decref, json_t};
use crate::lookup3::{hashlittle, hashmask, hashsize};
use crate::memory::{jsonp_free, jsonp_malloc};

pub const INITIAL_HASHTABLE_ORDER: usize = 3;

#[repr(C)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
}

/* "pair" may be a bit confusing a name, but think of it as a key-value pair.
   In this case, it just encodes some extra data, too */
#[repr(C)]
pub struct hashtable_pair {
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
    pub hash: usize,
    pub value: *mut json_t,
    pub key_len: usize,
    pub key: [c_char; 1],
}

#[repr(C)]
pub struct hashtable_bucket {
    pub first: *mut hashtable_list,
    pub last: *mut hashtable_list,
}

#[repr(C)]
pub struct hashtable {
    pub size: usize,
    pub buckets: *mut hashtable_bucket,
    pub order: usize, /* hashtable has pow(2, order) buckets */
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

pub type hashtable_t = hashtable;

type list_t = hashtable_list;
type pair_t = hashtable_pair;
type bucket_t = hashtable_bucket;

pub const OFFSET_OF_KEY: usize = core::mem::offset_of!(pair_t, key);
pub const OFFSET_OF_ORDERED_LIST: usize = core::mem::offset_of!(pair_t, ordered_list);

#[inline]
unsafe fn list_to_pair(list_: *mut list_t) -> *mut pair_t {
    list_ as *mut pair_t
}

#[inline]
unsafe fn ordered_list_to_pair(list_: *mut list_t) -> *mut pair_t {
    (list_ as *mut u8).sub(OFFSET_OF_ORDERED_LIST) as *mut pair_t
}

/// `hashtable_key_to_iter()` from the header: given the address of a pair's
/// key, return the address of the pair's `ordered_list` member.
#[inline]
pub unsafe fn hashtable_key_to_iter(key_: *const c_char) -> *mut c_void {
    let pair = (key_ as *const u8).sub(OFFSET_OF_KEY) as *mut pair_t;
    core::ptr::addr_of_mut!((*pair).ordered_list) as *mut c_void
}

#[inline]
unsafe fn hash_str(key: *const c_char, len: usize) -> usize {
    hashlittle(
        key as *const c_void,
        len,
        crate::hashtable_seed::seed_value(),
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
    (*bucket).first == core::ptr::addr_of_mut!((*hashtable).list)
        && (*bucket).first == (*bucket).last
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
            && ffi::memcmp(
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
    let index = hash & hashmask((*hashtable).order);
    let bucket = (*hashtable).buckets.add(index);

    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
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
    let mut list = (*hashtable).list.next;
    let head = core::ptr::addr_of_mut!((*hashtable).list);
    while list != head {
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

    let new_buckets =
        jsonp_malloc(new_size * core::mem::size_of::<bucket_t>()) as *mut bucket_t;
    if new_buckets.is_null() {
        return -1;
    }

    jsonp_free((*hashtable).buckets as *mut c_void);
    (*hashtable).buckets = new_buckets;
    (*hashtable).order = new_order;

    for i in 0..new_size {
        (*(*hashtable).buckets.add(i)).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
    }

    let mut list = (*hashtable).list.next;
    list_init(core::ptr::addr_of_mut!((*hashtable).list));

    let head = core::ptr::addr_of_mut!((*hashtable).list);
    while list != head {
        let next = (*list).next;
        let pair = list_to_pair(list);
        let index = (*pair).hash % new_size;
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
    (*hashtable).size = 0;
    (*hashtable).order = INITIAL_HASHTABLE_ORDER;
    (*hashtable).buckets =
        jsonp_malloc(hashsize((*hashtable).order) * core::mem::size_of::<bucket_t>())
            as *mut bucket_t;
    if (*hashtable).buckets.is_null() {
        return -1;
    }

    list_init(core::ptr::addr_of_mut!((*hashtable).list));
    list_init(core::ptr::addr_of_mut!((*hashtable).ordered_list));

    for i in 0..hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
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
    /* offsetof(...) returns the size of pair_t without the last, flexible
    member. This way, the correct amount is allocated. */

    if key_len >= usize::MAX - OFFSET_OF_KEY {
        /* Avoid an overflow if the key is very long */
        return core::ptr::null_mut();
    }

    let pair = jsonp_malloc(OFFSET_OF_KEY + key_len + 1) as *mut pair_t;

    if pair.is_null() {
        return core::ptr::null_mut();
    }

    (*pair).hash = hash;
    ffi::memcpy(
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
    /* rehash if the load ratio exceeds 1 */
    if (*hashtable).size >= hashsize((*hashtable).order) && hashtable_do_rehash(hashtable) != 0 {
        return -1;
    }

    let hash = hash_str(key, key_len);
    let index = hash & hashmask((*hashtable).order);
    let bucket = (*hashtable).buckets.add(index);
    let mut pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);

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
    let hash = hash_str(key, key_len);
    let bucket = (*hashtable)
        .buckets
        .add(hash & hashmask((*hashtable).order));

    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
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
    hashtable_do_clear(hashtable);

    for i in 0..hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).last = core::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).first = (*(*hashtable).buckets.add(i)).last;
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
    let hash = hash_str(key, key_len);
    let bucket = (*hashtable)
        .buckets
        .add(hash & hashmask((*hashtable).order));

    let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
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
