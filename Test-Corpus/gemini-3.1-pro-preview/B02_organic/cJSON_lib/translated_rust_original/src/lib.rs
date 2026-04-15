pub mod test;

use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::ptr;

pub const CJSON_INVALID: c_int = 0;
pub const CJSON_FALSE: c_int = 1 << 0;
pub const CJSON_TRUE: c_int = 1 << 1;
pub const CJSON_NULL: c_int = 1 << 2;
pub const CJSON_NUMBER: c_int = 1 << 3;
pub const CJSON_STRING: c_int = 1 << 4;
pub const CJSON_ARRAY: c_int = 1 << 5;
pub const CJSON_OBJECT: c_int = 1 << 6;
pub const CJSON_RAW: c_int = 1 << 7;

pub const CJSON_ISREFERENCE: c_int = 256;
pub const CJSON_STRINGISCONST: c_int = 512;

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
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

static mut GLOBAL_HOOKS: cJSON_Hooks = cJSON_Hooks {
    malloc_fn: None,
    free_fn: None,
};

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    unsafe {
        if hooks.is_null() {
            GLOBAL_HOOKS.malloc_fn = None;
            GLOBAL_HOOKS.free_fn = None;
        } else {
            GLOBAL_HOOKS.malloc_fn = (*hooks).malloc_fn;
            GLOBAL_HOOKS.free_fn = (*hooks).free_fn;
        }
    }
}

unsafe fn internal_malloc(size: usize) -> *mut c_void {
    if let Some(f) = GLOBAL_HOOKS.malloc_fn {
        f(size)
    } else {
        libc::malloc(size)
    }
}

unsafe fn internal_free(ptr: *mut c_void) {
    if let Some(f) = GLOBAL_HOOKS.free_fn {
        f(ptr)
    } else {
        libc::free(ptr)
    }
}

unsafe fn cJSON_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    let len = libc::strlen(s) + 1;
    let copy = internal_malloc(len) as *mut c_char;
    if !copy.is_null() {
        ptr::copy_nonoverlapping(s, copy, len);
    }
    copy
}

fn cJSON_New_Item() -> *mut cJSON {
    unsafe {
        let ptr = internal_malloc(std::mem::size_of::<cJSON>()) as *mut cJSON;
        if !ptr.is_null() {
            ptr::write_bytes(ptr, 0, 1);
        }
        ptr
    }
}

fn value_to_cjson(v: &serde_json::Value) -> *mut cJSON {
    match v {
        serde_json::Value::Null => cJSON_CreateNull(),
        serde_json::Value::Bool(b) => cJSON_CreateBool(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                cJSON_CreateNumber(f)
            } else {
                cJSON_CreateNull()
            }
        }
        serde_json::Value::String(s) => {
            let c_str = CString::new(s.as_str()).unwrap_or_default();
            cJSON_CreateString(c_str.as_ptr())
        }
        serde_json::Value::Array(arr) => {
            let c_arr = cJSON_CreateArray();
            for item in arr {
                let c_item = value_to_cjson(item);
                unsafe { cJSON_AddItemToArray(c_arr, c_item) };
            }
            c_arr
        }
        serde_json::Value::Object(obj) => {
            let c_obj = cJSON_CreateObject();
            for (k, v) in obj {
                let c_k = CString::new(k.as_str()).unwrap_or_default();
                let c_v = value_to_cjson(v);
                unsafe { cJSON_AddItemToObject(c_obj, c_k.as_ptr(), c_v) };
            }
            c_obj
        }
    }
}

