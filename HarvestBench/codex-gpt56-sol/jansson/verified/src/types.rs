use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::AtomicUsize;

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

pub const JSON_MAX_INDENT: usize = 0x1f;
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;

#[repr(C)]
pub struct json_t {
    pub type_: c_int,
    pub refcount: AtomicUsize,
}

#[repr(C)]
pub struct json_error_t {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; 80],
    pub text: [c_char; 160],
}

pub type json_malloc_t = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
pub type json_realloc_t = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type json_free_t = Option<unsafe extern "C" fn(*mut c_void)>;
pub type json_load_callback_t =
    Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>;
pub type json_dump_callback_t =
    Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>;

pub struct Entry {
    pub key: Box<[u8]>,
    pub key_len: usize,
    pub value: *mut json_t,
    pub allocation: *mut c_void,
}

#[repr(C)]
pub struct JsonObject {
    pub json: json_t,
    pub entries: Vec<Box<Entry>>,
    pub buckets: *mut c_void,
}

#[repr(C)]
pub struct JsonArray {
    pub json: json_t,
    pub values: Vec<*mut json_t>,
    pub table: *mut c_void,
    pub table_capacity: usize,
}

#[repr(C)]
pub struct JsonString {
    pub json: json_t,
    pub value: Vec<u8>,
    pub allocation: *mut c_void,
}

#[repr(C)]
pub struct JsonInteger {
    pub json: json_t,
    pub value: i64,
}

#[repr(C)]
pub struct JsonReal {
    pub json: json_t,
    pub value: f64,
}

#[inline]
pub unsafe fn type_of(json: *const json_t) -> Option<c_int> {
    json.as_ref().map(|j| j.type_)
}

#[inline]
pub unsafe fn is_type(json: *const json_t, ty: c_int) -> bool {
    type_of(json) == Some(ty)
}

#[inline]
pub unsafe fn object_mut(json: *mut json_t) -> &'static mut JsonObject {
    &mut *json.cast()
}

#[inline]
pub unsafe fn object_ref(json: *const json_t) -> &'static JsonObject {
    &*json.cast()
}

#[inline]
pub unsafe fn array_mut(json: *mut json_t) -> &'static mut JsonArray {
    &mut *json.cast()
}

#[inline]
pub unsafe fn array_ref(json: *const json_t) -> &'static JsonArray {
    &*json.cast()
}

#[inline]
pub unsafe fn string_mut(json: *mut json_t) -> &'static mut JsonString {
    &mut *json.cast()
}

#[inline]
pub unsafe fn string_ref(json: *const json_t) -> &'static JsonString {
    &*json.cast()
}

#[inline]
pub unsafe fn integer_mut(json: *mut json_t) -> &'static mut JsonInteger {
    &mut *json.cast()
}

#[inline]
pub unsafe fn integer_ref(json: *const json_t) -> &'static JsonInteger {
    &*json.cast()
}

#[inline]
pub unsafe fn real_mut(json: *mut json_t) -> &'static mut JsonReal {
    &mut *json.cast()
}

#[inline]
pub unsafe fn real_ref(json: *const json_t) -> &'static JsonReal {
    &*json.cast()
}

pub fn base(ty: c_int) -> json_t {
    json_t {
        type_: ty,
        refcount: AtomicUsize::new(1),
    }
}
