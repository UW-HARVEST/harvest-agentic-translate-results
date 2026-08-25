use std::ffi::{c_char, c_double, c_int, c_uchar, c_void};
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
pub const CJSON_IS_REFERENCE: c_int = 256;
pub const CJSON_STRING_IS_CONST: c_int = 512;
pub const CJSON_NESTING_LIMIT: usize = 1000;
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

pub type cJSON_bool = c_int;
pub type AllocateFn = unsafe extern "C" fn(usize) -> *mut c_void;
pub type DeallocateFn = unsafe extern "C" fn(*mut c_void);
pub type ReallocateFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;

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
    pub malloc_fn: Option<AllocateFn>,
    pub free_fn: Option<DeallocateFn>,
}

#[derive(Clone, Copy)]
pub(crate) struct InternalHooks {
    pub allocate: Option<AllocateFn>,
    pub deallocate: Option<DeallocateFn>,
    pub reallocate: Option<ReallocateFn>,
}

#[derive(Clone, Copy)]
pub(crate) struct Error {
    pub json: *const c_uchar,
    pub position: usize,
}

unsafe impl Sync for Error {}

unsafe extern "C" {
    pub(crate) fn malloc(size: usize) -> *mut c_void;
    pub(crate) fn free(pointer: *mut c_void);
    pub(crate) fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    pub(crate) fn strlen(string: *const c_char) -> usize;
    pub(crate) fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    pub(crate) fn strncmp(left: *const c_char, right: *const c_char, count: usize) -> c_int;
    pub(crate) fn strcpy(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    pub(crate) fn memcpy(
        destination: *mut c_void,
        source: *const c_void,
        count: usize,
    ) -> *mut c_void;
    pub(crate) fn memset(destination: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    pub(crate) fn tolower(character: c_int) -> c_int;
    pub(crate) fn strtod(string: *const c_char, end: *mut *mut c_char) -> c_double;
    pub(crate) fn sprintf(destination: *mut c_char, format: *const c_char, ...) -> c_int;
    pub(crate) fn sscanf(source: *const c_char, format: *const c_char, ...) -> c_int;
    pub(crate) fn printf(format: *const c_char, ...) -> c_int;
    pub(crate) fn exit(status: c_int) -> !;
}

pub(crate) static mut GLOBAL_ERROR: Error = Error {
    json: ptr::null(),
    position: 0,
};

pub(crate) static mut GLOBAL_HOOKS: InternalHooks = InternalHooks {
    allocate: Some(malloc),
    deallocate: Some(free),
    reallocate: Some(realloc),
};

#[inline]
pub(crate) unsafe fn allocate(hooks: &InternalHooks, size: usize) -> *mut c_void {
    match hooks.allocate {
        Some(function) => function(size),
        None => ptr::null_mut(),
    }
}

#[inline]
pub(crate) unsafe fn deallocate(hooks: &InternalHooks, pointer: *mut c_void) {
    if let Some(function) = hooks.deallocate {
        function(pointer);
    }
}

#[inline]
pub(crate) unsafe fn reallocate(
    hooks: &InternalHooks,
    pointer: *mut c_void,
    size: usize,
) -> *mut c_void {
    match hooks.reallocate {
        Some(function) => function(pointer, size),
        None => ptr::null_mut(),
    }
}

pub(crate) unsafe fn new_item(hooks: &InternalHooks) -> *mut cJSON {
    let node = allocate(hooks, size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        memset(node.cast(), 0, size_of::<cJSON>());
    }
    node
}

pub(crate) unsafe fn duplicate_string(
    string: *const c_uchar,
    hooks: &InternalHooks,
) -> *mut c_uchar {
    if string.is_null() {
        return ptr::null_mut();
    }
    let length = strlen(string.cast()) + 1;
    let copy = allocate(hooks, length) as *mut c_uchar;
    if copy.is_null() {
        return ptr::null_mut();
    }
    memcpy(copy.cast(), string.cast(), length);
    copy
}

pub(crate) unsafe fn case_insensitive_strcmp(
    mut string1: *const c_uchar,
    mut string2: *const c_uchar,
) -> c_int {
    if string1.is_null() || string2.is_null() {
        return 1;
    }
    if string1 == string2 {
        return 0;
    }
    while tolower(*string1 as c_int) == tolower(*string2 as c_int) {
        if *string1 == 0 {
            return 0;
        }
        string1 = string1.add(1);
        string2 = string2.add(1);
    }
    tolower(*string1 as c_int) - tolower(*string2 as c_int)
}

#[inline]
pub(crate) fn is_type(item: *const cJSON, expected: c_int) -> cJSON_bool {
    if item.is_null() {
        0
    } else {
        unsafe { (((*item).type_ & 0xff) == expected) as cJSON_bool }
    }
}

#[inline]
pub(crate) fn clamp_int(number: c_double) -> c_int {
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
