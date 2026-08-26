use libc::{c_char, c_double, c_int, c_void, size_t};
use serde_json::{Map, Number, Value};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

pub const CJSON_VERSION_MAJOR: c_int = 1;
pub const CJSON_VERSION_MINOR: c_int = 7;
pub const CJSON_VERSION_PATCH: c_int = 19;

pub const CJSON_INVALID: c_int = 0;
pub const CJSON_FALSE: c_int = 1 << 0;
pub const CJSON_TRUE: c_int = 1 << 1;
pub const CJSON_NULL: c_int = 1 << 2;
pub const CJSON_NUMBER: c_int = 1 << 3;
pub const CJSON_STRING: c_int = 1 << 4;
pub const CJSON_ARRAY: c_int = 1 << 5;
pub const CJSON_OBJECT: c_int = 1 << 6;
pub const CJSON_RAW: c_int = 1 << 7;
pub const CJSON_IS_REFERENCE: c_int = 256;
pub const CJSON_STRING_IS_CONST: c_int = 512;

#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<unsafe extern "C" fn(size_t) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

pub type cJSON_bool = c_int;

static GLOBAL_ERROR: AtomicPtr<c_char> = AtomicPtr::new(ptr::null_mut());

fn set_error_ptr(p: *mut c_char) {
    GLOBAL_ERROR.store(p, Ordering::Relaxed);
}

fn cstr_to_string(ptr_in: *const c_char) -> Option<String> {
    if ptr_in.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr_in) }.to_string_lossy().into_owned())
    }
}

unsafe fn dup_c_string(s: &str) -> *mut c_char {
    CString::new(s).ok().map_or(ptr::null_mut(), CString::into_raw)
}

unsafe fn free_c_string(p: *mut c_char) {
    if !p.is_null() {
        let _ = CString::from_raw(p);
    }
}

unsafe fn new_item() -> *mut cJSON {
    Box::into_raw(Box::new(cJSON {
        next: ptr::null_mut(),
        prev: ptr::null_mut(),
        child: ptr::null_mut(),
        type_: CJSON_INVALID,
        valuestring: ptr::null_mut(),
        valueint: 0,
        valuedouble: 0.0,
        string: ptr::null_mut(),
    }))
}

unsafe fn last_child(mut child: *mut cJSON) -> *mut cJSON {
    if child.is_null() {
        return ptr::null_mut();
    }
    while !(*child).next.is_null() {
        child = (*child).next;
    }
    child
}

unsafe fn append_child(parent: *mut cJSON, item: *mut cJSON) -> bool {
    if parent.is_null() || item.is_null() || parent == item {
        return false;
    }
    if (*parent).child.is_null() {
        (*parent).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
        true
    } else {
        let last = if !(*(*parent).child).prev.is_null() { (*(*parent).child).prev } else { last_child((*parent).child) };
        (*last).next = item;
        (*item).prev = last;
        (*item).next = ptr::null_mut();
        (*(*parent).child).prev = item;
        true
    }
}

unsafe fn rust_value_to_cjson(v: &Value) -> *mut cJSON {
    let item = new_item();
    if item.is_null() {
        return ptr::null_mut();
    }
    match v {
        Value::Null => {
            (*item).type_ = CJSON_NULL;
        }
        Value::Bool(b) => {
            (*item).type_ = if *b { CJSON_TRUE } else { CJSON_FALSE };
            (*item).valueint = if *b { 1 } else { 0 };
        }
        Value::Number(n) => {
            (*item).type_ = CJSON_NUMBER;
            let d = n.as_f64().unwrap_or(f64::NAN);
            (*item).valuedouble = d;
            (*item).valueint = if d.is_nan() {
                0
            } else if d >= i32::MAX as f64 {
                i32::MAX
            } else if d <= i32::MIN as f64 {
                i32::MIN
            } else {
                d as i32
            };
        }
        Value::String(s) => {
            (*item).type_ = CJSON_STRING;
            (*item).valuestring = dup_c_string(s);
            if (*item).valuestring.is_null() {
                cJSON_Delete(item);
                return ptr::null_mut();
            }
        }
        Value::Array(arr) => {
            (*item).type_ = CJSON_ARRAY;
            for elem in arr {
                let child = rust_value_to_cjson(elem);
                if child.is_null() || !append_child(item, child) {
                    cJSON_Delete(child);
                    cJSON_Delete(item);
                    return ptr::null_mut();
                }
            }
        }
        Value::Object(obj) => {
            (*item).type_ = CJSON_OBJECT;
            for (k, val) in obj {
                let child = rust_value_to_cjson(val);
                if child.is_null() {
                    cJSON_Delete(item);
                    return ptr::null_mut();
                }
                (*child).string = dup_c_string(k);
                if (*child).string.is_null() || !append_child(item, child) {
                    cJSON_Delete(child);
                    cJSON_Delete(item);
                    return ptr::null_mut();
                }
            }
        }
    }
    item
}

