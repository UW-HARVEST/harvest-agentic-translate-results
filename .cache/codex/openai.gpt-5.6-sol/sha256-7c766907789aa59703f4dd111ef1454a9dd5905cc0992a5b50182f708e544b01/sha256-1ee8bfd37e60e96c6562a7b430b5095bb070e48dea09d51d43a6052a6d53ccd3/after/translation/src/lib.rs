#![allow(non_camel_case_types, non_snake_case)]

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_double, c_float, c_int, c_uchar, c_uint, c_void};
use std::ptr;

const CJSON_INVALID: c_int = 0;
const CJSON_FALSE: c_int = 1;
const CJSON_TRUE: c_int = 2;
const CJSON_NULL: c_int = 4;
const CJSON_NUMBER: c_int = 8;
const CJSON_STRING: c_int = 16;
const CJSON_ARRAY: c_int = 32;
const CJSON_OBJECT: c_int = 64;
const CJSON_RAW: c_int = 128;
const CJSON_IS_REFERENCE: c_int = 256;
const CJSON_STRING_IS_CONST: c_int = 512;
const CJSON_NESTING_LIMIT: usize = 1000;
const CJSON_CIRCULAR_LIMIT: usize = 10000;

#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub r#type: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

pub type cJSON_bool = c_int;
type Allocate = unsafe extern "C" fn(usize) -> *mut c_void;
type Deallocate = unsafe extern "C" fn(*mut c_void);
type Reallocate = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<Allocate>,
    pub free_fn: Option<Deallocate>,
}

#[derive(Clone, Copy)]
struct InternalHooks {
    allocate: Allocate,
    deallocate: Deallocate,
    reallocate: Option<Reallocate>,
}

struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}

impl<T> Global<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    unsafe fn get(&self) -> *mut T {
        self.0.get()
    }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn strlen(string: *const c_char) -> usize;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strncmp(left: *const c_char, right: *const c_char, count: usize) -> c_int;
    fn strcpy(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
    fn memset(destination: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn strtod(string: *const c_char, end: *mut *mut c_char) -> c_double;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn sscanf(buffer: *const c_char, format: *const c_char, ...) -> c_int;
    fn tolower(character: c_int) -> c_int;
    fn fabs(number: c_double) -> c_double;
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
    fn exit(status: c_int) -> !;
}

static GLOBAL_HOOKS: Global<InternalHooks> = Global::new(InternalHooks {
    allocate: malloc,
    deallocate: free,
    reallocate: Some(realloc),
});

#[derive(Clone, Copy)]
struct Error {
    json: *const c_uchar,
    position: usize,
}

static GLOBAL_ERROR: Global<Error> = Global::new(Error {
    json: ptr::null(),
    position: 0,
});

static VERSION: Global<[c_char; 15]> = Global::new([0; 15]);

#[inline]
unsafe fn hooks() -> InternalHooks {
    unsafe { *GLOBAL_HOOKS.get() }
}

unsafe fn cjson_strdup(string: *const c_uchar, hooks: &InternalHooks) -> *mut c_uchar {
    if string.is_null() {
        return ptr::null_mut();
    }
    let length = unsafe { strlen(string.cast()) } + 1;
    let copy = unsafe { (hooks.allocate)(length) }.cast::<c_uchar>();
    if copy.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        memcpy(copy.cast(), string.cast(), length);
    }
    copy
}

unsafe fn new_item(hooks: &InternalHooks) -> *mut cJSON {
    let node = unsafe { (hooks.allocate)(size_of::<cJSON>()) }.cast::<cJSON>();
    if !node.is_null() {
        unsafe {
            memset(node.cast(), 0, size_of::<cJSON>());
        }
    }
    node
}

#[inline]
fn saturated_int(number: c_double) -> c_int {
    if number.is_nan() {
        c_int::MIN
    } else if number >= c_int::MAX as c_double {
        c_int::MAX
    } else if number <= c_int::MIN as c_double {
        c_int::MIN
    } else {
        number as c_int
    }
}

