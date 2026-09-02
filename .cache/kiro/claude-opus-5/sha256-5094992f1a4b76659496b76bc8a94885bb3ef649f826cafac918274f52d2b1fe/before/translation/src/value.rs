//! Translation of `src/value.c`.

use crate::cffi;
use crate::hashtable::{
    hashtable_clear, hashtable_close, hashtable_del, hashtable_get, hashtable_init,
    hashtable_iter, hashtable_iter_at, hashtable_iter_key, hashtable_iter_key_len,
    hashtable_iter_next, hashtable_iter_set, hashtable_iter_value, hashtable_key_to_iter,
    hashtable_set, hashtable_t,
};
use crate::hashtable_seed::{hashtable_seed_value, json_object_seed};
use crate::jtypes::*;
use crate::memory::{jsonp_free, jsonp_malloc, jsonp_strndup};
use crate::utf::utf8_check_string;
use crate::valist::{VaList, VaListTag, va_copy};
use core::ffi::{c_char, c_int, c_void};
use core::ptr::{null, null_mut};

#[inline]
unsafe fn json_init(json: *mut json_t, type_: c_int) {
    unsafe {
        (*json).type_ = type_;
        (*json).refcount = 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_loop_check(
    parents: *mut hashtable_t,
    json: *const json_t,
    key: *mut c_char,
    key_size: usize,
    key_len_out: *mut usize,
) -> c_int {
    unsafe {
        let key_len = cffi::snprintf(key, key_size, c"%p".as_ptr(), json) as usize;

        if !key_len_out.is_null() {
            *key_len_out = key_len;
        }

        if !hashtable_get(parents, key, key_len).is_null() {
            return -1;
        }

        hashtable_set(parents, key, key_len, json_null())
    }
}

/* ------------------------------------------------------------------ */
/* object                                                             */
/* ------------------------------------------------------------------ */

/// Iterator helper reproducing `json_object_keylen_foreach`.
struct KeylenIter {
    object: *mut json_t,
    key: *const c_char,
    key_len: usize,
}

impl KeylenIter {
    unsafe fn new(object: *mut json_t) -> Self {
        unsafe {
            let key = json_object_iter_key(json_object_iter(object));
            let key_len = json_object_iter_key_len(json_object_key_to_iter(key));
            KeylenIter {
                object,
                key,
                key_len,
            }
        }
    }

    /// Returns `(key, key_len, value)` or `None` when the loop is finished.
    unsafe fn next(&mut self) -> Option<(*const c_char, usize, *mut json_t)> {
        unsafe {
            if self.key.is_null() {
                return None;
            }
            let value = json_object_iter_value(json_object_key_to_iter(self.key));
            if value.is_null() {
                return None;
            }
            Some((self.key, self.key_len, value))
        }
    }

    unsafe fn advance(&mut self) {
        unsafe {
            self.key = json_object_iter_key(json_object_iter_next(
                self.object,
                json_object_key_to_iter(self.key),
            ));
            self.key_len = json_object_iter_key_len(json_object_key_to_iter(self.key));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object() -> *mut json_t {
    unsafe {
        let object = jsonp_malloc(core::mem::size_of::<json_object_t>()) as *mut json_object_t;
        if object.is_null() {
            return null_mut();
        }

        if hashtable_seed_value() == 0 {
            /* Autoseed */
            json_object_seed(0);
        }

        json_init(core::ptr::addr_of_mut!((*object).json), JSON_OBJECT);

        if hashtable_init(core::ptr::addr_of_mut!((*object).hashtable)) != 0 {
            jsonp_free(object as *mut c_void);
            return null_mut();
        }

        core::ptr::addr_of_mut!((*object).json)
    }
}

unsafe fn json_delete_object(object: *mut json_object_t) {
    unsafe {
        hashtable_close(core::ptr::addr_of_mut!((*object).hashtable));
        jsonp_free(object as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_size(json: *const json_t) -> usize {
    unsafe {
        if !json_is_object(json) {
            return 0;
        }

        let object = json_to_object(json);
        (*object).hashtable.size
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_get(json: *const json_t, key: *const c_char) -> *mut json_t {
    unsafe {
        if key.is_null() {
            return null_mut();
        }

        json_object_getn(json, key, cffi::c_strlen(key))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_getn(
    json: *const json_t,
    key: *const c_char,
    key_len: usize,
) -> *mut json_t {
    unsafe {
        if key.is_null() || !json_is_object(json) {
            return null_mut();
        }

        let object = json_to_object(json);
        hashtable_get(core::ptr::addr_of_mut!((*object).hashtable), key, key_len) as *mut json_t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new_nocheck(
    json: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if key.is_null() {
            json_decref(value);
            return -1;
        }
        json_object_setn_new_nocheck(json, key, cffi::c_strlen(key), value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new_nocheck(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if value.is_null() {
            return -1;
        }

        if key.is_null() || !json_is_object(json) || json == value {
            json_decref(value);
            return -1;
        }
        let object = json_to_object(json);

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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_set_new(
    json: *mut json_t,
    key: *const c_char,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if key.is_null() {
            json_decref(value);
            return -1;
        }

        json_object_setn_new(json, key, cffi::c_strlen(key), value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_setn_new(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if key.is_null() || utf8_check_string(key, key_len) == 0 {
            json_decref(value);
            return -1;
        }

        json_object_setn_new_nocheck(json, key, key_len, value)
    }
}

/// `json_object_setn_nocheck()` (static inline in jansson.h)
#[inline]
unsafe fn json_object_setn_nocheck(
    object: *mut json_t,
    key: *const c_char,
    key_len: usize,
    value: *mut json_t,
) -> c_int {
    unsafe { json_object_setn_new_nocheck(object, key, key_len, json_incref(value)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_del(json: *mut json_t, key: *const c_char) -> c_int {
    unsafe {
        if key.is_null() {
            return -1;
        }

        json_object_deln(json, key, cffi::c_strlen(key))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_deln(
    json: *mut json_t,
    key: *const c_char,
    key_len: usize,
) -> c_int {
    unsafe {
        if key.is_null() || !json_is_object(json) {
            return -1;
        }

        let object = json_to_object(json);
        hashtable_del(core::ptr::addr_of_mut!((*object).hashtable), key, key_len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_clear(json: *mut json_t) -> c_int {
    unsafe {
        if !json_is_object(json) {
            return -1;
        }

        let object = json_to_object(json);
        hashtable_clear(core::ptr::addr_of_mut!((*object).hashtable));

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update(object: *mut json_t, other: *mut json_t) -> c_int {
    unsafe {
        if !json_is_object(object) || !json_is_object(other) {
            return -1;
        }

        let mut it = KeylenIter::new(other);
        while let Some((key, key_len, value)) = it.next() {
            if json_object_setn_nocheck(object, key, key_len, value) != 0 {
                return -1;
            }
            it.advance();
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_existing(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    unsafe {
        if !json_is_object(object) || !json_is_object(other) {
            return -1;
        }

        let mut it = KeylenIter::new(other);
        while let Some((key, key_len, value)) = it.next() {
            if !json_object_getn(object, key, key_len).is_null() {
                json_object_setn_nocheck(object, key, key_len, value);
            }
            it.advance();
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_missing(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    unsafe {
        if !json_is_object(object) || !json_is_object(other) {
            return -1;
        }

        let mut it = KeylenIter::new(other);
        while let Some((key, key_len, value)) = it.next() {
            if json_object_getn(object, key, key_len).is_null() {
                json_object_setn_nocheck(object, key, key_len, value);
            }
            it.advance();
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_object_update_recursive(
    object: *mut json_t,
    other: *mut json_t,
    parents: *mut hashtable_t,
) -> c_int {
    unsafe {
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

        let mut it = KeylenIter::new(other);
        while let Some((key, key_len, value)) = it.next() {
            let v = json_object_getn(object, key, key_len);

            if json_is_object(v) && json_is_object(value) {
                if do_object_update_recursive(v, value, parents) != 0 {
                    res = -1;
                    break;
                }
            } else {
                if json_object_setn_nocheck(object, key, key_len, value) != 0 {
                    res = -1;
                    break;
                }
            }
            it.advance();
        }

        hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

        res
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_update_recursive(
    object: *mut json_t,
    other: *mut json_t,
) -> c_int {
    unsafe {
        let mut parents_set = hashtable_t::new();

        if hashtable_init(&mut parents_set) != 0 {
            return -1;
        }
        let res = do_object_update_recursive(object, other, &mut parents_set);
        hashtable_close(&mut parents_set);

        res
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter(json: *mut json_t) -> *mut c_void {
    unsafe {
        if !json_is_object(json) {
            return null_mut();
        }

        let object = json_to_object(json);
        hashtable_iter(core::ptr::addr_of_mut!((*object).hashtable))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_at(
    json: *mut json_t,
    key: *const c_char,
) -> *mut c_void {
    unsafe {
        if key.is_null() || !json_is_object(json) {
            return null_mut();
        }

        let object = json_to_object(json);
        hashtable_iter_at(
            core::ptr::addr_of_mut!((*object).hashtable),
            key,
            cffi::c_strlen(key),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_next(
    json: *mut json_t,
    iter: *mut c_void,
) -> *mut c_void {
    unsafe {
        if !json_is_object(json) || iter.is_null() {
            return null_mut();
        }

        let object = json_to_object(json);
        hashtable_iter_next(core::ptr::addr_of_mut!((*object).hashtable), iter)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_key(iter: *mut c_void) -> *const c_char {
    unsafe {
        if iter.is_null() {
            return null();
        }

        hashtable_iter_key(iter) as *const c_char
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_key_len(iter: *mut c_void) -> usize {
    unsafe {
        if iter.is_null() {
            return 0;
        }

        hashtable_iter_key_len(iter)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_value(iter: *mut c_void) -> *mut json_t {
    unsafe {
        if iter.is_null() {
            return null_mut();
        }

        hashtable_iter_value(iter) as *mut json_t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_iter_set_new(
    json: *mut json_t,
    iter: *mut c_void,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if !json_is_object(json) || iter.is_null() || value.is_null() {
            json_decref(value);
            return -1;
        }

        hashtable_iter_set(iter, value);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_key_to_iter(key: *const c_char) -> *mut c_void {
    unsafe {
        if key.is_null() {
            return null_mut();
        }

        hashtable_key_to_iter(key)
    }
}

unsafe fn json_object_equal(object1: *const json_t, object2: *const json_t) -> c_int {
    unsafe {
        if json_object_size(object1) != json_object_size(object2) {
            return 0;
        }

        let mut it = KeylenIter::new(object1 as *mut json_t);
        while let Some((key, key_len, value1)) = it.next() {
            let value2 = json_object_getn(object2, key, key_len);

            if json_equal(value1, value2) == 0 {
                return 0;
            }
            it.advance();
        }

        1
    }
}

unsafe fn json_object_copy(object: *mut json_t) -> *mut json_t {
    unsafe {
        let result = json_object();
        if result.is_null() {
            return null_mut();
        }

        let mut it = KeylenIter::new(object);
        while let Some((key, key_len, value)) = it.next() {
            json_object_setn_nocheck(result, key, key_len, value);
            it.advance();
        }

        result
    }
}

unsafe fn json_object_deep_copy(object: *const json_t, parents: *mut hashtable_t) -> *mut json_t {
    unsafe {
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
            return null_mut();
        }

        let mut result = json_object();
        if !result.is_null() {
            /* Cannot use json_object_foreach because object has to be cast
            non-const */
            let mut iter = json_object_iter(object as *mut json_t);
            while !iter.is_null() {
                let key = json_object_iter_key(iter);
                let key_len = json_object_iter_key_len(iter);
                let value = json_object_iter_value(iter);

                if json_object_setn_new_nocheck(result, key, key_len, do_deep_copy(value, parents))
                    != 0
                {
                    json_decref(result);
                    result = null_mut();
                    break;
                }
                iter = json_object_iter_next(object as *mut json_t, iter);
            }
        }

        hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

        result
    }
}

/* ------------------------------------------------------------------ */
/* array                                                              */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array() -> *mut json_t {
    unsafe {
        let array = jsonp_malloc(core::mem::size_of::<json_array_t>()) as *mut json_array_t;
        if array.is_null() {
            return null_mut();
        }
        json_init(core::ptr::addr_of_mut!((*array).json), JSON_ARRAY);

        (*array).entries = 0;
        (*array).size = 8;

        (*array).table =
            jsonp_malloc((*array).size * core::mem::size_of::<*mut json_t>()) as *mut *mut json_t;
        if (*array).table.is_null() {
            jsonp_free(array as *mut c_void);
            return null_mut();
        }

        core::ptr::addr_of_mut!((*array).json)
    }
}

unsafe fn json_delete_array(array: *mut json_array_t) {
    unsafe {
        for i in 0..(*array).entries {
            json_decref(*(*array).table.add(i));
        }

        jsonp_free((*array).table as *mut c_void);
        jsonp_free(array as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_size(json: *const json_t) -> usize {
    unsafe {
        if !json_is_array(json) {
            return 0;
        }

        (*json_to_array(json)).entries
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_get(json: *const json_t, index: usize) -> *mut json_t {
    unsafe {
        if !json_is_array(json) {
            return null_mut();
        }
        let array = json_to_array(json);

        if index >= (*array).entries {
            return null_mut();
        }

        *(*array).table.add(index)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_set_new(
    json: *mut json_t,
    index: usize,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if value.is_null() {
            return -1;
        }

        if !json_is_array(json) || json == value {
            json_decref(value);
            return -1;
        }
        let array = json_to_array(json);

        if index >= (*array).entries {
            json_decref(value);
            return -1;
        }

        json_decref(*(*array).table.add(index));
        *(*array).table.add(index) = value;

        0
    }
}

unsafe fn array_move(array: *mut json_array_t, dest: usize, src: usize, count: usize) {
    unsafe {
        core::ptr::copy(
            (*array).table.add(src),
            (*array).table.add(dest),
            count,
        );
    }
}

unsafe fn array_copy(
    dest: *mut *mut json_t,
    dpos: usize,
    src: *mut *mut json_t,
    spos: usize,
    count: usize,
) {
    unsafe {
        core::ptr::copy_nonoverlapping(src.add(spos), dest.add(dpos), count);
    }
}

unsafe fn json_array_grow(array: *mut json_array_t, amount: usize) -> *mut *mut json_t {
    unsafe {
        if (*array).entries + amount <= (*array).size {
            return (*array).table;
        }

        let old_table = (*array).table;

        let a = (*array).size + amount;
        let b = (*array).size * 2;
        let new_size = if a > b { a } else { b };
        let new_table = crate::memory::jsonp_realloc(
            old_table as *mut c_void,
            (*array).size * core::mem::size_of::<*mut json_t>(),
            new_size * core::mem::size_of::<*mut json_t>(),
        ) as *mut *mut json_t;
        if new_table.is_null() {
            return null_mut();
        }

        (*array).size = new_size;
        (*array).table = new_table;

        (*array).table
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_append_new(json: *mut json_t, value: *mut json_t) -> c_int {
    unsafe {
        if value.is_null() {
            return -1;
        }

        if !json_is_array(json) || json == value {
            json_decref(value);
            return -1;
        }
        let array = json_to_array(json);

        if json_array_grow(array, 1).is_null() {
            json_decref(value);
            return -1;
        }

        *(*array).table.add((*array).entries) = value;
        (*array).entries += 1;

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_insert_new(
    json: *mut json_t,
    index: usize,
    value: *mut json_t,
) -> c_int {
    unsafe {
        if value.is_null() {
            return -1;
        }

        if !json_is_array(json) || json == value {
            json_decref(value);
            return -1;
        }
        let array = json_to_array(json);

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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_remove(json: *mut json_t, index: usize) -> c_int {
    unsafe {
        if !json_is_array(json) {
            return -1;
        }
        let array = json_to_array(json);

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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_clear(json: *mut json_t) -> c_int {
    unsafe {
        if !json_is_array(json) {
            return -1;
        }
        let array = json_to_array(json);

        for i in 0..(*array).entries {
            json_decref(*(*array).table.add(i));
        }

        (*array).entries = 0;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_array_extend(json: *mut json_t, other_json: *mut json_t) -> c_int {
    unsafe {
        if !json_is_array(json) || !json_is_array(other_json) {
            return -1;
        }
        let array = json_to_array(json);
        let other = json_to_array(other_json);

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
}

/// `json_array_append()` (static inline in jansson.h)
#[inline]
unsafe fn json_array_append(array: *mut json_t, value: *mut json_t) -> c_int {
    unsafe { json_array_append_new(array, json_incref(value)) }
}

unsafe fn json_array_equal(array1: *const json_t, array2: *const json_t) -> c_int {
    unsafe {
        let size = json_array_size(array1);
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
}

unsafe fn json_array_copy(array: *mut json_t) -> *mut json_t {
    unsafe {
        let result = json_array();
        if result.is_null() {
            return null_mut();
        }

        let mut i = 0usize;
        while i < json_array_size(array) {
            json_array_append(result, json_array_get(array, i));
            i += 1;
        }

        result
    }
}

unsafe fn json_array_deep_copy(array: *const json_t, parents: *mut hashtable_t) -> *mut json_t {
    unsafe {
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
            return null_mut();
        }

        let mut result = json_array();
        if !result.is_null() {
            let mut i = 0usize;
            while i < json_array_size(array) {
                if json_array_append_new(result, do_deep_copy(json_array_get(array, i), parents))
                    != 0
                {
                    json_decref(result);
                    result = null_mut();
                    break;
                }
                i += 1;
            }
        }

        hashtable_del(parents, loop_key.as_ptr(), loop_key_len);

        result
    }
}

/* ------------------------------------------------------------------ */
/* string                                                             */
/* ------------------------------------------------------------------ */

unsafe fn string_create(value: *const c_char, len: usize, own: c_int) -> *mut json_t {
    unsafe {
        if value.is_null() {
            return null_mut();
        }

        let v: *mut c_char;
        if own != 0 {
            v = value as *mut c_char;
        } else {
            v = jsonp_strndup(value, len);
            if v.is_null() {
                return null_mut();
            }
        }

        let string = jsonp_malloc(core::mem::size_of::<json_string_t>()) as *mut json_string_t;
        if string.is_null() {
            jsonp_free(v as *mut c_void);
            return null_mut();
        }
        json_init(core::ptr::addr_of_mut!((*string).json), JSON_STRING);
        (*string).value = v;
        (*string).length = len;

        core::ptr::addr_of_mut!((*string).json)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_nocheck(value: *const c_char) -> *mut json_t {
    unsafe {
        if value.is_null() {
            return null_mut();
        }

        string_create(value, cffi::c_strlen(value), 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn_nocheck(value: *const c_char, len: usize) -> *mut json_t {
    unsafe { string_create(value, len, 0) }
}

/* this is private; "steal" is not a public API concept */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_stringn_nocheck_own(
    value: *const c_char,
    len: usize,
) -> *mut json_t {
    unsafe { string_create(value, len, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string(value: *const c_char) -> *mut json_t {
    unsafe {
        if value.is_null() {
            return null_mut();
        }

        json_stringn(value, cffi::c_strlen(value))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_stringn(value: *const c_char, len: usize) -> *mut json_t {
    unsafe {
        if value.is_null() || utf8_check_string(value, len) == 0 {
            return null_mut();
        }

        json_stringn_nocheck(value, len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_value(json: *const json_t) -> *const c_char {
    unsafe {
        if !json_is_string(json) {
            return null();
        }

        (*json_to_string(json)).value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_length(json: *const json_t) -> usize {
    unsafe {
        if !json_is_string(json) {
            return 0;
        }

        (*json_to_string(json)).length
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set_nocheck(
    json: *mut json_t,
    value: *const c_char,
) -> c_int {
    unsafe {
        if value.is_null() {
            return -1;
        }

        json_string_setn_nocheck(json, value, cffi::c_strlen(value))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn_nocheck(
    json: *mut json_t,
    value: *const c_char,
    len: usize,
) -> c_int {
    unsafe {
        if !json_is_string(json) || value.is_null() {
            return -1;
        }

        let dup = jsonp_strndup(value, len);
        if dup.is_null() {
            return -1;
        }

        let string = json_to_string(json);
        jsonp_free((*string).value as *mut c_void);
        (*string).value = dup;
        (*string).length = len;

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_set(json: *mut json_t, value: *const c_char) -> c_int {
    unsafe {
        if value.is_null() {
            return -1;
        }

        json_string_setn(json, value, cffi::c_strlen(value))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_string_setn(
    json: *mut json_t,
    value: *const c_char,
    len: usize,
) -> c_int {
    unsafe {
        if value.is_null() || utf8_check_string(value, len) == 0 {
            return -1;
        }

        json_string_setn_nocheck(json, value, len)
    }
}

unsafe fn json_delete_string(string: *mut json_string_t) {
    unsafe {
        jsonp_free((*string).value as *mut c_void);
        jsonp_free(string as *mut c_void);
    }
}

unsafe fn json_string_equal(string1: *const json_t, string2: *const json_t) -> c_int {
    unsafe {
        let s1 = json_to_string(string1);
        let s2 = json_to_string(string2);
        if (*s1).length == (*s2).length
            && cffi::c_memcmp((*s1).value, (*s2).value, (*s1).length) == 0
        {
            1
        } else {
            0
        }
    }
}

unsafe fn json_string_copy(string: *const json_t) -> *mut json_t {
    unsafe {
        let s = json_to_string(string);
        json_stringn_nocheck((*s).value, (*s).length)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vsprintf(fmt: *const c_char, ap: VaList) -> *mut json_t {
    unsafe {
        let mut json: *mut json_t = null_mut();
        let mut aq = VaListTag {
            gp_offset: 0,
            fp_offset: 0,
            overflow_arg_area: null_mut(),
            reg_save_area: null_mut(),
        };
        va_copy(&mut aq, ap);

        let length = cffi::vsnprintf(null_mut(), 0, fmt, ap);
        if length < 0 {
            return json;
        }
        if length == 0 {
            json = json_string(c"".as_ptr());
            return json;
        }

        let buf = jsonp_malloc(length as usize + 1) as *mut c_char;
        if buf.is_null() {
            return json;
        }

        cffi::vsnprintf(buf, length as usize + 1, fmt, &mut aq);
        if utf8_check_string(buf, length as usize) == 0 {
            jsonp_free(buf as *mut c_void);
            return json;
        }

        json = jsonp_stringn_nocheck_own(buf, length as usize);

        json
    }
}

/* json_sprintf() is provided by the assembly trampoline. */

/* ------------------------------------------------------------------ */
/* integer                                                            */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer(value: json_int_t) -> *mut json_t {
    unsafe {
        let integer = jsonp_malloc(core::mem::size_of::<json_integer_t>()) as *mut json_integer_t;
        if integer.is_null() {
            return null_mut();
        }
        json_init(core::ptr::addr_of_mut!((*integer).json), JSON_INTEGER);

        (*integer).value = value;
        core::ptr::addr_of_mut!((*integer).json)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_value(json: *const json_t) -> json_int_t {
    unsafe {
        if !json_is_integer(json) {
            return 0;
        }

        (*json_to_integer(json)).value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_integer_set(json: *mut json_t, value: json_int_t) -> c_int {
    unsafe {
        if !json_is_integer(json) {
            return -1;
        }

        (*json_to_integer(json)).value = value;

        0
    }
}

unsafe fn json_delete_integer(integer: *mut json_integer_t) {
    unsafe { jsonp_free(integer as *mut c_void) }
}

unsafe fn json_integer_equal(integer1: *const json_t, integer2: *const json_t) -> c_int {
    unsafe {
        if json_integer_value(integer1) == json_integer_value(integer2) {
            1
        } else {
            0
        }
    }
}

unsafe fn json_integer_copy(integer: *const json_t) -> *mut json_t {
    unsafe { json_integer(json_integer_value(integer)) }
}

/* ------------------------------------------------------------------ */
/* real                                                               */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real(value: f64) -> *mut json_t {
    unsafe {
        if value.is_nan() || value.is_infinite() {
            return null_mut();
        }

        let real = jsonp_malloc(core::mem::size_of::<json_real_t>()) as *mut json_real_t;
        if real.is_null() {
            return null_mut();
        }
        json_init(core::ptr::addr_of_mut!((*real).json), JSON_REAL);

        (*real).value = value;
        core::ptr::addr_of_mut!((*real).json)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_value(json: *const json_t) -> f64 {
    unsafe {
        if !json_is_real(json) {
            return 0.0;
        }

        (*json_to_real(json)).value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_real_set(json: *mut json_t, value: f64) -> c_int {
    unsafe {
        if !json_is_real(json) || value.is_nan() || value.is_infinite() {
            return -1;
        }

        (*json_to_real(json)).value = value;

        0
    }
}

unsafe fn json_delete_real(real: *mut json_real_t) {
    unsafe { jsonp_free(real as *mut c_void) }
}

unsafe fn json_real_equal(real1: *const json_t, real2: *const json_t) -> c_int {
    unsafe {
        if json_real_value(real1) == json_real_value(real2) {
            1
        } else {
            0
        }
    }
}

unsafe fn json_real_copy(real: *const json_t) -> *mut json_t {
    unsafe { json_real(json_real_value(real)) }
}

/* ------------------------------------------------------------------ */
/* number                                                             */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_number_value(json: *const json_t) -> f64 {
    unsafe {
        if json_is_integer(json) {
            json_integer_value(json) as f64
        } else if json_is_real(json) {
            json_real_value(json)
        } else {
            0.0
        }
    }
}

/* ------------------------------------------------------------------ */
/* simple values                                                      */
/* ------------------------------------------------------------------ */

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
    &raw mut THE_TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_false() -> *mut json_t {
    &raw mut THE_FALSE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_null() -> *mut json_t {
    &raw mut THE_NULL
}

/* ------------------------------------------------------------------ */
/* deletion                                                           */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_delete(json: *mut json_t) {
    unsafe {
        if json.is_null() {
            return;
        }

        match json_typeof(json) {
            JSON_OBJECT => json_delete_object(json_to_object(json)),
            JSON_ARRAY => json_delete_array(json_to_array(json)),
            JSON_STRING => json_delete_string(json_to_string(json)),
            JSON_INTEGER => json_delete_integer(json_to_integer(json)),
            JSON_REAL => json_delete_real(json_to_real(json)),
            _ => return,
        }

        /* json_delete is not called for true, false or null */
    }
}

/* ------------------------------------------------------------------ */
/* equality                                                           */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_equal(json1: *const json_t, json2: *const json_t) -> c_int {
    unsafe {
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
}

/* ------------------------------------------------------------------ */
/* copying                                                            */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_copy(json: *mut json_t) -> *mut json_t {
    unsafe {
        if json.is_null() {
            return null_mut();
        }

        match json_typeof(json) {
            JSON_OBJECT => json_object_copy(json),
            JSON_ARRAY => json_array_copy(json),
            JSON_STRING => json_string_copy(json),
            JSON_INTEGER => json_integer_copy(json),
            JSON_REAL => json_real_copy(json),
            JSON_TRUE | JSON_FALSE | JSON_NULL => json,
            _ => null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_deep_copy(json: *const json_t) -> *mut json_t {
    unsafe {
        let mut parents_set = hashtable_t::new();

        if hashtable_init(&mut parents_set) != 0 {
            return null_mut();
        }
        let res = do_deep_copy(json, &mut parents_set);
        hashtable_close(&mut parents_set);

        res
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn do_deep_copy(
    json: *const json_t,
    parents: *mut hashtable_t,
) -> *mut json_t {
    unsafe {
        if json.is_null() {
            return null_mut();
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
            _ => null_mut(),
        }
    }
}
