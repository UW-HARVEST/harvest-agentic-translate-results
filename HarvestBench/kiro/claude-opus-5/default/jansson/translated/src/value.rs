//! Translation of `src/value.c`.

use crate::hashtable::*;
use crate::memory::*;
use crate::types::*;
use crate::utf::utf8_check_string;
use crate::varargs::VaListTag;
use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::Ordering;

#[inline]
unsafe fn json_init(json: *mut JsonT, type_: c_int) {
    (*json).type_ = type_;
    (*json).refcount = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_loop_check(
    parents: *mut HashtableT,
    json: *const JsonT,
    key: *mut c_char,
    key_size: usize,
    key_len_out: *mut usize,
) -> c_int {
    let key_len = snprintf(key, key_size, b"%p\0".as_ptr() as *const c_char, json) as usize;

    if !key_len_out.is_null() {
        *key_len_out = key_len;
    }

    if !hashtable_get(parents, key, key_len).is_null() {
        return -1;
    }

    hashtable_set(parents, key, key_len, json_null())
}

/*** object ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object() -> *mut JsonT {
    let object = jsonp_malloc(core::mem::size_of::<JsonObjectT>()) as *mut JsonObjectT;
    if object.is_null() {
        return core::ptr::null_mut();
    }

    if crate::hashtable_seed::hashtable_seed.load(Ordering::Relaxed) == 0 {
        /* Autoseed */
        crate::hashtable_seed::json_object_seed(0);
    }

    json_init(&mut (*object).json, JSON_OBJECT);

    if hashtable_init(&mut (*object).hashtable) != 0 {
        jsonp_free(object as *mut c_void);
        return core::ptr::null_mut();
    }

    &mut (*object).json
}

