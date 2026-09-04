//! Public and private jansson data types (`jansson.h` / `jansson_private.h`).

use crate::hashtable::hashtable_t;
use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicUsize, Ordering};

pub const JANSSON_MAJOR_VERSION: c_int = 2;
pub const JANSSON_MINOR_VERSION: c_int = 15;
pub const JANSSON_MICRO_VERSION: c_int = 0;
pub const JANSSON_VERSION: &[u8] = b"2.15.0\0";

pub const JSON_PARSER_MAX_DEPTH: usize = 2048;

/* json_type */
pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

pub type json_int_t = i64;

#[repr(C)]
pub struct json_t {
    pub type_: c_int,
    pub refcount: usize,
}

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

#[repr(C)]
pub struct json_error_t {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

/* enum json_error_code */
pub const json_error_unknown: c_int = 0;
pub const json_error_out_of_memory: c_int = 1;
pub const json_error_stack_overflow: c_int = 2;
pub const json_error_cannot_open_file: c_int = 3;
pub const json_error_invalid_argument: c_int = 4;
pub const json_error_invalid_utf8: c_int = 5;
pub const json_error_premature_end_of_input: c_int = 6;
pub const json_error_end_of_input_expected: c_int = 7;
pub const json_error_invalid_syntax: c_int = 8;
pub const json_error_invalid_format: c_int = 9;
pub const json_error_wrong_type: c_int = 10;
pub const json_error_null_character: c_int = 11;
pub const json_error_null_value: c_int = 12;
pub const json_error_null_byte_in_key: c_int = 13;
pub const json_error_duplicate_key: c_int = 14;
pub const json_error_numeric_overflow: c_int = 15;
pub const json_error_item_not_found: c_int = 16;
pub const json_error_index_out_of_range: c_int = 17;

/* decoding flags */
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

/* encoding flags */
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;

/* pack/unpack flags */
pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

pub type json_load_callback_t =
    Option<unsafe extern "C" fn(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize>;
pub type json_dump_callback_t =
    Option<unsafe extern "C" fn(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int>;

pub type json_malloc_t = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
pub type json_realloc_t = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type json_free_t = Option<unsafe extern "C" fn(*mut c_void)>;

/* ------------------------------------------------------------------ */
/* private container types                                            */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct json_object_t {
    pub json: json_t,
    pub hashtable: hashtable_t,
}

#[repr(C)]
pub struct json_array_t {
    pub json: json_t,
    pub size: usize,
    pub entries: usize,
    pub table: *mut *mut json_t,
}

#[repr(C)]
pub struct json_string_t {
    pub json: json_t,
    pub value: *mut c_char,
    pub length: usize,
}

#[repr(C)]
pub struct json_real_t {
    pub json: json_t,
    pub value: f64,
}

#[repr(C)]
pub struct json_integer_t {
    pub json: json_t,
    pub value: json_int_t,
}

/// `LOOP_KEY_LEN`: space for "0x", two hex digits per pointer byte, and a NUL.
pub const LOOP_KEY_LEN: usize = 2 + (core::mem::size_of::<*mut json_t>() * 2) + 1;

/* ------------------------------------------------------------------ */
/* inline helpers                                                     */
/* ------------------------------------------------------------------ */

#[inline]
pub unsafe fn json_typeof(json: *const json_t) -> c_int {
    unsafe { (*json).type_ }
}

#[inline]
pub unsafe fn json_is_object(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_OBJECT
}

#[inline]
pub unsafe fn json_is_array(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_ARRAY
}

#[inline]
pub unsafe fn json_is_string(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_STRING
}

#[inline]
pub unsafe fn json_is_integer(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_INTEGER
}

#[inline]
pub unsafe fn json_is_real(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_REAL
}

#[inline]
pub unsafe fn json_is_number(json: *const json_t) -> bool {
    unsafe { json_is_integer(json) || json_is_real(json) }
}

#[inline]
pub unsafe fn json_is_true(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_TRUE
}

#[inline]
pub unsafe fn json_is_false(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_FALSE
}

#[inline]
pub unsafe fn json_is_boolean(json: *const json_t) -> bool {
    unsafe { json_is_true(json) || json_is_false(json) }
}

#[inline]
pub unsafe fn json_is_null(json: *const json_t) -> bool {
    !json.is_null() && unsafe { json_typeof(json) } == JSON_NULL
}

#[inline]
unsafe fn refcount_atomic(json: *mut json_t) -> &'static AtomicUsize {
    unsafe { AtomicUsize::from_ptr(core::ptr::addr_of_mut!((*json).refcount)) }
}

/// `json_incref()` (JSON_HAVE_ATOMIC_BUILTINS variant).
#[inline]
pub unsafe fn json_incref(json: *mut json_t) -> *mut json_t {
    unsafe {
        if !json.is_null() && (*json).refcount != usize::MAX {
            refcount_atomic(json).fetch_add(1, Ordering::Acquire);
        }
        json
    }
}

/// `json_decref()` (JSON_HAVE_ATOMIC_BUILTINS variant).
#[inline]
pub unsafe fn json_decref(json: *mut json_t) {
    unsafe {
        if !json.is_null() && (*json).refcount != usize::MAX {
            if refcount_atomic(json).fetch_sub(1, Ordering::Release) == 1 {
                crate::value::json_delete(json);
            }
        }
    }
}

/* container_of helpers */

#[inline]
pub unsafe fn json_to_object(json: *const json_t) -> *mut json_object_t {
    json as *mut json_object_t
}

#[inline]
pub unsafe fn json_to_array(json: *const json_t) -> *mut json_array_t {
    json as *mut json_array_t
}

#[inline]
pub unsafe fn json_to_string(json: *const json_t) -> *mut json_string_t {
    json as *mut json_string_t
}

#[inline]
pub unsafe fn json_to_real(json: *const json_t) -> *mut json_real_t {
    json as *mut json_real_t
}

#[inline]
pub unsafe fn json_to_integer(json: *const json_t) -> *mut json_integer_t {
    json as *mut json_integer_t
}