unsafe fn case_insensitive_strcmp(mut left: *const c_uchar, mut right: *const c_uchar) -> c_int {
    if left.is_null() || right.is_null() {
        return 1;
    }
    if left == right {
        return 0;
    }
    unsafe {
        while tolower(*left as c_int) == tolower(*right as c_int) {
            if *left == 0 {
                return 0;
            }
            left = left.add(1);
            right = right.add(1);
        }
        tolower(*left as c_int) - tolower(*right as c_int)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    let error = unsafe { *GLOBAL_ERROR.get() };
    error.json.wrapping_add(error.position).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    let version = unsafe { (*VERSION.get()).as_mut_ptr() };
    unsafe {
        snprintf(version, 15, c"%i.%i.%i".as_ptr(), 1, 7, 19);
    }
    version
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(input: *mut cJSON_Hooks) {
    let global = unsafe { &mut *GLOBAL_HOOKS.get() };
    if input.is_null() {
        *global = InternalHooks {
            allocate: malloc,
            deallocate: free,
            reallocate: Some(realloc),
        };
        return;
    }

    let input = unsafe { &*input };
    global.allocate = input.malloc_fn.unwrap_or(malloc);
    global.deallocate = input.free_fn.unwrap_or(free);
    global.reallocate = None;
    if std::ptr::fn_addr_eq(global.allocate, malloc as Allocate)
        && std::ptr::fn_addr_eq(global.deallocate, free as Deallocate)
    {
        global.reallocate = Some(realloc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    let global = unsafe { hooks() };
    while !item.is_null() {
        let next = unsafe { (*item).next };
        if unsafe { (*item).r#type & CJSON_IS_REFERENCE == 0 && !(*item).child.is_null() } {
            unsafe { cJSON_Delete((*item).child) };
        }
        if unsafe { (*item).r#type & CJSON_IS_REFERENCE == 0 && !(*item).valuestring.is_null() } {
            unsafe { (global.deallocate)((*item).valuestring.cast()) };
            unsafe { (*item).valuestring = ptr::null_mut() };
        }
        if unsafe { (*item).r#type & CJSON_STRING_IS_CONST == 0 && !(*item).string.is_null() } {
            unsafe { (global.deallocate)((*item).string.cast()) };
            unsafe { (*item).string = ptr::null_mut() };
        }
        unsafe { (global.deallocate)(item.cast()) };
        item = next;
    }
}

macro_rules! type_predicate {
    ($name:ident, $kind:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(item: *const cJSON) -> cJSON_bool {
            if item.is_null() {
                return 0;
            }
            (unsafe { (*item).r#type } & 0xff == $kind) as c_int
        }
    };
}

type_predicate!(cJSON_IsInvalid, CJSON_INVALID);
type_predicate!(cJSON_IsFalse, CJSON_FALSE);
type_predicate!(cJSON_IsTrue, CJSON_TRUE);
type_predicate!(cJSON_IsNull, CJSON_NULL);
type_predicate!(cJSON_IsNumber, CJSON_NUMBER);
type_predicate!(cJSON_IsString, CJSON_STRING);
type_predicate!(cJSON_IsArray, CJSON_ARRAY);
type_predicate!(cJSON_IsObject, CJSON_OBJECT);
type_predicate!(cJSON_IsRaw, CJSON_RAW);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    (unsafe { (*item).r#type } & (CJSON_TRUE | CJSON_FALSE) != 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if unsafe { cJSON_IsString(item) } == 0 {
        return ptr::null_mut();
    }
    unsafe { (*item).valuestring }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    if unsafe { cJSON_IsNumber(item) } == 0 {
        return -c_double::NAN;
    }
    unsafe { (*item).valuedouble }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    unsafe {
        (*object).valueint = saturated_int(number);
        (*object).valuedouble = number;
    }
    number
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    if object.is_null()
        || unsafe { (*object).r#type } & CJSON_STRING == 0
        || unsafe { (*object).r#type } & CJSON_IS_REFERENCE != 0
    {
        return ptr::null_mut();
    }
    if unsafe { (*object).valuestring.is_null() } || valuestring.is_null() {
        return ptr::null_mut();
    }

    let new_length = unsafe { strlen(valuestring) };
    let old_length = unsafe { strlen((*object).valuestring) };
    if new_length <= old_length {
        let old = unsafe { (*object).valuestring };
        if !(valuestring.wrapping_add(new_length) < old
            || old.wrapping_add(old_length) < valuestring.cast_mut())
        {
            return ptr::null_mut();
        }
        unsafe { strcpy(old, valuestring) };
        return old;
    }

    let global = unsafe { hooks() };
    let copy = unsafe { cjson_strdup(valuestring.cast(), &global) }.cast::<c_char>();
    if copy.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        cJSON_free((*object).valuestring.cast());
        (*object).valuestring = copy;
    }
    copy
}

unsafe fn create_scalar(kind: c_int) -> *mut cJSON {
    let global = unsafe { hooks() };
    let item = unsafe { new_item(&global) };
    if !item.is_null() {
        unsafe { (*item).r#type = kind };
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    unsafe { create_scalar(CJSON_NULL) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    unsafe { create_scalar(CJSON_TRUE) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    unsafe { create_scalar(CJSON_FALSE) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    unsafe {
        create_scalar(if boolean != 0 {
            CJSON_TRUE
        } else {
            CJSON_FALSE
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(number: c_double) -> *mut cJSON {
    let item = unsafe { create_scalar(CJSON_NUMBER) };
    if !item.is_null() {
        unsafe {
            (*item).valuedouble = number;
            (*item).valueint = saturated_int(number);
        }
    }
    item
}

unsafe fn create_owned_string(value: *const c_char, kind: c_int) -> *mut cJSON {
    let global = unsafe { hooks() };
    let item = unsafe { new_item(&global) };
    if !item.is_null() {
        unsafe {
            (*item).r#type = kind;
            (*item).valuestring = cjson_strdup(value.cast(), &global).cast();
            if (*item).valuestring.is_null() {
                cJSON_Delete(item);
                return ptr::null_mut();
            }
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    unsafe { create_owned_string(string, CJSON_STRING) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    unsafe { create_owned_string(raw, CJSON_RAW) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    unsafe { create_scalar(CJSON_ARRAY) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    unsafe { create_scalar(CJSON_OBJECT) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = unsafe { create_scalar(CJSON_STRING | CJSON_IS_REFERENCE) };
    if !item.is_null() {
        unsafe { (*item).valuestring = string.cast_mut() };
    }
    item
}

unsafe fn create_container_reference(child: *const cJSON, kind: c_int) -> *mut cJSON {
    let item = unsafe { create_scalar(kind | CJSON_IS_REFERENCE) };
    if !item.is_null() {
        unsafe { (*item).child = child.cast_mut() };
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    unsafe { create_container_reference(child, CJSON_OBJECT) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    unsafe { create_container_reference(child, CJSON_ARRAY) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    unsafe { (hooks().allocate)(size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    unsafe { (hooks().deallocate)(object) };
}

unsafe fn suffix_object(previous: *mut cJSON, item: *mut cJSON) {
    unsafe {
        (*previous).next = item;
        (*item).prev = previous;
    }
}

unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if item.is_null() || array.is_null() || array == item {
        return 0;
    }

    let child = unsafe { (*array).child };
    if child.is_null() {
        unsafe {
            (*array).child = item;
            (*item).prev = item;
            (*item).next = ptr::null_mut();
        }
    } else if !unsafe { (*child).prev }.is_null() {
        unsafe {
            suffix_object((*child).prev, item);
            (*child).prev = item;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    unsafe { add_item_to_array(array, item) }
}

unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
    local_hooks: &InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return 0;
    }

    let (new_key, new_type) = if constant_key != 0 {
        (
            string.cast_mut(),
            unsafe { (*item).r#type } | CJSON_STRING_IS_CONST,
        )
    } else {
        let key = unsafe { cjson_strdup(string.cast(), local_hooks) }.cast::<c_char>();
        if key.is_null() {
            return 0;
        }
        (key, unsafe { (*item).r#type } & !CJSON_STRING_IS_CONST)
    };

    if unsafe { (*item).r#type } & CJSON_STRING_IS_CONST == 0
        && !unsafe { (*item).string }.is_null()
    {
        unsafe { (local_hooks.deallocate)((*item).string.cast()) };
    }
    unsafe {
        (*item).string = new_key;
        (*item).r#type = new_type;
        add_item_to_array(object, item)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    let global = unsafe { hooks() };
    unsafe { add_item_to_object(object, string, item, &global, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    let global = unsafe { hooks() };
    unsafe { add_item_to_object(object, string, item, &global, 1) }
}

unsafe fn create_reference(item: *const cJSON, local_hooks: &InternalHooks) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }
    let reference = unsafe { new_item(local_hooks) };
    if reference.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        memcpy(reference.cast(), item.cast(), size_of::<cJSON>());
        (*reference).string = ptr::null_mut();
        (*reference).r#type |= CJSON_IS_REFERENCE;
        (*reference).next = ptr::null_mut();
        (*reference).prev = ptr::null_mut();
    }
    reference
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() {
        return 0;
    }
    let global = unsafe { hooks() };
    unsafe { add_item_to_array(array, create_reference(item, &global)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    if object.is_null() || string.is_null() {
        return 0;
    }
    let global = unsafe { hooks() };
    unsafe { add_item_to_object(object, string, create_reference(item, &global), &global, 0) }
}

unsafe fn add_created_to_object(
    object: *mut cJSON,
    name: *const c_char,
    item: *mut cJSON,
) -> *mut cJSON {
    let global = unsafe { hooks() };
    if unsafe { add_item_to_object(object, name, item, &global, 0) } != 0 {
        return item;
    }
    unsafe { cJSON_Delete(item) };
    ptr::null_mut()
}

macro_rules! add_simple_to_object {
    ($name:ident, $constructor:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
            unsafe { add_created_to_object(object, name, $constructor()) }
        }
    };
}

add_simple_to_object!(cJSON_AddNullToObject, cJSON_CreateNull);
add_simple_to_object!(cJSON_AddTrueToObject, cJSON_CreateTrue);
add_simple_to_object!(cJSON_AddFalseToObject, cJSON_CreateFalse);
add_simple_to_object!(cJSON_AddObjectToObject, cJSON_CreateObject);
add_simple_to_object!(cJSON_AddArrayToObject, cJSON_CreateArray);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    unsafe { add_created_to_object(object, name, cJSON_CreateBool(boolean)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    unsafe { add_created_to_object(object, name, cJSON_CreateNumber(number)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { add_created_to_object(object, name, cJSON_CreateString(string)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    unsafe { add_created_to_object(object, name, cJSON_CreateRaw(raw)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() {
        return 0;
    }
    let mut child = unsafe { (*array).child };
    let mut size: usize = 0;
    while !child.is_null() {
        size = size.wrapping_add(1);
        child = unsafe { (*child).next };
    }
    size as c_int
}

unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
    if array.is_null() {
        return ptr::null_mut();
    }
    let mut child = unsafe { (*array).child };
    while !child.is_null() && index > 0 {
        index -= 1;
        child = unsafe { (*child).next };
    }
    child
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 {
        return ptr::null_mut();
    }
    unsafe { get_array_item(array, index as usize) }
}

unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if object.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let mut current = unsafe { (*object).child };
    if case_sensitive != 0 {
        while !current.is_null()
            && !unsafe { (*current).string }.is_null()
            && unsafe { strcmp(name, (*current).string) } != 0
        {
            current = unsafe { (*current).next };
        }
    } else {
        while !current.is_null()
            && unsafe { case_insensitive_strcmp(name.cast(), (*current).string.cast()) } != 0
        {
            current = unsafe { (*current).next };
        }
    }
    if current.is_null() || unsafe { (*current).string }.is_null() {
        ptr::null_mut()
    } else {
        current
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { get_object_item(object, string, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { get_object_item(object, string, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    (!unsafe { cJSON_GetObjectItem(object, string) }.is_null()) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
    if parent.is_null()
        || item.is_null()
        || (item != unsafe { (*parent).child } && unsafe { (*item).prev }.is_null())
    {
        return ptr::null_mut();
    }

    unsafe {
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
pub unsafe extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    if which < 0 {
        return ptr::null_mut();
    }
    unsafe { cJSON_DetachItemViaPointer(array, get_array_item(array, which as usize)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromArray(array, which)) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { cJSON_DetachItemViaPointer(object, cJSON_GetObjectItem(object, string)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { cJSON_DetachItemViaPointer(object, cJSON_GetObjectItemCaseSensitive(object, string)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromObject(object, string)) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string)) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 || newitem.is_null() {
        return 0;
    }
    let after = unsafe { get_array_item(array, which as usize) };
    if after.is_null() {
        return unsafe { add_item_to_array(array, newitem) };
    }
    if after != unsafe { (*array).child } && unsafe { (*after).prev }.is_null() {
        return 0;
    }
    unsafe {
        (*newitem).next = after;
        (*newitem).prev = (*after).prev;
        (*after).prev = newitem;
        if after == (*array).child {
            (*array).child = newitem;
        } else {
            (*(*newitem).prev).next = newitem;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
    if parent.is_null()
        || unsafe { (*parent).child }.is_null()
        || replacement.is_null()
        || item.is_null()
    {
        return 0;
    }
    if replacement == item {
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
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 {
        return 0;
    }
    unsafe { cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem) }
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if replacement.is_null() || string.is_null() {
        return 0;
    }
    if unsafe { (*replacement).r#type } & CJSON_STRING_IS_CONST == 0
        && !unsafe { (*replacement).string }.is_null()
    {
        unsafe { cJSON_free((*replacement).string.cast()) };
    }
    let global = unsafe { hooks() };
    unsafe {
        (*replacement).string = cjson_strdup(string.cast(), &global).cast();
        if (*replacement).string.is_null() {
            return 0;
        }
        (*replacement).r#type &= !CJSON_STRING_IS_CONST;
        cJSON_ReplaceItemViaPointer(
            object,
            get_object_item(object, string, case_sensitive),
            replacement,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    unsafe { replace_item_in_object(object, string, newitem, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    unsafe { replace_item_in_object(object, string, newitem, 1) }
}

unsafe fn create_number_array(
    numbers: *const c_void,
    count: c_int,
    element_kind: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let array = unsafe { cJSON_CreateArray() };
    let mut previous = ptr::null_mut();
    let mut latest = ptr::null_mut();
    let mut index = 0usize;
    while !array.is_null() && index < count as usize {
        let number = unsafe {
            match element_kind {
                0 => *numbers.cast::<c_int>().add(index) as c_double,
                1 => *numbers.cast::<c_float>().add(index) as c_double,
                _ => *numbers.cast::<c_double>().add(index),
            }
        };
        latest = unsafe { cJSON_CreateNumber(number) };
        if latest.is_null() {
            unsafe { cJSON_Delete(array) };
            return ptr::null_mut();
        }
        if index == 0 {
            unsafe { (*array).child = latest };
        } else {
            unsafe { suffix_object(previous, latest) };
        }
        previous = latest;
        index += 1;
    }
    if !array.is_null() && !unsafe { (*array).child }.is_null() {
        unsafe { (*(*array).child).prev = latest };
    }
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    unsafe { create_number_array(numbers.cast(), count, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(
    numbers: *const c_float,
    count: c_int,
) -> *mut cJSON {
    unsafe { create_number_array(numbers.cast(), count, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
    unsafe { create_number_array(numbers.cast(), count, 2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || strings.is_null() {
        return ptr::null_mut();
    }
    let array = unsafe { cJSON_CreateArray() };
    let mut previous = ptr::null_mut();
    let mut latest = ptr::null_mut();
    let mut index = 0usize;
    while !array.is_null() && index < count as usize {
        latest = unsafe { cJSON_CreateString(*strings.add(index)) };
        if latest.is_null() {
            unsafe { cJSON_Delete(array) };
            return ptr::null_mut();
        }
        if index == 0 {
            unsafe { (*array).child = latest };
        } else {
            unsafe { suffix_object(previous, latest) };
        }
        previous = latest;
        index += 1;
    }
    if !array.is_null() && !unsafe { (*array).child }.is_null() {
        unsafe { (*(*array).child).prev = latest };
    }
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    unsafe { cJSON_Duplicate_rec(item, 0, recurse) }
}

unsafe fn cJSON_Duplicate_rec(item: *const cJSON, depth: usize, recurse: cJSON_bool) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }
    let global = unsafe { hooks() };
    let newitem = unsafe { new_item(&global) };
    if newitem.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*newitem).r#type = (*item).r#type & !CJSON_IS_REFERENCE;
        (*newitem).valueint = (*item).valueint;
        (*newitem).valuedouble = (*item).valuedouble;
        if !(*item).valuestring.is_null() {
            (*newitem).valuestring = cjson_strdup((*item).valuestring.cast(), &global).cast();
            if (*newitem).valuestring.is_null() {
                cJSON_Delete(newitem);
                return ptr::null_mut();
            }
        }
        if !(*item).string.is_null() {
            (*newitem).string = if (*item).r#type & CJSON_STRING_IS_CONST != 0 {
                (*item).string
            } else {
                cjson_strdup((*item).string.cast(), &global).cast()
            };
            if (*newitem).string.is_null() {
                cJSON_Delete(newitem);
                return ptr::null_mut();
            }
        }
        if recurse == 0 {
            return newitem;
        }
    }

    let mut child = unsafe { (*item).child };
    let mut tail: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();
    while !child.is_null() {
        if depth >= CJSON_CIRCULAR_LIMIT {
            unsafe { cJSON_Delete(newitem) };
            return ptr::null_mut();
        }
        newchild = unsafe { cJSON_Duplicate_rec(child, depth + 1, 1) };
        if newchild.is_null() {
            unsafe { cJSON_Delete(newitem) };
            return ptr::null_mut();
        }
        if !tail.is_null() {
            unsafe {
                (*tail).next = newchild;
                (*newchild).prev = tail;
            }
            tail = newchild;
        } else {
            unsafe { (*newitem).child = newchild };
            tail = newchild;
        }
        child = unsafe { (*child).next };
    }
    if !unsafe { (*newitem).child }.is_null() {
        unsafe { (*(*newitem).child).prev = newchild };
    }
    newitem
}

#[inline]
unsafe fn compare_double(left: c_double, right: c_double) -> bool {
    let max_value = unsafe { fabs(left) }.max(unsafe { fabs(right) });
    (unsafe { fabs(left - right) }) <= max_value * c_double::EPSILON
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    left: *const cJSON,
    right: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if left.is_null()
        || right.is_null()
        || (unsafe { (*left).r#type } & 0xff != unsafe { (*right).r#type } & 0xff)
    {
        return 0;
    }
    let kind = unsafe { (*left).r#type } & 0xff;
    if !matches!(
        kind,
        CJSON_FALSE
            | CJSON_TRUE
            | CJSON_NULL
            | CJSON_NUMBER
            | CJSON_STRING
            | CJSON_RAW
            | CJSON_ARRAY
            | CJSON_OBJECT
    ) {
        return 0;
    }
    if left == right {
        return 1;
    }

    match kind {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL => 1,
        CJSON_NUMBER => unsafe {
            compare_double((*left).valuedouble, (*right).valuedouble) as c_int
        },
        CJSON_STRING | CJSON_RAW => {
            if unsafe { (*left).valuestring }.is_null() || unsafe { (*right).valuestring }.is_null()
            {
                0
            } else {
                (unsafe { strcmp((*left).valuestring, (*right).valuestring) } == 0) as c_int
            }
        }
        CJSON_ARRAY => {
            let mut a = unsafe { (*left).child };
            let mut b = unsafe { (*right).child };
            while !a.is_null() && !b.is_null() {
                if unsafe { cJSON_Compare(a, b, case_sensitive) } == 0 {
                    return 0;
                }
                a = unsafe { (*a).next };
                b = unsafe { (*b).next };
            }
            (a == b) as c_int
        }
        CJSON_OBJECT => {
            let mut a = unsafe { (*left).child };
            while !a.is_null() {
                let b = unsafe { get_object_item(right, (*a).string, case_sensitive) };
                if b.is_null() || unsafe { cJSON_Compare(a, b, case_sensitive) } == 0 {
                    return 0;
                }
                a = unsafe { (*a).next };
            }
            let mut b = unsafe { (*right).child };
            while !b.is_null() {
                let a = unsafe { get_object_item(left, (*b).string, case_sensitive) };
                if a.is_null() || unsafe { cJSON_Compare(b, a, case_sensitive) } == 0 {
                    return 0;
                }
                b = unsafe { (*b).next };
            }
            1
        }
        _ => 0,
    }
}

unsafe fn skip_oneline_comment(input: &mut *mut c_char) {
    *input = input.wrapping_add(2);
    while unsafe { **input } != 0 {
        if unsafe { **input } == b'\n' as c_char {
            *input = input.wrapping_add(1);
            return;
        }
        *input = input.wrapping_add(1);
    }
}

unsafe fn skip_multiline_comment(input: &mut *mut c_char) {
    *input = input.wrapping_add(2);
    while unsafe { **input } != 0 {
        if unsafe { **input } == b'*' as c_char && unsafe { *(*input).add(1) } == b'/' as c_char {
            *input = input.wrapping_add(2);
            return;
        }
        *input = input.wrapping_add(1);
    }
}

unsafe fn minify_string(input: &mut *mut c_char, output: &mut *mut c_char) {
    unsafe { **output = **input };
    *input = input.wrapping_add(1);
    *output = output.wrapping_add(1);
    while unsafe { **input } != 0 {
        unsafe { **output = **input };
        if unsafe { **input } == b'"' as c_char {
            *input = input.wrapping_add(1);
            *output = output.wrapping_add(1);
            return;
        }
        if unsafe { **input } == b'\\' as c_char && unsafe { *(*input).add(1) } == b'"' as c_char {
            unsafe { *(*output).add(1) = *(*input).add(1) };
            *input = input.wrapping_add(1);
            *output = output.wrapping_add(1);
        }
        *input = input.wrapping_add(1);
        *output = output.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Minify(mut json: *mut c_char) {
    let mut into = json;
    if json.is_null() {
        return;
    }
    while unsafe { *json } != 0 {
        match unsafe { *json } as u8 {
            b' ' | b'\t' | b'\r' | b'\n' => json = json.wrapping_add(1),
            b'/' => {
                if unsafe { *json.add(1) } == b'/' as c_char {
                    unsafe { skip_oneline_comment(&mut json) };
                } else if unsafe { *json.add(1) } == b'*' as c_char {
                    unsafe { skip_multiline_comment(&mut json) };
                } else {
                    json = json.wrapping_add(1);
                }
            }
            b'"' => unsafe { minify_string(&mut json, &mut into) },
            _ => {
                unsafe { *into = *json };
                json = json.wrapping_add(1);
                into = into.wrapping_add(1);
            }
        }
    }
    unsafe { *into = 0 };
}

struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: InternalHooks,
}

#[inline]
fn can_read(buffer: &ParseBuffer, size: usize) -> bool {
    buffer.offset.wrapping_add(size) <= buffer.length
}

#[inline]
fn can_access(buffer: &ParseBuffer, index: usize) -> bool {
    buffer.offset.wrapping_add(index) < buffer.length
}

#[inline]
unsafe fn buffer_at_offset(buffer: &ParseBuffer) -> *const c_uchar {
    buffer.content.wrapping_add(buffer.offset)
}

unsafe fn buffer_skip_whitespace(buffer: &mut ParseBuffer) {
    if buffer.content.is_null() || !can_access(buffer, 0) {
        return;
    }
    while can_access(buffer, 0) && unsafe { *buffer_at_offset(buffer) } <= 32 {
        buffer.offset += 1;
    }
    if buffer.offset == buffer.length {
        buffer.offset -= 1;
    }
}

unsafe fn skip_utf8_bom(buffer: &mut ParseBuffer) {
    if buffer.content.is_null() || buffer.offset != 0 {
        return;
    }
    if can_access(buffer, 4)
        && unsafe { strncmp(buffer_at_offset(buffer).cast(), c"\xEF\xBB\xBF".as_ptr(), 3) } == 0
    {
        buffer.offset += 3;
    }
}

unsafe fn parse_number(item: *mut cJSON, input: &mut ParseBuffer) -> cJSON_bool {
    if input.content.is_null() {
        return 0;
    }
    let mut length = 0usize;
    let mut has_decimal_point = false;
    while can_access(input, length) {
        match unsafe { *buffer_at_offset(input).add(length) } {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => length += 1,
            b'.' => {
                length += 1;
                has_decimal_point = true;
            }
            _ => break,
        }
    }

    let number_string = unsafe { (input.hooks.allocate)(length + 1) }.cast::<c_uchar>();
    if number_string.is_null() {
        return 0;
    }
    unsafe {
        memcpy(number_string.cast(), buffer_at_offset(input).cast(), length);
        *number_string.add(length) = 0;
    }
    if has_decimal_point {
        for index in 0..length {
            if unsafe { *number_string.add(index) } == b'.' {
                unsafe { *number_string.add(index) = b'.' };
            }
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = unsafe { strtod(number_string.cast(), &mut after_end) };
    if number_string.cast::<c_char>() == after_end {
        unsafe { (input.hooks.deallocate)(number_string.cast()) };
        return 0;
    }
    unsafe {
        (*item).valuedouble = number;
        (*item).valueint = saturated_int(number);
        (*item).r#type = CJSON_NUMBER;
    }
    input.offset += unsafe { after_end.offset_from(number_string.cast()) } as usize;
    unsafe { (input.hooks.deallocate)(number_string.cast()) };
    1
}

unsafe fn parse_hex4(input: *const c_uchar) -> c_uint {
    let mut value = 0u32;
    for index in 0..4 {
        let byte = unsafe { *input.add(index) };
        value += match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'A'..=b'F' => 10 + (byte - b'A') as u32,
            b'a'..=b'f' => 10 + (byte - b'a') as u32,
            _ => return 0,
        };
        if index < 3 {
            value <<= 4;
        }
    }
    value
}

unsafe fn utf16_literal_to_utf8(
    input: *const c_uchar,
    input_end: *const c_uchar,
    output: &mut *mut c_uchar,
) -> c_uchar {
    if unsafe { input_end.offset_from(input) } < 6 {
        return 0;
    }
    let first_code = unsafe { parse_hex4(input.add(2)) };
    if (0xdc00..=0xdfff).contains(&first_code) {
        return 0;
    }

    let (mut codepoint, sequence_length) = if (0xd800..=0xdbff).contains(&first_code) {
        let second = input.wrapping_add(6);
        if unsafe { input_end.offset_from(second) } < 6
            || unsafe { *second } != b'\\'
            || unsafe { *second.add(1) } != b'u'
        {
            return 0;
        }
        let second_code = unsafe { parse_hex4(second.add(2)) };
        if !(0xdc00..=0xdfff).contains(&second_code) {
            return 0;
        }
        (
            0x10000u32 + (((first_code & 0x3ff) << 10) | (second_code & 0x3ff)),
            12,
        )
    } else {
        (first_code, 6)
    };

    let (utf8_length, first_mark) = if codepoint < 0x80 {
        (1u8, 0u8)
    } else if codepoint < 0x800 {
        (2, 0xc0)
    } else if codepoint < 0x10000 {
        (3, 0xe0)
    } else if codepoint <= 0x10ffff {
        (4, 0xf0)
    } else {
        return 0;
    };

    let mut position = utf8_length - 1;
    while position > 0 {
        unsafe { *output.add(position as usize) = ((codepoint | 0x80) & 0xbf) as u8 };
        codepoint >>= 6;
        position -= 1;
    }
    unsafe {
        **output = if utf8_length > 1 {
            ((codepoint | first_mark as u32) & 0xff) as u8
        } else {
            (codepoint & 0x7f) as u8
        };
    }
    *output = output.wrapping_add(utf8_length as usize);
    sequence_length
}

unsafe fn parse_string(item: *mut cJSON, input: &mut ParseBuffer) -> cJSON_bool {
    let start = unsafe { buffer_at_offset(input) };
    let mut input_pointer = start.wrapping_add(1);
    let mut input_end = start.wrapping_add(1);
    let output: *mut c_uchar;

    if unsafe { *start } != b'"' {
        return 0;
    }

    let mut skipped_bytes = 0usize;
    while (unsafe { input_end.offset_from(input.content) } as usize) < input.length
        && unsafe { *input_end } != b'"'
    {
        if unsafe { *input_end } == b'\\' {
            if unsafe { input_end.add(1).offset_from(input.content) } as usize >= input.length {
                input.offset = unsafe { input_pointer.offset_from(input.content) } as usize;
                return 0;
            }
            skipped_bytes += 1;
            input_end = input_end.wrapping_add(1);
        }
        input_end = input_end.wrapping_add(1);
    }
    if unsafe { input_end.offset_from(input.content) } as usize >= input.length
        || unsafe { *input_end } != b'"'
    {
        input.offset = unsafe { input_pointer.offset_from(input.content) } as usize;
        return 0;
    }

    let allocation_length = unsafe { input_end.offset_from(start) } as usize - skipped_bytes;
    output = unsafe { (input.hooks.allocate)(allocation_length + 1) }.cast();
    if output.is_null() {
        input.offset = unsafe { input_pointer.offset_from(input.content) } as usize;
        return 0;
    }
    let mut output_pointer = output;

    while input_pointer < input_end {
        if unsafe { *input_pointer } != b'\\' {
            unsafe { *output_pointer = *input_pointer };
            output_pointer = output_pointer.wrapping_add(1);
            input_pointer = input_pointer.wrapping_add(1);
            continue;
        }

        let mut sequence_length = 2u8;
        if unsafe { input_end.offset_from(input_pointer) } < 1 {
            unsafe { (input.hooks.deallocate)(output.cast()) };
            input.offset = unsafe { input_pointer.offset_from(input.content) } as usize;
            return 0;
        }
        let escaped = unsafe { *input_pointer.add(1) };
        let translated = match escaped {
            b'b' => b'\x08',
            b'f' => b'\x0c',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'"' | b'\\' | b'/' => escaped,
            b'u' => {
                sequence_length =
                    unsafe { utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer) };
                if sequence_length == 0 {
                    unsafe { (input.hooks.deallocate)(output.cast()) };
                    input.offset = unsafe { input_pointer.offset_from(input.content) } as usize;
                    return 0;
                }
                input_pointer = input_pointer.wrapping_add(sequence_length as usize);
                continue;
            }
            _ => {
                unsafe { (input.hooks.deallocate)(output.cast()) };
                input.offset = unsafe { input_pointer.offset_from(input.content) } as usize;
                return 0;
            }
        };
        unsafe { *output_pointer = translated };
        output_pointer = output_pointer.wrapping_add(1);
        input_pointer = input_pointer.wrapping_add(sequence_length as usize);
    }

    unsafe {
        *output_pointer = 0;
        (*item).r#type = CJSON_STRING;
        (*item).valuestring = output.cast();
    }
    input.offset = unsafe { input_end.offset_from(input.content) } as usize + 1;
    1
}

unsafe fn parse_array(item: *mut cJSON, input: &mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current: *mut cJSON = ptr::null_mut();
    if input.depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    input.depth += 1;
    if unsafe { *buffer_at_offset(input) } != b'[' {
        return 0;
    }
    input.offset += 1;
    unsafe { buffer_skip_whitespace(input) };
    if can_access(input, 0) && unsafe { *buffer_at_offset(input) } == b']' {
        input.depth -= 1;
        unsafe {
            (*item).r#type = CJSON_ARRAY;
            (*item).child = head;
        }
        input.offset += 1;
        return 1;
    }
    if !can_access(input, 0) {
        input.offset -= 1;
        return 0;
    }
    input.offset -= 1;

    loop {
        let new_item = unsafe { new_item(&input.hooks) };
        if new_item.is_null() {
            if !head.is_null() {
                unsafe { cJSON_Delete(head) };
            }
            return 0;
        }
        if head.is_null() {
            head = new_item;
            current = new_item;
        } else {
            unsafe {
                (*current).next = new_item;
                (*new_item).prev = current;
            }
            current = new_item;
        }
        input.offset += 1;
        unsafe { buffer_skip_whitespace(input) };
        if unsafe { parse_value(current, input) } == 0 {
            unsafe { cJSON_Delete(head) };
            return 0;
        }
        unsafe { buffer_skip_whitespace(input) };
        if !can_access(input, 0) || unsafe { *buffer_at_offset(input) } != b',' {
            break;
        }
    }
    if !can_access(input, 0) || unsafe { *buffer_at_offset(input) } != b']' {
        unsafe { cJSON_Delete(head) };
        return 0;
    }
    input.depth -= 1;
    if !head.is_null() {
        unsafe { (*head).prev = current };
    }
    unsafe {
        (*item).r#type = CJSON_ARRAY;
        (*item).child = head;
    }
    input.offset += 1;
    1
}

unsafe fn parse_object(item: *mut cJSON, input: &mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current: *mut cJSON = ptr::null_mut();
    if input.depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    input.depth += 1;
    if !can_access(input, 0) || unsafe { *buffer_at_offset(input) } != b'{' {
        return 0;
    }
    input.offset += 1;
    unsafe { buffer_skip_whitespace(input) };
    if can_access(input, 0) && unsafe { *buffer_at_offset(input) } == b'}' {
        input.depth -= 1;
        unsafe {
            (*item).r#type = CJSON_OBJECT;
            (*item).child = head;
        }
        input.offset += 1;
        return 1;
    }
    if !can_access(input, 0) {
        input.offset -= 1;
        return 0;
    }
    input.offset -= 1;

    loop {
        let new_item = unsafe { new_item(&input.hooks) };
        if new_item.is_null() {
            if !head.is_null() {
                unsafe { cJSON_Delete(head) };
            }
            return 0;
        }
        if head.is_null() {
            head = new_item;
            current = new_item;
        } else {
            unsafe {
                (*current).next = new_item;
                (*new_item).prev = current;
            }
            current = new_item;
        }
        if !can_access(input, 1) {
            unsafe { cJSON_Delete(head) };
            return 0;
        }
        input.offset += 1;
        unsafe { buffer_skip_whitespace(input) };
        if unsafe { parse_string(current, input) } == 0 {
            unsafe { cJSON_Delete(head) };
            return 0;
        }
        unsafe { buffer_skip_whitespace(input) };
        unsafe {
            (*current).string = (*current).valuestring;
            (*current).valuestring = ptr::null_mut();
        }
        if !can_access(input, 0) || unsafe { *buffer_at_offset(input) } != b':' {
            unsafe { cJSON_Delete(head) };
            return 0;
        }
        input.offset += 1;
        unsafe { buffer_skip_whitespace(input) };
        if unsafe { parse_value(current, input) } == 0 {
            unsafe { cJSON_Delete(head) };
            return 0;
        }
        unsafe { buffer_skip_whitespace(input) };
        if !can_access(input, 0) || unsafe { *buffer_at_offset(input) } != b',' {
            break;
        }
    }
    if !can_access(input, 0) || unsafe { *buffer_at_offset(input) } != b'}' {
        unsafe { cJSON_Delete(head) };
        return 0;
    }
    input.depth -= 1;
    if !head.is_null() {
        unsafe { (*head).prev = current };
    }
    unsafe {
        (*item).r#type = CJSON_OBJECT;
        (*item).child = head;
    }
    input.offset += 1;
    1
}

unsafe fn parse_value(item: *mut cJSON, input: &mut ParseBuffer) -> cJSON_bool {
    if input.content.is_null() {
        return 0;
    }
    let current = unsafe { buffer_at_offset(input) };
    if can_read(input, 4) && unsafe { strncmp(current.cast(), c"null".as_ptr(), 4) } == 0 {
        unsafe { (*item).r#type = CJSON_NULL };
        input.offset += 4;
        return 1;
    }
    if can_read(input, 5) && unsafe { strncmp(current.cast(), c"false".as_ptr(), 5) } == 0 {
        unsafe { (*item).r#type = CJSON_FALSE };
        input.offset += 5;
        return 1;
    }
    if can_read(input, 4) && unsafe { strncmp(current.cast(), c"true".as_ptr(), 4) } == 0 {
        unsafe {
            (*item).r#type = CJSON_TRUE;
            (*item).valueint = 1;
        }
        input.offset += 4;
        return 1;
    }
    if can_access(input, 0) && unsafe { *current } == b'"' {
        return unsafe { parse_string(item, input) };
    }
    if can_access(input, 0) && (unsafe { *current } == b'-' || unsafe { *current }.is_ascii_digit())
    {
        return unsafe { parse_number(item, input) };
    }
    if can_access(input, 0) && unsafe { *current } == b'[' {
        return unsafe { parse_array(item, input) };
    }
    if can_access(input, 0) && unsafe { *current } == b'{' {
        return unsafe { parse_object(item, input) };
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    unsafe {
        *GLOBAL_ERROR.get() = Error {
            json: ptr::null(),
            position: 0,
        };
    }
    let global = unsafe { hooks() };
    let mut buffer = ParseBuffer {
        content: ptr::null(),
        length: 0,
        offset: 0,
        depth: 0,
        hooks: global,
    };
    let mut item: *mut cJSON = ptr::null_mut();

    let success = if !value.is_null() && buffer_length != 0 {
        buffer.content = value.cast();
        buffer.length = buffer_length;
        item = unsafe { new_item(&global) };
        if item.is_null() {
            false
        } else {
            unsafe {
                skip_utf8_bom(&mut buffer);
                buffer_skip_whitespace(&mut buffer);
            }
            if unsafe { parse_value(item, &mut buffer) } == 0 {
                false
            } else if require_null_terminated != 0 {
                unsafe { buffer_skip_whitespace(&mut buffer) };
                buffer.offset < buffer.length && unsafe { *buffer_at_offset(&buffer) } == 0
            } else {
                true
            }
        }
    } else {
        false
    };

    if success {
        if !return_parse_end.is_null() {
            unsafe { *return_parse_end = buffer_at_offset(&buffer).cast() };
        }
        return item;
    }

    if !item.is_null() {
        unsafe { cJSON_Delete(item) };
    }
    if !value.is_null() {
        let position = if buffer.offset < buffer.length {
            buffer.offset
        } else if buffer.length > 0 {
            buffer.length - 1
        } else {
            0
        };
        let local_error = Error {
            json: value.cast(),
            position,
        };
        if !return_parse_end.is_null() {
            unsafe {
                *return_parse_end = local_error.json.wrapping_add(position).cast();
            }
        }
        unsafe { *GLOBAL_ERROR.get() = local_error };
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    if value.is_null() {
        return ptr::null_mut();
    }
    let length = unsafe { strlen(value) } + 1;
    unsafe { cJSON_ParseWithLengthOpts(value, length, return_parse_end, require_null_terminated) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    unsafe { cJSON_ParseWithOpts(value, ptr::null_mut(), 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLength(
    value: *const c_char,
    buffer_length: usize,
) -> *mut cJSON {
    unsafe { cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0) }
}

struct PrintBuffer {
    buffer: *mut c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: InternalHooks,
}

unsafe fn ensure(output: &mut PrintBuffer, mut needed: usize) -> *mut c_uchar {
    if output.buffer.is_null()
        || (output.length > 0 && output.offset >= output.length)
        || needed > c_int::MAX as usize
    {
        return ptr::null_mut();
    }
    needed = needed.wrapping_add(output.offset).wrapping_add(1);
    if needed <= output.length {
        return output.buffer.wrapping_add(output.offset);
    }
    if output.noalloc != 0 {
        return ptr::null_mut();
    }

    let new_size = if needed > c_int::MAX as usize / 2 {
        if needed <= c_int::MAX as usize {
            c_int::MAX as usize
        } else {
            return ptr::null_mut();
        }
    } else {
        needed * 2
    };

    let new_buffer = if let Some(reallocate) = output.hooks.reallocate {
        let resized = unsafe { reallocate(output.buffer.cast(), new_size) }.cast::<c_uchar>();
        if resized.is_null() {
            unsafe { (output.hooks.deallocate)(output.buffer.cast()) };
            output.length = 0;
            output.buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        resized
    } else {
        let resized = unsafe { (output.hooks.allocate)(new_size) }.cast::<c_uchar>();
        if resized.is_null() {
            unsafe { (output.hooks.deallocate)(output.buffer.cast()) };
            output.length = 0;
            output.buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        unsafe {
            memcpy(resized.cast(), output.buffer.cast(), output.offset + 1);
            (output.hooks.deallocate)(output.buffer.cast());
        }
        resized
    };
    output.length = new_size;
    output.buffer = new_buffer;
    new_buffer.wrapping_add(output.offset)
}

unsafe fn update_offset(output: &mut PrintBuffer) {
    if output.buffer.is_null() {
        return;
    }
    output.offset += unsafe { strlen(output.buffer.add(output.offset).cast()) };
}

unsafe fn print_number(item: *const cJSON, output: &mut PrintBuffer) -> cJSON_bool {
    let number = unsafe { (*item).valuedouble };
    let mut number_buffer = [0 as c_char; 26];
    let length;
    if number.is_nan() || number.is_infinite() {
        length = unsafe {
            snprintf(
                number_buffer.as_mut_ptr(),
                number_buffer.len(),
                c"null".as_ptr(),
            )
        };
    } else if number == unsafe { (*item).valueint } as c_double {
        length = unsafe {
            snprintf(
                number_buffer.as_mut_ptr(),
                number_buffer.len(),
                c"%d".as_ptr(),
                (*item).valueint,
            )
        };
    } else {
        let mut current_length = unsafe {
            snprintf(
                number_buffer.as_mut_ptr(),
                number_buffer.len(),
                c"%1.15g".as_ptr(),
                number,
            )
        };
        let mut test = 0.0;
        if unsafe {
            sscanf(
                number_buffer.as_ptr(),
                c"%lg".as_ptr(),
                &mut test as *mut c_double,
            )
        } != 1
            || !unsafe { compare_double(test, number) }
        {
            current_length = unsafe {
                snprintf(
                    number_buffer.as_mut_ptr(),
                    number_buffer.len(),
                    c"%1.17g".as_ptr(),
                    number,
                )
            };
        }
        length = current_length;
    }
    if length < 0 || length > number_buffer.len() as c_int - 1 {
        return 0;
    }

    let destination = unsafe { ensure(output, length as usize + 1) };
    if destination.is_null() {
        return 0;
    }
    for index in 0..length as usize {
        unsafe { *destination.add(index) = number_buffer[index] as c_uchar };
    }
    unsafe { *destination.add(length as usize) = 0 };
    output.offset += length as usize;
    1
}

unsafe fn print_string_ptr(input: *const c_uchar, output: &mut PrintBuffer) -> cJSON_bool {
    if input.is_null() {
        let destination = unsafe { ensure(output, 3) };
        if destination.is_null() {
            return 0;
        }
        unsafe { strcpy(destination.cast(), c"\"\"".as_ptr()) };
        return 1;
    }

    let mut input_pointer = input;
    let mut escape_characters = 0usize;
    while unsafe { *input_pointer } != 0 {
        match unsafe { *input_pointer } {
            b'"' | b'\\' | b'\x08' | b'\x0c' | b'\n' | b'\r' | b'\t' => escape_characters += 1,
            byte if byte < 32 => escape_characters += 5,
            _ => {}
        }
        input_pointer = input_pointer.wrapping_add(1);
    }
    let input_length = unsafe { input_pointer.offset_from(input) } as usize;
    let output_length = input_length + escape_characters;
    let destination = unsafe { ensure(output, output_length + 3) };
    if destination.is_null() {
        return 0;
    }
    if escape_characters == 0 {
        unsafe {
            *destination = b'"';
            memcpy(destination.add(1).cast(), input.cast(), output_length);
            *destination.add(output_length + 1) = b'"';
            *destination.add(output_length + 2) = 0;
        }
        return 1;
    }

    unsafe { *destination = b'"' };
    input_pointer = input;
    let mut output_pointer = destination.wrapping_add(1);
    while unsafe { *input_pointer } != 0 {
        let byte = unsafe { *input_pointer };
        if byte > 31 && byte != b'"' && byte != b'\\' {
            unsafe { *output_pointer = byte };
        } else {
            unsafe { *output_pointer = b'\\' };
            output_pointer = output_pointer.wrapping_add(1);
            match byte {
                b'\\' => unsafe { *output_pointer = b'\\' },
                b'"' => unsafe { *output_pointer = b'"' },
                b'\x08' => unsafe { *output_pointer = b'b' },
                b'\x0c' => unsafe { *output_pointer = b'f' },
                b'\n' => unsafe { *output_pointer = b'n' },
                b'\r' => unsafe { *output_pointer = b'r' },
                b'\t' => unsafe { *output_pointer = b't' },
                _ => {
                    unsafe {
                        snprintf(output_pointer.cast(), 6, c"u%04x".as_ptr(), byte as c_uint);
                    }
                    output_pointer = output_pointer.wrapping_add(4);
                }
            }
        }
        input_pointer = input_pointer.wrapping_add(1);
        output_pointer = output_pointer.wrapping_add(1);
    }
    unsafe {
        *destination.add(output_length + 1) = b'"';
        *destination.add(output_length + 2) = 0;
    }
    1
}

unsafe fn print_array(item: *const cJSON, output: &mut PrintBuffer) -> cJSON_bool {
    let mut destination = unsafe { ensure(output, 1) };
    if destination.is_null() {
        return 0;
    }
    unsafe { *destination = b'[' };
    output.offset += 1;
    output.depth += 1;
    let mut current = unsafe { (*item).child };
    while !current.is_null() {
        if unsafe { print_value(current, output) } == 0 {
            return 0;
        }
        unsafe { update_offset(output) };
        if !unsafe { (*current).next }.is_null() {
            let length = if output.format != 0 { 2 } else { 1 };
            destination = unsafe { ensure(output, length + 1) };
            if destination.is_null() {
                return 0;
            }
            unsafe { *destination = b',' };
            destination = destination.wrapping_add(1);
            if output.format != 0 {
                unsafe { *destination = b' ' };
                destination = destination.wrapping_add(1);
            }
            unsafe { *destination = 0 };
            output.offset += length;
        }
        current = unsafe { (*current).next };
    }
    destination = unsafe { ensure(output, 2) };
    if destination.is_null() {
        return 0;
    }
    unsafe {
        *destination = b']';
        *destination.add(1) = 0;
    }
    output.depth -= 1;
    1
}

unsafe fn print_object(item: *const cJSON, output: &mut PrintBuffer) -> cJSON_bool {
    let mut length = if output.format != 0 { 2 } else { 1 };
    let mut destination = unsafe { ensure(output, length + 1) };
    if destination.is_null() {
        return 0;
    }
    unsafe { *destination = b'{' };
    destination = destination.wrapping_add(1);
    output.depth += 1;
    if output.format != 0 {
        unsafe { *destination = b'\n' };
    }
    output.offset += length;

    let mut current = unsafe { (*item).child };
    while !current.is_null() {
        if output.format != 0 {
            destination = unsafe { ensure(output, output.depth) };
            if destination.is_null() {
                return 0;
            }
            for index in 0..output.depth {
                unsafe { *destination.add(index) = b'\t' };
            }
            output.offset += output.depth;
        }
        if unsafe { print_string_ptr((*current).string.cast(), output) } == 0 {
            return 0;
        }
        unsafe { update_offset(output) };

        length = if output.format != 0 { 2 } else { 1 };
        destination = unsafe { ensure(output, length) };
        if destination.is_null() {
            return 0;
        }
        unsafe { *destination = b':' };
        destination = destination.wrapping_add(1);
        if output.format != 0 {
            unsafe { *destination = b'\t' };
        }
        output.offset += length;

        if unsafe { print_value(current, output) } == 0 {
            return 0;
        }
        unsafe { update_offset(output) };

        length = (output.format != 0) as usize + (!unsafe { (*current).next }.is_null()) as usize;
        destination = unsafe { ensure(output, length + 1) };
        if destination.is_null() {
            return 0;
        }
        if !unsafe { (*current).next }.is_null() {
            unsafe { *destination = b',' };
            destination = destination.wrapping_add(1);
        }
        if output.format != 0 {
            unsafe { *destination = b'\n' };
            destination = destination.wrapping_add(1);
        }
        unsafe { *destination = 0 };
        output.offset += length;
        current = unsafe { (*current).next };
    }

    destination = unsafe {
        ensure(
            output,
            if output.format != 0 {
                output.depth + 1
            } else {
                2
            },
        )
    };
    if destination.is_null() {
        return 0;
    }
    if output.format != 0 {
        for _ in 0..output.depth - 1 {
            unsafe { *destination = b'\t' };
            destination = destination.wrapping_add(1);
        }
    }
    unsafe {
        *destination = b'}';
        *destination.add(1) = 0;
    }
    output.depth -= 1;
    1
}

unsafe fn print_value(item: *const cJSON, output: &mut PrintBuffer) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    match unsafe { (*item).r#type } & 0xff {
        CJSON_NULL => {
            let destination = unsafe { ensure(output, 5) };
            if destination.is_null() {
                0
            } else {
                unsafe { strcpy(destination.cast(), c"null".as_ptr()) };
                1
            }
        }
        CJSON_FALSE => {
            let destination = unsafe { ensure(output, 6) };
            if destination.is_null() {
                0
            } else {
                unsafe { strcpy(destination.cast(), c"false".as_ptr()) };
                1
            }
        }
        CJSON_TRUE => {
            let destination = unsafe { ensure(output, 5) };
            if destination.is_null() {
                0
            } else {
                unsafe { strcpy(destination.cast(), c"true".as_ptr()) };
                1
            }
        }
        CJSON_NUMBER => unsafe { print_number(item, output) },
        CJSON_RAW => {
            if unsafe { (*item).valuestring }.is_null() {
                return 0;
            }
            let length = unsafe { strlen((*item).valuestring) } + 1;
            let destination = unsafe { ensure(output, length) };
            if destination.is_null() {
                0
            } else {
                unsafe { memcpy(destination.cast(), (*item).valuestring.cast(), length) };
                1
            }
        }
        CJSON_STRING => unsafe { print_string_ptr((*item).valuestring.cast(), output) },
        CJSON_ARRAY => unsafe { print_array(item, output) },
        CJSON_OBJECT => unsafe { print_object(item, output) },
        _ => 0,
    }
}

unsafe fn print_allocated(
    item: *const cJSON,
    format: cJSON_bool,
    local_hooks: &InternalHooks,
) -> *mut c_uchar {
    let mut output = PrintBuffer {
        buffer: unsafe { (local_hooks.allocate)(256) }.cast(),
        length: 256,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format,
        hooks: *local_hooks,
    };
    if output.buffer.is_null() || unsafe { print_value(item, &mut output) } == 0 {
        if !output.buffer.is_null() {
            unsafe { (local_hooks.deallocate)(output.buffer.cast()) };
        }
        return ptr::null_mut();
    }
    unsafe { update_offset(&mut output) };

    if let Some(reallocate) = local_hooks.reallocate {
        let printed =
            unsafe { reallocate(output.buffer.cast(), output.offset + 1) }.cast::<c_uchar>();
        if printed.is_null() {
            unsafe { (local_hooks.deallocate)(output.buffer.cast()) };
            return ptr::null_mut();
        }
        printed
    } else {
        let printed = unsafe { (local_hooks.allocate)(output.offset + 1) }.cast::<c_uchar>();
        if printed.is_null() {
            unsafe { (local_hooks.deallocate)(output.buffer.cast()) };
            return ptr::null_mut();
        }
        unsafe {
            memcpy(
                printed.cast(),
                output.buffer.cast(),
                output.length.min(output.offset + 1),
            );
            *printed.add(output.offset) = 0;
            (local_hooks.deallocate)(output.buffer.cast());
        }
        printed
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    let global = unsafe { hooks() };
    unsafe { print_allocated(item, 1, &global) }.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    let global = unsafe { hooks() };
    unsafe { print_allocated(item, 0, &global) }.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(
    item: *const cJSON,
    prebuffer: c_int,
    format: cJSON_bool,
) -> *mut c_char {
    if prebuffer < 0 {
        return ptr::null_mut();
    }
    let global = unsafe { hooks() };
    let mut output = PrintBuffer {
        buffer: unsafe { (global.allocate)(prebuffer as usize) }.cast(),
        length: prebuffer as usize,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format,
        hooks: global,
    };
    if output.buffer.is_null() {
        return ptr::null_mut();
    }
    if unsafe { print_value(item, &mut output) } == 0 {
        unsafe { (global.deallocate)(output.buffer.cast()) };
        return ptr::null_mut();
    }
    output.buffer.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintPreallocated(
    item: *mut cJSON,
    buffer: *mut c_char,
    length: c_int,
    format: cJSON_bool,
) -> cJSON_bool {
    if length < 0 || buffer.is_null() {
        return 0;
    }
    let mut output = PrintBuffer {
        buffer: buffer.cast(),
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: unsafe { hooks() },
    };
    unsafe { print_value(item, &mut output) }
}

#[repr(C)]
pub struct record {
    precision: *const c_char,
    lat: c_double,
    lon: c_double,
    address: *const c_char,
    city: *const c_char,
    state: *const c_char,
    zip: *const c_char,
    country: *const c_char,
}

unsafe fn driver_print_preallocated(root: *mut cJSON) -> c_int {
    let output = unsafe { cJSON_Print(root) };
    let length = unsafe { strlen(output) } + 5;
    let buffer = unsafe { malloc(length) }.cast::<c_char>();
    if buffer.is_null() {
        unsafe {
            puts(c"Failed to allocate memory.".as_ptr());
            exit(1);
        }
    }
    let failure_length = unsafe { strlen(output) };
    let failure_buffer = unsafe { malloc(failure_length) }.cast::<c_char>();
    if failure_buffer.is_null() {
        unsafe {
            puts(c"Failed to allocate memory.".as_ptr());
            exit(1);
        }
    }

    if unsafe { cJSON_PrintPreallocated(root, buffer, length as c_int, 1) } == 0 {
        unsafe { puts(c"cJSON_PrintPreallocated failed!".as_ptr()) };
        if unsafe { strcmp(output, buffer) } != 0 {
            unsafe {
                puts(c"cJSON_PrintPreallocated not the same as cJSON_Print!".as_ptr());
                puts(c"cJSON_Print result:".as_ptr());
                puts(output);
                puts(c"cJSON_PrintPreallocated result:".as_ptr());
                puts(buffer);
            }
        }
        unsafe {
            free(output.cast());
            free(failure_buffer.cast());
            free(buffer.cast());
        }
        return -1;
    }
    unsafe { puts(buffer) };

    if unsafe { cJSON_PrintPreallocated(root, failure_buffer, failure_length as c_int, 1) } != 0 {
        unsafe {
            puts(
                c"cJSON_PrintPreallocated failed to show error with insufficient memory!".as_ptr(),
            );
            puts(c"cJSON_Print result:".as_ptr());
            puts(output);
            puts(c"cJSON_PrintPreallocated result:".as_ptr());
            puts(failure_buffer);
            free(output.cast());
            free(failure_buffer.cast());
            free(buffer.cast());
        }
        return -1;
    }
    unsafe {
        free(output.cast());
        free(failure_buffer.cast());
        free(buffer.cast());
    }
    0
}

unsafe fn driver_check_print(root: *mut cJSON) {
    if unsafe { driver_print_preallocated(root) } != 0 {
        unsafe {
            cJSON_Delete(root);
            exit(1);
        }
    }
    unsafe { cJSON_Delete(root) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *mut [c_int; 3],
    ids: *mut c_int,
    fields: *mut record,
) -> c_int {
    unsafe {
        printf(c"Version: %s\n".as_ptr(), cJSON_Version());
    }

    let root = unsafe { cJSON_CreateObject() };
    unsafe {
        cJSON_AddItemToObject(
            root,
            c"name".as_ptr(),
            cJSON_CreateString(c"Jack (\"Bee\") Nimble".as_ptr()),
        );
    }
    let format = unsafe { cJSON_CreateObject() };
    unsafe {
        cJSON_AddItemToObject(root, c"format".as_ptr(), format);
        cJSON_AddStringToObject(format, c"type".as_ptr(), c"rect".as_ptr());
        cJSON_AddNumberToObject(format, c"width".as_ptr(), 1920.0);
        cJSON_AddNumberToObject(format, c"height".as_ptr(), 1080.0);
        cJSON_AddFalseToObject(format, c"interlace".as_ptr());
        cJSON_AddNumberToObject(format, c"frame rate".as_ptr(), 24.0);
        driver_check_print(root);
    }

    let root = unsafe { cJSON_CreateStringArray(strings, 7) };
    unsafe { driver_check_print(root) };

    let root = unsafe { cJSON_CreateArray() };
    for index in 0..3 {
        unsafe {
            cJSON_AddItemToArray(
                root,
                cJSON_CreateIntArray((*numbers.add(index)).as_ptr(), 3),
            );
        }
    }
    unsafe { driver_check_print(root) };

    let root = unsafe { cJSON_CreateObject() };
    let image = unsafe { cJSON_CreateObject() };
    unsafe {
        cJSON_AddItemToObject(root, c"Image".as_ptr(), image);
        cJSON_AddNumberToObject(image, c"Width".as_ptr(), 800.0);
        cJSON_AddNumberToObject(image, c"Height".as_ptr(), 600.0);
        cJSON_AddStringToObject(image, c"Title".as_ptr(), c"View from 15th Floor".as_ptr());
    }
    let thumbnail = unsafe { cJSON_CreateObject() };
    unsafe {
        cJSON_AddItemToObject(image, c"Thumbnail".as_ptr(), thumbnail);
        cJSON_AddStringToObject(
            thumbnail,
            c"Url".as_ptr(),
            c"http:/*www.example.com/image/481989943".as_ptr(),
        );
        cJSON_AddNumberToObject(thumbnail, c"Height".as_ptr(), 125.0);
        cJSON_AddStringToObject(thumbnail, c"Width".as_ptr(), c"100".as_ptr());
        cJSON_AddItemToObject(image, c"IDs".as_ptr(), cJSON_CreateIntArray(ids, 4));
        driver_check_print(root);
    }

    let root = unsafe { cJSON_CreateArray() };
    for index in 0..2 {
        let field = unsafe { &*fields.add(index) };
        let object = unsafe { cJSON_CreateObject() };
        unsafe {
            cJSON_AddItemToArray(root, object);
            cJSON_AddStringToObject(object, c"precision".as_ptr(), field.precision);
            cJSON_AddNumberToObject(object, c"Latitude".as_ptr(), field.lat);
            cJSON_AddNumberToObject(object, c"Longitude".as_ptr(), field.lon);
            cJSON_AddStringToObject(object, c"Address".as_ptr(), field.address);
            cJSON_AddStringToObject(object, c"City".as_ptr(), field.city);
            cJSON_AddStringToObject(object, c"State".as_ptr(), field.state);
            cJSON_AddStringToObject(object, c"Zip".as_ptr(), field.zip);
            cJSON_AddStringToObject(object, c"Country".as_ptr(), field.country);
        }
    }
    unsafe { driver_check_print(root) };

    let root = unsafe { cJSON_CreateObject() };
    unsafe {
        cJSON_AddNumberToObject(root, c"number".as_ptr(), c_double::INFINITY);
        driver_check_print(root);
    }
    0
}
