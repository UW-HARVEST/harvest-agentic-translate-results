#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_double, c_float, c_int, c_void};

pub type cJSON_bool = c_int;

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
    pub malloc_fn: Option<extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<extern "C" fn(*mut c_void)>,
}

#[repr(C)]
pub struct record {
    pub precision: *const c_char,
    pub lat: c_double,
    pub lon: c_double,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

unsafe extern "C" {
    #[link_name = "rust_internal_cJSON_GetErrorPtr"]
    fn inner_cJSON_GetErrorPtr() -> *const c_char;
    #[link_name = "rust_internal_cJSON_GetStringValue"]
    fn inner_cJSON_GetStringValue(item: *const cJSON) -> *mut c_char;
    #[link_name = "rust_internal_cJSON_GetNumberValue"]
    fn inner_cJSON_GetNumberValue(item: *const cJSON) -> c_double;
    #[link_name = "rust_internal_cJSON_Version"]
    fn inner_cJSON_Version() -> *const c_char;
    #[link_name = "rust_internal_cJSON_InitHooks"]
    fn inner_cJSON_InitHooks(hooks: *mut cJSON_Hooks);
    #[link_name = "rust_internal_cJSON_Delete"]
    fn inner_cJSON_Delete(item: *mut cJSON);
    #[link_name = "rust_internal_cJSON_SetNumberHelper"]
    fn inner_cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double;
    #[link_name = "rust_internal_cJSON_SetValuestring"]
    fn inner_cJSON_SetValuestring(object: *mut cJSON, valuestring: *const c_char) -> *mut c_char;
    #[link_name = "rust_internal_cJSON_ParseWithOpts"]
    fn inner_cJSON_ParseWithOpts(
        value: *const c_char,
        return_parse_end: *mut *const c_char,
        require_null_terminated: cJSON_bool,
    ) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_ParseWithLengthOpts"]
    fn inner_cJSON_ParseWithLengthOpts(
        value: *const c_char,
        buffer_length: usize,
        return_parse_end: *mut *const c_char,
        require_null_terminated: cJSON_bool,
    ) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_Parse"]
    fn inner_cJSON_Parse(value: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_ParseWithLength"]
    fn inner_cJSON_ParseWithLength(value: *const c_char, buffer_length: usize) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_Print"]
    fn inner_cJSON_Print(item: *const cJSON) -> *mut c_char;
    #[link_name = "rust_internal_cJSON_PrintUnformatted"]
    fn inner_cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char;
    #[link_name = "rust_internal_cJSON_PrintBuffered"]
    fn inner_cJSON_PrintBuffered(item: *const cJSON, prebuffer: c_int, fmt: cJSON_bool) -> *mut c_char;
    #[link_name = "rust_internal_cJSON_PrintPreallocated"]
    fn inner_cJSON_PrintPreallocated(
        item: *mut cJSON,
        buffer: *mut c_char,
        length: c_int,
        format: cJSON_bool,
    ) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_GetArraySize"]
    fn inner_cJSON_GetArraySize(array: *const cJSON) -> c_int;
    #[link_name = "rust_internal_cJSON_GetArrayItem"]
    fn inner_cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_GetObjectItem"]
    fn inner_cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_GetObjectItemCaseSensitive"]
    fn inner_cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_HasObjectItem"]
    fn inner_cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_AddItemToArray"]
    fn inner_cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_AddItemToObject"]
    fn inner_cJSON_AddItemToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_AddItemToObjectCS"]
    fn inner_cJSON_AddItemToObjectCS(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_AddItemReferenceToArray"]
    fn inner_cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_AddItemReferenceToObject"]
    fn inner_cJSON_AddItemReferenceToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_AddNullToObject"]
    fn inner_cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddTrueToObject"]
    fn inner_cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddFalseToObject"]
    fn inner_cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddBoolToObject"]
    fn inner_cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: cJSON_bool) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddNumberToObject"]
    fn inner_cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddStringToObject"]
    fn inner_cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddRawToObject"]
    fn inner_cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddObjectToObject"]
    fn inner_cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_AddArrayToObject"]
    fn inner_cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_DetachItemViaPointer"]
    fn inner_cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_DetachItemFromArray"]
    fn inner_cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_DeleteItemFromArray"]
    fn inner_cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int);
    #[link_name = "rust_internal_cJSON_DetachItemFromObject"]
    fn inner_cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_DetachItemFromObjectCaseSensitive"]
    fn inner_cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_DeleteItemFromObject"]
    fn inner_cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char);
    #[link_name = "rust_internal_cJSON_DeleteItemFromObjectCaseSensitive"]
    fn inner_cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char);
    #[link_name = "rust_internal_cJSON_InsertItemInArray"]
    fn inner_cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_ReplaceItemViaPointer"]
    fn inner_cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_ReplaceItemInArray"]
    fn inner_cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_ReplaceItemInObject"]
    fn inner_cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_ReplaceItemInObjectCaseSensitive"]
    fn inner_cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_CreateNull"]
    fn inner_cJSON_CreateNull() -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateTrue"]
    fn inner_cJSON_CreateTrue() -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateFalse"]
    fn inner_cJSON_CreateFalse() -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateBool"]
    fn inner_cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateNumber"]
    fn inner_cJSON_CreateNumber(num: c_double) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateString"]
    fn inner_cJSON_CreateString(string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateStringReference"]
    fn inner_cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateObjectReference"]
    fn inner_cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateArrayReference"]
    fn inner_cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateRaw"]
    fn inner_cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateArray"]
    fn inner_cJSON_CreateArray() -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateObject"]
    fn inner_cJSON_CreateObject() -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateIntArray"]
    fn inner_cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateFloatArray"]
    fn inner_cJSON_CreateFloatArray(numbers: *const c_float, count: c_int) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateDoubleArray"]
    fn inner_cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_CreateStringArray"]
    fn inner_cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_Duplicate"]
    fn inner_cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON;
    #[link_name = "rust_internal_cJSON_Minify"]
    fn inner_cJSON_Minify(json: *mut c_char);
    #[link_name = "rust_internal_cJSON_IsInvalid"]
    fn inner_cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsFalse"]
    fn inner_cJSON_IsFalse(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsTrue"]
    fn inner_cJSON_IsTrue(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsBool"]
    fn inner_cJSON_IsBool(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsNull"]
    fn inner_cJSON_IsNull(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsNumber"]
    fn inner_cJSON_IsNumber(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsString"]
    fn inner_cJSON_IsString(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsArray"]
    fn inner_cJSON_IsArray(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsObject"]
    fn inner_cJSON_IsObject(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_IsRaw"]
    fn inner_cJSON_IsRaw(item: *const cJSON) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_Compare"]
    fn inner_cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: cJSON_bool) -> cJSON_bool;
    #[link_name = "rust_internal_cJSON_malloc"]
    fn inner_cJSON_malloc(size: usize) -> *mut c_void;
    #[link_name = "rust_internal_cJSON_free"]
    fn inner_cJSON_free(object: *mut c_void);
    #[link_name = "rust_internal_driver"]
    fn inner_driver(
        strings: *const *const c_char,
        numbers: *mut [c_int; 3],
        ids: *mut c_int,
        fields: *mut record,
    ) -> c_int;
}

macro_rules! forward {
    ($(fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty => $inner:ident;)+) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
                unsafe { $inner($($arg),*) }
            }
        )+
    };
    ($(fn $name:ident($($arg:ident: $ty:ty),*) => $inner:ident;)+) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name($($arg: $ty),*) {
                unsafe { $inner($($arg),*) }
            }
        )+
    };
}

forward! {
    fn cJSON_GetErrorPtr() -> *const c_char => inner_cJSON_GetErrorPtr;
    fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char => inner_cJSON_GetStringValue;
    fn cJSON_GetNumberValue(item: *const cJSON) -> c_double => inner_cJSON_GetNumberValue;
    fn cJSON_Version() -> *const c_char => inner_cJSON_Version;
    fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double => inner_cJSON_SetNumberHelper;
    fn cJSON_SetValuestring(object: *mut cJSON, valuestring: *const c_char) -> *mut c_char => inner_cJSON_SetValuestring;
    fn cJSON_ParseWithOpts(value: *const c_char, return_parse_end: *mut *const c_char, require_null_terminated: cJSON_bool) -> *mut cJSON => inner_cJSON_ParseWithOpts;
    fn cJSON_ParseWithLengthOpts(value: *const c_char, buffer_length: usize, return_parse_end: *mut *const c_char, require_null_terminated: cJSON_bool) -> *mut cJSON => inner_cJSON_ParseWithLengthOpts;
    fn cJSON_Parse(value: *const c_char) -> *mut cJSON => inner_cJSON_Parse;
    fn cJSON_ParseWithLength(value: *const c_char, buffer_length: usize) -> *mut cJSON => inner_cJSON_ParseWithLength;
    fn cJSON_Print(item: *const cJSON) -> *mut c_char => inner_cJSON_Print;
    fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char => inner_cJSON_PrintUnformatted;
    fn cJSON_PrintBuffered(item: *const cJSON, prebuffer: c_int, fmt: cJSON_bool) -> *mut c_char => inner_cJSON_PrintBuffered;
    fn cJSON_PrintPreallocated(item: *mut cJSON, buffer: *mut c_char, length: c_int, format: cJSON_bool) -> cJSON_bool => inner_cJSON_PrintPreallocated;
    fn cJSON_GetArraySize(array: *const cJSON) -> c_int => inner_cJSON_GetArraySize;
    fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON => inner_cJSON_GetArrayItem;
    fn cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON => inner_cJSON_GetObjectItem;
    fn cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON => inner_cJSON_GetObjectItemCaseSensitive;
    fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> cJSON_bool => inner_cJSON_HasObjectItem;
    fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool => inner_cJSON_AddItemToArray;
    fn cJSON_AddItemToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool => inner_cJSON_AddItemToObject;
    fn cJSON_AddItemToObjectCS(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool => inner_cJSON_AddItemToObjectCS;
    fn cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool => inner_cJSON_AddItemReferenceToArray;
    fn cJSON_AddItemReferenceToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool => inner_cJSON_AddItemReferenceToObject;
    fn cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON => inner_cJSON_AddNullToObject;
    fn cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON => inner_cJSON_AddTrueToObject;
    fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON => inner_cJSON_AddFalseToObject;
    fn cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: cJSON_bool) -> *mut cJSON => inner_cJSON_AddBoolToObject;
    fn cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON => inner_cJSON_AddNumberToObject;
    fn cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON => inner_cJSON_AddStringToObject;
    fn cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON => inner_cJSON_AddRawToObject;
    fn cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON => inner_cJSON_AddObjectToObject;
    fn cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON => inner_cJSON_AddArrayToObject;
    fn cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON => inner_cJSON_DetachItemViaPointer;
    fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON => inner_cJSON_DetachItemFromArray;
    fn cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON => inner_cJSON_DetachItemFromObject;
    fn cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON => inner_cJSON_DetachItemFromObjectCaseSensitive;
    fn cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool => inner_cJSON_InsertItemInArray;
    fn cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> cJSON_bool => inner_cJSON_ReplaceItemViaPointer;
    fn cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool => inner_cJSON_ReplaceItemInArray;
    fn cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool => inner_cJSON_ReplaceItemInObject;
    fn cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool => inner_cJSON_ReplaceItemInObjectCaseSensitive;
    fn cJSON_CreateNull() -> *mut cJSON => inner_cJSON_CreateNull;
    fn cJSON_CreateTrue() -> *mut cJSON => inner_cJSON_CreateTrue;
    fn cJSON_CreateFalse() -> *mut cJSON => inner_cJSON_CreateFalse;
    fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON => inner_cJSON_CreateBool;
    fn cJSON_CreateNumber(num: c_double) -> *mut cJSON => inner_cJSON_CreateNumber;
    fn cJSON_CreateString(string: *const c_char) -> *mut cJSON => inner_cJSON_CreateString;
    fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON => inner_cJSON_CreateStringReference;
    fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON => inner_cJSON_CreateObjectReference;
    fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON => inner_cJSON_CreateArrayReference;
    fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON => inner_cJSON_CreateRaw;
    fn cJSON_CreateArray() -> *mut cJSON => inner_cJSON_CreateArray;
    fn cJSON_CreateObject() -> *mut cJSON => inner_cJSON_CreateObject;
    fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON => inner_cJSON_CreateIntArray;
    fn cJSON_CreateFloatArray(numbers: *const c_float, count: c_int) -> *mut cJSON => inner_cJSON_CreateFloatArray;
    fn cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON => inner_cJSON_CreateDoubleArray;
    fn cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON => inner_cJSON_CreateStringArray;
    fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON => inner_cJSON_Duplicate;
    fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsInvalid;
    fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsFalse;
    fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsTrue;
    fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsBool;
    fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsNull;
    fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsNumber;
    fn cJSON_IsString(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsString;
    fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsArray;
    fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsObject;
    fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool => inner_cJSON_IsRaw;
    fn cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: cJSON_bool) -> cJSON_bool => inner_cJSON_Compare;
    fn cJSON_malloc(size: usize) -> *mut c_void => inner_cJSON_malloc;
    fn driver(strings: *const *const c_char, numbers: *mut [c_int; 3], ids: *mut c_int, fields: *mut record) -> c_int => inner_driver;
}

forward! {
    fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) => inner_cJSON_InitHooks;
    fn cJSON_Delete(item: *mut cJSON) => inner_cJSON_Delete;
    fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) => inner_cJSON_DeleteItemFromArray;
    fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) => inner_cJSON_DeleteItemFromObject;
    fn cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) => inner_cJSON_DeleteItemFromObjectCaseSensitive;
    fn cJSON_Minify(json: *mut c_char) => inner_cJSON_Minify;
    fn cJSON_free(object: *mut c_void) => inner_cJSON_free;
}