unsafe fn cjson_to_rust_value(item: *const cJSON) -> Option<Value> {
    if item.is_null() {
        return None;
    }
    match (*item).type_ & 0xFF {
        CJSON_NULL => Some(Value::Null),
        CJSON_FALSE => Some(Value::Bool(false)),
        CJSON_TRUE => Some(Value::Bool(true)),
        CJSON_NUMBER => {
            let d = (*item).valuedouble;
            Number::from_f64(d).map(Value::Number).or(Some(Value::Null))
        }
        CJSON_STRING | CJSON_RAW => {
            let s = cstr_to_string((*item).valuestring).unwrap_or_default();
            Some(Value::String(s))
        }
        CJSON_ARRAY => {
            let mut arr = Vec::new();
            let mut child = (*item).child;
            while !child.is_null() {
                arr.push(cjson_to_rust_value(child)?);
                child = (*child).next;
            }
            Some(Value::Array(arr))
        }
        CJSON_OBJECT => {
            let mut map = Map::new();
            let mut child = (*item).child;
            while !child.is_null() {
                let key = cstr_to_string((*child).string)?;
                let val = cjson_to_rust_value(child)?;
                map.insert(key, val);
                child = (*child).next;
            }
            Some(Value::Object(map))
        }
        _ => None,
    }
}

unsafe fn get_array_item_mut(array: *mut cJSON, mut index: usize) -> *mut cJSON {
    if array.is_null() {
        return ptr::null_mut();
    }
    let mut child = (*array).child;
    while !child.is_null() && index > 0 {
        child = (*child).next;
        index -= 1;
    }
    child
}

