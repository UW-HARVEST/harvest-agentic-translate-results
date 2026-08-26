//! Translation of c_src/src/hashtable.c and c_src/src/lookup3.h
#![allow(dead_code)]

use crate::jansson::{json_decref, json_t};
use crate::libc;
use crate::memory::{jsonp_free, jsonp_malloc};
use std::ffi::{c_char, c_int, c_void};

pub const INITIAL_HASHTABLE_ORDER: usize = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
}

pub type list_t = hashtable_list;

#[repr(C)]
pub struct hashtable_pair {
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
    pub hash: usize,
    pub value: *mut json_t,
    pub key_len: usize,
    pub key: [c_char; 1],
}

pub type pair_t = hashtable_pair;

/* offsetof(pair_t, key) */
pub const PAIR_KEY_OFFSET: usize = 56;
/* offsetof(pair_t, ordered_list) */
pub const PAIR_ORDERED_LIST_OFFSET: usize = 16;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hashtable_bucket {
    pub first: *mut hashtable_list,
    pub last: *mut hashtable_list,
}

pub type bucket_t = hashtable_bucket;

#[repr(C)]
pub struct hashtable {
    pub size: usize,
    pub buckets: *mut hashtable_bucket,
    pub order: usize,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

pub type hashtable_t = hashtable;

impl hashtable {
    pub const fn new() -> hashtable {
        hashtable {
            size: 0,
            buckets: std::ptr::null_mut(),
            order: 0,
            list: hashtable_list {
                prev: std::ptr::null_mut(),
                next: std::ptr::null_mut(),
            },
            ordered_list: hashtable_list {
                prev: std::ptr::null_mut(),
                next: std::ptr::null_mut(),
            },
        }
    }
}

#[inline]
pub fn hashsize(n: usize) -> usize {
    1usize << n
}

#[inline]
pub fn hashmask(n: usize) -> usize {
    hashsize(n) - 1
}

/* --- lookup3.h: Bob Jenkins' hashlittle() --- */

#[inline]
fn rot(x: u32, k: u32) -> u32 {
    (x << k) | (x >> (32 - k))
}

#[inline]
fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 4);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 6);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 8);
    *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 16);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 19);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 4);
    *b = b.wrapping_add(*a);
}

#[inline]
fn final_mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 14));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 11));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 25));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 16));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 4));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 14));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 24));
}

/// Byte-oriented implementation of hashlittle(); on a little-endian machine
/// it yields exactly the same result as the word-at-a-time variants used by
/// the C code (including the masking trick).
pub unsafe fn hashlittle(key: *const c_void, length0: usize, initval: u32) -> u32 {
    let mut length = length0;
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;

    a = 0xdeadbeefu32
        .wrapping_add(length as u32)
        .wrapping_add(initval);
    b = a;
    c = a;

    let mut k = key as *const u8;

    while length > 12 {
        a = a.wrapping_add(*k.add(0) as u32);
        a = a.wrapping_add((*k.add(1) as u32) << 8);
        a = a.wrapping_add((*k.add(2) as u32) << 16);
        a = a.wrapping_add((*k.add(3) as u32) << 24);
        b = b.wrapping_add(*k.add(4) as u32);
        b = b.wrapping_add((*k.add(5) as u32) << 8);
        b = b.wrapping_add((*k.add(6) as u32) << 16);
        b = b.wrapping_add((*k.add(7) as u32) << 24);
        c = c.wrapping_add(*k.add(8) as u32);
        c = c.wrapping_add((*k.add(9) as u32) << 8);
        c = c.wrapping_add((*k.add(10) as u32) << 16);
        c = c.wrapping_add((*k.add(11) as u32) << 24);
        mix(&mut a, &mut b, &mut c);
        length -= 12;
        k = k.add(12);
    }

    /* last block: affect all 32 bits of (c) */
    if length == 12 {
        c = c.wrapping_add((*k.add(11) as u32) << 24);
    }
    if length >= 11 {
        c = c.wrapping_add((*k.add(10) as u32) << 16);
    }
    if length >= 10 {
        c = c.wrapping_add((*k.add(9) as u32) << 8);
    }
    if length >= 9 {
        c = c.wrapping_add(*k.add(8) as u32);
    }
    if length >= 8 {
        b = b.wrapping_add((*k.add(7) as u32) << 24);
    }
    if length >= 7 {
        b = b.wrapping_add((*k.add(6) as u32) << 16);
    }
    if length >= 6 {
        b = b.wrapping_add((*k.add(5) as u32) << 8);
    }
    if length >= 5 {
        b = b.wrapping_add(*k.add(4) as u32);
    }
    if length >= 4 {
        a = a.wrapping_add((*k.add(3) as u32) << 24);
    }
    if length >= 3 {
        a = a.wrapping_add((*k.add(2) as u32) << 16);
    }
    if length >= 2 {
        a = a.wrapping_add((*k.add(1) as u32) << 8);
    }
    if length >= 1 {
        a = a.wrapping_add(*k.add(0) as u32);
    } else {
        /* zero length strings require no mixing */
        return c;
    }

    final_mix(&mut a, &mut b, &mut c);
    c
}

