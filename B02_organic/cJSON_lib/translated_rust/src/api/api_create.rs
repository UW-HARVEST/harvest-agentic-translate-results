use std::ffi::{c_char, c_double, c_int, c_void};
use std::ptr;
use crate::types::*;
use crate::globals::*;
use crate::helpers::*;
use super::api_core::cJSON_Delete;

// ---- Create basic types ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() { (*item).type_ = CJSON_NULL; }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() { (*item).type_ = CJSON_TRUE; }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() { (*item).type_ = CJSON_FALSE; }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = if boolean != 0 { CJSON_TRUE } else { CJSON_FALSE };
    }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_NUMBER;
        (*item).valuedouble = num;
        if num >= c_int::MAX as f64 {
            (*item).valueint = c_int::MAX;
        } else if num <= c_int::MIN as f64 {
            (*item).valueint = c_int::MIN;
        } else {
            (*item).valueint = num as c_int;
        }
    }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_STRING;
        (*item).valuestring = cjson_strdup(string as *const u8, &GLOBAL_HOOKS) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_RAW;
        (*item).valuestring = cjson_strdup(raw as *const u8, &GLOBAL_HOOKS) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() { (*item).type_ = CJSON_ARRAY; }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() { (*item).type_ = CJSON_OBJECT; }
    item
}

// ---- Reference creators ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_STRING | CJSON_IS_REFERENCE;
        (*item).valuestring = string as *mut c_char;
    }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_OBJECT | CJSON_IS_REFERENCE;
        (*item).child = child as *mut cJSON;
    }
    item
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cjson_new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_ARRAY | CJSON_IS_REFERENCE;
        (*item).child = child as *mut cJSON;
    }
    item
}

// ---- Array creators ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateNumber(*numbers.add(i) as f64);
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 { (*a).child = n; } else { suffix_object(p, n); }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() { (*(*a).child).prev = n; }
    a
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(numbers: *const f32, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateNumber(*numbers.add(i) as f64);
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 { (*a).child = n; } else { suffix_object(p, n); }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() { (*(*a).child).prev = n; }
    a
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateNumber(*numbers.add(i));
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 { (*a).child = n; } else { suffix_object(p, n); }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() { (*(*a).child).prev = n; }
    a
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON {
    if count < 0 || strings.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateString(*strings.add(i));
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 { (*a).child = n; } else { suffix_object(p, n); }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() { (*(*a).child).prev = n; }
    a
}

// ---- Duplicate ----
unsafe fn cjson_duplicate_rec(item: *const cJSON, depth: usize, recurse: cJSON_bool) -> *mut cJSON {
    if item.is_null() { return ptr::null_mut(); }
    let newitem = cjson_new_item(&GLOBAL_HOOKS);
    if newitem.is_null() { return ptr::null_mut(); }

    (*newitem).type_ = (*item).type_ & (!CJSON_IS_REFERENCE);
    (*newitem).valueint = (*item).valueint;
    (*newitem).valuedouble = (*item).valuedouble;
    if !(*item).valuestring.is_null() {
        (*newitem).valuestring = cjson_strdup((*item).valuestring as *const u8, &GLOBAL_HOOKS) as *mut c_char;
        if (*newitem).valuestring.is_null() { cJSON_Delete(newitem); return ptr::null_mut(); }
    }
    if !(*item).string.is_null() {
        if ((*item).type_ & CJSON_STRING_IS_CONST) != 0 {
            (*newitem).string = (*item).string;
        } else {
            (*newitem).string = cjson_strdup((*item).string as *const u8, &GLOBAL_HOOKS) as *mut c_char;
        }
        if (*newitem).string.is_null() { cJSON_Delete(newitem); return ptr::null_mut(); }
    }
    if recurse == 0 { return newitem; }

    let mut child = (*item).child;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();
    while !child.is_null() {
        if depth >= CJSON_CIRCULAR_LIMIT { cJSON_Delete(newitem); return ptr::null_mut(); }
        newchild = cjson_duplicate_rec(child, depth + 1, 1);
        if newchild.is_null() { cJSON_Delete(newitem); return ptr::null_mut(); }
        if !next.is_null() {
            (*next).next = newchild;
            (*newchild).prev = next;
            next = newchild;
        } else {
            (*newitem).child = newchild;
            next = newchild;
        }
        child = (*child).next;
    }
    if !newitem.is_null() && !(*newitem).child.is_null() {
        (*(*newitem).child).prev = newchild;
    }
    newitem
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cjson_duplicate_rec(item, 0, recurse)
}

// ---- Compare ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: cJSON_bool) -> cJSON_bool {
    if a.is_null() || b.is_null() || ((*a).type_ & 0xFF) != ((*b).type_ & 0xFF) {
        return 0;
    }
    match (*a).type_ & 0xFF {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL | CJSON_NUMBER | CJSON_STRING | CJSON_RAW | CJSON_ARRAY | CJSON_OBJECT => {}
        _ => return 0,
    }
    if a == b { return 1; }
    match (*a).type_ & 0xFF {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL => 1,
        CJSON_NUMBER => compare_double((*a).valuedouble, (*b).valuedouble),
        CJSON_STRING | CJSON_RAW => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() { return 0; }
            if libc::strcmp((*a).valuestring, (*b).valuestring) == 0 { 1 } else { 0 }
        }
        CJSON_ARRAY => {
            let mut ae = (*a).child;
            let mut be = (*b).child;
            while !ae.is_null() && !be.is_null() {
                if cJSON_Compare(ae, be, case_sensitive) == 0 { return 0; }
                ae = (*ae).next;
                be = (*be).next;
            }
            if ae != be { 0 } else { 1 }
        }
        CJSON_OBJECT => {
            let mut a_el = (*a).child;
            while !a_el.is_null() {
                let b_el = get_object_item(b, (*a_el).string, case_sensitive);
                if b_el.is_null() { return 0; }
                if cJSON_Compare(a_el, b_el, case_sensitive) == 0 { return 0; }
                a_el = (*a_el).next;
            }
            let mut b_el = (*b).child;
            while !b_el.is_null() {
                let a_el2 = get_object_item(a, (*b_el).string, case_sensitive);
                if a_el2.is_null() { return 0; }
                if cJSON_Compare(b_el, a_el2, case_sensitive) == 0 { return 0; }
                b_el = (*b_el).next;
            }
            1
        }
        _ => 0,
    }
}

