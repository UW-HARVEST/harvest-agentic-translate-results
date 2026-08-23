//! Translation of `src/value.c`.

use crate::hashtable::*;
use crate::memory::{jsonp_free, jsonp_malloc, jsonp_strndup};
use crate::types::*;
use crate::utf::utf8_check_string;
use core::ffi::{c_char, c_int, c_void};

/* ------------------------------------------------- header inline wrappers */

#[inline]
pub unsafe fn json_object_set_inline(
    object: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    json_object_set_new(object, key, json_incref(value))
}

#[inline]
pub unsafe fn json_object_setn_inline(
    object: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    json_object_setn_new(object, key, key_len, json_incref(value))
}

#[inline]
pub unsafe fn json_object_set_nocheck_inline(
    object: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    json_object_set_new_nocheck(object, key, json_incref(value))
}

#[inline]
pub unsafe fn json_object_setn_nocheck_inline(
    object: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    json_object_setn_new_nocheck(object, key, key_len, json_incref(value))
}

#[inline]
pub unsafe fn json_array_append_inline(array: *mut json_t, value: *mut json_t) -> c_int {
    json_array_append_new(array, json_incref(value))
}

/* ------------------------------------------------------ container_of casts */

#[inline]
unsafe fn json_to_object(json: *const json_t) -> *mut json_object_t {
    json as *mut json_object_t
}

#[inline]
unsafe fn json_to_array(json: *const json_t) -> *mut json_array_t {
    json as *mut json_array_t
}

#[inline]
unsafe fn json_to_string(json: *const json_t) -> *mut json_string_t {
    json as *mut json_string_t
}

#[inline]
unsafe fn json_to_real(json: *const json_t) -> *mut json_real_t {
    json as *mut json_real_t
}

#[inline]
unsafe fn json_to_integer(json: *const json_t) -> *mut json_integer_t {
    json as *mut json_integer_t
}