#[inline]
unsafe fn hash_str(key: *const c_char, len: usize) -> usize {
    hashlittle(
        key as *const c_void,
        len,
        crate::hashtable_seed::get_hashtable_seed(),
    ) as usize
}

/* --- list helpers --- */

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
unsafe fn list_to_pair(list: *mut list_t) -> *mut pair_t {
    list as *mut pair_t
}

#[inline]
unsafe fn ordered_list_to_pair(list: *mut list_t) -> *mut pair_t {
    (list as *mut u8).sub(PAIR_ORDERED_LIST_OFFSET) as *mut pair_t
}

#[inline]
unsafe fn pair_key_ptr(pair: *mut pair_t) -> *mut c_char {
    (pair as *mut u8).add(PAIR_KEY_OFFSET) as *mut c_char
}

/// hashtable_key_to_iter(key) from hashtable.h
#[inline]
pub unsafe fn hashtable_key_to_iter(key: *const c_char) -> *mut c_void {
    (key as *mut u8)
        .sub(PAIR_KEY_OFFSET)
        .add(PAIR_ORDERED_LIST_OFFSET) as *mut c_void
}

#[inline]
unsafe fn bucket_is_empty(hashtable: *mut hashtable_t, bucket: *mut bucket_t) -> bool {
    (*bucket).first == std::ptr::addr_of_mut!((*hashtable).list) && (*bucket).first == (*bucket).last
}