unsafe fn json_delete_object(object: *mut JsonObjectT) {
    hashtable_close(&mut (*object).hashtable);
    jsonp_free(object as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_size(json: *const JsonT) -> usize {
    let object: *mut JsonObjectT;

    if !json_is_object(json) {
        return 0;
    }

    object = json_to_object(json);
    (*object).hashtable.size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_get(json: *const JsonT, key: *const c_char) -> *mut JsonT {
    if key.is_null() {
        return core::ptr::null_mut();
    }

    json_object_getn(json, key, strlen(key))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_getn(
    json: *const JsonT,
    key: *const c_char,
    key_len: usize,
) -> *mut JsonT {
    let object: *mut JsonObjectT;

    if key.is_null() || !json_is_object(json) {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_get(&mut (*object).hashtable, key, key_len) as *mut JsonT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new_nocheck(
    json: *mut JsonT,
    key: *const c_char,
    value: *mut JsonT,
) -> c_int {
    if key.is_null() {
        json_decref(value);
        return -1;
    }
    json_object_setn_new_nocheck(json, key, strlen(key), value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new_nocheck(
    json: *mut JsonT,
    key: *const c_char,
    key_len: usize,
    value: *mut JsonT,
) -> c_int {
    let object: *mut JsonObjectT;

    if value.is_null() {
        return -1;
    }

    if key.is_null() || !json_is_object(json) || json == value {
        json_decref(value);
        return -1;
    }
    object = json_to_object(json);

    if hashtable_set(&mut (*object).hashtable, key, key_len, value) != 0 {
        json_decref(value);
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new(
    json: *mut JsonT,
    key: *const c_char,
    value: *mut JsonT,
) -> c_int {
    if key.is_null() {
        json_decref(value);
        return -1;
    }

    json_object_setn_new(json, key, strlen(key), value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new(
    json: *mut JsonT,
    key: *const c_char,
    key_len: usize,
    value: *mut JsonT,
) -> c_int {
    if key.is_null() || utf8_check_string(key, key_len) == 0 {
        json_decref(value);
        return -1;
    }

    json_object_setn_new_nocheck(json, key, key_len, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_del(json: *mut JsonT, key: *const c_char) -> c_int {
    if key.is_null() {
        return -1;
    }

    json_object_deln(json, key, strlen(key))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_deln(
    json: *mut JsonT,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    let object: *mut JsonObjectT;

    if key.is_null() || !json_is_object(json) {
        return -1;
    }

    object = json_to_object(json);
    hashtable_del(&mut (*object).hashtable, key, key_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_clear(json: *mut JsonT) -> c_int {
    let object: *mut JsonObjectT;

    if !json_is_object(json) {
        return -1;
    }

    object = json_to_object(json);
    hashtable_clear(&mut (*object).hashtable);

    0
}

/// Reproduces the `json_object_keylen_foreach(object, key, key_len, value)`
/// iteration. `$body` runs with `$key`, `$klen` and `$val` bound; the iterator
/// is only advanced afterwards, exactly like the `for` loop's third clause.
macro_rules! keylen_foreach {
    ($object:expr, $key:ident, $klen:ident, $val:ident, $body:block) => {{
        let obj = $object;
        let mut iter = json_object_iter(obj);
        while !iter.is_null() {
            let $key = json_object_iter_key(iter);
            let $klen = json_object_iter_key_len(iter);
            let $val = json_object_iter_value(iter);
            $body
            iter = json_object_iter_next(obj, iter);
        }
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update(object: *mut JsonT, other: *mut JsonT) -> c_int {
    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    keylen_foreach!(other, key, key_len, value, {
        if json_object_setn_new_nocheck(object, key, key_len, json_incref(value)) != 0 {
            return -1;
        }
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_existing(
    object: *mut JsonT,
    other: *mut JsonT,
) -> c_int {
    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    keylen_foreach!(other, key, key_len, value, {
        if !json_object_getn(object, key, key_len).is_null() {
            json_object_setn_new_nocheck(object, key, key_len, json_incref(value));
        }
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_missing(
    object: *mut JsonT,
    other: *mut JsonT,
) -> c_int {
    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    keylen_foreach!(other, key, key_len, value, {
        if json_object_getn(object, key, key_len).is_null() {
            json_object_setn_new_nocheck(object, key, key_len, json_incref(value));
        }
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_object_update_recursive(
    object: *mut JsonT,
    other: *mut JsonT,
    parents: *mut HashtableT,
) -> c_int {
    let mut loop_key = [0 as c_char; LOOP_KEY_LEN];
    let mut res: c_int = 0;
    let mut loop_key_len: usize = 0;

    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    if jsonp_loop_check(
        parents,
        other,
        loop_key.as_mut_ptr(),
        LOOP_KEY_LEN,
        &mut loop_key_len,
    ) != 0
    {
        return -1;
    }

    let mut iter = json_object_iter(other);
    while !iter.is_null() {
        let key = json_object_iter_key(iter);
        let key_len = json_object_iter_key_len(iter);
        let value = json_object_iter_value(iter);

        let v = json_object_getn(object, key, key_len);

        if json_is_object(v) && json_is_object(value) {
            if do_object_update_recursive(v, value, parents) != 0 {
                res = -1;
                break;
            }
        } else if json_object_setn_new_nocheck(object, key, key_len, json_incref(value)) != 0 {
            res = -1;
            break;
        }
        iter = json_object_iter_next(other, iter);
    }

    hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_recursive(
    object: *mut JsonT,
    other: *mut JsonT,
) -> c_int {
    let res: c_int;
    let mut parents_set: HashtableT = core::mem::zeroed();

    if hashtable_init(&mut parents_set) != 0 {
        return -1;
    }
    res = do_object_update_recursive(object, other, &mut parents_set);
    hashtable_close(&mut parents_set);

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter(json: *mut JsonT) -> *mut c_void {
    let object: *mut JsonObjectT;

    if !json_is_object(json) {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_iter(&mut (*object).hashtable)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_at(json: *mut JsonT, key: *const c_char) -> *mut c_void {
    let object: *mut JsonObjectT;

    if key.is_null() || !json_is_object(json) {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_iter_at(&mut (*object).hashtable, key, strlen(key))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_next(
    json: *mut JsonT,
    iter: *mut c_void,
) -> *mut c_void {
    let object: *mut JsonObjectT;

    if !json_is_object(json) || iter.is_null() {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_iter_next(&mut (*object).hashtable, iter)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_key(iter: *mut c_void) -> *const c_char {
    if iter.is_null() {
        return core::ptr::null();
    }

    hashtable_iter_key(iter) as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_key_len(iter: *mut c_void) -> usize {
    if iter.is_null() {
        return 0;
    }

    hashtable_iter_key_len(iter)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_value(iter: *mut c_void) -> *mut JsonT {
    if iter.is_null() {
        return core::ptr::null_mut();
    }

    hashtable_iter_value(iter) as *mut JsonT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_set_new(
    json: *mut JsonT,
    iter: *mut c_void,
    value: *mut JsonT,
) -> c_int {
    if !json_is_object(json) || iter.is_null() || value.is_null() {
        json_decref(value);
        return -1;
    }

    hashtable_iter_set(iter, value);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_key_to_iter(key: *const c_char) -> *mut c_void {
    if key.is_null() {
        return core::ptr::null_mut();
    }

    /* hashtable_key_to_iter(key) */
    &mut (*key_to_pair(key)).ordered_list as *mut _ as *mut c_void
}

unsafe fn json_object_equal(object1: *const JsonT, object2: *const JsonT) -> c_int {
    if json_object_size(object1) != json_object_size(object2) {
        return 0;
    }

    keylen_foreach!(object1 as *mut JsonT, key, key_len, value1, {
        let value2 = json_object_getn(object2, key, key_len);

        if json_equal(value1, value2) == 0 {
            return 0;
        }
    });

    1
}

unsafe fn json_object_copy(object: *mut JsonT) -> *mut JsonT {
    let result: *mut JsonT;

    result = json_object();
    if result.is_null() {
        return core::ptr::null_mut();
    }

    keylen_foreach!(object, key, key_len, value, {
        json_object_setn_new_nocheck(result, key, key_len, json_incref(value));
    });

    result
}

unsafe fn json_object_deep_copy(object: *const JsonT, parents: *mut HashtableT) -> *mut JsonT {
    let mut result: *mut JsonT;
    let mut iter: *mut c_void;
    let mut loop_key = [0 as c_char; LOOP_KEY_LEN];
    let mut loop_key_len: usize = 0;

    if jsonp_loop_check(
        parents,
        object,
        loop_key.as_mut_ptr(),
        LOOP_KEY_LEN,
        &mut loop_key_len,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = json_object();
    if !result.is_null() {
        /* Cannot use json_object_foreach because object has to be cast non-const */
        iter = json_object_iter(object as *mut JsonT);
        while !iter.is_null() {
            let key = json_object_iter_key(iter);
            let key_len = json_object_iter_key_len(iter);
            let value = json_object_iter_value(iter);

            if json_object_setn_new_nocheck(result, key, key_len, do_deep_copy(value, parents)) != 0
            {
                json_decref(result);
                result = core::ptr::null_mut();
                break;
            }
            iter = json_object_iter_next(object as *mut JsonT, iter);
        }
    }

    hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

    result
}

/*** array ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array() -> *mut JsonT {
    let array = jsonp_malloc(core::mem::size_of::<JsonArrayT>()) as *mut JsonArrayT;
    if array.is_null() {
        return core::ptr::null_mut();
    }
    json_init(&mut (*array).json, JSON_ARRAY);

    (*array).entries = 0;
    (*array).size = 8;

    (*array).table =
        jsonp_malloc((*array).size * core::mem::size_of::<*mut JsonT>()) as *mut *mut JsonT;
    if (*array).table.is_null() {
        jsonp_free(array as *mut c_void);
        return core::ptr::null_mut();
    }

    &mut (*array).json
}

unsafe fn json_delete_array(array: *mut JsonArrayT) {
    for i in 0..(*array).entries {
        json_decref(*(*array).table.add(i));
    }

    jsonp_free((*array).table as *mut c_void);
    jsonp_free(array as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_size(json: *const JsonT) -> usize {
    if !json_is_array(json) {
        return 0;
    }

    (*json_to_array(json)).entries
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_get(json: *const JsonT, index: usize) -> *mut JsonT {
    let array: *mut JsonArrayT;
    if !json_is_array(json) {
        return core::ptr::null_mut();
    }
    array = json_to_array(json);

    if index >= (*array).entries {
        return core::ptr::null_mut();
    }

    *(*array).table.add(index)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_set_new(
    json: *mut JsonT,
    index: usize,
    value: *mut JsonT,
) -> c_int {
    let array: *mut JsonArrayT;

    if value.is_null() {
        return -1;
    }

    if !json_is_array(json) || json == value {
        json_decref(value);
        return -1;
    }
    array = json_to_array(json);

    if index >= (*array).entries {
        json_decref(value);
        return -1;
    }

    json_decref(*(*array).table.add(index));
    *(*array).table.add(index) = value;

    0
}

unsafe fn array_move(array: *mut JsonArrayT, dest: usize, src: usize, count: usize) {
    memmove(
        (*array).table.add(dest) as *mut c_void,
        (*array).table.add(src) as *const c_void,
        count * core::mem::size_of::<*mut JsonT>(),
    );
}

unsafe fn array_copy(
    dest: *mut *mut JsonT,
    dpos: usize,
    src: *mut *mut JsonT,
    spos: usize,
    count: usize,
) {
    memcpy(
        dest.add(dpos) as *mut c_void,
        src.add(spos) as *const c_void,
        count * core::mem::size_of::<*mut JsonT>(),
    );
}

unsafe fn json_array_grow(array: *mut JsonArrayT, amount: usize) -> *mut *mut JsonT {
    let new_size: usize;
    let old_table: *mut *mut JsonT;
    let new_table: *mut *mut JsonT;

    if (*array).entries + amount <= (*array).size {
        return (*array).table;
    }

    old_table = (*array).table;

    let a = (*array).size + amount;
    let b = (*array).size * 2;
    new_size = if a > b { a } else { b };
    new_table = jsonp_realloc(
        old_table as *mut c_void,
        (*array).size * core::mem::size_of::<*mut JsonT>(),
        new_size * core::mem::size_of::<*mut JsonT>(),
    ) as *mut *mut JsonT;
    if new_table.is_null() {
        return core::ptr::null_mut();
    }

    (*array).size = new_size;
    (*array).table = new_table;

    (*array).table
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_append_new(json: *mut JsonT, value: *mut JsonT) -> c_int {
    let array: *mut JsonArrayT;

    if value.is_null() {
        return -1;
    }

    if !json_is_array(json) || json == value {
        json_decref(value);
        return -1;
    }
    array = json_to_array(json);

    if json_array_grow(array, 1).is_null() {
        json_decref(value);
        return -1;
    }

    *(*array).table.add((*array).entries) = value;
    (*array).entries += 1;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_insert_new(
    json: *mut JsonT,
    index: usize,
    value: *mut JsonT,
) -> c_int {
    let array: *mut JsonArrayT;

    if value.is_null() {
        return -1;
    }

    if !json_is_array(json) || json == value {
        json_decref(value);
        return -1;
    }
    array = json_to_array(json);

    if index > (*array).entries {
        json_decref(value);
        return -1;
    }

    if json_array_grow(array, 1).is_null() {
        json_decref(value);
        return -1;
    }
    if index != (*array).entries {
        array_move(array, index + 1, index, (*array).entries - index);
    }

    *(*array).table.add(index) = value;
    (*array).entries += 1;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_remove(json: *mut JsonT, index: usize) -> c_int {
    let array: *mut JsonArrayT;

    if !json_is_array(json) {
        return -1;
    }
    array = json_to_array(json);

    if index >= (*array).entries {
        return -1;
    }

    json_decref(*(*array).table.add(index));

    /* If we're removing the last element, nothing has to be moved */
    if index < (*array).entries - 1 {
        array_move(array, index, index + 1, (*array).entries - index - 1);
    }

    (*array).entries -= 1;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_clear(json: *mut JsonT) -> c_int {
    let array: *mut JsonArrayT;

    if !json_is_array(json) {
        return -1;
    }
    array = json_to_array(json);

    for i in 0..(*array).entries {
        json_decref(*(*array).table.add(i));
    }

    (*array).entries = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_extend(json: *mut JsonT, other_json: *mut JsonT) -> c_int {
    let array: *mut JsonArrayT;
    let other: *mut JsonArrayT;

    if !json_is_array(json) || !json_is_array(other_json) {
        return -1;
    }
    array = json_to_array(json);
    other = json_to_array(other_json);

    if json_array_grow(array, (*other).entries).is_null() {
        return -1;
    }

    for i in 0..(*other).entries {
        json_incref(*(*other).table.add(i));
    }

    array_copy(
        (*array).table,
        (*array).entries,
        (*other).table,
        0,
        (*other).entries,
    );

    (*array).entries += (*other).entries;
    0
}

unsafe fn json_array_equal(array1: *const JsonT, array2: *const JsonT) -> c_int {
    let size: usize;

    size = json_array_size(array1);
    if size != json_array_size(array2) {
        return 0;
    }

    for i in 0..size {
        let value1 = json_array_get(array1, i);
        let value2 = json_array_get(array2, i);

        if json_equal(value1, value2) == 0 {
            return 0;
        }
    }

    1
}

unsafe fn json_array_copy(array: *mut JsonT) -> *mut JsonT {
    let result: *mut JsonT;

    result = json_array();
    if result.is_null() {
        return core::ptr::null_mut();
    }

    let mut i = 0;
    while i < json_array_size(array) {
        json_array_append_new(result, json_incref(json_array_get(array, i)));
        i += 1;
    }

    result
}

unsafe fn json_array_deep_copy(array: *const JsonT, parents: *mut HashtableT) -> *mut JsonT {
    let mut result: *mut JsonT;
    let mut loop_key = [0 as c_char; LOOP_KEY_LEN];
    let mut loop_key_len: usize = 0;

    if jsonp_loop_check(
        parents,
        array,
        loop_key.as_mut_ptr(),
        LOOP_KEY_LEN,
        &mut loop_key_len,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = json_array();
    if !result.is_null() {
        let mut i = 0;
        while i < json_array_size(array) {
            if json_array_append_new(result, do_deep_copy(json_array_get(array, i), parents)) != 0 {
                json_decref(result);
                result = core::ptr::null_mut();
                break;
            }
            i += 1;
        }
    }

    hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

    result
}

/*** string ***/

unsafe fn string_create(value: *const c_char, len: usize, own: c_int) -> *mut JsonT {
    let v: *mut c_char;
    let string: *mut JsonStringT;

    if value.is_null() {
        return core::ptr::null_mut();
    }

    if own != 0 {
        v = value as *mut c_char;
    } else {
        v = jsonp_strndup(value, len);
        if v.is_null() {
            return core::ptr::null_mut();
        }
    }

    string = jsonp_malloc(core::mem::size_of::<JsonStringT>()) as *mut JsonStringT;
    if string.is_null() {
        jsonp_free(v as *mut c_void);
        return core::ptr::null_mut();
    }
    json_init(&mut (*string).json, JSON_STRING);
    (*string).value = v;
    (*string).length = len;

    &mut (*string).json
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_nocheck(value: *const c_char) -> *mut JsonT {
    if value.is_null() {
        return core::ptr::null_mut();
    }

    string_create(value, strlen(value), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn_nocheck(value: *const c_char, len: usize) -> *mut JsonT {
    string_create(value, len, 0)
}

/* this is private; "steal" is not a public API concept */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_stringn_nocheck_own(value: *const c_char, len: usize) -> *mut JsonT {
    string_create(value, len, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string(value: *const c_char) -> *mut JsonT {
    if value.is_null() {
        return core::ptr::null_mut();
    }

    json_stringn(value, strlen(value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn(value: *const c_char, len: usize) -> *mut JsonT {
    if value.is_null() || utf8_check_string(value, len) == 0 {
        return core::ptr::null_mut();
    }

    json_stringn_nocheck(value, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_value(json: *const JsonT) -> *const c_char {
    if !json_is_string(json) {
        return core::ptr::null();
    }

    (*json_to_string(json)).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_length(json: *const JsonT) -> usize {
    if !json_is_string(json) {
        return 0;
    }

    (*json_to_string(json)).length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set_nocheck(
    json: *mut JsonT,
    value: *const c_char,
) -> c_int {
    if value.is_null() {
        return -1;
    }

    json_string_setn_nocheck(json, value, strlen(value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn_nocheck(
    json: *mut JsonT,
    value: *const c_char,
    len: usize,
) -> c_int {
    let dup: *mut c_char;
    let string: *mut JsonStringT;

    if !json_is_string(json) || value.is_null() {
        return -1;
    }

    dup = jsonp_strndup(value, len);
    if dup.is_null() {
        return -1;
    }

    string = json_to_string(json);
    jsonp_free((*string).value as *mut c_void);
    (*string).value = dup;
    (*string).length = len;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set(json: *mut JsonT, value: *const c_char) -> c_int {
    if value.is_null() {
        return -1;
    }

    json_string_setn(json, value, strlen(value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn(
    json: *mut JsonT,
    value: *const c_char,
    len: usize,
) -> c_int {
    if value.is_null() || utf8_check_string(value, len) == 0 {
        return -1;
    }

    json_string_setn_nocheck(json, value, len)
}

unsafe fn json_delete_string(string: *mut JsonStringT) {
    jsonp_free((*string).value as *mut c_void);
    jsonp_free(string as *mut c_void);
}

unsafe fn json_string_equal(string1: *const JsonT, string2: *const JsonT) -> c_int {
    let s1 = json_to_string(string1);
    let s2 = json_to_string(string2);
    ((*s1).length == (*s2).length
        && memcmp(
            (*s1).value as *const c_void,
            (*s2).value as *const c_void,
            (*s1).length,
        ) == 0) as c_int
}

unsafe fn json_string_copy(string: *const JsonT) -> *mut JsonT {
    let s = json_to_string(string);
    json_stringn_nocheck((*s).value, (*s).length)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vsprintf(fmt: *const c_char, ap: *mut VaListTag) -> *mut JsonT {
    let mut json: *mut JsonT = core::ptr::null_mut();
    let length: c_int;
    let buf: *mut c_char;
    let mut aq: VaListTag = crate::varargs::va_copy(ap);

    length = vsnprintf(core::ptr::null_mut(), 0, fmt, ap);
    if length < 0 {
        return json;
    }
    if length == 0 {
        json = json_string(b"\0".as_ptr() as *const c_char);
        return json;
    }

    buf = jsonp_malloc(length as usize + 1) as *mut c_char;
    if buf.is_null() {
        return json;
    }

    vsnprintf(buf, length as usize + 1, fmt, &mut aq);
    if utf8_check_string(buf, length as usize) == 0 {
        jsonp_free(buf as *mut c_void);
        return json;
    }

    json = jsonp_stringn_nocheck_own(buf, length as usize);

    json
}

/*** integer ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer(value: JsonIntT) -> *mut JsonT {
    let integer = jsonp_malloc(core::mem::size_of::<JsonIntegerT>()) as *mut JsonIntegerT;
    if integer.is_null() {
        return core::ptr::null_mut();
    }
    json_init(&mut (*integer).json, JSON_INTEGER);

    (*integer).value = value;
    &mut (*integer).json
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_value(json: *const JsonT) -> JsonIntT {
    if !json_is_integer(json) {
        return 0;
    }

    (*json_to_integer(json)).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_set(json: *mut JsonT, value: JsonIntT) -> c_int {
    if !json_is_integer(json) {
        return -1;
    }

    (*json_to_integer(json)).value = value;

    0
}

unsafe fn json_delete_integer(integer: *mut JsonIntegerT) {
    jsonp_free(integer as *mut c_void);
}

unsafe fn json_integer_equal(integer1: *const JsonT, integer2: *const JsonT) -> c_int {
    (json_integer_value(integer1) == json_integer_value(integer2)) as c_int
}

unsafe fn json_integer_copy(integer: *const JsonT) -> *mut JsonT {
    json_integer(json_integer_value(integer))
}

/*** real ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real(value: f64) -> *mut JsonT {
    let real: *mut JsonRealT;

    if value.is_nan() || value.is_infinite() {
        return core::ptr::null_mut();
    }

    real = jsonp_malloc(core::mem::size_of::<JsonRealT>()) as *mut JsonRealT;
    if real.is_null() {
        return core::ptr::null_mut();
    }
    json_init(&mut (*real).json, JSON_REAL);

    (*real).value = value;
    &mut (*real).json
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_value(json: *const JsonT) -> f64 {
    if !json_is_real(json) {
        return 0.0;
    }

    (*json_to_real(json)).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_set(json: *mut JsonT, value: f64) -> c_int {
    if !json_is_real(json) || value.is_nan() || value.is_infinite() {
        return -1;
    }

    (*json_to_real(json)).value = value;

    0
}

unsafe fn json_delete_real(real: *mut JsonRealT) {
    jsonp_free(real as *mut c_void);
}

unsafe fn json_real_equal(real1: *const JsonT, real2: *const JsonT) -> c_int {
    (json_real_value(real1) == json_real_value(real2)) as c_int
}

unsafe fn json_real_copy(real: *const JsonT) -> *mut JsonT {
    json_real(json_real_value(real))
}

/*** number ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_number_value(json: *const JsonT) -> f64 {
    if json_is_integer(json) {
        json_integer_value(json) as f64
    } else if json_is_real(json) {
        json_real_value(json)
    } else {
        0.0
    }
}

/*** simple values ***/

static mut THE_TRUE: JsonT = JsonT {
    type_: JSON_TRUE,
    refcount: usize::MAX,
};
static mut THE_FALSE: JsonT = JsonT {
    type_: JSON_FALSE,
    refcount: usize::MAX,
};
static mut THE_NULL: JsonT = JsonT {
    type_: JSON_NULL,
    refcount: usize::MAX,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_true() -> *mut JsonT {
    core::ptr::addr_of_mut!(THE_TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_false() -> *mut JsonT {
    core::ptr::addr_of_mut!(THE_FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_null() -> *mut JsonT {
    core::ptr::addr_of_mut!(THE_NULL)
}

/*** deletion ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_delete(json: *mut JsonT) {
    if json.is_null() {
        return;
    }

    match json_typeof(json) {
        JSON_OBJECT => json_delete_object(json_to_object(json)),
        JSON_ARRAY => json_delete_array(json_to_array(json)),
        JSON_STRING => json_delete_string(json_to_string(json)),
        JSON_INTEGER => json_delete_integer(json_to_integer(json)),
        JSON_REAL => json_delete_real(json_to_real(json)),
        _ => (),
    }

    /* json_delete is not called for true, false or null */
}

/*** equality ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_equal(json1: *const JsonT, json2: *const JsonT) -> c_int {
    if json1.is_null() || json2.is_null() {
        return 0;
    }

    if json_typeof(json1) != json_typeof(json2) {
        return 0;
    }

    /* this covers true, false and null as they are singletons */
    if json1 == json2 {
        return 1;
    }

    match json_typeof(json1) {
        JSON_OBJECT => json_object_equal(json1, json2),
        JSON_ARRAY => json_array_equal(json1, json2),
        JSON_STRING => json_string_equal(json1, json2),
        JSON_INTEGER => json_integer_equal(json1, json2),
        JSON_REAL => json_real_equal(json1, json2),
        _ => 0,
    }
}

/*** copying ***/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_copy(json: *mut JsonT) -> *mut JsonT {
    if json.is_null() {
        return core::ptr::null_mut();
    }

    match json_typeof(json) {
        JSON_OBJECT => json_object_copy(json),
        JSON_ARRAY => json_array_copy(json),
        JSON_STRING => json_string_copy(json),
        JSON_INTEGER => json_integer_copy(json),
        JSON_REAL => json_real_copy(json),
        JSON_TRUE | JSON_FALSE | JSON_NULL => json,
        _ => core::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_deep_copy(json: *const JsonT) -> *mut JsonT {
    let res: *mut JsonT;
    let mut parents_set: HashtableT = core::mem::zeroed();

    if hashtable_init(&mut parents_set) != 0 {
        return core::ptr::null_mut();
    }
    res = do_deep_copy(json, &mut parents_set);
    hashtable_close(&mut parents_set);

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_deep_copy(
    json: *const JsonT,
    parents: *mut HashtableT,
) -> *mut JsonT {
    if json.is_null() {
        return core::ptr::null_mut();
    }

    match json_typeof(json) {
        JSON_OBJECT => json_object_deep_copy(json, parents),
        JSON_ARRAY => json_array_deep_copy(json, parents),
        /* for the rest of the types, deep copying doesn't differ from
        shallow copying */
        JSON_STRING => json_string_copy(json),
        JSON_INTEGER => json_integer_copy(json),
        JSON_REAL => json_real_copy(json),
        JSON_TRUE | JSON_FALSE | JSON_NULL => json as *mut JsonT,
        _ => core::ptr::null_mut(),
    }
}
