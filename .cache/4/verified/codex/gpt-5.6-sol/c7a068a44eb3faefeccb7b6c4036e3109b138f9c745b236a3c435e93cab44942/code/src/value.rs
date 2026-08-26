use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use crate::types::*;
use std::collections::HashSet;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

static TRUE_VALUE: json_t = json_t {
    type_: JSON_TRUE,
    refcount: std::sync::atomic::AtomicUsize::new(usize::MAX),
};
static FALSE_VALUE: json_t = json_t {
    type_: JSON_FALSE,
    refcount: std::sync::atomic::AtomicUsize::new(usize::MAX),
};
static NULL_VALUE: json_t = json_t {
    type_: JSON_NULL,
    refcount: std::sync::atomic::AtomicUsize::new(usize::MAX),
};

unsafe fn alloc_value<T>(value: T) -> *mut T {
    let out = jsonp_malloc(std::mem::size_of::<T>()).cast::<T>();
    if !out.is_null() {
        ptr::write(out, value);
    }
    out
}

unsafe fn free_value<T>(value: *mut T) {
    ptr::drop_in_place(value);
    jsonp_free(value.cast());
}

#[inline]
pub unsafe fn incref(value: *mut json_t) -> *mut json_t {
    if let Some(value) = value.as_ref() {
        if value.refcount.load(Ordering::Relaxed) != usize::MAX {
            value.refcount.fetch_add(1, Ordering::AcqRel);
        }
    }
    value
}

#[inline]
pub unsafe fn decref(value: *mut json_t) {
    if let Some(value_ref) = value.as_ref() {
        let count = value_ref.refcount.load(Ordering::Relaxed);
        if count != usize::MAX && value_ref.refcount.fetch_sub(1, Ordering::AcqRel) == 1 {
            json_delete(value);
        }
    }
}

unsafe fn key_bytes(entry: &Entry) -> &[u8] {
    &entry.key[std::mem::size_of::<usize>()..][..entry.key_len]
}

unsafe fn key_ptr(entry: &Entry) -> *const c_char {
    entry.key.as_ptr().add(std::mem::size_of::<usize>()).cast()
}

unsafe fn make_entry(key: &[u8], value: *mut json_t) -> Option<Box<Entry>> {
    let allocation = jsonp_malloc(56 + key.len() + 1);
    if allocation.is_null() {
        return None;
    }
    let mut entry = Box::new(Entry {
        key: vec![0; std::mem::size_of::<usize>() + key.len() + 1].into_boxed_slice(),
        key_len: key.len(),
        value,
        allocation,
    });
    let entry_ptr = (&mut *entry as *mut Entry) as usize;
    entry.key[..std::mem::size_of::<usize>()].copy_from_slice(&entry_ptr.to_ne_bytes());
    entry.key[std::mem::size_of::<usize>()..std::mem::size_of::<usize>() + key.len()]
        .copy_from_slice(key);
    Some(entry)
}

unsafe fn find_entry(object: &JsonObject, key: &[u8]) -> Option<usize> {
    object
        .entries
        .iter()
        .position(|entry| key_bytes(entry) == key)
}

