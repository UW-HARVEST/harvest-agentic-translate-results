use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use crate::types::json_t;
use crate::value::decref;
use crate::value::json_null;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
pub struct strbuffer_t {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

#[repr(C)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
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
    pub order: usize,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

#[repr(C)]
struct PairHead {
    list: hashtable_list,
    ordered_list: hashtable_list,
    hash: usize,
    value: *mut json_t,
    key_len: usize,
}

const KEY_OFFSET: usize = std::mem::size_of::<PairHead>();

#[unsafe(no_mangle)]
pub static mut hashtable_seed: u32 = 0;

#[inline]
unsafe fn list_init(list: *mut hashtable_list) {
    (*list).prev = list;
    (*list).next = list;
}

#[inline]
unsafe fn list_insert(tail: *mut hashtable_list, node: *mut hashtable_list) {
    (*node).next = tail;
    (*node).prev = (*tail).prev;
    (*(*tail).prev).next = node;
    (*tail).prev = node;
}

#[inline]
unsafe fn list_remove(node: *mut hashtable_list) {
    (*(*node).prev).next = (*node).next;
    (*(*node).next).prev = (*node).prev;
}

#[inline]
unsafe fn pair_from_iter(iter: *mut c_void) -> *mut PairHead {
    iter.cast::<u8>()
        .sub(std::mem::offset_of!(PairHead, ordered_list))
        .cast()
}

#[inline]
unsafe fn pair_key(pair: *mut PairHead) -> *mut c_char {
    pair.cast::<u8>().add(KEY_OFFSET).cast()
}

unsafe fn find_pair(table: *mut hashtable_t, key: *const c_char, key_len: usize) -> *mut PairHead {
    let sentinel = &mut (*table).ordered_list as *mut hashtable_list;
    let mut node = (*sentinel).next;
    while node != sentinel {
        let pair = pair_from_iter(node.cast());
        if (*pair).key_len == key_len
            && libc::memcmp(pair_key(pair).cast(), key.cast(), key_len) == 0
        {
            return pair;
        }
        node = (*node).next;
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_init(table: *mut hashtable_t) -> c_int {
    if table.is_null() {
        return -1;
    }
    (*table).size = 0;
    (*table).order = 3;
    (*table).buckets = jsonp_malloc(8 * std::mem::size_of::<hashtable_bucket>()).cast();
    if (*table).buckets.is_null() {
        return -1;
    }
    list_init(&mut (*table).list);
    list_init(&mut (*table).ordered_list);
    for index in 0..8 {
        (*(*table).buckets.add(index)).first = &mut (*table).list;
        (*(*table).buckets.add(index)).last = &mut (*table).list;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_close(table: *mut hashtable_t) {
    if table.is_null() {
        return;
    }
    hashtable_clear(table);
    jsonp_free((*table).buckets.cast());
    (*table).buckets = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_set(
    table: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    let pair = find_pair(table, key, key_len);
    if !pair.is_null() {
        decref((*pair).value);
        (*pair).value = value;
        return 0;
    }
    let pair = jsonp_malloc(KEY_OFFSET + key_len + 1).cast::<PairHead>();
    if pair.is_null() {
        return -1;
    }
    ptr::write_bytes(pair.cast::<u8>(), 0, KEY_OFFSET);
    (*pair).value = value;
    (*pair).key_len = key_len;
    ptr::copy_nonoverlapping(key, pair_key(pair), key_len);
    *pair_key(pair).add(key_len) = 0;
    list_init(&mut (*pair).list);
    list_init(&mut (*pair).ordered_list);
    list_insert(&mut (*table).list, &mut (*pair).list);
    list_insert(&mut (*table).ordered_list, &mut (*pair).ordered_list);
    (*table).size += 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_get(
    table: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let pair = find_pair(table, key, key_len);
    if pair.is_null() {
        ptr::null_mut()
    } else {
        (*pair).value.cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_del(
    table: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    let pair = find_pair(table, key, key_len);
    if pair.is_null() {
        return -1;
    }
    list_remove(&mut (*pair).list);
    list_remove(&mut (*pair).ordered_list);
    decref((*pair).value);
    jsonp_free(pair.cast());
    (*table).size -= 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_clear(table: *mut hashtable_t) {
    if table.is_null() {
        return;
    }
    let sentinel = &mut (*table).ordered_list as *mut hashtable_list;
    let mut node = (*sentinel).next;
    while node != sentinel {
        let next = (*node).next;
        let pair = pair_from_iter(node.cast());
        decref((*pair).value);
        jsonp_free(pair.cast());
        node = next;
    }
    list_init(&mut (*table).list);
    list_init(&mut (*table).ordered_list);
    (*table).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter(table: *mut hashtable_t) -> *mut c_void {
    if table.is_null() {
        return ptr::null_mut();
    }
    hashtable_iter_next(
        table,
        (&mut (*table).ordered_list as *mut hashtable_list).cast(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_at(
    table: *mut hashtable_t,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void {
    let pair = find_pair(table, key, key_len);
    if pair.is_null() {
        ptr::null_mut()
    } else {
        (&mut (*pair).ordered_list as *mut hashtable_list).cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_next(
    table: *mut hashtable_t,
    iter: *mut c_void,
) -> *mut c_void {
    if table.is_null() || iter.is_null() {
        return ptr::null_mut();
    }
    let next = (*iter.cast::<hashtable_list>()).next;
    if ptr::eq(next, &(*table).ordered_list) {
        ptr::null_mut()
    } else {
        next.cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key(iter: *mut c_void) -> *mut c_void {
    if iter.is_null() {
        ptr::null_mut()
    } else {
        pair_key(pair_from_iter(iter)).cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_key_len(iter: *mut c_void) -> usize {
    if iter.is_null() {
        0
    } else {
        (*pair_from_iter(iter)).key_len
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_value(iter: *mut c_void) -> *mut c_void {
    if iter.is_null() {
        ptr::null_mut()
    } else {
        (*pair_from_iter(iter)).value.cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashtable_iter_set(iter: *mut c_void, value: *mut json_t) {
    if !iter.is_null() {
        let pair = pair_from_iter(iter);
        decref((*pair).value);
        (*pair).value = value;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_seed(seed: usize) {
    let seed = if seed != 0 {
        seed as u32
    } else {
        let mut value = 0u32;
        let fd = libc::open(c"/dev/urandom".as_ptr(), libc::O_RDONLY);
        if fd >= 0 {
            libc::read(
                fd,
                (&mut value as *mut u32).cast(),
                std::mem::size_of::<u32>(),
            );
            libc::close(fd);
        }
        if value == 0 {
            value = (libc::time(ptr::null_mut()) as u32) ^ (libc::getpid() as u32);
        }
        value
    };
    hashtable_seed = seed;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_init(buffer: *mut strbuffer_t) -> c_int {
    (*buffer).size = 16;
    (*buffer).length = 0;
    (*buffer).value = jsonp_malloc(16).cast();
    if (*buffer).value.is_null() {
        return -1;
    }
    *(*buffer).value = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_close(buffer: *mut strbuffer_t) {
    jsonp_free((*buffer).value.cast());
    (*buffer).value = ptr::null_mut();
    (*buffer).length = 0;
    (*buffer).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_clear(buffer: *mut strbuffer_t) {
    (*buffer).length = 0;
    *(*buffer).value = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_value(buffer: *const strbuffer_t) -> *const c_char {
    (*buffer).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_steal_value(buffer: *mut strbuffer_t) -> *mut c_char {
    let value = (*buffer).value;
    (*buffer).value = ptr::null_mut();
    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_append_byte(buffer: *mut strbuffer_t, byte: c_char) -> c_int {
    strbuffer_append_bytes(buffer, &byte, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_append_bytes(
    buffer: *mut strbuffer_t,
    data: *const c_char,
    size: usize,
) -> c_int {
    if size >= (*buffer).size.wrapping_sub((*buffer).length) {
        let Some(required) = (*buffer)
            .length
            .checked_add(size)
            .and_then(|v| v.checked_add(1))
        else {
            return -1;
        };
        let Some(double) = (*buffer).size.checked_mul(2) else {
            return -1;
        };
        let new_size = double.max(required);
        let value: *mut c_char =
            jsonp_realloc((*buffer).value.cast(), (*buffer).size, new_size).cast();
        if value.is_null() {
            return -1;
        }
        (*buffer).value = value;
        (*buffer).size = new_size;
    }
    ptr::copy_nonoverlapping(data, (*buffer).value.add((*buffer).length), size);
    (*buffer).length += size;
    *(*buffer).value.add((*buffer).length) = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_pop(buffer: *mut strbuffer_t) -> c_char {
    if (*buffer).length == 0 {
        return 0;
    }
    (*buffer).length -= 1;
    let byte = *(*buffer).value.add((*buffer).length);
    *(*buffer).value.add((*buffer).length) = 0;
    byte
}

pub fn utf8_valid(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_encode(
    codepoint: i32,
    buffer: *mut c_char,
    size: *mut usize,
) -> c_int {
    let Ok(codepoint) = u32::try_from(codepoint) else {
        return -1;
    };
    let Some(character) = char::from_u32(codepoint) else {
        return -1;
    };
    let mut encoded_buffer = [0; 4];
    let encoded = character.encode_utf8(&mut encoded_buffer).as_bytes();
    ptr::copy_nonoverlapping(encoded.as_ptr(), buffer.cast(), encoded.len());
    *size = encoded.len();
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn utf8_check_first(byte: c_char) -> usize {
    match byte as u8 {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_check_full(
    buffer: *const c_char,
    size: usize,
    codepoint: *mut i32,
) -> usize {
    if !(2..=4).contains(&size) {
        return 0;
    }
    let bytes = std::slice::from_raw_parts(buffer.cast::<u8>(), size);
    let Ok(text) = std::str::from_utf8(bytes) else {
        return 0;
    };
    let Some(character) = text.chars().next() else {
        return 0;
    };
    if character.len_utf8() != size {
        return 0;
    }
    if !codepoint.is_null() {
        *codepoint = character as i32;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_iterate(
    buffer: *const c_char,
    size: usize,
    codepoint: *mut i32,
) -> *const c_char {
    if size == 0 {
        return buffer;
    }
    let count = utf8_check_first(*buffer);
    if count == 0 || count > size {
        return ptr::null();
    }
    if count == 1 {
        if !codepoint.is_null() {
            *codepoint = *buffer as u8 as i32;
        }
    } else if utf8_check_full(buffer, count, codepoint) == 0 {
        return ptr::null();
    }
    buffer.add(count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_check_string(string: *const c_char, length: usize) -> c_int {
    utf8_valid(std::slice::from_raw_parts(string.cast(), length)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_loop_check(
    parents: *mut hashtable_t,
    json: *const json_t,
    key: *mut c_char,
    key_size: usize,
    key_len_out: *mut usize,
) -> c_int {
    let length = libc::snprintf(key, key_size, c"%p".as_ptr(), json.cast::<c_void>()) as usize;
    if !key_len_out.is_null() {
        *key_len_out = length;
    }
    if !hashtable_get(parents, key, length).is_null() {
        -1
    } else {
        hashtable_set(parents, key, length, json_null())
    }
}
