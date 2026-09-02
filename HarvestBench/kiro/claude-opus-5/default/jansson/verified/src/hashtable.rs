//! Translation of `src/hashtable.c` / `src/hashtable.h`.

use crate::cffi;
use crate::hashtable_seed::hashtable_seed_value;
use crate::jtypes::{json_decref, json_t};
use crate::lookup3::{hashlittle, hashmask, hashsize};
use crate::memory::{jsonp_free, jsonp_malloc};
use core::ffi::{c_char, c_int, c_void};

pub const INITIAL_HASHTABLE_ORDER: usize = 3;

#[repr(C)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
}

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
pub struct hashtable_t {
    pub size: usize,
    pub buckets: *mut hashtable_bucket,
    /// hashtable has pow(2, order) buckets
    pub order: usize,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

impl hashtable_t {
    pub const fn new() -> Self {
        hashtable_t {
            size: 0,
            buckets: core::ptr::null_mut(),
            order: 0,
            list: hashtable_list {
                prev: core::ptr::null_mut(),
                next: core::ptr::null_mut(),
            },
            ordered_list: hashtable_list {
                prev: core::ptr::null_mut(),
                next: core::ptr::null_mut(),
            },
        }
    }
}

/// `offsetof(struct hashtable_pair, key)`
pub const PAIR_KEY_OFFSET: usize = core::mem::offset_of!(hashtable_pair, key);
/// `offsetof(struct hashtable_pair, ordered_list)`
pub const PAIR_ORDERED_LIST_OFFSET: usize = core::mem::offset_of!(hashtable_pair, ordered_list);

/// `hashtable_key_to_iter(key)`
#[inline]
pub unsafe fn hashtable_key_to_iter(key: *const c_char) -> *mut c_void {
    unsafe {
        (key as *const u8)
            .sub(PAIR_KEY_OFFSET)
            .add(PAIR_ORDERED_LIST_OFFSET) as *mut c_void
    }
}

#[inline]
unsafe fn list_to_pair(list: *mut hashtable_list) -> *mut hashtable_pair {
    list as *mut hashtable_pair
}

#[inline]
unsafe fn ordered_list_to_pair(list: *mut hashtable_list) -> *mut hashtable_pair {
    unsafe { (list as *mut u8).sub(PAIR_ORDERED_LIST_OFFSET) as *mut hashtable_pair }
}

#[inline]
unsafe fn hash_str(key: *const c_char, len: usize) -> usize {
    unsafe { hashlittle(key as *const u8, len, hashtable_seed_value()) as usize }
}

#[inline]
unsafe fn list_init(list: *mut hashtable_list) {
    unsafe {
        (*list).next = list;
        (*list).prev = list;
    }
}

#[inline]
unsafe fn list_insert(list: *mut hashtable_list, node: *mut hashtable_list) {
    unsafe {
        (*node).next = list;
        (*node).prev = (*list).prev;
        (*(*list).prev).next = node;
        (*list).prev = node;
    }
}

#[inline]
unsafe fn list_remove(list: *mut hashtable_list) {
    unsafe {
        (*(*list).prev).next = (*list).next;
        (*(*list).next).prev = (*list).prev;
    }
}

#[inline]
unsafe fn bucket_is_empty(hashtable: *mut hashtable_t, bucket: *mut hashtable_bucket) -> bool {
    unsafe {
        (*bucket).first == core::ptr::addr_of_mut!((*hashtable).list)
            && (*bucket).first == (*bucket).last
    }
}

unsafe fn insert_to_bucket(
    hashtable: *mut hashtable_t,
    bucket: *mut hashtable_bucket,
    list: *mut hashtable_list,
) {
    unsafe {
        if bucket_is_empty(hashtable, bucket) {
            list_insert(core::ptr::addr_of_mut!((*hashtable).list), list);
            (*bucket).first = list;
            (*bucket).last = list;
        } else {
            list_insert((*bucket).first, list);
            (*bucket).first = list;
        }
    }
}

unsafe fn hashtable_find_pair(
    hashtable: *mut hashtable_t,
    bucket: *mut hashtable_bucket,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> *mut hashtable_pair {
    unsafe {
        if bucket_is_empty(hashtable, bucket) {
            return core::ptr::null_mut();
        }

        let mut list = (*bucket).first;
        loop {
            let pair = list_to_pair(list);
            if (*pair).hash == hash
                && (*pair).key_len == key_len
                && cffi::c_memcmp((*pair).key.as_ptr(), key, key_len) == 0
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
}

/* returns 0 on success, -1 if key was not found */
unsafe fn hashtable_do_del(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> c_int {
    unsafe {
        let index = hash & hashmask((*hashtable).order);
        let bucket = (*hashtable).buckets.add(index);

        let pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash);
        if pair.is_null() {
            return -1;
        }

        let plist = core::ptr::addr_of_mut!((*pair).list);
        let hlist = core::ptr::addr_of_mut!((*hashtable).list);

        if plist == (*bucket).first && plist == (*bucket).last {
            (*bucket).first = hlist;
            (*bucket).last = hlist;
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
}

unsafe fn hashtable_do_clear(hashtable: *mut hashtable_t) {
    unsafe {
        let head = core::ptr::addr_of_mut!((*hashtable).list);
        let mut list = (*hashtable).list.next;
        while list != head {
            let next = (*list).next;
            let pair = list_to_pair(list);
            json_decref((*pair).value);
            jsonp_free(pair as *mut c_void);
            list = next;
        }
    }
}

unsafe fn hashtable_do_rehash(hashtable: *mut hashtable_t) -> c_int {
    unsafe {
        let new_order = (*hashtable).order + 1;
        let new_size = hashsize(new_order);

        let new_buckets =
            jsonp_malloc(new_size * core::mem::size_of::<hashtable_bucket>()) as *mut hashtable_bucket;
        if new_buckets.is_null() {
            return -1;
        }

        jsonp_free((*hashtable).buckets as *mut c_void);
        (*hashtable).buckets = new_buckets;
        (*hashtable).order = new_order;

        let hlist = core::ptr::addr_of_mut!((*hashtable).list);
        for i in 0..new_size {
            (*(*hashtable).buckets.add(i)).first = hlist;
            (*(*hashtable).buckets.add(i)).last = hlist;
        }

        let mut list = (*hashtable).list.next;
        list_init(hlist);

        while list != hlist {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_init(hashtable: *mut hashtable_t) -> c_int {
    unsafe {
        (*hashtable).size = 0;
        (*hashtable).order = INITIAL_HASHTABLE_ORDER;
        (*hashtable).buckets = jsonp_malloc(
            hashsize((*hashtable).order) * core::mem::size_of::<hashtable_bucket>(),
        ) as *mut hashtable_bucket;
        if (*hashtable).buckets.is_null() {
            return -1;
        }

        list_init(core::ptr::addr_of_mut!((*hashtable).list));
        list_init(core::ptr::addr_of_mut!((*hashtable).ordered_list));

        let hlist = core::ptr::addr_of_mut!((*hashtable).list);
        for i in 0..hashsize((*hashtable).order) {
            (*(*hashtable).buckets.add(i)).first = hlist;
            (*(*hashtable).buckets.add(i)).last = hlist;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_close(hashtable: *mut hashtable_t) {
    unsafe {
        hashtable_do_clear(hashtable);
        jsonp_free((*hashtable).buckets as *mut c_void);
    }
}

unsafe fn init_pair(
    value: *mut json_t,
    key: *const c_char,
    key_len: usize,
    hash: usize,
) -> *mut hashtable_pair {
    unsafe {
        /* offsetof(...) returns the size of pair_t without the last,
        flexible member. This way, the correct amount is allocated. */
        if key_len >= usize::MAX - PAIR_KEY_OFFSET {
            /* Avoid an overflow if the key is very long */
            return core::ptr::null_mut();
        }

        let pair = jsonp_malloc(PAIR_KEY_OFFSET + key_len + 1) as *mut hashtable_pair;

        if pair.is_null() {
            return core::ptr::null_mut();
        }

        (*pair).hash = hash;
        let kp = (*pair).key.as_mut_ptr();
        core::ptr::copy_nonoverlapping(key as *const u8, kp as *mut u8, key_len);
        *kp.add(key_len) = 0;
        (*pair).key_len = key_len;
        (*pair).value = value;

        list_init(core::ptr::addr_of_mut!((*pair).list));
        list_init(core::ptr::addr_of_mut!((*pair).ordered_list));

        pair
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_set(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    unsafe {
        /* rehash if the load ratio exceeds 1 */
        if (*hashtable).size >= hashsize((*hashtable).order) {
            if hashtable_do_rehash(hashtable) != 0 {
                return -1;
            }
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_get(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_del(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    unsafe {
        let hash = hash_str(key, key_len);
        hashtable_do_del(hashtable, key, key_len, hash)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_clear(hashtable: *mut hashtable_t) {
    unsafe {
        hashtable_do_clear(hashtable);

        let hlist = core::ptr::addr_of_mut!((*hashtable).list);
        for i in 0..hashsize((*hashtable).order) {
            (*(*hashtable).buckets.add(i)).first = hlist;
            (*(*hashtable).buckets.add(i)).last = hlist;
        }

        list_init(core::ptr::addr_of_mut!((*hashtable).list));
        list_init(core::ptr::addr_of_mut!((*hashtable).ordered_list));
        (*hashtable).size = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter(hashtable: *mut hashtable_t) -> *mut c_void {
    unsafe {
        hashtable_iter_next(
            hashtable,
            core::ptr::addr_of_mut!((*hashtable).ordered_list) as *mut c_void,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_at(
    hashtable: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_next(
    hashtable: *mut hashtable_t,
    iter: *mut c_void,
) -> *mut c_void {
    unsafe {
        let list = iter as *mut hashtable_list;
        if (*list).next == core::ptr::addr_of_mut!((*hashtable).ordered_list) {
            return core::ptr::null_mut();
        }
        (*list).next as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key(iter: *mut c_void) -> *mut c_void {
    unsafe {
        let pair = ordered_list_to_pair(iter as *mut hashtable_list);
        (*pair).key.as_mut_ptr() as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key_len(iter: *mut c_void) -> usize {
    unsafe {
        let pair = ordered_list_to_pair(iter as *mut hashtable_list);
        (*pair).key_len
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_value(iter: *mut c_void) -> *mut c_void {
    unsafe {
        let pair = ordered_list_to_pair(iter as *mut hashtable_list);
        (*pair).value as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_set(iter: *mut c_void, value: *mut json_t) {
    unsafe {
        let pair = ordered_list_to_pair(iter as *mut hashtable_list);

        json_decref((*pair).value);
        (*pair).value = value;
    }
}