unsafe fn setn_new(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
    check: bool,
) -> c_int {
    if value.is_null() {
        return -1;
    }
    if key.is_null()
        || !is_type(json, JSON_OBJECT)
        || json == value
        || (check && !crate::private::utf8_valid(std::slice::from_raw_parts(key.cast(), key_len)))
    {
        decref(value);
        return -1;
    }
    let bytes = std::slice::from_raw_parts(key.cast::<u8>(), key_len);
    let object = object_mut(json);
    if let Some(index) = find_entry(object, bytes) {
        let old = object.entries[index].value;
        object.entries[index].value = value;
        decref(old);
    } else {
        let Some(entry) = make_entry(bytes, value) else {
            decref(value);
            return -1;
        };
        object.entries.push(entry);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object() -> *mut json_t {
    let object = alloc_value(JsonObject {
        json: base(JSON_OBJECT),
        entries: Vec::new(),
        buckets: ptr::null_mut(),
    });
    if object.is_null() {
        return ptr::null_mut();
    }
    (*object).buckets = jsonp_malloc(8 * 2 * std::mem::size_of::<usize>());
    if (*object).buckets.is_null() {
        free_value(object);
        return ptr::null_mut();
    }
    object.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_size(json: *const json_t) -> usize {
    if is_type(json, JSON_OBJECT) {
        object_ref(json).entries.len()
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_get(json: *const json_t, key: *const c_char) -> *mut json_t {
    if key.is_null() {
        return ptr::null_mut();
    }
    json_object_getn(json, key, CStr::from_ptr(key).to_bytes().len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_getn(
    json: *const json_t,
    key: *const c_char,
    key_len: usize,
) -> *mut json_t {
    if key.is_null() || !is_type(json, JSON_OBJECT) {
        return ptr::null_mut();
    }
    let object = object_ref(json);
    let bytes = std::slice::from_raw_parts(key.cast::<u8>(), key_len);
    find_entry(object, bytes)
        .map(|index| object.entries[index].value)
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new_nocheck(
    json: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    if key.is_null() {
        decref(value);
        return -1;
    }
    setn_new(
        json,
        key,
        CStr::from_ptr(key).to_bytes().len(),
        value,
        false,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new_nocheck(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    setn_new(json, key, key_len, value, false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new(
    json: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    if key.is_null() {
        decref(value);
        return -1;
    }
    setn_new(json, key, CStr::from_ptr(key).to_bytes().len(), value, true)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    setn_new(json, key, key_len, value, true)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_del(json: *mut json_t, key: *const c_char) -> c_int {
    if key.is_null() {
        return -1;
    }
    json_object_deln(json, key, CStr::from_ptr(key).to_bytes().len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_deln(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    if key.is_null() || !is_type(json, JSON_OBJECT) {
        return -1;
    }
    let bytes = std::slice::from_raw_parts(key.cast::<u8>(), key_len);
    let object = object_mut(json);
    let Some(index) = find_entry(object, bytes) else {
        return -1;
    };
    let entry = object.entries.remove(index);
    decref(entry.value);
    jsonp_free(entry.allocation);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_clear(json: *mut json_t) -> c_int {
    if !is_type(json, JSON_OBJECT) {
        return -1;
    }
    let object = object_mut(json);
    for entry in object.entries.drain(..) {
        decref(entry.value);
        jsonp_free(entry.allocation);
    }
    0
}

unsafe fn update(object: *mut json_t, other: *mut json_t, mode: u8) -> c_int {
    if !is_type(object, JSON_OBJECT) || !is_type(other, JSON_OBJECT) {
        return -1;
    }
    for entry in &object_ref(other).entries {
        let key = key_ptr(entry);
        let exists = !json_object_getn(object, key, entry.key_len).is_null();
        if (mode == 0 || (mode == 1 && exists) || (mode == 2 && !exists))
            && setn_new(object, key, entry.key_len, incref(entry.value), false) != 0
        {
            return -1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update(object: *mut json_t, other: *mut json_t) -> c_int {
    update(object, other, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_existing(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    update(object, other, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_missing(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    update(object, other, 2)
}

unsafe fn update_recursive(
    object: *mut json_t,
    other: *mut json_t,
    seen: &mut HashSet<usize>,
) -> c_int {
    if !is_type(object, JSON_OBJECT) || !is_type(other, JSON_OBJECT) || !seen.insert(other as usize)
    {
        return -1;
    }
    for entry in &object_ref(other).entries {
        let key = key_ptr(entry);
        let current = json_object_getn(object, key, entry.key_len);
        let result = if is_type(current, JSON_OBJECT) && is_type(entry.value, JSON_OBJECT) {
            update_recursive(current, entry.value, seen)
        } else {
            setn_new(object, key, entry.key_len, incref(entry.value), false)
        };
        if result != 0 {
            seen.remove(&(other as usize));
            return -1;
        }
    }
    seen.remove(&(other as usize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_recursive(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    update_recursive(object, other, &mut HashSet::new())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_object_update_recursive(
    object: *mut json_t,
    other: *mut json_t,
    _parents: *mut c_void,
) -> c_int {
    json_object_update_recursive(object, other)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter(json: *mut json_t) -> *mut c_void {
    if !is_type(json, JSON_OBJECT) {
        return ptr::null_mut();
    }
    object_mut(json)
        .entries
        .first_mut()
        .map(|entry| (&mut **entry as *mut Entry).cast())
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_at(json: *mut json_t, key: *const c_char) -> *mut c_void {
    if key.is_null() || !is_type(json, JSON_OBJECT) {
        return ptr::null_mut();
    }
    let bytes = CStr::from_ptr(key).to_bytes();
    let object = object_mut(json);
    find_entry(object, bytes)
        .map(|index| (&mut *object.entries[index] as *mut Entry).cast())
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_next(
    json: *mut json_t,
    iter: *mut c_void,
) -> *mut c_void {
    if iter.is_null() || !is_type(json, JSON_OBJECT) {
        return ptr::null_mut();
    }
    let object = object_mut(json);
    let Some(index) = object
        .entries
        .iter()
        .position(|entry| (&**entry as *const Entry).cast::<c_void>() == iter)
    else {
        return ptr::null_mut();
    };
    object
        .entries
        .get_mut(index + 1)
        .map(|entry| (&mut **entry as *mut Entry).cast())
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_key(iter: *mut c_void) -> *const c_char {
    iter.cast::<Entry>()
        .as_ref()
        .map(|entry| key_ptr(entry))
        .unwrap_or(ptr::null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_key_len(iter: *mut c_void) -> usize {
    iter.cast::<Entry>()
        .as_ref()
        .map(|entry| entry.key_len)
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_value(iter: *mut c_void) -> *mut json_t {
    iter.cast::<Entry>()
        .as_ref()
        .map(|entry| entry.value)
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_set_new(
    json: *mut json_t,
    iter: *mut c_void,
    value: *mut json_t,
) -> c_int {
    if !is_type(json, JSON_OBJECT) || iter.is_null() || value.is_null() {
        decref(value);
        return -1;
    }
    let entry = &mut *iter.cast::<Entry>();
    let old = entry.value;
    entry.value = value;
    decref(old);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_key_to_iter(key: *const c_char) -> *mut c_void {
    if key.is_null() {
        return ptr::null_mut();
    }
    let prefix = key.cast::<u8>().sub(std::mem::size_of::<usize>());
    ptr::read_unaligned(prefix.cast::<usize>()) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array() -> *mut json_t {
    let array = alloc_value(JsonArray {
        json: base(JSON_ARRAY),
        values: Vec::new(),
        table: ptr::null_mut(),
        table_capacity: 8,
    });
    if array.is_null() {
        return ptr::null_mut();
    }
    (*array).table = jsonp_malloc(8 * std::mem::size_of::<*mut json_t>());
    if (*array).table.is_null() {
        free_value(array);
        return ptr::null_mut();
    }
    array.cast()
}

unsafe fn array_grow(array: *mut JsonArray, amount: usize) -> bool {
    if (*array).values.len().saturating_add(amount) <= (*array).table_capacity {
        return true;
    }
    let new_capacity = (*array)
        .table_capacity
        .saturating_add(amount)
        .max((*array).table_capacity.saturating_mul(2));
    let table = jsonp_realloc(
        (*array).table,
        (*array).table_capacity * std::mem::size_of::<*mut json_t>(),
        new_capacity * std::mem::size_of::<*mut json_t>(),
    );
    if table.is_null() {
        return false;
    }
    (*array).table = table;
    (*array).table_capacity = new_capacity;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_size(json: *const json_t) -> usize {
    if is_type(json, JSON_ARRAY) {
        array_ref(json).values.len()
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_get(json: *const json_t, index: usize) -> *mut json_t {
    if !is_type(json, JSON_ARRAY) {
        return ptr::null_mut();
    }
    array_ref(json)
        .values
        .get(index)
        .copied()
        .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_set_new(
    json: *mut json_t,
    index: usize,
    value: *mut json_t,
) -> c_int {
    if value.is_null() {
        return -1;
    }
    if !is_type(json, JSON_ARRAY) || json == value || index >= array_ref(json).values.len() {
        decref(value);
        return -1;
    }
    let old = std::mem::replace(&mut array_mut(json).values[index], value);
    decref(old);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_append_new(json: *mut json_t, value: *mut json_t) -> c_int {
    if value.is_null() {
        return -1;
    }
    if !is_type(json, JSON_ARRAY) || json == value {
        decref(value);
        return -1;
    }
    if !array_grow(json.cast(), 1) {
        decref(value);
        return -1;
    }
    array_mut(json).values.push(value);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_insert_new(
    json: *mut json_t,
    index: usize,
    value: *mut json_t,
) -> c_int {
    if value.is_null() {
        return -1;
    }
    if !is_type(json, JSON_ARRAY) || json == value || index > array_ref(json).values.len() {
        decref(value);
        return -1;
    }
    if !array_grow(json.cast(), 1) {
        decref(value);
        return -1;
    }
    array_mut(json).values.insert(index, value);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_remove(json: *mut json_t, index: usize) -> c_int {
    if !is_type(json, JSON_ARRAY) || index >= array_ref(json).values.len() {
        return -1;
    }
    let old = array_mut(json).values.remove(index);
    decref(old);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_clear(json: *mut json_t) -> c_int {
    if !is_type(json, JSON_ARRAY) {
        return -1;
    }
    for value in array_mut(json).values.drain(..) {
        decref(value);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_extend(json: *mut json_t, other: *mut json_t) -> c_int {
    if !is_type(json, JSON_ARRAY) || !is_type(other, JSON_ARRAY) {
        return -1;
    }
    let values = array_ref(other).values.clone();
    if !array_grow(json.cast(), values.len()) {
        return -1;
    }
    for value in values {
        array_mut(json).values.push(incref(value));
    }
    0
}

unsafe fn make_string(value: *const c_char, len: usize) -> *mut json_t {
    if value.is_null() {
        return ptr::null_mut();
    }
    let mut bytes = std::slice::from_raw_parts(value.cast::<u8>(), len).to_vec();
    bytes.push(0);
    let allocation = jsonp_malloc(len + 1);
    if allocation.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(value, allocation.cast(), len);
    *allocation.cast::<c_char>().add(len) = 0;
    let string = alloc_value(JsonString {
        json: base(JSON_STRING),
        value: bytes,
        allocation,
    });
    if string.is_null() {
        jsonp_free(allocation);
        ptr::null_mut()
    } else {
        string.cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_nocheck(value: *const c_char) -> *mut json_t {
    if value.is_null() {
        ptr::null_mut()
    } else {
        make_string(value, CStr::from_ptr(value).to_bytes().len())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn_nocheck(value: *const c_char, len: usize) -> *mut json_t {
    make_string(value, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_stringn_nocheck_own(
    value: *const c_char,
    len: usize,
) -> *mut json_t {
    let result = make_string(value, len);
    jsonp_free(value.cast_mut().cast());
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string(value: *const c_char) -> *mut json_t {
    if value.is_null() {
        ptr::null_mut()
    } else {
        json_stringn(value, CStr::from_ptr(value).to_bytes().len())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn(value: *const c_char, len: usize) -> *mut json_t {
    if value.is_null() || !crate::private::utf8_valid(std::slice::from_raw_parts(value.cast(), len))
    {
        ptr::null_mut()
    } else {
        make_string(value, len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_value(json: *const json_t) -> *const c_char {
    if is_type(json, JSON_STRING) {
        string_ref(json).value.as_ptr().cast()
    } else {
        ptr::null()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_length(json: *const json_t) -> usize {
    if is_type(json, JSON_STRING) {
        string_ref(json).value.len() - 1
    } else {
        0
    }
}

unsafe fn string_setn(json: *mut json_t, value: *const c_char, len: usize, check: bool) -> c_int {
    if value.is_null()
        || !is_type(json, JSON_STRING)
        || (check && !crate::private::utf8_valid(std::slice::from_raw_parts(value.cast(), len)))
    {
        return -1;
    }
    let mut bytes = std::slice::from_raw_parts(value.cast::<u8>(), len).to_vec();
    bytes.push(0);
    let allocation = jsonp_malloc(len + 1);
    if allocation.is_null() {
        return -1;
    }
    ptr::copy_nonoverlapping(value, allocation.cast(), len);
    *allocation.cast::<c_char>().add(len) = 0;
    let string = string_mut(json);
    jsonp_free(string.allocation);
    string.allocation = allocation;
    string.value = bytes;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set_nocheck(json: *mut json_t, value: *const c_char) -> c_int {
    if value.is_null() {
        -1
    } else {
        string_setn(json, value, CStr::from_ptr(value).to_bytes().len(), false)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn_nocheck(
    json: *mut json_t,
    value: *const c_char,
    len: usize,
) -> c_int {
    string_setn(json, value, len, false)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set(json: *mut json_t, value: *const c_char) -> c_int {
    if value.is_null() {
        -1
    } else {
        string_setn(json, value, CStr::from_ptr(value).to_bytes().len(), true)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn(
    json: *mut json_t,
    value: *const c_char,
    len: usize,
) -> c_int {
    string_setn(json, value, len, true)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer(value: i64) -> *mut json_t {
    alloc_value(JsonInteger {
        json: base(JSON_INTEGER),
        value,
    })
    .cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_value(json: *const json_t) -> i64 {
    if is_type(json, JSON_INTEGER) {
        integer_ref(json).value
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_set(json: *mut json_t, value: i64) -> c_int {
    if !is_type(json, JSON_INTEGER) {
        -1
    } else {
        integer_mut(json).value = value;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real(value: f64) -> *mut json_t {
    if !value.is_finite() {
        return ptr::null_mut();
    }
    alloc_value(JsonReal {
        json: base(JSON_REAL),
        value,
    })
    .cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_value(json: *const json_t) -> f64 {
    if is_type(json, JSON_REAL) {
        real_ref(json).value
    } else {
        0.0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_set(json: *mut json_t, value: f64) -> c_int {
    if !is_type(json, JSON_REAL) || !value.is_finite() {
        -1
    } else {
        real_mut(json).value = value;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_number_value(json: *const json_t) -> f64 {
    match type_of(json) {
        Some(JSON_INTEGER) => json_integer_value(json) as f64,
        Some(JSON_REAL) => json_real_value(json),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn json_true() -> *mut json_t {
    (&TRUE_VALUE as *const json_t).cast_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn json_false() -> *mut json_t {
    (&FALSE_VALUE as *const json_t).cast_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn json_null() -> *mut json_t {
    (&NULL_VALUE as *const json_t).cast_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_delete(json: *mut json_t) {
    match type_of(json) {
        Some(JSON_OBJECT) => {
            json_object_clear(json);
            jsonp_free(object_mut(json).buckets);
            free_value(json.cast::<JsonObject>());
        }
        Some(JSON_ARRAY) => {
            json_array_clear(json);
            jsonp_free(array_mut(json).table);
            free_value(json.cast::<JsonArray>());
        }
        Some(JSON_STRING) => {
            jsonp_free(string_mut(json).allocation);
            free_value(json.cast::<JsonString>());
        }
        Some(JSON_INTEGER) => free_value(json.cast::<JsonInteger>()),
        Some(JSON_REAL) => free_value(json.cast::<JsonReal>()),
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_equal(left: *const json_t, right: *const json_t) -> c_int {
    if left.is_null() || right.is_null() || type_of(left) != type_of(right) {
        return 0;
    }
    if left == right {
        return 1;
    }
    let equal = match type_of(left).unwrap() {
        JSON_OBJECT => {
            let a = object_ref(left);
            let b = object_ref(right);
            a.entries.len() == b.entries.len()
                && a.entries.iter().all(|entry| {
                    let value = json_object_getn(right, key_ptr(entry), entry.key_len);
                    json_equal(entry.value, value) != 0
                })
        }
        JSON_ARRAY => {
            let a = array_ref(left);
            let b = array_ref(right);
            a.values.len() == b.values.len()
                && a.values
                    .iter()
                    .zip(&b.values)
                    .all(|(&a, &b)| json_equal(a, b) != 0)
        }
        JSON_STRING => string_ref(left).value == string_ref(right).value,
        JSON_INTEGER => integer_ref(left).value == integer_ref(right).value,
        JSON_REAL => real_ref(left).value == real_ref(right).value,
        _ => false,
    };
    equal as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_copy(json: *mut json_t) -> *mut json_t {
    match type_of(json) {
        Some(JSON_OBJECT) => {
            let result = json_object();
            for entry in &object_ref(json).entries {
                setn_new(
                    result,
                    key_ptr(entry),
                    entry.key_len,
                    incref(entry.value),
                    false,
                );
            }
            result
        }
        Some(JSON_ARRAY) => {
            let result = json_array();
            for &value in &array_ref(json).values {
                json_array_append_new(result, incref(value));
            }
            result
        }
        Some(JSON_STRING) => {
            json_stringn_nocheck(json_string_value(json), json_string_length(json))
        }
        Some(JSON_INTEGER) => json_integer(json_integer_value(json)),
        Some(JSON_REAL) => json_real(json_real_value(json)),
        Some(JSON_TRUE | JSON_FALSE | JSON_NULL) => json,
        _ => ptr::null_mut(),
    }
}

unsafe fn deep_copy(json: *const json_t, seen: &mut HashSet<usize>) -> *mut json_t {
    match type_of(json) {
        Some(JSON_OBJECT) => {
            if !seen.insert(json as usize) {
                return ptr::null_mut();
            }
            let result = json_object();
            for entry in &object_ref(json).entries {
                let child = deep_copy(entry.value, seen);
                if child.is_null()
                    || setn_new(result, key_ptr(entry), entry.key_len, child, false) != 0
                {
                    decref(result);
                    seen.remove(&(json as usize));
                    return ptr::null_mut();
                }
            }
            seen.remove(&(json as usize));
            result
        }
        Some(JSON_ARRAY) => {
            if !seen.insert(json as usize) {
                return ptr::null_mut();
            }
            let result = json_array();
            for &value in &array_ref(json).values {
                let child = deep_copy(value, seen);
                if child.is_null() || json_array_append_new(result, child) != 0 {
                    decref(result);
                    seen.remove(&(json as usize));
                    return ptr::null_mut();
                }
            }
            seen.remove(&(json as usize));
            result
        }
        _ => json_copy(json.cast_mut()),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_deep_copy(json: *const json_t) -> *mut json_t {
    deep_copy(json, &mut HashSet::new())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_deep_copy(json: *const json_t, _parents: *mut c_void) -> *mut json_t {
    json_deep_copy(json)
}
