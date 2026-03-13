use std::ffi::{c_char, c_int};
use std::ptr;
use crate::types::*;
use crate::helpers::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() { return 0; }
    let mut child = (*array).child;
    let mut size: usize = 0;
    while !child.is_null() {
        size += 1;
        child = (*child).next;
    }
    size as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 { return ptr::null_mut(); }
    get_array_item(array, index as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    get_object_item(object, string, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    get_object_item(object, string, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> cJSON_bool {
    if cJSON_GetObjectItem(object, string).is_null() { 0 } else { 1 }
}