unsafe fn insert_to_bucket(hashtable: *mut hashtable_t, bucket: *mut bucket_t, list: *mut list_t) {
    if bucket_is_empty(hashtable, bucket) {
        list_insert(std::ptr::addr_of_mut!((*hashtable).list), list);
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
    let mut list: *mut list_t;
    let mut pair: *mut pair_t;

    if bucket_is_empty(hashtable, bucket) {
        return std::ptr::null_mut();
    }

    list = (*bucket).first;
    loop {
        pair = list_to_pair(list);
        if (*pair).hash == hash
            && (*pair).key_len == key_len
            && libc::memcmp(
                pair_key_ptr(pair) as *const c_void,
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

    std::ptr::null_mut()
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

    let pair_list = std::ptr::addr_of_mut!((*pair).list);
    if pair_list == (*bucket).first && pair_list == (*bucket).last {
        (*bucket).first = std::ptr::addr_of_mut!((*hashtable).list);
        (*bucket).last = std::ptr::addr_of_mut!((*hashtable).list);
    } else if pair_list == (*bucket).first {
        (*bucket).first = (*pair).list.next;
    } else if pair_list == (*bucket).last {
        (*bucket).last = (*pair).list.prev;
    }

    list_remove(std::ptr::addr_of_mut!((*pair).list));
    list_remove(std::ptr::addr_of_mut!((*pair).ordered_list));
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
    while list != std::ptr::addr_of_mut!((*hashtable).list) {
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

    new_buckets = jsonp_malloc(new_size * std::mem::size_of::<bucket_t>()) as *mut hashtable_bucket;
    if new_buckets.is_null() {
        return -1;
    }

    jsonp_free((*hashtable).buckets as *mut c_void);
    (*hashtable).buckets = new_buckets;
    (*hashtable).order = new_order;

    i = 0;
    while i < new_size {
        (*(*hashtable).buckets.add(i)).first = std::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).last = std::ptr::addr_of_mut!((*hashtable).list);
        i += 1;
    }

    list = (*hashtable).list.next;
    list_init(std::ptr::addr_of_mut!((*hashtable).list));

    while list != std::ptr::addr_of_mut!((*hashtable).list) {
        next = (*list).next;
        pair = list_to_pair(list);
        index = (*pair).hash % new_size;
        insert_to_bucket(
            hashtable,
            (*hashtable).buckets.add(index),
            std::ptr::addr_of_mut!((*pair).list),
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
        jsonp_malloc(hashsize((*hashtable).order) * std::mem::size_of::<bucket_t>())
            as *mut hashtable_bucket;
    if (*hashtable).buckets.is_null() {
        return -1;
    }

    list_init(std::ptr::addr_of_mut!((*hashtable).list));
    list_init(std::ptr::addr_of_mut!((*hashtable).ordered_list));

    i = 0;
    while i < hashsize((*hashtable).order) {
        (*(*hashtable).buckets.add(i)).first = std::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).last = std::ptr::addr_of_mut!((*hashtable).list);
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

    if key_len >= usize::MAX - PAIR_KEY_OFFSET {
        /* Avoid an overflow if the key is very long */
        return std::ptr::null_mut();
    }

    pair = jsonp_malloc(PAIR_KEY_OFFSET + key_len + 1) as *mut pair_t;

    if pair.is_null() {
        return std::ptr::null_mut();
    }

    (*pair).hash = hash;
    libc::memcpy(
        pair_key_ptr(pair) as *mut c_void,
        key as *const c_void,
        key_len,
    );
    *pair_key_ptr(pair).add(key_len) = 0;
    (*pair).key_len = key_len;
    (*pair).value = value;

    list_init(std::ptr::addr_of_mut!((*pair).list));
    list_init(std::ptr::addr_of_mut!((*pair).ordered_list));

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

        insert_to_bucket(hashtable, bucket, std::ptr::addr_of_mut!((*pair).list));
        list_insert(
            std::ptr::addr_of_mut!((*hashtable).ordered_list),
            std::ptr::addr_of_mut!((*pair).ordered_list),
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
        return std::ptr::null_mut();
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
        (*(*hashtable).buckets.add(i)).first = std::ptr::addr_of_mut!((*hashtable).list);
        (*(*hashtable).buckets.add(i)).last = std::ptr::addr_of_mut!((*hashtable).list);
        i += 1;
    }

    list_init(std::ptr::addr_of_mut!((*hashtable).list));
    list_init(std::ptr::addr_of_mut!((*hashtable).ordered_list));
    (*hashtable).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter(hashtable: *mut hashtable_t) -> *mut c_void {
    hashtable_iter_next(
        hashtable,
        std::ptr::addr_of_mut!((*hashtable).ordered_list) as *mut c_void,
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
        return std::ptr::null_mut();
    }

    std::ptr::addr_of_mut!((*pair).ordered_list) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_next(
    hashtable: *mut hashtable_t,
    iter: *mut c_void,
) -> *mut c_void {
    let list = iter as *mut list_t;
    if (*list).next == std::ptr::addr_of_mut!((*hashtable).ordered_list) {
        return std::ptr::null_mut();
    }
    (*list).next as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key(iter: *mut c_void) -> *mut c_void {
    let pair = ordered_list_to_pair(iter as *mut list_t);
    pair_key_ptr(pair) as *mut c_void
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