unsafe fn cjson_to_value(item: *const cJSON) -> serde_json::Value {
    if item.is_null() {
        return serde_json::Value::Null;
    }
    let t = (*item).type_ & 0xFF;
    match t {
        CJSON_NULL => serde_json::Value::Null,
        CJSON_FALSE => serde_json::Value::Bool(false),
        CJSON_TRUE => serde_json::Value::Bool(true),
        CJSON_NUMBER => {
            if let Some(n) = serde_json::Number::from_f64((*item).valuedouble) {
                serde_json::Value::Number(n)
            } else {
                serde_json::Value::Null
            }
        }
        CJSON_STRING | CJSON_RAW => {
            if (*item).valuestring.is_null() {
                serde_json::Value::String(String::new())
            } else {
                let s = CStr::from_ptr((*item).valuestring).to_string_lossy().into_owned();
                serde_json::Value::String(s)
            }
        }
        CJSON_ARRAY => {
            let mut arr = Vec::new();
            let mut child = (*item).child;
            while !child.is_null() {
                arr.push(cjson_to_value(child));
                child = (*child).next;
            }
            serde_json::Value::Array(arr)
        }
        CJSON_OBJECT => {
            let mut obj = serde_json::Map::new();
            let mut child = (*item).child;
            while !child.is_null() {
                let key = if (*child).string.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr((*child).string).to_string_lossy().into_owned()
                };
                obj.insert(key, cjson_to_value(child));
                child = (*child).next;
            }
            serde_json::Value::Object(obj)
        }
        _ => serde_json::Value::Null,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ParseWithLength(value: *const c_char, buffer_length: usize) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: c_int,
) -> *mut cJSON {
    if value.is_null() {
        return ptr::null_mut();
    }
    let c_str = unsafe { CStr::from_ptr(value) };
    let s = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    
    let mut stream = serde_json::Deserializer::from_str(s).into_iter::<serde_json::Value>();
    let parsed = match stream.next() {
        Some(Ok(v)) => v,
        _ => return ptr::null_mut(),
    };
    
    let offset = stream.byte_offset();
    if require_null_terminated != 0 {
        let remainder = &s[offset..];
        if !remainder.trim().is_empty() {
            return ptr::null_mut();
        }
    }
    
    if !return_parse_end.is_null() {
        unsafe {
            *return_parse_end = value.add(offset);
        }
    }
    
    value_to_cjson(&parsed)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: c_int,
) -> *mut cJSON {
    if value.is_null() || buffer_length == 0 {
        return ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(value as *const u8, buffer_length) };
    let s = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let s = s.trim_end_matches('\0');
    
    let mut stream = serde_json::Deserializer::from_str(s).into_iter::<serde_json::Value>();
    let parsed = match stream.next() {
        Some(Ok(v)) => v,
        _ => return ptr::null_mut(),
    };
    
    let offset = stream.byte_offset();
    if require_null_terminated != 0 {
        let remainder = &s[offset..];
        if !remainder.trim().is_empty() {
            return ptr::null_mut();
        }
    }
    
    if !return_parse_end.is_null() {
        unsafe {
            *return_parse_end = value.add(offset);
        }
    }
    
    value_to_cjson(&parsed)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    let v = unsafe { cjson_to_value(item) };
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    v.serialize(&mut ser).unwrap();
    let s = String::from_utf8(buf).unwrap_or_default();
    let c_str = CString::new(s).unwrap_or_default();
    unsafe { cJSON_strdup(c_str.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    let v = unsafe { cjson_to_value(item) };
    let s = serde_json::to_string(&v).unwrap_or_default();
    let c_str = CString::new(s).unwrap_or_default();
    unsafe { cJSON_strdup(c_str.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_PrintBuffered(item: *const cJSON, prebuffer: c_int, fmt: c_int) -> *mut c_char {
    let _ = prebuffer;
    if fmt != 0 {
        cJSON_Print(item)
    } else {
        cJSON_PrintUnformatted(item)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_PrintPreallocated(item: *mut cJSON, buffer: *mut c_char, length: c_int, format: c_int) -> c_int {
    let s = if format != 0 {
        cJSON_Print(item)
    } else {
        cJSON_PrintUnformatted(item)
    };
    if s.is_null() {
        return 0;
    }
    unsafe {
        let len = libc::strlen(s);
        if len + 1 > length as usize {
            internal_free(s as *mut c_void);
            return 0;
        }
        ptr::copy_nonoverlapping(s, buffer, len + 1);
        internal_free(s as *mut c_void);
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Delete(item: *mut cJSON) {
    let mut current = item;
    while !current.is_null() {
        unsafe {
            let next = (*current).next;
            if ((*current).type_ & CJSON_ISREFERENCE) == 0 && !(*current).child.is_null() {
                cJSON_Delete((*current).child);
            }
            if ((*current).type_ & CJSON_ISREFERENCE) == 0 && !(*current).valuestring.is_null() {
                internal_free((*current).valuestring as *mut c_void);
            }
            if ((*current).type_ & CJSON_STRINGISCONST) == 0 && !(*current).string.is_null() {
                internal_free((*current).string as *mut c_void);
            }
            internal_free(current as *mut c_void);
            current = next;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() {
        return 0;
    }
    let mut size = 0;
    unsafe {
        let mut child = (*array).child;
        while !child.is_null() {
            size += 1;
            child = (*child).next;
        }
    }
    size
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if array.is_null() || index < 0 {
        return ptr::null_mut();
    }
    unsafe {
        let mut child = (*array).child;
        let mut i = 0;
        while !child.is_null() && i < index {
            child = (*child).next;
            i += 1;
        }
        child
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    if object.is_null() || string.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let mut child = (*object).child;
        while !child.is_null() {
            if !(*child).string.is_null() && libc::strcasecmp((*child).string, string) == 0 {
                return child;
            }
            child = (*child).next;
        }
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    if object.is_null() || string.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let mut child = (*object).child;
        while !child.is_null() {
            if !(*child).string.is_null() && libc::strcmp((*child).string, string) == 0 {
                return child;
            }
            child = (*child).next;
        }
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> c_int {
    if !cJSON_GetObjectItem(object, string).is_null() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if item.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if ((*item).type_ & 0xFF) == CJSON_STRING {
            (*item).valuestring
        } else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    if item.is_null() {
        return f64::NAN;
    }
    unsafe {
        if ((*item).type_ & 0xFF) == CJSON_NUMBER {
            (*item).valuedouble
        } else {
            f64::NAN
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_INVALID { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsFalse(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_FALSE { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsTrue(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_TRUE { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsBool(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { let t = (*item).type_ & 0xFF; if t == CJSON_TRUE || t == CJSON_FALSE { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsNull(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_NULL { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsNumber(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_NUMBER { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsString(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_STRING { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsArray(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_ARRAY { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsObject(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_OBJECT { 1 } else { 0 } } }
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_IsRaw(item: *const cJSON) -> c_int {
    if item.is_null() { 0 } else { unsafe { if ((*item).type_ & 0xFF) == CJSON_RAW { 1 } else { 0 } } }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() { unsafe { (*item).type_ = CJSON_NULL; } }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() { unsafe { (*item).type_ = CJSON_TRUE; } }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() { unsafe { (*item).type_ = CJSON_FALSE; } }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateBool(boolean: c_int) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() { unsafe { (*item).type_ = if boolean != 0 { CJSON_TRUE } else { CJSON_FALSE }; } }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() {
        unsafe {
            (*item).type_ = CJSON_NUMBER;
            (*item).valuedouble = num;
            (*item).valueint = num as c_int;
        }
    }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() {
        unsafe {
            (*item).type_ = CJSON_STRING;
            (*item).valuestring = cJSON_strdup(string);
            if (*item).valuestring.is_null() {
                cJSON_Delete(item);
                return ptr::null_mut();
            }
        }
    }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() {
        unsafe {
            (*item).type_ = CJSON_RAW;
            (*item).valuestring = cJSON_strdup(raw);
        }
    }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() { unsafe { (*item).type_ = CJSON_ARRAY; } }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() { unsafe { (*item).type_ = CJSON_OBJECT; } }
    item
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() {
        unsafe {
            (*item).type_ = CJSON_STRING | CJSON_ISREFERENCE;
            (*item).valuestring = string as *mut c_char;
        }
    }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() {
        unsafe {
            (*item).type_ = CJSON_OBJECT | CJSON_ISREFERENCE;
            (*item).child = child as *mut cJSON;
        }
    }
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item();
    if !item.is_null() {
        unsafe {
            (*item).type_ = CJSON_ARRAY | CJSON_ISREFERENCE;
            (*item).child = child as *mut cJSON;
        }
    }
    item
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    let arr = cJSON_CreateArray();
    for i in 0..count {
        unsafe {
            let num = *numbers.add(i as usize);
            cJSON_AddItemToArray(arr, cJSON_CreateNumber(num as c_double));
        }
    }
    arr
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateFloatArray(numbers: *const f32, count: c_int) -> *mut cJSON {
    let arr = cJSON_CreateArray();
    for i in 0..count {
        unsafe {
            let num = *numbers.add(i as usize);
            cJSON_AddItemToArray(arr, cJSON_CreateNumber(num as c_double));
        }
    }
    arr
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON {
    let arr = cJSON_CreateArray();
    for i in 0..count {
        unsafe {
            let num = *numbers.add(i as usize);
            cJSON_AddItemToArray(arr, cJSON_CreateNumber(num));
        }
    }
    arr
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON {
    let arr = cJSON_CreateArray();
    for i in 0..count {
        unsafe {
            let s = *strings.add(i as usize);
            cJSON_AddItemToArray(arr, cJSON_CreateString(s));
        }
    }
    arr
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> c_int {
    if array.is_null() || item.is_null() {
        return 0;
    }
    unsafe {
        let mut child = (*array).child;
        if child.is_null() {
            (*array).child = item;
            (*item).prev = item;
            (*item).next = ptr::null_mut();
        } else {
            let prev = (*child).prev;
            if !prev.is_null() {
                (*prev).next = item;
                (*item).prev = prev;
                (*child).prev = item;
            }
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> c_int {
    if object.is_null() || string.is_null() || item.is_null() {
        return 0;
    }
    unsafe {
        if ((*item).type_ & CJSON_STRINGISCONST) == 0 && !(*item).string.is_null() {
            internal_free((*item).string as *mut c_void);
        }
        (*item).string = cJSON_strdup(string);
        (*item).type_ &= !CJSON_STRINGISCONST;
        cJSON_AddItemToArray(object, item)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> c_int {
    if object.is_null() || string.is_null() || item.is_null() {
        return 0;
    }
    unsafe {
        if ((*item).type_ & CJSON_STRINGISCONST) == 0 && !(*item).string.is_null() {
            internal_free((*item).string as *mut c_void);
        }
        (*item).string = string as *mut c_char;
        (*item).type_ |= CJSON_STRINGISCONST;
        cJSON_AddItemToArray(object, item)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> c_int {
    if array.is_null() || item.is_null() {
        return 0;
    }
    let ref_item = cJSON_New_Item();
    if ref_item.is_null() {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(item, ref_item, 1);
        (*ref_item).string = ptr::null_mut();
        (*ref_item).type_ |= CJSON_ISREFERENCE;
        (*ref_item).next = ptr::null_mut();
        (*ref_item).prev = ptr::null_mut();
        cJSON_AddItemToArray(array, ref_item)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> c_int {
    if object.is_null() || string.is_null() || item.is_null() {
        return 0;
    }
    let ref_item = cJSON_New_Item();
    if ref_item.is_null() {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(item, ref_item, 1);
        (*ref_item).string = cJSON_strdup(string);
        (*ref_item).type_ |= CJSON_ISREFERENCE;
        (*ref_item).type_ &= !CJSON_STRINGISCONST;
        (*ref_item).next = ptr::null_mut();
        (*ref_item).prev = ptr::null_mut();
        cJSON_AddItemToArray(object, ref_item)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON {
    if parent.is_null() || item.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if item != (*parent).child && (*item).prev.is_null() {
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
        } else if (*item).next.is_null() {
            (*(*parent).child).prev = (*item).prev;
        }
        (*item).prev = ptr::null_mut();
        (*item).next = ptr::null_mut();
    }
    item
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    cJSON_DetachItemViaPointer(array, cJSON_GetArrayItem(array, which))
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    cJSON_DetachItemViaPointer(object, cJSON_GetObjectItem(object, string))
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    cJSON_DetachItemViaPointer(object, cJSON_GetObjectItemCaseSensitive(object, string))
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> c_int {
    if which < 0 || newitem.is_null() {
        return 0;
    }
    let after_inserted = cJSON_GetArrayItem(array, which);
    if after_inserted.is_null() {
        return cJSON_AddItemToArray(array, newitem);
    }
    unsafe {
        (*newitem).next = after_inserted;
        (*newitem).prev = (*after_inserted).prev;
        (*after_inserted).prev = newitem;
        if after_inserted == (*array).child {
            (*array).child = newitem;
        } else {
            (*(*newitem).prev).next = newitem;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> c_int {
    if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
        return 0;
    }
    if item == replacement {
        return 1;
    }
    unsafe {
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
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> c_int {
    if which < 0 {
        return 0;
    }
    cJSON_ReplaceItemViaPointer(array, cJSON_GetArrayItem(array, which), newitem)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> c_int {
    if newitem.is_null() || string.is_null() {
        return 0;
    }
    unsafe {
        if ((*newitem).type_ & CJSON_STRINGISCONST) == 0 && !(*newitem).string.is_null() {
            internal_free((*newitem).string as *mut c_void);
        }
        (*newitem).string = cJSON_strdup(string);
        (*newitem).type_ &= !CJSON_STRINGISCONST;
    }
    cJSON_ReplaceItemViaPointer(object, cJSON_GetObjectItem(object, string), newitem)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> c_int {
    if newitem.is_null() || string.is_null() {
        return 0;
    }
    unsafe {
        if ((*newitem).type_ & CJSON_STRINGISCONST) == 0 && !(*newitem).string.is_null() {
            internal_free((*newitem).string as *mut c_void);
        }
        (*newitem).string = cJSON_strdup(string);
        (*newitem).type_ &= !CJSON_STRINGISCONST;
    }
    cJSON_ReplaceItemViaPointer(object, cJSON_GetObjectItemCaseSensitive(object, string), newitem)
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: c_int) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }
    let newitem = cJSON_New_Item();
    if newitem.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        (*newitem).type_ = (*item).type_ & !CJSON_ISREFERENCE;
        (*newitem).valueint = (*item).valueint;
        (*newitem).valuedouble = (*item).valuedouble;
        if !(*item).valuestring.is_null() {
            (*newitem).valuestring = cJSON_strdup((*item).valuestring);
        }
        if !(*item).string.is_null() {
            (*newitem).string = if ((*item).type_ & CJSON_STRINGISCONST) != 0 {
                (*item).string
            } else {
                cJSON_strdup((*item).string)
            };
        }
        if recurse == 0 {
            return newitem;
        }
        let mut child = (*item).child;
        let mut next: *mut cJSON = ptr::null_mut();
        let mut newchild: *mut cJSON = ptr::null_mut();
        while !child.is_null() {
            newchild = cJSON_Duplicate(child, 1);
            if newchild.is_null() {
                cJSON_Delete(newitem);
                return ptr::null_mut();
            }
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
        if !(*newitem).child.is_null() {
            (*(*newitem).child).prev = newchild;
        }
    }
    newitem
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: c_int) -> c_int {
    if a == b {
        return 1;
    }
    if a.is_null() || b.is_null() {
        return 0;
    }
    unsafe {
        if ((*a).type_ & 0xFF) != ((*b).type_ & 0xFF) {
            return 0;
        }
        match (*a).type_ & 0xFF {
            CJSON_FALSE | CJSON_TRUE | CJSON_NULL => 1,
            CJSON_NUMBER => if (*a).valuedouble == (*b).valuedouble { 1 } else { 0 },
            CJSON_STRING | CJSON_RAW => {
                if (*a).valuestring.is_null() || (*b).valuestring.is_null() {
                    0
                } else if libc::strcmp((*a).valuestring, (*b).valuestring) == 0 {
                    1
                } else {
                    0
                }
            }
            CJSON_ARRAY => {
                let mut a_child = (*a).child;
                let mut b_child = (*b).child;
                while !a_child.is_null() && !b_child.is_null() {
                    if cJSON_Compare(a_child, b_child, case_sensitive) == 0 {
                        return 0;
                    }
                    a_child = (*a_child).next;
                    b_child = (*b_child).next;
                }
                if a_child.is_null() && b_child.is_null() { 1 } else { 0 }
            }
            CJSON_OBJECT => {
                let mut a_child = (*a).child;
                while !a_child.is_null() {
                    let b_child = if case_sensitive != 0 {
                        cJSON_GetObjectItemCaseSensitive(b, (*a_child).string)
                    } else {
                        cJSON_GetObjectItem(b, (*a_child).string)
                    };
                    if b_child.is_null() || cJSON_Compare(a_child, b_child, case_sensitive) == 0 {
                        return 0;
                    }
                    a_child = (*a_child).next;
                }
                let mut b_child = (*b).child;
                while !b_child.is_null() {
                    let a_child = if case_sensitive != 0 {
                        cJSON_GetObjectItemCaseSensitive(a, (*b_child).string)
                    } else {
                        cJSON_GetObjectItem(a, (*b_child).string)
                    };
                    if a_child.is_null() {
                        return 0;
                    }
                    b_child = (*b_child).next;
                }
                1
            }
            _ => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Minify(json: *mut c_char) {
    if json.is_null() {
        return;
    }
    unsafe {
        let mut src = json;
        let mut dst = json;
        let mut in_string = false;
        while *src != 0 {
            let c = *src;
            if c == b'"' as c_char {
                in_string = !in_string;
            }
            if in_string || (c != b' ' as c_char && c != b'\t' as c_char && c != b'\n' as c_char && c != b'\r' as c_char) {
                *dst = c;
                dst = dst.add(1);
            }
            src = src.add(1);
        }
        *dst = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateNull();
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateTrue();
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateFalse();
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: c_int) -> *mut cJSON {
    let item = cJSON_CreateBool(boolean);
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON {
    let item = cJSON_CreateNumber(number);
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateString(string);
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateRaw(raw);
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateObject();
    cJSON_AddItemToObject(object, name, item);
    item
}
#[unsafe(no_mangle)]
pub extern "C" fn cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let item = cJSON_CreateArray();
    cJSON_AddItemToObject(object, name, item);
    item
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if !object.is_null() {
        unsafe {
            (*object).valuedouble = number;
            (*object).valueint = number as c_int;
        }
    }
    number
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_SetValuestring(object: *mut cJSON, valuestring: *const c_char) -> *mut c_char {
    if object.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if ((*object).type_ & CJSON_STRING) == 0 || ((*object).type_ & CJSON_ISREFERENCE) != 0 {
            return ptr::null_mut();
        }
        if !(*object).valuestring.is_null() {
            internal_free((*object).valuestring as *mut c_void);
        }
        (*object).valuestring = cJSON_strdup(valuestring);
        (*object).valuestring
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_Version() -> *const c_char {
    c"1.7.19".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    unsafe { internal_malloc(size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn cJSON_free(object: *mut c_void) {
    unsafe { internal_free(object) }
}