unsafe fn get_object_item_mut(object: *mut cJSON, key: &str, case_sensitive: bool) -> *mut cJSON {
    if object.is_null() {
        return ptr::null_mut();
    }
    let mut child = (*object).child;
    while !child.is_null() {
        if !(*child).string.is_null() {
            let s = CStr::from_ptr((*child).string).to_string_lossy();
            let ok = if case_sensitive { s == key } else { s.eq_ignore_ascii_case(key) };
            if ok {
                return child;
            }
        }
        child = (*child).next;
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Version() -> *const c_char {
    static VERSION: &[u8] = b"1.7.19\0";
    VERSION.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_InitHooks(_hooks: *mut cJSON_Hooks) {}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    GLOBAL_ERROR.load(Ordering::Relaxed) as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    unsafe {
        if cJSON_IsString(item) == 0 {
            ptr::null_mut()
        } else {
            (*item).valuestring
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    unsafe {
        if cJSON_IsNumber(item) == 0 {
            f64::NAN
        } else {
            (*item).valuedouble
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ParseWithLength(value: *const c_char, buffer_length: size_t) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ParseWithOpts(value: *const c_char, return_parse_end: *mut *const c_char, _require_null_terminated: cJSON_bool) -> *mut cJSON {
    if value.is_null() {
        return ptr::null_mut();
    }
    let s = unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned();
    cJSON_ParseWithLengthOpts(value, s.len() + 1, return_parse_end, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ParseWithLengthOpts(value: *const c_char, buffer_length: size_t, return_parse_end: *mut *const c_char, _require_null_terminated: cJSON_bool) -> *mut cJSON {
    if value.is_null() || buffer_length == 0 {
        return ptr::null_mut();
    }
    unsafe {
        let bytes = std::slice::from_raw_parts(value as *const u8, buffer_length);
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(buffer_length);
        let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
        match serde_json::from_str::<Value>(&s) {
            Ok(v) => {
                set_error_ptr(ptr::null_mut());
                if !return_parse_end.is_null() {
                    *return_parse_end = value.add(end);
                }
                rust_value_to_cjson(&v)
            }
            Err(e) => {
                let off = e.column().saturating_sub(1);
                let p = value.add(off.min(end));
                set_error_ptr(p as *mut c_char);
                if !return_parse_end.is_null() {
                    *return_parse_end = p;
                }
                ptr::null_mut()
            }
        }
    }
}

unsafe fn print_item(item: *const cJSON, formatted: bool) -> *mut c_char {
    if item.is_null() {
        return ptr::null_mut();
    }
    let value = match cjson_to_rust_value(item) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let s = if formatted {
        serde_json::to_string_pretty(&value).ok()
    } else {
        serde_json::to_string(&value).ok()
    };
    match s {
        Some(x) => dup_c_string(&x),
        None => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    unsafe { print_item(item, true) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    unsafe { print_item(item, false) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_PrintBuffered(item: *const cJSON, _prebuffer: c_int, fmt: cJSON_bool) -> *mut c_char {
    unsafe { print_item(item, fmt != 0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_PrintPreallocated(item: *mut cJSON, buffer: *mut c_char, length: c_int, format: cJSON_bool) -> cJSON_bool {
    if item.is_null() || buffer.is_null() || length < 0 {
        return 0;
    }
    unsafe {
        let printed = print_item(item, format != 0);
        if printed.is_null() {
            return 0;
        }
        let bytes = CStr::from_ptr(printed).to_bytes_with_nul();
        if bytes.len() > length as usize {
            free_c_string(printed);
            return 0;
        }
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, bytes.len());
        free_c_string(printed);
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Delete(item: *mut cJSON) {
    unsafe {
        let mut cur = item;
        while !cur.is_null() {
            let next = (*cur).next;
            if (*cur).type_ & CJSON_IS_REFERENCE == 0 && !(*cur).child.is_null() {
                cJSON_Delete((*cur).child);
            }
            if (*cur).type_ & CJSON_IS_REFERENCE == 0 && !(*cur).valuestring.is_null() {
                free_c_string((*cur).valuestring);
            }
            if (*cur).type_ & CJSON_STRING_IS_CONST == 0 && !(*cur).string.is_null() {
                free_c_string((*cur).string);
            }
            drop(Box::from_raw(cur));
            cur = next;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    unsafe {
        if array.is_null() {
            return 0;
        }
        let mut n = 0;
        let mut child = (*array).child;
        while !child.is_null() {
            n += 1;
            child = (*child).next;
        }
        n
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 {
        return ptr::null_mut();
    }
    unsafe { get_array_item_mut(array as *mut cJSON, index as usize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    let Some(key) = cstr_to_string(string) else { return ptr::null_mut(); };
    unsafe { get_object_item_mut(object as *mut cJSON, &key, false) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    let Some(key) = cstr_to_string(string) else { return ptr::null_mut(); };
    unsafe { get_object_item_mut(object as *mut cJSON, &key, true) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> cJSON_bool {
    if cJSON_GetObjectItem(object, string).is_null() { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() { (*item).type_ = CJSON_NULL; }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() { (*item).type_ = CJSON_TRUE; (*item).valueint = 1; }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() { (*item).type_ = CJSON_FALSE; }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    if boolean != 0 { cJSON_CreateTrue() } else { cJSON_CreateFalse() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() {
            (*item).type_ = CJSON_NUMBER;
            (*item).valuedouble = num;
            (*item).valueint = if num >= i32::MAX as f64 { i32::MAX } else if num <= i32::MIN as f64 { i32::MIN } else { num as i32 };
        }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let Some(s) = cstr_to_string(string) else { return ptr::null_mut(); };
    unsafe {
        let item = new_item();
        if item.is_null() { return ptr::null_mut(); }
        (*item).type_ = CJSON_STRING;
        (*item).valuestring = dup_c_string(&s);
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let Some(s) = cstr_to_string(raw) else { return ptr::null_mut(); };
    unsafe {
        let item = new_item();
        if item.is_null() { return ptr::null_mut(); }
        (*item).type_ = CJSON_RAW;
        (*item).valuestring = dup_c_string(&s);
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() { (*item).type_ = CJSON_ARRAY; }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() { (*item).type_ = CJSON_OBJECT; }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() {
            (*item).type_ = CJSON_STRING | CJSON_IS_REFERENCE;
            (*item).valuestring = string as *mut c_char;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() {
            (*item).type_ = CJSON_OBJECT | CJSON_IS_REFERENCE;
            (*item).child = child as *mut cJSON;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    unsafe {
        let item = new_item();
        if !item.is_null() {
            (*item).type_ = CJSON_ARRAY | CJSON_IS_REFERENCE;
            (*item).child = child as *mut cJSON;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    unsafe { if append_child(array, item) { 1 } else { 0 } }
}

unsafe fn add_item_to_object_impl(object: *mut cJSON, key: *const c_char, item: *mut cJSON, constant: bool) -> cJSON_bool {
    if object.is_null() || key.is_null() || item.is_null() || object == item {
        return 0;
    }
    if (*item).type_ & CJSON_STRING_IS_CONST == 0 && !(*item).string.is_null() {
        free_c_string((*item).string);
    }
    if constant {
        (*item).string = key as *mut c_char;
        (*item).type_ |= CJSON_STRING_IS_CONST;
    } else {
        let Some(s) = cstr_to_string(key) else { return 0; };
        (*item).string = dup_c_string(&s);
        if (*item).string.is_null() { return 0; }
        (*item).type_ &= !CJSON_STRING_IS_CONST;
    }
    if append_child(object, item) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool {
    unsafe { add_item_to_object_impl(object, string, item, false) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemToObjectCS(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool {
    unsafe { add_item_to_object_impl(object, string, item, true) }
}

unsafe fn create_reference(item: *mut cJSON) -> *mut cJSON {
    if item.is_null() { return ptr::null_mut(); }
    let r = new_item();
    if r.is_null() { return ptr::null_mut(); }
    ptr::copy_nonoverlapping(item, r, 1);
    (*r).string = ptr::null_mut();
    (*r).type_ |= CJSON_IS_REFERENCE;
    (*r).next = ptr::null_mut();
    (*r).prev = ptr::null_mut();
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    unsafe { cJSON_AddItemToArray(array, create_reference(item)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemReferenceToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool {
    unsafe { cJSON_AddItemToObject(object, string, create_reference(item)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateNull();
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateTrue();
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateFalse();
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_CreateBool(boolean);
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON {
    let item = cJSON_CreateNumber(number);
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateString(string);
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateRaw(raw);
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateObject();
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateArray();
    if cJSON_AddItemToObject(object, name, item) != 0 { item } else { unsafe { cJSON_Delete(item) }; ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON {
    unsafe {
        if parent.is_null() || item.is_null() || ((*parent).child != item && (*item).prev.is_null()) {
            return ptr::null_mut();
        }
        if item != (*parent).child {
            (*(*item).prev).next = (*item).next;
        }
        if !(*item).next.is_null() {
            (*(*item).next).prev = (*item).prev;
        }
        if item == (*parent).child {
            (*parent).child = (*item).next;
        } else if (*item).next.is_null() && !(*parent).child.is_null() {
            (*(*parent).child).prev = (*item).prev;
        }
        (*item).prev = ptr::null_mut();
        (*item).next = ptr::null_mut();
        item
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    if which < 0 { return ptr::null_mut(); }
    unsafe { cJSON_DetachItemViaPointer(array, get_array_item_mut(array, which as usize)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromArray(array, which)); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    let item = cJSON_GetObjectItem(object, string);
    cJSON_DetachItemViaPointer(object, item)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    let item = cJSON_GetObjectItemCaseSensitive(object, string);
    cJSON_DetachItemViaPointer(object, item)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromObject(object, string)); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string)); }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool {
    if which < 0 || newitem.is_null() {
        return 0;
    }
    unsafe {
        let after = get_array_item_mut(array, which as usize);
        if after.is_null() {
            return cJSON_AddItemToArray(array, newitem);
        }
        if after != (*array).child && (*after).prev.is_null() {
            return 0;
        }
        (*newitem).next = after;
        (*newitem).prev = (*after).prev;
        (*after).prev = newitem;
        if after == (*array).child {
            (*array).child = newitem;
        } else {
            (*(*newitem).prev).next = newitem;
        }
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> cJSON_bool {
    unsafe {
        if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
            return 0;
        }
        if replacement == item {
            return 1;
        }
        (*replacement).next = (*item).next;
        (*replacement).prev = (*item).prev;
        if !(*replacement).next.is_null() {
            (*(*replacement).next).prev = replacement;
        }
        if (*parent).child == item {
            if (*(*parent).child).prev == (*parent).child {
                (*replacement).prev = replacement;
            }
            (*parent).child = replacement;
        } else {
            if !(*replacement).prev.is_null() {
                (*(*replacement).prev).next = replacement;
            }
            if (*replacement).next.is_null() {
                (*(*parent).child).prev = replacement;
            }
        }
        (*item).next = ptr::null_mut();
        (*item).prev = ptr::null_mut();
        cJSON_Delete(item);
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool {
    if which < 0 { return 0; }
    unsafe { cJSON_ReplaceItemViaPointer(array, get_array_item_mut(array, which as usize), newitem) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool {
    cJSON_ReplaceItemInObjectCaseSensitive(object, string, newitem)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool {
    if object.is_null() || string.is_null() || newitem.is_null() {
        return 0;
    }
    unsafe {
        if (*newitem).type_ & CJSON_STRING_IS_CONST == 0 && !(*newitem).string.is_null() {
            free_c_string((*newitem).string);
        }
        let Some(key) = cstr_to_string(string) else { return 0; };
        (*newitem).string = dup_c_string(&key);
        if (*newitem).string.is_null() {
            return 0;
        }
        (*newitem).type_ &= !CJSON_STRING_IS_CONST;
        let old = get_object_item_mut(object, &key, true);
        cJSON_ReplaceItemViaPointer(object, old, newitem)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let array = cJSON_CreateArray();
    if array.is_null() { return ptr::null_mut(); }
    unsafe {
        for i in 0..count as isize {
            let item = cJSON_CreateNumber(*numbers.offset(i) as f64);
            if item.is_null() || cJSON_AddItemToArray(array, item) == 0 {
                cJSON_Delete(item);
                cJSON_Delete(array);
                return ptr::null_mut();
            }
        }
    }
    array
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateFloatArray(numbers: *const f32, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let array = cJSON_CreateArray();
    if array.is_null() { return ptr::null_mut(); }
    unsafe {
        for i in 0..count as isize {
            let item = cJSON_CreateNumber(*numbers.offset(i) as f64);
            if item.is_null() || cJSON_AddItemToArray(array, item) == 0 {
                cJSON_Delete(item);
                cJSON_Delete(array);
                return ptr::null_mut();
            }
        }
    }
    array
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let array = cJSON_CreateArray();
    if array.is_null() { return ptr::null_mut(); }
    unsafe {
        for i in 0..count as isize {
            let item = cJSON_CreateNumber(*numbers.offset(i));
            if item.is_null() || cJSON_AddItemToArray(array, item) == 0 {
                cJSON_Delete(item);
                cJSON_Delete(array);
                return ptr::null_mut();
            }
        }
    }
    array
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON {
    if count < 0 || strings.is_null() { return ptr::null_mut(); }
    let array = cJSON_CreateArray();
    if array.is_null() { return ptr::null_mut(); }
    unsafe {
        for i in 0..count as isize {
            let item = cJSON_CreateString(*strings.offset(i));
            if item.is_null() || cJSON_AddItemToArray(array, item) == 0 {
                cJSON_Delete(item);
                cJSON_Delete(array);
                return ptr::null_mut();
            }
        }
    }
    array
}

unsafe fn duplicate_item(item: *const cJSON, recurse: bool) -> *mut cJSON {
    if item.is_null() { return ptr::null_mut(); }
    let newitem = new_item();
    if newitem.is_null() { return ptr::null_mut(); }
    (*newitem).type_ = (*item).type_ & !CJSON_IS_REFERENCE;
    (*newitem).valueint = (*item).valueint;
    (*newitem).valuedouble = (*item).valuedouble;
    if !(*item).valuestring.is_null() {
        let s = CStr::from_ptr((*item).valuestring).to_string_lossy().into_owned();
        (*newitem).valuestring = dup_c_string(&s);
        if (*newitem).valuestring.is_null() { cJSON_Delete(newitem); return ptr::null_mut(); }
    }
    if !(*item).string.is_null() {
        if (*item).type_ & CJSON_STRING_IS_CONST != 0 {
            (*newitem).string = (*item).string;
        } else {
            let s = CStr::from_ptr((*item).string).to_string_lossy().into_owned();
            (*newitem).string = dup_c_string(&s);
            if (*newitem).string.is_null() { cJSON_Delete(newitem); return ptr::null_mut(); }
        }
    }
    if recurse {
        let mut child = (*item).child;
        while !child.is_null() {
            let dup = duplicate_item(child, true);
            if dup.is_null() || !append_child(newitem, dup) {
                cJSON_Delete(dup);
                cJSON_Delete(newitem);
                return ptr::null_mut();
            }
            child = (*child).next;
        }
    }
    newitem
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    unsafe { duplicate_item(item, recurse != 0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Minify(json: *mut c_char) {
    if json.is_null() {
        return;
    }
    unsafe {
        let src = CStr::from_ptr(json).to_string_lossy().into_owned();
        let minified = serde_json::from_str::<Value>(&src)
            .ok()
            .and_then(|v| serde_json::to_string(&v).ok())
            .unwrap_or(src);
        let bytes = minified.as_bytes();
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, json, bytes.len());
        *json.add(bytes.len()) = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_INVALID { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_FALSE { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_TRUE { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & (CJSON_TRUE | CJSON_FALSE)) != 0 { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_NULL { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_NUMBER { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_STRING { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_ARRAY { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_OBJECT { 1 } else { 0 } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    unsafe { if item.is_null() { 0 } else if ((*item).type_ & 0xFF) == CJSON_RAW { 1 } else { 0 } }
}

unsafe fn compare_items(a: *const cJSON, b: *const cJSON, case_sensitive: bool) -> bool {
    if a.is_null() || b.is_null() || ((*a).type_ & 0xFF) != ((*b).type_ & 0xFF) {
        return false;
    }
    if a == b {
        return true;
    }
    match (*a).type_ & 0xFF {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL => true,
        CJSON_NUMBER => ((*a).valuedouble - (*b).valuedouble).abs() <= f64::EPSILON * (*a).valuedouble.abs().max((*b).valuedouble.abs()),
        CJSON_STRING | CJSON_RAW => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() { return false; }
            CStr::from_ptr((*a).valuestring).to_bytes() == CStr::from_ptr((*b).valuestring).to_bytes()
        }
        CJSON_ARRAY => {
            let mut ca = (*a).child;
            let mut cb = (*b).child;
            while !ca.is_null() && !cb.is_null() {
                if !compare_items(ca, cb, case_sensitive) { return false; }
                ca = (*ca).next;
                cb = (*cb).next;
            }
            ca.is_null() && cb.is_null()
        }
        CJSON_OBJECT => {
            let mut ca = (*a).child;
            while !ca.is_null() {
                if (*ca).string.is_null() { return false; }
                let key = CStr::from_ptr((*ca).string).to_string_lossy().into_owned();
                let cb = get_object_item_mut(b as *mut cJSON, &key, case_sensitive);
                if cb.is_null() || !compare_items(ca, cb, case_sensitive) { return false; }
                ca = (*ca).next;
            }
            let mut cb = (*b).child;
            while !cb.is_null() {
                if (*cb).string.is_null() { return false; }
                let key = CStr::from_ptr((*cb).string).to_string_lossy().into_owned();
                let ca2 = get_object_item_mut(a as *mut cJSON, &key, case_sensitive);
                if ca2.is_null() || !compare_items(cb, ca2, case_sensitive) { return false; }
                cb = (*cb).next;
            }
            true
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: cJSON_bool) -> cJSON_bool {
    unsafe { if compare_items(a, b, case_sensitive != 0) { 1 } else { 0 } }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    unsafe {
        if object.is_null() {
            return number;
        }
        (*object).valueint = if number >= i32::MAX as f64 { i32::MAX } else if number <= i32::MIN as f64 { i32::MIN } else { number as i32 };
        (*object).valuedouble = number;
        number
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_SetValuestring(object: *mut cJSON, valuestring: *const c_char) -> *mut c_char {
    if object.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if (*object).type_ & CJSON_STRING == 0 || (*object).type_ & CJSON_IS_REFERENCE != 0 {
            return ptr::null_mut();
        }
        let Some(s) = cstr_to_string(valuestring) else { return ptr::null_mut(); };
        let newp = dup_c_string(&s);
        if newp.is_null() {
            return ptr::null_mut();
        }
        if !(*object).valuestring.is_null() {
            free_c_string((*object).valuestring);
        }
        (*object).valuestring = newp;
        newp
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_malloc(size: size_t) -> *mut c_void {
    unsafe { libc::malloc(size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_free(object: *mut c_void) {
    unsafe { libc::free(object) }
}
