use std::ffi::{c_char, c_double, c_int, c_void};

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
pub const CJSON_NESTING_LIMIT: usize = 1000;
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

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
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

#[derive(Clone, Copy)]
pub(crate) struct InternalHooks {
    pub allocate: unsafe extern "C" fn(usize) -> *mut c_void,
    pub deallocate: unsafe extern "C" fn(*mut c_void),
    pub reallocate: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
}

pub(crate) struct ParseBuffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
    pub hooks: InternalHooks,
}

pub(crate) struct PrintBuffer {
    pub buffer: *mut u8,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
    pub noalloc: cJSON_bool,
    pub format: cJSON_bool,
    pub hooks: InternalHooks,
}

pub(crate) struct ErrorInfo {
    pub json: *const u8,
    pub position: usize,
}