// ---- Minify ----
unsafe fn skip_oneline_comment(input: *mut *mut u8) {
    *input = (*input).add(2);
    while *(*input) != 0 {
        if *(*input) == b'\n' {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut u8) {
    *input = (*input).add(2);
    while *(*input) != 0 {
        if *(*input) == b'*' && *(*input).add(1) == b'/' {
            *input = (*input).add(2);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn minify_string(input: *mut *mut u8, output: *mut *mut u8) {
    *(*output) = *(*input);
    *input = (*input).add(1);
    *output = (*output).add(1);
    while *(*input) != 0 {
        *(*output) = *(*input);
        if *(*input) == b'\"' {
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        } else if *(*input) == b'\\' && *(*input).add(1) == b'\"' {
            *(*output).add(1) = *(*input).add(1);
            *input = (*input).add(1);
            *output = (*output).add(1);
        }
        *input = (*input).add(1);
        *output = (*output).add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Minify(json: *mut c_char) {
    if json.is_null() { return; }
    let mut json_ptr = json as *mut u8;
    let mut into = json as *mut u8;
    while *json_ptr != 0 {
        match *json_ptr {
            b' ' | b'\t' | b'\r' | b'\n' => { json_ptr = json_ptr.add(1); }
            b'/' => {
                if *json_ptr.add(1) == b'/' {
                    skip_oneline_comment(&mut json_ptr);
                } else if *json_ptr.add(1) == b'*' {
                    skip_multiline_comment(&mut json_ptr);
                } else {
                    json_ptr = json_ptr.add(1);
                }
            }
            b'\"' => {
                minify_string(&mut json_ptr, &mut into);
            }
            _ => {
                *into = *json_ptr;
                json_ptr = json_ptr.add(1);
                into = into.add(1);
            }
        }
    }
    *into = 0;
}

// ---- AddXToObject helpers ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let null = cJSON_CreateNull();
    if add_item_to_object(object, name, null, &GLOBAL_HOOKS, 0) != 0 { return null; }
    cJSON_Delete(null);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateTrue();
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateFalse();
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON {
    let item = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateString(string);
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateObject();
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateArray();
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 { return item; }
    cJSON_Delete(item);
    ptr::null_mut()
}