#[inline]
unsafe fn json_init(json: *mut json_t, type_: c_int) {
    (*json).type_ = type_;
    (*json).refcount = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_loop_check(
    parents: *mut hashtable_t,
    json: *const json_t,
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

/* ----------------------------------------------------------------- object */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object() -> *mut json_t {
    let object = jsonp_malloc(core::mem::size_of::<json_object_t>()) as *mut json_object_t;
    if object.is_null() {
        return core::ptr::null_mut();
    }

    if core::ptr::read_volatile(core::ptr::addr_of!(crate::hashtable_seed::hashtable_seed)) == 0 {
        /* Autoseed */
        crate::hashtable_seed::json_object_seed(0);
    }

    json_init(core::ptr::addr_of_mut!((*object).json), JSON_OBJECT);

    if hashtable_init(core::ptr::addr_of_mut!((*object).hashtable)) != 0 {
        jsonp_free(object as *mut c_void);
        return core::ptr::null_mut();
    }

    core::ptr::addr_of_mut!((*object).json)
}

unsafe fn json_delete_object(object: *mut json_object_t) {
    hashtable_close(core::ptr::addr_of_mut!((*object).hashtable));
    jsonp_free(object as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_size(json: *const json_t) -> usize {
    let object: *mut json_object_t;

    if !json_is_object(json) {
        return 0;
    }

    object = json_to_object(json);
    (*object).hashtable.size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_get(json: *const json_t, key: *const c_char) -> *mut json_t {
    if key.is_null() {
        return core::ptr::null_mut();
    }

    json_object_getn(json, key, strlen(key))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_getn(
    json: *const json_t,
    key: *const c_char,
    key_len: usize,
) -> *mut json_t {
    let object: *mut json_object_t;

    if key.is_null() || !json_is_object(json) {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_get(core::ptr::addr_of_mut!((*object).hashtable), key, key_len) as *mut json_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new_nocheck(
    json: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    if key.is_null() {
        json_decref(value);
        return -1;
    }
    json_object_setn_new_nocheck(json, key, strlen(key), value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new_nocheck(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    let object: *mut json_object_t;

    if value.is_null() {
        return -1;
    }

    if key.is_null() || !json_is_object(json) || json == value {
        json_decref(value);
        return -1;
    }
    object = json_to_object(json);

    if hashtable_set(
        core::ptr::addr_of_mut!((*object).hashtable),
        key,
        key_len,
        value,
    ) != 0
    {
        json_decref(value);
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new(
    json: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    if key.is_null() {
        json_decref(value);
        return -1;
    }

    json_object_setn_new(json, key, strlen(key), value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    if key.is_null() || utf8_check_string(key, key_len) == 0 {
        json_decref(value);
        return -1;
    }

    json_object_setn_new_nocheck(json, key, key_len, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_del(json: *mut json_t, key: *const c_char) -> c_int {
    if key.is_null() {
        return -1;
    }

    json_object_deln(json, key, strlen(key))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_deln(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    let object: *mut json_object_t;

    if key.is_null() || !json_is_object(json) {
        return -1;
    }

    object = json_to_object(json);
    hashtable_del(core::ptr::addr_of_mut!((*object).hashtable), key, key_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_clear(json: *mut json_t) -> c_int {
    let object: *mut json_object_t;

    if !json_is_object(json) {
        return -1;
    }

    object = json_to_object(json);
    hashtable_clear(core::ptr::addr_of_mut!((*object).hashtable));

    0
}

/// Faithful expansion of the `json_object_keylen_foreach` macro.
///
/// The closure returns `true` to continue iterating and `false` to `break`.
unsafe fn keylen_foreach(
    object: *mut json_t,
    mut body: impl FnMut(*const c_char, usize, *mut json_t) -> bool,
) {
    let mut key = json_object_iter_key(json_object_iter(object));
    let mut key_len = json_object_iter_key_len(json_object_key_to_iter(key));
    loop {
        if key.is_null() {
            break;
        }
        let value = json_object_iter_value(json_object_key_to_iter(key));
        if value.is_null() {
            break;
        }

        if !body(key, key_len, value) {
            break;
        }

        key = json_object_iter_key(json_object_iter_next(object, json_object_key_to_iter(key)));
        key_len = json_object_iter_key_len(json_object_key_to_iter(key));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update(object: *mut json_t, other: *mut json_t) -> c_int {
    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    let mut failed = false;
    keylen_foreach(other, |key, key_len, value| {
        if json_object_setn_nocheck_inline(object, key, key_len, value) != 0 {
            failed = true;
            return false;
        }
        true
    });
    if failed {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_existing(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    keylen_foreach(other, |key, key_len, value| {
        if !json_object_getn(object, key, key_len).is_null() {
            json_object_setn_nocheck_inline(object, key, key_len, value);
        }
        true
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_missing(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    if !json_is_object(object) || !json_is_object(other) {
        return -1;
    }

    keylen_foreach(other, |key, key_len, value| {
        if json_object_getn(object, key, key_len).is_null() {
            json_object_setn_nocheck_inline(object, key, key_len, value);
        }
        true
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_object_update_recursive(
    object: *mut json_t,
    other: *mut json_t,
    parents: *mut hashtable_t,
) -> c_int {
    let mut loop_key = [0i8; LOOP_KEY_LEN];
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

    keylen_foreach(other, |key, key_len, value| {
        let v = json_object_getn(object, key, key_len);

        if json_is_object(v) && json_is_object(value) {
            if do_object_update_recursive(v, value, parents) != 0 {
                res = -1;
                return false;
            }
        } else if json_object_setn_nocheck_inline(object, key, key_len, value) != 0 {
            res = -1;
            return false;
        }
        true
    });

    hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_recursive(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    let res: c_int;
    let mut parents_set = core::mem::MaybeUninit::<hashtable_t>::uninit();

    if hashtable_init(parents_set.as_mut_ptr()) != 0 {
        return -1;
    }
    res = do_object_update_recursive(object, other, parents_set.as_mut_ptr());
    hashtable_close(parents_set.as_mut_ptr());

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter(json: *mut json_t) -> *mut c_void {
    let object: *mut json_object_t;

    if !json_is_object(json) {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_iter(core::ptr::addr_of_mut!((*object).hashtable))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_at(
    json: *mut json_t,
    key: *const c_char,
) -> *mut c_void {
    let object: *mut json_object_t;

    if key.is_null() || !json_is_object(json) {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_iter_at(
        core::ptr::addr_of_mut!((*object).hashtable),
        key,
        strlen(key),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_next(
    json: *mut json_t,
    iter: *mut c_void,
) -> *mut c_void {
    let object: *mut json_object_t;

    if !json_is_object(json) || iter.is_null() {
        return core::ptr::null_mut();
    }

    object = json_to_object(json);
    hashtable_iter_next(core::ptr::addr_of_mut!((*object).hashtable), iter)
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
pub unsafe extern "C" fn json_object_iter_value(iter: *mut c_void) -> *mut json_t {
    if iter.is_null() {
        return core::ptr::null_mut();
    }

    hashtable_iter_value(iter) as *mut json_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_set_new(
    json: *mut json_t,
    iter: *mut c_void,
    value: *mut json_t,
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

    hashtable_key_to_iter(key)
}

unsafe fn json_object_equal(object1: *const json_t, object2: *const json_t) -> c_int {
    if json_object_size(object1) != json_object_size(object2) {
        return 0;
    }

    let mut equal = 1;
    keylen_foreach(object1 as *mut json_t, |key, key_len, value1| {
        let value2 = json_object_getn(object2, key, key_len);

        if json_equal(value1, value2) == 0 {
            equal = 0;
            return false;
        }
        true
    });

    equal
}

unsafe fn json_object_copy(object: *mut json_t) -> *mut json_t {
    let result: *mut json_t;

    result = json_object();
    if result.is_null() {
        return core::ptr::null_mut();
    }

    keylen_foreach(object, |key, key_len, value| {
        json_object_setn_nocheck_inline(result, key, key_len, value);
        true
    });

    result
}

unsafe fn json_object_deep_copy(object: *const json_t, parents: *mut hashtable_t) -> *mut json_t {
    let mut result: *mut json_t;
    let mut iter: *mut c_void;
    let mut loop_key = [0i8; LOOP_KEY_LEN];
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
        /* Cannot use json_object_foreach because object has to be cast
        non-const */
        iter = json_object_iter(object as *mut json_t);
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
            iter = json_object_iter_next(object as *mut json_t, iter);
        }
    }

    hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

    result
}

/* ------------------------------------------------------------------ array */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array() -> *mut json_t {
    let array = jsonp_malloc(core::mem::size_of::<json_array_t>()) as *mut json_array_t;
    if array.is_null() {
        return core::ptr::null_mut();
    }
    json_init(core::ptr::addr_of_mut!((*array).json), JSON_ARRAY);

    (*array).entries = 0;
    (*array).size = 8;

    (*array).table =
        jsonp_malloc((*array).size * core::mem::size_of::<*mut json_t>()) as *mut *mut json_t;
    if (*array).table.is_null() {
        jsonp_free(array as *mut c_void);
        return core::ptr::null_mut();
    }

    core::ptr::addr_of_mut!((*array).json)
}

unsafe fn json_delete_array(array: *mut json_array_t) {
    let mut i: usize = 0;

    while i < (*array).entries {
        json_decref(*(*array).table.add(i));
        i += 1;
    }

    jsonp_free((*array).table as *mut c_void);
    jsonp_free(array as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_size(json: *const json_t) -> usize {
    if !json_is_array(json) {
        return 0;
    }

    (*json_to_array(json)).entries
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_get(json: *const json_t, index: usize) -> *mut json_t {
    let array: *mut json_array_t;
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
    json: *mut json_t,
    index: usize,
    value: *mut json_t,
) -> c_int {
    let array: *mut json_array_t;

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

unsafe fn array_move(array: *mut json_array_t, dest: usize, src: usize, count: usize) {
    memmove(
        (*array).table.add(dest) as *mut c_void,
        (*array).table.add(src) as *const c_void,
        count * core::mem::size_of::<*mut json_t>(),
    );
}

unsafe fn array_copy(
    dest: *mut *mut json_t,
    dpos: usize,
    src: *mut *mut json_t,
    spos: usize,
    count: usize,
) {
    memcpy(
        dest.add(dpos) as *mut c_void,
        src.add(spos) as *const c_void,
        count * core::mem::size_of::<*mut json_t>(),
    );
}

unsafe fn json_array_grow(array: *mut json_array_t, amount: usize) -> *mut *mut json_t {
    let new_size: usize;
    let old_table: *mut *mut json_t;
    let new_table: *mut *mut json_t;

    if (*array).entries + amount <= (*array).size {
        return (*array).table;
    }

    old_table = (*array).table;

    new_size = max_usize((*array).size + amount, (*array).size * 2);
    new_table = crate::memory::jsonp_realloc(
        old_table as *mut c_void,
        (*array).size * core::mem::size_of::<*mut json_t>(),
        new_size * core::mem::size_of::<*mut json_t>(),
    ) as *mut *mut json_t;
    if new_table.is_null() {
        return core::ptr::null_mut();
    }

    (*array).size = new_size;
    (*array).table = new_table;

    (*array).table
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_append_new(json: *mut json_t, value: *mut json_t) -> c_int {
    let array: *mut json_array_t;

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
    json: *mut json_t,
    index: usize,
    value: *mut json_t,
) -> c_int {
    let array: *mut json_array_t;

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
pub unsafe extern "C" fn json_array_remove(json: *mut json_t, index: usize) -> c_int {
    let array: *mut json_array_t;

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
pub unsafe extern "C" fn json_array_clear(json: *mut json_t) -> c_int {
    let array: *mut json_array_t;
    let mut i: usize = 0;

    if !json_is_array(json) {
        return -1;
    }
    array = json_to_array(json);

    while i < (*array).entries {
        json_decref(*(*array).table.add(i));
        i += 1;
    }

    (*array).entries = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_extend(json: *mut json_t, other_json: *mut json_t) -> c_int {
    let array: *mut json_array_t;
    let other: *mut json_array_t;
    let mut i: usize;

    if !json_is_array(json) || !json_is_array(other_json) {
        return -1;
    }
    array = json_to_array(json);
    other = json_to_array(other_json);

    if json_array_grow(array, (*other).entries).is_null() {
        return -1;
    }

    i = 0;
    while i < (*other).entries {
        json_incref(*(*other).table.add(i));
        i += 1;
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

unsafe fn json_array_equal(array1: *const json_t, array2: *const json_t) -> c_int {
    let mut i: usize;
    let size: usize;

    size = json_array_size(array1);
    if size != json_array_size(array2) {
        return 0;
    }

    i = 0;
    while i < size {
        let value1 = json_array_get(array1, i);
        let value2 = json_array_get(array2, i);

        if json_equal(value1, value2) == 0 {
            return 0;
        }
        i += 1;
    }

    1
}

unsafe fn json_array_copy(array: *mut json_t) -> *mut json_t {
    let result: *mut json_t;
    let mut i: usize;

    result = json_array();
    if result.is_null() {
        return core::ptr::null_mut();
    }

    i = 0;
    while i < json_array_size(array) {
        json_array_append_inline(result, json_array_get(array, i));
        i += 1;
    }

    result
}

unsafe fn json_array_deep_copy(array: *const json_t, parents: *mut hashtable_t) -> *mut json_t {
    let mut result: *mut json_t;
    let mut i: usize;
    let mut loop_key = [0i8; LOOP_KEY_LEN];
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
        i = 0;
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

/* ----------------------------------------------------------------- string */

unsafe fn string_create(value: *const c_char, len: usize, own: c_int) -> *mut json_t {
    let v: *mut c_char;
    let string: *mut json_string_t;

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

    string = jsonp_malloc(core::mem::size_of::<json_string_t>()) as *mut json_string_t;
    if string.is_null() {
        jsonp_free(v as *mut c_void);
        return core::ptr::null_mut();
    }
    json_init(core::ptr::addr_of_mut!((*string).json), JSON_STRING);
    (*string).value = v;
    (*string).length = len;

    core::ptr::addr_of_mut!((*string).json)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_nocheck(value: *const c_char) -> *mut json_t {
    if value.is_null() {
        return core::ptr::null_mut();
    }

    string_create(value, strlen(value), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn_nocheck(value: *const c_char, len: usize) -> *mut json_t {
    string_create(value, len, 0)
}

/* this is private; "steal" is not a public API concept */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_stringn_nocheck_own(
    value: *const c_char,
    len: usize,
) -> *mut json_t {
    string_create(value, len, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string(value: *const c_char) -> *mut json_t {
    if value.is_null() {
        return core::ptr::null_mut();
    }

    json_stringn(value, strlen(value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn(value: *const c_char, len: usize) -> *mut json_t {
    if value.is_null() || utf8_check_string(value, len) == 0 {
        return core::ptr::null_mut();
    }

    json_stringn_nocheck(value, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_value(json: *const json_t) -> *const c_char {
    if !json_is_string(json) {
        return core::ptr::null();
    }

    (*json_to_string(json)).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_length(json: *const json_t) -> usize {
    if !json_is_string(json) {
        return 0;
    }

    (*json_to_string(json)).length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set_nocheck(
    json: *mut json_t,
    value: *const c_char,
) -> c_int {
    if value.is_null() {
        return -1;
    }

    json_string_setn_nocheck(json, value, strlen(value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn_nocheck(
    json: *mut json_t,
    value: *const c_char,
    len: usize,
) -> c_int {
    let dup: *mut c_char;
    let string: *mut json_string_t;

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
pub unsafe extern "C" fn json_string_set(json: *mut json_t, value: *const c_char) -> c_int {
    if value.is_null() {
        return -1;
    }

    json_string_setn(json, value, strlen(value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn(
    json: *mut json_t,
    value: *const c_char,
    len: usize,
) -> c_int {
    if value.is_null() || utf8_check_string(value, len) == 0 {
        return -1;
    }

    json_string_setn_nocheck(json, value, len)
}

unsafe fn json_delete_string(string: *mut json_string_t) {
    jsonp_free((*string).value as *mut c_void);
    jsonp_free(string as *mut c_void);
}

unsafe fn json_string_equal(string1: *const json_t, string2: *const json_t) -> c_int {
    let s1 = json_to_string(string1);
    let s2 = json_to_string(string2);
    ((*s1).length == (*s2).length
        && memcmp(
            (*s1).value as *const c_void,
            (*s2).value as *const c_void,
            (*s1).length,
        ) == 0) as c_int
}

unsafe fn json_string_copy(string: *const json_t) -> *mut json_t {
    let s = json_to_string(string);
    json_stringn_nocheck((*s).value, (*s).length)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vsprintf(fmt: *const c_char, ap: va_list) -> *mut json_t {
    let mut json: *mut json_t = core::ptr::null_mut();
    let length: c_int;
    let buf: *mut c_char;

    /* va_copy(aq, ap) */
    let mut aq_storage: VaListTag = *ap;
    let aq: va_list = &mut aq_storage;

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

    vsnprintf(buf, length as usize + 1, fmt, aq);
    if utf8_check_string(buf, length as usize) == 0 {
        jsonp_free(buf as *mut c_void);
        return json;
    }

    json = jsonp_stringn_nocheck_own(buf, length as usize);

    json
}

/* ---------------------------------------------------------------- integer */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer(value: json_int_t) -> *mut json_t {
    let integer = jsonp_malloc(core::mem::size_of::<json_integer_t>()) as *mut json_integer_t;
    if integer.is_null() {
        return core::ptr::null_mut();
    }
    json_init(core::ptr::addr_of_mut!((*integer).json), JSON_INTEGER);

    (*integer).value = value;
    core::ptr::addr_of_mut!((*integer).json)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_value(json: *const json_t) -> json_int_t {
    if !json_is_integer(json) {
        return 0;
    }

    (*json_to_integer(json)).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_set(json: *mut json_t, value: json_int_t) -> c_int {
    if !json_is_integer(json) {
        return -1;
    }

    (*json_to_integer(json)).value = value;

    0
}

unsafe fn json_delete_integer(integer: *mut json_integer_t) {
    jsonp_free(integer as *mut c_void);
}

unsafe fn json_integer_equal(integer1: *const json_t, integer2: *const json_t) -> c_int {
    (json_integer_value(integer1) == json_integer_value(integer2)) as c_int
}

unsafe fn json_integer_copy(integer: *const json_t) -> *mut json_t {
    json_integer(json_integer_value(integer))
}

/* ------------------------------------------------------------------- real */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real(value: f64) -> *mut json_t {
    let real: *mut json_real_t;

    if value.is_nan() || value.is_infinite() {
        return core::ptr::null_mut();
    }

    real = jsonp_malloc(core::mem::size_of::<json_real_t>()) as *mut json_real_t;
    if real.is_null() {
        return core::ptr::null_mut();
    }
    json_init(core::ptr::addr_of_mut!((*real).json), JSON_REAL);

    (*real).value = value;
    core::ptr::addr_of_mut!((*real).json)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_value(json: *const json_t) -> f64 {
    if !json_is_real(json) {
        return 0.0;
    }

    (*json_to_real(json)).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_set(json: *mut json_t, value: f64) -> c_int {
    if !json_is_real(json) || value.is_nan() || value.is_infinite() {
        return -1;
    }

    (*json_to_real(json)).value = value;

    0
}

unsafe fn json_delete_real(real: *mut json_real_t) {
    jsonp_free(real as *mut c_void);
}

unsafe fn json_real_equal(real1: *const json_t, real2: *const json_t) -> c_int {
    (json_real_value(real1) == json_real_value(real2)) as c_int
}

unsafe fn json_real_copy(real: *const json_t) -> *mut json_t {
    json_real(json_real_value(real))
}

/* ----------------------------------------------------------------- number */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_number_value(json: *const json_t) -> f64 {
    if json_is_integer(json) {
        json_integer_value(json) as f64
    } else if json_is_real(json) {
        json_real_value(json)
    } else {
        0.0
    }
}

/* ---------------------------------------------------------- simple values */

static mut THE_TRUE: json_t = json_t {
    type_: JSON_TRUE,
    refcount: usize::MAX,
};
static mut THE_FALSE: json_t = json_t {
    type_: JSON_FALSE,
    refcount: usize::MAX,
};
static mut THE_NULL: json_t = json_t {
    type_: JSON_NULL,
    refcount: usize::MAX,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_true() -> *mut json_t {
    core::ptr::addr_of_mut!(THE_TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_false() -> *mut json_t {
    core::ptr::addr_of_mut!(THE_FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_null() -> *mut json_t {
    core::ptr::addr_of_mut!(THE_NULL)
}

/* --------------------------------------------------------------- deletion */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_delete(json: *mut json_t) {
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

/* --------------------------------------------------------------- equality */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_equal(json1: *const json_t, json2: *const json_t) -> c_int {
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

/* ---------------------------------------------------------------- copying */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_copy(json: *mut json_t) -> *mut json_t {
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
pub unsafe extern "C" fn json_deep_copy(json: *const json_t) -> *mut json_t {
    let res: *mut json_t;
    let mut parents_set = core::mem::MaybeUninit::<hashtable_t>::uninit();

    if hashtable_init(parents_set.as_mut_ptr()) != 0 {
        return core::ptr::null_mut();
    }
    res = do_deep_copy(json, parents_set.as_mut_ptr());
    hashtable_close(parents_set.as_mut_ptr());

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_deep_copy(
    json: *const json_t,
    parents: *mut hashtable_t,
) -> *mut json_t {
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
        JSON_TRUE | JSON_FALSE | JSON_NULL => json as *mut json_t,
        _ => core::ptr::null_mut(),
    }
}
